//! Match engine: ball resolution, orders, runner.
//!
//! The Rust side is authoritative during a match; the UI sends orders and
//! "advance one unit" requests and renders returned snapshots.

pub mod engine;
pub mod sim;
pub mod state;

pub use engine::resolve_ball;
pub use sim::{MatchConfig, SimState, TeamSelection};
pub use state::{
    BallOutcome, BattingCard, BowlingCard, Dismissal, DismissalType, ExtraKind, FieldPreset,
    Innings, MatchClock, MatchInput, MatchPhase, MatchResult, MatchSnapshot, TossDecision,
    WinMargin,
};
