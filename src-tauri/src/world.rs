//! World data: the fictional 2027 database.
//!
//! Ships as `data/world.json` compiled into the binary. The DB is versioned;
//! saves pin the `db_version` they started with (specs DR-1, UR-5).

use crate::core::types::World;

pub use crate::core::types;

/// Current bundled DB version. Bump on every data change.
pub const DB_VERSION: &str = "2027.01";

pub const FALLBACK_WORLD_JSON: &str = include_str!("../../data/world.json");

/// Load the bundled world.
pub fn load_world() -> World {
    serde_json::from_str(FALLBACK_WORLD_JSON).unwrap_or_else(|e| {
        eprintln!("[world] bundled world.json is invalid: {e}");
        World::default()
    })
}
