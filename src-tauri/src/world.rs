//! World data: the fictional 2027 database.
//!
//! Ships as `data/world.json` compiled into the binary. The DB is versioned;
//! saves pin the `db_version` they started with (specs DR-1, UR-5).

use crate::core::types::World;

pub use crate::core::types;

/// Current bundled DB version. Bump on every data change.
pub const DB_VERSION: &str = "2027.01";

pub const FALLBACK_WORLD_JSON: &str = include_str!("../../data/world.json");

/// Optional database packs, selectable at new-game.
///
/// The shipped default stays fictional (specs §4 DR-1). These packs are opt-in
/// and clearly labelled: `real-international` uses real player and ground names
/// and is intended for testing, so it is never the database the game boots
/// into. Each pack carries its own `db_version`, which a save pins, so a
/// career started against one pack can never be silently mixed with another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackId {
    /// Real ICC full-member nations with real squads and grounds.
    RealInternational,
}

impl PackId {
    pub fn key(self) -> &'static str {
        match self {
            PackId::RealInternational => "real-international",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PackId::RealInternational => "International (real names)",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            PackId::RealInternational => {
                "The ten ICC full members with real squads, grounds and competitions. \
                 A test database — the shipped default stays fictional."
            }
        }
    }

    pub const ALL: &'static [PackId] = &[PackId::RealInternational];
}

pub fn pack_from_key(key: &str) -> Option<PackId> {
    PackId::ALL.iter().copied().find(|p| p.key() == key)
}

/// A pack id plus its JSON, as shipped in the binary.
pub struct DatabasePack {
    pub id: PackId,
    pub json: &'static str,
}

/// Every pack compiled into the binary.
pub const PACKS: &[DatabasePack] = &[DatabasePack {
    id: PackId::RealInternational,
    json: include_str!("../../data/packs/real-international/world.json"),
}];

/// Load a pack by id, falling back to an empty world on malformed JSON.
pub fn load_pack(id: PackId) -> World {
    let Some(pack) = PACKS.iter().find(|p| p.id == id) else {
        return World::default();
    };
    serde_json::from_str(pack.json).unwrap_or_else(|e| {
        eprintln!("[world] pack {} is invalid: {e}", id.key());
        World::default()
    })
}

/// Load the bundled world.
pub fn load_world() -> World {
    serde_json::from_str(FALLBACK_WORLD_JSON).unwrap_or_else(|e| {
        eprintln!("[world] bundled world.json is invalid: {e}");
        World::default()
    })
}
