<div align="center">

<img src="app-icon.png" width="108" height="108" alt="MCLR Logo" />

# MCLR — Minecraft Launcher Rewrite

次世代 Windows 原生 Minecraft 启动器 · 基于 Tauri 2 + Rust 2024 + Vue 3.5

[![Version](https://img.shields.io/badge/version-v0.0.33-blue.svg?style=flat-square)](CHANGELOG.md)
[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-DEA584.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB.svg?style=flat-square&logo=tauri)](https://tauri.app/)
[![Vue](https://img.shields.io/badge/Vue-3.5-4FC08D.svg?style=flat-square&logo=vue.js)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-6.0-3178C6.svg?style=flat-square&logo=typescript)](https://www.typescriptlang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011%20x64-0078D6.svg?style=flat-square&logo=windows)](https://www.microsoft.com/)

[**下载体验**](#-下载与安装) · [**功能特性**](#-特性亮点) · [**文档导航**](#-文档导航) · [**构建指南**](#-开发与构建) · [**赞助支持**](#-赞助支持)

</div>

---

**MCLR** 是一款面向 Windows 平台打造的高性能、模块化 Minecraft 启动器。项目旨在保持与 Plain Craft Launcher 2 (PCL2) 核心体验高度对齐的前提下，采用现代化的 **Tauri 2**、**Rust (Edition 2024)** 与 **Vue 3.5 + TypeScript** 彻底重构底层技术栈，摆脱传统启动器的历史包袱，带来零内存泄漏、短锁并发、原生硬件级流畅度与优雅纯粹的 UI 体验。

---

## ⚡ 特性亮点

### 🚀 极致性能与可靠网络 (`mclr-net`, `mclr-core`)
- **多源并发竞速下载**：聚合 Mojang 官方、Modrinth、CurseForge、BMCLAPI、MCIMirror 等镜像源，梯级回退与毫秒级延迟竞速。
- **动态 HTTP Range 分片**：大文件自动切片（8 MiB 分片），支持动态慢速分片窃取与自动续传，慢速链路不再超时崩溃。
- **全方位哈希校验**：流式 SHA1 / SHA256 / MD5 单遍计算，杜绝全文件二次重读 I/O 开销；支持 DNS-over-HTTPS (`hickory-resolver`)。

### 🎨 Tonal Layers 现代设计语言
- **OKLCH 动态取色引擎**：由单颗种子色实时派生出兼顾色彩连贯性与可读性感知的高保真色阶系统。
- **双材质即时切换**：**全实色扁平 (Flat)** 与 **液态玻璃 (Liquid Glass)** 材质（Mica/Acrylic 原生模糊 + SVG 微折射特效），无缝贴合 Win10/11 审美。
- **流式瀑布流与弹性虚拟滚动**：集成官方新闻瀑布流、分页悬浮定位槽 (`MPagination`) 以及自适应视口的虚拟长列表 (`MVirtualList`)。

### 🔑 完整多账号认证体系 (`mclr-auth`)
- **微软正版设备代码流**：内置成熟公用 Client ID，免除配置 Azure 应用烦恼，开箱即用；完整的 Xbox Live / XSTS / Minecraft 6 步鉴权链，支持披风与皮肤即时切换。
- **第三方统一登录**：全面支持 Authlib-Injector / LittleSkin 多角色自由切换与 OIDC 发现协议。
- **离线账号安全规范**：标准 RFC 4122 v3 MD5、旧版兼容算法与自定义 32 位 UUID。
- **硬件级本地加密存储**：账号令牌使用 Windows DPAPI 加密的 32 字节主密钥 (`master.key`) 与 AES-256-GCM 保护 (`accounts.enc`)，绝不明文落盘。

### 📦 社区资源与整合包生态 (`mclr-comp`)
- **双平台聚合检索**：无缝聚合 **Modrinth** 与 **CurseForge** 海量资源（模组、整合包、材质包、光影包、数据包、世界存档）。
- **智能兼容与严谨更新**：严格匹配实例游戏版本与 Mod 加载器（遵循 PCL 校验哲学），不跨加载器误推，已安装 Mod 自动检查更新。
- **全格式整合包导入导出**：支持 CurseForge `.zip`、Modrinth `.mrpack`、MultiMC/Prism、MCBBS、HMCL 以及原生 `.minecraft` 格式；整合包安装进度平滑单调递增，清晰展示各阶段。

### 🛠️ 自动化环境配置与诊断 (`mclr-game`, `mclr-loaders`, `mclr-crash`)
- **全加载器全自动部署**：一键安装 Vanilla、Fabric、NeoForge、Forge（Legacy & Modern）、Cleanroom、OptiFine、LiteLoader 与 LabyMod。
- **独立 Java 运行时中心**：自动扫描注册表、PATH、常见安装路径，支持多架构版本自动匹配与目录一键直达。
- **四阶段启发式崩溃分析**：覆盖 45 种经典崩溃根因，精准定位冲突模组与环境故障，支持一键导出排障报告。
- **全能系统工具箱**：内置游戏与启动器双层滚动日志排查、一键清理失败任务与冗余缓存。

---

## 📚 文档导航

所有工程与设计文档统一维护在 [`docs/`](docs/) 目录下：

| 文档 | 说明 |
|---|---|
| [**系统架构图谱**](docs/architecture.md) | Crate 职责划分、模块依赖拓扑与进程通信模型 |
| [**设计语言与规范**](docs/design-language.md) | Tonal Layers 颜色代号、表面层次、UI 契约与动效参数 |
| [**接口与行为契约**](docs/contracts.md) | IPC 命令规范、错误码体系、路径安全约束与多语言规范 (i18n) |
| [**版本迭代路线图**](docs/progress.md) | 从 v0.0.1 到当前版本的里程碑演进记录与路线图 |
| [**更新日志**](CHANGELOG.md) | 详尽版本变更记录 (Keep a Changelog 格式) |

---

## 💻 开发与构建

### 环境要求 (Prerequisites)
- **操作系统**：Windows 10 / 11 x64（已安装 WebView2 运行时）
- **Rust 工具链**：`1.99+`（MSVC `x86_64-pc-windows-msvc`）
- **Node.js**：`22+`
- **包管理器**：**pnpm** `10+` / `12+`

### 常用命令

```bash
# 1. 安装项目依赖
pnpm install

# 2. 启动前端 Vite 调试服务（浏览器中带 Mock IPC 预览）
pnpm dev

# 3. 启动完整的 Tauri 桌面开发模式
pnpm tauri dev

# 4. 从 Rust 类型重新生成前端 TypeScript 绑定 (tauri-specta)
pnpm gen:bindings

# 5. 基于 Oxc 高性能工具链的代码检查与格式化
pnpm lint
pnpm fmt

# 6. 运行全量发布门禁 (cargo fmt, clippy -D warnings, cargo test, vitest, i18n lint, vite build)
pnpm check

# 7. 构建 Windows 生产版安装包 (NSIS x64)
pnpm tauri build
```

### 独立无头命令行客户端 (`mclr-cli`)

无需启动 GUI 即可执行核心操作：

```bash
# 查看启动器信息
cargo run -p mclr-cli -- info

# 扫描本机 Java 环境
cargo run -p mclr-cli -- java scan

# 命令行安装 Minecraft 1.20.1 Fabric 实例
cargo run -p mclr-cli -- install 1.20.1 my-fabric --fabric

# 离线账号无头启动
cargo run -p mclr-cli -- launch my-fabric --offline Steve
```

---

## 💖 赞助支持

MCLR 是一款完全由社区独立维护的开源 Minecraft 启动器。如果您觉得 MCLR 对您有所帮助，欢迎赞助支持项目的持续开发与维护！

<div align="center">

| 微信支付 (WeChat Pay) | 支付宝 (Alipay) |
| :---: | :---: |
| <img src="public/sponsor/wechat.png" width="220" alt="微信赞助二维码" /> | <img src="public/sponsor/alipay.png" width="220" alt="支付宝赞助二维码" /> |
| 扫描二维码通过微信支持 | 扫描二维码通过支付宝支持 |

*非常感谢每一位赞助者的慷慨支持，您的鼓励是项目精益求精的最大动力！*

</div>

---

## 📄 许可说明

本项目保留所有权利（All Rights Reserved）。第三方组件遵循各自开源协议，详情请参见 [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)。

特别致谢 **Plain Craft Launcher 2 (PCL2)**、**Prism Launcher** 等优秀开源社区项目的灵感与先驱贡献。
