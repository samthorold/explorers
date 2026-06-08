pub mod ensemble;

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
    /// boundary): whether a persistent decomposer guild was present, read off the
    /// full-log `TopologyProjection` the evaluator already builds. A reported
    /// observable — never a behaviour axis, never a fitness term. The atlas
    /// aggregates it across a cell's seed ensemble into a *fraction of seeds*,
    /// honouring the existence-vs-distributional boundary. False on the gated
    /// path.
    pub has_decomposer_guild: bool,
}

#[derive(Clone, Debug)]
pub struct EvalConfig {
    pub max_population: usize,
    pub energy_death_window: usize,
    pub nutrient_lock_window: usize,
    /// Tick interval at which the rollout snapshots living-population trait vectors
    /// for the coexistence descriptor (issue #394). Coarse, not per-tick: DBSCAN is
    /// O(n²) and `max_population` is large, so clustering every tick of every seed
    /// is disproportionate for one noisy 0.2-weight term. A co-presence *fraction*
    /// needs only a representative sample of the post-grace window.
    pub coexistence_sample_interval: usize,
    pub clustering_threshold: f32,
    pub dbscan_eps: f32,
    pub dbscan_min_points: usize,
    pub generalist_threshold: f32,
    pub generalist_dominance_fraction: f32,
    pub grace_period_fraction: f32,
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
            grace_period_fraction: 0.2,
        }
    }
}

/// The per-tick series a rollout observes about a world, sampled over time and
/// handed to the evaluator as one bundle. Each field is one signal the caller
/// samples once per tick (or, for `cluster_snapshots`, at a coarse interval)
/// during the rollout; the evaluator reads them to compute the descriptors that
/// need a temporal trace rather than just the final state. Bundling them keeps
/// `evaluate_from_log`'s signature from accreting a new slice parameter every
/// time a descriptor grows a per-tick appetite — the series are conceptually one
/// thing: what the rollout observed, sampled over time.
#[derive(Clone, Debug, Default)]
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
}

pub fn evaluate_from_log(
    world: &explorers_sim::World,
    observations: &RolloutObservations,
    config: &EvalConfig,
    max_ticks: u64,
) -> FitnessBreakdown {
    let RolloutObservations {
        free_energy: free_energy_per_tick,
        carcass_fraction: carcass_fraction_per_tick,
        producer_share: producer_share_per_tick,
        cluster_snapshots,
    } = observations;
    let agents = world.agents();
    let ticks_survived = world.tick();

    let zero_breakdown = |failure: FailureMode| FitnessBreakdown {
        fitness: 0.0,
        failure: Some(failure),
        oscillation_strength: 0.0,
        clustering_strength: 0.0,
        coexistence_duration: 0.0,
        turnover_score: 0.0,
        trophic_balance_score: 0.0,
        ticks_survived,
        // A degenerate world has no meaningful behaviour coordinate (it is routed
        // to the dead frontier by cliff, not binned), so the descriptors are zero.
        carcass_locked_fraction: 0.0,
        has_decomposer_guild: false,
    };

    if agents.is_empty() {
        return zero_breakdown(FailureMode::Extinction);
    }

    if is_population_explosion(agents.len(), config.max_population) {
        return zero_breakdown(FailureMode::PopulationExplosion);
    }

    let log = world.event_log();
    let total_births = log
        .by_kind(&explorers_sim::event::EventKind::Reproduced)
        .len();
    let total_deaths = log.by_kind(&explorers_sim::event::EventKind::Died).len();
    let ts = turnover_score(total_births, total_deaths, max_ticks);

    let trait_vectors: Vec<_> = agents.iter().map(|a| a.traits).collect();
    let energies: Vec<_> = agents.iter().map(|a| a.energy()).collect();

    let cs = if trait_vectors.len() >= 4 {
        clustering_strength(&trait_vectors)
    } else {
        0.0
    };

    let labels = dbscan(&trait_vectors, config.dbscan_eps, config.dbscan_min_points);
    let tb = trophic_balance_score(&trait_vectors, &labels, &energies);

    let grace_ticks = (max_ticks as f32 * config.grace_period_fraction) as u64;
    if ticks_survived > grace_ticks {
        // Free (non-carcass-locked) energy stock, sampled once per tick by the
        // caller. Energy death is this living-system stock trending irreversibly
        // toward zero as energy locks into carcasses — a stock trend, not the
        // predation flow the old detector summed (issue #302). The grace prefix
        // is dropped so early transients before the world settles don't count.
        let post_grace: Vec<f32> = free_energy_per_tick
            .iter()
            .copied()
            .skip(grace_ticks as usize)
            .collect();
        if is_free_energy_dead(&post_grace, config.energy_death_window) {
            return zero_breakdown(FailureMode::EnergyDeath);
        }

        // Carcass-locked nutrient fraction, sampled once per tick by the caller.
        // Nutrient lockup is the dead pool's share trending high and staying
        // there — nutrient sequestered into carcasses the living decomposers
        // cannot turn over (issue #342). The nutrient-side sibling of energy
        // death, checked after it: a world can photosynthesise fine while its
        // nutrient irreversibly silts up the dead pool. Same grace prefix drop.
        let post_grace_nutrient: Vec<f32> = carcass_fraction_per_tick
            .iter()
            .copied()
            .skip(grace_ticks as usize)
            .collect();
        if is_nutrient_locked(&post_grace_nutrient, config.nutrient_lock_window) {
            return zero_breakdown(FailureMode::NutrientLockup);
        }
    }

    if ticks_survived > grace_ticks && trait_vectors.len() >= 20 {
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

    // The full-log topology projection is still required for the decomposer-guild
    // signal below; the lineage-clustering machinery that once fed coexistence is
    // gone (issue #394) — coexistence now reads trait-space clusters, not clades.
    let mut topo = explorers_sim::topology::TopologyProjection::new();
    topo.update(log);

    // Coexistence (issue #394): the fraction of post-grace sampled ticks whose
    // living population carries >=2 trait-space DBSCAN clusters. Genesis snapshots
    // raw trait vectors at a coarse interval through the rollout (pure observation,
    // no clustering); the evaluator owns all clustering, running DBSCAN on each
    // post-grace snapshot here. Each snapshot's tick is stored so the grace cutoff
    // stays index/interval-free. Seed-invariant by construction: a pure function of
    // the snapshots and the DBSCAN config, with no `initial_population_size` leak.
    let cluster_counts_per_snapshot: Vec<usize> = cluster_snapshots
        .iter()
        .filter(|(tick, _)| *tick > grace_ticks)
        .map(|(_, traits)| {
            distinct_cluster_count(traits, config.dbscan_eps, config.dbscan_min_points)
        })
        .collect();

    // Oscillation: the producer↔consumer rhythm read off the per-tick producer-
    // energy-share series the caller sampled (issue #392). Drop the grace prefix
    // (same skip the energy/nutrient gates and clustering use) so the early
    // colonization transient doesn't count, then measure over the full post-grace
    // window — a slow ecological cycle needs several periods, not a trailing tail.
    let post_grace_share: Vec<f32> = producer_share_per_tick
        .iter()
        .copied()
        .skip(grace_ticks as usize)
        .collect();
    let os = if ticks_survived > grace_ticks {
        oscillation_strength(&post_grace_share)
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

    // Decomposer-guild signal: read off the full-log topology projection the
    // evaluator already built. A persistent guild reads as ≥1 surviving agent
    // classified `Decomposer` by realised diet over the whole run. A reported
    // observable, aggregated into a per-cell seed fraction by the atlas — never an
    // axis, never a fitness term (the authority boundary, genesis-search.md).
    let has_decomposer_guild = topo
        .trophic_roles(agents)
        .values()
        .any(|&r| r == explorers_sim::topology::TrophicRole::Decomposer);

    FitnessBreakdown {
        fitness,
        failure: None,
        oscillation_strength: os,
        clustering_strength: cs,
        coexistence_duration: cd,
        turnover_score: ts,
        trophic_balance_score: tb,
        ticks_survived,
        carcass_locked_fraction,
        has_decomposer_guild,
    }
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

/// Energy death: free (non-carcass-locked) energy trends irreversibly toward
/// zero (expected-properties.md). `free_energy_per_tick` is the living-system
/// energy stock sampled once per tick — agent reserve + structure summed across
/// the population, i.e. energy NOT locked in carcasses.
///
/// The signal is a *stock trend*, not a flow: energy death is the living pool
/// collapsing as energy locks into carcasses faster than decomposition returns
/// it. We flag it when, over the trailing `window`, the free-energy stock has
/// collapsed to a small fraction of its earlier peak and does not recover — the
/// best the window manages stays far below the peak the system once held.
///
/// A living, reproducing or producer-fed world keeps regenerating free energy,
/// so its trailing window holds a substantial fraction of its peak and is not
/// flagged. A world whose living pool drains into carcasses sees the trailing
/// window sit near zero relative to the peak.
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
            grace_period_fraction: 1.0,
            ..EvalConfig::default()
        };
        let max_ticks = 50;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let free = run_collecting_free_energy(&mut world, max_ticks);
        let result = evaluate_from_log(&world, &obs(&free, &[], &[], &[]), &config, max_ticks);
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
            grace_period_fraction: 0.0,
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
            grace_period_fraction: 0.0,
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
            grace_period_fraction: 0.0,
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
            grace_period_fraction: 0.0,
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
    fn evaluate_from_log_coexistence_fraction_from_cluster_snapshots() {
        // Coexistence is the fraction of post-grace sampled ticks whose living
        // population carries >=2 trait-space clusters (issue #394). Feed a known
        // K of N post-grace snapshots that carry >=2 clusters and assert the
        // breakdown's coexistence_duration is exactly K/N. The fitness is the
        // weighted sum of the five components.
        let params = test_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_period_fraction: 0.0,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 50;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let free = run_collecting_free_energy(&mut world, max_ticks);
        if world.agents().is_empty() {
            return;
        }
        // 5 snapshots (all post-grace because grace_period_fraction = 0): 3 with
        // >=2 clusters, 2 with a single cluster → K/N = 3/5.
        let snapshots: Vec<(u64, Vec<explorers_sim::TraitVector>)> = vec![
            (1, snapshot_with_clusters(2)),
            (2, snapshot_with_clusters(1)),
            (3, snapshot_with_clusters(3)),
            (4, snapshot_with_clusters(1)),
            (5, snapshot_with_clusters(2)),
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

    #[test]
    fn coexistence_is_invariant_to_initial_population_size() {
        // Coexistence is now a pure function of the snapshots and DBSCAN config —
        // the searched seed parameter `initial_population_size` must NOT leak in
        // (issue #394, the leak #390/#392 removed from oscillation). Two worlds
        // built with different initial_population_size, fed identical snapshots,
        // must report identical coexistence_duration.
        let dist = test_distribution();
        let config = EvalConfig {
            grace_period_fraction: 0.0,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 50;

        let snapshots: Vec<(u64, Vec<explorers_sim::TraitVector>)> = vec![
            (1, snapshot_with_clusters(2)),
            (2, snapshot_with_clusters(1)),
            (3, snapshot_with_clusters(2)),
        ];

        let build = |pop: u32| {
            let params = explorers_sim::WorldParameters {
                initial_population_size: pop,
                ..test_world_params()
            };
            let mut world = explorers_sim::World::new(params, dist.clone(), 42);
            let free = run_collecting_free_energy(&mut world, max_ticks);
            (world, free)
        };

        let (world_a, free_a) = build(10);
        let (world_b, free_b) = build(80);
        if world_a.agents().is_empty() || world_b.agents().is_empty() {
            return;
        }
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
        assert_eq!(
            a.coexistence_duration, b.coexistence_duration,
            "coexistence must not depend on initial_population_size"
        );
    }

    #[test]
    fn coexistence_excludes_snapshots_at_or_before_grace() {
        // Only snapshots with tick > grace_ticks count. With grace_ticks = 10
        // (max_ticks=50, grace_fraction=0.2), the two pre-grace 2-cluster
        // snapshots are excluded; only the post-grace snapshots set the fraction.
        let params = test_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_period_fraction: 0.2,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 50;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let free = run_collecting_free_energy(&mut world, max_ticks);
        if world.agents().is_empty() || world.tick() <= 10 {
            return;
        }
        // grace_ticks = 10. Pre-grace snapshots (excluded) all carry 2 clusters;
        // post-grace snapshots are 1 of 2 with >=2 clusters → fraction = 1/2, not
        // dragged up by the excluded pre-grace ones.
        let snapshots: Vec<(u64, Vec<explorers_sim::TraitVector>)> = vec![
            (5, snapshot_with_clusters(2)),
            (10, snapshot_with_clusters(2)),
            (15, snapshot_with_clusters(2)),
            (20, snapshot_with_clusters(1)),
        ];
        let result = evaluate_from_log(
            &world,
            &obs(&free, &[], &[], &snapshots),
            &config,
            max_ticks,
        );
        assert!(
            (result.coexistence_duration - 0.5).abs() < 1e-5,
            "only post-grace snapshots count: expected 1/2, got {}",
            result.coexistence_duration
        );
    }

    #[test]
    fn coexistence_zero_when_survival_within_grace() {
        // Guard mirroring the oscillation guard (#392): if ticks_survived <=
        // grace_ticks, coexistence_duration is 0.0 regardless of snapshots.
        let params = test_world_params();
        let dist = test_distribution();
        // grace_period_fraction = 1.0 → grace_ticks = max_ticks, so a surviving
        // world never exceeds grace.
        let config = EvalConfig {
            grace_period_fraction: 1.0,
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
            grace_period_fraction: 0.0,
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
        // Oscillation now reads the per-tick producer-energy-share series the
        // caller samples (issue #392), over the post-grace window. A surviving
        // world fed an oscillating share series surfaces a positive oscillation
        // strength that matches the standalone descriptor on the post-grace slice;
        // a flat series surfaces ~0. Seed-invariant by construction — no lineage
        // clusters, no initial_population_size.
        let params = test_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_period_fraction: 0.2,
            ..EvalConfig::default()
        };
        let max_ticks: u64 = 64;
        let mut world = explorers_sim::World::new(params, dist, 42);
        let (free, _share) = run_collecting_free_energy_and_share(&mut world, max_ticks);
        if world.agents().is_empty() {
            return;
        }
        let n = world.tick() as usize;
        let grace_ticks = (max_ticks as f32 * config.grace_period_fraction) as u64;
        if (n as u64) <= grace_ticks {
            return;
        }

        // A synthetic oscillating producer-share series: a clean producer↔consumer
        // rhythm the evaluator should pick up over the post-grace window.
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
        let post_grace: Vec<f32> = oscillating
            .iter()
            .copied()
            .skip(grace_ticks as usize)
            .collect();
        assert_eq!(
            bd.oscillation_strength,
            oscillation_strength(&post_grace),
            "breakdown oscillation must be the descriptor over the post-grace slice"
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
    fn evaluate_from_log_clustering_and_trophic_from_final_state() {
        let params = test_world_params();
        let dist = test_distribution();
        let config = EvalConfig {
            grace_period_fraction: 1.0,
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
        };
        assert_eq!(breakdown.oscillation_strength, 0.1);
        assert_eq!(breakdown.clustering_strength, 0.2);
        assert_eq!(breakdown.coexistence_duration, 0.3);
        assert_eq!(breakdown.turnover_score, 0.4);
        assert_eq!(breakdown.trophic_balance_score, 0.5);
        assert_eq!(breakdown.ticks_survived, 50);
    }

    #[test]
    fn grace_period_defaults_to_twenty_percent() {
        let config = EvalConfig::default();
        assert!((config.grace_period_fraction - 0.2).abs() < 1e-5);
    }
}
