pub mod logs;
pub mod nodes;
pub mod rules;
pub mod settings;
pub mod subscriptions;

use crate::app::App;
use crate::tui::app_state::UiState;

pub fn render(f: &mut ratatui::Frame<'_>, app: &App, ui: &mut UiState, area: ratatui::layout::Rect) {
    use crate::tui::app_state::Tab;
    match ui.tab {
        Tab::Nodes => nodes::render(f, app, ui, area),
        Tab::Subscriptions => subscriptions::render(f, app, ui, area),
        Tab::Rules => rules::render(f, app, ui, area),
        Tab::Logs => logs::render(f, app, ui, area),
        Tab::Settings => settings::render(f, app, ui, area),
    }
}
