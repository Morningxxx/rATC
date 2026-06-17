use crate::error::Result;
use crate::store::paths::proxy_sh;
use std::path::Path;

fn no_proxy() -> &'static str {
    "localhost,127.0.0.1,::1,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,*.cn"
}

/// Write a shell snippet exporting proxy env vars for the given HTTP port to the
/// canonical `~/.config/ratc/proxy.sh` path.
pub fn enable(http_port: u16) -> Result<()> {
    enable_at(&proxy_sh(), http_port)
}

/// Same as [`enable`] but writes to an explicit path. Used by tests to avoid
/// depending on (or racing on) the global `XDG_CONFIG_HOME` env var.
pub fn enable_at(path: &Path, http_port: u16) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = format!(
        "# managed by ratc\nexport http_proxy=\"http://127.0.0.1:{p}\"\nexport https_proxy=\"http://127.0.0.1:{p}\"\nexport HTTP_PROXY=\"http://127.0.0.1:{p}\"\nexport HTTPS_PROXY=\"http://127.0.0.1:{p}\"\nexport no_proxy=\"{n}\"\nexport NO_PROXY=\"{n}\"\n",
        p = http_port,
        n = no_proxy()
    );
    std::fs::write(path, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).ok();
    }
    Ok(())
}

/// Remove proxy env vars (write unset snippet so a sourced file cleans up) to
/// the canonical `~/.config/ratc/proxy.sh` path.
pub fn disable() -> Result<()> {
    disable_at(&proxy_sh())
}

/// Same as [`disable`] but writes to an explicit path. Used by tests.
pub fn disable_at(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = "# managed by ratc\nunset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY no_proxy NO_PROXY\n";
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Tests use explicit tempdir paths instead of mutating XDG_CONFIG_HOME, so
    // they are deterministic and safe to run in parallel with other suites.

    #[test]
    fn enable_writes_exports() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("proxy.sh");
        enable_at(&path, 7890).unwrap();
        let t = std::fs::read_to_string(&path).unwrap();
        assert!(t.contains("http_proxy=\"http://127.0.0.1:7890\""));
        assert!(t.contains("no_proxy"));
    }

    #[test]
    fn disable_writes_unset() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("proxy.sh");
        disable_at(&path).unwrap();
        let t = std::fs::read_to_string(&path).unwrap();
        assert!(t.contains("unset http_proxy"));
    }
}
