# rATC 使用手册

[English](MANUAL.en.md) | **[简体中文](MANUAL.md)**

rATC 是一个基于 xray-core 的 Linux 终端代理管理器。本手册覆盖安装、首次配置、各 Tab 操作、快捷键、系统代理、规则兼容性、故障排查与配置参考。

## 目录
1. [安装](#1-安装)
2. [首次运行](#2-首次运行)
3. [界面与 Tab 操作](#3-界面与-tab-操作)
4. [快捷键速查](#4-快捷键速查)
5. [系统代理](#5-系统代理)
6. [规则兼容性](#6-规则兼容性)
7. [故障排查](#7-故障排查)
8. [配置文件参考](#8-配置文件参考)

## 1. 安装

### 1.1 依赖
- Linux（终端或 X11）
- xray-core（推荐 1.8.x，支持 vless+reality）
- （可选）`geoip.dat` / `geosite.dat`，用于 GEOIP 规则

### 1.2 安装 xray-core
```sh
bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install
# 安装到 /usr/local/bin/xray
xray version
```

### 1.3 放置 GEOIP/GEOSITE 数据
```sh
mkdir -p ~/.config/ratc
cd ~/.config/ratc
wget https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geoip.dat
wget https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geosite.dat
```
> rATC 启动 xray 时工作目录为 `~/.config/ratc/`，因此放在该目录即可被 GEOIP 规则命中。

### 1.4 构建 rATC
```sh
cargo build --release
sudo cp target/release/ratc /usr/local/bin/
```

## 2. 首次运行

通过环境变量注入第一个订阅：
```sh
RATC_SUB_URL='http://your-provider/modules/.../sub?token=...' ratc
```
首次启动会把该 URL 存为默认订阅并缓存到 `~/.config/ratc/cache/`。

> **秒开启动**：首次成功拉取后，订阅和规则集会写入本地快照
> （`cache/last_subscription.yaml`、`cache/last_rulesets.json`）。之后每次启动直接读快照，
> **不再联网**。只有首次打开（无快照）或手动刷新（`r` / 订阅页 `u`）才走网络。

> rATC 是交互式 TUI，需要在真实终端（tty / 终端模拟器）中运行。在无 TTY 的环境（如管道、CI）中启动会在初始化终端时退出。

## 3. 界面与 Tab 操作

界面自上而下：状态栏 → Tab 栏 → 内容区 → 反馈行 → 帮助栏。

### 3.1 状态栏
显示 xray 运行状态、当前节点、系统代理开关、节点总数与可用数。

### 3.2 Tab 1 — 节点
- `●` 当前节点；`○` 可选；`✕` 不可用（灰显）
- `j/k`、`↑/↓` 移动；`Enter` 切换并重载 xray
- 列表较长时自动滚动跟随光标；支持翻页（见第 4 节）
- 光标高亮在所有行（含灰显）上都可见
- 切换到不可用节点会被拒绝并记录在日志 Tab

### 3.3 Tab 2 — 订阅（交互式）
- `★` 当前激活订阅，`☆` 普通订阅；已缓存的订阅显示"(已缓存)"
- `↑/↓` 选择条目；`Enter` 激活（优先读本地缓存，无缓存则提示按 `u` 联网）
- `a` 添加：先输入 **URL**，回车；再输入**可选名称**（留空回车则从 URL 域名自动派生，重名加 `-2/-3`）
- `d` 删除当前条目（删的是激活项则自动激活剩余第一条）
- `u` 联网更新当前条目（同时设为激活）

### 3.4 Tab 3 — 规则
只读展示转换前的规则列表。标题栏显示转换统计（成功/跳过/兜底数）。`[skip]` 前缀表示该规则因 xray 不支持被跳过（如 `PROCESS-NAME`）。

### 3.5 Tab 4 — 日志
rATC 自身事件日志（订阅刷新、xray 启动、错误等），按时间倒序。如需 xray 详细日志，运行：

```sh
xray -test -config ~/.config/ratc/xray.json
```

### 3.6 Tab 5 — 设置（可编辑表单）
- `↑/↓` 选择字段；`Enter` 按类型修改：
  - **布尔项**（允许局域网 / 退出时关闭xray / 系统代理）：直接切换
  - **日志级别**：在 `debug/info/warning/error` 间循环
  - **HTTP 端口 / SOCKS 端口 / 监听地址 / xray 路径**：进入内联编辑（预填当前值），`Enter` 保存，`Esc` 取消；端口校验 1–65535
- 改动会立即写入 `config.json`。端口 / 监听 / 日志级别在下次切节点（重建 xray 配置）时生效；xray 路径需重启 rATC 生效

## 4. 快捷键速查

全局：

| 键 | 功能 |
|----|------|
| `1`–`5` | 切换 Tab |
| `r` | 刷新订阅（联网） |
| `s` | 切换系统代理 |
| `q` | 退出 |

导航（节点 / 订阅 / 设置 列表）：

| 键 | 功能 |
|----|------|
| `↑` `↓` / `k` `j` | 上下移动一项 |
| `←` `→` / `PgUp` `PgDn` / `空格` | 上下翻整页 |
| `Ctrl-F` / `Ctrl-B` | 整页下 / 上 |
| `Ctrl-D` / `Ctrl-U` | 半页下 / 上 |
| `g` / `Home` · `G` / `End` | 跳到首 · 尾 |

输入提示时：打字追加，`Backspace` 删除，`Enter` 确认，`Esc` 取消。

## 5. 系统代理

rATC 通过生成 `~/.config/ratc/proxy.sh` 控制终端代理。按 `s` 开启后，在 shell 配置中加载它：

**Bash (`~/.bashrc`) 或 Zsh (`~/.zshrc`)：**
```sh
[ -f ~/.config/ratc/proxy.sh ] && source ~/.config/ratc/proxy.sh
```

之后新开的终端中 `curl`、`git`、`apt` 等会自动走代理。按 `s` 关闭后该文件会写入 `unset`，重开终端即解除。

## 6. 规则兼容性

| Clash 规则 | rATC 支持 |
|-----------|----------|
| DOMAIN / DOMAIN-SUFFIX / DOMAIN-KEYWORD | ✅ |
| IP-CIDR / IP-CIDR6 | ✅ |
| GEOIP | ✅（需 geoip.dat） |
| RULE-SET | ✅（下载并展开，目标由 RULE-SET 引用决定） |
| MATCH | ✅（通过 xray 默认出站实现兜底） |
| PROCESS-NAME | ❌（xray 不支持，跳过） |

无论订阅规则覆盖如何，rATC 始终追加最小兜底规则：私有网段直连、国内域名/GeoIP 直连、其余走代理（默认出站）。

## 7. 故障排查

**启动慢**：仅首次启动联网；之后走本地快照。随时按 `r`（或订阅页 `u`）强制刷新。

**端口被占用**：`lsof -i :10809`，或在设置 Tab 修改 `http_port` / `socks_port`（切节点后生效）。

**xray 启动失败**：进入 Tab 4 查看日志；命令行运行 `xray -test -config ~/.config/ratc/xray.json` 查看 stderr。

**订阅拉取失败**：rATC 会回退到缓存。检查网络，或在 Tab 2 用 `u` 重试。

**GEOIP 规则无效**：确认 `~/.config/ratc/geoip.dat` 存在（见 1.3）。

**节点灰显**：shadow-tls 的 ss 与 hysteria2 协议 xray 不支持，属预期行为。

**首次运行提示"未配置订阅"**：通过 `RATC_SUB_URL='http://...' ./ratc` 注入，或在订阅 Tab 按 `a` 添加。

## 8. 配置文件参考

`~/.config/ratc/config.json`：

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

| 字段 | 说明 |
|------|------|
| `http_port` | xray HTTP 入站端口 |
| `socks_port` | xray SOCKS 入站端口 |
| `listen` | 监听地址（默认仅本机） |
| `xray_path` | xray 二进制路径 |
| `allow_lan` | 是否允许局域网（默认 false） |
| `log_level` | xray 日志级别（debug/info/warning/error） |
| `exit_kills_xray` | 退出 rATC 时是否关闭 xray |
| `sys_proxy_on` | 系统代理开关 |
| `current_proxy` | 当前选中节点名 |
| `subscriptions` | 订阅列表，`active: true` 者生效 |

运行时目录一览：

| 路径 | 用途 |
|------|------|
| `cache/last_subscription.yaml` | 上次拉取的订阅快照（秒开用） |
| `cache/last_rulesets.json` | 已下载的规则集快照 |
| `cache/<hash>.yaml` | 按 URL 的订阅 YAML 缓存 |
| `xray.json` | 生成的 xray 配置 |
| `proxy.sh` | 系统代理加载片段 |
