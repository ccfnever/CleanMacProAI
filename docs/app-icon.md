# 应用图标

图标由内置 imagegen 工具生成，使用薄荷绿圆角底座、白色猫爪和清洁星光，呼应应用的猫爪标识。

## 生成提示词

> A production macOS Dock icon for CleanMacProAI: centered mint/teal squircle tile on a transparent square canvas, sculpted warm-white cat paw with four rounded toes and a broad central pad, one small four-point cleaning sparkle at the upper-right. Premium native macOS aesthetic, restrained depth, crisp silhouette readable at 32px. No text, watermark, or mockup.

## 资源与更新

- `src-tauri/icons/icon.png`：透明背景原始图稿。
- `src-tauri/icons/icon.icns`：macOS 程序坞与应用包图标。
- `32x32.png`、`128x128.png`、`128x128@2x.png`、`icon.ico`：Tauri 所需的其他图标资源。
- `src-tauri/tauri.conf.json` 的 `bundle.icon` 显式引用这些文件。macOS 开发模式也会从此配置加载 `.icns`。

更换图稿后，用 `pnpm tauri icon src-tauri/icons/icon.png --output <临时目录>` 生成尺寸资源，将上述五个文件复制到 `src-tauri/icons/`，保留原始 `icon.png`，然后重新构建并启动应用。
