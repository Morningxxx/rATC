# rATC — Rust Agent for Terminal Clash

[English](README.md) | **[简体中文](README.zh-CN.md)**

[![Release](https://img.shields.io/github/v/release/Morningxxx/rATC?color=blue)](https://github.com/Morningxxx/rATC/releases)
[![License](https://img.shields.io/badge/license-MIT-green)](#许可证)

一个 Linux 终端 (TUI) 代理管理器,风格类似 Clash Verge,底层基于
[xray-core](https://github.com/XTLS/Xray-core)。它拉取 Clash Meta YAML 订阅,
把节点和路由规则转换成 xray 配置,并通过 [Ratatui](https://ratatui.rs/) 界面驱动 xray 子进程。

> **v0.1.0** — 首个正式版。更新见下方 [更新日志](#更新日志)。

## 功能特性

- Clash Meta YAML 订阅解析,带本地磁盘缓存(离线时自动回退缓存)
- **秒开启动** —— 上次的订阅会本地快照保存;只有首次启动或手动刷新才联网
- 节点切换,按协议显示兼容性标记(不支持的节点置灰)
- 节点列表超长时自动滚动翻页;光标始终可见(灰显行也清晰)
- **订阅 Tab 全交互** —— 添加 / 删除 / 更新 / 激活,支持多订阅
- **设置 Tab 可编辑表单** —— 切换布尔项、循环日志级别、内联编辑端口 / 监听地址 / xray 路径
- 自动 Clash → xray-core 规则转换,带最小兜底规则集
- xray-core 生命周期管理(启动 / 切节点时热重载)
- 本地 HTTP `10809` + SOCKS `10808` 入站
- 可选系统代理开关,通过 `proxy.sh` 加载
- 5 个 Tab:节点 / 订阅 / 规则 / 日志 / 设置

## 环境要求

- Linux(终端或 X11)
- 已安装 [xray-core](https://github.com/XTLS/Xray-core) 且在 `PATH` 中(或位于 `/usr/local/bin/xray`);推荐 v1.8+ 以支持 vless+reality
- (可选)`geoip.dat` / `geosite.dat`,用于 GEOIP 规则 —— 放到 `~/.config/ratc/`

安装 xray-core:

```sh
bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install
```

## 构建

```sh
cargo build --release
# 产物:target/release/ratc
```

或从 [Releases](https://github.com/Morningxxx/rATC/releases) 下载预编译二进制。

## 快速上手

1. 按上文构建,或下载 release 二进制。
2. 首次运行注入订阅:

   ```sh
   RATC_SUB_URL='http://your-provider/sub?token=...' ./target/release/ratc
   ```

3. 进入 TUI:按 `1` 进节点页,用 `j`/`k`(或方向键)移动,`Enter` 切换。
4. (可选)为终端程序开启系统代理 —— 按 `s`,然后在 `~/.bashrc` / `~/.zshrc` 加入:

   ```sh
   [ -f ~/.config/ratc/proxy.sh ] && source ~/.config/ratc/proxy.sh
   ```

5. 按 `q` 退出。

> rATC 是交互式 TUI,必须在真实终端(tty)中运行。在无 TTY 环境(管道、CI)中启动会在初始化终端时退出。

## 快捷键速查

全局:

| 键 | 功能 |
|----|------|
| `1`–`5` | 切换 Tab |
| `r` | 刷新订阅(联网) |
| `s` | 系统代理开关 |
| `q` | 退出 |

导航(适用于 节点 / 订阅 / 设置 列表):

| 键 | 功能 |
|----|------|
| `↑` `↓` / `k` `j` | 上下移动一项 |
| `←` `→` / `PgUp` `PgDn` / `空格` | 上下翻整页 |
| `Ctrl-F` / `Ctrl-B` | 整页下 / 上(vim) |
| `Ctrl-D` / `Ctrl-U` | 半页下 / 上(vim) |
| `g` / `Home` · `G` / `End` | 跳到首 · 尾 |

各 Tab 操作:

| Tab | 键 | 功能 |
|----|----|------|
| 节点 | `Enter` | 选择节点(重启 xray) |
| 订阅 | `Enter` | 激活(读本地缓存) |
| 订阅 | `a` | 添加 —— 先输 URL,再输可选名称 |
| 订阅 | `d` | 删除 |
| 订阅 | `u` | 联网更新 |
| 设置 | `Enter` | 切换布尔 / 循环日志级别 / 编辑字段 |

输入提示时:打字追加,`Backspace` 删除,`Enter` 确认,`Esc` 取消。

## 协议兼容性

| 协议 | xray 支持 |
|------|-----------|
| vless(+reality / +ws) | ✅ |
| vmess(+ws / +tls) | ✅ |
| shadowsocks(无插件) | ✅ |
| trojan | ✅ |
| shadowsocks + shadow-tls | ❌(跳过 —— 服务端需要插件) |
| hysteria2 | ❌(跳过 —— xray 未实现) |

不支持的节点会置灰,并从生成的 xray 配置中排除。

## 路由说明

- Clash 规则转换为 xray 原生路由:`ip`(CIDR + `geoip:xx`)与 `domain`(`full:` / `domain:` / `keyword:` 前缀)。
- `RULE-SET` 提供者会被下载并在原位展开。
- `PROCESS-NAME` 规则 xray 不支持,跳过(在规则 Tab 计数)。
- `MATCH` 兜底由 xray 默认出站(当前节点)实现。
- 始终追加最小兜底规则:私有网段直连、国内域名/IP 直连、其余走代理。

## 配置

运行时数据位于 `~/.config/ratc/`:

| 路径 | 用途 |
|------|------|
| `config.json` | 应用配置(端口、xray 路径、订阅、当前节点) |
| `proxy.sh` | 系统代理加载片段 |
| `cache/` | 按 URL 的订阅 YAML 缓存 |
| `cache/last_subscription.yaml` | 上次拉取的订阅快照(用于秒开) |
| `cache/last_rulesets.json` | 已下载的规则集快照 |
| `ruleset/` | 规则集缓存 |
| `xray.json` | 生成的 xray 配置 |
| `logs/` | 应用日志 |

## 常见问题

**Q:启动慢。** 只有首次启动会联网;之后都走本地快照。随时按 `r`(或订阅页 `u`)强制刷新。

**Q:xray 启动失败。** 按 `4`(日志)查看信息;在设置页核对 xray 路径。命令行调试:

```sh
xray -test -config ~/.config/ratc/xray.json
```

**Q:GEOIP 规则无效。** 把 `geoip.dat` / `geosite.dat` 放到 `~/.config/ratc/`:

```sh
cd ~/.config/ratc
wget https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geoip.dat
wget https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geosite.dat
```

**Q:hysteria2 / shadow-tls 节点是灰的。** xray-core 不支持这些协议,故主动跳过。

**Q:首次运行提示"未配置订阅"。** 通过 `RATC_SUB_URL='http://...' ./ratc` 注入,或在订阅 Tab 按 `a` 添加。

完整手册见 **[docs/MANUAL.md](docs/MANUAL.md)**。

## 更新日志

### v0.1.0
- 首个正式版。
- 订阅 Tab 全交互(添加 / 删除 / 更新 / 激活);设置 Tab 可编辑表单。
- 订阅本地快照 —— 秒开启动,仅首次启动 / 手动刷新才联网。
- 节点列表:所有行可见光标、自动滚动翻页。
- 新增导航键:`←/→`、`PgUp/PgDn`、`Ctrl-F/B`(整页)、`Ctrl-D/U`(半页)、`g/G`、`Home/End`。
- 默认端口对齐 `10809` / `10808`。

## 许可证

MIT
