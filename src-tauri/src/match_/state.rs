//! Match engine types: state, orders, cards, snapshots.

use crate::core::types::Format;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchPhase {
    PreMatch,
    Innings1,
    Innings2,
    Innings3,
    Innings4,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DismissalType {
    NotOut,
    Bowled,
    Caught,
    Lbw,
    RunOut,
    Stumped,
    CaughtAndBowled,
    HitWicket,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dismissal {
    pub out_type: DismissalType,
    pub bowler_id: Option<u32>,
    pub fielder_id: Option<u32>,
}

/// What happened on one delivery (post-resolution).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BallOutcome {
    pub runs: u32,
    pub extra_runs: u32,
    pub is_boundary_four: bool,
    pub is_boundary_six: bool,
    pub extra: Option<ExtraKind>,
    pub dismissal: Option<Dismissal>,
    /// Batsman on strike for the *next* ball (single changes strike).
    pub strike_changes: bool,
    pub commentary: String,
}

impl BallOutcome {
    /// Total runs added to the score (bat runs + extras).
    pub fn total(&self) -> u32 {
        self.runs + self.extra_runs
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtraKind {
    Wide,
    NoBall,
    Bye,
    LegBye,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldPreset {
    Attacking,
    Balanced,
    Defensive,
    BoundaryRiding,
    SpinTrap,
}

/// In-match orders. Mutable while the match is live; engine applies them each ball.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchInput {
    pub batting_aggression: u8,
    pub keep_strike_toggle: bool,
    pub hit_and_run_toggle: bool,
    pub target_bowler_id: u32,
    pub field_preset: FieldPreset,
}

impl Default for MatchInput {
    fn default() -> Self {
        MatchInput {
            batting_aggression: 50,
            keep_strike_toggle: false,
            hit_and_run_toggle: false,
            target_bowler_id: 0,
            field_preset: FieldPreset::Balanced,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattingCard {
    pub player_id: u32,
    pub name: String,
    pub runs: u32,
    pub balls: u32,
    pub fours: u8,
    pub sixes: u8,
    pub out: Option<Dismissal>,
    pub batting_position: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BowlingCard {
    pub player_id: u32,
    pub name: String,
    pub balls: u32,
    pub runs: u32,
    pub wickets: u8,
    pub maidens: u8,
    pub wides: u8,
    pub no_balls: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Innings {
    pub batting_team_id: u32,
    pub bowling_team_id: u32,
    pub runs: u32,
    pub wickets: u8,
    pub balls: u32,
    pub extras: u32,
    pub batting_cards: Vec<BattingCard>,
    pub bowling_cards: Vec<BowlingCard>,
    pub fow: Vec<u32>,
    pub over_summaries: Vec<OverSummary>,
    pub is_declared: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverSummary {
    pub over: u32,
    pub runs: u32,
    pub wickets: u8,
    pub bowler_id: u32,
}

impl Innings {
    pub fn overs(&self) -> f64 {
        self.balls as f64 / 6.0
    }

    pub fn striker_index(&self, striker_id: u32) -> Option<usize> {
        self.batting_cards
            .iter()
            .position(|c| c.player_id == striker_id)
    }
}

impl MatchSnapshot {
    /// An empty placeholder used when no match is live (post-match screens).
    pub fn empty() -> Self {
        MatchSnapshot {
            match_id: 0,
            format: crate::core::types::Format::T20,
            ground_id: 0,
            home_team_id: 0,
            away_team_id: 0,
            phase: MatchPhase::Complete,
            current_innings: 0,
            innings: Vec::new(),
            striker_id: None,
            non_striker_id: None,
            bowler_id: None,
            toss_winner: 0,
            toss_decision: TossDecision::Bat,
            seed: 0,
            clock: MatchClock::default(),
            recent_balls: Vec::new(),
            result: None,
        }
    }
}

/// Full match snapshot: everything the UI needs to render.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchSnapshot {
    pub match_id: u32,
    pub format: Format,
    pub ground_id: u32,
    pub home_team_id: u32,
    pub away_team_id: u32,
    pub phase: MatchPhase,
    pub current_innings: usize,
    pub innings: Vec<Innings>,
    pub striker_id: Option<u32>,
    pub non_striker_id: Option<u32>,
    pub bowler_id: Option<u32>,
    pub toss_winner: u32,
    pub toss_decision: TossDecision,
    pub seed: u64,
    pub clock: MatchClock,
    pub recent_balls: Vec<BallOutcome>,
    pub result: Option<MatchResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TossDecision {
    Bat,
    Bowl,
}

/// Tracks day/session for Tests so day boundaries and follow-on logic work.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchClock {
    pub day: u8,
    pub session: u8,
    pub balls_today: u32,
    pub balls_in_session: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchResult {
    pub winner_team_id: Option<u32>,
    pub win_margin: WinMargin,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WinMargin {
    None,
    Runs(u32),
    Wickets(u8),
    Innings(u32),
    Draw,
}
