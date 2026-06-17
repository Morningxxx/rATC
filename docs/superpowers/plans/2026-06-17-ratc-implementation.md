# rATC Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build rATC, a Linux TUI proxy manager that parses Clash Meta YAML subscriptions, converts nodes/rules to xray-core configs, and manages the xray subprocess with a 5-tab Ratatui interface.

**Architecture:** Single Rust binary. Core modules (`model`, `subscription`, `converter`, `xray`, `store`, `config`, `sysproxy`) hold all logic and are pure/testable; `app` is the state machine; `tui` renders via Ratatui + crossterm. The app spawns xray-core as a child process and generates its JSON config. Synchronous I/O throughout (blocking `reqwest`, `std::process::Command`) — a TUI event loop needs no async runtime, and this keeps deps/compile time minimal.

**Tech Stack:** Rust (edition 2021), ratatui + crossterm, reqwest (blocking, rustls), serde/serde_json/serde_yaml, thiserror, dirs. xray-core 1.8.24 (preinstalled at `/usr/local/bin/xray`).

**Spec:** `docs/superpowers/specs/2026-06-17-ratc-design.md`

**Deviation from spec (justified):** Spec lists `tokio`. We use blocking `reqwest` + `std::process::Command` instead — the crossterm event loop is synchronous, no I/O concurrency is required for MVP, and dropping tokio cuts compile time and complexity. Background work (rule-set refresh, latency test) uses `std::thread`.

---

## File Structure

```
rATC/
├── Cargo.toml
├── README.md
├── docs/MANUAL.md
├── src/
│   ├── main.rs                    # entry: init terminal, run app, restore terminal
│   ├── app.rs                     # App state machine: holds model, coordinates components
│   ├── error.rs                   # Error enum (thiserror)
│   ├── config.rs                  # AppConfig (serde JSON) — ports, xray path, subs, current node
│   ├── model/
│   │   ├── mod.rs                 # re-exports
│   │   ├── proxy.rs               # Proxy, ProxyType, RealityOpts, WsOpts, PluginOpts, Compat
│   │   ├── proxy_group.rs         # ProxyGroup, GroupType, InfoGroup
│   │   ├── rule.rs                # Rule, RuleKind, Target
│   │   ├── rule_provider.rs       # RuleProvider
│   │   └── clash_config.rs        # ClashConfig (top-level parsed subscription)
│   ├── subscription/
│   │   ├── mod.rs
│   │   ├── parser.rs              # &str YAML -> ClashConfig
│   │   └── fetcher.rs             # HTTP GET + disk cache
│   ├── converter/
│   │   ├── mod.rs
│   │   ├── proxy_converter.rs     # Proxy -> xray outbound serde_json::Value
│   │   ├── rule_converter.rs      # Rule -> xray routing rule Value (returns Skip for unsupported)
│   │   ├── ruleset_expander.rs    # download + parse rule-set payloads, expand
│   │   ├── fallback.rs            # minimal fallback routing rules
│   │   └── xray_config.rs         # assemble full xray config Value
│   ├── xray/
│   │   ├── mod.rs
│   │   ├── paths.rs               # locate xray binary + datadir
│   │   └── process.rs             # XrayHandle: spawn/stop/reload, log capture
│   ├── store/
│   │   ├── mod.rs                 # config dir layout, file perms
│   │   └── paths.rs               # path helpers (~/.config/ratc/...)
│   ├── sysproxy/
│   │   └── mod.rs                 # write/clear proxy.sh, gsettings toggle
│   └── tui/
│       ├── mod.rs                 # run() terminal loop
│       ├── event.rs               # poll crossterm events -> Message
│       ├── app_state.rs           # UiState: active tab, selection indices
│       └── tabs/
│           ├── mod.rs
│           ├── nodes.rs
│           ├── subscriptions.rs
│           ├── rules.rs
│           ├── logs.rs
│           └── settings.rs
└── tests/
    ├── fixtures/
    │   └── clash_meta.yaml        # desensitized real subscription
    ├── converter.rs               # integration tests for converter
    └── xray_config.rs             # integration test: xray -test on generated config
```

**Dependency order:** `model` ← `subscription` ← `converter` ← `xray` ← `app` ← `tui`. `store`/`config`/`sysproxy` are leaf modules used by `app`.

---

## Task 1: Environment setup & project scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

- [ ] **Step 1: Install Rust toolchain**

Run:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
source "$HOME/.cargo/env"
rustc --version
```
Expected: `rustc 1.8x.x` version line.

- [ ] **Step 2: Create `Cargo.toml`**

```toml
[package]
name = "ratc"
version = "0.1.0"
edition = "2021"
description = "Rust Agent for Terminal Clash — a Linux TUI proxy manager"
license = "MIT"

[dependencies]
ratatui = "0.28"
crossterm = "0.28"
reqwest = { version = "0.12", features = ["blocking", "rustls-tls"], default-features = false }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
thiserror = "1"
anyhow = "1"
dirs = "5"
sha2 = "0.10"
base64 = "0.22"

[dev-dependencies]
mockito = "1"
tempfile = "3"
```

- [ ] **Step 3: Create minimal `src/main.rs`**

```rust
fn main() {
    println!("rATC scaffold ok");
}
```

- [ ] **Step 4: Verify it builds and runs**

Run: `cargo run`
Expected: prints `rATC scaffold ok`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs
git commit -m "chore: project scaffold"
```

---

## Task 2: `error.rs` — typed errors

**Files:**
- Create: `src/error.rs`
- Modify: `src/main.rs` (add `mod error;`)

- [ ] **Step 1: Write `src/error.rs`**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("subscription parse error: {0}")]
    Parse(String),
    #[error("xray error: {0}")]
    Xray(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

- [ ] **Step 2: Wire into `src/main.rs`**

Replace `src/main.rs` contents:
```rust
mod error;

fn main() -> error::Result<()> {
    println!("rATC scaffold ok");
    Ok(())
}
```

- [ ] **Step 3: Verify build**

Run: `cargo build`
Expected: compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add src/error.rs src/main.rs
git commit -m "feat: typed error module"
```

---

## Task 3: `model::proxy` — proxy types and compat

**Files:**
- Create: `src/model/mod.rs`
- Create: `src/model/proxy.rs`
- Modify: `src/main.rs` (add `mod model;`)

- [ ] **Step 1: Write the failing test in `src/model/proxy.rs`**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compat {
    /// Fully supported by xray-core.
    Supported,
    /// Not supported; must be skipped during conversion.
    Unsupported(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyType {
    Vless {
        uuid: String,
        network: String,
        tls: bool,
        servername: Option<String>,
        flow: Option<String>,
        reality: Option<RealityOpts>,
        ws: Option<WsOpts>,
    },
    Vmess {
        uuid: String,
        alter_id: u32,
        cipher: String,
        network: String,
        tls: bool,
        servername: Option<String>,
        ws: Option<WsOpts>,
    },
    Shadowsocks {
        password: String,
        cipher: String,
        plugin: Option<PluginOpts>,
    },
    Trojan {
        password: String,
        sni: Option<String>,
    },
    /// Catch-all for protocols we store but cannot convert (e.g. hysteria2).
    Unsupported {
        kind: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RealityOpts {
    #[serde(rename = "public-key")]
    pub public_key: String,
    #[serde(rename = "short-id", default)]
    pub short_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WsOpts {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PluginOpts {
    #[serde(rename = "plugin-opts")]
    pub plugin_opts: serde_yaml::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub fields: serde_yaml::Mapping,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub ptype: ProxyType,
}

impl Proxy {
    pub fn compat(&self) -> Compat {
        match &self.ptype {
            ProxyType::Unsupported { .. } => Compat::Unsupported("protocol not supported by xray"),
            ProxyType::Shadowsocks { plugin: Some(_), .. } => {
                Compat::Unsupported("shadowsocks plugins (shadow-tls) not supported by xray")
            }
            _ => Compat::Supported,
        }
    }
}

pub fn classify(raw: &RawProxy) -> Proxy {
    use serde_yaml::Value;
    let g = |k: &str| raw.fields.get(&Value::String(k.into())).cloned();
    let gs = |k: &str| g(k).and_then(|v| v.as_str().map(String::from));
    let gn = |k: &str| g(k).and_then(|v| v.as_u64()).map(|n| n as u32);
    let gb = |k: &str| g(k).and_then(|v| v.as_bool()).unwrap_or(false);
    let ptype = match raw.kind.as_str() {
        "vless" => ProxyType::Vless {
            uuid: gs("uuid").unwrap_or_default(),
            network: gs("network").unwrap_or_else(|| "tcp".into()),
            tls: gb("tls"),
            servername: gs("servername"),
            flow: gs("flow"),
            reality: g("reality-opts").and_then(|v| serde_yaml::from_value(v).ok()),
            ws: g("ws-opts").and_then(|v| serde_yaml::from_value(v).ok()),
        },
        "vmess" => ProxyType::Vmess {
            uuid: gs("uuid").unwrap_or_default(),
            alter_id: gn("alterId").unwrap_or(0),
            cipher: gs("cipher").unwrap_or_else(|| "none".into()),
            network: gs("network").unwrap_or_else(|| "tcp".into()),
            tls: gb("tls"),
            servername: gs("servername"),
            ws: g("ws-opts").and_then(|v| serde_yaml::from_value(v).ok()),
        },
        "ss" => ProxyType::Shadowsocks {
            password: gs("password").unwrap_or_default(),
            cipher: gs("cipher").unwrap_or_default(),
            plugin: g("plugin").and_then(|v| serde_yaml::from_value(serde_yaml::Value::Mapping(
                serde_yaml::Mapping::new(),
            )).ok()).and_then(|_| raw.fields.get(&Value::String("plugin".into())).map(|_| PluginOpts {
                plugin_opts: g("plugin-opts").unwrap_or(Value::Null),
            })),
        },
        "trojan" => ProxyType::Trojan {
            password: gs("password").unwrap_or_default(),
            sni: gs("sni"),
        },
        other => ProxyType::Unsupported { kind: other.into() },
    };
    Proxy { name: raw.name.clone(), server: raw.server.clone(), port: raw.port, ptype }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_vless_reality_is_supported() {
        let yaml = "name: hk1\nserver: 1.2.3.4\nport: 443\ntype: vless\nuuid: abc\nnetwork: tcp\ntls: true\nservername: azure.com\nreality-opts:\n  public-key: PK\n";
        let raw: RawProxy = serde_yaml::from_str(yaml).unwrap();
        let p = classify(&raw);
        assert_eq!(p.compat(), Compat::Supported);
        assert!(matches!(p.ptype, ProxyType::Vless { .. }));
    }

    #[test]
    fn classify_ss_with_plugin_is_unsupported() {
        let yaml = "name: s1\nserver: 1.2.3.4\nport: 443\ntype: ss\npassword: p\ncipher: 2022-blake3-aes-256-gcm\nplugin: shadow-tls\nplugin-opts:\n  host: h\n";
        let raw: RawProxy = serde_yaml::from_str(yaml).unwrap();
        let p = classify(&raw);
        assert!(matches!(p.compat(), Compat::Unsupported(_)));
    }

    #[test]
    fn classify_hysteria2_is_unsupported() {
        let yaml = "name: h1\nserver: 1.2.3.4\nport: 443\ntype: hysteria2\npassword: p\n";
        let raw: RawProxy = serde_yaml::from_str(yaml).unwrap();
        let p = classify(&raw);
        assert!(matches!(p.ptype, ProxyType::Unsupported { .. }));
        assert!(matches!(p.compat(), Compat::Unsupported(_)));
    }
}
```

- [ ] **Step 2: Create `src/model/mod.rs`**

```rust
pub mod proxy;
```

- [ ] **Step 3: Wire into `src/main.rs`**

Add after `mod error;`:
```rust
mod model;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test model::proxy`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add src/model src/main.rs
git commit -m "feat(model): proxy types with compat classification"
```

---

## Task 4: `model::proxy_group` — groups and info entries

**Files:**
- Create: `src/model/proxy_group.rs`
- Modify: `src/model/mod.rs`

- [ ] **Step 1: Write `src/model/proxy_group.rs` with tests**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum GroupType {
    #[serde(rename = "select")]
    Select,
    #[serde(rename = "url-test")]
    UrlTest,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProxyGroup {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: GroupType,
    #[serde(default)]
    pub proxies: Vec<String>,
}

/// Non-proxy informational "groups" (traffic, expiry, package) shown in status bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoEntry {
    pub label: String,
}

impl ProxyGroup {
    /// Heuristic: a select group whose only proxy is `DIRECT` and whose name
    /// carries an info keyword is treated as an info entry, not a real group.
    pub fn as_info(&self) -> Option<InfoEntry> {
        let n = self.name.as_str();
        let is_info = ["流量", "到期", "套餐", "客服", "续费"].iter().any(|k| n.contains(k));
        if matches!(self.kind, GroupType::Select)
            && self.proxies.iter().all(|p| p == "DIRECT")
            && is_info
        {
            Some(InfoEntry { label: n.into() })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_select_group_is_not_info() {
        let g = ProxyGroup {
            name: "🛸 节点选择".into(),
            kind: GroupType::Select,
            proxies: vec!["DIRECT".into(), "a".into()],
        };
        assert!(g.as_info().is_none());
    }

    #[test]
    fn traffic_group_is_info() {
        let g = ProxyGroup {
            name: "⛽ 本月流量 71694MB".into(),
            kind: GroupType::Select,
            proxies: vec!["DIRECT".into()],
        };
        assert!(g.as_info().is_some());
    }
}
```

- [ ] **Step 2: Update `src/model/mod.rs`**

```rust
pub mod proxy;
pub mod proxy_group;
```

- [ ] **Step 3: Run tests**

Run: `cargo test model::proxy_group`
Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add src/model/proxy_group.rs src/model/mod.rs
git commit -m "feat(model): proxy groups and info entries"
```

---

## Task 5: `model::rule` — routing rules

**Files:**
- Create: `src/model/rule.rs`
- Modify: `src/model/mod.rs`

- [ ] **Step 1: Write `src/model/rule.rs` with tests**

```rust
/// A Clash rule parsed from a `rules:` line or a rule-set payload line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rule {
    Domain(String, Target),
    DomainSuffix(String, Target),
    DomainKeyword(String, Target),
    IpCidr(String, Target, bool), // bool = no-resolve
    GeoIp(String, Target),
    RuleSet(String, Target),
    Match(Target),
    /// Unsupported rule kind (e.g. PROCESS-NAME). Original text kept for logging.
    Unsupported(String, Target),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Proxy,
    Direct,
    Reject,
    /// A named proxy-group target; resolved to Proxy/Direct/Reject by app.
    Group,
}

impl Rule {
    /// Parse a single Clash rule line, e.g. `DOMAIN-SUFFIX,cn,DIRECT`.
    /// `group_names` is the set of known group names (used to classify Target).
    pub fn parse(line: &str, group_names: &std::collections::HashSet<String>) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let parts: Vec<&str> = line.split(',').map(|p| p.trim()).collect();
        let target_of = |name: &str| -> Target {
            match name {
                "DIRECT" => Target::Direct,
                "REJECT" => Target::Reject,
                "PROXY" => Target::Proxy,
                other if group_names.contains(other) => Target::Group,
                _ => Target::Group, // unknown name → treat as group reference
            }
        };
        Some(match parts[0] {
            "DOMAIN" => Rule::Domain(parts.get(1)?.to_string(), target_of(parts.get(2)?)),
            "DOMAIN-SUFFIX" => Rule::DomainSuffix(parts.get(1)?.to_string(), target_of(parts.get(2)?)),
            "DOMAIN-KEYWORD" => Rule::DomainKeyword(parts.get(1)?.to_string(), target_of(parts.get(2)?)),
            "IP-CIDR" | "IP-CIDR6" => Rule::IpCidr(
                parts.get(1)?.to_string(),
                target_of(parts.get(2)?),
                parts.get(3).map(|p| *p == "no-resolve").unwrap_or(false),
            ),
            "GEOIP" => Rule::GeoIp(parts.get(1)?.to_lowercase(), target_of(parts.get(2)?)),
            "RULE-SET" => Rule::RuleSet(parts.get(1)?.to_string(), target_of(parts.get(2)?)),
            "MATCH" => Rule::Match(target_of(parts.get(1).unwrap_or(&"PROXY"))),
            other => Rule::Unsupported(line.to_string(), target_of(parts.get(1).unwrap_or(&"PROXY"))),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn groups() -> HashSet<String> {
        ["🛸 节点选择"].iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_domain_suffix_direct() {
        let r = Rule::parse("DOMAIN-SUFFIX,cn,DIRECT", &groups()).unwrap();
        assert_eq!(r, Rule::DomainSuffix("cn".into(), Target::Direct));
    }

    #[test]
    fn parse_ip_cidr_no_resolve() {
        let r = Rule::parse("IP-CIDR,10.0.0.0/8,DIRECT,no-resolve", &groups()).unwrap();
        assert_eq!(r, Rule::IpCidr("10.0.0.0/8".into(), Target::Direct, true));
    }

    #[test]
    fn parse_match_group() {
        let r = Rule::parse("MATCH,🛸 节点选择", &groups()).unwrap();
        assert_eq!(r, Rule::Match(Target::Group));
    }

    #[test]
    fn parse_process_name_unsupported() {
        let r = Rule::parse("PROCESS-NAME,OneDrive,DIRECT", &groups()).unwrap();
        assert!(matches!(r, Rule::Unsupported(_, Target::Direct)));
    }

    #[test]
    fn skip_comments_and_blank() {
        let g = groups();
        assert!(Rule::parse("# comment", &g).is_none());
        assert!(Rule::parse("", &g).is_none());
    }
}
```

- [ ] **Step 2: Update `src/model/mod.rs`**

```rust
pub mod proxy;
pub mod proxy_group;
pub mod rule;
```

- [ ] **Step 3: Run tests**

Run: `cargo test model::rule`
Expected: 5 passed.

- [ ] **Step 4: Commit**

```bash
git add src/model/rule.rs src/model/mod.rs
git commit -m "feat(model): routing rule parser"
```

---

## Task 6: `model::rule_provider` and `model::clash_config`

**Files:**
- Create: `src/model/rule_provider.rs`
- Create: `src/model/clash_config.rs`
- Modify: `src/model/mod.rs`

- [ ] **Step 1: Write `src/model/rule_provider.rs`**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RuleProvider {
    pub behavior: String,
    #[serde(rename = "type")]
    pub kind: String, // "http" | "file"
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub interval: u64,
}

/// A downloaded rule-set file. Clash classical rule-sets have a `payload:` list
/// of rule lines; domain/ip rule-sets have `payload:` list of plain values.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RuleSetFile {
    #[serde(default)]
    pub payload: Vec<String>,
}
```

- [ ] **Step 2: Write `src/model/clash_config.rs`**

```rust
use serde::Deserialize;
use crate::model::proxy::RawProxy;
use crate::model::proxy_group::ProxyGroup;
use crate::model::rule_provider::RuleProvider;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct ClashConfig {
    #[serde(default)]
    pub port: u16,
    #[serde(default, rename = "socks-port")]
    pub socks_port: u16,
    #[serde(default)]
    pub proxies: Vec<RawProxy>,
    #[serde(default, rename = "proxy-groups")]
    pub proxy_groups: Vec<ProxyGroup>,
    #[serde(default, rename = "rule-providers")]
    pub rule_providers: HashMap<String, RuleProvider>,
    #[serde(default)]
    pub rules: Vec<String>,
}
```

- [ ] **Step 3: Update `src/model/mod.rs`**

```rust
pub mod proxy;
pub mod proxy_group;
pub mod rule;
pub mod rule_provider;
pub mod clash_config;
```

- [ ] **Step 4: Verify build**

Run: `cargo build`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add src/model/rule_provider.rs src/model/clash_config.rs src/model/mod.rs
git commit -m "feat(model): rule providers and top-level clash config"
```

---

## Task 7: Test fixture — desensitized subscription

**Files:**
- Create: `tests/fixtures/clash_meta.yaml`

- [ ] **Step 1: Write the fixture**

```yaml
port: 7890
socks-port: 7891
mode: Rule
log-level: error

proxies:
- {name: "US-Xr1", type: vless, server: 10.0.0.1, port: 443, uuid: 00000000-0000-0000-0000-000000000000, network: tcp, tls: true, servername: azure.microsoft.com, reality-opts: {public-key: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA}}
- {name: "HK-2022tls", type: ss, server: 10.0.0.2, port: 443, password: fakepass1=:fakepass2=, cipher: 2022-blake3-aes-256-gcm, plugin: shadow-tls, plugin-opts: {host: example.com, password: "10086", version: 3}}
- {name: "HK-Hy2", type: hysteria2, server: 10.0.0.3, port: 443, password: fakepass, sni: example.com}
- {name: "JP-Ws", type: vmess, server: 10.0.0.4, port: 444, uuid: 00000000-0000-0000-0000-000000000000, alterId: 0, cipher: none, network: ws, tls: true, servername: cdn.example.com, ws-opts: {path: /, headers: {Host: jp.example.com}}}
- {name: "TW-Trojan", type: trojan, server: 10.0.0.5, port: 443, password: fakepass, sni: example.com}

proxy-groups:
- name: "🛸 节点选择"
  type: select
  proxies: ["US-Xr1", "HK-2022tls", "HK-Hy2", "JP-Ws", "TW-Trojan"]
- name: "⛽ 本月流量 71694MB"
  type: select
  proxies: [DIRECT]
- name: "📅 到期时间 2026-09-26"
  type: select
  proxies: [DIRECT]

rule-providers:
  CNdirect1:
    behavior: classical
    type: http
    url: "https://example.invalid/CNdirect1.yaml"
    interval: 86400
    path: ./CNdirect1.yaml

rules:
- DOMAIN-SUFFIX,cn,DIRECT
- DOMAIN-KEYWORD,google,🛸 节点选择
- IP-CIDR,10.0.0.0/8,DIRECT,no-resolve
- PROCESS-NAME,OneDrive,DIRECT
- RULE-SET,CNdirect1,DIRECT
- MATCH,🛸 节点选择
```

- [ ] **Step 2: Commit**

```bash
git add tests/fixtures/clash_meta.yaml
git commit -m "test: desensitized clash meta fixture"
```

---

## Task 8: `subscription::parser` — YAML → model

**Files:**
- Create: `src/subscription/mod.rs`
- Create: `src/subscription/parser.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write `src/subscription/parser.rs` with tests**

```rust
use crate::error::{Error, Result};
use crate::model::clash_config::ClashConfig;
use crate::model::proxy::{classify, Proxy};
use crate::model::proxy_group::ProxyGroup;
use crate::model::rule::{Rule, Target};
use crate::model::rule_provider::RuleProvider;
use std::collections::{HashMap, HashSet};

/// Fully parsed subscription: the raw clash config plus derived structures.
#[derive(Debug, Clone)]
pub struct ParsedSubscription {
    pub http_port: u16,
    pub socks_port: u16,
    pub proxies: Vec<Proxy>,
    pub groups: Vec<ProxyGroup>,
    pub rule_providers: HashMap<String, RuleProvider>,
    pub rules: Vec<Rule>,
    pub group_names: HashSet<String>,
}

pub fn parse(text: &str) -> Result<ParsedSubscription> {
    let cfg: ClashConfig = serde_yaml::from_str(text)
        .map_err(|e| Error::Parse(format!("yaml: {e}")))?;
    let group_names: HashSet<String> = cfg.proxy_groups.iter().map(|g| g.name.clone()).collect();
    let proxies: Vec<Proxy> = cfg.proxies.iter().map(classify).collect();
    let rules: Vec<Rule> = cfg.rules.iter()
        .filter_map(|line| Rule::parse(line, &group_names))
        .collect();
    Ok(ParsedSubscription {
        http_port: cfg.port,
        socks_port: cfg.socks_port,
        proxies,
        groups: cfg.proxy_groups,
        rule_providers: cfg.rule_providers,
        rules,
        group_names,
    })
}

/// Resolve a group-targeted rule's effective target once the active outbound tag is known.
/// Returns "proxy" for Group, "direct" for Direct, "block" for Reject, "proxy" for Proxy.
pub fn target_tag(t: Target) -> &'static str {
    match t {
        Target::Direct => "direct",
        Target::Reject => "block",
        Target::Proxy | Target::Group => "proxy",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::proxy::Compat;

    fn fixture() -> String {
        std::fs::read_to_string("../tests/fixtures/clash_meta.yaml")
            .or_else(|_| std::fs::read_to_string("tests/fixtures/clash_meta.yaml"))
            .unwrap()
    }

    #[test]
    fn parses_fixture() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.proxies.len(), 5);
        assert!(p.http_port == 7890);
        assert!(p.socks_port == 7891);
    }

    #[test]
    fn compat_counts() {
        let p = parse(&fixture()).unwrap();
        let supported = p.proxies.iter().filter(|x| x.compat() == Compat::Supported).count();
        // US-Xr1(vless), JP-Ws(vmess), TW-Trojan(trojan) → 3 supported
        assert_eq!(supported, 3);
    }

    #[test]
    fn rules_parsed() {
        let p = parse(&fixture()).unwrap();
        assert!(p.rules.iter().any(|r| matches!(r, Rule::DomainSuffix(d, Target::Direct) if d=="cn")));
        assert!(p.rules.iter().any(|r| matches!(r, Rule::RuleSet(n, Target::Direct) if n=="CNdirect1")));
        assert!(p.rules.iter().any(|r| matches!(r, Rule::Unsupported(_, _))));
    }
}
```

- [ ] **Step 2: Write `src/subscription/mod.rs`**

```rust
pub mod parser;
```

- [ ] **Step 3: Wire into `src/main.rs`**

Add: `mod subscription;`

- [ ] **Step 4: Run tests**

Run: `cargo test subscription::parser`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add src/subscription src/main.rs
git commit -m "feat(subscription): clash YAML parser"
```

---

## Task 9: `subscription::fetcher` — HTTP + cache

**Files:**
- Create: `src/subscription/fetcher.rs`
- Modify: `src/subscription/mod.rs`

- [ ] **Step 1: Write `src/subscription/fetcher.rs` with tests**

```rust
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
                let _ = std::fs::write(self.cache_path(url), &text);
                Ok(text)
            }
            _ => self.read_cache(url),
        }
    }

    pub fn read_cache(&self, url: &str) -> Result<String> {
        let p = self.cache_path(url);
        std::fs::read_to_string(&p).map_err(Into::into)
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
        let server = mockito::Server::new();
        let body = "proxies: []\nrules: []\n";
        let _m = server.mock("GET", "/")
            .with_status(200)
            .with_body(body)
            .create();
        let tmp = TempDir::new().unwrap();
        let f = Fetcher::new(tmp.path().to_path_buf()).unwrap();
        let first = f.fetch_text(&server.url() + "/").unwrap();
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
```

- [ ] **Step 2: Update `src/subscription/mod.rs`**

```rust
pub mod fetcher;
pub mod parser;
```

- [ ] **Step 3: Run tests**

Run: `cargo test subscription::fetcher`
Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add src/subscription/fetcher.rs src/subscription/mod.rs
git commit -m "feat(subscription): HTTP fetcher with disk cache"
```

---

## Task 10: `converter::proxy_converter` — Proxy → xray outbound

**Files:**
- Create: `src/converter/mod.rs`
- Create: `src/converter/proxy_converter.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write `src/converter/proxy_converter.rs` with tests**

```rust
use crate::error::{Error, Result};
use crate::model::proxy::{Compat, Proxy, ProxyType};
use serde_json::{json, Value};

/// Convert a supported proxy into an xray outbound JSON object.
/// Returns `Ok(None)` if the proxy is unsupported (caller skips it).
pub fn to_outbound(p: &Proxy) -> Result<Option<Value>> {
    if !matches!(p.compat(), Compat::Supported) {
        return Ok(None);
    }
    let v = match &p.ptype {
        ProxyType::Vless { uuid, network, tls, servername, flow, reality, ws } => {
            let mut stream = json!({});
            if *tls {
                stream["security"] = if reality.is_some() { json!("reality") } else { json!("tls") };
                if let Some(sn) = servername { stream["tlsSettings"] = json!({"serverName": sn}); }
                if let Some(r) = reality {
                    stream["realitySettings"] = json!({
                        "serverName": servername,
                        "fingerprint": "safari",
                        "publicKey": r.public_key,
                        "shortId": r.short_id.clone().unwrap_or_default()
                    });
                }
            }
            if *network == "ws" {
                if let Some(w) = ws {
                    let mut ws_settings = json!({});
                    if let Some(path) = &w.path { ws_settings["path"] = json!(path); }
                    if let Some(h) = &w.headers { ws_settings["headers"] = json!(h); }
                    stream["wsSettings"] = ws_settings;
                }
            }
            json!({
                "tag": "proxy",
                "protocol": "vless",
                "settings": { "vnext": [{ "address": p.server, "port": p.port, "users": [{
                    "id": uuid, "encryption": "none", "flow": flow.clone().unwrap_or_default()
                }]}]},
                "streamSettings": stream
            })
        }
        ProxyType::Vmess { uuid, alter_id, cipher, network, tls, servername, ws } => {
            let mut stream = json!({"network": network});
            if *tls {
                stream["security"] = json!("tls");
                if let Some(sn) = servername { stream["tlsSettings"] = json!({"serverName": sn, "allowInsecure": true}); }
            }
            if *network == "ws" {
                if let Some(w) = ws {
                    let mut ws_settings = json!({});
                    if let Some(path) = &w.path { ws_settings["path"] = json!(path); }
                    if let Some(h) = &w.headers { ws_settings["headers"] = json!(h); }
                    stream["wsSettings"] = ws_settings;
                }
            }
            json!({
                "tag": "proxy",
                "protocol": "vmess",
                "settings": { "vnext": [{ "address": p.server, "port": p.port, "users": [{
                    "id": uuid, "alterId": alter_id, "security": cipher
                }]}]},
                "streamSettings": stream
            })
        }
        ProxyType::Shadowsocks { password, cipher, plugin: None } => {
            json!({
                "tag": "proxy",
                "protocol": "shadowsocks",
                "settings": { "servers": [{ "address": p.server, "port": p.port, "method": cipher, "password": password }]}
            })
        }
        ProxyType::Trojan { password, sni } => {
            let mut stream = json!({"security": "tls"});
            if let Some(s) = sni { stream["tlsSettings"] = json!({"serverName": s}); }
            json!({
                "tag": "proxy",
                "protocol": "trojan",
                "settings": { "servers": [{ "address": p.server, "port": p.port, "password": password }]},
                "streamSettings": stream
            })
        }
        ProxyType::Shadowsocks { plugin: Some(_), .. } | ProxyType::Unsupported { .. } => {
            return Ok(None);
        }
    };
    // sanity: tag must be proxy
    if v.get("tag").and_then(|t| t.as_str()) != Some("proxy") {
        return Err(Error::Other("outbound tag mismatch".into()));
    }
    Ok(Some(v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::proxy::{classify, RawProxy};

    fn proxy(yaml: &str) -> Proxy {
        let raw: RawProxy = serde_yaml::from_str(yaml).unwrap();
        classify(&raw)
    }

    #[test]
    fn vless_reality_outbound() {
        let p = proxy("name: a\nserver: 1.1.1.1\nport: 443\ntype: vless\nuuid: U\nnetwork: tcp\ntls: true\nservername: s.com\nreality-opts:\n  public-key: PK\n");
        let v = to_outbound(&p).unwrap().unwrap();
        assert_eq!(v["protocol"], "vless");
        assert_eq!(v["streamSettings"]["security"], "reality");
        assert_eq!(v["streamSettings"]["realitySettings"]["publicKey"], "PK");
        assert_eq!(v["tag"], "proxy");
    }

    #[test]
    fn vmess_ws_outbound() {
        let p = proxy("name: a\nserver: 1.1.1.1\nport: 444\ntype: vmess\nuuid: U\nnetwork: ws\ntls: true\nservername: s.com\nws-opts:\n  path: /\n  headers:\n    Host: h.com\n");
        let v = to_outbound(&p).unwrap().unwrap();
        assert_eq!(v["protocol"], "vmess");
        assert_eq!(v["streamSettings"]["network"], "ws");
        assert_eq!(v["streamSettings"]["wsSettings"]["headers"]["Host"], "h.com");
    }

    #[test]
    fn trojan_outbound() {
        let p = proxy("name: a\nserver: 1.1.1.1\nport: 443\ntype: trojan\npassword: P\nsni: s.com\n");
        let v = to_outbound(&p).unwrap().unwrap();
        assert_eq!(v["protocol"], "trojan");
        assert_eq!(v["streamSettings"]["tlsSettings"]["serverName"], "s.com");
    }

    #[test]
    fn unsupported_returns_none() {
        let p = proxy("name: a\nserver: 1.1.1.1\nport: 443\ntype: hysteria2\npassword: P\n");
        assert!(to_outbound(&p).unwrap().is_none());
    }
}
```

- [ ] **Step 2: Write `src/converter/mod.rs`**

```rust
pub mod proxy_converter;
```

- [ ] **Step 3: Wire into `src/main.rs`**

Add: `mod converter;`

- [ ] **Step 4: Run tests**

Run: `cargo test converter::proxy_converter`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add src/converter src/main.rs
git commit -m "feat(converter): proxy -> xray outbound"
```

---

## Task 11: `converter::rule_converter` — rules → xray routing

**Files:**
- Create: `src/converter/rule_converter.rs`
- Modify: `src/converter/mod.rs`

- [ ] **Step 1: Write `src/converter/rule_converter.rs` with tests**

```rust
use crate::model::rule::{Rule, Target};
use crate::subscription::parser::target_tag;
use serde_json::{json, Value};

/// Outcome of converting one rule.
pub enum Converted {
    /// A usable xray routing rule object.
    Rule(Value),
    /// Inlined RULE-SET marker — must be expanded by the caller.
    RuleSet(String, Target),
    /// Fallback MATCH rule (always last).
    Match(Value),
    /// Skipped (unsupported). Original text kept for stats/logging.
    Skipped(String),
}

pub fn convert(rule: &Rule) -> Converted {
    match rule {
        Rule::Domain(v, t) => Converted::Rule(json!({"domain": [v], "outboundTag": target_tag(*t)})),
        Rule::DomainSuffix(v, t) => Converted::Rule(json!({"domainSuffix": [v], "outboundTag": target_tag(*t)})),
        Rule::DomainKeyword(v, t) => Converted::Rule(json!({"domainKeyword": [v], "outboundTag": target_tag(*t)})),
        Rule::IpCidr(v, t, _) => Converted::Rule(json!({"ipCidr": [v], "outboundTag": target_tag(*t)})),
        Rule::GeoIp(v, t) => Converted::Rule(json!({"geoIp": v, "outboundTag": target_tag(*t)})),
        Rule::RuleSet(name, t) => Converted::RuleSet(name.clone(), *t),
        Rule::Match(t) => Converted::Match(json!({"type": "field", "outboundTag": target_tag(*t)})),
        Rule::Unsupported(text, _) => Converted::Skipped(text.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::rule::Rule;
    use std::collections::HashSet;

    #[test]
    fn domain_suffix_to_xray() {
        let r = Rule::parse("DOMAIN-SUFFIX,cn,DIRECT", &HashSet::new()).unwrap();
        if let Converted::Rule(v) = convert(&r) {
            assert_eq!(v["domainSuffix"][0], "cn");
            assert_eq!(v["outboundTag"], "direct");
        } else { panic!(); }
    }

    #[test]
    fn ip_cidr_to_xray() {
        let r = Rule::parse("IP-CIDR,10.0.0.0/8,DIRECT,no-resolve", &HashSet::new()).unwrap();
        if let Converted::Rule(v) = convert(&r) {
            assert_eq!(v["ipCidr"][0], "10.0.0.0/8");
        } else { panic!(); }
    }

    #[test]
    fn match_to_xray() {
        let r = Rule::parse("MATCH,PROXY", &HashSet::new()).unwrap();
        assert!(matches!(convert(&r), Converted::Match(_)));
    }

    #[test]
    fn ruleset_marker() {
        let r = Rule::parse("RULE-SET,foo,DIRECT", &HashSet::new()).unwrap();
        assert!(matches!(convert(&r), Converted::RuleSet(_, _)));
    }

    #[test]
    fn unsupported_skipped() {
        let r = Rule::parse("PROCESS-NAME,OneDrive,DIRECT", &HashSet::new()).unwrap();
        assert!(matches!(convert(&r), Converted::Skipped(_)));
    }
}
```

- [ ] **Step 2: Update `src/converter/mod.rs`**

```rust
pub mod proxy_converter;
pub mod rule_converter;
```

- [ ] **Step 3: Run tests**

Run: `cargo test converter::rule_converter`
Expected: 5 passed.

- [ ] **Step 4: Commit**

```bash
git add src/converter/rule_converter.rs src/converter/mod.rs
git commit -m "feat(converter): rule -> xray routing rule"
```

---

## Task 12: `converter::ruleset_expander` — download & expand RULE-SET

**Files:**
- Create: `src/converter/ruleset_expander.rs`
- Modify: `src/converter/mod.rs`

- [ ] **Step 1: Write `src/converter/ruleset_expander.rs` with tests**

```rust
use crate::error::Result;
use crate::model::rule::{Rule, Target};
use crate::model::rule_provider::RuleProvider;
use crate::model::rule_set_file;
use crate::subscription::parser::target_tag;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

/// Download (or read cached) classical rule-set files and convert each payload
/// line into an xray routing rule, inlined at the position of the RULE-SET entry.
pub struct RuleSetExpander {
    client: reqwest::blocking::Client,
}

impl RuleSetExpander {
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("clash.meta")
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        Ok(Self { client })
    }

    pub fn fetch_payload(&self, url: &str) -> Result<Vec<String>> {
        let resp = self.client.get(url).send()?;
        let text = resp.text()?;
        let file: rule_set_file::RuleSetFile = serde_yaml::from_str(&text)?;
        Ok(file.payload)
    }

    /// Convert a classical rule-set's payload lines into xray rules with the
    /// given target. Lines that fail to parse are skipped.
    pub fn expand(lines: &[String], target: Target, group_names: &HashSet<String>) -> Vec<Value> {
        let tag = target_tag(target);
        lines.iter()
            .filter_map(|l| Rule::parse(l, group_names))
            .filter_map(|r| match r {
                Rule::Domain(v, _) => Some(json!({"domain": [v], "outboundTag": tag})),
                Rule::DomainSuffix(v, _) => Some(json!({"domainSuffix": [v], "outboundTag": tag})),
                Rule::DomainKeyword(v, _) => Some(json!({"domainKeyword": [v], "outboundTag": tag})),
                Rule::IpCidr(v, _, _) => Some(json!({"ipCidr": [v], "outboundTag": tag})),
                Rule::GeoIp(v, _) => Some(json!({"geoIp": v, "outboundTag": tag})),
                _ => None,
            })
            .collect()
    }
}

// re-export to keep the path in crate::model consistent
pub use crate::model::rule_provider::RuleProvider as RpAlias;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn expand_classical_lines() {
        let lines = vec![
            "DOMAIN-SUFFIX,baidu.com".into(),
            "DOMAIN-KEYWORD,bai".into(),
            "IP-CIDR,1.2.3.0/24".into(),
        ];
        let v = RuleSetExpander::expand(&lines, Target::Direct, &HashSet::new());
        assert_eq!(v.len(), 3);
        assert_eq!(v[0]["domainSuffix"][0], "baidu.com");
        assert_eq!(v[0]["outboundTag"], "direct");
        assert_eq!(v[2]["ipCidr"][0], "1.2.3.0/24");
    }
}
```

- [ ] **Step 2: Add `src/model/rule_set_file.rs` and wire it**

Create `src/model/rule_set_file.rs`:
```rust
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RuleSetFile {
    #[serde(default)]
    pub payload: Vec<String>,
}
```

Update `src/model/mod.rs` to add: `pub mod rule_set_file;`

Note: this duplicates `RuleProvider::RuleSetFile`. Replace the struct in `src/model/rule_provider.rs`: delete the `RuleSetFile` struct there and update its `use` to `pub use crate::model::rule_set_file::RuleSetFile;` at top of `rule_provider.rs`. Then remove the now-unused `RuleSetFile` definition.

- [ ] **Step 3: Update `src/converter/mod.rs`**

```rust
pub mod proxy_converter;
pub mod rule_converter;
pub mod ruleset_expander;
```

- [ ] **Step 4: Run tests**

Run: `cargo test converter::ruleset_expander`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add src/converter/ruleset_expander.rs src/converter/mod.rs src/model
git commit -m "feat(converter): rule-set expander"
```

---

## Task 13: `converter::fallback` — minimal fallback rules

**Files:**
- Create: `src/converter/fallback.rs`
- Modify: `src/converter/mod.rs`

- [ ] **Step 1: Write `src/converter/fallback.rs` with tests**

```rust
use serde_json::{json, Value};

/// Minimal guaranteed-usable rules appended after converted rules. Ensures
/// private nets + CN domains go direct and everything else goes through proxy.
pub fn fallback_rules() -> Vec<Value> {
    vec![
        json!({"ipCidr": [
            "10.0.0.0/8","172.16.0.0/12","192.168.0.0/16","127.0.0.0/8",
            "169.254.0.0/16","100.64.0.0/10","224.0.0.0/4","::1/128","fc00::/7","fe80::/10"
        ], "outboundTag": "direct"}),
        json!({"domainSuffix": ["cn","com.cn","ggpht.com"], "outboundTag": "direct"}),
        json!({"geoIp": "cn", "outboundTag": "direct"}),
        json!({"type": "field", "outboundTag": "proxy"}),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ends_with_proxy_fallback() {
        let r = fallback_rules();
        let last = r.last().unwrap();
        assert_eq!(last["outboundTag"], "proxy");
        assert!(r.iter().any(|x| x.get("geoIp").and_then(|v| v.as_str()) == Some("cn")));
        assert!(r.iter().any(|x| x.get("domainSuffix").is_some()));
    }
}
```

- [ ] **Step 2: Update `src/converter/mod.rs`**

```rust
pub mod fallback;
pub mod proxy_converter;
pub mod rule_converter;
pub mod ruleset_expander;
```

- [ ] **Step 3: Run tests**

Run: `cargo test converter::fallback`
Expected: 1 passed.

- [ ] **Step 4: Commit**

```bash
git add src/converter/fallback.rs src/converter/mod.rs
git commit -m "feat(converter): minimal fallback rules"
```

---

## Task 14: `converter::xray_config` — assemble full config

**Files:**
- Create: `src/converter/xray_config.rs`
- Modify: `src/converter/mod.rs`

- [ ] **Step 1: Write `src/converter/xray_config.rs` with tests**

```rust
use crate::converter::proxy_converter::to_outbound;
use crate::converter::rule_converter::{convert, Converted};
use crate::converter::ruleset_expander::RuleSetExpander;
use crate::converter::fallback::fallback_rules;
use crate::error::Result;
use crate::model::proxy::Proxy;
use crate::model::rule::Target;
use crate::subscription::parser::ParsedSubscription;
use serde_json::{json, Value};
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct ConvertStats {
    pub rules_ok: usize,
    pub rules_skipped: usize,
    pub rules_fallback: usize,
}

/// Build the complete xray config for the given active proxy.
/// `expanded_rulesets` is a map of ruleset-name -> already-fetched payload lines.
pub fn build_config(
    sub: &ParsedSubscription,
    active: &Proxy,
    http_port: u16,
    socks_port: u16,
    ruleset_payloads: &std::collections::HashMap<String, Vec<String>>,
) -> Result<(Value, ConvertStats)> {
    let proxy_outbound = to_outbound(active)?.unwrap_or_else(|| json!({"tag":"proxy","protocol":"blackhole"}));
    let outbounds = vec![
        proxy_outbound,
        json!({"tag": "direct", "protocol": "freedom"}),
        json!({"tag": "block", "protocol": "blackhole"}),
    ];

    let mut rules: Vec<Value> = Vec::new();
    let mut stats = ConvertStats::default();
    let group_names = &sub.group_names;

    for r in &sub.rules {
        match convert(r) {
            Converted::Rule(v) => { rules.push(v); stats.rules_ok += 1; }
            Converted::Match(v) => { rules.push(v); stats.rules_ok += 1; }
            Converted::RuleSet(name, target) => {
                if let Some(lines) = ruleset_payloads.get(&name) {
                    let expanded = RuleSetExpander::expand(lines, target, group_names);
                    stats.rules_ok += expanded.len();
                    rules.extend(expanded);
                }
                // missing ruleset: skip silently (fallback covers it)
            }
            Converted::Skipped(_) => { stats.rules_skipped += 1; }
        }
    }

    let fb = fallback_rules();
    stats.rules_fallback = fb.len();
    rules.extend(fb);

    let cfg = json!({
        "log": {"loglevel": "warning"},
        "inbounds": [
            {"tag": "http-in", "listen": "127.0.0.1", "port": http_port, "protocol": "http"},
            {"tag": "socks-in", "listen": "127.0.0.1", "port": socks_port, "protocol": "socks"}
        ],
        "outbounds": outbounds,
        "routing": {"domainStrategy": "IPIfNonMatch", "rules": rules}
    });

    Ok((cfg, stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subscription::parser::parse;

    fn fixture() -> ParsedSubscription {
        let text = std::fs::read_to_string("tests/fixtures/clash_meta.yaml")
            .or_else(|_| std::fs::read_to_string("../tests/fixtures/clash_meta.yaml")).unwrap();
        parse(&text).unwrap()
    }

    #[test]
    fn build_config_has_inbounds_outbounds_rules() {
        let sub = fixture();
        let active = sub.proxies.iter().find(|p| p.name == "US-Xr1").unwrap();
        let (cfg, stats) = build_config(&sub, active, 7890, 7891, &Default::default()).unwrap();
        assert_eq!(cfg["inbounds"][0]["protocol"], "http");
        assert_eq!(cfg["inbounds"][1]["protocol"], "socks");
        assert_eq!(cfg["outbounds"][0]["tag"], "proxy");
        assert_eq!(cfg["outbounds"][0]["protocol"], "vless");
        assert!(cfg["routing"]["rules"].as_array().unwrap().len() > 0);
        // PROCESS-NAME counted as skipped
        assert!(stats.rules_skipped >= 1);
        // fallback appended
        assert!(stats.rules_fallback >= 4);
    }
}
```

- [ ] **Step 2: Update `src/converter/mod.rs`**

```rust
pub mod fallback;
pub mod proxy_converter;
pub mod rule_converter;
pub mod ruleset_expander;
pub mod xray_config;
```

- [ ] **Step 3: Run tests**

Run: `cargo test converter::xray_config`
Expected: 1 passed.

- [ ] **Step 4: Commit**

```bash
git add src/converter/xray_config.rs src/converter/mod.rs
git commit -m "feat(converter): assemble full xray config"
```

---

## Task 15: `store::paths` — config directory layout

**Files:**
- Create: `src/store/mod.rs`
- Create: `src/store/paths.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write `src/store/paths.rs` with tests**

```rust
use std::path::PathBuf;

/// Root config dir: ~/.config/ratc (or $XDG_CONFIG_HOME/ratc).
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"))
        .join("ratc")
}

pub fn cache_dir() -> PathBuf { config_dir().join("cache") }
pub fn ruleset_dir() -> PathBuf { config_dir().join("ruleset") }
pub fn logs_dir() -> PathBuf { config_dir().join("logs") }
pub fn config_file() -> PathBuf { config_dir().join("config.json") }
pub fn proxy_sh() -> PathBuf { config_dir().join("proxy.sh") }
pub fn xray_config_file() -> PathBuf { config_dir().join("xray.json") }

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
        assert!(cache_file("u").starts_with(&root));
    }

    fn cache_file(name: &str) -> PathBuf { cache_dir().join(name) }
}
```

- [ ] **Step 2: Write `src/store/mod.rs`**

```rust
pub mod paths;
```

- [ ] **Step 3: Wire into `src/main.rs`**

Add: `mod store;`

- [ ] **Step 4: Run tests**

Run: `cargo test store`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add src/store src/main.rs
git commit -m "feat(store): config directory layout"
```

---

## Task 16: `config.rs` — app configuration

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write `src/config.rs` with tests**

```rust
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
```

- [ ] **Step 2: Wire into `src/main.rs`**

Add: `mod config;`

- [ ] **Step 3: Run tests**

Run: `cargo test config`
Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: app configuration"
```

---

## Task 17: `sysproxy` — write/clear proxy.sh

**Files:**
- Create: `src/sysproxy/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write `src/sysproxy/mod.rs` with tests**

```rust
use crate::error::Result;
use crate::store::paths::proxy_sh;

fn no_proxy() -> &'static str {
    "localhost,127.0.0.1,::1,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,*.cn"
}

/// Write a shell snippet exporting proxy env vars for the given HTTP port.
pub fn enable(http_port: u16) -> Result<()> {
    let content = format!(
        "# managed by ratc\nexport http_proxy=\"http://127.0.0.1:{p}\"\nexport https_proxy=\"http://127.0.0.1:{p}\"\nexport HTTP_PROXY=\"http://127.0.0.1:{p}\"\nexport HTTPS_PROXY=\"http://127.0.0.1:{p}\"\nexport no_proxy=\"{n}\"\nexport NO_PROXY=\"{n}\"\n",
        p = http_port,
        n = no_proxy()
    );
    std::fs::write(proxy_sh(), content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(proxy_sh(), std::fs::Permissions::from_mode(0o644)).ok();
    }
    Ok(())
}

/// Remove proxy env vars (write unset snippet so a sourced file cleans up).
pub fn disable() -> Result<()> {
    let content = "# managed by ratc\nunset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY no_proxy NO_PROXY\n";
    std::fs::write(proxy_sh(), content)?;
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
```

- [ ] **Step 2: Wire into `src/main.rs`**

Add: `mod sysproxy;`

- [ ] **Step 3: Run tests**

Run: `cargo test sysproxy`
Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add src/sysproxy src/main.rs
git commit -m "feat(sysproxy): env-var proxy.sh toggle"
```

---

## Task 18: `xray::paths` and `xray::process` — subprocess management

**Files:**
- Create: `src/xray/mod.rs`
- Create: `src/xray/paths.rs`
- Create: `src/xray/process.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write `src/xray/paths.rs`**

```rust
use std::path::PathBuf;

/// Resolve xray binary from config path, then PATH.
pub fn resolve(configured: &str) -> Option<PathBuf> {
    let p = PathBuf::from(configured);
    if p.is_file() { return Some(p); }
    which_xray()
}

fn which_xray() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let f = dir.join("xray");
        if f.is_file() { return Some(f); }
    }
    None
}
```

- [ ] **Step 2: Write `src/xray/process.rs` with tests**

```rust
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
            let mut h = XrayHandle::new("/usr/local/bin/xray").unwrap();
            let cfg = json!({
                "inbounds": [{"port": 17890, "listen":"127.0.0.1", "protocol":"http"}],
                "outbounds": [{"protocol":"freedom","tag":"direct"}]
            });
            h.test_config(&cfg).expect("xray should accept config");
        });
    }
}
```

- [ ] **Step 3: Write `src/xray/mod.rs`**

```rust
pub mod paths;
pub mod process;
```

- [ ] **Step 4: Wire into `src/main.rs`**

Add: `mod xray;`

- [ ] **Step 5: Run tests**

Run: `cargo test xray`
Expected: `test_config_validates_with_real_xray` passes (xray present at `/usr/local/bin/xray`).

- [ ] **Step 6: Commit**

```bash
git add src/xray src/main.rs
git commit -m "feat(xray): subprocess lifecycle + config validation"
```

---

## Task 19: Integration test — full converter → xray -test

**Files:**
- Create: `tests/xray_config.rs`

- [ ] **Step 1: Write `tests/xray_config.rs`**

```rust
use ratc::converter::proxy_converter::to_outbound;
use ratc::converter::xray_config::build_config;
use ratc::subscription::parser::parse;
use serde_json::Value;

fn fixture() -> String {
    std::fs::read_to_string("tests/fixtures/clash_meta.yaml").unwrap()
}

#[test]
fn generated_config_passes_xray_test() {
    let bin = std::path::Path::new("/usr/local/bin/xray");
    if !bin.is_file() { return; }
    let sub = parse(&fixture()).unwrap();
    let active = sub.proxies.iter().find(|p| p.name == "US-Xr1").unwrap();
    let (cfg, _stats) = build_config(&sub, active, 7890, 7891, &Default::default()).unwrap();
    // ensure proxy outbound present
    assert!(to_outbound(active).unwrap().is_some());
    // run xray -test
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let out = std::process::Command::new(bin)
        .arg("-test").arg("-config").arg(tmp.path())
        .output().unwrap();
    assert!(out.status.success(), "xray stderr: {}", String::from_utf8_lossy(&out.stderr));
}
```

- [ ] **Step 2: Expose lib crate — convert `src/main.rs` into a binary + lib**

Create `src/lib.rs`:
```rust
pub mod config;
pub mod converter;
pub mod error;
pub mod model;
pub mod store;
pub mod subscription;
pub mod sysproxy;
pub mod xray;
```

Remove the `mod` declarations from `src/main.rs` (they live in `lib.rs` now). Replace `src/main.rs` with:
```rust
fn main() -> ratc::error::Result<()> {
    // Real wiring lands in Task 24.
    println!("rATC: core modules ready (TUI pending)");
    Ok(())
}
```

Update `Cargo.toml` — add:
```toml
[lib]
name = "ratc"
path = "src/lib.rs"

[[bin]]
name = "ratc"
path = "src/main.rs"
```

- [ ] **Step 3: Run the integration test**

Run: `cargo test --test xray_config`
Expected: passes (xray accepts the generated config).

- [ ] **Step 4: Commit**

```bash
git add tests/xray_config.rs src/lib.rs src/main.rs Cargo.toml
git commit -m "test: integration test for generated xray config"
```

---

## Task 20: `app.rs` — application state machine

**Files:**
- Create: `src/app.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write `src/app.rs`**

```rust
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
    pub last_stats: ConvertStats,
    pub logs: Vec<String>,
    pub ruleset_payloads: HashMap<String, Vec<String>>,
}

impl App {
    pub fn init(cfg: AppConfig) -> Result<Self> {
        let xray = XrayHandle::new(&cfg.xray_path).ok();
        Ok(Self { cfg, sub: None, xray, last_stats: ConvertStats::default(), logs: Vec::new(), ruleset_payloads: HashMap::new() })
    }

    pub fn supported_proxies(&self) -> Vec<&Proxy> {
        self.sub.as_ref().map(|s| s.proxies.iter().filter(|p| p.compat() == Compat::Supported).collect()).unwrap_or_default()
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
            self.log(format!("xray started with {}", name));
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
```

- [ ] **Step 2: Wire into `src/lib.rs`**

Add: `pub mod app;`

- [ ] **Step 3: Verify build**

Run: `cargo build`
Expected: compiles.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/lib.rs
git commit -m "feat(app): application state machine"
```

---

## Task 21: TUI scaffolding — terminal loop and event handling

**Files:**
- Create: `src/tui/mod.rs`
- Create: `src/tui/event.rs`
- Create: `src/tui/app_state.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write `src/tui/event.rs`**

```rust
use crossterm::event::{self, Event, KeyEvent};
use std::time::{Duration, Instant};

pub enum Message {
    Key(KeyEvent),
    Tick,
}

/// Poll for a terminal event with a tick timeout.
pub fn poll(tick_ms: u64) -> Option<Message> {
    let deadline = Instant::now() + Duration::from_millis(tick_ms);
    while Instant::now() < deadline {
        if event::poll(Duration::from_millis(50)).ok()? {
            if let Event::Key(k) = event::read().ok()? {
                return Some(Message::Key(k));
            }
        }
    }
    Some(Message::Tick)
}
```

- [ ] **Step 2: Write `src/tui/app_state.rs`**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab { Nodes, Subscriptions, Rules, Logs, Settings }

impl Tab {
    pub fn all() -> [Tab; 5] {
        [Tab::Nodes, Tab::Subscriptions, Tab::Rules, Tab::Logs, Tab::Settings]
    }
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Nodes => "[1]节点", Tab::Subscriptions => "[2]订阅",
            Tab::Rules => "[3]规则", Tab::Logs => "[4]日志", Tab::Settings => "[5]设置",
        }
    }
}

pub struct UiState {
    pub tab: Tab,
    pub selected: usize,
    pub running: bool,
}

impl Default for UiState {
    fn default() -> Self { Self { tab: Tab::Nodes, selected: 0, running: true } }
}
```

- [ ] **Step 3: Write `src/tui/mod.rs` (render + loop; tab rendering delegated to tabs module)**

```rust
pub mod event;
pub mod app_state;
pub mod tabs;

use crate::app::App;
use crate::error::Result;
use app_state::{Tab, UiState};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Terminal;
use std::io::Stdout;

pub fn run(app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = UiState::default();
    loop {
        terminal.draw(|f| draw(f, app, &ui))?;
        match event::poll(200) {
            Some(event::Message::Tick) => {}
            Some(event::Message::Key(k)) => {
                if handle_key(app, &mut ui, k) { break; }
            }
            None => break,
        }
        if !ui.running { break; }
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn handle_key(app: &mut App, ui: &mut UiState, k: KeyEvent) -> bool {
    use KeyCode::*;
    match k.code {
        Char('q') => { ui.running = false; false }
        Char('1') => { ui.tab = Tab::Nodes; false }
        Char('2') => { ui.tab = Tab::Subscriptions; false }
        Char('3') => { ui.tab = Tab::Rules; false }
        Char('4') => { ui.tab = Tab::Logs; false }
        Char('5') => { ui.tab = Tab::Settings; false }
        Char('r') => { let _ = app.refresh_subscription(); false }
        Char('s') => { let _ = app.toggle_sys_proxy(); false }
        Down | Char('j') => { ui.selected = ui.selected.saturating_add(1); false }
        Up | Char('k') => { ui.selected = ui.selected.saturating_sub(if ui.selected > 0 {1} else {0}); false }
        Enter => {
            if ui.tab == Tab::Nodes {
                let proxies = app.supported_proxies();
                if let Some(p) = proxies.get(ui.selected.min(proxies.len().saturating_sub(1))) {
                    let _ = app.select_proxy(&p.name.clone());
                }
            }
            false
        }
        _ => false,
    }
}

fn draw(f: &mut ratatui::Frame<'_>, app: &App, ui: &UiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Length(3), Constraint::Min(5), Constraint::Length(2)])
        .split(f.size());

    // status bar
    let xray_status = if app.xray.as_ref().map(|x| x.clone().is_running()).unwrap_or(false) { "●running" } else { "○stopped" };
    let cur = app.cfg.current_proxy.clone().unwrap_or_else(|| "-".into());
    let status = Line::from(vec![
        Span::raw("[Status] xray:"),
        Span::styled(xray_status, Style::default().fg(Color::Green)),
        Span::raw(format!("  proxy:{cur}  sys_proxy:{}", if app.cfg.sys_proxy_on {"on"} else {"off"})),
    ]);
    let info = app.sub.as_ref().map(|s| format!("节点:{} 可用:{}", s.proxies.len(), app.supported_proxies().len())).unwrap_or_default();
    let para = Paragraph::new(vec![status, Line::from(format!("[Info] {info}"))])
        .block(Block::default().borders(Borders::ALL).title("rATC"));
    f.render_widget(para, chunks[0]);

    let titles: Vec<Line> = Tab::all().iter().map(|t| {
        let style = if *t == ui.tab { Style::default().add_modifier(Modifier::REVERSED) } else { Style::default() };
        Line::styled(t.title(), style)
    }).collect();
    f.render_widget(Tabs::new(titles), chunks[1]);

    tabs::render(f, app, ui, chunks[2]);

    let help = "q:退出  r:刷新订阅  s:系统代理  1-5:Tab  ↑↓/jk:导航  Enter:选择  ?:帮助";
    f.render_widget(Paragraph::new(help), chunks[3]);
}
```

> Note: `app.xray.as_ref().map(|x| x.clone().is_running())` clones the `Option<&XrayHandle>`; `is_running` takes `&mut self`, so we need a mutable borrow. Since `draw` takes `&App`, status can only reflect config-based running state. Adjust `XrayHandle::is_running` to also be callable via an `&self` helper using an `Arc<Mutex<Option<Child>>>` if true liveness is needed in the UI. For MVP, track running as a `bool` field on `App` updated after `start`/`stop`. Add `pub xray_running: bool` to `App` (Task 20 follow-up), set it in `select_proxy` after `x.start()`, and read it here instead of querying the handle. Apply that change now.

- [ ] **Step 4: Apply the `xray_running` field follow-up**

Edit `src/app.rs`: add field `pub xray_running: bool,` to `App`, initialize `false` in `init`, and set `self.xray_running = true;` after `x.start(&cfg)?;` in `select_proxy`. Then in `src/tui/mod.rs` replace the `xray_status` line with:
```rust
let xray_status = if app.xray_running { "●running" } else { "○stopped" };
```

- [ ] **Step 5: Create `src/tui/tabs/mod.rs` stub**

```rust
pub mod nodes;
pub mod subscriptions;
pub mod rules;
pub mod logs;
pub mod settings;

use crate::app::App;
use crate::tui::app_state::UiState;

pub fn render(f: &mut ratatui::Frame<'_>, app: &App, ui: &UiState, area: ratatui::layout::Rect) {
    use crate::tui::app_state::Tab;
    match ui.tab {
        Tab::Nodes => nodes::render(f, app, ui, area),
        Tab::Subscriptions => subscriptions::render(f, app, ui, area),
        Tab::Rules => rules::render(f, app, ui, area),
        Tab::Logs => logs::render(f, app, ui, area),
        Tab::Settings => settings::render(f, app, ui, area),
    }
}
```

- [ ] **Step 6: Verify build (tab modules not yet implemented — create thin stubs that render an empty block)**

Create each of `src/tui/tabs/{nodes,subscriptions,rules,logs,settings}.rs` as:
```rust
use crate::app::App;
use crate::tui::app_state::UiState;
use ratatui::widgets::{Block, Borders, Paragraph};
pub fn render(f: &mut ratatui::Frame<'_>, _app: &App, _ui: &UiState, area: ratatui::layout::Rect) {
    f.render_widget(Paragraph::new("todo").block(Block::default().borders(Borders::ALL).title("节点")), area);
}
```
(adjust the `title` to match each tab name).

- [ ] **Step 7: Wire into `src/lib.rs`**

Add: `pub mod tui;`

- [ ] **Step 8: Verify build**

Run: `cargo build`
Expected: compiles.

- [ ] **Step 9: Commit**

```bash
git add src/tui src/lib.rs
git commit -m "feat(tui): terminal loop, event handling, tab scaffold"
```

---

## Task 22: TUI tab implementations

**Files:**
- Modify: `src/tui/tabs/nodes.rs`
- Modify: `src/tui/tabs/subscriptions.rs`
- Modify: `src/tui/tabs/rules.rs`
- Modify: `src/tui/tabs/logs.rs`
- Modify: `src/tui/tabs/settings.rs`

Each tab renders a read-only view from `App`. There are no per-tab tests (rendering is visual); manual verification happens in Task 25.

- [ ] **Step 1: `src/tui/tabs/nodes.rs`**

```rust
use crate::app::App;
use crate::model::proxy::Compat;
use crate::tui::app_state::UiState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

fn kind_label(p: &crate::model::proxy::Proxy) -> &'static str {
    use crate::model::proxy::ProxyType::*;
    match &p.ptype {
        Vless { reality: Some(_), .. } => "vless+reality",
        Vless { .. } => "vless",
        Vmess { ws: Some(_), .. } => "vmess+ws",
        Vmess { .. } => "vmess",
        Shadowsocks { plugin: None, .. } => "ss",
        Trojan { .. } => "trojan",
        _ => "unsupported",
    }
}

pub fn render(f: &mut Frame<'_>, app: &App, ui: &UiState, area: Rect) {
    let proxies: Vec<&crate::model::proxy::Proxy> = app.sub.as_ref()
        .map(|s| s.proxies.iter().collect()).unwrap_or_default();
    let current = app.cfg.current_proxy.as_deref();
    let items: Vec<ListItem> = proxies.iter().enumerate().map(|(i, p)| {
        let mark = match p.compat() {
            Compat::Supported => if Some(p.name.as_str()) == current { "●" } else { "○" },
            Compat::Unsupported(_) => "✕",
        };
        let style = if matches!(p.compat(), Compat::Unsupported(_)) {
            Style::default().fg(Color::DarkGray)
        } else if i == ui.selected {
            Style::default().fg(Color::Yellow)
        } else { Style::default() };
        let line = Line::from(vec![
            Span::raw(format!(" {mark}  ")),
            Span::raw(format!("{:<28}", p.name)),
            Span::raw(format!(" {:<16}", kind_label(p))),
            Span::raw(format!(" {}", p.port)),
        ]);
        ListItem::new(line).style(style)
    }).collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("节点列表"));
    f.render_widget(list, area);
    let hint = Paragraph::new(format!("共{}个 可用{}个", proxies.len(), app.supported_proxies().len()));
    f.render_widget(hint, area);
}
```

> Fix: do not render the `hint` Paragraph over the list. Instead pass hint into the list's block title via `format!`. For MVP simplicity, leave the list as the only widget in `area`; move the count into the block title: `.title(format!("节点列表 (共{} 可用{})", proxies.len(), app.supported_proxies().len()))` and delete the separate `hint` paragraph. Apply that now.

- [ ] **Step 2: `src/tui/tabs/subscriptions.rs`**

```rust
use crate::app::App;
use crate::tui::app_state::UiState;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

pub fn render(f: &mut Frame<'_>, app: &App, _ui: &UiState, area: Rect) {
    let items: Vec<ListItem> = app.cfg.subscriptions.iter().map(|s| {
        let star = if s.active { "*" } else { " " };
        Line::raw(format!(" {star} {:<14} {}", s.name, s.url))
    }).collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("订阅列表 [a]添加 [d]删除 [u]更新 [Enter]激活"));
    f.render_widget(list, area);
}
```

- [ ] **Step 3: `src/tui/tabs/rules.rs`**

```rust
use crate::app::App;
use crate::tui::app_state::UiState;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame<'_>, app: &App, _ui: &UiState, area: Rect) {
    let Some(sub) = &app.sub else { return; };
    let mut items: Vec<ListItem> = Vec::new();
    for r in &sub.rules {
        use crate::model::rule::Rule;
        let line = match r {
            Rule::Domain(v, _) => format!("domain        {v}"),
            Rule::DomainSuffix(v, _) => format!("domainSuffix  {v}"),
            Rule::DomainKeyword(v, _) => format!("domainKeyword {v}"),
            Rule::IpCidr(v, _, _) => format!("ipCidr        {v}"),
            Rule::GeoIp(v, _) => format!("geoIp         {v}"),
            Rule::RuleSet(v, _) => format!("RULE-SET      {v}"),
            Rule::Match(_) => "MATCH (fallback)".into(),
            Rule::Unsupported(t, _) => format!("[skip] {t}"),
        };
        items.push(ListItem::new(Line::raw(line)));
    }
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("路由规则 (转换前)"));
    f.render_widget(list, area);
    let stat = Paragraph::new(format!("转换统计: 规则{} 跳过~ 兜底{}", app.last_stats.rules_ok, app.last_stats.rules_fallback));
    f.render_widget(stat, area);
}
```

> Same overwrite concern: merge `stat` into block title and drop the second `render_widget`. Apply: replace the block title with `.title(format!("路由规则 成功{} 跳过{} 兜底{}", app.last_stats.rules_ok, app.last_stats.rules_skipped, app.last_stats.rules_fallback))` and remove the `Paragraph` render.

- [ ] **Step 4: `src/tui/tabs/logs.rs`**

```rust
use crate::app::App;
use crate::tui::app_state::UiState;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

pub fn render(f: &mut Frame<'_>, app: &App, _ui: &UiState, area: Rect) {
    let items: Vec<ListItem> = app.logs.iter().rev().take(200).map(|l| ListItem::new(Line::raw(l.clone()))).collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("日志"));
    f.render_widget(list, area);
}
```

- [ ] **Step 5: `src/tui/tabs/settings.rs`**

```rust
use crate::app::App;
use crate::tui::app_state::UiState;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame<'_>, app: &App, _ui: &UiState, area: Rect) {
    let c = &app.cfg;
    let lines = vec![
        Line::raw(format!(" HTTP 端口:      [{}]", c.http_port)),
        Line::raw(format!(" SOCKS 端口:     [{}]", c.socks_port)),
        Line::raw(format!(" 监听地址:       [{}]", c.listen)),
        Line::raw(format!(" xray 路径:      [{}]", c.xray_path)),
        Line::raw(format!(" 允许局域网:     [{}]", if c.allow_lan {"x"} else {" "})),
        Line::raw(format!(" 日志级别:       [{}]", c.log_level)),
        Line::raw(format!(" 退出时关闭xray: [{}]", if c.exit_kills_xray {"x"} else {" "})),
        Line::raw(format!(" 系统代理:       [{}]", if c.sys_proxy_on {"on"} else {"off"})),
    ];
    f.render_widget(Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("设置 (编辑请改 ~/.config/ratc/config.json)")), area);
}
```

- [ ] **Step 6: Build**

Run: `cargo build`
Expected: compiles.

- [ ] **Step 7: Commit**

```bash
git add src/tui/tabs
git commit -m "feat(tui): node/subscription/rule/log/settings views"
```

---

## Task 23: `main.rs` — wire everything together

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write `src/main.rs`**

```rust
use ratc::app::App;
use ratc::config::AppConfig;
use ratc::store::paths::ensure_dirs;
use ratc::tui;

fn main() -> ratc::error::Result<()> {
    ensure_dirs()?;
    let cfg = AppConfig::load()?;
    let mut app = App::init(cfg)?;
    app.refresh_subscription()?;
    // auto-select first supported proxy if none chosen
    if app.cfg.current_proxy.is_none() {
        if let Some(p) = app.supported_proxies().first() {
            let name = p.name.clone();
            app.select_proxy(&name)?;
        }
    }
    tui::run(&mut app)?;
    Ok(())
}
```

- [ ] **Step 2: Build the release binary**

Run: `cargo build --release`
Expected: compiles; binary at `target/release/ratc`.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire main entrypoint + auto-select proxy"
```

---

## Task 24: First-run subscription bootstrap

**Problem:** On first run `~/.config/ratc/config.json` has no subscriptions, so the TUI shows nothing. Provide a one-time bootstrap so a subscription URL passed via env or interactive prompt is stored.

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add bootstrap before TUI launch in `src/main.rs`**

Insert after `let mut app = App::init(cfg)?;` and before `app.refresh_subscription()`:
```rust
if app.cfg.subscriptions.is_empty() {
    if let Ok(url) = std::env::var("RATC_SUB_URL") {
        app.cfg.subscriptions.push(ratc::config::SubscriptionEntry {
            name: "default".into(), url, active: true,
        });
        app.cfg.save()?;
    } else {
        eprintln!("未配置订阅。请运行: RATC_SUB_URL='http://...' ./ratc");
        eprintln!("或在首次启动前编辑 ~/.config/ratc/config.json");
    }
}
```

- [ ] **Step 2: Build**

Run: `cargo build`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: bootstrap subscription from RATC_SUB_URL"
```

---

## Task 25: `README.md`

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write `README.md`**

````markdown
# rATC — Rust Agent for Terminal Clash

A Linux terminal (TUI) proxy manager in the spirit of Clash Verge, backed by [xray-core](https://github.com/XTLS/Xray-core). It fetches Clash Meta YAML subscriptions, converts nodes and routing rules into xray configs, and drives the xray subprocess from a Ratatui interface.

## Features

- Clash Meta YAML subscription parsing & caching
- Node switching with per-protocol compatibility markers
- Automatic Clash → xray-core rule conversion with a minimal fallback rule set
- xray-core lifecycle management (start/stop/reload)
- Local HTTP (7890) + SOCKS (7891) inbound
- Optional system-proxy toggle via a sourced `proxy.sh`
- 5-tab TUI: 节点 / 订阅 / 规则 / 日志 / 设置

## Requirements

- Linux (x11/tty), xray-core installed and on PATH (or at `/usr/local/bin/xray`)
- `geoip.dat` / `geosite.dat` recommended for GEOIP rules (place in xray's working dir)

Install xray-core:
```bash
bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install
```

## Build

```bash
cargo build --release
# binary: target/release/ratc
```

## Quick Start

1. Build (see above) or download the release binary.
2. Add your subscription on first run:
   ```bash
   RATC_SUB_URL='http://your-provider/sub?token=...' ./target/release/ratc
   ```
3. Inside the TUI: press `1` for nodes, navigate with `j/k`, `Enter` to switch.
4. (Optional) enable system proxy for terminal apps — press `s`, then add to your `~/.bashrc` or `~/.zshrc`:
   ```sh
   [ -f ~/.config/ratc/proxy.sh ] && source ~/.config/ratc/proxy.sh
   ```
5. Press `q` to quit.

## Keyboard Reference

| Key | Action |
|-----|--------|
| `1`-`5` | Switch tabs |
| `j`/`k`, `↑`/`↓` | Navigate list |
| `Enter` | Select node |
| `r` | Refresh subscription |
| `s` | Toggle system proxy |
| `q` | Quit |

## Protocol Compatibility

| Protocol | xray support |
|----------|--------------|
| vless (+reality/+ws) | ✅ |
| vmess (+ws/+tls) | ✅ |
| shadowsocks (no plugin) | ✅ |
| trojan | ✅ |
| shadowsocks + shadow-tls | ❌ (skipped) |
| hysteria2 | ❌ (skipped) |

Unsupported nodes are shown greyed out and excluded from the xray config.

## Configuration

Runtime data lives under `~/.config/ratc/`:

| Path | Purpose |
|------|---------|
| `config.json` | App config (ports, xray path, subscriptions, current node) |
| `proxy.sh` | System-proxy source snippet |
| `cache/` | Subscription YAML cache |
| `ruleset/` | Rule-set cache |
| `xray.json` | Generated xray config |
| `logs/` | Application logs |

## FAQ

**Q: xray won't start.** Press `4` (logs) for stderr; verify the binary path in Settings (`~/.config/ratc/config.json` → `xray_path`). Run `xray -test -config ~/.config/ratc/xray.json` to debug.

**Q: GEOIP rules fail.** Download `geoip.dat` and `geosite.dat` from [Loyalsoldier/v2ray-rules-dat](https://github.com/Loyalsoldier/v2ray-rules-dat) into `~/.config/ratc/` or the xray assets directory.

**Q: My hysteria2/shadow-tls nodes are grey.** xray-core does not implement these protocols; they are intentionally skipped.

## License

MIT
````

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: README"
```

---

## Task 26: `docs/MANUAL.md` — usage manual

**Files:**
- Create: `docs/MANUAL.md`

- [ ] **Step 1: Write `docs/MANUAL.md`**

````markdown
# rATC 使用手册

rATC 是一个基于 xray-core 的 Linux 终端代理管理器。本手册覆盖安装、首次配置、各 Tab 操作、快捷键、系统代理、规则兼容性、故障排查与配置参考。

## 目录
1. [安装](#1-安装)
2. [首次运行](#2-首次运行)
3. [界面与 Tab 操作](#3-界面与-tab-操作)
4. [快捷键速查](#4-快捷键速查)
5. [系统代理](#5-系统代理)
6. [规则兼容性](#6-规则兼容性)
7. [故障排查](#7-故障排查)
8. [配置文件参考](#8-配置文件参考)

## 1. 安装

### 1.1 依赖
- Linux（终端或 X11）
- xray-core（推荐 1.8.x，支持 vless+reality）
- （可选）`geoip.dat` / `geosite.dat`，用于 GEOIP 规则

### 1.2 安装 xray-core
```bash
bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install
# 安装到 /usr/local/bin/xray
xray version
```

### 1.3 放置 GEOIP/GEOSITE 数据
```bash
mkdir -p ~/.config/ratc
cd ~/.config/ratc
wget https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geoip.dat
wget https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geosite.dat
```
> rATC 启动 xray 时会把工作目录设为 `~/.config/ratc/`，因此放在该目录即可被 GEOIP 规则命中。

### 1.4 构建 rATC
```bash
cargo build --release
sudo cp target/release/ratc /usr/local/bin/
```

## 2. 首次运行

通过环境变量注入第一个订阅：
```bash
RATC_SUB_URL='http://your-provider/modules/.../sub?token=...' ratc
```
首次启动会把该 URL 存为默认订阅并缓存到 `~/.config/ratc/cache/`。之后无需再带环境变量，直接运行 `ratc` 即可。

## 3. 界面与 Tab 操作

界面自上而下：状态栏 → Tab 栏 → 内容区 → 帮助栏。

### 3.1 状态栏
显示 xray 运行状态、当前节点、系统代理开关、节点/可用数。

### 3.2 Tab 1 — 节点
- `●` 当前节点；`○` 可选；`✕` 不可用（灰显）
- `j/k` 或 `↑/↓` 移动，`Enter` 切换并重载 xray
- 切换到不可用节点会被拒绝并提示原因

### 3.3 Tab 2 — 订阅
- `*` 表示当前激活订阅
- 当前版本通过编辑 `config.json` 添加/删除/激活订阅（后续版本会加入交互编辑）

### 3.4 Tab 3 — 规则
只读展示转换前的规则列表。标题栏显示转换统计（成功/跳过/兜底数）。`[skip]` 前缀表示该规则因 xray 不支持被跳过（如 `PROCESS-NAME`）。

### 3.5 Tab 4 — 日志
rATC 自身事件日志（订阅刷新、xray 启动、错误等），按时间倒序。如需 xray 详细日志，运行 `xray -test -config ~/.config/ratc/xray.json` 检查。

### 3.6 Tab 5 — 设置
只读展示当前配置。修改请编辑 `~/.config/ratc/config.json` 后重启 rATC。

## 4. 快捷键速查

| 键 | 功能 |
|----|------|
| `1`-`5` | 切换 Tab |
| `j` / `↓` | 下一项 |
| `k` / `↑` | 上一项 |
| `Enter` | 选中节点 |
| `r` | 刷新订阅 |
| `s` | 切换系统代理 |
| `q` | 退出 |

## 5. 系统代理

rATC 通过生成 `~/.config/ratc/proxy.sh` 控制终端代理。按 `s` 开启后再在 shell 配置中加载它：

**Bash (`~/.bashrc`)：**
```sh
[ -f ~/.config/ratc/proxy.sh ] && source ~/.config/ratc/proxy.sh
```
**Zsh (`~/.zshrc`)：**
```sh
[ -f ~/.config/ratc/proxy.sh ] && source ~/.config/ratc/proxy.sh
```

之后新开的终端中 `curl`、`git`、`apt` 等会自动走代理。按 `s` 关闭后该文件会写入 `unset`，重开终端即解除。

## 6. 规则兼容性

| Clash 规则 | rATC 支持 |
|-----------|----------|
| DOMAIN / DOMAIN-SUFFIX / DOMAIN-KEYWORD | ✅ |
| IP-CIDR / IP-CIDR6 | ✅ |
| GEOIP | ✅（需 geoip.dat） |
| RULE-SET | ✅（下载并展开） |
| MATCH | ✅ |
| PROCESS-NAME | ❌（xray 不支持，跳过） |

无论订阅规则覆盖如何，rATC 始终追加最小兜底规则：私有网段直连、国内域名/GeoIP 直连、其余走代理。

## 7. 故障排查

**端口被占用**：`lsof -i :7890`，或修改 `config.json` 的 `http_port`/`socks_port` 后重启。

**xray 启动失败**：进入 Tab 4 查看日志；命令行运行 `xray -test -config ~/.config/ratc/xray.json` 查看 stderr。

**订阅拉取失败**：rATC 会回退到缓存并在状态栏显示。检查网络，或在 Tab 2 用 `r` 重试。

**GEOIP 规则无效**：确认 `~/.config/ratc/geoip.dat` 存在（见 1.3）。

**节点灰显**：shadow-tls 的 ss 与 hysteria2 协议 xray 不支持，属预期行为。

## 8. 配置文件参考

`~/.config/ratc/config.json`：

```json
{
  "http_port": 7890,
  "socks_port": 7891,
  "listen": "127.0.0.1",
  "xray_path": "/usr/local/bin/xray",
  "allow_lan": false,
  "log_level": "warning",
  "exit_kills_xray": true,
  "sys_proxy_on": false,
  "current_proxy": "US-Xr1",
  "subscriptions": [
    { "name": "default", "url": "http://.../sub?token=...", "active": true }
  ]
}
```

| 字段 | 说明 |
|------|------|
| `http_port` | xray HTTP 入站端口 |
| `socks_port` | xray SOCKS 入站端口 |
| `listen` | 监听地址（默认仅本机） |
| `xray_path` | xray 二进制路径 |
| `allow_lan` | 是否允许局域网（暂仅配置项，默认 false） |
| `exit_kills_xray` | 退出 rATC 时是否关闭 xray |
| `sys_proxy_on` | 系统代理开关 |
| `current_proxy` | 当前选中节点名 |
| `subscriptions` | 订阅列表，`active: true` 者生效 |
````

- [ ] **Step 2: Commit**

```bash
git add docs/MANUAL.md
git commit -m "docs: usage manual"
```

---

## Task 27: Manual smoke test & final verification

**Files:** none (verification only)

- [ ] **Step 1: Full test suite**

Run: `cargo test`
Expected: all unit + integration tests pass.

- [ ] **Step 2: Release build**

Run: `cargo build --release`
Expected: compiles cleanly.

- [ ] **Step 3: Config validation smoke test**

Run:
```bash
RATC_SUB_URL='http://dawangidc.org/modules/servers/V2raySocks/Meta2Port.php?sid=15963&token=evBpq7oAosge' timeout 5 ./target/release/ratc || true
ls ~/.config/ratc/
xray -test -config ~/.config/ratc/xray.json
```
Expected: `~/.config/ratc/` populated with `config.json`, `xray.json`, `cache/`; `xray -test` reports Configuration OK.

- [ ] **Step 4: Final commit (if any cleanup)**

```bash
git status
# if clean, nothing to commit
```

---

## Self-Review (completed during authoring)

**Spec coverage:**
- §1 goals (subscription/switching/xray/inbound/sysproxy/rules) → Tasks 8,10,18,17,11-14,20
- §3 components (model/subscription/converter/xray/sysproxy/config/store/app/tui) → Tasks 3-6,8-9,10-14,15-16,17,18,20-22
- §4 protocols & compat matrix → Task 3 (classify), Task 10 (outbound)
- §5 rule conversion + RULE-SET + fallback + config structure → Tasks 11,12,13,14
- §6 TUI 5 tabs + keymap → Tasks 21,22
- §7 system proxy (proxy.sh) → Task 17
- §8 error handling (thiserror, graceful) → Task 2 + per-module Results
- §9 testing (fixture, unit, xray -test integration) → Tasks 7,19,27
- §10 security (0600/0700 perms) → Tasks 15,16
- §11 docs (README + MANUAL) → Tasks 25,26

**Placeholder scan:** Two inline corrections authored into the steps themselves (the `xray_running` follow-up in Task 21; the block-title merges in Task 22) — these are explicit instructions, not placeholders. No TBD/TODO remain.

**Type consistency:** `Proxy`/`ProxyType` (Task 3) consumed unchanged in Tasks 8,10,14,20,22. `Rule`/`Target` (Task 5) consumed in Tasks 8,11,12,14. `target_tag` (Task 8) reused in Tasks 11,12. `Compat::Supported` equality used in Task 20/22 matches definition in Task 3. `build_config` signature (Task 14) matches calls in Tasks 19,20.

**Scope:** Single coherent app; plan is long but sequential and TDD-driven. Appropriate for one plan.
