//! Small helpers for describing skills in news text.

use crate::core::types::{Player, Rating};

/// Human label for a skill key.
pub fn skill_label(key: &str) -> &'static str {
    match key {
        "bat" => "batting",
        "test_bat" => "red-ball batting",
        "odi_bat" => "one-day batting",
        "t20_bat" => "T20 batting",
        "defence" => "defence",
        "pace" => "pace bowling",
        "spin" => "spin bowling",
        "odi_bowl" => "one-day bowling",
        "t20_bowl" => "T20 bowling",
        "fielding" => "fielding",
        "keeping" => "wicketkeeping",
        "leadership" => "leadership",
        _ => "ability",
    }
}

/// The current value of a named skill.
pub fn skill_value(player: &Player, key: &str) -> u8 {
    let s = player.skills;
    match key {
        "bat" => s.bat.0,
        "test_bat" => s.test_bat.0,
        "odi_bat" => s.odi_bat.0,
        "t20_bat" => s.t20_bat.0,
        "defence" => s.defence.0,
        "pace" => s.pace.0,
        "spin" => s.spin.0,
        "odi_bowl" => s.odi_bowl.0,
        "t20_bowl" => s.t20_bowl.0,
        "fielding" => s.fielding.0,
        "keeping" => s.keeping.0,
        "leadership" => s.leadership.0,
        _ => 0,
    }
}

/// The potential value of a named skill, for "reaching potential" checks.
pub fn potential_value(player: &Player, key: &str) -> Rating {
    let s = player.potential;
    match key {
        "bat" => s.bat,
        "test_bat" => s.test_bat,
        "odi_bat" => s.odi_bat,
        "t20_bat" => s.t20_bat,
        "defence" => s.defence,
        "pace" => s.pace,
        "spin" => s.spin,
        "odi_bowl" => s.odi_bowl,
        "t20_bowl" => s.t20_bowl,
        "fielding" => s.fielding,
        "keeping" => s.keeping,
        "leadership" => s.leadership,
        _ => Rating(0),
    }
}
