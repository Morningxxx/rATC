use ratc::app::App;
use ratc::config::AppConfig;
use ratc::store::paths::ensure_dirs;
use ratc::tui;

fn main() -> ratc::error::Result<()> {
    ensure_dirs()?;
    let cfg = AppConfig::load()?;
    let mut app = App::init(cfg)?;
    app.refresh_subscription()?;
    // auto-select first supported proxy if none chosen
    if app.cfg.current_proxy.is_none() {
        if let Some(p) = app.supported_proxies().first() {
            let name = p.name.clone();
            app.select_proxy(&name)?;
        }
    }
    tui::run(&mut app)?;
    Ok(())
}
