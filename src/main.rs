use ratc::app::App;
use ratc::config::AppConfig;
use ratc::store::paths::ensure_dirs;
use ratc::tui;

fn main() -> ratc::error::Result<()> {
    ensure_dirs()?;
    let cfg = AppConfig::load()?;
    let mut app = App::init(cfg)?;

    if app.cfg.subscriptions.is_empty() {
        if let Ok(url) = std::env::var("RATC_SUB_URL") {
            app.cfg.subscriptions.push(ratc::config::SubscriptionEntry {
                name: "default".into(), url, active: true,
            });
            app.cfg.save()?;
        } else {
            eprintln!("未配置订阅。请运行: RATC_SUB_URL='http://...' ./ratc");
            eprintln!("或在首次启动前编辑 ~/.config/ratc/config.json");
        }
    }

    app.refresh_subscription()?;
    if app.cfg.current_proxy.is_none() {
        if let Some(p) = app.supported_proxies().first() {
            let name = p.name.clone();
            app.select_proxy(&name)?;
        }
    }
    tui::run(&mut app)?;

    // Honor the "exit kills xray" setting. If false, detach the xray child so
    // it keeps running in the background after rATC exits (XrayHandle::Drop
    // would otherwise kill it).
    if !app.cfg.exit_kills_xray {
        if let Some(x) = app.xray.as_mut() {
            if x.detach() {
                eprintln!("rATC 已退出，xray 仍在后台运行。");
            }
        }
    }
    Ok(())
}
