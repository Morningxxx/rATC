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
