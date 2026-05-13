# 🧹 CleanMacProAI

> AI-powered Mac cleaner — lightweight, fast, and privacy-first.

对标 CleanMyMac X 的轻量级 macOS 清理工具，基于 Tauri (Rust + Vue 3) 构建。

## ✨ 核心特性

- **智能扫描** — 系统缓存、应用日志、浏览器缓存、开发工具残留
- **安全清理** — 所有文件先移到废纸篓，支持一键回滚
- **软件卸载** — 完整卸载 App + 清理关联残留文件
- **空间透镜** — 可视化磁盘占用分布
- **隐私保护** — 本地运行，数据不离开你的 Mac
- **极致轻量** — 安装包 < 10 MB，内存占用 < 50 MB

## 🛠️ 技术栈

| 层级 | 技术 |
|------|------|
| 前端 | Vue 3 + TypeScript + UnoCSS + ECharts |
| 后端 | Rust + Tauri v2 |
| 权限 | tauri-plugin-macos-permissions (Full Disk Access) |
| 构建 | Tauri CLI + GitHub Actions |
| 分发 | Sparkle (自动更新) + Paddle (支付) |

## 📐 架构

```
CleanMacProAI/
├── src/                          # Vue 3 前端
│   ├── views/                    # 页面组件
│   ├── components/               # 可复用组件
│   ├── stores/                   # Pinia 状态管理
│   └── composables/              # 组合式函数
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs               # Tauri 入口
│   │   ├── commands/             # Tauri commands（前端/后端边界）
│   │   ├── core_engine/          # 核心业务逻辑
│   │   ├── models/               # 数据结构
│   │   └── rules/                # 清理规则引擎（YAML 配置化）
│   ├── capabilities/             # 权限配置
│   ├── tauri.conf.json           # Tauri 配置
│   └── Info.plist                # macOS 权限描述
├── .github/
│   ├── WORKFLOW.md               # AI Agent 开发规范（Solo Symphony）
│   ├── workflows/ci.yml          # CI/CD 流水线
│   └── ISSUE_TEMPLATE/           # Issue 模板
└── SOLO_SYMPHONY.md              # 一人公司 AI 协作工作流指南
```

## 🚀 开发

```bash
# 安装依赖
pnpm install

# 开发模式
pnpm tauri dev

# 构建 release
pnpm tauri build

# Rust 测试
cd src-tauri && cargo test

# Rust Lint
cd src-tauri && cargo clippy -- -D warnings
```

## 🎼 Solo Symphony 工作流

本项目采用 **一人公司 AI 协作工作流**（Solo Symphony），详见：
- [`.github/WORKFLOW.md`](.github/WORKFLOW.md) — AI Agent 开发规范
- [`SOLO_SYMPHONY.md`](SOLO_SYMPHONY.md) — 完整工作流指南

核心流程：
```
GitHub Issue → AI Agent (Codex/Claude Code) → 隔离分支 → PR → 主人审核 → Merge
```

## 📋 清理规则

清理规则定义在 `src-tauri/src/rules/cleanup_rules.yaml`，按风险等级分类：

| 风险 | 说明 | 默认行为 |
|------|------|----------|
| 🟢 Low | 安全，删除后自动重建 | 默认勾选 |
| 🟡 Medium | 建议确认后删除 | 默认勾选 + 提示 |
| 🔴 High | 高风险操作 | 默认不勾选 |

## ⚠️ 安全承诺

1. **所有删除走废纸篓** — 不直接删除，30 天内可恢复
2. **系统文件零触碰** — `/System/`、`/usr/bin/` 永远不扫描
3. **白名单机制** — `always_exclude` 列表中的路径永远跳过
4. **开源透明** — 清理规则公开可查，无黑盒操作

## 📄 License

[待定 — 建议 Apache-2.0 或 MIT]

---

> 用 AI 做减法，让 Mac 回归轻盈。🦊
