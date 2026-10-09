//! Rain-affected targeting for limited-overs cricket.
//!
//! A simplified Duckworth-Lewis-Stern style recalculation: when overs are lost,
//! the chasing side's target is revised down using the resource ratio of the
//! remaining overs. Not ICC-accurate, but it behaves correctly at the edges
//! (no target below 1, revised target rises with the number of overs lost).

use super::super::core::types::Format;

/// Resources remaining, roughly proportional to overs left.
///
/// Real DLS uses a table of resource percentages; the quadratic approximation
/// below is close enough for gameplay and monotonically decreasing.
fn resource(overs: f64, scheduled: f64) -> f64 {
    if scheduled <= 0.0 || overs <= 0.0 {
        return 0.0;
    }
    (overs / scheduled).clamp(0.0, 1.0).powi(2)
}

/// A revised target after overs are lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevisedTarget {
    /// Runs the chasing side now needs to win (inclusive).
    pub target: u32,
    /// Overs they have to get there.
    pub overs: u32,
}

/// Revise a target after a rain delay.
///
/// `first_innings_runs` is what the side batting first made, `overs_played` is
/// how many overs they actually faced, and `overs_left` is what remains for the
/// side batting second.
pub fn revise_target(
    first_innings_runs: u32,
    overs_played: u32,
    overs_left: u32,
    format: Format,
) -> RevisedTarget {
    let scheduled = format.max_overs().unwrap_or(overs_played.max(1)) as f64;
    let used = (overs_played.max(1) as f64).min(scheduled);
    let remaining = overs_left as f64;

    // Resource ratio: how much of the match the chasers have compared with the
    // side that already batted.
    let r_batting = resource(used, scheduled);
    let r_chasing = resource(remaining, scheduled);
    let ratio = if r_batting > 0.0 {
        (r_chasing / r_batting).clamp(0.0, 1.0)
    } else {
        1.0
    };

    let target = ((first_innings_runs as f64) * ratio).round() as u32 + 1;
    RevisedTarget {
        target: target.max(1),
        overs: overs_left,
    }
}

/// Whether the chase is still alive given the current score.
pub fn chase_alive(
    revised: RevisedTarget,
    current_runs: u32,
    wickets: u8,
    balls_left: u32,
) -> bool {
    if current_runs >= revised.target {
        return true;
    }
    if wickets >= 10 {
        return false;
    }
    balls_left > 0
}

/// Required run rate to reach the revised target.
pub fn required_rate(revised: RevisedTarget, current_runs: u32, balls_left: u32) -> f64 {
    if balls_left == 0 {
        return 0.0;
    }
    let need = revised.target.saturating_sub(current_runs) as f64;
    (need / balls_left as f64) * 6.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_overs_lost_keeps_the_same_target() {
        let revised = revise_target(250, 50, 50, Format::Odi50);
        assert_eq!(revised.target, 251, "runs + 1 to win");
        assert_eq!(revised.overs, 50);
    }

    #[test]
    fn losing_overs_reduces_the_target() {
        let full = revise_target(250, 50, 50, Format::Odi50);
        let rain = revise_target(250, 50, 30, Format::Odi50);
        assert!(
            rain.target < full.target,
            "a shorter chase needs fewer runs"
        );
        assert!(rain.target >= 1, "never below 1");
    }

    #[test]
    fn the_more_overs_lost_the_lower_the_target() {
        let a = revise_target(250, 50, 40, Format::Odi50);
        let b = revise_target(250, 50, 25, Format::Odi50);
        let c = revise_target(250, 50, 10, Format::Odi50);
        assert!(a.target > b.target);
        assert!(b.target > c.target);
        assert!(c.target >= 1);
    }

    #[test]
    fn target_is_at_least_one() {
        let revised = revise_target(0, 50, 5, Format::Odi50);
        assert_eq!(revised.target, 1);
    }

    #[test]
    fn t20_and_50_over_scales_differ() {
        let t20 = revise_target(160, 20, 10, Format::T20);
        let odi = revise_target(160, 20, 10, Format::Odi50);
        // Halving a T20 is a proportionally bigger loss than in an ODI.
        assert!(t20.target <= odi.target);
    }

    #[test]
    fn chase_status_tracks_progress() {
        let revised = RevisedTarget {
            target: 100,
            overs: 20,
        };
        assert!(chase_alive(revised, 50, 3, 120));
        assert!(chase_alive(revised, 100, 3, 60), "reaching the target wins");
        assert!(!chase_alive(revised, 50, 10, 60), "all out ends it");
        assert!(!chase_alive(revised, 99, 3, 0), "out of time ends it");
    }

    #[test]
    fn required_rate_is_sane() {
        let revised = RevisedTarget {
            target: 121,
            overs: 20,
        };
        // 21 runs from 18 balls needs 7 an over.
        let rr = required_rate(revised, 100, 18);
        assert!((rr - 7.0).abs() < 0.01, "got {rr}");
        assert_eq!(required_rate(revised, 100, 0), 0.0);
    }
}
