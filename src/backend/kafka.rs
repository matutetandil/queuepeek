use std::time::Duration;

use rdkafka::admin::AdminClient;
use rdkafka::client::DefaultClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::producer::Producer;
use rdkafka::message::{Headers, Message};
use rdkafka::TopicPartitionList;
use rdkafka::Offset;

use super::{Backend, BrokerInfo, ConsumerGroupInfo, ConsumerGroupPartition, DetailEntry, DetailSection, MessageInfo, OffsetResetStrategy, PermissionEntry, QueueInfo};
use crate::config::Profile;

pub struct KafkaBackend {
    broker: String,
    username: String,
    password: String,
    tls: bool,
    tls_ca: Option<String>,
    tls_cert: Option<String>,
    tls_key: Option<String>,
}

impl KafkaBackend {
    pub fn new(profile: &Profile) -> Result<Self, String> {
        Ok(Self {
            broker: format!("{}:{}", profile.host, profile.port),
            username: profile.username.clone(),
            password: profile.password.clone(),
            tls: profile.tls.unwrap_or(false),
            tls_ca: profile.tls_ca.clone(),
            tls_cert: profile.tls_cert.clone(),
            tls_key: profile.tls_key.clone(),
        })
    }

    fn base_config(&self) -> ClientConfig {
        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", &self.broker);
        config.set("socket.timeout.ms", "10000");
        config.set("request.timeout.ms", "10000");

        if !self.username.is_empty() {
            let protocol = if self.tls { "SASL_SSL" } else { "SASL_PLAINTEXT" };
            config.set("security.protocol", protocol);
            config.set("sasl.mechanisms", "PLAIN");
            config.set("sasl.username", &self.username);
            config.set("sasl.password", &self.password);
        } else if self.tls {
            config.set("security.protocol", "SSL");
        }

        if let Some(ref ca) = self.tls_ca {
            config.set("ssl.ca.location", ca);
        }
        if let Some(ref cert) = self.tls_cert {
            config.set("ssl.certificate.location", cert);
        }
        if let Some(ref key) = self.tls_key {
            config.set("ssl.key.location", key);
        }

        config
    }

    fn make_admin(&self) -> Result<AdminClient<DefaultClientContext>, String> {
        self.base_config()
            .create()
            .map_err(|e| format!("Creating Kafka admin client: {}", e))
    }

    fn make_consumer(&self, group_id: &str) -> Result<BaseConsumer, String> {
        self.base_config()
            .set("group.id", group_id)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "latest")
            .create()
            .map_err(|e| format!("Creating Kafka consumer: {}", e))
    }
}

impl Backend for KafkaBackend {
    fn backend_type(&self) -> &str { "kafka" }
    fn broker_info(&self) -> Result<BrokerInfo, String> {
        let admin = self.make_admin()?;
        let metadata = admin
            .inner()
            .fetch_metadata(None, Duration::from_secs(10))
            .map_err(|e| format!("Fetching Kafka metadata: {}", e))?;

        let broker_list: Vec<String> = metadata
            .brokers()
            .iter()
            .map(|b| format!("{}:{}", b.host(), b.port()))
            .collect();

        Ok(BrokerInfo {
            _name: format!("Apache Kafka ({} broker{})", metadata.brokers().len(), if metadata.brokers().len() == 1 { "" } else { "s" }),
            _cluster: broker_list.join(", "),
        })
    }

    fn list_namespaces(&self) -> Result<Vec<String>, String> {
        Ok(vec!["default".to_string()])
    }

    fn list_queues(&self, _namespace: &str) -> Result<Vec<QueueInfo>, String> {
        let consumer: BaseConsumer = self.base_config()
            .set("group.id", "queuepeek-metadata")
            .create()
            .map_err(|e| format!("Creating Kafka consumer: {}", e))?;

        let metadata = consumer
            .fetch_metadata(None, Duration::from_secs(10))
            .map_err(|e| format!("Fetching Kafka metadata: {}", e))?;

        let mut queues = Vec::new();

        for topic in metadata.topics() {
            let topic_name = topic.name();

            // Skip internal topics
            if topic_name.starts_with("__") {
                continue;
            }

            // Calculate total messages from watermarks
            let mut total_messages: u64 = 0;
            for partition in topic.partitions() {
                if let Ok((low, high)) = consumer.fetch_watermarks(topic_name, partition.id(), Duration::from_secs(5)) {
                    total_messages += (high - low) as u64;
                }
            }

            queues.push(QueueInfo {
                name: topic_name.to_string(),
                messages: total_messages,
                consumers: 0,
                state: if topic.partitions().is_empty() { "empty".to_string() } else { "active".to_string() },
                publish_rate: 0.0,
                deliver_rate: 0.0,
            });
        }

        queues.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(queues)
    }

    fn peek_messages(&self, _namespace: &str, queue: &str, count: u32) -> Result<Vec<MessageInfo>, String> {
        let group_id = format!("queuepeek-peek-{}", uuid::Uuid::new_v4());
        let consumer = self.make_consumer(&group_id)?;

        // Get topic metadata to find partitions
        let metadata = consumer
            .fetch_metadata(Some(queue), Duration::from_secs(10))
            .map_err(|e| format!("Fetching topic metadata: {}", e))?;

        let topic_metadata = metadata
            .topics()
            .first()
            .ok_or_else(|| format!("Topic '{}' not found", queue))?;

        if topic_metadata.partitions().is_empty() {
            return Ok(Vec::new());
        }

        // Assign partitions with offsets near the end
        let mut tpl = TopicPartitionList::new();
        let msgs_per_partition = (count as i64 / topic_metadata.partitions().len() as i64).max(1);

        for partition in topic_metadata.partitions() {
            let pid = partition.id();
            match consumer.fetch_watermarks(queue, pid, Duration::from_secs(5)) {
                Ok((low, high)) => {
                    let offset = (high - msgs_per_partition).max(low);
                    tpl.add_partition_offset(queue, pid, Offset::Offset(offset))
                        .map_err(|e| format!("Setting partition offset: {}", e))?;
                }
                Err(e) => {
                    return Err(format!("Fetching watermarks for partition {}: {}", pid, e));
                }
            }
        }

        consumer
            .assign(&tpl)
            .map_err(|e| format!("Assigning partitions: {}", e))?;

        // Poll messages
        let mut messages = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);

        while messages.len() < count as usize && std::time::Instant::now() < deadline {
            match consumer.poll(Duration::from_millis(500)) {
                Some(Ok(msg)) => {
                    let body = match msg.payload_view::<str>() {
                        Some(Ok(s)) => s.to_string(),
                        Some(Err(_)) => {
                            // Binary payload — show as hex
                            msg.payload()
                                .map(|b| b.iter().map(|byte| format!("{:02x}", byte)).collect::<Vec<_>>().join(" "))
                                .unwrap_or_default()
                        }
                        None => String::new(),
                    };

                    let mut headers = Vec::new();
                    headers.push(("partition".to_string(), msg.partition().to_string()));
                    headers.push(("offset".to_string(), msg.offset().to_string()));

                    if let Some(key) = msg.key() {
                        let key_str = String::from_utf8_lossy(key).to_string();
                        headers.push(("key".to_string(), key_str));
                    }

                    if let Some(rdkafka_headers) = msg.headers() {
                        for header in rdkafka_headers.iter() {
                            let value = header.value
                                .map(|v| String::from_utf8_lossy(v).to_string())
                                .unwrap_or_default();
                            headers.push((header.key.to_string(), value));
                        }
                    }

                    let timestamp = match msg.timestamp() {
                        rdkafka::Timestamp::CreateTime(ts) | rdkafka::Timestamp::LogAppendTime(ts) => Some(ts / 1000),
                        rdkafka::Timestamp::NotAvailable => None,
                    };

                    messages.push(MessageInfo {
                        index: messages.len() + 1,
                        routing_key: format!("partition-{}", msg.partition()),
                        exchange: queue.to_string(),
                        redelivered: false,
                        timestamp,
                        content_type: String::new(),
                        headers,
                        body,
                    });
                }
                Some(Err(e)) => {
                    return Err(format!("Consuming message: {}", e));
                }
                None => {
                    // No message available, check if we've waited enough
                    if messages.is_empty() && std::time::Instant::now() > deadline - Duration::from_secs(5) {
                        break;
                    }
                }
            }
        }

        Ok(messages)
    }

    fn publish_message(
        &self,
        _namespace: &str,
        queue: &str,
        body: &str,
        routing_key: &str,
        headers: &[(String, String)],
        _content_type: &str,
    ) -> Result<(), String> {
        use rdkafka::producer::{BaseProducer, BaseRecord};
        use rdkafka::message::OwnedHeaders;

        let producer: BaseProducer = self.base_config()
            .set("message.timeout.ms", "5000")
            .create()
            .map_err(|e| format!("Creating Kafka producer: {}", e))?;

        let mut record = BaseRecord::to(queue).payload(body);

        let key_str;
        if !routing_key.is_empty() && !routing_key.starts_with("partition-") {
            key_str = routing_key.to_string();
            record = record.key(&key_str);
        }

        if !headers.is_empty() {
            let mut kafka_headers = OwnedHeaders::new();
            for (k, v) in headers {
                kafka_headers = kafka_headers.insert(rdkafka::message::Header { key: k, value: Some(v.as_bytes()) });
            }
            record = record.headers(kafka_headers);
        }

        producer.send(record)
            .map_err(|(e, _)| format!("Sending message: {}", e))?;

        producer.flush(Duration::from_secs(5))
            .map_err(|e| format!("Flushing producer: {}", e))?;

        Ok(())
    }

    fn purge_queue(&self, _namespace: &str, queue: &str) -> Result<(), String> {
        use rdkafka::admin::{AdminOptions, NewTopic, TopicReplication};

        let admin = self.make_admin()?;
        let opts = AdminOptions::new();

        // Get partition count before deleting
        let metadata = admin
            .inner()
            .fetch_metadata(Some(queue), Duration::from_secs(10))
            .map_err(|e| format!("Fetching topic metadata: {}", e))?;

        let partition_count = metadata
            .topics()
            .first()
            .map(|t| t.partitions().len())
            .unwrap_or(1);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Creating runtime: {}", e))?;

        rt.block_on(async {
            // Delete the topic
            let results = admin.delete_topics(&[queue], &opts).await
                .map_err(|e| format!("Deleting topic: {}", e))?;
            for result in results {
                if let Err((_, e)) = result {
                    return Err(format!("Deleting topic '{}': {}", queue, e));
                }
            }

            // Wait for deletion to propagate
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Recreate with same partition count
            let new_topic = NewTopic::new(queue, partition_count as i32, TopicReplication::Fixed(1));
            let results = admin.create_topics(&[new_topic], &opts).await
                .map_err(|e| format!("Recreating topic: {}", e))?;
            for result in results {
                if let Err((_, e)) = result {
                    return Err(format!("Recreating topic '{}': {}", queue, e));
                }
            }

            Ok(())
        })
    }

    fn consume_messages(&self, _namespace: &str, queue: &str, count: u32) -> Result<Vec<MessageInfo>, String> {
        let group_id = format!("queuepeek-consume-{}", uuid::Uuid::new_v4());
        let consumer = self.make_consumer(&group_id)?;

        let metadata = consumer
            .fetch_metadata(Some(queue), Duration::from_secs(10))
            .map_err(|e| format!("Fetching topic metadata: {}", e))?;

        let topic_metadata = metadata
            .topics()
            .first()
            .ok_or_else(|| format!("Topic '{}' not found", queue))?;

        if topic_metadata.partitions().is_empty() {
            return Ok(Vec::new());
        }

        // Assign partitions from the beginning (low watermark)
        let mut tpl = TopicPartitionList::new();
        for partition in topic_metadata.partitions() {
            let pid = partition.id();
            match consumer.fetch_watermarks(queue, pid, Duration::from_secs(5)) {
                Ok((low, _high)) => {
                    tpl.add_partition_offset(queue, pid, Offset::Offset(low))
                        .map_err(|e| format!("Setting partition offset: {}", e))?;
                }
                Err(e) => {
                    return Err(format!("Fetching watermarks for partition {}: {}", pid, e));
                }
            }
        }

        consumer
            .assign(&tpl)
            .map_err(|e| format!("Assigning partitions: {}", e))?;

        let mut messages = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(30);

        while messages.len() < count as usize && std::time::Instant::now() < deadline {
            match consumer.poll(Duration::from_millis(500)) {
                Some(Ok(msg)) => {
                    let body = match msg.payload_view::<str>() {
                        Some(Ok(s)) => s.to_string(),
                        Some(Err(_)) => {
                            msg.payload()
                                .map(|b| b.iter().map(|byte| format!("{:02x}", byte)).collect::<Vec<_>>().join(" "))
                                .unwrap_or_default()
                        }
                        None => String::new(),
                    };

                    let mut headers = Vec::new();
                    headers.push(("partition".to_string(), msg.partition().to_string()));
                    headers.push(("offset".to_string(), msg.offset().to_string()));

                    if let Some(key) = msg.key() {
                        let key_str = String::from_utf8_lossy(key).to_string();
                        headers.push(("key".to_string(), key_str));
                    }

                    if let Some(rdkafka_headers) = msg.headers() {
                        for header in rdkafka_headers.iter() {
                            let value = header.value
                                .map(|v| String::from_utf8_lossy(v).to_string())
                                .unwrap_or_default();
                            headers.push((header.key.to_string(), value));
                        }
                    }

                    let timestamp = match msg.timestamp() {
                        rdkafka::Timestamp::CreateTime(ts) | rdkafka::Timestamp::LogAppendTime(ts) => Some(ts / 1000),
                        rdkafka::Timestamp::NotAvailable => None,
                    };

                    messages.push(MessageInfo {
                        index: messages.len() + 1,
                        routing_key: format!("partition-{}", msg.partition()),
                        exchange: queue.to_string(),
                        redelivered: false,
                        timestamp,
                        content_type: String::new(),
                        headers,
                        body,
                    });
                }
                Some(Err(e)) => {
                    return Err(format!("Consuming message: {}", e));
                }
                None => {
                    if messages.is_empty() && std::time::Instant::now() > deadline - Duration::from_secs(25) {
                        break;
                    }
                }
            }
        }

        Ok(messages)
    }

    fn delete_queue(&self, _namespace: &str, queue: &str) -> Result<(), String> {
        use rdkafka::admin::AdminOptions;

        let admin = self.make_admin()?;
        let opts = AdminOptions::new();
        let topics = &[queue];

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Creating runtime: {}", e))?;

        rt.block_on(async {
            let results = admin.delete_topics(topics, &opts).await
                .map_err(|e| format!("Deleting topic: {}", e))?;

            for result in results {
                if let Err((_, e)) = result {
                    return Err(format!("Deleting topic '{}': {}", queue, e));
                }
            }
            Ok(())
        })
    }

    fn consumer_groups(&self, _namespace: &str, queue: &str) -> Result<Vec<ConsumerGroupInfo>, String> {
        let admin = self.make_admin()?;

        // List all consumer groups
        let group_list = admin.inner()
            .fetch_group_list(None, Duration::from_secs(10))
            .map_err(|e| format!("Listing consumer groups: {}", e))?;

        // Get topic partition info for watermarks
        let metadata = admin.inner()
            .fetch_metadata(Some(queue), Duration::from_secs(10))
            .map_err(|e| format!("Fetching topic metadata: {}", e))?;

        let topic_metadata = metadata.topics().first()
            .ok_or_else(|| format!("Topic '{}' not found", queue))?;

        let partition_ids: Vec<i32> = topic_metadata.partitions().iter().map(|p| p.id()).collect();

        // Get high watermarks for lag calculation
        let watermark_consumer: BaseConsumer = self.base_config()
            .set("group.id", "queuepeek-groups-check")
            .create()
            .map_err(|e| format!("Creating consumer: {}", e))?;

        let mut high_watermarks: std::collections::HashMap<i32, i64> = std::collections::HashMap::new();
        for &pid in &partition_ids {
            if let Ok((_low, high)) = watermark_consumer.fetch_watermarks(queue, pid, Duration::from_secs(5)) {
                high_watermarks.insert(pid, high);
            }
        }

        let mut results = Vec::new();

        for group in group_list.groups() {
            let group_name = group.name();
            // Skip internal groups
            if group_name.starts_with("queuepeek-") { continue; }

            // Create a consumer with this group ID to check committed offsets
            let check_consumer: BaseConsumer = match self.base_config()
                .set("group.id", group_name)
                .set("enable.auto.commit", "false")
                .create() {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Build TPL for this topic
            let mut tpl = TopicPartitionList::new();
            for &pid in &partition_ids {
                tpl.add_partition(queue, pid);
            }

            // Fetch committed offsets for this group on this topic
            let committed = match check_consumer.committed_offsets(tpl, Duration::from_secs(5)) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut partitions = Vec::new();
            let mut has_offsets = false;

            for elem in committed.elements() {
                if elem.topic() != queue { continue; }
                let offset = match elem.offset() {
                    Offset::Offset(o) => o,
                    _ => -1,
                };
                if offset < 0 { continue; }
                has_offsets = true;
                let hw = high_watermarks.get(&elem.partition()).copied().unwrap_or(0);
                let lag = (hw - offset).max(0);
                partitions.push(ConsumerGroupPartition {
                    partition: elem.partition(),
                    current_offset: offset,
                    high_watermark: hw,
                    lag,
                });
            }

            if !has_offsets { continue; }

            let total_lag: i64 = partitions.iter().map(|p| p.lag).sum();
            partitions.sort_by_key(|p| p.partition);

            results.push(ConsumerGroupInfo {
                name: group_name.to_string(),
                state: group.state().to_string(),
                members: group.members().len() as u32,
                total_lag,
                partitions,
            });
        }

        results.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(results)
    }

    fn queue_detail(&self, _namespace: &str, queue: &str) -> Result<Vec<DetailSection>, String> {
        use rdkafka::admin::{AdminOptions, ResourceSpecifier};

        let admin = self.make_admin()?;
        let metadata = admin
            .inner()
            .fetch_metadata(Some(queue), Duration::from_secs(10))
            .map_err(|e| format!("Fetching topic metadata: {}", e))?;

        let topic_metadata = metadata
            .topics()
            .first()
            .ok_or_else(|| format!("Topic '{}' not found", queue))?;

        let mut sections = Vec::new();

        // General
        let mut general = Vec::new();
        general.push(DetailEntry::kv("Partitions", topic_metadata.partitions().len().to_string()));

        let mut total_messages: u64 = 0;
        let consumer: BaseConsumer = self.base_config()
            .set("group.id", "queuepeek-detail")
            .create()
            .map_err(|e| format!("Creating consumer: {}", e))?;

        for partition in topic_metadata.partitions() {
            if let Ok((low, high)) = consumer.fetch_watermarks(queue, partition.id(), Duration::from_secs(5)) {
                total_messages += (high - low) as u64;
            }
        }
        general.push(DetailEntry::kv("Total messages", format!("{}", total_messages)));
        sections.push(DetailSection { title: "General".into(), entries: general });

        // Partitions
        let mut partitions = Vec::new();
        for partition in topic_metadata.partitions() {
            let pid = partition.id();
            let leader = partition.leader();
            let replicas: Vec<String> = partition.replicas().iter().map(|r| r.to_string()).collect();
            let isr: Vec<String> = partition.isr().iter().map(|r| r.to_string()).collect();

            let watermark = consumer.fetch_watermarks(queue, pid, Duration::from_secs(5))
                .map(|(low, high)| format!("{} msgs (offsets {}..{})", high - low, low, high))
                .unwrap_or_else(|_| "unknown".to_string());

            partitions.push(DetailEntry::kv(
                format!("Partition {}", pid),
                format!("leader={} replicas=[{}] ISR=[{}] {}", leader, replicas.join(","), isr.join(","), watermark),
            ));
        }
        if !partitions.is_empty() {
            sections.push(DetailSection { title: "Partitions".into(), entries: partitions });
        }

        // Topic config via describe_configs
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Creating runtime: {}", e))?;

        let config_result = rt.block_on(async {
            let opts = AdminOptions::new();
            let resource = ResourceSpecifier::Topic(queue);
            admin.describe_configs(&[resource], &opts).await
                .map_err(|e| format!("Describing config: {}", e))
        });

        if let Ok(configs) = config_result {
            let mut config_entries = Vec::new();
            for resource in configs.into_iter().flatten() {
                // Show non-default, interesting config entries
                let interesting = [
                    "retention.ms", "retention.bytes", "cleanup.policy",
                    "compression.type", "segment.bytes", "max.message.bytes",
                    "min.insync.replicas", "replication.factor",
                ];
                for key in &interesting {
                    if let Some(entry) = resource.get(key) {
                        if let Some(ref value) = entry.value {
                            let display_value = if *key == "retention.ms" {
                                format_duration_ms(value)
                            } else if *key == "segment.bytes" || *key == "retention.bytes" || *key == "max.message.bytes" {
                                format_config_bytes(value)
                            } else {
                                value.clone()
                            };
                            config_entries.push(DetailEntry::kv(*key, display_value));
                        }
                    }
                }
            }
            if !config_entries.is_empty() {
                sections.push(DetailSection { title: "Configuration".into(), entries: config_entries });
            }
        }

        Ok(sections)
    }

    fn replay_messages(
        &self,
        _namespace: &str,
        topic: &str,
        start_offset: i64,
        end_offset: i64,
        dest_topic: &str,
    ) -> Result<u64, String> {
        use rdkafka::producer::{BaseProducer, BaseRecord};

        let group_id = format!("queuepeek-replay-{}", uuid::Uuid::new_v4());
        let consumer = self.make_consumer(&group_id)?;

        let metadata = consumer
            .fetch_metadata(Some(topic), Duration::from_secs(10))
            .map_err(|e| format!("Fetching metadata: {}", e))?;

        let topic_metadata = metadata.topics().first()
            .ok_or_else(|| "Topic not found".to_string())?;

        // Assign all partitions from start_offset
        let mut tpl = TopicPartitionList::new();
        for partition in topic_metadata.partitions() {
            tpl.add_partition_offset(topic, partition.id(), Offset::Offset(start_offset))
                .map_err(|e| format!("Setting offset: {}", e))?;
        }
        consumer.assign(&tpl).map_err(|e| format!("Assigning: {}", e))?;

        let producer: BaseProducer = self.base_config()
            .set("message.timeout.ms", "5000")
            .create()
            .map_err(|e| format!("Creating producer: {}", e))?;

        let mut replayed = 0u64;
        let deadline = std::time::Instant::now() + Duration::from_secs(60);

        while std::time::Instant::now() < deadline {
            match consumer.poll(Duration::from_millis(500)) {
                Some(Ok(msg)) => {
                    if end_offset > 0 && msg.offset() >= end_offset {
                        break;
                    }
                    let payload = msg.payload().unwrap_or(&[]);
                    let key: &[u8] = &[];
                    let record = BaseRecord::to(dest_topic).payload(payload).key(key);
                    producer.send(record).map_err(|(e, _)| format!("Send: {}", e))?;
                    replayed += 1;
                }
                Some(Err(e)) => return Err(format!("Consuming: {}", e)),
                None => {
                    if replayed > 0 { break; }
                }
            }
        }

        producer.flush(Duration::from_secs(5))
            .map_err(|e| format!("Flush: {}", e))?;

        Ok(replayed)
    }

    fn reset_consumer_group_offsets(
        &self,
        _namespace: &str,
        queue: &str,
        group: &str,
        strategy: OffsetResetStrategy,
    ) -> Result<String, String> {
        let consumer: BaseConsumer = self.base_config()
            .set("group.id", group)
            .set("enable.auto.commit", "false")
            .create()
            .map_err(|e| format!("Creating consumer: {}", e))?;

        let metadata = consumer
            .fetch_metadata(Some(queue), Duration::from_secs(10))
            .map_err(|e| format!("Fetching metadata: {}", e))?;

        let topic = metadata.topics().first()
            .ok_or_else(|| "Topic not found".to_string())?;

        let partition_ids: Vec<i32> = topic.partitions().iter().map(|p| p.id()).collect();

        let mut tpl = TopicPartitionList::new();

        match strategy {
            OffsetResetStrategy::Earliest => {
                for &pid in &partition_ids {
                    let (low, _) = consumer.fetch_watermarks(queue, pid, Duration::from_secs(5))
                        .map_err(|e| format!("Fetch watermarks: {}", e))?;
                    tpl.add_partition_offset(queue, pid, Offset::Offset(low))
                        .map_err(|e| format!("Adding offset: {}", e))?;
                }
            }
            OffsetResetStrategy::Latest => {
                for &pid in &partition_ids {
                    let (_, high) = consumer.fetch_watermarks(queue, pid, Duration::from_secs(5))
                        .map_err(|e| format!("Fetch watermarks: {}", e))?;
                    tpl.add_partition_offset(queue, pid, Offset::Offset(high))
                        .map_err(|e| format!("Adding offset: {}", e))?;
                }
            }
            OffsetResetStrategy::ToTimestamp(ts) => {
                let mut ts_tpl = TopicPartitionList::new();
                for &pid in &partition_ids {
                    ts_tpl.add_partition_offset(queue, pid, Offset::Offset(ts))
                        .map_err(|e| format!("Adding offset: {}", e))?;
                }
                tpl = consumer.offsets_for_times(ts_tpl, Duration::from_secs(10))
                    .map_err(|e| format!("Offsets for times: {}", e))?;
            }
            OffsetResetStrategy::ToOffset(offset) => {
                for &pid in &partition_ids {
                    tpl.add_partition_offset(queue, pid, Offset::Offset(offset))
                        .map_err(|e| format!("Adding offset: {}", e))?;
                }
            }
        }

        consumer.commit(&tpl, rdkafka::consumer::CommitMode::Sync)
            .map_err(|e| format!("Committing offsets: {}", e))?;

        let strategy_name = match strategy {
            OffsetResetStrategy::Earliest => "earliest".to_string(),
            OffsetResetStrategy::Latest => "latest".to_string(),
            OffsetResetStrategy::ToTimestamp(ts) => format!("timestamp {}", ts),
            OffsetResetStrategy::ToOffset(o) => format!("offset {}", o),
        };

        Ok(format!("Reset offsets for group '{}' on '{}' to {}", group, queue, strategy_name))
    }

    fn list_permissions(&self, _namespace: &str) -> Result<Vec<PermissionEntry>, String> {
        let mut entries = Vec::new();

        // Show connection security info as permission context
        let protocol = if !self.username.is_empty() {
            if self.tls { "SASL_SSL" } else { "SASL_PLAINTEXT" }
        } else if self.tls {
            "SSL"
        } else {
            "PLAINTEXT"
        };

        entries.push(PermissionEntry {
            user_or_principal: if self.username.is_empty() { "anonymous".to_string() } else { self.username.clone() },
            _resource_type: "broker".to_string(),
            resource_name: self.broker.clone(),
            permission: protocol.to_string(),
            _operation: "connect".to_string(),
            host: "*".to_string(),
        });

        if !self.username.is_empty() {
            entries.push(PermissionEntry {
                user_or_principal: self.username.clone(),
                _resource_type: "broker".to_string(),
                resource_name: "SASL/PLAIN".to_string(),
                permission: "authenticated".to_string(),
                _operation: "sasl".to_string(),
                host: self.broker.clone(),
            });
        }

        // Try to get broker configs for authorizer info
        let admin = self.make_admin()?;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Creating runtime: {}", e))?;

        use rdkafka::admin::{AdminOptions, ResourceSpecifier};
        let configs = rt.block_on(async {
            let opts = AdminOptions::new();
            let resource = ResourceSpecifier::Broker(0);
            admin.describe_configs(&[resource], &opts).await
                .map_err(|e| format!("Describe configs: {}", e))
        })?;

        for result in &configs {
            match result {
                Ok(config) => {
                    for entry in config.entries.iter() {
                        let name = &entry.name;
                        let value = entry.value.as_deref().unwrap_or("(null)");
                        // Filter for security-related configs
                        if name.contains("authorizer") || name.contains("acl")
                            || name.contains("security") || name.contains("sasl")
                            || name.contains("listener") || name.contains("ssl.client.auth")
                            || name.contains("principal") || name.contains("super.users")
                        {
                            entries.push(PermissionEntry {
                                user_or_principal: "(broker config)".to_string(),
                                _resource_type: "config".to_string(),
                                resource_name: name.clone(),
                                permission: value.to_string(),
                                _operation: "broker-setting".to_string(),
                                host: self.broker.clone(),
                            });
                        }
                    }
                }
                Err(e) => {
                    entries.push(PermissionEntry {
                        user_or_principal: "(error)".to_string(),
                        _resource_type: "broker".to_string(),
                        resource_name: format!("Config error: {}", e),
                        permission: "error".to_string(),
                        _operation: "describe_configs".to_string(),
                        host: self.broker.clone(),
                    });
                }
            }
        }

        Ok(entries)
    }

    fn clone_backend(&self) -> Box<dyn Backend> {
        Box::new(Self {
            broker: self.broker.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            tls: self.tls,
            tls_ca: self.tls_ca.clone(),
            tls_cert: self.tls_cert.clone(),
            tls_key: self.tls_key.clone(),
        })
    }
}

fn format_duration_ms(ms_str: &str) -> String {
    if let Ok(ms) = ms_str.parse::<i64>() {
        if ms < 0 { return "infinite".to_string(); }
        let secs = ms / 1000;
        if secs >= 86400 { format!("{}d", secs / 86400) }
        else if secs >= 3600 { format!("{}h", secs / 3600) }
        else if secs >= 60 { format!("{}m", secs / 60) }
        else { format!("{}s", secs) }
    } else {
        ms_str.to_string()
    }
}

fn format_config_bytes(bytes_str: &str) -> String {
    if let Ok(bytes) = bytes_str.parse::<i64>() {
        if bytes < 0 { return "infinite".to_string(); }
        let bytes = bytes as u64;
        if bytes >= 1_073_741_824 { format!("{:.1} GB", bytes as f64 / 1_073_741_824.0) }
        else if bytes >= 1_048_576 { format!("{:.1} MB", bytes as f64 / 1_048_576.0) }
        else if bytes >= 1_024 { format!("{:.1} KB", bytes as f64 / 1_024.0) }
        else { format!("{} B", bytes) }
    } else {
        bytes_str.to_string()
    }
}
