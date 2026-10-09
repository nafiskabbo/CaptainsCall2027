//! International fixtures: World Cup groups, knockouts and bilateral series.
//!
//! National-team careers run on these instead of domestic round-robins —
//! a World Challenge cup followed by tours against other nations, the way the
//! original's international career works (specs FR-8, FR-12).

use super::super::core::rng::SimRng;
use super::super::core::scheduler::Fixture;
use super::super::core::types::Format;
use serde::{Deserialize, Serialize};

/// Which stage of an international programme a fixture belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    /// Group stage of a cup; the label is the group name.
    Group(String),
    SemiFinal,
    Final,
    /// A bilateral series; label is "1st Test", "2nd T20", etc.
    Series(String),
}

impl Stage {
    pub fn label(&self) -> String {
        match self {
            Stage::Group(name) => format!("Group {}", name),
            Stage::SemiFinal => "Semi-final".into(),
            Stage::Final => "Final".into(),
            Stage::Series(name) => name.clone(),
        }
    }
}

/// A fixture with its tournament stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternationalFixture {
    pub fixture: Fixture,
    pub stage: Stage,
}

/// Build a 12-team World Cup: four groups of three, top two advance to
/// semi-finals, then the final.
pub fn world_cup_fixtures(
    team_ids: &[u32],
    format: Format,
    ground_of: impl Fn(u32) -> u32,
    start_day: u32,
    seed: u64,
) -> Vec<InternationalFixture> {
    let mut teams = team_ids.to_vec();
    let mut rng = SimRng::new(seed);
    for i in (1..teams.len()).rev() {
        let j = rng.below((i + 1) as u32) as usize;
        teams.swap(i, j);
    }

    let mut out = Vec::new();
    let mut id = 900_000u32;
    let group_names = ["A", "B", "C", "D"];
    let groups: Vec<Vec<u32>> = teams.chunks(3).map(|c| c.to_vec()).collect();

    // Group stage: round robin inside each group of 3.
    let mut day = start_day;
    for (gi, group) in groups.iter().enumerate() {
        let label = group_names.get(gi).copied().unwrap_or("Z");
        for i in 0..group.len() {
            for j in (i + 1)..group.len() {
                let (home, away) = if (i + j) % 2 == 0 {
                    (group[i], group[j])
                } else {
                    (group[j], group[i])
                };
                out.push(InternationalFixture {
                    fixture: Fixture {
                        id,
                        competition_id: 90_000 + gi as u32,
                        home_id: home,
                        away_id: away,
                        format,
                        ground_id: ground_of(home),
                        day,
                        round: 0,
                        stage: None,
                        result: None,
                    },
                    stage: Stage::Group(label.into()),
                });
                id += 1;
                day += 2;
            }
        }
    }

    // Knockout rounds use placeholder slots: 0 = winner of group A, etc.
    // The engine fills these in as the group stage resolves.
    let semis = [(Stage::Group("A".into()), 0), (Stage::Group("B".into()), 1)];
    let _ = semis;

    let mut knockout_day = day + 4;
    // Two semi-finals (placeholders resolved by the UI/engine when known).
    for i in 0..2u32 {
        out.push(InternationalFixture {
            fixture: Fixture {
                id,
                competition_id: 90_100,
                home_id: 0,
                away_id: 0,
                format,
                ground_id: 1,
                day: knockout_day,
                round: 1,
                stage: None,
                result: None,
            },
            stage: Stage::SemiFinal,
        });
        id += 1;
        knockout_day += 2;
        let _ = i;
    }
    out.push(InternationalFixture {
        fixture: Fixture {
            id,
            competition_id: 90_101,
            home_id: 0,
            away_id: 0,
            format,
            ground_id: 1,
            day: knockout_day + 2,
            round: 2,
            stage: None,
            result: None,
        },
        stage: Stage::Final,
    });

    out
}

/// Resolve a knockout fixture once its qualifiers are known.
///
/// `qualifiers` are the team ids that advance, in order.
pub fn resolve_knockouts(fixtures: &mut [InternationalFixture], qualifiers: &[(usize, u32, u32)]) {
    for (fixture_index, home, away) in qualifiers {
        if let Some(entry) = fixtures.get_mut(*fixture_index) {
            entry.fixture.home_id = *home;
            entry.fixture.away_id = *away;
        }
    }
}

/// A bilateral tour: `matches` games against one opponent.
#[allow(clippy::too_many_arguments)]
pub fn series_fixtures(
    home_nation: u32,
    away_nation: u32,
    format: Format,
    matches: u32,
    ground_of: impl Fn(u32) -> u32,
    start_day: u32,
    competition_id: u32,
    first_id: u32,
) -> Vec<InternationalFixture> {
    let mut out = Vec::new();
    let labels = match format {
        Format::Test => ["1st Test", "2nd Test", "3rd Test", "4th Test", "5th Test"],
        Format::Odi50 => ["1st ODI", "2nd ODI", "3rd ODI", "4th ODI", "5th ODI"],
        Format::T20 => ["1st T20", "2nd T20", "3rd T20", "4th T20", "5th T20"],
    };
    for i in 0..matches {
        // Alternate home advantage across the series.
        let (home, away) = if i % 2 == 0 {
            (home_nation, away_nation)
        } else {
            (away_nation, home_nation)
        };
        let label = labels.get(i as usize).copied().unwrap_or("T20").to_string();
        out.push(InternationalFixture {
            fixture: Fixture {
                id: first_id + i,
                competition_id,
                home_id: home,
                away_id: away,
                format,
                ground_id: ground_of(home),
                day: start_day + i * days_between(format),
                round: 0,
                stage: None,
                result: None,
            },
            stage: Stage::Series(label),
        });
    }
    out
}

fn days_between(format: Format) -> u32 {
    match format {
        Format::Test => 7,
        Format::Odi50 => 3,
        Format::T20 => 2,
    }
}

/// Group-stage standings for a single group, used to pick semi-finalists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupStanding {
    pub team_id: u32,
    pub points: u32,
    pub run_rate_x100: i32,
}

/// Points table for one group (label matches the Stage::Group label).
pub fn group_table(fixtures: &[InternationalFixture], group: &str) -> Vec<GroupStanding> {
    let group_teams: Vec<u32> = fixtures
        .iter()
        .filter(|f| matches!(&f.stage, Stage::Group(g) if g == group))
        .flat_map(|f| [f.fixture.home_id, f.fixture.away_id])
        .collect();

    let mut rows: Vec<GroupStanding> = group_teams
        .iter()
        .map(|id| GroupStanding {
            team_id: *id,
            points: 0,
            run_rate_x100: 0,
        })
        .collect();

    for entry in fixtures
        .iter()
        .filter(|f| matches!(&f.stage, Stage::Group(g) if g == group))
    {
        let f = &entry.fixture;
        let Some(result) = &f.result else { continue };
        for &(id, opponent, is_home) in
            &[(f.home_id, f.away_id, true), (f.away_id, f.home_id, false)]
        {
            let Some(row) = rows.iter_mut().find(|r| r.team_id == id) else {
                continue;
            };
            let (rf, ra) = if is_home {
                (result.home_runs, result.away_runs)
            } else {
                (result.away_runs, result.home_runs)
            };
            row.run_rate_x100 += rf as i32 - ra as i32;
            if result.winner_id == Some(id) {
                row.points += 2;
            } else if result.winner_id.is_none() {
                row.points += 1;
            } else {
                let _ = opponent;
            }
        }
    }

    rows.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.run_rate_x100.cmp(&a.run_rate_x100))
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ground_of(id: u32) -> u32 {
        id
    }

    #[test]
    fn world_cup_has_groups_semis_and_final() {
        let teams: Vec<u32> = (1..=12).collect();
        let fixtures = world_cup_fixtures(&teams, Format::T20, ground_of, 1, 99);

        // 4 groups of 3 = 12 group games, plus 2 semis and a final.
        let groups = fixtures
            .iter()
            .filter(|f| matches!(f.stage, Stage::Group(_)))
            .count();
        let semis = fixtures
            .iter()
            .filter(|f| f.stage == Stage::SemiFinal)
            .count();
        let finals = fixtures.iter().filter(|f| f.stage == Stage::Final).count();

        assert_eq!(groups, 12, "each group of 3 plays 3 games");
        assert_eq!(semis, 2);
        assert_eq!(finals, 1);

        // All 12 nations appear in the group stage.
        let mut played: Vec<u32> = fixtures
            .iter()
            .filter(|f| matches!(f.stage, Stage::Group(_)))
            .flat_map(|f| [f.fixture.home_id, f.fixture.away_id])
            .collect();
        played.sort_unstable();
        played.dedup();
        assert_eq!(played.len(), 12, "every nation plays");
    }

    #[test]
    fn group_table_ranks_winners_first() {
        let teams = vec![10, 11, 12];
        let mut fixtures = world_cup_fixtures(&teams, Format::T20, ground_of, 1, 5);
        // Make team 10 beat everyone.
        for f in fixtures.iter_mut() {
            if !matches!(f.stage, Stage::Group(_)) {
                continue;
            }
            if f.fixture.home_id == 10 || f.fixture.away_id == 10 {
                let winner = 10;
                let (hr, ar) = (200, 100);
                let (home_runs, away_runs) = if f.fixture.home_id == 10 {
                    (hr, ar)
                } else {
                    (ar, hr)
                };
                f.fixture.result = Some(crate::core::scheduler::FixtureResult {
                    home_runs,
                    away_runs,
                    home_wickets: 5,
                    away_wickets: 10,
                    winner_id: Some(winner),
                    summary: "10 won".into(),
                });
            }
        }
        let table = group_table(&fixtures, "A");
        assert_eq!(table[0].team_id, 10, "undefeated team must top the group");
        assert!(table[0].points >= 4);
    }

    #[test]
    fn series_alternates_home_advantage() {
        let fixtures = series_fixtures(1, 2, Format::Test, 3, ground_of, 1, 5, 700);
        assert_eq!(fixtures.len(), 3);
        assert_eq!(fixtures[0].fixture.home_id, 1);
        assert_eq!(fixtures[1].fixture.home_id, 2);
        assert_eq!(fixtures[2].fixture.home_id, 1);
        assert_eq!(fixtures[0].fixture.day + 7, fixtures[1].fixture.day);
        assert_eq!(fixtures[0].stage.label(), "1st Test");
    }

    #[test]
    fn knockout_qualifiers_are_filled_in() {
        let teams: Vec<u32> = (1..=12).collect();
        let mut fixtures = world_cup_fixtures(&teams, Format::T20, ground_of, 1, 3);
        let semi_index = fixtures
            .iter()
            .position(|f| f.stage == Stage::SemiFinal)
            .unwrap();
        resolve_knockouts(&mut fixtures, &[(semi_index, 7, 8)]);
        assert_eq!(fixtures[semi_index].fixture.home_id, 7);
        assert_eq!(fixtures[semi_index].fixture.away_id, 8);
    }
}
