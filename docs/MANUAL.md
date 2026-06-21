# rATC 使用手册

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
首次启动会把该 URL 存为默认订阅并缓存到 `~/.config/ratc/cache/`。之后无需再带环境变量，直接运行 `ratc` 即可。

> rATC 是交互式 TUI，需要在真实终端（tty / 终端模拟器）中运行。在无 TTY 的环境（如管道、CI）中启动会在初始化终端时退出。

## 3. 界面与 Tab 操作

界面自上而下：状态栏 → Tab 栏 → 内容区 → 帮助栏。

### 3.1 状态栏
显示 xray 运行状态、当前节点、系统代理开关、节点总数与可用数。

### 3.2 Tab 1 — 节点
- `●` 当前节点；`○` 可选；`✕` 不可用（灰显）
- `j/k` 或 `↑/↓` 移动，`Enter` 切换并重载 xray
- 切换到不可用节点会被拒绝并记录在日志 Tab

### 3.3 Tab 2 — 订阅
- `*` 表示当前激活订阅
- 当前版本通过编辑 `~/.config/ratc/config.json` 添加/删除/激活订阅（交互编辑为后续版本计划）

### 3.4 Tab 3 — 规则
只读展示转换前的规则列表。标题栏显示转换统计（成功/跳过/兜底数）。`[skip]` 前缀表示该规则因 xray 不支持被跳过（如 `PROCESS-NAME`）。

### 3.5 Tab 4 — 日志
rATC 自身事件日志（订阅刷新、xray 启动、错误等），按时间倒序。如需 xray 详细日志，运行：

```sh
xray -test -config ~/.config/ratc/xray.json
```

### 3.6 Tab 5 — 设置
只读展示当前配置。修改请编辑 `~/.config/ratc/config.json` 后重启 rATC。

## 4. 快捷键速查

| 键 | 功能 |
|----|------|
| `1`-`5` | 切换 Tab |
| `j` / `↓` | 下一项 |
| `k` / `↑` | 上一项 |
| `Enter` | 选中节点 |
| `r` | 刷新订阅 |
| `s` | 切换系统代理 |
| `q` | 退出 |

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

**端口被占用**：`lsof -i :10809`，或修改 `~/.config/ratc/config.json` 的 `http_port` / `socks_port` 后重启。

**xray 启动失败**：进入 Tab 4 查看日志；命令行运行 `xray -test -config ~/.config/ratc/xray.json` 查看 stderr。

**订阅拉取失败**：rATC 会回退到缓存。检查网络，或在 Tab 2 用 `r` 重试。

**GEOIP 规则无效**：确认 `~/.config/ratc/geoip.dat` 存在（见 1.3）。

**节点灰显**：shadow-tls 的 ss 与 hysteria2 协议 xray 不支持，属预期行为。

**首次运行提示"未配置订阅"**：通过 `RATC_SUB_URL='http://...' ./ratc` 注入，或编辑 `~/.config/ratc/config.json`。

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
| `allow_lan` | 是否允许局域网（暂仅配置项，默认 false） |
| `log_level` | xray 日志级别 |
| `exit_kills_xray` | 退出 rATC 时是否关闭 xray |
| `sys_proxy_on` | 系统代理开关 |
| `current_proxy` | 当前选中节点名 |
| `subscriptions` | 订阅列表，`active: true` 者生效 |
