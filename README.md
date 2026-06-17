# rATC — Rust Agent for Terminal Clash

A Linux terminal (TUI) proxy manager in the spirit of Clash Verge, backed by [xray-core](https://github.com/XTLS/Xray-core). It fetches Clash Meta YAML subscriptions, converts nodes and routing rules into xray configs, and drives the xray subprocess from a Ratatui interface.

## Features

- Clash Meta YAML subscription parsing & on-disk caching (falls back to cache when offline)
- Node switching with per-protocol compatibility markers (unsupported nodes greyed out)
- Automatic Clash → xray-core rule conversion with a minimal fallback rule set
- xray-core lifecycle management (start / reload on node switch)
- Local HTTP (7890) + SOCKS (7891) inbound
- Optional system-proxy toggle via a sourced `proxy.sh`
- 5-tab TUI: 节点 / 订阅 / 规则 / 日志 / 设置

## Requirements

- Linux (terminal or X11). xray-core installed and on `PATH` (or at `/usr/local/bin/xray`)
- `geoip.dat` / `geosite.dat` recommended for GEOIP rules (place in `~/.config/ratc/`)

Install xray-core:

```sh
bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install
```

## Build

```sh
cargo build --release
# binary: target/release/ratc
```

## Quick Start

1. Build (see above) or download the release binary.
2. Add your subscription on first run:

   ```sh
   RATC_SUB_URL='http://your-provider/sub?token=...' ./target/release/ratc
   ```

3. Inside the TUI: press `1` for nodes, navigate with `j`/`k`, `Enter` to switch.
4. (Optional) enable system proxy for terminal apps — press `s`, then add to your `~/.bashrc` or `~/.zshrc`:

   ```sh
   [ -f ~/.config/ratc/proxy.sh ] && source ~/.config/ratc/proxy.sh
   ```

5. Press `q` to quit.

## Keyboard Reference

| Key | Action |
|-----|--------|
| `1`-`5` | Switch tabs |
| `j` / `k`, `↑` / `↓` | Navigate list |
| `Enter` | Select node (Nodes tab) |
| `r` | Refresh subscription |
| `s` | Toggle system proxy |
| `q` | Quit |

## Protocol Compatibility

| Protocol | xray support |
|----------|--------------|
| vless (+reality / +ws) | ✅ |
| vmess (+ws / +tls) | ✅ |
| shadowsocks (no plugin) | ✅ |
| trojan | ✅ |
| shadowsocks + shadow-tls | ❌ (skipped — server requires the plugin) |
| hysteria2 | ❌ (skipped — not implemented by xray) |

Unsupported nodes are shown greyed out and excluded from the generated xray config.

## Routing Notes

- Clash rules are converted to xray-native routing: `ip` (CIDR + `geoip:xx`) and `domain` (with `full:` / `domain:` / `keyword:` prefixes).
- `PROCESS-NAME` rules are unsupported by xray and are skipped (counted in the Rules tab).
- The `MATCH` catch-all is handled by xray's default outbound (the active node), not a field rule.
- A minimal fallback set is always appended: private nets → direct, CN domains/IPs → direct, everything else → proxy.

## Configuration

Runtime data lives under `~/.config/ratc/`:

| Path | Purpose |
|------|---------|
| `config.json` | App config (ports, xray path, subscriptions, current node) |
| `proxy.sh` | System-proxy source snippet |
| `cache/` | Subscription YAML cache |
| `ruleset/` | Rule-set cache |
| `xray.json` | Generated xray config |
| `logs/` | Application logs |

## FAQ

**Q: xray won't start.** Press `4` (logs) for messages; verify the binary path in Settings (`~/.config/ratc/config.json` → `xray_path`). Debug with:

```sh
xray -test -config ~/.config/ratc/xray.json
```

**Q: GEOIP rules fail.** Download `geoip.dat` / `geosite.dat` into `~/.config/ratc/`:

```sh
cd ~/.config/ratc
wget https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geoip.dat
wget https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geosite.dat
```

**Q: My hysteria2 / shadow-tls nodes are grey.** xray-core does not implement these protocols; they are intentionally skipped.

**Q: First run prints "未配置订阅".** Provide a subscription via `RATC_SUB_URL='http://...' ./ratc`, or edit `~/.config/ratc/config.json` directly.

## License

MIT
