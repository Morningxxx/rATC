use serde_json::{json, Value};

/// Minimal guaranteed-usable rules appended after converted rules. Ensures
/// private nets + CN domains go direct. The catch-all ("everything else →
/// proxy") is intentionally NOT a rule here: xray rejects field rules with no
/// matcher ("this rule has no effective fields"). Instead, unmatched traffic
/// falls through to the default outbound, which build_config places as
/// outbounds[0] (tag "proxy" = the active node).
pub fn fallback_rules() -> Vec<Value> {
    vec![
        json!({"ip": [
            "10.0.0.0/8","172.16.0.0/12","192.168.0.0/16","127.0.0.0/8",
            "169.254.0.0/16","100.64.0.0/10","224.0.0.0/4","::1/128","fc00::/7","fe80::/10"
        ], "outboundTag": "direct"}),
        json!({"domain": ["domain:cn","domain:com.cn","domain:ggpht.com"], "outboundTag": "direct"}),
        json!({"ip": ["geoip:cn"], "outboundTag": "direct"}),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_rules_have_effective_fields() {
        let r = fallback_rules();
        // Every rule must carry a matcher field (xray rejects matcher-less rules).
        for x in &r {
            assert!(
                x.get("ip").is_some() || x.get("domain").is_some(),
                "fallback rule missing matcher: {x}"
            );
        }
        // geoip encoded inside the `ip` field as "geoip:cn"
        assert!(r.iter().any(|x| x
            .get("ip")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().any(|e| e == "geoip:cn"))
            .unwrap_or(false)));
        assert!(r.iter().any(|x| x.get("domain").is_some()));
    }
}
