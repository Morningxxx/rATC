use crate::app::App;
use crate::tui::app_state::UiState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

pub fn render(f: &mut Frame<'_>, app: &App, ui: &mut UiState, area: Rect) {
    let n = app.cfg.subscriptions.len();
    if n > 0 {
        ui.sub_selected = ui.sub_selected.min(n - 1);
    }
    let active_url = app.cfg.active_subscription().map(|s| s.url.as_str());

    let items: Vec<ListItem> = app
        .cfg
        .subscriptions
        .iter()
        .map(|s| {
            let star = if s.active { "★" } else { "☆" };
            let style = if s.active {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            let line = Line::from(vec![
                Span::raw(format!(" {star} ")),
                Span::raw(format!("{:<16}", s.name)),
                Span::raw(format!(" {}", s.url)),
            ]);
            ListItem::new(line).style(style)
        })
        .collect();

    let cached_note = if let Some(url) = active_url {
        let p = crate::subscription::fetcher::Fetcher::cache_path_for(url);
        if p.exists() {
            "  (已缓存)"
        } else {
            ""
        }
    } else {
        ""
    };
    let title = format!("订阅列表  共 {n}{cached_note}  [a]添加 [d]删除 [u]更新 [Enter]激活");

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    ui.sub_list_state.select((n > 0).then_some(ui.sub_selected));
    f.render_stateful_widget(list, area, &mut ui.sub_list_state);
}
