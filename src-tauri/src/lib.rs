//! Captain's Call 2027 core library.
//!
//! Tauri entry point. The deterministic match engine (`core/`), save layer
//! and versioned-DB layer will live here as M0–M1 land.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    name: &'static str,
    version: &'static str,
    phase: &'static str,
}

/// Placeholder command proving the UI ↔ Rust bridge works.
/// Real commands (sim tick, save/load, DB update) arrive in M0–M1.
#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        name: "Captain's Call 2027",
        version: env!("CARGO_PKG_VERSION"),
        phase: "M0 scaffold",
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
