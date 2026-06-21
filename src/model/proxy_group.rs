use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum GroupType {
    #[serde(rename = "select")]
    Select,
    #[serde(rename = "url-test")]
    UrlTest,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProxyGroup {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: GroupType,
    #[serde(default)]
    pub proxies: Vec<String>,
}

/// Non-proxy informational "groups" (traffic, expiry, package) shown in status bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoEntry {
    pub label: String,
}

impl ProxyGroup {
    /// Heuristic: a select group whose only proxy is `DIRECT` and whose name
    /// carries an info keyword is treated as an info entry, not a real group.
    pub fn as_info(&self) -> Option<InfoEntry> {
        let n = self.name.as_str();
        let is_info = ["流量", "到期", "套餐", "客服", "续费"]
            .iter()
            .any(|k| n.contains(k));
        if matches!(self.kind, GroupType::Select)
            && self.proxies.iter().all(|p| p == "DIRECT")
            && is_info
        {
            Some(InfoEntry { label: n.into() })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_select_group_is_not_info() {
        let g = ProxyGroup {
            name: "🛸 节点选择".into(),
            kind: GroupType::Select,
            proxies: vec!["DIRECT".into(), "a".into()],
        };
        assert!(g.as_info().is_none());
    }

    #[test]
    fn traffic_group_is_info() {
        let g = ProxyGroup {
            name: "⛽ 本月流量 71694MB".into(),
            kind: GroupType::Select,
            proxies: vec!["DIRECT".into()],
        };
        assert!(g.as_info().is_some());
    }
}
