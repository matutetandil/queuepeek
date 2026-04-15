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
    #[serde(default)]
    pub schema_registry: Option<SchemaRegistryConfig>,
}

impl Profile {
    pub fn base_url(&self) -> String {
        let scheme = if self.tls.unwrap_or(false) { "https" } else { "http" };
        format!("{}://{}:{}", scheme, self.host, self.port)
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
pub struct SchemaRegistryConfig {
    pub url: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_base_url_http() {
        let p = Profile {
            host: "localhost".to_string(),
            port: 15672,
            tls: Some(false),
            ..Default::default()
        };
        assert_eq!(p.base_url(), "http://localhost:15672");
    }

    #[test]
    fn profile_base_url_https() {
        let p = Profile {
            host: "rabbit.example.com".to_string(),
            port: 443,
            tls: Some(true),
            ..Default::default()
        };
        assert_eq!(p.base_url(), "https://rabbit.example.com:443");
    }

    #[test]
    fn config_profile_crud() {
        let mut config = Config::default();
        assert!(config.profile_names().is_empty());

        let profile = Profile {
            profile_type: "rabbitmq".to_string(),
            host: "localhost".to_string(),
            port: 15672,
            username: "guest".to_string(),
            password: "guest".to_string(),
            ..Default::default()
        };

        config.add_profile("local".to_string(), profile);
        assert_eq!(config.profile_names(), vec!["local"]);

        config.delete_profile("local");
        assert!(config.profile_names().is_empty());
    }

    #[test]
    fn config_delete_clears_default() {
        let mut config = Config::default();
        config.default_profile = Some("test".to_string());
        config.add_profile("test".to_string(), Profile::default());

        config.delete_profile("test");
        assert_eq!(config.default_profile, None);
    }

    #[test]
    fn config_profile_names_sorted() {
        let mut config = Config::default();
        config.add_profile("zebra".to_string(), Profile::default());
        config.add_profile("alpha".to_string(), Profile::default());
        config.add_profile("middle".to_string(), Profile::default());

        assert_eq!(config.profile_names(), vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn config_save_load_roundtrip() {
        let dir = std::env::temp_dir().join("queuepeek-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test-config.toml");
        let path_str = path.to_str().unwrap();

        let mut config = Config::default();
        config.default_profile = Some("local".to_string());
        config.theme = Some("dracula".to_string());
        config.add_profile("local".to_string(), Profile {
            profile_type: "rabbitmq".to_string(),
            host: "localhost".to_string(),
            port: 15672,
            username: "guest".to_string(),
            password: "guest".to_string(),
            vhost: Some("/".to_string()),
            tls: Some(false),
            ..Default::default()
        });

        config.save(Some(path_str)).unwrap();

        let loaded = Config::load(Some(path_str));
        assert_eq!(loaded.default_profile, Some("local".to_string()));
        assert_eq!(loaded.theme, Some("dracula".to_string()));

        let profile = loaded.profiles.get("local").unwrap();
        assert_eq!(profile.host, "localhost");
        assert_eq!(profile.port, 15672);
        assert_eq!(profile.profile_type, "rabbitmq");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn config_load_nonexistent() {
        let config = Config::load(Some("/tmp/queuepeek-nonexistent-config-12345.toml"));
        // Should return default config, not error
        assert!(config.profiles.is_empty());
    }
}
