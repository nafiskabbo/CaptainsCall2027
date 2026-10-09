//! Deterministic RNG + stats primitives.
//!
//! Everything that consumes randomness takes an explicit `&mut SimRng`, so a
//! seed fully determines a match (specs FR-13, NFR-7).

/// xoshiro256** — fast, high quality, easy to snapshot for state dumps.
#[derive(Debug, Clone)]
pub struct SimRng {
    state: [u64; 4],
}

impl SimRng {
    pub fn new(seed: u64) -> Self {
        // SplitMix64 to expand a single seed into 4 words.
        let mut z = seed;
        let mut next = || {
            z = z.wrapping_add(0x9E3779B97F4A7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
            x ^ (x >> 31)
        };
        SimRng {
            state: [next(), next(), next(), next()],
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.state;
        let result = (s[1].wrapping_mul(5)).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// Uniform in [0,1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform integer in [0, bound).
    pub fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        (self.next_f64() * bound as f64)
            .floor()
            .clamp(0.0, (bound - 1) as f64) as u32
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u32) as usize]
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }
}

/// A discrete distribution over outcomes.
/// `picked` uses cumulative weights so callers can layer conditional logic.
pub struct Distribution {
    pub weights: Vec<f64>,
}

impl Distribution {
    pub fn new(weights: Vec<f64>) -> Self {
        Distribution { weights }
    }

    pub fn pick(&self, rng: &mut SimRng) -> usize {
        let total: f64 = self.weights.iter().sum();
        let mut roll = rng.next_f64() * total;
        for (i, w) in self.weights.iter().enumerate() {
            roll -= *w;
            if roll <= 0.0 {
                return i;
            }
        }
        self.weights.len() - 1
    }
}

/// Clamp helper.
pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_stream() {
        let mut a = SimRng::new(42);
        let mut b = SimRng::new(42);
        let first: Vec<u64> = (0..16).map(|_| a.next_u64()).collect();
        let second: Vec<u64> = (0..16).map(|_| b.next_u64()).collect();
        assert_eq!(first, second);
    }

    #[test]
    fn f64_in_unit_range() {
        let mut rng = SimRng::new(7);
        for _ in 0..10_000 {
            let v = rng.next_f64();
            assert!((0.0..1.0).contains(&v));
        }
    }

    #[test]
    fn below_bounds() {
        let mut rng = SimRng::new(9);
        for _ in 0..1_000 {
            assert!(rng.below(6) < 6);
        }
    }
}
