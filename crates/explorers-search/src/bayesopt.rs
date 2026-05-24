use rand::Rng;

use crate::gp::GaussianProcess;
use crate::lhs;

pub struct BayesianOptimiser {
    bounds: Vec<(f64, f64)>,
    fixed: Vec<Option<f64>>,
    x_observed: Vec<Vec<f64>>,
    y_observed: Vec<f64>,
}

pub struct OptimisationResult {
    pub best_x: Vec<f64>,
    pub best_y: f64,
    pub iterations: usize,
}

impl BayesianOptimiser {
    pub fn new(bounds: Vec<(f64, f64)>, fixed: Vec<Option<f64>>) -> Self {
        BayesianOptimiser {
            bounds,
            fixed,
            x_observed: vec![],
            y_observed: vec![],
        }
    }

    pub fn optimise(
        &mut self,
        evaluate: impl Fn(&[f64]) -> f64,
        iterations: usize,
        rng: &mut impl Rng,
    ) -> OptimisationResult {
        let dims = self.bounds.len();
        let n_initial = (2 * dims).max(3);

        let unit_samples = lhs::sample(dims, n_initial, rng);
        for s in &unit_samples {
            let x = self.to_full(s);
            let y = evaluate(&x);
            self.x_observed.push(x);
            self.y_observed.push(y);
        }

        let free_dims: Vec<usize> = (0..dims)
            .filter(|&d| self.fixed[d].is_none())
            .collect();

        for _ in 0..iterations {
            let x_free: Vec<Vec<f64>> = self
                .x_observed
                .iter()
                .map(|x| free_dims.iter().map(|&d| x[d]).collect())
                .collect();

            let gp = GaussianProcess::fit(&x_free, &self.y_observed);
            let best_y = self
                .y_observed
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);

            let next = self.maximise_ei(&gp, &free_dims, best_y, rng);
            let y = evaluate(&next);
            self.x_observed.push(next);
            self.y_observed.push(y);
        }

        let (best_idx, &best_y) = self
            .y_observed
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        OptimisationResult {
            best_x: self.x_observed[best_idx].clone(),
            best_y,
            iterations,
        }
    }

    fn to_full(&self, unit: &[f64]) -> Vec<f64> {
        (0..self.bounds.len())
            .map(|d| {
                if let Some(v) = self.fixed[d] {
                    v
                } else {
                    let (lo, hi) = self.bounds[d];
                    lo + unit[d] * (hi - lo)
                }
            })
            .collect()
    }

    fn maximise_ei(
        &self,
        gp: &GaussianProcess,
        free_dims: &[usize],
        best_y: f64,
        rng: &mut impl Rng,
    ) -> Vec<f64> {
        let n_candidates = 200;
        let mut best_ei = f64::NEG_INFINITY;
        let mut best_x = vec![0.0; self.bounds.len()];

        for _ in 0..n_candidates {
            let mut full = vec![0.0; self.bounds.len()];
            for d in 0..self.bounds.len() {
                full[d] = if let Some(v) = self.fixed[d] {
                    v
                } else {
                    let (lo, hi) = self.bounds[d];
                    rng.random_range(lo..hi)
                };
            }

            let x_free: Vec<f64> = free_dims.iter().map(|&d| full[d]).collect();
            let (mean, variance) = gp.predict(&x_free);
            let ei = expected_improvement(mean, variance, best_y);

            if ei > best_ei {
                best_ei = ei;
                best_x = full;
            }
        }

        best_x
    }
}

fn expected_improvement(mean: f64, variance: f64, best_y: f64) -> f64 {
    if variance < 1e-12 {
        return (mean - best_y).max(0.0);
    }
    let std = variance.sqrt();
    let z = (mean - best_y) / std;
    let pdf = (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let cdf = 0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2));
    (mean - best_y) * cdf + std * pdf
}

fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.3275911 * x.abs());
    let poly = t
        * (0.254829592
            + t * (-0.284496736
                + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    let result = 1.0 - poly * (-x * x).exp();
    if x >= 0.0 { result } else { -result }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn converges_on_1d_quadratic() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let f = |x: &[f64]| -> f64 { -(x[0] - 0.7).powi(2) };

        let mut opt = BayesianOptimiser::new(vec![(0.0, 1.0)], vec![None]);
        let result = opt.optimise(f, 20, &mut rng);

        assert!(
            (result.best_x[0] - 0.7).abs() < 0.1,
            "best_x = {}, expected near 0.7",
            result.best_x[0]
        );
    }

    #[test]
    fn respects_fixed_dimensions() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let f = |x: &[f64]| -> f64 { -(x[0] - 0.3).powi(2) - (x[1] - 0.8).powi(2) };

        let mut opt = BayesianOptimiser::new(
            vec![(0.0, 1.0), (0.0, 1.0)],
            vec![None, Some(0.5)],
        );
        let result = opt.optimise(f, 20, &mut rng);

        assert_eq!(result.best_x[1], 0.5, "fixed dimension should stay at 0.5");
        assert!(
            (result.best_x[0] - 0.3).abs() < 0.15,
            "best_x[0] = {}, expected near 0.3",
            result.best_x[0]
        );
    }
}
