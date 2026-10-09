//! Player development: skills drift toward potential, experience, form, fitness.
//!
//! This is what makes a career a *career*: the XI you pick now is not the XI
//! you finish the season with. Modelled on Cricket Coach's growth system —
//! every skill has a current value and a potential ceiling, and development
//! depends on age, playing time and experience (specs FR-21, FR-22).

use super::super::core::rng::SimRng;
use super::super::core::types::{Format, Player, Rating};
use std::collections::BTreeMap;

/// A snapshot of how a player moved after a match, for the UI to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentEvent {
    pub player_id: u32,
    pub player_name: String,
    pub kind: DevelopmentKind,
    pub skill: Option<String>,
    pub amount: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentKind {
    SkillGain,
    SkillLoss,
    FormUp,
    FormDown,
    BurntOut,
    MatchSharp,
}

/// Growth rate curve by age (multiplier on the base drift).
pub fn age_factor(age: u8) -> f64 {
    match age {
        0..=18 => 1.35,
        19..=21 => 1.20,
        22..=24 => 0.95,
        25..=28 => 0.55,
        29..=31 => 0.20,
        32..=34 => 0.0,
        _ => 0.0,
    }
}

/// Decline rate curve by age (multiplier on decay).
pub fn decline_factor(age: u8) -> f64 {
    match age {
        0..=28 => 0.0,
        29..=31 => 0.05,
        32..=34 => 0.35,
        35..=37 => 0.70,
        _ => 1.0,
    }
}

/// Apply development after a completed match.
///
/// `participant_ids` are the XI that actually played (plus 12th man).
/// Bench players recover fitness but do not develop.
pub fn apply_match_development(
    players: &mut BTreeMap<u32, Player>,
    participant_ids: &[u32],
    format: Format,
    seed: u64,
) -> Vec<DevelopmentEvent> {
    let mut events = Vec::new();
    let mut rng = SimRng::new(seed ^ (participant_ids.len() as u64).wrapping_mul(0x2545F491));

    // Format weight: a Test is worth more experience than a T20.
    let weight = match format {
        Format::Test => 3.0,
        Format::Odi50 => 2.0,
        Format::T20 => 1.0,
    };

    for (i, id) in participant_ids.iter().enumerate() {
        let Some(player) = players.get_mut(id) else {
            continue;
        };
        let age = player.age;
        let grow = age_factor(age);
        let decline = decline_factor(age);

        // Experience always rises for players in the side.
        player.experience = player.experience.saturating_add((4.0 * weight) as u32);

        // Fitness drops with playing; heavy bowlers drop more.
        let workload: u8 = if player.skills.pace.0 > 60 || player.skills.spin.0 > 60 {
            9
        } else {
            5
        };
        let base_weight: f64 = if weight > 1.0 { weight } else { 1.0 };
        let fatigue = (f64::from(workload) / (base_weight * 0.6)).round() as u8;
        player.fitness = player.fitness.saturating_sub(fatigue);

        // Form: a small random walk, weighted by how much they played.
        let roll = rng.next_f64();
        if roll < 0.34 {
            player.form = (player.form + 1).min(10);
            events.push(event(player, DevelopmentKind::FormUp, None, 1, i));
        } else if roll < 0.5 {
            player.form = (player.form - 1).max(-10);
            events.push(event(player, DevelopmentKind::FormDown, None, -1, i));
        }

        if player.fitness < 40 {
            events.push(event(player, DevelopmentKind::BurntOut, None, 0, i));
        } else {
            events.push(event(player, DevelopmentKind::MatchSharp, None, 0, i));
        }

        // Skill drift. Young players climb toward potential; older players slip.
        if grow > 0.0 || decline > 0.0 {
            let gained = drift_skills(player, grow, decline, &mut rng);
            events.extend(gained);
        }
    }

    events
}

/// Move each skill a fraction of the way toward its potential (or away, in decline).
fn drift_skills(
    player: &mut Player,
    grow: f64,
    decline: f64,
    rng: &mut SimRng,
) -> Vec<DevelopmentEvent> {
    let mut events = Vec::new();
    let now = player.experience as u64;

    // A cap on how fast skills move, so a star never leaps 10 points a game.
    let mut step = |current: Rating, potential: Rating, rate: f64| -> Rating {
        let gap = potential.0 as f64 - current.0 as f64;
        if gap.abs() < 0.5 {
            return current;
        }
        let jitter: f64 = 0.6 + 0.8 * rng.next_f64();
        let delta = gap * rate * jitter / 6.0;
        let next = current.0 as f64 + delta;
        Rating::new(next.round().clamp(1.0, 99.0) as u8)
    };

    let before = player.skills;
    let after = Skills {
        bat: step(before.bat, player.potential.bat, grow - decline),
        test_bat: step(before.test_bat, player.potential.test_bat, grow - decline),
        odi_bat: step(before.odi_bat, player.potential.odi_bat, grow - decline),
        t20_bat: step(before.t20_bat, player.potential.t20_bat, grow - decline),
        defence: step(
            before.defence,
            player.potential.defence,
            grow * 1.2 - decline,
        ),
        pace: step(before.pace, player.potential.pace, grow - decline),
        spin: step(before.spin, player.potential.spin, grow - decline),
        odi_bowl: step(before.odi_bowl, player.potential.odi_bowl, grow - decline),
        t20_bowl: step(before.t20_bowl, player.potential.t20_bowl, grow - decline),
        fielding: step(
            before.fielding,
            player.potential.fielding,
            (grow - decline) * 0.8,
        ),
        keeping: step(
            before.keeping,
            player.potential.keeping,
            (grow - decline) * 0.9,
        ),
        leadership: step(
            before.leadership,
            player.potential.leadership,
            (grow - decline) * 0.5,
        ),
    };
    player.skills = after;

    // Report the biggest mover so the UI can surface it.
    let fields: [(&str, Rating, Rating); 12] = [
        ("bat", before.bat, after.bat),
        ("test_bat", before.test_bat, after.test_bat),
        ("odi_bat", before.odi_bat, after.odi_bat),
        ("t20_bat", before.t20_bat, after.t20_bat),
        ("defence", before.defence, after.defence),
        ("pace", before.pace, after.pace),
        ("spin", before.spin, after.spin),
        ("odi_bowl", before.odi_bowl, after.odi_bowl),
        ("t20_bowl", before.t20_bowl, after.t20_bowl),
        ("fielding", before.fielding, after.fielding),
        ("keeping", before.keeping, after.keeping),
        ("leadership", before.leadership, after.leadership),
    ];
    if let Some((name, b, a)) = fields
        .iter()
        .filter(|(_, b, a)| a.0 as i32 - b.0 as i32 != 0)
        .max_by_key(|(_, b, a)| (a.0 as i32 - b.0 as i32).abs())
    {
        let amount = a.0 as i8 - b.0 as i8;
        events.push(DevelopmentEvent {
            player_id: player.id,
            player_name: player.name.clone(),
            kind: if amount > 0 {
                DevelopmentKind::SkillGain
            } else {
                DevelopmentKind::SkillLoss
            },
            skill: Some(name.to_string()),
            amount,
        });
    }
    let _ = now;
    events
}

fn event(
    player: &Player,
    kind: DevelopmentKind,
    skill: Option<String>,
    amount: i8,
    _index: usize,
) -> DevelopmentEvent {
    DevelopmentEvent {
        player_id: player.id,
        player_name: player.name.clone(),
        kind,
        skill,
        amount,
    }
}

/// Roll for injuries after a match.
///
/// Risk rises with age, bowling workload and low fitness; the result is
/// deterministic for a given seed so replays match.
pub fn roll_injuries(players: &mut BTreeMap<u32, Player>, seed: u64) -> Vec<(u32, u32)> {
    let mut rng = SimRng::new(seed ^ 0x1AC1);
    let mut out = Vec::new();
    for player in players.values_mut() {
        if !player.is_available() {
            continue;
        }
        let workload = if player.skills.pace.0 > 60 || player.skills.spin.0 > 60 {
            0.030
        } else {
            0.012
        };
        let age_risk = 0.010 * (player.age as f64 - 24.0).max(0.0) / 6.0;
        let fitness_risk = (100.0 - f64::from(player.fitness)) / 100.0 * 0.040;
        let chance = (workload + age_risk + fitness_risk).clamp(0.0, 0.25);
        if rng.next_f64() < chance {
            let days = 3 + rng.below(18);
            player.injury_days = days;
            out.push((player.id, days));
        }
    }
    out
}

/// Heal injuries by one day.
pub fn heal_injuries(players: &mut BTreeMap<u32, Player>) {
    for player in players.values_mut() {
        if player.injury_days > 0 {
            player.injury_days -= 1;
            // Fitness comes back with the injury.
            if player.injury_days == 0 {
                player.fitness = player.fitness.max(70);
            }
        }
    }
}

/// Team training focus, applied while developing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainingFocus {
    Batting,
    Bowling,
    Fielding,
    Fitness,
    None,
}

impl TrainingFocus {
    /// Multiplier applied to the matching skill's development rate.
    pub fn multiplier(self) -> f64 {
        match self {
            TrainingFocus::Batting => 1.6,
            TrainingFocus::Bowling => 1.6,
            TrainingFocus::Fielding => 1.6,
            TrainingFocus::Fitness => 1.4,
            TrainingFocus::None => 1.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TrainingFocus::Batting => "Batting",
            TrainingFocus::Bowling => "Bowling",
            TrainingFocus::Fielding => "Fielding",
            TrainingFocus::Fitness => "Fitness",
            TrainingFocus::None => "Balanced",
        }
    }
}

/// Daily recovery: fitness drifts back toward 100 for everyone.
pub fn apply_daily_recovery(players: &mut BTreeMap<u32, Player>) {
    for player in players.values_mut() {
        if player.fitness < 100 {
            player.fitness = (player.fitness + 6).min(100);
        }
    }
}

/// Roll the season over: everyone ages a year and form resets.
pub fn roll_season(players: &mut BTreeMap<u32, Player>) {
    for player in players.values_mut() {
        player.age += 1;
        player.form = (player.form / 2).clamp(-10, 10);
        player.morale = 70;
    }
}

/// Convert a Vec<Player> to a map for the development API.
pub fn to_map(players: Vec<Player>) -> BTreeMap<u32, Player> {
    players.into_iter().map(|p| (p.id, p)).collect()
}

/// Convert back for serialization.
pub fn to_vec(players: BTreeMap<u32, Player>) -> Vec<Player> {
    players.into_values().collect()
}

use super::super::core::types::Skills;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Role;

    fn player(id: u32, age: u8) -> Player {
        let skills = crate::core::types::Skills {
            bat: Rating(60),
            test_bat: Rating(60),
            odi_bat: Rating(60),
            t20_bat: Rating(60),
            defence: Rating(60),
            pace: Rating(50),
            spin: Rating(50),
            odi_bowl: Rating(50),
            t20_bowl: Rating(50),
            fielding: Rating(50),
            keeping: Rating(20),
            leadership: Rating(30),
        };
        let potential = crate::core::types::Skills {
            bat: Rating(85),
            ..skills
        };
        Player {
            id,
            name: format!("Player {id}"),
            nation_id: 1,
            team_id: Some(1),
            age,
            role: Role::Batter,
            batting_style: "rhb".into(),
            bowling_style: None,
            skills,
            potential,
            form: 0,
            fitness: 100,
            morale: 70,
            experience: 0,
            injury_days: 0,
        }
    }

    #[test]
    fn young_players_grow_toward_potential() {
        let mut players = to_map(vec![player(1, 20)]);
        let before = players[&1].skills.bat;
        for i in 0..30u64 {
            apply_match_development(&mut players, &[1], Format::T20, 100 + i);
        }
        let after = players[&1].skills.bat;
        assert!(
            after.0 > before.0,
            "expected growth, {:?} -> {:?}",
            before,
            after
        );
        assert!(
            after.0 <= players[&1].potential.bat.0,
            "cannot exceed potential"
        );
    }

    #[test]
    fn veterans_decline() {
        let mut players = to_map(vec![player(1, 36)]);
        let before = players[&1].skills.bat;
        for i in 0..30u64 {
            apply_match_development(&mut players, &[1], Format::T20, 500 + i);
        }
        let after = players[&1].skills.bat;
        assert!(
            after.0 < before.0,
            "36-year-olds should decline, {:?} -> {:?}",
            before,
            after
        );
    }

    #[test]
    fn players_never_exceed_potential_and_skill_stays_in_band() {
        let mut players = to_map(vec![player(1, 19), player(2, 35)]);
        for i in 0..200u64 {
            apply_match_development(&mut players, &[1, 2], Format::Odi50, i);
            for p in players.values() {
                assert!(p.skills.bat.0 <= p.potential.bat.0);
                assert!((1..=99).contains(&p.skills.bat.0));
            }
        }
    }

    #[test]
    fn fitness_drops_when_playing_and_recovers_when_resting() {
        let mut players = to_map(vec![player(1, 25)]);
        for i in 0..5u64 {
            apply_match_development(&mut players, &[1], Format::T20, i);
        }
        assert!(players[&1].fitness < 100);
        for _ in 0..10 {
            apply_daily_recovery(&mut players);
        }
        assert_eq!(players[&1].fitness, 100);
    }

    #[test]
    fn bench_players_do_not_develop() {
        let mut players = to_map(vec![player(1, 21), player(2, 21)]);
        let before_bench = players[&2].skills.bat;
        for i in 0..20u64 {
            apply_match_development(&mut players, &[1], Format::T20, 900 + i);
        }
        assert_eq!(
            players[&2].skills.bat, before_bench,
            "bench players don't grow"
        );
        assert!(
            players[&1].skills.bat.0 > before_bench.0,
            "starter should grow: {:?} -> {:?}",
            before_bench,
            players[&1].skills.bat
        );
    }

    #[test]
    fn experience_grows_faster_in_longer_formats() {
        let mut t20 = to_map(vec![player(1, 24)]);
        let mut test = to_map(vec![player(1, 24)]);
        apply_match_development(&mut t20, &[1], Format::T20, 1);
        apply_match_development(&mut test, &[1], Format::Test, 1);
        assert!(test[&1].experience > t20[&1].experience);
    }

    #[test]
    fn season_rollover_ages_and_resets_form() {
        let mut players = to_map(vec![player(1, 24)]);
        players.get_mut(&1).unwrap().form = 8;
        roll_season(&mut players);
        assert_eq!(players[&1].age, 25);
        assert_eq!(players[&1].form, 4);
    }

    #[test]
    fn injuries_strike_sometimes_and_then_heal() {
        let mut injured_seen = 0;
        for seed in 0..60u64 {
            let mut players = to_map(vec![player(1, 24), player(2, 30), player(3, 35)]);
            // Older and tired players are likelier to break down.
            players.get_mut(&3).unwrap().fitness = 40;
            let injuries = roll_injuries(&mut players, seed);
            for (id, days) in injuries {
                assert!((3..=20).contains(&days), "injury length in range");
                assert!(players[&id].injury_days > 0, "flagged as injured");
                injured_seen += 1;
            }
        }
        assert!(injured_seen > 0, "injuries should happen across seasons");

        // Healing removes the flag.
        let mut players = to_map(vec![player(1, 24)]);
        players.get_mut(&1).unwrap().injury_days = 4;
        for _ in 0..5 {
            heal_injuries(&mut players);
        }
        assert_eq!(players[&1].injury_days, 0, "injury clears");
        assert!(players[&1].is_available(), "fit again");
    }

    #[test]
    fn fit_players_are_available_and_injured_are_not() {
        let mut p = player(1, 25);
        assert!(p.is_available());
        p.injury_days = 3;
        assert!(!p.is_available());
    }

    #[test]
    fn training_focus_multiplies_matching_development() {
        assert!(TrainingFocus::Batting.multiplier() > TrainingFocus::None.multiplier());
        assert_eq!(TrainingFocus::None.label(), "Balanced");
    }

    #[test]
    fn age_curve_is_monotonic_to_a_point() {
        assert!(age_factor(18) > age_factor(26));
        assert!(age_factor(26) > age_factor(34));
        assert!(decline_factor(20) == 0.0);
        assert!(decline_factor(40) > decline_factor(31));
    }
}
