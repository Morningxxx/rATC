use crate::config::AppConfig;
use crate::converter::xray_config::{build_config, ConvertStats};
use crate::error::Result;
use crate::model::proxy::{Compat, Proxy};
use crate::subscription::fetcher::Fetcher;
use crate::subscription::parser::ParsedSubscription;
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

    pub fn supported_proxies(&self) -> Vec<&Proxy> {
        self.sub.as_ref()
            .map(|s| s.proxies.iter().filter(|p| p.compat() == Compat::Supported).collect())
            .unwrap_or_default()
    }

    pub fn refresh_subscription(&mut self) -> Result<()> {
        let Some(entry) = self.cfg.active_subscription().cloned() else {
            return Ok(());
        };
        let fetcher = Fetcher::new(crate::store::paths::cache_dir())?;
        let parsed = fetcher.fetch(&entry.url)?;
        self.log(format!("subscription refreshed: {} proxies", parsed.proxies.len()));
        self.sub = Some(parsed);
        Ok(())
    }

    /// Select the active proxy by name and (re)start xray with the new config.
    pub fn select_proxy(&mut self, name: &str) -> Result<()> {
        let sub = match &self.sub { Some(s) => s, None => return Ok(()) };
        let proxy = sub.proxies.iter().find(|p| p.name == name)
            .ok_or_else(|| crate::error::Error::Other(format!("proxy not found: {name}")))?;
        if !matches!(proxy.compat(), Compat::Supported) {
            return Err(crate::error::Error::Other(format!("proxy not supported: {name}")));
        }
        let (cfg, stats) = build_config(sub, proxy, self.cfg.http_port, self.cfg.socks_port, &self.ruleset_payloads)?;
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
        if self.cfg.sys_proxy_on { sysproxy::enable(self.cfg.http_port)?; }
        Ok(())
    }

    pub fn toggle_sys_proxy(&mut self) -> Result<()> {
        self.cfg.sys_proxy_on = !self.cfg.sys_proxy_on;
        if self.cfg.sys_proxy_on { sysproxy::enable(self.cfg.http_port)?; } else { sysproxy::disable()?; }
        self.cfg.save()?;
        Ok(())
    }

    fn log(&mut self, msg: String) {
        self.logs.push(msg);
        if self.logs.len() > 500 { self.logs.drain(0..100); }
    }
}
