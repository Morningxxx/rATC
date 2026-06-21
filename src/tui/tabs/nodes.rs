use crate::app::App;
use crate::model::proxy::{Compat, Proxy, ProxyType};
use crate::tui::app_state::UiState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

fn kind_label(p: &Proxy) -> &'static str {
    match &p.ptype {
        ProxyType::Vless {
            reality: Some(_), ..
        } => "vless+reality",
        ProxyType::Vless { .. } => "vless",
        ProxyType::Vmess { ws: Some(_), .. } => "vmess+ws",
        ProxyType::Vmess { .. } => "vmess",
        ProxyType::Shadowsocks { plugin: None, .. } => "ss",
        ProxyType::Trojan { .. } => "trojan",
        _ => "unsupported",
    }
}

pub fn render(f: &mut Frame<'_>, app: &App, ui: &mut UiState, area: Rect) {
    let proxies: Vec<&Proxy> = app.all_proxies();
    let total = proxies.len();
    let supported = app.supported_proxies().len();
    let current = app.cfg.current_proxy.as_deref();

    // Keep the selection inside the current list; the proxy set can shrink on
    // refresh, so a stale index would otherwise point past the end.
    if total > 0 {
        ui.selected = ui.selected.min(total - 1);
    }

    let items: Vec<ListItem> = proxies
        .iter()
        .map(|p| {
            let mark = match p.compat() {
                Compat::Supported => {
                    if Some(p.name.as_str()) == current {
                        "●"
                    } else {
                        "○"
                    }
                }
                Compat::Unsupported(_) => "✕",
            };
            // Per-row color encodes proxy *state* only. The navigational cursor
            // is drawn separately by List::highlight_style so it is visible on
            // every row — including grayed-out unsupported ones.
            let row_style = match p.compat() {
                Compat::Unsupported(_) => Style::default().fg(Color::DarkGray),
                _ if Some(p.name.as_str()) == current => Style::default().fg(Color::Cyan),
                _ => Style::default(),
            };
            let line = Line::from(vec![
                Span::raw(format!(" {mark}  ")),
                Span::raw(format!("{:<28}", p.name)),
                Span::raw(format!(" {:<14}", kind_label(p))),
                Span::raw(format!(" {}", p.port)),
            ]);
            ListItem::new(line).style(row_style)
        })
        .collect();

    let title = format!(" 节点列表  共 {total}  可用 {supported} ");
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    // Sync the persistent state; ratatui adjusts the offset to keep the
    // selected row in view, which is what gives us scrolling/pagination.
    ui.list_state.select(Some(ui.selected));
    ui.list_height = Block::default().borders(Borders::ALL).inner(area).height as usize;
    f.render_stateful_widget(list, area, &mut ui.list_state);
}
