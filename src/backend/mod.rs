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

/// A section of key-value pairs for queue detail display
#[derive(Debug, Clone)]
pub struct DetailSection {
    pub title: String,
    pub entries: Vec<DetailEntry>,
}

/// A single key-value entry in a detail section
#[derive(Debug, Clone)]
pub struct DetailEntry {
    pub key: String,
    pub value: String,
    pub rate_value: Option<f64>, // if set, renders a mini bar chart
}

impl DetailEntry {
    pub fn kv(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self { key: key.into(), value: value.into(), rate_value: None }
    }
    pub fn rate(key: impl Into<String>, value: impl Into<String>, rate: f64) -> Self {
        Self { key: key.into(), value: value.into(), rate_value: Some(rate) }
    }
}

/// Consumer group info for a topic
#[derive(Debug, Clone)]
pub struct ConsumerGroupInfo {
    pub name: String,
    pub state: String,
    pub members: u32,
    pub total_lag: i64,
    pub partitions: Vec<ConsumerGroupPartition>,
}

/// Per-partition consumer group offset and lag
#[derive(Debug, Clone)]
pub struct ConsumerGroupPartition {
    pub partition: i32,
    pub current_offset: i64,
    pub high_watermark: i64,
    pub lag: i64,
}

/// Generic backend trait — implement for each broker type
pub trait Backend: Send {
    fn backend_type(&self) -> &str;
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

    /// Publish a message to a specific exchange (for DLQ re-routing)
    fn publish_to_exchange(
        &self,
        _namespace: &str,
        _exchange: &str,
        _body: &str,
        _routing_key: &str,
        _headers: &[(String, String)],
        _content_type: &str,
    ) -> Result<(), String> {
        Err("Publish to exchange not supported by this backend".into())
    }

    /// List consumer groups for a queue/topic
    fn consumer_groups(&self, _namespace: &str, _queue: &str) -> Result<Vec<ConsumerGroupInfo>, String> {
        Err("Consumer groups not supported by this backend".into())
    }

    /// Get detailed queue/topic information as structured sections
    fn queue_detail(&self, _namespace: &str, _queue: &str) -> Result<Vec<DetailSection>, String> {
        Err("Queue detail not supported by this backend".into())
    }
}
