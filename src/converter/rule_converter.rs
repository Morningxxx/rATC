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
