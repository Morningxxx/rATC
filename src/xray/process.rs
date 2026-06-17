use crate::error::{Error, Result};
use crate::store::paths::xray_config_file;
use crate::xray::paths::resolve;
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

pub struct XrayHandle {
    child: Option<Child>,
    bin: PathBuf,
}

impl XrayHandle {
    pub fn new(bin_path: &str) -> Result<Self> {
        let bin = resolve(bin_path).ok_or_else(|| Error::Xray(format!("xray binary not found: {bin_path}")))?;
        Ok(Self { child: None, bin })
    }

    /// Validate a config with `xray -test` without launching.
    pub fn test_config(&self, cfg: &Value) -> Result<()> {
        self.write_config(cfg)?;
        let out = Command::new(&self.bin)
            .arg("-test")
            .arg("-config")
            .arg(xray_config_file())
            .output()?;
        if !out.status.success() {
            return Err(Error::Xray(String::from_utf8_lossy(&out.stderr).to_string()));
        }
        Ok(())
    }

    /// Write config to the canonical path, then (re)spawn xray.
    pub fn start(&mut self, cfg: &Value) -> Result<()> {
        self.write_config(cfg)?;
        self.stop();
        let child = Command::new(&self.bin)
            .arg("-config")
            .arg(xray_config_file())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        self.child = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    pub fn is_running(&mut self) -> bool {
        match self.child.as_mut() {
            Some(c) => match c.try_wait() {
                Ok(None) => true,
                _ => false,
            },
            None => false,
        }
    }

    fn write_config(&self, cfg: &Value) -> Result<()> {
        let p = xray_config_file();
        if let Some(parent) = p.parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(&p, serde_json::to_vec_pretty(cfg)?)?;
        Ok(())
    }
}

impl Drop for XrayHandle {
    fn drop(&mut self) { self.stop(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn with_tmp_home<F: FnOnce()>(f: F) {
        let tmp = TempDir::new().unwrap();
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        f();
        match prev { Some(v) => std::env::set_var("XDG_CONFIG_HOME", v), None => std::env::remove_var("XDG_CONFIG_HOME") }
    }

    #[test]
    fn test_config_validates_with_real_xray() {
        let bin = resolve("/usr/local/bin/xray");
        if bin.is_none() { return; } // skip if xray absent
        with_tmp_home(|| {
            let h = XrayHandle::new("/usr/local/bin/xray").unwrap();
            let cfg = json!({
                "inbounds": [{"port": 17890, "listen":"127.0.0.1", "protocol":"http"}],
                "outbounds": [{"protocol":"freedom","tag":"direct"}]
            });
            h.test_config(&cfg).expect("xray should accept config");
        });
    }
}
