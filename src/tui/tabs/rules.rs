use crate::app::App;
use crate::model::rule::Rule;
use crate::tui::app_state::UiState;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

pub fn render(f: &mut Frame<'_>, app: &App, _ui: &UiState, area: Rect) {
    let Some(sub) = &app.sub else {
        f.render_widget(
            Block::default().borders(Borders::ALL).title("路由规则"),
            area,
        );
        return;
    };
    let items: Vec<ListItem> = sub
        .rules
        .iter()
        .map(|r| {
            let line = match r {
                Rule::Domain(v, _) => format!("domain        {v}"),
                Rule::DomainSuffix(v, _) => format!("domainSuffix  {v}"),
                Rule::DomainKeyword(v, _) => format!("domainKeyword {v}"),
                Rule::IpCidr(v, _, _) => format!("ipCidr        {v}"),
                Rule::GeoIp(v, _) => format!("geoIp         {v}"),
                Rule::RuleSet(v, _) => format!("RULE-SET      {v}"),
                Rule::Match(_) => "MATCH (catch-all via default outbound)".into(),
                Rule::Unsupported(t, _) => format!("[skip] {t}"),
            };
            ListItem::new(Line::raw(line))
        })
        .collect();
    let title = format!(
        "路由规则 成功{} 跳过{} 兜底{}",
        app.last_stats.rules_ok, app.last_stats.rules_skipped, app.last_stats.rules_fallback
    );
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(list, area);
}
