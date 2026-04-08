use std::time::Duration;

use rdkafka::admin::AdminClient;
use rdkafka::client::DefaultClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::{Headers, Message};
use rdkafka::TopicPartitionList;
use rdkafka::Offset;

use super::{Backend, BrokerInfo, MessageInfo, QueueInfo};
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
            name: format!("Apache Kafka ({} broker{})", metadata.brokers().len(), if metadata.brokers().len() == 1 { "" } else { "s" }),
            cluster: broker_list.join(", "),
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
                match consumer.fetch_watermarks(topic_name, partition.id(), Duration::from_secs(5)) {
                    Ok((low, high)) => {
                        total_messages += (high - low) as u64;
                    }
                    Err(_) => {}
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
