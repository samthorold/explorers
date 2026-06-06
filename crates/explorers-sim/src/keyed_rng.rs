//! Keyed-stateless per-agent-per-phase RNG derivation.
//!
//! The simulation's determinism contract is *same run seed -> same trajectory*.
//! Historically that rested on a single `ChaCha8Rng` stream threaded through the
//! whole tick: every stochastic phase drew off it in consumption order, so each
//! agent's outcome depended on which agents drew before it, and in what order.
//! Determinism therefore rested on every RNG-consuming phase iterating agents in
//! a stable, world-state-derived order *forever* — a fragile invariant (issue
//! #343 was a `HashSet` iteration that broke exactly this).
//!
//! This module replaces the shared stream with **keyed-stateless** derivation:
//! at each RNG-consuming site we derive a key from the agent's *stable identity*,
//! the current `tick`, and a `phase_tag`, and seed a fresh local `ChaCha8Rng`
//! from that key for that unit of work. No RNG state is stored on the agent or
//! threaded through the tick. Draws within one agent's phase come off its local
//! stream in deterministic code order; draws across agents never share a stream.
//! Iteration order is then no longer load-bearing.
//!
//! ## The frozen key layout (determinism contract)
//!
//! The key is a 64-bit hash of five `u64` fields, folded in a **fixed order**:
//!
//! ```text
//!   run_seed, id_lo, id_hi, tick, phase_tag
//! ```
//!
//! For a single-agent site (movement jitter, asexual reproduction) the identity
//! is one agent id: `id_lo = agent.id`, `id_hi = SINGLE_AGENT_SENTINEL`. For the
//! sexual site the identity is the **ordered pair**, made symmetric by sorting:
//! `id_lo = min(a.id, b.id)`, `id_hi = max(a.id, b.id)`. The pair key is thus a
//! pure function of `(min_id, max_id, tick)` regardless of which parent is `a`.
//!
//! The fold is a SplitMix64-style finaliser applied per field and mixed by xor —
//! chosen because it is small, dependency-free, and **stable across toolchains**
//! (unlike `std`'s `DefaultHasher`, whose output is explicitly not guaranteed
//! stable). The exact constants below are part of the frozen contract: changing
//! them re-seeds every stochastic outcome in every run. The inner PRNG primitive
//! is an implementation detail; the key layout above is the contract clients may
//! rely on.
//!
//! ## Inner primitive: SplitMix64, not ChaCha8 (performance, #376)
//!
//! Keyed-stateless derivation seeds a *fresh* generator at every RNG-consuming
//! site — per agent per phase, every tick. With `ChaCha8Rng` the per-site key
//! schedule (a 256-bit ChaCha state expansion) dominated the cost: the example9
//! sweep regressed ~44% wall-clock, past the accepted gate. The architecture is
//! the value here, not the cipher's cryptographic strength (the sim needs a
//! well-distributed stream, not unpredictability), so the inner primitive is
//! swapped to **SplitMix64** — a two-instruction seed and a handful of
//! instructions per draw — which brings the regression back inside the gate
//! while leaving the keyed-stateless per-agent-per-phase architecture and the
//! frozen key layout untouched. SplitMix64 passes the standard statistical test
//! batteries (it is the seeder the xoshiro authors specify); its short period
//! (2^64) is irrelevant because each local stream draws only a handful of values
//! before being discarded.

use rand::{RngCore, SeedableRng};

/// Distinguishes each RNG-consuming phase so that adding or removing a draw in
/// one phase cannot perturb another (a firewall). The discriminant values are
/// part of the frozen key layout — do not renumber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum PhaseTag {
    /// Movement jitter (random-walk angle + magnitude). Single-agent.
    Movement = 1,
    /// Asexual reproduction (propensity roll, fecundity, mutation, dispersal).
    /// Single-agent.
    AsexualReproduction = 2,
    /// Sexual reproduction (fecundity, seed-parent coin, crossover, mutation,
    /// dispersal). Keyed on the ordered pair.
    SexualReproduction = 3,
}

/// Sentinel occupying the high id slot for single-agent (non-pair) sites, so a
/// single-agent key can never collide with a pair key that happens to share the
/// low id. `u64::MAX` is not a reachable agent id (ids are compact from 0).
pub const SINGLE_AGENT_SENTINEL: u64 = u64::MAX;

/// SplitMix64 finaliser. A bijective avalanche mix of a single `u64`.
#[inline]
fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// The local keyed stream: SplitMix64. Cheap to seed (just store the state) and
/// cheap to draw from (advance by the golden-ratio increment, then finalise).
/// Implements `RngCore` + `SeedableRng`, so `rand::Rng` adapters and the
/// `rand_distr` Poisson/Normal samplers consume it unchanged.
#[derive(Debug, Clone)]
pub struct KeyedRng {
    state: u64,
}

impl KeyedRng {
    #[inline]
    fn next_u64_impl(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
}

impl SeedableRng for KeyedRng {
    type Seed = [u8; 8];

    #[inline]
    fn from_seed(seed: Self::Seed) -> Self {
        KeyedRng {
            state: u64::from_le_bytes(seed),
        }
    }

    #[inline]
    fn seed_from_u64(state: u64) -> Self {
        // The key fed in is already a SplitMix64-finalised, well-avalanched
        // value (see `derive_key`), so it is used directly as the initial state.
        KeyedRng { state }
    }
}

impl RngCore for KeyedRng {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        (self.next_u64_impl() >> 32) as u32
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.next_u64_impl()
    }

    #[inline]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut chunks = dest.chunks_exact_mut(8);
        for chunk in &mut chunks {
            chunk.copy_from_slice(&self.next_u64_impl().to_le_bytes());
        }
        let rem = chunks.into_remainder();
        if !rem.is_empty() {
            let bytes = self.next_u64_impl().to_le_bytes();
            rem.copy_from_slice(&bytes[..rem.len()]);
        }
    }
}

/// Fold the five contract fields into a single 64-bit key, in the frozen field
/// order `run_seed, id_lo, id_hi, tick, phase_tag`. Each field is run through the
/// finaliser after being combined with the running accumulator, so every field
/// position is order-sensitive and well mixed.
#[inline]
fn derive_key(run_seed: u64, id_lo: u64, id_hi: u64, tick: u64, tag: PhaseTag) -> u64 {
    let mut acc = splitmix64(run_seed);
    acc = splitmix64(acc ^ id_lo);
    acc = splitmix64(acc ^ id_hi);
    acc = splitmix64(acc ^ tick);
    acc = splitmix64(acc ^ (tag as u64));
    acc
}

/// Seed a fresh local keyed stream for a **single-agent** site, keyed on the
/// agent's stable `id`, the `tick`, and the `phase_tag`.
#[inline]
pub fn agent_rng(run_seed: u64, agent_id: u64, tick: u64, tag: PhaseTag) -> KeyedRng {
    let key = derive_key(run_seed, agent_id, SINGLE_AGENT_SENTINEL, tick, tag);
    KeyedRng::seed_from_u64(key)
}

/// Seed a fresh local keyed stream for the **sexual-pair** site, keyed on the
/// symmetric ordered pair `(min, max)` of the two parents' ids, the `tick`, and
/// the sexual `phase_tag`. The result is independent of which parent is passed
/// as `a` vs `b`.
#[inline]
pub fn pair_rng(run_seed: u64, a_id: u64, b_id: u64, tick: u64, tag: PhaseTag) -> KeyedRng {
    let lo = a_id.min(b_id);
    let hi = a_id.max(b_id);
    let key = derive_key(run_seed, lo, hi, tick, tag);
    KeyedRng::seed_from_u64(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    /// The sexual-pair stream is symmetric: swapping which parent is `a` and
    /// which is `b` yields the identical stream. This is what makes the
    /// seed-parent coin flip a pure function of `(min_id, max_id, tick)`.
    #[test]
    fn pair_rng_is_symmetric_in_parent_order() {
        let mut forward = pair_rng(42, 7, 13, 100, PhaseTag::SexualReproduction);
        let mut reversed = pair_rng(42, 13, 7, 100, PhaseTag::SexualReproduction);
        let a: [u32; 8] = std::array::from_fn(|_| forward.random());
        let b: [u32; 8] = std::array::from_fn(|_| reversed.random());
        assert_eq!(a, b, "pair RNG must be symmetric in parent order");
    }
}
