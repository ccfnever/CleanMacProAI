// CleanMacProAI — Tauri v2 主入口

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod core_engine;
mod models;
mod rules;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::scanner::scan_system,
            commands::scanner::get_scan_progress,
            commands::scanner::cancel_scan,
            commands::cleaner::clean_items,
            commands::cleaner::get_clean_report,
            commands::uninstaller::list_installed_apps,
            commands::uninstaller::uninstall_app,
            commands::system::get_disk_info,
            commands::system::request_permissions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
