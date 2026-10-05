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
//!
//! #646 switches the network (flow 5) on: `--network-connection-cap <n>`,
//! `--network-creation-cost <e>`, `--network-maintenance-cost <e>`,
//! `--network-redistribution-rate <f>` and `--network-transfer-efficiency
//! <f>` pin those parameters on every decoded config (unset, each keeps its
//! decoded value: the network off). C. then also reads the network as a route
//! of nutrient (and energy) into living agents, per recipient and in each
//! direction, off the `Redistributed` events (one per currency, #646), and F.
//! counts live connections by endpoint roles and kinship. Both print only
//! when some seed ran with the network on, so an off run's tables are
//! unchanged.
//!
//! #655 reads #647's two tests of light-fed mixotrophy as a niche, and opens
//! the report with a **verdict block** (pass / fail per test, persisted seeds
//! and nutrient-lockup seeds against #642's b = 0 reference). G.
//! **Conditionality**: Spearman ρ(h_eff, pool N at the cell) over B's
//! second-half Producer-role samples, pooled across every config and seed and
//! per config (median), with B's bins' mean pool; passes when both are < 0.
//! H. **Niche share**: carcass structure drained, nutrient released and
//! nutrient retained by C's recipient buckets, whole run and second half;
//! passes when light-fed mixotrophs drain < 50 % of the attributed structure
//! in both. Rows from before #655 read back (G empty, H's unattributed share
//! unrecorded).
//!
//! #656 runs it on a fresh atlas. Each `atlas:` row records the atlas's
//! fingerprint (`AtlasUnits::fingerprint`), and the verdict block compares
//! persistence with #642 only on #642's atlas (rows without one predate the
//! fingerprint and ran there); otherwise it prints "no reference (different
//! atlas)".
//!
//! #668 calibrates the range of the autotrophy × heterotrophy cross-trait
//! cost `c_AH` (#667; world-rules.md, trade-off #5) and reads the deficit
//! rule (#666) alone. I. **Calibration**: per light-fed mixotroph (C's
//! bucket: Producer by recent-income role at the start of the tick, drain-time
//! h_eff > 0.05), over its second-half ticks in that bucket, its mean drain
//! energy income per tick (`realised_bites`' energy gained, living and
//! carcass) and its cross-trait term `(A·H)^(p/2)` (raw photosynthetic
//! absorption and heterotrophy, `p` = the config's
//! `maintenance_cost_exponent`); `c_max`, the `c_AH` at which a typical one's
//! cross-trait cost is twice its drain income, two ways (ratio of medians;
//! median of per-agent ratios), each also over drainers only. J. **Carcass
//! nutrient retained** by light-fed mixotrophs in the second half, and
//! Spearman ρ(retained per unit drained, pool N under the drainer) per
//! carcass bite, pooled and median per config. `--cross-trait-cost <c>` pins
//! `c_AH` on every decoded config (unset, each keeps its decoded value: 0),
//! for the probe at `c_max`; rows record the effective value. Rows from
//! before #668 read back with I and J empty.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use rayon::prelude::*;

use explorers_genesis_eval::{EvalConfig, RolloutObservations};
use explorers_search::config_source::{
    ConfigSource, NetworkPins, parse_founder_aggregation, parse_non_negative, parse_positive,
    parse_selector, parse_unit_interval, resolve_config, sampled_units, with_consumption_scales,
    with_cross_trait_cost, with_founder_aggregation, with_network, with_uptake_scaling,
};
use explorers_search::fullness::{FullnessBank, FullnessTracker, k_grid, tau_grid, tick_intakes};
use explorers_search::grazer_hunger::PreStep;
use explorers_search::intake_ceiling::realised_bites;
use explorers_search::role_diet::{DietLedger, Outcomes, census_failure, rollout_with_fullness};
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
    /// Network (flow 5, #646), per [`RECIPIENTS`] bucket: free nutrient
    /// received / given, and energy received / given (net of the transfer
    /// loss: the `Redistributed` event's received amount, on both sides).
    #[serde(default = "recipient_zeros")]
    network_n_in: Vec<f64>,
    #[serde(default = "recipient_zeros")]
    network_n_out: Vec<f64>,
    #[serde(default = "recipient_zeros")]
    network_e_in: Vec<f64>,
    #[serde(default = "recipient_zeros")]
    network_e_out: Vec<f64>,
    /// Network flow donor bucket → recipient bucket, `[donor * MEMO +
    /// recipient]` over the primary buckets (no memo), nutrient and energy.
    #[serde(default = "flow_zeros")]
    network_n_flow: Vec<f64>,
    #[serde(default = "flow_zeros")]
    network_e_flow: Vec<f64>,
    /// Network nutrient / energy on an event an endpoint of which is not on
    /// the drain-time roster (should be zero).
    #[serde(default)]
    network_n_unattributed: f64,
    #[serde(default)]
    network_e_unattributed: f64,
}

fn recipient_zeros() -> Vec<f64> {
    vec![0.0; RECIPIENTS.len()]
}

fn flow_zeros() -> Vec<f64> {
    vec![0.0; MEMO * MEMO]
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
            network_n_in: recipient_zeros(),
            network_n_out: recipient_zeros(),
            network_e_in: recipient_zeros(),
            network_e_out: recipient_zeros(),
            network_n_flow: flow_zeros(),
            network_e_flow: flow_zeros(),
            network_n_unattributed: 0.0,
            network_e_unattributed: 0.0,
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
        add(&mut self.network_n_in, &o.network_n_in);
        add(&mut self.network_n_out, &o.network_n_out);
        add(&mut self.network_e_in, &o.network_e_in);
        add(&mut self.network_e_out, &o.network_e_out);
        add(&mut self.network_n_flow, &o.network_n_flow);
        add(&mut self.network_e_flow, &o.network_e_flow);
        self.network_n_unattributed += o.network_n_unattributed;
        self.network_e_unattributed += o.network_e_unattributed;
    }
}

/// F's endpoint-role pairs, unordered over [`ROLES`]' slots: `pair(i, j)`.
const PAIRS: [&str; 10] = [
    "producer–producer",
    "producer–consumer",
    "producer–decomposer",
    "producer–no income",
    "consumer–consumer",
    "consumer–decomposer",
    "consumer–no income",
    "decomposer–decomposer",
    "decomposer–no income",
    "no income–no income",
];

fn pair(a: usize, b: usize) -> usize {
    let (i, j) = if a <= b { (a, b) } else { (b, a) };
    // Row i of the upper triangle starts after the rows above it.
    i * ROLES.len() - i * (i.saturating_sub(1)) / 2 + (j - i)
}

/// F (#646): live network connections on sample ticks, by endpoint roles
/// (the sample's recent-income role) and kinship (`DietLedger::is_kin`).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct NetCensus {
    sample_ticks: u64,
    /// Living agents on those sample ticks (the per-agent denominator).
    agent_samples: u64,
    /// Σ live connections over sample ticks.
    connections: u64,
    /// `pairs[pair] = [non-kin, kin]`.
    pairs: Vec<[u64; 2]>,
    /// Connections with an endpoint that died later in the tick (pruned at
    /// the next tick's network phase).
    dangling: u64,
    /// Connections formed / dissolved, from tick-to-tick set differences.
    formed: u64,
    dissolved: u64,
}

impl NetCensus {
    fn merge(&mut self, o: &NetCensus) {
        if self.pairs.is_empty() {
            self.pairs = vec![[0; 2]; PAIRS.len()];
        }
        self.sample_ticks += o.sample_ticks;
        self.agent_samples += o.agent_samples;
        self.connections += o.connections;
        for (a, b) in self.pairs.iter_mut().zip(&o.pairs) {
            a[0] += b[0];
            a[1] += b[1];
        }
        self.dangling += o.dangling;
        self.formed += o.formed;
        self.dissolved += o.dissolved;
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
    /// F (#646): whole run and second half.
    #[serde(default)]
    network: NetCensus,
    #[serde(default)]
    network_second_half: NetCensus,
    /// G (#655): every second-half Producer-role agent-sample B reads
    /// (missing reads excluded), as `[h_eff, pool N at the cell]`.
    #[serde(default)]
    het_pool: Vec<[f32; 2]>,
    /// H (#655): structure drained (`Consumed.energy_delta`) over every
    /// carcass `Consumed` event, attributed to a recipient or not; whole run
    /// and second half. Zero on pre-#655 rows.
    #[serde(default)]
    carcass_drained_all: f64,
    #[serde(default)]
    carcass_drained_all_second_half: f64,
    /// I (#668): one entry per light-fed mixotroph per seed, over the
    /// second-half ticks it spent in C's light-fed mixotroph bucket, as
    /// `[mean drain energy income per such tick, (A·H)^(p/2), ticks]`.
    #[serde(default)]
    lfm_calibration: Vec<[f32; 3]>,
    /// J (#668): every second-half carcass bite by a light-fed mixotroph
    /// (`realised_bites`' valued ones, as C's route counts them), as
    /// `[structure drained, nutrient retained, pool N in the cell under the
    /// drainer before the step]`. A bite on a carcass with no structure left
    /// drains 0 and can still retain nutrient.
    #[serde(default)]
    lfm_carcass_bites: Vec<[f32; 3]>,
    /// J: such bites with no pre-step pool reading (should be zero).
    #[serde(default)]
    lfm_carcass_bites_no_pool: u64,
    /// Second-half ticks run, summed over seeds (J's per-tick denominator).
    #[serde(default)]
    second_half_ticks: u64,
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
        self.network.merge(&o.network);
        self.network_second_half.merge(&o.network_second_half);
        self.het_pool.extend_from_slice(&o.het_pool);
        self.carcass_drained_all += o.carcass_drained_all;
        self.carcass_drained_all_second_half += o.carcass_drained_all_second_half;
        self.lfm_calibration.extend_from_slice(&o.lfm_calibration);
        self.lfm_carcass_bites
            .extend_from_slice(&o.lfm_carcass_bites);
        self.lfm_carcass_bites_no_pool += o.lfm_carcass_bites_no_pool;
        self.second_half_ticks += o.second_half_ticks;
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
        EventKind::Redistributed,
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
    let network_on = params.network_connection_cap > 0;
    // I (#668): per light-fed mixotroph, (second-half ticks in the bucket,
    // Σ drain energy gained on them, its cross-trait term).
    let mut calibration: HashMap<u64, (u32, f64, f32)> = HashMap::new();
    let p = params.maintenance_cost_exponent;
    let mut live_connections: HashSet<(u64, u64)> = HashSet::new();
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
        if network_on {
            let now: HashSet<(u64, u64)> = world
                .connections()
                .iter()
                .map(|c| (c.builder, c.partner))
                .collect();
            let formed = now.difference(&live_connections).count() as u64;
            let dissolved = live_connections.difference(&now).count() as u64;
            tally.ext.network.formed += formed;
            tally.ext.network.dissolved += dissolved;
            if second_half {
                tally.ext.network_second_half.formed += formed;
                tally.ext.network_second_half.dissolved += dissolved;
            }
            live_connections = now;
        }
        let intakes = tick_intakes(&pre, params, &tail);
        let start = pre.drain_start(params);
        let bites = realised_bites(&start, pre.carcasses(), params, &tail);
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
                // The network (flow 5): one `Redistributed` event per currency,
                // source = donor, target = recipient.
                for e in tail.iter().filter(|e| e.kind == EventKind::Redistributed) {
                    let (n, en) = (f64::from(e.nutrient_delta), f64::from(e.energy_delta));
                    let (Some(donor), Some(to)) =
                        (bucket_of(e.source), e.target.and_then(bucket_of))
                    else {
                        r.network_n_unattributed += n;
                        r.network_e_unattributed += en;
                        continue;
                    };
                    for (b, memo, inflow) in [(to.0, to.1, true), (donor.0, donor.1, false)] {
                        for b in std::iter::once(b).chain(memo.then_some(MEMO)) {
                            if inflow {
                                r.network_n_in[b] += n;
                                r.network_e_in[b] += en;
                            } else {
                                r.network_n_out[b] += n;
                                r.network_e_out[b] += en;
                            }
                        }
                    }
                    r.network_n_flow[donor.0 * MEMO + to.0] += n;
                    r.network_e_flow[donor.0 * MEMO + to.0] += en;
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
                // J: a light-fed mixotroph's carcass bite, against the pool
                // under it.
                if second_half
                    && e.target_was_carcass
                    && matches!(bucket_of(e.source), Some((0, _)))
                {
                    match pool_at.get(&e.source) {
                        Some(&pool) => tally.ext.lfm_carcass_bites.push([
                            e.energy_delta,
                            d.retained_nutrient,
                            pool,
                        ]),
                        None => tally.ext.lfm_carcass_bites_no_pool += 1,
                    }
                }
            }
            let carcass: Vec<&Event> = tail
                .iter()
                .filter(|e| e.kind == EventKind::Consumed && e.target_was_carcass)
                .collect();
            let carcass_events = carcass.len() as u64;
            let carcass_drained: f64 = carcass.iter().map(|e| f64::from(e.energy_delta)).sum();
            tally.ext.carcass_events += carcass_events;
            tally.ext.carcass_drained_all += carcass_drained;
            if second_half {
                tally.ext.carcass_events_second_half += carcass_events;
                tally.ext.carcass_drained_all_second_half += carcass_drained;
            }
        }
        // I. Each light-fed mixotroph's drain energy income on its
        // second-half ticks in the bucket (ticks without a drain count as 0).
        if second_half {
            tally.ext.second_half_ticks += 1;
            for a in &start.agents {
                if matches!(bucket_of(a.id), Some((0, _))) {
                    let s = split.get(&a.id).copied().unwrap_or_default();
                    let c = calibration.entry(a.id).or_insert_with(|| {
                        (
                            0,
                            0.0,
                            cross_trait_term(
                                a.traits.photosynthetic_absorption,
                                a.traits.heterotrophy,
                                p,
                            ),
                        )
                    });
                    c.0 += 1;
                    c.1 += f64::from(s[0] + s[2]);
                }
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
            if network_on {
                let alive: HashSet<u64> = world.agents().iter().map(|a| a.id).collect();
                let mut census = NetCensus {
                    sample_ticks: 1,
                    agent_samples: alive.len() as u64,
                    connections: world.connections().len() as u64,
                    pairs: vec![[0; 2]; PAIRS.len()],
                    ..Default::default()
                };
                for c in world.connections() {
                    if !alive.contains(&c.builder) || !alive.contains(&c.partner) {
                        census.dangling += 1;
                        continue;
                    }
                    let slot = |id: u64| role_slot(sample_roles.get(&id).copied());
                    let kin = usize::from(ledger.is_kin(c.builder, c.partner));
                    census.pairs[pair(slot(c.builder), slot(c.partner))][kin] += 1;
                }
                tally.ext.network.merge(&census);
                if second_half {
                    tally.ext.network_second_half.merge(&census);
                }
            }
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
                tally.ext.het_pool.push([eff, pool]);
                let pb = &mut tally.ext.producer_bins[b];
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
    let mut calibration: Vec<(u64, (u32, f64, f32))> = calibration.into_iter().collect();
    calibration.sort_by_key(|(id, _)| *id);
    tally.ext.lfm_calibration = calibration
        .into_iter()
        .map(|(_, (n, e, term))| [(e / f64::from(n)) as f32, term, n as f32])
        .collect();
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
    /// The network settings the rows ran at (#646): the config's effective
    /// five network parameters. Absent on older rows (network off).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    network: Option<NetworkRecord>,
    /// The atlas an `atlas:` row ran on (#656): its
    /// [`AtlasUnits::fingerprint`](explorers_search::sweep::AtlasUnits::fingerprint)
    /// in hex. Absent on older rows, which all ran on #642's atlas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    atlas: Option<String>,
    /// The config's effective cross-trait cost `c_AH` (#668: pinned by
    /// `--cross-trait-cost`, else as decoded). Absent on older rows, which
    /// ran without the term (0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cross_trait_cost: Option<f32>,
    /// The config's `maintenance_cost_exponent` p (#668: I's term is
    /// `(A·H)^(p/2)`). Absent on older rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    maintenance_cost_exponent: Option<f32>,
    /// Kin kills per seed (for the cross-check).
    seed_kin_kills: Vec<u64>,
    /// Summed over the seeds.
    tally: Tally,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
struct NetworkRecord {
    connection_cap: u32,
    creation_cost: f32,
    maintenance_cost: f32,
    redistribution_rate: f32,
    transfer_efficiency: f32,
}

impl NetworkRecord {
    fn of(p: &WorldParameters) -> Self {
        Self {
            connection_cap: p.network_connection_cap,
            creation_cost: p.network_creation_cost,
            maintenance_cost: p.network_maintenance_cost,
            redistribution_rate: p.network_redistribution_rate,
            transfer_efficiency: p.network_transfer_efficiency,
        }
    }

    fn label(&self) -> String {
        if self.connection_cap == 0 {
            return "off (connection cap 0)".into();
        }
        format!(
            "connection cap {}, creation cost {}, maintenance cost {}, redistribution rate {}, transfer efficiency {}",
            self.connection_cap,
            self.creation_cost,
            self.maintenance_cost,
            self.redistribution_rate,
            self.transfer_efficiency
        )
    }
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
    network: NetworkPins,
    cross_trait_cost: Option<f32>,
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
        network: NetworkPins::default(),
        cross_trait_cost: None,
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
            "--network-connection-cap" => {
                args.network.connection_cap = Some(
                    number(value()?)?
                        .try_into()
                        .map_err(|_| format!("{flag} is out of range"))?,
                )
            }
            "--network-creation-cost" => {
                args.network.creation_cost = Some(parse_non_negative(&flag, &value()?)?)
            }
            "--network-maintenance-cost" => {
                args.network.maintenance_cost = Some(parse_non_negative(&flag, &value()?)?)
            }
            "--network-redistribution-rate" => {
                args.network.redistribution_rate = Some(parse_unit_interval(&flag, &value()?)?)
            }
            "--network-transfer-efficiency" => {
                args.network.transfer_efficiency = Some(parse_unit_interval(&flag, &value()?)?)
            }
            "--cross-trait-cost" => {
                args.cross_trait_cost = Some(parse_non_negative(&flag, &value()?)?)
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
        let atlas_fingerprint = format!("{:016x}", atlas_units.fingerprint());
        let sampled = sampled_units();
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
            let config = with_network(config, &args.network);
            let config = with_cross_trait_cost(config, args.cross_trait_cost);
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
                    network: Some(NetworkRecord::of(&config.0)),
                    atlas: (source == ConfigSource::Atlas).then(|| atlas_fingerprint.clone()),
                    cross_trait_cost: Some(config.0.cross_trait_cost),
                    maintenance_cost_exponent: Some(config.0.maintenance_cost_exponent),
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
    println!("{}\n", network_label(rows));
    println!("{}\n", cross_trait_label(rows));
    verdict(rows, &t);
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
    if t.ext.network_enabled_seeds > 0 {
        network_census(rows, &t.ext);
    }
    conditionality(rows, &t.ext);
    niche_share(&t.ext);
    calibration(rows, &t.ext);
    retention(rows, &t.ext);
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

/// The network settings the rows ran at, each distinct setting listed.
fn network_label(rows: &[Row]) -> String {
    let mut v: Vec<String> = Vec::new();
    for r in rows {
        let s = r
            .network
            .map_or("unrecorded (pre-#646 row: off)".into(), |n| n.label());
        if !v.contains(&s) {
            v.push(s);
        }
    }
    format!(
        "Network (flow 5, #646): **{}**",
        if v.is_empty() {
            "–".into()
        } else {
            v.join(" / ")
        }
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
            " — read off its `Redistributed` events (one per currency, source = donor) in the network tables below each C table; the main C table leaves it out"
        }
    );
    println!(
        "Carcass `Consumed` events: whole run {}, second half {}.",
        x.carcass_events, x.carcass_events_second_half
    );
    routes_table("C. Whole run", &x.routes);
    if x.network_enabled_seeds > 0 {
        network_routes_table("C. Whole run, with the network route", &x.routes);
    }
    routes_table("C. Second half", &x.routes_second_half);
    if x.network_enabled_seeds > 0 {
        network_routes_table(
            "C. Second half, with the network route",
            &x.routes_second_half,
        );
    }

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

/// C's network route (#646): nutrient and energy each recipient bucket took
/// in and gave out along connections, the nutrient acquired by route with the
/// network's share, and the donor → recipient flow.
fn network_routes_table(title: &str, r: &Routes) {
    println!("\n#### {title}\n");
    println!(
        "Network N in / out: free nutrient received / given along connections (conserved). Network E in / out: energy received along connections, on the recipient's and the donor's side (net of the transfer loss). N acquired = pool uptake + living-drain + carcass-drain retained + network N in. Unattributed (an endpoint off the drain-time roster): N {:.2}, E {:.2}.\n",
        r.network_n_unattributed, r.network_e_unattributed
    );
    println!(
        "| recipient (role at start of tick) | pool uptake N | living-drain N | carcass-drain N | network N in | network N out | net network N | N acquired | % N from pool / living / carcass / network | network E in | network E out | net network E |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|");
    let primary = |v: &Vec<f64>| -> f64 { v[..MEMO].iter().sum() };
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
        let (u, l, c, n_in, n_out) = (
            get(&r.uptake),
            get(&r.living_retained),
            get(&r.carcass_retained),
            get(&r.network_n_in),
            get(&r.network_n_out),
        );
        let (e_in, e_out) = (get(&r.network_e_in), get(&r.network_e_out));
        let tot = u + l + c + n_in;
        println!(
            "| {name} | {u:.2} | {l:.2} | {c:.2} | {n_in:.2} | {n_out:.2} | {:.2} | {tot:.2} | {} / {} / {} / {} | {e_in:.1} | {e_out:.1} | {:.1} |",
            n_in - n_out,
            share(u, tot),
            share(l, tot),
            share(c, tot),
            share(n_in, tot),
            e_in - e_out,
        );
    }
    for (what, flow) in [
        ("nutrient", &r.network_n_flow),
        ("energy (received)", &r.network_e_flow),
    ] {
        let total: f64 = flow.iter().sum();
        println!("\nNetwork {what} flow, donor (row) → recipient (column), % of all {total:.2}:\n");
        print!("| donor \\ recipient |");
        for name in &RECIPIENTS[..MEMO] {
            print!(" {name} |");
        }
        println!();
        println!("|---|{}", "---:|".repeat(MEMO));
        for (d, name) in RECIPIENTS[..MEMO].iter().enumerate() {
            print!("| {name} |");
            for t in 0..MEMO {
                print!(" {} |", share(flow[d * MEMO + t], total));
            }
            println!();
        }
    }
}

/// F (#646): live connections by endpoint roles and kinship, and turnover.
fn network_census(rows: &[Row], x: &Ext) {
    println!(
        "\n### F. Network connections (sample ticks, endpoint roles = the sample's recent-income role, kin = `DietLedger::is_kin`)\n"
    );
    println!(
        "Mean live connections per agent-sample = 2 × connections / living agents (each connection has two endpoints). Formed / dissolved: tick-to-tick differences of the live set (a connection shed and rebuilt within one tick is not seen). Dangling: an endpoint died after the network phase (pruned next tick).\n"
    );
    println!(
        "| window | sample ticks | agent-samples | connections | mean live connections per agent-sample | connections per sample tick | dangling | formed | dissolved |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    for (name, n) in [
        ("whole run", &x.network),
        ("second half", &x.network_second_half),
    ] {
        println!(
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} |",
            n.sample_ticks,
            n.agent_samples,
            n.connections,
            per(2.0 * n.connections as f64, n.agent_samples),
            per(n.connections as f64, n.sample_ticks),
            n.dangling,
            n.formed,
            n.dissolved,
        );
    }
    println!("\nBy endpoint roles, second half (connections summed over sample ticks):\n");
    println!("| endpoint roles | non-kin | kin | all | % of classified |");
    println!("|---|---:|---:|---:|---:|");
    let n = &x.network_second_half;
    let classified: u64 = n.pairs.iter().map(|p| p[0] + p[1]).sum();
    for (name, p) in PAIRS.iter().zip(&n.pairs) {
        println!(
            "| {name} | {} | {} | {} | {} |",
            p[0],
            p[1],
            p[0] + p[1],
            pct(p[0] + p[1], classified)
        );
    }
    let kin: u64 = n.pairs.iter().map(|p| p[1]).sum();
    println!(
        "| **all** | {} | {kin} | {classified} | {} kin |",
        classified - kin,
        pct(kin, classified)
    );
    println!("\nPer config (whole run):\n");
    println!(
        "| config | connections | mean live connections per agent-sample | formed | dissolved | % producer–decomposer | % producer–producer | % kin |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows {
        let n = &r.tally.ext.network;
        let classified: u64 = n.pairs.iter().map(|p| p[0] + p[1]).sum();
        let at = |i: usize| n.pairs.get(i).map_or(0, |p| p[0] + p[1]);
        let kin: u64 = n.pairs.iter().map(|p| p[1]).sum();
        println!(
            "| {}:{} | {} | {} | {} | {} | {} | {} | {} |",
            r.source,
            r.config_index,
            n.connections,
            per(2.0 * n.connections as f64, n.agent_samples),
            n.formed,
            n.dissolved,
            pct(at(pair(0, 2)), classified),
            pct(at(pair(0, 0)), classified),
            pct(kin, classified),
        );
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

/// Midranks, 1-based: tied values share the mean of the ranks they span.
fn midranks(v: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..v.len()).collect();
    order.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
    let mut ranks = vec![0.0; v.len()];
    let mut i = 0;
    while i < order.len() {
        let mut j = i + 1;
        while j < order.len() && v[order[j]] == v[order[i]] {
            j += 1;
        }
        // Positions i..j hold ranks i + 1 ..= j.
        let mid = (i + 1 + j) as f64 / 2.0;
        for &k in &order[i..j] {
            ranks[k] = mid;
        }
        i = j;
    }
    ranks
}

/// Spearman's ρ of `[x, y]` pairs: Pearson's correlation of the midranks
/// (exact under ties). `None` below three pairs or when either side is
/// constant.
fn spearman(pairs: &[[f32; 2]]) -> Option<f64> {
    if pairs.len() < 3 {
        return None;
    }
    let side = |i: usize| -> Vec<f64> {
        midranks(&pairs.iter().map(|p| f64::from(p[i])).collect::<Vec<_>>())
    };
    let (x, y) = (side(0), side(1));
    let mean = (pairs.len() as f64 + 1.0) / 2.0;
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(&y) {
        let (dx, dy) = (a - mean, b - mean);
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    (sxx > 0.0 && syy > 0.0).then(|| sxy / (sxx * syy).sqrt())
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let m = v.len() / 2;
    Some(if v.len() % 2 == 1 {
        v[m]
    } else {
        (v[m - 1] + v[m]) / 2.0
    })
}

/// A config's ρ counts toward the per-config median only on at least this
/// many samples.
const MIN_CONFIG_SAMPLES: usize = 10;

/// G (#655): the trend of Producer h_eff against the pool nutrient at the
/// cell, over second-half Producer-role agent-samples.
#[derive(Clone, Debug, PartialEq)]
struct Conditionality {
    /// Samples pooled across every config and seed.
    samples: usize,
    /// Spearman ρ(h_eff, pool) over all of them.
    pooled: Option<f64>,
    /// Each config's ρ over its own seeds' samples, in input order; `None`
    /// below [`MIN_CONFIG_SAMPLES`] or when undefined.
    per_config: Vec<Option<f64>>,
    /// Configs with a ρ, and of them those with ρ < 0.
    configs: usize,
    negative: usize,
    /// Median of the per-config ρ.
    median: Option<f64>,
}

impl Conditionality {
    fn of(per_config: &[&[[f32; 2]]]) -> Self {
        let pooled: Vec<[f32; 2]> = per_config.iter().flat_map(|c| c.iter().copied()).collect();
        let rhos: Vec<Option<f64>> = per_config
            .iter()
            .map(|c| {
                if c.len() >= MIN_CONFIG_SAMPLES {
                    spearman(c)
                } else {
                    None
                }
            })
            .collect();
        let defined: Vec<f64> = rhos.iter().flatten().copied().collect();
        Self {
            samples: pooled.len(),
            pooled: spearman(&pooled),
            configs: defined.len(),
            negative: defined.iter().filter(|r| **r < 0.0).count(),
            median: median(defined),
            per_config: rhos,
        }
    }

    /// Heterotrophy falls as pool nutrient rises: the pooled ρ and the
    /// median per-config ρ both below zero. `None` when either is undefined.
    fn passes(&self) -> Option<bool> {
        Some(self.pooled? < 0.0 && self.median? < 0.0)
    }
}

/// H (#655): carcass structure drained, nutrient released (bound) and
/// nutrient retained, per primary recipient bucket ([`RECIPIENTS`] less the
/// memo).
#[derive(Clone, Debug, PartialEq)]
struct NicheShare {
    drained: [f64; MEMO],
    released: [f64; MEMO],
    retained: [f64; MEMO],
    /// Structure drained by carcass bites no bucket holds (the consumer, or
    /// the carcass, off the drain-time roster); `None` on rows without the
    /// all-events total.
    unattributed_drained: Option<f64>,
}

impl NicheShare {
    fn of(r: &Routes, all_drained: f64) -> Self {
        let take = |v: &Vec<f64>| -> [f64; MEMO] {
            let mut out = [0.0; MEMO];
            out.copy_from_slice(&v[..MEMO]);
            out
        };
        let drained = take(&r.carcass_drained);
        let attributed: f64 = drained.iter().sum();
        Self {
            drained,
            released: take(&r.carcass_bound),
            retained: take(&r.carcass_retained),
            unattributed_drained: (all_drained > 0.0 || attributed == 0.0)
                .then(|| (all_drained - attributed).max(0.0)),
        }
    }

    fn frac(v: &[f64; MEMO], b: usize) -> Option<f64> {
        let total: f64 = v.iter().sum();
        (total > 0.0).then(|| v[b] / total)
    }

    /// Bucket `b`'s share of the attributed carcass structure drained.
    fn structure_share(&self, b: usize) -> Option<f64> {
        Self::frac(&self.drained, b)
    }

    fn released_share(&self, b: usize) -> Option<f64> {
        Self::frac(&self.released, b)
    }

    fn retained_share(&self, b: usize) -> Option<f64> {
        Self::frac(&self.retained, b)
    }

    /// Light-fed mixotrophs drain strictly less than half the carcass
    /// structure.
    fn minority(&self) -> Option<bool> {
        self.structure_share(0).map(|s| s < 0.5)
    }
}

fn pct_of(x: Option<f64>) -> String {
    x.map_or("–".into(), |v| format!("{:.1}", 100.0 * v))
}

fn rho(x: Option<f64>) -> String {
    x.map_or("–".into(), |v| format!("{v:+.3}"))
}

fn pass(x: Option<bool>) -> &'static str {
    match x {
        Some(true) => "**PASS**",
        Some(false) => "**FAIL**",
        None => "**n/a**",
    }
}

fn conditionality_of(rows: &[Row]) -> Conditionality {
    let per: Vec<&[[f32; 2]]> = rows.iter().map(|r| &r.tally.ext.het_pool[..]).collect();
    Conditionality::of(&per)
}

/// #642's persisted seeds and nutrient-lockup seeds of 475 at b = 0
/// (`docs/research/645-size-scaled-uptake.md` §8), for decoded rows and
/// rows at founder_aggregation = 0, over `seeds` seeds of #642's atlas.
/// `Err` says why it does not apply: rows from another atlas (#656: a fresh
/// atlas can have 95 configs too, so the seed count alone cannot tell), or
/// other or mixed pins, or another seed count.
fn reference(rows: &[Row], seeds: u64) -> Result<(&'static str, u64, u64), String> {
    let other_atlas = rows.iter().any(|r| {
        r.source != ConfigSource::Atlas || r.atlas.as_deref().is_some_and(|a| a != REFERENCE_ATLAS)
    });
    if other_atlas {
        return Err("no reference (different atlas)".into());
    }
    let needs = || format!("no #642 reference (needs {REFERENCE_SEEDS} seeds, decoded or fa 0)");
    let Some(fa) = rows.first().map(|r| r.founder_aggregation) else {
        return Err(needs());
    };
    if seeds != REFERENCE_SEEDS || rows.iter().any(|r| r.founder_aggregation != fa) {
        return Err(needs());
    }
    match fa {
        None => Ok(("decoded", 403, 40)),
        Some(a) if a == 0.0 => Ok(("fa 0", 423, 41)),
        _ => Err(needs()),
    }
}

const LOCKUP: &str = "nutrient_lockup";
const REFERENCE_SEEDS: u64 = 475;
/// #642's atlas (the committed `atlas.json` from #545 until #663 replaced
/// it, 95 cells over the size-blind box): its
/// [`AtlasUnits::fingerprint`](explorers_search::sweep::AtlasUnits::fingerprint)
/// in hex.
const REFERENCE_ATLAS: &str = "e12caad9b8f2a5a2";

/// The pass/fail block at the top of the report (#655).
fn verdict(rows: &[Row], t: &Tally) {
    let g = conditionality_of(rows);
    let whole = NicheShare::of(&t.ext.routes, t.ext.carcass_drained_all);
    let second = NicheShare::of(
        &t.ext.routes_second_half,
        t.ext.carcass_drained_all_second_half,
    );
    let minority = match (whole.minority(), second.minority()) {
        (Some(a), Some(b)) => Some(a && b),
        _ => None,
    };
    let (seeds, persisted, lockup) = (
        t.outcomes.seeds(),
        t.outcomes.persisted(),
        t.outcomes.count(LOCKUP),
    );
    println!("### Verdict (#655)\n");
    println!(
        "Conditionality passes when the pooled and the median per-config Spearman ρ(h_eff, pool N at cell) are both < 0 (G). Minority share passes when light-fed mixotrophs drain < 50 % of the attributed carcass structure, whole run and second half (H). Persistence against #642 (b = 0, 475 seeds): within 2 % when |Δ| ≤ 2 % of the reference.\n"
    );
    println!("| test | verdict | reading |");
    println!("|---|---|---|");
    println!(
        "| conditionality: Producer h_eff falls as pool N rises | {} | pooled ρ = {} (n = {} second-half Producer agent-samples, all configs × seeds); median per-config ρ = {} over {} configs ({} with ρ < 0) |",
        pass(g.passes()),
        rho(g.pooled),
        g.samples,
        rho(g.median),
        g.configs,
        g.negative
    );
    println!(
        "| minority share: light-fed mixotrophs drain < 50 % of carcass structure | {} | mixotroph share of carcass structure: whole run {} %, second half {} % |",
        pass(minority),
        pct_of(whole.structure_share(0)),
        pct_of(second.structure_share(0))
    );
    let (p_ref, l_ref) = match reference(rows, seeds) {
        Ok((name, p, l)) => {
            let d = persisted as i64 - p as i64;
            (
                format!(
                    "#642 {name}: {p}; Δ = {d:+} ({:+.1} %), within 2 %: {}",
                    100.0 * d as f64 / p as f64,
                    if d.unsigned_abs() as f64 <= 0.02 * p as f64 {
                        "yes"
                    } else {
                        "no"
                    }
                ),
                format!(
                    "#642 {name}: {l}; Δ = {:+}, rising: {}",
                    lockup as i64 - l as i64,
                    if lockup > l { "yes" } else { "no" }
                ),
            )
        }
        Err(why) => (why, "–".into()),
    };
    println!(
        "| persisted seeds | {persisted} / {seeds} ({} %) | {p_ref} |",
        pct(persisted, seeds)
    );
    println!("| nutrient lockup seeds | {lockup} | {l_ref} |\n");
}

/// G (#655): conditionality.
fn conditionality(rows: &[Row], x: &Ext) {
    let g = conditionality_of(rows);
    println!(
        "\n### G. Conditionality (#655): Producer h_eff against the pool nutrient at the cell (second half)\n"
    );
    println!(
        "Samples: B's second-half Producer-role agent-samples (sample ticks, the sample's recent-income role) with a pre-step pool reading, each as (h_eff, pool N in the nutrient-grid cell under the agent before the step). Spearman ρ: Pearson's correlation of midranks (ties share their mean rank). **Pooled**: every such sample across all {} configs and all their seeds in one ranking (configs at different nutrient levels share it). **Per config**: each config's seeds pooled, ρ where it has ≥ {MIN_CONFIG_SAMPLES} samples and both sides vary; the median over those configs, so a few dense configs cannot carry it. Negative ρ = heterotrophy falls as pool nutrient rises.\n",
        rows.len()
    );
    println!(
        "Pooled: n = {}, ρ = **{}**. Per config: {} configs with a ρ, median ρ = **{}**, ρ < 0 in {} of them.\n",
        g.samples,
        rho(g.pooled),
        g.configs,
        rho(g.median),
        g.negative
    );
    println!("Mean pool N at the cell by h_eff bin (B's bins):\n");
    println!("| h_eff bin | samples with a reading | mean pool N at cell |");
    println!("|---|---:|---:|");
    for (name, b) in HET_BINS.iter().zip(&x.producer_bins) {
        let n = b.samples - b.missing_pre;
        println!("| {name} | {n} | {} |", per(b.pool, n));
    }
    println!("\nPer config:\n");
    println!("| config | samples | ρ(h_eff, pool N) |");
    println!("|---|---:|---:|");
    for (r, c) in rows.iter().zip(&g.per_config) {
        println!(
            "| {}:{} | {} | {} |",
            r.source,
            r.config_index,
            r.tally.ext.het_pool.len(),
            rho(*c)
        );
    }
}

/// H (#655): who drains carcass structure, and who gets its nutrient.
fn niche_share(x: &Ext) {
    println!(
        "\n### H. Niche share (#655): carcass structure drained and nutrient released, by role\n"
    );
    println!(
        "Recipient by recent-income role at the start of the tick and drain-time h_eff (C's buckets). Structure: `Consumed.energy_delta` on carcass bites; released: the nutrient bound in what was drained (`realised_bites`' bound nutrient); retained: what the consumer kept under the retention cap (reported, not deciding). Shares are of the attributed total; unattributed: carcass bites `realised_bites` drops (consumer or carcass off the drain-time roster), as a share of all carcass structure drained.\n"
    );
    for (title, r, all) in [
        ("Whole run", &x.routes, x.carcass_drained_all),
        (
            "Second half",
            &x.routes_second_half,
            x.carcass_drained_all_second_half,
        ),
    ] {
        let s = NicheShare::of(r, all);
        println!("#### H. {title}\n");
        println!(
            "| recipient (role at start of tick) | structure drained | % structure | N released | % N released | N retained | % N retained |"
        );
        println!("|---|---:|---:|---:|---:|---:|---:|");
        for (b, name) in RECIPIENTS[..MEMO].iter().enumerate() {
            println!(
                "| {name} | {:.1} | {} | {:.2} | {} | {:.2} | {} |",
                s.drained[b],
                pct_of(s.structure_share(b)),
                s.released[b],
                pct_of(s.released_share(b)),
                s.retained[b],
                pct_of(s.retained_share(b)),
            );
        }
        let attributed: f64 = s.drained.iter().sum();
        println!(
            "| **all attributed** | {attributed:.1} | 100 | {:.2} | 100 | {:.2} | 100 |",
            s.released.iter().sum::<f64>(),
            s.retained.iter().sum::<f64>()
        );
        match s.unattributed_drained {
            Some(u) => println!(
                "\nUnattributed structure drained: {u:.1} ({} % of all {all:.1}).\n",
                share(u, all)
            ),
            None => println!("\nUnattributed structure drained: unrecorded (pre-#655 rows).\n"),
        }
    }
}

/// The cross-trait term the stepper charges `c_AH` against
/// (`phase::metabolic_cost`, world-rules.md trade-off #5): `(A·H)^(p/2)` on
/// the raw autotrophy `A` and heterotrophy `H`, `p` the config's
/// `maintenance_cost_exponent`. The same expression as the sim's (which is
/// crate-private), so `c_AH × term` is what the agent would pay.
fn cross_trait_term(a: f32, h: f32, p: f32) -> f32 {
    (a * h).powf(0.5 * p)
}

/// Linear-interpolation quantile (type 7) of an ascending slice; agrees
/// with [`median`] at `q = 0.5`.
fn quantile(sorted: &[f64], q: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let x = q * (sorted.len() - 1) as f64;
    let (lo, hi) = (x.floor() as usize, x.ceil() as usize);
    Some(sorted[lo] + (x - lo as f64) * (sorted[hi] - sorted[lo]))
}

/// `(n, p25, median, p75)` of `v`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Spread {
    n: usize,
    p25: f64,
    median: f64,
    p75: f64,
}

impl Spread {
    fn of(mut v: Vec<f64>) -> Option<Self> {
        v.sort_by(f64::total_cmp);
        Some(Self {
            n: v.len(),
            p25: quantile(&v, 0.25)?,
            median: quantile(&v, 0.5)?,
            p75: quantile(&v, 0.75)?,
        })
    }
}

/// The cost multiple `c_max` is set at: a typical light-fed mixotroph's
/// cross-trait cost is this many times its drain income (world-rules.md,
/// trade-off #5).
const COST_MULTIPLE: f64 = 2.0;

/// I (#668): the scale of `c_AH`'s search range, over one entry per light-fed
/// mixotroph per seed (`Ext::lfm_calibration`: `[mean drain energy income per
/// tick in the bucket, (A·H)^(p/2), ticks]`).
#[derive(Clone, Debug, PartialEq)]
struct Calibration {
    agents: usize,
    /// Agents with no drain income on any of their ticks in the bucket.
    zero_income: usize,
    income: Option<Spread>,
    term: Option<Spread>,
    /// Memo: Σ income × ticks / Σ ticks — the agent-tick-weighted mean.
    income_per_agent_tick: Option<f64>,
    /// (a) `2 × median(income) / median(term)`; `None` when the median term
    /// is zero.
    c_max_ratio_of_medians: Option<f64>,
    /// (b) the median over agents with a non-zero term of `2 × income_i /
    /// term_i`.
    c_max_median_of_ratios: Option<f64>,
    /// Agents (b) reads (term > 0).
    ratio_agents: usize,
    /// (a) and (b) over the agents with drain income > 0 only (the median
    /// income is 0 once half the population never drains); `None` when every
    /// agent drains.
    drainers: Option<Box<Calibration>>,
}

impl Calibration {
    fn of(agents: &[[f32; 3]]) -> Self {
        let income: Vec<f64> = agents.iter().map(|a| f64::from(a[0])).collect();
        let term: Vec<f64> = agents.iter().map(|a| f64::from(a[1])).collect();
        let ticks: f64 = agents.iter().map(|a| f64::from(a[2])).sum();
        let weighted: f64 = agents
            .iter()
            .map(|a| f64::from(a[0]) * f64::from(a[2]))
            .sum();
        let ratios: Vec<f64> = agents
            .iter()
            .filter(|a| a[1] > 0.0)
            .map(|a| COST_MULTIPLE * f64::from(a[0]) / f64::from(a[1]))
            .collect();
        let (income, term) = (Spread::of(income), Spread::of(term));
        let drainers: Vec<[f32; 3]> = agents.iter().filter(|a| a[0] > 0.0).copied().collect();
        Self {
            drainers: (drainers.len() < agents.len()).then(|| Box::new(Calibration::of(&drainers))),
            agents: agents.len(),
            zero_income: agents.iter().filter(|a| a[0] <= 0.0).count(),
            income_per_agent_tick: (ticks > 0.0).then(|| weighted / ticks),
            c_max_ratio_of_medians: match (income, term) {
                (Some(i), Some(t)) if t.median > 0.0 => Some(COST_MULTIPLE * i.median / t.median),
                _ => None,
            },
            ratio_agents: ratios.len(),
            c_max_median_of_ratios: median(ratios),
            income,
            term,
        }
    }
}

fn spread_cells(s: Option<Spread>) -> String {
    s.map_or("– | – | – | 0".into(), |s| {
        format!("{:.4} | {:.4} | {:.4} | {}", s.p25, s.median, s.p75, s.n)
    })
}

fn g4(x: Option<f64>) -> String {
    x.map_or("–".into(), |v| format!("{v:.4}"))
}

/// The cross-trait cost the rows ran at, each distinct value listed.
fn cross_trait_label(rows: &[Row]) -> String {
    let mut v: Vec<String> = Vec::new();
    for r in rows {
        let s = r
            .cross_trait_cost
            .map_or("unrecorded (pre-#668 row: 0)".into(), |c| format!("{c}"));
        if !v.contains(&s) {
            v.push(s);
        }
    }
    format!(
        "Cross-trait cost (#667): c_AH = **{}**",
        if v.is_empty() {
            "–".into()
        } else {
            v.join(" / ")
        }
    )
}

/// I (#668): calibration of `c_AH`'s range.
fn calibration(rows: &[Row], x: &Ext) {
    println!(
        "\n### I. Calibration (#668): light-fed mixotrophs' drain energy income against the cross-trait term (second half)\n"
    );
    if x.lfm_calibration.is_empty() {
        println!(
            "No calibration entries on these rows (pre-#668 rows, or no light-fed mixotroph in the second half)."
        );
        return;
    }
    let c = Calibration::of(&x.lfm_calibration);
    println!(
        "Unit: **one light-fed mixotroph per seed** (an agent-lifetime), over the second-half ticks it spent in C's light-fed mixotroph bucket (Producer by recent-income role at the start of the tick, drain-time h_eff > 0.05). Income: its mean drain energy gained per such tick (`realised_bites`' energy, living and carcass; a tick without a drain counts 0). Term: `(A·H)^(p/2)` on its raw photosynthetic absorption A and heterotrophy H, p = its config's `maintenance_cost_exponent` (the expression `phase::metabolic_cost` charges `c_AH` against). Quantiles interpolate linearly. c_max: the c_AH at which a typical light-fed mixotroph's cross-trait cost `c_AH × term` is {COST_MULTIPLE}× its drain income (world-rules.md, trade-off #5) — (a) **ratio of medians** `{COST_MULTIPLE} × median(income) / median(term)`; (b) **median of per-agent ratios** `median_i({COST_MULTIPLE} × income_i / term_i)` over agents with term > 0.\n"
    );
    println!("| quantity | p25 | median | p75 | agents |");
    println!("|---|---:|---:|---:|---:|");
    println!(
        "| drain energy income per tick | {} |",
        spread_cells(c.income)
    );
    println!("| (A·H)^(p/2) | {} |", spread_cells(c.term));
    println!(
        "\nAgents with zero drain income: {} of {} ({} %). Memo, agent-tick-weighted mean income per tick: {}.\n",
        c.zero_income,
        c.agents,
        pct(c.zero_income as u64, c.agents as u64),
        g4(c.income_per_agent_tick)
    );
    println!(
        "c_max (a) ratio of medians = **{}**; (b) median of per-agent ratios = **{}** (over {} agents with term > 0).\n",
        g4(c.c_max_ratio_of_medians),
        g4(c.c_max_median_of_ratios),
        c.ratio_agents
    );
    let d = c.drainers.as_deref().cloned().unwrap_or_else(|| c.clone());
    println!(
        "Drainers only (the {} agents with drain income > 0): income median {} (p25 {}, p75 {}), term median {}; c_max (a) = **{}**, (b) = **{}**.\n",
        d.agents,
        g4(d.income.map(|s| s.median)),
        g4(d.income.map(|s| s.p25)),
        g4(d.income.map(|s| s.p75)),
        g4(d.term.map(|s| s.median)),
        g4(d.c_max_ratio_of_medians),
        g4(d.c_max_median_of_ratios),
    );
    println!("Per config:\n");
    println!(
        "| config | p | agents | median income | median term | c_max (a) | c_max (b) | drainers | c_max (a), drainers | c_max (b), drainers |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows {
        let c = Calibration::of(&r.tally.ext.lfm_calibration);
        let d = c.drainers.as_deref().cloned().unwrap_or_else(|| c.clone());
        println!(
            "| {}:{} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r.source,
            r.config_index,
            r.maintenance_cost_exponent
                .map_or("–".into(), |p| format!("{p}")),
            c.agents,
            g4(c.income.map(|s| s.median)),
            g4(c.term.map(|s| s.median)),
            g4(c.c_max_ratio_of_medians),
            g4(c.c_max_median_of_ratios),
            d.agents,
            g4(d.c_max_ratio_of_medians),
            g4(d.c_max_median_of_ratios),
        );
    }
}

/// J (#668): carcass nutrient light-fed mixotrophs retain, and whether the
/// share retained depends on the pool under them.
#[derive(Clone, Debug, PartialEq)]
struct Retention {
    bites: usize,
    drained: f64,
    retained: f64,
    /// Bites that drained no structure (a spent carcass), and what they
    /// retained; they carry no ratio, so ρ leaves them out.
    zero_structure_bites: usize,
    zero_structure_retained: f64,
    /// Spearman ρ(retained / drained, pool N) per bite; `Conditionality`'s
    /// pooled and per-config machinery.
    rho: Conditionality,
}

impl Retention {
    fn of(per_config: &[&[[f32; 3]]]) -> Self {
        let pairs: Vec<Vec<[f32; 2]>> = per_config
            .iter()
            .map(|c| {
                c.iter()
                    .filter(|b| b[0] > 0.0)
                    .map(|b| [b[1] / b[0], b[2]])
                    .collect()
            })
            .collect();
        let refs: Vec<&[[f32; 2]]> = pairs.iter().map(|p| &p[..]).collect();
        let all = per_config.iter().flat_map(|c| c.iter());
        let zero = per_config
            .iter()
            .flat_map(|c| c.iter())
            .filter(|b| b[0] <= 0.0);
        Self {
            zero_structure_bites: zero.clone().count(),
            zero_structure_retained: zero.map(|b| f64::from(b[1])).sum(),
            bites: per_config.iter().map(|c| c.len()).sum(),
            drained: all.clone().map(|b| f64::from(b[0])).sum(),
            retained: all.map(|b| f64::from(b[1])).sum(),
            rho: Conditionality::of(&refs),
        }
    }
}

fn retention(rows: &[Row], x: &Ext) {
    println!("\n### J. Carcass nutrient retained by light-fed mixotrophs (#668, second half)\n");
    let ticks: u64 = x.lfm_calibration.iter().map(|a| a[2] as u64).sum();
    let total = x.routes_second_half.carcass_retained[0];
    println!(
        "From C's second-half routes (bucket: light-fed mixotroph): carcass nutrient retained **{total:.2}**, carcass structure drained {:.1} (retained per unit drained {}); per second-half tick run (Σ over seeds, {} ticks) {}; per light-fed mixotroph agent-tick ({ticks} agent-ticks, I's unit) {}.\n",
        x.routes_second_half.carcass_drained[0],
        ratio(total, x.routes_second_half.carcass_drained[0]),
        x.second_half_ticks,
        per(total, x.second_half_ticks),
        per(total, ticks),
    );
    if x.lfm_carcass_bites.is_empty() {
        println!(
            "No per-bite entries on these rows (pre-#668 rows, or no light-fed mixotroph carcass bite in the second half)."
        );
        return;
    }
    let per_config: Vec<&[[f32; 3]]> = rows
        .iter()
        .map(|r| &r.tally.ext.lfm_carcass_bites[..])
        .collect();
    let j = Retention::of(&per_config);
    println!(
        "Per bite (every second-half carcass bite by a light-fed mixotroph that `realised_bites` values; {} without a pool reading left out): {} bites, drained {:.1}, retained {:.2}, retained per unit drained {}; of them {} bites drained no structure (a spent carcass's nutrient) and retained {:.2}, and carry no ratio. Spearman ρ(retained / drained, pool N in the cell under the drainer before the step), as G computes it: pooled ρ = **{}** (n = {}); per config (≥ {MIN_CONFIG_SAMPLES} bites) median ρ = **{}** over {} configs ({} with ρ < 0). Negative ρ = the drainer keeps less of a carcass's nutrient where the pool under it is richer.\n",
        x.lfm_carcass_bites_no_pool,
        j.bites,
        j.drained,
        j.retained,
        ratio(j.retained, j.drained),
        j.zero_structure_bites,
        j.zero_structure_retained,
        rho(j.rho.pooled),
        j.rho.samples,
        rho(j.rho.median),
        j.rho.configs,
        j.rho.negative,
    );
    println!("| config | bites | retained | retained / drained | ρ(retained / drained, pool N) |");
    println!("|---|---:|---:|---:|---:|");
    for (r, c) in rows.iter().zip(&j.rho.per_config) {
        let b = &r.tally.ext.lfm_carcass_bites;
        let (d, n): (f64, f64) = b.iter().fold((0.0, 0.0), |(d, n), x| {
            (d + f64::from(x[0]), n + f64::from(x[1]))
        });
        println!(
            "| {}:{} | {} | {n:.2} | {} | {} |",
            r.source,
            r.config_index,
            b.len(),
            ratio(n, d),
            rho(*c)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn midranks_share_the_mean_rank_among_ties() {
        assert_eq!(midranks(&[3.0, 1.0, 2.0]), vec![3.0, 1.0, 2.0]);
        // The two 0s hold ranks 1 and 2, the three 5s ranks 3–5.
        assert_eq!(
            midranks(&[5.0, 0.0, 5.0, 0.0, 5.0]),
            vec![4.0, 1.5, 4.0, 1.5, 4.0]
        );
    }

    #[test]
    fn spearman_reads_monotone_trends_whatever_their_shape() {
        // h_eff falling as the pool rises, non-linearly: ρ = −1.
        let falling: Vec<[f32; 2]> = (1..=6).map(|i| [1.0 / i as f32, (i * i) as f32]).collect();
        assert!(close(spearman(&falling).unwrap(), -1.0));
        let rising: Vec<[f32; 2]> = (1..=6).map(|i| [i as f32, (i * i * i) as f32]).collect();
        assert!(close(spearman(&rising).unwrap(), 1.0));
    }

    #[test]
    fn spearman_with_ties_is_pearson_on_midranks() {
        // h = [0, 0, 1, 1], pool = [1, 2, 3, 4]: ranks h = [1.5, 1.5, 3.5,
        // 3.5], pool = [1, 2, 3, 4]; Pearson = 4 / √(4 × 5) = 0.8944.
        let pairs = [[0.0, 1.0], [0.0, 2.0], [1.0, 3.0], [1.0, 4.0]];
        assert!(close(spearman(&pairs).unwrap(), 4.0 / 20f64.sqrt()));
    }

    #[test]
    fn spearman_is_undefined_when_too_few_or_constant() {
        assert_eq!(spearman(&[]), None);
        assert_eq!(spearman(&[[0.0, 1.0], [1.0, 2.0]]), None);
        assert_eq!(spearman(&[[0.1, 1.0], [0.1, 2.0], [0.1, 3.0]]), None);
        assert_eq!(spearman(&[[0.1, 1.0], [0.2, 1.0], [0.3, 1.0]]), None);
    }

    #[test]
    fn median_averages_the_middle_pair() {
        assert_eq!(median(vec![]), None);
        assert_eq!(median(vec![3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(vec![4.0, 1.0, 3.0, 2.0]), Some(2.5));
    }

    /// `n` pairs on which h_eff falls (or rises) with the pool, the pool
    /// offset so configs sit at different nutrient levels.
    fn trend(n: usize, offset: f32, falling: bool) -> Vec<[f32; 2]> {
        (0..n)
            .map(|i| {
                let h = i as f32 / n as f32;
                [if falling { 1.0 - h } else { h }, offset + i as f32]
            })
            .collect()
    }

    #[test]
    fn conditionality_pools_every_sample_and_takes_the_median_per_config() {
        let a = trend(20, 0.0, true);
        let b = trend(20, 100.0, true);
        let c = trend(20, 200.0, false);
        // Below MIN_CONFIG_SAMPLES: pooled, but no per-config ρ.
        let d = trend(MIN_CONFIG_SAMPLES - 1, 0.0, false);
        let g = Conditionality::of(&[&a, &b, &c, &d]);
        assert_eq!(g.samples, 20 * 3 + MIN_CONFIG_SAMPLES - 1);
        let pooled: Vec<[f32; 2]> = [&a, &b, &c, &d].into_iter().flatten().copied().collect();
        assert!(close(g.pooled.unwrap(), spearman(&pooled).unwrap()));
        assert_eq!(g.per_config, vec![Some(-1.0), Some(-1.0), Some(1.0), None]);
        assert_eq!((g.configs, g.negative), (3, 2));
        assert_eq!(g.median, Some(-1.0));
        assert_eq!(g.passes(), Some(g.pooled.unwrap() < 0.0));
    }

    #[test]
    fn conditionality_passes_only_when_pooled_and_median_both_fall() {
        let fall = trend(20, 0.0, true);
        let rise = trend(20, 0.0, false);
        assert_eq!(Conditionality::of(&[&fall, &fall]).passes(), Some(true));
        assert_eq!(Conditionality::of(&[&rise, &rise]).passes(), Some(false));
        assert_eq!(Conditionality::of(&[]).passes(), None);
        // One dense falling config cannot carry two rising ones: the pooled
        // ρ falls, the median rises.
        let dense = trend(2000, 0.0, true);
        let g = Conditionality::of(&[&dense, &rise, &rise]);
        assert!(g.pooled.unwrap() < 0.0);
        assert_eq!(g.median, Some(1.0));
        assert_eq!(g.passes(), Some(false));
    }

    fn routes(drained: [f64; 5], released: [f64; 5], retained: [f64; 5]) -> Routes {
        let mut r = Routes::default();
        r.carcass_drained[..MEMO].copy_from_slice(&drained);
        r.carcass_bound[..MEMO].copy_from_slice(&released);
        r.carcass_retained[..MEMO].copy_from_slice(&retained);
        // The memo overlaps the first two buckets and must not count again.
        r.carcass_drained[MEMO] = 1e6;
        r.carcass_bound[MEMO] = 1e6;
        r
    }

    #[test]
    fn niche_share_divides_carcass_structure_by_role() {
        let r = routes(
            [30.0, 10.0, 40.0, 15.0, 5.0],
            [3.0, 1.0, 4.0, 1.5, 0.5],
            [2.0, 1.0, 1.0, 0.0, 0.0],
        );
        let s = NicheShare::of(&r, 110.0);
        assert!(close(s.structure_share(0).unwrap(), 0.3));
        assert!(close(s.structure_share(2).unwrap(), 0.4));
        assert!(close(s.released_share(3).unwrap(), 0.15));
        assert!(close(s.retained_share(0).unwrap(), 0.5));
        assert!(close(s.unattributed_drained.unwrap(), 10.0));
        assert_eq!(s.minority(), Some(true));
        let majority = NicheShare::of(
            &routes([60.0, 10.0, 20.0, 10.0, 0.0], [0.0; 5], [0.0; 5]),
            100.0,
        );
        assert!(close(majority.structure_share(0).unwrap(), 0.6));
        assert_eq!(majority.minority(), Some(false));
        // Exactly half is not a minority.
        let half = NicheShare::of(
            &routes([50.0, 0.0, 50.0, 0.0, 0.0], [0.0; 5], [0.0; 5]),
            100.0,
        );
        assert_eq!(half.minority(), Some(false));
    }

    #[test]
    fn niche_share_is_undefined_without_drains_and_unrecorded_on_old_rows() {
        let none = NicheShare::of(&Routes::default(), 0.0);
        assert_eq!(none.structure_share(0), None);
        assert_eq!(none.minority(), None);
        // A pre-#655 row has drains but no all-events total.
        let old = NicheShare::of(&routes([1.0, 0.0, 1.0, 0.0, 0.0], [0.0; 5], [0.0; 5]), 0.0);
        assert_eq!(old.unattributed_drained, None);
        assert_eq!(old.minority(), Some(false));
    }

    #[test]
    fn rows_without_the_new_fields_still_read_back() {
        let mut t = Tally::new();
        t.ext.het_pool.push([0.3, 2.0]);
        t.ext.carcass_drained_all = 5.0;
        let mut json: serde_json::Value = serde_json::to_value(&t).unwrap();
        let ext = json["ext"].as_object_mut().unwrap();
        for k in [
            "het_pool",
            "carcass_drained_all",
            "carcass_drained_all_second_half",
        ] {
            assert!(ext.remove(k).is_some(), "{k} is serialised");
        }
        let back: Tally = serde_json::from_value(json).unwrap();
        assert!(back.ext.het_pool.is_empty());
        assert_eq!(back.ext.carcass_drained_all, 0.0);
    }
    fn row(atlas: Option<&str>, fa: Option<f32>) -> Row {
        Row {
            source: ConfigSource::Atlas,
            config_index: 0,
            horizon: 2000,
            base_seed: 1000,
            founder_aggregation: fa,
            satiation_sensitivity: None,
            uptake_structure_exponent: None,
            uptake_reference_structure: None,
            network: None,
            atlas: atlas.map(str::to_string),
            cross_trait_cost: None,
            maintenance_cost_exponent: None,
            seed_kin_kills: Vec::new(),
            tally: Tally::new(),
        }
    }

    /// #656: #642's persistence reference applies only to 475 seeds of
    /// #642's atlas, decoded or at fa 0. A fresh atlas of 95 configs also
    /// has 475 seeds, so the rows' atlas decides; rows from before the
    /// fingerprint ran on #642's atlas.
    #[test]
    fn the_642_reference_applies_only_on_its_own_atlas() {
        let old = [row(None, None), row(Some(REFERENCE_ATLAS), None)];
        assert_eq!(reference(&old, 475), Ok(("decoded", 403, 40)));
        let fa0 = [row(None, Some(0.0))];
        assert_eq!(reference(&fa0, 475), Ok(("fa 0", 423, 41)));
        assert!(reference(&old, 10).unwrap_err().contains("needs 475"));
        let mixed = [row(None, None), row(None, Some(0.0))];
        assert!(reference(&mixed, 475).is_err());
        let fresh = [row(Some("0123456789abcdef"), None)];
        assert_eq!(
            reference(&fresh, 475),
            Err("no reference (different atlas)".to_string())
        );
        let mut sampled = row(None, None);
        sampled.source = ConfigSource::SAMPLE;
        assert_eq!(
            reference(&[sampled], 475),
            Err("no reference (different atlas)".to_string())
        );
    }

    /// #668: the term `c_AH` is charged against is `(A·H)^(p/2)`, of degree
    /// p like the per-trait terms, and zero for a specialist.
    #[test]
    fn the_cross_trait_term_is_of_degree_p() {
        assert!(close(f64::from(cross_trait_term(4.0, 1.0, 2.0)), 4.0));
        // p = 3: (2 × 8)^1.5 = 64.
        assert!(close(f64::from(cross_trait_term(2.0, 8.0, 3.0)), 64.0));
        // p = 1: √(0.25 × 0.04) = 0.1.
        assert!((cross_trait_term(0.25, 0.04, 1.0) - 0.1).abs() < 1e-6);
        assert_eq!(cross_trait_term(0.0, 3.0, 1.5), 0.0);
        assert_eq!(cross_trait_term(3.0, 0.0, 2.5), 0.0);
    }

    #[test]
    fn quantiles_interpolate_and_agree_with_the_median() {
        let v = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(quantile(&v, 0.5), median(v.to_vec()));
        assert!(close(quantile(&v, 0.25).unwrap(), 1.75));
        assert!(close(quantile(&v, 0.75).unwrap(), 3.25));
        assert_eq!(quantile(&[], 0.5), None);
        assert_eq!(quantile(&[7.0], 0.25), Some(7.0));
    }

    /// c_max two ways: the ratio of medians, and the median of per-agent
    /// ratios (agents with a zero term left out of the latter only).
    #[test]
    fn c_max_is_read_as_a_ratio_of_medians_and_a_median_of_ratios() {
        // [income per tick, term, ticks].
        let agents = [
            [1.0, 1.0, 10.0],
            [2.0, 4.0, 10.0],
            [3.0, 0.5, 20.0],
            [0.0, 2.0, 60.0],
            [5.0, 0.0, 1.0],
        ];
        let c = Calibration::of(&agents);
        assert_eq!((c.agents, c.zero_income, c.ratio_agents), (5, 1, 4));
        // income [0, 1, 2, 3, 5] median 2; term [0, 0.5, 1, 2, 4] median 1.
        assert!(close(c.income.unwrap().median, 2.0));
        assert!(close(c.term.unwrap().median, 1.0));
        assert!(close(c.c_max_ratio_of_medians.unwrap(), 4.0));
        // 2 × income / term over term > 0: [2, 1, 12, 0] -> median 1.5.
        assert!(close(c.c_max_median_of_ratios.unwrap(), 1.5));
        // Weighted: (10 + 20 + 60 + 0 + 5) / 101.
        assert!(close(c.income_per_agent_tick.unwrap(), 95.0 / 101.0));
        // With p ≠ 2 the term comes from cross_trait_term at that p.
        let term = cross_trait_term(0.5, 0.5, 3.0);
        let one = Calibration::of(&[[0.25, term, 4.0]]);
        assert!(close(
            one.c_max_ratio_of_medians.unwrap(),
            2.0 * 0.25 / f64::from(term)
        ));
        let none = Calibration::of(&[]);
        assert_eq!(
            (
                none.income,
                none.c_max_ratio_of_medians,
                none.c_max_median_of_ratios
            ),
            (None, None, None)
        );
        // Drainers only: income [1, 2, 3, 5] median 2.5; term [1, 4, 0.5, 0]
        // median 0.75; ratios over term > 0 [2, 1, 12] median 2.
        let d = c.drainers.as_deref().unwrap();
        assert_eq!(d.agents, 4);
        assert!(close(d.c_max_ratio_of_medians.unwrap(), 2.0 * 2.5 / 0.75));
        assert!(close(d.c_max_median_of_ratios.unwrap(), 2.0));
        assert!(d.drainers.is_none(), "every drainer drains");
        let zero_term = Calibration::of(&[[1.0, 0.0, 1.0]]);
        assert_eq!(zero_term.c_max_ratio_of_medians, None);
        assert_eq!(zero_term.c_max_median_of_ratios, None);
    }

    /// J: retained per unit drained against the pool, per bite, through
    /// G's pooled and per-config ρ.
    #[test]
    fn retention_ranks_retained_per_unit_drained_against_the_pool() {
        // Retained share falls as the pool rises: ρ = −1 in each config.
        let falling: Vec<[f32; 3]> = (0..12)
            .map(|i| [2.0, 1.0 / (1.0 + i as f32), i as f32])
            .collect();
        let rising: Vec<[f32; 3]> = (0..12)
            .map(|i| [1.0, i as f32 / 12.0, 100.0 + i as f32])
            .collect();
        let zero_drain = [[0.0, 0.0, 5.0]];
        let j = Retention::of(&[&falling, &rising, &zero_drain]);
        assert_eq!(j.bites, 25);
        assert!(close(j.drained, 36.0));
        assert_eq!(j.zero_structure_bites, 1);
        let spent = Retention::of(&[&[[0.0, 0.75, 1.0], [1.0, 0.25, 2.0]][..]]);
        assert_eq!(spent.zero_structure_bites, 1);
        assert!(close(spent.zero_structure_retained, 0.75));
        assert!(close(spent.retained, 1.0));
        assert_eq!(j.rho.samples, 24, "a zero drain has no ratio");
        assert_eq!(j.rho.per_config, vec![Some(-1.0), Some(1.0), None]);
        assert_eq!(j.rho.median, Some(0.0));
    }

    /// Rows from before #668 (target/656) read back with I and J empty.
    #[test]
    fn rows_without_the_668_fields_still_read_back() {
        let mut r = row(Some("0123456789abcdef"), None);
        r.cross_trait_cost = Some(0.5);
        r.maintenance_cost_exponent = Some(2.0);
        r.tally.ext.lfm_calibration.push([1.0, 2.0, 3.0]);
        r.tally.ext.lfm_carcass_bites.push([1.0, 0.5, 4.0]);
        r.tally.ext.lfm_carcass_bites_no_pool = 1;
        r.tally.ext.second_half_ticks = 9;
        let mut json = serde_json::to_value(&r).unwrap();
        let o = json.as_object_mut().unwrap();
        for k in ["cross_trait_cost", "maintenance_cost_exponent"] {
            assert!(o.remove(k).is_some(), "{k} is serialised");
        }
        let ext = json["tally"]["ext"].as_object_mut().unwrap();
        for k in [
            "lfm_calibration",
            "lfm_carcass_bites",
            "lfm_carcass_bites_no_pool",
            "second_half_ticks",
        ] {
            assert!(ext.remove(k).is_some(), "{k} is serialised");
        }
        let back: Row = serde_json::from_value(json).unwrap();
        assert_eq!(back.cross_trait_cost, None);
        assert_eq!(back.maintenance_cost_exponent, None);
        assert!(back.tally.ext.lfm_calibration.is_empty());
        assert!(back.tally.ext.lfm_carcass_bites.is_empty());
        assert_eq!(back.tally.ext.second_half_ticks, 0);
        assert!(cross_trait_label(&[back]).contains("pre-#668"));
    }
}
