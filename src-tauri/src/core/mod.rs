//! Deterministic simulation primitives: RNG, distributions, math helpers.
//!
//! Everything that consumes randomness takes an explicit `&mut SimRng`, so a
//! seed fully determines a match (specs FR-13, NFR-7).

pub mod development;
pub mod names;
pub mod skills;
pub mod rng;
pub mod season;
pub mod transfers;
pub mod tournaments;
pub mod scheduler;
pub mod types;

pub use rng::{clamp, Distribution, SimRng};
pub use types::{
    Competition, CompetitionKind, Format, Ground, Nation, Player, Rating, Role, Skills, Team, World,
};
