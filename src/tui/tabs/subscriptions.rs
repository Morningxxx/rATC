use crate::app::App;
use crate::tui::app_state::UiState;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

pub fn render(f: &mut Frame<'_>, app: &App, _ui: &UiState, area: Rect) {
    let items: Vec<ListItem> = app
        .cfg
        .subscriptions
        .iter()
        .map(|s| {
            let star = if s.active { "*" } else { " " };
            ListItem::new(Line::raw(format!(" {star} {:<14} {}", s.name, s.url)))
        })
        .collect();
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("订阅列表 [a]添加 [d]删除 [u]更新 [Enter]激活"),
    );
    f.render_widget(list, area);
}
