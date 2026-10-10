//! Integration tests: determinism, invariants, and scoring sanity.
//!
//! These are the first slice of the M1.8 regression corpus.

use captains_call_2027_lib::core::rng::SimRng;
use captains_call_2027_lib::core::types::{
    Format, Ground, Nation, Player, Rating, Role, Skills, Team, World,
};
use captains_call_2027_lib::match_::{MatchConfig, SimState, TeamSelection};

fn player(id: u32, name: &str, bat: u8, bowl: u8, spin: bool, team_id: u32) -> Player {
    let mut skills = Skills {
        bat: Rating(bat),
        test_bat: Rating(bat),
        odi_bat: Rating(bat),
        t20_bat: Rating(bat),
        defence: Rating(bat),
        fielding: Rating(bat),
        ..Default::default()
    };
    if spin {
        skills.spin = Rating(bowl);
    } else {
        skills.pace = Rating(bowl);
    }
    skills.odi_bowl = Rating(bowl);
    skills.t20_bowl = Rating(bowl);
    Player {
        id,
        name: name.into(),
        nation_id: 1,
        team_id: Some(team_id),
        age: 27,
        role: if bowl > bat {
            Role::PaceBowler
        } else {
            Role::Batter
        },
        batting_style: "rhb".into(),
        bowling_style: None,
        skills,
        potential: skills,
        form: 0,
        fitness: 100,
        morale: 70,
        experience: 50,
        injury_days: 0,
    }
}

fn team(id: u32, name: &str, rep: u8) -> Team {
    Team {
        id,
        name: name.into(),
        nation_id: 1,
        home_ground_id: 1,
        budget: 1_000_000,
        reputation: Rating(rep),
        squad_ids: (1..=11).collect(),
        is_national: false,
    }
}

fn squad(team_id: u32, base: u8, offset: u32) -> Vec<Player> {
    (0..11)
        .map(|i| {
            let bat = base as i32 - (i as i32 / 2) + 5;
            let bowl = (base as i32 - (i as i32 / 2)).max(1);
            player(
                offset + i,
                &format!("Batter {} T{}", i, team_id),
                bat.max(10) as u8,
                bowl as u8,
                i % 2 == 0,
                team_id,
            )
        })
        .collect()
}

fn build_world() -> World {
    let players: Vec<Player> = squad(1, 85, 100)
        .into_iter()
        .chain(squad(2, 70, 200))
        .collect();
    World {
        schema_version: 1,
        db_version: "test".into(),
        nations: vec![Nation {
            id: 1,
            name: "Testland".into(),
            short_name: "TST".into(),
            strength: 80,
            has_test_status: true,
        }],
        teams: vec![team(1, "Test Warriors", 92), team(2, "Under Dogs", 60)],
        players,
        grounds: vec![Ground {
            id: 1,
            name: "Test Oval".into(),
            nation_id: 1,
            boundary_m: 70,
            batting_friendly: 55,
            day_night: false,
        }],
        competitions: vec![],
    }
}

fn sim(seed: u64) -> SimState {
    let world = build_world();
    let mut home_sel = TeamSelection::auto_for(&world.teams[0], &world.players, Format::T20);
    home_sel.batting_order = home_sel.batting_order.clone();
    let away_sel = TeamSelection::auto_for(&world.teams[1], &world.players, Format::T20);
    let cfg = MatchConfig {
        match_id: 1,
        home_team: world.teams[0].clone(),
        away_team: world.teams[1].clone(),
        format: Format::T20,
        ground_id: 1,
        days: 1,
    };
    let mut state = SimState::new(cfg, seed, home_sel, away_sel);
    state.attach_players(world.players.clone());
    state.start();
    state
}

#[test]
fn rng_is_deterministic() {
    let mut a = SimRng::new(1234);
    let mut b = SimRng::new(1234);
    for _ in 0..1000 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
}

#[test]
fn same_seed_produces_same_score() {
    let mut a = sim(77);
    let mut b = sim(77);
    a.bowl_many(240);
    b.bowl_many(240);
    let (sa, sb) = (a.snapshot(), b.snapshot());
    assert_eq!(sa.innings.len(), sb.innings.len());
    for (ia, ib) in sa.innings.iter().zip(sb.innings.iter()) {
        assert_eq!(ia.runs, ib.runs, "innings runs must match for same seed");
        assert_eq!(ia.wickets, ib.wickets);
    }
}

#[test]
fn t20_match_completes_within_240_balls() {
    let mut s = sim(9);
    s.bowl_many(600);
    let snap = s.snapshot();
    assert_eq!(
        snap.phase,
        captains_call_2027_lib::match_::state::MatchPhase::Complete
    );
    let total_balls: u32 = snap.innings.iter().map(|i| i.balls).sum();
    assert!(total_balls <= 240, "total legal balls {total_balls}");
}

#[test]
fn t20_scoring_lands_in_realistic_range() {
    let mut scores = Vec::new();
    for seed in 0..12 {
        let mut s = sim(seed);
        s.bowl_many(600);
        let snap = s.snapshot();
        let first = snap.innings.first().unwrap().runs;
        scores.push(first);
    }
    let avg = scores.iter().sum::<u32>() as f64 / scores.len() as f64;
    assert!(
        (100.0..=280.0).contains(&avg),
        "average T20 score {avg} out of expected band, samples: {scores:?}"
    );
}

#[test]
fn odi_scoring_lands_in_realistic_range() {
    let world = build_world();
    let cfg = |_seed: u64| MatchConfig {
        match_id: 2,
        home_team: world.teams[0].clone(),
        away_team: world.teams[1].clone(),
        format: Format::Odi50,
        ground_id: 1,
        days: 1,
    };
    let mut scores = Vec::new();
    for seed in 0..8u64 {
        let home = TeamSelection::auto_for(&world.teams[0], &world.players, Format::Odi50);
        let away = TeamSelection::auto_for(&world.teams[1], &world.players, Format::Odi50);
        let mut s = SimState::new(cfg(seed), seed, home, away);
        s.attach_players(world.players.clone());
        s.start();
        s.bowl_many(1500);
        scores.push(s.snapshot().innings.first().unwrap().runs);
    }
    let avg = scores.iter().sum::<u32>() as f64 / scores.len() as f64;
    assert!(
        (140.0..=420.0).contains(&avg),
        "average ODI score {avg} out of band, samples {scores:?}"
    );
}

/// Every delivery must report where the ball went and how it was played.
///
/// The 2D view animates from these fields. Without them the view had nothing
/// but a timer to animate and looped the same run-up on every ball, which is
/// exactly the "same animation always" symptom.
#[test]
fn every_delivery_reports_a_direction_and_shot_kind() {
    use captains_call_2027_lib::core::types::{Format, Player, Role, Team};
    use captains_call_2027_lib::match_::state::{ShotKind, WinMargin};

    // A pair of sides with known players, so resolve_ball has someone to use.
    let mk_player = |id: u32, bat: u8, bowl: u8| Player {
        id,
        name: format!("P{id}"),
        nation_id: 1,
        team_id: None,
        age: 25,
        role: if bowl > bat {
            Role::PaceBowler
        } else {
            Role::Batter
        },
        batting_style: "rhb".into(),
        bowling_style: None,
        skills: captains_call_2027_lib::core::types::Skills {
            bat: captains_call_2027_lib::core::types::Rating(bat),
            test_bat: captains_call_2027_lib::core::types::Rating(bat),
            odi_bat: captains_call_2027_lib::core::types::Rating(bat),
            t20_bat: captains_call_2027_lib::core::types::Rating(bat),
            defence: captains_call_2027_lib::core::types::Rating(60),
            pace: captains_call_2027_lib::core::types::Rating(bowl),
            spin: captains_call_2027_lib::core::types::Rating(1),
            odi_bowl: captains_call_2027_lib::core::types::Rating(bowl),
            t20_bowl: captains_call_2027_lib::core::types::Rating(bowl),
            fielding: captains_call_2027_lib::core::types::Rating(60),
            keeping: captains_call_2027_lib::core::types::Rating(1),
            leadership: captains_call_2027_lib::core::types::Rating(50),
        },
        ..Default::default()
    };

    let team = |id: u32| Team {
        id,
        name: format!("T{id}"),
        nation_id: 1,
        home_ground_id: 1,
        budget: 0,
        reputation: captains_call_2027_lib::core::types::Rating(50),
        squad_ids: vec![],
        is_national: false,
    };

    let players = [mk_player(1, 80, 70), mk_player(2, 70, 80)];
    let mut rng = captains_call_2027_lib::core::rng::SimRng::new(42);
    let mut innings = captains_call_2027_lib::match_::state::Innings {
        batting_team_id: 1,
        bowling_team_id: 2,
        ..Default::default()
    };
    let input = captains_call_2027_lib::match_::state::MatchInput::default();

    let mut seen_directions = std::collections::HashSet::new();
    let mut seen_shots = std::collections::HashSet::new();
    let _ = (Format::T20, WinMargin::Draw);

    for ball in 0..120u32 {
        let mut ctx = captains_call_2027_lib::match_::engine::BallContext {
            striker: &players[0],
            batter_id: players[0].id,
            bowler: &players[1],
            fielding_team: &team(2),
            innings: &mut innings,
            format: Format::T20,
            over: ball / 6 + 1,
            powerplay: ball < 36,
            chasing: false,
            run_rate_needed: 0.0,
            minnow_boost: 1.0,
            input: &input,
            rng: &mut rng,
        };
        let outcome = captains_call_2027_lib::match_::engine::resolve_ball(&mut ctx);
        assert!(
            outcome.direction_deg < 360,
            "ball {ball}: direction {} out of range",
            outcome.direction_deg
        );
        assert!(
            matches!(
                outcome.shot,
                ShotKind::Defended
                    | ShotKind::Driven
                    | ShotKind::Cut
                    | ShotKind::Pulled
                    | ShotKind::Lofted
                    | ShotKind::Edged
                    | ShotKind::Beaten
                    | ShotKind::Missed
            ),
            "ball {ball}: unrecognised shot kind {:?}",
            outcome.shot
        );
        seen_directions.insert(outcome.direction_deg);
        seen_shots.insert(outcome.shot);

        // Every ball advances the innings so the engine sees a realistic game.
        innings.balls += 1;
        innings.runs += outcome.total();
        if outcome.dismissal.is_some() {
            innings.wickets += 1;
        }
    }

    // A fixed animation would mean one direction for every ball. Real cricket
    // is not like that, and neither is the engine now.
    assert!(
        seen_directions.len() >= 20,
        "only {} distinct directions over 120 balls — the view would look fixed",
        seen_directions.len()
    );
    assert!(
        seen_shots.len() >= 3,
        "only {} distinct shot kinds — the view would look fixed",
        seen_shots.len()
    );
}
