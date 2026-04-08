use std::sync::mpsc;

use ratatui::widgets::ListState;

use crate::config::{Config, Profile};
use crate::backend::{Backend, BrokerInfo, QueueInfo, MessageInfo};
use crate::ui::theme::{get_theme, Theme};

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
}

pub const FETCH_PRESETS: &[u32] = &[10, 25, 50, 100, 250, 500];

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
    pub filtered_message_indices: Vec<usize>,

    // Message detail screen
    pub detail_message_idx: usize,
    pub detail_scroll: u16,
    pub detail_pretty: bool,

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
}

pub const BACKEND_TYPES: &[&str] = &["rabbitmq", "kafka", "mqtt"];

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
            filtered_message_indices: Vec::new(),
            detail_message_idx: 0,
            detail_scroll: 0,
            detail_pretty: true,
            fetch_count: 50,
            status_message: String::new(),
            status_is_error: false,
            loading: false,
            popup: Popup::None,
            popup_list_state: ListState::default(),
            bg_sender: tx,
            bg_receiver: rx,
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
        self.filtered_message_indices = self.messages.iter().enumerate()
            .filter(|(_, m)| {
                filter.is_empty()
                    || m.body.to_lowercase().contains(&filter)
                    || m.routing_key.to_lowercase().contains(&filter)
            })
            .map(|(i, _)| i)
            .collect();
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
