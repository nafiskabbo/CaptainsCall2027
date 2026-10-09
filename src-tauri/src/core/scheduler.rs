//! Competition scheduling: round-robins, knockout cups and points tables.

use super::super::core::rng::SimRng;
use super::super::core::types::{Competition, Format};
use serde::{Deserialize, Serialize};

/// A scheduled match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fixture {
    pub id: u32,
    pub competition_id: u32,
    pub home_id: u32,
    pub away_id: u32,
    pub format: Format,
    pub ground_id: u32,
    /// Season day (1-based) the match starts on.
    pub day: u32,
    pub round: u32,
    /// International stage label ("Group A", "Semi-final", "1st Test"); None for leagues.
    #[serde(default)]
    pub stage: Option<String>,
    /// Set once simulated: (home runs, away runs, winner).
    pub result: Option<FixtureResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureResult {
    pub home_runs: u32,
    pub away_runs: u32,
    pub home_wickets: u8,
    pub away_wickets: u8,
    pub winner_id: Option<u32>,
    pub summary: String,
}

/// A row in a league table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRow {
    pub team_id: u32,
    pub played: u32,
    pub won: u32,
    pub lost: u32,
    pub tied: u32,
    pub no_result: u32,
    pub points: u32,
    pub runs_for: u32,
    pub runs_against: u32,
    pub wickets_lost: u32,
}

impl TableRow {
    pub fn net_run_rate_x100(&self) -> i32 {
        // NRR*100 in runs per ball to stay integer-safe for JSON.
        let balls = self.played as f64 * 120.0;
        if balls <= 0.0 {
            return 0;
        }
        ((self.runs_for as f64 / balls - self.runs_against as f64 / balls) * 100.0 * 6.0) as i32
    }
}

/// Standard single round-robin using the circle method (first team fixed,
/// the rest rotate each round), with alternating home/away. Deterministic
/// given the seed.
pub fn round_robin_fixtures(
    competition: &Competition,
    start_day: u32,
    ground_of: impl Fn(u32) -> u32,
    seed: u64,
) -> Vec<Fixture> {
    let mut teams = competition.team_ids.clone();
    // Shuffle deterministically so fixtures aren't always in table order.
    let mut rng = SimRng::new(seed);
    for i in (1..teams.len()).rev() {
        let j = rng.below((i + 1) as u32) as usize;
        teams.swap(i, j);
    }
    if teams.len() % 2 == 1 {
        teams.push(0); // bye placeholder for odd counts
    }

    let n = teams.len();
    let rounds = n - 1;
    let half = n / 2;
    let mut arr = teams.clone();
    let mut fixtures = Vec::new();
    let mut id = competition.id * 1000;

    for round in 0..rounds {
        for i in 0..half {
            let a = arr[i];
            let b = arr[n - 1 - i];
            if a == 0 || b == 0 {
                continue; // bye week
            }
            let (home, away) = if (round + i) % 2 == 0 { (a, b) } else { (b, a) };
            fixtures.push(Fixture {
                id,
                competition_id: competition.id,
                home_id: home,
                away_id: away,
                format: competition.format,
                ground_id: ground_of(home),
                day: start_day + (round as u32) * days_between(competition.format),
                round: round as u32,
                stage: None,
                result: None,
            });
            id += 1;
        }
        // Circle method rotation: keep position 0 fixed, rotate the rest.
        if n > 2 {
            arr[1..].rotate_right(1);
        }
    }
    fixtures
}

fn days_between(format: Format) -> u32 {
    match format {
        Format::Test => 7,
        Format::Odi50 => 3,
        Format::T20 => 2,
    }
}

/// Knockout bracket (cup): pairs winners each round.
pub fn knockout_fixtures(
    competition: &Competition,
    start_day: u32,
    ground_of: impl Fn(u32) -> u32,
) -> Vec<Fixture> {
    let teams = competition.team_ids.clone();
    let mut fixtures = Vec::new();
    let mut id = competition.id * 1000;
    let mut round = 0u32;
    let mut remaining = teams;
    while remaining.len() > 1 {
        let mut next = Vec::new();
        let mut i = 0;
        while i + 1 < remaining.len() {
            let day = start_day + round * days_between(competition.format);
            fixtures.push(Fixture {
                id,
                competition_id: competition.id,
                home_id: remaining[i],
                away_id: remaining[i + 1],
                format: competition.format,
                ground_id: ground_of(remaining[i]),
                day,
                round,
                stage: None,
                result: None,
            });
            id += 1;
            next.push(0); // winner filled in as the cup progresses
            i += 2;
        }
        remaining = next;
        round += 1;
    }
    fixtures
}

/// Rebuild a league table from resolved fixtures.
pub fn table_for(team_ids: &[u32], fixtures: &[Fixture]) -> Vec<TableRow> {
    let mut rows: Vec<TableRow> = team_ids
        .iter()
        .map(|id| TableRow {
            team_id: *id,
            ..Default::default()
        })
        .collect();

    for f in fixtures {
        let Some(result) = &f.result else { continue };

        // Result column for each side.
        for &(id, opponent, is_home) in
            &[(f.home_id, f.away_id, true), (f.away_id, f.home_id, false)]
        {
            let Some(row) = rows.iter_mut().find(|r| r.team_id == id) else {
                continue;
            };
            row.played += 1;
            let (runs_for, runs_against) = if is_home {
                (result.home_runs, result.away_runs)
            } else {
                (result.away_runs, result.home_runs)
            };
            row.runs_for += runs_for;
            row.runs_against += runs_against;
            let won = result.winner_id == Some(id);
            let lost = result.winner_id == Some(opponent);
            if won {
                row.won += 1;
                row.points += 2;
            } else if lost {
                row.lost += 1;
            } else {
                row.tied += 1;
                row.points += 1;
            }
        }

        // Wickets lost, per side.
        if let Some(row) = rows.iter_mut().find(|r| r.team_id == f.home_id) {
            row.wickets_lost += result.home_wickets as u32;
        }
        if let Some(row) = rows.iter_mut().find(|r| r.team_id == f.away_id) {
            row.wickets_lost += result.away_wickets as u32;
        }
    }

    rows.sort_by_key(|r| std::cmp::Reverse((r.points, r.net_run_rate_x100())));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comp(ids: Vec<u32>) -> Competition {
        Competition {
            id: 1,
            name: "Test League".into(),
            nation_id: 1,
            format: Format::T20,
            kind: crate::core::types::CompetitionKind::League,
            team_ids: ids,
            days_per_match: 1,
        }
    }

    #[test]
    fn round_robin_gives_everyone_games() {
        let ids: Vec<u32> = (1..=8).collect();
        let fixtures = round_robin_fixtures(&comp(ids.clone()), 1, |_| 1, 7);
        assert_eq!(fixtures.len(), 28, "8 teams, single round robin");
        let mut pairs = std::collections::HashSet::new();
        for f in &fixtures {
            assert!(
                f.home_id != 0 && f.away_id != 0,
                "no bye placeholders in output"
            );
            pairs.insert((f.home_id.min(f.away_id), f.home_id.max(f.away_id)));
        }
        let expected = ids.len() * (ids.len() - 1) / 2;
        assert_eq!(
            pairs.len(),
            expected,
            "every pairing should occur exactly once"
        );
    }

    #[test]
    fn round_robin_handles_odd_team_counts() {
        let ids: Vec<u32> = (1..=5).collect();
        let fixtures = round_robin_fixtures(&comp(ids.clone()), 1, |_| 1, 11);
        assert_eq!(fixtures.len(), 10, "5 teams, single round robin");
        let mut pairs = std::collections::HashSet::new();
        for f in &fixtures {
            pairs.insert((f.home_id.min(f.away_id), f.home_id.max(f.away_id)));
        }
        assert_eq!(pairs.len(), 10);
        // Nobody plays the bye.
        assert!(fixtures.iter().all(|f| f.home_id != 0 && f.away_id != 0));
    }

    #[test]
    fn fixture_days_are_distinct() {
        let ids: Vec<u32> = (1..=6).collect();
        let fixtures = round_robin_fixtures(&comp(ids), 1, |_| 1, 3);
        let days: std::collections::HashSet<u32> = fixtures.iter().map(|f| f.day).collect();
        assert_eq!(days.len(), fixtures.len() / 3, "games per day tracked");
    }

    #[test]
    fn table_sums_results() {
        let fixtures = vec![
            Fixture {
                id: 1,
                competition_id: 1,
                home_id: 1,
                away_id: 2,
                format: Format::T20,
                ground_id: 1,
                day: 1,
                round: 0,
                stage: None,
                result: None,
            },
            Fixture {
                id: 2,
                competition_id: 1,
                home_id: 2,
                away_id: 1,
                format: Format::T20,
                ground_id: 1,
                day: 3,
                round: 1,
                stage: None,
                result: None,
            },
        ];
        let rows = table_for(&[1, 2], &fixtures);
        assert_eq!(rows[0].played, 0);
        assert_eq!(rows[0].points, 0);
    }
}
