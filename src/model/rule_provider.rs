use serde::Deserialize;
#[allow(unused_imports)]
pub use crate::model::rule_set_file::RuleSetFile;

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
