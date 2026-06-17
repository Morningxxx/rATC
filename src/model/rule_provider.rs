use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RuleProvider {
    pub behavior: String,
    #[serde(rename = "type")]
    pub kind: String, // "http" | "file"
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub interval: u64,
}

/// A downloaded rule-set file. Clash classical rule-sets have a `payload:` list
/// of rule lines; domain/ip rule-sets have `payload:` list of plain values.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RuleSetFile {
    #[serde(default)]
    pub payload: Vec<String>,
}
