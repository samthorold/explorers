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

/// Pin a resolved world's **founder aggregation** (#601) when `pin` is given,
/// leaving everything else as decoded. Decoded worlds found at the aggregated
/// design default; a measurement against a pre-#601 tree pins `0.0`, the
/// legacy well-mixed scatter exactly (#605).
pub fn with_founder_aggregation(
    (params, mut dist): (WorldParameters, InitialDistribution),
    pin: Option<f32>,
) -> (WorldParameters, InitialDistribution) {
    if let Some(a) = pin {
        dist.founder_aggregation = a;
    }
    (params, dist)
}

/// Pin a resolved world's **satiation sensitivity** and **recognition
/// distance** (the need gate's and recognition's scales, #600/#604) when given,
/// leaving everything else as decoded — #619's probe of whether those
/// mechanisms fail on scale or by design.
pub fn with_consumption_scales(
    (mut params, dist): (WorldParameters, InitialDistribution),
    satiation_sensitivity: Option<f32>,
    recognition_distance: Option<f32>,
) -> (WorldParameters, InitialDistribution) {
    if let Some(c) = satiation_sensitivity {
        params.satiation_sensitivity = c;
    }
    if let Some(d) = recognition_distance {
        params.recognition_distance = d;
    }
    (params, dist)
}

/// Pin a resolved world's **size-scaled uptake** (#644) — the exponent `b`
/// and the reference structure `s_ref` — when given, leaving everything else
/// as decoded: #645's probe of whether uptake that grows with the body ends
/// producers' carcass dependence.
pub fn with_uptake_scaling(
    (mut params, dist): (WorldParameters, InitialDistribution),
    exponent: Option<f32>,
    reference_structure: Option<f32>,
) -> (WorldParameters, InitialDistribution) {
    if let Some(b) = exponent {
        params.uptake_structure_exponent = b;
    }
    if let Some(s) = reference_structure {
        params.uptake_reference_structure = s;
    }
    (params, dist)
}

/// Pins for the five **network** parameters (flow 5); `None` keeps the
/// decoded value (every decoded world has the network off: cap 0).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NetworkPins {
    pub connection_cap: Option<u32>,
    pub creation_cost: Option<f32>,
    pub maintenance_cost: Option<f32>,
    pub redistribution_rate: Option<f32>,
    pub transfer_efficiency: Option<f32>,
}

/// Pin a resolved world's **network** parameters (flow 5) where given,
/// leaving everything else as decoded: #646's probe of whether a mycorrhizal
/// route changes who processes detritus.
pub fn with_network(
    (mut params, dist): (WorldParameters, InitialDistribution),
    pins: &NetworkPins,
) -> (WorldParameters, InitialDistribution) {
    if let Some(v) = pins.connection_cap {
        params.network_connection_cap = v;
    }
    if let Some(v) = pins.creation_cost {
        params.network_creation_cost = v;
    }
    if let Some(v) = pins.maintenance_cost {
        params.network_maintenance_cost = v;
    }
    if let Some(v) = pins.redistribution_rate {
        params.network_redistribution_rate = v;
    }
    if let Some(v) = pins.transfer_efficiency {
        params.network_transfer_efficiency = v;
    }
    (params, dist)
}

/// Parse a `flag`'s value as a finite number in `[0, 1]`
/// (`--network-transfer-efficiency`).
pub fn parse_unit_interval(flag: &str, raw: &str) -> Result<f32, String> {
    raw.parse::<f32>()
        .ok()
        .filter(|v| (0.0..=1.0).contains(v))
        .ok_or_else(|| format!("{flag} {raw:?} must be a number in [0, 1]"))
}

/// Parse a `flag`'s value as a finite number `> 0` (`--uptake-reference-structure`).
pub fn parse_positive(flag: &str, raw: &str) -> Result<f32, String> {
    raw.parse::<f32>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| format!("{flag} {raw:?} must be a finite number > 0"))
}

/// Parse a `flag`'s value as a finite number `≥ 0` (`--satiation-sensitivity`,
/// `--recognition-distance`; `0` is each mechanism's off limit).
pub fn parse_non_negative(flag: &str, raw: &str) -> Result<f32, String> {
    raw.parse::<f32>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or_else(|| format!("{flag} {raw:?} must be a finite number ≥ 0"))
}

/// Parse a `--founder-aggregation` value: a number in `[0, 1]`.
pub fn parse_founder_aggregation(raw: &str) -> Result<f32, String> {
    raw.parse::<f32>()
        .ok()
        .filter(|a| (0.0..=1.0).contains(a))
        .ok_or_else(|| format!("--founder-aggregation {raw:?} must be a number in [0, 1]"))
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
        let key = match (tok.contains(':'), bare) {
            (false, Some(source)) => parse_index(tok).map(|index| (source, index)),
            _ => parse_config_key(tok),
        };
        set.insert(key.unwrap_or_else(|e| panic!("{var} {e}")));
    }
    set
}

/// Parse one `source:index` key (`atlas:3`, `sample:12`, `sample@9421:12`) —
/// the grammar of a [`parse_selector`] token, and of the config reference
/// `export_recipe` takes (#581), so one reference names one config in every
/// bin.
pub fn parse_config_key(tok: &str) -> Result<(ConfigSource, usize), String> {
    let (label, idx) = tok
        .split_once(':')
        .ok_or_else(|| format!("token {tok:?} is not source:index"))?;
    Ok((label.parse()?, parse_index(idx)?))
}

fn parse_index(idx: &str) -> Result<usize, String> {
    idx.parse()
        .map_err(|_| format!("index {idx:?} is not a usize"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A decoded world founds aggregated (#601); a measurement comparing
    /// against a pre-#601 tree pins it back to the well-mixed scatter (#605).
    #[test]
    fn a_pinned_founder_aggregation_overrides_the_decoded_default() {
        let sampled = sampled_units(default_ranges().len());
        let decoded = resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        assert_eq!(
            decoded.1.founder_aggregation,
            explorers_sim::DEFAULT_FOUNDER_AGGREGATION
        );

        let unchanged = with_founder_aggregation(decoded.clone(), None);
        assert_eq!(unchanged, decoded, "no pin leaves the world as decoded");

        let (params, dist) = with_founder_aggregation(decoded.clone(), Some(0.0));
        assert_eq!(dist.founder_aggregation, 0.0);
        assert_eq!(params, decoded.0, "only the founding placement moves");
        let mut as_decoded = decoded.1.clone();
        as_decoded.founder_aggregation = 0.0;
        assert_eq!(dist, as_decoded);
    }

    /// #619 probes whether the need gate and recognition fail on scale: a
    /// pinned satiation sensitivity or recognition distance overrides only
    /// that parameter, and no pin leaves the world as decoded.
    #[test]
    fn pinned_consumption_scales_override_only_their_own_parameter() {
        let sampled = sampled_units(default_ranges().len());
        let decoded = resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        assert_eq!(
            with_consumption_scales(decoded.clone(), None, None),
            decoded
        );

        let (params, dist) = with_consumption_scales(decoded.clone(), Some(1.0), None);
        assert_eq!(dist, decoded.1);
        let mut want = decoded.0.clone();
        want.satiation_sensitivity = 1.0;
        assert_eq!(params, want);

        let (params, dist) = with_consumption_scales(decoded.clone(), None, Some(1.5));
        assert_eq!(dist, decoded.1);
        let mut want = decoded.0.clone();
        want.recognition_distance = 1.5;
        assert_eq!(params, want);
    }

    /// #645 measures size-scaled uptake (#644) on the atlas: a pinned
    /// exponent or reference structure overrides only its own parameter, and
    /// no pin leaves the world as decoded (b = 0, the size-blind rule).
    #[test]
    fn pinned_uptake_scaling_overrides_only_its_own_parameter() {
        let sampled = sampled_units(default_ranges().len());
        let decoded = resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        assert_eq!(decoded.0.uptake_structure_exponent, 0.0);
        assert_eq!(with_uptake_scaling(decoded.clone(), None, None), decoded);

        let (params, dist) = with_uptake_scaling(decoded.clone(), Some(2.0 / 3.0), None);
        assert_eq!(dist, decoded.1);
        let mut want = decoded.0.clone();
        want.uptake_structure_exponent = 2.0 / 3.0;
        assert_eq!(params, want);

        let (params, dist) = with_uptake_scaling(decoded.clone(), Some(1.0), Some(140.0));
        assert_eq!(dist, decoded.1);
        let mut want = decoded.0.clone();
        want.uptake_structure_exponent = 1.0;
        want.uptake_reference_structure = 140.0;
        assert_eq!(params, want);

        let (params, _) = with_uptake_scaling(decoded.clone(), None, Some(40.0));
        let mut want = decoded.0.clone();
        want.uptake_reference_structure = 40.0;
        assert_eq!(params, want);
    }

    /// #646 measures the network (flow 5) on the atlas: each pinned network
    /// parameter overrides only itself, and no pin leaves the world as decoded
    /// (connection cap 0, the network off).
    #[test]
    fn pinned_network_settings_override_only_their_own_parameters() {
        let sampled = sampled_units(default_ranges().len());
        let decoded = resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        assert_eq!(decoded.0.network_connection_cap, 0);
        assert_eq!(
            with_network(decoded.clone(), &NetworkPins::default()),
            decoded
        );

        let pins = NetworkPins {
            connection_cap: Some(4),
            creation_cost: Some(1.0),
            maintenance_cost: Some(0.05),
            redistribution_rate: Some(0.2),
            transfer_efficiency: Some(0.9),
        };
        let (params, dist) = with_network(decoded.clone(), &pins);
        assert_eq!(dist, decoded.1);
        let mut want = decoded.0.clone();
        want.network_connection_cap = 4;
        want.network_creation_cost = 1.0;
        want.network_maintenance_cost = 0.05;
        want.network_redistribution_rate = 0.2;
        want.network_transfer_efficiency = 0.9;
        assert_eq!(params, want);

        let (params, _) = with_network(
            decoded.clone(),
            &NetworkPins {
                redistribution_rate: Some(0.5),
                ..Default::default()
            },
        );
        let mut want = decoded.0.clone();
        want.network_redistribution_rate = 0.5;
        assert_eq!(params, want);
    }

    #[test]
    fn a_transfer_efficiency_flag_value_must_lie_in_the_unit_interval() {
        assert_eq!(parse_unit_interval("--x", "0.9"), Ok(0.9));
        assert_eq!(parse_unit_interval("--x", "0"), Ok(0.0));
        assert_eq!(parse_unit_interval("--x", "1"), Ok(1.0));
        assert!(parse_unit_interval("--x", "1.1").is_err());
        assert!(parse_unit_interval("--x", "-0.1").is_err());
        assert!(parse_unit_interval("--x", "NaN").is_err());
    }

    #[test]
    fn a_reference_structure_flag_value_must_be_a_positive_number() {
        assert_eq!(parse_positive("--x", "40"), Ok(40.0));
        assert_eq!(parse_positive("--x", "0.5"), Ok(0.5));
        assert!(parse_positive("--x", "0").is_err());
        assert!(parse_positive("--x", "-1").is_err());
        assert!(parse_positive("--x", "NaN").is_err());
        assert!(parse_positive("--x", "inf").is_err());
        assert!(parse_positive("--x", "x").is_err());
    }

    #[test]
    fn a_consumption_scale_flag_value_must_be_a_non_negative_number() {
        assert_eq!(parse_non_negative("--x", "0"), Ok(0.0));
        assert_eq!(parse_non_negative("--x", "3"), Ok(3.0));
        assert!(parse_non_negative("--x", "-0.1").is_err());
        assert!(parse_non_negative("--x", "NaN").is_err());
        assert!(parse_non_negative("--x", "inf").is_err());
        assert!(parse_non_negative("--x", "x").is_err());
    }

    #[test]
    fn a_founder_aggregation_flag_value_must_lie_in_the_unit_interval() {
        assert_eq!(parse_founder_aggregation("0"), Ok(0.0));
        assert_eq!(parse_founder_aggregation("0.8"), Ok(0.8));
        assert!(parse_founder_aggregation("1.5").is_err());
        assert!(parse_founder_aggregation("-0.1").is_err());
        assert!(parse_founder_aggregation("NaN").is_err());
        assert!(parse_founder_aggregation("x").is_err());
    }

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
