use crate::converter::proxy_converter::to_outbound;
use crate::converter::rule_converter::{convert, Converted};
use crate::converter::ruleset_expander::RuleSetExpander;
use crate::converter::fallback::fallback_rules;
use crate::error::Result;
use crate::model::proxy::Proxy;
use crate::subscription::parser::ParsedSubscription;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ConvertStats {
    pub rules_ok: usize,
    pub rules_skipped: usize,
    pub rules_fallback: usize,
}

/// Build the complete xray config for the given active proxy.
/// `ruleset_payloads` is a map of ruleset-name -> already-fetched payload lines.
pub fn build_config(
    sub: &ParsedSubscription,
    active: &Proxy,
    http_port: u16,
    socks_port: u16,
    ruleset_payloads: &HashMap<String, Vec<String>>,
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
            // MATCH is the catch-all. xray rejects matcher-less field rules, and
            // the active node is always outbounds[0] (the default), so unmatched
            // traffic already routes to proxy. We therefore drop MATCH rules
            // rather than emit an invalid rule. (MATCH→direct is an unusual edge
            // case not covered by MVP.)
            Converted::Match(_) => { stats.rules_ok += 1; }
            Converted::RuleSet(name, target) => {
                if let Some(lines) = ruleset_payloads.get(&name) {
                    let expanded = RuleSetExpander::expand(lines, target, group_names);
                    stats.rules_ok += expanded.len();
                    rules.extend(expanded);
                }
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
        assert!(stats.rules_skipped >= 1);
        assert!(stats.rules_fallback >= 3);
    }
}
