//! Config sourcing shared by the research bins (`role_emergence`,
//! `energy_bound_check`, `permanence_crosscheck`, `invasion_growth`, …): the
//! `source:index` selector grammar and the seed-421 LHS draw, so `sample:i`
//! names the same config in every instrument.
//!
//! A selector may name another LHS draw (#553): `sample@S:i` is the `i`-th
//! config of the seed-`S` draw — an independent sample of the same box, for
//! held-out checks of a fit made on the seed-421 draw. Bare `sample:i` keeps
//! meaning seed 421 byte-for-byte, and the label a row records
//! ([`ConfigSource`]'s `source` field) carries the seed wherever it is not
//! 421, so rows of two draws can never collide in one file or be joined by
//! mistake. [`resolve_config`] turns any key into the world it names.

use std::collections::HashSet;

use explorers_genesis::{InitialDistribution, WorldParameters};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::lhs;
use crate::search::{decode, default_ranges};
use crate::sweep::AtlasUnits;

/// Low-discrepancy configs drawn in addition to the atlas cells — same count
/// and seed in every bin.
pub const SAMPLE_CONFIGS: usize = 200;
pub const SAMPLE_SEED: u64 = 421;

/// Where a config came from: an atlas live cell, or the LHS draw of the
/// given seed (`SAMPLE_SEED` for the shared draw every bin names `sample:i`).
///
/// Its label — in a selector and in a row's `source` field — is `atlas`,
/// `sample` for the seed-421 draw, and `sample@SEED` for any other draw, so a
/// row from another draw has a label no seed-421 row can have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConfigSource {
    Atlas,
    Sample(u64),
}

impl ConfigSource {
    /// The shared seed-421 draw: bare `sample`.
    pub const SAMPLE: ConfigSource = ConfigSource::Sample(SAMPLE_SEED);

    /// A config of an LHS draw, whichever seed: summaries bucket every draw
    /// as "sample" (a file holds rows of one draw unless files were mixed).
    pub fn is_sample(self) -> bool {
        matches!(self, ConfigSource::Sample(_))
    }
}

impl std::fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigSource::Atlas => f.write_str("atlas"),
            ConfigSource::Sample(SAMPLE_SEED) => f.write_str("sample"),
            ConfigSource::Sample(seed) => write!(f, "sample@{seed}"),
        }
    }
}

impl serde::Serialize for ConfigSource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for ConfigSource {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let label = String::deserialize(deserializer)?;
        label.parse().map_err(serde::de::Error::custom)
    }
}

impl std::str::FromStr for ConfigSource {
    type Err = String;

    fn from_str(label: &str) -> Result<Self, String> {
        match label {
            "atlas" => Ok(ConfigSource::Atlas),
            "sample" => Ok(ConfigSource::SAMPLE),
            other => other
                .strip_prefix("sample@")
                .and_then(|seed| seed.parse().ok())
                .map(ConfigSource::Sample)
                .ok_or_else(|| format!("source {other:?} must be atlas|sample|sample@SEED")),
        }
    }
}

/// The seed-421 LHS draw of `SAMPLE_CONFIGS` unit vectors over `dims`.
pub fn sampled_units(dims: usize) -> Vec<Vec<f64>> {
    sample_draw(SAMPLE_SEED, dims)
}

/// The seed-`seed` LHS draw of `SAMPLE_CONFIGS` unit vectors over `dims`.
pub fn sample_draw(seed: u64, dims: usize) -> Vec<Vec<f64>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    lhs::sample(dims, SAMPLE_CONFIGS, &mut rng)
}

/// The world a `(source, index)` key names (#559): an atlas cell decoded over
/// the atlas's own search box, or a config of an LHS draw — the caller's
/// seed-421 copy `sampled`, or a fresh draw of any other seed — decoded over
/// the full box (`default_ranges`). The LHS draws are instruments over the
/// whole box, not the search's, whatever box the atlas was drawn under.
pub fn resolve_config(
    source: ConfigSource,
    index: usize,
    atlas: &AtlasUnits,
    sampled: &[Vec<f64>],
) -> (WorldParameters, InitialDistribution) {
    let full = default_ranges();
    match source {
        ConfigSource::Atlas => atlas.decode(index),
        ConfigSource::SAMPLE => decode(&sampled[index], &full),
        ConfigSource::Sample(seed) => decode(&sample_draw(seed, full.len())[index], &full),
    }
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
            Some((label, idx)) => (label.parse().unwrap_or_else(|e| panic!("{var} {e}")), idx),
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
            (ConfigSource::SAMPLE, 55),
            (ConfigSource::Atlas, 7),
        ]
        .into_iter()
        .collect();
        assert_eq!(set, expect);
    }

    /// `sample@S:i` names the `i`-th config of the seed-`S` draw; bare
    /// `sample:i` (and the redundant `sample@421:i`) is the seed-421 draw.
    #[test]
    fn selector_names_the_draw_a_sample_comes_from() {
        let set = parse_selector("sample:12, sample@9421:12, sample@421:3", "X", None);
        let expect: HashSet<_> = [
            (ConfigSource::SAMPLE, 12),
            (ConfigSource::Sample(9421), 12),
            (ConfigSource::Sample(SAMPLE_SEED), 3),
        ]
        .into_iter()
        .collect();
        assert_eq!(set, expect);
    }

    /// A row's `source` label is unchanged for the atlas and the seed-421
    /// draw (so every row on disk keeps its meaning) and carries the seed for
    /// any other draw, so rows from two draws can neither collide nor join.
    #[test]
    fn row_label_carries_the_seed_of_any_draw_but_421() {
        let label = |s: ConfigSource| serde_json::to_string(&s).unwrap();
        assert_eq!(label(ConfigSource::Atlas), r#""atlas""#);
        assert_eq!(label(ConfigSource::SAMPLE), r#""sample""#);
        assert_eq!(label(ConfigSource::Sample(9421)), r#""sample@9421""#);
        for source in [
            ConfigSource::Atlas,
            ConfigSource::SAMPLE,
            ConfigSource::Sample(9421),
        ] {
            let back: ConfigSource = serde_json::from_str(&label(source)).unwrap();
            assert_eq!(back, source);
        }
        let redundant: ConfigSource = serde_json::from_str(r#""sample@421""#).unwrap();
        assert_eq!(redundant, ConfigSource::SAMPLE);
        assert_eq!(ConfigSource::Sample(9421).to_string(), "sample@9421");
    }

    /// A sample key resolves to its own draw's world, over the full box: the
    /// caller's seed-421 copy for `sample:i`, a fresh draw for `sample@S:i`.
    /// An atlas key resolves over the atlas's own box.
    #[test]
    fn a_key_resolves_to_the_world_of_its_own_draw_and_box() {
        let full = crate::search::default_ranges();
        let sampled = sampled_units(full.len());
        let other = sample_draw(9421, full.len());
        assert_ne!(other[12], sampled[12], "an independent draw");
        let world = |source, i| resolve_config(source, i, &AtlasUnits::default(), &sampled);
        assert_eq!(
            world(ConfigSource::Sample(9421), 12),
            decode(&other[12], &full)
        );
        assert_eq!(world(ConfigSource::SAMPLE, 12), decode(&sampled[12], &full));
        let narrowed = crate::search::narrowed_ranges();
        let atlas = AtlasUnits::new(narrowed.clone(), vec![vec![0.25; full.len()]]);
        let cell = resolve_config(ConfigSource::Atlas, 0, &atlas, &sampled);
        assert_eq!(
            cell,
            decode(&[0.25; 32], &narrowed),
            "atlas cells: their own box"
        );
    }

    /// The seed-421 draw is the one every row on disk names as `sample:i`:
    /// pinned to the vectors it produced before the seed became selectable.
    #[test]
    fn the_seed_421_draw_is_unchanged() {
        let units = sample_draw(SAMPLE_SEED, 32);
        assert_eq!(units.len(), SAMPLE_CONFIGS);
        assert_eq!(units, sampled_units(32));
        assert_eq!(
            units[0][..3],
            [0.39506597665000004, 0.8860893027283057, 0.14197006724157732]
        );
        assert_eq!(units[199][31], 0.768770269676153);
        let weighted: f64 = units
            .iter()
            .enumerate()
            .flat_map(|(i, v)| {
                v.iter()
                    .enumerate()
                    .map(move |(j, x)| x * ((i * 31 + j) as f64))
            })
            .sum();
        assert_eq!(weighted, 9847351.185018335);
    }
}
