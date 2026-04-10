use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ratatui::widgets::ListState;

use crate::config::{Config, Profile};
use crate::backend::{Backend, BrokerInfo, ConsumerGroupInfo, DetailSection, OffsetResetStrategy, QueueInfo, MessageInfo};
use crate::ui::theme::{get_theme, Theme};
use crate::updater::UpdateChecker;

// Background task results sent via channel
pub enum BgResult {
    BrokerInfo(Result<BrokerInfo, String>),
    Namespaces(Result<Vec<String>, String>),
    Queues {
        namespace: String,
        result: Result<Vec<QueueInfo>, String>,
    },
    Messages {
        queue_name: String,
        result: Result<Vec<MessageInfo>, String>,
    },
    Published(Result<(), String>),
    Purged(Result<(), String>),
    Deleted(Result<(), String>),
    OperationProgress { completed: usize, total: usize },
    OperationComplete(Result<String, String>),
    QueueDetail(Result<Vec<DetailSection>, String>),
    ConsumerGroups(Result<Vec<ConsumerGroupInfo>, String>),
    OffsetReset(Result<String, String>),
    ScheduledPublished { id: u64, result: Result<(), String> },
    ReplayComplete(Result<u64, String>),
    Topology(Result<(Vec<crate::backend::ExchangeInfo>, Vec<crate::backend::BindingInfo>), String>),
    BenchmarkProgress { completed: u32, total: u32, latency_ms: u64 },
    BenchmarkComplete { total: u32, errors: u32, elapsed_ms: u64, avg_latency_ms: u64 },
    CompareMessages {
        queue_a: String,
        queue_b: String,
        messages_a: Result<Vec<MessageInfo>, String>,
        messages_b: Result<Vec<MessageInfo>, String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    ProfileSelect,
    QueueList,
    MessageList,
    MessageDetail,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Popup {
    None,
    Help,
    ProfileSwitch,
    NamespacePicker,
    FetchCount,
    ThemePicker,
    BackendTypePicker,
    PublishMessage,
    ConfirmPurge,
    ConfirmDelete,
    QueuePicker(QueueOperation),
    MessageQueuePicker(QueueOperation),
    OperationProgress,
    ConfirmDeleteMessages,
    ExportMessages,
    ImportFile,
    QueueInfo,
    EditMessage,
    ConfirmReroute { exchange: String, routing_key: String, count: usize },
    ConsumerGroups,
    ResetOffsetPicker,
    ResetOffsetInput,
    ConfirmResetOffset,
    ScheduleDelay,
    ScheduledMessages,
    CompareQueuePicker,
    CompareResults,
    MessageDiff,
    SavedFilters,
    SaveFilter,
    TemplatePicker,
    SaveTemplate,
    ReplayConfig,
    TopologyView,
    BenchmarkConfig,
    BenchmarkRunning,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueueOperation {
    Move,
    Copy,
}

pub const FETCH_PRESETS: &[u32] = &[10, 25, 50, 100, 250, 500];

pub const SCHEDULE_PRESETS: &[(u64, &str)] = &[
    (30, "30 seconds"),
    (60, "1 minute"),
    (300, "5 minutes"),
    (600, "10 minutes"),
    (1800, "30 minutes"),
    (3600, "1 hour"),
];

pub struct QueueComparisonResult {
    pub queue_a: String,
    pub queue_b: String,
    pub only_in_a: Vec<MessageInfo>,
    pub only_in_b: Vec<MessageInfo>,
    pub in_both: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonTab {
    Summary,
    OnlyInA,
    OnlyInB,
}

pub struct RateHistory {
    pub publish: VecDeque<f64>,
    pub deliver: VecDeque<f64>,
}

impl RateHistory {
    pub fn new() -> Self {
        Self {
            publish: VecDeque::with_capacity(60),
            deliver: VecDeque::with_capacity(60),
        }
    }

    pub fn push(&mut self, publish_rate: f64, deliver_rate: f64) {
        if self.publish.len() >= 60 { self.publish.pop_front(); }
        if self.deliver.len() >= 60 { self.deliver.pop_front(); }
        self.publish.push_back(publish_rate);
        self.deliver.push_back(deliver_rate);
    }

    pub fn sparkline_str(&self, width: usize) -> String {
        let blocks = [' ', '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
        let data = &self.publish;
        if data.is_empty() { return " ".repeat(width); }

        let max = data.iter().cloned().fold(0.0f64, f64::max).max(0.1);
        let start = if data.len() > width { data.len() - width } else { 0 };
        let mut result = String::new();
        let pad = width.saturating_sub(data.len().min(width));
        for _ in 0..pad { result.push(' '); }
        for &v in data.range(start..) {
            let idx = ((v / max) * 7.0).round() as usize;
            result.push(blocks[idx.min(8)]);
        }
        result
    }
}

pub struct BenchmarkStats {
    pub total: u32,
    pub errors: u32,
    pub elapsed_ms: u64,
    pub avg_latency_ms: u64,
}

pub struct ScheduledMessage {
    pub id: u64,
    pub namespace: String,
    pub queue: String,
    pub routing_key: String,
    pub content_type: String,
    pub body: String,
    pub scheduled_at: Instant,
    pub publish_at: Instant,
    pub delay_secs: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileMode {
    Select,
    Add,
    Edit(String), // editing profile name
    ConfirmDelete,
}

pub struct App {
    pub config: Config,
    pub config_path: Option<String>,
    pub screen: Screen,
    pub should_quit: bool,
    pub theme: &'static Theme,

    // Profile screen
    pub profile_mode: ProfileMode,
    pub profile_list_state: ListState,
    pub profile_form: ProfileForm,

    // Connection
    pub backend: Option<Box<dyn Backend>>,
    pub profile_name: String,
    pub broker_info: Option<BrokerInfo>,
    pub namespaces: Vec<String>,
    pub selected_namespace: String,

    // Queue list screen
    pub queues: Vec<QueueInfo>,
    pub queue_list_state: ListState,
    pub queue_filter: String,
    pub queue_filter_active: bool,
    pub filtered_queue_indices: Vec<usize>,

    // Message list screen
    pub messages: Vec<MessageInfo>,
    pub message_list_state: ListState,
    pub current_queue_name: String,
    pub message_filter: String,
    pub message_filter_active: bool,
    pub message_filter_advanced: bool,
    pub filtered_message_indices: Vec<usize>,

    // Message selection (multi-select)
    pub selected_messages: HashSet<usize>,

    // Message detail screen
    pub detail_message_idx: usize,
    pub detail_scroll: u16,
    pub detail_pretty: bool,
    pub detail_decoded: bool,

    // Message diff
    pub diff_messages: Option<(MessageInfo, MessageInfo)>,
    pub diff_scroll: u16,

    // Shared
    pub fetch_count: u32,
    pub status_message: String,
    pub status_is_error: bool,
    pub loading: bool,
    pub popup: Popup,
    pub popup_list_state: ListState,

    // Background channel
    pub bg_sender: mpsc::Sender<BgResult>,
    pub bg_receiver: mpsc::Receiver<BgResult>,

    // Publish
    pub publish_form: PublishForm,

    // Move/Copy operations
    pub operation_progress: (usize, usize),
    pub operation_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,

    // Queue picker filter
    pub queue_picker_filter: String,
    pub queue_picker_filter_active: bool,

    // Import
    pub import_file_path: String,

    // Message auto-refresh (tail mode)
    pub message_auto_refresh: bool,

    // Queue info popup
    pub queue_detail: Vec<DetailSection>,
    pub queue_info_scroll: u16,
    pub queue_info_name: String,

    // Consumer groups popup
    pub consumer_groups: Vec<ConsumerGroupInfo>,
    pub consumer_groups_scroll: u16,
    pub consumer_groups_selected: Option<usize>,

    // Offset reset
    pub reset_group_name: String,
    pub reset_strategy: Option<OffsetResetStrategy>,
    pub reset_input: String,

    // Saved filter input
    pub save_filter_name: String,
    pub saved_filter_list_state: ListState,

    // Templates
    pub template_list_state: ListState,
    pub save_template_name: String,
    pub template_counter: u64,

    // Replay config
    pub replay_start: String,
    pub replay_end: String,
    pub replay_dest: String,
    pub replay_focused_field: usize,

    // Topology
    pub topology_exchanges: Vec<crate::backend::ExchangeInfo>,
    pub topology_bindings: Vec<crate::backend::BindingInfo>,
    pub topology_scroll: u16,

    // Benchmark
    pub bench_count: String,
    pub bench_concurrency: String,
    pub bench_focused_field: usize,
    pub bench_stats: Option<BenchmarkStats>,
    pub bench_progress: (u32, u32),

    // Rate history for sparklines
    pub rate_history: HashMap<String, RateHistory>,

    // Queue comparison
    pub compare_queue_a: String,
    pub comparison_result: Option<QueueComparisonResult>,
    pub comparison_tab: ComparisonTab,
    pub comparison_scroll: u16,

    // Scheduled messages
    pub scheduled_messages: Vec<ScheduledMessage>,
    pub scheduled_next_id: u64,
    pub scheduled_list_state: ListState,

    // Auto-update
    pub update_checker: UpdateChecker,
}

pub const BACKEND_TYPES: &[&str] = &["rabbitmq", "kafka", "mqtt"];

#[derive(Debug, Clone, Default)]
pub struct PublishForm {
    pub routing_key: String,
    pub content_type: String,
    pub body: String,
    pub focused_field: usize, // 0=routing_key, 1=content_type, 2=body
    pub error: String,
}

impl PublishForm {
    pub fn new_for_queue(queue_name: &str) -> Self {
        Self {
            routing_key: queue_name.to_string(),
            content_type: "application/json".to_string(),
            body: String::new(),
            focused_field: 2, // Focus body by default
            error: String::new(),
        }
    }

    pub fn field_count() -> usize { 3 }

    pub fn field_label(idx: usize) -> &'static str {
        match idx {
            0 => "Routing Key",
            1 => "Content Type",
            2 => "Body",
            _ => "",
        }
    }

    pub fn push_char(&mut self, c: char) {
        match self.focused_field {
            0 => self.routing_key.push(c),
            1 => self.content_type.push(c),
            2 => self.body.push(c),
            _ => {}
        }
    }

    pub fn pop_char(&mut self) {
        match self.focused_field {
            0 => { self.routing_key.pop(); }
            1 => { self.content_type.pop(); }
            2 => { self.body.pop(); }
            _ => {}
        }
    }

    pub fn newline(&mut self) {
        if self.focused_field == 2 {
            self.body.push('\n');
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProfileForm {
    pub profile_type: String,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub vhost: String,
    pub tls: bool,
    pub focused_field: usize,
    pub error: String,
}

impl ProfileForm {
    pub fn field_count() -> usize {
        8 // type, name, host, port, username, password, vhost, tls
    }

    pub fn field_label(idx: usize) -> &'static str {
        match idx {
            0 => "Type",
            1 => "Name",
            2 => "Host",
            3 => "Port",
            4 => "Username",
            5 => "Password",
            6 => "Vhost",
            7 => "TLS",
            _ => "",
        }
    }

    pub fn field_value(&self, idx: usize) -> String {
        match idx {
            0 => self.profile_type.clone(),
            1 => self.name.clone(),
            2 => self.host.clone(),
            3 => self.port.clone(),
            4 => self.username.clone(),
            5 => self.password.clone(),
            6 => self.vhost.clone(),
            7 => if self.tls { "yes".into() } else { "no".into() },
            _ => String::new(),
        }
    }

    pub fn set_field(&mut self, idx: usize, val: String) {
        match idx {
            0 => self.profile_type = val,
            1 => self.name = val,
            2 => self.host = val,
            3 => self.port = val,
            4 => self.username = val,
            5 => self.password = val,
            6 => self.vhost = val,
            7 => self.tls = val == "yes",
            _ => {}
        }
    }

    fn is_cloud_host(host: &str) -> bool {
        let h = host.to_lowercase();
        h.contains("cloudamqp.com") || h.contains("amazonaws.com")
            || h.contains("azure.com") || h.contains("rabbitmq.cloud")
    }

    fn auto_detect_cloud(&mut self) {
        if self.profile_type != "rabbitmq" { return; }
        let default = Self::default_port("rabbitmq");
        if Self::is_cloud_host(&self.host) {
            if self.port == default || self.port.is_empty() {
                self.port = "443".to_string();
            }
            self.tls = true;
        } else if self.port == "443" {
            self.port = default.to_string();
            self.tls = false;
        }
    }

    pub fn default_port(backend_type: &str) -> &'static str {
        match backend_type {
            "kafka" => "9092",
            "mqtt" => "1883",
            _ => "15672",
        }
    }

    pub fn set_backend_type(&mut self, new_type: &str) {
        let old_default = Self::default_port(&self.profile_type);
        self.profile_type = new_type.to_string();
        if self.port == old_default || self.port.is_empty() {
            self.port = Self::default_port(&self.profile_type).to_string();
        }
        self.auto_detect_cloud();
    }

    pub fn push_char(&mut self, c: char) {
        // Type field is read-only — use BackendTypePicker popup via Enter
        if self.focused_field == 0 { return; }
        if self.focused_field == 7 {
            self.tls = !self.tls;
            return;
        }
        match self.focused_field {
            1 => self.name.push(c),
            2 => {
                self.host.push(c);
                self.auto_detect_cloud();
            }
            3 => self.port.push(c),
            4 => self.username.push(c),
            5 => self.password.push(c),
            6 => self.vhost.push(c),
            _ => {}
        }
    }

    pub fn pop_char(&mut self) {
        if self.focused_field == 0 || self.focused_field == 7 { return; }
        match self.focused_field {
            1 => { self.name.pop(); }
            2 => { self.host.pop(); self.auto_detect_cloud(); }
            3 => { self.port.pop(); }
            4 => { self.username.pop(); }
            5 => { self.password.pop(); }
            6 => { self.vhost.pop(); }
            _ => {}
        }
    }

    pub fn from_profile(name: &str, p: &Profile) -> Self {
        Self {
            profile_type: p.profile_type.clone(),
            name: name.to_string(),
            host: p.host.clone(),
            port: p.port.to_string(),
            username: p.username.clone(),
            password: p.password.clone(),
            vhost: p.vhost.clone().unwrap_or_default(),
            tls: p.tls.unwrap_or(false),
            focused_field: 0,
            error: String::new(),
        }
    }

    pub fn to_profile(&self) -> Result<Profile, String> {
        if self.name.is_empty() {
            return Err("Name is required".into());
        }
        let port: u16 = self.port.parse().map_err(|_| "Invalid port number")?;
        let host = if self.host.is_empty() { "localhost".into() } else { self.host.clone() };
        let username = if self.username.is_empty() { "guest".into() } else { self.username.clone() };
        let password = if self.password.is_empty() { "guest".into() } else { self.password.clone() };
        let vhost = if self.vhost.is_empty() { None } else { Some(self.vhost.clone()) };
        let profile_type = if self.profile_type.is_empty() { "rabbitmq".into() } else { self.profile_type.clone() };

        Ok(Profile {
            profile_type,
            host,
            port,
            username,
            password,
            vhost,
            tls: Some(self.tls),
            tls_cert: None,
            tls_key: None,
            tls_ca: None,
            topics: None,
        })
    }

    pub fn clear(&mut self) {
        let default_type = "rabbitmq";
        *self = Self {
            profile_type: default_type.into(),
            port: Self::default_port(default_type).into(),
            ..Default::default()
        };
    }
}

impl App {
    pub fn new(config: Config, config_path: Option<String>) -> Self {
        let theme_name = config.theme.as_deref().unwrap_or("slack");
        let theme = get_theme(theme_name);

        let (tx, rx) = mpsc::channel();

        let mut profile_list_state = ListState::default();
        if !config.profiles.is_empty() {
            profile_list_state.select(Some(0));
        }

        Self {
            config,
            config_path,
            screen: Screen::ProfileSelect,
            should_quit: false,
            theme,
            profile_mode: ProfileMode::Select,
            profile_list_state,
            profile_form: ProfileForm { profile_type: "rabbitmq".into(), port: "15672".into(), ..Default::default() },
            backend: None,
            profile_name: String::new(),
            broker_info: None,
            namespaces: Vec::new(),
            selected_namespace: String::new(),
            queues: Vec::new(),
            queue_list_state: ListState::default(),
            queue_filter: String::new(),
            queue_filter_active: false,
            filtered_queue_indices: Vec::new(),
            messages: Vec::new(),
            message_list_state: ListState::default(),
            current_queue_name: String::new(),
            message_filter: String::new(),
            message_filter_active: false,
            message_filter_advanced: false,
            filtered_message_indices: Vec::new(),
            selected_messages: HashSet::new(),
            detail_message_idx: 0,
            detail_scroll: 0,
            detail_pretty: true,
            detail_decoded: false,
            diff_messages: None,
            diff_scroll: 0,
            fetch_count: 50,
            status_message: String::new(),
            status_is_error: false,
            loading: false,
            popup: Popup::None,
            popup_list_state: ListState::default(),
            bg_sender: tx,
            bg_receiver: rx,
            publish_form: PublishForm::default(),
            operation_progress: (0, 0),
            operation_cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            queue_picker_filter: String::new(),
            queue_picker_filter_active: false,
            import_file_path: String::new(),
            message_auto_refresh: false,
            queue_detail: Vec::new(),
            queue_info_scroll: 0,
            queue_info_name: String::new(),
            consumer_groups: Vec::new(),
            consumer_groups_scroll: 0,
            consumer_groups_selected: None,
            reset_group_name: String::new(),
            reset_strategy: None,
            reset_input: String::new(),
            replay_start: String::new(),
            replay_end: String::new(),
            replay_dest: String::new(),
            replay_focused_field: 0,
            topology_exchanges: Vec::new(),
            topology_bindings: Vec::new(),
            topology_scroll: 0,
            bench_count: "1000".to_string(),
            bench_concurrency: "1".to_string(),
            bench_focused_field: 0,
            bench_stats: None,
            bench_progress: (0, 0),
            save_filter_name: String::new(),
            saved_filter_list_state: ListState::default(),
            template_list_state: ListState::default(),
            save_template_name: String::new(),
            template_counter: 0,
            rate_history: HashMap::new(),
            compare_queue_a: String::new(),
            comparison_result: None,
            comparison_tab: ComparisonTab::Summary,
            comparison_scroll: 0,
            scheduled_messages: Vec::new(),
            scheduled_next_id: 1,
            scheduled_list_state: ListState::default(),
            update_checker: UpdateChecker::new(),
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status_message = msg.into();
        self.status_is_error = is_error;
    }

    pub fn connect_profile(&mut self, name: &str) {
        let profile = match self.config.profiles.get(name) {
            Some(p) => p.clone(),
            None => {
                self.set_status(format!("Profile '{}' not found", name), true);
                return;
            }
        };

        let backend_result: Result<Box<dyn crate::backend::Backend>, String> = match profile.profile_type.as_str() {
            "kafka" => crate::backend::kafka::KafkaBackend::new(&profile).map(|b| Box::new(b) as Box<dyn crate::backend::Backend>),
            "mqtt" => crate::backend::mqtt::MqttBackend::new(&profile).map(|b| Box::new(b) as Box<dyn crate::backend::Backend>),
            _ => crate::backend::rabbitmq::RabbitMqBackend::new(&profile).map(|b| Box::new(b) as Box<dyn crate::backend::Backend>),
        };

        match backend_result {
            Ok(backend) => {
                self.backend = Some(backend);
                self.profile_name = name.to_string();
                self.loading = true;
                self.set_status("Connecting...", false);

                self.load_broker_info();
                self.load_namespaces();
            }
            Err(e) => {
                self.set_status(format!("Connection error: {}", e), true);
            }
        }
    }

    pub fn load_broker_info(&self) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = backend.broker_info();
                let _ = tx.send(BgResult::BrokerInfo(result));
            });
        }
    }

    pub fn load_namespaces(&self) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = backend.list_namespaces();
                let _ = tx.send(BgResult::Namespaces(result));
            });
        }
    }

    pub fn load_queues(&self) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = backend.list_queues(&namespace);
                let _ = tx.send(BgResult::Queues { namespace, result });
            });
        }
    }

    pub fn load_messages(&self) {
        if let Some(ref backend) = self.backend {
            let queue_name = self.current_queue_name.clone();
            if queue_name.is_empty() {
                return;
            }
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let count = self.fetch_count;
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = backend.peek_messages(&namespace, &queue_name, count);
                let _ = tx.send(BgResult::Messages { queue_name, result });
            });
        }
    }

    pub fn get_target_messages(&self) -> Vec<MessageInfo> {
        if self.selected_messages.is_empty() {
            // If nothing selected, use the currently highlighted message
            if let Some(selected) = self.message_list_state.selected() {
                if let Some(&idx) = self.filtered_message_indices.get(selected) {
                    if let Some(msg) = self.messages.get(idx) {
                        return vec![msg.clone()];
                    }
                }
            }
            Vec::new()
        } else {
            self.selected_messages.iter()
                .filter_map(|&idx| self.messages.get(idx).cloned())
                .collect()
        }
    }

    pub fn selection_count(&self) -> usize {
        if self.selected_messages.is_empty() {
            if self.message_list_state.selected().is_some() { 1 } else { 0 }
        } else {
            self.selected_messages.len()
        }
    }

    pub fn toggle_message_selection(&mut self) {
        if let Some(selected) = self.message_list_state.selected() {
            if let Some(&idx) = self.filtered_message_indices.get(selected) {
                if self.selected_messages.contains(&idx) {
                    self.selected_messages.remove(&idx);
                } else {
                    self.selected_messages.insert(idx);
                }
            }
        }
    }

    pub fn select_all_messages(&mut self) {
        if self.selected_messages.len() == self.filtered_message_indices.len() {
            self.selected_messages.clear();
        } else {
            self.selected_messages = self.filtered_message_indices.iter().copied().collect();
        }
    }

    pub fn do_copy_selected_to(&mut self, dest: &str) {
        if let Some(ref backend) = self.backend {
            let messages = self.get_target_messages();
            if messages.is_empty() { return; }

            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let dest = dest.to_string();
            let tx = self.bg_sender.clone();
            let cancel = self.operation_cancel.clone();
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            self.operation_progress = (0, 0);
            self.popup = Popup::OperationProgress;

            std::thread::spawn(move || {
                let total = messages.len();
                let _ = tx.send(BgResult::OperationProgress { completed: 0, total });

                for (i, msg) in messages.iter().enumerate() {
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = tx.send(BgResult::OperationComplete(
                            Ok(format!("Cancelled after {}/{} messages", i, total))
                        ));
                        return;
                    }
                    let headers: Vec<(String, String)> = msg.headers.clone();
                    if let Err(e) = backend.publish_message(
                        &namespace, &dest, &msg.body, &msg.routing_key, &headers, &msg.content_type,
                    ) {
                        let _ = tx.send(BgResult::OperationComplete(
                            Err(format!("Failed at message {}/{}: {}", i + 1, total, e))
                        ));
                        return;
                    }
                    let _ = tx.send(BgResult::OperationProgress { completed: i + 1, total });
                }
                let _ = tx.send(BgResult::OperationComplete(
                    Ok(format!("Copied {} messages to {}", total, dest))
                ));
            });
        }
    }

    pub fn do_delete_selected(&mut self) {
        if let Some(ref backend) = self.backend {
            let selected_indices: HashSet<usize> = if self.selected_messages.is_empty() {
                if let Some(sel) = self.message_list_state.selected() {
                    if let Some(&idx) = self.filtered_message_indices.get(sel) {
                        let mut s = HashSet::new();
                        s.insert(idx);
                        s
                    } else { return; }
                } else { return; }
            } else {
                self.selected_messages.clone()
            };

            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let queue = self.current_queue_name.clone();
            let tx = self.bg_sender.clone();
            let cancel = self.operation_cancel.clone();
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            self.operation_progress = (0, 0);
            self.popup = Popup::OperationProgress;

            std::thread::spawn(move || {
                use std::io::{BufRead, Write};

                // Step 1: create temp backup file
                let backup_path = std::env::temp_dir().join(format!("queuepeek-delete-backup-{}.jsonl", chrono_timestamp()));
                let backup_file = match std::fs::File::create(&backup_path) {
                    Ok(f) => f,
                    Err(e) => {
                        let _ = tx.send(BgResult::OperationComplete(Err(format!("Creating backup: {}", e))));
                        return;
                    }
                };
                let mut writer = std::io::BufWriter::new(backup_file);

                // Step 2: consume all messages, streaming to file one at a time
                let _ = tx.send(BgResult::OperationProgress { completed: 0, total: 0 });
                let batch_size = 100u32;
                let mut total_consumed = 0usize;

                loop {
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = tx.send(BgResult::OperationComplete(
                            Err(format!("Cancelled — backup at {}", backup_path.display()))
                        ));
                        return;
                    }

                    let batch = match backend.consume_messages(&namespace, &queue, batch_size) {
                        Ok(msgs) => msgs,
                        Err(e) => {
                            let _ = tx.send(BgResult::OperationComplete(
                                Err(format!("Consume failed after {}: {} — backup at {}", total_consumed, e, backup_path.display()))
                            ));
                            return;
                        }
                    };

                    if batch.is_empty() { break; }

                    for msg in &batch {
                        let json = message_to_json(msg);
                        if let Err(e) = writeln!(writer, "{}", json) {
                            let _ = tx.send(BgResult::OperationComplete(
                                Err(format!("Writing backup: {}", e))
                            ));
                            return;
                        }
                    }

                    total_consumed += batch.len();
                    let _ = tx.send(BgResult::OperationProgress { completed: total_consumed, total: 0 });

                    if (batch.len() as u32) < batch_size { break; }
                }

                drop(writer);

                // Step 3: read backup and re-publish messages NOT in selected_indices
                let file = match std::fs::File::open(&backup_path) {
                    Ok(f) => f,
                    Err(e) => {
                        let _ = tx.send(BgResult::OperationComplete(
                            Err(format!("Reading backup: {} — file at {}", e, backup_path.display()))
                        ));
                        return;
                    }
                };
                let reader = std::io::BufReader::new(file);
                let mut republished = 0usize;
                let mut deleted = 0usize;

                for (i, line) in reader.lines().enumerate() {
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = tx.send(BgResult::OperationComplete(
                            Err(format!("Cancelled during re-publish — backup at {}", backup_path.display()))
                        ));
                        return;
                    }

                    let line = match line {
                        Ok(l) => l,
                        Err(e) => {
                            let _ = tx.send(BgResult::OperationComplete(
                                Err(format!("Reading line {}: {} — backup at {}", i, e, backup_path.display()))
                            ));
                            return;
                        }
                    };

                    if selected_indices.contains(&i) {
                        deleted += 1;
                        continue;
                    }

                    // Parse and re-publish
                    let msg: serde_json::Value = match serde_json::from_str(&line) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    let body = msg["body"].as_str().unwrap_or("");
                    let routing_key = msg["routing_key"].as_str().unwrap_or("");
                    let content_type = msg["content_type"].as_str().unwrap_or("");
                    let headers: Vec<(String, String)> = msg["headers"].as_object()
                        .map(|h| h.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                        .unwrap_or_default();

                    if let Err(e) = backend.publish_message(
                        &namespace, &queue, body, routing_key, &headers, content_type,
                    ) {
                        let _ = tx.send(BgResult::OperationComplete(
                            Err(format!("Re-publish failed at msg {}: {} — backup at {}", i + 1, e, backup_path.display()))
                        ));
                        return;
                    }

                    republished += 1;
                    let _ = tx.send(BgResult::OperationProgress { completed: republished, total: total_consumed - deleted });
                }

                // Clean up backup on success
                let _ = std::fs::remove_file(&backup_path);

                let _ = tx.send(BgResult::OperationComplete(
                    Ok(format!("Deleted {} messages, kept {}", deleted, republished))
                ));
            });
        }
    }

    /// Export selected messages (from memory) to a JSON file
    pub fn export_messages_to_json(&self) -> Result<String, String> {
        let messages = self.get_target_messages();
        if messages.is_empty() {
            return Err("No messages to export".into());
        }

        let filename = format!("queuepeek-export-{}.json", chrono_timestamp());
        let path = std::env::current_dir()
            .unwrap_or_default()
            .join(&filename);

        let file = std::fs::File::create(&path)
            .map_err(|e| format!("Creating file: {}", e))?;
        let mut writer = std::io::BufWriter::new(file);

        use std::io::Write;
        writeln!(writer, "[").map_err(|e| format!("Writing: {}", e))?;
        for (i, m) in messages.iter().enumerate() {
            let json = message_to_json(m);
            let comma = if i + 1 < messages.len() { "," } else { "" };
            writeln!(writer, "  {}{}", json, comma).map_err(|e| format!("Writing: {}", e))?;
        }
        writeln!(writer, "]").map_err(|e| format!("Writing: {}", e))?;

        Ok(format!("Exported {} messages to {}", messages.len(), path.display()))
    }

    /// Dump entire queue to JSONL file (streaming, low memory)
    /// Strategy varies by backend:
    /// - RabbitMQ: consume all → dump → re-publish (temporarily removes messages)
    /// - Kafka: dedicated consumer from low watermark, non-destructive
    /// - MQTT: single peek batch (no history)
    pub fn do_dump_queue(&mut self) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let queue = self.current_queue_name.clone();
            let tx = self.bg_sender.clone();
            let cancel = self.operation_cancel.clone();
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            self.operation_progress = (0, 0);
            self.popup = Popup::OperationProgress;

            let backend_type = backend.backend_type().to_string();

            std::thread::spawn(move || {
                match backend_type.as_str() {
                    "rabbitmq" => dump_rabbitmq(backend, &namespace, &queue, tx, cancel),
                    "kafka" => dump_kafka(backend, &namespace, &queue, tx, cancel),
                    _ => dump_simple_peek(backend, &namespace, &queue, tx, cancel),
                }
            });
        }
    }

    /// Import messages from a JSONL or JSON array file into the current queue
    pub fn do_import_jsonl(&mut self) {
        if let Some(ref backend) = self.backend {
            let path_str = self.import_file_path.trim().to_string();
            if path_str.is_empty() { return; }

            let path = std::path::PathBuf::from(&path_str);
            if !path.exists() {
                self.set_status(format!("File not found: {}", path_str), true);
                return;
            }

            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let queue = self.current_queue_name.clone();
            let tx = self.bg_sender.clone();
            let cancel = self.operation_cancel.clone();
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            self.operation_progress = (0, 0);
            self.popup = Popup::OperationProgress;

            std::thread::spawn(move || {
                use std::io::{BufRead, Read};

                // Read first byte to detect format
                let mut file = match std::fs::File::open(&path) {
                    Ok(f) => f,
                    Err(e) => {
                        let _ = tx.send(BgResult::OperationComplete(Err(format!("Opening file: {}", e))));
                        return;
                    }
                };

                let mut first_bytes = [0u8; 64];
                let n = file.read(&mut first_bytes).unwrap_or(0);
                let first_content = String::from_utf8_lossy(&first_bytes[..n]);
                let is_json_array = first_content.trim_start().starts_with('[');
                drop(file);

                let messages: Vec<serde_json::Value> = if is_json_array {
                    // JSON array format (from export)
                    let content = match std::fs::read_to_string(&path) {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = tx.send(BgResult::OperationComplete(Err(format!("Reading file: {}", e))));
                            return;
                        }
                    };
                    match serde_json::from_str(&content) {
                        Ok(v) => v,
                        Err(e) => {
                            let _ = tx.send(BgResult::OperationComplete(Err(format!("Parsing JSON array: {}", e))));
                            return;
                        }
                    }
                } else {
                    // JSONL format — read line by line
                    let file = match std::fs::File::open(&path) {
                        Ok(f) => f,
                        Err(e) => {
                            let _ = tx.send(BgResult::OperationComplete(Err(format!("Opening file: {}", e))));
                            return;
                        }
                    };
                    let reader = std::io::BufReader::new(file);
                    let mut msgs = Vec::new();
                    for line in reader.lines() {
                        let line = match line {
                            Ok(l) if !l.trim().is_empty() => l,
                            _ => continue,
                        };
                        match serde_json::from_str(&line) {
                            Ok(v) => msgs.push(v),
                            Err(_) => continue,
                        }
                    }
                    msgs
                };

                if messages.is_empty() {
                    let _ = tx.send(BgResult::OperationComplete(Err("No messages found in file".into())));
                    return;
                }

                let total = messages.len();
                let _ = tx.send(BgResult::OperationProgress { completed: 0, total });

                for (i, msg) in messages.iter().enumerate() {
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = tx.send(BgResult::OperationComplete(
                            Ok(format!("Import cancelled after {}/{} messages", i, total))
                        ));
                        return;
                    }

                    let body = msg["body"].as_str().unwrap_or("");
                    let routing_key = msg["routing_key"].as_str().unwrap_or("");
                    let content_type = msg["content_type"].as_str().unwrap_or("");
                    let headers: Vec<(String, String)> = msg["headers"].as_object()
                        .map(|h| h.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                        .unwrap_or_default();

                    if let Err(e) = backend.publish_message(
                        &namespace, &queue, body, routing_key, &headers, content_type,
                    ) {
                        let _ = tx.send(BgResult::OperationComplete(
                            Err(format!("Import failed at message {}/{}: {}", i + 1, total, e))
                        ));
                        return;
                    }

                    let _ = tx.send(BgResult::OperationProgress { completed: i + 1, total });
                }

                let _ = tx.send(BgResult::OperationComplete(
                    Ok(format!("Imported {} messages from {}", total, path.display()))
                ));
            });
        }
    }

    /// Parse x-death header to extract original exchange and routing key
    pub fn parse_dlq_info(&self) -> Option<(String, String)> {
        let messages = self.get_target_messages();
        if messages.is_empty() { return None; }

        // Check first message for x-death header
        for (key, value) in &messages[0].headers {
            if key == "x-death" {
                return parse_x_death_value(value);
            }
        }
        None
    }

    /// Re-route selected messages to their original exchange/routing key
    pub fn do_reroute_messages(&mut self, exchange: &str, routing_key: &str) {
        if let Some(ref backend) = self.backend {
            let messages = self.get_target_messages();
            if messages.is_empty() { return; }

            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let exchange = exchange.to_string();
            let routing_key = routing_key.to_string();
            let tx = self.bg_sender.clone();
            let cancel = self.operation_cancel.clone();
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            self.operation_progress = (0, 0);
            self.popup = Popup::OperationProgress;

            std::thread::spawn(move || {
                let total = messages.len();
                let _ = tx.send(BgResult::OperationProgress { completed: 0, total });

                for (i, msg) in messages.iter().enumerate() {
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = tx.send(BgResult::OperationComplete(
                            Ok(format!("Re-route cancelled after {}/{} messages", i, total))
                        ));
                        return;
                    }

                    // Remove x-death header from the re-routed message
                    let headers: Vec<(String, String)> = msg.headers.iter()
                        .filter(|(k, _)| k != "x-death" && k != "x-first-death-exchange" && k != "x-first-death-queue" && k != "x-first-death-reason")
                        .cloned()
                        .collect();

                    let result = backend.publish_to_exchange(
                        &namespace, &exchange, &msg.body, &routing_key, &headers, &msg.content_type,
                    );

                    if let Err(e) = result {
                        let _ = tx.send(BgResult::OperationComplete(
                            Err(format!("Re-route failed at message {}/{}: {}", i + 1, total, e))
                        ));
                        return;
                    }

                    let _ = tx.send(BgResult::OperationProgress { completed: i + 1, total });
                }

                let _ = tx.send(BgResult::OperationComplete(
                    Ok(format!("Re-routed {} messages to exchange '{}' with key '{}'", total, exchange, routing_key))
                ));
            });
        }
    }

    pub fn re_publish_selected(&self) {
        if let Some(ref backend) = self.backend {
            let messages = self.get_target_messages();
            if messages.is_empty() { return; }

            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let queue = self.current_queue_name.clone();
            let tx = self.bg_sender.clone();

            std::thread::spawn(move || {
                let mut ok = 0;
                for msg in &messages {
                    let headers: Vec<(String, String)> = msg.headers.clone();
                    if backend.publish_message(
                        &namespace, &queue, &msg.body, &msg.routing_key, &headers, &msg.content_type,
                    ).is_ok() {
                        ok += 1;
                    }
                }
                let _ = tx.send(BgResult::Published(
                    Ok(())
                ));
            });
        }
    }

    pub fn do_publish(&self) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let queue = self.current_queue_name.clone();
            let routing_key = self.publish_form.routing_key.clone();
            let body = self.publish_form.body.clone();
            let content_type = self.publish_form.content_type.clone();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = backend.publish_message(&namespace, &queue, &body, &routing_key, &[], &content_type);
                let _ = tx.send(BgResult::Published(result));
            });
        }
    }

    pub fn do_purge(&self, queue: &str) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let queue = queue.to_string();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = backend.purge_queue(&namespace, &queue);
                let _ = tx.send(BgResult::Purged(result));
            });
        }
    }

    pub fn do_delete(&self, queue: &str) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let queue = queue.to_string();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = backend.delete_queue(&namespace, &queue);
                let _ = tx.send(BgResult::Deleted(result));
            });
        }
    }

    pub fn do_replay(&self) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let topic = self.current_queue_name.clone();
            let start: i64 = self.replay_start.parse().unwrap_or(0);
            let end: i64 = self.replay_end.parse().unwrap_or(0);
            let dest = self.replay_dest.clone();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = backend.replay_messages(&namespace, &topic, start, end, &dest);
                let _ = tx.send(BgResult::ReplayComplete(result));
            });
        }
    }

    pub fn load_topology(&self) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let exchanges = backend.list_exchanges(&namespace);
                let bindings = backend.list_bindings(&namespace);
                let result = match (exchanges, bindings) {
                    (Ok(e), Ok(b)) => Ok((e, b)),
                    (Err(e), _) => Err(e),
                    (_, Err(e)) => Err(e),
                };
                let _ = tx.send(BgResult::Topology(result));
            });
        }
    }

    pub fn do_benchmark(&mut self) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let queue = self.selected_queue().map(|q| q.name.clone()).unwrap_or_default();
            let count: u32 = self.bench_count.parse().unwrap_or(1000);
            let concurrency: u32 = self.bench_concurrency.parse().unwrap_or(1).max(1);
            let tx = self.bg_sender.clone();
            let cancel = self.operation_cancel.clone();
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            self.bench_progress = (0, count);
            self.bench_stats = None;
            self.popup = Popup::BenchmarkRunning;

            // Use the publish form body as message template, or a default
            let body = if self.publish_form.body.is_empty() {
                format!("{{\"benchmark\": true, \"timestamp\": {}}}", "{{timestamp}}")
            } else {
                self.publish_form.body.clone()
            };
            let routing_key = if self.publish_form.routing_key.is_empty() {
                queue.clone()
            } else {
                self.publish_form.routing_key.clone()
            };
            let content_type = if self.publish_form.content_type.is_empty() {
                "application/json".to_string()
            } else {
                self.publish_form.content_type.clone()
            };

            std::thread::spawn(move || {
                let start = Instant::now();
                let per_thread = count / concurrency;
                let mut total_completed = 0u32;
                let mut total_errors = 0u32;
                let mut total_latency_ms = 0u64;

                // Simple sequential approach (concurrency handled by fast publishing)
                for i in 0..count {
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                    let msg_start = Instant::now();
                    let result = backend.publish_message(
                        &namespace, &queue, &body, &routing_key, &[], &content_type,
                    );
                    let latency = msg_start.elapsed().as_millis() as u64;
                    total_latency_ms += latency;

                    match result {
                        Ok(()) => total_completed += 1,
                        Err(_) => total_errors += 1,
                    }

                    if i % 10 == 0 {
                        let _ = tx.send(BgResult::BenchmarkProgress {
                            completed: total_completed + total_errors,
                            total: count,
                            latency_ms: latency,
                        });
                    }
                }

                let elapsed = start.elapsed().as_millis() as u64;
                let avg_latency = if total_completed > 0 {
                    total_latency_ms / total_completed as u64
                } else { 0 };

                let _ = tx.send(BgResult::BenchmarkComplete {
                    total: total_completed,
                    errors: total_errors,
                    elapsed_ms: elapsed,
                    avg_latency_ms: avg_latency,
                });
            });
        }
    }

    pub fn load_comparison(&self, queue_a: &str, queue_b: &str) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let qa = queue_a.to_string();
            let qb = queue_b.to_string();
            let count = self.fetch_count;
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let messages_a = backend.peek_messages(&namespace, &qa, count);
                let messages_b = backend.peek_messages(&namespace, &qb, count);
                let _ = tx.send(BgResult::CompareMessages {
                    queue_a: qa,
                    queue_b: qb,
                    messages_a,
                    messages_b,
                });
            });
        }
    }

    pub fn schedule_message(&mut self, delay_secs: u64) {
        let now = Instant::now();
        let msg = ScheduledMessage {
            id: self.scheduled_next_id,
            namespace: self.selected_namespace.clone(),
            queue: if self.current_queue_name.is_empty() {
                self.publish_form.routing_key.clone()
            } else {
                self.current_queue_name.clone()
            },
            routing_key: self.publish_form.routing_key.clone(),
            content_type: self.publish_form.content_type.clone(),
            body: self.publish_form.body.clone(),
            scheduled_at: now,
            publish_at: now + Duration::from_secs(delay_secs),
            delay_secs,
        };
        self.scheduled_next_id += 1;
        self.scheduled_messages.push(msg);
    }

    pub fn check_scheduled_messages(&mut self) {
        let now = Instant::now();
        let mut to_publish = Vec::new();

        self.scheduled_messages.retain(|msg| {
            if now >= msg.publish_at {
                to_publish.push((msg.id, msg.namespace.clone(), msg.queue.clone(),
                    msg.routing_key.clone(), msg.content_type.clone(), msg.body.clone()));
                false
            } else {
                true
            }
        });

        for (id, namespace, queue, routing_key, content_type, body) in to_publish {
            if let Some(ref backend) = self.backend {
                let backend = backend.clone_backend();
                let tx = self.bg_sender.clone();
                std::thread::spawn(move || {
                    let result = backend.publish_message(&namespace, &queue, &body, &routing_key, &[], &content_type);
                    let _ = tx.send(BgResult::ScheduledPublished { id, result });
                });
            }
        }
    }

    pub fn cancel_scheduled_message(&mut self, id: u64) {
        self.scheduled_messages.retain(|m| m.id != id);
    }

    pub fn do_reset_offsets(&self) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let queue = self.current_queue_name_for_groups().clone();
            let group = self.reset_group_name.clone();
            let strategy = match &self.reset_strategy {
                Some(s) => s.clone(),
                None => return,
            };
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = backend.reset_consumer_group_offsets(&namespace, &queue, &group, strategy);
                let _ = tx.send(BgResult::OffsetReset(result));
            });
        }
    }

    /// Get the queue name for consumer groups context (from selected queue in queue list)
    pub fn current_queue_name_for_groups(&self) -> String {
        self.selected_queue()
            .map(|q| q.name.clone())
            .unwrap_or_default()
    }

    pub fn load_consumer_groups(&self, queue: &str) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let queue = queue.to_string();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = backend.consumer_groups(&namespace, &queue);
                let _ = tx.send(BgResult::ConsumerGroups(result));
            });
        }
    }

    pub fn load_queue_detail(&self, queue: &str) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let queue = queue.to_string();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = backend.queue_detail(&namespace, &queue);
                let _ = tx.send(BgResult::QueueDetail(result));
            });
        }
    }

    pub fn do_copy_or_move(&mut self, source: &str, dest: &str, operation: QueueOperation) {
        if let Some(ref backend) = self.backend {
            let backend = backend.clone_backend();
            let namespace = self.selected_namespace.clone();
            let source = source.to_string();
            let dest = dest.to_string();
            let fetch_count = self.fetch_count;
            let tx = self.bg_sender.clone();
            let cancel = self.operation_cancel.clone();
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            self.operation_progress = (0, 0);
            self.popup = Popup::OperationProgress;

            std::thread::spawn(move || {
                // Step 1: get messages from source
                let messages = if operation == QueueOperation::Copy {
                    backend.peek_messages(&namespace, &source, fetch_count)
                } else {
                    backend.consume_messages(&namespace, &source, fetch_count)
                };

                let messages = match messages {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = tx.send(BgResult::OperationComplete(Err(e)));
                        return;
                    }
                };

                let total = messages.len();
                let _ = tx.send(BgResult::OperationProgress { completed: 0, total });

                // Step 2: publish each message to destination
                for (i, msg) in messages.iter().enumerate() {
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = tx.send(BgResult::OperationComplete(
                            Ok(format!("Cancelled after {}/{} messages", i, total))
                        ));
                        return;
                    }

                    let headers: Vec<(String, String)> = msg.headers.clone();
                    if let Err(e) = backend.publish_message(
                        &namespace, &dest, &msg.body, &msg.routing_key, &headers, &msg.content_type,
                    ) {
                        let _ = tx.send(BgResult::OperationComplete(
                            Err(format!("Failed at message {}/{}: {}", i + 1, total, e))
                        ));
                        return;
                    }

                    let _ = tx.send(BgResult::OperationProgress { completed: i + 1, total });
                }

                let op_name = if operation == QueueOperation::Copy { "Copied" } else { "Moved" };
                let _ = tx.send(BgResult::OperationComplete(
                    Ok(format!("{} {} messages from {} to {}", op_name, total, source, dest))
                ));
            });
        }
    }

    pub fn process_bg_results(&mut self) {
        while let Ok(result) = self.bg_receiver.try_recv() {
            match result {
                BgResult::BrokerInfo(Ok(info)) => {
                    self.broker_info = Some(info);
                }
                BgResult::BrokerInfo(Err(_)) => {
                    // Silent fail for broker info
                }
                BgResult::Namespaces(Ok(ns)) => {
                    self.namespaces = ns;

                    // Check if profile has an explicit vhost configured
                    let configured_vhost = self.config.profiles.get(&self.profile_name)
                        .and_then(|p| p.vhost.as_ref())
                        .filter(|v| !v.is_empty());

                    if self.namespaces.is_empty() {
                        self.loading = false;
                        self.set_status("No namespaces available", true);
                    } else if self.namespaces.len() == 1 {
                        // Single namespace — use it directly
                        self.selected_namespace = self.namespaces[0].clone();
                        self.screen = Screen::QueueList;
                        self.loading = true;
                        self.load_queues();
                    } else if let Some(vhost) = configured_vhost {
                        // Explicit vhost configured — use it directly, no picker
                        let ns = if self.namespaces.contains(vhost) {
                            vhost.clone()
                        } else {
                            self.namespaces.first().cloned().unwrap_or_default()
                        };
                        self.selected_namespace = ns;
                        self.screen = Screen::QueueList;
                        self.loading = true;
                        self.load_queues();
                    } else {
                        // No vhost configured + multiple namespaces — show picker
                        let default_ns = self.namespaces.first().cloned().unwrap_or_default();
                        self.selected_namespace = default_ns.clone();
                        self.screen = Screen::QueueList;
                        self.popup = Popup::NamespacePicker;
                        let idx = self.namespaces.iter()
                            .position(|n| n == &default_ns)
                            .unwrap_or(0);
                        self.popup_list_state.select(Some(idx));
                        self.loading = true;
                        self.load_queues();
                    }
                }
                BgResult::Namespaces(Err(e)) => {
                    self.loading = false;
                    self.set_status(format!("Error loading namespaces: {}", e), true);
                }
                BgResult::Queues { namespace, result } => {
                    if namespace != self.selected_namespace {
                        continue; // stale result
                    }
                    match result {
                        Ok(queues) => {
                            // Preserve current selection if the queue still exists
                            let previously_selected = self.selected_queue()
                                .map(|q| q.name.clone());

                            // Record rate history for sparklines
                            for q in &queues {
                                let history = self.rate_history
                                    .entry(q.name.clone())
                                    .or_insert_with(RateHistory::new);
                                history.push(q.publish_rate, q.deliver_rate);
                            }

                            self.set_status(format!("{} queues loaded", queues.len()), false);
                            self.queues = queues;
                            self.update_filtered_queues();

                            // Try to restore selection
                            if let Some(prev_name) = previously_selected {
                                let restored = self.filtered_queue_indices.iter()
                                    .position(|&idx| self.queues[idx].name == prev_name);
                                if let Some(pos) = restored {
                                    self.queue_list_state.select(Some(pos));
                                } else if !self.filtered_queue_indices.is_empty() {
                                    self.queue_list_state.select(Some(0));
                                }
                            } else if !self.filtered_queue_indices.is_empty() {
                                self.queue_list_state.select(Some(0));
                            }
                            self.loading = false;
                        }
                        Err(e) => {
                            self.loading = false;
                            self.set_status(format!("Error: {}", e), true);
                        }
                    }
                }
                BgResult::Messages { queue_name, result } => {
                    if queue_name != self.current_queue_name {
                        continue; // stale result
                    }
                    self.loading = false;
                    match result {
                        Ok(messages) => {
                            self.set_status(
                                format!("{} messages from {}", messages.len(), queue_name),
                                false,
                            );
                            self.messages = messages;
                            self.selected_messages.clear();
                            self.update_filtered_messages();
                            if !self.filtered_message_indices.is_empty() {
                                self.message_list_state.select(Some(0));
                            }
                        }
                        Err(e) => {
                            self.set_status(format!("Error: {}", e), true);
                        }
                    }
                }
                BgResult::Published(Ok(())) => {
                    self.set_status("Message published", false);
                    self.popup = Popup::None;
                    if self.screen == Screen::MessageList {
                        self.loading = true;
                        self.load_messages();
                    }
                }
                BgResult::Published(Err(e)) => {
                    self.publish_form.error = format!("Publish failed: {}", e);
                }
                BgResult::Purged(Ok(())) => {
                    self.set_status("Queue purged", false);
                    self.loading = true;
                    self.load_queues();
                }
                BgResult::Purged(Err(e)) => {
                    self.set_status(format!("Purge failed: {}", e), true);
                }
                BgResult::Deleted(Ok(())) => {
                    self.set_status("Queue deleted", false);
                    self.loading = true;
                    self.load_queues();
                }
                BgResult::Deleted(Err(e)) => {
                    self.set_status(format!("Delete failed: {}", e), true);
                }
                BgResult::OperationProgress { completed, total } => {
                    self.operation_progress = (completed, total);
                }
                BgResult::OperationComplete(Ok(msg)) => {
                    self.popup = Popup::None;
                    self.set_status(msg, false);
                    self.loading = true;
                    self.load_queues();
                }
                BgResult::OperationComplete(Err(e)) => {
                    self.popup = Popup::None;
                    self.set_status(e, true);
                }
                BgResult::ConsumerGroups(Ok(groups)) => {
                    self.consumer_groups = groups;
                    self.consumer_groups_scroll = 0;
                    self.loading = false;
                }
                BgResult::ConsumerGroups(Err(e)) => {
                    self.popup = Popup::None;
                    self.loading = false;
                    self.set_status(format!("Consumer groups: {}", e), true);
                }
                BgResult::ReplayComplete(Ok(count)) => {
                    self.popup = Popup::None;
                    self.set_status(format!("Replayed {} messages", count), false);
                }
                BgResult::ReplayComplete(Err(e)) => {
                    self.popup = Popup::None;
                    self.set_status(format!("Replay failed: {}", e), true);
                }
                BgResult::Topology(Ok((exchanges, bindings))) => {
                    self.topology_exchanges = exchanges;
                    self.topology_bindings = bindings;
                    self.topology_scroll = 0;
                    self.loading = false;
                }
                BgResult::Topology(Err(e)) => {
                    self.popup = Popup::None;
                    self.loading = false;
                    self.set_status(format!("Topology: {}", e), true);
                }
                BgResult::BenchmarkProgress { completed, total, latency_ms: _ } => {
                    self.bench_progress = (completed, total);
                }
                BgResult::BenchmarkComplete { total, errors, elapsed_ms, avg_latency_ms } => {
                    self.bench_stats = Some(BenchmarkStats { total, errors, elapsed_ms, avg_latency_ms });
                    let msgs_per_sec = if elapsed_ms > 0 { total as f64 / (elapsed_ms as f64 / 1000.0) } else { 0.0 };
                    self.set_status(
                        format!("Benchmark: {} msgs in {}ms ({:.0} msg/s, avg {}ms, {} errors)",
                            total, elapsed_ms, msgs_per_sec, avg_latency_ms, errors),
                        errors > 0,
                    );
                }
                BgResult::CompareMessages { queue_a, queue_b, messages_a, messages_b } => {
                    self.loading = false;
                    match (messages_a, messages_b) {
                        (Ok(ma), Ok(mb)) => {
                            let result = compute_comparison(&queue_a, &queue_b, ma, mb);
                            self.comparison_result = Some(result);
                            self.comparison_tab = ComparisonTab::Summary;
                            self.comparison_scroll = 0;
                            self.popup = Popup::CompareResults;
                        }
                        (Err(e), _) => {
                            self.popup = Popup::None;
                            self.set_status(format!("Failed to load {}: {}", queue_a, e), true);
                        }
                        (_, Err(e)) => {
                            self.popup = Popup::None;
                            self.set_status(format!("Failed to load {}: {}", queue_b, e), true);
                        }
                    }
                }
                BgResult::ScheduledPublished { id: _, result } => {
                    match result {
                        Ok(()) => {
                            self.set_status("Scheduled message published", false);
                            if self.screen == Screen::MessageList {
                                self.load_messages();
                            }
                        }
                        Err(e) => {
                            self.set_status(format!("Scheduled publish failed: {}", e), true);
                        }
                    }
                }
                BgResult::OffsetReset(Ok(msg)) => {
                    self.popup = Popup::ConsumerGroups;
                    self.set_status(msg, false);
                    // Reload consumer groups to show updated offsets
                    let queue = self.current_queue_name_for_groups();
                    if !queue.is_empty() {
                        self.load_consumer_groups(&queue);
                    }
                }
                BgResult::OffsetReset(Err(e)) => {
                    self.popup = Popup::ConsumerGroups;
                    self.set_status(format!("Offset reset failed: {}", e), true);
                }
                BgResult::QueueDetail(Ok(detail)) => {
                    self.queue_detail = detail;
                    self.queue_info_scroll = 0;
                    self.loading = false;
                }
                BgResult::QueueDetail(Err(e)) => {
                    self.popup = Popup::None;
                    self.loading = false;
                    self.set_status(format!("Queue detail: {}", e), true);
                }
            }
        }
    }

    pub fn update_filtered_queues(&mut self) {
        let filter = self.queue_filter.to_lowercase();
        self.filtered_queue_indices = self.queues.iter().enumerate()
            .filter(|(_, q)| {
                filter.is_empty() || q.name.to_lowercase().contains(&filter)
            })
            .map(|(i, _)| i)
            .collect();
    }

    pub fn update_filtered_messages(&mut self) {
        let filter = self.message_filter.to_lowercase();
        if filter.is_empty() {
            self.filtered_message_indices = (0..self.messages.len()).collect();
            return;
        }

        if self.message_filter_advanced {
            let expr = parse_filter_expr(&self.message_filter);
            self.filtered_message_indices = self.messages.iter().enumerate()
                .filter(|(_, m)| eval_filter_expr(&expr, m))
                .map(|(i, _)| i)
                .collect();
        } else {
            self.filtered_message_indices = self.messages.iter().enumerate()
                .filter(|(_, m)| {
                    m.body.to_lowercase().contains(&filter)
                        || m.routing_key.to_lowercase().contains(&filter)
                })
                .map(|(i, _)| i)
                .collect();
        }
    }

    pub fn selected_queue(&self) -> Option<&QueueInfo> {
        let selected = self.queue_list_state.selected()?;
        let idx = *self.filtered_queue_indices.get(selected)?;
        self.queues.get(idx)
    }

    pub fn selected_message(&self) -> Option<&MessageInfo> {
        let selected = self.message_list_state.selected()?;
        let idx = *self.filtered_message_indices.get(selected)?;
        self.messages.get(idx)
    }
}

/// Filter expression for advanced message filtering
#[derive(Debug)]
enum FilterExpr {
    Substring(String),
    FieldEquals { field: String, value: String },
    FieldContains { field: String, value: String },
    FieldNotEquals { field: String, value: String },
}

/// Parse a filter expression string into a FilterExpr.
/// Supported syntax:
///   header.key = "value"    — exact match on header
///   body contains "text"    — substring match in body
///   routing_key = "value"   — exact match on routing_key
///   body.field = "value"    — JSON field match in body
///   field != "value"        — not-equals
fn parse_filter_expr(input: &str) -> FilterExpr {
    let input = input.trim();

    // Try "field contains value"
    if let Some(idx) = input.to_lowercase().find(" contains ") {
        let field = input[..idx].trim().to_string();
        let value = input[idx + 10..].trim().trim_matches('"').to_string();
        return FilterExpr::FieldContains { field, value };
    }

    // Try "field != value"
    if let Some(idx) = input.find("!=") {
        let field = input[..idx].trim().to_string();
        let value = input[idx + 2..].trim().trim_matches('"').to_string();
        return FilterExpr::FieldNotEquals { field, value };
    }

    // Try "field = value"
    if let Some(idx) = input.find('=') {
        let field = input[..idx].trim().to_string();
        let value = input[idx + 1..].trim().trim_matches('"').to_string();
        return FilterExpr::FieldEquals { field, value };
    }

    // Fallback to substring
    FilterExpr::Substring(input.to_lowercase())
}

/// Resolve a field path to a value from a MessageInfo
fn resolve_field(field: &str, msg: &MessageInfo) -> String {
    match field {
        "body" => msg.body.clone(),
        "routing_key" => msg.routing_key.clone(),
        "exchange" => msg.exchange.clone(),
        "content_type" => msg.content_type.clone(),
        "redelivered" => msg.redelivered.to_string(),
        _ if field.starts_with("header.") || field.starts_with("headers.") => {
            let key = field.splitn(2, '.').nth(1).unwrap_or("");
            msg.headers.iter()
                .find(|(k, _)| k.to_lowercase() == key.to_lowercase())
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        }
        _ if field.starts_with("body.") => {
            // JSON path lookup in body
            let path = field.splitn(2, '.').nth(1).unwrap_or("");
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg.body) {
                let parts: Vec<&str> = path.split('.').collect();
                let mut current = &val;
                for part in &parts {
                    if let Some(next) = current.get(part) {
                        current = next;
                    } else {
                        return String::new();
                    }
                }
                match current {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                }
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

fn eval_filter_expr(expr: &FilterExpr, msg: &MessageInfo) -> bool {
    match expr {
        FilterExpr::Substring(s) => {
            msg.body.to_lowercase().contains(s)
                || msg.routing_key.to_lowercase().contains(s)
        }
        FilterExpr::FieldEquals { field, value } => {
            let resolved = resolve_field(field, msg);
            resolved.to_lowercase() == value.to_lowercase()
        }
        FilterExpr::FieldContains { field, value } => {
            let resolved = resolve_field(field, msg);
            resolved.to_lowercase().contains(&value.to_lowercase())
        }
        FilterExpr::FieldNotEquals { field, value } => {
            let resolved = resolve_field(field, msg);
            resolved.to_lowercase() != value.to_lowercase()
        }
    }
}

fn compute_comparison(queue_a: &str, queue_b: &str, messages_a: Vec<MessageInfo>, messages_b: Vec<MessageInfo>) -> QueueComparisonResult {
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};

    fn hash_body(body: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        body.hash(&mut hasher);
        hasher.finish()
    }

    // Build hash maps: hash -> count for each queue
    let mut b_hashes: HashMap<u64, Vec<usize>> = HashMap::new();
    for (i, msg) in messages_b.iter().enumerate() {
        b_hashes.entry(hash_body(&msg.body)).or_default().push(i);
    }

    let mut matched_b: HashSet<usize> = HashSet::new();
    let mut only_in_a = Vec::new();
    let mut in_both = 0usize;

    for msg in &messages_a {
        let h = hash_body(&msg.body);
        if let Some(indices) = b_hashes.get_mut(&h) {
            // Find an unmatched index in B
            if let Some(pos) = indices.iter().position(|&idx| !matched_b.contains(&idx)) {
                matched_b.insert(indices[pos]);
                in_both += 1;
            } else {
                only_in_a.push(msg.clone());
            }
        } else {
            only_in_a.push(msg.clone());
        }
    }

    let only_in_b: Vec<MessageInfo> = messages_b.iter().enumerate()
        .filter(|(i, _)| !matched_b.contains(i))
        .map(|(_, m)| m.clone())
        .collect();

    QueueComparisonResult {
        queue_a: queue_a.to_string(),
        queue_b: queue_b.to_string(),
        only_in_a,
        only_in_b,
        in_both,
    }
}

fn chrono_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

/// Parse x-death header value to extract original exchange and routing key
pub fn parse_x_death_value(value: &str) -> Option<(String, String)> {
    // x-death is typically a JSON array: [{"exchange":"...", "routing-keys":["..."], ...}]
    // or a stringified version of it
    if let Ok(arr) = serde_json::from_str::<serde_json::Value>(value) {
        let entry = if arr.is_array() { arr.get(0)? } else { &arr };
        let exchange = entry.get("exchange").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let routing_key = entry.get("routing-keys")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !exchange.is_empty() || !routing_key.is_empty() {
            return Some((exchange, routing_key));
        }
    }
    None
}

fn message_to_json(m: &MessageInfo) -> String {
    let headers: serde_json::Map<String, serde_json::Value> = m.headers.iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
        .collect();
    let val = serde_json::json!({
        "index": m.index,
        "routing_key": m.routing_key,
        "exchange": m.exchange,
        "redelivered": m.redelivered,
        "timestamp": m.timestamp,
        "content_type": m.content_type,
        "headers": headers,
        "body": m.body,
    });
    serde_json::to_string(&val).unwrap_or_default()
}

/// RabbitMQ dump: consume all → write JSONL → re-publish all back
fn dump_rabbitmq(
    backend: Box<dyn Backend>,
    namespace: &str,
    queue: &str,
    tx: mpsc::Sender<BgResult>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::io::{BufRead, Write};

    let filename = format!("queuepeek-dump-{}-{}.jsonl", queue, chrono_timestamp());
    let path = std::env::current_dir().unwrap_or_default().join(&filename);
    let file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(BgResult::OperationComplete(Err(format!("Creating file: {}", e))));
            return;
        }
    };
    let mut writer = std::io::BufWriter::new(file);

    // Phase 1: consume all messages to JSONL file
    let batch_size = 100u32;
    let mut total = 0usize;

    loop {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = tx.send(BgResult::OperationComplete(
                Ok(format!("Dump cancelled after {} messages — saved to {}", total, path.display()))
            ));
            return;
        }

        let batch = match backend.consume_messages(namespace, queue, batch_size) {
            Ok(msgs) => msgs,
            Err(e) => {
                if total > 0 {
                    // We already consumed some — must re-publish what we have
                    drop(writer);
                    let _ = tx.send(BgResult::OperationProgress { completed: total, total: 0 });
                    republish_from_file(&backend, namespace, queue, &path, &tx, &cancel);
                    let _ = tx.send(BgResult::OperationComplete(
                        Ok(format!("Dumped {} messages to {} (consume stopped: {})", total, path.display(), e))
                    ));
                } else {
                    let _ = tx.send(BgResult::OperationComplete(Err(format!("Consume failed: {}", e))));
                }
                return;
            }
        };

        if batch.is_empty() { break; }

        for msg in &batch {
            let json = message_to_json(msg);
            if let Err(e) = writeln!(writer, "{}", json) {
                let _ = tx.send(BgResult::OperationComplete(
                    Err(format!("Writing: {} — partial dump at {}", e, path.display()))
                ));
                return;
            }
        }

        total += batch.len();
        let _ = tx.send(BgResult::OperationProgress { completed: total, total: 0 });

        if (batch.len() as u32) < batch_size { break; }
    }

    drop(writer);

    // Phase 2: re-publish all messages back to restore the queue
    republish_from_file(&backend, namespace, queue, &path, &tx, &cancel);

    let _ = tx.send(BgResult::OperationComplete(
        Ok(format!("Dumped {} messages to {}", total, path.display()))
    ));
}

/// Re-publish all messages from a JSONL file back to the queue
fn republish_from_file(
    backend: &Box<dyn Backend>,
    namespace: &str,
    queue: &str,
    path: &std::path::Path,
    tx: &mpsc::Sender<BgResult>,
    cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::io::BufRead;

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(BgResult::OperationComplete(
                Err(format!("Reading dump for re-publish: {} — file at {}", e, path.display()))
            ));
            return;
        }
    };
    let reader = std::io::BufReader::new(file);

    for line in reader.lines() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let body = msg["body"].as_str().unwrap_or("");
        let routing_key = msg["routing_key"].as_str().unwrap_or("");
        let content_type = msg["content_type"].as_str().unwrap_or("");
        let headers: Vec<(String, String)> = msg["headers"].as_object()
            .map(|h| h.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
            .unwrap_or_default();

        let _ = backend.publish_message(namespace, queue, body, routing_key, &headers, content_type);
    }
}

/// Kafka dump: dedicated consumer from low watermark, non-destructive full read
fn dump_kafka(
    backend: Box<dyn Backend>,
    namespace: &str,
    queue: &str,
    tx: mpsc::Sender<BgResult>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::io::Write;

    let filename = format!("queuepeek-dump-{}-{}.jsonl", queue, chrono_timestamp());
    let path = std::env::current_dir().unwrap_or_default().join(&filename);
    let file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(BgResult::OperationComplete(Err(format!("Creating file: {}", e))));
            return;
        }
    };
    let mut writer = std::io::BufWriter::new(file);

    // For Kafka, use consume_messages in large batches — it reads from low watermark
    // and Kafka doesn't actually delete data on consume
    let batch_size = 500u32;
    let mut total = 0usize;
    let mut empty_polls = 0;

    loop {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = tx.send(BgResult::OperationComplete(
                Ok(format!("Dump cancelled after {} messages — saved to {}", total, path.display()))
            ));
            return;
        }

        let batch = match backend.peek_messages(namespace, queue, batch_size) {
            Ok(msgs) => msgs,
            Err(e) => {
                if total > 0 {
                    let _ = tx.send(BgResult::OperationComplete(
                        Ok(format!("Dumped {} messages to {} (stopped: {})", total, path.display(), e))
                    ));
                } else {
                    let _ = tx.send(BgResult::OperationComplete(Err(format!("Peek failed: {}", e))));
                }
                return;
            }
        };

        if batch.is_empty() {
            empty_polls += 1;
            if empty_polls >= 2 { break; }
            continue;
        }
        empty_polls = 0;

        for msg in &batch {
            let json = message_to_json(msg);
            if let Err(e) = writeln!(writer, "{}", json) {
                let _ = tx.send(BgResult::OperationComplete(
                    Err(format!("Writing: {} — partial dump at {}", e, path.display()))
                ));
                return;
            }
        }

        total += batch.len();
        let _ = tx.send(BgResult::OperationProgress { completed: total, total: 0 });

        // Kafka peek calculates offsets from watermarks each call, so we get one batch
        break;
    }

    let _ = tx.send(BgResult::OperationComplete(
        Ok(format!("Dumped {} messages to {}", total, path.display()))
    ));
}

/// Simple peek-based dump for MQTT and other backends
fn dump_simple_peek(
    backend: Box<dyn Backend>,
    namespace: &str,
    queue: &str,
    tx: mpsc::Sender<BgResult>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::io::Write;

    let filename = format!("queuepeek-dump-{}-{}.jsonl", queue, chrono_timestamp());
    let path = std::env::current_dir().unwrap_or_default().join(&filename);
    let file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(BgResult::OperationComplete(Err(format!("Creating file: {}", e))));
            return;
        }
    };
    let mut writer = std::io::BufWriter::new(file);

    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        let _ = tx.send(BgResult::OperationComplete(Ok("Dump cancelled".into())));
        return;
    }

    let batch = match backend.peek_messages(namespace, queue, 100) {
        Ok(msgs) => msgs,
        Err(e) => {
            let _ = tx.send(BgResult::OperationComplete(Err(format!("Peek failed: {}", e))));
            return;
        }
    };

    for msg in &batch {
        let json = message_to_json(msg);
        if let Err(e) = writeln!(writer, "{}", json) {
            let _ = tx.send(BgResult::OperationComplete(
                Err(format!("Writing: {} — partial dump at {}", e, path.display()))
            ));
            return;
        }
    }

    let _ = tx.send(BgResult::OperationProgress { completed: batch.len(), total: batch.len() });
    let _ = tx.send(BgResult::OperationComplete(
        Ok(format!("Dumped {} messages to {}", batch.len(), path.display()))
    ));
}
