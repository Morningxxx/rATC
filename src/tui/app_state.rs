#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab { Nodes, Subscriptions, Rules, Logs, Settings }

impl Tab {
    pub fn all() -> [Tab; 5] {
        [Tab::Nodes, Tab::Subscriptions, Tab::Rules, Tab::Logs, Tab::Settings]
    }
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Nodes => "[1]节点", Tab::Subscriptions => "[2]订阅",
            Tab::Rules => "[3]规则", Tab::Logs => "[4]日志", Tab::Settings => "[5]设置",
        }
    }
}

pub struct UiState {
    pub tab: Tab,
    pub selected: usize,
    pub running: bool,
}

impl Default for UiState {
    fn default() -> Self { Self { tab: Tab::Nodes, selected: 0, running: true } }
}
