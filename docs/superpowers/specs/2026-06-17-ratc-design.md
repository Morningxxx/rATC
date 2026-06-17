# rATC 设计文档

- **项目代号**: rATC (Rust Agent for Terminal Clash)
- **日期**: 2026-06-17
- **状态**: 已确认，待实现
- **技术栈**: Rust + Ratatui + xray-core

## 1. 概述

### 1.1 目标

构建一个 Linux 终端（TUI）代理管理工具，类似桌面版 Clash Verge，功能包括：

1. **订阅管理**：拉取并解析 Clash Meta YAML 订阅
2. **节点切换**：在 TUI 中选择/切换代理节点
3. **xray 生命周期管理**：生成 xray 配置、启动/停止/重启 xray 进程
4. **本地入站**：提供 HTTP + SOCKS 代理端口（xray 无 mixed 协议，用两个独立 inbound）
5. **系统代理**：自动设置/取消 `http_proxy`/`https_proxy` 环境变量
6. **路由规则**：把 Clash 规则尽可能转换为 xray routing，不兼容项降级兜底

### 1.2 非目标 (YAGNI)

- 不做 TUN/透明代理（未来可扩展）
- 不做多用户/远程管理（单用户本地工具）
- 不自建代理协议实现（依赖 xray-core）
- 不做 Windows/macOS 适配（仅 Linux）

### 1.3 运行模式

单进程交互式 TUI。TUI 启动时拉起 xray 子进程，退出 TUI 时可选择保留或关闭 xray。配置和状态持久化到 `~/.config/ratc/`。

## 2. 高层架构

```
┌─────────────────────────────────────────────┐
│                  rATC (Rust)                │
│  ┌───────────┐   ┌──────────────────────┐   │
│  │  TUI 层   │◄─►│      核心层 (Core)    │   │
│  │ (Ratatui) │   │  ┌────────────────┐  │   │
│  └───────────┘   │  │ 订阅管理器      │  │   │
│                  │  │ 配置生成器      │  │   │
│                  │  │ xray 进程管理   │  │   │
│                  │  │ 系统代理控制    │  │   │
│                  │  │ 规则转换器      │  │   │
│                  │  └────────────────┘  │   │
│                  └──────────┬───────────┘   │
└─────────────────────────────┼───────────────┘
                              ▼
                    ┌──────────────────┐
                    │  xray-core (子进程) │
                    │  :7890 http      │
                    │  :7891 socks     │
                    └──────────────────┘
```

## 3. 核心组件

每个组件单一职责、可独立测试。

| 组件 | 职责 | 依赖 |
|------|------|------|
| **`subscription`** | 拉取订阅 URL、缓存、解析 Clash YAML 为内部数据模型 | `reqwest`, `serde_yaml` |
| **`model`** | 统一数据模型：`Proxy`, `ProxyGroup`, `Rule`, `RuleProvider` | 无（纯数据） |
| **`converter`** | Clash 规则 → xray routing；节点 → xray outbound | `model` |
| **`xray`** | 生成 xray JSON 配置；启动/停止/重启 xray 子进程；读取日志 | `tokio::process` |
| **`sysproxy`** | 设置/取消系统代理环境变量 | 平台 API |
| **`config`** | 应用自身配置：订阅列表、当前选中节点、端口、路径等 | `serde` |
| **`store`** | 持久化：`~/.config/ratc/` 下的配置、缓存订阅、规则集缓存 | `std::fs` |
| **`app`** | 应用状态机：协调上述组件，供 TUI 调用 | 全部 |
| **`tui`** | Ratatui 渲染与键盘事件分发 | `app` |

### 3.1 目录结构

```
rATC/
├── Cargo.toml
├── README.md
├── docs/
│   └── MANUAL.md            # 使用手册
├── src/
│   ├── main.rs
│   ├── app.rs               # 状态机与协调
│   ├── model/               # 数据模型
│   ├── subscription/        # 订阅拉取与解析
│   ├── converter/           # Clash→xray 转换
│   ├── xray/                # xray 进程管理
│   ├── sysproxy/            # 系统代理
│   ├── store/               # 持久化
│   └── tui/                 # Ratatui UI
└── tests/
    └── fixtures/
        └── clash_meta.yaml  # 脱敏的真实订阅样本
```

### 3.2 数据流（切换节点示例）

```
TUI 按键选择 "DaWang-HK-Xr1"
   │
   ▼
app.set_current_proxy("DaWang-HK-Xr1")
   │
   ├─► converter 根据 model 重新生成 xray config JSON
   ├─► store 持久化选中节点
   ├─► xray.reload()  (热重启：重启子进程)
   └─► TUI 刷新状态栏
```

## 4. 订阅解析与协议支持

### 4.1 统一节点模型

```rust
pub enum ProxyType {
    Vless { uuid: String, network: String, tls: bool, servername: Option<String>, reality: Option<RealityOpts>, flow: Option<String>, ws: Option<WsOpts> },
    Vmess { uuid: String, alter_id: u32, cipher: String, network: String, tls: bool, ws: Option<WsOpts> },
    Shadowsocks { password: String, cipher: String, plugin: Option<PluginOpts> },
    Hysteria2 { password: String, sni: String, ports: Option<String>, skip_cert_verify: bool },
    Trojan { password: String, sni: String },
}

pub struct Proxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub ptype: ProxyType,
}
```

### 4.2 协议兼容矩阵

| Clash 类型 | xray 支持情况 | 处理 |
|-----------|--------------|------|
| `vless` + reality | ✅ 原生支持 | 正常转换 |
| `vless` + tcp/tls | ✅ 原生支持 | 正常转换 |
| `vmess` + ws (+tls) | ✅ 原生支持 | 正常转换 |
| `ss` + shadow-tls | ❌ 不支持插件 | 标记不可用，跳过（服务端要求 shadow-tls，剥离插件无法连通） |
| `ss` (无插件) | ✅ 原生支持 | 正常转换 |
| `hysteria2` | ❌ 不支持 | 标记不可用，跳过 |

### 4.3 协议兼容处理策略

1. **可转换的节点**（vless/vmess/ss-无插件/trojan）→ 正常生成 xray outbound，UI 正常显示
2. **不兼容**（ss+shadow-tls、hysteria2）→ 标记 `❌ 不支持`，节点列表灰显，转换跳过

> 注：ss+shadow-tls 必须跳过——服务端强制要求 shadow-tls 封装，剥离插件后客户端无法连通。本设计只分"可用/不可用"两态，不引入"部分兼容"以免误导。

UI 用颜色/标记区分三种状态。

### 4.4 订阅解析流程

```
subscription.fetch(url)
   │
   ├─► HTTP GET（UA: clash.meta，超时 15s）
   ├─► 判断格式：尝试 serde_yaml 解析
   │     ├─ 成功 → Clash YAML 路径
   │     └─ 失败 → 尝试 base64 解码（未来扩展）
   ├─► 解析 proxies[] → Vec<Proxy>
   ├─► 解析 proxy-groups[] → Vec<ProxyGroup>（仅 select/url-test）
   └─► 缓存到 ~/.config/ratc/cache/<订阅hash>.yaml
```

### 4.5 订阅元数据

非代理的 group（流量信息、套餐、到期时间等）作为 `InfoGroup` 解析，在 UI 顶部状态栏展示，不作为节点。

## 5. 规则转换与 xray 配置生成

### 5.1 Clash 规则 → xray routing 映射

| Clash 规则 | xray 映射 | 兼容性 |
|-----------|----------|--------|
| `DOMAIN,x,Proxy` | `{domain:"x", outboundTag:"Proxy"}` | ✅ |
| `DOMAIN-SUFFIX,x,DIRECT` | `{domainSuffix:"x", outboundTag:"direct"}` | ✅ |
| `DOMAIN-KEYWORD,x,Proxy` | `{domainKeyword:"x", outboundTag:"Proxy"}` | ✅ |
| `IP-CIDR,x/8,DIRECT` | `{ipCidr:["x/8"], outboundTag:"direct"}` | ✅ |
| `IP-CIDR6,::/127,DIRECT` | `{ipCidr:["::/127"], outboundTag:"direct"}` | ✅ |
| `GEOIP,CN,DIRECT` | `{geoIp:"cn", outboundTag:"direct"}`（需 geoip.dat） | ✅ |
| `MATCH,Proxy` | fallback 规则放最后 | ✅ |
| `RULE-SET,xxx,Proxy` | 展开规则集内规则到数组 | ⚠️ |
| `PROCESS-NAME,X,DIRECT` | ❌ xray 不支持 → 跳过并记录 | ❌ |

### 5.2 Rule-Set 处理

1. 启动/首次拉取时按 `rule-providers` URL 下载 YAML
2. 解析每个规则集内的 `payload[]`（classical 行式规则）
3. 就地**展开**转换，插入主规则数组对应位置
4. 缓存到 `~/.config/ratc/ruleset/<name>.yaml`，按 `interval`（86400s）定时刷新
5. 下载失败 → 跳过并警告（不影响启动）

### 5.3 最小兜底规则（核心约束）

规则转换覆盖率不足时，始终追加最小兜底规则保证可用：

```
1. 私有网段 → direct   (10.0.0.0/8, 192.168.0.0/16 等)
2. 国内域名 → direct   (domainSuffix: cn, com.cn)
3. 国内 GeoIP → direct  (geoip:cn，需 geoip.dat)
4. 其余全部 → proxy    (fallback MATCH)
```

转换失败的规则（PROCESS-NAME 等）只记录日志，不阻断。

### 5.4 xray 配置结构

```json
{
  "log": { "loglevel": "warning" },
  "inbounds": [
    { "tag": "http-in",  "port": 7890, "listen": "127.0.0.1", "protocol": "http" },
    { "tag": "socks-in", "port": 7891, "listen": "127.0.0.1", "protocol": "socks" }
  ],
  "outbounds": [
    { "tag": "proxy", "protocol": "vless", ... },
    { "tag": "direct", "protocol": "freedom" },
    { "tag": "block", "protocol": "blackhole" }
  ],
  "routing": {
    "domainStrategy": "IPIfNonMatch",
    "rules": [ /* 转换+展开+兜底后 */ ]
  }
}
```

> 注：xray-core 没有 sing-box 的 `mixed` 协议，故 HTTP(7890) 与 SOCKS(7891) 用两个 inbound，与原订阅 `port:7890` / `socks-port:7891` 对应。

### 5.5 出站策略

- 同一时刻仅 **1 个活跃代理出站**（tag=`proxy`），即当前选中节点
- 切换节点时重新生成 outbounds[0]，重载 xray
- **不生成所有节点 outbound**（80+ 节点会让 config 巨大且产生空闲连接）
- proxy-groups select 类型：记录 UI 选择为当前节点；url-test：首选项为默认

## 6. TUI 界面设计

### 6.1 整体布局

```
┌─ rATC ───────────────────────────────────────────────────────┐
│ [Status] xray:●running  proxy:DaWang-HK-Xr1  ↑12K ↓340K/s    │  状态栏
│ [Info] 流量:71694MB  到期:2026-09-26  套餐:100云V2R一年        │
├──────────────────────────────────────────────────────────────┤
│ Tabs: [1]节点  [2]订阅  [3]规则  [4]日志  [5]设置             │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│                    (当前 Tab 内容区)                          │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ q:退出  r:刷新订阅  s:系统代理:on  Enter:选择  ?:帮助        │  帮助栏
└──────────────────────────────────────────────────────────────┘
```

### 6.2 Tab 1: 节点列表

```
┌─ 节点列表 ─────────────────────────────────┐
│  ●  DaWang-HK-Xr1     vless+reality  4411  │  当前选中
│  ○  DaWang-HK-Xr2     vless+reality  4412  │
│  ✕  DaWang-HK-2022tlsv2-1  ss+shadowtls 443│  不可用(灰显)
│  ✕  DaWang-HK-Hy2-1   hysteria2  不支持    │  不可用(灰显)
└────────────────────────────────────────────┘
```

- `↑/↓` 或 `j/k` 导航，`Enter` 切换节点（触发 xray 重载）
- `/` 搜索过滤（输入 `HK` 过滤香港）
- `t` 延迟测试：TCP 握手测速，右侧显示 ms

### 6.3 Tab 2: 订阅管理

```
┌─ 订阅列表 ─────────────────────────────────┐
│  * dawangidc    http://dawangidc.org/...    │  当前
│    上次更新: 2026-06-17 12:00               │
│    节点数: 85  可用: 68  不兼容: 17         │
│  [a]添加  [d]删除  [u]更新  [Enter]激活    │
└────────────────────────────────────────────┘
```

- 多订阅支持，激活其中一个
- `u` 重新拉取刷新

### 6.4 Tab 3: 规则查看

```
┌─ 路由规则 (转换后) ────────────────────────┐
│ #   类型          值                → 出站  │
│ 1   domainSuffix  cn                → direct│
│ 2   domainKeyword google            → proxy │
│ 3   ipCidr        10.0.0.0/8        → direct│
│ 转换统计: 成功 142  跳过 8  兜底规则 4      │
└────────────────────────────────────────────┘
```

只读展示转换后规则，底部统计让用户知道哪些被跳过。

### 6.5 Tab 4: 日志

- xray 子进程实时日志（级别着色）
- 应用自身日志（订阅拉取失败、规则转换警告等）

### 6.6 Tab 5: 设置

```
  HTTP 端口:     [7890]
  SOCKS 端口:    [7891]
  监听地址:      [127.0.0.1]
  xray 路径:     [/usr/local/bin/xray]
  允许局域网:    [ ]
  日志级别:      [warning]
  退出时关闭xray:[x]
```

### 6.7 全局按键

| 键 | 功能 |
|----|------|
| `1-5` | 切换 Tab |
| `q` | 退出（可选保留 xray） |
| `r` | 刷新当前订阅 |
| `s` | 切换系统代理开/关 |
| `t` | 测速当前节点 |
| `?` | 帮助 |
| `Esc` | 返回/取消 |

## 7. 系统代理控制

### 7.1 环境变量方式（主要）

无法直接改父进程环境，采用**生成 source 片段**方案：

- 开启时写入 `~/.config/ratc/proxy.sh`：
  ```sh
  export http_proxy="http://127.0.0.1:7890"
  export https_proxy="http://127.0.0.1:7890"
  export no_proxy="localhost,127.0.0.1,::1,10.0.0.0/8,192.168.0.0/16,*.cn"
  ```
- 提示用户在 `.bashrc`/`.zshrc` 添加：
  ```sh
  [ -f ~/.config/ratc/proxy.sh ] && source ~/.config/ratc/proxy.sh
  ```
- 关闭时改写文件内容（写 unset 或清空）
- 覆盖所有读环境变量的终端程序（curl、wget、git、apt 等）

### 7.2 GNOME/KDE 系统代理（可选）

通过 `gsettings set org.gnome.system.proxy mode 'manual'` 设置，适用于 GNOME GUI 应用。非 GNOME 桌面跳过。

`no_proxy` 默认：`localhost,127.0.0.1,::1,10.0.0.0/8,192.168.0.0/16,*.cn`

## 8. 错误处理

| 场景 | 处理 |
|------|------|
| 订阅拉取失败 | 用上次缓存；状态栏显示 `订阅离线(缓存)`；重试 3 次后放弃 |
| 订阅解析失败 | 报错并保留旧数据；日志记录错误位置 |
| 规则集下载失败 | 跳过该规则集继续启动；日志警告 |
| xray 启动失败 | 显示 xray stderr；提供"查看日志"入口；不进入代理状态 |
| xray 意外退出 | 自动重启 3 次，仍失败则暂停并通知 UI |
| 切换到不兼容节点 | UI 阻止选择并提示原因 |
| 配置文件损坏 | 备份后重置为默认配置 |
| 端口被占用 | 检测后提示，可改端口 |

错误用 `thiserror` 定义类型化错误，TUI 层用 `Result` 传递友好消息（不 panic）。

## 9. 测试策略

### 9.1 单元测试

重点覆盖纯逻辑组件（无 IO 依赖）：

| 组件 | 测试重点 |
|------|---------|
| `model` | Clash YAML 反序列化各协议类型，用真实片段做 fixture |
| `converter` | 每种规则类型 → xray 的正确映射；兜底规则；规则集展开 |
| `subscription` | 用 mock HTTP（`mockito`）测试解析与缓存 |
| `config` | 应用配置序列化/反序列化 |

**关键 fixture**：把订阅 YAML（脱敏 UUID/密码）存为 `tests/fixtures/clash_meta.yaml`，作为解析与转换黄金测试样本。

### 9.2 集成测试

- 启动真实 xray（CI 装 xray-core），用生成配置验证 `xray -test -config`
- 对本地 mixed-in 端口做 TCP 连通性测试

### 9.3 手动验收

- 拉取真实订阅、节点列表正确、切换后访问外网、系统代理生效、规则集下载正常

## 10. 安全考量

- 订阅含敏感信息（UUID、密码、token）：缓存文件权限 `0600`，配置目录 `0700`
- xray 监听默认仅 `127.0.0.1`，`allow-lan` 默认关闭
- 不记录任何凭证到日志（节点字段脱敏）
- 不把订阅 URL（含 token）写入任何明文日志

## 11. 文档交付物

实现中必须完成以下文档：

### 11.1 `README.md`

- 项目简介与截图
- 功能特性列表
- 系统依赖（xray-core 安装方式）
- 构建方法（`cargo build --release`）
- 快速上手（5 步内跑起来）
- 配置目录说明
- 常见问题（FAQ）
- 许可证

### 11.2 `docs/MANUAL.md`（使用手册）

- 完整安装指南（含 xray-core 下载、geoip.dat/geosite.dat 放置）
- 首次运行配置流程
- 每个 Tab 的详细操作说明（节点/订阅/规则/日志/设置）
- 全局快捷键速查表
- 系统代理启用步骤（含 shell rc 配置示例）
- 规则兼容性说明（哪些 Clash 规则不支持）
- 故障排查（端口占用、xray 启动失败、订阅解析失败等）
- 配置文件格式参考（`~/.config/ratc/config.json` 各字段）

## 12. MVP 范围（实现阶段拆分参考）

建议实现分阶段：

1. **阶段 1 (基础)**: 项目骨架、`model`、`subscription` 解析、单测 fixture
2. **阶段 2 (转换)**: `converter`（节点 + 规则转换）、兜底规则、单元测试
3. **阶段 3 (xray)**: `xray` 进程管理、配置生成、`xray -test` 集成测试
4. **阶段 4 (TUI)**: Ratatui 框架、5 个 Tab、节点切换交互
5. **阶段 5 (集成)**: `sysproxy`、`store` 持久化、错误处理完善
6. **阶段 6 (文档)**: README + MANUAL + 截图

## 附录 A: 技术栈依赖

| crate | 用途 |
|-------|------|
| `ratatui` + `crossterm` | TUI 渲染与终端事件 |
| `reqwest` | HTTP 订阅/规则集下载 |
| `serde` + `serde_json` + `serde_yaml` | 序列化 |
| `tokio` | 异步运行时与子进程管理 |
| `thiserror` + `anyhow` | 错误处理 |
| `mockito` | 测试用 mock HTTP |
| `dirs` | 标准配置目录定位 |

## 附录 B: 配置目录结构

```
~/.config/ratc/
├── config.json              # 应用配置
├── proxy.sh                 # 系统代理 source 片段
├── cache/
│   └── <hash>.yaml          # 订阅缓存
├── ruleset/
│   ├── CNdirect1.yaml       # 规则集缓存
│   └── ...
└── logs/
    └── ratc.log             # 应用日志
```
