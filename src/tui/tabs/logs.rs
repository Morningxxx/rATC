use crate::app::App;
use crate::tui::app_state::UiState;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

pub fn render(f: &mut Frame<'_>, app: &App, _ui: &UiState, area: Rect) {
    let items: Vec<ListItem> = app.logs.iter().rev().take(200).map(|l| ListItem::new(Line::raw(l.clone()))).collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("日志"));
    f.render_widget(list, area);
}
