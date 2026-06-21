pub mod app_state;
pub mod event;
pub mod tabs;

use crate::app::App;
use crate::error::Result;
use app_state::{Tab, UiState};
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Terminal;

pub fn run(app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = UiState::default();
    loop {
        terminal.draw(|f| draw(f, app, &ui))?;
        match event::poll(200) {
            Some(event::Message::Tick) => {}
            Some(event::Message::Key(k)) => {
                if handle_key(app, &mut ui, k) {
                    break;
                }
            }
            None => break,
        }
        if !ui.running {
            break;
        }
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn handle_key(app: &mut App, ui: &mut UiState, k: KeyEvent) -> bool {
    use KeyCode::{Char, Down, Enter, Up};
    match k.code {
        Char('q') => {
            ui.running = false;
            false
        }
        Char('1') => {
            ui.tab = Tab::Nodes;
            false
        }
        Char('2') => {
            ui.tab = Tab::Subscriptions;
            false
        }
        Char('3') => {
            ui.tab = Tab::Rules;
            false
        }
        Char('4') => {
            ui.tab = Tab::Logs;
            false
        }
        Char('5') => {
            ui.tab = Tab::Settings;
            false
        }
        Char('r') => {
            if let Err(e) = app.refresh_subscription() {
                app.push_log(format!("刷新订阅失败: {e}"));
            }
            false
        }
        Char('s') => {
            if let Err(e) = app.toggle_sys_proxy() {
                app.push_log(format!("切换系统代理失败: {e}"));
            }
            false
        }
        Down | Char('j') => {
            ui.selected = ui.selected.saturating_add(1);
            false
        }
        Up | Char('k') => {
            if ui.selected > 0 {
                ui.selected -= 1;
            }
            false
        }
        Enter => {
            if ui.tab == Tab::Nodes {
                let proxies = app.supported_proxies();
                if let Some(p) = proxies.get(ui.selected.min(proxies.len().saturating_sub(1))) {
                    let name = p.name.clone();
                    if let Err(e) = app.select_proxy(&name) {
                        app.push_log(format!("切换节点失败 [{name}]: {e}"));
                    }
                }
            }
            false
        }
        _ => false,
    }
}

fn draw(f: &mut ratatui::Frame<'_>, app: &App, ui: &UiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(f.area());

    let xray_status = if app.xray_running {
        "●running"
    } else {
        "○stopped"
    };
    let cur = app.cfg.current_proxy.clone().unwrap_or_else(|| "-".into());
    let status = Line::from(vec![
        Span::raw("[Status] xray:"),
        Span::styled(
            xray_status,
            Style::default().fg(if app.xray_running {
                Color::Green
            } else {
                Color::DarkGray
            }),
        ),
        Span::raw(format!(
            "  proxy:{cur}  sys_proxy:{}",
            if app.cfg.sys_proxy_on { "on" } else { "off" }
        )),
    ]);
    let info = app
        .sub
        .as_ref()
        .map(|s| {
            format!(
                "节点:{} 可用:{}",
                s.proxies.len(),
                app.supported_proxies().len()
            )
        })
        .unwrap_or_else(|| "无订阅".into());
    let para = Paragraph::new(vec![status, Line::from(format!("[Info] {info}"))])
        .block(Block::default().borders(Borders::ALL).title("rATC"));
    f.render_widget(para, chunks[0]);

    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let style = if *t == ui.tab {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            Line::styled(t.title(), style)
        })
        .collect();
    f.render_widget(Tabs::new(titles), chunks[1]);

    tabs::render(f, app, ui, chunks[2]);

    let help = "q:退出  r:刷新订阅  s:系统代理  1-5:Tab  ↑↓/jk:导航  Enter:选择";
    f.render_widget(Paragraph::new(help), chunks[3]);
}
