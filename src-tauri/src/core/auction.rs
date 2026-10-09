//! Franchise auction: purse-based bidding for franchise leagues.
//!
//! Genericised leagues (no real trademarks — specs DR-1): a set number of
//! franchise teams bid on a pool of players with a purse, overseas cap and
//! right-to-match rules, the way an IPL/BPL-style auction works (FR-26).

use super::super::core::rng::SimRng;
use super::super::core::types::{Player, Rating, Team};
use serde::{Deserialize, Serialize};

/// One lot in the auction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuctionLot {
    pub player_id: u32,
    pub name: String,
    pub role: String,
    pub rating: Rating,
    pub base_price: u32,
    pub overseas: bool,
}

/// A bid placed by a franchise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bid {
    pub team_id: u32,
    pub amount: u32,
}

/// The auction book: purses, what's sold and to whom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuctionBook {
    pub lots: Vec<AuctionLot>,
    pub purses: Vec<(u32, i64)>,
    /// player_id -> (team_id, price)
    pub sold: Vec<(u32, u32, u32)>,
    pub current_lot: usize,
    pub overseas_cap: u8,
}

impl AuctionBook {
    /// Build the auction from the player pool.
    ///
    /// `franchise_ids` are the bidding teams; `home_nation_id` is the league's
    /// own country, so everyone else counts as an overseas player.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        players: &[Player],
        _teams: &[Team],
        franchise_ids: &[u32],
        home_nation_id: u32,
        purse: i64,
        seed: u64,
    ) -> Self {
        let mut rng = SimRng::new(seed);
        let franchise_players: Vec<&Player> = players
            .iter()
            .filter(|p| {
                p.team_id
                    .map(|id| franchise_ids.contains(&id))
                    .unwrap_or(false)
            })
            .collect();

        let mut lots: Vec<AuctionLot> = franchise_players
            .iter()
            .map(|p| {
                let rating = Rating(p.skills.bat.0.max(p.skills.pace.0).max(p.skills.spin.0));
                let base = 10_000 + (rating.0 as u32) * 2_000;
                AuctionLot {
                    player_id: p.id,
                    name: p.name.clone(),
                    role: format!("{:?}", p.role),
                    rating,
                    base_price: base,
                    overseas: p.nation_id != home_nation_id,
                }
            })
            .collect();

        // Shuffle the order so marquee players are spread through the auction.
        for i in (1..lots.len()).rev() {
            let j = rng.below((i + 1) as u32) as usize;
            lots.swap(i, j);
        }

        AuctionBook {
            lots,
            purses: franchise_ids.iter().map(|id| (*id, purse)).collect(),
            sold: Vec::new(),
            current_lot: 0,
            overseas_cap: 4,
        }
    }

    pub fn purse_of(&self, team_id: u32) -> i64 {
        self.purses
            .iter()
            .find(|(id, _)| *id == team_id)
            .map(|(_, p)| *p)
            .unwrap_or(0)
    }

    pub fn overseas_signed(&self, team_id: u32) -> u8 {
        self.sold
            .iter()
            .filter(|(_, t, _)| *t == team_id)
            .filter(|(p, _, _)| self.lots.iter().any(|l| l.player_id == *p && l.overseas))
            .count() as u8
    }

    /// Can this team legally bid this much for this lot?
    pub fn can_bid(&self, team_id: u32, lot: &AuctionLot, amount: u32) -> Result<(), String> {
        if amount < lot.base_price {
            return Err("below the base price".into());
        }
        if amount as i64 > self.purse_of(team_id) {
            return Err("not enough purse left".into());
        }
        if lot.overseas && self.overseas_signed(team_id) >= self.overseas_cap {
            return Err(format!("overseas cap of {} reached", self.overseas_cap));
        }
        Ok(())
    }

    /// Record a winning bid and move to the next lot.
    pub fn award(&mut self, lot: &AuctionLot, team_id: u32, amount: u32) -> Result<(), String> {
        self.can_bid(team_id, lot, amount)?;
        if let Some((_, purse)) = self.purses.iter_mut().find(|(id, _)| *id == team_id) {
            *purse -= amount as i64;
        }
        self.sold.push((lot.player_id, team_id, amount));
        self.current_lot += 1;
        Ok(())
    }

    /// Next unsold lot, if any.
    pub fn next_lot(&self) -> Option<&AuctionLot> {
        self.lots
            .iter()
            .find(|l| !self.sold.iter().any(|(id, _, _)| *id == l.player_id))
    }

    /// The winner of the auction: whoever spent the most on the best players.
    pub fn spending(&self, team_id: u32) -> i64 {
        self.sold
            .iter()
            .filter(|(_, t, _)| *t == team_id)
            .map(|(_, _, amount)| *amount as i64)
            .sum()
    }
}

/// Simulate rival franchises bidding on a lot.
///
/// Returns the highest rival bid, or None if nobody is interested. Rivals value
/// a player by rating with a little noise, and never exceed their purse.
pub fn rival_bid(
    book: &AuctionBook,
    lot: &AuctionLot,
    my_bid: u32,
    exclude_team: u32,
    seed: u64,
) -> Option<Bid> {
    let mut rng = SimRng::new(seed ^ lot.player_id as u64);
    let mut best: Option<Bid> = None;
    for (team_id, purse) in &book.purses {
        if *team_id == exclude_team {
            continue;
        }
        let valuation = lot.base_price as f64 * (0.9 + rng.next_f64() * 0.5);
        let cap = (*purse as f64).min(valuation);
        let offer = (cap as u32).min(my_bid + (8000.0 * rng.next_f64()) as u32);
        if offer > my_bid {
            let candidate = Bid {
                team_id: *team_id,
                amount: offer,
            };
            if best
                .as_ref()
                .map(|b| candidate.amount > b.amount)
                .unwrap_or(true)
            {
                best = Some(candidate);
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Nation, Role, Skills};

    fn player(id: u32, nation: u32, team: u32, rating: u8) -> Player {
        Player {
            id,
            name: format!("Player {id}"),
            nation_id: nation,
            team_id: Some(team),
            age: 26,
            role: Role::Batter,
            batting_style: "rhb".into(),
            bowling_style: None,
            skills: Skills {
                bat: Rating(rating),
                ..Default::default()
            },
            potential: Skills {
                bat: Rating(rating),
                ..Default::default()
            },
            form: 0,
            fitness: 100,
            morale: 70,
            experience: 10,
            injury_days: 0,
        }
    }

    fn setup() -> AuctionBook {
        let nations = vec![Nation {
            id: 1,
            name: "League Nation".into(),
            short_name: "LGA".into(),
            strength: 80,
            has_test_status: true,
        }];
        let _ = nations;
        let teams: Vec<Team> = (100..104)
            .map(|i| Team {
                id: i,
                name: format!("Franchise {i}"),
                nation_id: 1,
                home_ground_id: 1,
                budget: 2_000_000,
                reputation: Rating(60),
                squad_ids: vec![],
                is_national: false,
            })
            .collect();
        let franchise_ids: Vec<u32> = teams.iter().map(|t| t.id).collect();
        let players: Vec<Player> = (0..20)
            .map(|i| player(1000 + i, i % 3, 100 + (i % 4), 50 + i as u8))
            .collect();
        AuctionBook::new(&players, &teams, &franchise_ids, 1, 1_000_000, 42)
    }

    #[test]
    fn auction_builds_lots_with_purses() {
        let book = setup();
        assert_eq!(book.purses.len(), 4, "four franchises");
        assert_eq!(book.purse_of(100), 1_000_000);
        assert_eq!(book.lots.len(), 20, "every franchise player is a lot");
        // Players from other nations are marked overseas.
        assert!(book.lots.iter().any(|l| l.overseas));
    }

    #[test]
    fn base_price_is_the_minimum_bid() {
        let book = setup();
        let lot = book.lots[0].clone();
        assert!(book.can_bid(100, &lot, lot.base_price - 1).is_err());
        assert!(book.can_bid(100, &lot, lot.base_price).is_ok());
    }

    #[test]
    fn winning_a_lot_spends_the_purse() {
        let mut book = setup();
        let lot = book.lots[0].clone();
        book.award(&lot, 100, lot.base_price).expect("award");
        assert_eq!(book.purse_of(100), 1_000_000 - lot.base_price as i64);
        assert_eq!(book.sold.len(), 1);
        assert_eq!(book.next_lot().unwrap().player_id, book.lots[1].player_id);
    }

    #[test]
    fn overspending_is_refused() {
        let mut book = setup();
        let lot = book.lots[0].clone();
        let too_much = lot.base_price * 100;
        assert!(book.award(&lot, 100, too_much).is_err());
        assert!(book.sold.is_empty(), "no lot is awarded on a failed bid");
    }

    #[test]
    fn overseas_cap_is_enforced() {
        let mut book = setup();
        let overseas: Vec<AuctionLot> = book.lots.iter().filter(|l| l.overseas).cloned().collect();
        assert!(overseas.len() > 4, "need more overseas lots than the cap");
        for lot in overseas.iter().take(5) {
            let amount = lot.base_price;
            let allowed = book.can_bid(100, lot, amount);
            if book.overseas_signed(100) >= book.overseas_cap {
                assert!(allowed.is_err(), "cap must block the 5th overseas signing");
            } else {
                book.award(lot, 100, amount).expect("within cap");
            }
        }
        assert!(book.overseas_signed(100) <= book.overseas_cap);
    }

    #[test]
    fn rival_franchises_compete() {
        let book = setup();
        let lot = book.lots[0].clone();
        let my_bid = lot.base_price;
        // Over many seeds somebody should outbid the base price eventually.
        let challenged = (0..20u64).any(|seed| rival_bid(&book, &lot, my_bid, 100, seed).is_some());
        assert!(challenged, "rivals should sometimes bid");
    }

    #[test]
    fn auction_reports_spending() {
        let mut book = setup();
        let lot = book.lots[0].clone();
        book.award(&lot, 100, lot.base_price * 2).unwrap();
        assert_eq!(book.spending(100), (lot.base_price * 2) as i64);
        assert_eq!(book.spending(101), 0);
    }
}
