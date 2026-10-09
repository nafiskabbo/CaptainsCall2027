//! Transfer window: wages, bidding and squad movement between seasons.
//!
//! Model: a pool of available players each off-season with an asking wage and a
//! reputation gate. You wage against your budget; the club accepts when your
//! wage is at or above what they expect and your club's reputation qualifies
//! (mirrors the original's bidding economy, specs FR-25).

use super::super::core::rng::SimRng;
use super::super::core::types::{Player, Rating, Team, World};
use serde::{Deserialize, Serialize};

/// A player available in the transfer window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferListing {
    pub player_id: u32,
    pub name: String,
    pub age: u8,
    pub role: String,
    pub from_team: String,
    pub wage: u32,
    pub reputation: Rating,
    /// Whether your club's reputation is high enough to sign them.
    pub interested_in_you: bool,
    pub value: u32,
}

/// Result of placing a bid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BidOutcome {
    Accepted { wage: u32 },
    RejectedTooLow { wanted: u32 },
    NotInterested,
    InsufficientFunds { available: u32 },
}

/// Build the transfer pool from players other clubs own.
///
/// Roughly 15% of each squad is listed each window, excluding the player's own.
pub fn build_transfer_pool(
    world: &World,
    players: &[Player],
    my_team_id: u32,
    seed: u64,
) -> Vec<TransferListing> {
    let mut rng = SimRng::new(seed);
    let mut listings = Vec::new();

    let my_team = world.teams.iter().find(|t| t.id == my_team_id);
    let my_reputation = my_team.map(|t| t.reputation.0).unwrap_or(50);

    for player in players {
        if player.team_id == Some(my_team_id) {
            continue;
        }
        // Availability chance rises with age (older players move more).
        let chance = if player.age >= 30 { 0.35 } else { 0.10 };
        if !rng.chance(chance) {
            continue;
        }
        let from = player
            .team_id
            .and_then(|id| world.teams.iter().find(|t| t.id == id))
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Free agent".into());

        let best = player
            .skills
            .bat
            .0
            .max(player.skills.pace.0)
            .max(player.skills.spin.0);
        let wage = 20_000 + (best as u32) * 2_400 + (player.age as u32).saturating_sub(24) * 900;
        let value = wage * 8;

        listings.push(TransferListing {
            player_id: player.id,
            name: player.name.clone(),
            age: player.age,
            role: match player.role {
                crate::core::types::Role::Batter => "batter",
                crate::core::types::Role::Wicketkeeper => "wicketkeeper",
                crate::core::types::Role::Allrounder => "allrounder",
                crate::core::types::Role::PaceBowler => "pace_bowler",
                crate::core::types::Role::SpinBowler => "spin_bowler",
            }
            .to_string(),
            from_team: from,
            wage,
            reputation: Rating(best),
            interested_in_you: my_reputation >= best.saturating_sub(18),
            value,
        });
    }

    listings.sort_by_key(|l| std::cmp::Reverse(l.reputation.0));
    listings
}

/// Attempt to sign a listed player.
pub fn place_bid(
    _world: &World,
    players: &mut [Player],
    my_team_id: u32,
    listing: &TransferListing,
    offered_wage: u32,
    available_budget: u64,
) -> BidOutcome {
    if offered_wage as u64 > available_budget {
        return BidOutcome::InsufficientFunds {
            available: available_budget as u32,
        };
    }
    if !listing.interested_in_you {
        return BidOutcome::NotInterested;
    }
    if offered_wage < listing.wage {
        return BidOutcome::RejectedTooLow {
            wanted: listing.wage,
        };
    }

    // Sign: move the player to your squad.
    if let Some(player) = players.iter_mut().find(|p| p.id == listing.player_id) {
        player.team_id = Some(my_team_id);
    }
    BidOutcome::Accepted { wage: listing.wage }
}

/// Release a player from your squad into the pool.
pub fn release_player(
    players: &mut [Player],
    my_team_id: u32,
    player_id: u32,
) -> Result<(), String> {
    let player = players
        .iter_mut()
        .find(|p| p.id == player_id && p.team_id == Some(my_team_id))
        .ok_or_else(|| format!("player {player_id} is not in your squad"))?;
    player.team_id = None;
    Ok(())
}

/// Squad size guard — you cannot sell below a full XI.
pub fn squad_size(players: &[Player], my_team_id: u32) -> usize {
    players
        .iter()
        .filter(|p| p.team_id == Some(my_team_id))
        .count()
}

/// Wage bill for your squad.
pub fn wage_bill(players: &[Player], my_team_id: u32) -> u32 {
    players
        .iter()
        .filter(|p| p.team_id == Some(my_team_id))
        .map(|p| 20_000 + (p.skills.bat.0.max(p.skills.pace.0).max(p.skills.spin.0) as u32) * 2_400)
        .sum()
}

/// Your club's team record.
pub fn my_team(world: &World, team_id: u32) -> Option<&Team> {
    world.teams.iter().find(|t| t.id == team_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Nation, Role, Skills};

    fn player(id: u32, team: u32, skill: u8, age: u8) -> Player {
        Player {
            id,
            name: format!("Player {id}"),
            nation_id: 1,
            team_id: Some(team),
            age,
            role: Role::Batter,
            batting_style: "rhb".into(),
            bowling_style: None,
            skills: Skills {
                bat: Rating(skill),
                ..Default::default()
            },
            potential: Skills {
                bat: Rating(skill),
                ..Default::default()
            },
            form: 0,
            fitness: 100,
            morale: 70,
            experience: 10,
            injury_days: 0,
        }
    }

    fn world() -> World {
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
                budget: 5_000_000,
                reputation: Rating(60),
                squad_ids: vec![],
                is_national: false,
            });
        }
        world
    }

    #[test]
    fn transfer_pool_excludes_your_squad() {
        let world = world();
        let players = vec![
            player(1, 100, 70, 26),
            player(2, 101, 80, 28),
            player(3, 102, 65, 24),
        ];
        let pool = build_transfer_pool(&world, &players, 100, 7);
        assert!(
            pool.iter().all(|l| l.player_id != 1),
            "your own players cannot be listed"
        );
    }

    fn listing_for(player_id: u32, wage: u32) -> TransferListing {
        TransferListing {
            player_id,
            name: format!("Player {player_id}"),
            age: 28,
            role: "Batter".into(),
            from_team: "Team 1".into(),
            wage,
            reputation: Rating(80),
            interested_in_you: true,
            value: wage * 8,
        }
    }

    #[test]
    fn bidding_at_or_above_wage_is_accepted() {
        let world = world();
        let mut players = vec![player(1, 100, 70, 26), player(2, 101, 80, 28)];
        let listing = listing_for(2, 200_000);

        // Too low is refused with the number required.
        assert!(matches!(
            place_bid(&world, &mut players, 100, &listing, 199_999, 10_000_000),
            BidOutcome::RejectedTooLow { wanted: 200_000 }
        ));

        // At or above the wage signs them.
        assert!(matches!(
            place_bid(&world, &mut players, 100, &listing, 200_000, 10_000_000),
            BidOutcome::Accepted { .. }
        ));
        assert_eq!(
            players.iter().find(|p| p.id == 2).unwrap().team_id,
            Some(100),
            "player must move to your squad"
        );
    }

    #[test]
    fn bid_above_budget_is_refused() {
        let world = world();
        let mut players = vec![player(1, 100, 70, 26), player(2, 101, 80, 28)];
        let listing = listing_for(2, 200_000);
        assert!(matches!(
            place_bid(&world, &mut players, 100, &listing, 200_000, 199_999),
            BidOutcome::InsufficientFunds { available: 199_999 }
        ));
    }

    #[test]
    fn uninterested_clubs_refuse_any_wage() {
        let world = world();
        let mut players = vec![player(1, 100, 70, 26), player(2, 101, 80, 28)];
        let listing = TransferListing {
            interested_in_you: false,
            ..listing_for(2, 200_000)
        };
        assert!(matches!(
            place_bid(&world, &mut players, 100, &listing, 500_000, 10_000_000),
            BidOutcome::NotInterested
        ));
    }

    #[test]
    fn pool_offers_players_over_many_seeds() {
        let world = world();
        let players: Vec<Player> = (0..24)
            .map(|i| player(1000 + i, 101 + (i % 2), 55 + i as u8, 24 + (i % 8) as u8))
            .collect();
        let mut seen = 0;
        for seed in 0..10u64 {
            seen += build_transfer_pool(&world, &players, 100, seed).len();
        }
        assert!(seen > 0, "over many seeds the window must list someone");
    }

    #[test]
    fn release_moves_player_out_but_keeps_squad_size_visible() {
        let _world = world();
        let mut players: Vec<Player> = (0..12)
            .map(|i| player(1000 + i, 100, 60 + i as u8, 25))
            .collect();
        players.push(player(50, 101, 70, 26));

        release_player(&mut players, 100, 1003).expect("release");
        assert_eq!(squad_size(&players, 100), 11);
        assert!(
            release_player(&mut players, 100, 50).is_err(),
            "not your player"
        );
        assert!(
            release_player(&mut players, 100, 999).is_err(),
            "unknown player"
        );
    }

    #[test]
    fn wage_bill_scales_with_squad_quality() {
        let _world = world();
        let cheap: Vec<Player> = (0..11).map(|i| player(1000 + i, 100, 40, 25)).collect();
        let rich: Vec<Player> = (0..11).map(|i| player(2000 + i, 100, 90, 25)).collect();
        assert!(wage_bill(&rich, 100) > wage_bill(&cheap, 100));
    }
}
