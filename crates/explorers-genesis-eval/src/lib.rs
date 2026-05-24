#[derive(Debug, Clone, PartialEq)]
pub enum FailureMode {
    Extinction,
    PopulationExplosion,
    EnergyDeath,
    FrozenDynamics,
    Monoculture,
    GeneralistDominance,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SanityCheckFailure {
    NoDemographicTurnover,
    NoTrophicPyramid,
}

#[derive(Debug, Clone)]
pub struct FitnessBreakdown {
    pub fitness: f32,
    pub failure: Option<FailureMode>,
    pub sanity_check_failed: Option<SanityCheckFailure>,
    pub oscillation_strength: f32,
    pub clustering_strength: f32,
    pub coexistence_duration: f32,
}

#[derive(Clone, Debug)]
pub struct EvalConfig {
    pub max_population: usize,
    pub energy_death_window: usize,
    pub frozen_dynamics_window: usize,
    pub clustering_threshold: f32,
    pub dbscan_eps: f32,
    pub dbscan_min_points: usize,
    pub generalist_threshold: f32,
    pub generalist_dominance_fraction: f32,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            max_population: 1000,
            energy_death_window: 50,
            frozen_dynamics_window: 50,
            clustering_threshold: 0.5,
            dbscan_eps: 1.0,
            dbscan_min_points: 5,
            generalist_threshold: 0.3,
            generalist_dominance_fraction: 0.5,
        }
    }
}

pub struct RunObserver {
    config: EvalConfig,
    energy_history: Vec<f32>,
    birth_history: Vec<usize>,
    death_history: Vec<usize>,
    total_births: usize,
    total_deaths: usize,
    cluster_counts: Vec<usize>,
    cluster_populations: Vec<Vec<usize>>,
    last_trait_vectors: Vec<explorers_sim::TraitVector>,
    last_labels: Vec<Option<usize>>,
    last_energies: Vec<f32>,
    failed: Option<FailureMode>,
}

impl RunObserver {
    pub fn new(config: EvalConfig) -> Self {
        Self {
            config,
            energy_history: Vec::new(),
            birth_history: Vec::new(),
            death_history: Vec::new(),
            total_births: 0,
            total_deaths: 0,
            cluster_counts: Vec::new(),
            cluster_populations: Vec::new(),
            last_trait_vectors: Vec::new(),
            last_labels: Vec::new(),
            last_energies: Vec::new(),
            failed: None,
        }
    }

    pub fn failed(&self) -> Option<&FailureMode> {
        self.failed.as_ref()
    }

    pub fn observe(&mut self, world: &explorers_sim::World) {
        if self.failed.is_some() {
            return;
        }

        let agents = world.agents();
        let agent_count = agents.len();

        if is_extinct(agent_count) {
            self.failed = Some(FailureMode::Extinction);
            return;
        }
        if is_population_explosion(agent_count, self.config.max_population) {
            self.failed = Some(FailureMode::PopulationExplosion);
            return;
        }

        let total_energy: f32 = agents.iter().map(|a| a.energy).sum();
        self.energy_history.push(total_energy);
        self.birth_history.push(world.last_tick_births());
        self.death_history.push(world.last_tick_deaths());
        self.total_births += world.last_tick_births();
        self.total_deaths += world.last_tick_deaths();

        if is_energy_death(&self.energy_history, self.config.energy_death_window) {
            self.failed = Some(FailureMode::EnergyDeath);
            return;
        }
        if is_frozen_dynamics(
            &self.birth_history,
            &self.death_history,
            self.config.frozen_dynamics_window,
        ) {
            self.failed = Some(FailureMode::FrozenDynamics);
            return;
        }

        let trait_vectors: Vec<_> = agents.iter().map(|a| a.traits).collect();
        let energies: Vec<_> = agents.iter().map(|a| a.energy).collect();

        if agent_count >= 20 {
            if is_monoculture(&trait_vectors, self.config.clustering_threshold) {
                self.failed = Some(FailureMode::Monoculture);
                return;
            }

            let labels = dbscan(
                &trait_vectors,
                self.config.dbscan_eps,
                self.config.dbscan_min_points,
            );
            let num_clusters = labels
                .iter()
                .filter_map(|l| *l)
                .collect::<std::collections::HashSet<_>>()
                .len();
            self.cluster_counts.push(num_clusters);

            // Track per-cluster populations
            if num_clusters > 0 {
                while self.cluster_populations.len() < num_clusters {
                    let backfill = vec![0; self.cluster_counts.len() - 1];
                    self.cluster_populations.push(backfill);
                }
                for (cid, pop) in self.cluster_populations.iter_mut().enumerate() {
                    let count = labels.iter().filter(|l| **l == Some(cid)).count();
                    pop.push(count);
                }
            }

            if is_generalist_dominant(
                &trait_vectors,
                &labels,
                &energies,
                self.config.generalist_threshold,
                self.config.generalist_dominance_fraction,
            ) {
                self.failed = Some(FailureMode::GeneralistDominance);
                return;
            }

            self.last_labels = labels;
        } else {
            self.cluster_counts.push(0);
        }

        self.last_trait_vectors = trait_vectors;
        self.last_energies = energies;
    }

    pub fn evaluate(&self) -> FitnessBreakdown {
        if let Some(failure) = &self.failed {
            return FitnessBreakdown {
                fitness: 0.0,
                failure: Some(failure.clone()),
                sanity_check_failed: None,
                oscillation_strength: 0.0,
                clustering_strength: 0.0,
                coexistence_duration: 0.0,
            };
        }

        let cs = if self.last_trait_vectors.len() >= 4 {
            clustering_strength(&self.last_trait_vectors)
        } else {
            0.0
        };

        let os = oscillation_strength(&self.cluster_populations);
        let cd = coexistence_duration(&self.cluster_counts);

        if !has_demographic_turnover(self.total_births, self.total_deaths) {
            return FitnessBreakdown {
                fitness: 0.0,
                failure: None,
                sanity_check_failed: Some(SanityCheckFailure::NoDemographicTurnover),
                oscillation_strength: os,
                clustering_strength: cs,
                coexistence_duration: cd,
            };
        }

        if !self.last_labels.is_empty()
            && !has_trophic_pyramid(&self.last_trait_vectors, &self.last_labels, &self.last_energies)
        {
            return FitnessBreakdown {
                fitness: 0.0,
                failure: None,
                sanity_check_failed: Some(SanityCheckFailure::NoTrophicPyramid),
                oscillation_strength: os,
                clustering_strength: cs,
                coexistence_duration: cd,
            };
        }

        FitnessBreakdown {
            fitness: os * cs * cd,
            failure: None,
            sanity_check_failed: None,
            oscillation_strength: os,
            clustering_strength: cs,
            coexistence_duration: cd,
        }
    }
}

pub fn is_extinct(agent_count: usize) -> bool {
    agent_count == 0
}

pub fn is_population_explosion(agent_count: usize, ceiling: usize) -> bool {
    agent_count > ceiling
}

pub fn is_frozen_dynamics(birth_history: &[usize], death_history: &[usize], window: usize) -> bool {
    if birth_history.len() < window || death_history.len() < window {
        return false;
    }
    let b_tail = &birth_history[birth_history.len() - window..];
    let d_tail = &death_history[death_history.len() - window..];
    b_tail.iter().all(|&b| b == 0) && d_tail.iter().all(|&d| d == 0)
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

pub fn trophic_coordinates(traits: &explorers_sim::TraitVector) -> (f32, f32, f32) {
    let sum = traits.photosynthetic_absorption + traits.consumption_rate + traits.scavenging_rate;
    if sum <= 0.0 {
        return (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0);
    }
    (
        traits.photosynthetic_absorption / sum,
        traits.consumption_rate / sum,
        traits.scavenging_rate / sum,
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
        let mut avg_cons = 0.0_f32;
        let mut avg_scav = 0.0_f32;
        for &i in &members {
            let (p, c, s) = trophic_coordinates(&trait_vectors[i]);
            avg_photo += p;
            avg_cons += c;
            avg_scav += s;
        }
        let n = members.len() as f32;
        avg_photo /= n;
        avg_cons /= n;
        avg_scav /= n;

        let is_generalist = avg_photo > generalist_threshold
            && avg_cons > generalist_threshold
            && avg_scav > generalist_threshold;

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

pub fn oscillation_strength(cluster_populations: &[Vec<usize>]) -> f32 {
    if cluster_populations.is_empty() {
        return 0.0;
    }
    let mut total = 0.0_f32;
    let mut count = 0;
    for pop in cluster_populations {
        if pop.len() < 4 {
            continue;
        }
        let series: Vec<f32> = pop.iter().map(|&x| x as f32).collect();
        let max_lag = pop.len() / 2;
        let mut max_ac = 0.0_f32;
        for lag in 1..=max_lag {
            let ac = autocorrelation(&series, lag);
            max_ac = max_ac.max(ac);
        }
        total += max_ac;
        count += 1;
    }
    if count == 0 {
        return 0.0;
    }
    (total / count as f32).clamp(0.0, 1.0)
}

pub fn has_demographic_turnover(total_births: usize, total_deaths: usize) -> bool {
    total_births > 0 && total_deaths > 0
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
        let mut avg_cons = 0.0_f32;
        for &i in &members {
            let (p, c, _) = trophic_coordinates(&trait_vectors[i]);
            avg_photo += p;
            avg_cons += c;
        }
        let n = members.len() as f32;
        avg_photo /= n;
        avg_cons /= n;

        let cluster_energy: f32 = members.iter().map(|&i| energies[i]).sum();
        if avg_photo > avg_cons {
            producer_energy += cluster_energy;
        } else {
            consumer_energy += cluster_energy;
        }
    }

    producer_energy > consumer_energy
}

pub fn coexistence_duration(cluster_counts_per_tick: &[usize]) -> f32 {
    if cluster_counts_per_tick.is_empty() {
        return 0.0;
    }
    let coexisting = cluster_counts_per_tick.iter().filter(|&&c| c >= 2).count();
    coexisting as f32 / cluster_counts_per_tick.len() as f32
}

pub fn dbscan(trait_vectors: &[explorers_sim::TraitVector], eps: f32, min_points: usize) -> Vec<Option<usize>> {
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

pub fn is_energy_death(energy_history: &[f32], window: usize) -> bool {
    if energy_history.len() < window {
        return false;
    }
    let tail = &energy_history[energy_history.len() - window..];
    tail.windows(2).all(|w| w[1] < w[0])
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn energy_death_when_monotonically_decreasing() {
        let history = vec![100.0, 90.0, 80.0, 70.0, 60.0];
        assert!(is_energy_death(&history, 5));
    }

    #[test]
    fn no_energy_death_when_energy_recovers() {
        let history = vec![100.0, 90.0, 80.0, 85.0, 90.0];
        assert!(!is_energy_death(&history, 5));
    }

    #[test]
    fn no_energy_death_when_history_shorter_than_window() {
        let history = vec![100.0, 90.0];
        assert!(!is_energy_death(&history, 5));
    }

    #[test]
    fn frozen_when_no_births_and_no_deaths() {
        let births = vec![0, 0, 0, 0, 0];
        let deaths = vec![0, 0, 0, 0, 0];
        assert!(is_frozen_dynamics(&births, &deaths, 5));
    }

    #[test]
    fn not_frozen_when_births_occur() {
        let births = vec![0, 0, 1, 0, 0];
        let deaths = vec![0, 0, 0, 0, 0];
        assert!(!is_frozen_dynamics(&births, &deaths, 5));
    }

    #[test]
    fn not_frozen_when_deaths_occur() {
        let births = vec![0, 0, 0, 0, 0];
        let deaths = vec![0, 0, 0, 1, 0];
        assert!(!is_frozen_dynamics(&births, &deaths, 5));
    }

    #[test]
    fn not_frozen_when_history_shorter_than_window() {
        let births = vec![0, 0];
        let deaths = vec![0, 0];
        assert!(!is_frozen_dynamics(&births, &deaths, 5));
    }

    fn make_trait_vector(vals: [f32; 8]) -> explorers_sim::TraitVector {
        explorers_sim::TraitVector {
            photosynthetic_absorption: vals[0],
            consumption_rate: vals[1],
            scavenging_rate: vals[2],
            mobility: vals[3],
            chemotaxis_sensitivity: vals[4],
            mate_selectivity: vals[5],
            sensing_range: vals[6],
            reproductive_investment: vals[7],
        }
    }

    #[test]
    fn clustering_strength_high_for_bimodal_traits() {
        let mut traits = Vec::new();
        for i in 0..50 {
            traits.push(make_trait_vector([i as f32 * 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
        }
        for i in 0..50 {
            traits.push(make_trait_vector([5.0 + i as f32 * 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
        }
        let strength = clustering_strength(&traits);
        assert!(strength > 0.5, "bimodal traits should have high clustering strength: {strength}");
    }

    #[test]
    fn clustering_strength_low_for_unimodal_traits() {
        use rand::SeedableRng;
        use rand_distr::{Distribution, Normal};
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);
        let dist = Normal::new(0.5_f32, 0.2).unwrap();
        let traits: Vec<_> = (0..100)
            .map(|_| make_trait_vector([dist.sample(&mut rng), dist.sample(&mut rng), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]))
            .collect();
        let strength = clustering_strength(&traits);
        assert!(strength < 0.5, "unimodal traits should have low clustering strength: {strength}");
    }

    #[test]
    fn monoculture_detected_for_unimodal_traits() {
        use rand::SeedableRng;
        use rand_distr::{Distribution, Normal};
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);
        let dist = Normal::new(0.5_f32, 0.2).unwrap();
        let traits: Vec<_> = (0..100)
            .map(|_| make_trait_vector([dist.sample(&mut rng), dist.sample(&mut rng), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]))
            .collect();
        assert!(is_monoculture(&traits, 0.5));
    }

    #[test]
    fn dbscan_finds_two_clusters() {
        let mut traits = Vec::new();
        for i in 0..10 {
            traits.push(make_trait_vector([i as f32 * 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
        }
        for i in 0..10 {
            traits.push(make_trait_vector([5.0 + i as f32 * 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
        }
        let labels = dbscan(&traits, 0.5, 3);
        let cluster_ids: std::collections::HashSet<_> = labels.iter().filter_map(|l| *l).collect();
        assert_eq!(cluster_ids.len(), 2, "should find 2 clusters, got {cluster_ids:?}");
    }

    #[test]
    fn dbscan_uniform_scatter_gives_no_clusters() {
        let traits: Vec<_> = (0..10)
            .map(|i| make_trait_vector([i as f32 * 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]))
            .collect();
        let labels = dbscan(&traits, 0.5, 3);
        let cluster_count = labels.iter().filter_map(|l| *l).collect::<std::collections::HashSet<_>>().len();
        assert!(cluster_count <= 1, "widely scattered points should have 0-1 clusters, got {cluster_count}");
    }

    #[test]
    fn dbscan_noise_points_are_none() {
        let mut traits = Vec::new();
        for i in 0..10 {
            traits.push(make_trait_vector([i as f32 * 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
        }
        // Add an outlier far away
        traits.push(make_trait_vector([100.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
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
            traits.push(make_trait_vector([0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
            labels.push(Some(0));
            energies.push(100.0);
        }
        // Consumers (low photosynthesis, high consumption)
        for _ in 0..5 {
            traits.push(make_trait_vector([0.1, 0.9, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
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
            traits.push(make_trait_vector([0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
            labels.push(Some(0));
            energies.push(10.0);
        }
        // Consumers with lots of energy (inverted pyramid)
        for _ in 0..10 {
            traits.push(make_trait_vector([0.1, 0.9, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
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
        assert!(ac > 0.8, "sinusoidal series at lag=period should have high autocorrelation: {ac}");
    }

    #[test]
    fn autocorrelation_near_zero_for_flat_series() {
        let series = vec![5.0; 100];
        let ac = autocorrelation(&series, 10);
        assert!(ac.abs() < 0.01, "flat series should have ~0 autocorrelation: {ac}");
    }

    #[test]
    fn oscillation_strength_high_for_oscillating_populations() {
        let period = 20;
        let n = 200;
        // Two clusters oscillating out of phase
        let cluster_0: Vec<usize> = (0..n)
            .map(|i| (50.0 + 30.0 * (2.0 * std::f32::consts::PI * i as f32 / period as f32).sin()) as usize)
            .collect();
        let cluster_1: Vec<usize> = (0..n)
            .map(|i| (50.0 + 30.0 * (2.0 * std::f32::consts::PI * i as f32 / period as f32 + std::f32::consts::PI).sin()) as usize)
            .collect();
        let strength = oscillation_strength(&[cluster_0, cluster_1]);
        assert!(strength > 0.5, "oscillating populations should have high oscillation strength: {strength}");
    }

    #[test]
    fn oscillation_strength_low_for_flat_populations() {
        let cluster_0 = vec![50; 100];
        let cluster_1 = vec![30; 100];
        let strength = oscillation_strength(&[cluster_0, cluster_1]);
        assert!(strength < 0.1, "flat populations should have low oscillation strength: {strength}");
    }

    #[test]
    fn trophic_coordinates_pure_producer() {
        let traits = make_trait_vector([1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let (photo, cons, scav) = trophic_coordinates(&traits);
        assert!((photo - 1.0).abs() < 1e-5);
        assert!(cons.abs() < 1e-5);
        assert!(scav.abs() < 1e-5);
    }

    #[test]
    fn trophic_coordinates_mixed() {
        let traits = make_trait_vector([0.3, 0.3, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let (photo, cons, scav) = trophic_coordinates(&traits);
        let sum = photo + cons + scav;
        assert!((sum - 1.0).abs() < 1e-5, "should sum to 1: {sum}");
        assert!((photo - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn trophic_coordinates_zero_energy_traits() {
        let traits = make_trait_vector([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
        let (photo, _cons, _scav) = trophic_coordinates(&traits);
        assert!((photo - 1.0 / 3.0).abs() < 0.01, "should default to equal: {photo}");
    }

    #[test]
    fn generalist_dominant_when_one_cluster_has_high_all_traits() {
        // Cluster 0: generalists (high photo, consumption, scavenging)
        let mut traits = Vec::new();
        let mut labels = Vec::new();
        let mut energies = Vec::new();
        for _ in 0..10 {
            traits.push(make_trait_vector([0.8, 0.8, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0]));
            labels.push(Some(0));
            energies.push(100.0);
        }
        // Cluster 1: specialists (only photo)
        for _ in 0..5 {
            traits.push(make_trait_vector([0.9, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
            labels.push(Some(1));
            energies.push(50.0);
        }
        assert!(is_generalist_dominant(&traits, &labels, &energies, 0.3, 0.5));
    }

    #[test]
    fn generalist_not_dominant_when_specialists_dominate() {
        let mut traits = Vec::new();
        let mut labels = Vec::new();
        let mut energies = Vec::new();
        // Cluster 0: producers (specialist)
        for _ in 0..10 {
            traits.push(make_trait_vector([0.9, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
            labels.push(Some(0));
            energies.push(100.0);
        }
        // Cluster 1: consumers (specialist)
        for _ in 0..5 {
            traits.push(make_trait_vector([0.0, 0.9, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
            labels.push(Some(1));
            energies.push(50.0);
        }
        assert!(!is_generalist_dominant(&traits, &labels, &energies, 0.3, 0.5));
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
    fn fitness_zero_on_extinction() {
        let config = EvalConfig::default();
        let mut observer = RunObserver::new(config);
        let params = explorers_sim::WorldParameters {
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 100.0,
            sensing_cost_coefficient: 0.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            reproduction_efficiency: 0.7,
            movement_cost_coefficient: 0.0,
            reproduction_energy_threshold: 50.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_radius: 5.0,
            world_extent: 100.0,
            initial_population_size: 1,
        };
        let dist = explorers_sim::InitialDistribution {
            mean_traits: explorers_sim::TraitVector {
                photosynthetic_absorption: 0.0,
                consumption_rate: 0.0,
                scavenging_rate: 0.0,
                mobility: 0.0,
                chemotaxis_sensitivity: 0.0,
                mate_selectivity: 0.0,
                sensing_range: 0.0,
                reproductive_investment: 0.0,
            },
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = explorers_sim::World::new(params, dist, 42);
        for _ in 0..10 {
            world.step();
            observer.observe(&world);
        }
        let result = observer.evaluate();
        assert_eq!(result.fitness, 0.0);
        assert_eq!(result.failure, Some(FailureMode::Extinction));
    }

    #[test]
    fn fitness_zero_on_sanity_check_failure() {
        let result = FitnessBreakdown {
            fitness: 0.0,
            failure: None,
            sanity_check_failed: Some(SanityCheckFailure::NoDemographicTurnover),
            oscillation_strength: 0.5,
            clustering_strength: 0.5,
            coexistence_duration: 0.5,
        };
        assert_eq!(result.fitness, 0.0);
    }

    #[test]
    fn fitness_is_product_of_three_criteria() {
        let os = 0.8;
        let cs = 0.6;
        let cd = 0.7;
        let expected = os * cs * cd;
        let result = FitnessBreakdown {
            fitness: expected,
            failure: None,
            sanity_check_failed: None,
            oscillation_strength: os,
            clustering_strength: cs,
            coexistence_duration: cd,
        };
        assert!((result.fitness - expected).abs() < 1e-5);
    }

    #[test]
    fn observer_detects_failure_and_stops_observing() {
        let config = EvalConfig {
            max_population: 5,
            ..EvalConfig::default()
        };
        let mut observer = RunObserver::new(config);
        let params = explorers_sim::WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.01,
            sensing_cost_coefficient: 0.0,
            consumption_efficiency: 0.5,
            decomposition_efficiency: 0.5,
            reproduction_efficiency: 0.9,
            movement_cost_coefficient: 0.0,
            reproduction_energy_threshold: 5.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_radius: 100.0,
            world_extent: 10.0,
            initial_population_size: 6,
        };
        let dist = explorers_sim::InitialDistribution {
            mean_traits: explorers_sim::TraitVector {
                photosynthetic_absorption: 1.0,
                consumption_rate: 0.0,
                scavenging_rate: 0.0,
                mobility: 0.0,
                chemotaxis_sensitivity: 0.0,
                mate_selectivity: 100.0,
                sensing_range: 0.0,
                reproductive_investment: 5.0,
            },
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        };
        let world = explorers_sim::World::new(params, dist, 42);
        observer.observe(&world);
        let result = observer.evaluate();
        assert_eq!(result.failure, Some(FailureMode::PopulationExplosion));
        assert_eq!(result.fitness, 0.0);
    }
}
