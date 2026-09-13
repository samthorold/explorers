//! Config sourcing shared by the research bins (`role_emergence`,
//! `energy_bound_check`, `permanence_crosscheck`, `invasion_growth`): the
//! `source:index` selector grammar and the seed-421 LHS draw, so `sample:i`
//! names the same config in every instrument.

use std::collections::HashSet;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::lhs;

/// Low-discrepancy configs drawn in addition to the atlas cells — same count
/// and seed in every bin.
pub const SAMPLE_CONFIGS: usize = 200;
pub const SAMPLE_SEED: u64 = 421;

/// Where a config came from: an atlas live cell or the shared LHS draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigSource {
    Atlas,
    Sample,
}

/// The seed-421 LHS draw of `SAMPLE_CONFIGS` unit vectors over `dims`.
pub fn sampled_units(dims: usize) -> Vec<Vec<f64>> {
    let mut rng = ChaCha8Rng::seed_from_u64(SAMPLE_SEED);
    lhs::sample(dims, SAMPLE_CONFIGS, &mut rng)
}

/// Parse a `source:index` selector list (`atlas:0,sample:12`). A bare
/// integer is accepted as an index into `bare` when one is given, and is an
/// error otherwise. `var` names the variable in panic messages.
pub fn parse_selector(
    raw: &str,
    var: &str,
    bare: Option<ConfigSource>,
) -> HashSet<(ConfigSource, usize)> {
    let mut set = HashSet::new();
    for tok in raw.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        let (source, idx) = match tok.split_once(':') {
            Some(("atlas", idx)) => (ConfigSource::Atlas, idx),
            Some(("sample", idx)) => (ConfigSource::Sample, idx),
            Some((other, _)) => panic!("{var} source {other:?} must be atlas|sample"),
            None => match bare {
                Some(source) => (source, tok),
                None => panic!("{var} token {tok:?} is not source:index"),
            },
        };
        let index: usize = idx
            .parse()
            .unwrap_or_else(|_| panic!("{var} index {idx:?} is not a usize"));
        set.insert((source, index));
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_parses_source_index_and_bare_atlas_indices() {
        let set = parse_selector("atlas:3, sample:55,7,,", "X", Some(ConfigSource::Atlas));
        let expect: HashSet<_> = [
            (ConfigSource::Atlas, 3),
            (ConfigSource::Sample, 55),
            (ConfigSource::Atlas, 7),
        ]
        .into_iter()
        .collect();
        assert_eq!(set, expect);
    }
}
