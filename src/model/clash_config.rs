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
