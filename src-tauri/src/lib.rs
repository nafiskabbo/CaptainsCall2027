//! Captain's Call 2027 — Rust core.
//!
//! Modules:
//! - `core`: world data types + deterministic RNG/stats.
//! - `match_`: 2D cricket match simulation engine and live match state.
//! - `commands`: Tauri command surface used by the React UI.
//! - `world`: bundled fictional 2027 database.
//!
//! `name` (2D only, no 3D).

pub mod career;
pub mod commands;
pub mod core;
pub mod db;
pub mod match_;
pub mod world;

pub use core::types::{
    Competition, CompetitionKind, Format, Ground, Nation, Player, Rating, Role, Skills, Team, World,
};
pub use world::{load_world, DB_VERSION};

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            app.manage(commands::init_state());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_world,
            commands::get_db_version,
            commands::list_db_packs,
            commands::load_db_pack,
            commands::quit_to_menu,
            commands::get_squad,
            commands::auto_select,
            commands::start_career,
            commands::get_career,
            commands::save_career,
            commands::list_saves,
            commands::load_career,
            commands::delete_save,
            commands::advance_day,
            commands::advance_one_day,
            commands::advance_to_my_match,
            commands::start_new_season,
            commands::get_transfer_pool,
            commands::bid_for_player,
            commands::release_player,
            commands::get_finances,
            commands::set_training_focus,
            commands::get_update_state,
            commands::set_update_settings,
            commands::check_local_update,
            commands::get_selection,
            commands::set_selection,
            commands::start_fixture,
            commands::get_match,
            commands::set_match_input,
            commands::preview_ball,
            commands::preview_ball_with,
            commands::sim_match,
            commands::sim_innings,
            commands::export_scorecard,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
