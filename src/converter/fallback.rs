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
