use crate::config::AppConfig;
use crate::converter::xray_config::{build_config, ConvertStats};
use crate::error::Result;
use crate::model::proxy::{Compat, Proxy};
use crate::subscription::fetcher::Fetcher;
use crate::subscription::parser::{parse, ParsedSubscription};
use crate::sysproxy;
use crate::xray::process::XrayHandle;
use std::collections::HashMap;

pub struct App {
    pub cfg: AppConfig,
    pub sub: Option<ParsedSubscription>,
    pub xray: Option<XrayHandle>,
    pub xray_running: bool,
    pub last_stats: ConvertStats,
    pub logs: Vec<String>,
    pub ruleset_payloads: HashMap<String, Vec<String>>,
}

impl App {
    pub fn init(cfg: AppConfig) -> Result<Self> {
        let xray = XrayHandle::new(&cfg.xray_path).ok();
        Ok(Self {
            cfg,
            sub: None,
            xray,
            xray_running: false,
            last_stats: ConvertStats::default(),
            logs: Vec::new(),
            ruleset_payloads: HashMap::new(),
        })
    }

    /// All parsed proxies, in display order (supported and unsupported alike).
    /// The node list renders from this, so selection must index into it too —
    /// otherwise the cursor row and the row actually selected on Enter drift
    /// apart whenever unsupported proxies are interspersed.
    pub fn all_proxies(&self) -> Vec<&Proxy> {
        self.sub
            .as_ref()
            .map(|s| s.proxies.iter().collect())
            .unwrap_or_default()
    }

    pub fn supported_proxies(&self) -> Vec<&Proxy> {
        self.sub
            .as_ref()
            .map(|s| {
                s.proxies
                    .iter()
                    .filter(|p| p.compat() == Compat::Supported)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Network-refresh the active subscription: fetch YAML, parse it, download
    /// rule-sets, then persist a local snapshot so the next launch is instant.
    pub fn refresh_subscription(&mut self) -> Result<()> {
        let Some(entry) = self.cfg.active_subscription().cloned() else {
            return Ok(());
        };
        let fetcher = Fetcher::new(crate::store::paths::cache_dir())?;
        let text = fetcher.fetch_text(&entry.url)?;
        let parsed = parse(&text)?;
        self.log(format!(
            "subscription refreshed: {} proxies ({} skipped)",
            parsed.proxies.len(),
            parsed.skipped_proxies
        ));

        // Best-effort: download each HTTP rule-provider so RULE-SET rules can be
        // expanded into xray routing. Failures are non-fatal (the fallback rules
        // still keep routing sane); they are logged and skipped.
        self.ruleset_payloads.clear();
        if let Ok(expander) = crate::converter::ruleset_expander::RuleSetExpander::new() {
            for (name, rp) in &parsed.rule_providers {
                if rp.kind != "http" {
                    continue;
                }
                let Some(url) = &rp.url else { continue };
                match expander.fetch_payload(url) {
                    Ok(lines) => {
                        self.log(format!("rule-set {name}: {} lines", lines.len()));
                        self.ruleset_payloads.insert(name.clone(), lines);
                    }
                    Err(e) => self.log(format!("rule-set {name} download failed: {e}")),
                }
            }
        }

        self.sub = Some(parsed);
        // Persist now that the fetch fully succeeded; ignore write errors
        // (worst case the next launch re-fetches).
        let _ = self.save_snapshot(&text);
        Ok(())
    }

    /// Load the last successfully fetched subscription from local snapshot,
    /// without touching the network. Returns `false` if no usable snapshot
    /// exists (caller should fall back to a real refresh).
    pub fn load_cached_subscription(&mut self) -> Result<bool> {
        let sub_path = crate::store::paths::subscription_snapshot();
        let text = match std::fs::read_to_string(&sub_path) {
            Ok(t) => t,
            Err(_) => return Ok(false),
        };
        let parsed = match parse(&text) {
            Ok(p) => p,
            Err(_) => return Ok(false),
        };
        // Rule-set payloads are best-effort; missing/currupt file just means
        // routing falls back until the next manual refresh.
        if let Ok(json) = std::fs::read_to_string(crate::store::paths::rulesets_snapshot()) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, Vec<String>>>(&json) {
                self.ruleset_payloads = map;
            }
        }
        self.log(format!(
            "loaded cached subscription: {} proxies ({} skipped)",
            parsed.proxies.len(),
            parsed.skipped_proxies
        ));
        self.sub = Some(parsed);
        Ok(true)
    }

    /// Persist the raw subscription text + rule-set payloads to the snapshot
    /// files (0600 — they contain credentials).
    fn save_snapshot(&self, sub_text: &str) -> Result<()> {
        let sub_path = crate::store::paths::subscription_snapshot();
        if let Some(parent) = sub_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&sub_path, sub_text)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&sub_path, std::fs::Permissions::from_mode(0o600)).ok();
        }
        let rs_path = crate::store::paths::rulesets_snapshot();
        std::fs::write(&rs_path, serde_json::to_vec_pretty(&self.ruleset_payloads)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&rs_path, std::fs::Permissions::from_mode(0o600)).ok();
        }
        Ok(())
    }

    /// Activate subscription `index`, persist config, and load that
    /// subscription's locally-cached YAML (no network). Returns whether a cache
    /// hit populated `self.sub`.
    pub fn switch_active(&mut self, index: usize) -> Result<bool> {
        if index >= self.cfg.subscriptions.len() {
            return Ok(false);
        }
        for (i, s) in self.cfg.subscriptions.iter_mut().enumerate() {
            s.active = i == index;
        }
        let url = self.cfg.subscriptions[index].url.clone();
        self.cfg.save()?;
        // Try the per-URL cache written by Fetcher on the last successful fetch.
        let path = Fetcher::cache_path_for(&url);
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(parsed) = parse(&text) {
                self.sub = Some(parsed);
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Add a subscription entry. `name` empty → derive from the URL host; on
    /// collision a `-2`/`-3` suffix is added. Only the first entry auto-activates.
    pub fn add_subscription(&mut self, url: &str, name: &str) -> usize {
        let name = if name.trim().is_empty() {
            derive_name(url, &self.cfg.subscriptions)
        } else {
            unique_name(name.trim(), &self.cfg.subscriptions)
        };
        let active = self.cfg.subscriptions.is_empty();
        self.cfg.subscriptions.push(crate::config::SubscriptionEntry {
            name,
            url: url.into(),
            active,
        });
        let idx = self.cfg.subscriptions.len() - 1;
        let _ = self.cfg.save();
        idx
    }

    /// Delete subscription `index`. If the active one was removed, activate the
    /// first remaining (if any). Returns whether deletion happened.
    pub fn delete_subscription(&mut self, index: usize) -> bool {
        if index >= self.cfg.subscriptions.len() {
            return false;
        }
        let was_active = self.cfg.subscriptions[index].active;
        self.cfg.subscriptions.remove(index);
        if was_active {
            if let Some(first) = self.cfg.subscriptions.first_mut() {
                first.active = true;
            }
        }
        let _ = self.cfg.save();
        true
    }

    /// Select the active proxy by name and (re)start xray with the new config.
    pub fn select_proxy(&mut self, name: &str) -> Result<()> {
        let sub = match &self.sub {
            Some(s) => s,
            None => return Ok(()),
        };
        let proxy = sub
            .proxies
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| crate::error::Error::Other(format!("proxy not found: {name}")))?;
        if !matches!(proxy.compat(), Compat::Supported) {
            return Err(crate::error::Error::Other(format!(
                "proxy not supported: {name}"
            )));
        }
        let (cfg, stats) = build_config(
            sub,
            proxy,
            self.cfg.http_port,
            self.cfg.socks_port,
            &self.ruleset_payloads,
        )?;
        self.last_stats = stats;
        if let Some(x) = self.xray.as_mut() {
            x.start(&cfg)?;
            self.xray_running = true;
            self.log(format!("xray started with {name}"));
        } else {
            self.log("xray binary unavailable; config generated but not started".into());
        }
        self.cfg.current_proxy = Some(name.into());
        self.cfg.save()?;
        if self.cfg.sys_proxy_on {
            sysproxy::enable(self.cfg.http_port)?;
        }
        Ok(())
    }

    pub fn toggle_sys_proxy(&mut self) -> Result<()> {
        self.cfg.sys_proxy_on = !self.cfg.sys_proxy_on;
        if self.cfg.sys_proxy_on {
            sysproxy::enable(self.cfg.http_port)?;
        } else {
            sysproxy::disable()?;
        }
        self.cfg.save()?;
        Ok(())
    }

    fn log(&mut self, msg: String) {
        self.logs.push(msg);
        if self.logs.len() > 500 {
            self.logs.drain(0..100);
        }
    }

    /// Push a message onto the application log (shown in the Logs tab).
    pub fn push_log(&mut self, msg: impl Into<String>) {
        self.log(msg.into());
    }
}

/// Extract a readable name from a subscription URL: the host part
/// (`https://user:pass@example.com:8443/path` → `example.com`).
fn derive_name(url: &str, existing: &[crate::config::SubscriptionEntry]) -> String {
    let no_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let after_auth = no_scheme.rsplit_once('@').map(|(_, h)| h).unwrap_or(no_scheme);
    let host = after_auth
        .split(['/', ':', '?', '#'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("subscription");
    unique_name(host, existing)
}

/// Ensure a name is unique among existing subscriptions by suffixing `-2`, `-3`, ...
fn unique_name(base: &str, existing: &[crate::config::SubscriptionEntry]) -> String {
    let taken: std::collections::HashSet<&str> =
        existing.iter().map(|s| s.name.as_str()).collect();
    if !taken.contains(base) {
        return base.to_string();
    }
    let mut n = 2;
    loop {
        let candidate = format!("{base}-{n}");
        if !taken.contains(candidate.as_str()) {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SubscriptionEntry;

    fn sub(name: &str) -> SubscriptionEntry {
        SubscriptionEntry {
            name: name.into(),
            url: format!("http://{name}"),
            active: false,
        }
    }

    #[test]
    fn derive_name_takes_host_and_uniquifies() {
        // host extracted, no collision
        let existing = vec![];
        assert_eq!(derive_name("https://a.com:8443/path", &existing), "a.com");
        // collision → -2 suffix
        let existing = vec![sub("a.com")];
        assert_eq!(derive_name("https://a.com/x", &existing), "a.com-2");
        // userinfo stripped
        assert_eq!(derive_name("https://u:p@b.io", &[]), "b.io");
    }

    #[test]
    fn unique_name_increments_suffix() {
        let existing = vec![sub("x"), sub("x-2")];
        assert_eq!(unique_name("x", &existing), "x-3");
        assert_eq!(unique_name("y", &existing), "y");
    }

    #[test]
    fn snapshot_round_trip_and_missing() {
        use tempfile::TempDir;
        // Redirect the config dir so we don't touch the real user config.
        let tmp = TempDir::new().unwrap();
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());

        std::fs::create_dir_all(crate::store::paths::cache_dir()).unwrap();
        let yaml = "proxies: []\nrules: []\n";
        std::fs::write(crate::store::paths::subscription_snapshot(), yaml).unwrap();
        std::fs::write(crate::store::paths::rulesets_snapshot(), "{}").unwrap();

        let mut app = App::init(AppConfig::default()).unwrap();
        // Hit: snapshot present → loaded, no network.
        assert!(app.load_cached_subscription().unwrap());
        assert!(app.sub.is_some());

        // Miss: wipe snapshot → returns false.
        let _ = std::fs::remove_file(crate::store::paths::subscription_snapshot());
        let mut app2 = App::init(AppConfig::default()).unwrap();
        assert!(!app2.load_cached_subscription().unwrap());

        match prev {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
}
