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

#[derive(Debug, Clone, Default)]
pub struct ProfileForm {
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
        7 // name, host, port, username, password, vhost, tls
    }

    pub fn field_label(idx: usize) -> &'static str {
        match idx {
            0 => "Name",
            1 => "Host",
            2 => "Port",
            3 => "Username",
            4 => "Password",
            5 => "Vhost",
            6 => "TLS",
            _ => "",
        }
    }

    pub fn field_value(&self, idx: usize) -> String {
        match idx {
            0 => self.name.clone(),
            1 => self.host.clone(),
            2 => self.port.clone(),
            3 => self.username.clone(),
            4 => self.password.clone(),
            5 => self.vhost.clone(),
            6 => if self.tls { "yes".into() } else { "no".into() },
            _ => String::new(),
        }
    }

    pub fn set_field(&mut self, idx: usize, val: String) {
        match idx {
            0 => self.name = val,
            1 => self.host = val,
            2 => self.port = val,
            3 => self.username = val,
            4 => self.password = val,
            5 => self.vhost = val,
            6 => self.tls = val == "yes",
            _ => {}
        }
    }

    pub fn push_char(&mut self, c: char) {
        if self.focused_field == 6 {
            // Toggle TLS
            self.tls = !self.tls;
            return;
        }
        match self.focused_field {
            0 => self.name.push(c),
            1 => self.host.push(c),
            2 => self.port.push(c),
            3 => self.username.push(c),
            4 => self.password.push(c),
            5 => self.vhost.push(c),
            _ => {}
        }
    }

    pub fn pop_char(&mut self) {
        if self.focused_field == 6 { return; }
        match self.focused_field {
            0 => { self.name.pop(); }
            1 => { self.host.pop(); }
            2 => { self.port.pop(); }
            3 => { self.username.pop(); }
            4 => { self.password.pop(); }
            5 => { self.vhost.pop(); }
            _ => {}
        }
    }

    pub fn from_profile(name: &str, p: &Profile) -> Self {
        Self {
            name: name.to_string(),
            host: p.host.clone(),
            port: p.port.to_string(),
            username: p.username.clone(),
            password: p.password.clone(),
            vhost: p.vhost.clone().unwrap_or_else(|| "/".into()),
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
        let vhost = if self.vhost.is_empty() { "/".into() } else { self.vhost.clone() };

        Ok(Profile {
            profile_type: "rabbitmq".into(),
            host,
            port,
            username,
            password,
            vhost: Some(vhost),
            tls: Some(self.tls),
            tls_cert: None,
            tls_key: None,
            tls_ca: None,
        })
    }

    pub fn clear(&mut self) {
        *self = Self {
            port: "15672".into(),
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
            profile_form: ProfileForm { port: "15672".into(), ..Default::default() },
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

        match crate::backend::rabbitmq::RabbitMqBackend::new(&profile) {
            Ok(backend) => {
                self.backend = Some(Box::new(backend));
                self.profile_name = name.to_string();
                self.loading = true;
                self.set_status("Connecting...", false);

                // Fire background tasks
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

                    // Try to find the profile's configured vhost in namespaces
                    let profile_vhost = self.config.profiles.get(&self.profile_name)
                        .and_then(|p| p.vhost.clone())
                        .unwrap_or_else(|| "/".into());

                    let default_ns = if self.namespaces.contains(&profile_vhost) {
                        profile_vhost
                    } else {
                        self.namespaces.first().cloned().unwrap_or_default()
                    };

                    if self.namespaces.len() == 1 {
                        self.selected_namespace = self.namespaces[0].clone();
                        self.screen = Screen::QueueList;
                        self.loading = true;
                        self.load_queues();
                    } else if self.namespaces.is_empty() {
                        self.loading = false;
                        self.set_status("No namespaces available", true);
                    } else {
                        // Multiple namespaces: move to QueueList and show picker popup
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
