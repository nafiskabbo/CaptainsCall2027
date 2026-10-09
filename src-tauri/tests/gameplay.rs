//! End-to-end gameplay tests: the same code path the React UI drives.
//!
//! These follow the career flow (start career → play fixture → record result →
//! table updates) against the bundled fictional 2027 world. A green run means
//! the playable build works.

use captains_call_2027_lib::career::{simulate_fixture, Career};
use captains_call_2027_lib::core::scheduler::table_for;

use captains_call_2027_lib::match_::state::MatchPhase;
use captains_call_2027_lib::world::load_world;

/// A domestic team with a full competition schedule.
fn playable_team() -> (u32, u32) {
    let world = load_world();
    let comp = world
        .competitions
        .iter()
        .find(|c| c.team_ids.len() >= 4)
        .or_else(|| world.competitions.iter().find(|c| c.team_ids.len() >= 3))
        .or_else(|| world.competitions.first())
        .expect("expected at least one competition");
    let team_id = comp.team_ids[0];
    (team_id, comp.id)
}

#[test]
fn bundled_world_is_valid() {
    let world = load_world();
    assert!(
        world.db_version.starts_with("2027"),
        "db_version: {}",
        world.db_version
    );
    assert!(
        world.players.len() >= 100,
        "players: {}",
        world.players.len()
    );
    assert!(world.nations.len() >= 6, "nations: {}", world.nations.len());
    assert!(!world.competitions.is_empty(), "competitions missing");
    for t in &world.teams {
        assert!(
            !t.is_national && t.squad_ids.len() >= 11 || t.is_national,
            "team {} has only {} players",
            t.name,
            t.squad_ids.len()
        );
    }
}

#[test]
fn career_starts_with_full_fixture_list() {
    let world = load_world();
    let (team_id, _comp) = playable_team();
    let career = Career::start(&world, team_id, "Test career".into(), 20270101)
        .expect("career should start");

    let comp_size = world
        .competitions
        .iter()
        .find(|c| c.id == career.state.competition_id)
        .unwrap()
        .team_ids
        .len();
    let expected = (comp_size * (comp_size - 1)) / 2;
    assert_eq!(
        career.state.fixtures.len(),
        expected as usize,
        "round robin size"
    );
    assert!(career.state.db_version.starts_with("2027"));
    assert!(!career.state.news.is_empty());

    // The player's team plays every round exactly once.
    let mine = career.player_fixtures();
    assert_eq!(mine.len(), comp_size - 1);
}

#[test]
fn playing_a_fixture_records_the_result() {
    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "Test career".into(), 4242).unwrap();
    let fixture_id = career.next_fixture().expect("a fixture to play").id;

    let snap = career.start_fixture(&world, fixture_id).unwrap();
    assert_eq!(snap.phase, MatchPhase::Innings1);
    assert_eq!(snap.innings.len(), 1);

    // Play it out ball by ball via the career API.
    for _ in 0..3 {
        let s = career.advance(60);
        if s.phase == MatchPhase::Complete {
            break;
        }
    }
    career.advance_innings();
    career.advance_innings();

    let fixture = career
        .state
        .fixtures
        .iter()
        .find(|f| f.id == fixture_id)
        .unwrap();
    assert!(
        fixture.result.is_some(),
        "fixture must be scored once complete"
    );

    // Career news should mention the player's team result.
    assert!(
        career.state.news.iter().any(|n| n.headline.contains("Win!")
            || n.headline.contains("Defeat")
            || n.headline.contains("Tie")),
        "playing your own match should produce career news"
    );

    // The table must reflect the played match.
    let table = career.table(&world);
    let played: u32 = table.iter().map(|r| r.played).sum();
    assert!(played >= 2, "table should count the played match");
}

#[test]
fn ai_fixture_simulation_scores_matches() {
    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "Sim career".into(), 1).unwrap();
    let news = career.advance_day(&world);
    let played = career
        .state
        .fixtures
        .iter()
        .filter(|f| f.result.is_some())
        .count();
    assert!(played > 0, "advancing a day should simulate matches");
    assert!(!news.is_empty() || played > 0);

    // Determinism: same seed + fixture → same result.
    let fixture = career
        .state
        .fixtures
        .iter()
        .find(|f| f.home_id != team_id && f.away_id != team_id)
        .cloned()
        .unwrap_or_else(|| career.state.fixtures[0].clone());
    let a = simulate_fixture(&world, &fixture, 777).unwrap();
    let b = simulate_fixture(&world, &fixture, 777).unwrap();
    assert_eq!(a.home_runs, b.home_runs, "same seed must repeat the match");
    assert_eq!(a.summary, b.summary);
}

#[test]
fn season_completes_when_all_fixtures_played() {
    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "Season".into(), 5).unwrap();
    for _ in 0..200 {
        if career.season_complete() {
            break;
        }
        // Play the player's fixture if it's up.
        if let Some(f) = career.next_fixture().cloned() {
            career.start_fixture(&world, f.id).unwrap();
            career.advance_innings();
            career.advance_innings();
        } else {
            career.advance_day(&world);
        }
    }
    assert!(
        career.season_complete(),
        "season should finish after all fixtures are played"
    );
    let table = career.table(&world);
    let total_played: u32 = table.iter().map(|r| r.played).sum();
    assert_eq!(total_played as usize, career.state.fixtures.len() * 2);
}

#[test]
fn scorecard_export_is_standalone_html() {
    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "Card".into(), 7).unwrap();
    let fixture = career.next_fixture().unwrap().clone();
    career.start_fixture(&world, fixture.id).unwrap();
    career.advance_innings();
    career.advance_innings();
    let html = captains_call_2027_lib::commands::almanack::scorecard_html(
        &career.live_snapshot().unwrap(),
        &world,
    );
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("Bowler"));
    assert!(html.contains("Batter"));
}

#[test]
fn table_sorting_puts_wins_on_top() {
    // Two teams, two matches: team 1 wins both.
    let world = load_world();
    let (team_id, comp_id) = playable_team();
    let mut career = Career::start(&world, team_id, "Sort".into(), 3).unwrap();
    let comp = world.competitions.iter().find(|c| c.id == comp_id).unwrap();
    let (a, b) = (comp.team_ids[0], comp.team_ids[1]);
    for f in career.state.fixtures.iter_mut() {
        if f.home_id == a && f.away_id == b {
            f.result = Some(captains_call_2027_lib::core::scheduler::FixtureResult {
                home_runs: 200,
                away_runs: 100,
                home_wickets: 3,
                away_wickets: 10,
                winner_id: Some(a),
                summary: "A won".into(),
            });
        }
    }
    let table = table_for(&comp.team_ids, &career.state.fixtures);
    assert_eq!(table[0].team_id, a, "winner should top the table");
    assert_eq!(table[0].points, 2);
}

#[test]
fn player_selection_is_validated_and_used() {
    use captains_call_2027_lib::match_::TeamSelection;

    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "XI".into(), 31).unwrap();
    let fixture = career.next_fixture().unwrap().clone();

    let squad: Vec<u32> = world
        .players
        .iter()
        .filter(|p| p.team_id == Some(team_id))
        .take(11)
        .map(|p| p.id)
        .collect();

    // Wrong count is rejected.
    let short = TeamSelection {
        team_id,
        xi_ids: squad[0..10].to_vec(),
        batting_order: squad[0..10].to_vec(),
        keeper_id: None,
    };
    assert!(career.set_selection(&world, fixture.id, short).is_err());

    // A full legal XI is accepted.
    let mut order = squad.clone();
    order.reverse();
    let legal = TeamSelection {
        team_id,
        xi_ids: squad.clone(),
        batting_order: order.clone(),
        keeper_id: None,
    };
    career
        .set_selection(&world, fixture.id, legal)
        .expect("legal XI accepted");
    let stored = career.selection_for(&world, fixture.id).expect("stored");
    assert_eq!(stored.batting_order, order, "player order must be honoured");

    // And it is the XI that takes the field.
    career.start_fixture(&world, fixture.id).unwrap();
    let snap = career.live_snapshot().unwrap();
    let in_xi: Vec<u32> = snap.innings[0]
        .batting_cards
        .iter()
        .map(|c| c.player_id)
        .collect();
    let team_batted_first = snap.innings[0].batting_team_id;
    if team_batted_first == team_id {
        assert_eq!(in_xi, order, "player's batting order must be used");
    }

    // Duplicates are rejected.
    let dupes = TeamSelection {
        team_id,
        xi_ids: vec![
            squad[0], squad[0], squad[1], squad[1], squad[2], squad[2], squad[3], squad[3],
            squad[4], squad[5], squad[6],
        ],
        batting_order: vec![
            squad[0], squad[0], squad[1], squad[1], squad[2], squad[2], squad[3], squad[3],
            squad[4], squad[5], squad[6],
        ],
        keeper_id: None,
    };
    assert!(career.set_selection(&world, fixture.id, dupes).is_err());
}

#[test]
fn selections_survive_a_save_round_trip() {
    use captains_call_2027_lib::db::CareerState;

    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "Persist".into(), 77).unwrap();
    let fixture = career.next_fixture().unwrap().clone();
    let squad: Vec<u32> = world
        .players
        .iter()
        .filter(|p| p.team_id == Some(team_id))
        .take(11)
        .map(|p| p.id)
        .collect();
    career
        .set_selection(
            &world,
            fixture.id,
            captains_call_2027_lib::match_::TeamSelection {
                team_id,
                xi_ids: squad.clone(),
                batting_order: squad.clone(),
                keeper_id: None,
            },
        )
        .unwrap();

    // Serialize as saves do, then deserialize.
    let json = serde_json::to_string(&career.state).unwrap();
    let loaded: CareerState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.selections.len(), 1, "the saved XI must persist");
    assert_eq!(loaded.selections.get(&fixture.id).unwrap().xi_ids, squad);
}

#[test]
fn aggression_changes_outcomes() {
    let world = load_world();
    let (team_id, _comp) = playable_team();

    let mut calm = Career::start(&world, team_id, "calm".into(), 555).unwrap();
    let mut wild = Career::start(&world, team_id, "wild".into(), 555).unwrap();

    let fixture = calm.next_fixture().unwrap().clone();
    calm.start_fixture(&world, fixture.id).unwrap();
    wild.start_fixture(&world, fixture.id).unwrap();
    if let Some(live) = calm.live.as_mut() {
        live.sim.input.batting_aggression = 5;
    }
    if let Some(live) = wild.live.as_mut() {
        live.sim.input.batting_aggression = 95;
    }

    let calm_snap = calm.advance(600);
    let wild_snap = wild.advance(600);
    let calm_score = calm_snap.innings.first().map(|i| i.runs).unwrap_or(0);
    let wild_score = wild_snap.innings.first().map(|i| i.runs).unwrap_or(0);
    assert_ne!(
        calm_score, wild_score,
        "aggression should change the scoreline"
    );
}

#[test]
fn players_develop_through_a_season_of_matches() {
    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "Growth".into(), 606).unwrap();

    // A young player who plays every game should improve.
    let youngest = career
        .players
        .iter()
        .filter(|p| p.team_id == Some(team_id))
        .min_by_key(|p| p.age)
        .cloned()
        .expect("a young player");
    let before = youngest.skills.bat;

    // Force that player into every XI we play.
    for round in 0..career.state.fixtures.len() {
        let fixture = career.state.fixtures[round].clone();
        if fixture.result.is_some() || career.season_complete() {
            continue;
        }
        if fixture.home_id != team_id && fixture.away_id != team_id {
            continue;
        }
        let order: Vec<u32> = {
            let mut ids = world
                .players
                .iter()
                .filter(|p| p.team_id == Some(team_id) && p.id != youngest.id)
                .take(10)
                .map(|p| p.id)
                .collect::<Vec<u32>>();
            ids.insert(0, youngest.id);
            ids
        };
        if order.len() == 11 {
            career
                .set_selection(
                    &world,
                    fixture.id,
                    captains_call_2027_lib::match_::TeamSelection {
                        team_id,
                        xi_ids: order.clone(),
                        batting_order: order,
                        keeper_id: None,
                    },
                )
                .unwrap();
        }
        career.start_fixture(&world, fixture.id).unwrap();
        career.advance_innings();
        career.advance_innings();
        if career.season_complete() {
            break;
        }
        career.advance_day(&world);
    }

    let after = career
        .players
        .iter()
        .find(|p| p.id == youngest.id)
        .expect("player still there");
    assert!(
        after.experience > youngest.experience,
        "playing should build experience"
    );
    // A young batter either gains skill or is already at potential — never above.
    assert!(
        after.skills.bat.0 >= before.0,
        "young players must not regress"
    );
    assert!(after.skills.bat.0 <= after.potential.bat.0);
    // And it must survive a save round-trip.
    let json = serde_json::to_string(&career.state).unwrap();
    let restored: captains_call_2027_lib::db::CareerState = serde_json::from_str(&json).unwrap();
    let restored_player = restored
        .players
        .iter()
        .find(|p| p.id == youngest.id)
        .unwrap();
    assert_eq!(restored_player.experience, after.experience);
    assert_eq!(restored_player.skills.bat, after.skills.bat);
}

#[test]
fn international_career_schedules_cup_and_tours() {
    let world = load_world();
    let nation_team = world
        .teams
        .iter()
        .find(|t| t.is_national)
        .expect("a national side");
    let mut career = Career::start(&world, nation_team.id, "World boss".into(), 31).unwrap();

    let fixture_count = career.state.fixtures.len();
    assert!(
        fixture_count > 3,
        "an international programme should be non-trivial"
    );
    // The whole cup is scheduled, so the tournament resolves around your
    // nation and the other games can be simulated on day advance.
    let mine = career
        .state
        .fixtures
        .iter()
        .filter(|f| f.home_id == nation_team.id || f.away_id == nation_team.id)
        .count();
    assert!(mine >= 3, "your nation should play group games, got {mine}");
    assert!(
        career.state.fixtures.len() > mine,
        "the rest of the cup runs too"
    );
    // And the squad comes from the whole nation.
    let squad: Vec<_> = career
        .players
        .iter()
        .filter(|p| p.nation_id == nation_team.nation_id)
        .collect();
    assert!(squad.len() >= 11, "national pool must field an XI");

    // Play the first fixture involving our nation (the cup also schedules
    // games between other nations, which are simulated on day advance).
    let first = career
        .state
        .fixtures
        .iter()
        .find(|f| f.home_id == nation_team.id || f.away_id == nation_team.id)
        .map(|f| f.id)
        .expect("our nation plays a group game");
    career.start_fixture(&world, first).unwrap();
    career.advance_innings();
    career.advance_innings();
    let played = career
        .state
        .fixtures
        .iter()
        .find(|f| f.id == first)
        .unwrap();
    assert!(played.result.is_some(), "our first game scores");

    // Off-season should keep producing an international programme.
    career.state.season = 1;
    let next = captains_call_2027_lib::core::season::roll_season(&mut career.state, &world, 1);
    assert!(career.state.fixtures.len() > 3, "season 2 has fixtures");
    assert!(next.new_season_fixtures > 3);
    let mine_next = career
        .state
        .fixtures
        .iter()
        .filter(|f| f.home_id == nation_team.id || f.away_id == nation_team.id)
        .count();
    assert!(
        mine_next >= 3,
        "your nation still has group games in season 2"
    );
}

#[test]
fn career_records_accumulate_across_matches() {
    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "Records".into(), 8181).unwrap();

    assert!(career.records.players.is_empty(), "records start empty");

    // Play every fixture available to us (a 3-team league gives two) and
    // confirm stats accumulate.
    let mut played = 0;
    for round in 0..career.state.fixtures.len() {
        if played >= 3 {
            break;
        }
        let fixture = career.state.fixtures[round].clone();
        if fixture.result.is_some() {
            continue;
        }
        if fixture.home_id != team_id && fixture.away_id != team_id {
            continue;
        }
        career.start_fixture(&world, fixture.id).unwrap();
        career.advance_innings();
        career.advance_innings();
        played += 1;
        if career.season_complete() {
            break;
        }
    }
    assert!(
        played >= 2,
        "should have played at least two matches, got {played}"
    );

    // 22 players per match appear in the records.
    let batters: Vec<_> = career
        .records
        .players
        .values()
        .filter(|p| p.matches > 0)
        .collect();
    assert!(
        batters.len() >= 11,
        "records should cover both XIs, got {}",
        batters.len()
    );
    let total_runs: u32 = batters.iter().map(|p| p.runs).sum();
    assert!(total_runs > 0, "runs should accumulate");
    let max = captains_call_2027_lib::core::records::leading_run_scorer(&career.records)
        .expect("a leading scorer");
    assert!(max.runs > 0);
    assert!(max.batting_average().unwrap_or(0.0) > 0.0);

    // Averages and strike rates are well-formed.
    for p in batters {
        assert!(p.strike_rate() >= 0.0 && p.strike_rate() <= 600.0);
    }
}

#[test]
fn training_focus_persists_in_saves() {
    use captains_call_2027_lib::db::TrainingFocus;

    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "Training".into(), 31).unwrap();
    assert_eq!(career.state.training_focus, TrainingFocus::None);

    // Change it, then round-trip through the save payload.
    career.state.training_focus = TrainingFocus::Bowling;
    let json = serde_json::to_string(&career.state).unwrap();
    // The wire format is snake_case, matching the UI contract.
    assert!(
        json.contains("\"training_focus\":\"bowling\""),
        "wire format is snake_case"
    );
    let restored: captains_call_2027_lib::db::CareerState = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.training_focus, TrainingFocus::Bowling);
}

#[test]
fn records_survive_a_career_save_round_trip() {
    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "Records".into(), 12).unwrap();

    let fixture = career.state.fixtures[0].clone();
    if fixture.home_id == team_id || fixture.away_id == team_id {
        career.start_fixture(&world, fixture.id).unwrap();
        career.advance_innings();
        career.advance_innings();
    }

    // Records are mirrored into the save state, so a reload keeps them.
    assert!(
        !career.state.records.players.is_empty() || career.state.fixtures[0].result.is_none(),
        "records should be synced into the save payload"
    );
    let json = serde_json::to_string(&career.state).unwrap();
    let restored: captains_call_2027_lib::db::CareerState = serde_json::from_str(&json).unwrap();
    assert_eq!(
        restored.records.players.len(),
        career.state.records.players.len(),
        "records must not be wiped by a save/load cycle"
    );
}
