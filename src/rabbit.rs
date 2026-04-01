use reqwest::blocking::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

use crate::config::Profile;

#[derive(Debug, Clone)]
pub struct Queue {
    pub name: String,
    pub messages: u64,
    pub consumers: u64,
    pub state: String,
    pub vhost: String,
    pub publish_rate: f64,
    pub deliver_rate: f64,
    pub ack_rate: f64,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub index: usize,
    pub routing_key: String,
    pub exchange: String,
    pub redelivered: bool,
    pub timestamp: Option<i64>,
    pub content_type: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct Overview {
    pub cluster_name: String,
    pub rabbitmq_version: String,
}

// API response structs
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
}

#[derive(serde::Serialize)]
struct PeekRequest {
    count: u32,
    ackmode: String,
    encoding: String,
    truncate: u32,
}

pub struct RabbitClient {
    client: Arc<Client>,
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub default_vhost: String,
}

impl RabbitClient {
    pub fn clone_for_thread(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            base_url: self.base_url.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            default_vhost: self.default_vhost.clone(),
        }
    }
}

impl RabbitClient {
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
                let identity = reqwest::Identity::from_pkcs8_pem(&cert_pem, &key_pem).map_err(|e| format!("Parsing client identity: {}", e))?;
                builder = builder.identity(identity);
            }
        }

        let client = Arc::new(builder.build().map_err(|e| format!("Building HTTP client: {}", e))?);

        Ok(Self {
            client,
            base_url: profile.base_url(),
            username: profile.username.clone(),
            password: profile.password.clone(),
            default_vhost: profile.vhost_or_default().to_string(),
        })
    }

    pub fn default_vhost(&self) -> &str {
        &self.default_vhost
    }

    pub fn get_overview(&self) -> Result<Overview, String> {
        let resp: OverviewResponse = self.get("/api/overview")?;
        Ok(Overview {
            cluster_name: resp.cluster_name,
            rabbitmq_version: resp.rabbitmq_version,
        })
    }

    pub fn list_vhosts(&self) -> Result<Vec<String>, String> {
        let vhosts: Vec<VhostResponse> = self.get("/api/vhosts")?;
        Ok(vhosts.into_iter().map(|v| v.name).collect())
    }

    pub fn list_queues(&self, vhost: &str) -> Result<Vec<Queue>, String> {
        let encoded = urlencoding::encode(vhost);
        let path = format!("/api/queues/{}", encoded);
        let api_queues: Vec<QueueApiResponse> = self.get(&path)?;

        Ok(api_queues.into_iter().map(|aq| {
            let (pub_rate, del_rate, ack_rate) = match aq.message_stats {
                Some(ref ms) => (
                    ms.publish_details.as_ref().map_or(0.0, |d| d.rate),
                    ms.deliver_details.as_ref().map_or(0.0, |d| d.rate),
                    ms.ack_details.as_ref().map_or(0.0, |d| d.rate),
                ),
                None => (0.0, 0.0, 0.0),
            };
            Queue {
                name: aq.name,
                messages: aq.messages,
                consumers: aq.consumers,
                state: aq.state,
                vhost: aq.vhost,
                publish_rate: pub_rate,
                deliver_rate: del_rate,
                ack_rate: ack_rate,
            }
        }).collect())
    }

    pub fn peek_messages(&self, vhost: &str, queue: &str, count: u32) -> Result<Vec<Message>, String> {
        let encoded_vhost = urlencoding::encode(vhost);
        let encoded_queue = urlencoding::encode(queue);
        let path = format!("/api/queues/{}/{}/get", encoded_vhost, encoded_queue);

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
            Message {
                index: i + 1,
                routing_key: pr.routing_key,
                exchange: pr.exchange,
                redelivered: pr.redelivered,
                timestamp: pr.properties.timestamp,
                content_type: pr.properties.content_type.unwrap_or_default(),
                body: pr.payload,
            }
        }).collect())
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
}
