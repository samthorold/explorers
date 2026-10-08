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
//!
//! #681 reads what `c_AH` charges, by trophic role, so the paired read can
//! tell whether the cost is what thins decomposers. K. **Charge by role**,
//! on every second-half agent-sample, for decomposers by role and for
//! producer → decomposer intermediates (mixotrophs by investment, raw A and
//! H both > 0.05, whose role is not producer): the autotrophy carried (mean,
//! median), the charge `c_AH × (A·H)^(p/2)` (`phase::cross_trait_charge`) as
//! a fraction of income (light plus drain energy received), and the group's
//! share of agent-samples; per config and pooled, beside C's second-half
//! carcass structure drained per seed by decomposers and light-fed
//! mixotrophs and the mixotrophs' share. Run the same worlds at their own
//! `c_AH` and at `--cross-trait-cost 0`. `--crosscheck` also reconciles each
//! agent-tick's charge with the stepper's `Metabolized` charge less its
//! per-trait cost. Rows from before #681 read back with K empty.
//!
//! #682 opens the report with **flow 1's verdicts** as world-rules.md flow 1
//! states them (`explorers_search::flow1_verdict`), for the mode the rows ran
//! in: conditionality's three tests on G's per-config ρ (median < 0; the
//! configs with ρ < 0 differ from a coin's, two-sided exact binomial p <
//! 0.05; a majority of 8 Ward lineage clusters over the atlas's unit
//! coordinates have a median < 0), PASS / FAIL, and the fallback's pooled ρ
//! ≤ 0; and the minority share (H, second half, < 50 %). In summary mode
//! `--atlas` names the atlas the rows ran on (the clusters; without it, or
//! on another atlas's rows, test 3 is not read), and `--baseline <jsonl>`
//! an earlier run's rows: the share is then classified by absolute drains
//! per seed as mixotroph excess or decomposer deficit.
//!
//!   cargo run --release -p explorers-search --example kin_killer_diet -- \
//!       --summary --out target/670/seed43/kkd-decoded.jsonl \
//!       --atlas target/670/atlas-seed43.json --baseline target/668/kkd-decoded.jsonl
//!
//! #683 adds L. **Mobile autotrophy** (world-rules.md flow 2: substrate
//! contact is held in reserve until mobile autotrophy is measured viable):
//! producers by role (the sample's recent-income role) on second-half sample
//! ticks, by effective mobility. The distribution of their effective mobility
//! and of their realised movement per tick (quantiles), and at each candidate
//! threshold (`MOBILE_THRESHOLDS`) the share of producer samples above it,
//! the second-half births by such producers and their distinct parents, and
//! the seeds where they **hold** as the heterotroph guild does: ≥
//! `GUILD_MIN_SIZE` on every second-half sample tick of a run that reached the
//! horizon, plus at least one such birth. Per config and pooled; rows from
//! before #683 read back with L empty. `scripts/683-census.sh` drives it.
//!
//! #700 adds M. **Carcass leaching calibration** (world-rules.md, *Carcass
//! energy decays only through agents; carcass nutrient leaches*: the top of
//! λ's range is set by a calibration read). Per seed, a
//! `explorers_search::leaching::LeachRun`: every carcass formed in the run
//! timed from death to the first bite any drainer takes (censored at the
//! run's end), tagged *bitten to death* when it was drained while living in
//! the step it died; carcass nutrient out by leaching against drains; the
//! carcass-locked fraction off the lockup gate's series, on every run; and
//! H's second-half light-fed mixotroph share. M1 pools the time to first
//! drain (quantiles over the drained, the Kaplan–Meier median with the
//! undrained censored), whole run and settled half, by how the carcass died;
//! M2 the λ_max = ln 2 / t* readings; M3 per config; M4 the sweep's
//! quantities at the rows' λ. `--leaching-rate <λ>` pins λ on every decoded
//! config (unset, each keeps its decoded value: 0). `--leaching-sweep
//! a.jsonl,b.jsonl,…` prints M4's line for each rows file as one table and
//! exits. `PreStep` replays the tick's leaching, so C–J read the drain-time
//! carcasses and pool at λ > 0. `scripts/700-leaching.sh` drives it.
//!
//! #728 reads hyphal uptake (#727; world-rules.md flow 2, *Hyphal uptake*)
//! for the partner the symbiosis needs (#654's reading rule).
//! `--hyphal-uptake on|off` and `--contact-distance <d_c>` pin the switch and
//! `d_c` on every decoded config (unset, each keeps its decoded value: off,
//! `DEFAULT_CONTACT_DISTANCE`); rows record the effective values (absent on
//! older rows: off). `PreStep` replays uptake with the world's last move
//! distances, so C–M read the stepper's uptake with the switch on. N.
//! **Partner**, per run (`PartnerRun`): on every settled-half sample tick, by
//! the sample's recent-income role, each agent's free nutrient ÷ reserve —
//! free nutrient is its unbound nutrient (the unearmarked free store plus the
//! reproductive earmark, not the nutrient bound in structure), reserve its
//! whole reserve (unallocated plus the reproductive allocation); agents with
//! no reserve are left out and counted. Over heterotrophs by role (consumer
//! or decomposer) and producers by role, the median of each and the sample
//! counts; on a persisted run with both, their ratio, the **gradient**. A
//! world (a config over its seeds) has as its gradient the median over its
//! qualifying runs, and carries heterotrophs by role when it has at least
//! one; the rest are counted by why they are out. The **hyphal share**: the
//! heterotrophs' (C's buckets) settled-half hyphal uptake over their pool
//! uptake plus nutrient retained from consumption, per run, per world and
//! pooled; 0 with the switch off. The section counts the worlds carrying
//! heterotrophs by role and those with gradient > 1 (a majority or not),
//! and with `--baseline <switch-off rows>` the paired sign test on each
//! world's gradient > 1 indicator (a world not carrying reads 0; worlds pair
//! by config), exact binomial over the discordant pairs, two-sided and
//! one-sided (on > off). Section N is new and printed last, so an off run's
//! other sections are unchanged.
//!
//!   cargo run --release -p explorers-search --example kin_killer_diet -- \
//!       --atlas atlas.json --hyphal-uptake on --out target/728/on.jsonl
//!   cargo run --release -p explorers-search --example kin_killer_diet -- \
//!       --summary --out target/728/on.jsonl --baseline target/728/off.jsonl
//!
//! #740 reads which currency limits growth, by role (#736's reading rule,
//! item 1, complementary limitation; Daufresne & Loreau 2001, Sterner &
//! Elser 2002, Kiers et al. 2011). O. **Growth limitation**, per run
//! (`LimitationRun`): on every settled-half sample tick, each agent's growth
//! event (`grazer_hunger::GrowthLimit`: the grow phase had surplus to
//! mobilise) off the metabolised roster, by the sample's recent-income role
//! (producers, consumers, decomposers): energy- or nutrient-limited by
//! Liebig's law, a tie reading nutrient-limited (the stepper's tie rule),
//! and the free nutrient left unbound on energy-limited events. A world (a
//! config, its events pooled over its seeds) passes item 1 when producers'
//! nutrient-limited share exceeds decomposers' and decomposers'
//! energy-limited share exceeds producers'; the section counts the worlds
//! carrying both roles, those passing, and whether that is a majority. No
//! flag: it reads every run, network on or off (off, it is the rule's
//! baseline). Section O is new and printed last; rows from before #740 read
//! back with no limitation runs.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use rayon::prelude::*;

use explorers_genesis_eval::guild::GUILD_MIN_SIZE;
use explorers_genesis_eval::{EvalConfig, RolloutObservations};
use explorers_search::config_source::{
    ConfigSource, NETWORK_FLAGS, NetworkPins, NetworkRecord, parse_founder_aggregation,
    parse_non_negative, parse_positive, parse_selector, parse_unit_interval, resolve_config,
    sampled_units, with_cross_trait_cost, with_founder_aggregation, with_hyphal_uptake,
    with_leaching_rate, with_network, with_uptake_scaling,
};
use explorers_search::flow1_verdict::{
    COIN_ALPHA, Conditionality as Flow1Conditionality, DrainShift, Drains, LINEAGE_CLUSTERS,
    MINORITY_BAR, binomial_two_sided_p, binomial_upper_tail_p, minority, ward_clusters,
};
use explorers_search::fullness::{FullnessBank, FullnessTracker, k_grid, tau_grid, tick_intakes};
use explorers_search::grazer_hunger::{GrowthLimit, PreStep};
use explorers_search::intake_ceiling::realised_bites;
use explorers_search::leaching::{
    CarcassNutrient, DrainClock, FirstDrain, LeachRun, SweepReading, lambda_max,
};
use explorers_search::role_diet::{
    DietLedger, Outcomes, census_failure, failure_label, rollout_with_fullness,
};
use explorers_search::sweep::{
    AtlasUnits, append_row, done_configs, plan_tasks, read_atlas_units, read_rows,
};
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
    /// The hyphal part of `uptake` (#728; `DrainStart::hyphal`). Zero with
    /// hyphal uptake off, and on pre-#728 rows.
    #[serde(default = "recipient_zeros")]
    hyphal: Vec<f64>,
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
            hyphal: z.clone(),
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
        add(&mut self.hyphal, &o.hyphal);
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
    /// K (#681): per [`CHARGE_GROUPS`] group, one entry per agent per seed
    /// over the second-half ticks it spent in the group (role at the start of
    /// the tick, drain-time roster), as `[raw autotrophy A, cross-trait charge
    /// per tick, ticks, Σ energy income on them]`; income is light plus drain
    /// energy received (`TickIntake::energy`).
    #[serde(default)]
    charge: [Vec<[f32; 4]>; 2],
    /// K: every agent on the drain-time roster on every second-half tick,
    /// summed over seeds (the groups' share denominator).
    #[serde(default)]
    second_half_agent_ticks: u64,
    /// L (#683): mobile autotrophy. Empty on pre-#683 rows.
    #[serde(default)]
    mobile: MobileCensus,
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
        for (a, b) in self.charge.iter_mut().zip(&o.charge) {
            a.extend_from_slice(b);
        }
        self.second_half_agent_ticks += o.second_half_agent_ticks;
        self.mobile.merge(&o.mobile);
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
    /// M (#700): one leaching readout per seed, in seed order. Empty on
    /// pre-#700 rows.
    #[serde(default)]
    leach_runs: Vec<LeachRun>,
    /// N (#728): one partner readout per seed, in seed order. Empty on
    /// pre-#728 rows.
    #[serde(default)]
    partner_runs: Vec<PartnerRun>,
    /// O (#740): one growth-limitation readout per seed, in seed order.
    /// Empty on pre-#740 rows.
    #[serde(default)]
    limitation_runs: Vec<LimitationRun>,
    /// `--crosscheck` (#681): K's charges against the stepper's. Not on the
    /// rows.
    #[serde(skip)]
    charge_check: ChargeCheck,
}

/// #681's crosscheck, over every agent-tick of a run: the stepper's
/// cross-trait charge — its `Metabolized` charge less the per-trait cost
/// (`phase::metabolic_cost` at c_AH = 0) — against the readout's
/// [`cross_trait_charge`]. A starving agent's charge is capped at its reserve,
/// so its cross-trait part cannot be told apart; it is counted, not compared.
#[derive(Clone, Debug, Default)]
struct ChargeCheck {
    agent_ticks: u64,
    starved: u64,
    /// Σ charge, the stepper's and the readout's, over the compared ticks.
    stepper: f64,
    readout: f64,
    /// Compared ticks whose charges differ beyond f32 rounding.
    mismatches: u64,
}

impl ChargeCheck {
    fn merge(&mut self, o: &ChargeCheck) {
        self.agent_ticks += o.agent_ticks;
        self.starved += o.starved;
        self.stepper += o.stepper;
        self.readout += o.readout;
        self.mismatches += o.mismatches;
    }
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
        self.leach_runs.extend_from_slice(&o.leach_runs);
        self.partner_runs.extend_from_slice(&o.partner_runs);
        self.limitation_runs.extend_from_slice(&o.limitation_runs);
        self.charge_check.merge(&o.charge_check);
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
    crosscheck: bool,
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
        EventKind::Leached,
    ]
    .into_iter()
    .chain(crosscheck.then_some(EventKind::Metabolized))
    {
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
    // K (#681): per group, per agent, `[A, charge per tick, ticks, Σ income]`.
    let mut charge: [HashMap<u64, [f64; 4]>; 2] = Default::default();
    let without_cross_trait = WorldParameters {
        cross_trait_cost: 0.0,
        ..params.clone()
    };
    let mut live_connections: HashSet<(u64, u64)> = HashSet::new();
    // L (#683): mobile autotrophy, and each agent's (Σ distance moved, ticks)
    // since the last sample tick, over second-half ticks.
    let mut mobile = MobileTracker::default();
    let mut moved: HashMap<u64, (f64, u32)> = HashMap::new();
    // M (#700): each carcass timed to its first drain, and carcass nutrient
    // out by route (whole run and second half).
    let mut drain_clock = DrainClock::default();
    let (mut leach_n, mut leach_n_second_half) =
        (CarcassNutrient::default(), CarcassNutrient::default());
    // N (#728): free nutrient ÷ reserve by role group, settled sample ticks.
    let mut partner = PartnerSamples::default();
    // O (#740): growth events by role and limiting currency, settled sample
    // ticks.
    let mut limitation = LimitationRun::default();
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
        // L: each agent's pre-step effective mobility (a parent's, at birth).
        let pre_mob: HashMap<u64, f32> = if second_half_tick {
            world
                .agents()
                .iter()
                .map(|a| (a.id, a.effective_trait_with_steepness(2, steep)))
                .collect()
        } else {
            HashMap::new()
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
        let carcasses_before = world.carcasses().to_vec();
        world.step();
        let tail: Vec<Event> = world.event_log().since(cursor).to_vec();
        cursor = world.event_log().len();
        drain_clock.observe(world.tick(), &tail);
        {
            let mut step = CarcassNutrient::default();
            step.observe(&carcasses_before, world.carcasses(), &tail);
            leach_n.merge(&step);
            if world.tick() >= window_start {
                leach_n_second_half.merge(&step);
            }
        }
        ledger.ingest(&world, &tail, Some(&pre));
        let mut kin_killers = Vec::new();
        for (g, heterotrophy, _) in ledger.take_kin_kill_readings() {
            if heterotrophy > 0.0 && observations.income().role(g) == Some(TrophicRole::Producer) {
                kin_killers.push(g);
            }
        }
        let tick = world.tick();
        let second_half = tick >= window_start;
        if second_half {
            let before: HashMap<u64, (f32, f32)> =
                pre.agents().iter().map(|a| (a.id, a.position)).collect();
            for a in world.agents() {
                if let Some(&from) = before.get(&a.id) {
                    let m = moved.entry(a.id).or_default();
                    m.0 += f64::from(toroidal_distance(from, a.position, params.world_extent));
                    m.1 += 1;
                }
            }
        }
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
                for (&id, &u) in &start.hyphal {
                    if let Some((b, memo)) = bucket_of(id) {
                        r.hyphal[b] += f64::from(u);
                        if memo {
                            r.hyphal[MEMO] += f64::from(u);
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
        // K. Each agent's cross-trait charge and income by group.
        if second_half {
            for a in &start.agents {
                tally.ext.second_half_agent_ticks += 1;
                let role = roles.get(&a.id).copied().flatten();
                let (aa, h) = (a.traits.photosynthetic_absorption, a.traits.heterotrophy);
                let income = intakes.get(&a.id).map_or(0.0, |i| f64::from(i.energy()));
                for (g, member) in charge_groups(role, aa, h).into_iter().enumerate() {
                    if member {
                        let e = charge[g].entry(a.id).or_insert_with(|| {
                            [
                                f64::from(aa),
                                f64::from(cross_trait_charge(aa, h, params)),
                                0.0,
                                0.0,
                            ]
                        });
                        e[2] += 1.0;
                        e[3] += income;
                    }
                }
            }
        }
        if crosscheck {
            let charged: HashMap<u64, f32> = tail
                .iter()
                .filter(|e| e.kind == EventKind::Metabolized)
                .map(|e| (e.source, e.energy_delta))
                .collect();
            let check = &mut tally.charge_check;
            for a in pre.agents() {
                let Some(&event) = charged.get(&a.id) else {
                    continue;
                };
                check.agent_ticks += 1;
                let full = phase::metabolic_cost(a, params);
                let available = a.reserve + start.light.get(&a.id).copied().unwrap_or(0.0);
                if available < full {
                    check.starved += 1;
                    continue;
                }
                let stepper = event - phase::metabolic_cost(a, &without_cross_trait);
                let readout = cross_trait_charge(
                    a.traits.photosynthetic_absorption,
                    a.traits.heterotrophy,
                    params,
                );
                check.stepper += f64::from(stepper);
                check.readout += f64::from(readout);
                if (stepper - readout).abs() > 1e-5 * full.max(1.0) {
                    check.mismatches += 1;
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
                        mobile.birth(p, pre_mob.get(&p).copied().unwrap_or(0.0));
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
            let earmark_fills = if second_half {
                pre.earmark_fills(params)
            } else {
                Vec::new()
            };
            for (a, _) in &earmark_fills {
                if let Some(limit) = GrowthLimit::of(a, params) {
                    limitation.record(sample_roles.get(&a.id).copied(), limit);
                }
            }
            let fills: HashMap<u64, (f32, explorers_search::grazer_hunger::Surplus)> =
                if second_half {
                    earmark_fills
                        .into_iter()
                        .map(|(a, fill)| {
                            let s = explorers_search::grazer_hunger::Surplus::of(&a, params);
                            (a.id, (fill, s))
                        })
                        .collect()
                } else {
                    HashMap::new()
                };
            if second_half {
                let producers: Vec<(u64, f32, Option<f32>)> = world
                    .agents()
                    .iter()
                    .filter(|a| sample_roles.get(&a.id) == Some(&TrophicRole::Producer))
                    .map(|a| {
                        (
                            a.id,
                            a.effective_trait_with_steepness(2, steep),
                            moved
                                .get(&a.id)
                                .filter(|m| m.1 > 0)
                                .map(|m| (m.0 / f64::from(m.1)) as f32),
                        )
                    })
                    .collect();
                mobile.sample(&producers);
                moved.clear();
            }
            if second_half {
                for a in world.agents() {
                    partner.sample(sample_roles.get(&a.id).copied(), a);
                }
            }
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
    tally.ext.mobile = mobile.finish(stopped.is_none() && world.tick() >= max_ticks);
    let failure = census_failure(&world, &observations, eval_config, max_ticks, stopped);
    tally.outcomes.record(failure.as_ref());
    tally.termination_ticks = world.tick();
    tally.leach_runs.push(leach_run(
        seed,
        &world,
        &observations,
        eval_config,
        window_start,
        failure.as_ref(),
        [leach_n, leach_n_second_half],
        &tally.ext.routes_second_half,
        drain_clock,
    ));
    {
        let r = &tally.ext.routes_second_half;
        let het = |v: &[f64]| v[2] + v[3];
        tally.partner_runs.push(partner.finish(
            seed,
            failure.as_ref().map_or("persisted", failure_label),
            [
                het(&r.hyphal),
                het(&r.uptake),
                het(&r.living_retained) + het(&r.carcass_retained),
            ],
        ));
    }
    limitation.seed = seed;
    limitation.outcome = failure
        .as_ref()
        .map_or("persisted", failure_label)
        .to_string();
    tally.limitation_runs.push(limitation);
    for (g, by_agent) in charge.into_iter().enumerate() {
        let mut by_agent: Vec<(u64, [f64; 4])> = by_agent.into_iter().collect();
        by_agent.sort_by_key(|(id, _)| *id);
        tally.ext.charge[g] = by_agent
            .into_iter()
            .map(|(_, e)| e.map(|v| v as f32))
            .collect();
    }
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

/// O (#740): the roles growth limitation is read for, by the sample's
/// recent-income role; agents with no role yet are left out.
const LIMIT_ROLES: [&str; 3] = ["producers", "consumers", "decomposers"];
const LIMIT_PRODUCER: usize = 0;
const LIMIT_CONSUMER: usize = 1;
const LIMIT_DECOMPOSER: usize = 2;

/// O (#740): one seed's growth events (`GrowthLimit`) on the settled half's
/// sample ticks, per role ([`LIMIT_ROLES`]): how many, how many nutrient-
/// limited (the rest are energy-limited), and Σ the free nutrient left
/// unbound on the energy-limited ones.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct LimitationRun {
    seed: u64,
    /// The census's verdict (`persisted` or a failure label).
    outcome: String,
    events: [u64; 3],
    nutrient_limited: [u64; 3],
    free_nutrient_left: [f64; 3],
}

impl LimitationRun {
    fn record(&mut self, role: Option<TrophicRole>, limit: GrowthLimit) {
        let r = match role {
            Some(TrophicRole::Producer) => LIMIT_PRODUCER,
            Some(TrophicRole::Consumer) => LIMIT_CONSUMER,
            Some(TrophicRole::Decomposer) => LIMIT_DECOMPOSER,
            None => return,
        };
        self.events[r] += 1;
        match limit {
            GrowthLimit::Nutrient => self.nutrient_limited[r] += 1,
            GrowthLimit::Energy { free_nutrient } => {
                self.free_nutrient_left[r] += f64::from(free_nutrient)
            }
        }
    }
}

/// O (#740): one world's (one config's, over every one of its seeds)
/// growth events, pooled per role.
#[derive(Clone, Debug)]
struct LimitationWorld {
    source: ConfigSource,
    config_index: usize,
    runs: usize,
    events: [u64; 3],
    nutrient_limited: [u64; 3],
    free_nutrient_left: [f64; 3],
}

impl LimitationWorld {
    fn of(row: &Row) -> Self {
        let mut w = LimitationWorld {
            source: row.source,
            config_index: row.config_index,
            runs: row.tally.limitation_runs.len(),
            events: [0; 3],
            nutrient_limited: [0; 3],
            free_nutrient_left: [0.0; 3],
        };
        for r in &row.tally.limitation_runs {
            for i in 0..LIMIT_ROLES.len() {
                w.events[i] += r.events[i];
                w.nutrient_limited[i] += r.nutrient_limited[i];
                w.free_nutrient_left[i] += r.free_nutrient_left[i];
            }
        }
        w
    }

    fn energy_limited(&self, role: usize) -> u64 {
        self.events[role] - self.nutrient_limited[role]
    }

    /// The role's share of growth events that are nutrient-limited; `None`
    /// without any.
    fn nutrient_share(&self, role: usize) -> Option<f64> {
        let n = self.events[role];
        (n > 0).then(|| self.nutrient_limited[role] as f64 / n as f64)
    }

    fn energy_share(&self, role: usize) -> Option<f64> {
        let n = self.events[role];
        (n > 0).then(|| self.energy_limited(role) as f64 / n as f64)
    }

    /// Mean free nutrient left unbound per energy-limited growth event.
    fn mean_free_nutrient_left(&self, role: usize) -> Option<f64> {
        let n = self.energy_limited(role);
        (n > 0).then(|| self.free_nutrient_left[role] / n as f64)
    }

    /// Both roles the rule compares grew: producers and decomposers by role
    /// each have at least one growth event.
    fn carries_both(&self) -> bool {
        self.events[LIMIT_PRODUCER] > 0 && self.events[LIMIT_DECOMPOSER] > 0
    }

    /// #736's item 1, complementary limitation: producers nutrient-limited in
    /// a larger share than decomposers, and decomposers energy-limited in a
    /// larger share than producers.
    fn passes(&self) -> bool {
        let (p, d) = (LIMIT_PRODUCER, LIMIT_DECOMPOSER);
        match (
            self.nutrient_share(p),
            self.nutrient_share(d),
            self.energy_share(p),
            self.energy_share(d),
        ) {
            (Some(pn), Some(dn), Some(pe), Some(de)) => pn > dn && de > pe,
            _ => false,
        }
    }
}

/// O (#740): the rows' worlds, counted against item 1.
#[derive(Clone, Debug)]
struct LimitationTally {
    worlds: usize,
    carrying: usize,
    passing: usize,
}

impl LimitationTally {
    fn of(rows: &[Row]) -> Self {
        let worlds: Vec<LimitationWorld> = rows.iter().map(LimitationWorld::of).collect();
        LimitationTally {
            worlds: worlds.len(),
            carrying: worlds.iter().filter(|w| w.carries_both()).count(),
            passing: worlds.iter().filter(|w| w.passes()).count(),
        }
    }

    /// More than half of the worlds carrying both roles pass item 1.
    fn majority(&self) -> bool {
        2 * self.passing > self.carrying
    }
}

/// O (#740): the growth-limitation readout as markdown — the worlds counted
/// against #736's item 1, then per world and per run.
fn limitation_report(rows: &[Row]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let o = &mut out;
    let fmt = |v: Option<f64>| v.map_or("–".to_string(), |v| format!("{v:.4}"));
    writeln!(
        o,
        "\n### O. Growth limitation: which currency limits growth, by role (#740)\n"
    )
    .unwrap();
    writeln!(o, "{}\n", network_label(rows)).unwrap();
    writeln!(
        o,
        "A **growth event** is an agent-tick on which the grow phase had surplus to mobilise (reserve above the retention buffer), read on every settled-half sample tick (tick ≥ T/2 + 1) off the metabolised roster, by the sample's recent-income role (agents with no role yet are left out). By Liebig's law the structure built is the smaller of what the soma energy affords and what the free (unearmarked) nutrient store supports: the event is **nutrient-limited** when the free store capped it (growth bound all of it; a tie between the caps reads nutrient-limited, the stepper's own tie rule) and **energy-limited** otherwise. **Free N left**: the mean free nutrient the grow phase left unbound per energy-limited event. A **world** is one config, its growth events pooled over all its seeds. It **carries both roles** when producers and decomposers by role each have a growth event, and **passes item 1** (#736, complementary limitation; Daufresne & Loreau 2001, Kiers et al. 2011) when producers' nutrient-limited share exceeds decomposers' and decomposers' energy-limited share exceeds producers'.\n"
    )
    .unwrap();
    let t = LimitationTally::of(rows);
    writeln!(
        o,
        "Worlds carrying both producers and decomposers by role: **{}** of {}; of them, passing item 1: **{}** of {} ({} %), a majority: **{}**.\n",
        t.carrying,
        t.worlds,
        t.passing,
        t.carrying,
        pct(t.passing as u64, t.carrying as u64),
        if t.majority() { "yes" } else { "no" }
    )
    .unwrap();
    let worlds: Vec<LimitationWorld> = rows.iter().map(LimitationWorld::of).collect();
    let runs: Vec<&LimitationRun> = rows.iter().flat_map(|r| &r.tally.limitation_runs).collect();
    writeln!(
        o,
        "Runs with a readout: {} ({} rows' seeds predate #740).\n",
        runs.len(),
        rows.iter()
            .filter(|r| r.tally.limitation_runs.is_empty())
            .map(|r| r.seed_kin_kills.len())
            .sum::<usize>(),
    )
    .unwrap();
    writeln!(
        o,
        "Per world, each role as growth events / % nutrient-limited / % energy-limited:\n"
    )
    .unwrap();
    writeln!(
        o,
        "| config | seeds | producers | consumers | decomposers | free N left (P / C / D) | item 1 |"
    )
    .unwrap();
    writeln!(o, "|---|---:|---:|---:|---:|---:|---|").unwrap();
    for w in &worlds {
        let role = |r: usize| {
            format!(
                "{} / {} / {}",
                w.events[r],
                pct(w.nutrient_limited[r], w.events[r]),
                pct(w.energy_limited(r), w.events[r])
            )
        };
        let left: Vec<String> = (0..LIMIT_ROLES.len())
            .map(|r| fmt(w.mean_free_nutrient_left(r)))
            .collect();
        writeln!(
            o,
            "| {}:{} | {} | {} | {} | {} | {} | {} |",
            w.source,
            w.config_index,
            w.runs,
            role(LIMIT_PRODUCER),
            role(LIMIT_CONSUMER),
            role(LIMIT_DECOMPOSER),
            left.join(" / "),
            if !w.carries_both() {
                "not carrying both"
            } else if w.passes() {
                "yes"
            } else {
                "no"
            },
        )
        .unwrap();
    }
    writeln!(
        o,
        "\nPer run, each role as growth events / nutrient-limited:\n"
    )
    .unwrap();
    writeln!(
        o,
        "| config | seed | verdict | producers | consumers | decomposers |"
    )
    .unwrap();
    writeln!(o, "|---|---:|---|---:|---:|---:|").unwrap();
    for r in rows {
        for l in &r.tally.limitation_runs {
            let role = |i: usize| format!("{} / {}", l.events[i], l.nutrient_limited[i]);
            writeln!(
                o,
                "| {}:{} | {} | {} | {} | {} | {} |",
                r.source,
                r.config_index,
                l.seed,
                l.outcome,
                role(LIMIT_PRODUCER),
                role(LIMIT_CONSUMER),
                role(LIMIT_DECOMPOSER),
            )
            .unwrap();
        }
    }
    out
}

/// An agent's **free nutrient ÷ reserve** (#728): its unbound nutrient (the
/// unearmarked free store plus the reproductive earmark; not the nutrient
/// bound in structure) over its whole reserve (unallocated plus the
/// reproductive allocation). `None` when it holds no reserve.
fn free_nutrient_per_reserve(a: &explorers_sim::Agent) -> Option<f32> {
    let reserve = a.reserve + a.repro_reserve;
    (reserve > 0.0).then(|| (a.nutrient + a.repro_nutrient) / reserve)
}

/// N (#728): one run's free nutrient ÷ reserve samples, by role group, over
/// the settled half's sample ticks (the sample's recent-income role).
#[derive(Clone, Debug, Default)]
struct PartnerSamples {
    heterotroph: Vec<f64>,
    producer: Vec<f64>,
    zero_reserve: u64,
}

impl PartnerSamples {
    /// One agent-sample: heterotroph by role (consumer or decomposer) or
    /// producer by role; an agent with no role yet is in neither group.
    fn sample(&mut self, role: Option<TrophicRole>, a: &explorers_sim::Agent) {
        let group = match role {
            Some(TrophicRole::Consumer | TrophicRole::Decomposer) => &mut self.heterotroph,
            Some(TrophicRole::Producer) => &mut self.producer,
            None => return,
        };
        match free_nutrient_per_reserve(a) {
            Some(r) => group.push(f64::from(r)),
            None => self.zero_reserve += 1,
        }
    }

    /// The run's readout; `[hyphal, pool uptake, consumption]` is the
    /// heterotrophs' settled-half nutrient income by route.
    fn finish(self, seed: u64, outcome: &str, [hyphal, uptake, consumed]: [f64; 3]) -> PartnerRun {
        PartnerRun {
            seed,
            outcome: outcome.to_string(),
            heterotroph_samples: self.heterotroph.len() as u64,
            producer_samples: self.producer.len() as u64,
            zero_reserve: self.zero_reserve,
            heterotroph: median(self.heterotroph),
            producer: median(self.producer),
            hyphal_n: hyphal,
            uptake_n: uptake,
            consumed_n: consumed,
        }
    }
}

/// N (#728): one seed's partner readout.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct PartnerRun {
    seed: u64,
    /// The census's verdict (`persisted` or a failure label).
    outcome: String,
    /// Settled-half agent-samples per group, those with reserve.
    heterotroph_samples: u64,
    producer_samples: u64,
    /// Agent-samples in either group with no reserve, left out.
    zero_reserve: u64,
    /// Median free nutrient ÷ reserve per group (`None`: no samples).
    heterotroph: Option<f64>,
    producer: Option<f64>,
    /// Heterotrophs by role (C's consumer and decomposer buckets, role at the
    /// start of the tick), settled half: hyphal uptake, all pool uptake
    /// (hyphal and root), and nutrient retained from consumption (living and
    /// carcass drains).
    hyphal_n: f64,
    uptake_n: f64,
    consumed_n: f64,
}

impl PartnerRun {
    /// The **gradient**: heterotrophs' median over producers'. Only on a
    /// persisted run with both groups sampled; `None` also when both medians
    /// are 0 (an undefined ratio). A producer median of 0 under a positive
    /// heterotroph median reads as +∞.
    fn gradient(&self) -> Option<f64> {
        if self.outcome != "persisted" {
            return None;
        }
        let g = self.heterotroph? / self.producer?;
        (!g.is_nan()).then_some(g)
    }

    /// The hyphal share of the heterotrophs' nutrient income: hyphal uptake
    /// over all pool uptake plus consumption.
    fn hyphal_share(&self) -> Option<f64> {
        let income = self.uptake_n + self.consumed_n;
        (income > 0.0).then(|| self.hyphal_n / income)
    }
}

/// N (#728): one world's (one config's, over its seeds) partner readout.
/// Its gradient is the median of the per-run gradients over its persisted
/// runs carrying both roles; it **carries heterotrophs by role** when at
/// least one such run exists.
#[derive(Clone, Debug)]
struct PartnerWorld {
    source: ConfigSource,
    config_index: usize,
    runs: usize,
    persisted: usize,
    /// Persisted runs with no heterotroph-by-role / producer-by-role sample
    /// (a run lacking both counts in both).
    lacking_heterotroph: usize,
    lacking_producer: usize,
    /// Persisted runs with both groups at a median of 0 (0 / 0).
    undefined: usize,
    /// Per qualifying run, in seed order.
    gradients: Vec<f64>,
    gradient: Option<f64>,
    /// Heterotrophs' settled-half `[hyphal, pool uptake, consumption]` N,
    /// summed over every run.
    income: [f64; 3],
}

impl PartnerWorld {
    fn of(row: &Row) -> Self {
        let runs = &row.tally.partner_runs;
        let persisted: Vec<&PartnerRun> =
            runs.iter().filter(|r| r.outcome == "persisted").collect();
        let gradients: Vec<f64> = runs.iter().filter_map(PartnerRun::gradient).collect();
        let lacking_heterotroph = persisted.iter().filter(|r| r.heterotroph.is_none()).count();
        let lacking_producer = persisted.iter().filter(|r| r.producer.is_none()).count();
        let both = persisted
            .iter()
            .filter(|r| r.heterotroph.is_some() && r.producer.is_some())
            .count();
        let mut income = [0.0; 3];
        for r in runs {
            income[0] += r.hyphal_n;
            income[1] += r.uptake_n;
            income[2] += r.consumed_n;
        }
        PartnerWorld {
            source: row.source,
            config_index: row.config_index,
            runs: runs.len(),
            persisted: persisted.len(),
            lacking_heterotroph,
            lacking_producer,
            undefined: both - gradients.len(),
            gradient: median(gradients.clone()),
            gradients,
            income,
        }
    }

    fn carries(&self) -> bool {
        self.gradient.is_some()
    }

    /// The indicator the sign test pairs on: carries heterotrophs by role
    /// with a gradient above 1.
    fn above_one(&self) -> bool {
        self.gradient.is_some_and(|g| g > 1.0)
    }

    fn hyphal_share(&self) -> Option<f64> {
        let total = self.income[1] + self.income[2];
        (total > 0.0).then(|| self.income[0] / total)
    }
}

/// N (#728): the rows' worlds, counted.
#[derive(Clone, Debug)]
struct PartnerTally {
    worlds: usize,
    carrying: usize,
    above_one: usize,
}

impl PartnerTally {
    fn of(rows: &[Row]) -> Self {
        let worlds: Vec<PartnerWorld> = rows.iter().map(PartnerWorld::of).collect();
        PartnerTally {
            worlds: worlds.len(),
            carrying: worlds.iter().filter(|w| w.carries()).count(),
            above_one: worlds.iter().filter(|w| w.above_one()).count(),
        }
    }

    /// More than half of the worlds carrying heterotrophs by role have a
    /// gradient above 1.
    fn majority(&self) -> bool {
        2 * self.above_one > self.carrying
    }
}

/// N (#728): the paired sign test of the switch-on rows against a baseline
/// (switch-off) rows file, on each world's gradient > 1 indicator
/// ([`PartnerWorld::above_one`]; a world not carrying heterotrophs by role
/// reads 0). Worlds pair by config (source and index); a world in one file
/// only is counted, not paired. The test is exact, over the discordant pairs.
#[derive(Clone, Debug, Default)]
struct SignTest {
    pairs: usize,
    unpaired_on: usize,
    unpaired_off: usize,
    /// Pairs above 1 with the switch on only (k), off only, in both, in neither.
    on_only: usize,
    off_only: usize,
    both: usize,
    neither: usize,
}

impl SignTest {
    fn of(on: &[Row], off: &[Row]) -> Self {
        let key = |r: &Row| (r.source, r.config_index);
        let off_worlds: HashMap<(ConfigSource, usize), bool> = off
            .iter()
            .map(|r| (key(r), PartnerWorld::of(r).above_one()))
            .collect();
        let mut t = SignTest::default();
        let mut paired: HashSet<(ConfigSource, usize)> = HashSet::new();
        for r in on {
            let Some(&b) = off_worlds.get(&key(r)) else {
                t.unpaired_on += 1;
                continue;
            };
            paired.insert(key(r));
            t.pairs += 1;
            match (PartnerWorld::of(r).above_one(), b) {
                (true, false) => t.on_only += 1,
                (false, true) => t.off_only += 1,
                (true, true) => t.both += 1,
                (false, false) => t.neither += 1,
            }
        }
        t.unpaired_off = off_worlds.keys().filter(|k| !paired.contains(k)).count();
        t
    }

    fn discordant(&self) -> usize {
        self.on_only + self.off_only
    }

    /// `min(1, 2 P(X ≤ min(k, n − k)))`, `X ~ Bin(n, ½)` over the discordant.
    fn two_sided_p(&self) -> f64 {
        binomial_two_sided_p(self.on_only, self.discordant())
    }

    /// `P(X ≥ k)`: more worlds above 1 with the switch on than off.
    fn one_sided_p(&self) -> f64 {
        binomial_upper_tail_p(self.on_only, self.discordant())
    }
}

/// N (#728): the partner readout as markdown — per world and per run, the
/// worlds counted, the hyphal share, and, given a baseline (switch-off)
/// rows file, the paired sign test.
fn partner_report(rows: &[Row], baseline: Option<(&str, &[Row])>) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let o = &mut out;
    let fmt = |v: Option<f64>| v.map_or("–".to_string(), |v| format!("{v:.4}"));
    let share = |v: Option<f64>| v.map_or("–".to_string(), |v| format!("{:.2} %", 100.0 * v));
    writeln!(
        o,
        "\n### N. Partner: free nutrient ÷ reserve by role (#728)\n"
    )
    .unwrap();
    writeln!(o, "{}\n", hyphal_label(rows)).unwrap();
    writeln!(
        o,
        "**Free nutrient** is an agent's unbound nutrient: the unearmarked free store plus the reproductive earmark (not the nutrient bound in its structure). **Reserve** is its whole reserve: unallocated plus the reproductive allocation. Read on every settled-half sample tick (tick ≥ T/2 + 1, the census's role samples), by the sample's recent-income role: **heterotrophs by role** (consumer or decomposer) and **producers by role**; an agent with no role yet is in neither. Agent-samples with no reserve are left out and counted. Per run, the **gradient** is the heterotrophs' median free nutrient ÷ reserve over the producers', on a persisted run with at least one sample of each (a producer median of 0 under a positive heterotroph median reads ∞; 0 / 0 is left out and counted). A **world** is one config over its seeds: its gradient is the median over its qualifying runs, and it **carries heterotrophs by role** when it has at least one. **Hyphal share**: the heterotrophs' (C's consumer and decomposer buckets, role at the start of the tick) settled-half hyphal uptake over their pool uptake (hyphal and root) plus nutrient retained from consumption (living and carcass drains); the network is not counted. 0 with the switch off.\n"
    )
    .unwrap();
    let worlds: Vec<PartnerWorld> = rows.iter().map(PartnerWorld::of).collect();
    let t = PartnerTally::of(rows);
    let runs: Vec<&PartnerRun> = rows.iter().flat_map(|r| &r.tally.partner_runs).collect();
    let sum = |f: &dyn Fn(&PartnerWorld) -> usize| -> usize { worlds.iter().map(f).sum() };
    writeln!(
        o,
        "Worlds carrying heterotrophs by role: **{}** of {}; of them, gradient > 1: **{}** of {} ({} %), a majority: **{}**.\n",
        t.carrying,
        t.worlds,
        t.above_one,
        t.carrying,
        pct(t.above_one as u64, t.carrying as u64),
        if t.majority() { "yes" } else { "no" }
    )
    .unwrap();
    let samples: u64 = runs
        .iter()
        .map(|r| r.heterotroph_samples + r.producer_samples)
        .sum();
    let zero: u64 = runs.iter().map(|r| r.zero_reserve).sum();
    writeln!(
        o,
        "Runs: {} with a readout ({} rows' seeds predate #728), {} persisted, {} qualifying; persisted runs lacking heterotrophs by role {}, lacking producers by role {}, 0 / 0 {}. Agent-samples with no reserve, left out: {zero} (of {} sampled).\n",
        runs.len(),
        rows.iter()
            .filter(|r| r.tally.partner_runs.is_empty())
            .map(|r| r.seed_kin_kills.len())
            .sum::<usize>(),
        sum(&|w| w.persisted),
        sum(&|w| w.gradients.len()),
        sum(&|w| w.lacking_heterotroph),
        sum(&|w| w.lacking_producer),
        sum(&|w| w.undefined),
        samples + zero
    )
    .unwrap();
    let income = worlds.iter().fold([0.0; 3], |mut a, w| {
        for (x, y) in a.iter_mut().zip(w.income) {
            *x += y;
        }
        a
    });
    let total = income[1] + income[2];
    writeln!(
        o,
        "Hyphal share, pooled: hyphal N {:.2} / (pool uptake N {:.2} + consumption N {:.2}) = **{}** (root uptake N {:.2}).\n",
        income[0],
        income[1],
        income[2],
        share((total > 0.0).then(|| income[0] / total)),
        income[1] - income[0]
    )
    .unwrap();
    writeln!(
        o,
        "| config | seeds | persisted | qualifying | lacking het / prod | run gradients | world gradient | > 1 | hyphal share |"
    )
    .unwrap();
    writeln!(o, "|---|---:|---:|---:|---:|---|---:|---|---:|").unwrap();
    for w in &worlds {
        let g: Vec<String> = w.gradients.iter().map(|g| format!("{g:.3}")).collect();
        writeln!(
            o,
            "| {}:{} | {} | {} | {} | {} / {} | {} | {} | {} | {} |",
            w.source,
            w.config_index,
            w.runs,
            w.persisted,
            w.gradients.len(),
            w.lacking_heterotroph,
            w.lacking_producer,
            if g.is_empty() {
                "–".into()
            } else {
                g.join(", ")
            },
            fmt(w.gradient),
            if !w.carries() {
                "no heterotrophs"
            } else if w.above_one() {
                "yes"
            } else {
                "no"
            },
            share(w.hyphal_share()),
        )
        .unwrap();
    }
    writeln!(o, "\nPer run:\n").unwrap();
    writeln!(
        o,
        "| config | seed | verdict | het samples | prod samples | no reserve | het median | prod median | gradient | hyphal share |"
    )
    .unwrap();
    writeln!(o, "|---|---:|---|---:|---:|---:|---:|---:|---:|---:|").unwrap();
    for r in rows {
        for p in &r.tally.partner_runs {
            writeln!(
                o,
                "| {}:{} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                r.source,
                r.config_index,
                p.seed,
                p.outcome,
                p.heterotroph_samples,
                p.producer_samples,
                p.zero_reserve,
                fmt(p.heterotroph),
                fmt(p.producer),
                fmt(p.gradient()),
                share(p.hyphal_share()),
            )
            .unwrap();
        }
    }
    let Some((name, base)) = baseline else {
        writeln!(
            o,
            "\nNo baseline run given (`--baseline`): no paired sign test.\n"
        )
        .unwrap();
        return out;
    };
    writeln!(o, "\n#### Paired sign test against `{name}`\n").unwrap();
    let differs = baseline_differences(rows, base);
    if !differs.is_empty() {
        writeln!(
            o,
            "**Warning: the baseline differs in {}**; the test assumes the switch off, same atlas, seeds and mode.\n",
            differs.join(", ")
        )
        .unwrap();
    }
    let s = SignTest::of(rows, base);
    writeln!(
        o,
        "Indicator per world: gradient > 1 (a world not carrying heterotrophs by role reads 0). {} paired worlds ({} on this run only, {} in the baseline only): unpaired worlds are left out. Above 1 on this run only {}, in the baseline only {}, in both {}, in neither {}: discordant n = **{}**, k = **{}**. Exact binomial over the discordant pairs: two-sided p = **{:.4}**; one-sided p (on > off) = **{:.4}**. Baseline worlds above 1: {} of {} carrying.\n",
        s.pairs,
        s.unpaired_on,
        s.unpaired_off,
        s.on_only,
        s.off_only,
        s.both,
        s.neither,
        s.discordant(),
        s.on_only,
        s.two_sided_p(),
        s.one_sided_p(),
        PartnerTally::of(base).above_one,
        PartnerTally::of(base).carrying,
    )
    .unwrap();
    out
}

/// What a baseline (switch-off) rows file differs in from the rows, of what
/// the paired test assumes they share: the switch off, the atlas, the seeds
/// and the mode (horizon, founder aggregation).
fn baseline_differences(rows: &[Row], base: &[Row]) -> Vec<String> {
    let mut d = Vec::new();
    if base.iter().any(|r| r.hyphal_uptake == Some(true)) {
        d.push("the switch (some baseline rows ran with hyphal uptake on)".to_string());
    }
    let distinct = |rows: &[Row], f: &dyn Fn(&Row) -> String| -> Vec<String> {
        let mut v: Vec<String> = rows.iter().map(f).collect();
        v.sort();
        v.dedup();
        v
    };
    let checks: [(&str, &dyn Fn(&Row) -> String); 5] = [
        ("atlas", &|r| format!("{:?}", r.atlas)),
        ("base seed", &|r| r.base_seed.to_string()),
        ("seeds per config", &|r| r.seed_kin_kills.len().to_string()),
        ("horizon", &|r| r.horizon.to_string()),
        ("founder aggregation", &|r| {
            format!("{:?}", r.founder_aggregation)
        }),
    ];
    for (name, f) in checks {
        if distinct(rows, f) != distinct(base, f) {
            d.push(name.to_string());
        }
    }
    d
}

/// M (#700): one seed's leaching readout. The carcass-locked fraction is
/// read off the series the lockup gate reads (`observations.carcass_fraction`,
/// entry `i` at tick `i + 1`), on every run; the light-fed mixotrophs' share
/// off H's second-half buckets.
#[allow(clippy::too_many_arguments)]
fn leach_run(
    seed: u64,
    world: &World,
    observations: &RolloutObservations,
    eval_config: &EvalConfig,
    window_start: u64,
    failure: Option<&explorers_genesis_eval::FailureMode>,
    [nutrient, nutrient_settled]: [CarcassNutrient; 2],
    second_half: &Routes,
    clock: DrainClock,
) -> LeachRun {
    let series = &observations.carcass_fraction;
    let tail = &series[series
        .len()
        .saturating_sub(eval_config.nutrient_lock_window)..];
    let settled = series
        .get(window_start.saturating_sub(1) as usize..)
        .unwrap_or(&[]);
    let mean = |v: &[f32]| (!v.is_empty()).then(|| v.iter().sum::<f32>() / v.len() as f32);
    LeachRun {
        seed,
        end_tick: world.tick(),
        outcome: failure.map_or("persisted", failure_label).to_string(),
        carcass_locked_tail: mean(tail).unwrap_or(0.0),
        carcass_locked_settled: mean(settled),
        nutrient,
        nutrient_settled,
        lfm_drained_settled: second_half.carcass_drained[0],
        attributed_drained_settled: second_half.carcass_drained[..MEMO].iter().sum(),
        carcasses: clock.finish(),
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Row {
    source: ConfigSource,
    config_index: usize,
    horizon: u64,
    base_seed: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    founder_aggregation: Option<f32>,
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
    /// The config's effective carcass leaching rate λ (#700: pinned by
    /// `--leaching-rate`, else as decoded). Absent on older rows (λ = 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    leaching_rate: Option<f32>,
    /// The config's effective hyphal uptake switch and contact distance `d_c`
    /// (#728: pinned by `--hyphal-uptake` and `--contact-distance`, else as
    /// decoded). Absent on older rows, which ran with the switch off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hyphal_uptake: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    contact_distance: Option<f32>,
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
    uptake_structure_exponent: Option<f32>,
    uptake_reference_structure: Option<f32>,
    network: NetworkPins,
    cross_trait_cost: Option<f32>,
    leaching_rate: Option<f32>,
    hyphal_uptake: Option<bool>,
    contact_distance: Option<f32>,
    leaching_sweep: Option<Vec<PathBuf>>,
    crosscheck: bool,
    baseline: Option<PathBuf>,
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
        uptake_structure_exponent: None,
        uptake_reference_structure: None,
        network: NetworkPins::default(),
        cross_trait_cost: None,
        leaching_rate: None,
        hyphal_uptake: None,
        contact_distance: None,
        leaching_sweep: None,
        crosscheck: false,
        baseline: None,
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
            "--baseline" => args.baseline = Some(PathBuf::from(value()?)),
            "--founder-aggregation" => {
                args.founder_aggregation = Some(parse_founder_aggregation(&value()?)?)
            }
            "--uptake-structure-exponent" => {
                args.uptake_structure_exponent = Some(parse_non_negative(&flag, &value()?)?)
            }
            "--uptake-reference-structure" => {
                args.uptake_reference_structure = Some(parse_positive(&flag, &value()?)?)
            }
            f if NETWORK_FLAGS.contains(&f) => args.network.set(f, &value()?)?,
            "--cross-trait-cost" => {
                args.cross_trait_cost = Some(parse_non_negative(&flag, &value()?)?)
            }
            "--leaching-rate" => args.leaching_rate = Some(parse_unit_interval(&flag, &value()?)?),
            "--hyphal-uptake" => {
                args.hyphal_uptake = Some(match value()?.as_str() {
                    "on" => true,
                    "off" => false,
                    v => return Err(format!("{flag} takes on or off, not {v:?}")),
                })
            }
            "--contact-distance" => args.contact_distance = Some(parse_positive(&flag, &value()?)?),
            "--leaching-sweep" => {
                args.leaching_sweep = Some(value()?.split(',').map(PathBuf::from).collect())
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
    if let Some(files) = &args.leaching_sweep {
        leaching_sweep(files);
        return;
    }
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
            let config = with_uptake_scaling(
                config,
                args.uptake_structure_exponent,
                args.uptake_reference_structure,
            );
            let config = with_network(config, &args.network);
            let config = with_cross_trait_cost(config, args.cross_trait_cost);
            let config = with_leaching_rate(config, args.leaching_rate);
            let config = with_hyphal_uptake(config, args.hyphal_uptake, args.contact_distance);
            let tallies: Vec<Tally> = (0..args.ensemble)
                .into_par_iter()
                .map(|i| {
                    rollout(
                        &config.0,
                        &config.1,
                        args.seed + i,
                        args.horizon,
                        &eval,
                        args.crosscheck,
                    )
                })
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
                let c = &tally.charge_check;
                eprintln!(
                    "  charge crosscheck {source}:{idx}: {} agent-ticks ({} starving, not compared): stepper Σ {:.6}, readout Σ {:.6}, {} mismatches -> {}",
                    c.agent_ticks,
                    c.starved,
                    c.stepper,
                    c.readout,
                    c.mismatches,
                    if c.mismatches == 0 {
                        "MATCH"
                    } else {
                        "MISMATCH"
                    }
                );
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
                    uptake_structure_exponent: Some(config.0.uptake_structure_exponent),
                    uptake_reference_structure: Some(config.0.uptake_reference_structure),
                    network: Some(NetworkRecord::of(&config.0)),
                    atlas: (source == ConfigSource::Atlas).then(|| atlas_fingerprint.clone()),
                    cross_trait_cost: Some(config.0.cross_trait_cost),
                    maintenance_cost_exponent: Some(config.0.maintenance_cost_exponent),
                    leaching_rate: Some(config.0.leaching_rate),
                    hyphal_uptake: Some(config.0.hyphal_uptake),
                    contact_distance: Some(config.0.contact_distance),
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
    let clusters = match args.atlas.as_deref() {
        Some(path) => lineage_clusters(&rows, &read_atlas_units(path), LINEAGE_CLUSTERS),
        None => Err("no atlas given (`--atlas`)".into()),
    };
    let baseline = args
        .baseline
        .as_deref()
        .map(|p| (p.display().to_string(), read_rows::<Row>(p)));
    summary(
        &rows,
        &clusters,
        baseline.as_ref().map(|(n, r)| (n.as_str(), &r[..])),
    );
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

fn summary(rows: &[Row], clusters: &Result<Vec<usize>, String>, baseline: Option<(&str, &[Row])>) {
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
    flow1_verdict(rows, &t, clusters, baseline);
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
    charge_by_role(rows, &t.ext);
    mobile_autotrophy(rows, &t.ext);
    leaching(rows);
    print!("{}", partner_report(rows, baseline));
    print!("{}", limitation_report(rows));
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

/// The hyphal uptake setting the rows ran at, each distinct one listed.
fn hyphal_label(rows: &[Row]) -> String {
    let mut v: Vec<String> = Vec::new();
    for r in rows {
        let s = match (r.hyphal_uptake, r.contact_distance) {
            (None, _) => "off (unrecorded, pre-#728 row)".to_string(),
            (Some(false), _) => "off".to_string(),
            (Some(true), d) => format!(
                "on, d_c = {}",
                d.map_or("unrecorded".to_string(), |d| d.to_string())
            ),
        };
        if !v.contains(&s) {
            v.push(s);
        }
    }
    format!(
        "Hyphal uptake (#727): **{}**",
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

/// #682: each row's lineage cluster, its atlas cell's label among
/// `clusters` Ward clusters over every cell of `atlas` (unit coordinates).
/// `Err` says why the rows cannot be clustered: not all `atlas:` rows, or
/// rows that did not run on this atlas (fingerprint absent or different).
fn lineage_clusters(
    rows: &[Row],
    atlas: &AtlasUnits,
    clusters: usize,
) -> Result<Vec<usize>, String> {
    let fp = format!("{:016x}", atlas.fingerprint());
    if let Some(r) = rows.iter().find(|r| r.source != ConfigSource::Atlas) {
        return Err(format!(
            "{}:{} is not an atlas cell",
            r.source, r.config_index
        ));
    }
    if let Some(r) = rows.iter().find(|r| r.atlas.as_deref() != Some(&fp)) {
        return Err(format!(
            "atlas:{} ran on atlas {}, not this one ({fp})",
            r.config_index,
            r.atlas.as_deref().unwrap_or("unrecorded")
        ));
    }
    let labels = ward_clusters(atlas.units(), clusters);
    Ok(rows.iter().map(|r| labels[r.config_index]).collect())
}

/// #682: flow 1's two verdicts as world-rules.md flow 1 states them, for
/// the mode these rows ran in. `clusters`: each row's lineage cluster, or
/// why there are none; `baseline`: an earlier run's rows to classify the
/// share against by absolute drains.
fn flow1_verdict(
    rows: &[Row],
    t: &Tally,
    clusters: &Result<Vec<usize>, String>,
    baseline: Option<(&str, &[Row])>,
) {
    let g = conditionality_of(rows);
    let v = Flow1Conditionality::of(g.pooled, &g.per_config, clusters.as_deref().ok());
    println!("### Flow 1 verdict (#682, world-rules.md flow 1)\n");
    println!(
        "Conditionality: Spearman ρ(h_eff, pool N at cell) over second-half Producer-role samples, per config (G). It passes only if all three hold: (1) the median per-config ρ < 0; (2) the configs with ρ < 0 differ from a coin's share (two-sided exact binomial p < {COIN_ALPHA}); (3) a majority of the configs' lineage clusters ({LINEAGE_CLUSTERS} Ward clusters over the atlas's cells, unit coordinates) have a median ρ < 0. Fallback: heterotrophy does not run backwards, pooled ρ ≤ 0. Minority share: light-fed mixotrophs drain < {:.0} % of the second half's attributed carcass structure (H).\n",
        100.0 * MINORITY_BAR
    );
    println!("| test | verdict | reading |");
    println!("|---|---|---|");
    println!(
        "| (1) median per-config ρ < 0 | {} | median ρ = {} over {} configs |",
        pass(v.median_negative()),
        rho(v.median),
        v.configs
    );
    println!(
        "| (2) configs with ρ < 0 differ from a coin's | {} | {} of {}, p = {:.3} |",
        pass(v.not_a_coin()),
        v.negative,
        v.configs,
        v.binomial_p
    );
    let cluster_reading = match (clusters, v.clusters) {
        (Ok(labels), Some(c)) => {
            let mut sizes = vec![0usize; LINEAGE_CLUSTERS];
            for &l in labels {
                sizes[l] += 1;
            }
            format!(
                "{} of {} clusters with a median have it < 0 (configs per cluster: {})",
                c.negative,
                c.with_median,
                sizes
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        (Err(why), _) => format!("not read: {why}"),
        _ => "not read".into(),
    };
    println!(
        "| (3) a majority of lineage clusters have a median ρ < 0 | {} | {cluster_reading} |",
        pass(v.clusters_negative())
    );
    println!(
        "| **conditionality** | {} | all of (1)–(3) |",
        pass(v.passes())
    );
    println!(
        "| fallback: does not run backwards (pooled ρ ≤ 0) | {} | pooled ρ = {} (n = {}) |",
        pass(v.does_not_run_backwards()),
        rho(g.pooled),
        g.samples
    );
    let share = NicheShare::of(
        &t.ext.routes_second_half,
        t.ext.carcass_drained_all_second_half,
    )
    .structure_share(0);
    println!(
        "| **minority share** (second half) | {} | light-fed mixotrophs drain {} % of carcass structure |",
        pass(minority(share)),
        pct_of(share)
    );
    let Some((name, base_rows)) = baseline else {
        println!(
            "\nNo baseline run given (`--baseline`): the share is not classified by absolute drains.\n"
        );
        return;
    };
    let mut bt = Tally::new();
    for r in base_rows {
        bt.merge(&r.tally);
    }
    match (drains(rows, &t.ext), drains(base_rows, &bt.ext)) {
        (Some(run), Some(base)) => {
            let delta = |a: f64, b: f64| {
                if b > 0.0 {
                    format!("{:+.0} %", 100.0 * (a - b) / b)
                } else {
                    "–".into()
                }
            };
            println!(
                "| share by absolute drains, against {name} | {} | carcass structure drained per seed, second half: light-fed mixotrophs {:.1} vs {:.1} ({}), decomposers {:.1} vs {:.1} ({}) |",
                DrainShift::of(run, base).label(),
                run.mixotrophs,
                base.mixotrophs,
                delta(run.mixotrophs, base.mixotrophs),
                run.decomposers,
                base.decomposers,
                delta(run.decomposers, base.decomposers)
            );
        }
        _ => println!("| share by absolute drains, against {name} | n/a | no seeds |"),
    }
    println!(
        "\nMixotroph excess: mixotrophs drain more than the baseline. Decomposer deficit: decomposers drain less, and mixotrophs no more.\n"
    );
}

/// #682: settled-half carcass structure drained per seed by light-fed
/// mixotrophs and decomposers by role (C's buckets 0 and 3). `None` without
/// seeds.
fn drains(rows: &[Row], x: &Ext) -> Option<Drains> {
    let seeds: usize = rows.iter().map(|r| r.seed_kin_kills.len()).sum();
    let r = &x.routes_second_half;
    (seeds > 0 && !r.carcass_drained.is_empty()).then(|| Drains {
        mixotrophs: r.carcass_drained[0] / seeds as f64,
        decomposers: r.carcass_drained[3] / seeds as f64,
    })
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
/// `maintenance_cost_exponent`. The same expression as the sim's, so
/// `c_AH × term` is what the agent would pay (K reads the charge itself off
/// `phase::cross_trait_charge`).
fn cross_trait_term(a: f32, h: f32, p: f32) -> f32 {
    (a * h).powf(0.5 * p)
}

/// #681: the cross-trait cost an agent with raw autotrophy `a` and
/// heterotrophy `h` is charged per tick, `c_AH × (A·H)^(p/2)` — the sim's own
/// expression (`phase::cross_trait_charge`), so the readout cannot drift from
/// the stepper.
fn cross_trait_charge(a: f32, h: f32, params: &WorldParameters) -> f32 {
    phase::cross_trait_charge(a, h, params)
}

/// The trait level above which the readout counts an investment (#681): the
/// 0.05 its light-fed mixotroph bucket reads heterotrophy against.
const INVESTMENT_THRESHOLD: f32 = 0.05;

/// #681's groups, in this order: decomposers by trophic role; and producer →
/// decomposer intermediates, mixotrophs by investment (raw autotrophy and
/// heterotrophy both above [`INVESTMENT_THRESHOLD`]) with a role that is not
/// producer. An agent with no income yet has no role, so is in neither.
const CHARGE_GROUPS: [&str; 2] = [
    "decomposer (by role)",
    "producer → decomposer intermediate (mixotroph by investment, role not producer)",
];

fn charge_groups(role: Option<TrophicRole>, a: f32, h: f32) -> [bool; 2] {
    let mixotroph = a > INVESTMENT_THRESHOLD && h > INVESTMENT_THRESHOLD;
    [
        role == Some(TrophicRole::Decomposer),
        mixotroph && matches!(role, Some(TrophicRole::Consumer | TrophicRole::Decomposer)),
    ]
}

/// #681: one group's read over its entries, one per agent per seed
/// (`Ext::charge`: `[raw autotrophy A, cross-trait charge per tick, ticks in
/// the group, Σ energy income on them]`).
#[derive(Clone, Debug, PartialEq)]
struct ChargeRead {
    agents: usize,
    /// Agent-samples: Σ ticks.
    samples: u64,
    /// Of all second-half agent-samples; `None` with none.
    share: Option<f64>,
    /// Autotrophy carried, per agent-sample.
    autotrophy_mean: Option<f64>,
    autotrophy_median: Option<f64>,
    /// Σ charge / Σ income over the group's agent-samples; `None` with no
    /// income.
    charge_of_income: Option<f64>,
    /// The median over agents with income of their charge / income.
    median_charge_of_income: Option<f64>,
    /// Agents with no income on any of their ticks in the group.
    zero_income: usize,
}

impl ChargeRead {
    fn of(agents: &[[f32; 4]], all_samples: u64) -> Self {
        let ticks = |a: &[f32; 4]| a[2] as u64;
        let samples: u64 = agents.iter().map(ticks).sum();
        let charged = |a: &[f32; 4]| f64::from(a[1]) * f64::from(a[2]);
        let income: f64 = agents.iter().map(|a| f64::from(a[3])).sum();
        let mut by_a: Vec<(f64, u64)> =
            agents.iter().map(|a| (f64::from(a[0]), ticks(a))).collect();
        by_a.sort_by(|x, y| x.0.total_cmp(&y.0));
        // The k-th agent-sample (0-based) in ascending A.
        let nth = |k: u64| {
            let mut seen = 0;
            by_a.iter()
                .find(|(_, n)| {
                    seen += n;
                    seen > k
                })
                .map(|(a, _)| *a)
        };
        let autotrophy_median = match samples {
            0 => None,
            n if n % 2 == 1 => nth(n / 2),
            n => nth(n / 2 - 1)
                .zip(nth(n / 2))
                .map(|(lo, hi)| (lo + hi) / 2.0),
        };
        let ratios: Vec<f64> = agents
            .iter()
            .filter(|a| a[3] > 0.0)
            .map(|a| charged(a) / f64::from(a[3]))
            .collect();
        Self {
            agents: agents.len(),
            samples,
            share: (all_samples > 0).then(|| samples as f64 / all_samples as f64),
            autotrophy_mean: (samples > 0).then(|| {
                agents
                    .iter()
                    .map(|a| f64::from(a[0]) * f64::from(a[2]))
                    .sum::<f64>()
                    / samples as f64
            }),
            autotrophy_median,
            charge_of_income: (income > 0.0)
                .then(|| agents.iter().map(charged).sum::<f64>() / income),
            median_charge_of_income: median(ratios),
            zero_income: agents.iter().filter(|a| a[3] <= 0.0).count(),
        }
    }
}

/// A group's read as table cells: share of agent-samples, autotrophy mean
/// and median, charge / income two ways.
fn charge_cells(g: &ChargeRead) -> String {
    format!(
        "{} | {} | {} | {} | {}",
        pct_of(g.share),
        g4(g.autotrophy_mean),
        g4(g.autotrophy_median),
        pct_of(g.charge_of_income),
        pct_of(g.median_charge_of_income),
    )
}

/// Second-half carcass structure drained by C's decomposer and light-fed
/// mixotroph buckets, per seed, and the mixotrophs' share of the attributed
/// total (H's), as cells.
fn drained_cells(x: &Ext, seeds: usize) -> String {
    let r = &x.routes_second_half;
    let per_seed = |b: usize| {
        if seeds == 0 {
            "–".into()
        } else {
            format!("{:.1}", r.carcass_drained[b] / seeds as f64)
        }
    };
    format!(
        "{} | {} | {}",
        per_seed(3),
        per_seed(0),
        pct_of(NicheShare::of(r, x.carcass_drained_all_second_half).structure_share(0))
    )
}

/// K (#681): what the cross-trait cost charges, by trophic role.
fn charge_by_role(rows: &[Row], x: &Ext) {
    println!("\n### K. The cross-trait cost's charge by trophic role (#681, second half)\n");
    if x.second_half_agent_ticks == 0 {
        println!("No charge entries on these rows (pre-#681 rows).");
        return;
    }
    let seeds: usize = rows.iter().map(|r| r.seed_kin_kills.len()).sum();
    println!(
        "Unit: the **agent-sample**, every agent on the drain-time roster on every second-half tick ({} over {seeds} seeds). Groups, by recent-income role at the start of the tick: (1) **decomposers by role**, whatever they invest in; (2) **producer → decomposer intermediates**, mixotrophs by investment (raw autotrophy A and heterotrophy H both > {INVESTMENT_THRESHOLD}) whose role is consumer or decomposer (an agent with no income yet has no role, so is in neither). The groups overlap: a decomposer that is a mixotroph by investment is in both. Autotrophy: raw A per agent-sample. Charge: `c_AH × (A·H)^(p/2)` per tick, the stepper's own term (`phase::cross_trait_charge`), at the c_AH the rows ran at; income: light plus drain energy received that tick (`TickIntake::energy`). Charge / income: Σ charge / Σ income over the group's agent-samples, and the median over agents (one per agent per seed, those with income) of their own ratio. Carcass structure drained: C's second-half routes, per seed, by the decomposer bucket and the light-fed mixotroph bucket (P, h_eff > 0.05), and the mixotrophs' share of the attributed total (H's).\n",
        x.second_half_agent_ticks
    );
    println!(
        "| group | agents | agent-samples | % of agent-samples | A mean | A median | % charge / income (Σ / Σ) | % charge / income (median agent) | agents without income |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    for (name, entries) in CHARGE_GROUPS.iter().zip(&x.charge) {
        let g = ChargeRead::of(entries, x.second_half_agent_ticks);
        println!(
            "| {name} | {} | {} | {} | {} |",
            g.agents,
            g.samples,
            charge_cells(&g),
            g.zero_income
        );
    }
    println!(
        "\nCarcass structure drained per seed, second half: decomposers | light-fed mixotrophs | mixotrophs' share (%) = {}.\n",
        drained_cells(x, seeds)
    );
    println!("Per config (D = decomposers by role, I = intermediates):\n");
    println!(
        "| config | c_AH | seeds | D % samples | D A mean | D A median | D % charge / income | D % median agent | I % samples | I A mean | I A median | I % charge / income | I % median agent | decomposer carcass drained / seed | mixotroph carcass drained / seed | mixotroph % |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows {
        let e = &r.tally.ext;
        let read = |g: usize| ChargeRead::of(&e.charge[g], e.second_half_agent_ticks);
        println!(
            "| {}:{} | {} | {} | {} | {} | {} |",
            r.source,
            r.config_index,
            r.cross_trait_cost.map_or("–".into(), |c| format!("{c}")),
            r.seed_kin_kills.len(),
            charge_cells(&read(0)),
            charge_cells(&read(1)),
            drained_cells(e, r.seed_kin_kills.len()),
        );
    }
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

/// L (#683): the quantiles L reports of [`FineHist`]s.
const MOBILE_QUANTILES: [(&str, f64); 6] = [
    ("p50", 0.5),
    ("p75", 0.75),
    ("p90", 0.9),
    ("p95", 0.95),
    ("p99", 0.99),
    ("max", 1.0),
];

fn fine_cells(h: &FineHist) -> String {
    let mut cells: Vec<String> = vec![h.n.to_string(), g4(h.mean())];
    cells.extend(MOBILE_QUANTILES.iter().map(|&(_, q)| g4(h.quantile(q))));
    cells.join(" | ")
}

fn above_cells(c: &MobileCensus, a: &MobileAbove) -> String {
    format!(
        "{} | {} | {} | {} | {} | {} / {} | {} / {} | {} / {}",
        a.samples,
        pct(a.samples, c.samples),
        g4((a.movement_n > 0).then(|| a.movement_sum / a.movement_n as f64)),
        a.births,
        a.breeders,
        a.breeding_seeds,
        c.seeds,
        a.sustained_seeds,
        c.completed_seeds,
        a.guild_seeds,
        c.completed_seeds
    )
}

/// L (#683): mobile autotrophy, which decides whether substrate contact
/// (#648) comes out of reserve (world-rules.md flow 2).
fn mobile_autotrophy(rows: &[Row], x: &Ext) {
    println!(
        "\n### L. Mobile autotrophy: producers by role that move, persist and breed (#683, second half)\n"
    );
    let c = &x.mobile;
    if c.seeds == 0 {
        println!("No mobile-autotrophy entries on these rows (pre-#683 rows).");
        return;
    }
    println!(
        "Unit: the **producer agent-sample**, every agent whose recent-income role is Producer on a second-half sample tick ({} over {} seeds, {} of them run to the horizon). Effective mobility: the wear-degraded trait (`effective_trait_with_steepness(2, k)`). Realised movement: the agent's mean toroidal distance moved per tick since the previous sample tick (second-half ticks only; none for an agent that has not yet stepped in that span). Above a threshold: effective mobility strictly greater. Births: second-half births whose parent was a producer by role at the start of the tick with pre-step effective mobility above it; breeders: their distinct parents, summed over seeds. **Sustained**: a run to the horizon whose producers above the threshold number ≥ {GUILD_MIN_SIZE} on every second-half sample tick; **holds**: sustained and at least one such birth (the heterotroph guild's rule, applied to the mobile producers as a population). Quantiles: nearest rank, at {FINE_WIDTH}-wide bins' midpoints; max exact.\n",
        c.samples, c.seeds, c.completed_seeds
    );
    let head: Vec<&str> = MOBILE_QUANTILES.iter().map(|&(n, _)| n).collect();
    println!(
        "| producer agent-samples | n | mean | {} |",
        head.join(" | ")
    );
    println!("|---|---:|---:|{}", "---:|".repeat(head.len()));
    println!("| effective mobility | {} |", fine_cells(&c.mobility));
    println!(
        "| realised movement per tick | {} |",
        fine_cells(&c.movement)
    );
    println!(
        "\n| effective mobility > | agent-samples | % of producer samples | mean movement per tick | births | breeders | seeds breeding | seeds sustained | seeds holding |"
    );
    println!("|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for (t, a) in MOBILE_THRESHOLDS.iter().zip(&c.above) {
        println!("| {t} | {} |", above_cells(c, a));
    }
    println!(
        "\nPer config: effective mobility quantiles, realised movement per tick (p50, p90), and per threshold the % of producer samples above it · births · seeds holding / seeds run to the horizon.\n"
    );
    let thresholds: Vec<String> = MOBILE_THRESHOLDS.iter().map(|t| format!("> {t}")).collect();
    println!(
        "| config | seeds | producer samples | m p50 | m p90 | m p99 | m max | move p50 | move p90 | {} |",
        thresholds.join(" | ")
    );
    println!(
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|{}",
        "---:|".repeat(thresholds.len())
    );
    for r in rows {
        let c = &r.tally.ext.mobile;
        if c.seeds == 0 {
            continue;
        }
        let cells: Vec<String> = c
            .above
            .iter()
            .map(|a| {
                format!(
                    "{} · {} · {}/{}",
                    pct(a.samples, c.samples),
                    a.births,
                    a.guild_seeds,
                    c.completed_seeds
                )
            })
            .collect();
        println!(
            "| {}:{} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r.source,
            r.config_index,
            c.seeds,
            c.samples,
            g4(c.mobility.quantile(0.5)),
            g4(c.mobility.quantile(0.9)),
            g4(c.mobility.quantile(0.99)),
            g4(c.mobility.quantile(1.0)),
            g4(c.movement.quantile(0.5)),
            g4(c.movement.quantile(0.9)),
            cells.join(" | ")
        );
    }
}

/// L (#683): candidate thresholds on a producer's effective mobility, so the
/// census can be read at each and the threshold chosen afterwards.
const MOBILE_THRESHOLDS: [f32; 5] = [0.05, 0.1, 0.2, 0.3, 0.5];
/// L: [`FineHist`]'s bin width.
const FINE_WIDTH: f64 = 0.001;

/// A sparse histogram at [`FINE_WIDTH`] resolution, with the exact maximum.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct FineHist {
    n: u64,
    sum: f64,
    max: f32,
    bins: BTreeMap<u32, u64>,
}

impl FineHist {
    fn record(&mut self, v: f32) {
        let v = v.max(0.0);
        self.n += 1;
        self.sum += f64::from(v);
        self.max = self.max.max(v);
        *self
            .bins
            .entry((f64::from(v) / FINE_WIDTH) as u32)
            .or_default() += 1;
    }

    fn merge(&mut self, o: &FineHist) {
        self.n += o.n;
        self.sum += o.sum;
        self.max = self.max.max(o.max);
        for (&b, &c) in &o.bins {
            *self.bins.entry(b).or_default() += c;
        }
    }

    fn mean(&self) -> Option<f64> {
        (self.n > 0).then(|| self.sum / self.n as f64)
    }

    /// Nearest-rank quantile, at its bin's midpoint capped at the exact max
    /// (q = 1 is the max).
    fn quantile(&self, q: f64) -> Option<f64> {
        if self.n == 0 {
            return None;
        }
        if q >= 1.0 {
            return Some(f64::from(self.max));
        }
        let rank = (q * (self.n - 1) as f64).round() as u64;
        let mut seen = 0;
        for (&b, &c) in &self.bins {
            seen += c;
            if seen > rank {
                return Some(((f64::from(b) + 0.5) * FINE_WIDTH).min(f64::from(self.max)));
            }
        }
        None
    }
}

/// L: the census above one threshold (`MOBILE_THRESHOLDS[i]`), summed over
/// seeds.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct MobileAbove {
    /// Producer-role agent-samples with effective mobility above it.
    samples: u64,
    /// Σ realised movement per tick over those samples that have a reading,
    /// and how many have one.
    movement_sum: f64,
    movement_n: u64,
    /// Second-half births whose parent was such a producer (role at the start
    /// of the tick, pre-step effective mobility), and the distinct parents.
    births: u64,
    breeders: u64,
    /// Seeds with at least one such birth.
    breeding_seeds: u64,
    /// Completed seeds where such producers number ≥ `GUILD_MIN_SIZE` on
    /// every second-half sample tick.
    sustained_seeds: u64,
    /// Sustained and breeding: the mobile autotroph population holds.
    guild_seeds: u64,
}

/// L (#683): **mobile autotrophy**, producers by role (the sample's
/// recent-income role) on second-half sample ticks, by effective mobility.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct MobileCensus {
    seeds: u64,
    /// Seeds that ran to the horizon (no early stop).
    completed_seeds: u64,
    /// Producer-role agent-samples.
    samples: u64,
    /// Their effective mobility (wear-degraded, `ft_index` 2).
    mobility: FineHist,
    /// Their realised movement: mean toroidal distance moved per tick since
    /// the previous sample (second-half ticks only; absent for an agent with
    /// no step in that span).
    movement: FineHist,
    /// Per [`MOBILE_THRESHOLDS`].
    above: Vec<MobileAbove>,
}

impl MobileCensus {
    fn merge(&mut self, o: &MobileCensus) {
        self.seeds += o.seeds;
        self.completed_seeds += o.completed_seeds;
        self.samples += o.samples;
        self.mobility.merge(&o.mobility);
        self.movement.merge(&o.movement);
        if self.above.len() < o.above.len() {
            self.above.resize(o.above.len(), MobileAbove::default());
        }
        for (a, b) in self.above.iter_mut().zip(&o.above) {
            a.samples += b.samples;
            a.movement_sum += b.movement_sum;
            a.movement_n += b.movement_n;
            a.births += b.births;
            a.breeders += b.breeders;
            a.breeding_seeds += b.breeding_seeds;
            a.sustained_seeds += b.sustained_seeds;
            a.guild_seeds += b.guild_seeds;
        }
    }
}

/// L, per threshold, within one seed.
#[derive(Clone, Debug, Default)]
struct MobileTrack {
    above: MobileAbove,
    parents: HashSet<u64>,
    /// The fewest such producers on any second-half sample tick.
    min_count: Option<usize>,
}

/// L's per-seed state: fed every second-half sample tick's producers and
/// every second-half birth by a producer, read once at the end.
#[derive(Clone, Debug, Default)]
struct MobileTracker {
    census: MobileCensus,
    tracks: Vec<MobileTrack>,
}

impl MobileTracker {
    fn tracks(&mut self) -> &mut [MobileTrack] {
        if self.tracks.is_empty() {
            self.tracks = vec![MobileTrack::default(); MOBILE_THRESHOLDS.len()];
        }
        &mut self.tracks
    }

    /// One second-half sample tick: every producer by role, as `(id,
    /// effective mobility, realised movement per tick)`.
    fn sample(&mut self, producers: &[(u64, f32, Option<f32>)]) {
        for &(_, m, moved) in producers {
            self.census.samples += 1;
            self.census.mobility.record(m);
            if let Some(d) = moved {
                self.census.movement.record(d);
            }
        }
        for (track, t) in self.tracks().iter_mut().zip(MOBILE_THRESHOLDS) {
            let mut count = 0;
            for &(_, _, moved) in producers.iter().filter(|p| p.1 > t) {
                count += 1;
                track.above.samples += 1;
                if let Some(d) = moved {
                    track.above.movement_sum += f64::from(d);
                    track.above.movement_n += 1;
                }
            }
            track.min_count = Some(track.min_count.map_or(count, |c| c.min(count)));
        }
    }

    /// A second-half birth whose parent was a producer by role at the start
    /// of the tick, at the parent's pre-step effective mobility.
    fn birth(&mut self, parent: u64, mobility: f32) {
        for (track, t) in self.tracks().iter_mut().zip(MOBILE_THRESHOLDS) {
            if mobility > t {
                track.above.births += 1;
                track.parents.insert(parent);
            }
        }
    }

    /// The seed's census; `completed` when the run reached its horizon (an
    /// early stop holds no population over the settled half).
    fn finish(mut self, completed: bool) -> MobileCensus {
        self.tracks();
        let mut c = self.census;
        c.seeds = 1;
        c.completed_seeds = u64::from(completed);
        c.above = self
            .tracks
            .into_iter()
            .map(|t| {
                let mut a = t.above;
                a.breeders = t.parents.len() as u64;
                a.breeding_seeds = u64::from(a.births > 0);
                let sustained = completed && t.min_count.is_some_and(|n| n >= GUILD_MIN_SIZE);
                a.sustained_seeds = u64::from(sustained);
                a.guild_seeds = u64::from(sustained && a.births > 0);
                a
            })
            .collect();
        c
    }
}

/// The leaching rates the rows ran at, each distinct value listed.
fn leaching_label(rows: &[Row]) -> String {
    let mut seen: Vec<f32> = Vec::new();
    for r in rows {
        let l = r.leaching_rate.unwrap_or(0.0);
        if !seen.contains(&l) {
            seen.push(l);
        }
    }
    let list: Vec<String> = seen.iter().map(|l| format!("{l}")).collect();
    if list.is_empty() {
        "–".into()
    } else {
        list.join(", ")
    }
}

/// The settled half's first tick (the census's second half).
fn settled_from(rows: &[Row]) -> u64 {
    rows.first().map_or(1, |r| r.horizon / 2 + 1)
}

/// Which carcasses a reading keeps, by how they died.
#[derive(Clone, Copy, PartialEq)]
enum Deaths {
    All,
    /// Not drained while living in the step they died: a drainer must find
    /// them.
    Unbitten,
    /// Drained while living in the step they died (mostly kills).
    Bitten,
}

impl Deaths {
    fn keeps(self, f: &explorers_search::leaching::CarcassFate) -> bool {
        match self {
            Deaths::All => true,
            Deaths::Unbitten => !f.bitten,
            Deaths::Bitten => f.bitten,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Deaths::All => "all carcasses",
            Deaths::Unbitten => "not bitten to death",
            Deaths::Bitten => "bitten to death",
        }
    }
}

fn first_drain_of(rows: &[Row], settled: bool, deaths: Deaths) -> FirstDrain {
    let from = if settled { settled_from(rows) } else { 0 };
    FirstDrain::of(
        rows.iter()
            .flat_map(|r| &r.tally.leach_runs)
            .map(|run| (&run.carcasses[..], run.end_tick)),
        |f| f.died >= from && deaths.keeps(f),
    )
}

fn ticks_cell(x: Option<f64>) -> String {
    x.map_or("–".into(), |v| format!("{v:.1}"))
}

fn rate_cell(x: Option<f64>) -> String {
    x.map_or("–".into(), |v| format!("{v:.4}"))
}

/// Verdicts other than persisted and nutrient lockup, as `label n, …`.
fn other_verdicts(r: &SweepReading) -> String {
    let parts: Vec<String> = r
        .outcomes
        .iter()
        .filter(|(k, _)| k.as_str() != "persisted" && k.as_str() != "nutrient_lockup")
        .map(|(k, n)| format!("{k} {n}"))
        .collect();
    if parts.is_empty() {
        "–".into()
    } else {
        parts.join(", ")
    }
}

fn frac_cell(x: Option<f64>) -> String {
    x.map_or("–".into(), |v| format!("{v:.4}"))
}

/// M (#700): time to first drain, the proposed top of λ's range, and the
/// sweep's quantities at the rows' λ.
fn leaching(rows: &[Row]) {
    println!("\n### M. Carcass leaching calibration (#700)\n");
    let runs: Vec<&LeachRun> = rows.iter().flat_map(|r| &r.tally.leach_runs).collect();
    if runs.is_empty() {
        println!("No leaching readout on these rows (pre-#700).\n");
        return;
    }
    let from = settled_from(rows);
    println!(
        "Leaching rate λ: {}. Carcasses formed in the run (`Died`), from death to the first bite any drainer takes (`Consumed` on the carcass), in ticks; an undrained carcass is censored at its run's end (the horizon or an early stop). Settled half: died at tick ≥ {from}. *Bitten to death*: drained while living in the step it died (a kill, or a death a drain hastened), whose drainer is usually on it already; the others a drainer has to find. Per-carcass rows: `tally.leach_runs[].carcasses` as `[died, drained | null, bitten]`.\n",
        leaching_label(rows)
    );
    println!("#### M1. Time to first drain, pooled\n");
    println!(
        "| deaths | window | carcasses | drained | never drained % | p25 | median (t*) | p75 | p90 | Kaplan–Meier median (censored) |"
    );
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    let whole = first_drain_of(rows, false, Deaths::All);
    let settled = first_drain_of(rows, true, Deaths::All);
    let found = first_drain_of(rows, false, Deaths::Unbitten);
    let found_settled = first_drain_of(rows, true, Deaths::Unbitten);
    for deaths in [Deaths::All, Deaths::Unbitten, Deaths::Bitten] {
        for (window, settled) in [("whole run", false), ("settled half", true)] {
            let d = first_drain_of(rows, settled, deaths);
            println!(
                "| {} | {window} | {} | {} | {} | {} | {} | {} | {} | {} |",
                deaths.name(),
                d.carcasses,
                d.drained,
                pct_of(d.never_share()),
                ticks_cell(d.drained_quantile(0.25)),
                ticks_cell(d.drained_quantile(0.5)),
                ticks_cell(d.drained_quantile(0.75)),
                ticks_cell(d.drained_quantile(0.9)),
                ticks_cell(d.km_median()),
            );
        }
    }
    println!("\n#### M2. Proposed λ_max = ln 2 / t*\n");
    println!("| reading | t* | λ_max |");
    println!("|---|---:|---:|");
    for (name, t) in [
        (
            "median over drained carcasses, whole run (#700's reading)",
            whole.drained_quantile(0.5),
        ),
        (
            "median over drained carcasses, settled half",
            settled.drained_quantile(0.5),
        ),
        (
            "Kaplan–Meier median, undrained censored, whole run",
            whole.km_median(),
        ),
        (
            "Kaplan–Meier median, undrained censored, settled half",
            settled.km_median(),
        ),
        (
            "not bitten to death: median over drained, whole run",
            found.drained_quantile(0.5),
        ),
        (
            "not bitten to death: median over drained, settled half",
            found_settled.drained_quantile(0.5),
        ),
        (
            "not bitten to death: Kaplan–Meier median, whole run",
            found.km_median(),
        ),
        (
            "not bitten to death: Kaplan–Meier median, settled half",
            found_settled.km_median(),
        ),
    ] {
        println!(
            "| {name} | {} | {} |",
            ticks_cell(t),
            rate_cell(lambda_max(t))
        );
    }
    println!("\n#### M3. Time to first drain per config\n");
    println!(
        "| config | runs | carcasses | bitten to death % | never drained % | t* whole | t* settled | KM median whole | KM median settled | t* not bitten | KM median not bitten |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows {
        let one = std::slice::from_ref(r);
        let (w, st) = (
            first_drain_of(one, false, Deaths::All),
            first_drain_of(one, true, Deaths::All),
        );
        let (b, f) = (
            first_drain_of(one, false, Deaths::Bitten),
            first_drain_of(one, false, Deaths::Unbitten),
        );
        println!(
            "| {}:{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r.source,
            r.config_index,
            r.tally.leach_runs.len(),
            w.carcasses,
            pct_of((w.carcasses > 0).then(|| b.carcasses as f64 / w.carcasses as f64)),
            pct_of(w.never_share()),
            ticks_cell(w.drained_quantile(0.5)),
            ticks_cell(st.drained_quantile(0.5)),
            ticks_cell(w.km_median()),
            ticks_cell(st.km_median()),
            ticks_cell(f.drained_quantile(0.5)),
            ticks_cell(f.km_median()),
        );
    }
    let sweep = SweepReading::of(runs.iter().copied());
    println!("\n#### M4. The sweep's quantities at this λ\n");
    sweep_header();
    sweep_line(&leaching_label(rows), &sweep, &whole, &found);
    println!();
}

fn sweep_header() {
    println!(
        "| λ | runs | persisted | nutrient lockup | other verdicts | carcass-locked (gate window) | carcass-locked (settled mean) | leached % of carcass N out | leached %, settled | light-fed mixotroph % of settled carcass structure | t* | never drained % | t* not bitten | KM median not bitten |"
    );
    println!("|---|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
}

fn sweep_line(label: &str, r: &SweepReading, d: &FirstDrain, found: &FirstDrain) {
    println!(
        "| {label} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        r.runs,
        r.count("persisted"),
        r.count("nutrient_lockup"),
        other_verdicts(r),
        frac_cell(r.carcass_locked_tail),
        frac_cell(r.carcass_locked_settled),
        pct_of(r.nutrient.leached_share()),
        pct_of(r.nutrient_settled.leached_share()),
        pct_of(r.lfm_share_settled),
        ticks_cell(d.drained_quantile(0.5)),
        pct_of(d.never_share()),
        ticks_cell(found.drained_quantile(0.5)),
        ticks_cell(found.km_median()),
    );
}

/// `--leaching-sweep a.jsonl,b.jsonl,…` (#700): M4's line per file, one
/// file per λ, as one table.
fn leaching_sweep(files: &[PathBuf]) {
    println!("## Carcass leaching sweep (#700)\n");
    println!(
        "One line per rows file. Verdicts are the census's per seed; carcass-locked is the dead pool's share of conserved nutrient (the lockup gate's series), its trailing mean over the gate's window and its settled-half mean, averaged over runs; leached % is carcass nutrient out by leaching against drains; the light-fed mixotrophs' share is H's second-half bucket; t* is the median time to first drain over drained carcasses, whole run; *not bitten* keeps the carcasses not drained while living in the step they died (KM: undrained censored).\n"
    );
    sweep_header();
    for f in files {
        let rows: Vec<Row> = read_rows(f);
        let runs = rows.iter().flat_map(|r| &r.tally.leach_runs);
        let label = format!("{} ({})", leaching_label(&rows), f.display());
        sweep_line(
            &label,
            &SweepReading::of(runs),
            &first_drain_of(&rows, false, Deaths::All),
            &first_drain_of(&rows, false, Deaths::Unbitten),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// `n` producers with ids from `first`, all at effective mobility `m`,
    /// each moving `moved` per tick.
    fn producers(first: u64, n: u64, m: f32, moved: f32) -> Vec<(u64, f32, Option<f32>)> {
        (first..first + n).map(|id| (id, m, Some(moved))).collect()
    }

    #[test]
    fn a_still_producer_is_not_counted_as_mobile_whatever_it_breeds() {
        let mut t = MobileTracker::default();
        for _ in 0..10 {
            t.sample(&producers(0, 2 * GUILD_MIN_SIZE as u64, 0.0, 0.0));
            t.birth(0, 0.0);
        }
        let c = t.finish(true);
        assert_eq!(c.samples, 20 * GUILD_MIN_SIZE as u64);
        for above in &c.above {
            assert_eq!(above.samples, 0);
            assert_eq!((above.births, above.breeders), (0, 0));
            assert_eq!(above.guild_seeds, 0);
        }
    }

    #[test]
    fn a_moving_producer_population_that_breeds_is_counted() {
        let n = GUILD_MIN_SIZE as u64;
        let mut t = MobileTracker::default();
        for _ in 0..10 {
            // n movers at m = 0.25 beside n still producers.
            let mut p = producers(0, n, 0.25, 0.2);
            p.extend(producers(100, n, 0.0, 0.0));
            t.sample(&p);
        }
        t.birth(0, 0.25);
        t.birth(0, 0.25);
        t.birth(1, 0.25);
        let c = t.finish(true);
        assert_eq!(c.samples, 20 * n);
        // Thresholds 0.05, 0.1, 0.2 sit below 0.25; 0.3 and 0.5 above it.
        for (above, counted) in c.above.iter().zip([true, true, true, false, false]) {
            let k = u64::from(counted);
            assert_eq!(above.samples, k * 10 * n);
            assert_eq!((above.births, above.breeders), (k * 3, k * 2));
            assert_eq!(above.guild_seeds, k);
            assert_eq!(above.movement_n, k * 10 * n);
        }
        assert!((c.above[0].movement_sum - 0.2 * 10.0 * n as f64).abs() < 1e-4);
        // A bin midpoint (0.2505) is capped at the exact max.
        assert_eq!(c.mobility.quantile(0.75), Some(0.25));
        assert_eq!(c.mobility.quantile(0.25), Some(0.0005));
        assert_eq!(c.mobility.quantile(1.0), Some(0.25f32 as f64));
    }

    #[test]
    fn mobile_producers_that_dip_below_the_floor_or_never_breed_do_not_hold() {
        let n = GUILD_MIN_SIZE as u64;
        // Sustained, never breeding.
        let mut sterile = MobileTracker::default();
        for _ in 0..5 {
            sterile.sample(&producers(0, n, 0.6, 0.5));
        }
        let c = sterile.finish(true);
        assert_eq!((c.above[4].sustained_seeds, c.above[4].guild_seeds), (1, 0));
        // Breeding, but one sample tick below the floor.
        let mut dip = MobileTracker::default();
        dip.sample(&producers(0, n, 0.6, 0.5));
        dip.sample(&producers(0, n - 1, 0.6, 0.5));
        dip.birth(0, 0.6);
        let c = dip.finish(true);
        assert_eq!((c.above[4].breeding_seeds, c.above[4].guild_seeds), (1, 0));
        // Both, but the run stopped early.
        let mut stopped = MobileTracker::default();
        stopped.sample(&producers(0, n, 0.6, 0.5));
        stopped.birth(0, 0.6);
        assert_eq!(stopped.finish(false).above[4].guild_seeds, 0);
    }

    #[test]
    fn mobile_censuses_merge_across_seeds() {
        let n = GUILD_MIN_SIZE as u64;
        let seed = |m: f32| {
            let mut t = MobileTracker::default();
            t.sample(&producers(0, n, m, 0.1));
            t.birth(0, m);
            t.finish(true)
        };
        let mut c = MobileCensus::default();
        c.merge(&seed(0.6));
        c.merge(&seed(0.0));
        assert_eq!((c.seeds, c.completed_seeds, c.samples), (2, 2, 2 * n));
        assert_eq!(c.above[0].guild_seeds, 1);
        assert_eq!(c.above[0].breeders, 1);
        assert_eq!(c.mobility.n, 2 * n);
        assert_eq!(c.movement.mean(), Some(0.1f32 as f64));
    }

    #[test]
    fn rows_from_before_the_mobile_census_read_back_empty() {
        let mut t = MobileTracker::default();
        t.sample(&producers(0, 3, 0.4, 0.1));
        let mut r = row(None, None);
        r.tally.ext.mobile = t.finish(true);
        let mut json = serde_json::to_value(&r).unwrap();
        let back: Row = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(back.tally.ext.mobile, r.tally.ext.mobile);
        let ext = json["tally"]["ext"].as_object_mut().unwrap();
        assert!(ext.remove("mobile").is_some());
        let back: Row = serde_json::from_value(json).unwrap();
        assert_eq!(back.tally.ext.mobile.seeds, 0);
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
            uptake_structure_exponent: None,
            uptake_reference_structure: None,
            network: None,
            atlas: atlas.map(str::to_string),
            cross_trait_cost: None,
            maintenance_cost_exponent: None,
            leaching_rate: None,
            hyphal_uptake: None,
            contact_distance: None,
            seed_kin_kills: Vec::new(),
            tally: Tally::new(),
        }
    }

    /// #700: section M's readings pool every seed's carcasses, censored at
    /// that seed's end, split by window (settled half from T/2 + 1) and by
    /// how the carcass died; rows from before #700 read back with none.
    #[test]
    fn leaching_readings_split_by_window_and_by_how_the_carcass_died() {
        use explorers_search::leaching::CarcassFate;
        let fate = |died, drained, bitten| CarcassFate {
            died,
            drained,
            bitten,
        };
        let mut r = row(None, Some(0.0));
        r.tally.leach_runs = vec![
            LeachRun {
                end_tick: 2000,
                carcasses: vec![fate(10, Some(11), true), fate(1500, Some(1540), false)],
                ..Default::default()
            },
            LeachRun {
                end_tick: 900,
                carcasses: vec![fate(800, None, false)],
                ..Default::default()
            },
        ];
        let rows = [r];
        let all = first_drain_of(&rows, false, Deaths::All);
        assert_eq!((all.carcasses, all.drained), (3, 2));
        let settled = first_drain_of(&rows, true, Deaths::All);
        assert_eq!(
            (settled.carcasses, settled.drained_quantile(0.5)),
            (1, Some(40.0))
        );
        let unbitten = first_drain_of(&rows, false, Deaths::Unbitten);
        // 40 drained; 100 censored at its seed's end (900): S(40) = 1/2.
        assert_eq!(unbitten.km_median(), Some(40.0));
        assert_eq!(first_drain_of(&rows, false, Deaths::Bitten).carcasses, 1);

        let mut json = serde_json::to_value(Tally::new()).unwrap();
        json.as_object_mut().unwrap().remove("leach_runs").unwrap();
        let back: Tally = serde_json::from_value(json).unwrap();
        assert!(back.leach_runs.is_empty());
    }

    fn unit_atlas(units: &[f64]) -> AtlasUnits {
        let dims = explorers_search::search::default_ranges();
        AtlasUnits::new(
            dims.clone(),
            units.iter().map(|&u| vec![u; dims.len()]).collect(),
        )
    }

    /// #682: each row's lineage cluster is its atlas cell's Ward cluster
    /// over every cell of the atlas, and only on the atlas the rows ran on.
    #[test]
    fn rows_take_their_cells_lineage_cluster_on_their_own_atlas() {
        // Cells 0, 2 and 4 near 0; 1 and 3 near 1. Rows for cells 3, 0, 4.
        let atlas = unit_atlas(&[0.0, 0.9, 0.05, 0.95, 0.1]);
        let fp = format!("{:016x}", atlas.fingerprint());
        let rows: Vec<Row> = [3, 0, 4]
            .iter()
            .map(|&i| Row {
                config_index: i,
                ..row(Some(&fp), None)
            })
            .collect();
        assert_eq!(lineage_clusters(&rows, &atlas, 2), Ok(vec![1, 0, 0]));
        // Another atlas's rows, or rows without the fingerprint, do not.
        let other = unit_atlas(&[0.0, 0.9, 0.05, 0.95, 0.2]);
        assert!(lineage_clusters(&rows, &other, 2).is_err());
        let unprinted = [row(None, None)];
        assert!(lineage_clusters(&unprinted, &atlas, 2).is_err());
        let mut sampled = row(Some(&fp), None);
        sampled.source = ConfigSource::SAMPLE;
        assert!(lineage_clusters(&[sampled], &atlas, 2).is_err());
    }

    /// #682: drains per seed, settled half, light-fed mixotrophs (bucket 0)
    /// and decomposers by role (bucket 3), over every row's seeds.
    #[test]
    fn drains_are_read_per_seed_over_the_settled_half() {
        let mut a = row(None, None);
        a.seed_kin_kills = vec![0; 2];
        a.tally.ext = Ext::new();
        a.tally.ext.routes_second_half = routes([30.0, 1.0, 2.0, 10.0, 0.0], [0.0; 5], [0.0; 5]);
        let mut b = a.clone();
        b.seed_kin_kills = vec![0; 3];
        let mut t = Tally::new();
        t.merge(&a.tally);
        t.merge(&b.tally);
        let d = drains(&[a, b], &t.ext).unwrap();
        assert!(close(d.mixotrophs, 12.0));
        assert!(close(d.decomposers, 4.0));
        assert_eq!(drains(&[], &Tally::new().ext), None);
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

    fn sample_params(i: usize) -> WorldParameters {
        resolve_config(
            ConfigSource::SAMPLE,
            i,
            &Default::default(),
            &sampled_units(),
        )
        .0
    }

    /// #681: the charge is the stepper's own cross-trait term,
    /// `c_AH × (A·H)^(p/2)` on raw A and H: zero for a specialist in either
    /// trait, the closed form for a mixotroph, and exactly zero at c_AH = 0.
    #[test]
    fn the_cross_trait_charge_spares_specialists_and_matches_the_closed_form() {
        let params = WorldParameters {
            cross_trait_cost: 0.08,
            maintenance_cost_exponent: 3.0,
            ..sample_params(0)
        };
        assert_eq!(cross_trait_charge(0.0, 0.9, &params), 0.0);
        assert_eq!(cross_trait_charge(0.7, 0.0, &params), 0.0);
        // 0.08 × (0.5 × 0.5)^1.5 = 0.08 × 0.125 = 0.01.
        assert!((cross_trait_charge(0.5, 0.5, &params) - 0.01).abs() < 1e-7);
        let off = WorldParameters {
            cross_trait_cost: 0.0,
            ..params
        };
        assert_eq!(cross_trait_charge(0.5, 0.5, &off), 0.0);
        assert_eq!(cross_trait_charge(1.3, 2.1, &off), 0.0);
    }

    /// #681: decomposers are read by trophic role alone, whatever they
    /// invest in; the intermediates are mixotrophs by investment (raw A and
    /// H both above 0.05) that are not producers by role — and an agent with
    /// no income yet has no role, so is neither.
    #[test]
    fn charge_groups_follow_trophic_role_and_investment_profile() {
        use TrophicRole::*;
        let groups = |role, a, h| charge_groups(role, a, h);
        // Decomposer by role, specialist or mixotroph by investment.
        assert_eq!(groups(Some(Decomposer), 0.0, 0.8), [true, false]);
        assert_eq!(groups(Some(Decomposer), 0.3, 0.8), [true, true]);
        // A mixotroph by investment that is a consumer by role.
        assert_eq!(groups(Some(Consumer), 0.3, 0.8), [false, true]);
        // A producer by role is never an intermediate, however mixed.
        assert_eq!(groups(Some(Producer), 0.3, 0.8), [false, false]);
        // At or below the threshold in either trait is not a mixotroph.
        assert_eq!(groups(Some(Consumer), 0.05, 0.8), [false, false]);
        assert_eq!(groups(Some(Consumer), 0.3, 0.05), [false, false]);
        assert_eq!(groups(None, 0.3, 0.8), [false, false]);
    }

    /// #681: a group's read over its agent entries `[A, charge per tick,
    /// ticks, Σ income]`: autotrophy weighted by agent-samples, the charge
    /// as a fraction of income (ratio of sums, and the median per agent),
    /// and the group's share of all agent-samples.
    #[test]
    fn a_groups_charge_is_read_per_agent_sample_against_income() {
        let agents = [
            // A, charge/tick, ticks, Σ income.
            [0.125, 0.0625, 8.0, 1.0],
            [0.5, 0.125, 24.0, 4.0],
            [0.75, 0.25, 16.0, 0.0],
        ];
        let g = ChargeRead::of(&agents, 192);
        assert_eq!((g.agents, g.samples), (3, 48));
        assert!(close(g.share.unwrap(), 0.25));
        // (0.125 × 8 + 0.5 × 24 + 0.75 × 16) / 48.
        assert!(close(g.autotrophy_mean.unwrap(), 25.0 / 48.0));
        // Agent-samples sorted by A: 8 at 0.125, 24 at 0.5, 16 at 0.75 —
        // the 24th and 25th both carry 0.5.
        assert!(close(g.autotrophy_median.unwrap(), 0.5));
        // Charged 0.5 + 3 + 4 against income 5.
        assert!(close(g.charge_of_income.unwrap(), 1.5));
        // Per agent with income: 0.5 / 1 and 3 / 4 -> median 0.625.
        assert!(close(g.median_charge_of_income.unwrap(), 0.625));
        assert_eq!(g.zero_income, 1);
        let none = ChargeRead::of(&[], 0);
        assert_eq!(
            (none.share, none.autotrophy_median, none.charge_of_income),
            (None, None, None)
        );
    }

    /// #681: on one small run the readout's charges reconcile with the
    /// stepper's maintenance accounting — each `Metabolized` charge less the
    /// per-trait cost (`phase::metabolic_cost` at c_AH = 0) is the charge the
    /// readout books — and at c_AH = 0 every booked charge is exactly 0.
    #[test]
    fn charges_reconcile_with_the_steppers_maintenance_on_a_small_run() {
        let (params, dist) = resolve_config(
            ConfigSource::SAMPLE,
            14,
            &Default::default(),
            &sampled_units(),
        );
        let eval = EvalConfig::default();
        let on = WorldParameters {
            cross_trait_cost: 0.1,
            ..params.clone()
        };
        let t = rollout(&on, &dist, 1000, 200, &eval, true);
        let c = &t.charge_check;
        assert!(c.agent_ticks > 100, "{c:?}");
        assert_eq!(c.mismatches, 0, "{c:?}");
        assert!(c.readout > 0.0, "a mixotroph is charged: {c:?}");
        assert!((c.stepper - c.readout).abs() <= 1e-4 * c.readout, "{c:?}");
        assert!(t.ext.second_half_agent_ticks > 0);
        assert!(
            t.ext.charge.iter().all(|g| !g.is_empty()),
            "both groups are populated on sample:14"
        );
        let off = WorldParameters {
            cross_trait_cost: 0.0,
            ..params
        };
        let t = rollout(&off, &dist, 1000, 200, &eval, true);
        assert_eq!(t.charge_check.mismatches, 0);
        assert_eq!(t.charge_check.readout, 0.0);
        assert!(t.ext.charge.iter().any(|g| !g.is_empty()));
        for e in t.ext.charge.iter().flatten() {
            assert_eq!(e[1], 0.0, "{e:?}");
        }
    }

    /// Rows from before #681 (target/670) read back with K empty.
    #[test]
    fn rows_without_the_681_fields_still_read_back() {
        let mut r = row(Some("0123456789abcdef"), None);
        r.tally.ext.charge[0].push([0.25, 0.5, 3.0, 1.0]);
        r.tally.ext.second_half_agent_ticks = 9;
        let mut json = serde_json::to_value(&r).unwrap();
        let ext = json["tally"]["ext"].as_object_mut().unwrap();
        for k in ["charge", "second_half_agent_ticks"] {
            assert!(ext.remove(k).is_some(), "{k} is serialised");
        }
        let back: Row = serde_json::from_value(json).unwrap();
        assert!(back.tally.ext.charge.iter().all(Vec::is_empty));
        assert_eq!(back.tally.ext.second_half_agent_ticks, 0);
        let read = ChargeRead::of(&back.tally.ext.charge[0], 0);
        assert_eq!((read.agents, read.share), (0, None));
    }

    /// An agent holding `free` unearmarked nutrient and `earmark` reproductive
    /// nutrient, on `reserve` unallocated and `allocation` reproductive reserve.
    fn holding(
        id: u64,
        free: f32,
        earmark: f32,
        reserve: f32,
        allocation: f32,
    ) -> explorers_sim::Agent {
        let traits = explorers_sim::TraitVector {
            photosynthetic_absorption: 0.5,
            heterotrophy: 0.5,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        };
        let mut a = explorers_sim::Agent::new(id, (0.0, 0.0), reserve, 1.0, free, traits);
        a.repro_nutrient = earmark;
        a.repro_reserve = allocation;
        a
    }

    /// #728: free nutrient ÷ reserve is (free store + earmark) ÷ (reserve +
    /// allocation), its median taken per role group; agents with no reserve
    /// are counted, not sampled; agents with no role are in neither group.
    #[test]
    fn the_partner_readout_takes_each_role_groups_median_free_nutrient_per_reserve() {
        let mut s = PartnerSamples::default();
        let c = Some(TrophicRole::Consumer);
        let d = Some(TrophicRole::Decomposer);
        let p = Some(TrophicRole::Producer);
        s.sample(c, &holding(1, 1.0, 1.0, 1.0, 1.0)); // 1
        s.sample(d, &holding(2, 3.0, 1.0, 1.0, 1.0)); // 2
        s.sample(c, &holding(3, 6.0, 0.0, 2.0, 0.0)); // 3
        s.sample(d, &holding(4, 9.0, 0.0, 0.0, 0.0)); // no reserve
        s.sample(p, &holding(5, 1.0, 0.0, 4.0, 0.0)); // 0.25
        s.sample(p, &holding(6, 1.0, 1.0, 3.0, 1.0)); // 0.5
        s.sample(None, &holding(7, 9.0, 9.0, 1.0, 0.0));
        let run = s.finish(1000, "persisted", [0.0; 3]);
        assert_eq!((run.heterotroph_samples, run.producer_samples), (3, 2));
        assert_eq!(run.zero_reserve, 1);
        assert_eq!(run.heterotroph, Some(2.0));
        assert_eq!(run.producer, Some(0.375));
        assert!(close(run.gradient().unwrap(), 2.0 / 0.375));
    }

    /// Runs that did not persist, or lack either role, carry no gradient.
    #[test]
    fn a_run_lacking_a_role_or_not_persisting_has_no_gradient() {
        let mut s = PartnerSamples::default();
        s.sample(Some(TrophicRole::Producer), &holding(1, 1.0, 0.0, 1.0, 0.0));
        assert_eq!(s.clone().finish(1, "persisted", [0.0; 3]).gradient(), None);
        s.sample(Some(TrophicRole::Consumer), &holding(2, 2.0, 0.0, 1.0, 0.0));
        assert_eq!(
            s.clone().finish(1, "persisted", [0.0; 3]).gradient(),
            Some(2.0)
        );
        assert_eq!(s.finish(1, "nutrient_lockup", [0.0; 3]).gradient(), None);
    }

    /// #728 on one small run: each seed books one partner readout over the
    /// settled half, both role groups sampled; with hyphal uptake on some of
    /// the heterotrophs' pool uptake is hyphal, and with it off none is.
    #[test]
    fn each_run_books_its_partner_readout_and_the_hyphal_share_is_zero_off() {
        let decoded = resolve_config(
            ConfigSource::SAMPLE,
            14,
            &Default::default(),
            &sampled_units(),
        );
        let eval = EvalConfig::default();
        let (on, dist) = with_hyphal_uptake(decoded.clone(), Some(true), None);
        let t = rollout(&on, &dist, 1000, 300, &eval, false);
        let [run] = &t.partner_runs[..] else {
            panic!("{:?}", t.partner_runs)
        };
        assert_eq!(run.seed, 1000);
        assert!(
            run.heterotroph_samples > 0 && run.producer_samples > 0,
            "{run:?}"
        );
        assert!(
            run.hyphal_n > 0.0 && run.hyphal_n <= run.uptake_n,
            "{run:?}"
        );
        assert!(close(
            run.hyphal_n,
            t.ext.routes_second_half.hyphal[2] + t.ext.routes_second_half.hyphal[3]
        ));
        let t = rollout(&decoded.0, &decoded.1, 1000, 300, &eval, false);
        assert_eq!(t.partner_runs[0].hyphal_n, 0.0);
        assert_eq!(t.partner_runs[0].hyphal_share(), Some(0.0));
    }

    /// A persisted run with medians `h` / `p` (`None`: that group unsampled).
    fn partner(outcome: &str, h: Option<f64>, p: Option<f64>) -> PartnerRun {
        PartnerRun {
            outcome: outcome.into(),
            heterotroph_samples: u64::from(h.is_some()),
            producer_samples: u64::from(p.is_some()),
            heterotroph: h,
            producer: p,
            ..Default::default()
        }
    }

    /// A row for atlas config `idx` carrying these partner runs.
    fn partner_row(idx: usize, runs: Vec<PartnerRun>) -> Row {
        let mut r = row(None, None);
        r.config_index = idx;
        r.tally.partner_runs = runs;
        r
    }

    /// #728: a world's gradient is the median over its persisted runs that
    /// carry both roles; the rest are counted by why they are out.
    #[test]
    fn a_worlds_gradient_is_the_median_over_its_qualifying_runs() {
        let w = PartnerWorld::of(&partner_row(
            3,
            vec![
                partner("persisted", Some(2.0), Some(1.0)),
                partner("persisted", Some(0.5), Some(1.0)),
                partner("persisted", Some(3.0), Some(1.0)),
                partner("persisted", None, Some(1.0)),
                partner("persisted", Some(1.0), None),
                partner("nutrient_lockup", Some(9.0), Some(1.0)),
            ],
        ));
        assert_eq!((w.runs, w.persisted), (6, 5));
        assert_eq!((w.lacking_heterotroph, w.lacking_producer), (1, 1));
        assert_eq!(w.gradients, vec![2.0, 0.5, 3.0]);
        assert_eq!(w.gradient, Some(2.0));
        assert!(w.carries() && w.above_one());

        let none = PartnerWorld::of(&partner_row(4, vec![partner("persisted", None, Some(1.0))]));
        assert!(!none.carries() && !none.above_one());
        assert_eq!(none.gradient, None);
    }

    /// The summary counts the worlds carrying heterotrophs by role and those
    /// above 1 among them, and whether they are a majority.
    #[test]
    fn the_partner_summary_counts_worlds_above_one_among_those_carrying() {
        let rows = vec![
            partner_row(0, vec![partner("persisted", Some(2.0), Some(1.0))]),
            partner_row(1, vec![partner("persisted", Some(0.5), Some(1.0))]),
            partner_row(2, vec![partner("persisted", Some(3.0), Some(1.0))]),
            partner_row(3, vec![partner("persisted", None, Some(1.0))]),
        ];
        let s = PartnerTally::of(&rows);
        assert_eq!((s.worlds, s.carrying, s.above_one), (4, 3, 2));
        assert!(s.majority());
        let s = PartnerTally::of(&rows[..2]);
        assert_eq!((s.carrying, s.above_one), (2, 1));
        assert!(!s.majority(), "half is not a majority");
    }

    /// #728: the paired sign test pairs worlds by config across the two
    /// files on the gradient > 1 indicator. Fixture: 7 worlds above 1 only
    /// with the switch on, 1 only off, 2 both, 2 neither (one of those
    /// carrying no heterotrophs in either arm), and one world in each file
    /// alone. Discordant n = 8, k = 7: two-sided p = 2 × 9 / 256, one-sided
    /// (on > off) p = 9 / 256.
    #[test]
    fn the_paired_sign_test_counts_discordant_worlds_and_reads_exact_p() {
        let above = || vec![partner("persisted", Some(2.0), Some(1.0))];
        let below = || vec![partner("persisted", Some(0.5), Some(1.0))];
        let absent = || vec![partner("persisted", None, Some(1.0))];
        let (mut on, mut off) = (Vec::new(), Vec::new());
        for i in 0..7 {
            on.push(partner_row(i, above()));
            off.push(partner_row(i, below()));
        }
        on.push(partner_row(7, below()));
        off.push(partner_row(7, above()));
        for i in 8..10 {
            on.push(partner_row(i, above()));
            off.push(partner_row(i, above()));
        }
        on.push(partner_row(10, below()));
        off.push(partner_row(10, below()));
        on.push(partner_row(11, absent()));
        off.push(partner_row(11, absent()));
        on.push(partner_row(12, above()));
        off.push(partner_row(13, above()));
        for r in &mut on {
            r.hyphal_uptake = Some(true);
        }
        let t = SignTest::of(&on, &off);
        assert_eq!(t.pairs, 12);
        assert_eq!((t.unpaired_on, t.unpaired_off), (1, 1));
        assert_eq!((t.on_only, t.off_only, t.both, t.neither), (7, 1, 2, 2));
        assert_eq!(t.discordant(), 8);
        assert!(close(t.two_sided_p(), 18.0 / 256.0));
        assert!(close(t.one_sided_p(), 9.0 / 256.0));

        // The report reads the same numbers.
        let report = partner_report(&on, Some(("off.jsonl", &off)));
        assert!(
            report.contains("carrying heterotrophs by role: **12** of 13"),
            "{report}"
        );
        assert!(
            report.contains("gradient > 1: **10** of 12 (83.3 %), a majority: **yes**"),
            "{report}"
        );
        assert!(
            report.contains("12 paired worlds (1 on this run only, 1 in the baseline only)"),
            "{report}"
        );
        assert!(
            report.contains("discordant n = **8**, k = **7**"),
            "{report}"
        );
        assert!(report.contains("two-sided p = **0.0703**"), "{report}");
        assert!(
            report.contains("one-sided p (on > off) = **0.0352**"),
            "{report}"
        );
        assert!(!report.contains("baseline differs"), "{report}");
        // A baseline that ran with the switch on is flagged.
        let report = partner_report(&on, Some(("on.jsonl", &on)));
        assert!(report.contains("baseline differs"), "{report}");
        let report = partner_report(&on, None);
        assert!(report.contains("No baseline"), "{report}");
    }

    /// Rows from before #728 read back with the switch off (no setting, no
    /// partner runs, no hyphal route), and the label says so.
    #[test]
    fn rows_without_the_728_fields_read_back_as_switch_off() {
        let mut r = row(None, None);
        r.hyphal_uptake = Some(true);
        r.contact_distance = Some(0.05);
        r.tally
            .partner_runs
            .push(partner("persisted", Some(2.0), Some(1.0)));
        assert!(hyphal_label(std::slice::from_ref(&r)).contains("on, d_c = 0.05"));
        let mut json = serde_json::to_value(&r).unwrap();
        let o = json.as_object_mut().unwrap();
        for k in ["hyphal_uptake", "contact_distance"] {
            assert!(o.remove(k).is_some(), "{k} is serialised");
        }
        assert!(
            json["tally"]
                .as_object_mut()
                .unwrap()
                .remove("partner_runs")
                .is_some()
        );
        for routes in ["routes", "routes_second_half"] {
            let r = json["tally"]["ext"][routes].as_object_mut().unwrap();
            assert!(r.remove("hyphal").is_some());
        }
        let back: Row = serde_json::from_value(json).unwrap();
        assert_eq!((back.hyphal_uptake, back.contact_distance), (None, None));
        assert!(back.tally.partner_runs.is_empty());
        assert_eq!(back.tally.ext.routes.hyphal, vec![0.0; RECIPIENTS.len()]);
        assert!(hyphal_label(&[back]).contains("off (unrecorded, pre-#728 row)"));
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

    /// #740 on one small run: each seed books one growth-limitation readout
    /// over the settled half's sample ticks, by the sample's role; every
    /// growth event is nutrient- or energy-limited, and the free nutrient left
    /// is booked on energy-limited events only.
    #[test]
    fn each_run_books_its_growth_events_by_role() {
        let decoded = resolve_config(
            ConfigSource::SAMPLE,
            14,
            &Default::default(),
            &sampled_units(),
        );
        let eval = EvalConfig::default();
        let t = rollout(&decoded.0, &decoded.1, 1000, 300, &eval, false);
        let [run] = &t.limitation_runs[..] else {
            panic!("{:?}", t.limitation_runs)
        };
        assert_eq!(run.seed, 1000);
        assert_eq!(run.outcome, t.partner_runs[0].outcome);
        assert!(run.events[LIMIT_PRODUCER] > 0, "{run:?}");
        for r in 0..LIMIT_ROLES.len() {
            assert!(run.nutrient_limited[r] <= run.events[r], "{run:?}");
            assert!(run.free_nutrient_left[r] >= 0.0, "{run:?}");
            if run.nutrient_limited[r] == run.events[r] {
                assert_eq!(run.free_nutrient_left[r], 0.0, "{run:?}");
            }
        }
    }

    /// A run with `[events, nutrient-limited]` per role (producers,
    /// consumers, decomposers) and `left` free nutrient per energy-limited
    /// event.
    fn limited(outcome: &str, counts: [[u64; 2]; 3], left: f64) -> LimitationRun {
        let mut r = LimitationRun {
            outcome: outcome.into(),
            ..Default::default()
        };
        for (i, [n, nl]) in counts.into_iter().enumerate() {
            r.events[i] = n;
            r.nutrient_limited[i] = nl;
            r.free_nutrient_left[i] = left * (n - nl) as f64;
        }
        r
    }

    /// A row for atlas config `idx` carrying these limitation runs.
    fn limitation_row(idx: usize, runs: Vec<LimitationRun>) -> Row {
        let mut r = row(None, None);
        r.config_index = idx;
        r.tally.limitation_runs = runs;
        r
    }

    /// #740: a world pools its runs' growth events per role; it passes
    /// item 1 when producers are nutrient-limited in a larger share than
    /// decomposers and decomposers energy-limited in a larger share than
    /// producers.
    #[test]
    fn a_world_pools_its_growth_events_and_reads_complementary_limitation() {
        let w = LimitationWorld::of(&limitation_row(
            2,
            vec![
                limited("persisted", [[4, 3], [2, 1], [4, 1]], 0.5),
                limited("nutrient_lockup", [[4, 3], [0, 0], [0, 0]], 0.5),
            ],
        ));
        assert_eq!(w.runs, 2);
        assert_eq!(w.nutrient_share(LIMIT_PRODUCER), Some(0.75));
        assert_eq!(w.energy_share(LIMIT_DECOMPOSER), Some(0.75));
        assert_eq!(w.nutrient_share(LIMIT_CONSUMER), Some(0.5));
        assert_eq!(w.mean_free_nutrient_left(LIMIT_DECOMPOSER), Some(0.5));
        assert!(w.carries_both() && w.passes());
        // Producers no more nutrient-limited than decomposers: fails.
        let w = LimitationWorld::of(&limitation_row(
            3,
            vec![limited("persisted", [[4, 1], [0, 0], [4, 1]], 0.0)],
        ));
        assert!(w.carries_both() && !w.passes());
        assert_eq!(w.nutrient_share(LIMIT_CONSUMER), None);
        // No decomposer growth events: not carrying both roles.
        let w = LimitationWorld::of(&limitation_row(
            4,
            vec![limited("persisted", [[4, 4], [2, 0], [0, 0]], 0.0)],
        ));
        assert!(!w.carries_both() && !w.passes());
        assert_eq!(w.mean_free_nutrient_left(LIMIT_PRODUCER), None);
    }

    /// #740: the section counts the worlds carrying both roles, those passing
    /// item 1 and whether that is a majority, lists each world's shares and
    /// leftover free nutrient, and counts rows from before #740 (which read
    /// back with no limitation runs).
    #[test]
    fn the_limitation_report_counts_worlds_passing_item_1_and_reads_old_rows_empty() {
        let mut json = serde_json::to_value(Tally::new()).unwrap();
        json.as_object_mut()
            .unwrap()
            .remove("limitation_runs")
            .unwrap();
        let back: Tally = serde_json::from_value(json).unwrap();
        assert!(back.limitation_runs.is_empty());
        let mut old = row(None, None);
        old.tally = back;
        old.config_index = 9;
        old.seed_kin_kills = vec![0, 0];
        let rows = vec![
            limitation_row(
                1,
                vec![limited("persisted", [[4, 3], [0, 0], [4, 1]], 0.25)],
            ),
            limitation_row(2, vec![limited("persisted", [[4, 1], [0, 0], [4, 1]], 0.0)]),
            limitation_row(3, vec![limited("persisted", [[4, 4], [2, 0], [0, 0]], 0.0)]),
            old,
        ];
        let t = LimitationTally::of(&rows);
        assert_eq!((t.worlds, t.carrying, t.passing), (4, 2, 1));
        assert!(!t.majority());
        let report = limitation_report(&rows);
        assert!(report.contains("### O. Growth limitation"), "{report}");
        assert!(
            report.contains(
                "Worlds carrying both producers and decomposers by role: **2** of 4; of them, passing item 1: **1** of 2 (50.0 %), a majority: **no**."
            ),
            "{report}"
        );
        assert!(report.contains("2 rows' seeds predate #740"), "{report}");
        // Config 1: producers 75 % nutrient-limited, decomposers 75 %
        // energy-limited, 0.25 left per energy-limited event.
        assert!(
            report.contains("| atlas:1 | 1 | 4 / 75.0 / 25.0 | 0 / – / – | 4 / 25.0 / 75.0 | 0.2500 / – / 0.2500 | yes |"),
            "{report}"
        );
        assert!(report.contains("| atlas:3 |"), "{report}");
        assert!(report.contains("not carrying both"), "{report}");
    }
}
