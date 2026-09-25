//! Invasion-growth-rate instrument (issue #443, formal viability D1): the
//! mutual-invasibility coexistence oracle.
//!
//! ## What it measures
//!
//! Chesson's criterion: a set of species coexists when each can invade the
//! community formed by the others at their attractor — every species has a
//! positive per-capita growth rate *when rare*. This instrument reads that
//! off the simulation for every atlas live cell, per trophic role
//! (producer / consumer / decomposer as `topology::trophic_roles` classifies
//! them), and turns it into a per-cell verdict that can stand next to the
//! atlas's `coexistence_fraction`.
//!
//! Per (cell, seed): the resident web is `World::new` on the cell's decoded
//! config, stepped to `t_inj` (the search horizon, `SearchConfig::max_ticks` —
//! 2000 ticks since #507 — by default; `INVASION_GROWTH_T_INJ` overrides it).
//! At `t_inj` the
//! roles are classified and each role's **realised centroid** (mean trait
//! vector of the agents in that role) is read; a role absent from the
//! resident is **not testable** (`not_testable`, reason `role_absent`) and
//! gets no injection — Chesson's question is whether the role can re-invade
//! *this* web, and a phenotype the web never produced does not ask it
//! (#491; the canonical-vertex injections it replaces starved at a median
//! tick of 13). A role that is tagged but is not a **guild** over the
//! resident phase — the evaluator's #490 predicate (`genesis_eval::guild`)
//! read over `(t_inj/2, t_inj]` on roster samples every
//! `GUILD_SAMPLE_INTERVAL` ticks: role count ≥ `GUILD_MIN_SIZE` on every
//! sample plus ≥ 1 birth naming a member — is likewise not testable
//! (`no_guild`, #493): one sessile individual is not a community to ask
//! Chesson's question of (443 §4.1). The resident is then forked
//! (`World: Clone`) into one
//! control and, per testable role × arm, one injection: a cohort of
//! `INVADER_COHORT` agents at the centroid, spread uniformly over the world
//! and provisioned exactly as founders are. Two
//! arms: `intact` injects into the resident as it stands (the issue's literal
//! protocol; the injected role's own residents stay) and `removed` first
//! removes the injected role's residents (Chesson's "community without the
//! species"), returning their nutrient to the pool so `N_total` is kept.
//!
//! ## Lineage tracking
//!
//! The invader's growth is that of its **lineage** — the cohort and its
//! descendants — not the role's headcount. Descent is read off the event
//! log's `Born` events (one per offspring, `source` = offspring id, `target`
//! = parent, `second_parent` = mate; #443's observability addition): a birth
//! joins the lineage when either parent is a member, membership is permanent,
//! and the live lineage is the members on the roster. Sexual crosses with a
//! resident mate are members too (the either-parent rule) and are counted
//! separately (`cross_births`) so the reader can judge the rule. The
//! lineage's diet is read off `Consumed` events the same way
//! (`lineage_consumed_events`, living / carcass), so a flat cohort can be
//! told from a starving one.
//!
//! The rate is `r = ln(max(N_end, ½) / N_0) / ticks` over the window (`t_inj`
//! ticks by default; an extinct lineage reads at half an agent so its rate is a
//! finite negative number; only the sign enters the criterion).
//!
//! ## Criterion
//!
//! Per (cell, role, arm) over the 8 seeds: the role *invades* when the median
//! rate is > 0 and the distribution-free interval on the median (the sign
//! test's order-statistic interval, #434's binomial computation applied to
//! the sign of each seed's rate; at `n = 8` the ≥ 95 % interval is
//! `[min, max]`) does not straddle zero. A role is *present* in a cell when
//! it holds the guild predicate on at least half the seeds that reached
//! `t_inj` (`guild_seeds`); the pre-#493 tag count (≥ 1 agent at `t_inj`)
//! is kept alongside as `role_tagged_seeds`. A cell is
//! **coexisting-by-criterion** when every present role invades;
//! the relaxed read (median > 0 only) is reported alongside.
//!
//! ## Cross-check against the reduction
//!
//! At the injected centroids the A1/A2 boundary eigenvalues give a predicted
//! sign per role: producer `λ_P(𝓥) − 1 = min(r_P, α_P/θ_P) − μ_P` (A2 i),
//! consumer `λ_C(K_P) − 1 = β·K_P − m` (A1 clause 2, the sign of `I − 1`),
//! decomposer `λ_H(𝓛) − 1` (A2 ii, the sign of `Λ − 1` at the reference
//! lumping). The coefficient mapping is copied from `permanence_crosscheck`
//! and pinned to its example10 numbers; both conversions count both of `κ`'s
//! branches (`χ_P`, #466; `χ_C`, #482), so a heterotroph centroid at `κ ≈ 0`
//! is read like any other.
//!
//! ## What it does NOT do
//!
//! No change to the stepper's dynamics, the evaluator, or the search. No
//! assertion on emergent values beyond smoke checks. Not a CI gate.
//!
//! ## Determinism, sourcing, output
//!
//! Atlas live cells decoded via `decode` over `default_ranges`, plus on
//! request the seed-421 LHS draw `role_emergence` / `energy_bound_check` /
//! `permanence_crosscheck` share (`config_source`), so `sample:i` names the
//! same config everywhere; seeds a fixed contiguous block; cohort positions
//! from a stream keyed on (seed, role); one rayon task per cell with an
//! order-stable per-seed collect, so the artifact is byte-identical across
//! runs. `target/invasion-growth.json` (gitignored) plus a stdout summary.
//! Subset selectors: `INVASION_GROWTH_CELLS=atlas:3,sample:55` (a bare
//! integer is an atlas index; `sample@S:i` is a config of the seed-`S` LHS
//! draw; the unfiltered run is the atlas only),
//! `INVASION_GROWTH_SEEDS=2`, `INVASION_GROWTH_ARMS=intact|removed|both`,
//! `INVASION_GROWTH_T_INJ=2000` (resident phase; the window follows it
//! unless `INVASION_GROWTH_WINDOW` is set), `INVASION_GROWTH_OUT=<path>`.
//!
//! Run with:
//!   cargo run --release -p explorers-search --bin invasion_growth
//! (optional first arg: path to the atlas JSON; default `atlas.json`)
//!
//! Full run: 82 cells × 8 seeds × (2000 resident + up to 7 × 2000 window
//! ticks) at the #507 horizon; the 500-tick run was ~15 minutes in release,
//! so budget hours, or subset it (`INVASION_GROWTH_CELLS`, `_SEEDS`).

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

use explorers_genesis::EvalConfig;
use explorers_genesis_eval::guild::{RoleGuilds, RosterSnapshot, role_guilds};
use explorers_search::config_source::{ConfigSource, parse_selector, resolve_unit, sampled_units};
use explorers_search::qd::COEXISTENCE_FLOOR;
use explorers_search::search::{SearchConfig, decode, default_ranges};
use explorers_search::sweep::plan_tasks;
use explorers_sim::event::{Event, EventKind};
use explorers_sim::topology::{TopologyProjection, TrophicRole};
use explorers_sim::{Agent, TraitVector, World};

/// Fixed contiguous seed block per cell (the `permanence_crosscheck` convention).
const N_SEEDS: u64 = 8;
const SEED_BASE: u64 = 1000;
/// Invader cohort size: small enough to be rare against any live cell's
/// resident (tens to thousands of agents), large enough that one unlucky
/// founder does not decide the lineage.
const INVADER_COHORT: usize = 8;
/// Lineage size is sampled into the record every this many ticks.
const SERIES_INTERVAL: u64 = 25;
/// The living roster is sampled every this many ticks over the second half
/// of the resident phase for the guild read (#493) — `role_emergence`'s
/// classification cadence, so the predicate reads the same series here as
/// there.
const GUILD_SAMPLE_INTERVAL: u64 = 10;
/// A lineage with no survivor is logged at half an agent so its rate is a
/// finite negative number; only the sign carries meaning for the criterion.
const EXTINCT_FLOOR: f64 = 0.5;

/// The three trophic roles, in the order they are injected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum Role {
    Producer,
    Consumer,
    Decomposer,
}

const ROLES: [Role; 3] = [Role::Producer, Role::Consumer, Role::Decomposer];

impl Role {
    fn of(role: TrophicRole) -> Role {
        match role {
            TrophicRole::Producer => Role::Producer,
            TrophicRole::Consumer => Role::Consumer,
            TrophicRole::Decomposer => Role::Decomposer,
        }
    }

    fn trophic(self) -> TrophicRole {
        match self {
            Role::Producer => TrophicRole::Producer,
            Role::Consumer => TrophicRole::Consumer,
            Role::Decomposer => TrophicRole::Decomposer,
        }
    }

    fn tag(self) -> u64 {
        match self {
            Role::Producer => 1,
            Role::Consumer => 2,
            Role::Decomposer => 3,
        }
    }
}

/// Which resident web the cohort is injected into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum Arm {
    /// The resident web as it stands, the injected role's residents included
    /// (the issue's literal protocol).
    Intact,
    /// The resident web with the injected role's residents removed at
    /// `t_inj` (Chesson's invasibility: the community *without* the species).
    Removed,
}

/// Role headcount of a roster, by `topology::trophic_roles`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
struct RoleCounts {
    producers: usize,
    consumers: usize,
    decomposers: usize,
}

impl RoleCounts {
    fn count(&self, role: Role) -> usize {
        match role {
            Role::Producer => self.producers,
            Role::Consumer => self.consumers,
            Role::Decomposer => self.decomposers,
        }
    }

    fn add(&mut self, role: Role) {
        match role {
            Role::Producer => self.producers += 1,
            Role::Consumer => self.consumers += 1,
            Role::Decomposer => self.decomposers += 1,
        }
    }
}

/// Classify a roster and count the roles, optionally excluding a set of ids
/// (the lineage) so the *resident's* composition can be read on its own.
fn role_counts(
    topo: &TopologyProjection,
    agents: &[Agent],
    exclude: Option<&Lineage>,
) -> (HashMap<u64, Role>, RoleCounts) {
    let roles: HashMap<u64, Role> = topo
        .trophic_roles(agents)
        .into_iter()
        .map(|(id, r)| (id, Role::of(r)))
        .collect();
    let mut counts = RoleCounts::default();
    for (id, role) in &roles {
        if exclude.is_some_and(|l| l.is_member(*id)) {
            continue;
        }
        counts.add(*role);
    }
    (roles, counts)
}

/// Mean trait vector of the agents classified into `role`, if any.
fn role_centroid(agents: &[Agent], roles: &HashMap<u64, Role>, role: Role) -> Option<TraitVector> {
    let members: Vec<&Agent> = agents
        .iter()
        .filter(|a| roles.get(&a.id) == Some(&role))
        .collect();
    if members.is_empty() {
        return None;
    }
    let mut mean = members[0].traits;
    for dim in 0..TraitVector::NUM_DIMS {
        let sum: f64 = members.iter().map(|a| a.traits.get(dim) as f64).sum();
        mean.set(dim, (sum / members.len() as f64) as f32);
    }
    Some(mean)
}

/// Inject a cohort of `n` agents with the given traits, spread uniformly over
/// the world and provisioned exactly as `World::new` provisions founders
/// (`initial_energy_per_agent` split by `provision_initial_reserve_structure`,
/// birth structure's bound nutrient drawn from the pool under the agent, free
/// store empty). Positions come from a stream keyed on (seed, role) so the
/// cohort is a pure function of the run. Returns the cohort's ids.
fn inject_cohort(
    world: &mut World,
    traits: TraitVector,
    n: usize,
    energy_per_agent: f32,
    seed: u64,
    role: Role,
) -> Vec<u64> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_mul(7919) ^ role.tag());
    let extent = world.params().world_extent;
    let (reserve, structure, _heat) =
        explorers_sim::provision_initial_reserve_structure(energy_per_agent, world.params());
    let mut ids = Vec::with_capacity(n);
    for _ in 0..n {
        let x = rng.random_range(-extent / 2.0..extent / 2.0);
        let y = rng.random_range(-extent / 2.0..extent / 2.0);
        let agent = Agent::new(0, (x, y), reserve, structure, 0.0, traits);
        let bound = agent.bound_nutrient(world.params());
        if bound > 0.0 {
            *world.nutrient_grid_mut().at_position((x, y)) -= bound;
        }
        world.add_agent(agent);
        ids.push(world.agents().last().expect("just added").id);
    }
    ids
}

/// Remove every agent classified into `role`, returning each body's nutrient
/// to the pool under it so the residual web keeps the cell's nutrient budget
/// (`N_total` is an A2 coefficient). Returns how many were removed.
fn remove_role(world: &mut World, roles: &HashMap<u64, Role>, role: Role) -> usize {
    let removed = world.retain_agents(|a| roles.get(&a.id) != Some(&role));
    let params = world.params().clone();
    for a in &removed {
        let total = a.nutrient_total(&params);
        if total > 0.0 {
            *world.nutrient_grid_mut().at_position(a.position) += total;
        }
    }
    removed.len()
}

/// How the resident phase ended.
#[derive(Clone, Debug, serde::Serialize)]
struct Resident {
    /// `alive` (reached `t_inj`), `extinct`, or `explosion`.
    termination: &'static str,
    tick: u64,
    population: usize,
    roles: RoleCounts,
    /// The #490 guild predicate per role over `(t_inj/2, t_inj]` (#493):
    /// role count ≥ `GUILD_MIN_SIZE` on every sampled tick and ≥ 1 birth
    /// naming a member. This, not the tag count, is what makes a role
    /// testable.
    guilds: RoleGuilds,
}

/// The control arm: the resident stepped through the window with nothing
/// injected, so a role's drift can be told from the injection's effect.
#[derive(Clone, Debug, serde::Serialize)]
struct Control {
    ticks_run: u64,
    stopped: Option<&'static str>,
    roles_end: RoleCounts,
}

/// One injection arm: a cohort of one role into one resident, followed for
/// the window.
#[derive(Clone, Debug, serde::Serialize)]
struct Injection {
    role: Role,
    arm: Arm,
    /// Always `realised` (the role's mean trait vector at `t_inj`): an absent
    /// role is not injected (#491), it is recorded as `not_testable`.
    centroid_source: &'static str,
    centroid: TraitVector,
    cohort: usize,
    residents_removed: usize,
    ticks_run: u64,
    stopped: Option<&'static str>,
    /// Live lineage size every `SERIES_INTERVAL` ticks from injection (the
    /// first entry is the cohort), plus the terminal tick.
    lineage_series: Vec<usize>,
    lineage_end: usize,
    extinct_tick: Option<u64>,
    /// Per-capita per-tick growth of the lineage over the window:
    /// `ln(max(N_end, ½) / N_0) / ticks_run`.
    growth_rate: f64,
    pure_births: usize,
    cross_births: usize,
    /// `Consumed` events whose source is a lineage member over the window,
    /// split by `target_was_carcass` (#491): does the cohort eat at all?
    lineage_consumed_events: ConsumedCounts,
    /// `Consumed` events on a *living* lineage member (the lineage as prey).
    lineage_preyed_upon_events: usize,
    /// Lineage deaths in a tick the member was drained alive (drain-kill or
    /// predation) vs. with no drain that tick (starvation / threshold).
    lineage_deaths_drained: usize,
    lineage_deaths_undrained: usize,
    /// The resident's (non-lineage) composition at the end of the window.
    resident_roles_end: RoleCounts,
}

/// Consumption events by target kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
struct ConsumedCounts {
    living: usize,
    carcass: usize,
}

/// The trait vector a role is injected at, and where it came from.
#[derive(Clone, Debug, serde::Serialize)]
struct Centroid {
    role: Role,
    /// Always `realised` (see `Injection::centroid_source`).
    source: &'static str,
    traits: TraitVector,
}

/// A role that could not be injected on this seed and why: `role_absent`
/// (no agent of the role in the resident at `t_inj`, so there is no realised
/// centroid to inject at — Chesson's question is whether the role can
/// re-invade *this* web, not whether an invented phenotype can) or
/// `no_guild` (the role is tagged on the resident but is not a population by
/// the #490 predicate — one sessile individual is not a community to ask
/// Chesson's question of, #493).
#[derive(Clone, Debug, serde::Serialize)]
struct NotTestable {
    role: Role,
    reason: &'static str,
}

/// One (cell, seed): the resident, the control, and every injection.
#[derive(Clone, Debug, serde::Serialize)]
struct SeedRecord {
    seed: u64,
    resident: Resident,
    centroids: Vec<Centroid>,
    /// Roles not injected on this seed and why (`role_absent` / `no_guild`).
    not_testable: Vec<NotTestable>,
    /// The reduction's boundary eigenvalues at these centroids (only the
    /// roles whose centroids exist).
    predicted: Vec<PredictedSign>,
    control: Option<Control>,
    injections: Vec<Injection>,
}

fn growth_rate(cohort: usize, end: usize, ticks: u64) -> f64 {
    let n_end = if end == 0 { EXTINCT_FLOOR } else { end as f64 };
    (n_end / cohort as f64).ln() / ticks.max(1) as f64
}

/// What a window run returns.
struct WindowOutcome {
    ticks_run: u64,
    stopped: Option<&'static str>,
    series: Vec<usize>,
    alive: usize,
    extinct_tick: Option<u64>,
    lineage: Option<Lineage>,
}

/// Step a forked world through the window, following the lineage (if any)
/// off the log.
fn run_window(
    world: &mut World,
    topo: &mut TopologyProjection,
    mut lineage: Option<Lineage>,
    window: u64,
    max_population: usize,
) -> WindowOutcome {
    let mut cursor = world.event_log().len();
    let cohort = lineage.as_ref().map_or(0, |l| l.alive(world.agents()));
    let mut series = vec![cohort];
    let mut alive = cohort;
    let mut extinct_tick = None;
    let mut ticks_run = 0;
    let mut stopped = None;
    for t in 1..=window {
        world.step();
        ticks_run = t;
        if let Some(l) = lineage.as_mut() {
            l.absorb(world.event_log().since(cursor));
            cursor = world.event_log().len();
            alive = l.alive(world.agents());
            if alive == 0 && extinct_tick.is_none() {
                extinct_tick = Some(t);
            }
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
    topo.update(world.event_log());
    WindowOutcome {
        ticks_run,
        stopped,
        series,
        alive,
        extinct_tick,
        lineage,
    }
}

/// Run one (cell, seed): the resident to `t_inj`, then the control and each
/// (role × arm) injection as forks of that one resident.
fn run_cell_seed(unit: &[f64], seed: u64, t_inj: u64, window: u64, arms: &[Arm]) -> SeedRecord {
    let ranges = default_ranges();
    let (params, dist) = decode(unit, &ranges);
    let max_population = EvalConfig::default().max_population;
    let mut world = World::new(params, dist.clone(), seed);
    // Everything this instrument reads off the log — the projection's roles
    // (Consumed / Reproduced / Died), the guild read and lineage (Born,
    // Consumed, Died) — is in these kinds; the per-agent-per-tick
    // bookkeeping events are dropped at source so a 2000-tick resident at
    // P ≈ 1000 fits in memory. Observer-side: no trajectory changes.
    world.retain_event_kinds(&[
        EventKind::Consumed,
        EventKind::Reproduced,
        EventKind::Died,
        EventKind::Born,
    ]);
    let mut topo = TopologyProjection::new();
    let mut termination = "alive";
    // Living roster on each second-half sample (and on `t_inj` itself), for
    // the guild read: the guild is a population over time, which the
    // history-free world cannot supply after the fact.
    let mut roster_snapshots: Vec<RosterSnapshot> = Vec::new();
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
        let tick = world.tick();
        if tick > t_inj / 2 && (tick % GUILD_SAMPLE_INTERVAL == 0 || tick == t_inj) {
            roster_snapshots.push((
                tick,
                world.agents().iter().map(|a| (a.id, a.traits)).collect(),
            ));
        }
    }
    topo.update(world.event_log());
    let (roles, counts) = role_counts(&topo, world.agents(), None);
    let guilds = if termination == "alive" {
        role_guilds(world.event_log(), &roster_snapshots, t_inj)
    } else {
        RoleGuilds::default()
    };
    let resident = Resident {
        termination,
        tick: world.tick(),
        population: world.agents().len(),
        roles: counts,
        guilds,
    };
    if termination != "alive" {
        return SeedRecord {
            seed,
            resident,
            centroids: Vec::new(),
            not_testable: Vec::new(),
            predicted: Vec::new(),
            control: None,
            injections: Vec::new(),
        };
    }
    let mut centroids = Vec::new();
    let mut not_testable = Vec::new();
    for role in ROLES {
        match role_centroid(world.agents(), &roles, role) {
            Some(traits) => {
                centroids.push(Centroid {
                    role,
                    source: "realised",
                    traits,
                });
                if !guilds.get(role.trophic()) {
                    not_testable.push(NotTestable {
                        role,
                        reason: "no_guild",
                    });
                }
            }
            None => not_testable.push(NotTestable {
                role,
                reason: "role_absent",
            }),
        }
    }
    // Injected: tagged *and* a guild.
    let testable: Vec<&Centroid> = centroids
        .iter()
        .filter(|c| guilds.get(c.role.trophic()))
        .collect();
    // The history has been read (guilds, roles, centroids) and the projection
    // is at the present; drop it so the control and each injection fork clone
    // a near-empty log rather than the resident's whole past — at P ≈ 1000 ×
    // 2000 ticks that past is what does not fit in memory. Absolute indices
    // are kept, so the projection's cursor and the lineage's stay valid.
    world.compact_event_log_before(world.event_log().len());
    let centroid_of = |role: Role| centroids.iter().find(|c| c.role == role).map(|c| &c.traits);
    let predicted = predicted_signs(
        centroid_of(Role::Producer),
        centroid_of(Role::Consumer),
        centroid_of(Role::Decomposer),
        world.params(),
    );

    let control = {
        let mut fork = world.clone();
        let mut fork_topo = topo.clone();
        let out = run_window(&mut fork, &mut fork_topo, None, window, max_population);
        let (_, roles_end) = role_counts(&fork_topo, fork.agents(), None);
        Control {
            ticks_run: out.ticks_run,
            stopped: out.stopped,
            roles_end,
        }
    };

    let mut injections = Vec::with_capacity(testable.len() * arms.len());
    for &arm in arms {
        for c in &testable {
            let (role, centroid_source, centroid) = (c.role, c.source, c.traits);
            let mut fork = world.clone();
            let mut fork_topo = topo.clone();
            let residents_removed = match arm {
                Arm::Intact => 0,
                Arm::Removed => remove_role(&mut fork, &roles, role),
            };
            let ids = inject_cohort(
                &mut fork,
                centroid,
                INVADER_COHORT,
                dist.initial_energy_per_agent,
                seed,
                role,
            );
            let out = run_window(
                &mut fork,
                &mut fork_topo,
                Some(Lineage::new(ids)),
                window,
                max_population,
            );
            let lineage = out.lineage.expect("lineage arm");
            let (_, resident_roles_end) = role_counts(&fork_topo, fork.agents(), Some(&lineage));
            injections.push(Injection {
                role,
                arm,
                centroid_source,
                centroid,
                cohort: INVADER_COHORT,
                residents_removed,
                ticks_run: out.ticks_run,
                stopped: out.stopped,
                lineage_series: out.series,
                lineage_end: out.alive,
                extinct_tick: out.extinct_tick,
                growth_rate: growth_rate(INVADER_COHORT, out.alive, out.ticks_run),
                pure_births: lineage.pure_births,
                cross_births: lineage.cross_births,
                lineage_consumed_events: lineage.consumed,
                lineage_preyed_upon_events: lineage.preyed_upon_events,
                lineage_deaths_drained: lineage.deaths_drained,
                lineage_deaths_undrained: lineage.deaths_undrained,
                resident_roles_end,
            });
        }
    }
    SeedRecord {
        seed,
        resident,
        centroids,
        not_testable,
        predicted,
        control: Some(control),
        injections,
    }
}

/// An invader cohort and its descendants, followed by descent off the event
/// log's `Born` events (#443): a birth joins the lineage when *either* parent
/// is a member. Membership is permanent (the dead stay members), so the live
/// lineage at any tick is the intersection with the world's roster.
#[derive(Debug, Clone)]
struct Lineage {
    members: HashSet<u64>,
    /// Births with both parents in the lineage (or a single lineage parent).
    pure_births: usize,
    /// Sexual births with exactly one parent in the lineage: the lineage's
    /// genes leaving through a resident mate. Counted as members (the
    /// either-parent rule) but reported so the reader can judge the rule.
    cross_births: usize,
    /// `Consumed` events by a member, by target kind.
    consumed: ConsumedCounts,
    /// Mortality attribution (#493 §6): `Consumed` events whose *target* is a
    /// living member (the lineage being eaten), and member deaths split by
    /// whether the member was drained in the tick it died.
    preyed_upon_events: usize,
    deaths_drained: usize,
    deaths_undrained: usize,
    /// Tick on which each member was last drained alive.
    last_drained: HashMap<u64, u64>,
}

impl Lineage {
    fn new(founders: impl IntoIterator<Item = u64>) -> Self {
        Lineage {
            members: founders.into_iter().collect(),
            pure_births: 0,
            cross_births: 0,
            consumed: ConsumedCounts::default(),
            preyed_upon_events: 0,
            deaths_drained: 0,
            deaths_undrained: 0,
            last_drained: HashMap::new(),
        }
    }

    /// Absorb a slice of the event log (in sequence order): descent off
    /// `Born`, diet off `Consumed`.
    fn absorb(&mut self, events: &[Event]) {
        for ev in events {
            if ev.kind == EventKind::Consumed {
                if self.members.contains(&ev.source) {
                    if ev.target_was_carcass {
                        self.consumed.carcass += 1;
                    } else {
                        self.consumed.living += 1;
                    }
                }
                if !ev.target_was_carcass && ev.target.is_some_and(|t| self.members.contains(&t)) {
                    self.preyed_upon_events += 1;
                    self.last_drained.insert(ev.target.unwrap(), ev.tick);
                }
                continue;
            }
            if ev.kind == EventKind::Died && self.members.contains(&ev.source) {
                if self.last_drained.get(&ev.source) == Some(&ev.tick) {
                    self.deaths_drained += 1;
                } else {
                    self.deaths_undrained += 1;
                }
                continue;
            }
            if ev.kind != EventKind::Born {
                continue;
            }
            let first = ev.target.is_some_and(|p| self.members.contains(&p));
            let second = ev.second_parent.is_some_and(|p| self.members.contains(&p));
            if !(first || second) {
                continue;
            }
            self.members.insert(ev.source);
            let outside_mate =
                (ev.target.is_some() && !first) || (ev.second_parent.is_some() && !second);
            if outside_mate {
                self.cross_births += 1;
            } else {
                self.pure_births += 1;
            }
        }
    }

    fn is_member(&self, id: u64) -> bool {
        self.members.contains(&id)
    }

    /// Members alive on the given roster.
    fn alive(&self, agents: &[Agent]) -> usize {
        agents.iter().filter(|a| self.is_member(a.id)).count()
    }
}

// ---------------------------------------------------------------------------
// The reduction's predicted invasion signs (A1 / A2 at the injected centroids)
// ---------------------------------------------------------------------------
//
// The coefficient mapping is copied term for term from
// `permanence_crosscheck.rs` / `permanence_prototype.rs` (a bin cannot be
// imported) and pinned to the same example10 numbers by unit test.

const REFERENCE_BODY_MASS: f32 = 1.0;
const FECUNDITY_FLOOR: f64 = 0.1;
/// A2's reference lumping (`ι = 1`, `μ_P = 0.02`, `q = ν = θ_P`).
const REFERENCE_MU_P: f64 = 0.02;
const REFERENCE_IOTA: f64 = 1.0;

fn maintenance(traits: &TraitVector, p: &explorers_sim::WorldParameters) -> f64 {
    let exp = p.maintenance_cost_exponent;
    (p.base_metabolic_rate
        + traits.photosynthetic_absorption.max(0.0).powf(exp) * p.photo_maintenance_cost
        + traits.heterotrophy.max(0.0).powf(exp) * p.heterotrophy_maintenance_cost
        + traits.mobility.max(0.0).powf(exp) * p.mobility_maintenance_cost
        + REFERENCE_BODY_MASS * p.structure_maintenance_coefficient) as f64
}

/// A cluster's biomass conversion `χ` (`χ_P`, #466; `χ_C`, #482): both of
/// `κ`'s branches followed to the body they build.
fn biomass_conversion(traits: &TraitVector, p: &explorers_sim::WorldParameters) -> f64 {
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

/// A1's producer face: `r_P = χ_P·γ·(F − B_P)` and `K_P = F / B_P`.
fn producer_face(producer: &TraitVector, p: &explorers_sim::WorldParameters) -> (f64, f64, f64) {
    let b_p = maintenance(producer, p);
    let flux = p.solar_flux_magnitude as f64;
    let r_p = biomass_conversion(producer, p) * p.growth_efficiency as f64 * (flux - b_p)
        / REFERENCE_BODY_MASS as f64;
    let k_p = if b_p > 0.0 { flux / b_p } else { f64::INFINITY };
    (r_p, k_p, b_p)
}

/// The heterotroph's A1 coefficients against a producer: attack `a`,
/// maintenance `m`, kernel `e`, conversion `β = χ_C·γ·e·a` (#482).
fn heterotroph_terms(
    producer: &TraitVector,
    heterotroph: &TraitVector,
    p: &explorers_sim::WorldParameters,
) -> (f64, f64, f64, f64) {
    let a = heterotroph.heterotrophy.max(0.0) as f64 / REFERENCE_BODY_MASS as f64;
    let m = maintenance(heterotroph, p) / REFERENCE_BODY_MASS as f64;
    let d = producer.distance(heterotroph) as f64;
    let e = p.base_trophic_efficiency as f64 * (-(p.trophic_distance_decay as f64) * d).exp();
    let beta = biomass_conversion(heterotroph, p) * p.growth_efficiency as f64 * e * a;
    (a, m, e, beta)
}

/// The reduction's predicted sign for one role's invasion.
#[derive(Clone, Debug, serde::Serialize)]
struct PredictedSign {
    role: Role,
    /// Which boundary eigenvalue: `lambda_P(virgin)` (A2 i), `lambda_C(K_P)`
    /// (A1 clause 2), `lambda_H(lockup)` (A2 ii).
    eigenvalue: &'static str,
    /// `λ − 1`: the per-tick invasion growth rate the reduction predicts.
    eigen_excess: f64,
    /// The note's headline ratio for that clause: `r_P`, `I`, `Λ`.
    rate_coefficient: f64,
    positive: bool,
}

fn predicted_signs(
    producer: Option<&TraitVector>,
    consumer: Option<&TraitVector>,
    decomposer: Option<&TraitVector>,
    p: &explorers_sim::WorldParameters,
) -> Vec<PredictedSign> {
    // Every eigenvalue is read against the producer's face; without a
    // producer centroid nothing is predicted.
    let Some(producer) = producer else {
        return Vec::new();
    };
    let (r_p, k_p, _) = producer_face(producer, p);
    let mut out = Vec::new();
    // (i) the producer invades the virgin pool: λ_P(𝓥) = 1 + min(r_P, α_P/θ_P) − μ_P.
    let theta_p = explorers_sim::stoichiometric_demand(producer, 1.0, p) as f64;
    let alpha_p = producer.photosynthetic_absorption.max(0.0) as f64;
    let virgin_rate = if theta_p > 0.0 {
        r_p.min(alpha_p / theta_p)
    } else {
        r_p
    };
    let producer_excess = virgin_rate - REFERENCE_MU_P;
    out.push(PredictedSign {
        role: Role::Producer,
        eigenvalue: "lambda_P(virgin)",
        eigen_excess: producer_excess,
        rate_coefficient: r_p,
        positive: producer_excess > 0.0,
    });
    // (ii) the consumer invades the standing crop: λ_C(K_P) = 1 + β·K_P − m.
    if let Some(consumer) = consumer {
        let (_, m_c, _, beta_c) = heterotroph_terms(producer, consumer, p);
        let consumer_excess = beta_c * k_p - m_c;
        let invasion_ratio = if m_c > 0.0 {
            beta_c * k_p / m_c
        } else {
            f64::INFINITY
        };
        out.push(PredictedSign {
            role: Role::Consumer,
            eigenvalue: "lambda_C(K_P)",
            eigen_excess: consumer_excess,
            rate_coefficient: invasion_ratio,
            positive: consumer_excess > 0.0,
        });
    }
    // (iii) the heterotroph invades the lockup corner on the pile:
    // λ_H(𝓛) = 1 + σ·N_total·min(χ_C·γ·e_C, q/θ_C) − m at the reference lumping.
    if let Some(decomposer) = decomposer {
        let (a_d, m_d, e_d, _) = heterotroph_terms(producer, decomposer, p);
        let theta_d = explorers_sim::stoichiometric_demand(decomposer, 1.0, p) as f64;
        let chi_d_gamma = biomass_conversion(decomposer, p) * p.growth_efficiency as f64;
        let n_total = p.initial_nutrient_pool as f64;
        let (q, nu) = (theta_p, theta_p * REFERENCE_BODY_MASS as f64);
        let sigma = if nu > 0.0 {
            a_d * REFERENCE_IOTA / nu
        } else {
            0.0
        };
        let liebig = if theta_d > 0.0 {
            (chi_d_gamma * e_d).min(q / theta_d)
        } else {
            chi_d_gamma * e_d
        };
        let pile_growth = sigma * n_total * liebig;
        let decomposer_excess = pile_growth - m_d;
        let lockup_escape_ratio = if m_d > 0.0 {
            pile_growth / m_d
        } else {
            f64::INFINITY
        };
        out.push(PredictedSign {
            role: Role::Decomposer,
            eigenvalue: "lambda_H(lockup)",
            eigen_excess: decomposer_excess,
            rate_coefficient: lockup_escape_ratio,
            positive: decomposer_excess > 0.0,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Seed-ensemble statistics
// ---------------------------------------------------------------------------

fn binomial_cdf_half(n: usize, below: usize) -> f64 {
    // P(Bin(n, ½) < below)
    let mut total = 0.0;
    let mut c = 1.0_f64;
    for i in 0..below.min(n + 1) {
        if i > 0 {
            c = c * (n - i + 1) as f64 / i as f64;
        }
        total += c;
    }
    total / 2f64.powi(n as i32)
}

/// Rank `k` of the distribution-free two-sided ≥ 95 % confidence interval on
/// the median, `[x_(k), x_(n+1−k)]` (the sign test / order-statistic
/// interval — the binomial computation of #434 applied to the sign of each
/// seed's rate). `None` when no rank achieves it (`n < 6`).
fn median_interval_rank(n: usize) -> Option<usize> {
    (1..=n.div_ceil(2))
        .rev()
        .find(|&k| binomial_cdf_half(n, k) <= 0.025)
}

fn binomial_sf(k: usize, n: usize, p: f64) -> f64 {
    // P(X ≥ k | n, p)
    let mut total = 0.0;
    let mut c = 1.0_f64;
    for i in 0..=n {
        if i > 0 {
            c = c * (n - i + 1) as f64 / i as f64;
        }
        if i >= k {
            total += c * p.powi(i as i32) * (1.0 - p).powi((n - i) as i32);
        }
    }
    total
}

/// Exact two-sided 95 % Clopper–Pearson lower bound on a binomial proportion
/// `k/n` (bisection on the binomial tail, as #434).
fn clopper_pearson_lower(k: usize, n: usize) -> f64 {
    if k == 0 || n == 0 {
        return 0.0;
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if binomial_sf(k, n, mid) < 0.025 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n == 0 {
        f64::NAN
    } else if n % 2 == 1 {
        sorted[n / 2]
    } else {
        0.5 * (sorted[n / 2 - 1] + sorted[n / 2])
    }
}

/// A role's invasion growth rate over the seed ensemble.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
struct RateSummary {
    n: usize,
    median: f64,
    min: f64,
    max: f64,
    positive_seeds: usize,
    /// Exact 95 % Clopper–Pearson lower bound on the positive-seed fraction.
    positive_fraction_lower: f64,
    /// The order-statistic interval on the median (`None` below `n = 6`).
    interval: Option<(f64, f64)>,
    median_positive: bool,
    /// The criterion: median > 0 and the interval does not straddle zero.
    invades: bool,
}

fn summarise_rates(rates: &[f64]) -> RateSummary {
    let mut sorted = rates.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("finite rates"));
    let n = sorted.len();
    let median = median(&sorted);
    let positive_seeds = sorted.iter().filter(|r| **r > 0.0).count();
    let interval = median_interval_rank(n).map(|k| (sorted[k - 1], sorted[n - k]));
    let median_positive = median > 0.0;
    RateSummary {
        n,
        median,
        min: sorted.first().copied().unwrap_or(f64::NAN),
        max: sorted.last().copied().unwrap_or(f64::NAN),
        positive_seeds,
        positive_fraction_lower: clopper_pearson_lower(positive_seeds, n),
        interval,
        median_positive,
        invades: median_positive && interval.is_some_and(|(lo, _)| lo > 0.0),
    }
}

/// One (role, arm) of a cell over the seeds that reached `t_inj`.
#[derive(Clone, Debug, serde::Serialize)]
struct RoleVerdict {
    role: Role,
    arm: Arm,
    /// Seeds on which ≥ 1 agent carried the role at `t_inj` — the pre-#493
    /// presence read, kept so the two sit side by side.
    role_tagged_seeds: usize,
    /// Seeds on which the role held the #490 guild predicate over the
    /// resident phase (the seeds that were injected).
    guild_seeds: usize,
    n_seeds: usize,
    /// A guild on at least half the seeds (#493).
    present: bool,
    rates: RateSummary,
}

/// Per-seed input to the verdict: the resident's roles and guilds at `t_inj`
/// and each (role, arm) growth rate.
type SeedRates = (RoleCounts, RoleGuilds, Vec<(Role, Arm, f64)>);

fn role_verdicts(seeds: &[SeedRates], arms: &[Arm]) -> Vec<RoleVerdict> {
    let n_seeds = seeds.len();
    let mut out = Vec::new();
    for &arm in arms {
        for role in ROLES {
            let role_tagged_seeds = seeds
                .iter()
                .filter(|(counts, _, _)| counts.count(role) > 0)
                .count();
            let guild_seeds = seeds
                .iter()
                .filter(|(_, guilds, _)| guilds.get(role.trophic()))
                .count();
            let rates: Vec<f64> = seeds
                .iter()
                .flat_map(|(_, _, rs)| {
                    rs.iter()
                        .filter(|(r, a, _)| *r == role && *a == arm)
                        .map(|(_, _, rate)| *rate)
                })
                .collect();
            out.push(RoleVerdict {
                role,
                arm,
                role_tagged_seeds,
                guild_seeds,
                n_seeds,
                present: n_seeds > 0 && 2 * guild_seeds >= n_seeds,
                rates: summarise_rates(&rates),
            });
        }
    }
    out
}

/// The cell's verdict on one arm: every role present in the resident web
/// invades when rare.
#[derive(Clone, Debug, serde::Serialize)]
struct CellVerdict {
    arm: Arm,
    roles_present: usize,
    roles_invading: usize,
    roles_invading_by_median: usize,
    /// Every present role has median > 0 with an interval clear of zero.
    coexisting_strict: bool,
    /// Every present role has median > 0 (the relaxed read).
    coexisting_by_median: bool,
}

fn cell_verdict(verdicts: &[RoleVerdict], arm: Arm) -> CellVerdict {
    let present: Vec<&RoleVerdict> = verdicts
        .iter()
        .filter(|v| v.arm == arm && v.present)
        .collect();
    let roles_invading = present.iter().filter(|v| v.rates.invades).count();
    let roles_invading_by_median = present.iter().filter(|v| v.rates.median_positive).count();
    CellVerdict {
        arm,
        roles_present: present.len(),
        roles_invading,
        roles_invading_by_median,
        coexisting_strict: !present.is_empty() && roles_invading == present.len(),
        coexisting_by_median: !present.is_empty() && roles_invading_by_median == present.len(),
    }
}

// ---------------------------------------------------------------------------
// Predicted vs observed
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum Agreement {
    Agree,
    /// The reduction says the role invades; the lineage's median rate is ≤ 0.
    PredictedPositiveObservedNegative,
    /// The reduction says the role cannot invade; the lineage grew.
    PredictedNegativeObservedPositive,
    /// The reduction's sign differs across seeds (the centroids move).
    PredictedSplit,
    /// The role is absent from the resident web; not compared.
    Absent,
}

fn sign_agreement(
    predicted_positive: usize,
    n: usize,
    observed_median: f64,
    present: bool,
) -> Agreement {
    if !present || n == 0 {
        return Agreement::Absent;
    }
    let predicted = if 2 * predicted_positive > n {
        Some(true)
    } else if 2 * predicted_positive < n {
        Some(false)
    } else {
        None
    };
    let observed = observed_median > 0.0;
    match predicted {
        None => Agreement::PredictedSplit,
        Some(p) if p == observed => Agreement::Agree,
        Some(true) => Agreement::PredictedPositiveObservedNegative,
        Some(false) => Agreement::PredictedNegativeObservedPositive,
    }
}

/// One row of the predicted-vs-observed table: a (role, arm) of a cell.
#[derive(Clone, Debug, serde::Serialize)]
struct SignRow {
    role: Role,
    arm: Arm,
    eigenvalue: &'static str,
    predicted_positive_seeds: usize,
    n_seeds: usize,
    median_eigen_excess: f64,
    observed_median_rate: f64,
    observed_positive_seeds: usize,
    agreement: Agreement,
}

/// The resident's response to an injection: median over seeds of (resident
/// role count at the end of the injected window − the control's), per role.
#[derive(Clone, Debug, serde::Serialize)]
struct Displacement {
    role: Role,
    arm: Arm,
    n_seeds: usize,
    /// Median Δ producers / consumers / decomposers of the *resident*
    /// (non-lineage) roster against the control, negative = displaced.
    median_delta: [f64; 3],
    /// Seeds on which some resident role fell below the control.
    seeds_with_any_loss: usize,
}

/// An atlas live cell's metadata, carried on its record.
#[derive(Clone, Debug, serde::Serialize)]
struct AtlasMeta {
    cell: [usize; 3],
    fitness: f32,
    coexistence_fraction: f32,
    sample_count: u32,
    decomposer_fraction: f32,
}

/// One config: an atlas live cell or a `sample:` config from the shared
/// seed-421 LHS draw (#491).
#[derive(Clone, Debug, serde::Serialize)]
struct CellRecord {
    source: ConfigSource,
    /// Index into the source: the atlas's cell list or the LHS draw.
    index: usize,
    /// The atlas cell's metadata; `None` for a `sample:` config.
    atlas: Option<AtlasMeta>,
    /// `coexistence_fraction ≥ COEXISTENCE_FLOOR` (the projection's floor);
    /// `false` for a `sample:` config, which the atlas never judged.
    atlas_coexisting: bool,
    initial_cluster_count: u32,
    initial_population_size: u32,
    seeds_reached_injection: usize,
    median_resident_population: usize,
    role_verdicts: Vec<RoleVerdict>,
    verdicts: Vec<CellVerdict>,
    sign_table: Vec<SignRow>,
    displacement: Vec<Displacement>,
    seeds: Vec<SeedRecord>,
}

fn median_f64(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    median(values)
}

fn evaluate_cell(
    source: ConfigSource,
    index: usize,
    unit: &[f64],
    atlas: Option<AtlasMeta>,
    seeds: u64,
    t_inj: u64,
    window: u64,
    arms: &[Arm],
) -> CellRecord {
    let ranges = default_ranges();
    let (params, dist) = decode(unit, &ranges);
    // Seeds are independent; rayon's indexed collect preserves seed order, so
    // the record is bit-identical to the sequential map.
    let records: Vec<SeedRecord> = (0..seeds)
        .into_par_iter()
        .map(|s| run_cell_seed(unit, SEED_BASE + s, t_inj, window, arms))
        .collect();
    let reached: Vec<&SeedRecord> = records
        .iter()
        .filter(|r| r.resident.termination == "alive")
        .collect();
    let seed_rates: Vec<SeedRates> = reached
        .iter()
        .map(|r| {
            (
                r.resident.roles,
                r.resident.guilds,
                r.injections
                    .iter()
                    .map(|i| (i.role, i.arm, i.growth_rate))
                    .collect(),
            )
        })
        .collect();
    let role_verdicts = role_verdicts(&seed_rates, arms);
    let verdicts: Vec<CellVerdict> = arms
        .iter()
        .map(|&a| cell_verdict(&role_verdicts, a))
        .collect();
    let sign_table: Vec<SignRow> = role_verdicts
        .iter()
        .map(|v| {
            let preds: Vec<&PredictedSign> = reached
                .iter()
                .flat_map(|r| r.predicted.iter().filter(|p| p.role == v.role))
                .collect();
            let predicted_positive_seeds = preds.iter().filter(|p| p.positive).count();
            let mut excess: Vec<f64> = preds.iter().map(|p| p.eigen_excess).collect();
            SignRow {
                role: v.role,
                arm: v.arm,
                eigenvalue: preds.first().map_or("", |p| p.eigenvalue),
                predicted_positive_seeds,
                n_seeds: preds.len(),
                median_eigen_excess: median_f64(&mut excess),
                observed_median_rate: v.rates.median,
                observed_positive_seeds: v.rates.positive_seeds,
                agreement: sign_agreement(
                    predicted_positive_seeds,
                    preds.len(),
                    v.rates.median,
                    v.present,
                ),
            }
        })
        .collect();
    let displacement: Vec<Displacement> = arms
        .iter()
        .flat_map(|&arm| ROLES.into_iter().map(move |role| (role, arm)))
        .map(|(role, arm)| {
            let mut deltas: [Vec<f64>; 3] = [Vec::new(), Vec::new(), Vec::new()];
            let mut seeds_with_any_loss = 0;
            for r in &reached {
                let Some(control) = r.control.as_ref() else {
                    continue;
                };
                let Some(inj) = r.injections.iter().find(|i| i.role == role && i.arm == arm) else {
                    continue;
                };
                let mut any_loss = false;
                for (k, other) in ROLES.into_iter().enumerate() {
                    let d = inj.resident_roles_end.count(other) as f64
                        - control.roles_end.count(other) as f64;
                    if d < 0.0 {
                        any_loss = true;
                    }
                    deltas[k].push(d);
                }
                if any_loss {
                    seeds_with_any_loss += 1;
                }
            }
            Displacement {
                role,
                arm,
                n_seeds: deltas[0].len(),
                median_delta: [
                    median_f64(&mut deltas[0]),
                    median_f64(&mut deltas[1]),
                    median_f64(&mut deltas[2]),
                ],
                seeds_with_any_loss,
            }
        })
        .collect();
    let mut pops: Vec<f64> = reached
        .iter()
        .map(|r| r.resident.population as f64)
        .collect();
    CellRecord {
        source,
        index,
        atlas_coexisting: atlas
            .as_ref()
            .is_some_and(|a| a.coexistence_fraction >= COEXISTENCE_FLOOR),
        atlas,
        initial_cluster_count: dist.initial_cluster_count,
        initial_population_size: params.initial_population_size,
        seeds_reached_injection: reached.len(),
        median_resident_population: if pops.is_empty() {
            0
        } else {
            median_f64(&mut pops) as usize
        },
        role_verdicts,
        verdicts,
        sign_table,
        displacement,
        seeds: records,
    }
}

// ---------------------------------------------------------------------------
// Summary, artifact, main
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Serialize)]
struct ArmSummary {
    arm: Arm,
    cells: usize,
    /// Cells coexisting by the strict criterion (every present role: median
    /// > 0 and interval clear of zero).
    coexisting_strict: usize,
    coexisting_by_median: usize,
    /// The headline: atlas cells with `coexistence_fraction ≥ 0.5` that are
    /// coexisting by the strict criterion, and the rest of the 2 × 2.
    atlas_coexisting: usize,
    atlas_coexisting_and_strict: usize,
    atlas_coexisting_and_by_median: usize,
    atlas_not_coexisting_and_strict: usize,
    atlas_not_coexisting_and_by_median: usize,
    /// Cells with at least two roles present — the only ones on which the
    /// criterion says anything about coexistence (a producer-only web passes
    /// as soon as the producer can invade its own emptied world).
    multi_role: usize,
    multi_role_and_strict: usize,
    multi_role_and_by_median: usize,
    /// Multi-role cells that are also atlas-coexisting.
    atlas_coexisting_multi_role: usize,
    atlas_coexisting_multi_role_and_strict: usize,
    atlas_coexisting_multi_role_and_by_median: usize,
    /// Per role: (present cells, invading strict, invading by median).
    per_role: Vec<(Role, usize, usize, usize)>,
    /// Per role: cells on which the role was *tagged* on ≥ half the seeds —
    /// the pre-#493 presence read, next to the guild read in `per_role`.
    per_role_tagged: Vec<(Role, usize)>,
    /// Per role: agreement tallies (agree, pred+/obs−, pred−/obs+, split, absent).
    sign_agreement: Vec<(Role, [usize; 5])>,
}

fn arm_summary(records: &[CellRecord], arm: Arm) -> ArmSummary {
    let verdict = |r: &CellRecord| r.verdicts.iter().find(|v| v.arm == arm).cloned();
    let strict = |r: &CellRecord| verdict(r).is_some_and(|v| v.coexisting_strict);
    let by_median = |r: &CellRecord| verdict(r).is_some_and(|v| v.coexisting_by_median);
    let per_role = ROLES
        .into_iter()
        .map(|role| {
            let rows: Vec<&RoleVerdict> = records
                .iter()
                .flat_map(|r| r.role_verdicts.iter())
                .filter(|v| v.arm == arm && v.role == role && v.present)
                .collect();
            (
                role,
                rows.len(),
                rows.iter().filter(|v| v.rates.invades).count(),
                rows.iter().filter(|v| v.rates.median_positive).count(),
            )
        })
        .collect();
    let sign_agreement = ROLES
        .into_iter()
        .map(|role| {
            let mut tally = [0usize; 5];
            for row in records
                .iter()
                .flat_map(|r| r.sign_table.iter())
                .filter(|s| s.arm == arm && s.role == role)
            {
                let k = match row.agreement {
                    Agreement::Agree => 0,
                    Agreement::PredictedPositiveObservedNegative => 1,
                    Agreement::PredictedNegativeObservedPositive => 2,
                    Agreement::PredictedSplit => 3,
                    Agreement::Absent => 4,
                };
                tally[k] += 1;
            }
            (role, tally)
        })
        .collect();
    let multi = |r: &CellRecord| verdict(r).is_some_and(|v| v.roles_present >= 2);
    let per_role_tagged = ROLES
        .into_iter()
        .map(|role| {
            let n = records
                .iter()
                .filter(|r| {
                    r.role_verdicts.iter().any(|v| {
                        v.arm == arm
                            && v.role == role
                            && v.n_seeds > 0
                            && 2 * v.role_tagged_seeds >= v.n_seeds
                    })
                })
                .count();
            (role, n)
        })
        .collect();
    ArmSummary {
        arm,
        cells: records.len(),
        multi_role: records.iter().filter(|r| multi(r)).count(),
        multi_role_and_strict: records.iter().filter(|r| multi(r) && strict(r)).count(),
        multi_role_and_by_median: records.iter().filter(|r| multi(r) && by_median(r)).count(),
        atlas_coexisting_multi_role: records
            .iter()
            .filter(|r| r.atlas_coexisting && multi(r))
            .count(),
        atlas_coexisting_multi_role_and_strict: records
            .iter()
            .filter(|r| r.atlas_coexisting && multi(r) && strict(r))
            .count(),
        atlas_coexisting_multi_role_and_by_median: records
            .iter()
            .filter(|r| r.atlas_coexisting && multi(r) && by_median(r))
            .count(),
        coexisting_strict: records.iter().filter(|r| strict(r)).count(),
        coexisting_by_median: records.iter().filter(|r| by_median(r)).count(),
        atlas_coexisting: records.iter().filter(|r| r.atlas_coexisting).count(),
        atlas_coexisting_and_strict: records
            .iter()
            .filter(|r| r.atlas_coexisting && strict(r))
            .count(),
        atlas_coexisting_and_by_median: records
            .iter()
            .filter(|r| r.atlas_coexisting && by_median(r))
            .count(),
        atlas_not_coexisting_and_strict: records
            .iter()
            .filter(|r| !r.atlas_coexisting && strict(r))
            .count(),
        atlas_not_coexisting_and_by_median: records
            .iter()
            .filter(|r| !r.atlas_coexisting && by_median(r))
            .count(),
        per_role,
        per_role_tagged,
        sign_agreement,
    }
}

#[derive(serde::Serialize)]
struct Summary {
    t_inj: u64,
    window: u64,
    n_seeds: u64,
    cohort: usize,
    atlas_cells: usize,
    sampled_configs: usize,
    cells_run: usize,
    seeds_reached_injection: usize,
    arms: Vec<ArmSummary>,
}

#[derive(serde::Serialize)]
struct Artifact {
    summary: Summary,
    cells: Vec<CellRecord>,
}

#[derive(serde::Deserialize)]
struct AtlasFile {
    cells: Vec<AtlasCellIn>,
}

#[derive(serde::Deserialize)]
struct AtlasCellIn {
    cell: [usize; 3],
    fitness: f32,
    coexistence_fraction: f32,
    sample_count: u32,
    decomposer_fraction: f32,
    unit: Vec<f64>,
}

impl AtlasCellIn {
    fn meta(&self) -> AtlasMeta {
        AtlasMeta {
            cell: self.cell,
            fitness: self.fitness,
            coexistence_fraction: self.coexistence_fraction,
            sample_count: self.sample_count,
            decomposer_fraction: self.decomposer_fraction,
        }
    }
}

/// `INVASION_GROWTH_CELLS=atlas:3,sample:55` — the `source:index` grammar of
/// `permanence_crosscheck` / `role_emergence`; a bare integer is an atlas
/// index. `None` when unset.
fn parse_cell_filter() -> Option<HashSet<(ConfigSource, usize)>> {
    std::env::var("INVASION_GROWTH_CELLS")
        .ok()
        .map(|raw| parse_cell_filter_from(&raw))
}

fn parse_cell_filter_from(raw: &str) -> HashSet<(ConfigSource, usize)> {
    parse_selector(raw, "INVASION_GROWTH_CELLS", Some(ConfigSource::Atlas))
}

/// The configs to run, in a fixed order (atlas, then the seed-421 LHS draw,
/// then any other draw a `sample@S:i` names — the shared sweep order). Unset
/// filter means the full atlas run; `sample` configs run only when named.
fn select_tasks(
    atlas_len: usize,
    sample_len: usize,
    filter: Option<&HashSet<(ConfigSource, usize)>>,
) -> Vec<(ConfigSource, usize)> {
    match filter {
        None => (0..atlas_len).map(|i| (ConfigSource::Atlas, i)).collect(),
        Some(f) => plan_tasks(atlas_len, sample_len, Some(f), &HashSet::new(), None),
    }
}

/// `INVASION_GROWTH_ARMS=intact|removed|both` (default both).
fn parse_arms() -> Vec<Arm> {
    match std::env::var("INVASION_GROWTH_ARMS").as_deref() {
        Ok("intact") => vec![Arm::Intact],
        Ok("removed") => vec![Arm::Removed],
        Ok("both") | Err(_) => vec![Arm::Intact, Arm::Removed],
        Ok(other) => panic!("INVASION_GROWTH_ARMS {other:?} must be intact|removed|both"),
    }
}

fn main() {
    let atlas_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "atlas.json".to_string());
    let env_u64 = |name: &str| std::env::var(name).ok().and_then(|v| v.parse::<u64>().ok());
    let t_inj = env_u64("INVASION_GROWTH_T_INJ")
        .filter(|&t| t > 0)
        .unwrap_or(SearchConfig::default().max_ticks);
    let window = env_u64("INVASION_GROWTH_WINDOW")
        .filter(|&w| w > 0)
        .unwrap_or(t_inj);
    let contents =
        std::fs::read_to_string(&atlas_path).unwrap_or_else(|e| panic!("read {atlas_path}: {e}"));
    let atlas: AtlasFile =
        serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {atlas_path}: {e}"));
    let sampled = sampled_units(default_ranges().len());
    let cell_filter = parse_cell_filter();
    let seeds = std::env::var("INVASION_GROWTH_SEEDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map_or(N_SEEDS, |n| n.clamp(1, N_SEEDS));
    let arms = parse_arms();
    eprintln!(
        "invasion_growth: {} atlas cells (+ {} sampled configs on request) × {} roles × {:?} × {seeds} seeds; t_inj {t_inj}, window {window}, cohort {INVADER_COHORT}",
        atlas.cells.len(),
        sampled.len(),
        ROLES.len(),
        arms
    );
    if cell_filter.is_some() || seeds != N_SEEDS || t_inj != SearchConfig::default().max_ticks {
        eprintln!(
            "invasion_growth: SUBSET MODE — cells={:?}, seeds={seeds}, t_inj={t_inj} (summary is partial)",
            cell_filter.as_ref().map(|f| f.len())
        );
    }
    let tasks = select_tasks(atlas.cells.len(), sampled.len(), cell_filter.as_ref());
    let total = tasks.len();
    let done = AtomicUsize::new(0);
    let start = Instant::now();
    let log_step = (total / 20).max(1);
    // One task per cell; this collect and the per-seed one inside are both
    // order-stable, so the artifact is byte-identical across runs.
    let records: Vec<CellRecord> = tasks
        .par_iter()
        .map(|&(source, i)| {
            let (unit, meta) = match source {
                ConfigSource::Atlas => (
                    Cow::Borrowed(atlas.cells[i].unit.as_slice()),
                    Some(atlas.cells[i].meta()),
                ),
                source @ ConfigSource::Sample(_) => (resolve_unit(source, i, &[], &sampled), None),
            };
            let record = evaluate_cell(source, i, &unit, meta, seeds, t_inj, window, &arms);
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n.is_multiple_of(log_step) || n == total {
                eprintln!(
                    "  progress: {n}/{total} cells done ({:.0}s elapsed)",
                    start.elapsed().as_secs_f64()
                );
            }
            record
        })
        .collect();
    eprintln!(
        "invasion_growth: all {total} cells complete in {:.0}s",
        start.elapsed().as_secs_f64()
    );
    let summary = Summary {
        t_inj,
        window,
        n_seeds: seeds,
        cohort: INVADER_COHORT,
        atlas_cells: atlas.cells.len(),
        sampled_configs: sampled.len(),
        cells_run: records.len(),
        seeds_reached_injection: records.iter().map(|r| r.seeds_reached_injection).sum(),
        arms: arms.iter().map(|&a| arm_summary(&records, a)).collect(),
    };
    print_summary(&summary);
    write_artifact(&Artifact {
        summary,
        cells: records,
    });
}

fn print_summary(s: &Summary) {
    println!("\n# Invasion growth (issue #443)");
    println!(
        "# {} atlas cells + {} sampled configs ({} run) × 3 roles × {} seeds; t_inj {}, window {}, cohort {}; {} seeds reached t_inj",
        s.atlas_cells,
        s.sampled_configs,
        s.cells_run,
        s.n_seeds,
        s.t_inj,
        s.window,
        s.cohort,
        s.seeds_reached_injection
    );
    for a in &s.arms {
        println!("\n## arm: {:?}", a.arm);
        println!(
            "  coexisting by criterion: strict {}/{}, by median {}/{}",
            a.coexisting_strict, a.cells, a.coexisting_by_median, a.cells
        );
        println!(
            "  atlas coexisting (coexistence_fraction >= {COEXISTENCE_FLOOR}): {} — of which strict {}, by median {}; not-atlas-coexisting but strict {}, by median {}",
            a.atlas_coexisting,
            a.atlas_coexisting_and_strict,
            a.atlas_coexisting_and_by_median,
            a.atlas_not_coexisting_and_strict,
            a.atlas_not_coexisting_and_by_median
        );
        println!(
            "  multi-role cells (>= 2 roles present): {} — strict {}, by median {}; of those atlas-coexisting: {} — strict {}, by median {}",
            a.multi_role,
            a.multi_role_and_strict,
            a.multi_role_and_by_median,
            a.atlas_coexisting_multi_role,
            a.atlas_coexisting_multi_role_and_strict,
            a.atlas_coexisting_multi_role_and_by_median
        );
        for (role, present, strict, by_median) in &a.per_role {
            let tagged = a
                .per_role_tagged
                .iter()
                .find(|(r, _)| r == role)
                .map_or(0, |(_, n)| *n);
            println!(
                "  {role:?}: present (guild) in {present} cells [tagged in {tagged}], invades strict {strict}, by median {by_median}"
            );
        }
        for (role, t) in &a.sign_agreement {
            println!(
                "  sign vs reduction, {role:?}: agree {}, pred+/obs- {}, pred-/obs+ {}, split {}, absent {}",
                t[0], t[1], t[2], t[3], t[4]
            );
        }
    }
}

fn write_artifact(artifact: &Artifact) {
    std::fs::create_dir_all("target").ok();
    let path = std::env::var("INVASION_GROWTH_OUT")
        .unwrap_or_else(|_| "target/invasion-growth.json".to_string());
    let json = serde_json::to_string_pretty(artifact).expect("serialise artifact");
    std::fs::write(&path, json).unwrap_or_else(|e| panic!("write {path}: {e}"));
    eprintln!(
        "invasion_growth: wrote {path} ({} cells)",
        artifact.cells.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_sim::TraitVector;

    fn born(seq: u64, child: u64, parent: u64, mate: Option<u64>) -> Event {
        Event {
            tick: 1,
            seq,
            kind: EventKind::Born,
            source: child,
            target: Some(parent),
            energy_delta: 0.0,
            position: None,
            target_was_carcass: false,
            second_parent: mate,
        }
    }

    fn died(seq: u64, id: u64) -> Event {
        Event {
            tick: 2,
            seq,
            kind: EventKind::Died,
            source: id,
            target: None,
            energy_delta: 0.0,
            position: None,
            target_was_carcass: false,
            second_parent: None,
        }
    }

    fn roster(ids: &[u64]) -> Vec<Agent> {
        ids.iter()
            .map(|&id| {
                Agent::new(
                    id,
                    (0.0, 0.0),
                    1.0,
                    1.0,
                    0.0,
                    TraitVector {
                        photosynthetic_absorption: 0.0,
                        heterotrophy: 0.0,
                        mobility: 0.0,
                        kappa: 0.0,
                        fecundity: 0.0,
                        asexual_propensity: 0.0,
                        dispersal: 0.0,
                    },
                )
            })
            .collect()
    }

    /// Smoke check on one (config, seed): the resident runs to `t_inj`, every
    /// role with a guild is injected on both arms (an absent or guild-less
    /// role is `not_testable`), the control runs alongside, and the whole
    /// record is deterministic. No assertion on the emergent rates.
    #[test]
    fn run_cell_seed_injects_every_role_on_every_arm_and_is_deterministic() {
        let ranges = default_ranges();
        let unit = vec![0.5; ranges.len()];
        let run = || run_cell_seed(&unit, SEED_BASE, 30, 30, &[Arm::Intact, Arm::Removed]);
        let record = run();
        assert_eq!(record.seed, SEED_BASE);
        assert_eq!(record.resident.termination, "alive");
        assert_eq!(record.resident.tick, 30);
        let control = record.control.as_ref().expect("control arm ran");
        assert_eq!(control.ticks_run, 30);
        let guilds = ROLES
            .iter()
            .filter(|&&r| record.resident.guilds.get(r.trophic()))
            .count();
        assert!(guilds >= 1, "the producers breed inside 30 ticks");
        assert_eq!(record.injections.len(), guilds * 2);
        assert_eq!(record.not_testable.len(), ROLES.len() - guilds);
        for inj in &record.injections {
            assert!(record.resident.guilds.get(inj.role.trophic()));
            assert_eq!(inj.cohort, INVADER_COHORT);
            assert!(inj.growth_rate.is_finite(), "{inj:?}");
            assert!(inj.ticks_run >= 1 && inj.ticks_run <= 30);
            assert_eq!(inj.lineage_series.first().copied(), Some(INVADER_COHORT));
            if inj.arm == Arm::Removed {
                assert_eq!(inj.residents_removed, record.resident.roles.count(inj.role));
            } else {
                assert_eq!(inj.residents_removed, 0);
            }
        }
        let a = serde_json::to_string(&record).unwrap();
        let b = serde_json::to_string(&run()).unwrap();
        assert_eq!(a, b, "record is deterministic");
    }

    /// The distribution-free interval on the median is the sign test's: at
    /// `n = 8` the narrowest ≥ 95 % interval is `[x_(1), x_(8)]`, so "does not
    /// straddle zero" means every seed agrees with the median's sign. Below
    /// `n = 6` no such interval exists.
    #[test]
    fn median_interval_rank_is_the_sign_tests() {
        assert_eq!(median_interval_rank(8), Some(1));
        assert_eq!(median_interval_rank(6), Some(1));
        assert_eq!(median_interval_rank(20), Some(6));
        assert_eq!(median_interval_rank(5), None);
        assert_eq!(median_interval_rank(0), None);
    }

    /// Exact two-sided 95 % Clopper–Pearson lower bound on the positive-seed
    /// fraction, pinned to the #434 table.
    #[test]
    fn clopper_pearson_lower_matches_the_434_table() {
        assert!((clopper_pearson_lower(8, 8) - 0.631).abs() < 2e-3);
        assert!((clopper_pearson_lower(7, 8) - 0.473).abs() < 2e-3);
        assert!((clopper_pearson_lower(4, 8) - 0.157).abs() < 2e-3);
        assert_eq!(clopper_pearson_lower(0, 8), 0.0);
    }

    #[test]
    fn rate_summary_reads_the_median_its_interval_and_the_positive_count() {
        let s = summarise_rates(&[0.01, 0.02, -0.005, 0.03, 0.015, 0.004, 0.02, 0.01]);
        assert_eq!(s.n, 8);
        assert_eq!(s.positive_seeds, 7);
        assert!((s.median - 0.0125).abs() < 1e-12);
        assert_eq!(s.interval, Some((-0.005, 0.03)));
        assert!(s.median_positive && !s.invades);
        let s = summarise_rates(&[0.01, 0.02, 0.005, 0.03, 0.015, 0.004, 0.02, 0.01]);
        assert_eq!(s.positive_seeds, 8);
        assert!(s.invades);
        let s = summarise_rates(&[-0.01, -0.02, -0.005, -0.03, -0.015, -0.004, -0.02, -0.01]);
        assert!(!s.median_positive && !s.invades);
        assert_eq!(s.positive_seeds, 0);
        let s = summarise_rates(&[0.01, 0.02]);
        assert_eq!(s.interval, None);
        assert!(!s.invades, "no interval at n = 2, so no verdict");
        assert!(summarise_rates(&[]).median.is_nan());
    }

    /// A role counts as present in the cell when it is present at `t_inj`
    /// on at least half the seeds that reached `t_inj`; the cell coexists by
    /// criterion when every present role invades.
    #[test]
    fn cell_verdict_requires_every_present_role_to_invade() {
        let present = |p: usize, c: usize, d: usize| RoleCounts {
            producers: p,
            consumers: c,
            decomposers: d,
        };
        // Four seeds: decomposer present on 2/4 (= half → present).
        let seeds: Vec<SeedRates> = (0..4)
            .map(|i| {
                let counts = present(10, 5, if i < 2 { 1 } else { 0 });
                let guilds = RoleGuilds {
                    producer: true,
                    consumer: true,
                    decomposer: i < 2,
                };
                let rates = vec![
                    (Role::Producer, Arm::Intact, 0.01),
                    (Role::Consumer, Arm::Intact, 0.02),
                    (
                        Role::Decomposer,
                        Arm::Intact,
                        if i == 0 { -0.01 } else { 0.01 },
                    ),
                    (Role::Producer, Arm::Removed, 0.01),
                    (Role::Consumer, Arm::Removed, 0.02),
                    (Role::Decomposer, Arm::Removed, 0.03),
                ];
                (counts, guilds, rates)
            })
            .collect();
        let verdicts = role_verdicts(&seeds, &[Arm::Intact, Arm::Removed]);
        let find = |r: Role, a: Arm| verdicts.iter().find(|v| v.role == r && v.arm == a).unwrap();
        assert!(find(Role::Producer, Arm::Intact).present);
        assert!(find(Role::Decomposer, Arm::Intact).present);
        assert_eq!(find(Role::Decomposer, Arm::Intact).guild_seeds, 2);
        // n = 4 has no interval, so the strict criterion cannot fire; the
        // median criterion can.
        let cell = cell_verdict(&verdicts, Arm::Intact);
        assert!(!cell.coexisting_strict);
        assert!(cell.coexisting_by_median);
        assert_eq!(cell.roles_present, 3);
        assert_eq!(cell.roles_invading_by_median, 3);
    }

    fn example10() -> (TraitVector, TraitVector, explorers_sim::WorldParameters) {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scenarios/example10_predator_prey_hopf.json"
        );
        let recipe: explorers_sim::WorldRecipe =
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

    /// The predicted signs are the boundary eigenvalues of A1/A2 read at the
    /// injected centroids: producer `λ_P(𝓥) − 1`, consumer `λ_C(K_P) − 1`
    /// (= sign of `I − 1`), decomposer `λ_H(𝓛) − 1` (= sign of `Λ − 1`).
    /// Pinned to the example10 numbers `permanence_crosscheck` pins.
    #[test]
    fn predicted_signs_reproduce_the_reduction_on_example10() {
        let (producer, consumer, params) = example10();
        let pred = predicted_signs(Some(&producer), Some(&consumer), Some(&consumer), &params);
        let by_role = |r: Role| pred.iter().find(|p| p.role == r).unwrap();
        let p = by_role(Role::Producer);
        assert!(
            (p.rate_coefficient - 1.3479).abs() < 1e-3,
            "r_P {}",
            p.rate_coefficient
        );
        assert!(p.eigen_excess > 0.0 && p.positive);
        let c = by_role(Role::Consumer);
        // #482: I = χ_C·γ·e·a·K_P/m with χ_C = 0.5012 at κ_C = 0.45 (pre-#482
        // the κ_C lumping gave I = 23.135, Λ = 81333.9).
        assert!(
            (c.rate_coefficient - 25.766).abs() < 1e-2,
            "I {}",
            c.rate_coefficient
        );
        assert!(c.positive);
        let d = by_role(Role::Decomposer);
        assert!(
            (d.rate_coefficient - 90583.8).abs() < 1.0,
            "Lambda {}",
            d.rate_coefficient
        );
        assert!(d.positive);

        // Flux at the floor: the producer cannot invade the virgin pool.
        let mut p = params.clone();
        p.solar_flux_magnitude = 0.1;
        let pred = predicted_signs(Some(&producer), Some(&consumer), Some(&consumer), &p);
        assert!(
            !pred
                .iter()
                .find(|p| p.role == Role::Producer)
                .unwrap()
                .positive
        );
        // No heterotrophy: neither heterotroph route is open.
        let mut c = consumer;
        c.heterotrophy = 0.0;
        let pred = predicted_signs(Some(&producer), Some(&c), Some(&c), &params);
        assert!(
            !pred
                .iter()
                .find(|p| p.role == Role::Consumer)
                .unwrap()
                .positive
        );
        assert!(
            !pred
                .iter()
                .find(|p| p.role == Role::Decomposer)
                .unwrap()
                .positive
        );
        // κ_C = 0 routes the consumer's surplus through offspring, which are
        // biomass: β and Λ stay positive (#482), so both routes stay open.
        let mut c = consumer;
        c.kappa = 0.0;
        let pred = predicted_signs(Some(&producer), Some(&c), Some(&c), &params);
        let cc = pred.iter().find(|p| p.role == Role::Consumer).unwrap();
        assert!(
            cc.positive && cc.rate_coefficient > 1.0,
            "I {}",
            cc.rate_coefficient
        );
        let dd = pred.iter().find(|p| p.role == Role::Decomposer).unwrap();
        assert!(
            dd.positive && dd.rate_coefficient > 1.0,
            "Lambda {}",
            dd.rate_coefficient
        );
    }

    /// Predicted vs observed sign per (role, arm): the reduction's majority
    /// sign over seeds against the criterion's median sign; an absent role
    /// is not compared.
    #[test]
    fn sign_agreement_reads_the_majority_sign_against_the_median() {
        let row = |predicted_positive: usize, n: usize, observed: f64, present: bool| {
            sign_agreement(predicted_positive, n, observed, present)
        };
        assert_eq!(row(8, 8, 0.01, true), Agreement::Agree);
        assert_eq!(row(0, 8, -0.01, true), Agreement::Agree);
        assert_eq!(
            row(8, 8, -0.01, true),
            Agreement::PredictedPositiveObservedNegative
        );
        assert_eq!(
            row(0, 8, 0.01, true),
            Agreement::PredictedNegativeObservedPositive
        );
        assert_eq!(row(8, 8, 0.01, false), Agreement::Absent);
        assert_eq!(row(4, 8, 0.01, true), Agreement::PredictedSplit);
    }

    /// Smoke check on a whole cell record: no assertion on emergent values.
    #[test]
    fn evaluate_cell_produces_verdicts_a_sign_table_and_serialises() {
        let ranges = default_ranges();
        let cell = AtlasCellIn {
            cell: [0, 0, 0],
            fitness: 0.5,
            coexistence_fraction: 0.8,
            sample_count: 5,
            decomposer_fraction: 0.0,
            unit: vec![0.5; ranges.len()],
        };
        let record = evaluate_cell(
            ConfigSource::Atlas,
            0,
            &cell.unit,
            Some(cell.meta()),
            2,
            20,
            20,
            &[Arm::Intact, Arm::Removed],
        );
        assert_eq!(record.seeds.len(), 2);
        assert_eq!(record.role_verdicts.len(), 6);
        assert_eq!(record.sign_table.len(), 6);
        assert_eq!(record.displacement.len(), 6);
        assert_eq!(record.verdicts.len(), 2);
        assert!(record.atlas_coexisting);
        let json = serde_json::to_string(&record).expect("record serialises");
        assert!(json.contains("lambda_C(K_P)"));
        let summary = arm_summary(std::slice::from_ref(&record), Arm::Intact);
        assert_eq!(summary.cells, 1);
    }

    /// Founders 10 and 11 are injected into a resident world of 0..3.
    /// Descent: 10 → 20 (asexual); 11 × resident 2 → 21 (cross); 20 → 22
    /// (grandchild); resident 1 → 23 (not lineage). 10 then dies.
    #[test]
    fn lineage_follows_descent_through_either_parent_and_counts_the_living() {
        let mut lineage = Lineage::new([10, 11]);
        lineage.absorb(&[
            born(0, 20, 10, None),
            born(1, 21, 2, Some(11)),
            born(2, 23, 1, None),
        ]);
        lineage.absorb(&[born(3, 22, 20, Some(23)), died(4, 10)]);
        assert!(lineage.is_member(20) && lineage.is_member(21) && lineage.is_member(22));
        assert!(!lineage.is_member(23) && !lineage.is_member(1));
        assert_eq!(lineage.pure_births, 1);
        assert_eq!(lineage.cross_births, 2);
        // 10 is dead (off the roster); 11, 20, 21, 22 alive; 23 and residents not counted.
        let alive = lineage.alive(&roster(&[0, 1, 2, 3, 11, 20, 21, 22, 23]));
        assert_eq!(alive, 4);
    }

    fn consumed(seq: u64, eater: u64, target: u64, carcass: bool) -> Event {
        Event {
            tick: 3,
            seq,
            kind: EventKind::Consumed,
            source: eater,
            target: Some(target),
            energy_delta: 0.0,
            position: None,
            target_was_carcass: carcass,
            second_parent: None,
        }
    }

    /// `INVASION_GROWTH_CELLS` selects from both sources (and any LHS draw,
    /// `sample@S:i`); unset runs the
    /// atlas (the full run) and never the LHS draw.
    #[test]
    fn selector_picks_atlas_and_sample_configs_and_defaults_to_the_atlas() {
        let filter = parse_cell_filter_from("atlas:1,sample@9421:2,sample:55,0");
        let tasks = select_tasks(3, 60, Some(&filter));
        assert_eq!(
            tasks,
            vec![
                (ConfigSource::Atlas, 0),
                (ConfigSource::Atlas, 1),
                (ConfigSource::SAMPLE, 55),
                (ConfigSource::Sample(9421), 2)
            ]
        );
        let all = select_tasks(3, 60, None);
        assert_eq!(
            all,
            (0..3).map(|i| (ConfigSource::Atlas, i)).collect::<Vec<_>>()
        );
    }

    /// The lineage's diet is read off `Consumed` events whose source is a
    /// member, split by whether the target was a carcass; residents eating
    /// (even eating a lineage member) do not count.
    #[test]
    fn lineage_counts_its_members_consumption_split_by_carcass() {
        let mut lineage = Lineage::new([10, 11]);
        lineage.absorb(&[
            consumed(0, 10, 1, false),
            consumed(1, 11, 2, true),
            consumed(2, 10, 3, true),
            consumed(3, 1, 10, false),
        ]);
        assert_eq!(lineage.consumed.living, 1);
        assert_eq!(lineage.consumed.carcass, 2);
        // Event 3 is a resident (1) draining member 10 alive.
        assert_eq!(lineage.preyed_upon_events, 1);
    }

    /// A member's death is attributed to the drain if it was drained alive in
    /// the tick it died, otherwise to starvation / the structure threshold.
    #[test]
    fn lineage_splits_deaths_by_whether_the_member_was_drained_that_tick() {
        let died = |tick: u64, seq: u64, id: u64| Event {
            tick,
            seq,
            kind: EventKind::Died,
            source: id,
            target: None,
            energy_delta: 0.0,
            position: None,
            target_was_carcass: false,
            second_parent: None,
        };
        let mut lineage = Lineage::new([10, 11, 12]);
        // 10 is drained on tick 3 and dies on tick 3; 11 is drained on tick 3
        // and dies later (tick 7) undrained; 12 is never drained.
        lineage.absorb(&[consumed(0, 1, 10, false), consumed(1, 1, 11, false)]);
        lineage.absorb(&[died(3, 2, 10)]);
        lineage.absorb(&[died(7, 0, 11), died(7, 1, 12)]);
        assert_eq!(lineage.preyed_upon_events, 2);
        assert_eq!(lineage.deaths_drained, 1);
        assert_eq!(lineage.deaths_undrained, 2);
        // A non-member's death is not counted.
        lineage.absorb(&[died(8, 0, 99)]);
        assert_eq!(lineage.deaths_drained + lineage.deaths_undrained, 3);
    }

    /// An absent role is not injected at an invented phenotype: it is
    /// recorded as `not_testable` (`role_absent`) and every injection is at
    /// a realised centroid of a role present in the resident at `t_inj`.
    #[test]
    fn absent_role_is_not_testable_rather_than_injected_at_a_canonical_vertex() {
        let ranges = default_ranges();
        let unit = vec![0.5; ranges.len()];
        // At t_inj = 1 no heterotroph has eaten a carcass, so the decomposer
        // role is absent by construction.
        let record = run_cell_seed(&unit, SEED_BASE, 1, 10, &[Arm::Intact, Arm::Removed]);
        assert_eq!(record.resident.termination, "alive");
        assert_eq!(record.resident.roles.decomposers, 0);
        let absent: Vec<Role> = record
            .not_testable
            .iter()
            .filter(|n| n.reason == "role_absent")
            .map(|n| n.role)
            .collect();
        assert!(absent.contains(&Role::Decomposer));
        for role in ROLES {
            let present = record.resident.roles.count(role) > 0;
            assert_eq!(absent.contains(&role), !present, "{role:?}");
            if !present {
                assert!(record.injections.iter().all(|i| i.role != role));
            }
        }
        assert!(
            record
                .injections
                .iter()
                .all(|i| i.centroid_source == "realised")
        );
        assert!(record.centroids.iter().all(|c| c.source == "realised"));
        // The reduction's sign is read only where its centroids exist.
        for p in &record.predicted {
            assert!(
                !absent.contains(&p.role),
                "{:?} predicted while absent",
                p.role
            );
        }
    }

    /// #493: a role that is *tagged* on the resident but has no guild over the
    /// resident phase is `not_testable: no_guild` and is not injected. At
    /// `t_inj = 1` the producers are all there (tagged) but nothing has bred
    /// inside the window, so the guild predicate cannot hold.
    #[test]
    fn tagged_role_without_a_guild_is_not_testable_no_guild() {
        let ranges = default_ranges();
        let unit = vec![0.5; ranges.len()];
        let record = run_cell_seed(&unit, SEED_BASE, 1, 10, &[Arm::Intact, Arm::Removed]);
        assert_eq!(record.resident.termination, "alive");
        assert!(record.resident.roles.producers > 0);
        assert!(!record.resident.guilds.producer);
        let producer = record
            .not_testable
            .iter()
            .find(|n| n.role == Role::Producer)
            .expect("producer not testable");
        assert_eq!(producer.reason, "no_guild");
        assert!(record.injections.is_empty());
        // The realised centroid is still recorded — it is the tag, not the
        // guild, that defines it.
        assert!(record.centroids.iter().any(|c| c.role == Role::Producer));
        // Absent roles keep their own reason.
        let decomposer = record
            .not_testable
            .iter()
            .find(|n| n.role == Role::Decomposer)
            .unwrap();
        assert_eq!(decomposer.reason, "role_absent");
    }

    /// #493: cell presence is the guild read on ≥ half the seeds; the role-tag
    /// count is kept alongside.
    #[test]
    fn cell_presence_is_the_guild_read_and_keeps_the_tag_count() {
        let counts = RoleCounts {
            producers: 10,
            consumers: 5,
            decomposers: 1,
        };
        // Four seeds: consumer tagged on 4/4 but a guild on 1/4; producer a
        // guild on 4/4; decomposer tagged on 4/4, never a guild.
        let seeds: Vec<SeedRates> = (0..4)
            .map(|i| {
                let guilds = RoleGuilds {
                    producer: true,
                    consumer: i == 0,
                    decomposer: false,
                };
                let mut rates = vec![(Role::Producer, Arm::Intact, 0.01)];
                if i == 0 {
                    rates.push((Role::Consumer, Arm::Intact, 0.02));
                }
                (counts, guilds, rates)
            })
            .collect();
        let verdicts = role_verdicts(&seeds, &[Arm::Intact]);
        let find = |r: Role| verdicts.iter().find(|v| v.role == r).unwrap();
        assert!(find(Role::Producer).present);
        assert_eq!(find(Role::Producer).guild_seeds, 4);
        assert_eq!(find(Role::Producer).role_tagged_seeds, 4);
        assert!(!find(Role::Consumer).present);
        assert_eq!(find(Role::Consumer).guild_seeds, 1);
        assert_eq!(find(Role::Consumer).role_tagged_seeds, 4);
        assert!(!find(Role::Decomposer).present);
        assert_eq!(find(Role::Decomposer).role_tagged_seeds, 4);
        let cell = cell_verdict(&verdicts, Arm::Intact);
        assert_eq!(cell.roles_present, 1);
    }
}
