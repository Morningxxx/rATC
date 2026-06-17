use crate::error::Result;
use crate::model::rule::{Rule, Target};
use crate::model::rule_set_file::RuleSetFile;
use crate::subscription::parser::target_tag;
use serde_json::{json, Value};
use std::collections::HashSet;

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
        let file: RuleSetFile = serde_yaml::from_str(&text)?;
        Ok(file.payload)
    }

    /// Convert a classical rule-set's payload lines into xray rules with the
    /// given target. Lines that fail to parse are skipped.
    pub fn expand(lines: &[String], target: Target, group_names: &HashSet<String>) -> Vec<Value> {
        let tag = target_tag(target);
        lines.iter()
            .filter_map(|l| {
                // classical rule-set payload lines omit the target field that
                // Rule::parse requires; supply a placeholder so they classify
                // (the target is overridden by `target` below).
                let l = l.trim();
                let normalized = if l.split(',').count() < 3 { format!("{l},DIRECT") } else { l.to_string() };
                Rule::parse(&normalized, group_names)
            })
            .filter_map(|r| match r {
                Rule::Domain(v, _) => Some(json!({"domain": [format!("full:{v}")], "outboundTag": tag})),
                Rule::DomainSuffix(v, _) => Some(json!({"domain": [format!("domain:{v}")], "outboundTag": tag})),
                Rule::DomainKeyword(v, _) => Some(json!({"domain": [format!("keyword:{v}")], "outboundTag": tag})),
                Rule::IpCidr(v, _, _) => Some(json!({"ip": [v], "outboundTag": tag})),
                Rule::GeoIp(v, _) => Some(json!({"ip": [format!("geoip:{v}")], "outboundTag": tag})),
                _ => None,
            })
            .collect()
    }
}

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
        assert_eq!(v[0]["domain"][0], "domain:baidu.com");
        assert_eq!(v[0]["outboundTag"], "direct");
        assert_eq!(v[2]["ip"][0], "1.2.3.0/24");
    }
}
