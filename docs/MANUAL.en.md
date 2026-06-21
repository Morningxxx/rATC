# rATC User Manual

**[English](MANUAL.en.md)** | [简体中文](MANUAL.md)

rATC is a Linux terminal proxy manager built on xray-core. This manual covers installation, first-run setup, per-tab operation, keyboard shortcuts, the system proxy, rule compatibility, troubleshooting, and the configuration reference.

## Table of Contents
1. [Installation](#1-installation)
2. [First Run](#2-first-run)
3. [UI & Tab Operation](#3-ui--tab-operation)
4. [Keyboard Reference](#4-keyboard-reference)
5. [System Proxy](#5-system-proxy)
6. [Rule Compatibility](#6-rule-compatibility)
7. [Troubleshooting](#7-troubleshooting)
8. [Configuration Reference](#8-configuration-reference)

## 1. Installation

### 1.1 Dependencies
- Linux (terminal or X11)
- xray-core (1.8.x recommended, for vless+reality)
- (optional) `geoip.dat` / `geosite.dat` for GEOIP rules

### 1.2 Install xray-core
```sh
bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install
# installs to /usr/local/bin/xray
xray version
```

### 1.3 Place GEOIP/GEOSITE data
```sh
mkdir -p ~/.config/ratc
cd ~/.config/ratc
wget https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geoip.dat
wget https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geosite.dat
```
> rATC launches xray with `~/.config/ratc/` as the working directory, so placing the data here lets GEOIP rules resolve.

### 1.4 Build rATC
```sh
cargo build --release
sudo cp target/release/ratc /usr/local/bin/
```

## 2. First Run

Inject your first subscription via an environment variable:
```sh
RATC_SUB_URL='http://your-provider/modules/.../sub?token=...' ratc
```
On first launch the URL is stored as the default subscription and cached under `~/.config/ratc/cache/`.

> **Instant startup**: after the first successful fetch, the subscription and rule-sets are written to a
> local snapshot (`cache/last_subscription.yaml`, `cache/last_rulesets.json`). Every subsequent launch
> reads the snapshot and **does not hit the network**. Only the first launch (no snapshot) or a manual
> refresh (`r` / `u` in the Subscriptions tab) goes online.

> rATC is an interactive TUI and must run in a real terminal (tty). Starting it in a non-TTY environment
> (pipes, CI) exits during terminal init.

## 3. UI & Tab Operation

Top to bottom: status bar → tab bar → content area → feedback line → help line.

### 3.1 Status bar
Shows xray status, current node, the system-proxy toggle, and total / supported node counts.

### 3.2 Tab 1 — Nodes (节点)
- `●` current node; `○` selectable; `✕` unsupported (greyed)
- Move with `j/k`, `↑/↓`; `Enter` switches and reloads xray
- Long lists auto-scroll to follow the cursor; paging is supported (see section 4)
- The cursor highlight is visible on every row, including greyed ones
- Selecting an unsupported node is rejected and logged

### 3.3 Tab 2 — Subscriptions (订阅, interactive)
- `★` active subscription, `☆` others; cached ones show "(已缓存)"
- `↑/↓` select an entry; `Enter` activates (reads local cache first; if none, prompts to press `u`)
- `a` add: type the **URL**, Enter; then an **optional name** (leave empty + Enter to auto-derive from the URL host; collisions get a `-2/-3` suffix)
- `d` delete the current entry (if it was active, the first remaining becomes active)
- `u` network-update the current entry (also activates it)

### 3.4 Tab 3 — Rules (规则)
Read-only view of the pre-conversion rule list. The title bar shows conversion stats (ok / skipped / fallback). A `[skip]` prefix means the rule was dropped because xray doesn't support it (e.g. `PROCESS-NAME`).

### 3.5 Tab 4 — Logs (日志)
rATC's own event log (subscription refresh, xray start, errors…) in reverse-chronological order. For verbose xray logs:

```sh
xray -test -config ~/.config/ratc/xray.json
```

### 3.6 Tab 5 — Settings (设置, editable form)
- `↑/↓` select a field; `Enter` edits by type:
  - **Booleans** (allow-LAN / kill xray on exit / system proxy): toggle directly
  - **Log level**: cycles through `debug/info/warning/error`
  - **HTTP port / SOCKS port / listen address / xray path**: inline edit (prefilled), `Enter` to save, `Esc` to cancel; ports validated 1–65535
- Changes are written to `config.json` immediately. Ports / listen / log level take effect on the next node switch (xray config rebuild); the xray path takes effect after restarting rATC.

## 4. Keyboard Reference

Global:

| Key | Action |
|----|------|
| `1`–`5` | Switch tabs |
| `r` | Refresh subscription (network) |
| `s` | Toggle system proxy |
| `q` | Quit |

Navigation (Nodes / Subscriptions / Settings lists):

| Key | Action |
|----|------|
| `↑` `↓` / `k` `j` | Move one item |
| `←` `→` / `PgUp` `PgDn` / `Space` | Move a full page |
| `Ctrl-F` / `Ctrl-B` | Full page down / up |
| `Ctrl-D` / `Ctrl-U` | Half page down / up |
| `g` / `Home` · `G` / `End` | Jump to top · bottom |

While typing into a prompt: type to add, `Backspace` to delete, `Enter` to confirm, `Esc` to cancel.

## 5. System Proxy

rATC drives the terminal proxy by generating `~/.config/ratc/proxy.sh`. After pressing `s` to enable, load it in your shell config:

**Bash (`~/.bashrc`) or Zsh (`~/.zshrc`):**
```sh
[ -f ~/.config/ratc/proxy.sh ] && source ~/.config/ratc/proxy.sh
```

In new terminals, `curl`, `git`, `apt`, etc. will then go through the proxy. Pressing `s` again writes an `unset` to the file; reopen the terminal to clear it.

## 6. Rule Compatibility

| Clash rule | rATC support |
|-----------|----------|
| DOMAIN / DOMAIN-SUFFIX / DOMAIN-KEYWORD | ✅ |
| IP-CIDR / IP-CIDR6 | ✅ |
| GEOIP | ✅ (needs geoip.dat) |
| RULE-SET | ✅ (downloaded & expanded; target from the RULE-SET reference) |
| MATCH | ✅ (fallback via xray's default outbound) |
| PROCESS-NAME | ❌ (xray doesn't support; skipped) |

Regardless of the subscription's rule coverage, rATC always appends a minimal fallback set: private nets → direct, CN domains/GeoIP → direct, everything else → proxy (default outbound).

## 7. Troubleshooting

**Slow startup.** Only the first launch goes online; afterwards the local snapshot is used. Force a refresh anytime with `r` (or `u` in the Subscriptions tab).

**Port in use.** `lsof -i :10809`, or change `http_port` / `socks_port` in the Settings tab (effective after the next node switch).

**xray won't start.** Open Tab 4 for logs; run `xray -test -config ~/.config/ratc/xray.json` to see stderr.

**Subscription fetch failed.** rATC falls back to cache. Check the network, or retry with `u` in Tab 2.

**GEOIP rules ineffective.** Make sure `~/.config/ratc/geoip.dat` exists (see 1.3).

**Greyed nodes.** ss with shadow-tls and hysteria2 are not implemented by xray — expected behaviour.

**First run shows "未配置订阅".** Inject via `RATC_SUB_URL='http://...' ./ratc`, or add one in the Subscriptions tab (`a`).

## 8. Configuration Reference

`~/.config/ratc/config.json`:

```json
{
  "http_port": 10809,
  "socks_port": 10808,
  "listen": "127.0.0.1",
  "xray_path": "/usr/local/bin/xray",
  "allow_lan": false,
  "log_level": "warning",
  "exit_kills_xray": true,
  "sys_proxy_on": false,
  "current_proxy": "US-Xr1",
  "subscriptions": [
    { "name": "default", "url": "http://.../sub?token=...", "active": true }
  ]
}
```

| Field | Description |
|------|------|
| `http_port` | xray HTTP inbound port |
| `socks_port` | xray SOCKS inbound port |
| `listen` | Listen address (localhost only by default) |
| `xray_path` | xray binary path |
| `allow_lan` | Allow LAN (default false) |
| `log_level` | xray log level (debug/info/warning/error) |
| `exit_kills_xray` | Kill xray when rATC exits |
| `sys_proxy_on` | System-proxy toggle |
| `current_proxy` | Selected node name |
| `subscriptions` | Subscription list; the `active: true` one is used |

Runtime directory overview:

| Path | Purpose |
|------|------|
| `cache/last_subscription.yaml` | Snapshot of the last fetched subscription (for instant startup) |
| `cache/last_rulesets.json` | Snapshot of downloaded rule-set payloads |
| `cache/<hash>.yaml` | Per-URL subscription YAML cache |
| `xray.json` | Generated xray config |
| `proxy.sh` | System-proxy source snippet |
