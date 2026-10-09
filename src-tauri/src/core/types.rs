use serde::{Deserialize, Serialize};

/// Match format. Rules differ materially (overs, PP, red vs white ball).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Test,
    Odi50,
    T20,
}

impl Format {
    pub fn max_overs(self) -> Option<u32> {
        match self {
            Format::Test => None,
            Format::Odi50 => Some(50),
            Format::T20 => Some(20),
        }
    }

    pub fn powerplay_overs(self) -> u32 {
        match self {
            Format::Test => 0,
            Format::Odi50 => 10,
            Format::T20 => 6,
        }
    }

    pub fn is_red_ball(self) -> bool {
        matches!(self, Format::Test)
    }

    pub fn label(self) -> &'static str {
        match self {
            Format::Test => "Test",
            Format::Odi50 => "50-over",
            Format::T20 => "T20",
        }
    }
}

/// How a player can be used in the XI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Batter,
    Wicketkeeper,
    Allrounder,
    PaceBowler,
    SpinBowler,
}

/// Skill rating, 1-99. Used for both current form-adjusted and ceiling potential.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rating(pub u8);

impl Rating {
    pub fn new(v: u8) -> Self {
        Rating(v.min(99))
    }

    /// 0.0-1.0 normalized for arithmetic.
    pub fn norm(self) -> f64 {
        (self.0 as f64) / 99.0
    }
}

/// Batting/bowling/fielding skill block.
///
/// Format-specific bowling ratings exist because Cricket Coach 2014 models
/// distinct LO skills; pace ratings are red-ball, tempo ratings white-ball.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skills {
    pub bat: Rating,
    pub test_bat: Rating,
    pub odi_bat: Rating,
    pub t20_bat: Rating,
    pub defence: Rating,
    pub pace: Rating,
    pub spin: Rating,
    pub odi_bowl: Rating,
    pub t20_bowl: Rating,
    pub fielding: Rating,
    pub keeping: Rating,
    pub leadership: Rating,
}

impl Default for Skills {
    fn default() -> Self {
        Skills {
            bat: Rating(20),
            test_bat: Rating(20),
            odi_bat: Rating(20),
            t20_bat: Rating(20),
            defence: Rating(20),
            pace: Rating(1),
            spin: Rating(1),
            odi_bowl: Rating(1),
            t20_bowl: Rating(1),
            fielding: Rating(20),
            keeping: Rating(1),
            leadership: Rating(20),
        }
    }
}

impl Skills {
    /// Effective batting rating for a format, blending general bat with format skill.
    pub fn bat_for(self, format: Format) -> Rating {
        let specific = match format {
            Format::Test => self.test_bat,
            Format::Odi50 => self.odi_bat,
            Format::T20 => self.t20_bat,
        };
        // 60% format-specific, 40% general batting ceiling.
        let blended = 0.6 * specific.norm() + 0.4 * self.bat.norm();
        Rating::new((blended * 99.0).round() as u8)
    }

    /// Effective bowling rating for a format, taking the better of pace/spin
    /// then blending with the format-specific ball-execution rating.
    pub fn bowl_for(self, format: Format) -> Rating {
        let raw = self.pace.max(self.spin);
        let exec = match format {
            Format::Test => self.pace.max(self.spin),
            Format::Odi50 => self.odi_bowl,
            Format::T20 => self.t20_bowl,
        };
        let blended = 0.65 * raw.norm() + 0.35 * exec.norm();
        Rating::new((blended * 99.0).round() as u8)
    }
}

trait RatingExt {
    fn max(self, other: Rating) -> Rating;
}

impl RatingExt for Rating {
    fn max(self, other: Rating) -> Rating {
        if self.0 >= other.0 {
            self
        } else {
            other
        }
    }
}

/// A player. Fictional names only in shipped data (see `data/README.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub nation_id: u32,
    pub team_id: Option<u32>,
    pub age: u8,
    pub role: Role,
    pub batting_style: String,
    pub bowling_style: Option<String>,
    /// Current ability (form adjusted, moves toward potential).
    pub skills: Skills,
    /// Career ceiling; growth systems use this in M5.
    pub potential: Skills,
    pub form: i8,
    pub fitness: u8,
    pub morale: u8,
    pub experience: u32,
}

impl Default for Player {
    fn default() -> Self {
        Player {
            id: 0,
            name: "New Player".into(),
            nation_id: 0,
            team_id: None,
            age: 22,
            role: Role::Batter,
            batting_style: "rhb".into(),
            bowling_style: None,
            skills: Skills::default(),
            potential: Skills::default(),
            form: 0,
            fitness: 100,
            morale: 70,
            experience: 0,
        }
    }
}

impl Player {
    /// Placeholder for an unknown id (keeps the UI crash-free).
    pub fn with_name(id: u32) -> Self {
        Player {
            name: format!("Player #{}", id),
            ..Default::default()
        }
    }
}

/// A nation / cricket board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nation {
    pub id: u32,
    pub name: String,
    pub short_name: String,
    /// 0-100 board strength; drives regens, rankings floors, minnow weighting.
    pub strength: u8,
    pub has_test_status: bool,
}

/// A team: domestic club or national side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Team {
    pub id: u32,
    pub name: String,
    pub nation_id: u32,
    pub home_ground_id: u32,
    pub budget: i64,
    pub reputation: Rating,
    /// Full-time contract list (squad).
    #[serde(default)]
    pub squad_ids: Vec<u32>,
    /// True for the national side (as opposed to domestic clubs).
    #[serde(default)]
    pub is_national: bool,
}

/// Ground properties affect scoring (boundary size, pitch, altitude, weather bias).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ground {
    pub id: u32,
    pub name: String,
    pub nation_id: u32,
    pub boundary_m: u16,
    /// 0 = flat batting deck, 100 = bowler's graveyard.
    pub batting_friendly: u8,
    pub day_night: bool,
}

/// Competition definition (league round-robin, cup, series, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Competition {
    pub id: u32,
    pub name: String,
    pub nation_id: u32,
    pub format: Format,
    pub kind: CompetitionKind,
    pub team_ids: Vec<u32>,
    pub days_per_match: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompetitionKind {
    League,
    KnockoutCup,
    Series,
    Tour,
}

/// Immutable world data: the fictional 2027 database.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct World {
    pub schema_version: u32,
    /// Bumped per DB publish; saves pin this (specs §5 UR-5).
    pub db_version: String,
    #[serde(default)]
    pub nations: Vec<Nation>,
    #[serde(default)]
    pub teams: Vec<Team>,
    #[serde(default)]
    pub players: Vec<Player>,
    #[serde(default)]
    pub grounds: Vec<Ground>,
    #[serde(default)]
    pub competitions: Vec<Competition>,
}
