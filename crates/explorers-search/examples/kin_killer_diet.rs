//! Throwaway diagnostic (after #637): what do nutrient-hungry, light-fed
//! mixotrophs eat apart from their kin, and could they reach anything else?
//!
//! The rollout mirrors `role_diet::rollout_with_fullness` step for step (the
//! same retention, observations, early stops and ledger), observer-only.
//! The population is the **light-fed mixotrophs**: agents with heterotrophy
//! above zero whose recent-income role (the evaluator's
//! `observations.income().role`) is `Producer` as of the start of the tick —
//! the read the #634/#637 kin-kill classification uses.
//!
//! 1. **Diet.** Every `Consumed` event of theirs, bucketed by target: kin
//!    (`DietLedger::is_kin`), non-kin living by the target's recent-income role
//!    at the start of the tick (producer / consumer / decomposer / no income
//!    yet), or carcass. Per bucket: bites, drained structure, energy gained
//!    and nutrient bound / retained (`intake_ceiling::realised_bites`, the
//!    per-bite form of `realised_drains`), and the target's nutrient
//!    concentration at drain time (per bite and per body). Split by the
//!    consumer's fullness state at #637's closest decoded cell (k = 1.7,
//!    τ ≈ 17.2, n = 4), from the fullness carried into the tick.
//! 2. **Reach at each kin kill** (one per kin-killing pair, as #637 counts
//!    them): living agents and carcasses inside the killer's
//!    `phase::consumption_reach` at drain time, and how many are kin.
//! 3. **Mobility / dispersal** traits of the kin killers against the light-fed
//!    mixotrophs that never killed kin.
//!
//! A–D (`Tally::ext`) ask whether light-fed mixotrophy is a conditional
//! strategy or a bypass of decomposition: A. prevalence and effective
//! heterotrophy by recent-income role on second-half sample ticks; B.
//! Producer-role samples binned by effective heterotrophy (births, earmark
//! fill, nutrient limitation, pool at the cell, income by route, lifespan);
//! C. carcass processing and nutrient into living agents by route and
//! recipient; D. Producer heterotrophy by quarter of the run.
//!
//!   cargo run --release -p explorers-search --example kin_killer_diet -- \
//!       --atlas atlas.json --configs atlas:0,atlas:1 --out target/kkd/decoded.jsonl
//!   cargo run --release -p explorers-search --example kin_killer_diet -- \
//!       --summary --out target/kkd/decoded.jsonl
//!
//! `--crosscheck` also runs `role_diet::rollout_with_fullness` per seed and
//! prints its `fullness.kills` beside this tool's kin-kill count.
//!
//! #645 reruns it with size-scaled uptake (#644): `--uptake-structure-exponent
//! <b>` and `--uptake-reference-structure <s_ref>` pin those parameters on
//! every decoded config (unset, each keeps its decoded value: b = 0, s_ref =
//! 100). E. reads whether the worlds still persist: the census's verdict per
//! seed (`role_diet::census_failure`), per config and pooled, and the kin
//! kills per Producer-role agent-tick.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use rayon::prelude::*;

use explorers_genesis_eval::{EvalConfig, RolloutObservations};
use explorers_search::config_source::{
    ConfigSource, parse_founder_aggregation, parse_non_negative, parse_positive, parse_selector,
    resolve_config, sampled_units, with_consumption_scales, with_founder_aggregation,
    with_uptake_scaling,
};
use explorers_search::fullness::{FullnessBank, FullnessTracker, k_grid, tau_grid, tick_intakes};
use explorers_search::grazer_hunger::PreStep;
use explorers_search::intake_ceiling::realised_bites;
use explorers_search::role_diet::{DietLedger, Outcomes, census_failure, rollout_with_fullness};
use explorers_search::search::default_ranges;
use explorers_search::sweep::{append_row, done_configs, plan_tasks, read_atlas_units, read_rows};
use explorers_sim::event::{Event, EventKind};
use explorers_sim::topology::TrophicRole;
use explorers_sim::{InitialDistribution, World, WorldParameters, phase, toroidal_distance};

const BUCKETS: [&str; 6] = [
    "kin, living",
    "non-kin producer",
    "non-kin consumer",
    "non-kin decomposer",
    "non-kin, no income yet",
    "carcass",
];
const KIN: usize = 0;
const CARCASS: usize = 5;
const STATES: [&str; 4] = [
    "(i) E-sated, N-hungry",
    "(ii) E-hungry",
    "(iii) sated both",
    "no reading",
];
const TARGET_K: f32 = 1.7;
const TARGET_TAU: f32 = 17.2;
const N: f32 = 4.0;
const FRAC_BINS: usize = 11;

/// The closest grid points to #637's closest decoded cell.
fn cell() -> (f32, usize) {
    let k = k_grid()
        .into_iter()
        .min_by(|a, b| (a - TARGET_K).abs().total_cmp(&(b - TARGET_K).abs()))
        .unwrap();
    let tau = tau_grid();
    let j = (0..tau.len())
        .min_by(|&a, &b| {
            (tau[a] - TARGET_TAU)
                .abs()
                .total_cmp(&(tau[b] - TARGET_TAU).abs())
        })
        .unwrap();
    (k, j)
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct Cell {
    bites: u64,
    /// Structure drained (`Consumed.energy_delta`).
    drained: f64,
    /// Energy gained (drained × transfer efficiency).
    energy: f64,
    bound_nutrient: f64,
    retained_nutrient: f64,
    /// Σ nutrient bound per unit structure drained, over bites that drained.
    conc_bite: f64,
    conc_bite_n: u64,
    /// Σ the target's whole nutrient per structure (living: total nutrient
    /// / structure; carcass: nutrient / energy, while it has energy).
    conc_body: f64,
    conc_body_n: u64,
    /// Σ body concentration × structure drained, and Σ that structure: the
    /// drain-weighted body concentration (a near-spent carcass's ratio can
    /// be huge, which swamps the plain mean).
    #[serde(default)]
    conc_body_w: f64,
    #[serde(default)]
    conc_body_w_drained: f64,
}

impl Cell {
    fn merge(&mut self, o: &Cell) {
        self.bites += o.bites;
        self.drained += o.drained;
        self.energy += o.energy;
        self.bound_nutrient += o.bound_nutrient;
        self.retained_nutrient += o.retained_nutrient;
        self.conc_bite += o.conc_bite;
        self.conc_bite_n += o.conc_bite_n;
        self.conc_body += o.conc_body;
        self.conc_body_n += o.conc_body_n;
        self.conc_body_w += o.conc_body_w;
        self.conc_body_w_drained += o.conc_body_w_drained;
    }
}

/// `cells[bucket][state]`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Diet {
    cells: Vec<Vec<Cell>>,
}

impl Default for Diet {
    fn default() -> Self {
        Self {
            cells: vec![vec![Cell::default(); STATES.len()]; BUCKETS.len()],
        }
    }
}

impl Diet {
    fn merge(&mut self, o: &Diet) {
        for (a, b) in self.cells.iter_mut().zip(&o.cells) {
            for (x, y) in a.iter_mut().zip(b) {
                x.merge(y);
            }
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Reach {
    kills: u64,
    /// Nothing but (living) kin in reach.
    only_kin: u64,
    /// Some non-kin living agent or carcass in reach.
    other_in_reach: u64,
    nonkin_living_any: u64,
    carcass_any: u64,
    /// Kin share of everything in reach, bins `floor(10 f)`; bin 10 is
    /// exactly 1.
    kin_frac: Vec<u64>,
    living_kin: u64,
    living_nonkin: u64,
    /// Non-kin living in reach by recent-income role: producer, consumer,
    /// decomposer, none.
    nonkin_by_role: Vec<u64>,
    carcasses: u64,
    /// Carcasses whose agent was kin of the killer.
    carcasses_kin: u64,
}

impl Default for Reach {
    fn default() -> Self {
        Self {
            kills: 0,
            only_kin: 0,
            other_in_reach: 0,
            nonkin_living_any: 0,
            carcass_any: 0,
            kin_frac: vec![0; FRAC_BINS],
            living_kin: 0,
            living_nonkin: 0,
            nonkin_by_role: vec![0; 4],
            carcasses: 0,
            carcasses_kin: 0,
        }
    }
}

impl Reach {
    fn merge(&mut self, o: &Reach) {
        self.kills += o.kills;
        self.only_kin += o.only_kin;
        self.other_in_reach += o.other_in_reach;
        self.nonkin_living_any += o.nonkin_living_any;
        self.carcass_any += o.carcass_any;
        for (a, b) in self.kin_frac.iter_mut().zip(&o.kin_frac) {
            *a += b;
        }
        self.living_kin += o.living_kin;
        self.living_nonkin += o.living_nonkin;
        for (a, b) in self.nonkin_by_role.iter_mut().zip(&o.nonkin_by_role) {
            *a += b;
        }
        self.carcasses += o.carcasses;
        self.carcasses_kin += o.carcasses_kin;
    }
}

/// Raw per-agent trait values (traits are unbounded above), pooled by
/// concatenation; nearest-rank percentiles.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct Hist {
    values: Vec<f32>,
}

impl Hist {
    fn record(&mut self, v: f32) {
        self.values.push(v);
    }

    fn merge(&mut self, o: &Hist) {
        self.values.extend_from_slice(&o.values);
    }

    fn percentile(&self, q: f64) -> Option<f64> {
        if self.values.is_empty() {
            return None;
        }
        let mut v = self.values.clone();
        v.sort_by(f32::total_cmp);
        Some(f64::from(v[(q * (v.len() - 1) as f64).round() as usize]))
    }
}

/// Effective-heterotrophy histogram: [`EH_BINS`] bins of width [`EH_WIDTH`]
/// from 0, the last also holding everything above; exact count, sum and
/// shares above [`ABOVE`]. Percentiles read at the bin midpoint.
const EH_BINS: usize = 501;
const EH_WIDTH: f64 = 0.01;
const ABOVE: [f32; 3] = [0.05, 0.2, 0.5];

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct EffHist {
    n: u64,
    sum: f64,
    above: [u64; 3],
    bins: Vec<u64>,
}

impl EffHist {
    fn record(&mut self, v: f32) {
        if self.bins.is_empty() {
            self.bins = vec![0; EH_BINS];
        }
        self.n += 1;
        self.sum += f64::from(v);
        for (c, t) in self.above.iter_mut().zip(ABOVE) {
            *c += u64::from(v > t);
        }
        let b = ((f64::from(v.max(0.0)) / EH_WIDTH) as usize).min(EH_BINS - 1);
        self.bins[b] += 1;
    }

    fn merge(&mut self, o: &EffHist) {
        if o.bins.is_empty() {
            return;
        }
        if self.bins.is_empty() {
            self.bins = vec![0; EH_BINS];
        }
        self.n += o.n;
        self.sum += o.sum;
        for (a, b) in self.above.iter_mut().zip(o.above) {
            *a += b;
        }
        for (a, b) in self.bins.iter_mut().zip(&o.bins) {
            *a += b;
        }
    }

    fn mean(&self) -> Option<f64> {
        (self.n > 0).then(|| self.sum / self.n as f64)
    }

    /// Nearest-rank percentile, at its bin's midpoint.
    fn percentile(&self, q: f64) -> Option<f64> {
        if self.n == 0 {
            return None;
        }
        let rank = (q * (self.n - 1) as f64).round() as u64;
        let mut seen = 0;
        for (i, &c) in self.bins.iter().enumerate() {
            seen += c;
            if seen > rank {
                return Some((i as f64 + 0.5) * EH_WIDTH);
            }
        }
        None
    }
}

/// Recent-income roles in A's order: Producer, Consumer, Decomposer, none.
const ROLES: [&str; 4] = ["Producer", "Consumer", "Decomposer", "none"];

/// Section B's effective-heterotrophy bins among Producer-role agents.
const HET_BINS: [&str; 5] = ["[0, 0.05)", "[0.05, 0.2)", "[0.2, 0.5)", "[0.5, 1)", "≥ 1"];

fn het_bin(h: f32) -> usize {
    if h < 0.05 {
        0
    } else if h < 0.2 {
        1
    } else if h < 0.5 {
        2
    } else if h < 1.0 {
        3
    } else {
        4
    }
}

/// One B bin's tallies over Producer-role agent-samples (second half).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct ProducerBin {
    samples: u64,
    /// Producer-role agents at the start of every second-half tick (pre-step
    /// effective heterotrophy): the denominator births are counted against.
    agent_ticks: u64,
    /// Second-half births whose parent was Producer-role at the start of the
    /// tick, binned by the parent's pre-step effective heterotrophy.
    births: u64,
    earmark_fill: f64,
    nutrient_limited: u64,
    pool: f64,
    light: f64,
    uptake: f64,
    living_energy: f64,
    living_retained: f64,
    carcass_energy: f64,
    carcass_retained: f64,
    /// Samples with no pre-step / drain-time reading (should be zero).
    missing_pre: u64,
    /// Second-half deaths of agents whose last role sample was Producer in
    /// this bin, and Σ their age at death (ticks since birth, founders from 0).
    deaths: u64,
    death_age: f64,
}

impl ProducerBin {
    fn merge(&mut self, o: &ProducerBin) {
        self.samples += o.samples;
        self.agent_ticks += o.agent_ticks;
        self.births += o.births;
        self.earmark_fill += o.earmark_fill;
        self.nutrient_limited += o.nutrient_limited;
        self.pool += o.pool;
        self.light += o.light;
        self.uptake += o.uptake;
        self.living_energy += o.living_energy;
        self.living_retained += o.living_retained;
        self.carcass_energy += o.carcass_energy;
        self.carcass_retained += o.carcass_retained;
        self.missing_pre += o.missing_pre;
        self.deaths += o.deaths;
        self.death_age += o.death_age;
    }
}

/// Recipient buckets for section C: light-fed mixotroph (Producer, effective
/// heterotrophy > 0.05), pure producer (Producer, ≤ 0.05), consumer,
/// decomposer, no income yet; and a memo row overlapping the first two —
/// Producer with raw heterotrophy > 0, section 1's population.
const RECIPIENTS: [&str; 6] = [
    "light-fed mixotroph (P, h_eff > 0.05)",
    "pure producer (P, h_eff ≤ 0.05)",
    "consumer",
    "decomposer",
    "no income yet",
    "memo: P with raw h > 0 (section 1's population)",
];
const MEMO: usize = 5;

fn recipient(role: Option<TrophicRole>, eff: f32) -> usize {
    match role {
        Some(TrophicRole::Producer) if eff > 0.05 => 0,
        Some(TrophicRole::Producer) => 1,
        Some(TrophicRole::Consumer) => 2,
        Some(TrophicRole::Decomposer) => 3,
        None => 4,
    }
}

/// Section C: nutrient (and drain energy) entering living agents by route,
/// per recipient bucket.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Routes {
    uptake: Vec<f64>,
    living_energy: Vec<f64>,
    living_retained: Vec<f64>,
    carcass_bites: Vec<u64>,
    carcass_drained: Vec<f64>,
    carcass_energy: Vec<f64>,
    carcass_bound: Vec<f64>,
    carcass_retained: Vec<f64>,
}

impl Default for Routes {
    fn default() -> Self {
        let z = vec![0.0; RECIPIENTS.len()];
        Self {
            uptake: z.clone(),
            living_energy: z.clone(),
            living_retained: z.clone(),
            carcass_bites: vec![0; RECIPIENTS.len()],
            carcass_drained: z.clone(),
            carcass_energy: z.clone(),
            carcass_bound: z.clone(),
            carcass_retained: z,
        }
    }
}

impl Routes {
    fn merge(&mut self, o: &Routes) {
        let add = |a: &mut Vec<f64>, b: &Vec<f64>| {
            for (x, y) in a.iter_mut().zip(b) {
                *x += y;
            }
        };
        add(&mut self.uptake, &o.uptake);
        add(&mut self.living_energy, &o.living_energy);
        add(&mut self.living_retained, &o.living_retained);
        for (x, y) in self.carcass_bites.iter_mut().zip(&o.carcass_bites) {
            *x += y;
        }
        add(&mut self.carcass_drained, &o.carcass_drained);
        add(&mut self.carcass_energy, &o.carcass_energy);
        add(&mut self.carcass_bound, &o.carcass_bound);
        add(&mut self.carcass_retained, &o.carcass_retained);
    }
}

/// D's populations, on sample ticks over the whole run.
const D_POPS: [&str; 3] = [
    "P with raw h > 0 (section 1's)",
    "P with h_eff > 0.05 (A's light-fed mixotroph)",
    "all Producer-role",
];

/// Sections A–D (added after the first full-atlas pass).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct Ext {
    /// A: per recent-income role ([`ROLES`]), second-half sample ticks.
    roles: Vec<EffHist>,
    /// A: Producer-role samples with raw heterotrophy > 0.
    producer_raw_het: u64,
    /// B: per [`HET_BINS`].
    producer_bins: Vec<ProducerBin>,
    /// C: whole run and second half.
    routes: Routes,
    routes_second_half: Routes,
    /// C: every `Consumed` carcass event, before `realised_bites` drops
    /// consumers off the drain-time roster.
    carcass_events: u64,
    carcass_events_second_half: u64,
    /// Seeds whose config has the network enabled (connection cap > 0).
    network_enabled_seeds: u64,
    /// D: `quarters[q][pop]` over [`D_POPS`].
    quarters: Vec<Vec<EffHist>>,
}

impl Ext {
    fn new() -> Self {
        Self {
            roles: vec![EffHist::default(); ROLES.len()],
            producer_bins: vec![ProducerBin::default(); HET_BINS.len()],
            quarters: vec![vec![EffHist::default(); D_POPS.len()]; 4],
            ..Default::default()
        }
    }

    fn merge(&mut self, o: &Ext) {
        if self.roles.is_empty() {
            *self = Ext::new();
        }
        if o.roles.is_empty() {
            return;
        }
        for (a, b) in self.roles.iter_mut().zip(&o.roles) {
            a.merge(b);
        }
        self.producer_raw_het += o.producer_raw_het;
        for (a, b) in self.producer_bins.iter_mut().zip(&o.producer_bins) {
            a.merge(b);
        }
        self.routes.merge(&o.routes);
        self.routes_second_half.merge(&o.routes_second_half);
        self.carcass_events += o.carcass_events;
        self.carcass_events_second_half += o.carcass_events_second_half;
        self.network_enabled_seeds += o.network_enabled_seeds;
        for (a, b) in self.quarters.iter_mut().zip(&o.quarters) {
            for (x, y) in a.iter_mut().zip(b) {
                x.merge(y);
            }
        }
    }
}

/// One seed's (or, summed, one config's) tallies.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct Tally {
    /// Kin kills by a light-fed mixotroph (one per pair), as #637 counts.
    kin_kills: u64,
    diet: Diet,
    diet_second_half: Diet,
    /// Per fullness state.
    reach: Vec<Reach>,
    reach_second_half: Vec<Reach>,
    killers: u64,
    non_killers: u64,
    killer_mobility: Hist,
    killer_dispersal: Hist,
    non_killer_mobility: Hist,
    non_killer_dispersal: Hist,
    /// Sections A–D.
    #[serde(default)]
    ext: Ext,
    /// E: Producer-role agents (recent-income role at the start of the
    /// tick, the kin-kill classification's read) summed over every tick.
    #[serde(default)]
    producer_agent_ticks: u64,
    /// E: the census's verdict per seed, and Σ termination tick.
    #[serde(default)]
    outcomes: Outcomes,
    #[serde(default)]
    termination_ticks: u64,
}

impl Tally {
    fn new() -> Self {
        Self {
            reach: vec![Reach::default(); STATES.len()],
            reach_second_half: vec![Reach::default(); STATES.len()],
            ext: Ext::new(),
            ..Default::default()
        }
    }

    fn merge(&mut self, o: &Tally) {
        if self.reach.is_empty() {
            *self = Tally::new();
        }
        self.kin_kills += o.kin_kills;
        self.diet.merge(&o.diet);
        self.diet_second_half.merge(&o.diet_second_half);
        for (a, b) in self.reach.iter_mut().zip(&o.reach) {
            a.merge(b);
        }
        for (a, b) in self.reach_second_half.iter_mut().zip(&o.reach_second_half) {
            a.merge(b);
        }
        self.killers += o.killers;
        self.non_killers += o.non_killers;
        self.killer_mobility.merge(&o.killer_mobility);
        self.killer_dispersal.merge(&o.killer_dispersal);
        self.non_killer_mobility.merge(&o.non_killer_mobility);
        self.non_killer_dispersal.merge(&o.non_killer_dispersal);
        self.ext.merge(&o.ext);
        self.producer_agent_ticks += o.producer_agent_ticks;
        self.outcomes.merge(&o.outcomes);
        self.termination_ticks += o.termination_ticks;
    }
}

fn role_slot(role: Option<TrophicRole>) -> usize {
    match role {
        Some(TrophicRole::Producer) => 0,
        Some(TrophicRole::Consumer) => 1,
        Some(TrophicRole::Decomposer) => 2,
        None => 3,
    }
}

/// The consumer's fullness state at the cell, from the fullness carried
/// into the tick (a founder new to the tracker starts at its first tick's
/// light and uptake, as `FullnessTracker::observe` seeds it).
fn state(
    tracker: &FullnessTracker,
    intakes: &HashMap<u64, explorers_search::fullness::TickIntake>,
    id: u64,
    k: f32,
    j: usize,
) -> usize {
    let Some(i) = intakes.get(&id) else {
        return 3;
    };
    let bank = tracker
        .bank(id)
        .copied()
        .unwrap_or_else(|| FullnessBank::at(i.light, i.uptake));
    let e = bank.expression(j, k, N, i);
    if e.energy >= 0.5 {
        1
    } else if e.co_limited >= 0.5 {
        // E_E < 0.5 and max(E_E, E_N) ≥ 0.5: the nutrient side is hungry.
        0
    } else {
        2
    }
}

/// One seed, mirroring `role_diet::rollout_with_fullness`'s loop.
fn rollout(
    params: &WorldParameters,
    dist: &InitialDistribution,
    seed: u64,
    max_ticks: u64,
    eval_config: &EvalConfig,
) -> Tally {
    let (cell_k, cell_j) = cell();
    let mut tally = Tally::new();
    let mut tracker = FullnessTracker::new();
    let mut world = World::new(params.clone(), dist.clone(), seed);
    let mut kinds = explorers_genesis_eval::EVALUATOR_EVENT_KINDS.to_vec();
    for k in [
        EventKind::Photosynthesized,
        EventKind::Born,
        EventKind::Died,
    ] {
        if !kinds.contains(&k) {
            kinds.push(k);
        }
    }
    world.retain_event_kinds(&kinds);
    let mut observations = RolloutObservations::with_capacity(max_ticks as usize);
    let interval = eval_config.coexistence_sample_interval;
    let sustainable_stock = explorers_genesis_eval::sustainable_stock(params);
    let window_start = max_ticks / 2 + 1;
    let steep = params.wear_degradation_steepness;

    let mut ledger = DietLedger::new(&world).with_kin_kill_readings();
    let mut cursor = 0;
    // Every light-fed mixotroph seen, with (mobility, dispersal); the kin
    // killers among them.
    let mut lfm_ever: HashMap<u64, (f32, f32)> = HashMap::new();
    let mut killers: HashSet<u64> = HashSet::new();
    tally.ext.network_enabled_seeds = u64::from(params.network_connection_cap > 0);
    // Lifespan bookkeeping: birth tick per agent (founders 0), and each
    // agent's B bin at its last second-half role sample (absent when that
    // sample's role was not Producer).
    let mut birth_tick: HashMap<u64, u64> = HashMap::new();
    let mut last_bin: HashMap<u64, usize> = HashMap::new();
    let mut stopped = None;
    for _ in 0..max_ticks {
        let tick_before = world.tick();
        let second_half_tick = tick_before + 1 >= window_start;
        // Pre-step reads for B: effective heterotrophy and the pool cell
        // uptake will read (photosynthesis leaves the grid untouched).
        let (pre_eff, pool_at): (HashMap<u64, f32>, HashMap<u64, f32>) = if second_half_tick {
            let grid = world.nutrient_grid();
            world
                .agents()
                .iter()
                .map(|a| {
                    (
                        (a.id, a.effective_trait_with_steepness(1, steep)),
                        (a.id, grid.cells()[grid.cell_index_for(a.position)]),
                    )
                })
                .unzip()
        } else {
            Default::default()
        };
        // Start-of-tick reads: the income ledger has not walked this tick.
        let income = observations.income();
        let lfm: HashSet<u64> = world
            .agents()
            .iter()
            .filter(|a| {
                a.traits.heterotrophy > 0.0 && income.role(a.id) == Some(TrophicRole::Producer)
            })
            .map(|a| {
                lfm_ever.insert(a.id, (a.traits.mobility, a.traits.dispersal));
                a.id
            })
            .collect();
        let roles: HashMap<u64, Option<TrophicRole>> = world
            .agents()
            .iter()
            .map(|a| (a.id, income.role(a.id)))
            .collect();
        tally.producer_agent_ticks += roles
            .values()
            .filter(|r| **r == Some(TrophicRole::Producer))
            .count() as u64;
        let pre = PreStep::capture(&world);
        world.step();
        let tail: Vec<Event> = world.event_log().since(cursor).to_vec();
        cursor = world.event_log().len();
        ledger.ingest(&world, &tail, Some(&pre));
        let mut kin_killers = Vec::new();
        for (g, heterotrophy, _) in ledger.take_kin_kill_readings() {
            if heterotrophy > 0.0 && observations.income().role(g) == Some(TrophicRole::Producer) {
                kin_killers.push(g);
            }
        }
        let tick = world.tick();
        let second_half = tick >= window_start;
        let intakes = tick_intakes(&pre, params, &tail);
        let start = pre.drain_start(params);
        let bites = realised_bites(&start.agents, pre.carcasses(), params, &tail);
        // C. Routes into living agents, by recipient bucket at the start of
        // the tick (drain-time effective heterotrophy, as the drain reads it).
        let drain_eff: HashMap<u64, (f32, f32)> = start
            .agents
            .iter()
            .map(|a| {
                (
                    a.id,
                    (
                        a.effective_trait_with_steepness(1, steep),
                        a.traits.heterotrophy,
                    ),
                )
            })
            .collect();
        let bucket_of = |id: u64| -> Option<(usize, bool)> {
            let (eff, raw) = drain_eff.get(&id).copied()?;
            let role = roles.get(&id).copied().flatten();
            Some((
                recipient(role, eff),
                role == Some(TrophicRole::Producer) && raw > 0.0,
            ))
        };
        // Per-consumer drain split for B: (living energy, living retained,
        // carcass energy, carcass retained).
        let mut split: HashMap<u64, [f32; 4]> = HashMap::new();
        {
            let mut routes = vec![&mut tally.ext.routes];
            if second_half {
                routes.push(&mut tally.ext.routes_second_half);
            }
            for r in routes {
                for (&id, &u) in &start.uptake {
                    if let Some((b, memo)) = bucket_of(id) {
                        r.uptake[b] += f64::from(u);
                        if memo {
                            r.uptake[MEMO] += f64::from(u);
                        }
                    }
                }
                for (e, d) in &bites {
                    let Some((b, memo)) = bucket_of(e.source) else {
                        continue;
                    };
                    for b in std::iter::once(b).chain(memo.then_some(MEMO)) {
                        if e.target_was_carcass {
                            r.carcass_bites[b] += 1;
                            r.carcass_drained[b] += f64::from(e.energy_delta);
                            r.carcass_energy[b] += f64::from(d.energy);
                            r.carcass_bound[b] += f64::from(d.bound_nutrient);
                            r.carcass_retained[b] += f64::from(d.retained_nutrient);
                        } else {
                            r.living_energy[b] += f64::from(d.energy);
                            r.living_retained[b] += f64::from(d.retained_nutrient);
                        }
                    }
                }
            }
            for (e, d) in &bites {
                let s = split.entry(e.source).or_default();
                let o = if e.target_was_carcass { 2 } else { 0 };
                s[o] += d.energy;
                s[o + 1] += d.retained_nutrient;
            }
            let carcass_events = tail
                .iter()
                .filter(|e| e.kind == EventKind::Consumed && e.target_was_carcass)
                .count() as u64;
            tally.ext.carcass_events += carcass_events;
            if second_half {
                tally.ext.carcass_events_second_half += carcass_events;
            }
        }
        // B. Births and agent-ticks by Producer-role parents / agents at the
        // start of the tick; lifespans.
        for e in &tail {
            match e.kind {
                EventKind::Born => {
                    birth_tick.insert(e.source, e.tick);
                    if second_half
                        && let Some(p) = e.target
                        && roles.get(&p).copied().flatten() == Some(TrophicRole::Producer)
                        && let Some(&h) = pre_eff.get(&p)
                    {
                        tally.ext.producer_bins[het_bin(h)].births += 1;
                    }
                }
                EventKind::Died if second_half => {
                    if let Some(b) = last_bin.remove(&e.source) {
                        let pb = &mut tally.ext.producer_bins[b];
                        pb.deaths += 1;
                        let born = birth_tick.get(&e.source).copied().unwrap_or(0);
                        pb.death_age += e.tick.saturating_sub(born) as f64;
                    }
                }
                _ => {}
            }
        }
        if second_half {
            for (id, &h) in &pre_eff {
                if roles.get(id).copied().flatten() == Some(TrophicRole::Producer) {
                    tally.ext.producer_bins[het_bin(h)].agent_ticks += 1;
                }
            }
        }
        let need_start = !kin_killers.is_empty() || tail.iter().any(|e| lfm.contains(&e.source));
        if need_start {
            let living: HashMap<u64, &explorers_sim::Agent> =
                start.agents.iter().map(|a| (a.id, a)).collect();
            let carcasses: HashMap<u64, &explorers_sim::Carcass> =
                pre.carcasses().iter().map(|c| (c.id, c)).collect();
            // 1. Diet.
            for (e, d) in bites.iter().map(|(e, d)| (*e, *d)) {
                if !lfm.contains(&e.source) {
                    continue;
                }
                let Some(target) = e.target else { continue };
                let bucket = if e.target_was_carcass {
                    CARCASS
                } else if ledger.is_kin(e.source, target) {
                    KIN
                } else {
                    1 + role_slot(roles.get(&target).copied().flatten())
                };
                let body = if e.target_was_carcass {
                    carcasses
                        .get(&target)
                        .filter(|c| c.energy > 0.0)
                        .map(|c| c.nutrient / c.energy)
                } else {
                    living
                        .get(&target)
                        .filter(|a| a.structure > 0.0)
                        .map(|a| a.nutrient_total(params) / a.structure)
                };
                let s = state(&tracker, &intakes, e.source, cell_k, cell_j);
                let mut cells = vec![&mut tally.diet.cells[bucket][s]];
                if second_half {
                    cells.push(&mut tally.diet_second_half.cells[bucket][s]);
                }
                for c in cells {
                    c.bites += 1;
                    c.drained += f64::from(e.energy_delta);
                    c.energy += f64::from(d.energy);
                    c.bound_nutrient += f64::from(d.bound_nutrient);
                    c.retained_nutrient += f64::from(d.retained_nutrient);
                    if e.energy_delta > 0.0 {
                        c.conc_bite += f64::from(d.bound_nutrient / e.energy_delta);
                        c.conc_bite_n += 1;
                    }
                    if let Some(b) = body {
                        c.conc_body += f64::from(b);
                        c.conc_body_n += 1;
                        c.conc_body_w += f64::from(b) * f64::from(e.energy_delta);
                        c.conc_body_w_drained += f64::from(e.energy_delta);
                    }
                }
            }
            // 2. Reach at each kin kill, counted as #637 counts kills: only
            // where the killer has a drain-time intake.
            for &g in &kin_killers {
                if !intakes.contains_key(&g) {
                    continue;
                }
                tally.kin_kills += 1;
                killers.insert(g);
                let s = state(&tracker, &intakes, g, cell_k, cell_j);
                let Some(a) = living.get(&g) else { continue };
                let reach = phase::consumption_reach(
                    a.effective_trait_with_steepness(1, steep),
                    a.structure,
                    params,
                );
                let extent = params.world_extent;
                let mut r = Reach::default();
                r.kills = 1;
                for o in &start.agents {
                    if o.id == g
                        || o.structure <= 0.0
                        || toroidal_distance(a.position, o.position, extent) > reach
                    {
                        continue;
                    }
                    if ledger.is_kin(g, o.id) {
                        r.living_kin += 1;
                    } else {
                        r.living_nonkin += 1;
                        r.nonkin_by_role[role_slot(roles.get(&o.id).copied().flatten())] += 1;
                    }
                }
                for c in pre.carcasses() {
                    if (c.energy <= 0.0 && c.nutrient <= 0.0)
                        || toroidal_distance(a.position, c.position, extent) > reach
                    {
                        continue;
                    }
                    r.carcasses += 1;
                    r.carcasses_kin += u64::from(ledger.is_kin(g, c.id));
                }
                let total = r.living_kin + r.living_nonkin + r.carcasses;
                r.only_kin = u64::from(r.living_nonkin == 0 && r.carcasses == 0);
                r.other_in_reach = 1 - r.only_kin;
                r.nonkin_living_any = u64::from(r.living_nonkin > 0);
                r.carcass_any = u64::from(r.carcasses > 0);
                if total > 0 {
                    let f = r.living_kin as f64 / total as f64;
                    let bin = if r.living_kin == total {
                        FRAC_BINS - 1
                    } else {
                        ((f * 10.0) as usize).min(FRAC_BINS - 2)
                    };
                    r.kin_frac[bin] += 1;
                }
                tally.reach[s].merge(&r);
                if second_half {
                    tally.reach_second_half[s].merge(&r);
                }
            }
        }
        tracker.observe(&intakes, &tail, &kin_killers, &[]);
        let sampled_before = observations.role_snapshots.len();
        observations.observe(&world, interval);
        if observations.role_snapshots.len() > sampled_before
            && let Some((_, sample_roles)) = observations.role_snapshots.last()
        {
            // Every living agent, read by the sample's role (the income
            // ledger as of this tick, as the census reads it).
            let quarter = (((tick.max(1) - 1) * 4) / max_ticks.max(1)).min(3) as usize;
            let fills: HashMap<u64, (f32, explorers_search::grazer_hunger::Surplus)> =
                if second_half {
                    pre.earmark_fills(params)
                        .into_iter()
                        .map(|(a, fill)| {
                            let s = explorers_search::grazer_hunger::Surplus::of(&a, params);
                            (a.id, (fill, s))
                        })
                        .collect()
                } else {
                    HashMap::new()
                };
            for a in world.agents() {
                let role = sample_roles.get(&a.id).copied();
                let eff = a.effective_trait_with_steepness(1, steep);
                if role == Some(TrophicRole::Producer) {
                    let q = &mut tally.ext.quarters[quarter];
                    if a.traits.heterotrophy > 0.0 {
                        q[0].record(eff);
                    }
                    if eff > 0.05 {
                        q[1].record(eff);
                    }
                    q[2].record(eff);
                }
                if !second_half {
                    continue;
                }
                tally.ext.roles[role_slot(role)].record(eff);
                if role != Some(TrophicRole::Producer) {
                    last_bin.remove(&a.id);
                    continue;
                }
                tally.ext.producer_raw_het += u64::from(a.traits.heterotrophy > 0.0);
                let b = het_bin(eff);
                last_bin.insert(a.id, b);
                let pb = &mut tally.ext.producer_bins[b];
                pb.samples += 1;
                let (Some(&(fill, surplus)), Some(i), Some(&pool)) =
                    (fills.get(&a.id), intakes.get(&a.id), pool_at.get(&a.id))
                else {
                    pb.missing_pre += 1;
                    continue;
                };
                pb.earmark_fill += f64::from(fill);
                pb.nutrient_limited += u64::from(surplus.nutrient < surplus.energy);
                pb.pool += f64::from(pool);
                pb.light += f64::from(i.light);
                pb.uptake += f64::from(i.uptake);
                let s = split.get(&a.id).copied().unwrap_or_default();
                pb.living_energy += f64::from(s[0]);
                pb.living_retained += f64::from(s[1]);
                pb.carcass_energy += f64::from(s[2]);
                pb.carcass_retained += f64::from(s[3]);
            }
        }
        world.compact_event_log_before(cursor.min(observations.consumed_events()));
        cursor = world.event_log().len();
        // Every early stop ends the run, as in the census rollout.
        stopped = explorers_genesis_eval::early_stop(
            world.agents().len(),
            &observations,
            eval_config,
            sustainable_stock,
        );
        if stopped.is_some() {
            break;
        }
    }
    let failure = census_failure(&world, &observations, eval_config, max_ticks, stopped);
    tally.outcomes.record(failure.as_ref());
    tally.termination_ticks = world.tick();
    for (id, (m, d)) in lfm_ever {
        if killers.contains(&id) {
            tally.killers += 1;
            tally.killer_mobility.record(m);
            tally.killer_dispersal.record(d);
        } else {
            tally.non_killers += 1;
            tally.non_killer_mobility.record(m);
            tally.non_killer_dispersal.record(d);
        }
    }
    tally
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Row {
    source: ConfigSource,
    config_index: usize,
    horizon: u64,
    base_seed: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    founder_aggregation: Option<f32>,
    satiation_sensitivity: Option<f32>,
    /// The uptake scaling the rows ran at (#645): the config's effective
    /// `uptake_structure_exponent` and `uptake_reference_structure`. Absent
    /// on older rows (which ran before #644, so at b = 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    uptake_structure_exponent: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    uptake_reference_structure: Option<f32>,
    /// Kin kills per seed (for the cross-check).
    seed_kin_kills: Vec<u64>,
    /// Summed over the seeds.
    tally: Tally,
}

struct Args {
    limit: Option<usize>,
    horizon: u64,
    ensemble: u64,
    seed: u64,
    out: PathBuf,
    atlas: Option<PathBuf>,
    configs: Option<HashSet<(ConfigSource, usize)>>,
    summary_only: bool,
    founder_aggregation: Option<f32>,
    satiation_sensitivity: Option<f32>,
    uptake_structure_exponent: Option<f32>,
    uptake_reference_structure: Option<f32>,
    crosscheck: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        limit: None,
        horizon: 2000,
        ensemble: 5,
        seed: 1000,
        out: PathBuf::from("target/kkd/kin-killer-diet.jsonl"),
        atlas: None,
        configs: None,
        summary_only: false,
        founder_aggregation: None,
        satiation_sensitivity: Some(0.0),
        uptake_structure_exponent: None,
        uptake_reference_structure: None,
        crosscheck: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        let number = |raw: String| -> Result<u64, String> {
            raw.parse()
                .map_err(|_| format!("{flag} {raw:?} is not an integer"))
        };
        match flag.as_str() {
            "--limit" => args.limit = Some(number(value()?)? as usize),
            "--max-ticks" | "--horizon" => args.horizon = number(value()?)?,
            "--ensemble" => args.ensemble = number(value()?)?,
            "--seed" => args.seed = number(value()?)?,
            "--out" | "--output" => args.out = PathBuf::from(value()?),
            "--atlas" => args.atlas = Some(PathBuf::from(value()?)),
            "--configs" => args.configs = Some(parse_selector(&value()?, "--configs", None)),
            "--summary" => args.summary_only = true,
            "--crosscheck" => args.crosscheck = true,
            "--founder-aggregation" => {
                args.founder_aggregation = Some(parse_founder_aggregation(&value()?)?)
            }
            "--satiation-sensitivity" => {
                args.satiation_sensitivity = Some(parse_non_negative(&flag, &value()?)?)
            }
            "--uptake-structure-exponent" => {
                args.uptake_structure_exponent = Some(parse_non_negative(&flag, &value()?)?)
            }
            "--uptake-reference-structure" => {
                args.uptake_reference_structure = Some(parse_positive(&flag, &value()?)?)
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(args)
}

fn main() {
    let args = parse_args().unwrap_or_else(|e| {
        eprintln!("kin_killer_diet: {e}");
        std::process::exit(2)
    });
    if !args.summary_only {
        let atlas_units = args
            .atlas
            .as_deref()
            .map(read_atlas_units)
            .unwrap_or_default();
        let sampled = sampled_units(default_ranges().len());
        let done = done_configs(&args.out);
        let tasks = plan_tasks(
            atlas_units.len(),
            sampled.len(),
            args.configs.as_ref(),
            &done,
            args.limit,
        );
        let (k, j) = cell();
        eprintln!(
            "kin_killer_diet: {} configs × {} seeds, horizon {}, cell k = {k}, τ = {} (index {j}); {} done in {}",
            tasks.len(),
            args.ensemble,
            args.horizon,
            tau_grid()[j],
            done.len(),
            args.out.display()
        );
        let start = Instant::now();
        let eval = EvalConfig::default();
        for (n, (source, idx)) in tasks.iter().copied().enumerate() {
            let t = Instant::now();
            let config = resolve_config(source, idx, &atlas_units, &sampled);
            let config = with_founder_aggregation(config, args.founder_aggregation);
            let config = with_consumption_scales(config, args.satiation_sensitivity, None);
            let config = with_uptake_scaling(
                config,
                args.uptake_structure_exponent,
                args.uptake_reference_structure,
            );
            let tallies: Vec<Tally> = (0..args.ensemble)
                .into_par_iter()
                .map(|i| rollout(&config.0, &config.1, args.seed + i, args.horizon, &eval))
                .collect();
            let mut tally = Tally::new();
            for s in &tallies {
                tally.merge(s);
            }
            let seed_kin_kills: Vec<u64> = tallies.iter().map(|t| t.kin_kills).collect();
            if args.crosscheck {
                let theirs: Vec<u64> = (0..args.ensemble)
                    .into_par_iter()
                    .map(|i| {
                        rollout_with_fullness(
                            &config.0,
                            &config.1,
                            args.seed + i,
                            args.horizon,
                            &eval,
                            None,
                            true,
                        )
                        .fullness
                        .map_or(0, |f| f.kills)
                    })
                    .collect();
                eprintln!(
                    "  crosscheck {source}:{idx}: ours {seed_kin_kills:?}, role_diet fullness.kills {theirs:?} -> {}",
                    if theirs == seed_kin_kills {
                        "MATCH"
                    } else {
                        "MISMATCH"
                    }
                );
            }
            append_row(
                &args.out,
                &Row {
                    source,
                    config_index: idx,
                    horizon: args.horizon,
                    base_seed: args.seed,
                    founder_aggregation: args.founder_aggregation,
                    satiation_sensitivity: args.satiation_sensitivity,
                    uptake_structure_exponent: Some(config.0.uptake_structure_exponent),
                    uptake_reference_structure: Some(config.0.uptake_reference_structure),
                    seed_kin_kills,
                    tally,
                },
            );
            eprintln!(
                "  {source}:{idx} ({}/{}): {:.1}s; {:.0}s elapsed",
                n + 1,
                tasks.len(),
                t.elapsed().as_secs_f64(),
                start.elapsed().as_secs_f64()
            );
        }
    }
    let rows: Vec<Row> = read_rows(&args.out);
    summary(&rows);
}

fn pct(n: u64, d: u64) -> String {
    if d == 0 {
        "–".into()
    } else {
        format!("{:.1}", 100.0 * n as f64 / d as f64)
    }
}

fn ratio(n: f64, d: f64) -> String {
    if d == 0.0 {
        "–".into()
    } else {
        format!("{:.4}", n / d)
    }
}

fn share(n: f64, d: f64) -> String {
    if d == 0.0 {
        "–".into()
    } else {
        format!("{:.1}", 100.0 * n / d)
    }
}

fn diet_table(title: &str, d: &Diet) {
    println!("\n### {title}\n");
    let total = |f: &dyn Fn(&Cell) -> f64| -> f64 { d.cells.iter().flatten().map(f).sum() };
    let (tb, te, tn) = (
        total(&|c| c.bites as f64),
        total(&|c| c.energy),
        total(&|c| c.retained_nutrient),
    );
    println!(
        "| target | bites | % bites | drained | energy gained | % energy | N bound | N retained | % N retained | N/structure, bite: mean / drain-weighted | N/structure, body: mean / drain-weighted |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for (b, name) in BUCKETS.iter().enumerate() {
        let mut c = Cell::default();
        for s in &d.cells[b] {
            c.merge(s);
        }
        println!(
            "| {name} | {} | {} | {:.1} | {:.1} | {} | {:.2} | {:.2} | {} | {} | {} |",
            c.bites,
            pct(c.bites, tb as u64),
            c.drained,
            c.energy,
            share(c.energy, te),
            c.bound_nutrient,
            c.retained_nutrient,
            share(c.retained_nutrient, tn),
            format!(
                "{} / {}",
                ratio(c.conc_bite, c.conc_bite_n as f64),
                ratio(c.bound_nutrient, c.drained)
            ),
            format!(
                "{} / {}",
                ratio(c.conc_body, c.conc_body_n as f64),
                ratio(c.conc_body_w, c.conc_body_w_drained)
            ),
        );
    }
    println!(
        "\nSplit by the consumer's fullness state (bites / % of the state's bites / energy gained / N retained):\n"
    );
    print!("| target |");
    for s in STATES {
        print!(" {s} |");
    }
    println!();
    println!("|---|{}", "---:|".repeat(STATES.len()));
    let state_bites: Vec<u64> = (0..STATES.len())
        .map(|s| d.cells.iter().map(|b| b[s].bites).sum())
        .collect();
    for (b, name) in BUCKETS.iter().enumerate() {
        print!("| {name} |");
        for (s, c) in d.cells[b].iter().enumerate() {
            print!(
                " {} ({} %) / {:.1} / {:.2} |",
                c.bites,
                pct(c.bites, state_bites[s]),
                c.energy,
                c.retained_nutrient
            );
        }
        println!();
    }
    print!("| all |");
    for (s, n) in state_bites.iter().enumerate() {
        print!(" {n} ({} % of bites) |", pct(*n, tb as u64));
        let _ = s;
    }
    println!();
}

fn reach_table(title: &str, reach: &[Reach]) {
    println!("\n### {title}\n");
    let mut all = Reach::default();
    for r in reach {
        all.merge(r);
    }
    let rows: Vec<(&str, &Reach)> = STATES
        .iter()
        .copied()
        .zip(reach.iter())
        .chain(std::iter::once(("all", &all)))
        .collect();
    println!(
        "| fullness state | kin kills | only kin in reach % | non-kin or carcass in reach % | non-kin living in reach % | carcass in reach % | mean living kin | mean living non-kin (P / C / D / none) | mean carcasses (of kin) |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    for (name, r) in &rows {
        let m = |x: u64| {
            if r.kills == 0 {
                "–".to_string()
            } else {
                format!("{:.2}", x as f64 / r.kills as f64)
            }
        };
        println!(
            "| {name} | {} | {} | {} | {} | {} | {} | {} ({} / {} / {} / {}) | {} ({}) |",
            r.kills,
            pct(r.only_kin, r.kills),
            pct(r.other_in_reach, r.kills),
            pct(r.nonkin_living_any, r.kills),
            pct(r.carcass_any, r.kills),
            m(r.living_kin),
            m(r.living_nonkin),
            m(r.nonkin_by_role[0]),
            m(r.nonkin_by_role[1]),
            m(r.nonkin_by_role[2]),
            m(r.nonkin_by_role[3]),
            m(r.carcasses),
            m(r.carcasses_kin),
        );
    }
    println!(
        "\nKin share of everything in reach (living kin / (living + carcasses)), % of kin kills:\n"
    );
    print!("| fullness state |");
    for b in 0..FRAC_BINS - 1 {
        print!(" [{:.1}, {:.1}) |", b as f64 / 10.0, (b + 1) as f64 / 10.0);
    }
    println!(" = 1 |");
    println!("|---|{}", "---:|".repeat(FRAC_BINS));
    for (name, r) in &rows {
        print!("| {name} |");
        let n: u64 = r.kin_frac.iter().sum();
        for c in &r.kin_frac {
            print!(" {} |", pct(*c, n));
        }
        println!();
    }
}

fn summary(rows: &[Row]) {
    let mut t = Tally::new();
    for r in rows {
        t.merge(&r.tally);
    }
    let seeds: usize = rows.iter().map(|r| r.seed_kin_kills.len()).sum();
    let (k, j) = cell();
    let fa = rows
        .first()
        .and_then(|r| r.founder_aggregation)
        .map_or("decoded".to_string(), |a| {
            format!("founder_aggregation = {a}")
        });
    println!(
        "## Light-fed mixotroph diet: {} configs, {seeds} seeds, {fa}\n",
        rows.len()
    );
    println!("{}\n", uptake_label(rows));
    println!(
        "Population: heterotrophy > 0 and recent-income role Producer at the start of the tick. Fullness state at k = {k}, τ = {:.2} (index {j}), n = {N}: (i) E_E < 0.5 ≤ E_N; (ii) E_E ≥ 0.5; (iii) both < 0.5; from the fullness carried into the tick. Kin = parent, offspring or sibling (`DietLedger::is_kin`). Non-kin targets by recent-income role at the start of the tick.\n",
        tau_grid()[j]
    );
    println!(
        "Light-fed mixotroph kin kills (one per pair): **{}**",
        t.kin_kills
    );
    println!("{}", kin_kill_rates(&t));
    diet_table("1. Diet, whole run", &t.diet);
    diet_table("1. Diet, second half (tick ≥ T/2 + 1)", &t.diet_second_half);
    reach_table("2. Reach at each kin kill, whole run", &t.reach);
    reach_table(
        "2. Reach at each kin kill, second half",
        &t.reach_second_half,
    );
    println!("\n### 3. Traits: kin killers vs light-fed mixotrophs that never kill kin\n");
    println!("| group | trait | agents | p10 | p25 | median | p75 | p90 |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|");
    for (g, trait_, h, n) in [
        ("kin killers", "mobility", &t.killer_mobility, t.killers),
        (
            "never kill kin",
            "mobility",
            &t.non_killer_mobility,
            t.non_killers,
        ),
        ("kin killers", "dispersal", &t.killer_dispersal, t.killers),
        (
            "never kill kin",
            "dispersal",
            &t.non_killer_dispersal,
            t.non_killers,
        ),
    ] {
        let p = |q| h.percentile(q).map_or("–".into(), |v| format!("{v:.3}"));
        println!(
            "| {g} | {trait_} | {n} | {} | {} | {} | {} | {} |",
            p(0.1),
            p(0.25),
            p(0.5),
            p(0.75),
            p(0.9)
        );
    }
    ext_summary(&t.ext);
    persistence(rows, &t);
}

/// The uptake scaling the rows ran at, each distinct value listed.
fn uptake_label(rows: &[Row]) -> String {
    let distinct = |f: &dyn Fn(&Row) -> Option<f32>| -> String {
        let mut v: Vec<String> = Vec::new();
        for r in rows {
            let s = f(r).map_or("unrecorded (pre-#645 row)".into(), |x| format!("{x}"));
            if !v.contains(&s) {
                v.push(s);
            }
        }
        if v.is_empty() {
            "–".into()
        } else {
            v.join(" / ")
        }
    };
    format!(
        "Uptake scaling (#644): b = uptake_structure_exponent = **{}**, s_ref = uptake_reference_structure = **{}**",
        distinct(&|r| r.uptake_structure_exponent),
        distinct(&|r| r.uptake_reference_structure)
    )
}

/// Kin kills per 1000 Producer-role agent-ticks, whole run and second half.
fn kin_kill_rates(t: &Tally) -> String {
    let second_kills: u64 = t.reach_second_half.iter().map(|r| r.kills).sum();
    let second_ticks: u64 = t.ext.producer_bins.iter().map(|b| b.agent_ticks).sum();
    format!(
        "Per 1000 Producer-role agent-ticks: whole run {} ({} agent-ticks), second half {} ({} kin kills / {} agent-ticks)",
        per(1000.0 * t.kin_kills as f64, t.producer_agent_ticks),
        t.producer_agent_ticks,
        per(1000.0 * second_kills as f64, second_ticks),
        second_kills,
        second_ticks
    )
}

fn persistence(rows: &[Row], t: &Tally) {
    println!("\n### E. Persistence: the census's verdict per seed (`role_diet::census_failure`)\n");
    println!(
        "Pooled over {} seeds: {} persisted ({} %); {}\n",
        t.outcomes.seeds(),
        t.outcomes.persisted(),
        pct(t.outcomes.persisted(), t.outcomes.seeds()),
        t.outcomes.describe()
    );
    println!(
        "| config | seeds | persisted | verdicts | mean termination tick | kin kills | Producer agent-ticks | kin kills / 1000 P agent-ticks |"
    );
    println!("|---|---:|---:|---|---:|---:|---:|---:|");
    for r in rows {
        let o = &r.tally.outcomes;
        println!(
            "| {}:{} | {} | {} | {} | {} | {} | {} | {} |",
            r.source,
            r.config_index,
            o.seeds(),
            o.persisted(),
            o.describe(),
            ticks(r.tally.termination_ticks, o.seeds()),
            r.tally.kin_kills,
            r.tally.producer_agent_ticks,
            per(
                1000.0 * r.tally.kin_kills as f64,
                r.tally.producer_agent_ticks
            ),
        );
    }
}

fn f4(x: Option<f64>) -> String {
    x.map_or("–".into(), |v| format!("{v:.3}"))
}

fn per(n: f64, d: u64) -> String {
    if d == 0 {
        "–".into()
    } else {
        format!("{:.4}", n / d as f64)
    }
}

fn routes_table(title: &str, r: &Routes) {
    println!("\n#### {title}\n");
    println!(
        "| recipient (role at start of tick) | carcass bites | carcass drained | carcass energy gained | carcass N bound | carcass N retained | % of carcass N retained | pool uptake N | living-drain N retained | carcass-drain N retained | N acquired | % N from pool / living / carcass | living-drain energy |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|");
    let primary = |v: &Vec<f64>| -> f64 { v[..MEMO].iter().sum() };
    let carcass_n = primary(&r.carcass_retained);
    let mut rows: Vec<(String, usize)> = RECIPIENTS
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != MEMO)
        .map(|(i, n)| (n.to_string(), i))
        .collect();
    rows.push(("**all**".into(), usize::MAX));
    rows.push((RECIPIENTS[MEMO].into(), MEMO));
    for (name, i) in rows {
        let get = |v: &Vec<f64>| if i == usize::MAX { primary(v) } else { v[i] };
        let bites = if i == usize::MAX {
            r.carcass_bites[..MEMO].iter().sum()
        } else {
            r.carcass_bites[i]
        };
        let (u, l, c) = (
            get(&r.uptake),
            get(&r.living_retained),
            get(&r.carcass_retained),
        );
        let tot = u + l + c;
        println!(
            "| {name} | {bites} | {:.1} | {:.1} | {:.2} | {:.2} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {} / {} / {} | {:.1} |",
            get(&r.carcass_drained),
            get(&r.carcass_energy),
            get(&r.carcass_bound),
            c,
            share(c, carcass_n),
            u,
            l,
            c,
            tot,
            share(u, tot),
            share(l, tot),
            share(c, tot),
            get(&r.living_energy),
        );
    }
}

fn ext_summary(x: &Ext) {
    if x.roles.is_empty() || x.roles.iter().all(|h| h.n == 0) {
        println!("\n(no sections A–D in these rows)");
        return;
    }
    let all: u64 = x.roles.iter().map(|h| h.n).sum();
    let prod = &x.roles[0];
    println!(
        "\n### A. Prevalence and investment (second-half sample ticks, every living agent, role = the sample's recent-income role; h_eff = effective heterotrophy `effective_trait_with_steepness(1, wear_degradation_steepness)`)\n"
    );
    println!(
        "| role | agent-samples | % | h_eff p10 | p25 | median | p75 | p90 | mean | % > 0.05 | % > 0.2 | % > 0.5 |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for (name, h) in ROLES.iter().zip(&x.roles) {
        println!(
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            h.n,
            pct(h.n, all),
            f4(h.percentile(0.1)),
            f4(h.percentile(0.25)),
            f4(h.percentile(0.5)),
            f4(h.percentile(0.75)),
            f4(h.percentile(0.9)),
            f4(h.mean()),
            pct(h.above[0], h.n),
            pct(h.above[1], h.n),
            pct(h.above[2], h.n),
        );
    }
    println!("\nShare of all {all} agent-samples:\n");
    println!("| group | agent-samples | % |");
    println!("|---|---:|---:|");
    for (name, n) in [
        (
            "light-fed mixotroph (Producer, h_eff > 0.05)",
            prod.above[0],
        ),
        (
            "pure producer (Producer, h_eff ≤ 0.05)",
            prod.n - prod.above[0],
        ),
        (
            "memo: Producer with raw h > 0 (section 1's definition)",
            x.producer_raw_het,
        ),
        ("consumer", x.roles[1].n),
        ("decomposer", x.roles[2].n),
        ("no income yet", x.roles[3].n),
    ] {
        println!("| {name} | {n} | {} |", pct(n, all));
    }

    println!(
        "\n### B. Producer-role agent-samples (second half) by h_eff\n\nPer agent-sample means. Births: second-half births by a parent that was Producer at the start of the tick, binned by its pre-step h_eff; per 1000 agent-ticks counts Producer agents at the start of every second-half tick on the same classification, and per agent-sample divides by the sample count (samples every {} ticks). Earmark fill and nutrient-limited (`Surplus` nutrient < energy) off the metabolised roster, as the census reads them. Pool: the nutrient-grid cell under the agent before the step (what uptake reads). Income: light and pool uptake from the drain-start replay; drains from `realised_bites`, energy gained and nutrient retained. Lifespan: second-half deaths of agents whose last role sample was Producer in the bin; age = death tick − birth tick (founders from 0; agents alive at the end are censored).\n",
        EvalConfig::default().coexistence_sample_interval
    );
    println!(
        "| h_eff bin | samples | births / sample | births / 1000 agent-ticks | earmark fill | % N-limited | pool N at cell | light E | uptake N | living-drain E | living-drain N | carcass-drain E | carcass-drain N | N from carcass % | deaths | mean age at death | missing reads |"
    );
    println!(
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    let mut sum = ProducerBin::default();
    let mut lines: Vec<(String, ProducerBin)> = HET_BINS
        .iter()
        .zip(&x.producer_bins)
        .map(|(n, b)| {
            sum.merge(b);
            (n.to_string(), b.clone())
        })
        .collect();
    lines.push(("**all**".into(), sum.clone()));
    for (name, b) in &lines {
        let n = b.samples - b.missing_pre;
        let nut = b.uptake + b.living_retained + b.carcass_retained;
        println!(
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            b.samples,
            per(b.births as f64, b.samples),
            per(1000.0 * b.births as f64, b.agent_ticks),
            per(b.earmark_fill, n),
            pct(b.nutrient_limited, n),
            per(b.pool, n),
            per(b.light, n),
            per(b.uptake, n),
            per(b.living_energy, n),
            per(b.living_retained, n),
            per(b.carcass_energy, n),
            per(b.carcass_retained, n),
            share(b.carcass_retained, nut),
            b.deaths,
            per(b.death_age, b.deaths),
            b.missing_pre,
        );
    }
    println!(
        "\nCheck: B samples sum {} vs A Producer {} -> {}",
        sum.samples,
        prod.n,
        if sum.samples == prod.n {
            "MATCH"
        } else {
            "MISMATCH"
        }
    );

    println!(
        "\n### C. Who processes the detritus, and nutrient into living agents by route\n\nRecipient by recent-income role at the start of the tick and drain-time h_eff. Carcass bites: `Consumed` with `target_was_carcass`, valued by `realised_bites` (consumers on the drain-time roster). Pool uptake: the drain-start replay's `NutrientAbsorbed` credits (`tick_intakes`' uptake). Network redistribution: enabled in {} of the seeds' configs (connection cap > 0){}.\n",
        x.network_enabled_seeds,
        if x.network_enabled_seeds == 0 {
            " — so no network route"
        } else {
            " — its nutrient transfer emits no event and is not counted here"
        }
    );
    println!(
        "Carcass `Consumed` events: whole run {}, second half {}.",
        x.carcass_events, x.carcass_events_second_half
    );
    routes_table("C. Whole run", &x.routes);
    routes_table("C. Second half", &x.routes_second_half);

    println!(
        "\n### D. Appetite over time: h_eff of Producer-role agents by quarter of the run (on sample ticks, every living agent, the sample's role)\n"
    );
    println!(
        "| population | quarter | agent-samples | mean h_eff | median h_eff | % > 0.05 | % > 0.5 |"
    );
    println!("|---|---|---:|---:|---:|---:|---:|");
    for (p, name) in D_POPS.iter().enumerate() {
        for (q, hs) in x.quarters.iter().enumerate() {
            let h = &hs[p];
            println!(
                "| {name} | Q{} | {} | {} | {} | {} | {} |",
                q + 1,
                h.n,
                f4(h.mean()),
                f4(h.percentile(0.5)),
                pct(h.above[0], h.n),
                pct(h.above[2], h.n),
            );
        }
    }
}

/// A mean tick count, to one decimal.
fn ticks(sum: u64, n: u64) -> String {
    if n == 0 {
        "–".into()
    } else {
        format!("{:.1}", sum as f64 / n as f64)
    }
}
