# MCLR Progress

- **Current Version**: `0.0.34`
- **Next Milestone**: `v0.0.35` (towards `v0.1.0`)

| Milestone | Title | Status | Commit | Tag | Date |
|---|---|---|---|---|---|
| v0.0.1 | Foundation | done | 08bf7f0 | v0.0.1 | 2026-10-24 |
| v0.0.2 | Core runtime + UI shell | done | e2b7823 | v0.0.2 | 2026-10-24 |
| v0.0.3 | Network, task UI, game model, Java | done | b025445 | v0.0.3 | 2026-10-24 |
| v0.0.4 | Accounts + vanilla install | done | dafbc5f | v0.0.4 | 2026-10-24 |
| v0.0.5 | Launch pipeline (first playable) | done | c25aedb | v0.0.5 | 2026-10-24 |
| v0.0.6 | Mod loaders | done | 0bb2575 | v0.0.6 | 2026-10-24 |
| v0.0.7 | Community platforms | done | a2cdbf3 | v0.0.7 | 2026-10-24 |
| v0.0.8 | Modpacks + local mod management | done | 0c21da7 | v0.0.8 | 2026-10-24 |
| v0.0.9 | Instance management pages | done | 09d9755 | v0.0.9 | 2026-10-24 |
| v0.0.10 | Modpack export + crash analysis | done | 93cacea | v0.0.10 | 2026-10-24 |
| v0.0.11 | Personalisation, home, tools | done | a952780 | v0.0.11 | 2026-10-24 |
| v0.0.12 | Updates, remaining settings, i18n audit, packaging | done | v0.0.12 | v0.0.12 | 2026-10-25 |
| v0.0.13 | UI redesign: Tonal Layers | done | v0.0.13 | v0.0.13 | 2026-10-25 |
| v0.0.14 | Instance-first flow and fluidity | done | v0.0.14 | v0.0.14 | 2026-10-25 |
| v0.0.15 | UI alignment, PCL version parity, Minecraft icons & component polish | done | v0.0.15 | v0.0.15 | 2026-10-25 |
| v0.0.16 | Home & Launch consolidation, standalone Java page, Help in About & cleanups | done | v0.0.16 | v0.0.16 | 2026-10-25 |
| v0.0.17 | UVMC parity, select-to-add flows, MHelpTooltip, compact UI & polish | done | v0.0.17 | v0.0.17 | 2026-10-25 |
| v0.0.18 | Shell redesign, news masonry, floating paginator, folders dialog, MMarkdown, PCL skin parity | done | v0.0.18 | v0.0.18 | 2026-10-25 |
| v0.0.19 | Tasks in logs, Java folder reveal, MSelect adaptive width, layout refinements & PCL comp parity | done | v0.0.19 | v0.0.19 | 2026-10-25 |
| v0.0.20 | Oxc toolchain, NeoForge detection, simplified instance card, liquid glass titlebar & PCL polish | done | v0.0.20 | v0.0.20 | 2026-10-26 |
| v0.0.21 | Unified overlay architecture, codebase health overhaul & UX consistency | done | v0.0.21 | v0.0.21 | 2026-10-26 |
| v0.0.22 | — (skipped: fold into 0.0.23) | skipped | — | — | 2026-10-26 |
| v0.0.23 | Design system overhaul, solid flat material, rich iconography & universal MImage | done | v0.0.23 | v0.0.23 | 2026-10-26 |
| v0.0.24 | Sidebar scale tracking, overflow fixes, drawer frosting & concurrent mod update checks | done | v0.0.24 | v0.0.24 | 2026-10-26 |
| v0.0.25 | Advanced launch options, loader artwork & PCL version rules, floating collision, responsive layout | done | v0.0.25 | v0.0.25 | 2026-10-28 |
| v0.0.26 | Launch agents, security hardening, Java resolution rework & material-aware selection | done | v0.0.26 | v0.0.26 | 2026-10-28 |
| v0.0.27 | Smoothness, CPU & memory: incremental logs, compositor shimmer, verified-manifest fast path, download pool | done | v0.0.27 | v0.0.27 | 2026-10-28 |
| v0.0.28 | Standalone modpack instances, mods-first resource tabs, mod artwork pipeline & full dependency refresh | done | v0.0.28 | v0.0.28 | 2026-10-29 |
| v0.0.29 | Performance preset, system tray & background residency, real task pause/cancel, launcher log maintenance & fake-feature audit sweep | done | 9896837 | v0.0.29 | 2026-10-30 |
| v0.0.30 | Zero-allocation hashing, short-lock mod cache, parallel scans, buffered downloads, virtualised logs & legacy Forge subfolder fix | done | 5e70b61 | v0.0.30 | 2026-10-31 |
| v0.0.31 | Glass material fix, adjacent page prefetching, dynamic comp filters & deferred modpack naming | done | 6f5a996 | v0.0.31 | 2026-11-01 |
| v0.0.32 | Community session resume, honest modpack progress, slow-link download fix, strict loader-aware updates & failed-task cleanup | done | 7f7446a | v0.0.32 | 2026-11-01 |
| v0.0.33 | Public Microsoft OAuth client ID, device code flow & accounts refresh | done | ed0e919 | v0.0.33 | 2026-11-02 |

| v0.0.34 | TypeScript 6.0.3, secrets via .env, shadow clipping & mod update race fix | done | a9d4d8d | v0.0.34 | 2026-11-02 |
| v0.0.35 | 资源预设 + PCL 同款模组能力 | in_progress (unreleased) | — | — | — |

## v0.0.35 — 资源预设 + PCL 同款模组能力 (Unreleased)

| ID | slice | owner | status | acceptance |
|---|---|---|---|---|
| A | 中文模组名 + 命名格式 | Wave 1A | done | cargo test -p mclr-comp modname naming |
| B | 模组依赖分析 + 版本范围 | Wave 1B | done | cargo test -p mclr-comp depcheck versionrange |
| C | 内容寻址缓存 + 硬链接 | Wave 1C | done | cargo test -p mclr-comp store |
| D | 预设模型/仓储/过滤/索引/账本/计划引擎 | Wave 1D | done | cargo test -p mclr-comp preset |
| E1 | 预设 IPC 指令 + 任务编排 | Wave 2E1 | done | cargo test -p mclr-app |
| E2 | 命名/中文名/依赖 IPC 集成 | Wave 2E2 | done | cargo test -p mclr-app -p mclr-comp |
| F1 | 前端预设特性 + 弹窗 | Wave 2F1 | done | vitest run src/features/presets |
| F2 | 前端社区资源/本地资源/系统设置联动 | Wave 2F2 | done | vitest run src/features/comp src/features/resources |
| F3 | 实例详情预设 Tab + 安装流程联动 | Wave 3F3 | done | vitest run src/features/download src/features/instance-detail |

### Notes
- Release gate: version bumps, release notes finalization, and git tags strictly deferred until explicit user confirmation.

