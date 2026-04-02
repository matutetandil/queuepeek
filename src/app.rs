use std::sync::mpsc;

use ratatui::widgets::ListState;

use crate::config::{Config, Profile};
use crate::rabbit::{Message, Overview, Queue, RabbitClient};
use crate::ui::theme::{get_theme, Theme};

// Background task results sent via channel
pub enum BgResult {
    Vhosts(Result<Vec<String>, String>),
    Overview(Result<Overview, String>),
    Queues {
        vhost: String,
        result: Result<Vec<Queue>, String>,
    },
    Messages {
        queue_name: String,
        result: Result<Vec<Message>, String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    ProfileSelect,
    Main,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Sidebar,
    RightHeader,
    RightTabs,
    RightContent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Popup {
    None,
    Help,
    ProfileSwitch,
    VhostPicker,
    QueuePicker,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RightView {
    Overview,
    Queues,
    Exchanges,
    Policies,
    Vhosts,
    Users,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueueTab {
    Overview,
    Publish,
    Consume,
    Routing,
    Settings,
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

    // Theme
    pub theme: &'static Theme,

    // Profile selection
    pub profile_mode: ProfileMode,
    pub profile_list_state: ListState,
    pub profile_form: ProfileForm,

    // Main screen state
    pub client: Option<RabbitClient>,
    pub profile_name: String,
    pub cluster_name: String,

    // Sidebar
    pub sidebar_cursor: usize,  // 0=Overview, 1=Queues, 2=Exchanges, 3=Policies, 4=Vhosts, 5=Users

    // Right panel
    pub right_view: RightView,
    pub queue_tab: QueueTab,

    // Overview data (stored from BgResult::Overview)
    pub rabbitmq_version: String,

    // Vhosts
    pub vhosts: Vec<String>,
    pub selected_vhost: String,

    // Queues
    pub queues: Vec<Queue>,
    pub queue_list_state: ListState,
    pub vhost_list_state: ListState,
    pub queue_filter: String,
    pub queue_filter_active: bool,
    pub filtered_indices: Vec<usize>,

    // Messages
    pub messages: Vec<Message>,
    pub message_scroll: u16,
    pub current_queue_name: String,

    // Focus & overlays
    pub focus: Focus,
    pub popup: Popup,
    pub popup_list_state: ListState,
    pub picker_filter: String,
    pub picker_filter_active: bool,

    // Fetch
    pub fetch_count: u32,

    // Status
    pub status_message: String,
    pub status_is_error: bool,
    pub loading: bool,

    // Background task channel
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
            client: None,
            profile_name: String::new(),
            cluster_name: String::new(),
            sidebar_cursor: 1, // Start on Queues
            right_view: RightView::Queues,
            queue_tab: QueueTab::Consume,
            rabbitmq_version: String::new(),
            vhosts: Vec::new(),
            selected_vhost: String::new(),
            queues: Vec::new(),
            queue_list_state: ListState::default(),
            vhost_list_state: ListState::default(),
            queue_filter: String::new(),
            queue_filter_active: false,
            filtered_indices: Vec::new(),
            messages: Vec::new(),
            message_scroll: 0,
            current_queue_name: String::new(),
            focus: Focus::Sidebar,
            popup: Popup::None,
            popup_list_state: ListState::default(),
            picker_filter: String::new(),
            picker_filter_active: false,
            fetch_count: 50,
            status_message: String::new(),
            status_is_error: false,
            loading: false,
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

        match RabbitClient::new(&profile) {
            Ok(client) => {
                self.client = Some(client);
                self.profile_name = name.to_string();
                self.screen = Screen::Main;
                self.focus = Focus::Sidebar;
                self.loading = true;
                self.set_status("Connecting...", false);

                // Fire background tasks
                self.load_vhosts();
                self.load_overview();
            }
            Err(e) => {
                self.set_status(format!("Connection error: {}", e), true);
            }
        }
    }

    pub fn load_vhosts(&self) {
        if let Some(ref client) = self.client {
            let client = client.clone_for_thread();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = client.list_vhosts();
                let _ = tx.send(BgResult::Vhosts(result));
            });
        }
    }

    pub fn load_overview(&self) {
        if let Some(ref client) = self.client {
            let client = client.clone_for_thread();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = client.get_overview();
                let _ = tx.send(BgResult::Overview(result));
            });
        }
    }

    pub fn load_queues(&self) {
        if let Some(ref client) = self.client {
            let client = client.clone_for_thread();
            let vhost = self.selected_vhost.clone();
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = client.list_queues(&vhost);
                let _ = tx.send(BgResult::Queues { vhost, result });
            });
        }
    }

    pub fn load_messages(&self) {
        if let Some(ref client) = self.client {
            let queue_name = self.current_queue_name.clone();
            if queue_name.is_empty() {
                return;
            }
            let client = client.clone_for_thread();
            let vhost = self.selected_vhost.clone();
            let count = self.fetch_count;
            let tx = self.bg_sender.clone();
            std::thread::spawn(move || {
                let result = client.peek_messages(&vhost, &queue_name, count);
                let _ = tx.send(BgResult::Messages { queue_name, result });
            });
        }
    }

    pub fn process_bg_results(&mut self) {
        while let Ok(result) = self.bg_receiver.try_recv() {
            match result {
                BgResult::Vhosts(Ok(vhosts)) => {
                    self.vhosts = vhosts;
                    // Select default vhost
                    if let Some(ref client) = self.client {
                        let default = client.default_vhost().to_string();
                        if self.vhosts.contains(&default) {
                            self.selected_vhost = default;
                        } else if let Some(first) = self.vhosts.first() {
                            self.selected_vhost = first.clone();
                        }
                    }
                    // Set vhost list selection
                    let vhost_idx = self.vhosts.iter().position(|v| v == &self.selected_vhost).unwrap_or(0);
                    self.vhost_list_state.select(Some(vhost_idx));
                    self.set_status(format!("{} vhosts available", self.vhosts.len()), false);
                    self.load_queues();
                }
                BgResult::Vhosts(Err(e)) => {
                    self.loading = false;
                    self.set_status(format!("Error: {}", e), true);
                }
                BgResult::Overview(Ok(overview)) => {
                    self.cluster_name = overview.cluster_name;
                    self.rabbitmq_version = overview.rabbitmq_version;
                }
                BgResult::Overview(Err(_)) => {} // silent fail for overview
                BgResult::Queues { vhost, result } => {
                    if vhost != self.selected_vhost {
                        return; // stale result
                    }
                    match result {
                        Ok(queues) => {
                            self.set_status(format!("{} queues loaded", queues.len()), false);
                            self.queues = queues;
                            self.update_filtered_queues();
                            if !self.filtered_indices.is_empty() {
                                self.queue_list_state.select(Some(0));
                                // Auto-select first queue
                                let idx = self.filtered_indices[0];
                                self.current_queue_name = self.queues[idx].name.clone();
                                self.loading = true;
                                self.load_messages();
                            } else {
                                self.loading = false;
                            }
                        }
                        Err(e) => {
                            self.loading = false;
                            self.set_status(format!("Error: {}", e), true);
                        }
                    }
                }
                BgResult::Messages { queue_name, result } => {
                    self.loading = false;
                    match result {
                        Ok(messages) => {
                            self.set_status(format!("{} messages from {}", messages.len(), queue_name), false);
                            self.messages = messages;
                            self.message_scroll = 0;
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
        // Use picker_filter when picker popup is open, otherwise queue_filter
        let filter = if self.popup == Popup::QueuePicker {
            self.picker_filter.to_lowercase()
        } else {
            self.queue_filter.to_lowercase()
        };
        self.filtered_indices = self.queues.iter().enumerate()
            .filter(|(_, q)| {
                if filter.is_empty() {
                    true
                } else {
                    q.name.to_lowercase().contains(&filter)
                }
            })
            .map(|(i, _)| i)
            .collect();
    }

    pub fn selected_queue(&self) -> Option<&Queue> {
        let selected = self.queue_list_state.selected()?;
        let idx = *self.filtered_indices.get(selected)?;
        self.queues.get(idx)
    }

    pub fn cycle_vhost(&mut self) {
        if self.vhosts.len() <= 1 {
            return;
        }
        let current_idx = self.vhosts.iter().position(|v| v == &self.selected_vhost).unwrap_or(0);
        let next_idx = (current_idx + 1) % self.vhosts.len();
        self.selected_vhost = self.vhosts[next_idx].clone();
        self.queues.clear();
        self.filtered_indices.clear();
        self.messages.clear();
        self.queue_filter.clear();
        self.current_queue_name.clear();
        self.queue_list_state.select(None);
        self.loading = true;
        self.set_status(format!("Switching to vhost: {}", self.selected_vhost), false);
        self.load_queues();
    }

    pub fn select_queue(&mut self) {
        if let Some(q) = self.selected_queue() {
            self.current_queue_name = q.name.clone();
            self.loading = true;
            self.set_status(format!("Loading messages from {}", self.current_queue_name), false);
            self.load_messages();
        }
    }

    pub fn sidebar_items() -> &'static [(&'static str, &'static str)] {
        // (label, section_header) - empty section_header means no header before this item
        &[
            ("Overview", "Navigation"),
            ("Queues", ""),
            ("Exchanges", ""),
            ("Policies", ""),
            ("Virtual Hosts", "Admin"),
            ("Users", ""),
        ]
    }

    pub fn sidebar_item_count() -> usize {
        6
    }

    pub fn right_view_for_sidebar(cursor: usize) -> RightView {
        match cursor {
            0 => RightView::Overview,
            1 => RightView::Queues,
            2 => RightView::Exchanges,
            3 => RightView::Policies,
            4 => RightView::Vhosts,
            5 => RightView::Users,
            _ => RightView::Queues,
        }
    }
}
