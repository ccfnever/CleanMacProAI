//! Core engine namespace.
//!
//! The first production slice keeps the implementation inside Tauri command
//! modules so the native boundary remains easy to inspect. Shared engine code
//! can move here once scanner, cleaner, and uninstaller behavior stabilizes.
