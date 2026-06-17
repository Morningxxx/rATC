use crate::error::Result;
use crate::store::paths::config_file;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEntry {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub http_port: u16,
    pub socks_port: u16,
    pub listen: String,
    pub xray_path: String,
    #[serde(default)]
    pub allow_lan: bool,
    pub log_level: String,
    #[serde(default = "default_true")]
    pub exit_kills_xray: bool,
    #[serde(default)]
    pub sys_proxy_on: bool,
    #[serde(default)]
    pub current_proxy: Option<String>,
    #[serde(default)]
    pub subscriptions: Vec<SubscriptionEntry>,
}

fn default_true() -> bool { true }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            http_port: 7890,
            socks_port: 7891,
            listen: "127.0.0.1".into(),
            xray_path: "/usr/local/bin/xray".into(),
            allow_lan: false,
            log_level: "warning".into(),
            exit_kills_xray: true,
            sys_proxy_on: false,
            current_proxy: None,
            subscriptions: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let p = config_file();
        if !p.exists() {
            let cfg = Self::default();
            cfg.save()?;
            return Ok(cfg);
        }
        let text = std::fs::read_to_string(&p)?;
        let cfg: AppConfig = serde_json::from_str(&text)?;
        Ok(cfg)
    }
    pub fn save(&self) -> Result<()> {
        let p = config_file();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&p, serde_json::to_vec_pretty(self)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600)).ok();
        }
        Ok(())
    }
    pub fn active_subscription(&self) -> Option<&SubscriptionEntry> {
        self.subscriptions.iter().find(|s| s.active).or_else(|| self.subscriptions.first())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_roundtrip() {
        let cfg = AppConfig::default();
        let s = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(back.http_port, 7890);
        assert!(back.exit_kills_xray);
    }

    #[test]
    fn active_subscription_prefers_flag() {
        let cfg = AppConfig {
            subscriptions: vec![
                SubscriptionEntry { name: "a".into(), url: "u1".into(), active: false },
                SubscriptionEntry { name: "b".into(), url: "u2".into(), active: true },
            ],
            ..Default::default()
        };
        assert_eq!(cfg.active_subscription().unwrap().name, "b");
    }
}
