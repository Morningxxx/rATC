use crate::app::App;
use crate::model::proxy::{Compat, Proxy, ProxyType};
use crate::tui::app_state::UiState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
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

pub fn render(f: &mut Frame<'_>, app: &App, ui: &UiState, area: Rect) {
    let proxies: Vec<&Proxy> = app
        .sub
        .as_ref()
        .map(|s| s.proxies.iter().collect())
        .unwrap_or_default();
    let current = app.cfg.current_proxy.as_deref();
    let items: Vec<ListItem> = proxies
        .iter()
        .enumerate()
        .map(|(i, p)| {
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
            let style = if matches!(p.compat(), Compat::Unsupported(_)) {
                Style::default().fg(Color::DarkGray)
            } else if i == ui.selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            let line = Line::from(vec![
                Span::raw(format!(" {mark}  ")),
                Span::raw(format!("{:<28}", p.name)),
                Span::raw(format!(" {:<16}", kind_label(p))),
                Span::raw(format!(" {}", p.port)),
            ]);
            ListItem::new(line).style(style)
        })
        .collect();
    let title = format!(
        "节点列表 (共{} 可用{})",
        proxies.len(),
        app.supported_proxies().len()
    );
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(list, area);
}
