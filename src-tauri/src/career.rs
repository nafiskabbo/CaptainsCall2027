//! Career mode: the game loop that ties competitions, fixtures and saves together.
//!
//! Flow: pick team → career starts with a round-robin → each match day, the
//! player's match is played ball-by-ball and the rest are simulated → table and
//! news update → season advances.

use crate::core::rng::SimRng;
use crate::core::scheduler::{self, Fixture, FixtureResult, TableRow};
use crate::db::NewsItem;
use crate::core::types::{Format, Player, World};
use crate::match_::state::{MatchPhase, MatchSnapshot};
use crate::match_::{MatchConfig, SimState, TeamSelection};
use crate::world::DB_VERSION;

/// A career the UI is managing.
#[derive(Debug)]
pub struct Career {
    pub state: crate::db::CareerState,
    /// Live match, present only while the player is playing their fixture.
    pub live: Option<LiveMatch>,
    /// Snapshot of the last completed player match (kept after the match ends).
    pub last_snapshot: Option<MatchSnapshot>,
}

/// A match the player is controlling.
pub struct LiveMatch {
    pub fixture_id: u32,
    pub sim: SimState,
    pub home_selection: TeamSelection,
    pub away_selection: TeamSelection,
}

impl std::fmt::Debug for LiveMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveMatch").field("fixture_id", &self.fixture_id).finish()
    }
}

impl LiveMatch {
    pub fn snapshot(&self) -> MatchSnapshot {
        self.sim.snapshot()
    }
}

impl Career {
    /// Start a career: builds the player's competition schedule.
    pub fn start(world: &World, team_id: u32, name: String, seed: u64) -> Result<Self, String> {
        let team = world
            .teams
            .iter()
            .find(|t| t.id == team_id)
            .cloned()
            .ok_or_else(|| format!("team {team_id} not found"))?;

        // Pick the first competition covering this team; default: a new T20 cup.
        let competition = world
            .competitions
            .iter()
            .find(|c| c.team_ids.contains(&team_id))
            .cloned()
            .ok_or_else(|| "no competition found for this team".to_string())?;

        let ground_of = |id: u32| {
            world
                .teams
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.home_ground_id)
                .unwrap_or(1)
        };
        let fixtures = scheduler::round_robin_fixtures(&competition, 1, ground_of, seed);

        let state = crate::db::CareerState {
            id: None,
            name,
            player_team_id: team_id,
            player_team_name: team.name.clone(),
            nation_id: team.nation_id,
            season_day: 1,
            db_version: DB_VERSION.to_string(),
            competition_id: competition.id,
            fixtures,
            news: vec![NewsItem {
                day: 1,
                headline: format!("You take charge of {}", team.name),
            }],
            seed,
        };

        Ok(Career {
            state,
            live: None,
            last_snapshot: None,
        })
    }

    /// Fixtures the player must (or may) play, soonest first.
    pub fn player_fixtures(&self) -> Vec<&Fixture> {
        let mut v: Vec<&Fixture> = self
            .state
            .fixtures
            .iter()
            .filter(|f| {
                (f.home_id == self.state.player_team_id || f.away_id == self.state.player_team_id)
                    && f.result.is_none()
            })
            .collect();
        v.sort_by_key(|f| f.day);
        v
    }

    /// The next player fixture, if any.
    pub fn next_fixture(&self) -> Option<&Fixture> {
        self.player_fixtures().into_iter().next()
    }

    /// League table for the competition.
    pub fn table(&self, world: &World) -> Vec<TableRow> {
        let team_ids: Vec<u32> = world
            .competitions
            .iter()
            .find(|c| c.id == self.state.competition_id)
            .map(|c| c.team_ids.clone())
            .unwrap_or_default();
        scheduler::table_for(&team_ids, &self.state.fixtures)
    }

    /// Begin the player's fixture. CPU selection is the default for now
    /// (full squad selection screen is M4).
    pub fn start_fixture(&mut self, world: &World, fixture_id: u32) -> Result<MatchSnapshot, String> {
        if self.live.is_some() {
            return Err("a match is already in progress".into());
        }
        let fixture = self
            .state
            .fixtures
            .iter()
            .find(|f| f.id == fixture_id)
            .cloned()
            .ok_or_else(|| format!("fixture {fixture_id} not found"))?;
        if fixture.result.is_some() {
            return Err("that match is already played".into());
        }
        if fixture.home_id != self.state.player_team_id
            && fixture.away_id != self.state.player_team_id
        {
            return Err("that match does not involve your team".into());
        }

        let home = world
            .teams
            .iter()
            .find(|t| t.id == fixture.home_id)
            .cloned()
            .ok_or("home team missing")?;
        let away = world
            .teams
            .iter()
            .find(|t| t.id == fixture.away_id)
            .cloned()
            .ok_or("away team missing")?;

        let home_sel = TeamSelection::auto_for(&home, &world.players, fixture.format);
        let away_sel = TeamSelection::auto_for(&away, &world.players, fixture.format);

        // Seed derived from career + fixture so results are reproducible.
        let seed = self
            .state
            .seed
            .wrapping_add(fixture.id as u64)
            .wrapping_mul(0x9E3779B97F4A7C15);

        let participants: Vec<Player> = home_sel
            .xi_ids
            .iter()
            .chain(away_sel.xi_ids.iter())
            .filter_map(|id| world.players.iter().find(|p| p.id == *id).cloned())
            .collect();

        let mut sim = SimState::new(
            MatchConfig {
                match_id: fixture.id,
                home_team: home,
                away_team: away,
                format: fixture.format,
                ground_id: fixture.ground_id,
                days: if matches!(fixture.format, Format::Test) {
                    crate::match_::sim::MAX_DAYS_TEST
                } else {
                    1
                },
            },
            seed,
            home_sel.clone(),
            away_sel.clone(),
        );
        sim.attach_players(participants);
        sim.start();

        let snapshot = sim.snapshot();
        self.live = Some(LiveMatch {
            fixture_id,
            sim,
            home_selection: home_sel,
            away_selection: away_sel,
        });
        Ok(snapshot)
    }

    pub fn live_snapshot(&self) -> Option<MatchSnapshot> {
        self.live
            .as_ref()
            .map(|m| m.snapshot())
            .or_else(|| self.last_snapshot.clone())
    }

    /// Simulate N balls in the live match (no-op once the match is over).
    pub fn advance(&mut self, balls: u32) -> MatchSnapshot {
        let Some(live) = self.live.as_mut() else {
            return self.last_snapshot.clone().unwrap_or_else(MatchSnapshot::empty);
        };
        live.sim.bowl_many(balls.max(1));
        let snapshot = live.snapshot();
        self.maybe_finish();
        snapshot
    }

    /// Apply engine-side CPU automation for the opposition's innings.
    pub fn set_orders(&mut self, orders: match_orders::Orders) {
        if let Some(live) = self.live.as_mut() {
            orders.apply(&mut live.sim.input);
        }
    }

    /// Fast-forward to the end of the current innings (no-op if match is over).
    pub fn advance_innings(&mut self) -> MatchSnapshot {
        let Some(live) = self.live.as_mut() else {
            return self.last_snapshot.clone().unwrap_or_else(MatchSnapshot::empty);
        };
        live.sim.bowl_many(1_000_000);
        let snapshot = live.snapshot();
        self.maybe_finish();
        snapshot
    }

    /// Finish the live match (if complete), record it, and simulate the rest of
    /// the day's fixtures. Returns news generated.
    pub fn maybe_finish(&mut self) {
        let complete = self
            .live
            .as_ref()
            .map(|l| l.sim.snapshot().phase == MatchPhase::Complete)
            .unwrap_or(false);
        if !complete {
            return;
        }
        let (fixture_id, result) = {
            let live = self.live.as_ref().expect("live match");
            let snap = live.sim.snapshot();
            let home = &snap.innings[0];
            let away = snap
                .innings
                .get(1)
                .cloned()
                .unwrap_or_else(|| empty_innings(home.batting_team_id, home.bowling_team_id));
            let (home_runs, home_wickets, away_runs, away_wickets) = (
                home.runs,
                home.wickets,
                away.runs,
                away.wickets,
            );
            let winner_id = snap.result.as_ref().and_then(|r| r.winner_team_id);
            let summary = snap
                .result
                .as_ref()
                .map(|r| r.summary.clone())
                .unwrap_or_else(|| "Match complete".into());
            (
                live.fixture_id,
                FixtureResult {
                    home_runs,
                    away_runs,
                    home_wickets,
                    away_wickets,
                    winner_id,
                    summary,
                },
            )
        };
        let final_snapshot = self.live.as_ref().map(|l| l.snapshot());
        self.live = None;
        self.last_snapshot = final_snapshot;
        crate::db::record_result(&mut self.state, fixture_id, result);
    }

    /// Simulate every other fixture on the given day.
    pub fn simulate_day(&mut self, world: &World, day: u32) -> Vec<NewsItem> {
        let mut news = Vec::new();
        let day_fixtures: Vec<Fixture> = self
            .state
            .fixtures
            .iter()
            .filter(|f| f.day == day && f.result.is_none())
            .cloned()
            .collect();

        for fixture in day_fixtures {
            if fixture.home_id == self.state.player_team_id
                || fixture.away_id == self.state.player_team_id
            {
                continue; // the player plays this one
            }
            let result = simulate_fixture(world, &fixture, self.state.seed ^ (fixture.id as u64));
            if let Some(result) = result {
                news.extend(crate::db::record_result(&mut self.state, fixture.id, result));
            }
        }
        news
    }

    /// Move to the next day with fixtures; auto-simulate passed days.
    pub fn advance_day(&mut self, world: &World) -> Vec<NewsItem> {
        let current = self.state.season_day;
        let next = self
            .state
            .fixtures
            .iter()
            .filter(|f| f.day > current)
            .map(|f| f.day)
            .min()
            .unwrap_or(current + 1);
        // Simulate everything between now and then.
        let mut news = Vec::new();
        for day in current..next {
            news.extend(self.simulate_day(world, day));
        }
        self.state.season_day = next;
        news
    }

    /// Is the season finished?
    pub fn season_complete(&self) -> bool {
        self.state.fixtures.iter().all(|f| f.result.is_some())
    }
}

mod match_orders {
    use crate::match_::state::{FieldPreset, MatchInput};

    #[derive(Debug, Clone, Copy)]
    pub struct Orders {
        pub aggression: u8,
        pub field: FieldPreset,
        pub hit_and_run: bool,
        pub keep_strike: bool,
    }

    impl Orders {
        pub fn apply(&self, input: &mut MatchInput) {
            input.batting_aggression = self.aggression;
            input.field_preset = self.field;
            input.hit_and_run_toggle = self.hit_and_run;
            input.keep_strike_toggle = self.keep_strike;
        }
    }

}

/// Simulate a fixture end-to-end and score it.
pub fn simulate_fixture(world: &World, fixture: &Fixture, seed: u64) -> Option<FixtureResult> {
    let home = world.teams.iter().find(|t| t.id == fixture.home_id)?;
    let away = world.teams.iter().find(|t| t.id == fixture.away_id)?;
    let home_sel = TeamSelection::auto_for(home, &world.players, fixture.format);
    let away_sel = TeamSelection::auto_for(away, &world.players, fixture.format);
    let participants: Vec<Player> = home_sel
        .xi_ids
        .iter()
        .chain(away_sel.xi_ids.iter())
        .filter_map(|id| world.players.iter().find(|p| p.id == *id).cloned())
        .collect();

    let mut sim = SimState::new(
        MatchConfig {
            match_id: fixture.id,
            home_team: home.clone(),
            away_team: away.clone(),
            format: fixture.format,
            ground_id: fixture.ground_id,
            days: if matches!(fixture.format, Format::Test) {
                crate::match_::sim::MAX_DAYS_TEST
            } else {
                1
            },
        },
        seed,
        home_sel,
        away_sel,
    );
    sim.attach_players(participants);
    sim.start();
    sim.bowl_many(1_000_000);

    let snap = sim.snapshot();
    let first = snap.innings.first()?;
    let second = snap
        .innings
        .get(1)
        .cloned()
        .unwrap_or_else(|| empty_innings(first.batting_team_id, first.bowling_team_id));

    // Map innings back to home/away regardless of toss.
    let (home_runs, home_wickets, away_runs, away_wickets) = if snap.toss_winner == fixture.home_id
    {
        (first.runs, first.wickets, second.runs, second.wickets)
    } else {
        (second.runs, second.wickets, first.runs, first.wickets)
    };

    let winner_id = snap.result.as_ref().and_then(|r| r.winner_team_id);
    let summary = snap
        .result
        .as_ref()
        .map(|r| r.summary.clone())
        .unwrap_or_else(|| "Match drawn".into());

    Some(FixtureResult {
        home_runs,
        away_runs,
        home_wickets,
        away_wickets,
        winner_id,
        summary,
    })
}

fn empty_innings(batting: u32, bowling: u32) -> crate::match_::state::Innings {
    crate::match_::state::Innings {
        batting_team_id: batting,
        bowling_team_id: bowling,
        runs: 0,
        wickets: 0,
        balls: 0,
        extras: 0,
        batting_cards: vec![],
        bowling_cards: vec![],
        fow: vec![],
        over_summaries: vec![],
        is_declared: false,
    }
}

/// Helper: a deterministic seed for a career's fixtures.
pub fn derive_seed(base: u64, fixture_id: u32) -> u64 {
    let mut rng = SimRng::new(base ^ fixture_id as u64);
    rng.next_u64()
}
