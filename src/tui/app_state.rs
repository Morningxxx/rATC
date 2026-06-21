use ratatui::widgets::ListState;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Nodes,
    Subscriptions,
    Rules,
    Logs,
    Settings,
}

impl Tab {
    pub fn all() -> [Tab; 5] {
        [
            Tab::Nodes,
            Tab::Subscriptions,
            Tab::Rules,
            Tab::Logs,
            Tab::Settings,
        ]
    }
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Nodes => "[1]节点",
            Tab::Subscriptions => "[2]订阅",
            Tab::Rules => "[3]规则",
            Tab::Logs => "[4]日志",
            Tab::Settings => "[5]设置",
        }
    }
    /// Position within `Tab::all()` — used to highlight the active tab.
    pub fn index(&self) -> usize {
        Tab::all().iter().position(|t| t == self).unwrap_or(0)
    }
}

/// Visual class of a transient feedback message (drives its color).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Feedback {
    #[default]
    Info,
    Ok,
    Warn,
    Err,
}

/// A blocking action deferred from key handling so the loop can repaint a
/// "busy" frame before actually performing it. This keeps the UI from looking
/// frozen during slow work (xray restart, network fetch, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingAction {
    /// Select the node at this index in the full proxy list.
    Select(usize),
    Refresh,
    ToggleSysProxy,
    /// Activate the subscription at this index (loads local cache, no network).
    SwitchActive(usize),
    /// Network-refresh the subscription at this index (also makes it active).
    RefreshSub(usize),
    /// Delete the subscription at this index.
    DeleteSub(usize),
}

/// Inline text-entry state, shared by "add subscription" and editing settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    None,
    /// Step 1 of adding a subscription: typing the URL.
    AddSubUrl,
    /// Step 2: an optional name (URL captured from step 1).
    AddSubName { url: String },
    /// Editing settings field at `field` (index into the settings list).
    EditSettings { field: usize },
}

impl InputMode {
    pub fn is_active(&self) -> bool {
        !matches!(self, InputMode::None)
    }
}

pub struct UiState {
    pub tab: Tab,
    pub selected: usize,
    pub running: bool,
    /// Drives the node list's scroll offset; persisted so the view only moves
    /// when the selection actually leaves the viewport (ratatui auto-scrolls).
    pub list_state: ListState,
    /// Inner height of the node list, captured during render so PageUp/PageDown
    /// can jump by a full page.
    pub list_height: usize,
    /// Transient one-line feedback shown above the help line.
    pub feedback: String,
    pub feedback_kind: Feedback,
    pub feedback_at: Instant,
    pub pending: Option<PendingAction>,
    /// Per-tab cursor rows (Nodes reuses `selected`).
    pub sub_selected: usize,
    pub settings_selected: usize,
    pub sub_list_state: ListState,
    /// Inline text entry: which field is being typed and the buffer contents.
    pub input_mode: InputMode,
    pub input_buffer: String,
}

impl UiState {
    pub fn set_feedback(&mut self, kind: Feedback, msg: impl Into<String>) {
        self.feedback = msg.into();
        self.feedback_kind = kind;
        self.feedback_at = Instant::now();
    }

    /// Clear feedback once it has been on screen past `ttl`. Returns whether it
    /// was cleared (so the caller knows a redraw is warranted).
    pub fn maybe_expire_feedback(&mut self, ttl: Duration) -> bool {
        if !self.feedback.is_empty() && self.feedback_at.elapsed() >= ttl {
            self.feedback.clear();
            true
        } else {
            false
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            tab: Tab::Nodes,
            selected: 0,
            running: true,
            list_state: ListState::default(),
            list_height: 0,
            feedback: String::new(),
            feedback_kind: Feedback::Info,
            feedback_at: Instant::now(),
            pending: None,
            sub_selected: 0,
            settings_selected: 0,
            sub_list_state: ListState::default(),
            input_mode: InputMode::None,
            input_buffer: String::new(),
        }
    }
}
