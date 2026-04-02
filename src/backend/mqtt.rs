use super::{Backend, BrokerInfo, MessageInfo, QueueInfo};
use crate::config::Profile;

pub struct MqttBackend {
    broker: String,
}

impl MqttBackend {
    pub fn new(profile: &Profile) -> Result<Self, String> {
        Ok(Self {
            broker: format!("{}:{}", profile.host, profile.port),
        })
    }
}

impl Backend for MqttBackend {
    fn broker_info(&self) -> Result<BrokerInfo, String> {
        Ok(BrokerInfo {
            name: "MQTT Broker".to_string(),
            cluster: self.broker.clone(),
        })
    }

    fn list_namespaces(&self) -> Result<Vec<String>, String> {
        // MQTT doesn't have namespaces — return a single "default"
        Ok(vec!["default".to_string()])
    }

    fn list_queues(&self, _namespace: &str) -> Result<Vec<QueueInfo>, String> {
        Err("MQTT backend: topic listing not yet implemented. Coming soon!".to_string())
    }

    fn peek_messages(&self, _namespace: &str, _queue: &str, _count: u32) -> Result<Vec<MessageInfo>, String> {
        Err("MQTT backend: message subscription not yet implemented. Coming soon!".to_string())
    }

    fn clone_backend(&self) -> Box<dyn Backend> {
        Box::new(Self {
            broker: self.broker.clone(),
        })
    }
}
