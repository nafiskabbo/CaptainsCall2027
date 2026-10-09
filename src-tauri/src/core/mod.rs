//! Deterministic simulation primitives: RNG, distributions, math helpers.
//!
//! Everything that consumes randomness takes an explicit `&mut SimRng`, so a
//! seed fully determines a match (specs FR-13, NFR-7).

pub mod auction;
pub mod development;
pub mod dls;
pub mod names;
pub mod records;
pub mod rng;
pub mod scheduler;
pub mod season;
pub mod skills;
pub mod tournaments;
pub mod transfers;
pub mod types;
pub mod updater;

pub use rng::{clamp, Distribution, SimRng};
pub use types::{
    Competition, CompetitionKind, Format, Ground, Nation, Player, Rating, Role, Skills, Team, World,
};
