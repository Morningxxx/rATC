use std::path::PathBuf;

/// Root config dir: ~/.config/ratc (or $XDG_CONFIG_HOME/ratc).
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"))
        .join("ratc")
}

pub fn cache_dir() -> PathBuf {
    config_dir().join("cache")
}
pub fn ruleset_dir() -> PathBuf {
    config_dir().join("ruleset")
}
pub fn logs_dir() -> PathBuf {
    config_dir().join("logs")
}
pub fn config_file() -> PathBuf {
    config_dir().join("config.json")
}
pub fn proxy_sh() -> PathBuf {
    config_dir().join("proxy.sh")
}
pub fn xray_config_file() -> PathBuf {
    config_dir().join("xray.json")
}

/// Ensure all runtime directories exist with secure perms (0700).
pub fn ensure_dirs() -> std::io::Result<()> {
    for d in [config_dir(), cache_dir(), ruleset_dir(), logs_dir()] {
        if !d.exists() {
            std::fs::create_dir_all(&d)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o700))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_under_ratc() {
        let root = config_dir();
        assert!(root.ends_with("ratc"));
        assert!(cache_dir().join("x").starts_with(&root));
    }
}
