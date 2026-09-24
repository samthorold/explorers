pub mod ensemble;
pub mod guild;

use explorers_sim::event::EventKind;
use explorers_sim::topology::TopologyProjection;

/// The event kinds the evaluator consumes — the retention a rollout applies
/// with `World::retain_event_kinds` so a settled-community horizon fits in
/// memory (#502). Every read the evaluator takes off the log is taken tick by
/// tick in [`RolloutObservations::observe`], so a rollout may also drop the
/// log before [`RolloutObservations::consumed_events`] after each step; what
/// it keeps at all is this list. Audit of the reads, so a future read that
/// needs another kind knows to add it here:
///
/// - `Reproduced`, `Died` — counted for the turnover term
///   (`turnover_score`; `total_births` / `total_deaths`).
/// - `Consumed`, `Reproduced`, `Died` — walked by the
///   [`TopologyProjection`] whose realised-diet edges classify each roster
///   sample into trophic roles for the heterotroph guild read (#490).
/// - `Born` — the guild's recruitment clause: a second-half birth naming a
///   guild member as a parent ([`guild::Birth`]).
///
/// Nothing else the evaluator computes reads the log: the energy-death,
/// lockup, oscillation and coexistence reads are off the per-tick series and
/// snapshots in [`RolloutObservations`], and clustering, trophic balance,
/// monoculture and generalist dominance read the final roster.
pub const EVALUATOR_EVENT_KINDS: &[EventKind] = &[
    EventKind::Consumed,
    EventKind::Reproduced,
    EventKind::Died,
    EventKind::Born,
];

#[derive(Debug, Clone, PartialEq)]
pub enum FailureMode {
    Extinction,
    PopulationExplosion,
    EnergyDeath,
    Monoculture,
    GeneralistDominance,
    /// Nutrient sequestered irreversibly into the dead pool: carcasses
    /// accumulate faster than the living decomposers turn them over, starving
    /// the living system of nutrient (issue #342). The nutrient-side sibling of
    /// `EnergyDeath` — distinct pool, distinct quantity.
    NutrientLockup,
}

#[derive(Debug, Clone)]
pub struct FitnessBreakdown {
    pub fitness: f32,
    pub failure: Option<FailureMode>,
    pub oscillation_strength: f32,
    pub clustering_strength: f32,
    pub coexistence_duration: f32,
    pub turnover_score: f32,
    pub trophic_balance_score: f32,
    pub ticks_survived: u64,
    /// Genesis behaviour axis iii (genesis-search.md): the dead pool's share of
    /// conserved nutrient, as the trailing-window mean of the per-tick carcass
    /// fraction the lockup gate reads. An additive, read-only descriptor — the
    /// atlas bins worlds on it but it is never summed into fitness. Zero on the
    /// gated (degenerate) path, where no behaviour coordinate is meaningful.
    pub carcass_locked_fraction: f32,
    /// Genesis decomposer-guild signal (genesis-search.md, the authority
    /// boundary): whether a decomposer *guild* — a population, not a role tag on
    /// one individual — held over the second half of the run: at least
    /// [`guild::GUILD_MIN_SIZE`] agents classified `Decomposer` on every sampled
    /// tick, with at least one birth to a member in that window (#490). A
    /// reported observable — never a behaviour axis, never a fitness term. The
    /// atlas aggregates it across a cell's seed ensemble into a *fraction of
    /// seeds*, honouring the existence-vs-distributional boundary. Read even on
    /// a world gated at the horizon, whose settled window is classified all the
    /// same (#527); false on the early-stop path, which has no such window.
    pub has_decomposer_guild: bool,
    /// The consumer twin of `has_decomposer_guild`: the same guild read for
    /// agents classified `Consumer`. Same authority boundary.
    pub has_consumer_guild: bool,
}

impl FitnessBreakdown {
    /// The breakdown of a world that hit a terminal gate at `ticks_survived`:
    /// zero fitness, the mode, every *scored* descriptor zero, and the guild
    /// observables as read. A degenerate world has no meaningful behaviour
    /// coordinate (it is routed to the dead frontier by cliff, not binned), so
    /// nothing is scored — whether the gate fired at the horizon or stopped
    /// the rollout where it died. The guild flags are not coordinates but
    /// reported observables, so they carry `guilds` instead (#527): a world
    /// gated on its terminal read has a settled window `(T/2, T]` that was
    /// classified during observation, and the read over it is as true of a
    /// monoculture as of a live world. A rollout stopped *before* the horizon
    /// has no settled window to read and passes [`guild::RoleGuilds::default`],
    /// so "no guild" and "not read" stay indistinguishable there — accepted
    /// rather than made tri-state (#527 triage).
    pub fn gated(failure: FailureMode, ticks_survived: u64, guilds: guild::RoleGuilds) -> Self {
        FitnessBreakdown {
            fitness: 0.0,
            failure: Some(failure),
            oscillation_strength: 0.0,
            clustering_strength: 0.0,
            coexistence_duration: 0.0,
            turnover_score: 0.0,
            trophic_balance_score: 0.0,
            ticks_survived,
            carcass_locked_fraction: 0.0,
            has_decomposer_guild: guilds.decomposer,
            has_consumer_guild: guilds.consumer,
        }
    }
}

/// The smallest living roster the roster gates (monoculture, generalist
/// dominance) read: below it a population is too small to carry a trait
/// distribution worth classifying. Also the population floor of "visibly
/// alive at the horizon" in the zero-false-positive checks a gate must clear
/// before promotion (viability.md).
pub const ROSTER_FLOOR: usize = 20;

#[derive(Clone, Debug)]
pub struct EvalConfig {
    pub max_population: usize,
    pub energy_death_window: usize,
    pub nutrient_lock_window: usize,
    /// Tick interval at which the rollout snapshots living-population trait vectors
    /// for the coexistence descriptor (issue #394). Coarse, not per-tick: DBSCAN is
    /// O(n²) and `max_population` is large, so clustering every tick of every seed
    /// is disproportionate for one noisy 0.2-weight term. A co-presence *fraction*
    /// needs only a representative sample of the settled window `(T/2, T]`.
    pub coexistence_sample_interval: usize,
    pub clustering_threshold: f32,
    pub dbscan_eps: f32,
    pub dbscan_min_points: usize,
    pub generalist_threshold: f32,
    pub generalist_dominance_fraction: f32,
    /// Ticks of trajectory the gates' reference excludes: the founder cohort is
    /// *provisioned*, so until photosynthetic income overtakes the provisioning
    /// the stock series describes the founder budget, not the world. An
    /// absolute tick count sized from that provisioning transient — an
    /// ecological constant, not a fraction of the horizon (genesis-search.md,
    /// *The gates' reference excludes the founder transient*). Used only to
    /// start the energy-death and lockup references and to hold the roster
    /// gates (monoculture, generalist dominance) off the founder cohort; the
    /// behaviour axes read the settled window `(T/2, T]` instead. 260 is the
    /// measured value — the 90th percentile of the provisioning-transient
    /// tick over the live runs of the #505 sweep (atlas live cells + the
    /// 200-point LHS box, 8 seeds, 3000-tick horizon), rounded up to 10;
    /// `docs/research/505-settling-time.md` holds the distribution.
    pub grace_ticks: u64,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            max_population: 10_000,
            energy_death_window: 50,
            nutrient_lock_window: 50,
            coexistence_sample_interval: 10,
            clustering_threshold: 0.5,
            dbscan_eps: 1.0,
            dbscan_min_points: 5,
            generalist_threshold: 0.3,
            generalist_dominance_fraction: 0.5,
            grace_ticks: 260,
        }
    }
}

/// The per-tick series a rollout observes about a world, sampled over time and
/// handed to the evaluator as one bundle. Each field is one signal the caller
/// samples once per tick (or, for the snapshots, at a coarse interval) during
/// the rollout; the evaluator reads them to compute the descriptors that need
/// a temporal trace rather than just the final state. Bundling them keeps
/// `evaluate_from_log`'s signature from accreting a new slice parameter every
/// time a descriptor grows a per-tick appetite — the series are conceptually one
/// thing: what the rollout observed, sampled over time.
///
/// The bundle also carries every read the evaluator takes off the event log,
/// consumed tick by tick from the log's tail (#502): the turnover counts, the
/// `Born` descent facts, and the roster samples already classified into
/// trophic roles by the projection as of the sample tick. Reading them as they
/// happen is what lets a rollout keep only [`EVALUATOR_EVENT_KINDS`] and drop
/// the log before [`consumed_events`](Self::consumed_events) after every
/// step — the world stays history-free and so, at the horizon, does the log.
#[derive(Clone, Debug)]
pub struct RolloutObservations {
    /// Free (non-carcass-locked) energy stock per tick, for the energy-death
    /// stock-trend signal (issue #302).
    pub free_energy: Vec<f32>,
    /// Dead pool's share of system nutrient per tick, for the nutrient-lockup
    /// stock-trend signal (issue #342).
    pub carcass_fraction: Vec<f32>,
    /// Producer (autotroph) share of living energy per tick, for the oscillation
    /// producer↔consumer rhythm signal (issue #392).
    pub producer_share: Vec<f32>,
    /// Living-population trait vectors snapshotted at a coarse interval, each
    /// tagged with its tick, for the coexistence descriptor (issue #394). Sparse,
    /// not per-tick, because DBSCAN is O(n²) and clustering every tick of every
    /// seed is disproportionate for one noisy 0.2-weight term.
    pub cluster_snapshots: Vec<(u64, Vec<explorers_sim::TraitVector>)>,
    /// Living roster snapshotted at the same coarse interval, tagged with its
    /// tick and read into trophic roles by the projection as of that tick, for
    /// the heterotroph guild read (#490). The guild is a population over time
    /// — role count on every sampled tick of the second half plus recruitment
    /// — so it needs the roster *at each sample*, which the history-free world
    /// cannot supply after the fact; classifying at the sample is what lets
    /// the log history behind the projection be dropped (#502).
    pub role_snapshots: Vec<guild::RoleSnapshot>,
    /// `Reproduced` events seen so far — the turnover term's births.
    pub total_births: usize,
    /// `Died` events seen so far — the turnover term's deaths.
    pub total_deaths: usize,
    /// Every `Born` event's descent facts, for the guild's recruitment clause.
    pub born: Vec<guild::Birth>,
    /// The projection that classifies each roster sample, walked to the
    /// present on every `observe`.
    topology: TopologyProjection,
    /// Absolute log index up to which the counts and `born` have been read.
    log_cursor: usize,
}

impl Default for RolloutObservations {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

impl RolloutObservations {
    /// Pre-size the per-tick series for a run of `max_ticks`.
    pub fn with_capacity(max_ticks: usize) -> Self {
        Self {
            free_energy: Vec::with_capacity(max_ticks),
            carcass_fraction: Vec::with_capacity(max_ticks),
            producer_share: Vec::with_capacity(max_ticks),
            cluster_snapshots: Vec::new(),
            role_snapshots: Vec::new(),
            total_births: 0,
            total_deaths: 0,
            born: Vec::new(),
            topology: TopologyProjection::new(),
            log_cursor: 0,
        }
    }

    /// Record one stepped tick of `world`: the log tail since the last call
    /// (turnover counts, `Born` facts, the projection), the three per-tick
    /// series every tick, and the trait-vector and role snapshots when the
    /// world's tick is a multiple of `sample_interval` (the evaluator's
    /// `coexistence_sample_interval`). Pure observation — no clustering; the
    /// evaluator owns that. Role classification is the projection's read of
    /// the log as of this tick, taken here because it cannot be taken later
    /// from a log that has been dropped.
    pub fn observe(&mut self, world: &explorers_sim::World, sample_interval: usize) {
        let log = world.event_log();
        for event in log.since(self.log_cursor) {
            match event.kind {
                EventKind::Reproduced => self.total_births += 1,
                EventKind::Died => self.total_deaths += 1,
                EventKind::Born => self.born.extend(guild::Birth::of(event)),
                _ => {}
            }
        }
        self.log_cursor = log.len();
        // Events are stamped with the tick they happened in and the world's
        // counter advances after the step, so walking to the present here is
        // the projection as of `world.tick()`.
        self.topology.update(log);

        self.free_energy.push(world.free_energy());
        self.carcass_fraction
            .push(world.carcass_locked_nutrient_fraction());
        self.producer_share.push(world.producer_energy_share());
        if world.tick().is_multiple_of(sample_interval.max(1) as u64) {
            self.cluster_snapshots.push((
                world.tick(),
                world.agents().iter().map(|a| a.traits).collect(),
            ));
            self.role_snapshots.push((
                world.tick(),
                self.topology
                    .trophic_roles_of(world.agents().iter().map(|a| (a.id, &a.traits))),
            ));
        }
    }

    /// Absolute event-log index below which everything has been read: a
    /// rollout that wants to drop history once read compacts the world's log
    /// before it (`World::compact_event_log_before`) after each `observe`.
    pub fn consumed_events(&self) -> usize {
        self.log_cursor
    }
}

/// The terminal verdict on a rollout: the gates in order, then the five
/// fitness components. Unbounded — for callers with no wall-clock budget (the
/// tests, the genesis rollout driver). A budgeted caller uses
/// [`evaluate_from_log_within`], which reaches the same verdict when it
/// finishes.
pub fn evaluate_from_log(
    world: &explorers_sim::World,
    observations: &RolloutObservations,
    config: &EvalConfig,
    max_ticks: u64,
) -> FitnessBreakdown {
    evaluate(world, observations, config, max_ticks, None)
        .expect("an evaluation with no deadline always reaches a verdict")
}

/// [`evaluate_from_log`] under a cooperative wall-clock `deadline` (#523):
/// `None` — no verdict — when the deadline passes before the verdict is
/// reached. The deadline is read on entry, before the terminal roster's
/// clustering, and between the settled-window snapshots whose per-snapshot
/// DBSCAN is the unit of work on a dense roster, so an abandoned evaluation
/// returns on the calling thread and leaves nothing running. A verdict that
/// is reached is bit-identical to [`evaluate_from_log`]'s: the deadline only
/// compares a clock, and touches nothing the verdict reads.
pub fn evaluate_from_log_within(
    world: &explorers_sim::World,
    observations: &RolloutObservations,
    config: &EvalConfig,
    max_ticks: u64,
    deadline: std::time::Instant,
) -> Option<FitnessBreakdown> {
    evaluate(world, observations, config, max_ticks, Some(deadline))
}

fn evaluate(
    world: &explorers_sim::World,
    observations: &RolloutObservations,
    config: &EvalConfig,
    max_ticks: u64,
    deadline: Option<std::time::Instant>,
) -> Option<FitnessBreakdown> {
    let within = || deadline.is_none_or(|d| std::time::Instant::now() < d);
    if !within() {
        return None;
    }
    let RolloutObservations {
        carcass_fraction: carcass_fraction_per_tick,
        producer_share: producer_share_per_tick,
        cluster_snapshots,
        role_snapshots,
        total_births,
        total_deaths,
        born,
        ..
    } = observations;
    let agents = world.agents();
    let ticks_survived = world.tick();

    // Heterotroph guilds (#490): a population read over the second-half role
    // snapshots — sustained size plus recruitment — for both heterotroph roles.
    // Reported observables, aggregated into per-cell seed fractions by the atlas
    // — never an axis, never a fitness term (the authority boundary,
    // genesis-search.md). Read before the gates, because a world gated on its
    // terminal read still ran the settled window the predicate reads, and the
    // gate zeroes coordinates, not observables (#527).
    let guilds = guild::role_guilds_from_samples(role_snapshots, born, max_ticks);

    let zero_breakdown =
        |failure: FailureMode| Some(FitnessBreakdown::gated(failure, ticks_survived, guilds));

    if agents.is_empty() {
        return zero_breakdown(FailureMode::Extinction);
    }

    if is_population_explosion(agents.len(), config.max_population) {
        return zero_breakdown(FailureMode::PopulationExplosion);
    }

    // Turnover reads the counts the rollout took off the log tail tick by
    // tick (`EVALUATOR_EVENT_KINDS`), not the log itself, which the rollout
    // may have dropped.
    let ts = turnover_score(*total_births, *total_deaths, max_ticks);

    if !within() {
        return None;
    }
    let trait_vectors: Vec<_> = agents.iter().map(|a| a.traits).collect();
    let energies: Vec<_> = agents.iter().map(|a| a.energy()).collect();

    let cs = if trait_vectors.len() >= 4 {
        clustering_strength(&trait_vectors)
    } else {
        0.0
    };

    let labels = dbscan(&trait_vectors, config.dbscan_eps, config.dbscan_min_points);
    let tb = trophic_balance_score(&trait_vectors, &labels, &energies);

    let grace_ticks = config.grace_ticks;
    if let Some(failure) = dead_pool_gate(observations, config, sustainable_stock(world.params())) {
        return zero_breakdown(failure);
    }

    if ticks_survived > grace_ticks && trait_vectors.len() >= ROSTER_FLOOR {
        if is_monoculture(&trait_vectors, config.clustering_threshold) {
            return zero_breakdown(FailureMode::Monoculture);
        }
        if is_generalist_dominant(
            &trait_vectors,
            &labels,
            &energies,
            config.generalist_threshold,
            config.generalist_dominance_fraction,
        ) {
            return zero_breakdown(FailureMode::GeneralistDominance);
        }
    }

    // Coexistence (issue #394): the fraction of settled-window sampled ticks
    // (tick in `(T/2, T]`, the window the guild predicate shares) whose living
    // population carries >=2 trait-space DBSCAN clusters. Genesis snapshots
    // raw trait vectors at a coarse interval through the rollout (pure observation,
    // no clustering); the evaluator owns all clustering, running DBSCAN on each
    // settled snapshot here. Each snapshot's tick is stored so the window cutoff
    // stays index/interval-free. Seed-invariant by construction: a pure function of
    // the snapshots and the DBSCAN config, with no `initial_population_size` leak.
    // The deadline is read before each snapshot's DBSCAN; a spent one
    // abandons the whole read (`None` through the collect).
    let cluster_counts_per_snapshot: Vec<usize> = cluster_snapshots
        .iter()
        .filter(|(tick, _)| *tick > max_ticks / 2)
        .map(|(_, traits)| {
            within().then(|| {
                distinct_cluster_count(traits, config.dbscan_eps, config.dbscan_min_points)
            })
        })
        .collect::<Option<_>>()?;

    // Oscillation: the producer↔consumer rhythm read off the per-tick producer-
    // energy-share series the caller sampled (issue #392), over the settled
    // window `(T/2, T]` (genesis-search.md, *One rollout, one settled window*):
    // the atlas maps the settled community, and the bloom-stage rhythm is a
    // colonisation artefact. The series is indexed from tick 1, so the window
    // is the slice from index `T/2` on. Measured over the whole window — a
    // slow ecological cycle needs several periods, not a trailing tail.
    let settled_share = settled_window(producer_share_per_tick, max_ticks);
    let os = if ticks_survived > grace_ticks {
        oscillation_strength(settled_share)
    } else {
        0.0
    };
    let cd = if ticks_survived > grace_ticks {
        coexistence_duration(&cluster_counts_per_snapshot)
    } else {
        0.0
    };

    let fitness = 0.2 * os + 0.2 * cs + 0.2 * cd + 0.2 * ts + 0.2 * tb;

    // Behaviour axis iii: the carcass-locked fraction the lockup gate reads, as
    // the trailing-window mean of the per-tick series the caller sampled (same
    // window). An additive descriptor — read off, never summed into fitness.
    let carcass_locked_fraction =
        trailing_mean(carcass_fraction_per_tick, config.nutrient_lock_window);

    Some(FitnessBreakdown {
        fitness,
        failure: None,
        oscillation_strength: os,
        clustering_strength: cs,
        coexistence_duration: cd,
        turnover_score: ts,
        trophic_balance_score: tb,
        ticks_survived,
        carcass_locked_fraction,
        has_decomposer_guild: guilds.decomposer,
        has_consumer_guild: guilds.consumer,
    })
}

/// The two dead-pool gates read on a rollout's series-so-far: energy death
/// first, then nutrient lockup, each a trailing window read from
/// `grace_ticks` on — energy death against the config's `sustainable_stock`
/// ([`is_free_energy_dead_sustainable`]), lockup against the history since
/// the grace. This is the one definition both the horizon verdict
/// ([`evaluate_from_log`]) and the incremental stop ([`early_stop`]) read, so
/// a rollout stopped where it dies is verdicted exactly as it would be had
/// the same series been read at the horizon (#506). `None` inside the grace.
fn dead_pool_gate(
    observations: &RolloutObservations,
    config: &EvalConfig,
    sustainable_stock: f32,
) -> Option<FailureMode> {
    let ticks_so_far = observations.free_energy.len() as u64;
    if ticks_so_far <= config.grace_ticks {
        return None;
    }
    let grace = config.grace_ticks as usize;
    // Free (non-carcass-locked) energy stock, sampled once per tick by the
    // caller. Energy death is this living-system stock being small against
    // what the resource base can sustain — a stock read against a
    // history-free reference (#508), not the predation flow the old
    // detector summed (issue #302). The grace prefix is dropped so the
    // founder provisioning is never consulted.
    let post_grace = observations.free_energy.get(grace..).unwrap_or(&[]);
    if is_free_energy_dead_sustainable(post_grace, config.energy_death_window, sustainable_stock) {
        return Some(FailureMode::EnergyDeath);
    }
    // Carcass-locked nutrient fraction, sampled once per tick by the caller.
    // Nutrient lockup is the dead pool's share trending high and staying
    // there — nutrient sequestered into carcasses the living decomposers
    // cannot turn over (issue #342). The nutrient-side sibling of energy
    // death, checked after it: a world can photosynthesise fine while its
    // nutrient irreversibly silts up the dead pool. Same grace prefix drop.
    let post_grace_nutrient = observations.carcass_fraction.get(grace..).unwrap_or(&[]);
    if is_nutrient_locked(post_grace_nutrient, config.nutrient_lock_window) {
        return Some(FailureMode::NutrientLockup);
    }
    None
}

/// The rollout's incremental terminal check, asked after every stepped tick
/// has been observed: the failure mode on which to stop the rollout *now*,
/// or `None` to keep stepping (genesis-search.md, *The frontier costs a
/// bloom, the atlas costs the horizon*). Extinction and population explosion
/// read the living count every tick; the two dead-pool gates are read on the
/// series-so-far — exactly the horizon definitions, reference starting at
/// `grace_ticks` — at the lockup-window cadence, so a world that collapses
/// at tick `t` and stays collapsed stops at the first window boundary past
/// `t + window`, tallied to the dead frontier where it died rather than
/// carried to the horizon. Nothing is scored at an early stop.
///
/// The gates as defined are not proven irreversible — a world flagged here
/// might recover by the horizon — which is why the search carries a sampled
/// fraction of early-stopped rollouts to the horizon anyway and surfaces
/// every disagreement (the same falsification interlock the prefilter has).
///
/// `sustainable_stock` is the config's [`sustainable_stock`], computed once
/// per rollout by the caller, the energy-death reference.
pub fn early_stop(
    agent_count: usize,
    observations: &RolloutObservations,
    config: &EvalConfig,
    sustainable_stock: f32,
) -> Option<FailureMode> {
    if is_extinct(agent_count) {
        return Some(FailureMode::Extinction);
    }
    if is_population_explosion(agent_count, config.max_population) {
        return Some(FailureMode::PopulationExplosion);
    }
    let tick = observations.free_energy.len() as u64;
    let cadence = config.nutrient_lock_window.max(1) as u64;
    if !tick.is_multiple_of(cadence) {
        return None;
    }
    dead_pool_gate(observations, config, sustainable_stock)
}

/// The settled window `(T/2, T]` of a per-tick series sampled from tick 1 —
/// the tail of the series from index `T/2` on, empty if the series never got
/// there. The window every behaviour axis with a temporal read shares with
/// the guild predicate (genesis-search.md, *One rollout, one settled window*).
fn settled_window(series: &[f32], max_ticks: u64) -> &[f32] {
    let start = (max_ticks / 2) as usize;
    series.get(start..).unwrap_or(&[])
}

/// Mean of the trailing `window` samples (the whole series if shorter), 0 on an
/// empty series. Matches the trailing window the lockup gate inspects, so the
/// carcass-locked-fraction descriptor reads the same tail the gate does.
fn trailing_mean(series: &[f32], window: usize) -> f32 {
    if series.is_empty() {
        return 0.0;
    }
    let start = series.len().saturating_sub(window);
    let tail = &series[start..];
    tail.iter().sum::<f32>() / tail.len() as f32
}

pub fn is_extinct(agent_count: usize) -> bool {
    agent_count == 0
}

pub fn is_population_explosion(agent_count: usize, ceiling: usize) -> bool {
    agent_count > ceiling
}

pub fn dip_statistic(sorted_data: &[f32]) -> f32 {
    let n = sorted_data.len();
    if n < 4 {
        return 0.0;
    }
    let range = sorted_data[n - 1] - sorted_data[0];
    if range <= 0.0 {
        return 0.0;
    }
    let expected_gap = range / (n - 1) as f32;
    let max_gap = sorted_data
        .windows(2)
        .map(|w| w[1] - w[0])
        .fold(0.0_f32, f32::max);
    let gap_ratio = max_gap / expected_gap;
    // gap_ratio >= 1 always; for uniform data it's ~1; for bimodal data it's >> 1
    // Map to [0, 1): 1 - 1/gap_ratio gives 0 for uniform, approaches 1 for large gaps
    1.0 - 1.0 / gap_ratio
}

pub fn clustering_strength(trait_vectors: &[explorers_sim::TraitVector]) -> f32 {
    let n = trait_vectors.len();
    if n < 4 {
        return 0.0;
    }
    let mut distances = Vec::with_capacity(n * (n - 1) / 2);
    for i in 0..n {
        for j in (i + 1)..n {
            distances.push(trait_vectors[i].distance(&trait_vectors[j]));
        }
    }
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let m = distances.len();
    let range = distances[m - 1] - distances[0];
    if range <= 0.0 {
        return 0.0;
    }

    let num_bins = 20;
    let bin_width = range / num_bins as f32;
    let mut bins = vec![0usize; num_bins];
    for &d in &distances {
        let bin = ((d - distances[0]) / bin_width).floor() as usize;
        bins[bin.min(num_bins - 1)] += 1;
    }

    let mut max_valley_depth = 0.0_f32;
    for i in 1..num_bins - 1 {
        let left_max = bins[..i].iter().copied().max().unwrap_or(0) as f32;
        let right_max = bins[i + 1..].iter().copied().max().unwrap_or(0) as f32;
        let valley_floor = bins[i] as f32;
        let peak_height = left_max.min(right_max);
        if peak_height > 0.0 {
            let depth = (peak_height - valley_floor) / peak_height;
            max_valley_depth = max_valley_depth.max(depth);
        }
    }

    max_valley_depth
}

pub fn is_monoculture(trait_vectors: &[explorers_sim::TraitVector], threshold: f32) -> bool {
    clustering_strength(trait_vectors) < threshold
}

/// Trophic coordinates: (autotrophy_fraction, heterotrophy_fraction).
/// With unified heterotrophy, the trophic position is a 2D coordinate
/// rather than a 3D barycentric coordinate.
pub fn trophic_coordinates(traits: &explorers_sim::TraitVector) -> (f32, f32) {
    let sum = traits.photosynthetic_absorption + traits.heterotrophy;
    if sum <= 0.0 {
        return (0.5, 0.5);
    }
    (
        traits.photosynthetic_absorption / sum,
        traits.heterotrophy / sum,
    )
}

pub fn is_generalist_dominant(
    trait_vectors: &[explorers_sim::TraitVector],
    labels: &[Option<usize>],
    energies: &[f32],
    generalist_threshold: f32,
    dominance_fraction: f32,
) -> bool {
    let total_energy: f32 = energies.iter().sum();
    if total_energy <= 0.0 {
        return false;
    }

    let max_cluster = labels.iter().filter_map(|l| *l).max();
    let Some(max_cluster) = max_cluster else {
        return false;
    };

    let mut generalist_energy = 0.0_f32;
    for cluster_id in 0..=max_cluster {
        let members: Vec<usize> = labels
            .iter()
            .enumerate()
            .filter(|(_, l)| **l == Some(cluster_id))
            .map(|(i, _)| i)
            .collect();
        if members.is_empty() {
            continue;
        }
        let mut avg_photo = 0.0_f32;
        let mut avg_hetero = 0.0_f32;
        for &i in &members {
            let (p, h) = trophic_coordinates(&trait_vectors[i]);
            avg_photo += p;
            avg_hetero += h;
        }
        let n = members.len() as f32;
        avg_photo /= n;
        avg_hetero /= n;

        // A generalist has significant investment in both autotrophy and heterotrophy
        let is_generalist = avg_photo > generalist_threshold && avg_hetero > generalist_threshold;

        if is_generalist {
            for &i in &members {
                generalist_energy += energies[i];
            }
        }
    }

    generalist_energy / total_energy > dominance_fraction
}

pub fn autocorrelation(series: &[f32], lag: usize) -> f32 {
    let n = series.len();
    if n <= lag || n < 2 {
        return 0.0;
    }
    let mean: f32 = series.iter().sum::<f32>() / n as f32;
    let variance: f32 = series.iter().map(|&x| (x - mean) * (x - mean)).sum::<f32>();
    if variance <= 0.0 {
        return 0.0;
    }
    let covariance: f32 = (0..n - lag)
        .map(|i| (series[i] - mean) * (series[i + lag] - mean))
        .sum();
    covariance / variance
}

/// Anti-correlation depth of the linearly-detrended producer-energy-share series
/// — the seed-invariant, search-smooth readout of the producer↔consumer rhythm
/// (the Hopf bifurcation's frozen-fixed-point ↔ limit-cycle axis, issue #392).
///
/// The signal is `World::producer_energy_share` sampled per tick: a scale-
/// invariant ratio, so a searched `initial_population_size` cannot leak in. We
/// linearly detrend (least-squares residual against tick index) to kill the
/// monotonic colonization transient, then report the deepest anti-correlation
/// over lags `[LAG_MIN, n/2]` as `clamp(-min_ac, 0, 1)`.
///
/// This is continuous everywhere it matters — the output is the *value* of a min
/// over continuous autocorrelations, never the *lag* — so it gives CMA-MAE a
/// gradient through the Hopf onset: a sustained cycle reads high, damped ringing
/// on the stable side reads small-but-positive, and a frozen fixed point, a
/// monotonic ramp (detrended away) or white noise (uncorrelated, not anti-)
/// all read ~0.
pub fn oscillation_strength(producer_share: &[f32]) -> f32 {
    const LAG_MIN: usize = 2;
    const MIN_LEN: usize = 8;
    // Absolute swing below which the series is not a regime signal: a producer
    // share that moves less than 1% over the whole window is a frozen fixed point
    // with noise, not an oscillation, no matter what structure that noise carries
    // (issue #403). Sits at the top of the "<~0.5–1% is not a signal" band, with
    // comfortable margin over the example10 seed-4 witness (~0.3% swing).
    const MIN_SWING: f32 = 0.01;

    let n = producer_share.len();
    if n < MIN_LEN {
        return 0.0;
    }
    let max_lag = n / 2;
    if max_lag < LAG_MIN {
        return 0.0;
    }
    // Absolute near-flatness guard: the relative guard below keys on the
    // residual-to-raw variance ratio, which a near-constant series with a tiny
    // monotone creep slips past — the detrend removes the creep, leaving noise
    // whose variance is not small *relative* to the (also tiny) raw variance, and
    // its autocorrelation then reports a spurious deep anti-correlation (#403). A
    // sub-MIN_SWING peak-to-peak is not a regime signal regardless of that ratio.
    let (min_x, max_x) = producer_share
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &x| {
            (lo.min(x), hi.max(x))
        });
    if max_x - min_x < MIN_SWING {
        return 0.0;
    }
    let residual = linear_detrend(producer_share);
    // Flat-after-detrend guard: a pure constant or pure linear ramp detrends to a
    // residual that is float-rounding noise, not signal. Autocorrelating that
    // noise yields a meaningless deep "anti-correlation". A monotonic colonization
    // curve must read ~0 (the whole point of detrending), so if the residual
    // carries negligible energy relative to the raw series we report 0 directly.
    let nf = n as f32;
    let raw_mean = producer_share.iter().sum::<f32>() / nf;
    let raw_var = producer_share
        .iter()
        .map(|&x| (x - raw_mean) * (x - raw_mean))
        .sum::<f32>();
    let residual_var = residual.iter().map(|&x| x * x).sum::<f32>();
    // residual is mean-zero by construction, so its sum-of-squares is its variance.
    if raw_var <= 0.0 || residual_var <= 1e-6 * raw_var {
        return 0.0;
    }
    let mut min_ac = f32::INFINITY;
    for lag in LAG_MIN..=max_lag {
        let ac = autocorrelation(&residual, lag);
        if ac < min_ac {
            min_ac = ac;
        }
    }
    (-min_ac).clamp(0.0, 1.0)
}

/// Least-squares linear detrend: the residual of `series` against `x = 0..n`,
/// i.e. with its best-fit straight line subtracted. A monotonic ramp's residual
/// is ~0, so the oscillation descriptor rejects steady growth; a cycle's residual
/// keeps its oscillation.
fn linear_detrend(series: &[f32]) -> Vec<f32> {
    let n = series.len();
    if n == 0 {
        return Vec::new();
    }
    let nf = n as f32;
    let mean_x = (n - 1) as f32 / 2.0;
    let mean_y = series.iter().sum::<f32>() / nf;
    let mut sxx = 0.0_f32;
    let mut sxy = 0.0_f32;
    for (i, &y) in series.iter().enumerate() {
        let dx = i as f32 - mean_x;
        sxx += dx * dx;
        sxy += dx * (y - mean_y);
    }
    let slope = if sxx > 0.0 { sxy / sxx } else { 0.0 };
    let intercept = mean_y - slope * mean_x;
    series
        .iter()
        .enumerate()
        .map(|(i, &y)| y - (slope * i as f32 + intercept))
        .collect()
}

pub fn has_demographic_turnover(total_births: usize, total_deaths: usize) -> bool {
    total_births > 0 && total_deaths > 0
}

pub fn turnover_score(total_births: usize, total_deaths: usize, max_ticks: u64) -> f32 {
    if max_ticks == 0 {
        return 0.0;
    }
    let min_events = total_births.min(total_deaths) as f32;
    (min_events / max_ticks as f32).clamp(0.0, 1.0)
}

pub fn has_trophic_pyramid(
    trait_vectors: &[explorers_sim::TraitVector],
    labels: &[Option<usize>],
    energies: &[f32],
) -> bool {
    let max_cluster = labels.iter().filter_map(|l| *l).max();
    let Some(max_cluster) = max_cluster else {
        return false;
    };

    let mut producer_energy = 0.0_f32;
    let mut consumer_energy = 0.0_f32;

    for cluster_id in 0..=max_cluster {
        let members: Vec<usize> = labels
            .iter()
            .enumerate()
            .filter(|(_, l)| **l == Some(cluster_id))
            .map(|(i, _)| i)
            .collect();
        if members.is_empty() {
            continue;
        }

        let mut avg_photo = 0.0_f32;
        let mut avg_hetero = 0.0_f32;
        for &i in &members {
            let (p, h) = trophic_coordinates(&trait_vectors[i]);
            avg_photo += p;
            avg_hetero += h;
        }
        let n = members.len() as f32;
        avg_photo /= n;
        avg_hetero /= n;

        let cluster_energy: f32 = members.iter().map(|&i| energies[i]).sum();
        if avg_photo > avg_hetero {
            producer_energy += cluster_energy;
        } else {
            consumer_energy += cluster_energy;
        }
    }

    producer_energy > consumer_energy
}

/// Producer share of living energy: `producer_energy / (producer + consumer)`,
/// bucketing each cluster as producer or consumer by whether its mean photo-
/// synthetic coordinate exceeds its mean heterotrophic one. It rewards the
/// pyramid base — energy concentrated in producers — per the "Trophic structure"
/// expected property.
///
/// It is deliberately **decomposer-blind**: decomposers are heterotrophs, so they
/// fall in `consumer_energy`, and this score does not — and cannot — reward a
/// decomposer guild distinctly. Decomposer-ness is not in the trait vector to
/// score (see `docs/system-design/trait-space.md`, "Decomposer is a behavioural
/// role, not a heritable trait"), so the detrital pathway is held to account
/// negatively by the `EnergyDeath` failure gate (a world where it fails locks
/// matter in carcasses and scores zero fitness), never by this term.
pub fn trophic_balance_score(
    trait_vectors: &[explorers_sim::TraitVector],
    labels: &[Option<usize>],
    energies: &[f32],
) -> f32 {
    let max_cluster = labels.iter().filter_map(|l| *l).max();
    let Some(max_cluster) = max_cluster else {
        return 0.0;
    };

    let mut producer_energy = 0.0_f32;
    let mut consumer_energy = 0.0_f32;

    for cluster_id in 0..=max_cluster {
        let members: Vec<usize> = labels
            .iter()
            .enumerate()
            .filter(|(_, l)| **l == Some(cluster_id))
            .map(|(i, _)| i)
            .collect();
        if members.is_empty() {
            continue;
        }

        let mut avg_photo = 0.0_f32;
        let mut avg_hetero = 0.0_f32;
        for &i in &members {
            let (p, h) = trophic_coordinates(&trait_vectors[i]);
            avg_photo += p;
            avg_hetero += h;
        }
        let n = members.len() as f32;
        avg_photo /= n;
        avg_hetero /= n;

        let cluster_energy: f32 = members.iter().map(|&i| energies[i]).sum();
        if avg_photo > avg_hetero {
            producer_energy += cluster_energy;
        } else {
            consumer_energy += cluster_energy;
        }
    }

    let total = producer_energy + consumer_energy;
    if total <= 0.0 {
        return 0.0;
    }
    producer_energy / total
}

pub fn coexistence_duration(cluster_counts_per_tick: &[usize]) -> f32 {
    if cluster_counts_per_tick.is_empty() {
        return 0.0;
    }
    let coexisting = cluster_counts_per_tick.iter().filter(|&&c| c >= 2).count();
    coexisting as f32 / cluster_counts_per_tick.len() as f32
}

/// Number of distinct non-noise DBSCAN clusters in a trait-space snapshot — the
/// living population's co-present niche count (CONTEXT.md *Cluster labelling*).
/// Reuses the evaluator's existing `dbscan`; counts distinct `Some(_)` labels and
/// ignores noise (`None`). This is `coexistence_duration`'s per-snapshot input
/// (issue #394): the community-level "how many niches are occupied right now",
/// not the dip/valley-depth existence statistic that is `clustering_strength`.
pub fn distinct_cluster_count(
    trait_vectors: &[explorers_sim::TraitVector],
    eps: f32,
    min_points: usize,
) -> usize {
    let labels = dbscan(trait_vectors, eps, min_points);
    labels
        .iter()
        .filter_map(|l| *l)
        .collect::<std::collections::HashSet<_>>()
        .len()
}

pub fn dbscan(
    trait_vectors: &[explorers_sim::TraitVector],
    eps: f32,
    min_points: usize,
) -> Vec<Option<usize>> {
    let n = trait_vectors.len();
    let mut labels: Vec<Option<usize>> = vec![None; n];
    let mut visited = vec![false; n];
    let mut cluster_id = 0;

    for i in 0..n {
        if visited[i] {
            continue;
        }
        visited[i] = true;
        let neighbors = region_query(trait_vectors, i, eps);
        if neighbors.len() < min_points {
            continue;
        }
        labels[i] = Some(cluster_id);
        let mut queue = neighbors;
        let mut qi = 0;
        while qi < queue.len() {
            let j = queue[qi];
            qi += 1;
            if !visited[j] {
                visited[j] = true;
                let j_neighbors = region_query(trait_vectors, j, eps);
                if j_neighbors.len() >= min_points {
                    for &k in &j_neighbors {
                        if !queue.contains(&k) {
                            queue.push(k);
                        }
                    }
                }
            }
            if labels[j].is_none() {
                labels[j] = Some(cluster_id);
            }
        }
        cluster_id += 1;
    }

    labels
}

fn region_query(trait_vectors: &[explorers_sim::TraitVector], idx: usize, eps: f32) -> Vec<usize> {
    let mut neighbors = Vec::new();
    for (j, tv) in trait_vectors.iter().enumerate() {
        if trait_vectors[idx].distance(tv) <= eps {
            neighbors.push(j);
        }
    }
    neighbors
}

/// The **history-peak** energy-death read — the stand-in the evaluator used
/// until the history-free read ([`is_free_energy_dead_sustainable`]) cleared
/// the zero-false-positive check (#508; `docs/research/508-energy-death-sustainable.md`).
/// No longer wired into any gate; kept as the comparison arm of that check
/// (`energy_death_check`), so the check stays re-runnable when the stepper
/// changes.
///
/// `free_energy_per_tick` is the living-system energy stock sampled once per
/// tick. Flagged when, over the trailing `window`, the stock has collapsed to
/// a small fraction of its earlier peak and does not recover. Its defect is
/// the reference: a world that cedes its bloom-stage stock to the heterotroph
/// niche reads as dead for the ecology working, and a world that never
/// recovers its founder stock never acquires a reference and passes as live.
pub fn is_free_energy_dead(free_energy_per_tick: &[f32], window: usize) -> bool {
    if free_energy_per_tick.len() < window || window == 0 {
        return false;
    }
    let split = free_energy_per_tick.len() - window;
    let peak = free_energy_per_tick[..split]
        .iter()
        .copied()
        .fold(0.0_f32, f32::max);
    if peak <= 0.0 {
        return false;
    }
    let window_peak = free_energy_per_tick[split..]
        .iter()
        .copied()
        .fold(0.0_f32, f32::max);
    // Collapsed and non-recovering: the best the trailing window achieves is a
    // small fraction of the free energy the living system previously held.
    const COLLAPSE_FRACTION: f32 = 0.1;
    window_peak < peak * COLLAPSE_FRACTION
}

/// The fraction of the config's [`sustainable_stock`] below which the
/// history-free energy-death read ([`is_free_energy_dead_sustainable`]) calls
/// the living stock dead: an order of magnitude under the reference, the
/// same tenth the history-peak stand-in reads (`COLLAPSE_FRACTION`).
pub const SUSTAINABLE_FRACTION: f32 = 0.1;

/// The world's **sustainable stock** — the energy form of viability's solar
/// ceiling (viability.md, *Bound — sustained population*), a property of the
/// config alone.
///
/// Derivation from the committed rules. Solar flux is the sole external
/// energy source and is capped per tick at `P_max = F·m²`, one flux per
/// light-competition tile, `m = ⌊√2·L/r⌋ + 1`; every survivor is charged at
/// least the base metabolic rate `B` every tick (flow 8). So the long-run
/// mean survivor count is bounded by `N̄_max = F·m²/B = π_F·m²`. A survivor of
/// a tick paid `≥ B` from its own stock and ended the tick with reserve still
/// positive, so a body at the ceiling holds at least one tick of base
/// metabolism, `ε = B·τ` — the energy unit of the dimensionless groups. The
/// stock of a population at the ceiling is therefore scaled by body
/// maintenance as `N̄_max · ε = F·m²·τ`: one tick of the solar cap, in energy
/// (`τ = 1`). Independent of `B` — a cheaper body means more bodies, each
/// holding less — and of every searched coefficient but `F`, `L`, `r`.
/// Requires `r > 0`, which the search box guarantees.
pub fn sustainable_stock(params: &explorers_sim::WorldParameters) -> f32 {
    let l = f64::from(params.world_extent);
    let r = f64::from(params.light_competition_radius);
    let f = f64::from(params.solar_flux_magnitude);
    let m = (std::f64::consts::SQRT_2 * l / r).floor() + 1.0;
    (f * m * m) as f32
}

/// Energy death read against the sustainable stock (expected-properties.md,
/// *Energy death — How it is read*): the trailing `window` peak of the living
/// free-energy stock sits below [`SUSTAINABLE_FRACTION`] of the config's
/// [`sustainable_stock`]. `post_grace` is the stock series from the grace on
/// (the founder provisioning is never consulted); the reference is a property
/// of the config, not of the run's own history, so the read says the same
/// thing at every horizon and for every bloom shape. Not flagged while the
/// series is shorter than the window.
pub fn is_free_energy_dead_sustainable(
    post_grace: &[f32],
    window: usize,
    sustainable_stock: f32,
) -> bool {
    if post_grace.len() < window || window == 0 {
        return false;
    }
    let window_peak = post_grace[post_grace.len() - window..]
        .iter()
        .copied()
        .fold(0.0_f32, f32::max);
    window_peak < sustainable_stock * SUSTAINABLE_FRACTION
}

/// Whether nutrient is locked irreversibly in the dead pool — the pathology a
/// world without viable decomposers exhibits (world-rules.md: "a world without
/// decomposers accumulates resources in the dead pool until the living system
/// starves"). `carcass_fraction_per_tick` is the dead pool's share of the
/// conserved system nutrient, sampled once per tick by the caller.
///
/// Lockup is the carcass-locked fraction sitting high across the whole trailing
/// window *and* not receding: even the window's low point stays above the lock
/// threshold (sustained sequestration, not a transient carcass spike) and is no
/// lower than the pre-window low (still climbing or stuck, not being turned
/// over). A world whose decomposers keep up sees the fraction drain back down,
/// driving the window low below threshold; a world recovering from a glut sees
/// the fraction recede below its earlier level. Neither is flagged.
pub fn is_nutrient_locked(carcass_fraction_per_tick: &[f32], window: usize) -> bool {
    if carcass_fraction_per_tick.len() < window || window == 0 {
        return false;
    }
    let split = carcass_fraction_per_tick.len() - window;
    let pre_low = carcass_fraction_per_tick[..split]
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    let window_low = carcass_fraction_per_tick[split..]
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    // Sustained and non-receding: the dead pool's smallest share over the whole
    // trailing window still exceeds the lock threshold and has not fallen below
    // its pre-window low.
    const LOCK_FRACTION: f32 = 0.4;
    window_low >= LOCK_FRACTION && window_low >= pre_low
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_world_params() -> explorers_sim::WorldParameters {
        explorers_sim::WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.01,
            sensing_range_coefficient: 10.0,
            base_trophic_efficiency: 0.5,
            trophic_distance_decay: 0.0,
            reproduction_efficiency: 0.9,
            movement_cost_coefficient: 0.0,
            reproduction_energy_threshold: 5.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.1,
            mutation_magnitude: 0.1,
            contact_range_coefficient: 10.0,
            world_extent: 20.0,
            initial_population_size: 30,
            light_competition_radius: 1000.0,
            photo_maintenance_cost: 0.0,
            heterotrophy_maintenance_cost: 0.0,
            initial_nutrient_pool: 0.0,
            growth_efficiency: 0.5,
            wear_rate: 0.0,
            wear_degradation_steepness: 0.0,
            somatic_maintenance_cost_coefficient: 0.0,
            use_wear_rate: 0.0,
            structure_maintenance_coefficient: 0.0,
            repair_decay: 0.0,
            base_nutrient_ratio: 0.1,
            specification_nutrient_coefficient: 0.2,
            reproductive_compatibility_distance: 2.0,
            mobility_maintenance_cost: 0.0,
            maintenance_cost_exponent: 1.0,
            nutrient_grid_cell_size: 10.0,
            growth_retention_multiplier: 2.0,
            reserve_mobilisation_rate: 1.0,
            offspring_structure_fraction: 0.2,
            asexual_propensity_maintenance_cost: 0.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            dispersal_reach_coefficient: 0.0,
            body_reach_coefficient: 0.0,
            network_connection_cap: 0,
            network_creation_cost: 0.0,
            network_maintenance_cost: 0.0,
            network_redistribution_rate: 0.0,
            network_transfer_efficiency: 0.0,
        }
    }

    /// Step a world to `max_ticks` (terminating early on extinction), sampling
    /// the free-energy stock each tick exactly as the real callers do.
    /// Build a `RolloutObservations` from the per-tick series a test wants to
    /// exercise — a thin constructor so each test still reads as "this free-energy
    /// trace, this carcass trace, …" while the evaluator takes the one bundle.
    fn obs(
        free_energy: &[f32],
        carcass_fraction: &[f32],
        producer_share: &[f32],
        cluster_snapshots: &[(u64, Vec<explorers_sim::TraitVector>)],
    ) -> RolloutObservations {
        RolloutObservations {
            free_energy: free_energy.to_vec(),
            carcass_fraction: carcass_fraction.to_vec(),
            producer_share: producer_share.to_vec(),
            cluster_snapshots: cluster_snapshots.to_vec(),
            ..RolloutObservations::default()
        }
    }

    fn run_collecting_free_energy(world: &mut explorers_sim::World, max_ticks: u64) -> Vec<f32> {
        let mut free = Vec::with_capacity(max_ticks as usize);
        for _ in 0..max_ticks {
            world.step();
            free.push(world.free_energy());
            if world.agents().is_empty() {
                break;
            }
        }
        free
    }

    /// Step a world to `max_ticks`, sampling both the free-energy stock and the
    /// producer-energy share each tick, exactly as the real callers do. Returns
    /// `(free_energy_per_tick, producer_share_per_tick)`.
    fn run_collecting_free_energy_and_share(
        world: &mut explorers_sim::World,
        max_ticks: u64,
    ) -> (Vec<f32>, Vec<f32>) {
        let mut free = Vec::with_capacity(max_ticks as usize);
        let mut share = Vec::with_capacity(max_ticks as usize);
        for _ in 0..max_ticks {
            world.step();
            free.push(world.free_energy());
            share.push(world.producer_energy_share());
            if world.agents().is_empty() {
                break;
            }
        }
        (free, share)
    }

    /// A world that survives to a 200-tick horizon on every seed tried: the
    /// base fixture packs 30 agents within one contact range of each other on
    /// a 20-unit world and consumes itself in a single step, which is fine for
    /// the final-state and gate tests but leaves nothing to read a settled
    /// window off. Widening the world keeps the founders apart.
    fn live_world_params() -> explorers_sim::WorldParameters {
        explorers_sim::WorldParameters {
            contact_range_coefficient: 2.0,
            world_extent: 50.0,
            ..test_world_params()
        }
    }

    fn test_distribution() -> explorers_sim::InitialDistribution {
        explorers_sim::InitialDistribution {
            mean_traits: explorers_sim::TraitVector {
                photosynthetic_absorption: 0.8,
                heterotrophy: 0.3,
                mobility: 0.3,
                kappa: 0.7,
                fecundity: 0.0,
                asexual_propensity: 0.0,
                dispersal: 0.0,
            },
            trait_covariance: 0.5,
            initial_cluster_count: 2,
            initial_energy_per_agent: 50.0,
        }
    }

    #[test]
    fn evaluate_from_log_turnover_matches_event_counts() {
        let params = test_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 50,
            ..EvalConfig::default()
        };
        let max_ticks = 50;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let mut observations = RolloutObservations::with_capacity(max_ticks as usize);
        for _ in 0..max_ticks {
            world.step();
            observations.observe(&world, config.coexistence_sample_interval);
            if world.agents().is_empty() {
                break;
            }
        }
        let result = evaluate_from_log(&world, &observations, &config, max_ticks);
        let born_count = world
            .event_log()
            .by_kind(&explorers_sim::event::EventKind::Reproduced)
            .len();
        let died_count = world
            .event_log()
            .by_kind(&explorers_sim::event::EventKind::Died)
            .len();
        let expected_ts = turnover_score(born_count, died_count, max_ticks);
        // With reproduction not yet implemented, births may be zero.
        // Turnover score computation should still be consistent.
        assert_eq!(result.turnover_score, expected_ts);
    }

    #[test]
    fn evaluate_from_log_flags_energy_death_only_when_free_energy_collapses() {
        // A surviving world plus a free-energy series the evaluator inspects.
        // Same world, two trajectories: a collapse into carcasses is energy
        // death; a sustained living stock is not. Confirms the detector reads
        // the free-energy stock trend, not predation flow (issue #302).
        let params = test_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 0,
            energy_death_window: 5,
            ..EvalConfig::default()
        };
        let max_ticks = 20;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let _ = run_collecting_free_energy(&mut world, max_ticks);
        if world.agents().is_empty() {
            return; // need a surviving world to reach the energy-death branch
        }
        let n = world.tick() as usize;

        let collapsing: Vec<f32> = (0..n)
            .map(|t| if t < n / 2 { 100.0 } else { 1.0 })
            .collect();
        let dead = evaluate_from_log(&world, &obs(&collapsing, &[], &[], &[]), &config, max_ticks);
        assert_eq!(
            dead.failure,
            Some(FailureMode::EnergyDeath),
            "free energy collapsing into carcasses is energy death"
        );

        let sustained: Vec<f32> = vec![100.0; n];
        let alive = evaluate_from_log(&world, &obs(&sustained, &[], &[], &[]), &config, max_ticks);
        assert_ne!(
            alive.failure,
            Some(FailureMode::EnergyDeath),
            "a sustained free-energy stock is not energy death"
        );
    }

    #[test]
    fn energy_death_verdict_is_history_free_whatever_the_horizon_or_grace() {
        // The reference is the config's sustainable stock, not the run's own
        // history (#508): the same free-energy series — a provisioned founder
        // stock two hundred times the steady living stock for the first 20
        // ticks, then that steady stock — reads alive at two horizons and
        // under either grace, because the bloom-scale peak it once held is
        // not a reference anything collapses against. (Under the old
        // history-peak read the no-grace verdict was energy death.) Held at
        // the fixture's sustainable stock, the steady stock is at the ceiling.
        let founder_ticks = 20usize;
        let stock = sustainable_stock(&live_world_params());
        let series = |n: usize| -> Vec<f32> {
            (0..n)
                .map(|t| {
                    if t < founder_ticks {
                        200.0 * stock
                    } else {
                        stock
                    }
                })
                .collect()
        };
        let verdict = |max_ticks: u64, grace_ticks: u64| {
            let config = EvalConfig {
                grace_ticks,
                energy_death_window: 10,
                ..EvalConfig::default()
            };
            let mut world = explorers_sim::World::new(live_world_params(), test_distribution(), 42);
            let _ = run_collecting_free_energy(&mut world, max_ticks);
            assert!(
                !world.agents().is_empty() && world.tick() == max_ticks,
                "fixture world must survive to the horizon"
            );
            let n = max_ticks as usize;
            evaluate_from_log(&world, &obs(&series(n), &[], &[], &[]), &config, max_ticks).failure
        };

        for horizon in [60u64, 120] {
            for grace in [founder_ticks as u64, 0] {
                assert_ne!(
                    verdict(horizon, grace),
                    Some(FailureMode::EnergyDeath),
                    "T = {horizon}, grace {grace}: a steady stock at the ceiling is alive"
                );
            }
        }
    }

    /// Feed a synthetic per-tick series into a rollout's observations one tick
    /// at a time, asking the incremental early-stop check after each, and
    /// report the first `(failure, tick)` it fires on — the trajectory a
    /// rollout would actually take under the gates (#506). The other series
    /// are held healthy so only the one under test can fire.
    /// The synthetic series read against a sustainable stock of
    /// `SYNTHETIC_STOCK`, so a stock of 100 is at the reference and 1 is 1 %.
    const SYNTHETIC_STOCK: f32 = 100.0;

    fn first_early_stop(
        free_energy: &[f32],
        carcass_fraction: &[f32],
        config: &EvalConfig,
    ) -> Option<(FailureMode, u64)> {
        let mut observations = RolloutObservations::default();
        for (fe, cf) in free_energy.iter().zip(carcass_fraction) {
            observations.free_energy.push(*fe);
            observations.carcass_fraction.push(*cf);
            observations.producer_share.push(1.0);
            let tick = observations.free_energy.len() as u64;
            if let Some(failure) = early_stop(30, &observations, config, SYNTHETIC_STOCK) {
                return Some((failure, tick));
            }
        }
        None
    }

    #[test]
    fn early_stop_fires_energy_death_one_window_after_a_sustained_collapse() {
        // A living stock that holds 100 through tick 300 then collapses to 1
        // and stays there: the trailing window is entirely post-collapse from
        // tick 350 on, and the gate — evaluated on the series-so-far at the
        // window cadence — stops the rollout there, not at the horizon.
        let config = EvalConfig {
            grace_ticks: 260,
            energy_death_window: 50,
            nutrient_lock_window: 50,
            ..EvalConfig::default()
        };
        let stock: Vec<f32> = (1..=2000)
            .map(|t| if t <= 300 { 100.0 } else { 1.0 })
            .collect();
        let carcass = vec![0.1; 2000];
        assert_eq!(
            first_early_stop(&stock, &carcass, &config),
            Some((FailureMode::EnergyDeath, 350))
        );
    }

    #[test]
    fn early_stop_fires_nutrient_lockup_one_window_after_the_dead_pool_locks() {
        // The lockup analogue: a healthy stock, but a dead-pool share that
        // sits at 0.1 through tick 300 then climbs to 0.9 and stays — the
        // window low first clears the lock threshold and the pre-window low
        // at tick 350, and the rollout stops there with `NutrientLockup`.
        let config = EvalConfig {
            grace_ticks: 260,
            energy_death_window: 50,
            nutrient_lock_window: 50,
            ..EvalConfig::default()
        };
        let stock = vec![100.0; 2000];
        let carcass: Vec<f32> = (1..=2000)
            .map(|t| if t <= 300 { 0.1 } else { 0.9 })
            .collect();
        assert_eq!(
            first_early_stop(&stock, &carcass, &config),
            Some((FailureMode::NutrientLockup, 350))
        );
    }

    #[test]
    fn early_stop_holds_off_inside_the_grace_and_between_window_boundaries() {
        // A founder provisioning at 1 % of the sustainable stock is never
        // consulted: the gate is silent through the grace, and fires only
        // once a full post-grace window sits below the reference — the first
        // window boundary past `grace + window`, tick 350 — so a world whose
        // stock climbs to the ceiling by tick 300 is never stopped, and a
        // world that stays at the founder scale is (the case the history-peak
        // read passed as live for want of a reference). A collapse at tick
        // 270 (inside the first post-grace window) is likewise read only at
        // tick 350, never at tick 320 between boundaries.
        let config = EvalConfig {
            grace_ticks: 260,
            energy_death_window: 50,
            nutrient_lock_window: 50,
            ..EvalConfig::default()
        };
        let carcass = vec![0.1; 2000];
        let climbs: Vec<f32> = (1..=2000)
            .map(|t| if t <= 300 { 1.0 } else { 100.0 })
            .collect();
        assert_eq!(first_early_stop(&climbs, &carcass, &config), None);
        let founder_scale = vec![1.0; 2000];
        assert_eq!(
            first_early_stop(&founder_scale, &carcass, &config),
            Some((FailureMode::EnergyDeath, 350))
        );

        let stock: Vec<f32> = (1..=2000)
            .map(|t| if t <= 270 { 100.0 } else { 1.0 })
            .collect();
        assert_eq!(
            first_early_stop(&stock, &carcass, &config),
            Some((FailureMode::EnergyDeath, 350))
        );
    }

    #[test]
    fn a_collapse_the_world_recovers_from_stops_the_rollout_yet_reads_alive_at_the_horizon() {
        // The gate as defined is not proven irreversible: a stock that
        // collapses at tick 6 and recovers at tick 13 trips the incremental
        // check at tick 12 (collapse + window, at the window cadence), but the
        // same series carried to T reads alive — the window at T holds the
        // recovered stock. This is exactly the case the search's carry-to-T
        // cross-check counts as a disagreement (#506).
        let params = test_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 0,
            energy_death_window: 4,
            nutrient_lock_window: 4,
            ..EvalConfig::default()
        };
        let max_ticks = 20;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let _ = run_collecting_free_energy(&mut world, max_ticks);
        if world.agents().is_empty() {
            return; // need a surviving world to reach the gate branch
        }
        let n = world.tick() as usize;
        let recovering: Vec<f32> = (1..=n)
            .map(|t| if (7..=12).contains(&t) { 1.0 } else { 100.0 })
            .collect();
        let carcass = vec![0.1; n];
        assert_eq!(
            first_early_stop(&recovering, &carcass, &config),
            Some((FailureMode::EnergyDeath, 12))
        );
        let at_horizon = evaluate_from_log(
            &world,
            &obs(&recovering, &carcass, &[], &[]),
            &config,
            max_ticks,
        );
        assert_ne!(at_horizon.failure, Some(FailureMode::EnergyDeath));
    }

    #[test]
    fn evaluate_from_log_reports_carcass_locked_fraction_as_trailing_window_mean() {
        // Genesis behaviour axis iii (genesis-search.md): the breakdown must
        // carry the carcass-locked fraction the lockup gate reads — the
        // trailing-window mean of the per-tick carcass-fraction series — as an
        // additive read-only descriptor, never folded into fitness. A surviving
        // world with a healthy free-energy stock and a low, turned-over dead pool
        // reports a low (but recorded) carcass fraction; raising the late dead-pool
        // share raises the reported descriptor, monotonically.
        let params = test_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 0,
            nutrient_lock_window: 4,
            ..EvalConfig::default()
        };
        let max_ticks = 20;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let _ = run_collecting_free_energy(&mut world, max_ticks);
        if world.agents().is_empty() {
            return; // need a surviving world to reach the descriptor branch
        }
        let n = world.tick() as usize;
        let healthy_energy: Vec<f32> = vec![100.0; n];

        // A flat low carcass series: the trailing-window mean is that level, and
        // it is not high enough (or non-receding) to gate as lockup.
        let low: Vec<f32> = vec![0.1; n];
        let low_bd = evaluate_from_log(
            &world,
            &obs(&healthy_energy, &low, &[], &[]),
            &config,
            max_ticks,
        );
        assert_ne!(low_bd.failure, Some(FailureMode::NutrientLockup));
        assert!(
            (low_bd.carcass_locked_fraction - 0.1).abs() < 1e-5,
            "carcass_locked_fraction should be the trailing-window mean (0.1), got {}",
            low_bd.carcass_locked_fraction
        );
        // It must not be summed into fitness (the descriptor is read-only).
        let expected_fitness = 0.2
            * (low_bd.oscillation_strength
                + low_bd.clustering_strength
                + low_bd.coexistence_duration
                + low_bd.turnover_score
                + low_bd.trophic_balance_score);
        assert!((low_bd.fitness - expected_fitness).abs() < 1e-5);

        // A flat higher carcass series reports a higher fraction (monotone read).
        let high: Vec<f32> = vec![0.3; n];
        let high_bd = evaluate_from_log(
            &world,
            &obs(&healthy_energy, &high, &[], &[]),
            &config,
            max_ticks,
        );
        assert!(
            high_bd.carcass_locked_fraction > low_bd.carcass_locked_fraction,
            "a higher dead-pool share should report a higher carcass fraction"
        );
    }

    #[test]
    fn evaluate_from_log_flags_nutrient_lockup_when_dead_pool_share_stays_high() {
        // A surviving world with a healthy free-energy stock (so EnergyDeath does
        // not preempt) plus two carcass-fraction trajectories. Nutrient piling
        // irreversibly into the dead pool is nutrient lockup; a turned-over dead
        // pool is not. The one scenario this targets (example9) photosynthesises
        // fine while sequestering nutrient — the two pools fail independently
        // (issue #342).
        let params = test_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 0,
            nutrient_lock_window: 5,
            ..EvalConfig::default()
        };
        let max_ticks = 20;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let _ = run_collecting_free_energy(&mut world, max_ticks);
        if world.agents().is_empty() {
            return; // need a surviving world to reach the nutrient-lockup branch
        }
        let n = world.tick() as usize;
        let healthy_energy: Vec<f32> = vec![100.0; n];

        let locked: Vec<f32> = (0..n).map(|t| if t < n / 2 { 0.05 } else { 0.6 }).collect();
        let dead = evaluate_from_log(
            &world,
            &obs(&healthy_energy, &locked, &[], &[]),
            &config,
            max_ticks,
        );
        assert_eq!(
            dead.failure,
            Some(FailureMode::NutrientLockup),
            "nutrient sequestering into the dead pool is nutrient lockup"
        );

        let turned_over: Vec<f32> = vec![0.05; n];
        let alive = evaluate_from_log(
            &world,
            &obs(&healthy_energy, &turned_over, &[], &[]),
            &config,
            max_ticks,
        );
        assert_ne!(
            alive.failure,
            Some(FailureMode::NutrientLockup),
            "a turned-over dead pool is not nutrient lockup"
        );
    }

    #[test]
    fn free_energy_dead_when_stock_collapses_into_carcasses() {
        // Living-energy stock peaks early, then drains to near zero and stays
        // there: energy is locking in carcasses and not coming back.
        let stock = vec![100.0, 95.0, 80.0, 50.0, 5.0, 3.0, 2.0, 1.0];
        assert!(is_free_energy_dead(&stock, 4));
    }

    #[test]
    fn free_energy_not_dead_when_stock_sustained() {
        // A living, reproducing world keeps regenerating free energy: the
        // trailing window holds a substantial fraction of the peak.
        let stock = vec![100.0, 90.0, 110.0, 95.0, 105.0, 98.0, 102.0, 100.0];
        assert!(!is_free_energy_dead(&stock, 5));
    }

    #[test]
    fn free_energy_not_dead_when_window_recovers() {
        // Stock dips but the living system recovers within the window — not an
        // irreversible trend toward zero.
        let stock = vec![100.0, 80.0, 10.0, 5.0, 2.0, 60.0, 90.0, 95.0];
        assert!(!is_free_energy_dead(&stock, 5));
    }

    #[test]
    fn free_energy_not_dead_when_shorter_than_window() {
        let stock = vec![0.0, 0.0];
        assert!(!is_free_energy_dead(&stock, 5));
    }

    /// The viable baseline of the search box: `F = 10`, `L = 100`, `r = 8`
    /// (`m = 18`), `B = 0.3`.
    fn baseline_params() -> explorers_sim::WorldParameters {
        let mut p = test_world_params();
        p.solar_flux_magnitude = 10.0;
        p.world_extent = 100.0;
        p.light_competition_radius = 8.0;
        p.base_metabolic_rate = 0.3;
        p
    }

    #[test]
    fn sustainable_stock_is_one_tick_of_base_metabolism_for_the_ceiling_population() {
        // N̄_max = F·m²/B = 10·324/0.3 bodies; each holds one tick of `B`,
        // so the stock is F·m² = 3240 E — independent of `B`.
        let stock = sustainable_stock(&baseline_params());
        assert!((stock - 3240.0).abs() < 1e-3, "{stock}");
        let mut cheaper = baseline_params();
        cheaper.base_metabolic_rate = 0.01;
        assert_eq!(sustainable_stock(&cheaper), stock);
        // A radius wider than the world tiles to one cell: F alone.
        let mut one_cell = baseline_params();
        one_cell.light_competition_radius = 1000.0;
        assert!((sustainable_stock(&one_cell) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn sustainable_read_alive_when_trailing_window_holds_the_ceiling_stock() {
        let ceiling = sustainable_stock(&baseline_params());
        let stock = vec![ceiling; 8];
        assert!(!is_free_energy_dead_sustainable(&stock, 4, ceiling));
    }

    #[test]
    fn sustainable_read_dead_when_trailing_window_peaks_at_five_percent_of_the_ceiling() {
        let ceiling = sustainable_stock(&baseline_params());
        // A bloom to the ceiling earlier in the run does not rescue a window
        // that peaks at 5 %: the reference is the config's, not the peak's.
        let mut stock = vec![ceiling; 4];
        stock.extend([
            0.05 * ceiling,
            0.04 * ceiling,
            0.05 * ceiling,
            0.03 * ceiling,
        ]);
        assert!(is_free_energy_dead_sustainable(&stock, 4, ceiling));
        // Shorter than the window: not consulted.
        assert!(!is_free_energy_dead_sustainable(&stock[..3], 4, ceiling));
    }

    #[test]
    fn sustainable_read_never_consults_the_founder_stock_inside_the_grace() {
        // A founder cohort provisioned at 1 % of the ceiling, then a world
        // that climbs to it: the horizon verdict reads post-grace only, so
        // the tick-0 stock is not on the series the read sees, and a run
        // still inside the grace has no read at all.
        let params = baseline_params();
        let ceiling = sustainable_stock(&params);
        let founder = 0.01 * ceiling;
        let mut series = vec![founder; 20];
        series.extend(vec![ceiling; 40]);
        let grace = 20usize;
        let window = 10usize;
        assert!(!is_free_energy_dead_sustainable(
            &series[grace..],
            window,
            ceiling
        ));
        // Inside the grace the caller hands the read nothing.
        let inside: &[f32] = series.get(grace..5).unwrap_or(&[]);
        assert!(!is_free_energy_dead_sustainable(inside, window, ceiling));
        // And the same founder stock, read with no grace at all, would be
        // consulted — the grace is what keeps the provisioning out.
        assert!(is_free_energy_dead_sustainable(
            &series[..grace],
            window,
            ceiling
        ));
    }

    #[test]
    fn nutrient_locked_when_dead_pool_share_stays_high() {
        // The carcass-locked fraction climbs and sits high across the whole
        // trailing window: nutrient is sequestered in the dead pool and the
        // living decomposers are not turning it over (issue #342).
        let frac = vec![0.05, 0.1, 0.2, 0.35, 0.45, 0.5, 0.52, 0.55];
        assert!(is_nutrient_locked(&frac, 4));
    }

    #[test]
    fn nutrient_not_locked_when_dead_pool_share_stays_low() {
        // The dead pool never holds much: carcasses are turned over as fast as
        // they form, so the fraction stays well below the lock threshold.
        let frac = vec![0.05, 0.1, 0.08, 0.12, 0.09, 0.11, 0.1, 0.07];
        assert!(!is_nutrient_locked(&frac, 5));
    }

    #[test]
    fn nutrient_not_locked_when_dead_pool_drains_back() {
        // The fraction spikes then drains back down — decomposers eat the dead
        // pool down within the window. Not an irreversible lockup.
        let frac = vec![0.1, 0.3, 0.6, 0.7, 0.5, 0.3, 0.15, 0.1];
        assert!(!is_nutrient_locked(&frac, 4));
    }

    #[test]
    fn nutrient_not_locked_when_dead_pool_high_but_receding() {
        // Still above the threshold late, but on a downward trend the whole
        // window — the system is turning the dead pool over, not locking it up.
        let frac = vec![0.9, 0.8, 0.7, 0.6, 0.5, 0.45, 0.42, 0.41];
        assert!(!is_nutrient_locked(&frac, 4));
    }

    #[test]
    fn nutrient_not_locked_when_shorter_than_window() {
        let frac = vec![0.5, 0.6];
        assert!(!is_nutrient_locked(&frac, 5));
    }

    #[test]
    fn evaluate_from_log_detects_monoculture() {
        let params = explorers_sim::WorldParameters {
            solar_flux_magnitude: 5.0,
            base_metabolic_rate: 0.01,
            reproduction_energy_threshold: 500.0,
            reproduction_nutrient_threshold: 1.0,
            contact_range_coefficient: 5.0,
            world_extent: 20.0,
            initial_population_size: 30,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_world_params()
        };
        let dist = explorers_sim::InitialDistribution {
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            ..test_distribution()
        };
        let config = EvalConfig {
            grace_ticks: 0,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 10;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let free = run_collecting_free_energy(&mut world, max_ticks);
        if world.agents().len() < 20 {
            return; // can't test monoculture with too few agents
        }
        let result = evaluate_from_log(&world, &obs(&free, &[], &[], &[]), &config, max_ticks);
        assert_eq!(
            result.failure,
            Some(FailureMode::Monoculture),
            "identical traits from single cluster should be monoculture, \
             clustering_strength={}",
            result.clustering_strength
        );
        assert_eq!(result.fitness, 0.0);
    }

    /// A monoculture world that reaches the horizon, with fabricated role
    /// snapshots over the settled window: `count` agents read in `role` on
    /// every sample, one of them the parent of a birth inside the window.
    /// The gate fires on the terminal roster, so this is the horizon-verdict
    /// path, not an early stop.
    fn gated_at_horizon_with_role(
        role: explorers_sim::topology::TrophicRole,
        count: u64,
    ) -> FitnessBreakdown {
        let params = explorers_sim::WorldParameters {
            solar_flux_magnitude: 5.0,
            base_metabolic_rate: 0.01,
            reproduction_energy_threshold: 500.0,
            reproduction_nutrient_threshold: 1.0,
            contact_range_coefficient: 5.0,
            world_extent: 20.0,
            initial_population_size: 30,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_world_params()
        };
        let dist = explorers_sim::InitialDistribution {
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            ..test_distribution()
        };
        let config = EvalConfig {
            grace_ticks: 0,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 10;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let free = run_collecting_free_energy(&mut world, max_ticks);
        assert!(
            world.agents().len() >= ROSTER_FLOOR,
            "the fixture must reach the horizon with a roster the gates read"
        );
        let roles: std::collections::HashMap<u64, explorers_sim::topology::TrophicRole> =
            (900_001..900_001 + count).map(|id| (id, role)).collect();
        let role_snapshots: Vec<guild::RoleSnapshot> =
            (6..=max_ticks).map(|t| (t, roles.clone())).collect();
        let born = vec![guild::Birth {
            tick: 7,
            parent: Some(900_001),
            second_parent: None,
        }];
        let observations = RolloutObservations {
            free_energy: free,
            role_snapshots,
            born,
            ..RolloutObservations::default()
        };
        let result = evaluate_from_log(&world, &observations, &config, max_ticks);
        assert_eq!(
            result.failure,
            Some(FailureMode::Monoculture),
            "the fixture must be gated at the horizon"
        );
        result
    }

    #[test]
    fn gated_at_horizon_still_reports_the_decomposer_guild() {
        // A world gated on its terminal read has already had its settled
        // window classified; the guild is a reported observable, not a scored
        // coordinate, so it survives the gate (#527).
        let result = gated_at_horizon_with_role(
            explorers_sim::topology::TrophicRole::Decomposer,
            guild::GUILD_MIN_SIZE as u64,
        );
        assert!(result.has_decomposer_guild);
        assert!(!result.has_consumer_guild);
    }

    #[test]
    fn gated_at_horizon_still_reports_the_consumer_guild() {
        let result = gated_at_horizon_with_role(
            explorers_sim::topology::TrophicRole::Consumer,
            guild::GUILD_MIN_SIZE as u64,
        );
        assert!(result.has_consumer_guild);
        assert!(!result.has_decomposer_guild);
    }

    #[test]
    fn gated_at_horizon_scores_nothing_though_it_reports_the_guild() {
        // The authority boundary does not move: carrying the observable
        // through the gate leaves every scored descriptor zero.
        let result = gated_at_horizon_with_role(
            explorers_sim::topology::TrophicRole::Decomposer,
            guild::GUILD_MIN_SIZE as u64,
        );
        assert_eq!(result.fitness, 0.0);
        assert!(result.failure.is_some());
        assert_eq!(result.oscillation_strength, 0.0);
        assert_eq!(result.clustering_strength, 0.0);
        assert_eq!(result.coexistence_duration, 0.0);
        assert_eq!(result.turnover_score, 0.0);
        assert_eq!(result.trophic_balance_score, 0.0);
        assert_eq!(result.carcass_locked_fraction, 0.0);
    }

    #[test]
    fn gated_at_horizon_below_the_guild_floor_reports_no_guild() {
        // Carrying the read is not carrying a `true`: the same predicate
        // applies, so a window short of `GUILD_MIN_SIZE` still reads false.
        let result = gated_at_horizon_with_role(
            explorers_sim::topology::TrophicRole::Decomposer,
            guild::GUILD_MIN_SIZE as u64 - 1,
        );
        assert!(!result.has_decomposer_guild);
        assert!(!result.has_consumer_guild);
    }

    #[test]
    fn evaluate_from_log_detects_population_explosion() {
        let params = explorers_sim::WorldParameters {
            initial_population_size: 10001,
            ..test_world_params()
        };
        let dist = test_distribution();
        let config = EvalConfig::default();
        let max_ticks: u64 = 1;
        let world = explorers_sim::World::new(params, dist, 42);
        let result = evaluate_from_log(&world, &obs(&[], &[], &[], &[]), &config, max_ticks);
        assert_eq!(result.failure, Some(FailureMode::PopulationExplosion));
        assert_eq!(result.fitness, 0.0);
    }

    /// A snapshot of `count` tight, well-separated trait blobs (each dense enough
    /// to form a DBSCAN cluster under the default config), so
    /// `distinct_cluster_count` reads exactly `count`.
    fn snapshot_with_clusters(count: usize) -> Vec<explorers_sim::TraitVector> {
        let mut traits = Vec::new();
        for c in 0..count {
            for i in 0..10 {
                traits.push(make_trait_vector([
                    c as f32 * 5.0 + i as f32 * 0.01,
                    0.0,
                    0.0,
                    0.0,
                ]));
            }
        }
        traits
    }

    #[test]
    fn evaluation_is_unchanged_when_the_rollout_keeps_only_the_evaluator_kinds_and_drops_history_once_read()
     {
        // Issue #502: the evaluator's reads off the event log (turnover counts,
        // the guild's roles and recruitment) are taken tick by tick by
        // `observe`, so a rollout can retain only `EVALUATOR_EVENT_KINDS` and
        // compact the log before `consumed_events()` after every step. That is
        // observer-side only — the breakdown must be identical to a full-log
        // rollout's.
        // A live fixture: `test_world_params` is extinct by tick 1.
        let params = explorers_sim::WorldParameters {
            base_metabolic_rate: 0.1,
            movement_cost_coefficient: 0.01,
            reproduction_energy_threshold: 10.0,
            offspring_structure_fraction: 0.0,
            initial_nutrient_pool: 50.0,
            ..test_world_params()
        };
        let dist = explorers_sim::InitialDistribution {
            initial_energy_per_agent: 100.0,
            mean_traits: explorers_sim::TraitVector {
                fecundity: 0.5,
                asexual_propensity: 1.0,
                kappa: 0.5,
                ..test_distribution().mean_traits
            },
            ..test_distribution()
        };
        let config = EvalConfig::default();
        let max_ticks: u64 = 200;
        let interval = config.coexistence_sample_interval;

        let mut full = explorers_sim::World::new(params.clone(), dist.clone(), 42);
        let mut full_obs = RolloutObservations::with_capacity(max_ticks as usize);
        let mut lean = explorers_sim::World::new(params, dist, 42);
        let mut lean_obs = RolloutObservations::with_capacity(max_ticks as usize);
        lean.retain_event_kinds(EVALUATOR_EVENT_KINDS);
        for _ in 0..max_ticks {
            full.step();
            full_obs.observe(&full, interval);
            lean.step();
            lean_obs.observe(&lean, interval);
            lean.compact_event_log_before(lean_obs.consumed_events());
            assert_eq!(
                lean.event_log().retained(),
                0,
                "history is dropped once read"
            );
            if full.agents().is_empty() {
                break;
            }
        }
        assert!(
            full.event_log().len() > lean.event_log().retained(),
            "the full-log rollout is the one paying for history"
        );

        let expected = evaluate_from_log(&full, &full_obs, &config, max_ticks);
        let actual = evaluate_from_log(&lean, &lean_obs, &config, max_ticks);
        assert!(
            expected.turnover_score > 0.0,
            "the fixture turns over, so the counts are exercised: {expected:?}"
        );
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
    }

    #[test]
    fn evaluate_from_log_reads_both_heterotroph_guilds_off_the_role_snapshots() {
        // The guild read (#490) is a population read over the second-half role
        // snapshots, not a terminal-roster role tag. Feed a real run's births
        // with fabricated role snapshots: six ids read as consumers on every
        // sample, one of them a real second-half parent, so the consumer guild
        // holds and the decomposer guild does not.
        let params = test_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 60,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 60;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let free = run_collecting_free_energy(&mut world, max_ticks);
        if world.agents().is_empty() {
            return;
        }
        let parent = world
            .event_log()
            .by_kind(&explorers_sim::event::EventKind::Born)
            .iter()
            .find(|e| e.tick >= max_ticks / 2)
            .and_then(|e| e.target)
            .expect("the test run breeds in its second half");
        let born: Vec<guild::Birth> = world
            .event_log()
            .by_kind(&explorers_sim::event::EventKind::Born)
            .into_iter()
            .filter_map(guild::Birth::of)
            .collect();
        let roles: std::collections::HashMap<u64, explorers_sim::topology::TrophicRole> = [parent]
            .into_iter()
            .chain(900_001..900_006)
            .map(|id| (id, explorers_sim::topology::TrophicRole::Consumer))
            .collect();
        let role_snapshots: Vec<guild::RoleSnapshot> =
            (1..=6).map(|k| (k * 10, roles.clone())).collect();
        let observations = RolloutObservations {
            free_energy: free,
            role_snapshots,
            born,
            ..RolloutObservations::default()
        };
        let result = evaluate_from_log(&world, &observations, &config, max_ticks);
        assert!(result.has_consumer_guild);
        assert!(!result.has_decomposer_guild);

        // Without any roster samples there is no population to read.
        let none = evaluate_from_log(
            &world,
            &RolloutObservations {
                free_energy: observations.free_energy.clone(),
                ..RolloutObservations::default()
            },
            &config,
            max_ticks,
        );
        assert!(!none.has_consumer_guild);
        assert!(!none.has_decomposer_guild);
    }

    #[test]
    fn evaluate_from_log_coexistence_fraction_from_cluster_snapshots() {
        // Coexistence is the fraction of settled-window sampled ticks whose
        // living population carries >=2 trait-space clusters (issue #394). Feed
        // a known K of N settled snapshots that carry >=2 clusters and assert
        // the breakdown's coexistence_duration is exactly K/N. The fitness is
        // the weighted sum of the five components.
        let params = live_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 0,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 50;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let free = run_collecting_free_energy(&mut world, max_ticks);
        assert!(
            !world.agents().is_empty() && world.tick() == max_ticks,
            "fixture world must survive to the horizon"
        );
        // 5 snapshots, all in the settled window (tick > 25): 3 with >=2
        // clusters, 2 with a single cluster → K/N = 3/5.
        let snapshots: Vec<(u64, Vec<explorers_sim::TraitVector>)> = vec![
            (30, snapshot_with_clusters(2)),
            (35, snapshot_with_clusters(1)),
            (40, snapshot_with_clusters(3)),
            (45, snapshot_with_clusters(1)),
            (50, snapshot_with_clusters(2)),
        ];
        let result = evaluate_from_log(
            &world,
            &obs(&free, &[], &[], &snapshots),
            &config,
            max_ticks,
        );
        assert!(
            (result.coexistence_duration - 3.0 / 5.0).abs() < 1e-5,
            "coexistence should be K/N = 3/5, got {}",
            result.coexistence_duration
        );
        let fitness = 0.2 * result.oscillation_strength
            + 0.2 * result.clustering_strength
            + 0.2 * result.coexistence_duration
            + 0.2 * result.turnover_score
            + 0.2 * result.trophic_balance_score;
        assert_eq!(
            result.fitness, fitness,
            "fitness should be weighted sum of components"
        );
    }

    /// The coexistence fixture: a world run to the horizon with a set of
    /// settled-window cluster snapshots, so the evaluation reaches the
    /// per-snapshot DBSCAN a deadline is checked between.
    fn settled_fixture() -> (explorers_sim::World, RolloutObservations, EvalConfig, u64) {
        let config = EvalConfig {
            grace_ticks: 0,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 50;
        let mut world = explorers_sim::World::new(live_world_params(), test_distribution(), 42);
        let free = run_collecting_free_energy(&mut world, max_ticks);
        let snapshots: Vec<(u64, Vec<explorers_sim::TraitVector>)> = vec![
            (30, snapshot_with_clusters(2)),
            (40, snapshot_with_clusters(1)),
            (50, snapshot_with_clusters(3)),
        ];
        (world, obs(&free, &[], &[], &snapshots), config, max_ticks)
    }

    #[test]
    fn an_evaluation_past_its_deadline_abandons_with_no_verdict() {
        let (world, observations, config, max_ticks) = settled_fixture();
        let deadline = std::time::Instant::now();
        assert!(
            evaluate_from_log_within(&world, &observations, &config, max_ticks, deadline).is_none(),
            "a spent deadline yields no verdict rather than a sentinel breakdown"
        );
    }

    #[test]
    fn an_evaluation_inside_its_deadline_is_the_unbounded_verdict() {
        let (world, observations, config, max_ticks) = settled_fixture();
        let unbounded = evaluate_from_log(&world, &observations, &config, max_ticks);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3600);
        let bounded = evaluate_from_log_within(&world, &observations, &config, max_ticks, deadline)
            .expect("a generous deadline reaches a verdict");
        assert_eq!(format!("{bounded:?}"), format!("{unbounded:?}"));
    }

    #[test]
    fn coexistence_is_invariant_to_initial_population_size() {
        // Coexistence is now a pure function of the snapshots and DBSCAN config —
        // the searched seed parameter `initial_population_size` must NOT leak in
        // (issue #394, the leak #390/#392 removed from oscillation). Two worlds
        // built with different initial_population_size, fed identical snapshots,
        // must report identical coexistence_duration.
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 0,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 50;

        // Settled-window snapshots (tick > 25): 2 of 3 carry >=2 clusters.
        let snapshots: Vec<(u64, Vec<explorers_sim::TraitVector>)> = vec![
            (30, snapshot_with_clusters(2)),
            (40, snapshot_with_clusters(1)),
            (50, snapshot_with_clusters(2)),
        ];

        let build = |pop: u32| {
            let params = explorers_sim::WorldParameters {
                initial_population_size: pop,
                ..live_world_params()
            };
            let mut world = explorers_sim::World::new(params, dist.clone(), 42);
            let free = run_collecting_free_energy(&mut world, max_ticks);
            assert!(
                !world.agents().is_empty() && world.tick() == max_ticks,
                "fixture world must survive to the horizon"
            );
            (world, free)
        };

        let (world_a, free_a) = build(10);
        let (world_b, free_b) = build(30);
        let a = evaluate_from_log(
            &world_a,
            &obs(&free_a, &[], &[], &snapshots),
            &config,
            max_ticks,
        );
        let b = evaluate_from_log(
            &world_b,
            &obs(&free_b, &[], &[], &snapshots),
            &config,
            max_ticks,
        );
        assert!(
            a.failure.is_none() && b.failure.is_none(),
            "both fixtures must reach the ungated read (a: {:?}, b: {:?})",
            a.failure,
            b.failure
        );
        assert_eq!(
            a.coexistence_duration, b.coexistence_duration,
            "coexistence must not depend on initial_population_size"
        );
        assert!((a.coexistence_duration - 2.0 / 3.0).abs() < 1e-5);
    }

    #[test]
    fn coexistence_reads_snapshots_in_the_settled_window_only() {
        // Coexistence is the fraction of settled-window snapshots — tick in
        // `(T/2, T]` — carrying >=2 clusters (genesis-search.md, *One rollout,
        // one settled window*), not the post-grace snapshots. Structure that
        // exists only in the first half reads zero; structure on every settled
        // snapshot reads full, however many bloom-stage snapshots carried one
        // cluster. The snapshot at exactly T/2 is outside the window.
        let params = live_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 0,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 100;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let free = run_collecting_free_energy(&mut world, max_ticks);
        assert!(
            !world.agents().is_empty() && world.tick() == max_ticks,
            "fixture world must survive to the horizon"
        );

        let first_half_only: Vec<(u64, Vec<explorers_sim::TraitVector>)> = vec![
            (10, snapshot_with_clusters(2)),
            (30, snapshot_with_clusters(3)),
            (50, snapshot_with_clusters(2)),
            (70, snapshot_with_clusters(1)),
            (90, snapshot_with_clusters(1)),
        ];
        let bd = evaluate_from_log(
            &world,
            &obs(&free, &[], &[], &first_half_only),
            &config,
            max_ticks,
        );
        assert_eq!(
            bd.coexistence_duration, 0.0,
            "clusters confined to the bloom half (and the T/2 snapshot) read zero"
        );

        let second_half_only: Vec<(u64, Vec<explorers_sim::TraitVector>)> = vec![
            (10, snapshot_with_clusters(1)),
            (30, snapshot_with_clusters(1)),
            (50, snapshot_with_clusters(1)),
            (70, snapshot_with_clusters(2)),
            (90, snapshot_with_clusters(3)),
        ];
        let bd = evaluate_from_log(
            &world,
            &obs(&free, &[], &[], &second_half_only),
            &config,
            max_ticks,
        );
        assert_eq!(
            bd.coexistence_duration, 1.0,
            "clusters on every settled snapshot read full"
        );
    }

    #[test]
    fn coexistence_zero_when_survival_within_grace() {
        // Guard mirroring the oscillation guard (#392): if ticks_survived <=
        // grace_ticks, coexistence_duration is 0.0 regardless of snapshots.
        let params = test_world_params();
        let dist = test_distribution();
        // grace_ticks = max_ticks, so a surviving
        // world never exceeds grace.
        let config = EvalConfig {
            grace_ticks: 50,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 50;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let free = run_collecting_free_energy(&mut world, max_ticks);
        if world.agents().is_empty() {
            return;
        }
        let snapshots: Vec<(u64, Vec<explorers_sim::TraitVector>)> = vec![
            (5, snapshot_with_clusters(2)),
            (40, snapshot_with_clusters(3)),
        ];
        let result = evaluate_from_log(
            &world,
            &obs(&free, &[], &[], &snapshots),
            &config,
            max_ticks,
        );
        assert_eq!(
            result.coexistence_duration, 0.0,
            "survival within grace → coexistence 0.0"
        );
    }

    #[test]
    fn coexistence_zero_when_no_snapshots() {
        // Empty cluster_snapshots → coexistence 0.0 (a fixed-state caller with no
        // rollout to sample passes an empty slice).
        let params = test_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 0,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 50;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let free = run_collecting_free_energy(&mut world, max_ticks);
        if world.agents().is_empty() {
            return;
        }
        let result = evaluate_from_log(&world, &obs(&free, &[], &[], &[]), &config, max_ticks);
        assert_eq!(result.coexistence_duration, 0.0);
    }

    #[test]
    fn evaluate_from_log_oscillation_from_producer_share_series() {
        // Oscillation reads the per-tick producer-energy-share series the
        // caller samples (issue #392), over the settled window. A surviving
        // world fed an oscillating share series surfaces a positive oscillation
        // strength that matches the standalone descriptor on the settled slice;
        // a flat series surfaces ~0. Seed-invariant by construction — no lineage
        // clusters, no initial_population_size.
        let params = live_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 12,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 64;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let (free, _share) = run_collecting_free_energy_and_share(&mut world, max_ticks);
        assert!(
            !world.agents().is_empty() && world.tick() == max_ticks,
            "fixture world must survive to the horizon"
        );
        let n = max_ticks as usize;

        // A synthetic oscillating producer-share series: a clean producer↔consumer
        // rhythm the evaluator should pick up over the settled window.
        let period = 8.0;
        let oscillating: Vec<f32> = (0..n)
            .map(|i| 0.5 + 0.3 * (2.0 * std::f32::consts::PI * i as f32 / period).sin())
            .collect();
        let bd = evaluate_from_log(
            &world,
            &obs(&free, &[], &oscillating, &[]),
            &config,
            max_ticks,
        );
        assert_eq!(
            bd.oscillation_strength,
            oscillation_strength(&oscillating[n / 2..]),
            "breakdown oscillation must be the descriptor over the settled slice"
        );
        assert!(
            bd.oscillation_strength > 0.6,
            "an oscillating producer-share series should read high: {}",
            bd.oscillation_strength
        );

        // A flat producer-share series is a frozen fixed point → ~0 oscillation.
        let flat: Vec<f32> = vec![0.5; n];
        let bd_flat = evaluate_from_log(&world, &obs(&free, &[], &flat, &[]), &config, max_ticks);
        assert_eq!(
            bd_flat.oscillation_strength, 0.0,
            "a flat producer-share series should read 0 oscillation"
        );
    }

    #[test]
    fn oscillation_reads_the_settled_window_not_the_post_grace_series() {
        // The atlas maps the settled community: oscillation is read off the
        // producer-share series over `(T/2, T]` (genesis-search.md, *One
        // rollout, one settled window*), not the post-grace series. A rhythm
        // that lives only in the first half reads zero; one that lives only in
        // the second half reads exactly the descriptor on that half.
        let params = live_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 0,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 64;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let (free, _share) = run_collecting_free_energy_and_share(&mut world, max_ticks);
        assert!(
            !world.agents().is_empty() && world.tick() == max_ticks,
            "fixture world must survive to the horizon for the window to be read"
        );
        let n = max_ticks as usize;
        let half = n / 2;
        let period = 8.0;
        let rhythm = |i: usize| 0.5 + 0.3 * (2.0 * std::f32::consts::PI * i as f32 / period).sin();

        let first_half_only: Vec<f32> = (0..n)
            .map(|i| if i < half { rhythm(i) } else { 0.5 })
            .collect();
        let bd = evaluate_from_log(
            &world,
            &obs(&free, &[], &first_half_only, &[]),
            &config,
            max_ticks,
        );
        assert_eq!(
            bd.oscillation_strength, 0.0,
            "a rhythm confined to the bloom half must not read on the settled window"
        );

        let second_half_only: Vec<f32> = (0..n)
            .map(|i| if i >= half { rhythm(i) } else { 0.5 })
            .collect();
        let bd = evaluate_from_log(
            &world,
            &obs(&free, &[], &second_half_only, &[]),
            &config,
            max_ticks,
        );
        assert_eq!(
            bd.oscillation_strength,
            oscillation_strength(&second_half_only[half..]),
            "the settled window is exactly the second half of the series"
        );
        assert!(
            bd.oscillation_strength > 0.6,
            "a rhythm over the whole settled window reads high: {}",
            bd.oscillation_strength
        );
    }

    #[test]
    fn evaluate_from_log_clustering_and_trophic_from_final_state() {
        let params = test_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_ticks: 50,
            ..EvalConfig::default()
        };
        let max_ticks = 50;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let free = run_collecting_free_energy(&mut world, max_ticks);
        if world.agents().is_empty() {
            return; // can't test final-state metrics on extinct world
        }
        let result = evaluate_from_log(&world, &obs(&free, &[], &[], &[]), &config, max_ticks);

        let trait_vectors: Vec<_> = world.agents().iter().map(|a| a.traits).collect();
        let energies: Vec<_> = world.agents().iter().map(|a| a.energy()).collect();
        let expected_cs = if trait_vectors.len() >= 4 {
            clustering_strength(&trait_vectors)
        } else {
            0.0
        };
        let labels = dbscan(&trait_vectors, config.dbscan_eps, config.dbscan_min_points);
        let expected_tb = trophic_balance_score(&trait_vectors, &labels, &energies);

        assert_eq!(result.clustering_strength, expected_cs);
        assert_eq!(result.trophic_balance_score, expected_tb);
    }

    #[test]
    fn evaluate_from_log_returns_zero_fitness_on_extinction() {
        let params = explorers_sim::WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 100.0,
            sensing_range_coefficient: 10.0,
            base_trophic_efficiency: 0.5,
            trophic_distance_decay: 0.0,
            reproduction_efficiency: 0.7,
            movement_cost_coefficient: 0.0,
            reproduction_energy_threshold: 50.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_range_coefficient: 5.0,
            world_extent: 100.0,
            initial_population_size: 1,
            light_competition_radius: 1000.0,
            photo_maintenance_cost: 0.0,
            heterotrophy_maintenance_cost: 0.0,
            initial_nutrient_pool: 0.0,
            growth_efficiency: 0.0,
            wear_rate: 0.0,
            wear_degradation_steepness: 0.0,
            somatic_maintenance_cost_coefficient: 0.0,
            use_wear_rate: 0.0,
            structure_maintenance_coefficient: 0.0,
            repair_decay: 0.0,
            base_nutrient_ratio: 0.1,
            specification_nutrient_coefficient: 0.2,
            reproductive_compatibility_distance: 2.0,
            mobility_maintenance_cost: 0.0,
            maintenance_cost_exponent: 1.0,
            nutrient_grid_cell_size: 10.0,
            growth_retention_multiplier: 2.0,
            reserve_mobilisation_rate: 1.0,
            offspring_structure_fraction: 0.2,
            asexual_propensity_maintenance_cost: 0.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            dispersal_reach_coefficient: 0.0,
            body_reach_coefficient: 0.0,
            network_connection_cap: 0,
            network_creation_cost: 0.0,
            network_maintenance_cost: 0.0,
            network_redistribution_rate: 0.0,
            network_transfer_efficiency: 0.0,
        };
        let dist = explorers_sim::InitialDistribution {
            mean_traits: explorers_sim::TraitVector {
                photosynthetic_absorption: 0.0,
                heterotrophy: 0.0,
                mobility: 0.0,
                kappa: 0.0,
                fecundity: 0.0,
                asexual_propensity: 0.0,
                dispersal: 0.0,
            },
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let config = EvalConfig::default();
        let max_ticks = 100;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let free = run_collecting_free_energy(&mut world, 10);
        let result = evaluate_from_log(&world, &obs(&free, &[], &[], &[]), &config, max_ticks);
        assert_eq!(result.fitness, 0.0);
        assert_eq!(result.failure, Some(FailureMode::Extinction));
    }

    #[test]
    fn extinct_when_no_agents() {
        assert!(is_extinct(0));
    }

    #[test]
    fn not_extinct_when_agents_exist() {
        assert!(!is_extinct(5));
    }

    #[test]
    fn population_explosion_above_ceiling() {
        assert!(is_population_explosion(101, 100));
    }

    #[test]
    fn no_population_explosion_at_or_below_ceiling() {
        assert!(!is_population_explosion(100, 100));
        assert!(!is_population_explosion(50, 100));
    }

    fn make_trait_vector(vals: [f32; 4]) -> explorers_sim::TraitVector {
        explorers_sim::TraitVector {
            photosynthetic_absorption: vals[0],
            heterotrophy: vals[1],
            mobility: vals[2],
            kappa: vals[3],
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        }
    }

    #[test]
    fn clustering_strength_high_for_bimodal_traits() {
        let mut traits = Vec::new();
        for i in 0..50 {
            traits.push(make_trait_vector([i as f32 * 0.01, 0.0, 0.0, 0.0]));
        }
        for i in 0..50 {
            traits.push(make_trait_vector([5.0 + i as f32 * 0.01, 0.0, 0.0, 0.0]));
        }
        let strength = clustering_strength(&traits);
        assert!(
            strength > 0.5,
            "bimodal traits should have high clustering strength: {strength}"
        );
    }

    #[test]
    fn clustering_strength_low_for_unimodal_traits() {
        use rand::SeedableRng;
        use rand_distr::{Distribution, Normal};
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);
        let dist = Normal::new(0.5_f32, 0.2).unwrap();
        let traits: Vec<_> = (0..100)
            .map(|_| make_trait_vector([dist.sample(&mut rng), dist.sample(&mut rng), 0.0, 0.0]))
            .collect();
        let strength = clustering_strength(&traits);
        assert!(
            strength < 0.5,
            "unimodal traits should have low clustering strength: {strength}"
        );
    }

    #[test]
    fn monoculture_detected_for_unimodal_traits() {
        use rand::SeedableRng;
        use rand_distr::{Distribution, Normal};
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);
        let dist = Normal::new(0.5_f32, 0.2).unwrap();
        let traits: Vec<_> = (0..100)
            .map(|_| make_trait_vector([dist.sample(&mut rng), dist.sample(&mut rng), 0.0, 0.0]))
            .collect();
        assert!(is_monoculture(&traits, 0.5));
    }

    #[test]
    fn dbscan_finds_two_clusters() {
        let mut traits = Vec::new();
        for i in 0..10 {
            traits.push(make_trait_vector([i as f32 * 0.01, 0.0, 0.0, 0.0]));
        }
        for i in 0..10 {
            traits.push(make_trait_vector([5.0 + i as f32 * 0.01, 0.0, 0.0, 0.0]));
        }
        let labels = dbscan(&traits, 0.5, 3);
        let cluster_ids: std::collections::HashSet<_> = labels.iter().filter_map(|l| *l).collect();
        assert_eq!(
            cluster_ids.len(),
            2,
            "should find 2 clusters, got {cluster_ids:?}"
        );
    }

    #[test]
    fn dbscan_uniform_scatter_gives_no_clusters() {
        let traits: Vec<_> = (0..10)
            .map(|i| make_trait_vector([i as f32 * 10.0, 0.0, 0.0, 0.0]))
            .collect();
        let labels = dbscan(&traits, 0.5, 3);
        let cluster_count = labels
            .iter()
            .filter_map(|l| *l)
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert!(
            cluster_count <= 1,
            "widely scattered points should have 0-1 clusters, got {cluster_count}"
        );
    }

    #[test]
    fn distinct_cluster_count_two_blobs() {
        // Two well-separated trait blobs → two distinct DBSCAN clusters.
        let mut traits = Vec::new();
        for i in 0..10 {
            traits.push(make_trait_vector([i as f32 * 0.01, 0.0, 0.0, 0.0]));
        }
        for i in 0..10 {
            traits.push(make_trait_vector([5.0 + i as f32 * 0.01, 0.0, 0.0, 0.0]));
        }
        assert_eq!(distinct_cluster_count(&traits, 0.5, 3), 2);
    }

    #[test]
    fn distinct_cluster_count_one_blob() {
        // One tight blob → a single cluster.
        let traits: Vec<_> = (0..10)
            .map(|i| make_trait_vector([i as f32 * 0.01, 0.0, 0.0, 0.0]))
            .collect();
        assert_eq!(distinct_cluster_count(&traits, 0.5, 3), 1);
    }

    #[test]
    fn distinct_cluster_count_sparse_scatter_is_zero() {
        // Points too sparse to meet min_points density → all noise → zero clusters.
        let traits: Vec<_> = (0..10)
            .map(|i| make_trait_vector([i as f32 * 10.0, 0.0, 0.0, 0.0]))
            .collect();
        assert_eq!(distinct_cluster_count(&traits, 0.5, 3), 0);
    }

    #[test]
    fn dbscan_noise_points_are_none() {
        let mut traits = Vec::new();
        for i in 0..10 {
            traits.push(make_trait_vector([i as f32 * 0.01, 0.0, 0.0, 0.0]));
        }
        // Add an outlier far away
        traits.push(make_trait_vector([100.0, 0.0, 0.0, 0.0]));
        let labels = dbscan(&traits, 0.5, 3);
        assert_eq!(labels[10], None, "outlier should be noise");
    }

    #[test]
    fn demographic_turnover_requires_both_births_and_deaths() {
        assert!(has_demographic_turnover(5, 3));
        assert!(!has_demographic_turnover(0, 3));
        assert!(!has_demographic_turnover(5, 0));
        assert!(!has_demographic_turnover(0, 0));
    }

    #[test]
    fn trophic_pyramid_producers_have_more_energy_than_consumers() {
        let mut traits = Vec::new();
        let mut labels = Vec::new();
        let mut energies = Vec::new();
        // Producers (high photosynthesis, low consumption)
        for _ in 0..10 {
            traits.push(make_trait_vector([0.9, 0.1, 0.0, 0.0]));
            labels.push(Some(0));
            energies.push(100.0);
        }
        // Consumers (low photosynthesis, high consumption)
        for _ in 0..5 {
            traits.push(make_trait_vector([0.1, 0.9, 0.0, 0.0]));
            labels.push(Some(1));
            energies.push(50.0);
        }
        assert!(has_trophic_pyramid(&traits, &labels, &energies));
    }

    #[test]
    fn trophic_pyramid_fails_when_consumers_have_more_energy() {
        let mut traits = Vec::new();
        let mut labels = Vec::new();
        let mut energies = Vec::new();
        // Producers with little energy
        for _ in 0..5 {
            traits.push(make_trait_vector([0.9, 0.1, 0.0, 0.0]));
            labels.push(Some(0));
            energies.push(10.0);
        }
        // Consumers with lots of energy (inverted pyramid)
        for _ in 0..10 {
            traits.push(make_trait_vector([0.1, 0.9, 0.0, 0.0]));
            labels.push(Some(1));
            energies.push(100.0);
        }
        assert!(!has_trophic_pyramid(&traits, &labels, &energies));
    }

    #[test]
    fn coexistence_full_when_always_multiple_clusters() {
        let counts = vec![3, 3, 2, 4, 3];
        assert!((coexistence_duration(&counts) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn coexistence_zero_when_always_one_cluster() {
        let counts = vec![1, 1, 1, 1];
        assert!(coexistence_duration(&counts).abs() < 1e-5);
    }

    #[test]
    fn coexistence_partial() {
        // 2 out of 4 ticks have >=2 clusters
        let counts = vec![1, 2, 1, 3];
        assert!((coexistence_duration(&counts) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn autocorrelation_high_for_sinusoidal_series() {
        let n = 200;
        let period = 20.0;
        let series: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * i as f32 / period).sin())
            .collect();
        let ac = autocorrelation(&series, 20);
        assert!(
            ac > 0.8,
            "sinusoidal series at lag=period should have high autocorrelation: {ac}"
        );
    }

    #[test]
    fn autocorrelation_near_zero_for_flat_series() {
        let series = vec![5.0; 100];
        let ac = autocorrelation(&series, 10);
        assert!(
            ac.abs() < 0.01,
            "flat series should have ~0 autocorrelation: {ac}"
        );
    }

    #[test]
    fn oscillation_strength_high_for_sustained_sine() {
        // A sustained producer↔consumer rhythm (limit cycle): the detrended
        // share series has a deep anti-correlation at half its period, so the
        // descriptor reads high (issue #392).
        let n = 64;
        let period = 8.0;
        let series: Vec<f32> = (0..n)
            .map(|i| 0.5 + 0.3 * (2.0 * std::f32::consts::PI * i as f32 / period).sin())
            .collect();
        let strength = oscillation_strength(&series);
        assert!(
            strength > 0.6,
            "a sustained sine should read high: {strength}"
        );
    }

    #[test]
    fn oscillation_strength_zero_for_flat_constant() {
        // A frozen fixed point: a flat residual has no anti-correlation → 0.
        let series = vec![0.5_f32; 64];
        assert_eq!(oscillation_strength(&series), 0.0);
    }

    #[test]
    fn oscillation_strength_near_zero_for_linear_ramp() {
        // Steady colonization growth, not a cycle: the linear detrend removes the
        // ramp, leaving a ~0 residual → ~0. (The old metric scored this maximal.)
        let series: Vec<f32> = (0..64).map(|i| 0.1 + 0.005 * i as f32).collect();
        let strength = oscillation_strength(&series);
        assert!(strength < 0.05, "a linear ramp should read ~0: {strength}");
    }

    #[test]
    fn oscillation_strength_below_threshold_for_white_noise() {
        // Uncorrelated noise is not anti-correlated: min autocorrelation hovers
        // near 0, so the descriptor stays small. A deterministic pseudo-noise
        // series keeps the test reproducible.
        let mut state: u32 = 0x1234_5678;
        let series: Vec<f32> = (0..256)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 8) as f32 / (1u32 << 24) as f32
            })
            .collect();
        let strength = oscillation_strength(&series);
        assert!(strength < 0.2, "white noise should read low: {strength}");
    }

    #[test]
    fn oscillation_strength_small_positive_for_damped_sine() {
        // Damped ringing on the stable side of the Hopf bifurcation: the
        // anti-correlation is present but shallow, giving a small positive read —
        // a pre-onset gradient pointing toward the bifurcation, well below a
        // sustained cycle.
        let n = 64;
        let period = 8.0;
        let series: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32;
                let envelope = (-t / 3.0).exp();
                0.5 + 0.3 * envelope * (2.0 * std::f32::consts::PI * t / period).sin()
            })
            .collect();
        let strength = oscillation_strength(&series);
        // Positive (the ringing leaves an anti-correlation footprint) but well
        // below a sustained cycle (> 0.6) — the pre-onset gradient.
        assert!(
            strength > 0.0 && strength < 0.4,
            "damped sine should be small-but-positive: {strength}"
        );
    }

    #[test]
    fn oscillation_strength_zero_for_series_shorter_than_min_len() {
        let series = vec![0.1, 0.9, 0.1, 0.9, 0.1, 0.9, 0.1];
        assert_eq!(oscillation_strength(&series), 0.0);
    }

    #[test]
    fn oscillation_strength_zero_for_near_flat_drift_with_noise() {
        // Issue #403: a near-constant producer share with a microscopic monotone
        // creep plus low-amplitude noise — the example10 seed-4 witness, which
        // ranges only ~0.997–1.000 (peak-to-peak ~0.003). The linear detrend
        // removes the drift, leaving noise whose variance is *not* small relative
        // to the (also tiny) raw variance, so the relative guard does not trip;
        // autocorrelating that noise finds a spurious deep anti-correlation at a
        // long lag and reports a meaningless positive strength. A swing this far
        // below the regime-signal floor must read 0 regardless of residual
        // structure.
        let mut state: u32 = 0x0bad_5eed;
        let n = 512;
        let series: Vec<f32> = (0..n)
            .map(|i| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                // Drift 0.997 → 1.000 across the series, plus ±~0.0003 noise.
                let drift = 0.997 + 0.003 * i as f32 / (n - 1) as f32;
                let noise = ((state >> 8) as f32 / (1u32 << 24) as f32 - 0.5) * 0.0006;
                drift + noise
            })
            .collect();
        assert_eq!(
            oscillation_strength(&series),
            0.0,
            "a ~0.003-swing near-flat series must read 0, not a spurious anti-correlation"
        );
    }

    #[test]
    fn trophic_coordinates_pure_producer() {
        let traits = make_trait_vector([1.0, 0.0, 0.0, 0.0]);
        let (photo, hetero) = trophic_coordinates(&traits);
        assert!((photo - 1.0).abs() < 1e-5);
        assert!(hetero.abs() < 1e-5);
    }

    #[test]
    fn trophic_coordinates_mixed() {
        let traits = make_trait_vector([0.3, 0.3, 0.0, 0.0]);
        let (photo, hetero) = trophic_coordinates(&traits);
        let sum = photo + hetero;
        assert!((sum - 1.0).abs() < 1e-5, "should sum to 1: {sum}");
        assert!((photo - 0.5).abs() < 0.01);
    }

    #[test]
    fn trophic_coordinates_zero_energy_traits() {
        let traits = make_trait_vector([0.0, 0.0, 1.0, 0.0]);
        let (photo, _hetero) = trophic_coordinates(&traits);
        assert!(
            (photo - 0.5).abs() < 0.01,
            "should default to equal: {photo}"
        );
    }

    #[test]
    fn generalist_dominant_when_one_cluster_has_high_all_traits() {
        // Cluster 0: generalists (high photo, consumption, scavenging)
        let mut traits = Vec::new();
        let mut labels = Vec::new();
        let mut energies = Vec::new();
        for _ in 0..10 {
            traits.push(make_trait_vector([0.8, 0.8, 0.0, 0.0]));
            labels.push(Some(0));
            energies.push(100.0);
        }
        // Cluster 1: specialists (only photo)
        for _ in 0..5 {
            traits.push(make_trait_vector([0.9, 0.0, 0.0, 0.0]));
            labels.push(Some(1));
            energies.push(50.0);
        }
        assert!(is_generalist_dominant(
            &traits, &labels, &energies, 0.3, 0.5
        ));
    }

    #[test]
    fn generalist_not_dominant_when_specialists_dominate() {
        let mut traits = Vec::new();
        let mut labels = Vec::new();
        let mut energies = Vec::new();
        // Cluster 0: producers (specialist)
        for _ in 0..10 {
            traits.push(make_trait_vector([0.9, 0.0, 0.0, 0.0]));
            labels.push(Some(0));
            energies.push(100.0);
        }
        // Cluster 1: consumers (specialist)
        for _ in 0..5 {
            traits.push(make_trait_vector([0.0, 0.9, 0.0, 0.0]));
            labels.push(Some(1));
            energies.push(50.0);
        }
        assert!(!is_generalist_dominant(
            &traits, &labels, &energies, 0.3, 0.5
        ));
    }

    #[test]
    fn dip_statistic_low_for_uniform_distribution() {
        let n = 200;
        let data: Vec<f32> = (0..n).map(|i| i as f32 / n as f32).collect();
        let dip = dip_statistic(&data);
        assert!(dip < 0.02, "uniform data should have low dip: {dip}");
    }

    #[test]
    fn dip_statistic_high_for_bimodal_distribution() {
        let mut data: Vec<f32> = Vec::new();
        for i in 0..100 {
            data.push(i as f32 / 100.0 * 0.2);
        }
        for i in 0..100 {
            data.push(0.8 + i as f32 / 100.0 * 0.2);
        }
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let dip = dip_statistic(&data);
        assert!(dip > 0.05, "bimodal data should have high dip: {dip}");
    }

    #[test]
    fn fitness_is_weighted_sum_of_five_criteria() {
        let os = 0.8_f32;
        let cs = 0.6;
        let cd = 0.7;
        let ts = 0.5;
        let tb = 0.9;
        let expected = 0.2 * os + 0.2 * cs + 0.2 * cd + 0.2 * ts + 0.2 * tb;
        let result = FitnessBreakdown {
            fitness: expected,
            failure: None,
            oscillation_strength: os,
            clustering_strength: cs,
            coexistence_duration: cd,
            turnover_score: ts,
            trophic_balance_score: tb,
            ticks_survived: 100,
            carcass_locked_fraction: 0.0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        };
        assert!((result.fitness - expected).abs() < 1e-5);
    }

    #[test]
    fn turnover_score_zero_when_no_births_or_deaths() {
        assert_eq!(turnover_score(0, 0, 100), 0.0);
        assert_eq!(turnover_score(5, 0, 100), 0.0);
        assert_eq!(turnover_score(0, 5, 100), 0.0);
    }

    #[test]
    fn turnover_score_increases_with_more_turnover() {
        let low = turnover_score(10, 10, 100);
        let high = turnover_score(50, 50, 100);
        assert!(low > 0.0);
        assert!(high > low);
    }

    #[test]
    fn turnover_score_clamps_to_one() {
        let score = turnover_score(200, 200, 100);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn trophic_balance_high_when_producers_dominate() {
        let mut traits = Vec::new();
        let mut labels = Vec::new();
        let mut energies = Vec::new();
        for _ in 0..10 {
            traits.push(make_trait_vector([0.9, 0.1, 0.0, 0.0]));
            labels.push(Some(0));
            energies.push(100.0);
        }
        for _ in 0..5 {
            traits.push(make_trait_vector([0.1, 0.9, 0.0, 0.0]));
            labels.push(Some(1));
            energies.push(50.0);
        }
        let score = trophic_balance_score(&traits, &labels, &energies);
        assert!(
            score > 0.5,
            "producers dominating should score > 0.5: {score}"
        );
    }

    #[test]
    fn trophic_balance_low_when_consumers_dominate() {
        let mut traits = Vec::new();
        let mut labels = Vec::new();
        let mut energies = Vec::new();
        for _ in 0..5 {
            traits.push(make_trait_vector([0.9, 0.1, 0.0, 0.0]));
            labels.push(Some(0));
            energies.push(10.0);
        }
        for _ in 0..10 {
            traits.push(make_trait_vector([0.1, 0.9, 0.0, 0.0]));
            labels.push(Some(1));
            energies.push(100.0);
        }
        let score = trophic_balance_score(&traits, &labels, &energies);
        assert!(
            score < 0.5,
            "consumers dominating should score < 0.5: {score}"
        );
    }

    #[test]
    fn trophic_balance_zero_when_no_labelled_clusters() {
        let traits = vec![make_trait_vector([0.5, 0.5, 0.0, 0.0])];
        let labels = vec![None];
        let energies = vec![100.0];
        let score = trophic_balance_score(&traits, &labels, &energies);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn weighted_sum_of_five_equal_values() {
        let breakdown = FitnessBreakdown {
            fitness: 0.5,
            failure: None,
            oscillation_strength: 0.5,
            clustering_strength: 0.5,
            coexistence_duration: 0.5,
            turnover_score: 0.5,
            trophic_balance_score: 0.5,
            ticks_survived: 100,
            carcass_locked_fraction: 0.0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        };
        assert!((breakdown.fitness - 0.5).abs() < 1e-5);
    }

    #[test]
    fn fitness_breakdown_includes_all_five_criteria() {
        let breakdown = FitnessBreakdown {
            fitness: 0.0,
            failure: None,
            oscillation_strength: 0.1,
            clustering_strength: 0.2,
            coexistence_duration: 0.3,
            turnover_score: 0.4,
            trophic_balance_score: 0.5,
            ticks_survived: 50,
            carcass_locked_fraction: 0.0,
            has_decomposer_guild: false,
            has_consumer_guild: false,
        };
        assert_eq!(breakdown.oscillation_strength, 0.1);
        assert_eq!(breakdown.clustering_strength, 0.2);
        assert_eq!(breakdown.coexistence_duration, 0.3);
        assert_eq!(breakdown.turnover_score, 0.4);
        assert_eq!(breakdown.trophic_balance_score, 0.5);
        assert_eq!(breakdown.ticks_survived, 50);
    }

    #[test]
    fn grace_is_the_measured_provisioning_transient_of_two_hundred_and_sixty_ticks() {
        // The gates' reference prefix is sized from the founder provisioning
        // transient, an ecological constant — not a fraction of the horizon
        // (genesis-search.md). 260 is the measured value: the 90th percentile
        // of the provisioning-transient tick over the 1357 live runs of the
        // #505 sweep (253), rounded up to the nearest 10
        // (docs/research/505-settling-time.md).
        let config = EvalConfig::default();
        assert_eq!(config.grace_ticks, 260u64);
    }
}
