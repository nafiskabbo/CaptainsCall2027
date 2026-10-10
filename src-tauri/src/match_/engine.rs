//! Ball resolution model.
//!
//! Design: outcome ~ a base per-format weighting table, scaled by relative
//! batter/bowler quality, aggression, field preset and phase. Determinism comes
//! from the caller passing `&mut SimRng` (specs FR-13, NFR-7).

use super::state::{
    BallOutcome, Dismissal, DismissalType, ExtraKind, FieldPreset, Innings, MatchInput, ShotKind,
};
use crate::core::rng::{clamp, Distribution, SimRng};
use crate::core::types::{Format, Player, Team};

/// Outcome bucket indices.
const DOT: usize = 0;
const ONE: usize = 1;
const TWO: usize = 2;
const THREE: usize = 3;
const FOUR: usize = 4;
const SIX: usize = 5;
const OUT: usize = 6;
const WIDE: usize = 7;
const NOBALL: usize = 8;

/// Per-format shape anchors.
/// [Test, ODI, T20] — (target runs per ball, boundary multiplier, wickets per 100 balls).
const FORMAT_ANCHOR: [(f64, f64, f64); 3] = [
    (0.50, 0.85, 1.6), // Test
    (0.92, 1.55, 3.6), // ODI
    (1.40, 1.95, 5.4), // T20
];

/// Base per-100-balls weights, rescaled per format below.
const BASE: [f64; 9] = [
    40.0, // dot
    28.0, // one
    7.0,  // two
    1.1,  // three
    5.0,  // four
    1.4,  // six
    4.2,  // wicket
    0.7,  // wide
    0.12, // no ball
];

/// Context handed to the ball resolver.
pub struct BallContext<'a> {
    pub striker: &'a Player,
    pub batter_id: u32,
    pub bowler: &'a Player,
    pub fielding_team: &'a Team,
    pub innings: &'a Innings,
    pub format: Format,
    pub over: u32,
    pub powerplay: bool,
    pub chasing: bool,
    pub run_rate_needed: f64,
    /// Minnow weighting: >1.0 when this batting side is the underdog.
    pub minnow_boost: f64,
    pub input: &'a MatchInput,
    pub rng: &'a mut SimRng,
}

/// Deliver a single ball and resolve the outcome.
pub fn resolve_ball(ctx: &mut BallContext<'_>) -> BallOutcome {
    let (_target_rpb, power_k, target_wpb) = FORMAT_ANCHOR[format_index(ctx.format)];

    // --- relative quality, smoothed so extreme ratings can't explode ---
    let bat = ctx.striker.skills.bat_for(ctx.format).norm();
    let bowl = ctx.bowler.skills.bowl_for(ctx.format).norm();
    let defence = ctx.striker.skills.defence.norm();
    let rel = (0.62 * bat + 0.38 * defence + 0.05) / (0.80 * bowl + 0.15);

    let aggression = (ctx.input.batting_aggression as f64 / 100.0).clamp(0.0, 1.0);
    let rr_now = if ctx.innings.balls > 0 {
        ctx.innings.runs as f64 / (ctx.innings.balls as f64 / 6.0)
    } else {
        0.0
    };
    let chase_pressure = if ctx.chasing && ctx.innings.balls > 0 {
        clamp((ctx.run_rate_needed - rr_now) / 4.0, -0.5, 0.9)
    } else {
        0.0
    };

    // --- weights: base shape then modifiers ---
    // Minnow weighting lifts the underdog's scoring so an upset is possible.
    let boost = ctx.minnow_boost.clamp(1.0, 1.25);
    let mut w = BASE;
    // Wicket-less format baseline: better batter = more runs, fewer dots.
    w[DOT] *= 1.0 / rel;
    w[ONE] *= rel;
    w[TWO] *= rel;
    w[THREE] *= rel;
    w[FOUR] *= rel;
    w[SIX] *= rel;
    w[OUT] *= (0.72 * bowl + 0.12) / defence.max(0.2) * 0.42;
    w[WIDE] *= 1.08 - 0.16 * bowl;
    w[NOBALL] *= 1.08 - 0.1 * bowl;

    // Scaling set: aggression pushes power, keeps rotation.
    w[ONE] *= 0.78 + 0.44 * aggression;
    w[TWO] *= 0.62 + 0.76 * aggression;
    w[THREE] *= 0.55 + 0.9 * aggression;
    w[FOUR] *= 0.45 + 1.15 * aggression;
    w[SIX] *= 0.30 + 1.85 * aggression;
    w[DOT] *= 1.28 - 0.5 * aggression;

    // Format calibration on rotation + boundaries.
    w[FOUR] *= power_k;
    w[SIX] *= power_k;
    // Powerplay: more boundaries, slightly more risk.
    if ctx.powerplay {
        w[FOUR] *= 1.18;
        w[SIX] *= 1.1;
        w[OUT] *= 1.05;
    }

    // New batter: settling in.
    let balls_faced = ctx
        .innings
        .batting_cards
        .iter()
        .find(|c| c.player_id == ctx.batter_id)
        .map(|c| c.balls)
        .unwrap_or(0);
    let set_mult = if balls_faced < 12 {
        0.86
    } else if balls_faced < 30 {
        1.06
    } else {
        1.0
    };
    w[FOUR] *= set_mult;
    w[SIX] *= set_mult;
    if balls_faced < 12 {
        w[OUT] *= 1.06;
    }

    // Chase pressure.
    if chase_pressure > 0.0 {
        w[FOUR] *= 1.0 + 0.22 * chase_pressure;
        w[SIX] *= 1.0 + 0.3 * chase_pressure;
        w[THREE] *= 1.0 + 0.15 * chase_pressure;
        w[OUT] *= 1.0 + 0.06 * chase_pressure;
    } else if chase_pressure < 0.0 {
        let ease = (-chase_pressure).min(0.5);
        w[DOT] *= 1.0 + 0.22 * ease;
        w[FOUR] *= 1.0 - 0.12 * ease;
        w[OUT] *= 1.0 - 0.1 * ease;
    }

    // Field presets.
    match ctx.input.field_preset {
        FieldPreset::BoundaryRiding => {
            w[FOUR] *= 0.84;
            w[SIX] *= 0.8;
            w[OUT] *= 1.06;
        }
        FieldPreset::Attacking => {
            w[OUT] *= 1.14;
            w[FOUR] *= 1.06;
        }
        FieldPreset::Defensive => {
            w[DOT] *= 1.14;
            w[FOUR] *= 0.93;
            w[OUT] *= 0.88;
        }
        FieldPreset::Balanced => {}
        FieldPreset::SpinTrap => {
            w[DOT] *= 1.2;
            w[OUT] *= 1.12;
        }
    }
    if ctx.input.hit_and_run_toggle {
        w[ONE] *= 1.14;
        w[OUT] *= 1.04;
    }

    // Apply the minnow boost before the final normalisation so the RPB anchor
    // still holds, but the *distribution* favours the underdog's boundaries.
    if boost > 1.0 {
        w[FOUR] *= boost;
        w[SIX] *= boost;
        w[ONE] *= boost.sqrt();
    }

    // Final pass: normalise scoring toward the format runs anchor. The batch
    // derivation makes this fast but approximate; the anchor keeps mean runs and
    // WP balls per 100 in band no matter how modifiers have shifted them.
    let target_rpb = FORMAT_ANCHOR[format_index(ctx.format)].0;
    let total: f64 = w.iter().sum();
    if total > 0.0 {
        let expected =
            (w[ONE] + 2.0 * w[TWO] + 3.0 * w[THREE] + 4.0 * w[FOUR] + 6.0 * w[SIX]) / total;
        if expected > 0.0 {
            let k = (target_rpb / expected).clamp(0.4, 2.5);
            for i in [ONE, TWO, THREE, FOUR, SIX] {
                w[i] *= k;
            }
        }
    }
    let wpb_now = w[OUT];
    if wpb_now > 0.0 {
        let k = target_wpb / wpb_now;
        w[OUT] *= k;
    }

    let wicket_ctx = WicketContext::from(ctx);
    let rng: &mut SimRng = &mut *ctx.rng;
    let dist = Distribution::new(w.to_vec());
    let idx = dist.pick(rng);

    match idx {
        DOT => plain(0, false, rng),
        ONE => plain(1, false, rng),
        TWO => plain(2, false, rng),
        THREE => plain(3, true, rng),
        FOUR => four(rng),
        SIX => six(rng),
        OUT => wicket(&wicket_ctx, rng),
        WIDE => BallOutcome {
            runs: 0,
            extra_runs: 1,
            is_boundary_four: false,
            is_boundary_six: false,
            extra: Some(ExtraKind::Wide),
            dismissal: None,
            strike_changes: false,
            commentary: commentary(
                rng,
                &[
                    "Wide, too far outside off.",
                    "Sprayed down leg, wide called.",
                ],
            ),
            // A wide never reaches the bat, so it goes past the off side.
            direction_deg: (rng.below(50) + 70) as u16,
            shot: ShotKind::Beaten,
        },
        NOBALL => BallOutcome {
            runs: 0,
            extra_runs: 1,
            is_boundary_four: false,
            is_boundary_six: false,
            extra: Some(ExtraKind::NoBall),
            dismissal: None,
            strike_changes: false,
            commentary: commentary(
                rng,
                &[
                    "No ball — overstepped!",
                    "No ball called, free hit incoming.",
                ],
            ),
            direction_deg: (rng.below(120) + 30) as u16,
            shot: ShotKind::Driven,
        },
        _ => plain(0, false, rng),
    }
}

/// Pick a bearing, weighted so square and straight shots are commoner than
/// fine edges. `centre` is the mode in degrees, `spread` the half-width.
fn bearing(rng: &mut SimRng, centre: u16, spread: u16) -> u16 {
    let offset = rng.below(spread.max(1) as u32) as i32 - (spread / 2) as i32;
    (centre as i32 + offset).rem_euclid(360) as u16
}

/// The off side spans roughly 20-100 degrees; leg is 100-200.
fn off_side(rng: &mut SimRng) -> u16 {
    bearing(rng, 60, 80)
}

fn leg_side(rng: &mut SimRng) -> u16 {
    bearing(rng, 150, 70)
}

fn four(rng: &mut SimRng) -> BallOutcome {
    // Boundaries favour the off side and square cuts over the leg side.
    let direction_deg = match rng.below(4) {
        0 => bearing(rng, 30, 30),
        1 => bearing(rng, 90, 40),
        2 => bearing(rng, 165, 45),
        _ => bearing(rng, 60, 60),
    };
    BallOutcome {
        runs: 4,
        extra_runs: 0,
        is_boundary_four: true,
        is_boundary_six: false,
        extra: None,
        dismissal: None,
        strike_changes: false,
        commentary: commentary(
            rng,
            &[
                "FOUR! Cracked through the covers.",
                "FOUR! Threaded fine past the slips.",
                "FOUR! Timing, no hesitation.",
                "FOUR! Beats the sweeper.",
            ],
        ),
        direction_deg,
        shot: ShotKind::Driven,
    }
}

fn six(rng: &mut SimRng) -> BallOutcome {
    // A six is nearly always lofted straight or over midwicket.
    let direction_deg = match rng.below(3) {
        0 => bearing(rng, 20, 30),
        1 => bearing(rng, 120, 50),
        _ => bearing(rng, 160, 40),
    };
    BallOutcome {
        runs: 6,
        extra_runs: 0,
        is_boundary_four: false,
        is_boundary_six: true,
        extra: None,
        dismissal: None,
        strike_changes: false,
        commentary: commentary(
            rng,
            &[
                "SIX! Launched over midwicket.",
                "SIX! That's out of the ground.",
                "SIX! Picked the length early.",
            ],
        ),
        direction_deg,
        shot: ShotKind::Lofted,
    }
}

/// Extracted wicket inputs (keeps `rng` borrowable independently of `ctx`).
struct WicketContext {
    bowler_id: u32,
    bowl_norm: f64,
    spin_affinity: f64,
}

impl WicketContext {
    fn from(ctx: &BallContext<'_>) -> Self {
        let spin_affinity = if ctx.bowler.skills.spin.0 > ctx.bowler.skills.pace.0 {
            1.2
        } else {
            1.0
        };
        WicketContext {
            bowler_id: ctx.bowler.id,
            bowl_norm: ctx.bowler.skills.bowl_for(ctx.format).norm(),
            spin_affinity,
        }
    }
}

fn plain(runs: u32, strike_changes: bool, rng: &mut SimRng) -> BallOutcome {
    let text = run_text(runs);
    let lines: Vec<&str> = vec![text.as_str()];
    // Scoring shots go where the ball is actually played: mostly through the
    // off side, with leg-side singles and a few pushed straight.
    let (direction_deg, shot) = match runs {
        0 => match rng.below(3) {
            0 => (bearing(rng, 0, 40), ShotKind::Defended),
            1 => (bearing(rng, 175, 40), ShotKind::Defended),
            _ => (leg_side(rng), ShotKind::Beaten),
        },
        1 => (off_side(rng), ShotKind::Driven),
        2 => (off_side(rng), ShotKind::Driven),
        _ => (leg_side(rng), ShotKind::Driven),
    };
    BallOutcome {
        runs,
        extra_runs: 0,
        is_boundary_four: runs == 4,
        is_boundary_six: runs == 6,
        extra: None,
        dismissal: None,
        strike_changes,
        commentary: commentary(rng, &lines),
        direction_deg,
        shot,
    }
}

fn run_text(runs: u32) -> String {
    match runs {
        0 => "Defended back down the ground.".to_string(),
        1 => "Pushed into the gap, single taken.".to_string(),
        2 => "Driven into the deep, they come back for the second.".to_string(),
        3 => "Three! Excellent running between the wickets.".to_string(),
        4 => "FOUR! Cracked through the covers.".to_string(),
        6 => "SIX! Launched high over long-on!".to_string(),
        _ => "Ball played away.".to_string(),
    }
}

/// Resolve a wicket ball.
fn wicket(ctx: &WicketContext, rng: &mut SimRng) -> BallOutcome {
    let bowl = ctx.bowl_norm;
    let spin_affinity = ctx.spin_affinity;
    let weights = vec![
        16.0 * spin_affinity,     // caught
        3.2,                      // lbw
        6.0 * (0.7 * bowl + 0.3), // bowled (pace bowls more)
        0.9,                      // caught & bowled
        1.3,                      // run out
        0.45,                     // stumped
        0.35,                     // hit wicket
    ];
    let idx = Distribution::new(weights).pick(rng);
    let kind = match idx {
        0 => DismissalType::Caught,
        1 => DismissalType::Lbw,
        2 => DismissalType::Bowled,
        3 => DismissalType::CaughtAndBowled,
        4 => DismissalType::RunOut,
        5 => DismissalType::Stumped,
        _ => DismissalType::HitWicket,
    };
    let commentary = match kind {
        DismissalType::Caught => "Taken! Caught, huge appeal, up goes the finger!",
        DismissalType::Lbw => "Given! Plumb in front, LBW.",
        DismissalType::Bowled => "Bowled! Through the gate, timber!",
        DismissalType::CaughtAndBowled => "Caught and bowled! Sharp chance, taken!",
        DismissalType::RunOut => "Run out! Direct hit, superb fielding!",
        DismissalType::Stumped => "Stumped! Beaten by the turn, keeper does the rest!",
        DismissalType::HitWicket => "Hit wicket! Trod on his own stumps.",
        DismissalType::NotOut => "Not out.",
    };
    // Where the dismissal happened, which is what the view animates to:
    // caught goes to a fielder, bowled/lbw stay at the pitch, a run out is
    // made down the pitch to the stumps at the bowler's end.
    let (direction_deg, shot) = match kind {
        DismissalType::Caught => (bearing(rng, 55, 90), ShotKind::Edged),
        DismissalType::CaughtAndBowled => (bearing(rng, 150, 70), ShotKind::Edged),
        DismissalType::Lbw => (0, ShotKind::Missed),
        DismissalType::Bowled => (0, ShotKind::Missed),
        DismissalType::Stumped => (170, ShotKind::Beaten),
        DismissalType::RunOut => (180, ShotKind::Defended),
        DismissalType::HitWicket => (0, ShotKind::Defended),
        DismissalType::NotOut => (0, ShotKind::Defended),
    };
    BallOutcome {
        runs: 0,
        extra_runs: 0,
        is_boundary_four: false,
        is_boundary_six: false,
        extra: None,
        dismissal: Some(Dismissal {
            out_type: kind,
            bowler_id: if matches!(kind, DismissalType::RunOut) {
                None
            } else {
                Some(ctx.bowler_id)
            },
            fielder_id: None,
        }),
        strike_changes: false,
        commentary: commentary.to_string(),
        direction_deg,
        shot,
    }
}

/// Average rating of a side, used to size the minnow weighting.
pub fn side_strength(players: &[Player]) -> u8 {
    if players.is_empty() {
        return 50;
    }
    let total: u32 = players
        .iter()
        .map(|p| {
            let best_bat = p.skills.bat_for(Format::Test).0;
            let best_bowl = p.skills.pace.0.max(p.skills.spin.0);
            best_bat.max(best_bowl) as u32
        })
        .sum();
    (total / players.len() as u32).clamp(20, 99) as u8
}

/// Boost applied to the weaker side's scoring (1.0 at even strength).
pub fn minnow_boost(mine: u8, theirs: u8) -> f64 {
    let gap = (theirs as f64 - mine as f64).max(0.0);
    (1.0 + (gap / 100.0) * 0.62).min(1.25)
}

fn format_index(format: Format) -> usize {
    match format {
        Format::Test => 0,
        Format::Odi50 => 1,
        Format::T20 => 2,
    }
}

/// Pick a line at random and append a period.
fn commentary(rng: &mut SimRng, lines: &[&str]) -> String {
    format!("{}.", rng.pick(lines))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Rating;

    fn base_player(id: u32, bat: u8, bowl: u8, spin: bool) -> Player {
        let mut skills = crate::core::types::Skills {
            bat: Rating(bat),
            test_bat: Rating(bat),
            odi_bat: Rating(bat),
            t20_bat: Rating(bat),
            defence: Rating(bat),
            fielding: Rating(bat),
            ..Default::default()
        };
        let bowl_r = Rating(bowl);
        if spin {
            skills.spin = bowl_r;
        } else {
            skills.pace = bowl_r;
        }
        skills.odi_bowl = bowl_r;
        skills.t20_bowl = bowl_r;
        Player {
            id,
            name: format!("P{id}"),
            nation_id: 1,
            team_id: Some(1),
            age: 26,
            role: crate::core::types::Role::Batter,
            batting_style: "rhb".into(),
            bowling_style: None,
            skills,
            potential: skills,
            form: 0,
            fitness: 100,
            morale: 70,
            experience: 30,
            injury_days: 0,
        }
    }

    /// Rough sanity of the outcome distribution: mean runs per ball and
    /// wicket rate should sit in a sensible band per format.
    #[test]
    fn minnow_boost_favours_the_weaker_side() {
        assert_eq!(minnow_boost(80, 80), 1.0, "even sides get no boost");
        assert!(minnow_boost(60, 90) > 1.0, "underdogs are boosted");
        assert!(
            minnow_boost(90, 60) == 1.0,
            "the stronger side gets nothing"
        );
        assert!(minnow_boost(40, 99) <= 1.25, "the boost is capped");
        assert!(
            minnow_boost(60, 90) > minnow_boost(70, 90),
            "a bigger gap means a bigger boost"
        );
    }

    #[test]
    fn side_strength_is_an_average_of_the_best_skills() {
        let strong: Vec<Player> = (0..11).map(|i| base_player(1 + i, 85, 30, false)).collect();
        let weak: Vec<Player> = (0..11).map(|i| base_player(1 + i, 45, 30, false)).collect();
        let (s, w) = (side_strength(&strong), side_strength(&weak));
        assert!(s > w, "a better squad must rate higher ({s} vs {w})");
        assert!((20..=99).contains(&s) && (20..=99).contains(&w));
    }

    #[test]
    fn underdogs_can_produce_competitive_totals() {
        // A weak side batting against a strong attack should still be able to
        // post a defendable score thanks to the boost.
        let weak_batters: Vec<Player> =
            (0..11).map(|i| base_player(1 + i, 45, 20, false)).collect();
        let strong_bowlers: Vec<Player> = (0..8)
            .map(|i| base_player(100 + i, 30, 88, i % 2 == 0))
            .collect();
        let team = crate::core::types::Team {
            id: 9,
            name: "Underdogs".into(),
            nation_id: 1,
            home_ground_id: 1,
            budget: 0,
            reputation: Rating(40),
            squad_ids: vec![],
            is_national: false,
        };
        let input = MatchInput::default();
        let mut innings = super::super::state::Innings {
            batting_team_id: 1,
            bowling_team_id: 2,
            runs: 0,
            wickets: 0,
            balls: 0,
            extras: 0,
            batting_cards: vec![],
            bowling_cards: vec![],
            fow: vec![],
            over_summaries: vec![],
            is_declared: false,
        };
        let mut runs = 0u32;
        let mut balls = 0u32;
        for i in 0..2000 {
            innings.balls = (i % 6) as u32;
            let batter = &weak_batters[i % 11];
            let bowler = &strong_bowlers[i % 8];
            let mut ctx = BallContext {
                striker: batter,
                batter_id: batter.id,
                bowler,
                fielding_team: &team,
                innings: &innings,
                format: Format::T20,
                over: 0,
                powerplay: false,
                chasing: false,
                run_rate_needed: 0.0,
                minnow_boost: sim_minnow(),
                input: &input,
                rng: &mut SimRng::new(77 + i as u64),
            };
            let o = resolve_ball(&mut ctx);
            runs += o.total();
            balls += 1;
        }
        let rpo = runs as f64 / (balls as f64 / 6.0);
        assert!(rpo > 6.0, "underdogs should still score, got {rpo:.2} rpo");
    }

    fn sim_minnow() -> f64 {
        minnow_boost(45, 88)
    }

    #[test]
    fn distribution_is_sane() {
        for (format, expected_rpb, max_wp_100) in [
            (Format::Test, 0.52, 4.0),
            (Format::Odi50, 0.92, 5.0),
            (Format::T20, 1.40, 6.5),
        ] {
            let striker = base_player(1, 70, 30, false);
            let bowler = base_player(2, 30, 70, true);
            let team = crate::core::types::Team {
                id: 9,
                name: "T".into(),
                nation_id: 1,
                home_ground_id: 1,
                budget: 0,
                reputation: Rating(60),
                squad_ids: vec![],
                is_national: false,
            };
            let mut innings = super::super::state::Innings {
                batting_team_id: 1,
                bowling_team_id: 2,
                runs: 0,
                wickets: 0,
                balls: 0,
                extras: 0,
                batting_cards: vec![],
                bowling_cards: vec![],
                fow: vec![],
                over_summaries: vec![],
                is_declared: false,
            };
            let input = MatchInput::default();
            let mut rng = SimRng::new(4242);
            let mut runs = 0.0;
            let mut balls = 0.0;
            let mut wickets = 0.0;
            for i in 0..20000 {
                innings.balls = (i % 6) as u32;
                let mut ctx = BallContext {
                    striker: &striker,
                    batter_id: striker.id,
                    bowler: &bowler,
                    fielding_team: &team,
                    innings: &innings,
                    format,
                    over: 0,
                    powerplay: false,
                    chasing: false,
                    run_rate_needed: 0.0,
                    minnow_boost: 1.0,
                    input: &input,
                    rng: &mut rng,
                };
                let o = resolve_ball(&mut ctx);
                runs += o.total() as f64;
                balls += if o.extra.is_none() { 1.0 } else { 0.0 };
                if o.dismissal.is_some() {
                    wickets += 1.0;
                }
            }
            let rpb: f64 = runs / balls;
            let wp_100: f64 = wickets / balls * 100.0;
            assert!(
                (rpb - expected_rpb).abs() < 0.45,
                "{:?} runs per ball {rpb:.3}, expected ~{expected_rpb}",
                format
            );
            assert!(
                wp_100 < max_wp_100,
                "{:?} wickets per 100 balls {wp_100:.2} too high",
                format
            );
        }
    }
}
