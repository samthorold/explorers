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
//! Two deviations from `hopf_prototype`, deliberate and documented in the doc.
//! `r_P = χ_P·γ·(F − B_P)` counts the producer's whole surplus as biomass —
//! the `(1 − κ_P)` share becomes offspring through the committed reproductive
//! branch, at its own efficiency `η_P` (#466; `hopf_prototype` lumps only the
//! somatic `κ_P` share). And `r_P` is kept **signed** (`hopf_prototype` clamps it at 0 because it only
//! needs the interior fixed point). The sign of `r_P` *is* the extinction gate,
//! so the boundary analysis must see it. When `r_P ≤ 0` the producer face is
//! read as density-independent decline `f = 1 + r_P` (shading can only lower
//! income further, so this bounds the true face factor from above).
//!
//! **Issue #437 extension.** The second half of this bin couples the map above
//! to the three-compartment nutrient cycle `A ⇌ L ⇌ C_N` (available, living,
//! carcass-locked; conserved `N_total`) through the committed Liebig
//! co-limitation and the binary-reach drain run over carcasses — the
//! reduction of `docs/research/437-permanence-alc.md`. It carries the
//! *endogenous* lumpings (carcass richness, nutrient per carcass, reach
//! geometry, producer mortality) as explicit inputs so the doc can say which
//! part of the lockup condition is a characterisation and which a gate. Same
//! banner: NOT PART OF THE PRODUCTION STEPPER.
//!
//! Usage:
//!   cargo run -p explorers-sim --bin permanence_prototype -- scenarios/example10_predator_prey_hopf.json

use explorers_sim::{TraitVector, WorldParameters, WorldRecipe};

/// Reference body mass for the mean-field reduction (as in `hopf_prototype`).
const REFERENCE_BODY_MASS: f32 = 1.0;

/// The fecundity floor `resolve_reproduction` applies before the Poisson draw
/// (`fecundity.max(0.1)` in `phase.rs`).
const FECUNDITY_FLOOR: f64 = 0.1;

/// How much of a producer's mobilised surplus ends up as **structure** — the
/// biomass coordinate `P` carries — once both of `κ`'s branches are followed
/// to the body they build (#466).
#[derive(Debug, Clone, Copy)]
pub struct BiomassConversion {
    /// Somatic allocation `κ_P` (flow 9's split of the mobilised flow).
    pub kappa: f64,
    /// Reproductive-branch efficiency `η_P`: the fraction of energy routed to
    /// the reproductive allocation that reaches an offspring's body —
    /// `reproduction_efficiency` (flow 4 heat), times the dispersal propagule
    /// share left over, times the probability the Poisson brood is non-empty
    /// (a zero draw dissipates the whole investment).
    pub eta: f64,
    /// `offspring_structure_fraction` — the share of an offspring's energy
    /// embodied at birth; the rest is reserve the offspring itself mobilises.
    pub structure_fraction: f64,
    /// The conversion `χ_P`: structure built (before `γ`) per unit surplus,
    /// closing the loop in which an offspring's reserve is re-split by `κ_P`.
    /// `χ_P = 1` when `κ_P = 1` or the reproductive branch is lossless.
    pub chi: f64,
}

/// Derive the producer's biomass conversion from committed parameters and its
/// trait vector (`kappa`, `dispersal`, `fecundity`).
pub fn biomass_conversion(producer: &TraitVector, p: &WorldParameters) -> BiomassConversion {
    let kappa = producer.kappa.clamp(0.0, 1.0) as f64;
    let propagule = explorers_sim::dispersal_propagule_cost_fraction(producer.dispersal, p) as f64;
    let fecundity = (producer.fecundity as f64).max(FECUNDITY_FLOOR);
    let eta = (p.reproduction_efficiency as f64).clamp(0.0, 1.0)
        * (1.0 - propagule)
        * (1.0 - (-fecundity).exp());
    let s = p.offspring_structure_fraction.clamp(0.0, 1.0) as f64;
    // One unit of surplus: κ builds κ structure (× γ, applied by the caller);
    // (1 − κ) reaches offspring at η, of which s is embodied at birth and
    // (1 − s) is reserve the offspring re-splits the same way:
    //   χ = κ + (1 − κ)·η·(s + (1 − s)·χ).
    let feedback = (1.0 - kappa) * eta * (1.0 - s);
    let chi = if feedback >= 1.0 {
        1.0
    } else {
        (kappa + (1.0 - kappa) * eta * s) / (1.0 - feedback)
    };
    BiomassConversion {
        kappa,
        eta,
        structure_fraction: s,
        chi,
    }
}

/// The lumped coefficients of the 2-compartment map. Same derivation as
/// `hopf_prototype::Compartments` (citations there), except `r_p` is signed
/// and the producer's growth counts both of `κ_P`'s branches (#466).
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
    /// vectors — as `hopf_prototype::Compartments::derive` except that `r_p`
    /// keeps its sign and counts both of `κ_P`'s branches (`χ_P`, #466).
    pub fn derive(producer: &TraitVector, consumer: &TraitVector, p: &WorldParameters) -> Self {
        let kappa_c = consumer.kappa.clamp(0.0, 1.0) as f64;
        let gamma = p.growth_efficiency as f64;
        let b_p = Self::maintenance(producer, p);
        let b_c = Self::maintenance(consumer, p);
        let flux = p.solar_flux_magnitude as f64;

        let chi_p = biomass_conversion(producer, p).chi;
        let r_p = chi_p * gamma * (flux - b_p) / REFERENCE_BODY_MASS as f64;
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

// ---------------------------------------------------------------------------
// Coupled energy × nutrient reduction (issue #437): the A1 map above coupled to
// the three-compartment nutrient cycle `A ⇌ L ⇌ C_N`. Five named state
// variables — `P`, `C_E` (structure, reference bodies) and `A`, `L`, `C_N`
// (nutrient) — of which four are independent because `A + L + C_N = N_total`.
// Every flux is a mean-field reading of a committed phase (citations on the
// fields); the lumped coefficients that are *endogenous* in the stepper
// (carcass richness, nutrient per carcass, reach geometry, producer mortality)
// are collected in `AlcLumping` so the doc can point at exactly which terms
// keep the lockup condition a characterisation rather than a gate.
// ---------------------------------------------------------------------------

/// Lumped, endogenous coefficients of the coupled reduction. None is a world
/// parameter; each is set by the realised economy of a run. They are inputs
/// here so the map can be iterated — never bounds.
#[derive(Debug, Clone, Copy)]
pub struct AlcLumping {
    /// Producer per-tick mortality (flow 6: wear-driven intrinsic death plus
    /// the below-threshold residue of drain kills), netted out of A1's `r_P`.
    pub mu_p: f64,
    /// Carcass richness `q`: nutrient per unit carcass energy in the standing
    /// dead pool (a carcass carries the dead agent's exact content).
    pub q: f64,
    /// Nutrient per carcass `ν` (= `q` × the dead body's structure): fixes the
    /// carcass *count* `C_N / ν` that the binary-reach drain multiplies.
    pub nu: f64,
    /// In-reach geometry `ι ∈ [0, 1]`: the fraction of carcasses within a
    /// consumer body's feeding reach (flow 3, `consumption_reach`). A1 sets
    /// the living-prey indicator to 1; kept explicit here for the carcass term.
    pub iota: f64,
    /// Trophic kernel on the carcass pool `e_C`: `base · exp(−λ·d)` at the
    /// consumer↔carcass trait distance. A producer carcass keeps the producer's
    /// exact trait vector, so for a producer-dominated pile `e_C = e` (A1's).
    /// `None` reads the pile as producer carcasses.
    pub e_c: Option<f64>,
}

impl Default for AlcLumping {
    fn default() -> Self {
        AlcLumping {
            mu_p: 0.02,
            q: f64::NAN, // filled from θ_P by `derive` when NaN
            nu: f64::NAN,
            iota: 1.0,
            e_c: None,
        }
    }
}

/// State of the coupled reduction: the five named stocks, all carried
/// explicitly so that no compartment is a difference of two near-equal
/// stocks (near the lockup corner `L = N_total − A − C_N` loses every digit).
/// Conservation `A + L + C_N = N_total` is then a *check* on the map, not a
/// definition — see the ledger test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlcState {
    /// Producer structure (reference bodies).
    pub p: f64,
    /// Consumer structure (reference bodies).
    pub c_e: f64,
    /// Available pool.
    pub a: f64,
    /// Free (unbound) living nutrient — free store plus reproductive earmark,
    /// pooled across the living. `L = θ_P·P + θ_C·C_E + Φ`.
    pub phi: f64,
    /// Carcass-locked nutrient.
    pub c_n: f64,
}

/// The analytic persistence verdict of the coupled reduction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlcPersistence {
    /// `min(r_P, α_P/θ_P) > μ_P`: the producer invades the virgin pool.
    pub producer_invades_virgin: bool,
    /// `Λ > 1`: the heterotroph invades the carcass pile (lockup repeller).
    pub heterotroph_invades_lockup: bool,
    /// `ln λ_P(virgin) · ln λ_C(lockup) > ln(1/(1−m)) · ln(1/(1−μ_P))`: the
    /// dead-line heteroclinic cycle is repelling.
    pub cycle_repels: bool,
}

impl AlcPersistence {
    pub fn permanent(self) -> bool {
        self.producer_invades_virgin && self.heterotroph_invades_lockup && self.cycle_repels
    }
}

/// The coupled map. The energy side is the A1 `Map` verbatim; the nutrient
/// side adds uptake, Liebig co-limitation, excretion, death and decomposition.
#[derive(Debug, Clone, Copy)]
pub struct AlcMap {
    /// A1's producer↔consumer energy map (reused, not re-derived).
    pub energy: Map,
    /// Producer per-body pool uptake `α_P` per tick (absorb_nutrients: the
    /// per-agent demand is the effective autotrophy trait, pool-capped).
    pub alpha_p: f64,
    /// Stoichiometric demand `θ_P`, `θ_C`: nutrient bound per unit structure
    /// (`stoichiometric_demand` at unit structure).
    pub theta_p: f64,
    pub theta_c: f64,
    /// Consumer somatic conversion `κ_C·γ` (grow phase), kept separate so
    /// the carcass return can carry its own kernel `e_C`.
    pub kappa_c_gamma: f64,
    /// Conserved nutrient total (`initial_nutrient_pool`; the unavailable pool
    /// is geological and constant, so it is left out of the ledger).
    pub n_total: f64,
    pub lump: AlcLumping,
}

impl AlcMap {
    pub fn derive(
        producer: &TraitVector,
        consumer: &TraitVector,
        p: &WorldParameters,
        mut lump: AlcLumping,
    ) -> Self {
        let energy = Map::derive(producer, consumer, p);
        let theta_p = explorers_sim::stoichiometric_demand(producer, 1.0, p) as f64;
        let theta_c = explorers_sim::stoichiometric_demand(consumer, 1.0, p) as f64;
        // A producer carcass with no free store: richness = θ_P per unit
        // energy, and one reference body of structure per carcass.
        if lump.q.is_nan() {
            lump.q = theta_p;
        }
        if lump.nu.is_nan() {
            lump.nu = lump.q * REFERENCE_BODY_MASS as f64;
        }
        AlcMap {
            energy,
            alpha_p: producer.photosynthetic_absorption.max(0.0) as f64,
            theta_p,
            theta_c,
            kappa_c_gamma: consumer.kappa.clamp(0.0, 1.0) as f64 * p.growth_efficiency as f64,
            n_total: p.initial_nutrient_pool as f64,
            lump,
        }
    }

    /// Living nutrient `L = θ_P·P + θ_C·C_E + Φ` (bound plus free).
    pub fn living(&self, x: &AlcState) -> f64 {
        self.bound(x) + x.phi
    }

    /// Nutrient bound in living structure, `θ_P·P + θ_C·C_E ≤ L` (embodiment).
    /// Tracked directly, so it keeps its precision where `L` as a difference of
    /// two near-equal stocks does not; permanence in `(P, C_E)` bounds it, and
    /// therefore `L`, away from zero.
    pub fn bound(&self, x: &AlcState) -> f64 {
        self.theta_p * x.p + self.theta_c * x.c_e
    }

    /// Trophic kernel on the carcass pool `e_C`. A producer carcass carries the
    /// producer's exact trait vector, so for a producer-dominated pile it is
    /// A1's `e = β / (κ_C·γ·a)`.
    pub fn e_c(&self) -> f64 {
        self.lump
            .e_c
            .unwrap_or(self.energy.beta / (self.kappa_c_gamma * self.energy.a))
    }

    /// Per-consumer-body, per-unit-carcass-nutrient clearance rate
    /// `σ = a·ι/ν` (binary-reach drain of every in-reach carcass at `a`).
    pub fn sigma(&self) -> f64 {
        self.energy.a * self.lump.iota / self.lump.nu
    }

    /// Per-consumer-body growth on the full carcass pile: the Liebig minimum
    /// of the energy conversion `κ_C·γ·e_C` and the carcass richness over the
    /// consumer's own demand `q/θ_C`, times the clearance `σ·N_total`.
    fn pile_growth_per_body(&self) -> f64 {
        let liebig = (self.kappa_c_gamma * self.e_c()).min(self.lump.q / self.theta_c);
        self.sigma() * self.n_total * liebig
    }

    /// The lockup-escape ratio `Λ = σ·N_total·min(κ_C·γ·e_C, q/θ_C) / m`: the
    /// heterotroph's per-tick conversion on the whole carcass pile over its
    /// maintenance floor. `Λ > 1` is the lockup repeller condition — the
    /// carcass-pile analogue of A1's invasion ratio `I = β·K_P/m`.
    pub fn lockup_escape_ratio(&self) -> f64 {
        self.pile_growth_per_body() / self.energy.m
    }

    /// Transverse eigenvalues `[λ_P, λ_C]` at the lockup corner
    /// `(0, 0, 0, N_total)`: `1 − μ_P` (no pool, so Liebig stops producer
    /// growth at zero whatever the flux) and `1 + σ·N_total·min(…) − m`.
    pub fn lockup_eigenvalues(&self) -> [f64; 2] {
        [
            1.0 - self.lump.mu_p,
            1.0 + self.pile_growth_per_body() - self.energy.m,
        ]
    }

    /// Transverse eigenvalues `[λ_P, λ_C]` at the virgin end of the dead line,
    /// `(0, 0, N_total, 0)`: the producer's Liebig-limited invasion
    /// `1 + min(r_P, α_P/θ_P) − μ_P` and the starving consumer's `1 − m`.
    pub fn virgin_eigenvalues(&self) -> [f64; 2] {
        [
            1.0 + self.energy.r_p.min(self.alpha_p / self.theta_p) - self.lump.mu_p,
            1.0 - self.energy.m,
        ]
    }

    /// The coupled persistence verdict.
    pub fn persistence(&self) -> AlcPersistence {
        let [lp_v, lc_v] = self.virgin_eigenvalues();
        let [lp_l, lc_l] = self.lockup_eigenvalues();
        let producer_invades_virgin = lp_v > 1.0;
        let heterotroph_invades_lockup = lc_l > 1.0;
        // A common average-Lyapunov weight (a₁, a₂) exists for both ends of the
        // dead line iff the product of the two invasion log-rates exceeds the
        // product of the two decay log-rates — the heteroclinic-cycle criterion.
        let cycle_repels = producer_invades_virgin
            && heterotroph_invades_lockup
            && lp_v.ln() * lc_l.ln() > (-lc_v.ln()) * (-lp_l.ln());
        AlcPersistence {
            producer_invades_virgin,
            heterotroph_invades_lockup,
            cycle_repels,
        }
    }

    /// Brute-force boundary escape from a point hugging the lockup corner.
    /// `lim inf L > 0` reads as in the A1 classifier, on the bound nutrient
    /// `θ_P·P + θ_C·C_E ≤ L`: its trough over
    /// the last quarter of the horizon is within a factor 10 of its trough over
    /// the third quarter and above an absolute floor. A tick on which a drain
    /// hits its cap leaves the unsaturated regime and is reported as such.
    pub fn classify_by_boundary_escape(&self, horizon: usize) -> NumericClass {
        let body = self.theta_p.max(self.theta_c);
        // The heterotroph inoculum must be small on the pile-cap scale
        // `1/(σ·q)` (the consumer mass that would drain the whole pile in one
        // tick) as well as on the nutrient scale, or the first tick leaves H2.
        let cap_scale = if self.sigma() > 0.0 && self.lump.q > 0.0 {
            1.0 / (self.sigma() * self.lump.q)
        } else {
            f64::INFINITY
        };
        let mut x = AlcState {
            p: 1e-3 * self.n_total / body,
            c_e: 1e-6 * (self.n_total / body).min(cap_scale),
            a: 0.0,
            phi: 0.0,
            c_n: 0.0,
        };
        x.c_n = self.n_total - self.bound(&x);
        let mut mins = [f64::INFINITY; 2];
        for t in 0..horizon {
            let (next, in_regime) = self.step_checked(&x);
            if !in_regime {
                return NumericClass::LeftRegime;
            }
            x = next;
            if t >= horizon / 2 {
                let w = if t < 3 * horizon / 4 { 0 } else { 1 };
                mins[w] = mins[w].min(self.bound(&x));
            }
        }
        if mins[1] > 1e-100 * self.n_total && mins[1] > 0.1 * mins[0] {
            NumericClass::Permanent
        } else {
            NumericClass::NotPermanent
        }
    }

    /// One tick of the coupled map.
    pub fn step(&self, x: &AlcState) -> AlcState {
        self.step_checked(x).0
    }

    /// One tick, plus whether the tick stayed in the unsaturated regime: the
    /// living drain `a·P·C_E` below the standing crop and the carcass drain
    /// `σ·C_E·C_N` below the pile (A1's H2 — a drain that hits its cap is the
    /// stepper's one-shot annihilation, where the mass-action reading stops).
    pub fn step_checked(&self, x: &AlcState) -> (AlcState, bool) {
        let m = &self.energy;
        let (p, c_e, a, phi, c_n) = (x.p, x.c_e, x.a, x.phi, x.c_n);
        let q = self.lump.q;

        // Flow 2: uptake, per-body demand α_P, capped by the pool.
        let u = (self.alpha_p * p).min(a);
        // Flows 1, 8, 9 + Liebig: energy-limited increment is A1's logistic
        // growth branch; nutrient-limited increment is the bindable free store.
        let logistic = if m.r_p > 0.0 {
            m.r_p * p * (1.0 - p / m.k_p)
        } else {
            m.r_p * p
        };
        let g_e = logistic.max(0.0);
        let shading = (-logistic).max(0.0);
        let g = g_e.min((phi + u) / self.theta_p);
        // Flow 3 on living prey (A1's bilinear term) and on carcasses (the same
        // binary-reach drain over the carcass count C_N/ν, capped by the pile).
        let d_l = (m.a * p * c_e).min(p);
        let d_c = if q > 0.0 {
            (self.sigma() * c_e * c_n).min(c_n / q)
        } else {
            0.0
        };
        let in_regime = (p == 0.0 || m.a * p * c_e < p)
            && (c_n == 0.0 || q <= 0.0 || self.sigma() * c_e * c_n < c_n / q);
        // Consumer growth: A1's β·P on prey plus the carcass return, Liebig-
        // limited by the nutrient that arrives with the drained structure.
        let energy_limited = m.beta * p * c_e + self.kappa_c_gamma * self.e_c() * d_c;
        let nutrient_arriving = self.theta_p * d_l + q * d_c;
        let dc = energy_limited.min(nutrient_arriving / self.theta_c);
        // Stoichiometric mismatch: what the consumer does not bind is excreted
        // to the available pool at the feeding site.
        let excreted = nutrient_arriving - self.theta_c * dc;
        // Flow 6: producer deaths (lumped μ_P plus shading) carry their bound
        // nutrient and their share of the free store; consumer deaths (A1's
        // removal `m`) carry θ_C per body.
        let m_p = (self.lump.mu_p * p + shading).min(p - d_l);
        let m_c = m.m * c_e;
        // The free store after this tick's growth has bound its share; the
        // dying fraction of bodies takes the same fraction of it to the carcass.
        let phi_after_growth = phi + u - self.theta_p * g;
        let free_dying = if p > 0.0 {
            phi_after_growth * (m_p / p)
        } else {
            0.0
        };
        let death_nutrient = self.theta_p * m_p + free_dying + self.theta_c * m_c;

        let p1 = p + g - d_l - m_p;
        let c_e1 = c_e + dc - m_c;
        let next = AlcState {
            p: p1,
            c_e: c_e1,
            a: a - u + excreted,
            phi: phi_after_growth - free_dying,
            c_n: c_n - q * d_c + death_nutrient,
        };
        (next, in_regime)
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
    let conv = biomass_conversion(&producer, params);
    println!(
        "  eta_P (reproductive branch)    = {:.4}  (= repro_eff*(1 - propagule)*(1 - exp(-fecundity)))",
        conv.eta
    );
    println!(
        "  chi_P (biomass conversion)     = {:.4}  (kappa_P = {:.2}, s = {:.2})",
        conv.chi, conv.kappa, conv.structure_fraction
    );
    println!("  r_P = chi_P*gamma*(F - B_P)    = {:.4}", map.r_p);
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
    println!();

    // --- Coupled A ⇌ L ⇌ C reduction (issue #437) ---
    let alc = AlcMap::derive(&producer, &consumer, params, AlcLumping::default());
    println!("# Coupled energy x nutrient reduction (issue #437) — same banner applies");
    println!("## Nutrient-side coefficients (committed) and lumpings (endogenous)");
    println!("  N_total (initial_nutrient_pool)  = {:.4}", alc.n_total);
    println!("  alpha_P (per-body uptake)        = {:.4}", alc.alpha_p);
    println!(
        "  theta_P, theta_C (demand/struct) = {:.4}, {:.4}",
        alc.theta_p, alc.theta_c
    );
    println!(
        "  kappa_C*gamma                    = {:.4}",
        alc.kappa_c_gamma
    );
    println!(
        "  e_C (kernel on the pile)         = {:.4}   [lumped: producer carcasses]",
        alc.e_c()
    );
    println!(
        "  mu_P (producer mortality)        = {:.4}   [lumped, endogenous]",
        alc.lump.mu_p
    );
    println!(
        "  q (carcass richness)             = {:.4}   [lumped, endogenous]",
        alc.lump.q
    );
    println!(
        "  nu (nutrient per carcass)        = {:.4}   [lumped, endogenous]",
        alc.lump.nu
    );
    println!(
        "  iota (reach geometry)            = {:.4}   [lumped, endogenous]",
        alc.lump.iota
    );
    println!("  sigma = a*iota/nu                = {:.4}", alc.sigma());
    println!();
    let [vp, vc] = alc.virgin_eigenvalues();
    let [lp, lc] = alc.lockup_eigenvalues();
    println!("## Dead-line ends: transverse eigenvalues (lambda_P, lambda_C)");
    println!("  virgin end (0,0,N_total,0)  : {:.4}  {:.4}", vp, vc);
    println!("  lockup corner (0,0,0,N_total): {:.4}  {:.4}", lp, lc);
    let cond = alc.persistence();
    println!("## Coupled persistence condition");
    println!(
        "  producer invades virgin pool  min(r_P, alpha_P/theta_P) = {:.4} > mu_P   {}",
        alc.energy.r_p.min(alc.alpha_p / alc.theta_p),
        cond.producer_invades_virgin
    );
    println!(
        "  Lambda = sigma*N_total*min(kappa_C*gamma*e_C, q/theta_C)/m = {:.4} > 1   {}",
        alc.lockup_escape_ratio(),
        cond.heterotroph_invades_lockup
    );
    println!(
        "  cycle repels: ln(lambda_P^v)*ln(lambda_C^L) = {:.4} > ln(1/(1-m))*ln(1/(1-mu_P)) = {:.4}   {}",
        vp.ln() * lc.ln(),
        (-vc.ln()) * (-lp.ln()),
        cond.cycle_repels
    );
    println!("  persistent (analytic) = {}", cond.permanent());
    println!(
        "  persistent (numerics) = {:?}",
        alc.classify_by_boundary_escape(20_000)
    );
    println!();
    println!(
        "## Sweep: N_total (rows, log-spaced) x base_trophic_efficiency (cols), other lumpings default"
    );
    println!(
        "   '#' persistent   '.' not persistent   'x' left the unsaturated regime   '!'/'?' disagreement"
    );
    let mut agree = 0usize;
    let mut disagree = 0usize;
    let mut left = 0usize;
    let mut near = 0usize;
    print!("  {:>10}  {:>9} ", "N_total", "Lam@eff=1");
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
    for i in 0..25 {
        let n_total = 0.1 * (1e4f64).powf(i as f64 / 24.0);
        let mut line = String::new();
        let mut lam_at_one = 0.0;
        for e in &effs {
            let mut p = params.clone();
            p.initial_nutrient_pool = n_total as f32;
            p.base_trophic_efficiency = *e;
            let m = AlcMap::derive(&producer, &consumer, &p, AlcLumping::default());
            let lam = m.lockup_escape_ratio();
            lam_at_one = lam / *e as f64;
            let analytic = m.persistence().permanent();
            let numeric = m.classify_by_boundary_escape(20_000);
            let g = match (analytic, numeric) {
                (true, NumericClass::Permanent) => '#',
                (false, NumericClass::NotPermanent) => '.',
                (_, NumericClass::LeftRegime) => 'x',
                (true, NumericClass::NotPermanent) => '!',
                (false, NumericClass::Permanent) => '?',
            };
            line.push(g);
            let on_boundary = (lam - 1.0).abs() <= 0.02;
            match g {
                'x' => left += 1,
                _ if on_boundary => near += 1,
                '!' | '?' => disagree += 1,
                _ => agree += 1,
            }
        }
        println!("  {:>10.3}  {:>9.3} {line}", n_total, lam_at_one);
    }
    println!("  cols: eff = 0.04 .. 1.00 in steps of 0.04; '|' marks 0.2, 0.4, 0.6, 0.8, 1.0");
    println!("## Agreement (coupled reduction, 20k-tick horizon)");
    println!(
        "  agree = {agree}, disagree = {disagree}, within 2% of Lambda = 1 (not scored) = {near}, left regime = {left}"
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

    /// `κ_P` splits the producer's surplus between growth and offspring; it
    /// does not decide whether the surplus becomes biomass (#466). A producer
    /// that routes everything to reproduction (`κ_P = 0`) with `ρ > 1` still
    /// has a live face — offspring are biomass the map carries — and the map
    /// is permanent when the consumer invades.
    #[test]
    fn kappa_zero_producer_with_rho_above_one_keeps_its_face_alive() {
        let mut t = producer();
        t.kappa = 0.0;
        // A modest invasion ratio so the escape orbit stays in the unsaturated
        // regime (H2) and the numerics can score it.
        let mut p = params();
        p.base_trophic_efficiency = 0.1;
        let map = Map::derive(&t, &consumer(), &p);
        assert!(map.k_p > 1.0, "rho must exceed 1 here");
        assert!(
            map.r_p > 0.0,
            "r_P = {} should be positive at kappa_P = 0",
            map.r_p
        );
        assert!(map.permanence().producer_face_alive);
        assert!(map.permanence().permanent());
        assert_eq!(map.classify_by_simulation(20_000), NumericClass::Permanent);
    }

    /// `r_P` is a `κ_P`-weighted mix of the two branches: the somatic limit
    /// (`κ_P = 1`) is the old `γ·(F − B_P)`; the reproductive branch is
    /// strictly less productive because of the committed heat on the way to
    /// an offspring (`reproduction_efficiency`, propagule cost, the empty
    /// Poisson brood); and when that branch is lossless `κ_P` drops out.
    #[test]
    fn r_p_mixes_the_somatic_and_reproductive_branches_by_kappa() {
        let base = params();
        let somatic = |kappa: f32, p: &WorldParameters| {
            let mut t = producer();
            t.kappa = kappa;
            Map::derive(&t, &consumer(), p).r_p
        };
        let b_p = Map::maintenance(&producer(), &base);
        let full = base.growth_efficiency as f64 * (base.solar_flux_magnitude as f64 - b_p);
        assert!((somatic(1.0, &base) - full).abs() < 1e-12);
        let repro_only = somatic(0.0, &base);
        assert!(repro_only > 0.0 && repro_only < full);
        assert!(somatic(0.5, &base) > repro_only && somatic(0.5, &base) < full);

        // Lossless reproductive branch: no heat at the event, no propagules,
        // every offspring embodied at birth, a brood that is never empty.
        let mut lossless = base.clone();
        lossless.reproduction_efficiency = 1.0;
        lossless.dispersal_propagule_cost_coefficient = 0.0;
        lossless.offspring_structure_fraction = 1.0;
        let mut fecund = producer();
        fecund.fecundity = 1e3;
        fecund.kappa = 0.0;
        let r_lossless = Map::derive(&fecund, &consumer(), &lossless).r_p;
        assert!(
            (r_lossless - full).abs() < 1e-9,
            "kappa-free: {r_lossless} vs {full}"
        );

        // A propagule cost that eats the whole reproductive budget makes the
        // kappa_P = 0 face genuinely dead: nothing becomes biomass.
        let mut burnt = base.clone();
        burnt.dispersal_propagule_cost_coefficient = 1.0;
        burnt.dispersal_propagule_cost_exponent = 1.0;
        let mut broadcaster = producer();
        broadcaster.dispersal = 1.0;
        broadcaster.kappa = 0.0;
        assert_eq!(Map::derive(&broadcaster, &consumer(), &burnt).r_p, 0.0);
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

    // ---- Coupled A ⇌ L ⇌ C reduction (issue #437) ----

    fn alc() -> AlcMap {
        AlcMap::derive(&producer(), &consumer(), &params(), AlcLumping::default())
    }

    /// The coupled map is a closed nutrient ledger: `A + L + C_N = N_total`
    /// after every tick (world-rules conservation), and the lockup corner
    /// `(P, C_E, A, C_N) = (0, 0, 0, N_total)` is a fixed point — no passive
    /// decay, so a dead world stays exactly where it died.
    #[test]
    fn coupled_map_conserves_nutrient_and_lockup_corner_is_fixed() {
        let map = alc();
        let n_total = map.n_total;
        let mut x = AlcState {
            p: 2.0,
            c_e: 0.5,
            a: 0.4 * n_total,
            phi: 0.0,
            c_n: 0.1 * n_total,
        };
        x.phi = n_total - x.a - x.c_n - map.bound(&x);
        assert!(x.phi > 0.0);
        for _ in 0..500 {
            x = map.step(&x);
            let l = map.living(&x);
            assert!(l >= -1e-9, "living nutrient went negative: {l}");
            assert!(
                (x.a + l + x.c_n - n_total).abs() < 1e-9 * n_total,
                "ledger drift: A={} L={l} C_N={} N_total={n_total}",
                x.a,
                x.c_n
            );
        }
        let corner = AlcState {
            p: 0.0,
            c_e: 0.0,
            a: 0.0,
            phi: 0.0,
            c_n: n_total,
        };
        let next = map.step(&corner);
        assert_eq!(next, corner, "lockup corner should be a fixed point");
    }

    /// With no heterotroph the nutrient cycle is one-way (`A → L → C_N`, no
    /// passive decay): a viable producer takes the pool up, dies at `μ_P`, and
    /// every death strands nutrient. The producer-only face therefore flows
    /// into the lockup corner — `C_N → N_total`, `L → 0` — whatever the flux.
    /// This is world-rules' "a world without decomposers accumulates resources
    /// in the dead pool until the living system starves", as a limit.
    #[test]
    fn producer_only_face_silts_into_the_lockup_corner() {
        for flux in [10.0f32, 100.0, 1000.0] {
            let mut p = params();
            p.solar_flux_magnitude = flux;
            p.initial_nutrient_pool = 50.0;
            let map = AlcMap::derive(&producer(), &consumer(), &p, AlcLumping::default());
            assert!(map.energy.r_p > 0.0, "producer must be energy-viable here");
            let mut x = AlcState {
                p: 1.0,
                c_e: 0.0,
                a: map.n_total - map.theta_p,
                phi: 0.0,
                c_n: 0.0,
            };
            let mut c_n_prev = x.c_n;
            for _ in 0..20_000 {
                x = map.step(&x);
                assert!(
                    x.c_n >= c_n_prev - 1e-12,
                    "carcass pool is monotone without decomposers"
                );
                c_n_prev = x.c_n;
            }
            assert_eq!(x.c_e, 0.0);
            assert!(
                x.p < 1e-6,
                "flux {flux}: producers should die out, P = {}",
                x.p
            );
            assert!(
                map.living(&x) < 1e-6 * map.n_total,
                "flux {flux}: living nutrient should vanish, L = {}",
                map.living(&x)
            );
            assert!(
                (x.c_n - map.n_total).abs() < 1e-6 * map.n_total,
                "flux {flux}: all nutrient should be carcass-locked, C_N = {} of {}",
                x.c_n,
                map.n_total
            );
        }
    }

    /// The lockup repeller condition is a statement about the corner's
    /// transverse spectrum: in `(P, C_E)` the Jacobian at
    /// `(0, 0, 0, N_total)` is `diag(1 − μ_P, λ_C(𝓛))` with
    /// `λ_C(𝓛) = 1 + σ·N_total·min(κ_C·γ·e_C, q/θ_C) − m`. A central-difference
    /// Jacobian of `step` at the corner must reproduce both entries — the
    /// producer cannot invade (no uptake from an empty pool), only the
    /// heterotroph feeding on the pile can.
    #[test]
    fn lockup_corner_eigenvalues_match_finite_difference_jacobian() {
        for (eff, n_total) in [(0.5f32, 5.0f32), (0.05, 5.0), (0.5, 0.5), (1.0, 50.0)] {
            let mut p = params();
            p.base_trophic_efficiency = eff;
            p.initial_nutrient_pool = n_total;
            let map = AlcMap::derive(&producer(), &consumer(), &p, AlcLumping::default());
            let corner = AlcState {
                p: 0.0,
                c_e: 0.0,
                a: 0.0,
                phi: 0.0,
                c_n: map.n_total,
            };
            let h = 1e-7;
            let dp = map.step(&AlcState { p: h, ..corner });
            let dc = map.step(&AlcState { c_e: h, ..corner });
            let [lambda_p, lambda_c] = map.lockup_eigenvalues();
            assert!(
                (dp.p / h - lambda_p).abs() < 1e-6 && dp.c_e.abs() < 1e-12,
                "eff {eff} N {n_total}: producer column {} vs {lambda_p}",
                dp.p / h
            );
            assert!(
                (dc.c_e / h - lambda_c).abs() < 1e-6 && dc.p.abs() < 1e-12,
                "eff {eff} N {n_total}: consumer column {} vs {lambda_c}",
                dc.c_e / h
            );
            assert!(lambda_p < 1.0, "producer never invades the lockup corner");
            assert_eq!(map.lockup_escape_ratio() > 1.0, lambda_c > 1.0);
        }
    }

    /// Persistence of `L`, checked by boundary escape from a point hugging the
    /// lockup corner (tiny producer and heterotroph inocula, all nutrient in
    /// the pile). Sweeping `Λ` through 1 via `base_trophic_efficiency` at
    /// fixed `N_total`: below 1 the living nutrient decays to zero (the corner
    /// attracts); above 1, with the virgin-end clause and the heteroclinic
    /// cycle condition also holding, the trough of `L` stops shrinking.
    #[test]
    fn living_nutrient_persists_iff_the_coupled_condition_holds() {
        let mut checked = 0usize;
        for i in 0..40 {
            let eff = 0.01 + 0.99 * i as f32 / 39.0;
            let mut p = params();
            p.base_trophic_efficiency = eff;
            p.initial_nutrient_pool = 2.0;
            let map = AlcMap::derive(&producer(), &consumer(), &p, AlcLumping::default());
            let cond = map.persistence();
            if (map.lockup_escape_ratio() - 1.0).abs() <= 0.02 {
                continue;
            }
            match map.classify_by_boundary_escape(20_000) {
                NumericClass::LeftRegime => continue,
                NumericClass::Permanent => assert!(
                    cond.permanent(),
                    "eff {eff}: numerics persistent, analytic not: {cond:?}"
                ),
                NumericClass::NotPermanent => assert!(
                    !cond.permanent(),
                    "eff {eff}: analytic persistent, numerics not: {cond:?}"
                ),
            }
            checked += 1;
        }
        assert!(
            checked >= 20,
            "sweep should score most cells, got {checked}"
        );
    }

    /// The `C_N = 0, A → 0` limit of the reduction: all nutrient is living and
    /// bound, none is available. Uptake is `min(α_P·P, A) = 0`, so the Liebig
    /// minimum closes producer growth at zero *whatever the solar flux* — the
    /// map's living state is frozen at the nutrient it holds, and the only
    /// condition left is the static one on `L = N_total` (the energy-death
    /// floor). Energy abundance cannot rescue a nutrient-starved world.
    #[test]
    fn nutrient_starved_face_freezes_growth_regardless_of_flux() {
        for flux in [1.0f32, 10.0, 1000.0, 1e6] {
            let mut p = params();
            p.solar_flux_magnitude = flux;
            let map = AlcMap::derive(&producer(), &consumer(), &p, AlcLumping::default());
            let x = AlcState {
                p: 3.0,
                c_e: 0.0,
                a: 0.0,
                phi: 0.0,
                c_n: 0.0,
            };
            let next = map.step(&x);
            assert!(
                next.p <= x.p * (1.0 - map.lump.mu_p) + 1e-12,
                "flux {flux}: no growth without available nutrient, got P' = {}",
                next.p
            );
            assert_eq!(next.a, 0.0);
            assert!(next.phi.abs() < 1e-12);
        }
    }

    /// The one pure-parameter kill: `base_trophic_efficiency = 0` zeroes the
    /// kernel on the carcass pool, so `Λ = 0` for every value of the
    /// endogenous terms (reach, richness, carcass count, cluster traits) and
    /// every `N_total` — the lockup corner attracts. This is the degenerate
    /// corner viability.md's decomposer-return finding already names.
    #[test]
    fn zero_trophic_efficiency_makes_lockup_certain_for_every_lumping() {
        for (iota, q, nu, n_total) in [
            (1.0, 0.22, 0.22, 5.0),
            (1.0, 10.0, 0.01, 1e6),
            (0.5, 0.05, 1.0, 100.0),
        ] {
            let mut p = params();
            p.base_trophic_efficiency = 0.0;
            p.initial_nutrient_pool = n_total;
            let lump = AlcLumping {
                iota,
                q,
                nu,
                ..AlcLumping::default()
            };
            let map = AlcMap::derive(&producer(), &consumer(), &p, lump);
            assert_eq!(map.lockup_escape_ratio(), 0.0);
            assert!(!map.persistence().heterotroph_invades_lockup);
            assert_eq!(
                map.classify_by_boundary_escape(2_000),
                NumericClass::NotPermanent
            );
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
