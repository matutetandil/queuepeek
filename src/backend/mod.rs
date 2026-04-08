pub mod rabbitmq;
pub mod kafka;
pub mod mqtt;

/// Info about the connected broker
#[derive(Debug, Clone)]
pub struct BrokerInfo {
    pub name: String,
    pub cluster: String,
}

/// A queue/topic with basic stats
#[derive(Debug, Clone)]
pub struct QueueInfo {
    pub name: String,
    pub messages: u64,
    pub consumers: u64,
    pub state: String,
    pub publish_rate: f64,
    pub deliver_rate: f64,
}

/// A single message with full detail
#[derive(Debug, Clone)]
pub struct MessageInfo {
    pub index: usize,
    pub routing_key: String,
    pub exchange: String,
    pub redelivered: bool,
    pub timestamp: Option<i64>,
    pub content_type: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// Generic backend trait — implement for each broker type
pub trait Backend: Send {
    fn broker_info(&self) -> Result<BrokerInfo, String>;
    fn list_namespaces(&self) -> Result<Vec<String>, String>;
    fn list_queues(&self, namespace: &str) -> Result<Vec<QueueInfo>, String>;
    fn peek_messages(&self, namespace: &str, queue: &str, count: u32) -> Result<Vec<MessageInfo>, String>;
    fn clone_backend(&self) -> Box<dyn Backend>;

    /// Publish a message to a queue/topic
    fn publish_message(
        &self,
        _namespace: &str,
        _queue: &str,
        _body: &str,
        _routing_key: &str,
        _headers: &[(String, String)],
        _content_type: &str,
    ) -> Result<(), String> {
        Err("Publish not supported by this backend".into())
    }

    /// Delete a queue/topic entirely
    fn delete_queue(&self, _namespace: &str, _queue: &str) -> Result<(), String> {
        Err("Delete not supported by this backend".into())
    }

    /// Purge all messages from a queue
    fn purge_queue(&self, _namespace: &str, _queue: &str) -> Result<(), String> {
        Err("Purge not supported by this backend".into())
    }

    /// Consume messages destructively (ack without requeue)
    fn consume_messages(&self, _namespace: &str, _queue: &str, _count: u32) -> Result<Vec<MessageInfo>, String> {
        Err("Consume not supported by this backend".into())
    }
}
