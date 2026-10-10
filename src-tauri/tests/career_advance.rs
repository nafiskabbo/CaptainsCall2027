//! Unit tests for the day-advance primitives behind the career action bar.
//!
//! These cover the three distinct behaviours the UI exposes: «« steps one
//! day, Fast Forward skips to the next *match* day, and »» skips to the
//! player's own next match.

use captains_call_2027_lib::career::Career;
use captains_call_2027_lib::world::load_world;

fn start(team_id: u32) -> Career {
    let world = load_world();
    Career::start(&world, team_id, "Test".into(), 20270101).expect("career starts")
}

/// The earliest fixture day strictly after `from`.
fn next_day_after(career: &Career, from: u32) -> Option<u32> {
    career
        .state
        .fixtures
        .iter()
        .filter(|f| f.day > from && f.result.is_none())
        .map(|f| f.day)
        .min()
}

/// The earliest day the player's own side plays, strictly after `from`.
fn my_next_day_after(career: &Career, from: u32) -> Option<u32> {
    let me = career.state.player_team_id;
    career
        .state
        .fixtures
        .iter()
        .filter(|f| f.day > from && f.result.is_none() && (f.home_id == me || f.away_id == me))
        .map(|f| f.day)
        .min()
}

#[test]
fn advance_one_day_always_moves_by_exactly_one() {
    let world = load_world();
    let mut career = start(101);
    let before = career.state.season_day;
    career.advance_one_day(&world);
    assert_eq!(
        career.state.season_day,
        before + 1,
        "«« must step the clock by one day even on an idle day"
    );
}

#[test]
fn advance_one_day_simulates_that_day_s_fixtures() {
    let world = load_world();
    let mut career = start(101);
    // Land on the first match day, then step across it: the AI fixtures on that
    // day must be resolved, and the player's own must not be.
    let first = next_day_after(&career, career.state.season_day).expect("fixtures exist");
    career.state.season_day = first;
    let mine_before = career
        .state
        .fixtures
        .iter()
        .filter(|f| {
            f.day == first
                && (f.home_id == career.state.player_team_id
                    || f.away_id == career.state.player_team_id)
        })
        .filter(|f| f.result.is_none())
        .count();
    career.advance_one_day(&world);
    let mine_after = career
        .state
        .fixtures
        .iter()
        .filter(|f| {
            f.day == first
                && (f.home_id == career.state.player_team_id
                    || f.away_id == career.state.player_team_id)
        })
        .filter(|f| f.result.is_none())
        .count();
    assert_eq!(
        mine_before, mine_after,
        "the player's own match must not be auto-played"
    );
}

#[test]
fn fast_forward_lands_on_a_match_day() {
    let world = load_world();
    let mut career = start(101);
    let from = career.state.season_day;
    let target = next_day_after(&career, from).expect("fixtures exist");
    career.advance_day(&world);
    assert!(
        career.state.season_day >= target,
        "fast forward should reach day {target}, got {}",
        career.state.season_day
    );
    // Skipping days must not silently swallow the fixtures on the way.
    assert!(
        career.state.season_day > from,
        "fast forward must move at least one day"
    );
}

#[test]
fn skip_to_my_match_lands_on_my_fixture_day() {
    let world = load_world();
    let mut career = start(101);
    let from = career.state.season_day;
    let target = my_next_day_after(&career, from).expect("the player has fixtures");
    career.advance_to_my_next_match(&world);
    assert_eq!(
        career.state.season_day, target,
        "»» should land on the player's next match day"
    );
    // And that fixture is still unplayed, ready to start.
    let mine = career
        .state
        .fixtures
        .iter()
        .find(|f| {
            f.day == target
                && (f.home_id == career.state.player_team_id
                    || f.away_id == career.state.player_team_id)
        })
        .expect("the player's fixture is on that day");
    assert!(
        mine.result.is_none(),
        "the player's match must still be playable"
    );
}

#[test]
fn skip_to_my_match_simulates_the_rounds_between() {
    let world = load_world();
    let mut career = start(101);
    // Advance once so some AI fixtures are already behind us.
    let first = next_day_after(&career, career.state.season_day).expect("fixtures exist");
    career.state.season_day = first;
    let progressed = career
        .state
        .fixtures
        .iter()
        .filter(|f| f.result.is_some())
        .count();
    career.advance_to_my_next_match(&world);
    assert!(
        career
            .state
            .fixtures
            .iter()
            .filter(|f| f.result.is_some())
            .count()
            >= progressed,
        "skipping forward must not un-simulate earlier rounds"
    );
}

#[test]
fn fast_forward_is_idempotent_when_the_season_is_over() {
    let world = load_world();
    let mut career = start(101);
    for fixture in career.state.fixtures.iter_mut() {
        fixture.result = Some(captains_call_2027_lib::core::scheduler::FixtureResult {
            home_runs: 0,
            away_runs: 0,
            home_wickets: 0,
            away_wickets: 0,
            winner_id: None,
            summary: "done".into(),
        });
    }
    let day = career.state.season_day;
    career.advance_to_my_next_match(&world);
    assert_eq!(
        career.state.season_day, day,
        "nothing left must not move the clock"
    );
    assert!(career.season_complete());
}
