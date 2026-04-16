use reqwest::blocking::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

use super::{Backend, BindingInfo, BrokerInfo, DetailEntry, DetailSection, ExchangeInfo, MessageInfo, PermissionEntry, QueueInfo};
use crate::config::Profile;

// API response structs (same as current rabbit.rs)
#[derive(Deserialize)]
struct QueueApiResponse {
    name: String,
    #[serde(default)]
    messages: u64,
    #[serde(default)]
    consumers: u64,
    #[serde(default)]
    state: String,
    #[serde(default)]
    #[serde(rename = "vhost")]
    _vhost: String,
    #[serde(default)]
    message_stats: Option<MessageStats>,
}

#[derive(Deserialize)]
struct MessageStats {
    publish_details: Option<RateDetail>,
    deliver_details: Option<RateDetail>,
    #[serde(rename = "ack_details")]
    _ack_details: Option<RateDetail>,
}

#[derive(Deserialize)]
struct RateDetail {
    rate: f64,
}

#[derive(Deserialize)]
struct VhostResponse {
    name: String,
}

#[derive(Deserialize)]
struct OverviewResponse {
    cluster_name: String,
    #[serde(default)]
    rabbitmq_version: String,
}

#[derive(Deserialize)]
struct PeekResponse {
    #[serde(default)]
    redelivered: bool,
    #[serde(default)]
    exchange: String,
    #[serde(default)]
    routing_key: String,
    #[serde(default)]
    payload: String,
    #[serde(default)]
    properties: PeekProperties,
}

#[derive(Deserialize, Default)]
struct PeekProperties {
    #[serde(default)]
    content_type: Option<String>,
    #[serde(default)]
    timestamp: Option<i64>,
    #[serde(default)]
    headers: Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
struct PeekRequest {
    count: u32,
    ackmode: String,
    encoding: String,
    truncate: u32,
}

#[derive(serde::Serialize)]
struct PublishRequest {
    properties: PublishProperties,
    routing_key: String,
    payload: String,
    payload_encoding: String,
}

#[derive(serde::Serialize)]
struct PublishProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<serde_json::Value>,
}

pub struct RabbitMqBackend {
    client: Arc<Client>,
    base_url: String,
    username: String,
    password: String,
}

impl RabbitMqBackend {
    pub fn new(profile: &Profile) -> Result<Self, String> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(15));

        if profile.tls.unwrap_or(false) {
            if let Some(ref ca_path) = profile.tls_ca {
                let ca_cert = std::fs::read(ca_path).map_err(|e| format!("Reading CA cert: {}", e))?;
                let cert = reqwest::Certificate::from_pem(&ca_cert).map_err(|e| format!("Parsing CA cert: {}", e))?;
                builder = builder.add_root_certificate(cert);
            }
            if let (Some(cert_path), Some(key_path)) = (&profile.tls_cert, &profile.tls_key) {
                let cert_pem = std::fs::read(cert_path).map_err(|e| format!("Reading client cert: {}", e))?;
                let key_pem = std::fs::read(key_path).map_err(|e| format!("Reading client key: {}", e))?;
                let identity = reqwest::Identity::from_pkcs8_pem(&cert_pem, &key_pem)
                    .map_err(|e| format!("Parsing client identity: {}", e))?;
                builder = builder.identity(identity);
            }
        }

        let client = Arc::new(builder.build().map_err(|e| format!("Building HTTP client: {}", e))?);

        Ok(Self {
            client,
            base_url: profile.base_url(),
            username: profile.username.clone(),
            password: profile.password.clone(),
        })
    }

    fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let resp = self.client
            .get(format!("{}{}", self.base_url, path))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .map_err(|e| format!("Request to {}: {}", path, e))?;
        if !resp.status().is_success() {
            return Err(format!("{}: HTTP {}", path, resp.status()));
        }
        resp.json::<T>().map_err(|e| format!("Decoding {}: {}", path, e))
    }

    fn post_json<B: serde::Serialize>(&self, path: &str, body: &B) -> Result<reqwest::blocking::Response, String> {
        let resp = self.client
            .post(format!("{}{}", self.base_url, path))
            .basic_auth(&self.username, Some(&self.password))
            .json(body)
            .send()
            .map_err(|e| format!("POST {}: {}", path, e))?;
        if !resp.status().is_success() {
            return Err(format!("POST {}: HTTP {}", path, resp.status()));
        }
        Ok(resp)
    }

    fn http_delete(&self, path: &str) -> Result<(), String> {
        let resp = self.client
            .delete(format!("{}{}", self.base_url, path))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .map_err(|e| format!("DELETE {}: {}", path, e))?;
        if !resp.status().is_success() {
            return Err(format!("DELETE {}: HTTP {}", path, resp.status()));
        }
        Ok(())
    }
}

impl Backend for RabbitMqBackend {
    fn backend_type(&self) -> &str { "rabbitmq" }
    fn broker_info(&self) -> Result<BrokerInfo, String> {
        let resp: OverviewResponse = self.get("/api/overview")?;
        Ok(BrokerInfo {
            _name: format!("RabbitMQ {}", resp.rabbitmq_version),
            _cluster: resp.cluster_name,
        })
    }

    fn list_namespaces(&self) -> Result<Vec<String>, String> {
        let vhosts: Vec<VhostResponse> = self.get("/api/vhosts")?;
        Ok(vhosts.into_iter().map(|v| v.name).collect())
    }

    fn list_queues(&self, namespace: &str) -> Result<Vec<QueueInfo>, String> {
        let encoded = urlencoding::encode(namespace);
        let path = format!("/api/queues/{}", encoded);
        let api_queues: Vec<QueueApiResponse> = self.get(&path)?;

        Ok(api_queues.into_iter().map(|aq| {
            let (pub_rate, del_rate) = match aq.message_stats {
                Some(ref ms) => (
                    ms.publish_details.as_ref().map_or(0.0, |d| d.rate),
                    ms.deliver_details.as_ref().map_or(0.0, |d| d.rate),
                ),
                None => (0.0, 0.0),
            };
            QueueInfo {
                name: aq.name,
                messages: aq.messages,
                consumers: aq.consumers,
                state: aq.state,
                publish_rate: pub_rate,
                deliver_rate: del_rate,
            }
        }).collect())
    }

    fn peek_messages(&self, namespace: &str, queue: &str, count: u32) -> Result<Vec<MessageInfo>, String> {
        let encoded_ns = urlencoding::encode(namespace);
        let encoded_q = urlencoding::encode(queue);
        let path = format!("/api/queues/{}/{}/get", encoded_ns, encoded_q);

        let body = PeekRequest {
            count,
            ackmode: "ack_requeue_true".into(),
            encoding: "auto".into(),
            truncate: 50000,
        };

        let resp = self.client
            .post(format!("{}{}", self.base_url, path))
            .basic_auth(&self.username, Some(&self.password))
            .json(&body)
            .send()
            .map_err(|e| format!("Peeking messages: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Peeking messages: HTTP {}", resp.status()));
        }

        let peek_resp: Vec<PeekResponse> = resp.json().map_err(|e| format!("Decoding messages: {}", e))?;

        Ok(peek_resp.into_iter().enumerate().map(|(i, pr)| {
            // Extract headers from properties.headers JSON value
            let mut headers = Vec::new();
            if let Some(ref h) = pr.properties.headers {
                if let Some(obj) = h.as_object() {
                    for (k, v) in obj {
                        headers.push((k.clone(), v.to_string()));
                    }
                }
            }

            MessageInfo {
                index: i + 1,
                routing_key: pr.routing_key,
                exchange: pr.exchange,
                redelivered: pr.redelivered,
                timestamp: pr.properties.timestamp,
                content_type: pr.properties.content_type.unwrap_or_default(),
                headers,
                body: pr.payload,
            }
        }).collect())
    }

    fn publish_message(
        &self,
        namespace: &str,
        _queue: &str,
        body: &str,
        routing_key: &str,
        headers: &[(String, String)],
        content_type: &str,
    ) -> Result<(), String> {
        let encoded_ns = urlencoding::encode(namespace);
        // Publish via the default exchange (amq.default), routing_key = queue name
        let path = format!("/api/exchanges/{}/amq.default/publish", encoded_ns);

        let props_headers = if headers.is_empty() {
            None
        } else {
            let map: serde_json::Map<String, serde_json::Value> = headers
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            Some(serde_json::Value::Object(map))
        };

        let req = PublishRequest {
            properties: PublishProperties {
                content_type: if content_type.is_empty() { None } else { Some(content_type.to_string()) },
                headers: props_headers,
            },
            routing_key: routing_key.to_string(),
            payload: body.to_string(),
            payload_encoding: "string".to_string(),
        };

        self.post_json(&path, &req)?;
        Ok(())
    }

    fn publish_to_exchange(
        &self,
        namespace: &str,
        exchange: &str,
        body: &str,
        routing_key: &str,
        headers: &[(String, String)],
        content_type: &str,
    ) -> Result<(), String> {
        let encoded_ns = urlencoding::encode(namespace);
        let encoded_ex = urlencoding::encode(exchange);
        let path = format!("/api/exchanges/{}/{}/publish", encoded_ns, encoded_ex);

        let props_headers = if headers.is_empty() {
            None
        } else {
            let map: serde_json::Map<String, serde_json::Value> = headers
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            Some(serde_json::Value::Object(map))
        };

        let req = PublishRequest {
            properties: PublishProperties {
                content_type: if content_type.is_empty() { None } else { Some(content_type.to_string()) },
                headers: props_headers,
            },
            routing_key: routing_key.to_string(),
            payload: body.to_string(),
            payload_encoding: "string".to_string(),
        };

        self.post_json(&path, &req)?;
        Ok(())
    }

    fn delete_queue(&self, namespace: &str, queue: &str) -> Result<(), String> {
        let encoded_ns = urlencoding::encode(namespace);
        let encoded_q = urlencoding::encode(queue);
        let path = format!("/api/queues/{}/{}?if-unused=false&if-empty=false", encoded_ns, encoded_q);
        self.http_delete(&path)
    }

    fn purge_queue(&self, namespace: &str, queue: &str) -> Result<(), String> {
        let encoded_ns = urlencoding::encode(namespace);
        let encoded_q = urlencoding::encode(queue);
        let path = format!("/api/queues/{}/{}/contents", encoded_ns, encoded_q);
        self.http_delete(&path)
    }

    fn consume_messages(&self, namespace: &str, queue: &str, count: u32) -> Result<Vec<MessageInfo>, String> {
        let encoded_ns = urlencoding::encode(namespace);
        let encoded_q = urlencoding::encode(queue);
        let path = format!("/api/queues/{}/{}/get", encoded_ns, encoded_q);

        let body = PeekRequest {
            count,
            ackmode: "ack_requeue_false".into(),
            encoding: "auto".into(),
            truncate: 50000,
        };

        let resp = self.client
            .post(format!("{}{}", self.base_url, path))
            .basic_auth(&self.username, Some(&self.password))
            .json(&body)
            .send()
            .map_err(|e| format!("Consuming messages: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Consuming messages: HTTP {}", resp.status()));
        }

        let peek_resp: Vec<PeekResponse> = resp.json().map_err(|e| format!("Decoding messages: {}", e))?;

        Ok(peek_resp.into_iter().enumerate().map(|(i, pr)| {
            let mut headers = Vec::new();
            if let Some(ref h) = pr.properties.headers {
                if let Some(obj) = h.as_object() {
                    for (k, v) in obj {
                        headers.push((k.clone(), v.to_string()));
                    }
                }
            }

            MessageInfo {
                index: i + 1,
                routing_key: pr.routing_key,
                exchange: pr.exchange,
                redelivered: pr.redelivered,
                timestamp: pr.properties.timestamp,
                content_type: pr.properties.content_type.unwrap_or_default(),
                headers,
                body: pr.payload,
            }
        }).collect())
    }

    fn queue_detail(&self, namespace: &str, queue: &str) -> Result<Vec<DetailSection>, String> {
        let encoded_ns = urlencoding::encode(namespace);
        let encoded_q = urlencoding::encode(queue);
        let path = format!("/api/queues/{}/{}", encoded_ns, encoded_q);
        let data: serde_json::Value = self.get(&path)?;

        let mut sections = Vec::new();

        // General
        let mut general = Vec::new();
        if let Some(t) = data.get("type").and_then(|v| v.as_str()) {
            general.push(DetailEntry::kv("Type", t));
        }
        if let Some(s) = data.get("state").and_then(|v| v.as_str()) {
            general.push(DetailEntry::kv("State", s));
        }
        if let Some(n) = data.get("node").and_then(|v| v.as_str()) {
            general.push(DetailEntry::kv("Node", n));
        }
        if let Some(v) = data.get("vhost").and_then(|v| v.as_str()) {
            general.push(DetailEntry::kv("Vhost", v));
        }
        if let Some(d) = data.get("durable").and_then(|v| v.as_bool()) {
            general.push(DetailEntry::kv("Durable", if d { "yes" } else { "no" }));
        }
        if let Some(ad) = data.get("auto_delete").and_then(|v| v.as_bool()) {
            general.push(DetailEntry::kv("Auto-delete", if ad { "yes" } else { "no" }));
        }
        if let Some(ex) = data.get("exclusive").and_then(|v| v.as_bool()) {
            general.push(DetailEntry::kv("Exclusive", if ex { "yes" } else { "no" }));
        }
        if let Some(idle) = data.get("idle_since").and_then(|v| v.as_str()) {
            general.push(DetailEntry::kv("Idle since", idle));
        }
        if !general.is_empty() {
            sections.push(DetailSection { title: "General".into(), entries: general });
        }

        // Messages
        let mut messages = Vec::new();
        if let Some(n) = data.get("messages").and_then(|v| v.as_u64()) {
            messages.push(DetailEntry::kv("Total", format_number(n)));
        }
        if let Some(n) = data.get("messages_ready").and_then(|v| v.as_u64()) {
            messages.push(DetailEntry::kv("Ready", format_number(n)));
        }
        if let Some(n) = data.get("messages_unacknowledged").and_then(|v| v.as_u64()) {
            messages.push(DetailEntry::kv("Unacked", format_number(n)));
        }
        if !messages.is_empty() {
            sections.push(DetailSection { title: "Messages".into(), entries: messages });
        }

        // Consumers
        let mut consumers = Vec::new();
        if let Some(n) = data.get("consumers").and_then(|v| v.as_u64()) {
            consumers.push(DetailEntry::kv("Count", format_number(n)));
        }
        if let Some(utilisation) = data.get("consumer_utilisation").and_then(|v| v.as_f64()) {
            consumers.push(DetailEntry::kv("Utilisation", format!("{:.1}%", utilisation * 100.0)));
        }
        if !consumers.is_empty() {
            sections.push(DetailSection { title: "Consumers".into(), entries: consumers });
        }

        // Consumer Details
        if let Some(details) = data.get("consumer_details").and_then(|v| v.as_array()) {
            for (i, cd) in details.iter().enumerate() {
                let mut entries = Vec::new();

                if let Some(tag) = cd.get("consumer_tag").and_then(|v| v.as_str()) {
                    entries.push(DetailEntry::kv("Tag", tag));
                }

                if let Some(ch) = cd.get("channel_details") {
                    if let Some(host) = ch.get("peer_host").and_then(|v| v.as_str()) {
                        let port = ch.get("peer_port").and_then(|v| v.as_u64()).unwrap_or(0);
                        entries.push(DetailEntry::kv("Address", format!("{}:{}", host, port)));
                    }
                    if let Some(conn) = ch.get("connection_name").and_then(|v| v.as_str()) {
                        if !conn.is_empty() {
                            entries.push(DetailEntry::kv("Connection", conn));
                        }
                    }
                    if let Some(name) = ch.get("name").and_then(|v| v.as_str()) {
                        entries.push(DetailEntry::kv("Channel", name));
                    }
                }

                if let Some(prefetch) = cd.get("prefetch_count").and_then(|v| v.as_u64()) {
                    entries.push(DetailEntry::kv("Prefetch", format_number(prefetch)));
                }
                if let Some(ack) = cd.get("ack_required").and_then(|v| v.as_bool()) {
                    entries.push(DetailEntry::kv("Ack required", if ack { "yes" } else { "no" }));
                }

                if !entries.is_empty() {
                    sections.push(DetailSection {
                        title: format!("Consumer #{}", i + 1),
                        entries,
                    });
                }
            }
        }

        // Rates
        let mut rates = Vec::new();
        if let Some(ms) = data.get("message_stats") {
            let pub_rate = ms.get("publish_details").and_then(|d| d.get("rate")).and_then(|r| r.as_f64()).unwrap_or(0.0);
            let del_rate = ms.get("deliver_details").and_then(|d| d.get("rate")).and_then(|r| r.as_f64()).unwrap_or(0.0);
            let ack_rate = ms.get("ack_details").and_then(|d| d.get("rate")).and_then(|r| r.as_f64()).unwrap_or(0.0);
            let redel_rate = ms.get("redeliver_details").and_then(|d| d.get("rate")).and_then(|r| r.as_f64()).unwrap_or(0.0);

            rates.push(DetailEntry::rate("Publish", format!("{:.1}/s", pub_rate), pub_rate));
            rates.push(DetailEntry::rate("Deliver", format!("{:.1}/s", del_rate), del_rate));
            rates.push(DetailEntry::rate("Ack", format!("{:.1}/s", ack_rate), ack_rate));
            if redel_rate > 0.0 {
                rates.push(DetailEntry::rate("Redeliver", format!("{:.1}/s", redel_rate), redel_rate));
            }

            // Totals
            if let Some(n) = ms.get("publish").and_then(|v| v.as_u64()) {
                rates.push(DetailEntry::kv("Total published", format_number(n)));
            }
            if let Some(n) = ms.get("deliver").and_then(|v| v.as_u64()) {
                rates.push(DetailEntry::kv("Total delivered", format_number(n)));
            }
            if let Some(n) = ms.get("ack").and_then(|v| v.as_u64()) {
                rates.push(DetailEntry::kv("Total acked", format_number(n)));
            }
        }
        if !rates.is_empty() {
            sections.push(DetailSection { title: "Rates".into(), entries: rates });
        }

        // Memory
        let mut memory = Vec::new();
        if let Some(n) = data.get("memory").and_then(|v| v.as_u64()) {
            memory.push(DetailEntry::kv("Usage", format_bytes(n)));
        }
        if !memory.is_empty() {
            sections.push(DetailSection { title: "Memory".into(), entries: memory });
        }

        // Policy & Arguments
        let mut config = Vec::new();
        if let Some(p) = data.get("policy").and_then(|v| v.as_str()) {
            if !p.is_empty() {
                config.push(DetailEntry::kv("Policy", p));
            }
        }
        if let Some(args) = data.get("arguments").and_then(|v| v.as_object()) {
            for (k, v) in args {
                let val = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                config.push(DetailEntry::kv(k.as_str(), val));
            }
        }
        if !config.is_empty() {
            sections.push(DetailSection { title: "Configuration".into(), entries: config });
        }

        // Bindings — which exchanges feed this queue
        let bindings_path = format!("/api/queues/{}/{}/bindings", encoded_ns, encoded_q);
        if let Ok(bindings_data) = self.get::<Vec<serde_json::Value>>(&bindings_path) {
            let mut bindings = Vec::new();
            for b in &bindings_data {
                let source = b.get("source").and_then(|v| v.as_str()).unwrap_or("");
                // Skip the default exchange self-binding (empty source)
                if source.is_empty() {
                    continue;
                }
                let routing_key = b.get("routing_key").and_then(|v| v.as_str()).unwrap_or("");
                let rk_display = if routing_key.is_empty() { "(all)" } else { routing_key };
                bindings.push(DetailEntry::kv(source, rk_display));
            }
            if !bindings.is_empty() {
                sections.push(DetailSection { title: "Bindings".into(), entries: bindings });
            }
        }

        Ok(sections)
    }

    fn create_exchange(&self, namespace: &str, name: &str, exchange_type: &str, durable: bool) -> Result<(), String> {
        let vhost = urlencoding::encode(namespace);
        let exchange = urlencoding::encode(name);
        let path = format!("/api/exchanges/{}/{}", vhost, exchange);
        let body = serde_json::json!({
            "type": exchange_type,
            "durable": durable,
            "auto_delete": false,
            "internal": false,
            "arguments": {}
        });
        self.client
            .put(format!("{}{}", self.base_url, path))
            .basic_auth(&self.username, Some(&self.password))
            .json(&body)
            .send()
            .map_err(|e| format!("Creating exchange: {}", e))?;
        Ok(())
    }

    fn delete_exchange(&self, namespace: &str, name: &str) -> Result<(), String> {
        let vhost = urlencoding::encode(namespace);
        let exchange = urlencoding::encode(name);
        let path = format!("/api/exchanges/{}/{}", vhost, exchange);
        self.http_delete(&path)
    }

    fn list_exchanges(&self, namespace: &str) -> Result<Vec<ExchangeInfo>, String> {
        let vhost = urlencoding::encode(namespace);
        let url = format!("{}/api/exchanges/{}", self.base_url, vhost);
        let resp: Vec<serde_json::Value> = self.client.get(&url)
            .basic_auth(&self.username, Some(&self.password))
            .timeout(Duration::from_secs(10))
            .send().map_err(|e| format!("HTTP: {}", e))?
            .json().map_err(|e| format!("JSON: {}", e))?;

        Ok(resp.iter().map(|e| {
            ExchangeInfo {
                name: e["name"].as_str().unwrap_or("").to_string(),
                exchange_type: e["type"].as_str().unwrap_or("").to_string(),
                durable: e["durable"].as_bool().unwrap_or(false),
            }
        }).collect())
    }

    fn list_bindings(&self, namespace: &str) -> Result<Vec<BindingInfo>, String> {
        let vhost = urlencoding::encode(namespace);
        let url = format!("{}/api/bindings/{}", self.base_url, vhost);
        let resp: Vec<serde_json::Value> = self.client.get(&url)
            .basic_auth(&self.username, Some(&self.password))
            .timeout(Duration::from_secs(10))
            .send().map_err(|e| format!("HTTP: {}", e))?
            .json().map_err(|e| format!("JSON: {}", e))?;

        Ok(resp.iter().map(|b| {
            BindingInfo {
                source: b["source"].as_str().unwrap_or("").to_string(),
                destination: b["destination"].as_str().unwrap_or("").to_string(),
                routing_key: b["routing_key"].as_str().unwrap_or("").to_string(),
                destination_type: b["destination_type"].as_str().unwrap_or("queue").to_string(),
                properties_key: b["properties_key"].as_str().unwrap_or("").to_string(),
            }
        }).filter(|b| !b.source.is_empty()) // skip default exchange bindings
        .collect())
    }

    fn list_permissions(&self, namespace: &str) -> Result<Vec<PermissionEntry>, String> {
        let vhost = urlencoding::encode(namespace);
        let url = format!("{}/api/permissions/{}", self.base_url, vhost);
        let resp: Vec<serde_json::Value> = self.client.get(&url)
            .basic_auth(&self.username, Some(&self.password))
            .timeout(Duration::from_secs(10))
            .send().map_err(|e| format!("HTTP: {}", e))?
            .json().map_err(|e| format!("JSON: {}", e))?;

        let mut entries = Vec::new();
        for p in &resp {
            let user = p["user"].as_str().unwrap_or("").to_string();
            let configure = p["configure"].as_str().unwrap_or("").to_string();
            let write = p["write"].as_str().unwrap_or("").to_string();
            let read = p["read"].as_str().unwrap_or("").to_string();

            if !configure.is_empty() {
                entries.push(PermissionEntry {
                    user_or_principal: user.clone(),
                    _resource_type: "vhost".to_string(),
                    resource_name: configure.clone(),
                    permission: "configure".to_string(),
                    _operation: "configure".to_string(),
                    host: namespace.to_string(),
                });
            }
            if !write.is_empty() {
                entries.push(PermissionEntry {
                    user_or_principal: user.clone(),
                    _resource_type: "vhost".to_string(),
                    resource_name: write.clone(),
                    permission: "write".to_string(),
                    _operation: "write".to_string(),
                    host: namespace.to_string(),
                });
            }
            if !read.is_empty() {
                entries.push(PermissionEntry {
                    user_or_principal: user.clone(),
                    _resource_type: "vhost".to_string(),
                    resource_name: read.clone(),
                    permission: "read".to_string(),
                    _operation: "read".to_string(),
                    host: namespace.to_string(),
                });
            }
        }

        Ok(entries)
    }

    fn create_binding(&self, namespace: &str, exchange: &str, queue: &str, routing_key: &str) -> Result<(), String> {
        let encoded_ns = urlencoding::encode(namespace);
        let encoded_ex = urlencoding::encode(exchange);
        let encoded_q = urlencoding::encode(queue);
        let path = format!("/api/bindings/{}/e/{}/q/{}", encoded_ns, encoded_ex, encoded_q);
        let body = serde_json::json!({
            "routing_key": routing_key,
            "arguments": {}
        });
        self.post_json(&path, &body)?;
        Ok(())
    }

    fn delete_binding(&self, namespace: &str, exchange: &str, queue: &str, properties_key: &str) -> Result<(), String> {
        let encoded_ns = urlencoding::encode(namespace);
        let encoded_ex = urlencoding::encode(exchange);
        let encoded_q = urlencoding::encode(queue);
        let encoded_pk = urlencoding::encode(properties_key);
        let path = format!("/api/bindings/{}/e/{}/q/{}/{}", encoded_ns, encoded_ex, encoded_q, encoded_pk);
        self.http_delete(&path)
    }

    fn clone_backend(&self) -> Box<dyn Backend> {
        Box::new(Self {
            client: Arc::clone(&self.client),
            base_url: self.base_url.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
        })
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{}", n)
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{} B", bytes)
    }
}
