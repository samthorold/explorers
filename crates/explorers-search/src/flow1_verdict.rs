//! Flow 1's two verdicts on light-fed mixotrophy (#682), as
//! `docs/system-design/world-rules.md` flow 1 states them ("Conditionality is
//! read within worlds …"): **conditionality**, read within worlds over the
//! per-config Spearman ρ of producers' effective heterotrophy against the
//! pool nutrient under them on the settled half, and the **minority share**
//! of carcass structure drained, told apart by absolute drains when it fails.
//!
//! Pure statistics over numbers a readout has already reduced; the
//! `kin_killer_diet` example feeds it its rows and prints the verdicts.

/// Conditionality's second test (world-rules.md flow 1): the share of
/// configs with ρ < 0 differs from a coin's at two-sided binomial p below
/// this.
pub const COIN_ALPHA: f64 = 0.05;

/// Conditionality's third test (world-rules.md flow 1): the atlas's configs
/// grouped into this many Ward clusters on their unit coordinates, as a
/// proxy for the few emitter lineages its cells descend from (#663, #670
/// §7).
pub const LINEAGE_CLUSTERS: usize = 8;

/// The minority share (world-rules.md flow 1): light-fed mixotrophs drain a
/// minority of the settled half's carcass structure, strictly below this
/// share.
pub const MINORITY_BAR: f64 = 0.5;

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

/// The lineage clusters' read: clusters holding at least one config with a
/// ρ, and of them those whose median ρ is below zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClusterRead {
    pub with_median: usize,
    pub negative: usize,
}

/// Conditionality by world-rules.md flow 1, for one mode (decoded, or
/// `founder_aggregation = 0`): three tests on the per-config ρ, all of which
/// must hold, and the fallback's pooled ρ.
#[derive(Clone, Debug, PartialEq)]
pub struct Conditionality {
    /// Spearman ρ over every config's samples in one ranking (the
    /// fallback's "does not run backwards").
    pub pooled: Option<f64>,
    /// Configs with a ρ, and of them those with ρ < 0.
    pub configs: usize,
    pub negative: usize,
    pub median: Option<f64>,
    /// Two-sided binomial p of `negative` of `configs` against a coin.
    pub binomial_p: f64,
    /// `None` when the configs carry no lineage clusters.
    pub clusters: Option<ClusterRead>,
}

impl Conditionality {
    /// `per_config`: each config's ρ (`None` where undefined); `clusters`:
    /// each config's lineage cluster, parallel to it.
    pub fn of(pooled: Option<f64>, per_config: &[Option<f64>], clusters: Option<&[usize]>) -> Self {
        let defined: Vec<f64> = per_config.iter().flatten().copied().collect();
        let negative = defined.iter().filter(|r| **r < 0.0).count();
        let clusters = clusters.map(|labels| {
            assert_eq!(labels.len(), per_config.len(), "one cluster per config");
            let mut by: std::collections::BTreeMap<usize, Vec<f64>> = Default::default();
            for (rho, &c) in per_config.iter().zip(labels) {
                if let Some(r) = rho {
                    by.entry(c).or_default().push(*r);
                }
            }
            let medians: Vec<f64> = by.into_values().filter_map(median).collect();
            ClusterRead {
                with_median: medians.len(),
                negative: medians.iter().filter(|m| **m < 0.0).count(),
            }
        });
        Self {
            pooled,
            configs: defined.len(),
            negative,
            binomial_p: binomial_two_sided_p(negative, defined.len()),
            median: median(defined),
            clusters,
        }
    }

    /// (1) The median per-config ρ is negative.
    pub fn median_negative(&self) -> Option<bool> {
        self.median.map(|m| m < 0.0)
    }

    /// (2) The configs with ρ < 0 are not a coin's share. `None` without
    /// configs.
    pub fn not_a_coin(&self) -> Option<bool> {
        (self.configs > 0).then(|| self.binomial_p < COIN_ALPHA)
    }

    /// (3) A strict majority of the lineage clusters have a negative median.
    pub fn clusters_negative(&self) -> Option<bool> {
        self.clusters
            .filter(|c| c.with_median > 0)
            .map(|c| 2 * c.negative > c.with_median)
    }

    /// All three hold. A test that fails decides it; otherwise an unread
    /// one leaves it `None`.
    pub fn passes(&self) -> Option<bool> {
        let tests = [
            self.median_negative(),
            self.not_a_coin(),
            self.clusters_negative(),
        ];
        if tests.contains(&Some(false)) {
            Some(false)
        } else if tests.iter().all(|t| *t == Some(true)) {
            Some(true)
        } else {
            None
        }
    }

    /// The fallback's reading (world-rules.md flow 1): heterotrophy does not
    /// run backwards, pooled ρ ≤ 0.
    pub fn does_not_run_backwards(&self) -> Option<bool> {
        self.pooled.map(|p| p <= 0.0)
    }
}

/// The minority share (world-rules.md flow 1): light-fed mixotrophs'
/// settled-half share of carcass structure drained is below
/// [`MINORITY_BAR`]. `None` when nothing was drained.
pub fn minority(share: Option<f64>) -> Option<bool> {
    share.map(|s| s < MINORITY_BAR)
}

/// Settled-half carcass structure drained per seed by light-fed
/// mixotrophs and by decomposers by role.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drains {
    pub mixotrophs: f64,
    pub decomposers: f64,
}

/// How a run's absolute drains moved against a baseline run's, which is
/// what tells a share's two failures apart (world-rules.md flow 1: "a share
/// is a ratio, so it can fail two ways").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrainShift {
    /// Mixotrophs drain more than the baseline's: the route around
    /// decomposition, which carcass access answers.
    MixotrophExcess,
    /// Decomposers drain less, and mixotrophs no more: answered on the
    /// decomposer side.
    DecomposerDeficit,
    /// Mixotrophs drain no more and decomposers no less.
    Neither,
}

impl DrainShift {
    pub fn of(run: Drains, baseline: Drains) -> Self {
        if run.mixotrophs > baseline.mixotrophs {
            Self::MixotrophExcess
        } else if run.decomposers < baseline.decomposers {
            Self::DecomposerDeficit
        } else {
            Self::Neither
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::MixotrophExcess => "mixotroph excess",
            Self::DecomposerDeficit => "decomposer deficit",
            Self::Neither => "neither (mixotrophs drain no more, decomposers no less)",
        }
    }
}

/// Exact two-sided binomial test of `k` successes in `n` trials against a
/// fair coin: the probability of every outcome no more likely than `k`
/// (R's `binom.test`, with its relative tolerance on ties). 1 when `n = 0`.
pub fn binomial_two_sided_p(k: usize, n: usize) -> f64 {
    assert!(k <= n, "{k} successes in {n} trials");
    // ln C(n, i) − n ln 2, built up term by term.
    let mut ln_pmf = Vec::with_capacity(n + 1);
    let mut ln_c = 0.0;
    for i in 0..=n {
        if i > 0 {
            ln_c += ((n - i + 1) as f64).ln() - (i as f64).ln();
        }
        ln_pmf.push(ln_c - n as f64 * std::f64::consts::LN_2);
    }
    let bar = ln_pmf[k].exp() * (1.0 + 1e-7);
    let p: f64 = ln_pmf.iter().map(|l| l.exp()).filter(|&p| p <= bar).sum();
    p.min(1.0)
}

/// Exact one-sided binomial test of `k` successes in `n` trials against a
/// fair coin, alternative "more successes": `P(X ≥ k)`, `X ~ Bin(n, ½)`
/// (R's `binom.test(k, n, alternative = "greater")`). 1 when `k = 0`.
pub fn binomial_upper_tail_p(k: usize, n: usize) -> f64 {
    assert!(k <= n, "{k} successes in {n} trials");
    let mut ln_c = 0.0;
    let mut p = 0.0;
    for i in 0..=n {
        if i > 0 {
            ln_c += ((n - i + 1) as f64).ln() - (i as f64).ln();
        }
        if i >= k {
            p += (ln_c - n as f64 * std::f64::consts::LN_2).exp();
        }
    }
    p.min(1.0)
}

/// Ward's agglomerative clustering of `points` (Euclidean), cut at `k`
/// clusters: each step merges the pair whose union least increases the
/// within-cluster sum of squares (Lance–Williams on squared distances, as
/// scipy's `linkage(method="ward")` orders its merges). Deterministic: a tie
/// merges the pair whose lower-indexed member comes first. Returns one label
/// per point, clusters numbered by their first member in input order.
/// `min(k, n)` clusters; `k` must be at least 1.
pub fn ward_clusters(points: &[Vec<f64>], k: usize) -> Vec<usize> {
    assert!(k >= 1, "cut at {k} clusters");
    let n = points.len();
    // d[i][j]: squared Ward distance between clusters i and j (cluster i is
    // keyed by its lowest member, so the tie-break follows input order).
    let mut d: Vec<Vec<f64>> = points
        .iter()
        .map(|a| {
            points
                .iter()
                .map(|b| a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum())
                .collect()
        })
        .collect();
    let mut size = vec![1usize; n];
    let mut owner: Vec<usize> = (0..n).collect();
    let mut active: Vec<usize> = (0..n).collect();
    while active.len() > k {
        let mut best = (f64::INFINITY, 0, 0);
        for (a, &i) in active.iter().enumerate() {
            for &j in &active[a + 1..] {
                if d[i][j] < best.0 {
                    best = (d[i][j], i, j);
                }
            }
        }
        let (dij, i, j) = best;
        for &m in &active {
            if m == i || m == j {
                continue;
            }
            let (ni, nj, nm) = (size[i] as f64, size[j] as f64, size[m] as f64);
            let v = ((ni + nm) * d[m][i] + (nj + nm) * d[m][j] - nm * dij) / (ni + nj + nm);
            d[i][m] = v;
            d[m][i] = v;
        }
        size[i] += size[j];
        for o in owner.iter_mut() {
            if *o == j {
                *o = i;
            }
        }
        active.retain(|&m| m != j);
    }
    // Clusters are keyed by their lowest member, so `active` is already in
    // first-member order.
    owner
        .iter()
        .map(|o| {
            active
                .iter()
                .position(|a| a == o)
                .expect("an active cluster")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn binomial_p_is_two_sided_against_a_coin() {
        // #670 §3's per-config counts; exact values (R's `binom.test`).
        // §3 prints 55 of 99 as 0.32; exactly it is 0.3149.
        assert!(close(binomial_two_sided_p(52, 99), 0.68789, 1e-5));
        assert!(close(binomial_two_sided_p(62, 96), 0.0055730, 1e-6));
        assert!(close(binomial_two_sided_p(55, 99), 0.31488, 1e-5));
        assert!(close(binomial_two_sided_p(53, 96), 0.35840, 1e-5));
        // Symmetric, and exactly 1 at an even split.
        assert_eq!(binomial_two_sided_p(3, 10), binomial_two_sided_p(7, 10));
        assert!(close(binomial_two_sided_p(5, 10), 1.0, 1e-12));
        // Every one of 10 one way: 2 / 2^10.
        assert!(close(binomial_two_sided_p(10, 10), 2.0 / 1024.0, 1e-12));
        assert!(close(binomial_two_sided_p(0, 0), 1.0, 1e-12));
    }

    #[test]
    fn the_upper_tail_is_the_chance_of_at_least_k_heads() {
        assert!(close(binomial_upper_tail_p(10, 10), 1.0 / 1024.0, 1e-12));
        assert!(close(binomial_upper_tail_p(8, 10), 56.0 / 1024.0, 1e-12));
        assert!(close(binomial_upper_tail_p(7, 8), 9.0 / 256.0, 1e-12));
        assert!(close(binomial_upper_tail_p(0, 10), 1.0, 1e-12));
        assert!(close(binomial_upper_tail_p(0, 0), 1.0, 1e-12));
        // R: binom.test(62, 96, alternative = "greater").
        assert!(close(binomial_upper_tail_p(62, 96), 0.0027865, 1e-6));
    }

    #[test]
    fn ward_recovers_well_separated_blobs_labelled_by_first_member() {
        // Three tight blobs in 3-d, interleaved in input order.
        let centres = [[0.1, 0.1, 0.1], [0.9, 0.1, 0.5], [0.5, 0.9, 0.9]];
        let mut points = Vec::new();
        let mut truth = Vec::new();
        for i in 0..15 {
            let c = [2, 0, 1][i % 3];
            let jitter = 0.01 * (i as f64);
            points.push(centres[c].iter().map(|x| x + jitter * 0.3).collect());
            truth.push(c);
        }
        let labels = ward_clusters(&points, 3);
        // Labels are numbered by each cluster's first member in input order:
        // point 0 (blob 2) is cluster 0, point 1 (blob 0) cluster 1, …
        let expected: Vec<usize> = truth.iter().map(|&c| [1, 2, 0][c]).collect();
        assert_eq!(labels, expected);
    }

    #[test]
    fn ward_merges_by_least_increase_in_within_cluster_variance() {
        // {0, 1} and {10, 11} pair first; then {20} joins {10, 11} (ΔSS
        // = 2·1/3 · 9.5² ≈ 60.2) before the pairs join each other (2·2/4 ·
        // 10² = 100).
        let points: Vec<Vec<f64>> = [0.0, 1.0, 10.0, 11.0, 20.0]
            .iter()
            .map(|&x| vec![x])
            .collect();
        assert_eq!(ward_clusters(&points, 2), vec![0, 0, 1, 1, 1]);
        assert_eq!(ward_clusters(&points, 3), vec![0, 0, 1, 1, 2]);
        assert_eq!(ward_clusters(&points, 5), vec![0, 1, 2, 3, 4]);
        // Fewer points than clusters: each its own.
        assert_eq!(ward_clusters(&points[..2], 8), vec![0, 1]);
        assert!(ward_clusters(&[], 8).is_empty());
    }

    #[test]
    fn conditionality_passes_only_when_median_coin_and_clusters_all_hold() {
        // 20 configs, 16 negative (p ≈ 0.012), in 4 clusters of 5.
        let mut rhos: Vec<Option<f64>> = (0..20)
            .map(|i| Some(if i % 5 == 4 { 0.1 } else { -0.1 }))
            .collect();
        let clusters: Vec<usize> = (0..20).map(|i| i / 5).collect();
        let v = Conditionality::of(Some(-0.2), &rhos, Some(&clusters));
        assert_eq!((v.configs, v.negative), (20, 16));
        assert_eq!(v.median, Some(-0.1));
        assert!(v.binomial_p < COIN_ALPHA);
        assert_eq!(
            v.clusters,
            Some(ClusterRead {
                with_median: 4,
                negative: 4
            })
        );
        assert_eq!(v.median_negative(), Some(true));
        assert_eq!(v.not_a_coin(), Some(true));
        assert_eq!(v.clusters_negative(), Some(true));
        assert_eq!(v.passes(), Some(true));
        assert_eq!(v.does_not_run_backwards(), Some(true));

        // A coin's split with its median a hair below zero fails the second.
        let coin: Vec<Option<f64>> = (0..20)
            .map(|i| Some(if i < 11 { -0.01 } else { 0.01 }))
            .collect();
        let v = Conditionality::of(Some(-0.2), &coin, Some(&clusters));
        assert_eq!(v.median_negative(), Some(true));
        assert_eq!(v.not_a_coin(), Some(false));
        assert_eq!(v.passes(), Some(false));

        // The negative configs crowded into two of four clusters: the
        // clusters split 2 / 2, not a majority.
        let crowded: Vec<Option<f64>> = (0..20)
            .map(|i| Some(if i < 10 || i % 5 < 1 { -0.1 } else { 0.1 }))
            .collect();
        let v = Conditionality::of(Some(0.1), &crowded, Some(&clusters));
        assert_eq!(
            v.clusters,
            Some(ClusterRead {
                with_median: 4,
                negative: 2
            })
        );
        assert_eq!(v.clusters_negative(), Some(false));
        assert_eq!(v.passes(), Some(false));
        assert_eq!(v.does_not_run_backwards(), Some(false));

        // Without clusters the third is unread: a pass is n/a, a fail stays.
        let v = Conditionality::of(None, &rhos, None);
        assert_eq!(v.clusters_negative(), None);
        assert_eq!(v.passes(), None);
        assert_eq!(v.does_not_run_backwards(), None);
        let v = Conditionality::of(None, &coin, None);
        assert_eq!(v.passes(), Some(false));

        // Undefined ρ count nowhere; a cluster without one has no median.
        rhos[0] = None;
        rhos[5..10].iter_mut().for_each(|r| *r = None);
        let v = Conditionality::of(None, &rhos, Some(&clusters));
        assert_eq!((v.configs, v.negative), (14, 11));
        assert_eq!(
            v.clusters,
            Some(ClusterRead {
                with_median: 3,
                negative: 3
            })
        );
    }

    #[test]
    fn the_minority_share_is_strictly_below_half() {
        assert_eq!(minority(Some(0.454)), Some(true));
        assert_eq!(minority(Some(0.5)), Some(false));
        assert_eq!(minority(Some(0.616)), Some(false));
        assert_eq!(minority(None), None);
    }

    #[test]
    fn a_share_is_told_apart_by_absolute_drains() {
        // #670 §4 against #668 decoded, per seed: both fall, decomposers too.
        let base = Drains {
            mixotrophs: 51.0,
            decomposers: 38.8,
        };
        let seed42 = Drains {
            mixotrophs: 39.6,
            decomposers: 18.4,
        };
        assert_eq!(DrainShift::of(seed42, base), DrainShift::DecomposerDeficit);
        // Mixotrophs hold steady while decomposers fall: still a deficit.
        let steady = Drains {
            mixotrophs: 51.0,
            decomposers: 20.0,
        };
        assert_eq!(DrainShift::of(steady, base), DrainShift::DecomposerDeficit);
        // Mixotrophs drain more: excess, whatever the decomposers do.
        for decomposers in [20.0, 38.8, 50.0] {
            let more = Drains {
                mixotrophs: 60.0,
                decomposers,
            };
            assert_eq!(DrainShift::of(more, base), DrainShift::MixotrophExcess);
        }
        // Neither: mixotrophs no more, decomposers no less.
        let neither = Drains {
            mixotrophs: 40.0,
            decomposers: 40.0,
        };
        assert_eq!(DrainShift::of(neither, base), DrainShift::Neither);
    }
}
