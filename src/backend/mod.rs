pub mod rabbitmq;
pub mod kafka;
pub mod mqtt;

/// Info about the connected broker
#[derive(Debug, Clone)]
pub struct BrokerInfo {
    pub _name: String,
    pub _cluster: String,
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

/// Strategy for resetting consumer group offsets
#[derive(Debug, Clone)]
pub enum OffsetResetStrategy {
    Earliest,
    Latest,
    ToTimestamp(i64),     // unix millis
    ToOffset(i64),        // specific offset (applied to all partitions)
}

/// Exchange info for topology view
#[derive(Debug, Clone)]
pub struct ExchangeInfo {
    pub name: String,
    pub exchange_type: String,
    pub durable: bool,
}

/// Binding info for topology view
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BindingInfo {
    pub source: String,
    pub destination: String,
    pub routing_key: String,
    pub destination_type: String,
    pub properties_key: String,
}

/// Permission/ACL entry for a user or principal
#[derive(Debug, Clone)]
pub struct PermissionEntry {
    pub user_or_principal: String,
    pub _resource_type: String,
    pub resource_name: String,
    pub permission: String,
    pub _operation: String,
    pub host: String,
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

    /// Create an exchange
    fn create_exchange(&self, _namespace: &str, _name: &str, _exchange_type: &str, _durable: bool) -> Result<(), String> {
        Err("Not supported".into())
    }

    /// Delete an exchange
    fn delete_exchange(&self, _namespace: &str, _name: &str) -> Result<(), String> {
        Err("Not supported".into())
    }

    /// List exchanges in namespace (for topology view)
    fn list_exchanges(&self, _namespace: &str) -> Result<Vec<ExchangeInfo>, String> {
        Err("Exchange listing not supported by this backend".into())
    }

    /// List bindings in namespace (for topology view)
    fn list_bindings(&self, _namespace: &str) -> Result<Vec<BindingInfo>, String> {
        Err("Binding listing not supported by this backend".into())
    }

    /// Replay messages from offset range to destination
    fn replay_messages(
        &self,
        _namespace: &str,
        _topic: &str,
        _start_offset: i64,
        _end_offset: i64,
        _dest_topic: &str,
    ) -> Result<u64, String> {
        Err("Message replay not supported by this backend".into())
    }

    /// List permissions/ACLs for the current namespace
    fn list_permissions(&self, _namespace: &str) -> Result<Vec<PermissionEntry>, String> {
        Err("Permission listing not supported by this backend".into())
    }

    /// List retained messages (MQTT-specific, subscribe and collect retain=true)
    fn list_retained_messages(&self, _namespace: &str) -> Result<Vec<MessageInfo>, String> {
        Err("Retained message listing not supported by this backend".into())
    }

    /// Clear a retained message by publishing empty payload with retain=true
    fn clear_retained_message(&self, _namespace: &str, _topic: &str) -> Result<(), String> {
        Err("Clear retained not supported by this backend".into())
    }

    /// Create a binding between an exchange and a queue
    fn create_binding(&self, _namespace: &str, _exchange: &str, _queue: &str, _routing_key: &str) -> Result<(), String> {
        Err("Not supported".into())
    }

    /// Delete a binding between an exchange and a queue
    fn delete_binding(&self, _namespace: &str, _exchange: &str, _queue: &str, _properties_key: &str) -> Result<(), String> {
        Err("Not supported".into())
    }

    /// Reset consumer group offsets for a queue/topic
    fn reset_consumer_group_offsets(
        &self,
        _namespace: &str,
        _queue: &str,
        _group: &str,
        _strategy: OffsetResetStrategy,
    ) -> Result<String, String> {
        Err("Offset reset not supported by this backend".into())
    }
}
