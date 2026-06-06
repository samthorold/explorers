//! Closed-form *distance-to-bifurcation* readings of a decoded config — the
//! production lift of Research Brief F's two validated dynamics indicators
//! (#358 Hopf / Neimark–Sacker, #359 evolutionary branching) into the genesis
//! search.
//!
//! Symmetric to [`crate::prefilter`]: a search-side closed-form *reading* of
//! decoded [`WorldParameters`] (+ the founder mean trait vector), **never** part
//! of the stepper. It reuses the committed kernel
//! [`explorers_sim::trophic_transfer_efficiency`] and [`TraitVector::distance`]
//! verbatim, and replicates the mean-field per-capita growth `g` and the
//! maintenance lumping the two throwaway spikes already do
//! (`crates/explorers-sim/src/bin/hopf_prototype.rs`,
//! `crates/explorers-sim/src/bin/branching_detector.rs`), each line citing the
//! committed phase it reads.
//!
//! ## Authority boundary (Brief F, AC3) — load-bearing
//!
//! These two scalars arbitrate **existence/stability only**: whether a coexistence
//! interior point is invadable (branching direction) and whether the
//! living-mass↔available-pool coupling has a stable fixed point or a limit cycle
//! (Hopf). They must **never** be read for the decomposer guild or any per-seed
//! distributional property — that is the boundary [`viability.md`] already draws
//! around `C*`. Accordingly they enter the atlas as **per-cell descriptors plus a
//! predicted-vs-observed cross-check**, never summed into fitness and never a
//! binning axis (see `docs/system-design/genesis-search.md`).
//!
//! ## Validated only in restricted regimes — descriptors, not objectives
//!
//! Both #358 and #359 returned a *Qualified GO* that explicitly gates
//! objective-promotion on hardening the genesis observables — #358's
//! `oscillation_strength` is flat and demographic-pulsing-dominated at search
//! scale, #359's `clustering_strength` silently zeroes below n=4 — work this
//! module does **not** do. So the readings are descriptors. The cross-check in
//! [`crate::qd`] tags each disagreement with a regime so a mismatch in the
//! known-weak-observable regime localises to the observable/geometry, not F's
//! spectral reading.
//!
//! ## Reduced coordinates for a single-founder QD config
//!
//! A QD config carries one founder *mean* trait vector — no producer/consumer
//! pair (that is emergent). So:
//! - **Branching `D`** is computed on the founder mean directly: a rare
//!   heterotroph is swept along the trophic axis against the founder-as-producer
//!   in the founder-monoculture environment (the #359 invasion-fitness margin).
//! - **Oscillation `|λ|−1`** uses Brief F AC1's self-contained
//!   **living-biomass ↔ available-pool** coupling (the producer↔consumer 2-cluster
//!   form the Hopf prototype used needs two clusters a QD config lacks, and #358
//!   showed that observable decouples at search scale anyway).

use explorers_genesis::WorldParameters;
use explorers_sim::TraitVector;

/// Reference body mass/structure for the mean-field reduction — per-capita fluxes
/// are charged against one reference body of this size (mirrors the spikes'
/// `REFERENCE_BODY_MASS` / `REF_STRUCTURE`). The *sign* of both readings is
/// invariant to this constant; only the magnitude scales — a known degeneracy of
/// any mean-field lumping, called out in both spike verdicts.
const REFERENCE_BODY_MASS: f32 = 1.0;

/// Effective trait value in the mean field: clamped non-negative (the committed
/// `effective_trait_with_steepness` reduces to this with no wear — and wear is off
/// for every searched config).
fn eff(v: f32) -> f32 {
    v.max(0.0)
}

/// Per-tick maintenance of one reference body with the given traits — the
/// metabolise phase (`phase.rs:156`) charged on a unit body, superlinear in each
/// specification trait via `maintenance_cost_exponent`. The superset of the two
/// spikes' maintenance formulas (Hopf omitted the asexual term; branching kept
/// it) — every committed per-trait maintenance cost is charged.
fn maintenance(t: &TraitVector, p: &WorldParameters) -> f32 {
    let exp = p.maintenance_cost_exponent;
    p.base_metabolic_rate
        + eff(t.photosynthetic_absorption).powf(exp) * p.photo_maintenance_cost
        + eff(t.heterotrophy).powf(exp) * p.heterotrophy_maintenance_cost
        + eff(t.mobility).powf(exp) * p.mobility_maintenance_cost
        + eff(t.asexual_propensity).powf(exp) * p.asexual_propensity_maintenance_cost
        + REFERENCE_BODY_MASS * p.structure_maintenance_coefficient
}

// ===========================================================================
// Oscillation axis — Hopf / Neimark–Sacker on the living-mass ↔ available-pool
// coupling (lift of hopf_prototype.rs, #358).
// ===========================================================================

/// The lumped coefficients of the 3-compartment available-pool↔reserve↔living-mass
/// map, each derived from committed fluxes. This is the Hopf prototype's
/// [`Compartments`] reparametrised for a single founder: the **available pool `A`**
/// (the resource, role `P`) is filled/cycled by the founder's photosynthetic
/// production and bounded by the metabolic ceiling, the **consumer reserve `R`**
/// receives the bilinear trophic draw and mobilises it into growth at the flow-9
/// conductance rate, and the **living mass `M`** (the consumer, role `C`) is built
/// from that mobilised flow. Both sides are the same founder biomass, so the
/// trait-space distance is `0` and the committed kernel sits at its maximum — the
/// self-contained coupling F AC1 names.
///
/// The discrete map (`f = reserve_mobilisation_rate`, `ρb` the retention buffer):
/// ```text
///   A' = A + r_P · A · (1 − A/K_P)  −  a · A · M
///   R' = (1 − f) · (R + e · a · A · M)  +  f · ρb
///   M' = M + κ · γ · f · (R + e · a · A · M − ρb)  −  m · M
/// ```
/// The reserve sits **only on the consumer growth pathway** (the income→structure
/// flow that flow 9's conductance governs): the producer's photosynthetic income
/// refills reserve every tick, so the pool-fill diagonal `r_P` is f-invisible, and
/// maintenance stays lumped on the `m · M` decay rather than drawn from reserve —
/// both are what make the map collapse **exactly** to the prototype's
/// 2-compartment form at `f = 1` (reserve is slaved within one tick: `R' = ρb`,
/// `R + e·a·A·M − ρb = e·a·A·M`, so `M'` recovers `M + κγe·a·A·M − m·M`). At
/// `f < 1` reserve is a genuine slow mode (relaxation `~f`) whose lag shifts the
/// Hopf boundary; the interior fixed point `(A*, M*)` is unchanged (reserve
/// conservation fixes throughput at net income — see [`g`] and
/// `docs/system-design/genesis-search.md`).
struct PoolCoupling {
    /// Available-pool intrinsic recovery: the founder's net photosynthetic
    /// production cycles nutrient through living mass back to the pool — the
    /// `κ·γ·(flux − maintenance)` selection diagonal (photosynthesise + metabolise
    /// + grow), the same low-density rate the Hopf spike reads for its producer.
    r_p: f32,
    /// Light-saturated pool ceiling `flux ÷ per-biomass maintenance` — the biomass
    /// at which the density-dependent light share just meets the metabolic floor.
    k_p: f32,
    /// Mass-action draw rate: living mass draws the pool down at the founder's
    /// consumption capacity `eff_heterotrophy` (resolve_drains' binary-reach drain),
    /// per unit reference biomass. Zero for a pure autotroph — then the loop is
    /// direct uptake with no bilinear feedback (no Hopf).
    a: f32,
    /// Living-mass per-tick removal rate: the founder's metabolic floor it must
    /// out-take or decline toward the death threshold (metabolise → death).
    m: f32,
    /// Somatic allocation κ (grow phase, kappa split) — the founder's own κ.
    kappa: f32,
    /// Energy→structure conversion `growth_efficiency` (grow phase).
    gamma: f32,
    /// Distance-independent part of the trophic kernel exponent `exp(−decay·d)`;
    /// `d = 0` (same biomass on both sides of the coupling) ⇒ `e_dist = 1`.
    e_dist: f32,
}

impl PoolCoupling {
    fn from_founder(founder: &TraitVector, p: &WorldParameters) -> Self {
        let kappa = founder.kappa.clamp(0.0, 1.0);
        let gamma = p.growth_efficiency;
        let b = maintenance(founder, p);
        let flux = p.solar_flux_magnitude;

        // Pool-fill (producer) diagonal: a productive founder cycles the pool fast.
        // This is f-invariant (#387): the reserve mobilisation rate `f` (flow 9) is
        // invisible on the producer pathway — photosynthetic income refills reserve
        // every tick regardless of draw-down speed, so the pool-fill rate is not
        // slowed by a bounded mobilisation flow. The conductance factor enters the
        // Hopf reading only through the *consumer* reserve compartment in
        // `jacobian` (the income→structure growth flow), never here.
        let r_p = kappa * gamma * (flux - b).max(0.0) / REFERENCE_BODY_MASS;
        let k_p = if b > 0.0 { flux / b } else { f32::INFINITY };

        // Living-mass draw (consumer) bilinear: the founder's consumption rate.
        // Binary-reach drain (#380): a co-located cluster drains at its full
        // effective heterotrophy each tick, so no contact-duration factor enters.
        let eff_het = eff(founder.heterotrophy);
        let a = eff_het / REFERENCE_BODY_MASS;

        let m = b / REFERENCE_BODY_MASS;

        // Same biomass both sides ⇒ trait distance 0 ⇒ kernel at its maximum.
        let e_dist = (-p.trophic_distance_decay * 0.0).exp();

        PoolCoupling {
            r_p,
            k_p,
            a,
            m,
            kappa,
            gamma,
            e_dist,
        }
    }

    /// Trophic transfer efficiency `e = base_trophic_efficiency · exp(−decay·d)` at
    /// the config's own `base_trophic_efficiency` (no sweep — the founder is read
    /// at its actual parameters).
    fn e(&self, p: &WorldParameters) -> f32 {
        p.base_trophic_efficiency * self.e_dist
    }

    /// Living-mass conversion coefficient β = κ · γ · e · a.
    fn beta(&self, p: &WorldParameters) -> f32 {
        self.kappa * self.gamma * self.e(p) * self.a
    }

    /// Interior (coexistence) fixed point `(A*, M*)`, or `None` if none exists
    /// (a pure autotroph `a ≤ 0`, or `A* ≥ K_P` so living mass cannot persist).
    /// Verbatim the Hopf prototype's `interior_fixed_point`.
    fn interior_fixed_point(&self, p: &WorldParameters) -> Option<(f32, f32)> {
        let beta = self.beta(p);
        if beta <= 0.0 || self.a <= 0.0 {
            return None;
        }
        let a_star = self.m / beta; // from M-nullcline: β·A* = m
        if !(a_star > 0.0 && a_star < self.k_p) {
            return None;
        }
        let m_star = (self.r_p / self.a) * (1.0 - a_star / self.k_p);
        if m_star <= 0.0 {
            return None;
        }
        Some((a_star, m_star))
    }

    /// The 3×3 Jacobian of the discrete map at the interior fixed point, rows/cols
    /// ordered `(A, M, R)` — available pool, living mass, consumer reserve.
    ///
    /// The reserve row is what the flow-9 conductance factor `f`
    /// (`reserve_mobilisation_rate`) enters: trophic income `e·a·A·M` is received
    /// into reserve and only the mobilised flow `f·(R − ρb)` leaves it to build
    /// structure, all within one tick (income is added before mobilisation, as the
    /// stepper's photosynthesise→metabolise→grow order does). So reserve relaxes at
    /// rate `f` and feeds the living-mass growth with the same factor:
    /// ```text
    ///   ∂M'/∂A = β·f·M*      ∂M'/∂M = 1 − m + β·f·A*      ∂M'/∂R = κγ·f
    ///   ∂R'/∂A = (1−f)·e·a·M* ∂R'/∂M = (1−f)·e·a·A*        ∂R'/∂R = 1 − f
    /// ```
    /// At `f = 1` the reserve row's first two entries and its diagonal all vanish,
    /// the reserve eigenvalue collapses to `0`, and the leading 2×2 block recovers
    /// the prototype's frozen↔oscillation reading exactly. At `f < 1` reserve is a
    /// genuine slow third mode (eigenvalue `≈ 1 − f`) whose lag shifts the Hopf
    /// boundary even though the interior fixed point `(A*, M*)` does not move (it is
    /// `f`-invariant — reserve conservation fixes throughput at net income; see the
    /// note in [`g`] and `docs/system-design/genesis-search.md`).
    fn jacobian(&self, p: &WorldParameters) -> Option<[[f32; 3]; 3]> {
        let (a_star, m_star) = self.interior_fixed_point(p)?;
        let beta = self.beta(p);
        let e = self.e(p);
        let f = p.reserve_mobilisation_rate;
        let kg = self.kappa * self.gamma;

        // Row A (available pool): logistic recovery − consumer draw; reserve does
        // not enter the pool equation, so ∂A'/∂R = 0.
        let jaa = 1.0 + self.r_p - 2.0 * self.r_p * a_star / self.k_p - self.a * m_star;
        let jam = -self.a * a_star;
        // Row M (living mass): structure is built from the mobilised flow S = f·(R−ρb).
        let jma = beta * f * m_star;
        let jmm = 1.0 - self.m + beta * f * a_star;
        let jmr = kg * f;
        // Row R (consumer reserve): trophic income in, mobilised flow out; the 1−f
        // diagonal is the conductance relaxation (the slow mode f introduces).
        let jra = (1.0 - f) * e * self.a * m_star;
        let jrm = (1.0 - f) * e * self.a * a_star;
        let jrr = 1.0 - f;

        Some([[jaa, jam, 0.0], [jma, jmm, jmr], [jra, jrm, jrr]])
    }
}

/// The spectral radius (leading `|λ|`) of a 3×3 Jacobian: the largest modulus
/// over the three roots of its characteristic cubic, solved in closed form
/// (Cardano). A Neimark–Sacker / discrete-Hopf crossing is a complex-conjugate
/// pair leaving the unit circle, so the modulus must be read over *complex*
/// roots — a real-only method (power iteration) cannot see the pair. The cubic
/// is solved in `f64` for conditioning and the result returned as `f32`. The
/// coupling carries an explicit reserve compartment (the flow-9 conductance mode,
/// [`PoolCoupling::jacobian`]), so the Jacobian is 3×3 rather than the prototype's
/// 2×2; at `reserve_mobilisation_rate = 1` the reserve eigenvalue collapses to `0`
/// and this reduces to the leading 2×2 modulus.
fn spectral_radius(j: &[[f32; 3]; 3]) -> f32 {
    // Characteristic polynomial of `j`:  λ³ − c2·λ² + c1·λ − c0,
    // with c2 = trace, c1 = sum of principal 2×2 minors, c0 = determinant.
    let m = |a: f64, b: f64, c: f64, d: f64| a * d - b * c;
    let (j00, j01, j02) = (j[0][0] as f64, j[0][1] as f64, j[0][2] as f64);
    let (j10, j11, j12) = (j[1][0] as f64, j[1][1] as f64, j[1][2] as f64);
    let (j20, j21, j22) = (j[2][0] as f64, j[2][1] as f64, j[2][2] as f64);
    let c2 = j00 + j11 + j22;
    let c1 = m(j11, j12, j21, j22) + m(j00, j02, j20, j22) + m(j00, j01, j10, j11);
    let c0 =
        j00 * m(j11, j12, j21, j22) - j01 * m(j10, j12, j20, j22) + j02 * m(j10, j11, j20, j21);
    let r = max_modulus_cubic(c2, c1, c0);
    if r.is_finite() { r as f32 } else { 0.0 }
}

/// Largest root modulus of the monic cubic `λ³ − c2·λ² + c1·λ − c0` (Cardano).
/// Depress to `t³ + p·t + q` via `λ = t + c2/3`; a positive discriminant
/// `4p³ + 27q²` gives one real root and a complex-conjugate pair (the Hopf case),
/// otherwise three real roots read by the trigonometric form. All in `f64`.
fn max_modulus_cubic(c2: f64, c1: f64, c0: f64) -> f64 {
    let shift = c2 / 3.0;
    let p = c1 - c2 * c2 / 3.0;
    let q = -2.0 * c2 * c2 * c2 / 27.0 + c2 * c1 / 3.0 - c0;

    let disc = 4.0 * p * p * p + 27.0 * q * q;
    if disc > 0.0 {
        // One real root + a complex-conjugate pair.
        let s = (q * q / 4.0 + p * p * p / 27.0).sqrt();
        let u = (-q / 2.0 + s).cbrt();
        let v = (-q / 2.0 - s).cbrt();
        let real_root = (u + v + shift).abs();
        let re = -(u + v) / 2.0 + shift;
        let im = 3.0_f64.sqrt() / 2.0 * (u - v);
        let pair_mod = (re * re + im * im).sqrt();
        real_root.max(pair_mod)
    } else if p.abs() < 1e-12 {
        // Degenerate (triple root): t³ + q ≈ 0.
        ((-q).cbrt() + shift).abs()
    } else {
        // Three real roots (trigonometric form; p < 0 here since disc ≤ 0).
        let scale = 2.0 * (-p / 3.0).sqrt();
        let arg = (3.0 * q / (p * scale)).clamp(-1.0, 1.0);
        let theta = arg.acos() / 3.0;
        let mut best = 0.0_f64;
        for k in 0..3 {
            let t = scale * (theta - 2.0 * std::f64::consts::PI * k as f64 / 3.0).cos();
            best = best.max((t + shift).abs());
        }
        best
    }
}

/// Signed distance to the **frozen↔oscillation** (Neimark–Sacker / discrete Hopf)
/// boundary of the available-pool↔reserve↔living-mass coupling: `|λ| − 1` of the
/// 3×3 Jacobian at the interior fixed point.
///
/// - `< 0` ⇒ the interior point is a stable node/spiral — **frozen**;
/// - `> 0` ⇒ a limit cycle — **oscillation**;
/// - `0` at the boundary, and `0` when no interior coexistence point exists (a
///   pure autotroph has no consumer-resource loop to oscillate) or the reading is
///   non-finite (NaN-guarded).
///
/// Sign convention matches the spike (`|λ|` crossing the unit circle). Conductance-
/// aware (#387): the reserve compartment carries the flow-9 mobilisation rate `f`,
/// so the boundary shifts with `f` while the fixed point stays put — at `f = 1`
/// this reduces exactly to the prototype's 2×2 reading.
pub fn oscillation_distance(params: &WorldParameters, founder_mean: &TraitVector) -> f32 {
    let coupling = PoolCoupling::from_founder(founder_mean, params);
    let distance = match coupling.jacobian(params) {
        Some(j) => spectral_radius(&j) - 1.0,
        None => 0.0,
    };
    if distance.is_finite() { distance } else { 0.0 }
}

// ===========================================================================
// Branching axis — monoculture↔coexistence invasion margin (lift of
// branching_detector.rs, #359).
// ===========================================================================

/// The frozen environment the founder monoculture sets: the light-competition
/// background and the resident roster a rare mutant competes with, eats, and is
/// eaten by. Held fixed while the mutant trait is swept (the adaptive-dynamics
/// rare-mutant approximation). Verbatim the branching spike's `Environment`, with
/// the single founder as the sole resident.
struct Environment {
    /// Σ over residents of `mass · eff_autotrophy · REF` — the density-dependent
    /// light-share denominator (photosynthesise, `phase.rs:19`).
    light_background: f32,
    /// The resident roster (trait + mass): prey for the mutant and predators of it.
    residents: Vec<(TraitVector, f32)>,
}

impl Environment {
    /// The environment a single founder monoculture sets. `mass` is the
    /// founder-monoculture standing-mass proxy (the demographic-equilibrium
    /// stand-in the spike proxies by cluster member count).
    fn founder_monoculture(founder: &TraitVector, mass: f32) -> Self {
        Environment {
            light_background: mass * eff(founder.photosynthetic_absorption) * REFERENCE_BODY_MASS,
            residents: vec![(*founder, mass)],
        }
    }
}

/// The committed per-capita net growth rate `g(θ; E)` in the frozen resident
/// environment — the AC2 operator diagonal assembled from real phases. Verbatim
/// the branching spike's `g`:
/// - photosynthesise (`phase.rs:19`): `flux · density-dependent light share`;
/// - resolve_drains (`phase.rs:386`) + the committed kernel
///   ([`explorers_sim::trophic_transfer_efficiency`]): bilinear trophic intake from
///   eating residents, and predation loss from being eaten — the disruptive
///   frequency-dependent feedback;
/// - metabolise (`phase.rs:156`): the superlinear maintenance floor;
/// - grow (`phase.rs:189`): the κ·γ conversion of net energy into structure.
///
/// Nutrient is treated as non-limiting (the searched configs seed a large pool),
/// so growth is energy-limited — a documented mean-field simplification of the
/// absorb/grow Liebig co-limitation.
fn g(theta: &TraitVector, env: &Environment, p: &WorldParameters) -> f32 {
    let kappa = theta.kappa.clamp(0.0, 1.0);
    let gamma = p.growth_efficiency;

    let w = eff(theta.photosynthetic_absorption) * REFERENCE_BODY_MASS;
    let income_photo = if w > 0.0 {
        p.solar_flux_magnitude * w / (w + env.light_background)
    } else {
        0.0
    };

    let eff_het = eff(theta.heterotrophy);
    let mut income_trophic = 0.0;
    let mut pred_loss = 0.0;
    for (r_traits, r_mass) in &env.residents {
        if eff_het > 0.0 {
            let te = explorers_sim::trophic_transfer_efficiency(theta, r_traits, p);
            income_trophic += r_mass * eff_het * te;
        }
        let r_het = eff(r_traits.heterotrophy);
        if r_het > 0.0 {
            let te = explorers_sim::trophic_transfer_efficiency(r_traits, theta, p);
            pred_loss += r_mass * r_het * te;
        }
    }

    let cost = maintenance(theta, p);
    let net_energy = income_photo + income_trophic - cost;
    // This mean-field reproduction/growth term is the per-tick selection diagonal
    // `kappa·gamma·net_energy`, and it is f-invariant — correct for every
    // `reserve_mobilisation_rate` (#387), so the conductance factor `f` is
    // deliberately absent. The reason is reserve conservation, not a coincidence:
    // at any reserve fixed point the mobilised growth flow per tick equals net
    // income `I − b` (inflow must balance outflow when reserve returns to the same
    // value each tick), so `f` only sets the reserve level `R* = ρb + (I − b)/f`
    // and the relaxation timescale `~1/f`, never the steady-state throughput. The
    // branching margin is an invasion-fitness reading — an asymptotic quantity read
    // after the mutant's reserve has relaxed — so it sees that steady-state
    // throughput. A naïve scalar `f` on `net_energy` would therefore be *wrong*
    // (the intuition that f<1 "slows the diagonal" holds only for the transient,
    // which the descriptor does not read). The f-dependence is real but confined to
    // *stability*: it enters the Hopf reading through the reserve compartment in
    // `PoolCoupling::jacobian`. See `docs/system-design/genesis-search.md`.
    kappa * gamma * net_energy - pred_loss
}

/// Linear interpolation of the spec subspace (+kappa) between two trait vectors —
/// the trophic-axis sweep. Verbatim the branching spike's `lerp_spec`.
fn lerp_spec(from: &TraitVector, to: &TraitVector, t: f32) -> TraitVector {
    let mut out = *from;
    out.photosynthetic_absorption = from.photosynthetic_absorption
        + t * (to.photosynthetic_absorption - from.photosynthetic_absorption);
    out.heterotrophy = from.heterotrophy + t * (to.heterotrophy - from.heterotrophy);
    out.mobility = from.mobility + t * (to.mobility - from.mobility);
    out.kappa = from.kappa + t * (to.kappa - from.kappa);
    out
}

/// The consumer niche a rare mutant would occupy if it could invade the founder
/// monoculture — the branching spike's canonical heterotroph target (no consumer
/// is seeded in a single-founder QD config, so the niche is constructed from the
/// founder with the spec dims set to a heterotroph).
fn consumer_target(founder: &TraitVector) -> TraitVector {
    let mut t = *founder;
    t.photosynthetic_absorption = 0.0;
    t.heterotrophy = 0.5;
    t.mobility = 0.3;
    t.kappa = 0.3;
    t
}

/// Signed **distance-to-branching** `D` — the invasibility margin of the
/// complementary (consumer) niche against the founder monoculture (the #359
/// adaptive-dynamics invasion-fitness margin):
///
/// `D = max over the heterotroph half (t ≥ 0.5) of g(y(t); E) − g(founder; E)`,
///
/// where `y(t)` sweeps the trophic axis from the founder-as-producer (`t = 0`) to
/// the constructed consumer niche (`t = 1`) in the founder-monoculture
/// environment `E`.
///
/// - `> 0` ⇒ a rare consumer can invade ⇒ a second peak nucleates — **coexistence**;
/// - `< 0` ⇒ the consumer niche is closed — **monoculture**;
/// - `0` at the bifurcation. NaN-guarded to `0`.
pub fn branching_distance(params: &WorldParameters, founder_mean: &TraitVector) -> f32 {
    // Founder-monoculture standing mass proxy (the demographic-equilibrium
    // stand-in the spike proxies by member count). Searched configs seed ≥ 1.
    let mass = (params.initial_population_size as f32).max(1.0);
    let env = Environment::founder_monoculture(founder_mean, mass);
    let target = consumer_target(founder_mean);
    let baseline = g(founder_mean, &env, params);

    let steps = 20;
    let mut margin = f32::NEG_INFINITY;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        if t < 0.5 {
            continue;
        }
        let y = lerp_spec(founder_mean, &target, t);
        let s_rel = g(&y, &env, params) - baseline;
        if s_rel > margin {
            margin = s_rel;
        }
    }
    if margin.is_finite() { margin } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{decode, default_ranges};

    fn baseline_params() -> WorldParameters {
        let ranges = default_ranges();
        let unit = vec![0.5_f64; ranges.len()];
        decode(&unit, &ranges).0
    }

    fn founder(photo: f32, het: f32, kappa: f32) -> TraitVector {
        TraitVector {
            photosynthetic_absorption: photo,
            heterotrophy: het,
            mobility: 0.0,
            kappa,
            fecundity: 0.35,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        }
    }

    // --- Branching D ---------------------------------------------------------

    #[test]
    fn branching_distance_sign_flips_between_coexistence_and_monoculture_poles() {
        // Coexistence pole: a productive autotroph founder with a high trophic
        // efficiency and a shallow distance decay — a rare heterotroph extracts a
        // lot from the standing producer biomass, so the consumer niche is open
        // (D > 0). Monoculture pole: the same founder but with the trophic niche
        // shut — near-zero efficiency and a steep decay starve the would-be
        // consumer (D < 0).
        let producer = founder(0.7, 0.0, 0.5);

        let mut coexist = baseline_params();
        coexist.base_trophic_efficiency = 0.9;
        coexist.trophic_distance_decay = 0.2;
        coexist.heterotrophy_maintenance_cost = 0.001;
        let d_coexist = branching_distance(&coexist, &producer);

        let mut mono = baseline_params();
        mono.base_trophic_efficiency = 0.1;
        mono.trophic_distance_decay = 5.0;
        mono.heterotrophy_maintenance_cost = 0.1;
        let d_mono = branching_distance(&mono, &producer);

        assert!(
            d_coexist > 0.0,
            "coexistence pole should read D > 0, got {d_coexist}"
        );
        assert!(
            d_mono < 0.0,
            "monoculture pole should read D < 0, got {d_mono}"
        );
    }

    #[test]
    fn branching_distance_is_finite_across_a_coarse_decode_sweep() {
        let ranges = default_ranges();
        for step in 0..=4 {
            let u = step as f64 / 4.0;
            let (params, dist) = decode(&vec![u; ranges.len()], &ranges);
            let d = branching_distance(&params, &dist.mean_traits);
            assert!(d.is_finite(), "D must be finite at unit {u}, got {d}");
        }
    }

    #[test]
    fn branching_distance_matches_the_spike_reading_of_a_rich_producer_niche() {
        // Regression anchor: on a rich, low-decay producer monoculture the #359
        // spike reads the consumer niche as OPEN (a heterotroph mutant taps the
        // standing producer biomass — its `heterotroph_mutant_taps_the_producer_niche`
        // test). The production lift must reproduce that direction: D > 0.
        let mut p = baseline_params();
        p.base_trophic_efficiency = 0.8;
        p.trophic_distance_decay = 1.0;
        let producer = founder(0.6, 0.0, 0.5);
        assert!(
            branching_distance(&p, &producer) > 0.0,
            "a rich producer niche should read open (D > 0), matching the spike"
        );
    }

    // --- Oscillation |λ|−1 ---------------------------------------------------

    #[test]
    fn oscillation_distance_separates_a_stable_node_from_an_oscillatory_regime() {
        // A consumer-capable founder (positive heterotrophy → a living-mass↔pool
        // draw exists), held at a rich flux so the interior coexistence point
        // exists on both sides of the crossing. A low trophic efficiency sits that
        // point in the stable regime (|λ| < 1, frozen, D < 0); a high efficiency
        // (paradox of enrichment) pushes the complex pair across the unit circle
        // (|λ| > 1, limit cycle, D > 0).
        let f = founder(0.5, 0.5, 0.5);

        let mut stable = baseline_params();
        stable.solar_flux_magnitude = 15.0;
        stable.base_trophic_efficiency = 0.15;
        let d_stable = oscillation_distance(&stable, &f);

        let mut oscill = baseline_params();
        oscill.solar_flux_magnitude = 15.0;
        oscill.base_trophic_efficiency = 0.9;
        let d_oscill = oscillation_distance(&oscill, &f);

        assert!(
            d_stable < 0.0,
            "low-enrichment regime should read frozen (|λ|−1 < 0), got {d_stable}"
        );
        assert!(
            d_oscill > 0.0,
            "high-enrichment regime should read oscillation (|λ|−1 > 0), got {d_oscill}"
        );
    }

    #[test]
    fn oscillation_distance_is_zero_for_a_pure_autotroph_with_no_consumer_loop() {
        // A pure autotroph has no heterotrophic draw on the pool, so the
        // living-mass↔pool coupling has no interior consumer-resource point — the
        // mode cannot oscillate. The reading is 0 (frozen boundary), not NaN.
        let pure = founder(0.7, 0.0, 0.5);
        let d = oscillation_distance(&baseline_params(), &pure);
        assert_eq!(d, 0.0, "pure autotroph has no oscillator → distance 0");
    }

    #[test]
    fn oscillation_distance_is_finite_across_a_coarse_decode_sweep() {
        let ranges = default_ranges();
        for step in 0..=4 {
            let u = step as f64 / 4.0;
            let (params, dist) = decode(&vec![u; ranges.len()], &ranges);
            let d = oscillation_distance(&params, &dist.mean_traits);
            assert!(d.is_finite(), "|λ|−1 must be finite at unit {u}, got {d}");
        }
    }

    // --- Branching: conductance throughput crosscheck against the sim ---------

    #[test]
    fn branching_throughput_is_invariant_to_the_reserve_mobilisation_rate_in_rollout() {
        use explorers_sim::{InitialDistribution, World};
        // Reserve conservation predicts the realised steady-state mobilised surplus
        // per tick equals net income regardless of f — only the reserve level and
        // the transient length change. A single sessile autotroph with kappa = 0
        // routes its entire mobilised surplus into the reproductive allocation, so
        // the per-tick growth of repro_reserve at steady state IS that surplus.
        // Reproduction is gated off (huge threshold) so the allocation accumulates
        // monotonically and its slope is readable. The slope must not drift with f:
        // the empirical confirmation that g()'s throughput term needs no f factor
        // (the branching descriptor is f-invariant). Same seed + zero trait
        // covariance ⇒ the agent is identical across the f runs, so any drift is f.
        let mut params = baseline_params();
        params.solar_flux_magnitude = 15.0;
        params.reproduction_energy_threshold = 1.0e9; // never reproduce
        let founder = TraitVector {
            photosynthetic_absorption: 0.6,
            heterotrophy: 0.0,
            mobility: 0.0,
            kappa: 0.0,
            fecundity: 0.35,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        };
        let dist = InitialDistribution {
            mean_traits: founder,
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 10.0,
        };

        // Steady-state per-tick accumulation of the reproductive allocation, read
        // after the reserve transient (which lengthens as f → 0).
        let rate_at = |f: f32| -> f32 {
            let mut p = params.clone();
            p.reserve_mobilisation_rate = f;
            let mut world = World::new(p, dist.clone(), 42);
            let sum_repro = |w: &World| w.agents().iter().map(|a| a.repro_reserve).sum::<f32>();
            for _ in 0..100 {
                world.step();
            }
            let r1 = sum_repro(&world);
            for _ in 0..100 {
                world.step();
            }
            let r2 = sum_repro(&world);
            (r2 - r1) / 100.0
        };

        let base = rate_at(1.0);
        assert!(
            base > 0.0,
            "autotroph should accumulate reproductive allocation at f=1, got {base}"
        );
        for &f in &[0.5_f32, 0.25, 0.1] {
            let r = rate_at(f);
            assert!(
                (r - base).abs() <= 0.02 * base + 1e-4,
                "realised steady-state mobilised surplus drifted with f={f}: {r} vs f=1 {base}"
            );
        }
    }

    // --- 3×3 spectral radius (Cardano) ---------------------------------------

    #[test]
    fn spectral_radius_reads_the_largest_real_eigenvalue_of_a_diagonal() {
        // A diagonal matrix's eigenvalues are its diagonal; the spectral radius is
        // the largest modulus among them.
        let j = [[0.5, 0.0, 0.0], [0.0, -0.9, 0.0], [0.0, 0.0, 0.2]];
        assert!((spectral_radius(&j) - 0.9).abs() < 1e-5);
    }

    #[test]
    fn spectral_radius_reads_the_modulus_of_a_complex_pair() {
        // A 2×2 rotation-scaling block 0.6 ± 0.5i (modulus √0.61 ≈ 0.78102) plus a
        // smaller real eigenvalue 0.3. The Neimark–Sacker crossing is exactly this
        // complex pair, so the modulus must be read over the complex roots.
        let j = [[0.6, -0.5, 0.0], [0.5, 0.6, 0.0], [0.0, 0.0, 0.3]];
        let expected = (0.61_f32).sqrt();
        assert!(
            (spectral_radius(&j) - expected).abs() < 1e-5,
            "expected complex-pair modulus {expected}, got {}",
            spectral_radius(&j)
        );
    }

    #[test]
    fn interior_fixed_point_is_invariant_to_the_reserve_mobilisation_rate() {
        // Reserve conservation fixes the mobilised throughput at net income for
        // every f, so the interior coexistence point (A*, M*) does not move with
        // the conductance factor — only the reserve level R* and the relaxation
        // timescale do. f therefore enters stability (the Jacobian), never the
        // fixed-point location. This is the analytic invariant the branching
        // descriptor's f-invariance also rests on.
        let founder = founder(0.5, 0.5, 0.5);
        let mut base = baseline_params();
        base.solar_flux_magnitude = 15.0;
        base.base_trophic_efficiency = 0.9;

        let mut p1 = base.clone();
        p1.reserve_mobilisation_rate = 1.0;
        let (a_ref, m_ref) = PoolCoupling::from_founder(&founder, &p1)
            .interior_fixed_point(&p1)
            .expect("interior fixed point should exist at f=1");

        for &f in &[0.75_f32, 0.5, 0.25, 0.1, 0.05] {
            let mut p = base.clone();
            p.reserve_mobilisation_rate = f;
            let (a, m) = PoolCoupling::from_founder(&founder, &p)
                .interior_fixed_point(&p)
                .expect("interior fixed point should exist");
            assert!(
                (a - a_ref).abs() < 1e-6 && (m - m_ref).abs() < 1e-6,
                "fixed point moved with f={f}: ({a}, {m}) vs f=1 ({a_ref}, {m_ref})"
            );
        }
    }

    #[test]
    fn slower_mobilisation_moves_the_oscillation_reading_toward_the_hopf_boundary() {
        // The reserve compartment is a slow mode (relaxation ~f). As mobilisation
        // slows (f → 0) it pulls |λ|−1 monotonically toward the neutral boundary
        // from both sides: an oscillatory config's positive reading falls toward 0
        // (the feast-famine buffer damps the living-mass↔pool cycle), and a frozen
        // config's negative reading rises toward 0. Signs are preserved across the
        // sweep, so the reading stays a faithful side-of-boundary descriptor, and
        // every reading is finite.
        let founder = founder(0.5, 0.5, 0.5);
        let fs = [1.0_f32, 0.75, 0.5, 0.25, 0.1];

        let read = |eff: f32, f: f32| {
            let mut p = baseline_params();
            p.solar_flux_magnitude = 15.0;
            p.base_trophic_efficiency = eff;
            p.reserve_mobilisation_rate = f;
            oscillation_distance(&p, &founder)
        };
        let osc: Vec<f32> = fs.iter().map(|&f| read(0.9, f)).collect();
        let frz: Vec<f32> = fs.iter().map(|&f| read(0.15, f)).collect();

        for w in osc.windows(2) {
            assert!(
                w[0] > w[1] && w[1] > 0.0,
                "oscillatory side: |λ|−1 should fall toward 0 as f decreases, got {osc:?}"
            );
        }
        for w in frz.windows(2) {
            assert!(
                w[0] < w[1] && w[1] < 0.0,
                "frozen side: |λ|−1 should rise toward 0 as f decreases, got {frz:?}"
            );
        }
        for &d in osc.iter().chain(frz.iter()) {
            assert!(
                d.is_finite(),
                "|λ|−1 must be finite across the f-sweep, got {d}"
            );
        }
    }

    #[test]
    fn spectral_radius_reads_a_complex_pair_outside_the_unit_circle() {
        // A complex pair 0.8 ± 0.7i (modulus √1.13 ≈ 1.0630) — the oscillatory side
        // of a Hopf crossing — dominates a small real root.
        let j = [[0.8, -0.7, 0.0], [0.7, 0.8, 0.0], [0.0, 0.0, 0.1]];
        let expected = (1.13_f32).sqrt();
        assert!((spectral_radius(&j) - expected).abs() < 1e-5);
    }
}
