use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn default_profile_type() -> String { "rabbitmq".into() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    #[serde(default = "default_profile_type", rename = "type")]
    pub profile_type: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub vhost: Option<String>,
    #[serde(default)]
    pub tls: Option<bool>,
    #[serde(default)]
    pub tls_cert: Option<String>,
    #[serde(default)]
    pub tls_key: Option<String>,
    #[serde(default)]
    pub tls_ca: Option<String>,
    #[serde(default)]
    pub topics: Option<Vec<String>>,
}

impl Profile {
    pub fn base_url(&self) -> String {
        let scheme = if self.tls.unwrap_or(false) { "https" } else { "http" };
        format!("{}://{}:{}", scheme, self.host, self.port)
    }

    pub fn vhost_or_default(&self) -> &str {
        self.vhost.as_deref().unwrap_or("/")
    }

    pub fn is_cloud_host(&self) -> bool {
        let h = self.host.to_lowercase();
        h.contains("cloudamqp.com") || h.contains("amazonaws.com") || h.contains("azure.com") || h.contains("rabbitmq.cloud")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedFilter {
    pub name: String,
    pub expression: String,
    #[serde(default)]
    pub advanced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTemplate {
    pub name: String,
    #[serde(default)]
    pub routing_key: String,
    #[serde(default)]
    pub content_type: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookAlert {
    pub name: String,
    pub pattern: String,
    pub webhook_url: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub queues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub default_profile: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
    #[serde(default)]
    pub filters: HashMap<String, Vec<SavedFilter>>,
    #[serde(default)]
    pub templates: Vec<MessageTemplate>,
    #[serde(default)]
    pub webhook_alerts: Vec<WebhookAlert>,
}

impl Config {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("queuepeek").join("config.toml"))
    }

    pub fn load(path: Option<&str>) -> Self {
        let config_path = match path {
            Some(p) => PathBuf::from(p),
            None => match Self::config_path() {
                Some(p) => p,
                None => return Self::default(),
            },
        };

        match fs::read_to_string(&config_path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: Option<&str>) -> Result<(), String> {
        let config_path = match path {
            Some(p) => PathBuf::from(p),
            None => Self::config_path().ok_or("Cannot determine config directory")?,
        };

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Creating config dir: {}", e))?;
        }

        let contents = toml::to_string_pretty(self).map_err(|e| format!("Serializing config: {}", e))?;
        fs::write(&config_path, contents).map_err(|e| format!("Writing config: {}", e))?;
        Ok(())
    }

    pub fn profile_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.profiles.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn add_profile(&mut self, name: String, profile: Profile) {
        self.profiles.insert(name, profile);
    }

    pub fn delete_profile(&mut self, name: &str) {
        self.profiles.remove(name);
        if self.default_profile.as_deref() == Some(name) {
            self.default_profile = None;
        }
    }
}
