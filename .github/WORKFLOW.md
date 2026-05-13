# CleanMacProAI 开发工作流规范

> 本文件是 AI Agent（Codex / Claude Code）在此仓库工作的唯一规范。
> 所有开发任务必须遵循此规范执行。

---

## 🏗️ 架构概览

```
CleanMacProAI = Tauri v2 (Rust 后端 + Vue 3 前端)
├── src/              # Vue 3 + TypeScript + Pinia 状态管理
├── src-tauri/        # Rust 后端，Tauri commands 暴露给前端
│   ├── src/
│   │   ├── commands/ # Tauri #[tauri::command] 函数
│   │   ├── core/     # 核心业务逻辑（扫描/清理/卸载）
│   │   ├── models/   # 数据结构定义
│   │   └── rules/    # 清理规则（YAML 配置）
```

## 📐 开发原则

1. **安全第一** — 任何文件删除操作必须先移到废纸篓，不直接 `remove()`
2. **配置化规则** — 清理路径写在 `rules/cleanup_rules.yaml`，不硬编码
3. **TDD** — 每个 Rust 模块必须有对应单元测试
4. **小步提交** — 每个 commit 只做一件事，message 用 Conventional Commits
5. **权限最小化** — Tauri capabilities 只申请必需的权限

## 🔧 技术栈约定

### 前端
- Vue 3 (Composition API) + TypeScript
- Pinia 状态管理
- UnoCSS / TailwindCSS 样式
- ECharts 用于空间可视化

### 后端 (Rust)
- Tauri v2 commands 作为前端/后端边界
- `serde` 序列化数据
- `walkdir` 遍历文件系统
- `tauri-plugin-macos-permissions` 处理系统权限

## 🧪 测试要求

### Rust 测试
```bash
cd src-tauri && cargo test
```
- 每个 `pub fn` 必须有 `#[cfg(test)] mod tests`
- 测试覆盖：正常路径、边界情况、错误处理

### 前端测试
```bash
pnpm test
```
- 关键组件必须有单元测试
- 扫描/清理流程必须有 E2E 测试

## 🚨 安全红线（AI Agent 必须遵守）

1. **禁止直接删除系统文件** — 所有删除走 `trash` crate（移到废纸篓）
2. **禁止硬编码绝对路径** — 用 `dirs::home_dir()` 等安全方式获取路径
3. **禁止绕过 Tauri 权限系统** — 新 command 必须更新 `capabilities/default.toml`
4. **禁止跳过测试** — PR 必须通过所有 CI 检查
5. **用户数据无风险** — 清理前必须生成快照，支持回滚

## 📋 任务执行流程

当你被分配一个 Issue 时：

1. **创建分支** — `git checkout -b feat/issue-XXX-short-desc`
2. **理解需求** — 阅读 Issue 描述，不明确则 comment 提问
3. **写测试** — 先写 failing test（TDD）
4. **实现代码** — 最小实现让测试通过
5. **跑测试** — `cargo test` + `pnpm test`
6. **Lint 检查** — `cargo clippy -- -D warnings` + `pnpm lint`
7. **提交** — `git commit -m "feat(core): describe what changed"`
8. **创建 PR** — 关联 Issue，填写变更说明
9. **等待审核** — 主人 review 后 merge

## 📝 Commit Message 格式

```
<type>(<scope>): <description>

[optional body]
```

Type: `feat` | `fix` | `docs` | `style` | `refactor` | `test` | `chore`
Scope: `scanner` | `cleaner` | `uninstaller` | `ui` | `rules` | `ci` | `deps`

示例：
```
feat(scanner): add system cache scanning with progress callback
fix(cleaner): handle permission denied on protected folders
test(rules): add unit tests for YAML rule parsing
```

## 🔄 CI/CD 流程

```
Push → GitHub Actions:
  ├── Build (macOS) — cargo build + pnpm build
  ├── Test — cargo test + pnpm test
  ├── Lint — cargo clippy + pnpm lint
  └── Security — cargo audit + npm audit
```

通过所有检查后，PR 可 merge。
