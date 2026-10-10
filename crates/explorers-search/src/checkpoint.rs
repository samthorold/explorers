//! Checkpoint and exact resume for the QD atlas search (#530).
//!
//! A regeneration at the settled horizon runs for hours, and the archive lives
//! only in memory until the search returns. [`run_qd_checkpointed`] writes the
//! search's whole loop-carried state ([`SearchState`]) to disk at every
//! generation boundary; [`resume_qd`] reads it back and runs on. The resumed
//! search produces **exactly** the atlas the uninterrupted one would have — a
//! plausible-but-different atlas would be worse than no resume, because the
//! difference is invisible.

use std::path::{Path, PathBuf};

use explorers_genesis::BloomStop;
use rand_chacha::ChaCha8Rng;

use crate::qd::{Atlas, GenerationReport, QdConfig, SearchState};
use crate::search::{FixedParameters, Scale};

/// The checkpoint format version. Bump it whenever the written form of
/// [`SearchState`] or the stamp changes; a checkpoint from any other version is
/// refused rather than parsed. Version 2 (#699): a cell's recorded fitness is
/// the mean over its seeds, not the median seed's, so a version-1 archive
/// cannot be continued without building a hybrid atlas. Version 3 (#710): the
/// same again for `L² · F` over the plain mean, so a mean-scored archive does
/// not resume into a live-fraction-scored search.
pub const SCHEMA_VERSION: u32 = 3;

/// Why a checkpoint could not be written or resumed from.
#[derive(Debug)]
pub enum CheckpointError {
    /// Reading, writing or renaming the checkpoint file failed.
    Io {
        path: PathBuf,
        error: std::io::Error,
    },
    /// The checkpoint did not parse as a checkpoint.
    Malformed {
        path: PathBuf,
        error: serde_json::Error,
    },
    /// The checkpoint carries another format version (`found`), or none at all.
    UnsupportedSchema { path: PathBuf, found: Option<u64> },
    /// The search state would not read back exactly as written (a non-finite
    /// value, which JSON cannot carry, is the case this guards), so no
    /// checkpoint was written: an unresumable checkpoint must not stand in for
    /// a good one.
    Unfaithful { path: PathBuf, reason: String },
    /// The checkpoint was written by a search under a different configuration
    /// than the one resuming it. Each entry names a knob and both values.
    ConfigMismatch {
        path: PathBuf,
        mismatches: Vec<String>,
    },
}

impl std::fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckpointError::Io { path, error } => {
                write!(f, "checkpoint {}: {error}", path.display())
            }
            CheckpointError::Malformed { path, error } => {
                write!(f, "checkpoint {} is malformed: {error}", path.display())
            }
            CheckpointError::UnsupportedSchema { path, found } => match found {
                Some(v) => write!(
                    f,
                    "checkpoint {} has schema version {v}; this build reads version \
                     {SCHEMA_VERSION} only",
                    path.display()
                ),
                None => write!(
                    f,
                    "{} is not a QD search checkpoint (it carries no schema version)",
                    path.display()
                ),
            },
            CheckpointError::Unfaithful { path, reason } => write!(
                f,
                "refusing to write checkpoint {}: the search state would not read back \
                 exactly ({reason}); the previous checkpoint is left in place",
                path.display()
            ),
            CheckpointError::ConfigMismatch { path, mismatches } => write!(
                f,
                "checkpoint {} was written under a different search configuration, \
                 refusing to resume from it: {}",
                path.display(),
                mismatches.join("; ")
            ),
        }
    }
}

impl std::error::Error for CheckpointError {}

/// Run the QD search from scratch, writing a checkpoint to `path` at every
/// generation boundary. The atlas is the one [`crate::qd::run_qd_observed`]
/// returns for the same `(config, base_seed, rng)`.
pub fn run_qd_checkpointed(
    config: &QdConfig,
    base_seed: u64,
    rng: ChaCha8Rng,
    path: &Path,
    observer: &mut impl FnMut(&GenerationReport),
) -> Result<Atlas, CheckpointError> {
    let stamp = Stamp::of(config, base_seed);
    let state = SearchState::start(config, rng);
    state.run(config, base_seed, observer, &mut |state| {
        write(path, &CheckpointRef::new(&stamp, state))
    })
}

/// Resume the QD search from the checkpoint at `path`, continuing to checkpoint
/// there, and return the atlas the uninterrupted search would have returned.
pub fn resume_qd(
    config: &QdConfig,
    base_seed: u64,
    path: &Path,
    observer: &mut impl FnMut(&GenerationReport),
) -> Result<Atlas, CheckpointError> {
    let Checkpoint { stamp, state } = read(path)?;
    let mismatches = stamp.mismatches(&Stamp::of(config, base_seed));
    if !mismatches.is_empty() {
        return Err(CheckpointError::ConfigMismatch {
            path: path.to_path_buf(),
            mismatches,
        });
    }
    state.run(config, base_seed, observer, &mut |state| {
        write(path, &CheckpointRef::new(&stamp, state))
    })
}

/// What a checkpoint says about where its search stands.
#[derive(Clone, Debug)]
pub struct CheckpointSummary {
    /// The generation a resume runs next (`generations + 1` once the search
    /// has finished).
    pub next_generation: usize,
}

/// Read the checkpoint at `path` without resuming from it.
pub fn inspect(path: &Path) -> Result<CheckpointSummary, CheckpointError> {
    let checkpoint = read(path)?;
    Ok(CheckpointSummary {
        next_generation: checkpoint.state.generation,
    })
}

/// The configuration a checkpoint was written under. A resume must match it
/// exactly: continuing a search under different knobs would build a hybrid
/// atlas that reads as data.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Stamp {
    base_seed: u64,
    ranges: Vec<RangeStamp>,
    max_ticks: u64,
    ensemble_size: u32,
    batch: usize,
    generations: usize,
    sigma: f64,
    archive_learning_rate: f32,
    prefilter_crosscheck_fraction: f32,
    early_stop_crosscheck_fraction: f32,
    carcass_seed_count: usize,
    /// Absent from a checkpoint written before #573, which ran without one.
    #[serde(default)]
    bloom_stop: Option<BloomStop>,
    /// Absent from a checkpoint written before #699, which ran no top-up.
    #[serde(default)]
    top_up_screen: Option<u32>,
    /// The parameters held fixed outside the box (#716). Absent from a
    /// checkpoint written before it, whose search held none.
    #[serde(default)]
    fixed: FixedParameters,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct RangeStamp {
    name: String,
    min: f64,
    max: f64,
    /// Absent from a checkpoint written before #701, whose ranges were all
    /// linear; written only when not linear, as the atlas writes it.
    #[serde(default, skip_serializing_if = "Scale::is_linear")]
    scale: Scale,
}

impl Stamp {
    fn of(config: &QdConfig, base_seed: u64) -> Self {
        Stamp {
            base_seed,
            ranges: config
                .ranges
                .iter()
                .map(|r| RangeStamp {
                    name: r.name.clone(),
                    min: r.min,
                    max: r.max,
                    scale: r.scale,
                })
                .collect(),
            max_ticks: config.max_ticks,
            ensemble_size: config.ensemble_size,
            batch: config.batch,
            generations: config.generations,
            sigma: config.sigma,
            archive_learning_rate: config.archive_learning_rate,
            prefilter_crosscheck_fraction: config.prefilter_crosscheck_fraction,
            early_stop_crosscheck_fraction: config.early_stop_crosscheck_fraction,
            carcass_seed_count: config.carcass_seed_count,
            bloom_stop: config.bloom_stop,
            top_up_screen: config.top_up_screen,
            fixed: config.fixed.clone(),
        }
    }

    /// Every knob on which `self` (the checkpoint's) differs from `invocation`,
    /// named with both values.
    fn mismatches(&self, invocation: &Stamp) -> Vec<String> {
        let mut out = Vec::new();
        macro_rules! compare {
            ($($field:ident),*) => {$(
                if self.$field != invocation.$field {
                    out.push(format!(
                        "{} (checkpoint {:?}, this run {:?})",
                        stringify!($field),
                        self.$field,
                        invocation.$field
                    ));
                }
            )*};
        }
        compare!(
            base_seed,
            max_ticks,
            ensemble_size,
            batch,
            generations,
            sigma,
            archive_learning_rate,
            prefilter_crosscheck_fraction,
            early_stop_crosscheck_fraction,
            carcass_seed_count,
            bloom_stop,
            top_up_screen,
            fixed
        );
        if self.ranges != invocation.ranges {
            out.push(match self.ranges.len() == invocation.ranges.len() {
                true => {
                    let differing: Vec<String> = self
                        .ranges
                        .iter()
                        .zip(&invocation.ranges)
                        .filter(|(a, b)| a != b)
                        .map(|(a, b)| format!("{a:?} vs {b:?}"))
                        .collect();
                    format!("ranges (checkpoint vs this run: {})", differing.join(", "))
                }
                false => format!(
                    "ranges (checkpoint has {} dimensions, this run {})",
                    self.ranges.len(),
                    invocation.ranges.len()
                ),
            });
        }
        out
    }
}

/// The checkpoint as read: the stamp it is validated against, and the search
/// state it resumes. (Its `schema_version` is checked before this is parsed.)
#[derive(serde::Deserialize)]
struct Checkpoint {
    stamp: Stamp,
    state: SearchState<ChaCha8Rng>,
}

/// [`Checkpoint`]'s written form, borrowed from the running search.
#[derive(serde::Serialize)]
struct CheckpointRef<'a> {
    schema_version: u32,
    stamp: &'a Stamp,
    state: &'a SearchState<ChaCha8Rng>,
}

impl<'a> CheckpointRef<'a> {
    fn new(stamp: &'a Stamp, state: &'a SearchState<ChaCha8Rng>) -> Self {
        CheckpointRef {
            schema_version: SCHEMA_VERSION,
            stamp,
            state,
        }
    }
}

fn read(path: &Path) -> Result<Checkpoint, CheckpointError> {
    let text = std::fs::read_to_string(path).map_err(|error| CheckpointError::Io {
        path: path.to_path_buf(),
        error,
    })?;
    let malformed = |error| CheckpointError::Malformed {
        path: path.to_path_buf(),
        error,
    };
    // The version is read on its own first: a body from another version may
    // well parse, and parse wrongly.
    let json: serde_json::Value = serde_json::from_str(&text).map_err(malformed)?;
    let found = json.get("schema_version").and_then(|v| v.as_u64());
    if found != Some(u64::from(SCHEMA_VERSION)) {
        return Err(CheckpointError::UnsupportedSchema {
            path: path.to_path_buf(),
            found,
        });
    }
    serde_json::from_value(json).map_err(malformed)
}

/// Write the checkpoint atomically: serialise to a temporary file beside
/// `path`, flush it to disk, then rename it over `path`. A kill at any point
/// leaves either the previous complete checkpoint or the new complete one —
/// never a truncated file.
fn write(path: &Path, checkpoint: &CheckpointRef) -> Result<(), CheckpointError> {
    use std::io::Write;

    let json = serde_json::to_string(checkpoint).expect("a checkpoint serialises");
    // Read it back before it replaces anything: the resume must see exactly
    // this state, and a value JSON cannot carry would otherwise surface only
    // when the resume is attempted, hours of search too late.
    let unfaithful = |reason: String| CheckpointError::Unfaithful {
        path: path.to_path_buf(),
        reason,
    };
    let back: Checkpoint = serde_json::from_str(&json).map_err(|e| unfaithful(e.to_string()))?;
    let rewritten = serde_json::to_string(&CheckpointRef::new(&back.stamp, &back.state))
        .expect("a checkpoint serialises");
    if rewritten != json {
        return Err(unfaithful("it re-serialises differently".to_string()));
    }
    let mut temp_name = path.file_name().unwrap_or_default().to_os_string();
    temp_name.push(".tmp");
    let temp = path.with_file_name(temp_name);
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |error| CheckpointError::Io { path, error }
    };

    let mut file = std::fs::File::create(&temp).map_err(io(&temp))?;
    file.write_all(json.as_bytes()).map_err(io(&temp))?;
    file.sync_all().map_err(io(&temp))?;
    drop(file);
    std::fs::rename(&temp, path).map_err(io(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qd::{Atlas, GenerationReport, QdConfig, run_qd};
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use std::path::PathBuf;

    /// A fresh, empty scratch directory for one test.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "explorers-qd-checkpoint-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The atlas as written — in its canonical order (#536), so compared whole.
    fn written(atlas: &Atlas) -> serde_json::Value {
        serde_json::to_value(atlas).unwrap()
    }

    fn tiny() -> QdConfig {
        QdConfig {
            ensemble_size: 1,
            max_ticks: 20,
            batch: 4,
            generations: 3,
            ..QdConfig::default()
        }
    }

    /// A checkpoint written before #573 has no `bloom_stop`. It ran without
    /// one, so it resumes under a config without one (`--no-bloom-stop`),
    /// and is refused under the default, which has one.
    #[test]
    fn a_stamp_from_before_the_bloom_stop_reads_as_none() {
        let without = QdConfig {
            bloom_stop: None,
            ..tiny()
        };
        let mut old = serde_json::to_value(Stamp::of(&without, 42)).unwrap();
        old.as_object_mut().unwrap().remove("bloom_stop");
        let old: Stamp = serde_json::from_value(old).unwrap();
        assert!(old.mismatches(&Stamp::of(&without, 42)).is_empty());
        assert_eq!(old.mismatches(&Stamp::of(&tiny(), 42)).len(), 1);
    }

    /// #701: a checkpoint written under #677's atlas's box (34
    /// coordinates, the last `c_AH`, no scales recorded) is refused by a search
    /// under the leached box, which has as many coordinates with `λ` last; so
    /// is one whose box differs only in a coordinate's scale. A pre-#701 stamp
    /// still resumes under its own box.
    #[test]
    fn a_checkpoint_from_another_box_of_the_same_length_is_refused() {
        let taxed = QdConfig {
            ranges: crate::search::taxed_ranges(),
            ..tiny()
        };
        let leached = || QdConfig {
            ranges: crate::search::leached_ranges(),
            ..tiny()
        };
        let old = serde_json::to_value(Stamp::of(&taxed, 42)).unwrap();
        assert!(
            !old.to_string().contains("scale"),
            "a linear box stamps no scale"
        );
        let old: Stamp = serde_json::from_value(old).unwrap();
        assert!(old.mismatches(&Stamp::of(&taxed, 42)).is_empty());
        let refused = old.mismatches(&Stamp::of(&leached(), 42));
        assert_eq!(refused.len(), 1);
        assert!(refused[0].contains("cross_trait_cost"), "{refused:?}");

        let mut linear = leached();
        linear.ranges.last_mut().unwrap().scale = Scale::Linear;
        let refused = Stamp::of(&linear, 42).mismatches(&Stamp::of(&leached(), 42));
        assert_eq!(refused.len(), 1, "{refused:?}");
    }

    /// #773: a checkpoint from a search under the untaxed box, genesis's box
    /// before its trophic-efficiency coordinates were bounded, is refused by
    /// a search under the bounded box: the boxes have the same coordinates in
    /// the same order, and the stamp tells them apart by those two ranges.
    #[test]
    fn a_checkpoint_from_the_box_before_the_trophic_bound_is_refused() {
        let untaxed = QdConfig {
            ranges: crate::search::untaxed_ranges(),
            ..tiny()
        };
        let refused = Stamp::of(&untaxed, 42).mismatches(&Stamp::of(&tiny(), 42));
        assert_eq!(refused.len(), 1, "{refused:?}");
        assert!(refused[0].starts_with("ranges"), "{refused:?}");
        assert!(
            refused[0].contains("base_trophic_efficiency"),
            "{refused:?}"
        );
        assert!(refused[0].contains("trophic_distance_decay"), "{refused:?}");
    }

    /// #716: a checkpoint from a search with `λ` in its box (#686's, #711's)
    /// is refused by a search under genesis's box, which holds `λ` fixed
    /// outside it; so is one written before #716 under the untaxed box, whose
    /// stamp records no fixed rate and whose worlds ran at `λ = 0`.
    #[test]
    fn a_checkpoint_from_a_search_with_lambda_in_the_box_is_refused() {
        let leached = QdConfig {
            ranges: crate::search::leached_ranges(),
            fixed: FixedParameters::none(),
            ..tiny()
        };
        let refused = Stamp::of(&leached, 42).mismatches(&Stamp::of(&tiny(), 42));
        assert!(
            refused.iter().any(|m| m.starts_with("ranges")),
            "{refused:?}"
        );
        assert!(
            refused.iter().any(|m| m.starts_with("fixed")),
            "{refused:?}"
        );

        let mut old = serde_json::to_value(Stamp::of(&tiny(), 42)).unwrap();
        old.as_object_mut().unwrap().remove("fixed");
        let old: Stamp = serde_json::from_value(old).unwrap();
        let refused = old.mismatches(&Stamp::of(&tiny(), 42));
        assert_eq!(refused.len(), 1, "{refused:?}");
        assert!(refused[0].starts_with("fixed"), "{refused:?}");
        let untaxed_at_zero = QdConfig {
            fixed: FixedParameters::none(),
            ..tiny()
        };
        assert!(old.mismatches(&Stamp::of(&untaxed_at_zero, 42)).is_empty());
    }

    #[test]
    fn slow_an_interrupted_and_resumed_search_writes_the_uninterrupted_atlas() {
        // The whole point of #530: stop the search at a generation boundary,
        // resume from what was on disk there, and the atlas is the one the
        // uninterrupted search writes — not a plausible neighbour of it.
        let config = tiny();
        let dir = scratch("resume-equivalence");
        let live = dir.join("search.checkpoint.json");
        let interrupted = dir.join("after-generation-1.json");

        let uninterrupted = run_qd(&config, 42, &mut ChaCha8Rng::seed_from_u64(42));

        // The checkpoint for a generation is on disk by the time its report
        // fires, so copying it aside there is the state a kill would leave.
        let checkpointed = run_qd_checkpointed(
            &config,
            42,
            ChaCha8Rng::seed_from_u64(42),
            &live,
            &mut |r: &GenerationReport| {
                if r.generation == 1 {
                    std::fs::copy(&live, &interrupted).unwrap();
                }
            },
        )
        .unwrap();
        let resumed = resume_qd(&config, 42, &interrupted, &mut |_: &GenerationReport| {}).unwrap();

        assert_eq!(written(&checkpointed), written(&uninterrupted));
        assert_eq!(written(&resumed), written(&uninterrupted));
    }

    #[test]
    fn a_state_that_would_not_read_back_exactly_is_refused_and_the_last_checkpoint_kept() {
        // JSON carries no NaN or infinity (serde writes `null`, which reads back
        // as an error). If one ever reached the search state, a checkpoint
        // written anyway would be unresumable — so the write refuses, loudly,
        // and the previous complete checkpoint stays on disk.
        let config = tiny();
        let path = scratch("non-finite").join("search.checkpoint.json");
        let stamp = Stamp::of(&config, 42);
        let mut state = SearchState::start(&config, ChaCha8Rng::seed_from_u64(42));
        write(&path, &CheckpointRef::new(&stamp, &state)).unwrap();
        let good = std::fs::read(&path).unwrap();

        state.poison_emitter_for_test(f64::NAN);
        let err = write(&path, &CheckpointRef::new(&stamp, &state)).unwrap_err();
        assert!(matches!(err, CheckpointError::Unfaithful { .. }), "{err}");
        assert_eq!(std::fs::read(&path).unwrap(), good);
        assert_eq!(inspect(&path).unwrap().next_generation, 0);
    }

    #[test]
    fn the_checkpoint_serialiser_carries_every_finite_float_exactly() {
        // Emitter means and deviations, elite units and thresholds all cross the
        // checkpoint as JSON numbers; one ULP lost in any of them and the
        // resumed search is a different search. Sweep random bit patterns
        // across the whole finite range, both widths.
        use rand::Rng;
        let mut rng = ChaCha8Rng::seed_from_u64(530);
        for _ in 0..200_000 {
            let x = f64::from_bits(rng.random::<u64>());
            if x.is_finite() {
                let back: f64 = serde_json::from_str(&serde_json::to_string(&x).unwrap()).unwrap();
                assert_eq!(back.to_bits(), x.to_bits(), "{x:e}");
            }
            let y = f32::from_bits(rng.random::<u32>());
            if y.is_finite() {
                let back: f32 = serde_json::from_str(&serde_json::to_string(&y).unwrap()).unwrap();
                assert_eq!(back.to_bits(), y.to_bits(), "{y:e}");
            }
        }
    }

    /// A finished tiny search's checkpoint, for tests that only need one on disk.
    fn finished_checkpoint(name: &str, base_seed: u64) -> PathBuf {
        let path = scratch(name).join("search.checkpoint.json");
        run_qd_checkpointed(
            &tiny(),
            base_seed,
            ChaCha8Rng::seed_from_u64(base_seed),
            &path,
            &mut |_: &GenerationReport| {},
        )
        .unwrap();
        path
    }

    #[test]
    fn slow_resuming_a_finished_search_returns_its_atlas_without_running_a_generation() {
        // The last boundary is checkpointed too, so a kill after the search but
        // before the atlas reached disk costs nothing to recover.
        let path = finished_checkpoint("finished", 42);
        assert_eq!(
            inspect(&path).unwrap().next_generation,
            tiny().generations + 1
        );
        let uninterrupted = run_qd(&tiny(), 42, &mut ChaCha8Rng::seed_from_u64(42));

        let resumed = resume_qd(&tiny(), 42, &path, &mut |_: &GenerationReport| {
            panic!("a finished search has no generation left to run")
        })
        .unwrap();
        assert_eq!(written(&resumed), written(&uninterrupted));
    }

    #[test]
    fn slow_resuming_under_a_different_configuration_is_refused_naming_the_mismatch() {
        // A hybrid atlas half-built under two configurations would later be read
        // as data. Every stamped knob that differs is refused, by name, and the
        // checkpoint is left as it was.
        let path = finished_checkpoint("stamp-mismatch", 42);
        let before = std::fs::read(&path).unwrap();

        let cases: Vec<(&str, QdConfig, u64)> = vec![
            ("batch", QdConfig { batch: 6, ..tiny() }, 42),
            ("base_seed", tiny(), 43),
            (
                "generations",
                QdConfig {
                    generations: 5,
                    ..tiny()
                },
                42,
            ),
            (
                "max_ticks",
                QdConfig {
                    max_ticks: 21,
                    ..tiny()
                },
                42,
            ),
            (
                "ensemble_size",
                QdConfig {
                    ensemble_size: 2,
                    ..tiny()
                },
                42,
            ),
            (
                "sigma",
                QdConfig {
                    sigma: 0.2,
                    ..tiny()
                },
                42,
            ),
            (
                "archive_learning_rate",
                QdConfig {
                    archive_learning_rate: 0.25,
                    ..tiny()
                },
                42,
            ),
            (
                "prefilter_crosscheck_fraction",
                QdConfig {
                    prefilter_crosscheck_fraction: 0.5,
                    ..tiny()
                },
                42,
            ),
            (
                "early_stop_crosscheck_fraction",
                QdConfig {
                    early_stop_crosscheck_fraction: 0.5,
                    ..tiny()
                },
                42,
            ),
            (
                "bloom_stop",
                QdConfig {
                    bloom_stop: Some(explorers_genesis::BloomStop {
                        tick: 300,
                        factor: 5.0,
                    }),
                    ..tiny()
                },
                42,
            ),
            (
                "top_up_screen",
                QdConfig {
                    top_up_screen: Some(1),
                    ..tiny()
                },
                42,
            ),
            (
                "carcass_seed_count",
                QdConfig {
                    carcass_seed_count: 0,
                    ..tiny()
                },
                42,
            ),
            (
                "ranges",
                {
                    let mut config = tiny();
                    config.ranges[3].max += 1.0;
                    config
                },
                42,
            ),
        ];
        for (knob, config, base_seed) in cases {
            let err = resume_qd(&config, base_seed, &path, &mut |_: &GenerationReport| {
                panic!("a refused resume must not run a generation")
            })
            .expect_err(knob);
            assert!(
                matches!(err, CheckpointError::ConfigMismatch { .. }),
                "{knob}: {err}"
            );
            assert!(err.to_string().contains(knob), "{knob} not named in: {err}");
        }
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn slow_a_checkpoint_from_a_mean_scored_search_does_not_resume_under_the_live_fraction_score() {
        // Version 2 archives hold cells scored by the plain mean (#699). The
        // search now scores by `L² · F` (#710), so continuing one would build
        // an atlas ranked by two scores at once.
        let path = finished_checkpoint("mean-scored", 42);
        let mut json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        json["schema_version"] = serde_json::json!(2);
        std::fs::write(&path, json.to_string()).unwrap();

        let err = resume_qd(&tiny(), 42, &path, &mut |_: &GenerationReport| {}).unwrap_err();
        assert!(
            matches!(
                err,
                CheckpointError::UnsupportedSchema { found: Some(2), .. }
            ),
            "{err}"
        );
    }

    #[test]
    fn slow_a_checkpoint_from_another_schema_version_is_refused_not_misparsed() {
        // Rewrite a valid checkpoint's version stamp and nothing else: the body
        // would still parse, which is exactly why the version must be checked
        // first rather than trusting the parse.
        let path = finished_checkpoint("schema-version", 42);
        let mut json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        json["schema_version"] = serde_json::json!(SCHEMA_VERSION + 1);
        std::fs::write(&path, json.to_string()).unwrap();

        let err = resume_qd(&tiny(), 42, &path, &mut |_: &GenerationReport| {}).unwrap_err();
        assert!(
            matches!(err, CheckpointError::UnsupportedSchema { found: Some(v), .. } if v == u64::from(SCHEMA_VERSION + 1)),
            "{err}"
        );

        // A file with no version at all — an atlas.json passed by mistake, say —
        // is refused the same way.
        let atlas_path = path.with_file_name("atlas.json");
        let atlas = run_qd(&tiny(), 42, &mut ChaCha8Rng::seed_from_u64(42));
        std::fs::write(&atlas_path, serde_json::to_string(&atlas).unwrap()).unwrap();
        let err = resume_qd(&tiny(), 42, &atlas_path, &mut |_: &GenerationReport| {}).unwrap_err();
        assert!(
            matches!(err, CheckpointError::UnsupportedSchema { found: None, .. }),
            "{err}"
        );
    }

    #[test]
    fn slow_every_generation_boundary_leaves_a_complete_checkpoint_replaced_whole() {
        // A checkpoint is on disk at every boundary, naming the generation the
        // search runs next. And each one replaces the last whole — a new file
        // renamed into place, never the old one truncated and rewritten — so a
        // kill mid-write leaves the previous checkpoint intact. A hard link to
        // an early checkpoint shows it: rewriting in place would change what
        // the link reads; replacing by rename leaves it untouched.
        let config = tiny();
        let dir = scratch("every-boundary");
        let live = dir.join("search.checkpoint.json");
        let held = dir.join("held-after-generation-0.json");

        let mut next_generations = Vec::new();
        run_qd_checkpointed(
            &config,
            7,
            ChaCha8Rng::seed_from_u64(7),
            &live,
            &mut |r: &GenerationReport| {
                next_generations.push(inspect(&live).unwrap().next_generation);
                if r.generation == 0 {
                    std::fs::hard_link(&live, &held).unwrap();
                }
            },
        )
        .unwrap();

        assert_eq!(
            next_generations,
            (1..=config.generations + 1).collect::<Vec<_>>()
        );
        assert_eq!(inspect(&held).unwrap().next_generation, 1);
        // Nothing but the checkpoint and the held link is left behind.
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(
            names,
            ["held-after-generation-0.json", "search.checkpoint.json"]
        );
    }
}
