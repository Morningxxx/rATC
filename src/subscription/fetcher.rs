use crate::error::Result;
use crate::subscription::parser::{parse, ParsedSubscription};
use sha2::{Digest, Sha256};

const UA: &str = "clash.meta";

pub struct Fetcher {
    cache_dir: std::path::PathBuf,
    client: reqwest::blocking::Client,
}

impl Fetcher {
    pub fn new(cache_dir: std::path::PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        let client = reqwest::blocking::Client::builder()
            .user_agent(UA)
            .timeout(std::time::Duration::from_secs(15))
            .no_proxy()
            .build()?;
        Ok(Self { cache_dir, client })
    }

    fn cache_path(&self, url: &str) -> std::path::PathBuf {
        let mut h = Sha256::new();
        h.update(url.as_bytes());
        let hex = format!("{:x}", h.finalize());
        self.cache_dir.join(format!("{hex}.yaml"))
    }

    /// Fetch the raw YAML text, falling back to cache on error.
    pub fn fetch_text(&self, url: &str) -> Result<String> {
        match self.client.get(url).send() {
            Ok(resp) if resp.status().is_success() => {
                let text = resp.text()?;
                let _ = self.write_cache(url, &text);
                Ok(text)
            }
            Ok(resp) => {
                // Network returned non-success; try cache, but surface original status if cache missing.
                self.read_cache(url)
                    .map_err(|e| crate::error::Error::Other(format!(
                        "subscription fetch failed (HTTP {}) and cache unreadable: {}",
                        resp.status(), e
                    )))
            }
            Err(e) => {
                // Network completely failed; try cache.
                self.read_cache(url)
                    .map_err(|cache_err| crate::error::Error::Other(format!(
                        "subscription fetch failed: {e}; cache unreadable: {cache_err}"
                    )))
            }
        }
    }

    pub fn read_cache(&self, url: &str) -> Result<String> {
        let p = self.cache_path(url);
        std::fs::read_to_string(&p).map_err(Into::into)
    }

    /// Write the cache file with 0600 perms — it contains sensitive
    /// credentials (UUIDs/passwords) from the subscription.
    fn write_cache(&self, url: &str, text: &str) -> std::io::Result<()> {
        let p = self.cache_path(url);
        std::fs::write(&p, text)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600)).ok();
        }
        let _ = p;
        Ok(())
    }

    pub fn fetch(&self, url: &str) -> Result<ParsedSubscription> {
        let text = self.fetch_text(url)?;
        parse(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;
    use tempfile::TempDir;

    #[test]
    fn fetch_then_cache_hit() {
        let mut server = mockito::Server::new();
        let body = "proxies: []\nrules: []\n";
        let _m = server.mock("GET", "/")
            .with_status(200)
            .with_body(body)
            .create();
        let tmp = TempDir::new().unwrap();
        let f = Fetcher::new(tmp.path().to_path_buf()).unwrap();
        let first = f.fetch_text(&(server.url() + "/")).unwrap();
        assert_eq!(first, body);
        // cache file exists and equals body
        let cached = f.read_cache(&(server.url() + "/")).unwrap();
        assert_eq!(cached, body);
    }

    #[test]
    fn falls_back_to_cache_on_error() {
        let tmp = TempDir::new().unwrap();
        let f = Fetcher::new(tmp.path().to_path_buf()).unwrap();
        let url = "http://127.0.0.1:1/no-such";
        // seed cache
        std::fs::write(f.cache_path(url), "proxies: []\nrules: []\n").unwrap();
        let text = f.fetch_text(url).unwrap();
        assert!(text.contains("proxies"));
    }
}
