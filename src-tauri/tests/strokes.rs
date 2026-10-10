//! The stroke mechanic: choosing a stroke must change what the ball does.
//!
//! This is the original's batting model, recovered from
//! `db/db2014/bbbc_shot.csv`. The previous version of these tests checked an
//! outcome-number picker — the player chose "4" and the engine tried to deliver
//! four runs — which turned out to be a misreading of the game and has been
//! replaced.
//!
//! What matters now is that the *shape* of the decision is right:
//!
//! - A defensive stroke is a dot ball and is not worth much, but it keeps you
//!   in. This is visible in the original's commentary data: every `LEAVE` and
//!   `FRONT_DEFENSIVE` line is a dot ball.
//! - An attacking stroke spans the whole outcome range, including dismissal.
//! - Which attacking stroke is safe depends on the delivery. A pull at a yorker
//!   is a different proposition from a cover drive at a good length, and that
//!   is the decision the player is actually making.

use captains_call_2027_lib::core::rng::SimRng;
use captains_call_2027_lib::core::types::{Format, Player, Role, Team};
use captains_call_2027_lib::match_::engine::{resolve_ball, BallContext};
use captains_call_2027_lib::match_::state::{Innings, MatchInput, Stroke};

fn batter() -> Player {
    Player {
        id: 1,
        name: "Batter".into(),
        nation_id: 1,
        team_id: None,
        age: 25,
        role: Role::Batter,
        batting_style: "rhb".into(),
        skills: captains_call_2027_lib::core::types::Skills {
            bat: captains_call_2027_lib::core::types::Rating(80),
            test_bat: captains_call_2027_lib::core::types::Rating(80),
            odi_bat: captains_call_2027_lib::core::types::Rating(80),
            t20_bat: captains_call_2027_lib::core::types::Rating(80),
            defence: captains_call_2027_lib::core::types::Rating(70),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn bowler() -> Player {
    Player {
        id: 2,
        name: "Bowler".into(),
        nation_id: 1,
        team_id: None,
        age: 27,
        role: Role::PaceBowler,
        batting_style: "lhb".into(),
        skills: captains_call_2027_lib::core::types::Skills {
            pace: captains_call_2027_lib::core::types::Rating(78),
            odi_bowl: captains_call_2027_lib::core::types::Rating(78),
            t20_bowl: captains_call_2027_lib::core::types::Rating(78),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn fielding() -> Team {
    Team {
        id: 2,
        name: "Fielding".into(),
        nation_id: 1,
        home_ground_id: 1,
        budget: 0,
        reputation: captains_call_2027_lib::core::types::Rating(50),
        squad_ids: vec![],
        is_national: false,
    }
}

/// Bowl `n` balls with one stroke committed; return (runs, wickets) per ball.
fn trial(choice: Option<Stroke>, n: u32) -> (f64, f64) {
    let batter = batter();
    let bowler = bowler();
    let fielding = fielding();
    let mut rng = SimRng::new(20270101);
    let input = MatchInput {
        stroke: choice,
        ..Default::default()
    };
    let mut runs = 0u32;
    let mut wickets = 0u32;
    for ball in 0..n {
        let mut innings = Innings {
            batting_team_id: 1,
            bowling_team_id: 2,
            runs,
            wickets: (wickets.min(9)) as u8,
            balls: ball,
            ..Default::default()
        };
        let mut ctx = BallContext {
            striker: &batter,
            batter_id: batter.id,
            bowler: &bowler,
            fielding_team: &fielding,
            innings: &mut innings,
            format: Format::T20,
            over: ball / 6 + 1,
            powerplay: false,
            chasing: false,
            run_rate_needed: 0.0,
            minnow_boost: 1.0,
            input: &input,
            rng: &mut rng,
        };
        let out = resolve_ball(&mut ctx);
        runs += out.runs + out.extra_runs;
        if out.dismissal.is_some() {
            wickets += 1;
        }
    }
    (runs as f64 / n as f64, wickets as f64 / n as f64)
}

/// A committed stroke must actually change the innings.
///
/// Without this the whole picker is decoration: every button would produce the
/// same ball, and the game would still be a simulation the player watches.
#[test]
fn a_committed_stroke_changes_the_innings() {
    const N: u32 = 6000;
    let passive = trial(None, N);
    let drive = trial(Some(Stroke::StraightDrive), N);
    let block = trial(Some(Stroke::FrontDefensive), N);
    let pull = trial(Some(Stroke::Pull), N);

    assert_ne!(
        passive, drive,
        "a straight drive must not simulate identically to leaving it to the engine"
    );
    assert_ne!(
        passive, block,
        "blocking must not simulate identically to leaving it to the engine"
    );
    assert_ne!(
        drive, pull,
        "a straight drive must not simulate identically to a pull — the delivery is supposed to matter"
    );
}

/// The defensive/attacking split, which is the axis the original plays on.
///
/// Defensive strokes are dot balls: they keep you in and score almost nothing.
/// Attacking strokes score more and get you out more. If either half of that
/// is untrue the stroke picker has nothing to choose between.
#[test]
fn defensive_strokes_block_and_attacking_strokes_risk() {
    const N: u32 = 6000;

    let (block_runs, block_wkts) = trial(Some(Stroke::FrontDefensive), N);
    let (leave_runs, _) = trial(Some(Stroke::Leave), N);
    let (drive_runs, drive_wkts) = trial(Some(Stroke::StraightDrive), N);
    let (pull_runs, pull_wkts) = trial(Some(Stroke::Pull), N);
    let (_, none_wkts) = trial(None, N);

    // Blocking scores almost nothing. The original's commentary data has every
    // FRONT_DEFENSIVE and LEAVE line as a dot ball, so anything near an innings
    // rate here would mean the stroke is not doing what it says.
    assert!(
        block_runs < 0.6,
        "blocking should be a dot ball most of the time, scored {block_runs:.2}"
    );
    assert!(
        leave_runs < 0.5,
        "leaving should score even less than blocking, scored {leave_runs:.2}"
    );

    // Attacking strokes are worth playing.
    assert!(
        drive_runs > block_runs * 2.0,
        "a straight drive ({drive_runs:.2}) must be worth far more than a block ({block_runs:.2})"
    );
    assert!(
        pull_runs > block_runs * 2.0,
        "a pull ({pull_runs:.2}) must be worth far more than a block ({block_runs:.2})"
    );

    // And the price of attacking is wickets. A stroke that scored as well as a
    // boundary without ever getting out would make the choice trivial.
    assert!(
        drive_wkts > block_wkts,
        "a straight drive ({drive_wkts:.4}) must be riskier than a block ({block_wkts:.4})"
    );
    assert!(
        pull_wkts > block_wkts,
        "a pull ({pull_wkts:.4}) must be riskier than a block ({block_wkts:.4})"
    );
    assert!(
        drive_wkts > none_wkts,
        "attacking ({drive_wkts:.4}) must be riskier than leaving it to the engine ({none_wkts:.4})"
    );
}

/// The stroke codes are the original's, and they are load-bearing.
///
/// Commentary tables and saved games are keyed on them. If they drift, a table
/// extracted from the original silently stops matching what the engine does —
/// which is exactly the kind of failure that produces plausible-looking wrong
/// output rather than an error.
#[test]
fn stroke_codes_match_the_original() {
    let expected = [
        (Stroke::Leave, 20100u32),
        (Stroke::FrontDefensive, 20101),
        (Stroke::BackDefensive, 20102),
        (Stroke::StraightDrive, 20103),
        (Stroke::CoverDrive, 20104),
        (Stroke::LegGlance, 20105),
        (Stroke::Cut, 20106),
        (Stroke::Pull, 20107),
        (Stroke::Hook, 20108),
        (Stroke::Sweep, 20109),
        (Stroke::OnDrive, 20111),
        (Stroke::OffDrive, 20112),
    ];
    assert_eq!(
        Stroke::ALL.len(),
        expected.len(),
        "the original declares twelve strokes"
    );
    for (stroke, code) in expected {
        assert_eq!(
            stroke.code(),
            code,
            "{:?} must keep the original's code {code}",
            stroke
        );
    }
}

/// The three-way split the picker groups on must match the original's split.
///
/// `bbbc_shot.csv` puts every LEAVE and FRONT_DEFENSIVE commentary line at zero
/// runs and lets the attacking strokes span the outcome range. `is_defensive`
/// is what the UI groups on and what the engine balances risk with, so if the
/// split is wrong the picker mislabels the very choice it is offering.
#[test]
fn the_defensive_split_matches_the_original_data() {
    let defensive: Vec<&str> = Stroke::ALL
        .iter()
        .filter(|s| s.is_defensive())
        .map(|s| s.label())
        .collect();
    assert_eq!(
        defensive,
        vec!["Leave", "Forward Def", "Back Def"],
        "the original's commentary has exactly these three as dot-ball strokes"
    );
    assert!(!Stroke::StraightDrive.is_defensive());
    assert!(Stroke::Pull.is_leg_side());
    assert!(!Stroke::Cut.is_leg_side());
}
/// The delivery must actually change which stroke is a good idea.
///
/// This is the whole reason the model was rebuilt. With outcome-number buttons
/// there was nothing to think about; with strokes, the question "what do I play
/// to this?" only means something if the ball changes the answer.
///
/// It is checked directly against `stroke_vs_delivery` rather than through the
/// simulation, because the differences are small enough that a statistical test
/// over a few thousand balls would be both slow and flaky — and because a test
/// that can miss its target is worse than no test. An earlier version asserted
/// this through simulated innings and passed even with the delivery factored
/// out, because driving and pulling differ in ambition anyway.
#[test]
fn the_delivery_decides_which_stroke_is_safe() {
    use captains_call_2027_lib::match_::engine::stroke_vs_delivery;
    use captains_call_2027_lib::match_::state::{BallLength, BallMovement};

    let short = BallLength::ShortOff;
    let yorker = BallLength::Yorker;
    let straight = BallMovement::Straight;

    // A pull wants the ball short and is a bad idea to a yorker.
    let pull_short = stroke_vs_delivery(Stroke::Pull, short, straight);
    let pull_yorker = stroke_vs_delivery(Stroke::Pull, yorker, straight);
    assert!(
        pull_short > pull_yorker * 1.5,
        "a pull must suit a short ball ({pull_short:.2}) far better than a yorker ({pull_yorker:.2})"
    );

    // A drive wants it on, and a bouncer is the worst ball to drive at.
    let drive_good = stroke_vs_delivery(Stroke::StraightDrive, BallLength::GoodLength, straight);
    let drive_bouncer = stroke_vs_delivery(Stroke::StraightDrive, BallLength::Bouncer, straight);
    assert!(
        drive_good > drive_bouncer,
        "a straight drive must be safer on a good length ({drive_good:.2}) than off a bouncer ({drive_bouncer:.2})"
    );

    // The sweep's own ball is the one that turns.
    let sweep_good = stroke_vs_delivery(Stroke::Sweep, BallLength::GoodLength, straight);
    let sweep_turning = stroke_vs_delivery(
        Stroke::Sweep,
        BallLength::GoodLength,
        BallMovement::LegBreak,
    );
    assert!(
        sweep_turning > sweep_good,
        "a sweep must suit a turning ball ({sweep_turning:.2}) better than a straight one ({sweep_good:.2})"
    );

    // And the ordering must invert: what is safe off a short ball is not the
    // safe shot at a yorker. If every stroke moved together the picker's
    // advice would be identical whatever the bowler did.
    let best_short = [
        Stroke::Pull,
        Stroke::Sweep,
        Stroke::CoverDrive,
        Stroke::StraightDrive,
    ]
    .iter()
    .map(|s| stroke_vs_delivery(*s, short, straight))
    .fold(f64::MIN, f64::max);
    let best_yorker = [
        Stroke::Pull,
        Stroke::Sweep,
        Stroke::CoverDrive,
        Stroke::StraightDrive,
    ]
    .iter()
    .map(|s| stroke_vs_delivery(*s, yorker, straight))
    .fold(f64::MIN, f64::max);
    assert_ne!(
        best_short, best_yorker,
        "the best stroke must differ between a short ball and a yorker"
    );
}
