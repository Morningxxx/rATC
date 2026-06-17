use crate::error::Result;
use crate::store::paths::proxy_sh;

fn no_proxy() -> &'static str {
    "localhost,127.0.0.1,::1,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,*.cn"
}

/// Write a shell snippet exporting proxy env vars for the given HTTP port.
pub fn enable(http_port: u16) -> Result<()> {
    let p = proxy_sh();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = format!(
        "# managed by ratc\nexport http_proxy=\"http://127.0.0.1:{p}\"\nexport https_proxy=\"http://127.0.0.1:{p}\"\nexport HTTP_PROXY=\"http://127.0.0.1:{p}\"\nexport HTTPS_PROXY=\"http://127.0.0.1:{p}\"\nexport no_proxy=\"{n}\"\nexport NO_PROXY=\"{n}\"\n",
        p = http_port,
        n = no_proxy()
    );
    std::fs::write(&p, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).ok();
    }
    Ok(())
}

/// Remove proxy env vars (write unset snippet so a sourced file cleans up).
pub fn disable() -> Result<()> {
    let p = proxy_sh();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = "# managed by ratc\nunset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY no_proxy NO_PROXY\n";
    std::fs::write(&p, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn with_tmp_home<F: FnOnce()>(f: F) {
        let tmp = TempDir::new().unwrap();
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        f();
        match prev { Some(v) => std::env::set_var("XDG_CONFIG_HOME", v), None => std::env::remove_var("XDG_CONFIG_HOME") }
    }

    #[test]
    fn enable_writes_exports() {
        with_tmp_home(|| {
            enable(7890).unwrap();
            let t = std::fs::read_to_string(proxy_sh()).unwrap();
            assert!(t.contains("http_proxy=\"http://127.0.0.1:7890\""));
            assert!(t.contains("no_proxy"));
        });
    }

    #[test]
    fn disable_writes_unset() {
        with_tmp_home(|| {
            disable().unwrap();
            let t = std::fs::read_to_string(proxy_sh()).unwrap();
            assert!(t.contains("unset http_proxy"));
        });
    }
}
