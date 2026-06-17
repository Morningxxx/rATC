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
            _ => Rule::Unsupported(line.to_string(), target_of(parts.get(2).unwrap_or(&"PROXY"))),
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
