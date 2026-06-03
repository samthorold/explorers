//! THROWAWAY VALIDATION SPIKE — NOT PART OF THE PRODUCTION STEPPER (issue #358).
//!
//! A disposable compartment-ODE + Jacobian analysis instrument that tests
//! whether Research Brief F's *operator reading* of the committed tick map `T`
//! is trustworthy on one minimal example, before any larger investment (the
//! moment-closure genesis objective, F's move 3, is gated on this verdict).
//! See `docs/research/F-mean-field-operator.md` (AC2, AC5 cross-check 2) and
//! `docs/system-design/viability.md` (the three-compartment flux balance).
//!
//! It does **not** touch, fork, or alter the committed scalar stepper, the RNG,
//! or the evaluation order. It reuses the committed trophic kernel verbatim — the
//! `base_trophic_efficiency · exp(−trophic_distance_decay · d)` form (`lib.rs:770`)
//! over the committed `TraitVector::distance` (`lib.rs:73`) — and otherwise reads
//! a scenario file only to recover its parameters and its two trait clusters.
//! Everything else is a *mean-field lumping* of committed fluxes into two scalar
//! compartments, clearly flagged where it approximates.
//!
//! ## What it models (Brief F, AC2 — each term lands on a real phase)
//!
//! A 2-compartment producer-biomass `P` ↔ consumer-biomass `C` discrete map —
//! the mean-field reduction of `state_{t+1} = T(state_t)` onto the
//! frozen↔oscillation coordinate (Brief F, AC1: this mode lives on a
//! producer↔consumer coupling, a 2×2 Jacobian, *not* a trait grid):
//!
//! ```text
//!   P' = P + r_P · P · (1 − P/K_P)  −  a · P · C
//!   C' = C + κ_C · γ · e · a · P · C  −  m · C
//! ```
//!
//! - `r_P · P · (1 − P/K_P)` — the **selection/growth diagonal** assembled from
//!   photosynthesise (`phase.rs:19`, income = flux × light share, the share
//!   denominator a density-dependent spatial convolution → the logistic ceiling
//!   `K_P`) + metabolise (`phase.rs:156`, the cost floor) + grow (`phase.rs:189`,
//!   κ-split × `growth_efficiency`). This is Brief F's `G(θ; M, A)` term.
//! - `a · P · C` — the **bilinear (quadratic) trophic term**, the nonlinear heart
//!   and *the seat of the Hopf*, read off resolve_drains (`phase.rs:386`): a
//!   per-target segment reduction with edge weight `demand · trophic_eff`. In the
//!   mean field this is a Holling type-I (mass-action) attack rate `a` — valid in
//!   the unsaturated regime (total demand ≤ available structure), the regime the
//!   proportional split (`phase.rs:489`) reduces to.
//! - `e = base_trophic_efficiency · exp(−trophic_distance_decay · d)` — the
//!   **committed kernel** (`lib.rs:770`), `d` = trait-space distance
//!   producer↔consumer. This is the coefficient we sweep.
//! - `m · C` — the **removal diagonal** from death (`phase.rs:687`): a consumer
//!   that cannot net-clear metabolism (`phase.rs:156`) declines.
//!
//! ## What it computes
//!
//! The interior (coexistence) fixed point, its 2×2 Jacobian, the leading
//! eigenvalue pair, and — sweeping `base_trophic_efficiency` — the value at which
//! the complex pair crosses the unit circle: a **Neimark–Sacker (discrete Hopf)**
//! bifurcation. Below it the interior point is a stable node/spiral ("frozen");
//! above it a limit cycle ("oscillation"). That crossing is F's falsifiable
//! prediction, validated against the headless `oscillation_strength` sweep.
//!
//! ## Authority boundary (Brief F, AC3) — load-bearing
//!
//! This instrument arbitrates **existence/stability only** (is there an interior
//! attractor; is it a node or a limit cycle). It must **never** be read for any
//! per-seed distributional claim (decomposer-guild emergence, coexistence-per-
//! seed). A gap between this predicted crossing and the observed one is a
//! *diagnostic signal*, never a defect in either lens.
//!
//! Usage:
//!   cargo run -p explorers-sim --bin hopf_prototype -- scenarios/example10_predator_prey_hopf.json

use explorers_sim::{TraitVector, WorldParameters, WorldRecipe};

/// Reference body mass for the mean-field reduction. The two compartments carry
/// *biomass* (structure); the per-agent fluxes (base metabolic rate, the
/// per-consumer drain demand) are charged against one reference body of this
/// size. The Neimark–Sacker crossing scales linearly with this constant — a
/// known degeneracy of any mean-field lumping, called out in the verdict.
const REFERENCE_BODY_MASS: f32 = 1.0;

/// Representative sustained-contact duration (ticks) at which the contact
/// Michaelis ramp `ct/(ct+K)` (`phase.rs:460`) is evaluated for the steady drain.
/// Co-located clusters hold contact, so the ramp sits near its asymptote; this is
/// the *named term* the verdict implicates if the observed crossing is shifted.
const SUSTAINED_CONTACT_TICKS: f32 = 10.0;

/// The lumped coefficients of the 2-compartment map, each derived from committed
/// fluxes (citations on the fields). Everything here is a mean-field reading of
/// an existing phase; nothing introduces new physics.
#[derive(Debug, Clone)]
struct Compartments {
    /// Producer intrinsic per-tick growth rate: κ_P · growth_efficiency ·
    /// (flux − maintenance), the low-density selection diagonal (photosynthesise
    /// + metabolise + grow).
    r_p: f32,
    /// Producer light-saturated carrying capacity: flux ÷ per-biomass maintenance
    /// — the biomass at which the density-dependent light share (photosynthesise)
    /// just meets the metabolic floor.
    k_p: f32,
    /// Mass-action attack rate: steady per-consumer demand
    /// `eff_heterotrophy · ct/(ct+K)` (resolve_drains + the contact Michaelis
    /// `phase.rs:460`), per unit reference biomass.
    a: f32,
    /// Consumer per-tick removal rate: its metabolic floor (metabolise) it must
    /// out-ingest or decline toward the death threshold (check_death_thresholds).
    m: f32,
    /// Consumer somatic allocation κ_C (grow phase, kappa split).
    kappa_c: f32,
    /// Energy→structure conversion `growth_efficiency` (grow phase).
    gamma: f32,
    /// Distance-independent part of the trophic kernel exponent: `exp(−decay·d)`.
    e_dist: f32,
    /// Trait-space distance producer↔consumer (committed `TraitVector::distance`).
    d: f32,
}

impl Compartments {
    /// Per-tick maintenance cost of one reference body with the given traits —
    /// the metabolise phase (`phase.rs:156`) charged on a unit body.
    fn maintenance(traits: &TraitVector, p: &WorldParameters) -> f32 {
        let exp = p.maintenance_cost_exponent;
        p.base_metabolic_rate
            + traits.photosynthetic_absorption.max(0.0).powf(exp) * p.photo_maintenance_cost
            + traits.heterotrophy.max(0.0).powf(exp) * p.heterotrophy_maintenance_cost
            + traits.mobility.max(0.0).powf(exp) * p.mobility_maintenance_cost
            + REFERENCE_BODY_MASS * p.structure_maintenance_coefficient
    }

    fn derive(producer: &TraitVector, consumer: &TraitVector, p: &WorldParameters) -> Self {
        let kappa_p = producer.kappa.clamp(0.0, 1.0);
        let kappa_c = consumer.kappa.clamp(0.0, 1.0);
        let gamma = p.growth_efficiency;

        let b_p = Self::maintenance(producer, p);
        let b_c = Self::maintenance(consumer, p);
        let flux = p.solar_flux_magnitude;

        // Producer selection diagonal: a lone producer captures the full flux
        // (light share → 1, photosynthesise) and nets (flux − maintenance), of
        // which κ_P·γ becomes structure (grow). As biomass crowds, the light
        // share falls ∝ 1/P, so total captured flux saturates at `flux`; the
        // logistic ceiling K_P is the biomass where per-unit income = maintenance.
        let r_p = kappa_p * gamma * (flux - b_p).max(0.0) / REFERENCE_BODY_MASS;
        let k_p = if b_p > 0.0 { flux / b_p } else { f32::INFINITY };

        // Bilinear trophic term: steady per-consumer demand = eff_heterotrophy ×
        // contact-Michaelis (resolve_drains + phase.rs:460), as a mass-action
        // attack rate per unit reference biomass.
        let eff_het = consumer.heterotrophy.max(0.0);
        let k_half = p.consumption_contact_half_saturation;
        let michaelis = if k_half > 0.0 {
            SUSTAINED_CONTACT_TICKS / (SUSTAINED_CONTACT_TICKS + k_half)
        } else {
            1.0
        };
        let a = eff_het * michaelis / REFERENCE_BODY_MASS;

        // Consumer removal: the metabolic floor it must out-ingest (metabolise →
        // death). Without prey it declines at this per-tick fractional rate.
        let m = b_c / REFERENCE_BODY_MASS;

        let d = producer.distance(consumer);
        let e_dist = (-p.trophic_distance_decay * d).exp();

        Compartments {
            r_p,
            k_p,
            a,
            m,
            kappa_c,
            gamma,
            e_dist,
            d,
        }
    }

    /// Trophic transfer efficiency `e` at a given swept `base_trophic_efficiency`.
    fn e(&self, base: f32) -> f32 {
        base * self.e_dist
    }

    /// Consumer biomass-conversion coefficient β = κ_C · γ · e · a.
    fn beta(&self, base: f32) -> f32 {
        self.kappa_c * self.gamma * self.e(base) * self.a
    }

    /// Interior (coexistence) fixed point (P*, C*) at a given base efficiency, or
    /// `None` if no positive interior point exists (consumer cannot persist:
    /// P* ≥ K_P, so prey can never support it).
    fn interior_fixed_point(&self, base: f32) -> Option<(f32, f32)> {
        let beta = self.beta(base);
        if beta <= 0.0 || self.a <= 0.0 {
            return None;
        }
        let p_star = self.m / beta; // from C-nullcline: β·P* = m
        if !(p_star > 0.0 && p_star < self.k_p) {
            return None;
        }
        let c_star = (self.r_p / self.a) * (1.0 - p_star / self.k_p);
        if c_star <= 0.0 {
            return None;
        }
        Some((p_star, c_star))
    }

    /// The 2×2 Jacobian of the discrete map at the interior fixed point.
    /// Rows/cols ordered (P, C). Returned as [[j11, j12], [j21, j22]].
    fn jacobian(&self, base: f32) -> Option<[[f32; 2]; 2]> {
        let (p_star, c_star) = self.interior_fixed_point(base)?;
        let beta = self.beta(base);
        let j11 = 1.0 + self.r_p - 2.0 * self.r_p * p_star / self.k_p - self.a * c_star;
        let j12 = -self.a * p_star;
        let j21 = beta * c_star;
        let j22 = 1.0 + beta * p_star - self.m;
        Some([[j11, j12], [j21, j22]])
    }
}

/// The spectral reading of a 2×2 Jacobian: eigenvalue pair and whether it is a
/// complex-conjugate pair (a spiral, the Hopf precondition) or two reals.
#[derive(Debug, Clone, Copy)]
struct Spectrum {
    /// |λ| of the leading eigenvalue (the spectral radius).
    modulus: f32,
    /// True when the eigenvalues are a complex-conjugate pair (spiral) — the
    /// precondition for a Neimark–Sacker crossing rather than a real-eigenvalue
    /// (period-doubling / fold) crossing.
    complex: bool,
}

fn spectrum(j: &[[f32; 2]; 2]) -> Spectrum {
    let trace = j[0][0] + j[1][1];
    let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
    let disc = trace * trace - 4.0 * det;
    if disc < 0.0 {
        // Complex pair λ = (trace ± i√−disc)/2, modulus = √det.
        Spectrum {
            modulus: det.max(0.0).sqrt(),
            complex: true,
        }
    } else {
        let root = disc.sqrt();
        let l1 = (trace + root).abs() / 2.0;
        let l2 = (trace - root).abs() / 2.0;
        Spectrum {
            modulus: l1.max(l2),
            complex: false,
        }
    }
}

/// Closed-form Neimark–Sacker crossing in `base_trophic_efficiency`.
///
/// Algebra (verified in the unit test): at the interior FP the Jacobian has
/// `det Δ = 1 − r_P·P*/K_P + m·r_P·(1 − P*/K_P)` and `Δ = 1` (the |λ|=1 crossing
/// for a complex pair) reduces — `r_P` cancels — to `P*/K_P = m/(1+m)`. With
/// `P* = m/(κ_C·γ·e·a)` and `e = base·exp(−decay·d)` this gives a closed form for
/// `base*`. Returns `None` if the crossing falls outside the physical efficiency
/// range (0, 1] — i.e. F predicts the example is unconditionally stable (frozen).
fn analytic_crossing(c: &Compartments) -> Option<f32> {
    // base* = (1+m) / (κ_C·γ·a·K_P·e_dist)
    let denom = c.kappa_c * c.gamma * c.a * c.k_p * c.e_dist;
    if denom <= 0.0 {
        return None;
    }
    let base = (1.0 + c.m) / denom;
    Some(base)
}

/// Pick the producer cluster (highest mean photosynthetic_absorption) and the
/// consumer cluster (highest mean heterotrophy) from a scenario's agent roster,
/// returning the *cluster-mean* trait vector of each. The minimal predator-prey
/// scenario has exactly two clusters; this just averages each.
fn extract_clusters(agents: &[explorers_sim::AgentSpec]) -> Option<(TraitVector, TraitVector)> {
    let mut producer: Option<TraitVector> = None;
    let mut consumer: Option<TraitVector> = None;
    // Two clusters distinguished by which trophic trait dominates.
    let mut prod_sum = zero();
    let mut prod_n = 0u32;
    let mut cons_sum = zero();
    let mut cons_n = 0u32;
    for a in agents {
        if a.traits.photosynthetic_absorption >= a.traits.heterotrophy {
            accumulate(&mut prod_sum, &a.traits);
            prod_n += 1;
        } else {
            accumulate(&mut cons_sum, &a.traits);
            cons_n += 1;
        }
    }
    if prod_n > 0 {
        producer = Some(scale(&prod_sum, 1.0 / prod_n as f32));
    }
    if cons_n > 0 {
        consumer = Some(scale(&cons_sum, 1.0 / cons_n as f32));
    }
    Some((producer?, consumer?))
}

fn zero() -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy: 0.0,
        mobility: 0.0,
        kappa: 0.0,
        fecundity: 0.0,
        asexual_propensity: 0.0,
        dispersal: 0.0,
    }
}

fn accumulate(acc: &mut TraitVector, t: &TraitVector) {
    for i in 0..TraitVector::NUM_DIMS {
        acc.set(i, acc.get(i) + t.get(i));
    }
}

fn scale(t: &TraitVector, s: f32) -> TraitVector {
    let mut out = *t;
    for i in 0..TraitVector::NUM_DIMS {
        out.set(i, t.get(i) * s);
    }
    out
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!(
            "usage: hopf_prototype <scenario.json>\n\
             (throwaway analysis spike, issue #358 — not part of the stepper)"
        );
        std::process::exit(1);
    });

    let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let recipe: WorldRecipe =
        serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {path}: {e}"));
    let agents = recipe
        .agents
        .as_ref()
        .expect("scenario must carry an explicit agent roster");
    let (producer, consumer) =
        extract_clusters(agents).expect("scenario must have a producer and a consumer cluster");
    let params = &recipe.parameters;

    let c = Compartments::derive(&producer, &consumer, params);

    println!("# Hopf prototype — throwaway compartment-ODE + Jacobian spike (issue #358)");
    println!("# NOT PART OF THE PRODUCTION STEPPER. Mean-field reading of the committed map T.");
    println!("# scenario: {path}");
    println!();
    println!("## Derived compartment coefficients (from committed fluxes)");
    println!(
        "  producer cluster mean: photo={:.3} hetero={:.3} kappa={:.3}",
        producer.photosynthetic_absorption, producer.heterotrophy, producer.kappa
    );
    println!(
        "  consumer cluster mean: photo={:.3} hetero={:.3} kappa={:.3}",
        consumer.photosynthetic_absorption, consumer.heterotrophy, consumer.kappa
    );
    println!("  trait-space distance d              = {:.4}", c.d);
    println!("  exp(-decay*d) (kernel, lib.rs:770)  = {:.4}", c.e_dist);
    println!("  r_P  (producer intrinsic rate)      = {:.4}", c.r_p);
    println!("  K_P  (light-saturated capacity)     = {:.4}", c.k_p);
    println!("  a    (mass-action attack rate)      = {:.4}", c.a);
    println!("  m    (consumer removal rate)        = {:.4}", c.m);
    println!(
        "  kappa_C, gamma                      = {:.3}, {:.3}",
        c.kappa_c, c.gamma
    );
    println!();

    // --- Sweep base_trophic_efficiency across the physical range ---
    println!("## Sweep: leading eigenvalue pair vs base_trophic_efficiency");
    println!(
        "  {:>6}  {:>8}  {:>8}  {:>8}  {:>8}  {:>7}",
        "base", "P*", "C*", "|lambda|", "spiral?", "regime"
    );
    let mut prev: Option<(f32, f32, bool)> = None; // (base, modulus, complex)
    let mut swept_crossing: Option<f32> = None;
    let steps = 96;
    for i in 1..=steps {
        let base = i as f32 / steps as f32; // 0 < base <= 1.0 (physical range)
        let (p_star, c_star, modulus, complex, regime) = match c.jacobian(base) {
            Some(j) => {
                let (ps, cs) = c.interior_fixed_point(base).unwrap();
                let s = spectrum(&j);
                let regime = if s.modulus < 1.0 {
                    if s.complex {
                        "stable-spiral"
                    } else {
                        "stable-node"
                    }
                } else if s.complex {
                    "LIMIT-CYCLE"
                } else {
                    "unstable-node"
                };
                (ps, cs, s.modulus, s.complex, regime)
            }
            None => (f32::NAN, f32::NAN, f32::NAN, false, "no-interior-FP"),
        };

        // Detect the |lambda| = 1 crossing with a complex pair (Neimark–Sacker).
        if let Some((pb, pm, pc)) = prev
            && pm.is_finite()
            && modulus.is_finite()
            && pm < 1.0
            && modulus >= 1.0
            && (pc || complex)
            && swept_crossing.is_none()
        {
            // Linear interpolation of the crossing base.
            let t = (1.0 - pm) / (modulus - pm);
            swept_crossing = Some(pb + t * (base - pb));
        }
        prev = Some((base, modulus, complex));

        // Print a thinned table (every 4th row + near interesting points).
        if i % 4 == 0 || regime == "LIMIT-CYCLE" {
            println!(
                "  {:>6.3}  {:>8.3}  {:>8.3}  {:>8.4}  {:>8}  {:>7}",
                base,
                p_star,
                c_star,
                modulus,
                if complex { "yes" } else { "no" },
                regime
            );
        }
    }
    println!();

    // --- The prediction ---
    let analytic = analytic_crossing(&c);
    println!("## F's predicted Neimark–Sacker (discrete Hopf) crossing");
    match analytic {
        Some(b) if b <= 1.0 => {
            println!(
                "  analytic base* = {:.4}  (closed form: (1+m)/(kappa_C*gamma*a*K_P*e_dist))",
                b
            );
        }
        Some(b) => {
            println!(
                "  analytic base* = {:.4}  — OUTSIDE the physical range (0,1].",
                b
            );
            println!(
                "  => F predicts this example is UNCONDITIONALLY STABLE (frozen) for any\n     \
                 physical trophic efficiency: no freeze->oscillate crossing exists here."
            );
        }
        None => println!("  no interior fixed point at any efficiency — consumer cannot persist."),
    }
    match swept_crossing {
        Some(b) => println!(
            "  swept-detected crossing = {:.4}  (|lambda| passes 1, complex pair)",
            b
        ),
        None => {
            println!("  swept-detected crossing = none in (0,1] (no |lambda|=1 spiral crossing)")
        }
    }
    println!();
    println!("## Read this against the headless oscillation_strength sweep (eval_scenarios).");
    println!("   Below base* -> stable (low oscillation_strength); above -> limit cycle (high).");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> WorldParameters {
        // A self-contained parameter set for the unit test (values mirror the
        // predator-prey scenario's regime; the test checks the *math*, not a
        // particular world).
        let json = r#"{
            "solar_flux_magnitude": 10.0,
            "base_trophic_efficiency": 0.5,
            "trophic_distance_decay": 1.0,
            "reproduction_efficiency": 0.7,
            "base_metabolic_rate": 0.1,
            "movement_cost_coefficient": 0.05,
            "reproduction_energy_threshold": 15.0,
            "mutation_rate": 0.1,
            "mutation_magnitude": 0.05,
            "contact_range_coefficient": 5.0,
            "world_extent": 100.0,
            "initial_population_size": 0,
            "light_competition_radius": 8.0,
            "photo_maintenance_cost": 0.01,
            "heterotrophy_maintenance_cost": 0.01,
            "initial_nutrient_pool": 5000.0,
            "growth_efficiency": 0.3
        }"#;
        serde_json::from_str(json).unwrap()
    }

    fn producer() -> TraitVector {
        let mut t = zero();
        t.photosynthetic_absorption = 0.6;
        t.kappa = 0.5;
        t
    }

    fn consumer() -> TraitVector {
        let mut t = zero();
        t.heterotrophy = 0.6;
        t.kappa = 0.5;
        t
    }

    /// The analytic crossing must coincide with where the numerically-built
    /// Jacobian's spectral radius actually passes through 1 — the core claim of
    /// the spike (the closed form is the right reading of the operator's spectrum).
    #[test]
    fn analytic_crossing_matches_numerical_spectral_radius() {
        let c = Compartments::derive(&producer(), &consumer(), &params());
        let base = analytic_crossing(&c).expect("crossing should exist in range");
        assert!(
            base > 0.0 && base <= 1.0,
            "crossing {base} should be physical"
        );

        // At the analytic crossing the spectral radius must be ~1 with a complex
        // pair (Neimark–Sacker, not a real-eigenvalue crossing).
        let j = c.jacobian(base).expect("interior FP at the crossing");
        let s = spectrum(&j);
        assert!(
            (s.modulus - 1.0).abs() < 1e-3,
            "|lambda| at base* = {} should be 1, got {}",
            base,
            s.modulus
        );
        assert!(
            s.complex,
            "the crossing must be a complex pair (Hopf), not real"
        );
    }

    /// Just below the crossing the interior point is stable; just above it is a
    /// limit cycle. (The freeze->oscillate transition the headless run validates.)
    #[test]
    fn stability_flips_across_the_crossing() {
        let c = Compartments::derive(&producer(), &consumer(), &params());
        let base = analytic_crossing(&c).unwrap();
        let below = spectrum(&c.jacobian(base * 0.9).unwrap());
        let above = spectrum(&c.jacobian((base * 1.1).min(1.0)).unwrap());
        assert!(below.modulus < 1.0, "below crossing should be stable");
        // Only assert the above-case when 1.1*base is still in range.
        if base * 1.1 <= 1.0 {
            assert!(
                above.modulus > 1.0,
                "above crossing should be a limit cycle"
            );
        }
    }
}
