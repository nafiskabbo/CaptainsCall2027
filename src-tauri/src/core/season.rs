//! Season rollover: aging, retirements, youth intake, budgets, new fixtures.
//!
//! Without this a career dead-ends at the final round. This module takes a
//! finished season and produces the next one: players age and retire,
//! academies bring through replacements, club finances refresh and a fresh
//! fixture list is generated (specs FR-23, FR-24).

use super::super::core::development;
use super::super::core::names::{FIRST_NAMES, LAST_NAMES};
use super::super::core::rng::SimRng;
use super::super::core::scheduler::round_robin_fixtures;
use super::super::core::types::{Competition, Format, Nation, Player, Role, Skills, Team, World};
use serde::{Deserialize, Serialize};

/// What happened between seasons, for the news feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OffSeasonReport {
    pub retired: Vec<String>,
    pub intake: Vec<String>,
    pub promoted: Vec<String>,
    pub budget_refresh: i64,
    pub new_season_fixtures: usize,
}

/// A player considering retirement.
#[derive(Debug, Clone)]
struct RetirementCandidate {
    id: u32,
    score: f64,
}

/// Roll a finished season into the next one.
///
/// Returns the news-worthy events. Mutates `state.players` in place.
pub fn roll_season(
    state: &mut crate::db::CareerState,
    world: &World,
    season: u32,
) -> OffSeasonReport {
    let seed = state.seed ^ 0xA11CE ^ (season as u64).wrapping_mul(0x9E3779B97F4A7C15);
    let mut rng = SimRng::new(seed);

    // 1. Age everyone and reset form.
    let mut aged = development::to_map(state.players.clone());
    development::roll_season(&mut aged);
    state.players = development::to_vec(aged);

    // 2. Retirements.
    let mut retired = Vec::new();
    let mut candidates: Vec<RetirementCandidate> = Vec::new();
    for p in state.players.iter() {
        let age = p.age as f64;
        // Older, worse, tired players retire; promising youngsters never do.
        let peak = p
            .potential
            .bat
            .0
            .max(p.potential.pace.0)
            .max(p.potential.spin.0) as f64;
        let score = (age - 28.0) * 0.35 - (peak - 50.0) * 0.06 + (rng.next_f64() * 4.0 - 2.0);
        if age >= 29.0 && score > 0.0 {
            candidates.push(RetirementCandidate { id: p.id, score });
        }
    }
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Retire enough to keep squads healthy: roughly 2 per senior squad.
    let retire_target = (state.players.len() / 9).max(2).min(candidates.len());
    for candidate in candidates.into_iter().take(retire_target) {
        if let Some(idx) = state.players.iter().position(|p| p.id == candidate.id) {
            let name = state.players[idx].name.clone();
            state.players.remove(idx);
            retired.push(name);
        }
    }

    // 3. Youth intake: replace retirees (plus a fixed intake per nation) so every
    //    squad stays full across seasons.
    let intake = generate_intake(state, world, &mut rng);

    // 4. Budget refresh: club income scales with reputation.
    let player_team = world.teams.iter().find(|t| t.id == state.player_team_id);
    let reputation = player_team.map(|t| t.reputation.0).unwrap_or(50);
    let budget_refresh = (reputation as i64) * 45_000;
    state.budget = Some(budget_refresh);

    // 5. Fresh fixtures.
    let is_international = world
        .teams
        .iter()
        .find(|t| t.id == state.player_team_id)
        .map(|t| t.is_national)
        .unwrap_or(false);
    if is_international {
        let ground_of = |id: u32| {
            world
                .teams
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.home_ground_id)
                .unwrap_or(1)
        };
        use crate::core::tournaments;
        let nations: Vec<u32> = world
            .teams
            .iter()
            .filter(|t| t.is_national)
            .map(|t| t.id)
            .collect();
        let cup =
            tournaments::world_cup_fixtures(&nations, Format::T20, ground_of, 1, seed);
        let mut day = cup.iter().map(|e| e.fixture.day).max().unwrap_or(40) + 7;
        state.fixtures = Vec::new();
        for entry in &cup {
            if entry.fixture.home_id == state.player_team_id
                || entry.fixture.away_id == state.player_team_id
            {
                let mut fixture = entry.fixture.clone();
                fixture.stage = Some(entry.stage.label());
                state.fixtures.push(fixture);
            }
        }
        for (i, other) in nations
            .iter()
            .filter(|id| **id != state.player_team_id)
            .take(2)
            .enumerate()
        {
            let format = if i == 0 { Format::Odi50 } else { Format::Test };
            let matches = if format == Format::Test { 2 } else { 3 };
            let series = tournaments::series_fixtures(
                state.player_team_id,
                *other,
                format,
                matches,
                ground_of,
                day,
                91_000 + i as u32,
                950_000 + (i as u32) * 10,
            );
            state.fixtures.extend(series.into_iter().map(|e| {
                let mut fixture = e.fixture;
                fixture.stage = Some(e.stage.label());
                fixture
            }));
            day += matches * 4 + 3;
        }
        for f in state.fixtures.iter_mut() {
            if f.home_id == 0 {
                f.home_id = state.player_team_id;
            }
            if f.away_id == 0 {
                if let Some(other) = nations.iter().find(|id| **id != state.player_team_id) {
                    f.away_id = *other;
                }
            }
        }
        state.fixtures.sort_by_key(|f| f.day);
        state.competition_id = 90_000;
        let count = state.fixtures.len();
        state.season_day = 1;
        return OffSeasonReport {
            retired,
            intake,
            promoted: Vec::new(),
            budget_refresh,
            new_season_fixtures: count,
        };
    }

    let competition = world
        .competitions
        .iter()
        .find(|c| c.id == state.competition_id)
        .cloned();
    let new_season_fixtures = if let Some(comp) = competition {
        let ground_of = |id: u32| {
            world
                .teams
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.home_ground_id)
                .unwrap_or(1)
        };
        state.fixtures = round_robin_fixtures(&comp, 1, ground_of, seed);
        state.fixtures.len()
    } else {
        0
    };
    state.season_day = 1;
    state.news.insert(
        0,
        crate::db::NewsItem {
            day: 1,
            headline: format!("Season {} begins", season + 1),
        },
    );

    OffSeasonReport {
        retired,
        intake,
        promoted: Vec::new(),
        budget_refresh,
        new_season_fixtures,
    }
}

/// Create academy players to refill squads for the new season.
fn generate_intake(
    state: &mut crate::db::CareerState,
    world: &World,
    rng: &mut SimRng,
) -> Vec<String> {
    let nations: Vec<&Nation> = world.nations.iter().collect();
    if nations.is_empty() {
        return Vec::new();
    }
    let mut created = Vec::new();
    let mut next_id = state
        .players
        .iter()
        .map(|p| p.id)
        .max()
        .unwrap_or(1000)
        + 1;

    for nation in &nations {
        // How many short we are for this nation's teams.
        let owned: Vec<&Team> = world
            .teams
            .iter()
            .filter(|t| t.nation_id == nation.id)
            .collect();
        let need = (owned.len() * 8).max(4);
        let have = state
            .players
            .iter()
            .filter(|p| p.nation_id == nation.id)
            .count();
        let shortfall = need.saturating_sub(have).min(12);

        for i in 0..shortfall {
            let team = owned.get(i % owned.len().max(1));
            let team_id = team.map(|t| t.id);
            let base = (nation.strength as i32 - 18).clamp(20, 92) as u8;
            let jitter = rng.below(14) as i32 - 7;
            let bat = (base as i32 + jitter).clamp(20, 96) as u8;
            let bowl = (base as i32 + jitter + 6).clamp(20, 96) as u8;
            let age = 17 + rng.below(6) as u8; // 17-22

            let role = match i % 6 {
                0 => Role::Batter,
                1 => Role::Wicketkeeper,
                2 | 3 => Role::Allrounder,
                4 => Role::PaceBowler,
                _ => Role::SpinBowler,
            };
            let spin = matches!(role, Role::SpinBowler) || (matches!(role, Role::Allrounder) && rng.chance(0.4));

            use crate::core::types::Rating;
            let skills = Skills {
                bat: Rating(bat),
                test_bat: Rating(bat),
                odi_bat: Rating(bat),
                t20_bat: Rating(bat),
                defence: Rating(bat),
                pace: Rating(if spin { 15 } else { bowl }),
                spin: Rating(if spin { bowl } else { 15 }),
                odi_bowl: Rating(bowl),
                t20_bowl: Rating(bowl),
                fielding: Rating(50 + rng.below(20) as u8),
                keeping: Rating(if matches!(role, Role::Wicketkeeper) { 65 } else { 15 }),
                leadership: Rating(20),
            };
            let potential = Skills {
                bat: Rating((bat as i32 + 6 + rng.below(8) as i32).min(99) as u8),
                pace: Rating((skills.pace.0 as i32 + 5 + rng.below(8) as i32).min(99) as u8),
                spin: Rating((skills.spin.0 as i32 + 5 + rng.below(8) as i32).min(99) as u8),
                ..skills
            };

            let name = format!(
                "{} {}",
                FIRST_NAMES[rng.below(FIRST_NAMES.len() as u32) as usize],
                LAST_NAMES[rng.below(LAST_NAMES.len() as u32) as usize]
            );
            created.push(name.clone());
            state.players.push(Player {
                id: next_id,
                name,
                nation_id: nation.id,
                team_id,
                age,
                role,
                batting_style: if rng.chance(0.25) { "lhb" } else { "rhb" }.to_string(),
                bowling_style: None,
                skills,
                potential,
                form: 0,
                fitness: 100,
                morale: 70,
                experience: 0,
                injury_days: 0,
            });
            next_id += 1;
        }
    }
    created
}

/// The competition format for a career, used when generating fixtures.
pub fn competition_format(competition: &Competition) -> Format {
    competition.format
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Rating;
    use crate::db::NewsItem;

    fn world_with_players() -> World {
        let mut world = World::default();
        world.nations.push(Nation {
            id: 1,
            name: "Testland".into(),
            short_name: "TST".into(),
            strength: 80,
            has_test_status: true,
        });
        for i in 0..3u32 {
            world.teams.push(Team {
                id: 100 + i,
                name: format!("Team {i}"),
                nation_id: 1,
                home_ground_id: 1,
                budget: 1_000_000,
                reputation: crate::core::types::Rating(60),
                squad_ids: vec![],
                is_national: false,
            });
        }
        world.competitions.push(Competition {
            id: 1,
            name: "Test League".into(),
            nation_id: 1,
            format: Format::T20,
            kind: crate::core::types::CompetitionKind::League,
            team_ids: vec![100, 101, 102],
            days_per_match: 1,
        });
        world
    }

    fn career(world: &World, ages: Vec<u8>) -> crate::db::CareerState {
        let players: Vec<Player> = ages
            .into_iter()
            .enumerate()
            .map(|(i, age)| Player {
                id: 1000 + i as u32,
                name: format!("Player {i}"),
                nation_id: 1,
                team_id: Some(100 + (i % 3) as u32),
                age,
                role: Role::Batter,
                batting_style: "rhb".into(),
                bowling_style: None,
                skills: Skills {
                    bat: Rating(60),
                    ..Default::default()
                },
                potential: Skills {
                    bat: Rating(80),
                    ..Default::default()
                },
                form: 0,
                fitness: 100,
                morale: 70,
                experience: 10,
                injury_days: 0,
            })
            .collect();
        crate::db::CareerState {
            id: None,
            name: "Career".into(),
            player_team_id: 100,
            player_team_name: "Team 0".into(),
            nation_id: 1,
            season_day: 40,
            db_version: "2027.01".into(),
            competition_id: 1,
            fixtures: vec![crate::core::scheduler::Fixture {
                id: 1,
                competition_id: 1,
                home_id: 100,
                away_id: 101,
                format: Format::T20,
                ground_id: 1,
                day: 1,
                round: 0,
                stage: None,
                result: None,
            }],
            news: vec![],
            selections: Default::default(),
            players,
            budget: None,
            season: 1,
            seed: 42,
        }
    }

    #[test]
    fn roll_season_ages_players_and_refreshes_fixtures() {
        let world = world_with_players();
        let mut state = career(&world, vec![20, 30, 31, 32, 33, 34, 24, 25, 26, 27, 28, 29]);
        let report = roll_season(&mut state, &world, 1);

        assert_eq!(state.season_day, 1, "season resets");
        assert_eq!(state.fixtures.len(), 3, "3 teams -> 3 fixtures");
        assert!(report.new_season_fixtures > 0);
        assert!(state.budget.unwrap_or(0) > 0, "budget refreshes");
        // Everyone who survived is a year older (intake players are new).
        let survivors: Vec<&Player> = state
            .players
            .iter()
            .filter(|p| !report.intake.contains(&p.name))
            .collect();
        assert!(!survivors.is_empty());
        for p in survivors {
            assert!(p.age >= 21, "survivors aged {}", p.age);
        }
    }

    #[test]
    fn older_players_tend_to_retire() {
        let world = world_with_players();
        let mut total_retired = 0;
        for season in 0..8u32 {
            let mut state = career(&world, vec![19; 24]);
            // Age half the squad hard to force retirements.
            for (i, p) in state.players.iter_mut().enumerate() {
                if i % 2 == 0 {
                    p.age = 36 + (i % 4) as u8;
                    p.potential.bat = Rating(45);
                }
            }
            let report = roll_season(&mut state, &world, season);
            total_retired += report.retired.len();
        }
        assert!(total_retired > 4, "veterans should retire over time, got {total_retired}");
    }

    #[test]
    fn youth_intake_refills_squads() {
        let world = world_with_players();
        let mut state = career(&world, vec![36; 12]); // all retiring age
        let before = state.players.len();
        let report = roll_season(&mut state, &world, 1);
        assert!(
            !report.intake.is_empty(),
            "intake should bring through new players"
        );
        assert!(
            state.players.len() <= before + report.intake.len(),
            "intake should not duplicate"
        );
        // Newcomers (named in the intake report) are young.
        let ids: Vec<u32> = state
            .players
            .iter()
            .filter(|p| report.intake.contains(&p.name))
            .map(|p| p.id)
            .collect();
        assert!(!ids.is_empty(), "intake players exist in the squad");
        for p in state.players.iter().filter(|p| ids.contains(&p.id)) {
            assert!(p.age <= 22, "intake players must be academy age, got {}", p.age);
        }
    }

    #[test]
    fn news_records_the_new_season() {
        let world = world_with_players();
        let mut state = career(&world, vec![24; 11]);
        roll_season(&mut state, &world, 2);
        assert!(state.news.iter().any(|n: &NewsItem| n.headline.contains("Season 3")));
    }
}
