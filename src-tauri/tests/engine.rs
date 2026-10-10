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
    use captains_call_2027_lib::match_::state::ShotKind;

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

/// The shot-choice mechanic: committing to an outcome must raise the chance of
/// getting it, and raise the chance of being dismissed in proportion to the
/// risk taken.
///
/// This is the Cricket Coach batting model — you pick an outcome icon and the
/// engine resolves the attempt. Without it the "shot" controls are decorative:
/// the batter never actually chooses anything.
#[test]
fn committing_to_a_shot_raises_its_chance_and_the_risk() {
    use captains_call_2027_lib::core::rng::SimRng;
    use captains_call_2027_lib::match_::engine::{resolve_ball, BallContext};
    use captains_call_2027_lib::match_::state::{Innings, MatchInput, ShotChoice};

    fn trial(choice: Option<ShotChoice>, n: u32) -> (f64, f64) {
        let batter = Player {
            id: 1,
            name: "Batter".into(),
            nation_id: 1,
            team_id: None,
            age: 25,
            role: Role::Batter,
            batting_style: "rhb".into(),
            bowling_style: None,
            skills: Skills {
                bat: Rating(80),
                test_bat: Rating(80),
                odi_bat: Rating(80),
                t20_bat: Rating(80),
                defence: Rating(70),
                pace: Rating(1),
                spin: Rating(1),
                odi_bowl: Rating(1),
                t20_bowl: Rating(1),
                fielding: Rating(60),
                keeping: Rating(1),
                leadership: Rating(50),
            },
            ..Default::default()
        };
        let bowler = Player {
            id: 2,
            name: "Bowler".into(),
            nation_id: 1,
            team_id: None,
            age: 27,
            role: Role::PaceBowler,
            batting_style: "lhb".into(),
            bowling_style: Some("right-arm fast".into()),
            skills: Skills {
                bat: Rating(20),
                test_bat: Rating(20),
                odi_bat: Rating(20),
                t20_bat: Rating(20),
                defence: Rating(20),
                pace: Rating(78),
                spin: Rating(1),
                odi_bowl: Rating(78),
                t20_bowl: Rating(78),
                fielding: Rating(60),
                keeping: Rating(1),
                leadership: Rating(40),
            },
            ..Default::default()
        };
        let fielding = Team {
            id: 2,
            name: "Fielding".into(),
            nation_id: 1,
            home_ground_id: 1,
            budget: 0,
            reputation: Rating(50),
            squad_ids: vec![],
            is_national: false,
        };

        let mut rng = SimRng::new(20270101);
        let input = MatchInput {
            shot_choice: choice,
            ..Default::default()
        };
        let mut runs = 0u32;
        let mut wickets = 0u32;
        for ball in 0..n {
            let mut innings = Innings {
                batting_team_id: 1,
                bowling_team_id: 2,
                runs,
                wickets: wickets.min(9) as u8,
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
                powerplay: ball < 36,
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

    const N: u32 = 4000;

    let (none_runs, none_wkts) = trial(None, N);
    let (dot_runs, dot_wkts) = trial(Some(ShotChoice::Dot), N);
    let (one_runs, _) = trial(Some(ShotChoice::One), N);
    let (two_runs, _) = trial(Some(ShotChoice::Two), N);
    let (four_runs, four_wkts) = trial(Some(ShotChoice::Four), N);
    let (six_runs, six_wkts) = trial(Some(ShotChoice::Six), N);

    /*
        The engine's own numbers for this matchup, and the properties that make
        it a decision rather than a slot machine:

          choice   runs/ball   wickets/ball
          none        1.20          0.050
          dot         0.40          0.035
          one         1.02          0.059
          two         1.41          0.146
          four        2.16          0.219
          six         1.78          0.411

        Note `one` scores *below* leaving the ball alone. That is deliberate and
        correct: always playing for a single gives up the boundaries that make
        an innings, which is exactly why real batters take them. An earlier
        pass made singles beat the baseline, which rewarded a strategy no
        cricketer plays.
    */

    // Blocking is safe and scores nothing.
    assert!(
        dot_wkts < none_wkts,
        "blocking ({dot_wkts:.4}) must be safer than playing normally ({none_wkts:.4})"
    );
    assert!(
        dot_runs < none_runs,
        "blocking ({dot_runs:.2}) should score less than playing normally ({none_runs:.2})"
    );

    // Attacking for runs is worth it, and the risk rises with the ambition.
    assert!(
        four_runs > none_runs,
        "going for four ({four_runs:.2}) must beat playing normally ({none_runs:.2})"
    );
    assert!(
        six_runs > none_runs,
        "going for six ({six_runs:.2}) must beat playing normally ({none_runs:.2})"
    );
    assert!(
        two_runs > one_runs,
        "going for two ({two_runs:.2}) should beat rotating ({one_runs:.2})"
    );

    // And the price of ambition is wickets, monotonically in the risk taken.
    assert!(
        six_wkts > four_wkts,
        "a six ({six_wkts:.4}) must be riskier than a four ({four_wkts:.4})"
    );
    assert!(
        four_wkts > dot_wkts,
        "a four ({four_wkts:.4}) must be riskier than a block ({dot_wkts:.4})"
    );
    assert!(
        six_wkts > none_wkts,
        "a six ({six_wkts:.4}) must be riskier than playing normally ({none_wkts:.4})"
    );

    /*
        The ordering assertions above all pass on a mechanic that is merely
        *ordered* — clamp the wicket share hard enough and six is still riskier
        than four, because six's larger failure mass carries it. That is a
        mechanic where going for six is nearly free, and it slipped through
        while every comparison above stayed green.

        So assert the absolute cost too. A six must be a real gamble: several
        times likelier to end your innings than a ball left to the engine.
        Here it is 8x (0.41 against 0.05); the clamped version measured 3x.
    */
    assert!(
        six_wkts > none_wkts * 5.0,
        "a six must cost several times the dismissal risk of a normal ball, \
         got {six_wkts:.4} against {none_wkts:.4} ({:.1}x)",
        six_wkts / none_wkts
    );
    assert!(
        four_wkts > none_wkts * 2.0,
        "a four must be a genuine trade too, got {four_wkts:.4} against {none_wkts:.4} ({:.1}x)",
        four_wkts / none_wkts
    );
}
