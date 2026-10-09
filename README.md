# 🧹 CleanMacProAI

> AI-powered Mac cleaner — lightweight, fast, and privacy-first.

对标 CleanMyMac X 的轻量级 macOS 清理工具，基于 Tauri (Rust + Vue 3) 构建。

## ✨ 核心特性

- **智能扫描** — 系统缓存、应用日志、浏览器缓存、开发工具残留
- **安全清理** — 默认移入废纸篓；可选永久删除必须明确确认，无法从废纸篓恢复
- **软件卸载** — 完整卸载 App + 清理关联残留文件
- **空间透镜** — 可视化磁盘占用分布
- **隐私保护** — 本地运行，数据不离开你的 Mac
- **极致轻量** — 安装包 < 10 MB，内存占用 < 50 MB

## 🛠️ 技术栈

| 层级 | 技术 |
|------|------|
| 前端 | Vue 3 + TypeScript + UnoCSS + ECharts |
| 后端 | Rust + Tauri v2 |
| 权限 | macOS 系统权限 + 设置引导 + Rust 后端路径校验 |
| 构建 | Tauri CLI + GitHub Actions |
| 分发 | Tauri 打包；自动更新与支付尚未接入 |

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
| 🟢 Low | 通常可处理的缓存与日志 | 默认勾选，执行前仍需确认 |
| 🟡 Medium | 建议确认后处理 | 默认不勾选，手动选择并确认 |
| 🔴 High | 高风险操作 | 默认不勾选 |

## ⚠️ 安全承诺

1. **默认移入废纸篓** — 清理、卸载默认使用系统废纸篓；永久删除须选择该模式并确认不可恢复。恢复取决于文件仍保留在废纸篓中，不承诺恢复期限。
2. **系统文件零触碰** — `/System/`、`/usr/bin/` 永远不扫描
3. **统一后端边界** — 清理遵守规则和排除项；卸载仅处理选定应用及其关联数据，保留共享容器；空间透镜仅处理后端完整扫描结果。符号链接目标、系统文件、钥匙串、邮件及 Xcode 发布归档受保护。
4. **开源透明** — 清理规则公开可查，无黑盒操作

## 📝 开发日志

### 2026-05-18 — 核心功能实现

- **扫描引擎**：实现基于 YAML 规则文件的系统扫描，支持 glob 模式匹配、实时进度追踪、分类扫描与文件预览
- **清理引擎**：实现 clean_items，支持安全模式（移入废纸篓）和永久删除，添加路径安全校验（限制在 Caches/Logs/Xcode/Downloads/Trash）和 ~/ 路径展开
- **卸载引擎**：实现 list_installed_apps，解析 .app 包和 Info.plist 获取应用信息，关联查找 ~/Library 中的缓存/偏好设置/日志等文件；实现 uninstall_app，支持安全卸载并保护系统应用
- **磁盘信息**：替换硬编码数据为真实 `df` 命令解析结果
- **状态管理**：在 main.rs 中添加 ScanState 全局状态管理
- **配置更新**：tauri.conf.json identifier 改为 `com.cleanmacproai.desktop`，打包目标限定为 macOS app
- **代码重构**：清理 core_engine 模块占位代码，移除 rules 模块冗余的 resolve_paths 方法

### 2026-05-18 — 按分类清理 + 权限引导

- 后端新增 `clean_categories`：前端传分类 ID，Rust 按 `cleanup_rules.yaml` 重新解析真实路径并执行清理。
- 高风险分类会被后端拒绝清理，避免误删。
- 默认移入废纸篓使用系统 `NSFileManager`；失败会报告并保留文件，不自动改为永久删除。
- 设置页新增"完全磁盘访问"入口，会打开 macOS 系统设置里的 Full Disk Access 页面。
- 扫描页清理按钮现在会显示"清理中"，并在 native/demo 两种运行环境下给出不同提示。

## 📄 License

[待定 — 建议 Apache-2.0 或 MIT]

---

> 用 AI 做减法，让 Mac 回归轻盈。🦊

## 发布前安全改造（2026-10-09）

- 清理和卸载新增 `mode`（默认 `trash`）及 `permanentConfirmed` 参数。只有显式选择 `permanent` 并确认才能永久删除；无权限时跳过并提示，卸载不再通过管理员 `rm -rf` 绕过边界。
- `CleanReport.processed_bytes` 表示处理的数据大小。废纸篓模式 `freed_bytes` 为 0，永久删除时该字段仅为处理大小的估计；实际磁盘剩余容量重新读取，不以删除文件大小代替。
- Xcode `Archives` 单列为高风险，仅展示、需在 Xcode Organizer 手动管理；低风险自动选择只包含构建缓存。直接文件路径接口也不能绕过高风险限制。
- 后端共用绝对路径、路径组件、保护目录、目录根及符号链接校验。目录操作会检查内部排除项；文件身份在执行前复核。目录内的符号链接不跟随，顶层及祖先中的用户符号链接拒绝操作。
- 空间透镜删除权限来自后端最近一次完整扫描快照；伪造根路径、未扫描目标、被替换目标及部分扫描结果不能删除。
- 设置页仅保留已实现并持久化的主题、权限入口和真实应用版本；隐藏未实现的自动扫描、隐藏文件、扫描深度、语言和自动更新开关。
- 安全验证：`cargo test --manifest-path src-tauri/Cargo.toml`；前端回归：`pnpm test`；类型及构建：`pnpm build`。
