use rand::Rng;

pub struct SobolIndices {
    pub first_order: Vec<f64>,
    pub total_effect: Vec<f64>,
}

pub struct SensitivityRanking {
    pub rankings: Vec<(usize, f64)>,
}

pub fn rank_by_total_effect(indices: &SobolIndices) -> SensitivityRanking {
    let mut rankings: Vec<(usize, f64)> = indices
        .total_effect
        .iter()
        .enumerate()
        .map(|(i, &st)| (i, st))
        .collect();
    rankings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    SensitivityRanking { rankings }
}

/// Saltelli (2002) estimator for first-order and total-effect Sobol indices.
/// `evaluate` maps a point in [0,1]^dimensions to a scalar output.
/// `n` is the base sample size — total evaluations are n*(dimensions+2).
pub fn sobol_indices(
    evaluate: impl Fn(&[f64]) -> f64,
    dimensions: usize,
    n: usize,
    rng: &mut impl Rng,
) -> SobolIndices {
    let mut sample_matrix = || -> Vec<Vec<f64>> {
        (0..n)
            .map(|_| {
                (0..dimensions)
                    .map(|_| rng.random_range(0.0..1.0))
                    .collect()
            })
            .collect()
    };

    let a = sample_matrix();
    let b = sample_matrix();

    let y_a: Vec<f64> = a.iter().map(|x| evaluate(x)).collect();
    let y_b: Vec<f64> = b.iter().map(|x| evaluate(x)).collect();

    let mut first_order = vec![0.0; dimensions];
    let mut total_effect = vec![0.0; dimensions];

    let f0_sq = {
        let mean_a: f64 = y_a.iter().sum::<f64>() / n as f64;
        let mean_b: f64 = y_b.iter().sum::<f64>() / n as f64;
        mean_a * mean_b
    };

    let var_y = y_a.iter().map(|&y| y * y).sum::<f64>() / n as f64 - f0_sq;

    if var_y.abs() < 1e-12 {
        return SobolIndices {
            first_order: vec![0.0; dimensions],
            total_effect: vec![0.0; dimensions],
        };
    }

    for j in 0..dimensions {
        let ab_j: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let mut x = a[i].clone();
                x[j] = b[i][j];
                x
            })
            .collect();
        let y_ab_j: Vec<f64> = ab_j.iter().map(|x| evaluate(x)).collect();

        // Jansen (1999) estimator for first-order
        let s_j = (0..n).map(|i| y_b[i] * (y_ab_j[i] - y_a[i])).sum::<f64>() / (n as f64 * var_y);

        // Jansen (1999) estimator for total-effect
        let st_j =
            (0..n).map(|i| (y_a[i] - y_ab_j[i]).powi(2)).sum::<f64>() / (2.0 * n as f64 * var_y);

        first_order[j] = s_j;
        total_effect[j] = st_j;
    }

    SobolIndices {
        first_order,
        total_effect,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use std::f64::consts::PI;

    fn ishigami(x: &[f64]) -> f64 {
        let a = 7.0;
        let b = 0.1;
        (x[0]).sin() + a * (x[1]).sin().powi(2) + b * x[2].powi(4) * (x[0]).sin()
    }

    fn ishigami_analytic_first_order() -> [f64; 3] {
        let a: f64 = 7.0;
        let b: f64 = 0.1;
        let var_y = a * a / 8.0 + b * PI.powi(4) / 5.0 + b * b * PI.powi(8) / 18.0 + 0.5;
        let s1 = (0.5 * (1.0 + b * PI.powi(4) / 5.0).powi(2)) / var_y;
        let s2 = (a * a / 8.0) / var_y;
        let s3 = 0.0;
        [s1, s2, s3]
    }

    fn ishigami_analytic_total_effect() -> [f64; 3] {
        let a: f64 = 7.0;
        let b: f64 = 0.1;
        let var_y = a * a / 8.0 + b * PI.powi(4) / 5.0 + b * b * PI.powi(8) / 18.0 + 0.5;
        let s1 = (0.5 * (1.0 + b * PI.powi(4) / 5.0).powi(2)) / var_y;
        let s2 = (a * a / 8.0) / var_y;
        let s13 = (b * b * PI.powi(8) * 8.0 / 225.0) / var_y;
        let st1 = s1 + s13;
        let st2 = s2;
        let st3 = s13;
        [st1, st2, st3]
    }

    #[test]
    fn first_order_indices_match_ishigami_analytic() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let n = 10_000;

        let rescale = |x: &[f64]| -> f64 {
            let scaled: Vec<f64> = x.iter().map(|&v| v * 2.0 * PI - PI).collect();
            ishigami(&scaled)
        };

        let result = sobol_indices(rescale, 3, n, &mut rng);
        let analytic = ishigami_analytic_first_order();

        assert_eq!(result.first_order.len(), 3);
        for i in 0..3 {
            assert!(
                (result.first_order[i] - analytic[i]).abs() < 0.05,
                "S{} = {}, expected {} (tol 0.05)",
                i + 1,
                result.first_order[i],
                analytic[i]
            );
        }
    }

    #[test]
    fn ranking_sorts_by_total_effect_descending() {
        let indices = SobolIndices {
            first_order: vec![0.3, 0.1, 0.5],
            total_effect: vec![0.4, 0.1, 0.6],
        };
        let ranking = rank_by_total_effect(&indices);
        assert_eq!(ranking.rankings[0].0, 2);
        assert_eq!(ranking.rankings[1].0, 0);
        assert_eq!(ranking.rankings[2].0, 1);
    }

    #[test]
    fn total_effect_indices_match_ishigami_analytic() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let n = 10_000;

        let rescale = |x: &[f64]| -> f64 {
            let scaled: Vec<f64> = x.iter().map(|&v| v * 2.0 * PI - PI).collect();
            ishigami(&scaled)
        };

        let result = sobol_indices(rescale, 3, n, &mut rng);
        let analytic = ishigami_analytic_total_effect();

        assert_eq!(result.total_effect.len(), 3);
        for i in 0..3 {
            assert!(
                (result.total_effect[i] - analytic[i]).abs() < 0.05,
                "ST{} = {}, expected {} (tol 0.05)",
                i + 1,
                result.total_effect[i],
                analytic[i]
            );
        }
    }
}
