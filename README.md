# rATC — Rust Agent for Terminal Clash

**[English](README.md)** | [简体中文](README.zh-CN.md)

[![Release](https://img.shields.io/github/v/release/Morningxxx/rATC?color=blue)](https://github.com/Morningxxx/rATC/releases)
[![License](https://img.shields.io/badge/license-MIT-green)](#license)

A Linux terminal (TUI) proxy manager in the spirit of Clash Verge, backed by
[xray-core](https://github.com/XTLS/Xray-core). It fetches Clash Meta YAML
subscriptions, converts nodes and routing rules into xray configs, and drives the
xray subprocess from a [Ratatui](https://ratatui.rs/) interface.

> **v0.1.0** — first formal release. See [CHANGELOG](#changelog) below.

## Features

- Clash Meta YAML subscription parsing with on-disk caching (falls back to cache when offline)
- **Instant startup** — the last subscription is snapshotted locally; the network is only hit on the first ever launch or a manual refresh
- Node switching with per-protocol compatibility markers (unsupported nodes greyed out)
- Long node lists auto-scroll and paginate; the cursor is always visible (even on greyed rows)
- **Interactive Subscriptions tab** — add / delete / update / activate, with multi-subscription support
- **Interactive Settings form** — toggle booleans, cycle the log level, edit ports / listen address / xray path inline
- Automatic Clash → xray-core rule conversion with a minimal fallback rule set
- xray-core lifecycle management (start / reload on node switch)
- Local HTTP `10809` + SOCKS `10808` inbound
- Optional system-proxy toggle via a sourced `proxy.sh`
- 5-tab TUI: 节点 (Nodes) / 订阅 (Subscriptions) / 规则 (Rules) / 日志 (Logs) / 设置 (Settings)

## Requirements

- Linux (terminal or X11)
- [xray-core](https://github.com/XTLS/Xray-core) installed and on `PATH` (or at `/usr/local/bin/xray`); v1.8+ recommended for vless+reality
- (optional) `geoip.dat` / `geosite.dat` for GEOIP rules — place in `~/.config/ratc/`

Install xray-core:

```sh
bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install
```

## Build

```sh
cargo build --release
# binary: target/release/ratc
```

Or grab a prebuilt binary from the [Releases](https://github.com/Morningxxx/rATC/releases).

## Quick Start

1. Build (see above) or download the release binary.
2. Add your subscription on first run:

   ```sh
   RATC_SUB_URL='http://your-provider/sub?token=...' ./target/release/ratc
   ```

3. Inside the TUI: press `1` for nodes, navigate with `j`/`k` (or arrows), `Enter` to switch.
4. (Optional) enable the system proxy for terminal apps — press `s`, then add to `~/.bashrc` / `~/.zshrc`:

   ```sh
   [ -f ~/.config/ratc/proxy.sh ] && source ~/.config/ratc/proxy.sh
   ```

5. Press `q` to quit.

> rATC is an interactive TUI and must run in a real terminal (tty). Starting it in a
> non-TTY environment (pipes, CI) exits during terminal init.

## Keyboard Reference

Global:

| Key | Action |
|-----|--------|
| `1`–`5` | Switch tabs |
| `r` | Refresh subscription (network) |
| `s` | Toggle system proxy |
| `q` | Quit |

Navigation (works on the Nodes / Subscriptions / Settings lists):

| Key | Action |
|-----|--------|
| `↑` `↓` / `k` `j` | Move one item |
| `←` `→` / `PgUp` `PgDn` / `Space` | Move a full page |
| `Ctrl-F` / `Ctrl-B` | Full page down / up (vim) |
| `Ctrl-D` / `Ctrl-U` | Half page down / up (vim) |
| `g` / `Home` · `G` / `End` | Jump to top · bottom |

Per-tab actions:

| Tab | Key | Action |
|-----|-----|--------|
| Nodes | `Enter` | Select node (restarts xray) |
| Subscriptions | `Enter` | Activate (loads local cache) |
| Subscriptions | `a` | Add — type URL, then an optional name |
| Subscriptions | `d` | Delete |
| Subscriptions | `u` | Network update |
| Settings | `Enter` | Toggle bool / cycle log level / edit field |

While typing into a prompt: type to add, `Backspace` to delete, `Enter` to confirm, `Esc` to cancel.

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
- `RULE-SET` providers are downloaded and expanded inline.
- `PROCESS-NAME` rules are unsupported by xray and skipped (counted in the Rules tab).
- The `MATCH` catch-all is handled by xray's default outbound (the active node).
- A minimal fallback set is always appended: private nets → direct, CN domains/IPs → direct, everything else → proxy.

## Configuration

Runtime data lives under `~/.config/ratc/`:

| Path | Purpose |
|------|---------|
| `config.json` | App config (ports, xray path, subscriptions, current node) |
| `proxy.sh` | System-proxy source snippet |
| `cache/` | Per-URL subscription YAML cache |
| `cache/last_subscription.yaml` | Snapshot of the last fetched subscription (fast startup) |
| `cache/last_rulesets.json` | Snapshot of downloaded rule-set payloads |
| `ruleset/` | Rule-set cache |
| `xray.json` | Generated xray config |
| `logs/` | Application logs |

## FAQ

**Q: Startup is slow.** Only the first launch fetches over the network; subsequent launches load the local snapshot. Force a refresh anytime with `r` (or `u` in the Subscriptions tab).

**Q: xray won't start.** Press `4` (logs) for messages; verify the binary path in Settings. Debug with:

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

**Q: First run prints "未配置订阅".** Provide a subscription via `RATC_SUB_URL='http://...' ./ratc`, or add one in the Subscriptions tab (`a`).

For the full guide, see **[docs/MANUAL.en.md](docs/MANUAL.en.md)**.

## Changelog

### v0.1.0
- First formal release.
- Interactive Subscriptions tab (add / delete / update / activate) and Settings form.
- Local subscription snapshot → instant startup, network only on first launch / manual refresh.
- Node list: visible cursor on all rows, auto-scroll & pagination.
- Added navigation keys: `←/→`, `PgUp/PgDn`, `Ctrl-F/B` (full page), `Ctrl-D/U` (half page), `g/G`, `Home/End`.
- Default ports aligned to `10809` / `10808`.

## License

MIT
