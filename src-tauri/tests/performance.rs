//! Performance benchmarks against the specs' NFR-1/2/3 targets.
//!
//! These are intentionally generous assertions, not a benchmark suite: the goal
//! is to catch a 10x regression, not to measure nanoseconds. The numbers below
//! are the release gates recorded in tasks.plan.md (P7.1).

use captains_call_2027_lib::career::Career;
use captains_call_2027_lib::core::types::Format;
use captains_call_2027_lib::world::load_world;
use std::time::Instant;

fn playable_team() -> (u32, u32) {
    let world = load_world();
    let comp = world
        .competitions
        .iter()
        .find(|c| c.team_ids.len() >= 4)
        .or_else(|| world.competitions.iter().find(|c| c.team_ids.len() >= 3))
        .or_else(|| world.competitions.first())
        .expect("a competition");
    (comp.team_ids[0], comp.id)
}

#[test]
fn world_loads_quickly() {
    let start = Instant::now();
    let world = load_world();
    let elapsed = start.elapsed();
    assert!(world.players.len() >= 100);
    // Embedded JSON parse; the target is well under 50ms.
    assert!(
        elapsed.as_millis() < 50,
        "world load took {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn full_season_simulates_inside_the_time_budget() {
    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "Bench".into(), 4242).unwrap();

    let start = Instant::now();
    // Play every player fixture and fast-forward everything else.
    for _ in 0..200 {
        if career.season_complete() {
            break;
        }
        if let Some(f) = career.next_fixture().cloned() {
            career.start_fixture(&world, f.id).unwrap();
            career.advance_innings();
            career.advance_innings();
        } else {
            career.advance_day(&world);
        }
    }
    let elapsed = start.elapsed();
    assert!(career.season_complete(), "season should finish");
    // NFR-1: a domestic holiday sim should stay interactive (< 5s target).
    assert!(
        elapsed.as_secs() < 5,
        "full season took {:.2}s",
        elapsed.as_secs_f32()
    );
}

#[test]
fn single_match_advances_quickly() {
    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "Speed".into(), 7).unwrap();
    let fixture = career.next_fixture().unwrap().clone();
    career.start_fixture(&world, fixture.id).unwrap();

    let start = Instant::now();
    // Play out a whole innings one ball at a time.
    for _ in 0..600 {
        if career.live_snapshot().map(|s| s.phase)
            == Some(captains_call_2027_lib::match_::state::MatchPhase::Complete)
        {
            break;
        }
        career.advance(1);
    }
    let elapsed = start.elapsed();
    // Ball-by-ball play should stay responsive (< 2s for an innings).
    assert!(
        elapsed.as_secs() < 2,
        "ball-by-ball innings took {:.2}s",
        elapsed.as_secs_f32()
    );
}

#[test]
fn db_validation_runs_quickly() {
    // The validator is a JS tool; here we check the Rust-side load+parse path
    // stays cheap so the same target holds on the CLI validator.
    let start = Instant::now();
    let world = load_world();
    let _ = world.teams.len();
    assert!(start.elapsed().as_millis() < 100);
}

#[test]
fn career_state_serializes_quickly() {
    let world = load_world();
    let (team_id, _comp) = playable_team();
    let mut career = Career::start(&world, team_id, "Save speed".into(), 1).unwrap();
    if let Some(f) = career.next_fixture().cloned() {
        career.start_fixture(&world, f.id).unwrap();
        career.advance_innings();
        career.advance_innings();
    }
    let start = Instant::now();
    let json = serde_json::to_string(&career.state).expect("serialize");
    let elapsed = start.elapsed();
    assert!(json.len() > 1000, "save payload has content");
    assert!(
        elapsed.as_millis() < 50,
        "save serialization took {}ms",
        elapsed.as_millis()
    );
    let _ = Format::T20;
}
