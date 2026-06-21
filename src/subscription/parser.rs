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
    pub skipped_proxies: usize,
}

pub fn parse(text: &str) -> Result<ParsedSubscription> {
    let cfg: ClashConfig =
        serde_yaml::from_str(text).map_err(|e| Error::Parse(format!("yaml: {e}")))?;
    let group_names: HashSet<String> = cfg.proxy_groups.iter().map(|g| g.name.clone()).collect();
    let proxies: Vec<Proxy> = cfg.proxies.iter().filter_map(classify).collect();
    let skipped = cfg.proxies.len().saturating_sub(proxies.len());
    let rules: Vec<Rule> = cfg
        .rules
        .iter()
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
        skipped_proxies: skipped,
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
        std::fs::read_to_string("tests/fixtures/clash_meta.yaml")
            .or_else(|_| std::fs::read_to_string("../tests/fixtures/clash_meta.yaml"))
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
        let supported = p
            .proxies
            .iter()
            .filter(|x| x.compat() == Compat::Supported)
            .count();
        // US-Xr1(vless), JP-Ws(vmess), TW-Trojan(trojan) → 3 supported
        assert_eq!(supported, 3);
    }

    #[test]
    fn rules_parsed() {
        let p = parse(&fixture()).unwrap();
        assert!(p
            .rules
            .iter()
            .any(|r| matches!(r, Rule::DomainSuffix(d, Target::Direct) if d=="cn")));
        assert!(p
            .rules
            .iter()
            .any(|r| matches!(r, Rule::RuleSet(n, Target::Direct) if n=="CNdirect1")));
        assert!(p.rules.iter().any(|r| matches!(r, Rule::Unsupported(_, _))));
    }
}
