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
    /// Where the ball went, as a bearing in degrees off the striker's stumps.
    ///
    /// 0 = straight back past the bowler, 90 = square on the off side,
    /// 180 = fine leg. This is what lets the 2D view animate the actual shot
    /// instead of looping a fixed bowling action.
    #[serde(default)]
    pub direction_deg: u16,
    /// How the ball was played. Drives the ball's height/arc in the view.
    #[serde(default)]
    pub shot: ShotKind,
    /// The outcome the batter attempted, when they chose one.
    #[serde(default)]
    pub attempted: Option<Stroke>,
}

/// A stroke the batter can commit to on a delivery.
///
/// These are the original's twelve, in its own codes, from
/// `db/db2014/bbbc_shot.csv`. The earlier build had the player pick an outcome
/// number — `0` through `6` — which felt right and was wrong: the original asks
/// which *stroke* you are playing, and the runs follow from how that stroke
/// meets a particular delivery. Picking "cover drive" against a good-length
/// inswinger is a decision that can go several ways; picking "4" is a slot
/// machine with four buttons.
///
/// See `docs/cc2014-model.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stroke {
    Leave,
    FrontDefensive,
    BackDefensive,
    StraightDrive,
    CoverDrive,
    LegGlance,
    Cut,
    Pull,
    Hook,
    Sweep,
    OnDrive,
    OffDrive,
}

impl Stroke {
    /// The original's numeric code, kept so commentary tables and saved games
    /// line up with the source data.
    pub fn code(self) -> u32 {
        match self {
            Stroke::Leave => 20100,
            Stroke::FrontDefensive => 20101,
            Stroke::BackDefensive => 20102,
            Stroke::StraightDrive => 20103,
            Stroke::CoverDrive => 20104,
            Stroke::LegGlance => 20105,
            Stroke::Cut => 20106,
            Stroke::Pull => 20107,
            Stroke::Hook => 20108,
            Stroke::Sweep => 20109,
            Stroke::OnDrive => 20111,
            Stroke::OffDrive => 20112,
        }
    }

    pub const ALL: [Stroke; 12] = [
        Stroke::Leave,
        Stroke::FrontDefensive,
        Stroke::BackDefensive,
        Stroke::StraightDrive,
        Stroke::CoverDrive,
        Stroke::LegGlance,
        Stroke::Cut,
        Stroke::Pull,
        Stroke::Hook,
        Stroke::Sweep,
        Stroke::OnDrive,
        Stroke::OffDrive,
    ];

    /// Short label for the picker.
    pub fn label(self) -> &'static str {
        match self {
            Stroke::Leave => "Leave",
            Stroke::FrontDefensive => "Forward Def",
            Stroke::BackDefensive => "Back Def",
            Stroke::StraightDrive => "Straight Drive",
            Stroke::CoverDrive => "Cover Drive",
            Stroke::LegGlance => "Leg Glance",
            Stroke::Cut => "Cut",
            Stroke::Pull => "Pull",
            Stroke::Hook => "Hook",
            Stroke::Sweep => "Sweep",
            Stroke::OnDrive => "On Drive",
            Stroke::OffDrive => "Off Drive",
        }
    }

    /// The original's sprite name, without extension.
    pub fn sprite(self) -> &'static str {
        match self {
            Stroke::Leave => "SHOT_LEAVE",
            Stroke::FrontDefensive => "SHOT_FRDEF",
            Stroke::BackDefensive => "SHOT_BDEF",
            Stroke::StraightDrive => "SHOT_STRDRV",
            Stroke::CoverDrive => "SHOT_CDRIVE",
            Stroke::LegGlance => "SHOT_LEGGLANCE",
            Stroke::Cut => "SHOT_CUT",
            Stroke::Pull => "SHOT_PULL",
            Stroke::Hook => "SHOT_HOOK",
            Stroke::Sweep => "SHOT_SWEEP",
            Stroke::OnDrive => "SHOT_ONDRIVE",
            Stroke::OffDrive => "SHOT_DRIVE",
        }
    }

    /// Whether the stroke attacks.
    ///
    /// This is the axis the game plays on, and it is visible in the commentary
    /// data: every `LEAVE` and `FRONT_DEFENSIVE` line is a dot ball, while
    /// `STRAIGHTDRIVE` spans 0 through 6 and dismissal. Defensive strokes cannot
    /// score much and cannot get you out; attacking strokes can do both.
    pub fn is_defensive(self) -> bool {
        matches!(
            self,
            Stroke::Leave | Stroke::FrontDefensive | Stroke::BackDefensive
        )
    }

    /// Whether the stroke is played to the leg side.
    pub fn is_leg_side(self) -> bool {
        matches!(
            self,
            Stroke::Pull | Stroke::Hook | Stroke::Sweep | Stroke::LegGlance
        )
    }
}

/// Where and how the ball was bowled, on the original's two axes.
///
/// The game classifies every delivery before the batter plays a shot, so the
/// stroke is answered against a specific ball rather than in the abstract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BallLength {
    GoodLength,
    ShortOff,
    Yorker,
    Bouncer,
}

impl BallLength {
    /// The original's 8xxx code.
    pub fn code(self) -> u32 {
        match self {
            BallLength::GoodLength => 8001,
            BallLength::ShortOff => 8002,
            BallLength::Yorker => 8003,
            BallLength::Bouncer => 8004,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BallMovement {
    Straight,
    Inswing,
    Outswing,
    OffCutter,
    LegCutter,
    Slower,
    LegBreak,
    OffBreak,
    StraightOne,
    WrongUn,
    Speciality,
}

impl BallMovement {
    /// The original's 9xxx code.
    pub fn code(self) -> u32 {
        match self {
            BallMovement::Straight => 9001,
            BallMovement::Inswing => 9002,
            BallMovement::Outswing => 9003,
            BallMovement::OffCutter => 9004,
            BallMovement::LegCutter => 9005,
            BallMovement::Slower => 9006,
            BallMovement::LegBreak => 9007,
            BallMovement::OffBreak => 9008,
            BallMovement::StraightOne => 9009,
            BallMovement::WrongUn => 9010,
            BallMovement::Speciality => 9011,
        }
    }
}

/// How a shot was played, which decides the animation arc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShotKind {
    /// Straight to a fielder or back down the pitch.
    #[default]
    Defended,
    /// Along the ground into the gaps.
    Driven,
    /// Cross-bat.
    Cut,
    /// Leg-side pull.
    Pulled,
    /// Over the boundary on the full.
    Lofted,
    /// Left the bat and carried to a fielder/slip.
    Edged,
    /// Never left the bat.
    Beaten,
    /// Missed everything.
    Missed,
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
    /// The outcome the batter is attempting this delivery, if they chose one.
    ///
    /// This is the Cricket Coach shot mechanic: instead of the engine simply
    /// resolving a ball, the player commits to an outcome and the engine
    /// resolves *that attempt* — weighted by the matchup, and with the risk of
    /// missing it expressed as extra chances of being dismissed. `None` keeps
    /// the uninfluenced simulation, which is what CPU-vs-CPU and career
    /// auto-simulation use.
    #[serde(default)]
    pub stroke: Option<Stroke>,
}

impl Default for MatchInput {
    fn default() -> Self {
        MatchInput {
            batting_aggression: 50,
            keep_strike_toggle: false,
            hit_and_run_toggle: false,
            target_bowler_id: 0,
            field_preset: FieldPreset::Balanced,
            stroke: None,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Decided by a one-over eliminator after the scores finished level.
    SuperOver,
}
