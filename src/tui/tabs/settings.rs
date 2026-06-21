use crate::app::App;
use crate::tui::app_state::UiState;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame<'_>, app: &App, _ui: &UiState, area: Rect) {
    let c = &app.cfg;
    let lines = vec![
        Line::raw(format!(" HTTP 端口:      [{}]", c.http_port)),
        Line::raw(format!(" SOCKS 端口:     [{}]", c.socks_port)),
        Line::raw(format!(" 监听地址:       [{}]", c.listen)),
        Line::raw(format!(" xray 路径:      [{}]", c.xray_path)),
        Line::raw(format!(
            " 允许局域网:     [{}]",
            if c.allow_lan { "x" } else { " " }
        )),
        Line::raw(format!(" 日志级别:       [{}]", c.log_level)),
        Line::raw(format!(
            " 退出时关闭xray: [{}]",
            if c.exit_kills_xray { "x" } else { " " }
        )),
        Line::raw(format!(
            " 系统代理:       [{}]",
            if c.sys_proxy_on { "on" } else { "off" }
        )),
    ];
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("设置 (编辑 ~/.config/ratc/config.json)"),
        ),
        area,
    );
}
