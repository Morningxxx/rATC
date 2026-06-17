use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RuleSetFile {
    #[serde(default)]
    pub payload: Vec<String>,
}
