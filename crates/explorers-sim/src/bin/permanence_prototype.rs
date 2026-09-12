//! THROWAWAY VALIDATION SPIKE — NOT PART OF THE PRODUCTION STEPPER (issue #432).
//!
//! Numerics for the research finding `docs/research/432-permanence-pc.md`: the
//! **permanence** (uniform persistence) condition for the two-compartment
//! producer-biomass `P` ↔ consumer-biomass `C` mean-field map that
//! `hopf_prototype.rs` (issue #358) assembled from committed fluxes. The map is
//! reused verbatim — nothing here re-derives it — and, as there, it does **not**
//! touch, fork, or alter the committed scalar stepper, the RNG, the evaluator, or
//! the search.
//!
//! ```text
//!   P' = P · f(P,C),  f = 1 + r_P·(1 − P/K_P) − a·C
//!   C' = C · g(P,C),  g = 1 + β·P − m,          β = κ_C·γ·e·a
//! ```
//!
//! The finding: with `V = P^a·C^b` as an average Lyapunov function, the
//! extinction boundary `{P = 0} ∪ {C = 0}` is a repeller iff
//!
//! ```text
//!   r_P > 0        (⟺ F > B_P — the extinction gate `F ≤ B`, sharpened)
//!   β·K_P > m      (the consumer invades the producer-only equilibrium)
//! ```
//!
//! This bin (a) tabulates the boundary equilibria with their eigenvalues, (b)
//! sweeps the two dominant dimensionless ratios — the extinction ratio
//! `ρ = F/B_P` and the invasion ratio `I = β·K_P/m` (via `base_trophic_efficiency`)
//! — and (c) checks the analytic condition against brute-force *boundary escape*:
//! iterate the map from a point hugging the boundary and ask whether both
//! compartments stay bounded away from zero. The Hopf line from #358 is drawn on
//! the same sweep so the reader can see it sits inside the permanent region.
//!
//! One deviation from `hopf_prototype`, deliberate and documented in the doc:
//! `r_P` is kept **signed** (`hopf_prototype` clamps it at 0 because it only
//! needs the interior fixed point). The sign of `r_P` *is* the extinction gate,
//! so the boundary analysis must see it. When `r_P ≤ 0` the producer face is
//! read as density-independent decline `f = 1 + r_P` (shading can only lower
//! income further, so this bounds the true face factor from above).
//!
//! Usage:
//!   cargo run -p explorers-sim --bin permanence_prototype -- scenarios/example10_predator_prey_hopf.json

use explorers_sim::{TraitVector, WorldParameters, WorldRecipe};

/// Reference body mass for the mean-field reduction (as in `hopf_prototype`).
const REFERENCE_BODY_MASS: f32 = 1.0;

/// The lumped coefficients of the 2-compartment map. Same derivation as
/// `hopf_prototype::Compartments` (citations there), except `r_p` is signed.
#[derive(Debug, Clone, Copy)]
pub struct Map {
    /// Producer intrinsic per-tick rate κ_P·γ·(F − B_P). **Signed.**
    pub r_p: f64,
    /// Producer light-saturated carrying capacity F / B_P.
    pub k_p: f64,
    /// Mass-action attack rate (consumer effective heterotrophy per unit body).
    pub a: f64,
    /// Consumer removal rate: its maintenance floor B_C.
    pub m: f64,
    /// Consumer biomass-conversion coefficient β = κ_C·γ·e·a.
    pub beta: f64,
}

/// The analytic permanence verdict: both clauses, and their conjunction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permanence {
    /// `r_P > 0` ⟺ `F > B_P`: the producer-only face has a positive equilibrium.
    pub producer_face_alive: bool,
    /// `β·K_P > m`: the consumer's invasion eigenvalue at `(K_P, 0)` exceeds 1.
    pub consumer_invades: bool,
}

impl Permanence {
    pub fn permanent(self) -> bool {
        self.producer_face_alive && self.consumer_invades
    }
}

/// What brute-force iteration from the boundary says about a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericClass {
    /// Both compartments stay bounded away from zero over the late window.
    Permanent,
    /// At least one compartment decays toward zero.
    NotPermanent,
    /// The orbit left the unsaturated regime (`f ≤ 0` or `g ≤ 0`): the Euler
    /// map is no longer a faithful reading, and the analytic condition is silent.
    LeftRegime,
}

/// One row of the boundary-equilibrium table: the point, its two eigenvalues
/// (real parts along the coordinate axes for the boundary points; the
/// complex-pair modulus, repeated, for the interior point), and a label.
#[derive(Debug, Clone, Copy)]
pub struct Equilibrium {
    pub label: &'static str,
    pub point: (f64, f64),
    /// `[λ_P, λ_C]` for the boundary points (`(0,0)` and `(K_P,0)` have a
    /// diagonal/triangular Jacobian, so these are exact); `[|λ|, |λ|]` for the
    /// interior point when the pair is complex, else the two reals.
    pub eigenvalues: [f64; 2],
}

impl Map {
    /// The equilibria of the map with their eigenvalues. Rows: the origin
    /// `(0,0)`, the producer-only point `(K_P, 0)`, and — when it exists — the
    /// interior point `(m/β, (r_P/a)(1 − m/(β·K_P)))`.
    pub fn boundary_equilibria(&self) -> Vec<Equilibrium> {
        let mut rows = vec![Equilibrium {
            label: "(0,0)      extinction",
            point: (0.0, 0.0),
            eigenvalues: [1.0 + self.r_p, 1.0 - self.m],
        }];
        if self.r_p > 0.0 {
            rows.push(Equilibrium {
                label: "(K_P,0)    producer-only",
                point: (self.k_p, 0.0),
                eigenvalues: [1.0 - self.r_p, 1.0 + self.beta * self.k_p - self.m],
            });
        }
        if let Some((p_star, c_star)) = self.interior_fixed_point() {
            // Jacobian as in hopf_prototype: τ = 2 − r_P·P*/K_P,
            // Δ = 1 − r_P·P*/K_P + m·r_P·(1 − P*/K_P).
            let j11 = 1.0 + self.r_p - 2.0 * self.r_p * p_star / self.k_p - self.a * c_star;
            let j12 = -self.a * p_star;
            let j21 = self.beta * c_star;
            let j22 = 1.0 + self.beta * p_star - self.m;
            let tr = j11 + j22;
            let det = j11 * j22 - j12 * j21;
            let disc = tr * tr - 4.0 * det;
            let eigenvalues = if disc < 0.0 {
                let modulus = det.max(0.0).sqrt();
                [modulus, modulus]
            } else {
                [(tr + disc.sqrt()) / 2.0, (tr - disc.sqrt()) / 2.0]
            };
            rows.push(Equilibrium {
                label: "(P*,C*)    interior",
                point: (p_star, c_star),
                eigenvalues,
            });
        }
        rows
    }

    /// Interior (coexistence) fixed point, if positive (as in `hopf_prototype`).
    pub fn interior_fixed_point(&self) -> Option<(f64, f64)> {
        if self.beta <= 0.0 || self.a <= 0.0 || self.r_p <= 0.0 {
            return None;
        }
        let p_star = self.m / self.beta;
        if !(p_star > 0.0 && p_star < self.k_p) {
            return None;
        }
        let c_star = (self.r_p / self.a) * (1.0 - p_star / self.k_p);
        (c_star > 0.0).then_some((p_star, c_star))
    }

    /// Per-tick maintenance of one reference body (metabolise, as in
    /// `hopf_prototype::Compartments::maintenance`).
    fn maintenance(traits: &TraitVector, p: &WorldParameters) -> f64 {
        let exp = p.maintenance_cost_exponent;
        (p.base_metabolic_rate
            + traits.photosynthetic_absorption.max(0.0).powf(exp) * p.photo_maintenance_cost
            + traits.heterotrophy.max(0.0).powf(exp) * p.heterotrophy_maintenance_cost
            + traits.mobility.max(0.0).powf(exp) * p.mobility_maintenance_cost
            + REFERENCE_BODY_MASS * p.structure_maintenance_coefficient) as f64
    }

    /// Derive the map from committed parameters and the two cluster-mean trait
    /// vectors — identical to `hopf_prototype::Compartments::derive` except that
    /// `r_p` keeps its sign.
    pub fn derive(producer: &TraitVector, consumer: &TraitVector, p: &WorldParameters) -> Self {
        let kappa_p = producer.kappa.clamp(0.0, 1.0) as f64;
        let kappa_c = consumer.kappa.clamp(0.0, 1.0) as f64;
        let gamma = p.growth_efficiency as f64;
        let b_p = Self::maintenance(producer, p);
        let b_c = Self::maintenance(consumer, p);
        let flux = p.solar_flux_magnitude as f64;

        let r_p = kappa_p * gamma * (flux - b_p) / REFERENCE_BODY_MASS as f64;
        let k_p = if b_p > 0.0 { flux / b_p } else { f64::INFINITY };
        let a = consumer.heterotrophy.max(0.0) as f64 / REFERENCE_BODY_MASS as f64;
        let m = b_c / REFERENCE_BODY_MASS as f64;
        let d = producer.distance(consumer) as f64;
        let e = p.base_trophic_efficiency as f64 * (-(p.trophic_distance_decay as f64) * d).exp();
        let beta = kappa_c * gamma * e * a;
        Map {
            r_p,
            k_p,
            a,
            m,
            beta,
        }
    }

    /// The analytic permanence condition.
    pub fn permanence(&self) -> Permanence {
        Permanence {
            producer_face_alive: self.r_p > 0.0,
            consumer_invades: self.beta * self.k_p > self.m,
        }
    }

    /// Per-tick multipliers `(f, g)` of the map at `(P, C)`.
    pub fn factors(&self, p: f64, c: f64) -> (f64, f64) {
        let f = if self.r_p > 0.0 {
            1.0 + self.r_p * (1.0 - p / self.k_p) - self.a * c
        } else {
            1.0 + self.r_p - self.a * c
        };
        let g = 1.0 + self.beta * p - self.m;
        (f, g)
    }

    /// One tick of the map.
    pub fn step(&self, p: f64, c: f64) -> (f64, f64) {
        let (f, g) = self.factors(p, c);
        (p * f, c * g)
    }

    /// Brute-force boundary escape: iterate from a point hugging the boundary
    /// and classify by the late-window minimum of each compartment.
    ///
    /// The start point sits just off *both* faces (`P` at a tiny fraction of
    /// the producer scale, `C` at a tiny fraction of that), so the orbit must
    /// escape `(0,0)` along the producer face and then escape `(K_P, 0)` via
    /// consumer invasion — the two boundary ω-limit sets of the theorem.
    pub fn classify_by_simulation(&self, horizon: usize) -> NumericClass {
        let scale = if self.k_p.is_finite() && self.k_p > 0.0 {
            self.k_p
        } else {
            1.0
        };
        let (mut p, mut c) = (1e-3 * scale, 1e-6 * scale);
        // Trough of each compartment over the third and fourth quarters of the
        // horizon. `lim inf > 0` reads numerically as "the trough has stopped
        // shrinking": a compartment decaying geometrically loses a factor
        // ≥ (1 − ε)^(horizon/4) between the two windows, while an orbit settled
        // on an interior fixed point or cycle keeps the same trough.
        let mut mins = [[f64::INFINITY; 2]; 2]; // [window][P, C]
        for t in 0..horizon {
            let (f, g) = self.factors(p, c);
            if f <= 0.0 || g <= 0.0 {
                return NumericClass::LeftRegime;
            }
            p *= f;
            c *= g;
            if t >= horizon / 2 {
                let w = if t < 3 * horizon / 4 { 0 } else { 1 };
                mins[w][0] = mins[w][0].min(p);
                mins[w][1] = mins[w][1].min(c);
            }
        }
        // The absolute floor guards against a decaying compartment stalling at
        // the smallest subnormal and masquerading as a stable trough.
        let trough_stable = |k: usize| mins[1][k] > 1e-100 && mins[1][k] > 0.1 * mins[0][k];
        if trough_stable(0) && trough_stable(1) {
            NumericClass::Permanent
        } else {
            NumericClass::NotPermanent
        }
    }
}

/// Producer / consumer cluster means from a scenario roster (as in `hopf_prototype`).
fn extract_clusters(agents: &[explorers_sim::AgentSpec]) -> Option<(TraitVector, TraitVector)> {
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
    if prod_n == 0 || cons_n == 0 {
        return None;
    }
    Some((
        scale(&prod_sum, 1.0 / prod_n as f32),
        scale(&cons_sum, 1.0 / cons_n as f32),
    ))
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

/// Classification of one sweep cell, combining the analytic verdict, the
/// brute-force verdict, and the Hopf line.
fn cell_glyph(map: &Map, numeric: NumericClass) -> char {
    let analytic = map.permanence().permanent();
    let invasion = map.beta * map.k_p / map.m;
    let above_hopf = invasion > (1.0 + map.m) / map.m;
    match (analytic, numeric) {
        (true, NumericClass::Permanent) if above_hopf => 'o', // permanent, cycling
        (true, NumericClass::Permanent) => '#',               // permanent, stable node/spiral
        (false, NumericClass::NotPermanent) => '.',           // not permanent (agree)
        (_, NumericClass::LeftRegime) => 'x', // Euler map left the unsaturated regime
        (true, NumericClass::NotPermanent) => '!', // disagree: analytic yes, numerics no
        (false, NumericClass::Permanent) => '?', // disagree: analytic no, numerics yes
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!(
            "usage: permanence_prototype <scenario.json>\n\
             (throwaway analysis spike, issue #432 — not part of the stepper)"
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
    let map = Map::derive(&producer, &consumer, params);
    let b_p = Map::maintenance(&producer, params);

    println!("# Permanence prototype — throwaway boundary-escape numerics (issue #432)");
    println!("# NOT PART OF THE PRODUCTION STEPPER. Mean-field reading of the committed map T.");
    println!("# scenario: {path}");
    println!();
    println!("## Map coefficients (committed fluxes, as hopf_prototype; r_P signed)");
    println!(
        "  F (solar flux)                 = {:.4}",
        params.solar_flux_magnitude
    );
    println!("  B_P (producer maintenance)     = {:.4}", b_p);
    println!("  B_C = m (consumer maintenance) = {:.4}", map.m);
    println!("  r_P = kappa_P*gamma*(F - B_P)  = {:.4}", map.r_p);
    println!("  K_P = F / B_P                  = {:.4}", map.k_p);
    println!("  a   (attack rate)              = {:.4}", map.a);
    println!("  beta = kappa_C*gamma*e*a       = {:.5}", map.beta);
    println!();
    println!("## Equilibria and eigenvalues");
    println!(
        "  {:<26} {:>10} {:>10} {:>10} {:>10}",
        "point", "P", "C", "lambda_1", "lambda_2"
    );
    for row in map.boundary_equilibria() {
        println!(
            "  {:<26} {:>10.4} {:>10.4} {:>10.4} {:>10.4}",
            row.label, row.point.0, row.point.1, row.eigenvalues[0], row.eigenvalues[1]
        );
    }
    println!();
    let perm = map.permanence();
    let invasion = map.beta * map.k_p / map.m;
    println!("## Permanence condition at the committed parameters");
    println!(
        "  rho = F/B_P        = {:.3}   (clause 1: r_P > 0  <=>  rho > 1)   {}",
        map.k_p, perm.producer_face_alive
    );
    println!(
        "  I   = beta*K_P/m   = {:.3}   (clause 2: I > 1)                   {}",
        invasion, perm.consumer_invades
    );
    println!(
        "  Hopf line          I = (1+m)/m = {:.3}",
        (1.0 + map.m) / map.m
    );
    println!("  permanent (analytic)  = {}", perm.permanent());
    println!(
        "  permanent (numerics)  = {:?}",
        map.classify_by_simulation(20_000)
    );
    println!();

    // --- Sweep rho (via F) x base_trophic_efficiency ---
    println!("## Sweep: rho = F/B_P (rows, log-spaced) x base_trophic_efficiency (cols)");
    println!("   '#' permanent, stable interior   'o' permanent, above the Hopf line");
    println!("   '.' not permanent                'x' Euler map left the unsaturated regime");
    println!("   '!' / '?' analytic-vs-numeric disagreement (none expected off the boundary)");
    let effs: Vec<f32> = (1..=25).map(|j| j as f32 / 25.0).collect();
    print!("  {:>8}  {:>7} ", "rho", "I@eff=1");
    for e in &effs {
        print!(
            "{}",
            if (e * 100.0).round() as i32 % 20 == 0 {
                '|'
            } else {
                ' '
            }
        );
    }
    println!();
    let mut agree = 0usize;
    let mut disagree = 0usize;
    let mut left = 0usize;
    let mut near = 0usize;
    for i in 0..25 {
        let rho = 0.5 * (200f64).powf(i as f64 / 24.0);
        let mut line = String::new();
        let mut i_at_one = 0.0;
        for e in &effs {
            let mut p = params.clone();
            p.solar_flux_magnitude = (rho * b_p) as f32;
            p.base_trophic_efficiency = *e;
            let m = Map::derive(&producer, &consumer, &p);
            let inv = m.beta * m.k_p / m.m;
            i_at_one = inv / *e as f64;
            let numeric = m.classify_by_simulation(20_000);
            let g = cell_glyph(&m, numeric);
            line.push(g);
            let on_boundary = (rho - 1.0).abs() <= 0.02 || (inv - 1.0).abs() <= 0.02;
            match g {
                'x' => left += 1,
                _ if on_boundary => near += 1,
                '!' | '?' => disagree += 1,
                _ => agree += 1,
            }
        }
        println!("  {:>8.2}  {:>7.2} {line}", rho, i_at_one);
    }
    println!("  cols: eff = 0.04 .. 1.00 in steps of 0.04; '|' marks 0.2, 0.4, 0.6, 0.8, 1.0");
    println!();
    println!("## Agreement (analytic vs boundary-escape numerics, 20k-tick horizon)");
    println!(
        "  agree = {agree}, disagree = {disagree}, within 2% of a boundary (not scored) = {near}, left regime = {left}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> WorldParameters {
        // Mirrors hopf_prototype's self-contained test parameter set.
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

    /// Sweep the two dominant dimensionless ratios — the extinction ratio
    /// `ρ = F/B_P` (via `solar_flux_magnitude`) and the invasion ratio
    /// `I = β·K_P/m` (via `base_trophic_efficiency`) — and require the
    /// brute-force boundary-escape verdict to agree with the analytic condition
    /// on every cell that (a) stays in the unsaturated regime and (b) is not
    /// within the stated tolerance of either analytic boundary (`|ρ − 1| > 2%`,
    /// `|I − 1| > 2%`), where a finite horizon cannot resolve a growth factor
    /// that close to 1.
    #[test]
    fn boundary_escape_numerics_agree_with_analytic_condition() {
        let base = params();
        let b_p = Map::maintenance(&producer(), &base);
        let mut checked = 0usize;
        for i in 0..40 {
            // ρ from 0.5 to 100, log-spaced.
            let rho = 0.5 * (200f64).powf(i as f64 / 39.0);
            for j in 1..=40 {
                let eff = j as f32 / 40.0;
                let mut p = base.clone();
                p.solar_flux_magnitude = (rho * b_p) as f32;
                p.base_trophic_efficiency = eff;
                let map = Map::derive(&producer(), &consumer(), &p);
                let invasion = map.beta * map.k_p / map.m;
                if (rho - 1.0).abs() <= 0.02 || (invasion - 1.0).abs() <= 0.02 {
                    continue;
                }
                let analytic = map.permanence().permanent();
                match map.classify_by_simulation(20_000) {
                    NumericClass::LeftRegime => continue,
                    NumericClass::Permanent => assert!(
                        analytic,
                        "rho={rho:.3} eff={eff:.3} I={invasion:.3}: numerics permanent, analytic not"
                    ),
                    NumericClass::NotPermanent => assert!(
                        !analytic,
                        "rho={rho:.3} eff={eff:.3} I={invasion:.3}: analytic permanent, numerics not"
                    ),
                }
                checked += 1;
            }
        }
        assert!(
            checked > 1000,
            "sweep should exercise most cells, got {checked}"
        );
    }

    /// The extinction gate `F ≤ B` (viability.md) must fall out of the
    /// condition on the `C = 0` face: with solar flux at or below the *base*
    /// metabolic rate — before any trait or structure maintenance — the producer
    /// face contracts (`λ_P(0,0) = 1 + r_P ≤ 1`), so no trophic efficiency can
    /// rescue permanence.
    #[test]
    fn condition_fails_on_the_c0_face_whenever_flux_is_at_or_below_base_metabolism() {
        for flux_over_base in [0.0f32, 0.25, 0.5, 1.0] {
            for eff in [0.01f32, 0.5, 1.0] {
                let mut p = params();
                p.solar_flux_magnitude = flux_over_base * p.base_metabolic_rate;
                p.base_trophic_efficiency = eff;
                let map = Map::derive(&producer(), &consumer(), &p);
                let table = map.boundary_equilibria();
                let origin = &table[0];
                assert!(
                    origin.eigenvalues[0] <= 1.0,
                    "F/B={flux_over_base}: producer eigenvalue at (0,0) should be <= 1, got {}",
                    origin.eigenvalues[0]
                );
                assert!(!map.permanence().producer_face_alive);
                assert!(!map.permanence().permanent());
                assert_eq!(
                    map.classify_by_simulation(2_000),
                    NumericClass::NotPermanent
                );
            }
        }
    }

    /// The tabulated eigenvalues are the spectrum of the *actual* map: at each
    /// equilibrium a central-difference Jacobian of `step` must reproduce them
    /// (trace and determinant, which fix the pair), and each equilibrium must
    /// be a fixed point of `step`.
    #[test]
    fn tabulated_eigenvalues_match_finite_difference_jacobian_of_the_map() {
        let map = Map::derive(&producer(), &consumer(), &params());
        let table = map.boundary_equilibria();
        assert_eq!(table.len(), 3, "all three equilibria should exist here");
        for row in &table {
            let (p, c) = row.point;
            let (p1, c1) = map.step(p, c);
            assert!(
                (p1 - p).abs() < 1e-9 && (c1 - c).abs() < 1e-9,
                "{}: not fixed",
                row.label
            );
            let h = 1e-6;
            let (pp, cp) = map.step(p + h, c);
            let (pm, cm) = map.step(p - h, c);
            let (pq, cq) = map.step(p, c + h);
            let (pn, cn) = map.step(p, c - h);
            let j = [
                [(pp - pm) / (2.0 * h), (pq - pn) / (2.0 * h)],
                [(cp - cm) / (2.0 * h), (cq - cn) / (2.0 * h)],
            ];
            let tr = j[0][0] + j[1][1];
            let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
            let [l1, l2] = row.eigenvalues;
            let disc = tr * tr - 4.0 * det;
            if disc < 0.0 {
                assert!(
                    (l1 - l2).abs() < 1e-12,
                    "{}: complex pair reported as modulus",
                    row.label
                );
                assert!(
                    (l1 * l1 - det).abs() < 1e-6,
                    "{}: |λ|² = {} vs det {det}",
                    row.label,
                    l1 * l1
                );
            } else {
                assert!(
                    (l1 + l2 - tr).abs() < 1e-6,
                    "{}: trace {tr} vs {}",
                    row.label,
                    l1 + l2
                );
                assert!(
                    (l1 * l2 - det).abs() < 1e-6,
                    "{}: det {det} vs {}",
                    row.label,
                    l1 * l2
                );
            }
        }
    }

    /// The Neimark–Sacker line from #358, `I = (1+m)/m`, lies strictly inside
    /// the permanent region `I > 1`: oscillation is born *within* permanence,
    /// never at its edge. Checked on the swept efficiency axis at the anchor
    /// flux — the analytic crossing is permanent on both sides.
    #[test]
    fn hopf_line_sits_strictly_inside_the_permanent_region() {
        let map = Map::derive(&producer(), &consumer(), &params());
        let hopf_invasion_ratio = (1.0 + map.m) / map.m;
        assert!(hopf_invasion_ratio > 1.0);
        let invasion = map.beta * map.k_p / map.m;
        let hopf_eff = params().base_trophic_efficiency as f64 * hopf_invasion_ratio / invasion;
        assert!(
            hopf_eff > 0.0 && hopf_eff <= 1.0,
            "crossing {hopf_eff} should be physical here"
        );
        for factor in [0.9, 1.1] {
            let mut p = params();
            p.base_trophic_efficiency = (hopf_eff * factor) as f32;
            let m = Map::derive(&producer(), &consumer(), &p);
            assert!(
                m.permanence().permanent(),
                "factor {factor}: should be permanent"
            );
            let interior = m.boundary_equilibria().into_iter().last().unwrap();
            let stable = interior.eigenvalues[0] < 1.0;
            assert_eq!(
                stable,
                factor < 1.0,
                "factor {factor}: stability should flip at the Hopf line"
            );
        }
    }
}
