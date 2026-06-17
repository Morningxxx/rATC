use ratc::converter::proxy_converter::to_outbound;
use ratc::converter::xray_config::build_config;
use ratc::subscription::parser::parse;

fn fixture() -> String {
    std::fs::read_to_string("tests/fixtures/clash_meta.yaml").unwrap()
}

#[test]
fn generated_config_passes_xray_test() {
    let bin = std::path::Path::new("/usr/local/bin/xray");
    if !bin.is_file() { return; }
    let sub = parse(&fixture()).unwrap();
    let active = sub.proxies.iter().find(|p| p.name == "US-Xr1").unwrap();
    let (cfg, _stats) = build_config(&sub, active, 7890, 7891, &Default::default()).unwrap();
    // ensure proxy outbound present
    assert!(to_outbound(active).unwrap().is_some());
    // run xray -test
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    let out = std::process::Command::new(bin)
        .arg("-test").arg("-config").arg(tmp.path())
        .output().unwrap();
    assert!(out.status.success(), "xray stderr: {}", String::from_utf8_lossy(&out.stderr));
}
