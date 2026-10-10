//! Match runner: owns live state, advances deliveries, emits snapshots.
//!
//! Rust is authoritative. The UI sends orders and "advance one unit" requests;
//! all randomness flows through `SimRng` seeded from the match seed, so the same
//! seed + orders reproduces the same match (specs FR-13, NFR-7).

use serde::{Deserialize, Serialize};

use super::engine::{outcome_probabilities, resolve_ball, BallContext};
use super::state::{
    BallOutcome, BattingCard, BowlingCard, ExtraKind, FieldPreset, Innings, MatchClock, MatchInput,
    MatchPhase, MatchResult, OverSummary, TossDecision, WinMargin,
};
use crate::core::rng::SimRng;
use crate::core::types::{Format, Player, Team};
use std::collections::HashMap;

const OVERS_PER_DAY: u32 = 90;
pub(crate) const MAX_DAYS_TEST: u8 = 5;
const TEST_WICKETS: u8 = 10;

pub struct MatchConfig {
    pub match_id: u32,
    pub home_team: Team,
    pub away_team: Team,
    pub format: Format,
    pub ground_id: u32,
    pub days: u8,
}

/// A selection: batting XI, order, keeper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamSelection {
    pub team_id: u32,
    pub xi_ids: Vec<u32>,
    pub batting_order: Vec<u32>,
    pub keeper_id: Option<u32>,
}

impl TeamSelection {
    /// Sensible CPU XI/order. Domestic sides pick from their contracted squad;
    /// national sides pick from every player in that nation (as coaches do).
    pub fn auto_for(team: &Team, players: &[Player], format: Format) -> Self {
        let mut pool: Vec<&Player> = players
            .iter()
            .filter(|p| {
                if team.is_national {
                    p.nation_id == team.nation_id
                } else {
                    p.team_id == Some(team.id)
                }
            })
            .collect();
        // Sort by usefulness for the format: keepers and bowlers float up.
        pool.sort_by_key(|p| std::cmp::Reverse(usefulness(p, format)));
        let xi: Vec<Player> = pool.iter().take(11).map(|p| (*p).clone()).collect();

        // Batting order: top-7 by batting, then keepers/allrounders, then bowlers.
        let mut batting: Vec<Player> = xi.clone();
        batting.sort_by_key(|p| std::cmp::Reverse(p.skills.bat_for(format).0));

        let keeper = xi
            .iter()
            .filter(|p| matches!(p.role, crate::core::types::Role::Wicketkeeper))
            .max_by_key(|p| p.skills.keeping.0)
            .map(|p| p.id);

        TeamSelection {
            team_id: team.id,
            xi_ids: xi.iter().map(|p| p.id).collect(),
            batting_order: batting.iter().map(|p| p.id).collect(),
            keeper_id: keeper,
        }
    }
}

fn usefulness(p: &Player, format: Format) -> u32 {
    let bat = p.skills.bat_for(format).0 as u32;
    let bowl = p.skills.bowl_for(format).0 as u32;
    let keep = p.skills.keeping.0 as u32 / 2;
    let field = p.skills.fielding.0 as u32 / 4;
    bat + bowl + keep + field
}

pub struct SimState {
    pub cfg: MatchConfig,
    pub seed: u64,
    pub rng: SimRng,
    pub input: MatchInput,
    pub home_selection: TeamSelection,
    pub away_selection: TeamSelection,
    pub phase: MatchPhase,
    pub current_innings: usize,
    pub innings: Vec<Innings>,
    pub striker_id: Option<u32>,
    pub non_striker_id: Option<u32>,
    pub bowler_id: Option<u32>,
    pub last_bowler_id: Option<u32>,
    pub toss_winner: u32,
    pub toss_decision: TossDecision,
    pub clock: MatchClock,
    pub history: Vec<BallOutcome>,
    pub players: HashMap<u32, Player>,
    pub follow_on_offered: bool,
    pub over_start_runs: u32,
    pub over_start_wickets: u8,
    pub result: Option<MatchResult>,
}

impl SimState {
    pub fn new(
        cfg: MatchConfig,
        seed: u64,
        home_sel: TeamSelection,
        away_sel: TeamSelection,
    ) -> Self {
        let mut toss_rng = SimRng::new(seed ^ 0xA5A5_1234);
        let toss_winner = if toss_rng.chance(0.5) {
            cfg.home_team.id
        } else {
            cfg.away_team.id
        };
        let toss_decision = if toss_rng.chance(0.5) {
            TossDecision::Bat
        } else {
            TossDecision::Bowl
        };

        SimState {
            cfg,
            seed,
            rng: SimRng::new(seed),
            input: MatchInput::default(),
            home_selection: home_sel,
            away_selection: away_sel,
            phase: MatchPhase::PreMatch,
            current_innings: 0,
            innings: Vec::new(),
            striker_id: None,
            non_striker_id: None,
            bowler_id: None,
            last_bowler_id: None,
            toss_winner,
            toss_decision,
            clock: MatchClock {
                day: 1,
                session: 1,
                balls_today: 0,
                balls_in_session: 0,
            },
            history: Vec::new(),
            players: HashMap::new(),
            follow_on_offered: false,
            over_start_runs: 0,
            over_start_wickets: 0,
            result: None,
        }
    }

    pub fn attach_players(&mut self, players: Vec<Player>) {
        for p in players {
            self.players.insert(p.id, p);
        }
    }

    pub fn player(&self, id: u32) -> Player {
        self.players
            .get(&id)
            .cloned()
            .unwrap_or_else(|| Player::with_name(id))
    }

    fn selection_of(&self, team_id: u32) -> &TeamSelection {
        if team_id == self.cfg.home_team.id {
            &self.home_selection
        } else {
            &self.away_selection
        }
    }

    fn team_of(&self, team_id: u32) -> &Team {
        if team_id == self.cfg.home_team.id {
            &self.cfg.home_team
        } else {
            &self.cfg.away_team
        }
    }

    fn opponent(&self, team_id: u32) -> u32 {
        if team_id == self.cfg.home_team.id {
            self.cfg.away_team.id
        } else {
            self.cfg.home_team.id
        }
    }

    fn fresh_innings(&self, batting_team_id: u32) -> Innings {
        let bowling_team_id = self.opponent(batting_team_id);
        let sel = self.selection_of(batting_team_id);
        let batting_cards: Vec<BattingCard> = sel
            .batting_order
            .iter()
            .enumerate()
            .map(|(i, id)| BattingCard {
                player_id: *id,
                name: self.player(*id).name.clone(),
                runs: 0,
                balls: 0,
                fours: 0,
                sixes: 0,
                out: None,
                batting_position: (i + 1) as u8,
            })
            .collect();

        let bowling_sel = self.selection_of(bowling_team_id);
        let format = self.cfg.format;
        let mut bowlers: Vec<u32> = bowling_sel.batting_order.clone();
        bowlers.sort_by_key(|id| std::cmp::Reverse(self.player(*id).skills.bowl_for(format).0));
        let bowling_cards: Vec<BowlingCard> = bowlers
            .iter()
            .take(8)
            .map(|id| BowlingCard {
                player_id: *id,
                name: self.player(*id).name.clone(),
                balls: 0,
                runs: 0,
                wickets: 0,
                maidens: 0,
                wides: 0,
                no_balls: 0,
            })
            .collect();

        Innings {
            batting_team_id,
            bowling_team_id,
            runs: 0,
            wickets: 0,
            balls: 0,
            extras: 0,
            batting_cards,
            bowling_cards,
            fow: Vec::new(),
            over_summaries: Vec::new(),
            is_declared: false,
        }
    }

    /// Resolve the toss and open innings 1.
    pub fn start(&mut self) {
        let batting = if self.toss_decision == TossDecision::Bat {
            self.toss_winner
        } else {
            self.opponent(self.toss_winner)
        };
        self.innings = vec![self.fresh_innings(batting)];
        self.current_innings = 0;
        self.phase = MatchPhase::Innings1;
        self.open_innings();
    }

    fn open_innings(&mut self) {
        let batting_id = self.current_innings_ref().batting_team_id;
        let sel = self.selection_of(batting_id);
        let opener = sel.batting_order.first().copied().unwrap_or(0);
        let nopener = sel.batting_order.get(1).copied().unwrap_or(0);
        self.striker_id = Some(opener);
        self.non_striker_id = Some(nopener);
        self.next_bowler();
        self.over_start_runs = 0;
        self.over_start_wickets = 0;
    }

    pub fn current_innings_ref(&self) -> &Innings {
        &self.innings[self.current_innings]
    }

    fn current_innings_mut(&mut self) -> &mut Innings {
        &mut self.innings[self.current_innings]
    }

    fn current_batting_id(&self) -> u32 {
        self.current_innings_ref().batting_team_id
    }

    fn next_bowler(&mut self) {
        let innings = self.current_innings_ref();
        let preferred = self.input.target_bowler_id;
        let last = self.last_bowler_id;
        let mut candidates: Vec<u32> = innings
            .bowling_cards
            .iter()
            .filter(|c| Some(c.player_id) != last)
            .map(|c| c.player_id)
            .collect();
        // Guard: don't give a weak bowler the final overs in LO cricket.
        if self.cfg.format.max_overs().is_some() {
            let remaining = self.overs_remaining();
            if remaining <= 4 {
                candidates.sort_by_key(|id| {
                    std::cmp::Reverse(self.player(*id).skills.bowl_for(self.cfg.format).0)
                });
                // Keep only the top 3 rated options left.
                candidates.truncate(3);
            }
        }
        let pool = if preferred != 0 && candidates.contains(&preferred) {
            vec![preferred]
        } else {
            candidates
        };
        let chosen = pool
            .iter()
            .max_by_key(|id| self.player(**id).skills.bowl_for(self.cfg.format).0)
            .copied();
        self.bowler_id = chosen;
    }

    fn overs_remaining(&self) -> u32 {
        let inn = self.current_innings_ref();
        match self.cfg.format.max_overs() {
            Some(max) => (max * 6 - inn.balls).div_ceil(6),
            None => 0,
        }
    }

    pub fn snapshot(&self) -> super::state::MatchSnapshot {
        super::state::MatchSnapshot {
            match_id: self.cfg.match_id,
            format: self.cfg.format,
            ground_id: self.cfg.ground_id,
            home_team_id: self.cfg.home_team.id,
            away_team_id: self.cfg.away_team.id,
            phase: self.phase,
            current_innings: self.current_innings,
            innings: self.innings.clone(),
            striker_id: self.striker_id,
            non_striker_id: self.non_striker_id,
            bowler_id: self.bowler_id,
            toss_winner: self.toss_winner,
            toss_decision: self.toss_decision,
            seed: self.seed,
            clock: self.clock,
            recent_balls: self.history.iter().rev().take(24).cloned().collect(),
            result: self.result.clone(),
        }
    }

    /// How much of a boost the batting side gets from being the underdog.
    ///
    /// Compares the average rating of the two XIs actually on the field.
    fn minnow_boost(&self) -> f64 {
        let batting_ids = self.current_innings_ref().batting_team_id;
        let bowling_ids = self.current_innings_ref().bowling_team_id;
        let batting = self.side_strength_of(batting_ids);
        let bowling = self.side_strength_of(bowling_ids);
        crate::match_::engine::minnow_boost(batting, bowling)
    }

    /// The probability of each outcome on the next delivery, in engine order
    /// (dot, 1, 2, 3, 4, 6, out, wide, no-ball).
    ///
    /// This is what makes the shot picker a decision rather than a guess: the
    /// player can see what their shot is actually worth against the bowler
    /// currently in. Builds the same context as [`Self::bowl_ball`] but consumes
    /// no randomness, so previewing cannot alter the match.
    pub fn preview_ball(&self) -> [f64; 9] {
        self.preview_ball_with(&self.input)
    }

    /// The same preview against hypothetical orders.
    ///
    /// Takes the input by reference rather than reading `self.input`, so the
    /// picker can price options it has not committed to.
    pub fn preview_ball_with(&self, input: &MatchInput) -> [f64; 9] {
        let (Some(striker_id), Some(bowler_id)) = (self.striker_id, self.bowler_id) else {
            return [0.0; 9];
        };
        if self.phase == MatchPhase::Complete {
            return [0.0; 9];
        }
        let striker = self.player(striker_id);
        let bowler = self.player(bowler_id);
        let bowling_team = self
            .team_of(self.current_innings_ref().bowling_team_id)
            .clone();
        let innings = self.current_innings_ref().clone();
        let over = innings.balls / 6;
        let powerplay = over < self.cfg.format.powerplay_overs();
        let chasing = self.is_chasing();
        let rrr = self.required_rate();

        // Weight building borrows the innings mutably even though it only reads
        // them; `outcome_probabilities` takes the context by value.
        let mut scratch = innings;
        let ctx = BallContext {
            striker: &striker,
            batter_id: striker_id,
            bowler: &bowler,
            fielding_team: &bowling_team,
            innings: &mut scratch,
            format: self.cfg.format,
            over,
            powerplay,
            chasing,
            run_rate_needed: rrr,
            minnow_boost: self.minnow_boost(),
            input,
            rng: &mut SimRng::new(0),
        };
        outcome_probabilities(&ctx)
    }

    fn side_strength_of(&self, team_id: u32) -> u8 {
        let format = self.cfg.format;
        let ids: Vec<u32> = self.selection_of(team_id).xi_ids.clone();
        let players: Vec<Player> = ids
            .iter()
            .filter_map(|id| self.players.get(id).cloned())
            .collect();
        let strength = players
            .iter()
            .map(|p| {
                p.skills
                    .bat_for(format)
                    .0
                    .max(p.skills.pace.0)
                    .max(p.skills.spin.0) as u32
            })
            .sum::<u32>()
            / players.len().max(1) as u32;
        strength.clamp(20, 99) as u8
    }

    /// Advance one delivery.
    pub fn bowl_ball(&mut self) -> Option<BallOutcome> {
        if self.phase == MatchPhase::Complete {
            return None;
        }
        let striker_id = self.striker_id?;
        let bowler_id = self.bowler_id?;
        let striker = self.player(striker_id);
        let bowler = self.player(bowler_id);
        let bowling_team = self
            .team_of(self.current_innings_ref().bowling_team_id)
            .clone();
        let innings = self.current_innings_ref().clone();
        let over = innings.balls / 6;
        let powerplay = over < self.cfg.format.powerplay_overs();
        let chasing = self.is_chasing();
        let rrr = self.required_rate();

        let outcome = {
            let mut rng = std::mem::replace(&mut self.rng, SimRng::new(0));
            let out = {
                let mut ctx = BallContext {
                    striker: &striker,
                    batter_id: striker_id,
                    bowler: &bowler,
                    fielding_team: &bowling_team,
                    innings: &innings,
                    format: self.cfg.format,
                    over,
                    powerplay,
                    chasing,
                    run_rate_needed: rrr,
                    minnow_boost: self.minnow_boost(),
                    input: &self.input,
                    rng: &mut rng,
                };
                resolve_ball(&mut ctx)
            };
            self.rng = rng;
            out
        };

        self.apply_outcome(&outcome);
        self.history.push(outcome.clone());
        Some(outcome)
    }

    fn apply_outcome(&mut self, outcome: &BallOutcome) {
        let striker_id = self.striker_id.unwrap_or(0);
        let bowler_id = self.bowler_id.unwrap_or(0);
        let is_legal = !matches!(
            outcome.extra,
            Some(ExtraKind::Wide) | Some(ExtraKind::NoBall)
        );

        {
            let innings = self.current_innings_mut();
            innings.runs += outcome.runs + outcome.extra_runs;
            if outcome.extra_runs > 0 {
                innings.extras += outcome.extra_runs;
            }
            if is_legal {
                innings.balls += 1;
            }
            if let Some(card) = innings
                .batting_cards
                .iter_mut()
                .find(|c| c.player_id == striker_id)
            {
                if is_legal {
                    card.balls += 1;
                }
                card.runs += outcome.runs;
                if outcome.is_boundary_four {
                    card.fours += 1;
                }
                if outcome.is_boundary_six {
                    card.sixes += 1;
                }
            }
            if let Some(card) = innings
                .bowling_cards
                .iter_mut()
                .find(|c| c.player_id == bowler_id)
            {
                card.runs += outcome.runs + outcome.extra_runs;
                if matches!(outcome.extra, Some(ExtraKind::Wide)) {
                    card.wides += 1;
                }
                if matches!(outcome.extra, Some(ExtraKind::NoBall)) {
                    card.no_balls += 1;
                }
                if is_legal {
                    card.balls += 1;
                }
                if outcome.dismissal.is_some() {
                    card.wickets += 1;
                }
            }
            if let Some(d) = &outcome.dismissal {
                let score = innings.runs;
                innings.fow.push(score);
                if let Some(card) = innings
                    .batting_cards
                    .iter_mut()
                    .find(|c| c.player_id == striker_id)
                {
                    card.out = Some(d.clone());
                }
                innings.wickets += 1;
            }
        }

        if outcome.strike_changes {
            std::mem::swap(&mut self.striker_id, &mut self.non_striker_id);
        }

        if outcome.dismissal.is_some() {
            self.bring_new_batter();
        }

        if is_legal && self.current_innings_ref().balls.is_multiple_of(6) {
            let runs = self.current_innings_ref().runs;
            let wickets = self.current_innings_ref().wickets;
            let summary = OverSummary {
                over: self.current_innings_ref().balls / 6,
                runs: runs - self.over_start_runs,
                wickets: wickets - self.over_start_wickets,
                bowler_id,
            };
            self.current_innings_mut().over_summaries.push(summary);
            self.over_start_runs = runs;
            self.over_start_wickets = wickets;
            self.last_bowler_id = Some(bowler_id);
            self.advance_clock();
            self.next_bowler();
        }

        self.check_innings_end();
    }

    fn bring_new_batter(&mut self) {
        let inn = self.current_innings_ref();
        let used: Vec<u32> = inn
            .batting_cards
            .iter()
            .filter(|c| c.balls > 0 || c.out.is_some())
            .map(|c| c.player_id)
            .collect();
        let next = inn
            .batting_cards
            .iter()
            .find(|c| !used.contains(&c.player_id))
            .map(|c| c.player_id);
        if let Some(id) = next {
            self.striker_id = Some(id);
        } else {
            self.striker_id = Some(0);
        }
    }

    fn advance_clock(&mut self) {
        self.clock.balls_today += 6;
        self.clock.balls_in_session = self.clock.balls_in_session.wrapping_add(6);
        if self.clock.balls_in_session >= 30 * 6 {
            self.clock.balls_in_session = 0;
            self.clock.session = if self.clock.session >= 3 {
                self.clock.balls_today = 0;
                self.clock.day += 1;
                1
            } else {
                self.clock.session + 1
            };
        }
    }

    fn is_chasing(&self) -> bool {
        let idx = self.current_innings;
        if self.cfg.format.max_overs().is_some() {
            idx == 1
        } else {
            idx == 3
        }
    }

    fn required_rate(&self) -> f64 {
        let inn = self.current_innings_ref();
        if !self.is_chasing() {
            return 0.0;
        }
        let target = self.innings[0].runs as f64 + 1.0;
        let runs_left = target - inn.runs as f64;
        let balls = match self.cfg.format.max_overs() {
            Some(max) => max as f64 * 6.0 - inn.balls as f64,
            None => return 0.0,
        };
        if balls <= 0.0 {
            return 0.0;
        }
        (runs_left / balls) * 6.0
    }

    fn check_innings_end(&mut self) {
        let inn = self.current_innings_ref();
        let all_out = inn.wickets >= self.wickets_per_innings();
        let declared = inn.is_declared;
        let overs_done = match self.cfg.format.max_overs() {
            Some(max) => inn.balls >= max * 6,
            None => false,
        };
        let time_up = self.cfg.format.max_overs().is_none() && self.balls_left_in_match() == 0;
        // A limited-overs chase ends the instant the target is passed, on the
        // winning hit itself. Without this the batting side bowls out the full
        // allocation and the result reports runs scored past the target (a
        // 98 chase finishing on 171 in 20 overs), which reads as a broken match.
        let target_reached = self.chase_target_reached();
        if all_out || declared || overs_done || time_up || target_reached {
            self.finish_innings();
        }
    }

    /// True when the side chasing has reached the target to win the match.
    ///
    /// Only applies to the second innings of a limited-overs match, and uses the
    /// full first-innings total (target = first innings runs + 1). Test cricket
    /// has no target, so it always returns false there.
    fn chase_target_reached(&self) -> bool {
        let Some(max_overs) = self.cfg.format.max_overs() else {
            return false; // Tests: no fixed target.
        };
        let _ = max_overs;
        if self.current_innings != 1 || self.innings.len() < 2 {
            return false;
        }
        let target = self.innings[0].runs + 1;
        self.innings[1].runs >= target
    }

    fn wickets_per_innings(&self) -> u8 {
        if self.cfg.format.max_overs().is_some() {
            10
        } else {
            TEST_WICKETS
        }
    }

    fn balls_left_in_match(&self) -> u32 {
        if self.cfg.format.max_overs().is_some() {
            return 0;
        }
        let total = OVERS_PER_DAY * self.cfg.days.max(1) as u32 * 6;
        let played: u32 = self.innings.iter().map(|i| i.balls).sum();
        total.saturating_sub(played)
    }

    /// Tests: auto-declare when a team has built a big lead late in the match,
    /// so CPU-vs-CPU games always reach a result.
    fn maybe_auto_declare(&mut self) {
        if self.cfg.format.max_overs().is_some() {
            return;
        }
        if self.current_innings == 0 || self.current_innings == 2 {
            let bat_team = self.current_batting_id();
            let first_runs = self.innings[0].runs as i32;
            let this_runs = self.current_innings_ref().runs as i32;
            let lead = if self.current_innings == 0 {
                this_runs
            } else {
                first_runs + this_runs
            };
            let bowling_team_runs = self
                .innings
                .iter()
                .filter(|i| i.batting_team_id != bat_team)
                .map(|i| i.runs as i32)
                .sum::<i32>();
            if lead >= bowling_team_runs + 250 && self.balls_left_in_match() < OVERS_PER_DAY * 6 {
                self.current_innings_mut().is_declared = true;
            }
            let _ = first_runs;
        }
    }

    fn finish_innings(&mut self) {
        match self.cfg.format {
            Format::Test => self.finish_innings_test(),
            _ => self.finish_innings_limited(),
        }
    }

    fn finish_innings_limited(&mut self) {
        if self.current_innings == 0 {
            let batting = self.opponent(self.current_batting_id());
            let inn = self.fresh_innings(batting);
            self.innings.push(inn);
            self.current_innings = 1;
            self.phase = MatchPhase::Innings2;
            self.open_innings();
        } else {
            self.decide_limited_result();
        }
    }

    fn finish_innings_test(&mut self) {
        if self.balls_left_in_match() == 0 && self.current_innings >= 2 {
            self.decide_test_result();
            return;
        }
        match self.current_innings {
            0 => {
                self.maybe_auto_declare();
                let batting = self.opponent(self.current_batting_id());
                self.push_innings_for(batting);
            }
            1 => {
                let lead = self.lead();
                self.follow_on_offered = lead >= 200;
                // CPU default: take the follow-on on a big lead late in the game.
                if lead >= 200 && self.balls_left_in_match() < OVERS_PER_DAY * 3 * 6 {
                    let batting = self.current_batting_id();
                    self.push_innings_for(batting);
                } else {
                    let batting = self.opponent(self.current_batting_id());
                    self.push_innings_for(batting);
                }
            }
            2 => {
                let second_team = self.innings[1].batting_team_id;
                let trailing = self.innings[0].batting_team_id;
                let ahead_by = self.total_runs(second_team) - self.total_runs(trailing);
                if ahead_by <= 0 {
                    // Team 1 already leads; Team 2 must bat again unless all out.
                    let batting = self.opponent(self.innings[2].batting_team_id);
                    self.push_innings_for(batting);
                } else {
                    self.decide_test_result();
                }
            }
            _ => self.decide_test_result(),
        }
    }

    fn push_innings_for(&mut self, batting_team_id: u32) {
        let inn = self.fresh_innings(batting_team_id);
        self.innings.push(inn);
        self.current_innings += 1;
        self.phase = match self.current_innings {
            0 => MatchPhase::Innings1,
            1 => MatchPhase::Innings2,
            2 => MatchPhase::Innings3,
            _ => MatchPhase::Innings4,
        };
        self.open_innings();
    }

    fn total_runs(&self, team_id: u32) -> i32 {
        self.innings
            .iter()
            .filter(|i| i.batting_team_id == team_id)
            .map(|i| i.runs as i32)
            .sum()
    }

    fn lead(&self) -> i32 {
        if self.innings.len() < 2 {
            return self.innings[0].runs as i32;
        }
        self.total_runs(self.innings[0].batting_team_id)
            - self.total_runs(self.innings[1].batting_team_id)
    }

    /// Decide a limited-overs match that finished level after the second innings.
    ///
    /// Real cricket resolves this with a one-over eliminator, not a tie: each side
    /// bats one over in turn and the higher total wins. Simulating a full one-over
    /// duel keeps the result a genuine contest instead of a coin flip, and matches
    /// how the original treats drawn white-ball games.
    fn decide_super_over(&mut self) {
        let first_team = self.innings[0].batting_team_id;
        let second_team = self.innings[1].batting_team_id;
        // Both names are resolved up front: `play_super_over_innings` borrows
        // `self` mutably, so nothing may hold a borrow of it across those calls.
        let first_name = self.team_name(first_team);
        let second_name = self.team_name(second_team);

        let first_runs = self.play_super_over_innings(first_team, second_team);
        let second_runs = self.play_super_over_innings(second_team, first_team);

        let (winner, winner_name) = if first_runs > second_runs {
            (Some(first_team), first_name)
        } else if second_runs > first_runs {
            (Some(second_team), second_name)
        } else {
            // Even a Super Over can finish level, in which case the law awards it
            // to the side scoring more in the Super Over — and if that is also
            // level, to the side batting first.
            (Some(first_team), first_name)
        };

        self.phase = MatchPhase::Complete;
        self.result = Some(MatchResult {
            winner_team_id: winner,
            win_margin: WinMargin::SuperOver,
            summary: format!("{winner_name} win the Super Over"),
        });
    }

    /// Display name for a team id in this match.
    fn team_name(&self, team_id: u32) -> String {
        if team_id == self.cfg.home_team.id {
            self.cfg.home_team.name.clone()
        } else {
            self.cfg.away_team.name.clone()
        }
    }

    /// Simulate one side's Super Over against the other; returns runs scored.
    fn play_super_over_innings(&mut self, batting: u32, bowling: u32) -> u32 {
        // Super Overs use the same balance point but with no powerplay, so the
        // white-ball skills still apply without a fielding restriction.
        let bat_xi = self.selection_of(batting).xi_ids.clone();
        let bowl_xi = self.selection_of(bowling).xi_ids.clone();

        // Best available batters and the best bowler, which is how a captain picks.
        let mut batters: Vec<u32> = bat_xi.clone();
        batters.sort_by_key(|id| std::cmp::Reverse(self.player(*id).skills.t20_bat.0));
        batters.truncate(3);
        if batters.is_empty() {
            batters = bat_xi.clone();
        }
        let Some(bowler_id) = bowl_xi
            .iter()
            .copied()
            .max_by_key(|id| self.player(*id).skills.bowl_for(Format::T20).0)
        else {
            return 0;
        };

        let bowler = self.player(bowler_id).clone();
        // Cloned: the context borrows the fielding team while `self.rng` is
        // mutably borrowed for the same call.
        let fielding_team = self.team_of(bowling).clone();
        // A Super Over is played with intent, but not recklessly.
        let input = MatchInput {
            batting_aggression: 60,
            field_preset: FieldPreset::Balanced,
            ..Default::default()
        };

        let mut runs = 0u32;
        let mut striker_id = batters[0];
        let mut wickets = 0u32;
        // Scratch innings: the ball engine reads runs and wickets from here, so the
        // Super Over runs through exactly the same code path as a normal innings.
        let scratch = self.fresh_innings(batting);

        for ball in 0..6u32 {
            let striker = self.player(striker_id).clone();
            let mut ctx = BallContext {
                striker: &striker,
                batter_id: striker_id,
                bowler: &bowler,
                fielding_team: &fielding_team,
                innings: &scratch,
                format: Format::T20,
                over: ball + 1,
                powerplay: false,
                chasing: false,
                run_rate_needed: 0.0,
                minnow_boost: 1.0,
                input: &input,
                rng: &mut self.rng,
            };
            let outcome = resolve_ball(&mut ctx);
            runs += outcome.runs + outcome.extra_runs;

            if outcome.dismissal.is_some() {
                wickets += 1;
                if wickets >= 3 {
                    break;
                }
                // Bring in the next batter; with only three picked, the incoming
                // batter is whoever is next in the XI.
                let next_in_xi = batters
                    .get(wickets as usize)
                    .copied()
                    .or_else(|| bat_xi.get(wickets as usize).copied())
                    .unwrap_or(striker_id);
                striker_id = next_in_xi;
            }
        }
        runs
    }

    fn decide_limited_result(&mut self) {
        let first = &self.innings[0];
        let second = &self.innings[1];
        let first_team = first.batting_team_id;
        let second_team = second.batting_team_id;
        let name = |id: u32| -> String {
            if id == self.cfg.home_team.id {
                self.cfg.home_team.name.clone()
            } else {
                self.cfg.away_team.name.clone()
            }
        };
        let (winner, margin, summary) = if second.runs > first.runs {
            let wickets_left = 10 - second.wickets;
            (
                Some(second_team),
                WinMargin::Wickets(wickets_left),
                format!("{} win by {} wickets", name(second_team), wickets_left),
            )
        } else if first.runs > second.runs {
            (
                Some(first_team),
                WinMargin::Runs(first.runs - second.runs),
                format!(
                    "{} win by {} runs",
                    name(first_team),
                    first.runs - second.runs
                ),
            )
        } else {
            // A limited-overs match cannot end tied: it is decided by a Super
            // Over. Reporting a bare "Match tied" here was both wrong cricket
            // and unreachable-looking output in a fixture list.
            self.decide_super_over();
            return;
        };
        self.phase = MatchPhase::Complete;
        self.result = Some(MatchResult {
            winner_team_id: winner,
            win_margin: margin,
            summary,
        });
    }

    fn decide_test_result(&mut self) {
        let team_a = self.innings[0].batting_team_id;
        let team_b = self.innings[1].batting_team_id;
        let a = self.total_runs(team_a);
        let b = self.total_runs(team_b);
        let name = |id: u32| -> String {
            if id == self.cfg.home_team.id {
                self.cfg.home_team.name.clone()
            } else {
                self.cfg.away_team.name.clone()
            }
        };
        let (winner, margin, summary) = if a > b {
            (
                Some(team_a),
                WinMargin::Runs((a - b) as u32),
                format!("{} win by {} runs", name(team_a), a - b),
            )
        } else if b > a {
            // Batting last: did they chase it down?
            let last = self.innings.last().unwrap();
            if last.batting_team_id == team_b && last.wickets < 10 {
                let wickets_left = 10 - last.wickets as i32;
                (
                    Some(team_b),
                    WinMargin::Wickets(wickets_left as u8),
                    format!("{} win by {} wickets", name(team_b), wickets_left),
                )
            } else if last.batting_team_id == team_b && last.runs >= a as u32 {
                (
                    Some(team_b),
                    WinMargin::Innings((b - a) as u32),
                    format!("{} win by an innings", name(team_b)),
                )
            } else {
                (None, WinMargin::Draw, "Match drawn".to_string())
            }
        } else {
            (None, WinMargin::Draw, "Match drawn".to_string())
        };
        self.phase = MatchPhase::Complete;
        self.result = Some(MatchResult {
            winner_team_id: winner,
            win_margin: margin,
            summary,
        });
    }

    /// Advance N deliveries at once (fast-forward). Starts the match if pre-match.
    pub fn bowl_many(&mut self, n: u32) {
        for _ in 0..n {
            if self.phase == MatchPhase::PreMatch {
                self.start();
            }
            if self.phase == MatchPhase::Complete {
                break;
            }
            self.bowl_ball();
        }
    }
}

// Small helper removed: `Innings::fow` is pushed directly in `apply_outcome`.
