pub struct GaussianProcess {
    x_train: Vec<Vec<f64>>,
    length_scale: f64,
    signal_variance: f64,
    alpha: Vec<f64>,
    l_cholesky: Vec<Vec<f64>>,
}

impl GaussianProcess {
    pub fn fit(x: &[Vec<f64>], y: &[f64]) -> Self {
        let length_scale = 1.0;
        let signal_variance = 1.0;
        let noise_variance = 1e-6;

        let n = x.len();
        let mut k = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                k[i][j] = se_kernel(&x[i], &x[j], length_scale, signal_variance);
                if i == j {
                    k[i][j] += noise_variance;
                }
            }
        }

        let l = cholesky(&k);
        let alpha = cholesky_solve(&l, y);

        GaussianProcess {
            x_train: x.to_vec(),
            length_scale,
            signal_variance,
            alpha,
            l_cholesky: l,
        }
    }

    pub fn predict(&self, x: &[f64]) -> (f64, f64) {
        let n = self.x_train.len();
        let k_star: Vec<f64> = (0..n)
            .map(|i| se_kernel(x, &self.x_train[i], self.length_scale, self.signal_variance))
            .collect();

        let mean: f64 = k_star.iter().zip(&self.alpha).map(|(k, a)| k * a).sum();

        let v = forward_substitute(&self.l_cholesky, &k_star);
        let k_ss = se_kernel(x, x, self.length_scale, self.signal_variance);
        let variance = (k_ss - v.iter().map(|vi| vi * vi).sum::<f64>()).max(0.0);

        (mean, variance)
    }
}

fn se_kernel(a: &[f64], b: &[f64], length_scale: f64, signal_variance: f64) -> f64 {
    let sq_dist: f64 = a.iter().zip(b).map(|(ai, bi)| (ai - bi).powi(2)).sum();
    signal_variance * (-sq_dist / (2.0 * length_scale * length_scale)).exp()
}

fn cholesky(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0;
            for k in 0..j {
                sum += l[i][k] * l[j][k];
            }
            if i == j {
                l[i][j] = (a[i][i] - sum).sqrt();
            } else {
                l[i][j] = (a[i][j] - sum) / l[j][j];
            }
        }
    }
    l
}

fn forward_substitute(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    let mut x = vec![0.0; n];
    for i in 0..n {
        let mut sum = 0.0;
        for j in 0..i {
            sum += l[i][j] * x[j];
        }
        x[i] = (b[i] - sum) / l[i][i];
    }
    x
}

fn cholesky_solve(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let y = forward_substitute(l, b);
    let n = b.len();
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = 0.0;
        for j in (i + 1)..n {
            sum += l[j][i] * x[j];
        }
        x[i] = (y[i] - sum) / l[i][i];
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicts_known_training_points() {
        let x = vec![vec![0.0], vec![1.0], vec![2.0]];
        let y = vec![0.0, 1.0, 0.0];
        let gp = GaussianProcess::fit(&x, &y);

        for (xi, &yi) in x.iter().zip(y.iter()) {
            let (mean, variance) = gp.predict(xi);
            assert!(
                (mean - yi).abs() < 0.1,
                "at {:?}: predicted {mean}, expected {yi}",
                xi
            );
            assert!(
                variance < 0.1,
                "at {:?}: variance {variance} should be near zero at training points",
                xi
            );
        }
    }

    #[test]
    fn higher_uncertainty_far_from_training_data() {
        let x = vec![vec![0.0], vec![1.0]];
        let y = vec![0.0, 1.0];
        let gp = GaussianProcess::fit(&x, &y);

        let (_, var_near) = gp.predict(&[0.5]);
        let (_, var_far) = gp.predict(&[5.0]);

        assert!(
            var_far > var_near,
            "variance far ({var_far}) should exceed variance near ({var_near})"
        );
    }
}
