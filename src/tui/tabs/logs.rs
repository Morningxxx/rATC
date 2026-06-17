use crate::app::App;
use crate::tui::app_state::UiState;
use ratatui::widgets::{Block, Borders, Paragraph};
pub fn render(f: &mut ratatui::Frame<'_>, _app: &App, _ui: &UiState, area: ratatui::layout::Rect) {
    f.render_widget(Paragraph::new("todo").block(Block::default().borders(Borders::ALL).title("日志")), area);
}
