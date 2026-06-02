use rand::Rng;
use rand::seq::SliceRandom;

pub fn sample(dimensions: usize, n: usize, rng: &mut impl Rng) -> Vec<Vec<f64>> {
    let mut result: Vec<Vec<f64>> = (0..n).map(|_| Vec::with_capacity(dimensions)).collect();

    for _d in 0..dimensions {
        let mut permutation: Vec<usize> = (0..n).collect();
        permutation.shuffle(rng);

        for (i, &stratum) in permutation.iter().enumerate() {
            let low = stratum as f64 / n as f64;
            let high = (stratum + 1) as f64 / n as f64;
            let value = rng.random_range(low..high);
            result[i].push(value);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn all_values_in_unit_interval() {
        let mut rng = ChaCha8Rng::seed_from_u64(99);
        let samples = sample(5, 20, &mut rng);

        for s in &samples {
            for &v in s {
                assert!(v >= 0.0 && v < 1.0, "value {v} outside [0, 1)");
            }
        }
    }

    #[test]
    fn each_dimension_has_one_sample_per_stratum() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let n = 10;
        let dims = 3;
        let samples = sample(dims, n, &mut rng);

        assert_eq!(samples.len(), n);
        for s in &samples {
            assert_eq!(s.len(), dims);
        }

        for d in 0..dims {
            let mut strata: Vec<usize> = samples
                .iter()
                .map(|s| (s[d] * n as f64).floor() as usize)
                .collect();
            strata.sort();
            assert_eq!(
                strata,
                (0..n).collect::<Vec<_>>(),
                "dimension {d} must have exactly one sample per stratum"
            );
        }
    }
}
