//! Re-invasion barrier on the carcass pile (issue #587).
//!
//! ## The question
//!
//! `sample:31` blooms and then locks up: nutrient ends in a carcass pile no
//! one eats. Its founder heterotrophs die by tick 100–150, before any pile
//! exists, and across ~1,700 further ticks of producer births no heterotroph
//! reappears. Is lockup a **re-invasion barrier** — can no heterotroph
//! establish on the pile once it exists — and if so, is it the phenotype
//! (a full heterotroph cannot live there) or the path (the small steps
//! selection would have to climb do not pay)?
//!
//! ## Protocol
//!
//! `invasion_growth`'s protocol ([`explorers_search::invasion`]), with an
//! **explicit phenotype** in place of a realised role centroid: that bin
//! injects only roles the resident holds and reads an absent role as
//! `not_testable` (#491), which is exactly this case.
//!
//! Per seed: `World::new` on the referenced config, stepped to the injection
//! tick (`--t-inj`, default 1000, after the bloom's peak), then forked into
//! an uninjected **control** and one injection per arm. An arm is a
//! phenotype × a placement:
//!
//! - phenotype **`full`** — the founder heterotroph centroid the recipe's
//!   initial distribution seeds (`World::new`'s odd cluster: all of the
//!   trophic budget on heterotrophy, the other traits at the mean);
//! - phenotype **`step_k`** (k = 1, 2, 4) — the resident producer centroid
//!   at the injection tick (`topology::trophic_roles`, as `invasion_growth`)
//!   with heterotrophy raised by `k × mutation_magnitude` (the standard
//!   deviation of one mutation step), everything else unchanged;
//! - placement **`uniform`** — cohort positions uniform over the world (as
//!   `invasion_growth`);
//! - placement **`pile`** — cohort positions uniform within the
//!   [`PILE_CELLS`] nutrient-grid cells holding the most carcass nutrient,
//!   dealt round-robin from the top cell.
//!
//! Each cohort is [`INVADER_COHORT`] agents provisioned as founders; its
//! lineage is followed by descent off `Born` events (either-parent rule),
//! its diet split living / carcass, and its rate is
//! `r = ln(max(N_end, ½) / N_0) / ticks` over the window (`--window`,
//! default 1000, so the window ends at the `T = 2000` settled horizon).
//! Per arm, the seed ensemble is read as `invasion_growth` reads it: median
//! `r` and the sign test's interval (at `n = 8`, `[min, max]`); the arm
//! *invades* when the median is > 0 and the interval clears zero. Carcass
//! nutrient returned to the pool is read against the control fork at the
//! window end: `carcass_drawdown = carcass N (control) − carcass N (arm)`,
//! and the available pool's gain likewise.
//!
//! ## Determinism, sourcing, output
//!
//! The config reference is the shared `config_source` key (`sample:31`
//! default; `sample@S:i`; `atlas:N` with `--atlas PATH`). Seeds are the
//! fixed block `SEED_BASE + i`; cohort positions come from a stream keyed on
//! (seed, arm); seeds run in parallel with an order-stable collect, so the
//! artifact is byte-identical across runs. Chunking: `--seed-from A
//! --seeds N` runs seeds `A..A+N` and writes their records; `--merge F…`
//! re-reads chunk files and prints the combined summary. Artifact
//! `target/reinvasion-barrier.json` (`--out PATH`).
//!
//! ## Accounting mode (#591)
//!
//! `--accounting` runs the named arms (`step 0`, `step 1`, `full`, each at
//! both placements) under [`LineageAccountant`], which books each lineage's
//! realised energy account per member-tick: carcasses in reach against
//! #589's break-even `k*`, carcass intake against the `k · h · u_H · e`
//! ceiling (the gap split into wear, exhaustion and sharing), photosynthesis
//! against its unshaded ceiling, every maintenance term, growth loss,
//! movement, grazing (and by whom), the reproductive earmark and its gates,
//! deaths and births. The control fork books the resident's own consumers
//! (role consumer or decomposer at the injection tick) the same way: the
//! contrast case. The account reconciles with the stepper's energy ledger;
//! the tables print the worst residual. Artifact
//! `target/carcass-income-accounting.json`. The lineage read (rates, diet,
//! deaths) is the plain mode's, unchanged.
//!
//! Run with:
//!   cargo run --release -p explorers-search --bin reinvasion_barrier -- sample:31
//!   cargo run --release -p explorers-search --bin reinvasion_barrier -- sample:31 --accounting
//!
//! No change to the stepper, the evaluator or the search; no assertion on
//! emergent values beyond smoke checks.

use std::collections::HashMap;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

use explorers_genesis::EvalConfig;
use explorers_search::config_source::{parse_config_key, resolve_config, sampled_units};
use explorers_search::energy_accounting::{EnergyAccount, LineageAccountant};
use explorers_search::invasion::{
    ConsumedCounts, DrainedEnergy, Lineage, RateSummary, SERIES_INTERVAL, WindowOutcome,
    growth_rate, median, place_cohort, run_window, summarise_rates,
};
use explorers_search::search::default_ranges;
use explorers_search::sweep::read_atlas_units;
use explorers_sim::event::EventKind;
use explorers_sim::topology::{TopologyProjection, TrophicRole};
use explorers_sim::{Agent, InitialDistribution, TraitVector, World, WorldParameters};

/// Fixed contiguous seed block (the `invasion_growth` convention).
const N_SEEDS: u64 = 8;
const SEED_BASE: u64 = 1000;
/// Invader cohort size, as `invasion_growth`.
const INVADER_COHORT: usize = 8;
/// Nutrient-grid cells the `pile` placement spreads the cohort over.
const PILE_CELLS: usize = 2;
/// The small-step phenotypes: heterotrophy raised by this many mutation
/// magnitudes. `0` is the baseline — the resident producer centroid itself —
/// so a step's rate is read against what an unmodified producer cohort does
/// in the same (declining) resident.
const STEP_MULTIPLES: [u32; 4] = [0, 1, 2, 4];
const DEFAULT_T_INJ: u64 = 1000;
const DEFAULT_WINDOW: u64 = 1000;
const DEFAULT_REFERENCE: &str = "sample:31";

/// What is injected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phenotype {
    /// The founder heterotroph centroid of the initial distribution.
    Full,
    /// The resident producer centroid with heterotrophy raised by
    /// `k × mutation_magnitude`.
    Step(u32),
}

/// Where it is injected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum Placement {
    Uniform,
    Pile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
struct Arm {
    phenotype: Phenotype,
    placement: Placement,
}

impl Arm {
    /// A distinct tag per arm for the cohort's position stream.
    fn tag(self) -> u64 {
        let p = match self.phenotype {
            Phenotype::Full => 0,
            Phenotype::Step(k) => k as u64,
        };
        let q = match self.placement {
            Placement::Uniform => 0,
            Placement::Pile => 1,
        };
        p * 2 + q + 1
    }
}

/// Every arm: `full` and each small step, each at both placements.
fn all_arms() -> Vec<Arm> {
    let phenotypes = std::iter::once(Phenotype::Full).chain(STEP_MULTIPLES.map(Phenotype::Step));
    phenotypes
        .flat_map(|phenotype| {
            [Placement::Uniform, Placement::Pile].map(|placement| Arm {
                phenotype,
                placement,
            })
        })
        .collect()
}

/// The arms the accounting mode (#591) books: the unmodified producer
/// (`step 0`), one mutation step toward heterotrophy, and the full
/// heterotroph, each at both placements.
fn accounting_arms() -> Vec<Arm> {
    [Phenotype::Step(0), Phenotype::Step(1), Phenotype::Full]
        .into_iter()
        .flat_map(|phenotype| {
            [Placement::Uniform, Placement::Pile].map(|placement| Arm {
                phenotype,
                placement,
            })
        })
        .collect()
}

/// Whether an arm's window is stepped plainly (`run_window`) or under the
/// energy accountant (#591).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Plain,
    Accounting,
}

/// `run_window` under a [`LineageAccountant`]: the same lineage read (series,
/// extinction, the lineage's diet and deaths), plus its energy account.
fn run_accounted_window(
    world: &mut World,
    lineage: Lineage,
    window: u64,
    max_population: usize,
) -> (WindowOutcome, EnergyAccount) {
    let mut acct = LineageAccountant::new(world, lineage);
    let cohort = acct.alive(world);
    let mut series = vec![cohort];
    let (mut alive, mut extinct_tick, mut ticks_run, mut stopped) = (cohort, None, 0, None);
    for t in 1..=window {
        acct.step(world);
        ticks_run = t;
        alive = acct.alive(world);
        if alive == 0 && extinct_tick.is_none() {
            extinct_tick = Some(t);
        }
        if t.is_multiple_of(SERIES_INTERVAL) {
            series.push(alive);
        }
        if world.agents().is_empty() {
            stopped = Some("extinct");
            break;
        }
        if world.agents().len() > max_population {
            stopped = Some("explosion");
            break;
        }
    }
    if !ticks_run.is_multiple_of(SERIES_INTERVAL) {
        series.push(alive);
    }
    let account = *acct.account();
    let outcome = WindowOutcome {
        ticks_run,
        stopped,
        series,
        alive,
        extinct_tick,
        lineage: Some(acct.lineage().clone()),
    };
    (outcome, account)
}

/// #589's break-even count: carcasses in reach per tick the phenotype needs
/// for the carcass ceiling `h · u_H · e` to cover base and heterotrophy
/// maintenance, `k* = (base + h^x · c_H) / (h · u_H · e)`.
fn break_even_count(traits: &TraitVector, e: f32, p: &WorldParameters) -> f32 {
    let maintenance = p.base_metabolic_rate
        + traits.heterotrophy.powf(p.maintenance_cost_exponent) * p.heterotrophy_maintenance_cost;
    maintenance
        / (traits.heterotrophy * explorers_sim::units::HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK * e)
}

/// The founder heterotroph centroid `World::new` seeds for this
/// distribution: the odd cluster, all of the mean's trophic budget
/// (`photosynthetic_absorption + heterotrophy`) on heterotrophy, every other
/// trait at the mean.
fn founder_heterotroph_centroid(dist: &InitialDistribution) -> TraitVector {
    let mean = dist.mean_traits;
    TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy: mean.photosynthetic_absorption + mean.heterotrophy,
        ..mean
    }
}

/// A producer phenotype with heterotrophy raised by `k` mutation magnitudes.
fn small_step(producer: TraitVector, k: u32, mutation_magnitude: f32) -> TraitVector {
    TraitVector {
        heterotrophy: producer.heterotrophy + k as f32 * mutation_magnitude,
        ..producer
    }
}

/// Carcass nutrient per nutrient-grid cell, ranked highest first (ties by
/// cell index). Every cell appears.
fn carcass_nutrient_by_cell(world: &World) -> Vec<(usize, f32)> {
    let p = world.params();
    let cols = (p.world_extent / p.nutrient_grid_cell_size).ceil() as usize;
    let mut cells = vec![0.0_f32; cols * cols];
    for c in world.carcasses() {
        cells[world.nutrient_grid().cell_index_for(c.position)] += c.nutrient;
    }
    let mut ranked: Vec<(usize, f32)> = cells.into_iter().enumerate().collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked
}

/// Cohort positions for a placement, from a stream keyed on (seed, arm).
fn cohort_positions(
    world: &World,
    placement: Placement,
    n: usize,
    seed: u64,
    tag: u64,
) -> Vec<(f32, f32)> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_mul(7919) ^ (tag << 8));
    let p = world.params();
    let extent = p.world_extent;
    let half = extent / 2.0;
    match placement {
        Placement::Uniform => (0..n)
            .map(|_| (rng.random_range(-half..half), rng.random_range(-half..half)))
            .collect(),
        Placement::Pile => {
            let cs = p.nutrient_grid_cell_size;
            let cols = (extent / cs).ceil() as usize;
            let top: Vec<usize> = carcass_nutrient_by_cell(world)
                .into_iter()
                .take(PILE_CELLS)
                .map(|(i, _)| i)
                .collect();
            (0..n)
                .map(|i| {
                    let cell = top[i % top.len()];
                    let (col, row) = (cell % cols, cell / cols);
                    let span = |k: usize| {
                        let lo = -half + k as f32 * cs;
                        (lo, (lo + cs).min(half))
                    };
                    let (x0, x1) = span(col);
                    let (y0, y1) = span(row);
                    (rng.random_range(x0..x1), rng.random_range(y0..y1))
                })
                .collect()
        }
    }
}

/// The world's standing stocks at a moment.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
struct Stocks {
    population: usize,
    /// Agents with heterotrophy > photosynthetic absorption (the probe's
    /// nominal heterotroph).
    nominal_heterotrophs: usize,
    max_heterotrophy: f32,
    carcasses: usize,
    carcass_nutrient: f32,
    /// Available (free) nutrient on the grid.
    pool_nutrient: f32,
}

fn stocks(world: &World) -> Stocks {
    let agents = world.agents();
    Stocks {
        population: agents.len(),
        nominal_heterotrophs: agents
            .iter()
            .filter(|a| a.traits.heterotrophy > a.traits.photosynthetic_absorption)
            .count(),
        max_heterotrophy: agents
            .iter()
            .map(|a| a.traits.heterotrophy)
            .fold(0.0, f32::max),
        carcasses: world.carcasses().len(),
        carcass_nutrient: world.carcasses().iter().map(|c| c.nutrient).sum(),
        pool_nutrient: world.nutrient_pool(),
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Resident {
    /// `alive` (reached the injection tick), `extinct`, or `explosion`.
    termination: String,
    tick: u64,
    stocks: Stocks,
    producers: usize,
    /// Mean trait vector of the resident producers (`None` if none).
    producer_centroid: Option<TraitVector>,
    /// The `PILE_CELLS` top carcass-nutrient cells and their carcass N.
    pile_cells: Vec<(usize, f32)>,
    /// Share of all carcass nutrient in those cells.
    pile_share: f32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Control {
    ticks_run: u64,
    stopped: Option<String>,
    end: Stocks,
    /// The resident's consumers — agents in the consumer or decomposer role
    /// at the injection tick — and, in the accounting mode, their energy
    /// account (with descendants) over the window: the contrast case (#591).
    #[serde(default)]
    consumers_at_injection: usize,
    #[serde(default)]
    consumer_account: Option<EnergyAccount>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Injection {
    arm: Arm,
    traits: TraitVector,
    cohort: usize,
    ticks_run: u64,
    stopped: Option<String>,
    /// Live lineage size every `SERIES_INTERVAL` ticks (first entry the
    /// cohort), plus the terminal tick.
    lineage_series: Vec<usize>,
    lineage_end: usize,
    extinct_tick: Option<u64>,
    growth_rate: f64,
    /// The economics read: `trophic_transfer_efficiency(traits, resident
    /// producer centroid)` — the fraction of a drained producer carcass's
    /// energy the phenotype keeps — and the most carcass energy it can gain
    /// per tick with a carcass in reach, `heterotrophy × u_H × e` (the
    /// binary-reach drain, #380/#459). `None` without a resident producer.
    trophic_efficiency_on_resident: Option<f32>,
    carcass_intake_ceiling: Option<f32>,
    pure_births: usize,
    cross_births: usize,
    /// `Consumed` events by lineage members, living / carcass.
    lineage_consumed_events: ConsumedCounts,
    /// Energy the lineage drained (before the trophic efficiency), living /
    /// carcass.
    lineage_drained: DrainedEnergy,
    /// Mortality attribution, as `invasion_growth` (#493 §6): `Consumed`
    /// events on a living member, and member deaths in a tick the member was
    /// drained alive vs. undrained (starvation / structure threshold).
    lineage_preyed_upon_events: usize,
    lineage_deaths_drained: usize,
    lineage_deaths_undrained: usize,
    end: Stocks,
    /// Carcass N at the window end, control − arm: nutrient the arm took
    /// off the pile relative to the uninjected world.
    carcass_drawdown: f32,
    /// Available pool N at the window end, arm − control.
    pool_gain: f32,
    /// #589's break-even carcass count on the resident producer (`None`
    /// without one, or in the plain mode).
    #[serde(default)]
    k_star: Option<f32>,
    /// The lineage's realised energy account over the window (#591's
    /// accounting mode; `None` in the plain mode).
    #[serde(default)]
    account: Option<EnergyAccount>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SeedRecord {
    seed: u64,
    resident: Resident,
    control: Option<Control>,
    injections: Vec<Injection>,
    /// Arms not run on this seed and why (`no_producer` for a small step
    /// when the resident has no producer).
    skipped: Vec<(Arm, String)>,
}

/// Mean trait vector of the agents tagged `Producer`.
fn producer_centroid(agents: &[Agent], roles: &HashMap<u64, TrophicRole>) -> Option<TraitVector> {
    let members: Vec<&Agent> = agents
        .iter()
        .filter(|a| roles.get(&a.id) == Some(&TrophicRole::Producer))
        .collect();
    let first = members.first()?;
    let mut mean = first.traits;
    for dim in 0..TraitVector::NUM_DIMS {
        let sum: f64 = members.iter().map(|a| a.traits.get(dim) as f64).sum();
        mean.set(dim, (sum / members.len() as f64) as f32);
    }
    Some(mean)
}

/// One seed: the resident to `t_inj`, then the control and each arm as forks
/// of that one resident.
#[cfg(test)]
fn run_seed(
    config: &(WorldParameters, InitialDistribution),
    seed: u64,
    t_inj: u64,
    window: u64,
    arms: &[Arm],
) -> SeedRecord {
    run_seed_with(config, seed, t_inj, window, arms, Mode::Plain)
}

/// [`run_seed`], with each arm's window stepped in the given mode.
fn run_seed_with(
    (params, dist): &(WorldParameters, InitialDistribution),
    seed: u64,
    t_inj: u64,
    window: u64,
    arms: &[Arm],
    mode: Mode,
) -> SeedRecord {
    let max_population = EvalConfig::default().max_population;
    let mut world = World::new(params.clone(), dist.clone(), seed);
    // Only what the projection's roles and the lineage read (observer-side).
    world.retain_event_kinds(&[
        EventKind::Consumed,
        EventKind::Reproduced,
        EventKind::Died,
        EventKind::Born,
    ]);
    let mut topo = TopologyProjection::new();
    let mut termination = "alive";
    for _ in 0..t_inj {
        world.step();
        if world.agents().is_empty() {
            termination = "extinct";
            break;
        }
        if world.agents().len() > max_population {
            termination = "explosion";
            break;
        }
    }
    topo.update(world.event_log());
    let roles: HashMap<u64, TrophicRole> = topo.trophic_roles(world.agents()).into_iter().collect();
    let producers = roles
        .values()
        .filter(|r| **r == TrophicRole::Producer)
        .count();
    let producer_centroid = producer_centroid(world.agents(), &roles);
    let ranked = carcass_nutrient_by_cell(&world);
    let total_carcass: f32 = ranked.iter().map(|(_, n)| n).sum();
    let pile_cells: Vec<(usize, f32)> = ranked.into_iter().take(PILE_CELLS).collect();
    let pile_n: f32 = pile_cells.iter().map(|(_, n)| n).sum();
    let resident = Resident {
        termination: termination.to_string(),
        tick: world.tick(),
        stocks: stocks(&world),
        producers,
        producer_centroid,
        pile_cells,
        pile_share: if total_carcass > 0.0 {
            pile_n / total_carcass
        } else {
            0.0
        },
    };
    if termination != "alive" {
        return SeedRecord {
            seed,
            resident,
            control: None,
            injections: Vec::new(),
            skipped: arms.iter().map(|a| (*a, termination.to_string())).collect(),
        };
    }
    // The history has been read; forks clone a near-empty log.
    world.compact_event_log_before(world.event_log().len());

    let resident_consumers: Vec<u64> = roles
        .iter()
        .filter(|(_, r)| **r != TrophicRole::Producer)
        .map(|(id, _)| *id)
        .collect();
    let control = {
        let mut fork = world.clone();
        let (out, consumer_account) = match mode {
            Mode::Plain => {
                let mut fork_topo = topo.clone();
                let out = run_window(&mut fork, &mut fork_topo, None, window, max_population);
                (out, None)
            }
            Mode::Accounting => {
                let lineage = Lineage::new(resident_consumers.iter().copied());
                let (out, account) =
                    run_accounted_window(&mut fork, lineage, window, max_population);
                (out, Some(account))
            }
        };
        Control {
            ticks_run: out.ticks_run,
            stopped: out.stopped.map(str::to_string),
            end: stocks(&fork),
            consumers_at_injection: resident_consumers.len(),
            consumer_account,
        }
    };

    let mut injections = Vec::with_capacity(arms.len());
    let mut skipped = Vec::new();
    for &arm in arms {
        let traits = match arm.phenotype {
            Phenotype::Full => founder_heterotroph_centroid(dist),
            Phenotype::Step(k) => match producer_centroid {
                Some(p) => small_step(p, k, params.mutation_magnitude),
                None => {
                    skipped.push((arm, "no_producer".to_string()));
                    continue;
                }
            },
        };
        let efficiency = producer_centroid
            .map(|p| explorers_sim::trophic_transfer_efficiency(&traits, &p, params));
        let mut fork = world.clone();
        let mut fork_topo = topo.clone();
        let positions = cohort_positions(&fork, arm.placement, INVADER_COHORT, seed, arm.tag());
        let ids = place_cohort(&mut fork, traits, &positions, dist.initial_energy_per_agent);
        let (out, account) = match mode {
            Mode::Plain => {
                let out = run_window(
                    &mut fork,
                    &mut fork_topo,
                    Some(Lineage::new(ids)),
                    window,
                    max_population,
                );
                (out, None)
            }
            Mode::Accounting => {
                let (out, account) =
                    run_accounted_window(&mut fork, Lineage::new(ids), window, max_population);
                (out, Some(account))
            }
        };
        let lineage = out.lineage.expect("lineage arm");
        let end = stocks(&fork);
        injections.push(Injection {
            arm,
            traits,
            cohort: INVADER_COHORT,
            ticks_run: out.ticks_run,
            stopped: out.stopped.map(str::to_string),
            lineage_series: out.series,
            lineage_end: out.alive,
            extinct_tick: out.extinct_tick,
            growth_rate: growth_rate(INVADER_COHORT, out.alive, out.ticks_run),
            trophic_efficiency_on_resident: efficiency,
            carcass_intake_ceiling: efficiency.map(|e| {
                traits.heterotrophy
                    * explorers_sim::units::HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK
                    * e
            }),
            pure_births: lineage.pure_births,
            cross_births: lineage.cross_births,
            lineage_consumed_events: lineage.consumed,
            lineage_drained: lineage.drained,
            lineage_preyed_upon_events: lineage.preyed_upon_events,
            lineage_deaths_drained: lineage.deaths_drained,
            lineage_deaths_undrained: lineage.deaths_undrained,
            end,
            carcass_drawdown: control.end.carcass_nutrient - end.carcass_nutrient,
            pool_gain: end.pool_nutrient - control.end.pool_nutrient,
            k_star: match mode {
                Mode::Plain => None,
                Mode::Accounting => efficiency.map(|e| break_even_count(&traits, e, params)),
            },
            account,
        });
    }
    SeedRecord {
        seed,
        resident,
        control: Some(control),
        injections,
        skipped,
    }
}

/// Resolve a `config_source` key (`sample:31`, `sample@S:i`, `atlas:N`) to
/// its world. `atlas:` keys need the atlas file.
fn resolve_reference(
    reference: &str,
    atlas: Option<&std::path::Path>,
) -> Result<(WorldParameters, InitialDistribution), String> {
    let (source, index) = parse_config_key(reference)?;
    let sampled = sampled_units(default_ranges().len());
    let atlas_units = match (source.is_sample(), atlas) {
        (true, _) => Default::default(),
        (false, Some(path)) => read_atlas_units(path),
        (false, None) => return Err(format!("{reference}: an atlas key needs --atlas PATH")),
    };
    if source.is_sample() && index >= sampled.len() {
        return Err(format!("{reference}: index out of range"));
    }
    Ok(resolve_config(source, index, &atlas_units, &sampled))
}

// ---------------------------------------------------------------------------
// Seed-ensemble summary, artifact, CLI
// ---------------------------------------------------------------------------

/// One arm over the seeds that reached the injection tick.
#[derive(Clone, Debug, serde::Serialize)]
struct ArmSummary {
    arm: Arm,
    /// Median `r`, the sign test's interval, and the verdict
    /// (`invades` = median > 0 and interval clear of zero).
    rates: RateSummary,
    median_lineage_end: f64,
    max_lineage_end: usize,
    /// Seeds on which the lineage died out inside the window.
    seeds_extinct: usize,
    /// Median tick (from injection) at which the lineage died out, over the
    /// seeds on which it did.
    median_extinct_tick: f64,
    births_pure: usize,
    births_cross: usize,
    /// Lineage `Consumed` events summed over seeds, living / carcass.
    diet: ConsumedCounts,
    /// Lineage deaths summed over seeds: drained alive that tick (preyed
    /// upon) / undrained (starvation or structure threshold).
    deaths_drained: usize,
    deaths_undrained: usize,
    /// Median over seeds of the arm's transfer efficiency on the resident
    /// producer and its carcass-intake ceiling (E / tick).
    median_trophic_efficiency: f64,
    median_intake_ceiling: f64,
    /// Carcass energy the lineage drained, summed over seeds, and the part
    /// it kept (drain × the seed's efficiency on the resident producer).
    carcass_drained: f64,
    carcass_gained: f64,
    /// Median over seeds of carcass N (control − arm) and pool N (arm −
    /// control) at the window end.
    median_carcass_drawdown: f64,
    median_pool_gain: f64,
}

fn median_of(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut v: Vec<f64> = values.into_iter().collect();
    v.sort_by(f64::total_cmp);
    median(&v)
}

fn injections_of(seeds: &[SeedRecord], arm: Arm) -> Vec<&Injection> {
    seeds
        .iter()
        .flat_map(|s| s.injections.iter().filter(move |i| i.arm == arm))
        .collect()
}

/// Every seed's rate on one arm, in seed order.
fn row_rates(seeds: &[SeedRecord], arm: Arm) -> Vec<f64> {
    injections_of(seeds, arm)
        .iter()
        .map(|i| i.growth_rate)
        .collect()
}

fn summarise_arm(seeds: &[SeedRecord], arm: Arm) -> ArmSummary {
    let inj = injections_of(seeds, arm);
    let extinct: Vec<f64> = inj
        .iter()
        .filter_map(|i| i.extinct_tick.map(|t| t as f64))
        .collect();
    ArmSummary {
        arm,
        rates: summarise_rates(&row_rates(seeds, arm)),
        median_lineage_end: median_of(inj.iter().map(|i| i.lineage_end as f64)),
        max_lineage_end: inj.iter().map(|i| i.lineage_end).max().unwrap_or(0),
        seeds_extinct: extinct.len(),
        median_extinct_tick: median_of(extinct),
        births_pure: inj.iter().map(|i| i.pure_births).sum(),
        births_cross: inj.iter().map(|i| i.cross_births).sum(),
        diet: ConsumedCounts {
            living: inj.iter().map(|i| i.lineage_consumed_events.living).sum(),
            carcass: inj.iter().map(|i| i.lineage_consumed_events.carcass).sum(),
        },
        deaths_drained: inj.iter().map(|i| i.lineage_deaths_drained).sum(),
        deaths_undrained: inj.iter().map(|i| i.lineage_deaths_undrained).sum(),
        median_trophic_efficiency: median_of(
            inj.iter()
                .filter_map(|i| i.trophic_efficiency_on_resident.map(f64::from)),
        ),
        median_intake_ceiling: median_of(
            inj.iter()
                .filter_map(|i| i.carcass_intake_ceiling.map(f64::from)),
        ),
        carcass_drained: inj.iter().map(|i| i.lineage_drained.carcass).sum(),
        carcass_gained: inj
            .iter()
            .map(|i| {
                i.lineage_drained.carcass * i.trophic_efficiency_on_resident.map_or(0.0, f64::from)
            })
            .sum(),
        median_carcass_drawdown: median_of(inj.iter().map(|i| i.carcass_drawdown as f64)),
        median_pool_gain: median_of(inj.iter().map(|i| i.pool_gain as f64)),
    }
}

/// A run's (or merged chunks') records and their per-arm summary.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Artifact {
    reference: String,
    t_inj: u64,
    window: u64,
    cohort: usize,
    arms: Vec<Arm>,
    /// Whether the arms ran under the energy accountant (#591).
    #[serde(default)]
    accounting: bool,
    seeds: Vec<SeedRecord>,
    /// Derived from `seeds`: not read back from a file (an empty arm's
    /// medians are NaN, written as `null`); `merge` recomputes it.
    #[serde(skip_deserializing, default)]
    summary: Vec<ArmSummary>,
}

impl Artifact {
    /// Run seeds `SEED_BASE + seed_from ..` (`n` of them) in parallel; the
    /// indexed collect keeps seed order, so the artifact is deterministic.
    #[allow(clippy::too_many_arguments)]
    fn run(
        reference: &str,
        config: &(WorldParameters, InitialDistribution),
        t_inj: u64,
        window: u64,
        seed_from: u64,
        n: u64,
        arms: &[Arm],
        mode: Mode,
    ) -> Artifact {
        let seeds: Vec<SeedRecord> = (seed_from..seed_from + n)
            .into_par_iter()
            .map(|s| run_seed_with(config, SEED_BASE + s, t_inj, window, arms, mode))
            .collect();
        let mut a =
            Artifact::from_seeds(reference.to_string(), t_inj, window, arms.to_vec(), seeds);
        a.accounting = mode == Mode::Accounting;
        a
    }

    fn from_seeds(
        reference: String,
        t_inj: u64,
        window: u64,
        arms: Vec<Arm>,
        seeds: Vec<SeedRecord>,
    ) -> Artifact {
        let summary = arms.iter().map(|&a| summarise_arm(&seeds, a)).collect();
        Artifact {
            reference,
            t_inj,
            window,
            cohort: INVADER_COHORT,
            arms,
            accounting: false,
            seeds,
            summary,
        }
    }

    /// Combine chunk artifacts of one run (same reference, horizon and arms;
    /// disjoint seeds) into the whole run's artifact, seeds in order.
    fn merge(chunks: Vec<Artifact>) -> Result<Artifact, String> {
        let first = chunks.first().ok_or("nothing to merge")?;
        let key = |a: &Artifact| {
            (
                a.reference.clone(),
                a.t_inj,
                a.window,
                a.arms.clone(),
                a.accounting,
            )
        };
        let want = key(first);
        if let Some(bad) = chunks.iter().find(|a| key(a) != want) {
            return Err(format!(
                "chunk {:?} t_inj {} window {} does not match {:?} t_inj {} window {}",
                bad.reference, bad.t_inj, bad.window, want.0, want.1, want.2
            ));
        }
        let (reference, t_inj, window, arms, accounting) = want;
        let mut seeds: Vec<SeedRecord> = chunks.into_iter().flat_map(|a| a.seeds).collect();
        seeds.sort_by_key(|s| s.seed);
        if seeds.windows(2).any(|w| w[0].seed == w[1].seed) {
            return Err("chunks share a seed".to_string());
        }
        let mut a = Artifact::from_seeds(reference, t_inj, window, arms, seeds);
        a.accounting = accounting;
        Ok(a)
    }
}

const USAGE: &str = "usage: reinvasion_barrier [CONFIG_KEY (default sample:31)] [--accounting] [--t-inj N] [--window N] [--seed-from A] [--seeds N] [--atlas PATH] [--out PATH] | --merge FILE...";

#[derive(Debug)]
struct Cli {
    reference: String,
    t_inj: u64,
    window: u64,
    seed_from: u64,
    seeds: u64,
    atlas: Option<std::path::PathBuf>,
    out: String,
    merge: Vec<String>,
    /// Run the accounting mode (#591): the named arms under the energy
    /// accountant, written to `target/carcass-income-accounting.json`.
    accounting: bool,
}

impl Cli {
    fn parse<I: IntoIterator<Item = String>>(argv: I) -> Result<Cli, String> {
        let mut cli = Cli {
            reference: DEFAULT_REFERENCE.to_string(),
            t_inj: DEFAULT_T_INJ,
            window: DEFAULT_WINDOW,
            seed_from: 0,
            seeds: N_SEEDS,
            atlas: None,
            out: String::new(),
            merge: Vec::new(),
            accounting: false,
        };
        let mut it = argv.into_iter();
        let mut merging = false;
        while let Some(arg) = it.next() {
            let mut value = |flag: &str| it.next().ok_or_else(|| format!("{flag} needs a value"));
            let mut number = |flag: &str| -> Result<u64, String> {
                let raw = value(flag)?;
                raw.parse()
                    .map_err(|_| format!("{flag} {raw:?} is not an integer"))
            };
            match arg.as_str() {
                "--t-inj" => cli.t_inj = number("--t-inj")?,
                "--window" => cli.window = number("--window")?,
                "--seed-from" => cli.seed_from = number("--seed-from")?,
                "--seeds" => cli.seeds = number("--seeds")?,
                "--atlas" => cli.atlas = Some(value("--atlas")?.into()),
                "--out" => cli.out = value("--out")?,
                "--merge" => merging = true,
                "--accounting" => cli.accounting = true,
                flag if flag.starts_with("--") => {
                    return Err(format!("unknown argument {flag:?}\n{USAGE}"));
                }
                file if merging => cli.merge.push(file.to_string()),
                key => cli.reference = key.to_string(),
            }
        }
        if cli.out.is_empty() {
            cli.out = if cli.accounting {
                "target/carcass-income-accounting.json"
            } else {
                "target/reinvasion-barrier.json"
            }
            .to_string();
        }
        if cli.t_inj == 0 || cli.window == 0 {
            return Err("--t-inj and --window must be positive".to_string());
        }
        if cli.seeds == 0 || cli.seed_from + cli.seeds > N_SEEDS {
            return Err(format!("seeds must lie in the block 0..{N_SEEDS}"));
        }
        if merging && cli.merge.is_empty() {
            return Err(format!("--merge needs files\n{USAGE}"));
        }
        Ok(cli)
    }
}

fn read_artifact(path: &str) -> Result<Artifact, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse {path}: {e}"))
}

fn main() {
    let cli = Cli::parse(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    let artifact = if cli.merge.is_empty() {
        let config = resolve_reference(&cli.reference, cli.atlas.as_deref()).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2)
        });
        let (arms, mode) = if cli.accounting {
            (accounting_arms(), Mode::Accounting)
        } else {
            (all_arms(), Mode::Plain)
        };
        eprintln!(
            "reinvasion_barrier: {} × {} arms × seeds {}..{}; t_inj {}, window {}, cohort {INVADER_COHORT}",
            cli.reference,
            arms.len(),
            cli.seed_from,
            cli.seed_from + cli.seeds,
            cli.t_inj,
            cli.window
        );
        let start = std::time::Instant::now();
        let a = Artifact::run(
            &cli.reference,
            &config,
            cli.t_inj,
            cli.window,
            cli.seed_from,
            cli.seeds,
            &arms,
            mode,
        );
        eprintln!(
            "reinvasion_barrier: done in {:.0}s",
            start.elapsed().as_secs_f64()
        );
        a
    } else {
        let chunks: Vec<Artifact> = cli
            .merge
            .iter()
            .map(|p| read_artifact(p))
            .collect::<Result<_, _>>()
            .and_then(Artifact::merge)
            .map(|a| vec![a])
            .unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(2)
            });
        chunks.into_iter().next().expect("merged")
    };
    print_summary(&artifact);
    if let Some(dir) = std::path::Path::new(&cli.out).parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let json = serde_json::to_string_pretty(&artifact).expect("serialise artifact");
    std::fs::write(&cli.out, json).unwrap_or_else(|e| panic!("write {}: {e}", cli.out));
    eprintln!(
        "reinvasion_barrier: wrote {} ({} seeds)",
        cli.out,
        artifact.seeds.len()
    );
}

fn arm_label(arm: Arm) -> String {
    let p = match arm.phenotype {
        Phenotype::Full => "full".to_string(),
        Phenotype::Step(0) => "resident (step 0)".to_string(),
        Phenotype::Step(k) => format!("step {k}x"),
    };
    let q = match arm.placement {
        Placement::Uniform => "uniform",
        Placement::Pile => "pile",
    };
    format!("{p} / {q}")
}

fn print_summary(a: &Artifact) {
    let reached: Vec<&SeedRecord> = a
        .seeds
        .iter()
        .filter(|s| s.resident.termination == "alive")
        .collect();
    println!("\n# Re-invasion barrier (issue #587)");
    println!(
        "# {}: seeds {:?}, t_inj {}, window {}, cohort {}; {} of {} seeds reached t_inj",
        a.reference,
        a.seeds.iter().map(|s| s.seed).collect::<Vec<_>>(),
        a.t_inj,
        a.window,
        a.cohort,
        reached.len(),
        a.seeds.len()
    );
    let med = |f: &dyn Fn(&SeedRecord) -> f64| median_of(reached.iter().map(|s| f(s)));
    println!(
        "resident at t_inj (median): population {:.0}, producers {:.0}, nominal heterotrophs {:.0}, max heterotrophy {:.3}, carcass N {:.0}, pool N {:.0}, pile share {:.2}",
        med(&|s| s.resident.stocks.population as f64),
        med(&|s| s.resident.producers as f64),
        med(&|s| s.resident.stocks.nominal_heterotrophs as f64),
        med(&|s| s.resident.stocks.max_heterotrophy as f64),
        med(&|s| s.resident.stocks.carcass_nutrient as f64),
        med(&|s| s.resident.stocks.pool_nutrient as f64),
        med(&|s| s.resident.pile_share as f64),
    );
    let ctl = |f: &dyn Fn(&Control) -> f64| {
        median_of(
            reached
                .iter()
                .filter_map(|s| s.control.as_ref())
                .map(|c| f(c)),
        )
    };
    println!(
        "control at window end (median): population {:.0}, nominal heterotrophs {:.0}, carcass N {:.0}, pool N {:.0}",
        ctl(&|c| c.end.population as f64),
        ctl(&|c| c.end.nominal_heterotrophs as f64),
        ctl(&|c| c.end.carcass_nutrient as f64),
        ctl(&|c| c.end.pool_nutrient as f64),
    );
    // The resident's own per-capita rate over the window (control fork):
    // the drift every injected lineage is read against.
    println!(
        "resident per-capita rate over the window (control, median): {:.5}",
        median_of(reached.iter().filter_map(|s| {
            s.control
                .as_ref()
                .map(|c| growth_rate(s.resident.stocks.population, c.end.population, c.ticks_run))
        }))
    );
    println!(
        "\n| arm | n | median r | interval | r > 0 | invades | lineage end (med / max) | extinct (med tick) | births pure / cross | diet living / carcass | deaths drained / undrained | carcass drawdown | pool gain |"
    );
    println!("|---|---:|---:|---|---:|---|---:|---:|---:|---:|---:|---:|---:|");
    let mut economics = String::from(
        "\n| arm | e on resident producer | carcass intake ceiling (E/tick) | carcass E drained | carcass E kept |\n|---|---:|---:|---:|---:|",
    );
    for s in &a.summary {
        let r = &s.rates;
        let interval = r
            .interval
            .map_or("–".to_string(), |(lo, hi)| format!("[{lo:.5}, {hi:.5}]"));
        println!(
            "| {} | {} | {:.5} | {} | {}/{} | {} | {:.1} / {} | {} ({:.0}) | {} / {} | {} / {} | {} / {} | {:.1} | {:.1} |",
            arm_label(s.arm),
            r.n,
            r.median,
            interval,
            r.positive_seeds,
            r.n,
            if r.invades { "yes" } else { "no" },
            s.median_lineage_end,
            s.max_lineage_end,
            s.seeds_extinct,
            s.median_extinct_tick,
            s.births_pure,
            s.births_cross,
            s.diet.living,
            s.diet.carcass,
            s.deaths_drained,
            s.deaths_undrained,
            s.median_carcass_drawdown,
            s.median_pool_gain,
        );
        economics += &format!(
            "\n| {} | {:.4} | {:.4} | {:.1} | {:.1} |",
            arm_label(s.arm),
            s.median_trophic_efficiency,
            s.median_intake_ceiling,
            s.carcass_drained,
            s.carcass_gained
        );
    }
    println!("{economics}");
    if a.accounting {
        print_accounting(a);
    }
}

/// The accounting mode's tables (#591): per arm — and for the resident's own
/// consumers on the control fork, the contrast case — the median over seeds
/// of each term per member-tick (E / tick unless a count). Seeds on which a
/// row has no member-tick are left out of its medians.
fn print_accounting(a: &Artifact) {
    struct Row {
        label: String,
        accounts: Vec<EnergyAccount>,
        k_star: f64,
    }
    let mut rows: Vec<Row> = a
        .arms
        .iter()
        .map(|&arm| {
            let inj = injections_of(&a.seeds, arm);
            Row {
                label: arm_label(arm),
                accounts: inj.iter().filter_map(|i| i.account).collect(),
                k_star: median_of(inj.iter().filter_map(|i| i.k_star.map(f64::from))),
            }
        })
        .collect();
    rows.push(Row {
        label: "resident consumers (control)".to_string(),
        accounts: a
            .seeds
            .iter()
            .filter_map(|s| s.control.as_ref())
            .filter_map(|c| c.consumer_account)
            .collect(),
        k_star: f64::NAN,
    });
    for row in &mut rows {
        row.accounts.retain(|acc| acc.member_ticks > 0);
    }
    type Term = fn(&EnergyAccount) -> f64;
    let med = |row: &Row, f: Term| {
        median_of(
            row.accounts
                .iter()
                .map(|acc| f(acc) / acc.member_ticks as f64),
        )
    };
    let sum = |row: &Row, f: fn(&EnergyAccount) -> u64| row.accounts.iter().map(f).sum::<u64>();
    println!("\n# Energy accounting (issue #591): median over seeds of per-member-tick means");
    println!(
        "resident consumers at injection (control fork, median over seeds): {:.1}",
        median_of(
            a.seeds
                .iter()
                .filter_map(|s| s.control.as_ref())
                .map(|c| c.consumers_at_injection as f64)
        )
    );
    println!(
        "\n| row | seeds | member-ticks | k* | k reached | k with energy | carcasses in cell | P(k ≥ 1) | co-consumers per contact | carcass ceiling | wear gap | exhaustion gap | sharing gap | carcass gain | living gain | photosynthesis | photo ceiling (unshaded) |"
    );
    println!(
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    for r in &rows {
        let co = median_of(
            r.accounts
                .iter()
                .map(|x| x.carcass.co_consumers as f64 / x.carcass.reached.max(1) as f64),
        );
        println!(
            "| {} | {} | {:.0} | {:.2} | {:.3} | {:.3} | {:.1} | {:.3} | {:.2} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} |",
            r.label,
            r.accounts.len(),
            median_of(r.accounts.iter().map(|x| x.member_ticks as f64)),
            r.k_star,
            med(r, |x| x.carcass.reached as f64),
            med(r, |x| x.carcass.reached_with_energy as f64),
            med(r, |x| x.carcass.in_cell as f64),
            med(r, |x| (x.member_ticks - x.carcass.k_histogram[0]) as f64),
            co,
            med(r, |x| x.carcass.ceiling),
            med(r, |x| x.carcass.wear_gap),
            med(r, |x| x.carcass.exhaustion_gap),
            med(r, |x| x.carcass.sharing_gap),
            med(r, |x| x.carcass.gain),
            med(r, |x| x.living_gain),
            med(r, |x| x.photosynthesis),
            med(r, |x| x.photosynthesis_ceiling),
        );
    }
    println!(
        "\n| row | base | heterotrophy | #587 terms | photo | mobility | asexual | structure | maintenance total | unpaid | growth loss | repair | movement | upkeep | income | net (income − upkeep) | grazed | of it by kin |"
    );
    println!(
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    for r in &rows {
        println!(
            "| {} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} |",
            r.label,
            med(r, |x| x.maintenance.base),
            med(r, |x| x.maintenance.heterotrophy),
            med(r, |x| x.maintenance.margin_terms()),
            med(r, |x| x.maintenance.photosynthesis),
            med(r, |x| x.maintenance.mobility),
            med(r, |x| x.maintenance.asexual),
            med(r, |x| x.maintenance.structure),
            med(r, |x| x.maintenance.total()),
            med(r, |x| x.maintenance.unpaid),
            med(r, |x| x.growth_loss),
            med(r, |x| x.repair),
            med(r, |x| x.movement),
            med(r, |x| x.upkeep()),
            med(r, |x| x.income()),
            med(r, |x| x.income() - x.upkeep()),
            med(r, |x| x.grazed),
            med(r, |x| x.grazed_by_kin),
        );
    }
    println!(
        "\n| row | earmark fill | earmark level | energy gate met | nutrient earmark level | nutrient gate met | reproduction outlay | births (sum) | deaths (sum) | starved | grazed to death | infant (≤ 50 ticks) / grazed | death → carcass | death dissipated | stranded earmark | max residual / throughput | max ledger gap / throughput |"
    );
    println!(
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    let throughput = |x: &EnergyAccount| (x.income() + x.outgo() + x.births_endowment).max(1.0);
    for r in &rows {
        let max_residual = r
            .accounts
            .iter()
            .map(|x| x.residual_abs / throughput(x))
            .fold(0.0, f64::max);
        let max_ledger = r
            .accounts
            .iter()
            .map(|x| {
                let l = x.ledger;
                [
                    x.photosynthesis - l.photosynthesis,
                    x.carcass.gain - l.carcass_gain,
                    x.living_gain - l.living_gain,
                    x.outgo() - l.sent,
                    x.births_endowment - l.births_endowment,
                ]
                .into_iter()
                .map(f64::abs)
                .fold(0.0, f64::max)
                    / throughput(x)
            })
            .fold(0.0, f64::max);
        println!(
            "| {} | {:.4} | {:.2} | {:.3} | {:.3} | {:.3} | {:.4} | {} | {} | {} | {} | {} / {} | {:.4} | {:.4} | {:.4} | {:.1e} | {:.1e} |",
            r.label,
            med(r, |x| x.earmark_fill),
            med(r, |x| x.earmark_level),
            med(r, |x| x.energy_gate_met as f64),
            med(r, |x| x.nutrient_earmark_level),
            med(r, |x| x.nutrient_gate_met as f64),
            med(r, |x| x.reproduction_outlay),
            sum(r, |x| x.births),
            sum(r, |x| x.deaths),
            sum(r, |x| x.deaths_starved),
            sum(r, |x| x.deaths_grazed),
            sum(r, |x| x.infant_deaths),
            sum(r, |x| x.infant_deaths_grazed),
            med(r, |x| x.death_to_carcass),
            med(r, |x| x.death_dissipated),
            med(r, |x| x.stranded_earmark),
            max_residual,
            max_ledger,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke check on one seed of `sample:31` at a short horizon: the
    /// resident runs to the injection tick, the control and every
    /// (phenotype × placement) arm run as forks of it, each arm's lineage is
    /// the injected cohort at the start, and the record is deterministic. No
    /// assertion on emergent values.
    #[test]
    fn run_seed_forks_a_control_and_every_arm_and_is_deterministic() {
        let config = resolve_reference("sample:31", None).expect("sample:31 resolves");
        let arms = all_arms();
        let run = || run_seed(&config, SEED_BASE, 40, 20, &arms);
        let record = run();
        assert_eq!(record.resident.termination, "alive");
        assert_eq!(record.resident.tick, 40);
        let control = record.control.as_ref().expect("control ran");
        assert_eq!(control.ticks_run, 20);
        assert_eq!(record.injections.len(), arms.len());
        for (inj, arm) in record.injections.iter().zip(&arms) {
            assert_eq!(inj.arm, *arm);
            assert_eq!(inj.lineage_series.first().copied(), Some(INVADER_COHORT));
            assert!(inj.growth_rate.is_finite(), "{inj:?}");
            // The economics read: the arm's transfer efficiency on the
            // resident producer and its carcass-intake ceiling.
            let producer = record.resident.producer_centroid.expect("producers");
            let e = explorers_sim::trophic_transfer_efficiency(&inj.traits, &producer, &config.0);
            assert_eq!(inj.trophic_efficiency_on_resident, Some(e));
            assert_eq!(
                inj.carcass_intake_ceiling,
                Some(inj.traits.heterotrophy * e)
            );
        }
        // `step_0` is the baseline: the resident producer centroid itself.
        let baseline = record
            .injections
            .iter()
            .find(|i| i.arm.phenotype == Phenotype::Step(0))
            .expect("baseline arm");
        assert_eq!(Some(baseline.traits), record.resident.producer_centroid);
        let a = serde_json::to_string(&record).unwrap();
        let b = serde_json::to_string(&run()).unwrap();
        assert_eq!(a, b, "record is deterministic");
    }

    /// The `full` phenotype is the heterotroph cluster `World::new` seeds:
    /// with (near) zero trait noise, every odd-cluster founder sits on it.
    #[test]
    fn full_phenotype_is_the_founder_heterotroph_cluster() {
        let (params, mut dist) = resolve_reference("sample:31", None).unwrap();
        dist.trait_covariance = 1e-7;
        let centroid = founder_heterotroph_centroid(&dist);
        let world = World::new(params, dist.clone(), SEED_BASE);
        let founders: Vec<&Agent> = world
            .agents()
            .iter()
            .filter(|a| a.traits.heterotrophy > a.traits.photosynthetic_absorption)
            .collect();
        assert!(!founders.is_empty());
        for a in founders {
            for dim in 0..TraitVector::NUM_DIMS {
                assert!(
                    (a.traits.get(dim) - centroid.get(dim)).abs() < 1e-4,
                    "dim {dim}: founder {:?} vs centroid {centroid:?}",
                    a.traits
                );
            }
        }
    }

    /// A small step raises heterotrophy by `k` mutation magnitudes and
    /// leaves every other trait of the producer as it was.
    #[test]
    fn small_step_raises_only_heterotrophy_by_k_mutation_magnitudes() {
        let (_, dist) = resolve_reference("sample:31", None).unwrap();
        let producer = dist.mean_traits;
        let stepped = small_step(producer, 4, 0.25);
        assert!((stepped.heterotrophy - (producer.heterotrophy + 1.0)).abs() < 1e-6);
        for dim in (0..TraitVector::NUM_DIMS).filter(|&d| d != 1) {
            assert_eq!(stepped.get(dim), producer.get(dim));
        }
    }

    /// The `pile` placement puts the cohort in the `PILE_CELLS` cells with
    /// the most carcass nutrient — dealt round-robin from the top cell — and
    /// `uniform` spreads it over the whole world.
    #[test]
    fn pile_placement_lands_the_cohort_in_the_top_carcass_nutrient_cells() {
        let (params, dist) = resolve_reference("sample:31", None).unwrap();
        let carcass = |position: (f32, f32), nutrient: f32| explorers_sim::CarcassSpec {
            position,
            energy: 1.0,
            traits: dist.mean_traits,
            nutrient,
        };
        let recipe = explorers_sim::WorldRecipe {
            parameters: params,
            initial_distribution: None,
            agents: Some(vec![explorers_sim::AgentSpec {
                position: (0.0, 0.0),
                reserve: 5.0,
                traits: dist.mean_traits,
                nutrient: 0.0,
            }]),
            carcasses: Some(vec![
                carcass((-15.0, -15.0), 5.0),
                carcass((15.0, 15.0), 9.0),
                carcass((0.0, 0.0), 1.0),
                carcass((14.0, 16.0), 2.0),
            ]),
            max_ticks: 10,
        };
        let world = World::from_recipe(&recipe, SEED_BASE);
        let cell = |pos| world.nutrient_grid().cell_index_for(pos);
        let (top, second) = (cell((15.0, 15.0)), cell((-15.0, -15.0)));
        let ranked = carcass_nutrient_by_cell(&world);
        assert_eq!(ranked[0], (top, 11.0));
        assert_eq!(ranked[1], (second, 5.0));
        let pile = cohort_positions(&world, Placement::Pile, 8, SEED_BASE, 1);
        assert_eq!(pile.len(), 8);
        for (i, pos) in pile.iter().enumerate() {
            let expect = if i % 2 == 0 { top } else { second };
            assert_eq!(cell(*pos), expect, "agent {i} at {pos:?}");
        }
        let uniform = cohort_positions(&world, Placement::Uniform, 64, SEED_BASE, 1);
        let cells: std::collections::HashSet<usize> = uniform.iter().map(|p| cell(*p)).collect();
        assert!(cells.len() > PILE_CELLS, "uniform spreads over the world");
        assert_eq!(
            pile,
            cohort_positions(&world, Placement::Pile, 8, SEED_BASE, 1),
            "positions are a pure function of (seed, arm)"
        );
    }

    /// The summary reads one row per arm over the seeds that reached the
    /// injection tick, with the rate read as `invasion_growth` reads it; an
    /// artifact survives a write–merge round trip unchanged.
    #[test]
    fn summary_has_a_row_per_arm_and_merging_chunks_recovers_the_whole_run() {
        let config = resolve_reference("sample:31", None).unwrap();
        let arms = all_arms();
        let run = |from: u64, n: u64| {
            Artifact::run("sample:31", &config, 30, 10, from, n, &arms, Mode::Plain)
        };
        let whole = run(0, 2);
        assert_eq!(whole.summary.len(), arms.len());
        for row in &whole.summary {
            assert_eq!(row.rates.n, 2);
            assert_eq!(
                row.rates,
                summarise_rates(&row_rates(&whole.seeds, row.arm))
            );
        }
        // Chunks go through the file format, as `--merge` reads them.
        let via_json = |a: Artifact| -> Artifact {
            serde_json::from_str(&serde_json::to_string_pretty(&a).unwrap()).unwrap()
        };
        let merged = Artifact::merge(vec![via_json(run(1, 1)), via_json(run(0, 1))]).unwrap();
        assert_eq!(
            serde_json::to_string(&merged).unwrap(),
            serde_json::to_string(&whole).unwrap()
        );
        let other = Artifact::run("sample:31", &config, 31, 10, 2, 1, &arms, Mode::Plain);
        assert!(
            Artifact::merge(vec![whole, other]).is_err(),
            "t_inj differs"
        );
    }

    /// The accounting mode (#591) runs the named arms — `step 0`, one step and
    /// `full`, at both placements — and books each lineage's energy account;
    /// the record stays deterministic. Smoke only: the account's closure is
    /// pinned in `energy_accounting`.
    #[test]
    fn accounting_mode_books_an_energy_account_per_named_arm() {
        let config = resolve_reference("sample:31", None).expect("sample:31 resolves");
        let arms = accounting_arms();
        assert_eq!(arms.len(), 6);
        let run = || run_seed_with(&config, SEED_BASE, 40, 20, &arms, Mode::Accounting);
        let record = run();
        assert_eq!(record.injections.len(), arms.len());
        for inj in &record.injections {
            let account = inj.account.as_ref().expect("accounted");
            assert!(account.member_ticks >= INVADER_COHORT as u64, "{inj:?}");
            assert!(inj.k_star.is_some());
        }
        // The control books the resident's consumers (role consumer or
        // decomposer at the injection tick), the contrast case.
        let control = record.control.as_ref().expect("control");
        let consumers = control.consumer_account.as_ref().expect("consumers booked");
        assert!(consumers.member_ticks >= control.consumers_at_injection as u64);
        let a = serde_json::to_string(&record).unwrap();
        let b = serde_json::to_string(&run()).unwrap();
        assert_eq!(a, b, "record is deterministic");
        // The plain mode books nothing extra.
        let plain = run_seed(&config, SEED_BASE, 40, 20, &arms);
        assert!(plain.injections.iter().all(|i| i.account.is_none()));
    }

    #[test]
    fn cli_defaults_to_sample_31_at_tick_1000_and_takes_a_seed_chunk() {
        let parse = |args: &[&str]| Cli::parse(args.iter().map(|s| s.to_string()));
        let cli = parse(&[]).unwrap();
        assert_eq!(cli.reference, "sample:31");
        assert_eq!((cli.t_inj, cli.window), (1000, 1000));
        assert_eq!((cli.seed_from, cli.seeds), (0, N_SEEDS));
        let cli = parse(&[
            "sample@9421:3",
            "--t-inj",
            "600",
            "--seed-from",
            "4",
            "--seeds",
            "2",
        ])
        .unwrap();
        assert_eq!(cli.reference, "sample@9421:3");
        assert_eq!((cli.t_inj, cli.seed_from, cli.seeds), (600, 4, 2));
        assert!(parse(&["--seeds", "9"]).is_err(), "seed block is 8");
        assert!(parse(&["--bogus"]).is_err());
        assert!(!parse(&[]).unwrap().accounting);
        let cli = parse(&["--accounting"]).unwrap();
        assert!(cli.accounting);
        assert_eq!(cli.out, "target/carcass-income-accounting.json");
        let cli = parse(&["--merge", "a.json", "b.json"]).unwrap();
        assert_eq!(cli.merge, vec!["a.json".to_string(), "b.json".to_string()]);
    }
}
