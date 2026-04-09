use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rumqttc::{Client, Event, MqttOptions, Packet, QoS, TlsConfiguration, Transport};

use super::{Backend, BrokerInfo, MessageInfo, QueueInfo};
use crate::config::Profile;

pub struct MqttBackend {
    host: String,
    port: u16,
    username: String,
    password: String,
    tls: bool,
    tls_ca: Option<String>,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    topics: Option<Vec<String>>,
}

impl MqttBackend {
    pub fn new(profile: &Profile) -> Result<Self, String> {
        Ok(Self {
            host: profile.host.clone(),
            port: profile.port,
            username: profile.username.clone(),
            password: profile.password.clone(),
            tls: profile.tls.unwrap_or(false),
            tls_ca: profile.tls_ca.clone(),
            tls_cert: profile.tls_cert.clone(),
            tls_key: profile.tls_key.clone(),
            topics: profile.topics.clone(),
        })
    }

    fn make_options(&self, client_id: &str) -> Result<MqttOptions, String> {
        let mut opts = MqttOptions::new(client_id, &self.host, self.port);
        opts.set_keep_alive(Duration::from_secs(30));

        if !self.username.is_empty() {
            opts.set_credentials(&self.username, &self.password);
        }

        if self.tls {
            let ca = match &self.tls_ca {
                Some(path) => std::fs::read(path)
                    .map_err(|e| format!("Reading CA cert: {}", e))?,
                None => Vec::new(),
            };

            let client_auth = match (&self.tls_cert, &self.tls_key) {
                (Some(cert_path), Some(key_path)) => {
                    let cert = std::fs::read(cert_path)
                        .map_err(|e| format!("Reading client cert: {}", e))?;
                    let key = std::fs::read(key_path)
                        .map_err(|e| format!("Reading client key: {}", e))?;
                    Some((cert, key))
                }
                _ => None,
            };

            let tls_config = TlsConfiguration::Simple {
                ca,
                alpn: None,
                client_auth,
            };
            opts.set_transport(Transport::tls_with_config(tls_config));
        }

        Ok(opts)
    }
}

impl Backend for MqttBackend {
    fn backend_type(&self) -> &str { "mqtt" }
    fn broker_info(&self) -> Result<BrokerInfo, String> {
        let client_id = format!("queuepeek-info-{}", uuid::Uuid::new_v4());
        let opts = self.make_options(&client_id)?;
        let (client, mut connection) = Client::new(opts, 10);

        let mut broker_version = String::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut connected = false;

        for notification in connection.iter() {
            if Instant::now() > deadline { break; }
            match notification {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    connected = true;
                    let _ = client.subscribe("$SYS/broker/version", QoS::AtMostOnce);
                }
                Ok(Event::Incoming(Packet::Publish(msg))) => {
                    if msg.topic == "$SYS/broker/version" {
                        broker_version = String::from_utf8_lossy(&msg.payload).to_string();
                        break;
                    }
                }
                Err(e) => {
                    if !connected {
                        return Err(format!("MQTT connection failed: {}", e));
                    }
                    break;
                }
                _ => {}
            }
        }

        let _ = client.disconnect();

        let name = if broker_version.is_empty() {
            "MQTT Broker".to_string()
        } else {
            format!("MQTT ({})", broker_version)
        };

        Ok(BrokerInfo {
            name,
            cluster: format!("{}:{}", self.host, self.port),
        })
    }

    fn list_namespaces(&self) -> Result<Vec<String>, String> {
        Ok(vec!["default".to_string()])
    }

    fn list_queues(&self, _namespace: &str) -> Result<Vec<QueueInfo>, String> {
        let client_id = format!("queuepeek-discover-{}", uuid::Uuid::new_v4());
        let opts = self.make_options(&client_id)?;
        let (client, mut connection) = Client::new(opts, 256);

        let topics: Arc<Mutex<HashMap<String, u64>>> = Arc::new(Mutex::new(HashMap::new()));

        let subscribe_topics: Vec<String> = match &self.topics {
            Some(t) => t.clone(),
            None => vec!["#".to_string()],
        };

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut connected = false;

        for notification in connection.iter() {
            if Instant::now() > deadline { break; }

            match notification {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    connected = true;
                    for topic in &subscribe_topics {
                        let _ = client.subscribe(topic.as_str(), QoS::AtMostOnce);
                    }
                }
                Ok(Event::Incoming(Packet::Publish(msg))) => {
                    if msg.topic.starts_with("$SYS") { continue; }
                    let mut map = topics.lock().unwrap();
                    *map.entry(msg.topic.clone()).or_insert(0) += 1;
                }
                Err(e) => {
                    if !connected {
                        return Err(format!("MQTT connection failed: {}", e));
                    }
                    break;
                }
                _ => {}
            }
        }

        let _ = client.disconnect();

        let map = topics.lock().unwrap();
        let mut queues: Vec<QueueInfo> = if map.is_empty() {
            if let Some(ref configured) = self.topics {
                configured.iter().map(|t| QueueInfo {
                    name: t.clone(),
                    messages: 0,
                    consumers: 0,
                    state: "subscribed".to_string(),
                    publish_rate: 0.0,
                    deliver_rate: 0.0,
                }).collect()
            } else {
                Vec::new()
            }
        } else {
            map.iter().map(|(name, count)| QueueInfo {
                name: name.clone(),
                messages: *count,
                consumers: 0,
                state: "active".to_string(),
                publish_rate: 0.0,
                deliver_rate: 0.0,
            }).collect()
        };

        queues.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(queues)
    }

    fn peek_messages(&self, _namespace: &str, queue: &str, count: u32) -> Result<Vec<MessageInfo>, String> {
        let client_id = format!("queuepeek-read-{}", uuid::Uuid::new_v4());
        let opts = self.make_options(&client_id)?;
        let (client, mut connection) = Client::new(opts, 256);

        let mut messages = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut connected = false;

        for notification in connection.iter() {
            if Instant::now() > deadline { break; }
            if messages.len() >= count as usize { break; }

            match notification {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    connected = true;
                    let _ = client.subscribe(queue, QoS::AtMostOnce);
                }
                Ok(Event::Incoming(Packet::Publish(msg))) => {
                    let body = String::from_utf8_lossy(&msg.payload).to_string();

                    let mut headers = Vec::new();
                    headers.push(("topic".to_string(), msg.topic.clone()));
                    headers.push(("qos".to_string(), format!("{:?}", msg.qos)));
                    headers.push(("retain".to_string(), msg.retain.to_string()));
                    headers.push(("pkid".to_string(), msg.pkid.to_string()));

                    messages.push(MessageInfo {
                        index: messages.len() + 1,
                        routing_key: msg.topic.clone(),
                        exchange: String::new(),
                        redelivered: msg.dup,
                        timestamp: None,
                        content_type: String::new(),
                        headers,
                        body,
                    });
                }
                Err(e) => {
                    if !connected {
                        return Err(format!("MQTT connection failed: {}", e));
                    }
                    break;
                }
                _ => {}
            }
        }

        let _ = client.disconnect();
        Ok(messages)
    }

    fn publish_message(
        &self,
        _namespace: &str,
        queue: &str,
        body: &str,
        _routing_key: &str,
        _headers: &[(String, String)],
        _content_type: &str,
    ) -> Result<(), String> {
        let client_id = format!("queuepeek-pub-{}", uuid::Uuid::new_v4());
        let opts = self.make_options(&client_id)?;
        let (client, mut connection) = Client::new(opts, 10);

        let topic = queue.to_string();
        let payload = body.as_bytes().to_vec();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut connected = false;
        let mut published = false;

        for notification in connection.iter() {
            if Instant::now() > deadline { break; }
            match notification {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    connected = true;
                    client.publish(&topic, QoS::AtLeastOnce, false, payload.clone())
                        .map_err(|e| format!("Publishing: {}", e))?;
                }
                Ok(Event::Incoming(Packet::PubAck(_))) => {
                    published = true;
                    break;
                }
                Err(e) => {
                    if !connected {
                        return Err(format!("MQTT connection failed: {}", e));
                    }
                    break;
                }
                _ => {}
            }
        }

        let _ = client.disconnect();

        if !published && connected {
            // QoS 0 won't get PubAck, consider it sent
            return Ok(());
        }
        if !connected {
            return Err("Failed to connect to MQTT broker".into());
        }
        Ok(())
    }

    fn clone_backend(&self) -> Box<dyn Backend> {
        Box::new(Self {
            host: self.host.clone(),
            port: self.port,
            username: self.username.clone(),
            password: self.password.clone(),
            tls: self.tls,
            tls_ca: self.tls_ca.clone(),
            tls_cert: self.tls_cert.clone(),
            tls_key: self.tls_key.clone(),
            topics: self.topics.clone(),
        })
    }
}
