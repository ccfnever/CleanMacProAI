# 启动项管理

页面入口：侧栏 → 启动项管理。沿用暖阳、森林、雾蓝三套主题，以及应用管理的列表与原位详情交互。

## 已实现

- 扫描 `~/Library/LaunchAgents`、`/Library/LaunchAgents`、`/Library/LaunchDaemons`。
- 从 launchctl 覆盖状态及 plist 的 Disabled 属性读取自动启动许可，兼容 true/false 和 disabled/enabled 输出。
- 支持状态与类型筛选、名称/标识符/路径搜索、路径与参数详情、Finder 定位、重新扫描。
- 对当前用户和共享的普通第三方 LaunchAgent 使用 launchctl enable/disable，调整当前用户的启动许可；系统 LaunchDaemon 使用 macOS 管理员授权后执行同样操作，影响所有用户。操作后读取系统状态确认。不删除或改写配置，不停止当前服务。修改用于后续加载/登录，当前进程不变。
- 目标文件缺失标为“待检查”，不直接断定是残留；相对执行路径不误判为缺失。
- 所有项目统一显示开关。系统守护进程标注“需管理员授权”；Apple 服务、符号链接、重复 Label 和未知状态禁用开关并说明原因。命令端再次检查目录、路径与 Label，权限级别由后端推导。
- 原生扫描失败显示错误或部分读取提示，不替换为示例数据。浏览器预览使用显式标记的样例，仅模拟交互。

## 范围

此列表不是 macOS 登录项与扩展的完整镜像。通过 SMAppService 注册的登录项、后台授权及管理员管理的服务，由页面提供系统设置入口，亦可在所属应用中调整。普通用户和共享启动项不提权；系统守护进程在点击开关时通过 AppleScript 的 `do shell script ... with administrator privileges` 请求系统管理员授权。密码交由 macOS 接收，不在应用中读取、保存或传递。不申请 System Events 自动化权限。取消授权保持状态。

## 验证

- `pnpm build`：TypeScript 与生产构建。
- `cargo test --manifest-path src-tauri/Cargo.toml startup::tests`：状态解析、plist 默认值与覆盖、未知状态、路径与符号链接限制、目录权限范围推导、提权命令的 Shell 与 AppleScript 双层转义、真实本机只读扫描。
- `node scripts/test-startup-items.mjs`：组合筛选、未知状态、缺失目标和搜索。
- 浏览器人工检查：用户/共享/系统项目统一开关与启停统计同步、详情展开、空搜索恢复、待检查与类型筛选、重新扫描、三套主题、900px 与 1400px 桌面宽度。
- 未对用户现有启动项执行实际启停或重启验证；交互开关验证在明确标记的预览模式下完成。

预览图：`preview.jpg`。
