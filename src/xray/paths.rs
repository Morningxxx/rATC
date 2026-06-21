use std::path::PathBuf;

/// Resolve xray binary from config path, then PATH.
pub fn resolve(configured: &str) -> Option<PathBuf> {
    let p = PathBuf::from(configured);
    if p.is_file() {
        return Some(p);
    }
    which_xray()
}

fn which_xray() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let f = dir.join("xray");
        if f.is_file() {
            return Some(f);
        }
    }
    None
}
