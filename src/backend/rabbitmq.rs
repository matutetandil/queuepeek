use reqwest::blocking::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

use super::{Backend, BrokerInfo, MessageInfo, QueueInfo};
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
    vhost: String,
    #[serde(default)]
    message_stats: Option<MessageStats>,
}

#[derive(Deserialize)]
struct MessageStats {
    publish_details: Option<RateDetail>,
    deliver_details: Option<RateDetail>,
    ack_details: Option<RateDetail>,
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
            name: format!("RabbitMQ {}", resp.rabbitmq_version),
            cluster: resp.cluster_name,
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

    fn clone_backend(&self) -> Box<dyn Backend> {
        Box::new(Self {
            client: Arc::clone(&self.client),
            base_url: self.base_url.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
        })
    }
}
