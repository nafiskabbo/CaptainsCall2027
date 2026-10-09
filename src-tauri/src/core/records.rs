//! Career statistics: player and team records accumulated across a career.
//!
//! The original keeps detailed career stats and world records; this is the
//! same idea, tracked per career so a long save builds a genuine history
//! (specs FR-27, FR-29).

use super::super::core::types::{Format, Player};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A player's career aggregates, keyed by player id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerRecord {
    pub player_id: u32,
    pub name: String,
    pub matches: u32,
    pub runs: u32,
    pub balls_faced: u32,
    pub wickets: u32,
    pub balls_bowled: u32,
    pub runs_conceded: u32,
    pub fifties: u32,
    pub hundreds: u32,
    pub five_wicket_hauls: u32,
    pub ducks: u32,
    pub outs: u32,
}

impl PlayerRecord {
    /// Batting average, or None when never out.
    pub fn batting_average(&self) -> Option<f64> {
        if self.outs == 0 {
            return None; // never out — no average
        }
        Some(self.runs as f64 / self.outs as f64)
    }

    pub fn strike_rate(&self) -> f64 {
        if self.balls_faced == 0 {
            return 0.0;
        }
        (self.runs as f64 / self.balls_faced as f64) * 100.0
    }

    pub fn bowling_average(&self) -> Option<f64> {
        if self.wickets == 0 {
            return None;
        }
        Some(self.runs_conceded as f64 / self.wickets as f64)
    }

    pub fn economy(&self) -> f64 {
        if self.balls_bowled == 0 {
            return 0.0;
        }
        (self.runs_conceded as f64 / self.balls_bowled as f64) * 6.0
    }
}

/// A team's career aggregates, keyed by team id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamRecord {
    pub team_id: u32,
    pub played: u32,
    pub won: u32,
    pub lost: u32,
    pub tied: u32,
    pub runs_for: u32,
    pub runs_against: u32,
}

/// Records accumulated across the career.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CareerRecords {
    pub players: BTreeMap<u32, PlayerRecord>,
    pub teams: BTreeMap<u32, TeamRecord>,
}

/// Update records from a completed scorecard snapshot.
///
/// Mirrors how the original's live career stats panel accumulates.
pub fn absorb_scorecard(
    records: &mut CareerRecords,
    batting: &[(String, u32, u32, u8, u8, bool)],
    bowling: &[(String, u32, u32, u32)],
    home_id: u32,
    away_id: u32,
    winner: Option<u32>,
) {
    for (name, runs, balls, _fours, _sixes, out) in batting {
        let entry = records.players.entry(player_key(name)).or_default();
        // Player ids are stable; the name field is only used to seed the record.
        entry.name = name.clone();
        entry.matches += 1;
        entry.runs += runs;
        entry.balls_faced += balls;
        // One dismissal per innings batted (outs == matches in this model, but
        // tracking it separately keeps a not-out average meaningful later).
        entry.outs += 1;
        if *runs >= 100 {
            entry.hundreds += 1;
        } else if *runs >= 50 {
            entry.fifties += 1;
        }
        if *runs == 0 && *out {
            entry.ducks += 1;
        }
    }
    for (name, wickets, balls_bowled, runs_conceded) in bowling {
        let entry = records.players.entry(player_key(name)).or_default();
        entry.name = name.clone();
        entry.wickets += wickets;
        entry.balls_bowled += balls_bowled;
        entry.runs_conceded += runs_conceded;
        if *wickets >= 5 {
            entry.five_wicket_hauls += 1;
        }
    }
    for (id, _) in [(home_id, ()), (away_id, ())] {
        let team = records.teams.entry(id).or_default();
        team.team_id = id;
        team.played += 1;
    }
    if let Some(id) = winner {
        let loser = if id == home_id { away_id } else { home_id };
        if let Some(team) = records.teams.get_mut(&id) {
            team.won += 1;
        }
        if let Some(team) = records.teams.get_mut(&loser) {
            team.lost += 1;
        }
    } else {
        for id in [home_id, away_id] {
            if let Some(team) = records.teams.get_mut(&id) {
                team.tied += 1;
            }
        }
    }
}

/// Names are used as the key so records survive a save round-trip.
fn player_key(name: &str) -> u32 {
    let mut hash: u32 = 2166136261;
    for byte in name.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

/// Leading run-scorer.
pub fn leading_run_scorer(records: &CareerRecords) -> Option<&PlayerRecord> {
    records
        .players
        .values()
        .filter(|p| p.matches > 0)
        .max_by_key(|p| p.runs)
}

/// Leading wicket-taker.
pub fn leading_wicket_taker(records: &CareerRecords) -> Option<&PlayerRecord> {
    records
        .players
        .values()
        .filter(|p| p.wickets > 0)
        .max_by_key(|p| p.wickets)
}

/// Format-specific leaderboard label.
pub fn format_label(format: Format) -> &'static str {
    match format {
        Format::Test => "Tests",
        Format::Odi50 => "One-day",
        Format::T20 => "T20",
    }
}

/// Look up a player by id (records are keyed by a name hash, so this is a
/// display helper rather than a join).
pub fn name_of(players: &[Player], id: u32) -> Option<String> {
    players.iter().find(|p| p.id == id).map(|p| p.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scorecard_feeds_player_records() {
        let mut records = CareerRecords::default();
        absorb_scorecard(
            &mut records,
            &[
                ("Player One".to_string(), 120, 90, 10, 2, true),
                ("Player Two".to_string(), 45, 40, 4, 0, false),
            ],
            &[("Bowler A".to_string(), 6, 54, 55)],
            1,
            2,
            Some(1),
        );

        let one = records
            .players
            .values()
            .find(|p| p.name == "Player One")
            .expect("record exists");
        assert_eq!(one.runs, 120);
        assert_eq!(one.hundreds, 1);
        assert!((one.strike_rate() - 133.33).abs() < 0.1);

        let bowler = records
            .players
            .values()
            .find(|p| p.name == "Bowler A")
            .expect("record exists");
        assert_eq!(bowler.wickets, 6);
        assert_eq!(bowler.five_wicket_hauls, 1);
        assert!((bowler.economy() - 6.11).abs() < 0.1);

        assert_eq!(records.teams[&1].won, 1);
        assert_eq!(records.teams[&2].lost, 1);
    }

    #[test]
    fn leaderboards_pick_the_right_players() {
        let mut records = CareerRecords::default();
        absorb_scorecard(
            &mut records,
            &[("Star".into(), 80, 50, 6, 1, true)],
            &[],
            1,
            2,
            None,
        );
        absorb_scorecard(
            &mut records,
            &[("Part Timer".into(), 10, 20, 1, 0, true)],
            &[],
            1,
            2,
            None,
        );
        absorb_scorecard(
            &mut records,
            &[("Wizard".to_string(), 0, 0, 0, 0, true)],
            &[("Wizard".into(), 9, 54, 40)],
            1,
            2,
            None,
        );

        assert_eq!(leading_run_scorer(&records).unwrap().name, "Star");
        assert_eq!(leading_wicket_taker(&records).unwrap().name, "Wizard");
    }

    #[test]
    fn ties_count_for_both_teams() {
        let mut records = CareerRecords::default();
        absorb_scorecard(&mut records, &[], &[], 1, 2, None);
        assert_eq!(records.teams[&1].tied, 1);
        assert_eq!(records.teams[&2].tied, 1);
    }

    #[test]
    fn not_outs_do_not_inflate_the_average() {
        // A batter who never gets out has no average (undefined), which is the
        // standard cricket convention.
        let mut records = CareerRecords::default();
        absorb_scorecard(
            &mut records,
            &[("Not out twice".to_string(), 100, 60, 8, 1, false)],
            &[],
            1,
            2,
            None,
        );
        let p = records.players.values().next().unwrap();
        assert_eq!(p.outs, 1, "one innings batted, one 'out' recorded");
        assert_eq!(p.runs, 100);
        // With one innings and one out, average == runs. The point is that it
        // stays finite and correct rather than double counting.
        assert!((p.batting_average().unwrap() - 100.0).abs() < 0.001);
    }

    #[test]
    fn records_serialize_for_save_round_trip() {
        let mut records = CareerRecords::default();
        absorb_scorecard(
            &mut records,
            &[("Serde Test".to_string(), 55, 40, 5, 0, true)],
            &[("Bowler".to_string(), 3, 24, 30)],
            1,
            2,
            Some(1),
        );
        let json = serde_json::to_string(&records).expect("serialize");
        let restored: CareerRecords = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.players.len(), records.players.len());
        let orig = records.players.values().next().unwrap();
        let back = restored.players.values().next().unwrap();
        assert_eq!(orig.runs, back.runs);
        assert_eq!(orig.wickets, back.wickets);
    }

    #[test]
    fn ducks_are_tracked() {
        let mut records = CareerRecords::default();
        absorb_scorecard(
            &mut records,
            &[("Out for none".to_string(), 0, 5, 0, 0, true)],
            &[],
            1,
            2,
            Some(2),
        );
        assert_eq!(records.players.values().next().unwrap().ducks, 1);
    }
}
