use crate::app::App;
use crate::tui::app_state::{InputMode, UiState};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// Number of editable settings rows (kept in sync with `setting_label`).
pub const SETTINGS_COUNT: usize = 8;

pub fn setting_label(i: usize) -> &'static str {
    match i {
        0 => "HTTP 端口",
        1 => "SOCKS 端口",
        2 => "监听地址",
        3 => "xray 路径",
        4 => "允许局域网",
        5 => "日志级别",
        6 => "退出时关闭xray",
        7 => "系统代理",
        _ => "",
    }
}

fn yn(b: bool) -> &'static str {
    if b {
        "[x]"
    } else {
        "[ ]"
    }
}

fn current_value(app: &App, i: usize) -> String {
    match i {
        0 => app.cfg.http_port.to_string(),
        1 => app.cfg.socks_port.to_string(),
        2 => app.cfg.listen.clone(),
        3 => app.cfg.xray_path.clone(),
        4 => yn(app.cfg.allow_lan).into(),
        5 => app.cfg.log_level.clone(),
        6 => yn(app.cfg.exit_kills_xray).into(),
        7 => if app.cfg.sys_proxy_on { "on" } else { "off" }.into(),
        _ => String::new(),
    }
}

pub fn render(f: &mut Frame<'_>, app: &App, ui: &mut UiState, area: Rect) {
    if SETTINGS_COUNT > 0 {
        ui.settings_selected = ui.settings_selected.min(SETTINGS_COUNT - 1);
    }
    let editing = match &ui.input_mode {
        InputMode::EditSettings { field } => Some(*field),
        _ => None,
    };

    let lines: Vec<Line> = (0..SETTINGS_COUNT)
        .map(|i| {
            let marker = if i == ui.settings_selected { "▶" } else { " " };
            let label = setting_label(i);
            let selected = i == ui.settings_selected;
            let mut spans = vec![Span::raw(format!(" {marker} {label:<14}: "))];

            if editing == Some(i) {
                // Show the live edit buffer with a cursor block.
                let buf = if ui.input_buffer.is_empty() {
                    " ".to_string()
                } else {
                    ui.input_buffer.clone()
                };
                spans.push(Span::styled(
                    format!("[{buf}█]"),
                    Style::default().fg(Color::Black).bg(Color::Yellow),
                ));
            } else {
                let val = current_value(app, i);
                let val_color = match i {
                    4 | 6 => Color::Green, // booleans
                    7 => Color::Green,
                    _ if selected => Color::Yellow,
                    _ => Color::default(),
                };
                spans.push(Span::styled(format!("[{val}]"), Style::default().fg(val_color)));
            }

            if selected {
                Line::from(spans).style(Style::default().add_modifier(Modifier::BOLD))
            } else {
                Line::from(spans)
            }
        })
        .collect();

    let title = "设置 (↑↓选择  Enter:布尔切换/级别循环/文本编辑)";
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}
