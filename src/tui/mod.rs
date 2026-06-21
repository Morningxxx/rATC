pub mod app_state;
pub mod event;
pub mod tabs;

use crate::app::App;
use crate::error::{Error, Result};
use crate::model::proxy::Compat;
use app_state::{Feedback, InputMode, PendingAction, Tab, UiState};
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
use std::time::Duration;
use tabs::settings::{setting_label, SETTINGS_COUNT};

/// How long a feedback message stays on screen before auto-clearing.
const FEEDBACK_TTL: Duration = Duration::from_secs(4);

pub fn run(app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = UiState::default();
    loop {
        terminal.draw(|f| draw(f, app, &mut ui))?;
        match event::poll(200) {
            Some(event::Message::Tick) => {
                ui.maybe_expire_feedback(FEEDBACK_TTL);
            }
            Some(event::Message::Key(k)) => {
                if handle_key(app, &mut ui, k) {
                    break;
                }
            }
            None => break,
        }
        // Deferred blocking work: repaint a "busy" frame first, then run it,
        // so the user always sees what is happening instead of a frozen UI.
        if let Some(action) = ui.pending.take() {
            terminal.draw(|f| draw(f, app, &mut ui))?;
            process_pending(app, &mut ui, action);
        }
        if !ui.running {
            break;
        }
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn onoff(b: bool) -> &'static str {
    if b {
        "on"
    } else {
        "off"
    }
}

/// Last valid index for the active tab's list.
fn cursor_last(app: &App, tab: Tab) -> usize {
    let len = match tab {
        Tab::Nodes => app.all_proxies().len(),
        Tab::Subscriptions => app.cfg.subscriptions.len(),
        Tab::Settings => SETTINGS_COUNT,
        _ => 0,
    };
    len.saturating_sub(1)
}

fn cursor_mut(ui: &mut UiState) -> Option<&mut usize> {
    match ui.tab {
        Tab::Nodes => Some(&mut ui.selected),
        Tab::Subscriptions => Some(&mut ui.sub_selected),
        Tab::Settings => Some(&mut ui.settings_selected),
        _ => None,
    }
}

/// Move the active tab's cursor by `step` in the given direction.
fn move_cursor(app: &App, ui: &mut UiState, step: usize, down: bool) {
    let last = cursor_last(app, ui.tab);
    if let Some(pos) = cursor_mut(ui) {
        *pos = if down {
            pos.saturating_add(step).min(last)
        } else {
            pos.saturating_sub(step)
        };
    }
}

fn handle_key(app: &mut App, ui: &mut UiState, k: KeyEvent) -> bool {
    // Note: import KeyCode variants explicitly — a glob import would pull in
    // `KeyCode::Tab`, which clashes with our own `app_state::Tab` enum.
    use crossterm::event::KeyModifiers;
    use KeyCode::{Backspace, Char, Down, End, Enter, Home, Left, PageDown, PageUp, Right, Up};

    // While typing into a prompt, all keys go to the buffer.
    if ui.input_mode.is_active() {
        handle_input(app, ui, k);
        return false;
    }

    let full = ui.list_height.max(1);
    let half = (full / 2).max(1);

    // Global keys (work on every tab).
    match k.code {
        Char('q') => {
            ui.running = false;
            return false;
        }
        Char('1') => {
            ui.tab = Tab::Nodes;
            return false;
        }
        Char('2') => {
            ui.tab = Tab::Subscriptions;
            return false;
        }
        Char('3') => {
            ui.tab = Tab::Rules;
            return false;
        }
        Char('4') => {
            ui.tab = Tab::Logs;
            return false;
        }
        Char('5') => {
            ui.tab = Tab::Settings;
            return false;
        }
        Char('r') => {
            ui.pending = Some(PendingAction::Refresh);
            ui.set_feedback(Feedback::Info, "⟳ 正在刷新订阅 …");
            return false;
        }
        Char('s') => {
            ui.pending = Some(PendingAction::ToggleSysProxy);
            ui.set_feedback(Feedback::Info, "⟳ 正在切换系统代理 …");
            return false;
        }
        _ => {}
    }

    // Vim-style Ctrl paging (context-aware).
    if k.modifiers.contains(KeyModifiers::CONTROL) {
        if let Char(c) = k.code {
            match c {
                'f' | 'b' => {
                    move_cursor(app, ui, full, c == 'f');
                    return false;
                }
                'd' | 'u' => {
                    move_cursor(app, ui, half, c == 'd');
                    return false;
                }
                _ => {}
            }
        }
    }

    match k.code {
        Down | Char('j') => {
            move_cursor(app, ui, 1, true);
            false
        }
        Up | Char('k') => {
            move_cursor(app, ui, 1, false);
            false
        }
        PageDown | Char(' ') | Char('J') | Right => {
            move_cursor(app, ui, full, true);
            false
        }
        PageUp | Char('K') | Left => {
            move_cursor(app, ui, full, false);
            false
        }
        Home | Char('g') => {
            if let Some(pos) = cursor_mut(ui) {
                *pos = 0;
            }
            false
        }
        End | Char('G') => {
            let last = cursor_last(app, ui.tab);
            if let Some(pos) = cursor_mut(ui) {
                *pos = last;
            }
            false
        }
        Enter => handle_enter(app, ui),
        Backspace => false,
        _ => match ui.tab {
            Tab::Subscriptions => handle_subs_action(app, ui, k),
            _ => false,
        },
    }
}

/// Enter is tab-specific: pick a node / activate a subscription / edit a setting.
fn handle_enter(app: &mut App, ui: &mut UiState) -> bool {
    match ui.tab {
        Tab::Nodes => {
            let busy = app
                .all_proxies()
                .get(ui.selected)
                .map(|p| format!("⟳ 正在切换到 {} …", p.name))
                .unwrap_or_else(|| "⟳ 正在切换节点 …".to_string());
            ui.set_feedback(Feedback::Info, busy);
            ui.pending = Some(PendingAction::Select(ui.selected));
        }
        Tab::Subscriptions => {
            let name = app
                .cfg
                .subscriptions
                .get(ui.sub_selected)
                .map(|s| s.name.clone());
            ui.pending = Some(PendingAction::SwitchActive(ui.sub_selected));
            ui.set_feedback(
                Feedback::Info,
                name.map(|n| format!("⟳ 正在激活 {n} …"))
                    .unwrap_or_else(|| "⟳ 正在激活 …".into()),
            );
        }
        Tab::Settings => {
            handle_settings_action(app, ui);
        }
        _ => {}
    }
    false
}

/// Subscription-tab-only actions: add / delete / update.
fn handle_subs_action(app: &mut App, ui: &mut UiState, k: KeyEvent) -> bool {
    use KeyCode::Char;
    match k.code {
        Char('a') => {
            ui.input_mode = InputMode::AddSubUrl;
            ui.input_buffer.clear();
            ui.set_feedback(Feedback::Info, "添加订阅 — 请输入 URL:");
        }
        Char('d') => {
            let name = app
                .cfg
                .subscriptions
                .get(ui.sub_selected)
                .map(|s| s.name.clone());
            ui.pending = Some(PendingAction::DeleteSub(ui.sub_selected));
            ui.set_feedback(
                Feedback::Info,
                name.map(|n| format!("⟳ 正在删除 {n} …"))
                    .unwrap_or_else(|| "⟳ 正在删除 …".into()),
            );
        }
        Char('u') => {
            let name = app
                .cfg
                .subscriptions
                .get(ui.sub_selected)
                .map(|s| s.name.clone());
            ui.pending = Some(PendingAction::RefreshSub(ui.sub_selected));
            ui.set_feedback(
                Feedback::Info,
                name.map(|n| format!("⟳ 正在联网更新 {n} …"))
                    .unwrap_or_else(|| "⟳ 正在更新 …".into()),
            );
        }
        _ => {}
    }
    false
}

/// Settings-tab Enter: toggle a boolean, cycle the log level, or start editing
/// a text/numeric field.
fn handle_settings_action(app: &mut App, ui: &mut UiState) -> bool {
    let field = ui.settings_selected;
    match field {
        4 => {
            app.cfg.allow_lan = !app.cfg.allow_lan;
            let _ = app.cfg.save();
            ui.set_feedback(
                Feedback::Ok,
                format!("允许局域网: {}", onoff(app.cfg.allow_lan)),
            );
        }
        6 => {
            app.cfg.exit_kills_xray = !app.cfg.exit_kills_xray;
            let _ = app.cfg.save();
            ui.set_feedback(
                Feedback::Ok,
                format!("退出时关闭xray: {}", onoff(app.cfg.exit_kills_xray)),
            );
        }
        7 => {
            // Reuse the immediate toggle (also flips the OS proxy).
            ui.pending = Some(PendingAction::ToggleSysProxy);
            ui.set_feedback(Feedback::Info, "⟳ 正在切换系统代理 …");
        }
        5 => {
            const LEVELS: [&str; 4] = ["debug", "info", "warning", "error"];
            let cur = LEVELS.iter().position(|l| *l == app.cfg.log_level).unwrap_or(1);
            let next = LEVELS[(cur + 1) % LEVELS.len()];
            app.cfg.log_level = next.into();
            let _ = app.cfg.save();
            ui.set_feedback(Feedback::Ok, format!("日志级别: {next}"));
        }
        0..=3 => {
            ui.input_buffer = current_setting_value(app, field);
            ui.input_mode = InputMode::EditSettings { field };
            ui.set_feedback(
                Feedback::Info,
                format!("编辑 {} (Enter保存 Esc取消)", setting_label(field)),
            );
        }
        _ => {}
    }
    false
}

fn current_setting_value(app: &App, field: usize) -> String {
    match field {
        0 => app.cfg.http_port.to_string(),
        1 => app.cfg.socks_port.to_string(),
        2 => app.cfg.listen.clone(),
        3 => app.cfg.xray_path.clone(),
        _ => String::new(),
    }
}

/// Apply a settings text edit, persisting on success.
fn apply_settings_edit(app: &mut App, field: usize, raw: &str) -> Result<String> {
    let val = raw.trim();
    match field {
        0 => {
            let p: u16 = val
                .parse()
                .map_err(|_| Error::Other("端口号需为 1-65535 的数字".into()))?;
            if p == 0 {
                return Err(Error::Other("端口号不能为 0".into()));
            }
            app.cfg.http_port = p;
            app.cfg.save()?;
            Ok(format!("HTTP 端口 → {p} (切节点后生效)"))
        }
        1 => {
            let p: u16 = val
                .parse()
                .map_err(|_| Error::Other("端口号需为 1-65535 的数字".into()))?;
            if p == 0 {
                return Err(Error::Other("端口号不能为 0".into()));
            }
            app.cfg.socks_port = p;
            app.cfg.save()?;
            Ok(format!("SOCKS 端口 → {p} (切节点后生效)"))
        }
        2 => {
            if val.is_empty() {
                return Err(Error::Other("监听地址不能为空".into()));
            }
            app.cfg.listen = val.into();
            app.cfg.save()?;
            Ok(format!("监听地址 → {val} (切节点后生效)"))
        }
        3 => {
            if val.is_empty() {
                return Err(Error::Other("xray 路径不能为空".into()));
            }
            app.cfg.xray_path = val.into();
            app.cfg.save()?;
            Ok(format!("xray 路径 → {val} (重启 rATC 后生效)"))
        }
        _ => Ok("无需修改".into()),
    }
}

/// Inline text entry: route keys to the input buffer / submit / cancel.
fn handle_input(app: &mut App, ui: &mut UiState, k: KeyEvent) {
    use crossterm::event::KeyModifiers;
    use KeyCode::{Backspace, Char, Enter, Esc};
    match k.code {
        Esc => {
            ui.input_mode = InputMode::None;
            ui.input_buffer.clear();
            ui.set_feedback(Feedback::Info, "已取消输入");
        }
        Backspace => {
            ui.input_buffer.pop();
        }
        Enter => submit_input(app, ui),
        Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) => {
            ui.input_buffer.push(c);
        }
        _ => {}
    }
}

fn submit_input(app: &mut App, ui: &mut UiState) {
    match ui.input_mode.clone() {
        InputMode::AddSubUrl => {
            let url = ui.input_buffer.trim().to_string();
            if url.is_empty() {
                ui.set_feedback(Feedback::Warn, "URL 不能为空（继续输入或 Esc 取消）");
                return;
            }
            if !url.contains("://") {
                ui.set_feedback(Feedback::Warn, "URL 需以 http:// 或 https:// 开头");
                return;
            }
            ui.input_mode = InputMode::AddSubName { url };
            ui.input_buffer.clear();
            ui.set_feedback(Feedback::Info, "可选名称（留空跳过，Enter 确认）:");
        }
        InputMode::AddSubName { url } => {
            let name = ui.input_buffer.trim().to_string();
            let idx = app.add_subscription(&url, &name);
            let disp = app.cfg.subscriptions[idx].name.clone();
            ui.input_mode = InputMode::None;
            ui.input_buffer.clear();
            ui.sub_selected = idx;
            ui.set_feedback(
                Feedback::Ok,
                format!("✓ 已添加订阅 [{disp}]（按 u 联网更新）"),
            );
        }
        InputMode::EditSettings { field } => {
            let val = ui.input_buffer.clone();
            match apply_settings_edit(app, field, &val) {
                Ok(msg) => {
                    ui.input_mode = InputMode::None;
                    ui.input_buffer.clear();
                    ui.set_feedback(Feedback::Ok, msg);
                }
                Err(e) => ui.set_feedback(Feedback::Err, format!("✗ {e}（修改后重试）")),
            }
        }
        InputMode::None => {}
    }
}

/// Execute a deferred action and surface the outcome as feedback.
fn process_pending(app: &mut App, ui: &mut UiState, action: PendingAction) {
    match action {
        PendingAction::Select(idx) => apply_select(app, ui, idx),
        PendingAction::Refresh => match app.refresh_subscription() {
            Ok(()) => ui.set_feedback(Feedback::Ok, "✓ 订阅已刷新"),
            Err(e) => ui.set_feedback(Feedback::Err, format!("✗ 刷新订阅失败: {e}")),
        },
        PendingAction::ToggleSysProxy => match app.toggle_sys_proxy() {
            Ok(()) => ui.set_feedback(
                Feedback::Ok,
                format!("✓ 系统代理: {}", onoff(app.cfg.sys_proxy_on)),
            ),
            Err(e) => ui.set_feedback(Feedback::Err, format!("✗ 切换系统代理失败: {e}")),
        },
        PendingAction::SwitchActive(idx) => {
            let name = app
                .cfg
                .subscriptions
                .get(idx)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            match app.switch_active(idx) {
                Ok(true) => ui.set_feedback(Feedback::Ok, format!("✓ 已激活 [{name}]（本地缓存）")),
                Ok(false) => ui.set_feedback(
                    Feedback::Warn,
                    format!("✓ 已设为活跃 [{name}]，按 u 联网更新"),
                ),
                Err(e) => ui.set_feedback(Feedback::Err, format!("✗ 激活失败: {e}")),
            }
        }
        PendingAction::RefreshSub(idx) => {
            // Make the chosen entry active, then network-refresh.
            let name = app
                .cfg
                .subscriptions
                .get(idx)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            if idx < app.cfg.subscriptions.len() {
                for (i, s) in app.cfg.subscriptions.iter_mut().enumerate() {
                    s.active = i == idx;
                }
                let _ = app.cfg.save();
            }
            match app.refresh_subscription() {
                Ok(()) => ui.set_feedback(Feedback::Ok, format!("✓ 已联网更新 [{name}]")),
                Err(e) => ui.set_feedback(Feedback::Err, format!("✗ 更新失败: {e}")),
            }
        }
        PendingAction::DeleteSub(idx) => {
            let name = app
                .cfg
                .subscriptions
                .get(idx)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            if app.delete_subscription(idx) {
                let last = app.cfg.subscriptions.len().saturating_sub(1);
                if ui.sub_selected > last {
                    ui.sub_selected = last;
                }
                ui.set_feedback(Feedback::Ok, format!("✓ 已删除订阅 [{name}]"));
            } else {
                ui.set_feedback(Feedback::Warn, "无可删除的订阅");
            }
        }
    }
}

/// Select the node at `idx` within the *full* proxy list (matching what the
/// node tab displays), refusing unsupported nodes with a clear message.
fn apply_select(app: &mut App, ui: &mut UiState, idx: usize) {
    let proxies = app.all_proxies();
    let Some(p) = proxies.get(idx) else {
        ui.set_feedback(Feedback::Warn, "⚠ 没有可选择的节点");
        return;
    };
    if !matches!(p.compat(), Compat::Supported) {
        ui.set_feedback(Feedback::Warn, "✕ 该节点不支持，无法选择");
        return;
    }
    let name = p.name.clone();
    match app.select_proxy(&name) {
        Ok(()) => ui.set_feedback(Feedback::Ok, format!("✓ 已切换节点 → {name}")),
        Err(e) => ui.set_feedback(Feedback::Err, format!("✗ 切换失败 [{name}]: {e}")),
    }
}

fn draw(f: &mut ratatui::Frame<'_>, app: &App, ui: &mut UiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(1),
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
            onoff(app.cfg.sys_proxy_on)
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

    // Highlight the active tab (previously this was always stuck on "节点"
    // because the Tabs widget was never told which one was selected).
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| Line::styled(t.title(), Style::default()))
        .collect();
    let tabs_widget = Tabs::new(titles)
        .select(ui.tab.index())
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_widget(tabs_widget, chunks[1]);

    tabs::render(f, app, ui, chunks[2]);

    // Feedback line doubles as the inline input prompt when typing.
    let bottom = if ui.input_mode.is_active() {
        Line::from(Span::styled(
            input_prompt(ui),
            Style::default().fg(Color::Cyan),
        ))
    } else {
        let fb_color = match ui.feedback_kind {
            Feedback::Ok => Color::Green,
            Feedback::Warn => Color::Yellow,
            Feedback::Err => Color::Red,
            Feedback::Info => Color::Cyan,
        };
        if ui.feedback.is_empty() {
            Line::raw("")
        } else {
            Line::from(Span::styled(
                ui.feedback.clone(),
                Style::default().fg(fb_color),
            ))
        }
    };
    f.render_widget(Paragraph::new(bottom), chunks[3]);

    let help = match ui.tab {
        Tab::Nodes => {
            "↑↓/jk←→移动 Enter:选择 r:刷新 s:系统代理 1-5:Tab PgUp/PgDn/Ctrl-F/D翻页 g/G首尾 q:退出"
        }
        Tab::Subscriptions => "↑↓选择 a:添加 d:删除 u:更新 Enter:激活 1-5:Tab q:退出",
        Tab::Settings => "↑↓选择 Enter:修改(布尔切换/级别循环/文本编辑) 1-5:Tab q:退出",
        _ => "1-5:Tab q:退出",
    };
    f.render_widget(Paragraph::new(help), chunks[4]);
}

/// One-line prompt for the active input mode, buffer shown with a cursor block.
fn input_prompt(ui: &UiState) -> String {
    let buf = if ui.input_buffer.is_empty() {
        " ".to_string()
    } else {
        ui.input_buffer.clone()
    };
    match &ui.input_mode {
        InputMode::AddSubUrl => format!("URL: {buf}█"),
        InputMode::AddSubName { .. } => format!("名称(留空跳过): {buf}█"),
        InputMode::EditSettings { field } => format!("{}: {buf}█", setting_label(*field)),
        InputMode::None => String::new(),
    }
}
