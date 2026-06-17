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
            plugin: g("plugin").and_then(|_| raw.fields.get(&Value::String("plugin-opts".into())).map(|_| PluginOpts {
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
