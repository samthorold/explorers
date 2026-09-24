//! Cross-check of the A1/A2 permanence predictions against what the simulation
//! does (issue #439, formal viability A3).
//!
//! ## What it checks
//!
//! `docs/research/432-permanence-pc.md` (A1) derives a closed-form permanence
//! condition for the producer↔consumer mean-field map — `ρ = F/B_P > 1` and
//! `I = β·K_P/m > 1` — and `docs/research/437-permanence-alc.md` (A2) adds the
//! lockup repeller `Λ > 1` and the heteroclinic product condition. Both notes
//! state an authority boundary: the condition is about a deterministic,
//! well-mixed, continuous-biomass map, and mean-field permanence is *necessary,
//! not sufficient*, for individual-based persistence. This instrument measures
//! how that boundary plays out in practice: for every atlas live cell and a
//! low-discrepancy sample of the search box it evaluates the inequalities in
//! closed form from the *config alone* (`predicted`), runs a fixed seed ensemble
//! through the genesis step loop with the evaluator's own terminal failure-mode
//! classification (`observed`), and tabulates the confusion matrix. The
//! dangerous direction is **predicted permanent, observed collapse** (a false
//! positive); the false-positive rate is what gates A4.
//!
//! ## The predicate, from a config
//!
//! A1's coefficients need a producer and a consumer trait vector. A config is an
//! `InitialDistribution`, not a roster, so the instrument takes the **cluster
//! centroids `World::new` seeds**: for `initial_cluster_count ≥ 2` a pure
//! producer `(α+h, 0, …)` and a pure consumer `(0, α+h, …)` around the mean's
//! other dimensions (the alternating-cluster rule in `World::new`); for a single
//! cluster both are the mean itself. `B_P`, `B_C`, `h_C`, `d` are read at those
//! vectors. Nothing about the realised, evolved population enters the
//! prediction — that is the point of the test.
//!
//! A2's `Λ` contains endogenous terms (reach `ι`, nutrient per carcass `ν`,
//! richness `q`, mortality `μ_P`). They are fixed at the reference lumping the A2
//! note's own sweep uses (`ι = 1`, `q = ν = θ_P`, `μ_P = 0.02`, producer
//! carcasses); when A1 holds but an A2 clause fails *at that reference* the
//! A1+A2 verdict is `undecided`, because the failing clause is not config-only.
//! A1-only and A1+A2 are reported as separate columns.
//!
//! ## What it does NOT do
//!
//! No change to the stepper, the evaluator's scoring, or the search. No
//! assertion on emergent values beyond a smoke check that records were produced.
//! Not a CI gate — run it explicitly. The coefficient mapping is copied from
//! `crates/explorers-sim/src/bin/permanence_prototype.rs` term for term (a bin
//! cannot be imported) and pinned to that bin's example10 numbers by unit test.
//!
//! ## Determinism and sourcing
//!
//! Same sourcing as `energy_bound_check.rs`: the atlas live-cell `unit` vectors
//! decoded via `decode` over `default_ranges`, plus the seed-421 LHS draw of 200
//! configs `role_emergence.rs` uses, so `sample:i` coincides across instruments.
//! Seeds are a fixed contiguous block per config. Each (config, seed) run is
//! independent, so the rayon collect is order-stable and a row is
//! byte-identical across runs. `World::new` floors founder traits at zero
//! (#444), so no run can lose a compartment to a negative-trait cull and the
//! instrument carries no #444 tag; the tick-1 population it records is the
//! peak-relative death threshold's doing, not a founder artefact.
//!
//! The rollout stops only on extinction and explosion and otherwise carries
//! to the horizon, where the evaluator's full-series classification
//! (`evaluate_from_log`) is the observed verdict — permanence is a statement
//! about the state at `T`, so an incremental gate's early stop is not taken
//! as the verdict here (the carry-to-`T` shape of #506).
//!
//! ## Resumable by construction
//!
//! One JSON line per config appended to `--out` (default
//! `target/permanence-crosscheck.jsonl`); configs already present are
//! skipped; `--limit N` runs at most `N` further configs. Fixed order and
//! seed block, so a loop of short foreground calls produces a file
//! byte-identical to one uninterrupted run (`explorers_search::sweep`).
//! Subset selectors: `--configs atlas:0,sample:12` and `--seeds 2`.
//!
//! A (config, seed) run carries two wall-clock budgets (#523). The
//! **simulation budget** (`--run-timeout-secs`, default 300) bounds the step
//! loop: a knife-edge world that costs seconds per tick stops where it is and
//! is recorded as mode `timeout`. The **evaluation budget**
//! (`--eval-timeout-secs`, default 300) bounds the terminal evaluation of a
//! rollout that reached the horizon: a dense terminal roster whose clustering
//! overruns it is recorded as mode `eval_timeout`, with every
//! breakdown-derived field at its not-read value (`decomposer_guild = false`).
//! Neither is a collapse nor a persistence, and the aggregate counts them
//! apart. A config's ensemble verdict is read over its finished seeds; a
//! config with no finished seed is `unobserved` and enters no confusion
//! matrix.
//!
//! ## Running
//!
//!   cargo build --release -p explorers-search --bin permanence_crosscheck
//!   ./target/release/permanence_crosscheck --limit 5     # repeat until 0 run
//!   ./target/release/permanence_crosscheck --summary
//!
//! Full run: 282 configs × 8 seeds × the search horizon (`SearchConfig::max_ticks`,
//! 2000 ticks since #507); hours of sim time, hence the chunked shape.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use rayon::prelude::*;

use explorers_genesis::{EvalConfig, FailureMode};
use explorers_genesis_eval::{EVALUATOR_EVENT_KINDS, RolloutObservations};
use explorers_search::config_source::{ConfigSource, parse_selector, sampled_units};
use explorers_search::search::{SearchConfig, decode, default_ranges};
use explorers_search::sweep::{
    DEFAULT_EVAL_TIMEOUT_SECS, EVAL_TIMEOUT_FLAG, EVAL_TIMEOUT_MODE, TIMEOUT_MODE, append_row,
    done_configs, evaluate_within_budget, is_unfinished, plan_tasks, read_atlas_units, read_rows,
};
use explorers_sim::{InitialDistribution, TraitVector, World, WorldParameters};

/// Reference body mass for the mean-field reduction (as in the prototype).
const REFERENCE_BODY_MASS: f32 = 1.0;

/// The trait vectors the prediction is evaluated at: the cluster centroids
/// `World::new` seeds from an `InitialDistribution`. Mirrors its
/// alternating-cluster rule — with two or more clusters and a positive trophic
/// total, even clusters are pure producers and odd clusters pure consumers,
/// sharing every other dimension of the mean; otherwise both are the mean.
fn representative_clusters(dist: &InitialDistribution) -> (TraitVector, TraitVector) {
    let mean = dist.mean_traits;
    let n_clusters = (dist.initial_cluster_count as usize).max(1);
    let trophic_total = mean.photosynthetic_absorption + mean.heterotrophy;
    if n_clusters == 1 || trophic_total <= 0.0 {
        return (mean, mean);
    }
    let producer = TraitVector {
        photosynthetic_absorption: trophic_total,
        heterotrophy: 0.0,
        ..mean
    };
    let consumer = TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy: trophic_total,
        ..mean
    };
    (producer, consumer)
}

/// The fecundity floor `resolve_reproduction` applies before the Poisson draw
/// (`fecundity.max(0.1)` in `phase.rs`).
const FECUNDITY_FLOOR: f64 = 0.1;

/// A cluster's biomass conversion `χ` (copied from
/// `permanence_prototype::biomass_conversion`; `χ_P` in #466, `χ_C` in #482):
/// the structure built, before `γ`, per unit of mobilised surplus once both of
/// `κ`'s branches are followed to the body they build. The reproductive branch
/// reaches an offspring at `η = reproduction_efficiency · (1 − propagule
/// share) · (1 − e^{−fecundity})`; `s = offspring_structure_fraction` of that
/// is embodied at birth and the rest is reserve the offspring re-splits by
/// `κ`, so `χ = κ + (1 − κ)·η·(s + (1 − s)·χ)`. The reproductive path is the
/// same `resolve_reproduction` for every trait vector, so the producer and
/// the consumer share the form and differ only in `κ`, `δ`, `f`.
fn biomass_conversion(traits: &TraitVector, p: &WorldParameters) -> f64 {
    let kappa = traits.kappa.clamp(0.0, 1.0) as f64;
    let propagule = explorers_sim::dispersal_propagule_cost_fraction(traits.dispersal, p) as f64;
    let fecundity = (traits.fecundity as f64).max(FECUNDITY_FLOOR);
    let eta = (p.reproduction_efficiency as f64).clamp(0.0, 1.0)
        * (1.0 - propagule)
        * (1.0 - (-fecundity).exp());
    let s = p.offspring_structure_fraction.clamp(0.0, 1.0) as f64;
    let feedback = (1.0 - kappa) * eta * (1.0 - s);
    if feedback >= 1.0 {
        1.0
    } else {
        (kappa + (1.0 - kappa) * eta * s) / (1.0 - feedback)
    }
}

/// A1's lumped coefficients (copied from `permanence_prototype::Map`; `r_p` signed).
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
struct A1Map {
    /// Producer intrinsic per-tick rate `χ_P·γ·(F − B_P)`, signed (#466: both
    /// of `κ_P`'s branches become biomass; `χ_P` is the conversion).
    r_p: f64,
    /// Producer carrying capacity `F / B_P` (= `ρ`, the extinction ratio).
    k_p: f64,
    /// Attack rate: consumer effective heterotrophy per reference body.
    a: f64,
    /// Consumer maintenance floor `B_C`.
    m: f64,
    /// Conversion `β = χ_C·γ·e·a` (#482: both of `κ_C`'s branches become
    /// biomass; `χ_C` is the consumer's conversion).
    beta: f64,
    /// Producer maintenance `B_P`.
    b_p: f64,
    /// Producer↔consumer trait distance `d`.
    d: f64,
}

/// Per-tick maintenance of one reference body (as `permanence_prototype`).
fn maintenance(traits: &TraitVector, p: &WorldParameters) -> f64 {
    let exp = p.maintenance_cost_exponent;
    (p.base_metabolic_rate
        + traits.photosynthetic_absorption.max(0.0).powf(exp) * p.photo_maintenance_cost
        + traits.heterotrophy.max(0.0).powf(exp) * p.heterotrophy_maintenance_cost
        + traits.mobility.max(0.0).powf(exp) * p.mobility_maintenance_cost
        + REFERENCE_BODY_MASS * p.structure_maintenance_coefficient) as f64
}

impl A1Map {
    fn derive(producer: &TraitVector, consumer: &TraitVector, p: &WorldParameters) -> Self {
        let chi_c = biomass_conversion(consumer, p);
        let gamma = p.growth_efficiency as f64;
        let b_p = maintenance(producer, p);
        let b_c = maintenance(consumer, p);
        let flux = p.solar_flux_magnitude as f64;
        let r_p =
            biomass_conversion(producer, p) * gamma * (flux - b_p) / REFERENCE_BODY_MASS as f64;
        let k_p = if b_p > 0.0 { flux / b_p } else { f64::INFINITY };
        let a = consumer.heterotrophy.max(0.0) as f64 / REFERENCE_BODY_MASS as f64;
        let m = b_c / REFERENCE_BODY_MASS as f64;
        let d = producer.distance(consumer) as f64;
        let e = p.base_trophic_efficiency as f64 * (-(p.trophic_distance_decay as f64) * d).exp();
        A1Map {
            r_p,
            k_p,
            a,
            m,
            beta: chi_c * gamma * e * a,
            b_p,
            d,
        }
    }

    /// The invasion ratio `I = β·K_P/m`.
    fn invasion_ratio(&self) -> f64 {
        self.beta * self.k_p / self.m
    }
}

/// A closed-form verdict on a config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Prediction {
    Permanent,
    NotPermanent,
    /// The inequalities hold but a hypothesis the theorem needs fails (A1's
    /// H1), or the deciding clause involves an endogenous term (A2).
    Undecided,
}

/// A1 evaluated at the representative clusters.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
struct A1Verdict {
    prediction: Prediction,
    /// Clause (1): `r_P > 0` ⟺ `ρ > 1` — the extinction gate `F ≤ B`, sharpened.
    producer_face_alive: bool,
    /// Clause (2): `β·K_P > m` ⟺ `I > 1` — the consumer invades `(K_P, 0)`.
    consumer_invades: bool,
    /// H1: `0 < m < 1` and `r_P ≤ 2`. When it fails the clauses stay necessary
    /// but the sufficiency proof does not apply.
    hypothesis_h1: bool,
    rho: f64,
    invasion_ratio: f64,
    /// Producer somatic allocation `κ_P` (the search box allows 0; since #466
    /// that only routes the surplus through offspring, it no longer zeroes
    /// `r_P`).
    kappa_p: f64,
    /// Consumer somatic allocation `κ_C` (the search box allows 0; since #482
    /// that only routes the surplus through offspring, it no longer zeroes
    /// `β`, `I`, or `Λ`).
    kappa_c: f64,
    #[serde(flatten)]
    map: A1Map,
}

fn a1_verdict(map: A1Map, producer: &TraitVector, consumer: &TraitVector) -> A1Verdict {
    let producer_face_alive = map.r_p > 0.0;
    let consumer_invades = map.beta * map.k_p > map.m;
    let hypothesis_h1 = map.m > 0.0 && map.m < 1.0 && map.r_p <= 2.0;
    let prediction = if !(producer_face_alive && consumer_invades) {
        Prediction::NotPermanent
    } else if hypothesis_h1 {
        Prediction::Permanent
    } else {
        Prediction::Undecided
    };
    A1Verdict {
        prediction,
        producer_face_alive,
        consumer_invades,
        hypothesis_h1,
        rho: map.k_p,
        invasion_ratio: map.invasion_ratio(),
        kappa_p: producer.kappa.clamp(0.0, 1.0) as f64,
        kappa_c: consumer.kappa.clamp(0.0, 1.0) as f64,
        map,
    }
}

/// A2's reference lumping — the values the A2 note's own sweep uses. All four
/// are endogenous in the stepper; they are fixed here only so that `Λ` and the
/// virgin-end eigenvalue can be evaluated at all.
const REFERENCE_MU_P: f64 = 0.02;
const REFERENCE_IOTA: f64 = 1.0;

/// A2 evaluated at the representative clusters and the reference lumping
/// (`ι = 1`, `q = ν = θ_P`, `μ_P = 0.02`, producer carcasses so `e_C = e`).
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
struct A2Verdict {
    /// (i) `min(r_P, α_P/θ_P) > μ_P`: the producer invades the virgin pool.
    producer_invades_virgin: bool,
    /// (ii) `Λ > 1`: the heterotroph invades the lockup corner.
    heterotroph_invades_lockup: bool,
    /// (iii) the dead-line heteroclinic cycle repels.
    cycle_repels: bool,
    lockup_escape_ratio: f64,
    /// `min(r_P, α_P/θ_P)`: the producer's Liebig-limited invasion rate at
    /// the virgin end, compared against the reference `μ_P`.
    virgin_invasion_rate: f64,
    theta_p: f64,
    theta_c: f64,
    alpha_p: f64,
    n_total: f64,
}

impl A2Verdict {
    fn holds(&self) -> bool {
        self.producer_invades_virgin && self.heterotroph_invades_lockup && self.cycle_repels
    }
}

fn a2_verdict(
    map: &A1Map,
    producer: &TraitVector,
    consumer: &TraitVector,
    p: &WorldParameters,
) -> A2Verdict {
    let theta_p = explorers_sim::stoichiometric_demand(producer, 1.0, p) as f64;
    let theta_c = explorers_sim::stoichiometric_demand(consumer, 1.0, p) as f64;
    let alpha_p = producer.photosynthetic_absorption.max(0.0) as f64;
    // The consumer's conversion χ_C·γ (#482), the carcass-route twin of β's.
    let chi_c_gamma = biomass_conversion(consumer, p) * p.growth_efficiency as f64;
    let n_total = p.initial_nutrient_pool as f64;
    // Reference lumping: producer carcasses with no free store, so q = ν = θ_P
    // and e_C = e = β / (χ_C·γ·a).
    let (q, nu) = (theta_p, theta_p * REFERENCE_BODY_MASS as f64);
    let e_c = if chi_c_gamma > 0.0 && map.a > 0.0 {
        map.beta / (chi_c_gamma * map.a)
    } else {
        0.0
    };
    let sigma = if nu > 0.0 {
        map.a * REFERENCE_IOTA / nu
    } else {
        0.0
    };
    let liebig = if theta_c > 0.0 {
        (chi_c_gamma * e_c).min(q / theta_c)
    } else {
        chi_c_gamma * e_c
    };
    let pile_growth = sigma * n_total * liebig;
    let lockup_escape_ratio = if map.m > 0.0 {
        pile_growth / map.m
    } else {
        f64::INFINITY
    };
    let virgin_invasion_rate = if theta_p > 0.0 {
        map.r_p.min(alpha_p / theta_p)
    } else {
        map.r_p
    };
    let lp_virgin = 1.0 + virgin_invasion_rate - REFERENCE_MU_P;
    let lc_virgin = 1.0 - map.m;
    let lp_lockup = 1.0 - REFERENCE_MU_P;
    let lc_lockup = 1.0 + pile_growth - map.m;
    let producer_invades_virgin = lp_virgin > 1.0;
    let heterotroph_invades_lockup = lc_lockup > 1.0;
    let cycle_repels = producer_invades_virgin
        && heterotroph_invades_lockup
        && lp_virgin.ln() * lc_lockup.ln() > (-lc_virgin.ln()) * (-lp_lockup.ln());
    A2Verdict {
        producer_invades_virgin,
        heterotroph_invades_lockup,
        cycle_repels,
        lockup_escape_ratio,
        virgin_invasion_rate,
        theta_p,
        theta_c,
        alpha_p,
        n_total,
    }
}

/// The A2 (coupled) verdict. A2's theorem replaces A1's clause (2): in the
/// coupled map the producer-only face flows into the lockup corner, so the
/// consumer's escape route is the carcass pile (`Λ > 1`), not the standing
/// crop (`I > 1`). Clause (i) keeps A1's clause (1) inside it (`r_P > μ_P`).
/// A clause that fails *for every value of the endogenous terms* is a
/// config-only kill — `min(r_P, α_P/θ_P) ≤ 0` (no `μ_P ≥ 0` rescues it) or
/// `Λ = 0` (no heterotrophy, or no kernel). A clause that fails only at the
/// reference lumping is not a config-only verdict and is reported as
/// undecided. A1's H1 is inherited.
fn a2_prediction(a1: &A1Verdict, a2: &A2Verdict) -> Prediction {
    if a2.virgin_invasion_rate <= 0.0 || a2.lockup_escape_ratio <= 0.0 {
        Prediction::NotPermanent
    } else if !a1.hypothesis_h1 || !a2.holds() {
        Prediction::Undecided
    } else {
        Prediction::Permanent
    }
}

/// The evaluator's terminal failure mode, as a stable label.
fn mode_label(failure: &Option<FailureMode>) -> &'static str {
    match failure {
        None => "none",
        Some(FailureMode::Extinction) => "extinction",
        Some(FailureMode::PopulationExplosion) => "explosion",
        Some(FailureMode::EnergyDeath) => "energy-death",
        Some(FailureMode::Monoculture) => "monoculture",
        Some(FailureMode::GeneralistDominance) => "generalist-dominance",
        Some(FailureMode::NutrientLockup) => "nutrient-lockup",
    }
}

/// "Observed collapse" = the extinction boundary attracted: extinction, energy
/// death, or nutrient lockup. Everything else (none, monoculture, generalist
/// dominance, explosion) has a living population bounded away from zero at the
/// horizon and is read as persistence.
fn is_collapse(mode: &str) -> bool {
    matches!(mode, "extinction" | "energy-death" | "nutrient-lockup")
}

/// One (config × seed) run.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SeedOutcome {
    seed: u64,
    /// The evaluator's terminal classification (`evaluate_from_log`),
    /// `"timeout"` when the simulation budget ran out, or `"eval_timeout"`
    /// when the evaluation budget ran out at the horizon (neither is a
    /// collapse nor a persistence: the seed is left out of the ensemble read).
    mode: String,
    collapsed: bool,
    termination_tick: u64,
    founders: usize,
    population_after_tick1: usize,
    producers_after_tick1: usize,
    consumers_after_tick1: usize,
    terminal_producers: usize,
    terminal_consumers: usize,
    /// The evaluator's decomposer-guild observable (`has_decomposer_guild`):
    /// a sustained, recruiting population draining carcasses (#490) — A2's
    /// pile route, read off the same log the failure mode is.
    decomposer_guild: bool,
    peak_population: usize,
    /// First tick with no consumer alive while producers still stood, if any.
    first_tick_without_consumers: Option<u64>,
    first_tick_without_producers: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Observed {
    /// Every seed collapsed.
    Collapse,
    /// Some seeds collapsed, some persisted.
    Mixed,
    /// No seed collapsed.
    Persist,
    /// No seed finished (every seed exhausted a budget): nothing was observed.
    Unobserved,
}

/// The ensemble reduced to one observed outcome, with the facts the fault
/// hypotheses read. `Collapse` / `Persist` are the unanimous reads
/// (`scenarios/verdicts.md`; at `n = 8` unanimity bounds the per-seed rate at
/// `p ≥ 0.63`, #434); anything else is `Mixed`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Aggregate {
    /// Seeds that finished (the ensemble the verdict is read over).
    n: usize,
    /// Seeds stopped by the simulation budget, left out of every field below.
    timed_out: usize,
    /// Seeds whose evaluation exhausted the evaluation budget, left out of
    /// every field below. Omitted from the row when zero, so a row with none
    /// serialises as it did before the budget existed (and older rows parse).
    #[serde(default, skip_serializing_if = "is_zero")]
    eval_timed_out: usize,
    collapsed: usize,
    collapse_fraction: f64,
    modal_mode: String,
    modal_count: usize,
    observed: Observed,
    /// Persisting seeds with at least one consumer alive at the horizon.
    persisting_seeds_with_consumers: usize,
    /// Persisting seeds on which the evaluator saw a decomposer guild.
    persisting_seeds_with_decomposer_guild: usize,
    median_first_tick_without_consumers: Option<u64>,
    /// Median over the collapsing seeds.
    median_population_after_tick1_on_collapse: Option<u64>,
    median_peak_population: usize,
    median_founders: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

fn median_u64(values: &mut [u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(values[values.len() / 2])
}

fn aggregate(all_seeds: &[SeedOutcome]) -> Aggregate {
    let timed_out = all_seeds.iter().filter(|s| s.mode == TIMEOUT_MODE).count();
    let eval_timed_out = all_seeds
        .iter()
        .filter(|s| s.mode == EVAL_TIMEOUT_MODE)
        .count();
    let seeds: Vec<&SeedOutcome> = all_seeds
        .iter()
        .filter(|s| !is_unfinished(&s.mode))
        .collect();
    let n = seeds.len();
    let collapsed = seeds.iter().filter(|s| s.collapsed).count();
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for s in &seeds {
        match counts.iter_mut().find(|(m, _)| *m == s.mode) {
            Some((_, c)) => *c += 1,
            None => counts.push((s.mode.as_str(), 1)),
        }
    }
    // Modal mode; ties broken by first appearance (seed order) for stability.
    let (modal_mode, modal_count) =
        counts.iter().copied().fold(
            ("none", 0),
            |best, cur| if cur.1 > best.1 { cur } else { best },
        );
    let observed = if n == 0 {
        Observed::Unobserved
    } else if collapsed == n {
        Observed::Collapse
    } else if collapsed == 0 {
        Observed::Persist
    } else {
        Observed::Mixed
    };
    let persisting_seeds_with_consumers = seeds
        .iter()
        .filter(|s| !s.collapsed && s.terminal_consumers > 0)
        .count();
    let persisting_seeds_with_decomposer_guild = seeds
        .iter()
        .filter(|s| !s.collapsed && s.decomposer_guild)
        .count();
    let mut no_consumers: Vec<u64> = seeds
        .iter()
        .filter(|s| s.collapsed)
        .filter_map(|s| s.first_tick_without_consumers)
        .collect();
    let mut after_tick1: Vec<u64> = seeds
        .iter()
        .filter(|s| s.collapsed)
        .map(|s| s.population_after_tick1 as u64)
        .collect();
    let mut peaks: Vec<u64> = seeds.iter().map(|s| s.peak_population as u64).collect();
    let mut founders: Vec<u64> = seeds.iter().map(|s| s.founders as u64).collect();
    Aggregate {
        n,
        timed_out,
        eval_timed_out,
        collapsed,
        collapse_fraction: if n == 0 {
            0.0
        } else {
            collapsed as f64 / n as f64
        },
        modal_mode: modal_mode.to_string(),
        modal_count,
        observed,
        persisting_seeds_with_consumers,
        persisting_seeds_with_decomposer_guild,
        median_first_tick_without_consumers: median_u64(&mut no_consumers),
        median_population_after_tick1_on_collapse: median_u64(&mut after_tick1),
        median_peak_population: median_u64(&mut peaks).unwrap_or(0) as usize,
        median_founders: median_u64(&mut founders).unwrap_or(0) as usize,
    }
}

/// How a prediction and an observation relate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Agreement {
    Agree,
    /// Predicted permanent, every seed collapsed — the dangerous direction.
    FalsePositive,
    /// Predicted permanent, some seeds collapsed.
    FalsePositiveMixed,
    /// Predicted not-permanent, no seed collapsed, consumers persisted.
    FalseNegative,
    /// Predicted not-permanent, some seeds persisted.
    FalseNegativeMixed,
    /// Predicted not-permanent on clause (2) only and the persisting seeds
    /// (all of them, or the persisting share of a mixed cell) carry no
    /// consumer at the horizon: the producer-only face, as A1 says.
    ProducerOnly,
    /// The prediction was undecided; nothing to agree or disagree with.
    Undecided,
    /// No seed finished (every seed timed out); nothing was observed.
    Unobserved,
}

fn classify(prediction: Prediction, a1: &A1Verdict, agg: &Aggregate) -> Agreement {
    match (prediction, agg.observed) {
        (_, Observed::Unobserved) => Agreement::Unobserved,
        (Prediction::Undecided, _) => Agreement::Undecided,
        (Prediction::Permanent, Observed::Collapse) => Agreement::FalsePositive,
        (Prediction::Permanent, Observed::Mixed) => Agreement::FalsePositiveMixed,
        (Prediction::Permanent, Observed::Persist) => Agreement::Agree,
        (Prediction::NotPermanent, Observed::Collapse) => Agreement::Agree,
        (Prediction::NotPermanent, Observed::Persist)
            if a1.producer_face_alive && agg.persisting_seeds_with_consumers == 0 =>
        {
            Agreement::ProducerOnly
        }
        (Prediction::NotPermanent, Observed::Persist) => Agreement::FalseNegative,
        (Prediction::NotPermanent, Observed::Mixed)
            if a1.producer_face_alive && agg.persisting_seeds_with_consumers == 0 =>
        {
            Agreement::ProducerOnly
        }
        (Prediction::NotPermanent, Observed::Mixed) => Agreement::FalseNegativeMixed,
    }
}

/// One-line fault hypothesis for a disagreeing cell, from A1's authority
/// boundary (mean field / no space / no demographic noise / reduction regime
/// exit). Rules apply in order; the first that matches names the hypothesis.
fn fault_hypothesis(
    agreement: Agreement,
    a1: &A1Verdict,
    agg: &Aggregate,
    single_centroid: bool,
) -> Option<String> {
    match agreement {
        Agreement::Agree
        | Agreement::ProducerOnly
        | Agreement::Undecided
        | Agreement::Unobserved => None,
        Agreement::FalsePositive | Agreement::FalsePositiveMixed => Some(
            if agg.median_population_after_tick1_on_collapse == Some(0) {
                "reduction: every founder dies on tick 1 whatever its trait sign - a first-tick per-body floor (the peak-relative death threshold, endowment, embodiment) the biomass map has no coordinate for".to_string()
            } else if agg.modal_mode == "nutrient-lockup" {
                "reduction: nutrient lockup is outside A1's coordinates (A2's iota/nu/q are endogenous)"
                .to_string()
            } else if agg.modal_mode == "energy-death" {
                "reduction: energy-death stock trend with survivors is outside A1's coordinates"
                    .to_string()
            } else if agg
                .median_first_tick_without_consumers
                .is_some_and(|t| t <= 25)
            {
                format!(
                    "spatial: consumers lost by tick {} while producers stood - reach never met the crop (example10 signature)",
                    agg.median_first_tick_without_consumers.unwrap_or(0)
                )
            } else if agreement == Agreement::FalsePositiveMixed {
                format!(
                    "demographic-stochastic: {}/{} seeds collapse - a per-seed split the mean field does not resolve",
                    agg.collapsed, agg.n
                )
            } else if agg.median_peak_population <= 2 * agg.median_founders {
                format!(
                    "finite-size: population never left the founder scale (median peak {} from {} founders)",
                    agg.median_peak_population, agg.median_founders
                )
            } else {
                "reduction: mean-field regime exit (unclassified)".to_string()
            },
        ),
        Agreement::FalseNegative | Agreement::FalseNegativeMixed => {
            Some(if !a1.producer_face_alive && a1.rho > 1.0 {
                format!(
                    "reduction: rho = {:.1} > 1 but r_P <= 0 - the biomass conversion chi_P is zero (kappa_P = {} and the reproductive branch burns everything: propagule share 1 or reproduction_efficiency 0)",
                    a1.rho, a1.kappa_p
                )
            } else if !a1.producer_face_alive {
                "reduction: extinction gate fails at the centroid but founders persisted - realised maintenance below the centroid's (trait draw / evolution)".to_string()
            } else if !a1.consumer_invades && a1.map.beta == 0.0 && a1.map.a > 0.0 {
                format!(
                    "reduction: the consumer feeds (a = {:.2}) but beta = 0 - the biomass conversion chi_C is zero (kappa_C = {} and the reproductive branch burns everything: propagule share 1 or reproduction_efficiency 0); consumers persisted on {}/{} seeds",
                    a1.map.a,
                    a1.kappa_c,
                    agg.persisting_seeds_with_consumers,
                    agg.n - agg.collapsed
                )
            } else if single_centroid {
                format!(
                    "reduction: single mixotroph centroid - no consumer compartment to evaluate, I is read at the mean's own h; heterotroph-leaning agents present on {}/{} persisting seeds",
                    agg.persisting_seeds_with_consumers,
                    agg.n - agg.collapsed
                )
            } else if agg.persisting_seeds_with_decomposer_guild > 0 {
                format!(
                    "reduction: consumers persisted on {}/{} seeds despite I <= 1, with a decomposer guild on {} of them - A2's carcass route, which A1's map lacks",
                    agg.persisting_seeds_with_consumers,
                    agg.n - agg.collapsed,
                    agg.persisting_seeds_with_decomposer_guild
                )
            } else {
                format!(
                    "reduction: consumers persisted on {}/{} seeds despite I <= 1 and no decomposer guild - realised traits departed the centroid (evolution / draw)",
                    agg.persisting_seeds_with_consumers,
                    agg.n - agg.collapsed
                )
            })
        }
    }
}

const PREDICTIONS: [Prediction; 3] = [
    Prediction::Permanent,
    Prediction::NotPermanent,
    Prediction::Undecided,
];
const OBSERVATIONS: [Observed; 3] = [Observed::Collapse, Observed::Mixed, Observed::Persist];

/// Confusion matrix, rows = predicted (permanent, not-permanent, undecided),
/// columns = observed (collapse, mixed, persist). Unobserved cells (every
/// seed timed out) are not in it.
fn confusion(cells: &[(Prediction, Observed)]) -> [[usize; 3]; 3] {
    let mut m = [[0usize; 3]; 3];
    for (p, o) in cells {
        let i = PREDICTIONS.iter().position(|x| x == p).unwrap();
        let Some(j) = OBSERVATIONS.iter().position(|x| x == o) else {
            continue;
        };
        m[i][j] += 1;
    }
    m
}

/// False-positive rates: strict (every seed collapsed) and any-seed (at least
/// one seed collapsed). There is no "excluding #444" denominator any more:
/// `World::new` floors founder traits at zero, so no cell can owe its
/// collapse to the tick-1 cull.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
struct FalsePositives {
    predicted_permanent: usize,
    strict: usize,
    any_seed: usize,
    strict_rate: f64,
    any_seed_rate: f64,
}

fn false_positives(cells: &[(Prediction, Observed)]) -> FalsePositives {
    let rate = |k: usize, n: usize| if n == 0 { 0.0 } else { k as f64 / n as f64 };
    let permanent: Vec<&(Prediction, Observed)> = cells
        .iter()
        .filter(|(p, o)| *p == Prediction::Permanent && *o != Observed::Unobserved)
        .collect();
    let strict = permanent
        .iter()
        .filter(|(_, o)| *o == Observed::Collapse)
        .count();
    let any_seed = permanent
        .iter()
        .filter(|(_, o)| *o != Observed::Persist)
        .count();
    FalsePositives {
        predicted_permanent: permanent.len(),
        strict,
        any_seed,
        strict_rate: rate(strict, permanent.len()),
        any_seed_rate: rate(any_seed, permanent.len()),
    }
}

// ---------------------------------------------------------------------------
// Driving the simulation
// ---------------------------------------------------------------------------

/// Fixed contiguous seed block per config (the `role_emergence` convention).
const N_SEEDS: u64 = 8;
const SEED_BASE: u64 = 1000;
/// Producer / consumer split of a living roster, by the prototype's rule
/// (`photosynthetic_absorption ≥ heterotrophy` is a producer).
fn compartments(world: &World) -> (usize, usize) {
    let mut producers = 0;
    let mut consumers = 0;
    for a in world.agents() {
        if a.traits.photosynthetic_absorption >= a.traits.heterotrophy {
            producers += 1;
        } else {
            consumers += 1;
        }
    }
    (producers, consumers)
}

/// Drive one (config, seed) through the genesis step loop exactly as
/// `explorers_genesis::run_single` does (same observations, same early stops,
/// same `evaluate_from_log`), while also reading the compartment facts the
/// fault hypotheses need.
fn run_seed(
    params: &WorldParameters,
    dist: &InitialDistribution,
    seed: u64,
    horizon: u64,
    run_timeout: Duration,
    eval_timeout: Duration,
) -> SeedOutcome {
    let eval_config = EvalConfig::default();
    let started = Instant::now();
    let mut timed_out = false;
    let mut world = World::new(params.clone(), dist.clone(), seed);
    // As `run_single`: the log keeps only what the evaluator reads and is
    // dropped as soon as `observe` has read it, so a dense world at the
    // settled horizon fits in memory (observer-side — no trajectory changes).
    world.retain_event_kinds(EVALUATOR_EVENT_KINDS);
    let founders = world.agents().len();
    let mut observations = RolloutObservations::with_capacity(horizon as usize);
    let mut peak_population = founders;
    let mut population_after_tick1 = 0;
    let mut producers_after_tick1 = 0;
    let mut consumers_after_tick1 = 0;
    let mut first_tick_without_consumers = None;
    let mut first_tick_without_producers = None;
    for _ in 0..horizon {
        world.step();
        observations.observe(&world, eval_config.coexistence_sample_interval);
        world.compact_event_log_before(observations.consumed_events());
        let (producers, consumers) = compartments(&world);
        if world.tick() == 1 {
            population_after_tick1 = world.agents().len();
            producers_after_tick1 = producers;
            consumers_after_tick1 = consumers;
        }
        if consumers == 0 && producers > 0 && first_tick_without_consumers.is_none() {
            first_tick_without_consumers = Some(world.tick());
        }
        if producers == 0 && consumers > 0 && first_tick_without_producers.is_none() {
            first_tick_without_producers = Some(world.tick());
        }
        peak_population = peak_population.max(world.agents().len());
        if world.agents().is_empty() {
            break;
        }
        if world.agents().len() > eval_config.max_population {
            break;
        }
        if started.elapsed() > run_timeout {
            timed_out = true;
            break;
        }
    }
    // An unfinished seed has no breakdown: its guild takes the not-read value.
    let (mode, decomposer_guild) = if timed_out {
        (TIMEOUT_MODE, false)
    } else {
        match evaluate_within_budget(&world, &observations, &eval_config, horizon, eval_timeout) {
            Some(breakdown) => (
                mode_label(&breakdown.failure),
                breakdown.has_decomposer_guild,
            ),
            None => (EVAL_TIMEOUT_MODE, false),
        }
    };
    let (terminal_producers, terminal_consumers) = compartments(&world);
    SeedOutcome {
        seed,
        mode: mode.to_string(),
        collapsed: is_collapse(mode),
        termination_tick: world.tick(),
        founders,
        population_after_tick1,
        producers_after_tick1,
        consumers_after_tick1,
        terminal_producers,
        terminal_consumers,
        decomposer_guild,
        peak_population,
        first_tick_without_consumers,
        first_tick_without_producers,
    }
}

/// One config: the closed-form predictions, the ensemble, and how they relate.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ConfigRecord {
    source: ConfigSource,
    config_index: usize,
    horizon: u64,
    initial_cluster_count: u32,
    initial_population_size: u32,
    /// The prediction was evaluated on distinct producer and consumer
    /// centroids (`initial_cluster_count ≥ 2`); otherwise both are the mean.
    has_consumer_compartment: bool,
    producer_centroid: TraitVector,
    consumer_centroid: TraitVector,
    a1: A1Verdict,
    a2: A2Verdict,
    predicted_a1: Prediction,
    predicted_a2: Prediction,
    aggregate: Aggregate,
    agreement_a1: Agreement,
    agreement_a2: Agreement,
    fault_hypothesis_a1: Option<String>,
    fault_hypothesis_a2: Option<String>,
    seeds: Vec<SeedOutcome>,
}

fn evaluate_config(
    source: ConfigSource,
    config_index: usize,
    unit: &[f64],
    seeds: u64,
    horizon: u64,
    run_timeout: Duration,
    eval_timeout: Duration,
) -> ConfigRecord {
    let ranges = default_ranges();
    let (params, dist) = decode(unit, &ranges);
    let (producer, consumer) = representative_clusters(&dist);
    let has_consumer_compartment = producer != consumer;
    let map = A1Map::derive(&producer, &consumer, &params);
    let a1 = a1_verdict(map, &producer, &consumer);
    let a2 = a2_verdict(&map, &producer, &consumer, &params);
    let predicted_a1 = a1.prediction;
    let predicted_a2 = a2_prediction(&a1, &a2);
    // Seeds are independent; rayon's indexed collect preserves seed order, so
    // the record is bit-identical to the sequential map (the #350 contract).
    let outcomes: Vec<SeedOutcome> = (0..seeds)
        .into_par_iter()
        .map(|s| {
            run_seed(
                &params,
                &dist,
                SEED_BASE + s,
                horizon,
                run_timeout,
                eval_timeout,
            )
        })
        .collect();
    let aggregate = aggregate(&outcomes);
    let agreement_a1 = classify(predicted_a1, &a1, &aggregate);
    let agreement_a2 = classify(predicted_a2, &a1, &aggregate);
    ConfigRecord {
        source,
        config_index,
        horizon,
        initial_cluster_count: dist.initial_cluster_count,
        initial_population_size: params.initial_population_size,
        has_consumer_compartment,
        producer_centroid: producer,
        consumer_centroid: consumer,
        a1,
        a2,
        predicted_a1,
        predicted_a2,
        fault_hypothesis_a1: fault_hypothesis(
            agreement_a1,
            &a1,
            &aggregate,
            !has_consumer_compartment,
        ),
        fault_hypothesis_a2: fault_hypothesis(
            agreement_a2,
            &a1,
            &aggregate,
            !has_consumer_compartment,
        ),
        agreement_a1,
        agreement_a2,
        aggregate,
        seeds: outcomes,
    }
}

#[derive(Clone, Debug, serde::Serialize)]
struct Disagreement {
    source: ConfigSource,
    config_index: usize,
    predicted: Prediction,
    observed: Observed,
    agreement: Agreement,
    collapsed: usize,
    n: usize,
    modal_mode: String,
    rho: f64,
    invasion_ratio: f64,
    lockup_escape_ratio: f64,
    fault_hypothesis: String,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Column {
    /// Rows: predicted permanent / not-permanent / undecided. Columns:
    /// observed collapse / mixed / persist.
    confusion: [[usize; 3]; 3],
    false_positives: FalsePositives,
    agree: usize,
    producer_only: usize,
    false_negative: usize,
    false_negative_mixed: usize,
    undecided: usize,
    /// Configs on which every seed timed out: in no confusion matrix.
    unobserved: usize,
    disagreements: Vec<Disagreement>,
}

fn column(records: &[ConfigRecord], a1_only: bool) -> Column {
    let pick = |r: &ConfigRecord| {
        if a1_only {
            (
                r.predicted_a1,
                r.agreement_a1,
                r.fault_hypothesis_a1.clone(),
            )
        } else {
            (
                r.predicted_a2,
                r.agreement_a2,
                r.fault_hypothesis_a2.clone(),
            )
        }
    };
    let cells: Vec<(Prediction, Observed)> = records
        .iter()
        .map(|r| (pick(r).0, r.aggregate.observed))
        .collect();
    let count = |a: Agreement| records.iter().filter(|r| pick(r).1 == a).count();
    let disagreements = records
        .iter()
        .filter_map(|r| {
            let (predicted, agreement, hypothesis) = pick(r);
            hypothesis.map(|fault_hypothesis| Disagreement {
                source: r.source,
                config_index: r.config_index,
                predicted,
                observed: r.aggregate.observed,
                agreement,
                collapsed: r.aggregate.collapsed,
                n: r.aggregate.n,
                modal_mode: r.aggregate.modal_mode.clone(),
                rho: r.a1.rho,
                invasion_ratio: r.a1.invasion_ratio,
                lockup_escape_ratio: r.a2.lockup_escape_ratio,
                fault_hypothesis,
            })
        })
        .collect();
    Column {
        confusion: confusion(&cells),
        false_positives: false_positives(&cells),
        agree: count(Agreement::Agree),
        producer_only: count(Agreement::ProducerOnly),
        false_negative: count(Agreement::FalseNegative),
        false_negative_mixed: count(Agreement::FalseNegativeMixed),
        undecided: count(Agreement::Undecided),
        unobserved: count(Agreement::Unobserved),
        disagreements,
    }
}

#[derive(serde::Serialize)]
struct Summary {
    /// The horizon(s) the rows were run at (one, unless files were mixed).
    horizons: Vec<u64>,
    atlas_configs: usize,
    sampled_configs: usize,
    configs_run: usize,
    total_runs: usize,
    runs_collapsed: usize,
    /// Runs stopped by the simulation budget: neither collapsed nor
    /// persisted, left out of every ensemble read.
    runs_timed_out: usize,
    /// Runs whose evaluation exhausted the evaluation budget: likewise left
    /// out, counted apart from the simulation timeouts.
    runs_eval_timed_out: usize,
    a1: Column,
    a2: Column,
}

fn summarise(records: &[ConfigRecord]) -> Summary {
    let mut horizons: Vec<u64> = records.iter().map(|r| r.horizon).collect();
    horizons.sort_unstable();
    horizons.dedup();
    Summary {
        horizons,
        atlas_configs: records
            .iter()
            .filter(|r| r.source == ConfigSource::Atlas)
            .count(),
        sampled_configs: records
            .iter()
            .filter(|r| r.source == ConfigSource::Sample)
            .count(),
        configs_run: records.len(),
        total_runs: records.iter().map(|r| r.seeds.len()).sum(),
        runs_collapsed: records.iter().map(|r| r.aggregate.collapsed).sum(),
        runs_timed_out: records.iter().map(|r| r.aggregate.timed_out).sum(),
        runs_eval_timed_out: records.iter().map(|r| r.aggregate.eval_timed_out).sum(),
        a1: column(records, true),
        a2: column(records, false),
    }
}

/// Command line: `--limit N`, `--horizon T`, `--out PATH`, `--atlas PATH`,
/// `--seeds N` (1..=8), `--configs atlas:0,sample:12`,
/// `--run-timeout-secs N` (simulation budget), `--eval-timeout-secs N`
/// (evaluation budget), `--summary`.
#[derive(Clone, Debug, PartialEq)]
struct Args {
    limit: Option<usize>,
    horizon: u64,
    out: PathBuf,
    atlas: PathBuf,
    seeds: u64,
    configs: Option<HashSet<(ConfigSource, usize)>>,
    run_timeout: Duration,
    eval_timeout: Duration,
    summary_only: bool,
}

const DEFAULT_RUN_TIMEOUT_SECS: u64 = 300;
const DEFAULT_OUT: &str = "target/permanence-crosscheck.jsonl";

fn parse_args<I: IntoIterator<Item = String>>(argv: I) -> Args {
    let mut args = Args {
        limit: None,
        horizon: SearchConfig::default().max_ticks,
        out: PathBuf::from(DEFAULT_OUT),
        atlas: PathBuf::from("atlas.json"),
        seeds: N_SEEDS,
        configs: None,
        run_timeout: Duration::from_secs(DEFAULT_RUN_TIMEOUT_SECS),
        eval_timeout: Duration::from_secs(DEFAULT_EVAL_TIMEOUT_SECS),
        summary_only: false,
    };
    let mut it = argv.into_iter();
    let value = |flag: &str, it: &mut I::IntoIter| -> String {
        it.next()
            .unwrap_or_else(|| panic!("permanence_crosscheck: {flag} needs a value"))
    };
    let number = |flag: &str, raw: &str| -> u64 {
        raw.parse()
            .unwrap_or_else(|_| panic!("permanence_crosscheck: {flag} {raw:?} is not an integer"))
    };
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--limit" => args.limit = Some(number("--limit", &value("--limit", &mut it)) as usize),
            "--horizon" => {
                args.horizon = number("--horizon", &value("--horizon", &mut it));
                assert!(
                    args.horizon > 0,
                    "permanence_crosscheck: --horizon must be positive"
                );
            }
            "--out" => args.out = PathBuf::from(value("--out", &mut it)),
            "--atlas" => args.atlas = PathBuf::from(value("--atlas", &mut it)),
            "--seeds" => {
                args.seeds = number("--seeds", &value("--seeds", &mut it)).clamp(1, N_SEEDS)
            }
            "--configs" => {
                args.configs = Some(parse_selector(
                    &value("--configs", &mut it),
                    "--configs",
                    None,
                ))
            }
            "--run-timeout-secs" => {
                args.run_timeout = Duration::from_secs(number(
                    "--run-timeout-secs",
                    &value("--run-timeout-secs", &mut it),
                ))
            }
            EVAL_TIMEOUT_FLAG => {
                args.eval_timeout = Duration::from_secs(number(
                    EVAL_TIMEOUT_FLAG,
                    &value(EVAL_TIMEOUT_FLAG, &mut it),
                ))
            }
            "--summary" => args.summary_only = true,
            other => panic!("permanence_crosscheck: unknown argument {other:?}"),
        }
    }
    args
}

/// Run the configs not yet in `args.out` (in sweep order, up to `args.limit`),
/// appending one row each as it completes. Returns how many were run.
fn sweep(args: &Args, atlas_units: &[Vec<f64>], sampled: &[Vec<f64>]) -> usize {
    let done = done_configs(&args.out);
    let tasks = plan_tasks(
        atlas_units.len(),
        sampled.len(),
        args.configs.as_ref(),
        &done,
        args.limit,
    );
    eprintln!(
        "permanence_crosscheck: {} atlas + {} sampled configs × {} seeds, horizon {} ticks; {} done in {}, running {} now",
        atlas_units.len(),
        sampled.len(),
        args.seeds,
        args.horizon,
        done.len(),
        args.out.display(),
        tasks.len()
    );
    let start = Instant::now();
    let total = tasks.len();
    for (n, (source, idx)) in tasks.iter().copied().enumerate() {
        let unit = match source {
            ConfigSource::Atlas => &atlas_units[idx],
            ConfigSource::Sample => &sampled[idx],
        };
        let record = evaluate_config(
            source,
            idx,
            unit,
            args.seeds,
            args.horizon,
            args.run_timeout,
            args.eval_timeout,
        );
        append_row(&args.out, &record);
        eprintln!(
            "  {:?}:{idx} done ({}/{total}, {}/{} collapsed, {} timed out, {} eval timed out, A1 {:?}, A2 {:?}, {:.0}s elapsed)",
            source,
            n + 1,
            record.aggregate.collapsed,
            record.aggregate.n,
            record.aggregate.timed_out,
            record.aggregate.eval_timed_out,
            record.agreement_a1,
            record.agreement_a2,
            start.elapsed().as_secs_f64()
        );
    }
    eprintln!(
        "permanence_crosscheck: {total} configs run in {:.0}s; appended to {}",
        start.elapsed().as_secs_f64(),
        args.out.display()
    );
    total
}

fn main() {
    let args = parse_args(std::env::args().skip(1));
    if !args.summary_only {
        let atlas_units = read_atlas_units(&args.atlas);
        let sampled = sampled_units(default_ranges().len());
        sweep(&args, &atlas_units, &sampled);
    }
    let records: Vec<ConfigRecord> = read_rows(&args.out);
    eprintln!(
        "permanence_crosscheck: summarising {} rows from {}",
        records.len(),
        args.out.display()
    );
    print_summary(&summarise(&records));
}

fn print_column(name: &str, c: &Column) {
    println!("## {name}");
    println!("  confusion (rows predicted, cols observed)   collapse   mixed  persist");
    for (i, label) in ["permanent", "not-permanent", "undecided"]
        .iter()
        .enumerate()
    {
        println!(
            "  {:<42} {:>8} {:>7} {:>8}",
            label, c.confusion[i][0], c.confusion[i][1], c.confusion[i][2]
        );
    }
    let fp = &c.false_positives;
    println!(
        "  false positives (predicted permanent → observed collapse): strict {}/{} = {:.3}, any-seed {}/{} = {:.3}",
        fp.strict,
        fp.predicted_permanent,
        fp.strict_rate,
        fp.any_seed,
        fp.predicted_permanent,
        fp.any_seed_rate
    );
    println!(
        "  agree {}, producer-only (clause 2 fails, no consumer at horizon) {}, false negative {} (+{} mixed), undecided {}, unobserved {}",
        c.agree,
        c.producer_only,
        c.false_negative,
        c.false_negative_mixed,
        c.undecided,
        c.unobserved
    );
    println!("  disagreements: {}", c.disagreements.len());
    for d in &c.disagreements {
        println!(
            "    {:?}:{} {:?}→{:?} ({}/{} collapse, modal {}; rho {:.2}, I {:.2}, Lambda {:.3e}) — {}",
            d.source,
            d.config_index,
            d.predicted,
            d.observed,
            d.collapsed,
            d.n,
            d.modal_mode,
            d.rho,
            d.invasion_ratio,
            d.lockup_escape_ratio,
            d.fault_hypothesis
        );
    }
    println!();
}

fn print_summary(s: &Summary) {
    println!("\n# Permanence cross-check (issue #439, against #432 / #437)");
    println!(
        "# {} configs ({} atlas + {} sampled) = {} runs, horizon(s) {:?} ticks",
        s.atlas_configs + s.sampled_configs,
        s.atlas_configs,
        s.sampled_configs,
        s.total_runs,
        s.horizons
    );
    println!(
        "# runs collapsed (extinction | energy-death | nutrient-lockup): {}; timed out (excluded): {}; eval timed out (excluded): {}\n",
        s.runs_collapsed, s.runs_timed_out, s.runs_eval_timed_out
    );
    print_column("A1 only (rho > 1 and I > 1 at the seeded centroids)", &s.a1);
    print_column(
        "A2 coupled (clauses i-iii at the reference lumping; a clause failing only there → undecided)",
        &s.a2,
    );
    println!(
        "{}",
        serde_json::to_string_pretty(s).expect("serialise summary")
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_sim::WorldRecipe;

    fn example10() -> (TraitVector, TraitVector, WorldParameters) {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scenarios/example10_predator_prey_hopf.json"
        );
        let recipe: WorldRecipe =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let agents = recipe.agents.unwrap();
        let producer = agents
            .iter()
            .find(|a| a.traits.photosynthetic_absorption >= a.traits.heterotrophy)
            .unwrap()
            .traits;
        let consumer = agents
            .iter()
            .find(|a| a.traits.photosynthetic_absorption < a.traits.heterotrophy)
            .unwrap()
            .traits;
        (producer, consumer, recipe.parameters)
    }

    fn mean(photo: f32, hetero: f32, n_clusters: u32) -> InitialDistribution {
        InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: photo,
                heterotrophy: hetero,
                mobility: 0.2,
                kappa: 0.5,
                fecundity: 0.35,
                asexual_propensity: 0.1,
                dispersal: 0.3,
            },
            trait_covariance: 0.1,
            initial_cluster_count: n_clusters,
            initial_energy_per_agent: 10.0,
        }
    }

    #[test]
    fn representative_clusters_follow_world_new_seeding_rule() {
        // Two or more clusters: pure producer and pure consumer at the trophic
        // total, every other dimension shared with the mean.
        let (p, c) = representative_clusters(&mean(0.6, 0.2, 3));
        assert_eq!(p.photosynthetic_absorption, 0.8);
        assert_eq!(p.heterotrophy, 0.0);
        assert_eq!(c.photosynthetic_absorption, 0.0);
        assert_eq!(c.heterotrophy, 0.8);
        assert_eq!(p.mobility, 0.2);
        assert_eq!(c.kappa, 0.5);
        // One cluster: both are the mean.
        let (p, c) = representative_clusters(&mean(0.6, 0.2, 1));
        assert_eq!(p, mean(0.6, 0.2, 1).mean_traits);
        assert_eq!(p, c);
        // A zero trophic total is not split either.
        let (p, c) = representative_clusters(&mean(0.0, 0.0, 4));
        assert_eq!(p, c);
    }

    #[test]
    fn a1_verdict_reproduces_the_prototype_on_example10() {
        // Numbers quoted in 432-permanence-pc.md and printed by
        // permanence_prototype on the committed scenario.
        let (producer, consumer, params) = example10();
        let v = a1_verdict(
            A1Map::derive(&producer, &consumer, &params),
            &producer,
            &consumer,
        );
        assert!((v.rho - 71.111).abs() < 1e-2, "rho = {}", v.rho);
        // #482: β = χ_C·γ·e·a with χ_C = 0.5012 at κ_C = 0.45 (pre-#482 the
        // κ_C lumping gave β = 0.03677, I = 23.135).
        assert!(
            (v.invasion_ratio - 25.766).abs() < 1e-2,
            "I = {}",
            v.invasion_ratio
        );
        assert!((v.map.r_p - 1.3479).abs() < 1e-3, "r_P = {}", v.map.r_p);
        assert!((v.map.m - 0.1130).abs() < 1e-3);
        assert!((v.map.beta - 0.04095).abs() < 1e-4, "beta = {}", v.map.beta);
        assert_eq!(v.prediction, Prediction::Permanent);
        assert!(v.hypothesis_h1);
    }

    /// `κ_P = 0` routes the whole surplus to offspring, which are biomass the
    /// map carries: clause (1) is `ρ > 1`, not `ρ > 1 ∧ κ_P > 0` (#466). This
    /// is the shape of the ten atlas cells A3 listed as clause-(1) failures.
    #[test]
    fn a1_clause_one_holds_at_kappa_zero_when_rho_exceeds_one() {
        let (mut producer, consumer, params) = example10();
        producer.kappa = 0.0;
        let v = a1_verdict(
            A1Map::derive(&producer, &consumer, &params),
            &producer,
            &consumer,
        );
        assert!(v.rho > 1.0);
        assert!(v.map.r_p > 0.0, "r_P = {}", v.map.r_p);
        assert!(v.producer_face_alive);
        assert_eq!(v.prediction, Prediction::Permanent);
        assert_eq!(v.kappa_p, 0.0);
    }

    #[test]
    fn a1_verdict_is_not_permanent_when_either_clause_fails() {
        let (producer, consumer, params) = example10();
        // Clause 1: flux at the producer's maintenance floor.
        let mut p = params.clone();
        p.solar_flux_magnitude = 0.1;
        let v = a1_verdict(
            A1Map::derive(&producer, &consumer, &p),
            &producer,
            &consumer,
        );
        assert!(!v.producer_face_alive);
        assert_eq!(v.prediction, Prediction::NotPermanent);
        // Clause 2: a consumer with no heterotrophy cannot invade.
        let mut c = consumer;
        c.heterotrophy = 0.0;
        let v = a1_verdict(A1Map::derive(&producer, &c, &params), &producer, &c);
        assert!(v.producer_face_alive && !v.consumer_invades);
        assert_eq!(v.prediction, Prediction::NotPermanent);
    }

    #[test]
    fn a1_verdict_is_undecided_when_h1_fails_but_the_clauses_hold() {
        let (producer, consumer, params) = example10();
        // r_P > 2: the producer face carries a cycle, clause 2 is necessary
        // but no longer sufficient.
        let mut p = params.clone();
        p.solar_flux_magnitude = 20.0;
        let v = a1_verdict(
            A1Map::derive(&producer, &consumer, &p),
            &producer,
            &consumer,
        );
        assert!(v.map.r_p > 2.0);
        assert!(v.producer_face_alive && v.consumer_invades);
        assert_eq!(v.prediction, Prediction::Undecided);
    }
    #[test]
    fn a2_verdict_reproduces_the_prototype_on_example10() {
        let (producer, consumer, params) = example10();
        let map = A1Map::derive(&producer, &consumer, &params);
        let v = a2_verdict(&map, &producer, &consumer, &params);
        assert!((v.theta_p - 0.20).abs() < 1e-6);
        assert!((v.theta_c - 0.24).abs() < 1e-6);
        // #482: Λ carries χ_C·γ·e_C in its Liebig branch (pre-#482 the κ_C
        // lumping gave Λ = 81333.9).
        assert!(
            (v.lockup_escape_ratio - 90583.8).abs() < 1.0,
            "Lambda = {}",
            v.lockup_escape_ratio
        );
        assert!(v.producer_invades_virgin && v.heterotroph_invades_lockup && v.cycle_repels);
        assert_eq!(
            a2_prediction(&a1_verdict(map, &producer, &consumer), &v),
            Prediction::Permanent
        );
    }

    /// `κ_C = 0` routes the consumer's whole surplus to offspring, which are
    /// biomass the map carries: clause (2) is `I > 1`, not `I > 1 ∧ κ_C > 0`,
    /// and A2's `Λ` keeps its energy branch (#482). This is the shape of the
    /// nineteen atlas cells A3 tagged `#482`.
    #[test]
    fn clause_two_and_lambda_hold_at_kappa_c_zero_when_the_consumer_can_feed() {
        let (producer, mut consumer, params) = example10();
        consumer.kappa = 0.0;
        let map = A1Map::derive(&producer, &consumer, &params);
        let a1 = a1_verdict(map, &producer, &consumer);
        assert!(a1.map.beta > 0.0, "beta = {}", a1.map.beta);
        assert!(a1.producer_face_alive && a1.consumer_invades);
        assert_eq!(a1.prediction, Prediction::Permanent);
        assert_eq!(a1.kappa_c, 0.0);
        let a2 = a2_verdict(&map, &producer, &consumer, &params);
        assert!(
            a2.lockup_escape_ratio > 1.0,
            "Lambda = {}",
            a2.lockup_escape_ratio
        );
        assert!(a2.heterotroph_invades_lockup);
    }

    #[test]
    fn a2_prediction_separates_config_only_kills_from_reference_lumping_failures() {
        let (producer, consumer, params) = example10();
        // No nutrient at all: Λ = 0 at the reference — but ν, q, ι are
        // endogenous and Λ > 0, so this is undecided, not a call.
        let mut p = params.clone();
        p.initial_nutrient_pool = 1e-3;
        let map = A1Map::derive(&producer, &consumer, &p);
        let a1 = a1_verdict(map, &producer, &consumer);
        let a2 = a2_verdict(&map, &producer, &consumer, &p);
        assert_eq!(a1.prediction, Prediction::Permanent);
        assert!(!a2.heterotroph_invades_lockup && a2.lockup_escape_ratio > 0.0);
        assert_eq!(a2_prediction(&a1, &a2), Prediction::Undecided);
        // A consumer with no heterotrophy: Λ = 0 for every lumping — a kill.
        let mut c = consumer;
        c.heterotrophy = 0.0;
        let map = A1Map::derive(&producer, &c, &params);
        let a1 = a1_verdict(map, &producer, &c);
        let a2 = a2_verdict(&map, &producer, &c, &params);
        assert_eq!(a2.lockup_escape_ratio, 0.0);
        assert_eq!(a2_prediction(&a1, &a2), Prediction::NotPermanent);
        // Flux at the floor: r_P ≤ 0, so no μ_P ≥ 0 lets the producer invade.
        let mut p = params.clone();
        p.solar_flux_magnitude = 0.1;
        let map = A1Map::derive(&producer, &consumer, &p);
        let a1 = a1_verdict(map, &producer, &consumer);
        let a2 = a2_verdict(&map, &producer, &consumer, &p);
        assert!(a2.virgin_invasion_rate <= 0.0);
        assert_eq!(a2_prediction(&a1, &a2), Prediction::NotPermanent);
        // A1's clause 2 is not an A2 clause: a consumer that cannot invade the
        // standing crop (I ∝ F, ≤ 1 at low flux) can still be A2-permanent via
        // the pile (Λ ∝ N_total, independent of F).
        let mut p = params.clone();
        p.solar_flux_magnitude = 0.3;
        let map = A1Map::derive(&producer, &consumer, &p);
        let a1 = a1_verdict(map, &producer, &consumer);
        let a2 = a2_verdict(&map, &producer, &consumer, &p);
        assert_eq!(a1.prediction, Prediction::NotPermanent);
        assert!(a1.producer_face_alive && !a1.consumer_invades);
        assert_eq!(a2_prediction(&a1, &a2), Prediction::Permanent);
    }

    fn outcome(seed: u64, mode: &'static str, consumers: usize) -> SeedOutcome {
        SeedOutcome {
            seed,
            mode: mode.to_string(),
            collapsed: is_collapse(mode),
            termination_tick: 500,
            founders: 10,
            population_after_tick1: 5,
            producers_after_tick1: 5,
            consumers_after_tick1: 5,
            terminal_producers: 5,
            terminal_consumers: consumers,
            decomposer_guild: false,
            peak_population: 40,
            first_tick_without_consumers: None,
            first_tick_without_producers: None,
        }
    }

    #[test]
    fn aggregate_reads_unanimity_and_the_mixed_split() {
        let seeds = [
            outcome(0, "extinction", 0),
            outcome(1, "extinction", 0),
            outcome(2, "energy-death", 0),
        ];
        let agg = aggregate(&seeds);
        assert_eq!(agg.observed, Observed::Collapse);
        assert_eq!(
            (agg.modal_mode.as_str(), agg.modal_count),
            ("extinction", 2)
        );
        assert_eq!(agg.collapsed, 3);

        let seeds = [
            outcome(0, "none", 3),
            outcome(1, "extinction", 0),
            outcome(2, "monoculture", 0),
        ];
        let agg = aggregate(&seeds);
        assert_eq!(agg.observed, Observed::Mixed);
        assert!((agg.collapse_fraction - 1.0 / 3.0).abs() < 1e-12);
        assert_eq!(agg.persisting_seeds_with_consumers, 1);

        let seeds = [outcome(0, "none", 0), outcome(1, "explosion", 2)];
        let agg = aggregate(&seeds);
        assert_eq!(agg.observed, Observed::Persist);
    }

    #[test]
    fn classification_separates_the_dangerous_direction_from_producer_only_persistence() {
        let (producer, consumer, params) = example10();
        let permanent = a1_verdict(
            A1Map::derive(&producer, &consumer, &params),
            &producer,
            &consumer,
        );
        let collapse = aggregate(&[outcome(0, "extinction", 0)]);
        let mixed = aggregate(&[outcome(0, "extinction", 0), outcome(1, "none", 1)]);
        let persist_no_consumers = aggregate(&[outcome(0, "monoculture", 0)]);
        let persist_with_consumers = aggregate(&[outcome(0, "none", 2)]);
        assert_eq!(
            classify(Prediction::Permanent, &permanent, &collapse),
            Agreement::FalsePositive
        );
        assert_eq!(
            classify(Prediction::Permanent, &permanent, &mixed),
            Agreement::FalsePositiveMixed
        );
        assert_eq!(
            classify(Prediction::Permanent, &permanent, &persist_with_consumers),
            Agreement::Agree
        );
        // Clause 2 fails only: a producer-only world is what A1 predicts.
        let mut c = consumer;
        c.heterotrophy = 0.0;
        let no_invasion = a1_verdict(A1Map::derive(&producer, &c, &params), &producer, &c);
        assert_eq!(
            classify(
                Prediction::NotPermanent,
                &no_invasion,
                &persist_no_consumers
            ),
            Agreement::ProducerOnly
        );
        assert_eq!(
            classify(
                Prediction::NotPermanent,
                &no_invasion,
                &persist_with_consumers
            ),
            Agreement::FalseNegative
        );
        assert_eq!(
            classify(Prediction::NotPermanent, &no_invasion, &collapse),
            Agreement::Agree
        );
        assert_eq!(
            classify(Prediction::Undecided, &permanent, &collapse),
            Agreement::Undecided
        );
    }

    #[test]
    fn fault_hypothesis_names_the_reduction_exit_and_is_silent_on_agreement() {
        let (producer, consumer, params) = example10();
        let permanent = a1_verdict(
            A1Map::derive(&producer, &consumer, &params),
            &producer,
            &consumer,
        );
        let lockup = aggregate(&[outcome(0, "nutrient-lockup", 0)]);
        let h = fault_hypothesis(Agreement::FalsePositive, &permanent, &lockup, false).unwrap();
        assert!(h.contains("nutrient lockup"), "{h}");
        assert!(fault_hypothesis(Agreement::Agree, &permanent, &lockup, false).is_none());
    }

    /// `β = χ_C·γ·e·a` (#482) is zero only where `χ_C` is — `κ_C = 0` with a
    /// reproductive branch that burns everything (here a propagule share of
    /// 1). A false negative on such a cell is attributed to the degenerate
    /// conversion, the consumer twin of the `χ_P = 0` hypothesis, before any
    /// other cause; plain `κ_C = 0` no longer produces one.
    #[test]
    fn fault_hypothesis_names_a_zero_chi_c_false_negative() {
        let (producer, mut consumer, params) = example10();
        consumer.kappa = 0.0;
        let mut burnt = params.clone();
        burnt.dispersal_propagule_cost_coefficient = 1.0;
        burnt.dispersal_propagule_cost_exponent = 1.0;
        let v = a1_verdict(
            A1Map::derive(&producer, &consumer, &burnt),
            &producer,
            &consumer,
        );
        assert_eq!(v.map.beta, 0.0);
        assert!(v.producer_face_alive && !v.consumer_invades);
        let persist = aggregate(&[outcome(0, "none", 2)]);
        let h = fault_hypothesis(Agreement::FalseNegative, &v, &persist, false).unwrap();
        assert!(h.contains("chi_C is zero"), "{h}");
    }

    #[test]
    fn confusion_matrix_and_false_positive_rates_count_cells() {
        use Observed::*;
        use Prediction::*;
        let cells = [
            (Permanent, Persist),
            (Permanent, Collapse),
            (Permanent, Collapse),
            (Permanent, Mixed),
            (NotPermanent, Collapse),
            (Undecided, Persist),
        ];
        let m = confusion(&cells);
        assert_eq!(m, [[2, 1, 1], [1, 0, 0], [0, 0, 1]]);
        let fp = false_positives(&cells);
        assert_eq!((fp.predicted_permanent, fp.strict, fp.any_seed), (4, 2, 3));
        assert!((fp.strict_rate - 0.5).abs() < 1e-12);
        assert!((fp.any_seed_rate - 0.75).abs() < 1e-12);
    }
    /// `World::new` floors every founder trait at zero (#444), so no founder
    /// can be culled for a negative trait and the #444 tag has nothing to
    /// tag: the report carries no #444 field at any level.
    #[test]
    fn report_carries_no_444_tagging_fields() {
        let seeds = [outcome(0, "extinction", 0), outcome(1, "none", 2)];
        let agg = aggregate(&seeds);
        let fp = false_positives(&[(Prediction::Permanent, Observed::Collapse)]);
        for json in [
            serde_json::to_string(&seeds[0]).unwrap(),
            serde_json::to_string(&agg).unwrap(),
            serde_json::to_string(&fp).unwrap(),
        ] {
            for key in ["444", "doa", "negative_founders"] {
                assert!(!json.contains(key), "{key} in {json}");
            }
        }
    }

    #[test]
    fn evaluate_config_produces_a_record_per_seed_with_a_terminal_mode() {
        // Smoke check only: no assertion on the emergent outcome.
        let ranges = default_ranges();
        let unit = vec![0.5; ranges.len()];
        let record = evaluate_config(
            ConfigSource::Sample,
            0,
            &unit,
            2,
            20,
            Duration::MAX,
            Duration::MAX,
        );
        assert_eq!(record.seeds.len(), 2);
        assert_eq!(record.aggregate.n, 2);
        assert!(record.seeds.iter().all(|s| s.termination_tick <= 20));
        assert!(record.seeds.iter().all(|s| !s.mode.is_empty()));
        let _ = serde_json::to_string(&record).expect("record serialises");
    }
}

#[cfg(test)]
mod resume_tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("permanence-crosscheck-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn args_default_to_the_full_sweep_at_the_search_horizon_and_accept_each_flag() {
        let args = parse_args(std::iter::empty());
        assert_eq!(args.limit, None);
        assert_eq!(args.horizon, SearchConfig::default().max_ticks);
        assert_eq!(args.out, PathBuf::from(DEFAULT_OUT));
        assert_eq!(args.seeds, N_SEEDS);
        assert_eq!(args.configs, None);
        assert_eq!(args.run_timeout, Duration::from_secs(300));
        assert_eq!(args.eval_timeout, Duration::from_secs(300));
        assert!(!args.summary_only);
        let args = parse_args(
            "--limit 3 --horizon 600 --out target/x.jsonl --atlas a.json --seeds 2 --configs atlas:0,atlas:1 --run-timeout-secs 7 --eval-timeout-secs 11 --summary"
                .split(' ')
                .map(String::from),
        );
        assert_eq!(args.limit, Some(3));
        assert_eq!(args.horizon, 600);
        assert_eq!(args.out, PathBuf::from("target/x.jsonl"));
        assert_eq!(args.atlas, PathBuf::from("a.json"));
        assert_eq!(args.seeds, 2);
        assert_eq!(args.configs.as_ref().map(|c| c.len()), Some(2));
        assert_eq!(args.run_timeout, Duration::from_secs(7));
        assert_eq!(args.eval_timeout, Duration::from_secs(11));
        assert!(args.summary_only);
    }

    /// The resumable contract: a sweep split into `--limit` calls appends
    /// the same rows, in the same order, as one uninterrupted call.
    #[test]
    fn a_sweep_split_in_two_produces_the_same_file_as_one_run() {
        let dims = default_ranges().len();
        let atlas = vec![vec![0.5; dims], vec![0.4; dims]];
        let sample = vec![vec![0.6; dims]];
        let base = Args {
            limit: None,
            horizon: 20,
            out: tmp("one-shot.jsonl"),
            atlas: PathBuf::new(),
            seeds: 2,
            configs: None,
            run_timeout: Duration::MAX,
            eval_timeout: Duration::MAX,
            summary_only: false,
        };
        assert_eq!(sweep(&base, &atlas, &sample), 3);
        assert_eq!(sweep(&base, &atlas, &sample), 0, "nothing left to run");

        let split = Args {
            limit: Some(2),
            out: tmp("split.jsonl"),
            ..base.clone()
        };
        assert_eq!(sweep(&split, &atlas, &sample), 2);
        assert_eq!(sweep(&split, &atlas, &sample), 1);
        assert_eq!(sweep(&split, &atlas, &sample), 0);

        let one = std::fs::read(&base.out).unwrap();
        let two = std::fs::read(&split.out).unwrap();
        assert!(!one.is_empty());
        assert_eq!(one, two);
        let rows: Vec<ConfigRecord> = read_rows(&base.out);
        assert_eq!(
            rows.iter()
                .map(|r| (r.source, r.config_index))
                .collect::<Vec<_>>(),
            vec![
                (ConfigSource::Atlas, 0),
                (ConfigSource::Atlas, 1),
                (ConfigSource::Sample, 0)
            ]
        );
        assert!(rows.iter().all(|r| r.seeds.len() == 2 && r.horizon == 20));
        std::fs::remove_dir_all(base.out.parent().unwrap()).ok();
    }

    /// A timed-out seed is neither a collapse nor a persistence: the config's
    /// aggregate is read over the seeds that finished, and a config with no
    /// finished seed is reported as unobserved rather than judged.
    #[test]
    fn a_run_past_its_wall_clock_budget_reads_as_a_timeout_and_is_not_judged() {
        let dims = default_ranges().len();
        let unit = vec![0.5; dims];
        let record = evaluate_config(
            ConfigSource::Sample,
            0,
            &unit,
            1,
            20,
            Duration::ZERO,
            Duration::MAX,
        );
        assert_eq!(record.seeds[0].mode, "timeout");
        assert!(!record.seeds[0].collapsed);
        assert_eq!(record.aggregate.n, 0);
        assert_eq!(record.aggregate.timed_out, 1);
        assert_eq!(record.agreement_a1, Agreement::Unobserved);
        assert_eq!(record.agreement_a2, Agreement::Unobserved);
        let summary = summarise(&[record]);
        assert_eq!(summary.runs_timed_out, 1);
        assert_eq!(summary.a1.unobserved, 1);
        assert_eq!(summary.a1.false_positives.predicted_permanent, 0);
    }

    /// A seed that simulated to the horizon but whose evaluation overran its
    /// budget reached no verdict: it is neither a collapse nor a persistence,
    /// carries no breakdown-derived observable, and is counted apart from
    /// the simulation timeouts.
    #[test]
    fn a_run_whose_evaluation_overruns_its_budget_reads_as_an_eval_timeout_and_is_not_judged() {
        let dims = default_ranges().len();
        let unit = vec![0.5; dims];
        let record = evaluate_config(
            ConfigSource::Sample,
            0,
            &unit,
            1,
            20,
            Duration::MAX,
            Duration::ZERO,
        );
        let seed = &record.seeds[0];
        assert_eq!(seed.termination_tick, 20, "the rollout reached the horizon");
        assert_eq!(seed.mode, EVAL_TIMEOUT_MODE);
        assert!(!seed.collapsed);
        assert!(
            !seed.decomposer_guild,
            "the not-read value, as under timeout"
        );
        assert_eq!(record.aggregate.n, 0);
        assert_eq!(
            record.aggregate.timed_out, 0,
            "not folded into the timeouts"
        );
        assert_eq!(record.aggregate.eval_timed_out, 1);
        assert_eq!(record.agreement_a1, Agreement::Unobserved);
        assert_eq!(record.agreement_a2, Agreement::Unobserved);
        let summary = summarise(&[record]);
        assert_eq!(summary.runs_timed_out, 0);
        assert_eq!(summary.runs_eval_timed_out, 1);
        assert_eq!(summary.a1.unobserved, 1);
    }

    /// A row with no evaluation timeout serialises exactly as it did before
    /// the evaluation budget existed: no new key.
    #[test]
    fn a_row_without_an_eval_timeout_carries_no_new_key() {
        let dims = default_ranges().len();
        let unit = vec![0.5; dims];
        let record = evaluate_config(
            ConfigSource::Sample,
            0,
            &unit,
            1,
            20,
            Duration::MAX,
            Duration::MAX,
        );
        assert_ne!(record.seeds[0].mode, EVAL_TIMEOUT_MODE);
        let line = serde_json::to_string(&record).unwrap();
        assert!(!line.contains("eval_timed_out"), "{line}");
    }
}
