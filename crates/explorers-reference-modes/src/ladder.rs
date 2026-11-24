//! Reading by the ladder (#857; `docs/system-design/end-state.md`, *Reading
//! rules*, and *The decomposer guild*, *Its ladder*): seeds run in sequence
//! under a declared "at least k of n" rule, stopping as soon as the outcome
//! is decided either way.

/// What reaching the failure bound means: the reading is refuted (failed),
/// or a rule that declares no refuted line leaves it undecided.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnFailure {
    Fails,
    Undecided,
}

/// A declared rule over a seed sequence: at least `needed` of `seeds` pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SeedRule {
    pub needed: usize,
    pub seeds: usize,
    pub on_failure: OnFailure,
}

/// The outcome of a reading under a [`SeedRule`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// At least `needed` seeds passed.
    Passed,
    /// Too many failed for `needed` to be reached, under [`OnFailure::Fails`].
    Failed,
    /// Too many failed, under [`OnFailure::Undecided`].
    Undecided,
}

impl SeedRule {
    /// At least `needed` of `seeds`. Panics unless `1 <= needed <= seeds`.
    pub fn at_least(needed: usize, seeds: usize, on_failure: OnFailure) -> Self {
        assert!(
            (1..=seeds).contains(&needed),
            "a rule needs between 1 and its seeds passes"
        );
        Self {
            needed,
            seeds,
            on_failure,
        }
    }

    /// The outcome once `passes` and `failures` decide it: `None` while
    /// both bounds are still open.
    pub fn decided(&self, passes: usize, failures: usize) -> Option<Outcome> {
        if passes >= self.needed {
            Some(Outcome::Passed)
        } else if failures > self.seeds - self.needed {
            Some(match self.on_failure {
                OnFailure::Fails => Outcome::Failed,
                OnFailure::Undecided => Outcome::Undecided,
            })
        } else {
            None
        }
    }

    /// The outcome of the full reading: every one of the rule's seeds run,
    /// `outcomes` their passes in order. Panics unless there is one outcome
    /// per seed.
    pub fn verdict_on_all(&self, outcomes: &[bool]) -> Outcome {
        assert_eq!(outcomes.len(), self.seeds, "one outcome per seed");
        let passes = outcomes.iter().filter(|&&p| p).count();
        self.decided(passes, self.seeds - passes)
            .expect("every seed run decides the rule")
    }
}

/// A reading of seeds in sequence under a [`SeedRule`].
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SequentialReading {
    pub rule: SeedRule,
    pub outcome: Outcome,
    pub passes: usize,
    pub failures: usize,
    /// Each seed run, in order, and whether it passed.
    pub seeds: Vec<(u64, bool)>,
}

impl SequentialReading {
    /// How many seeds were run before the outcome was decided.
    pub fn seeds_run(&self) -> usize {
        self.seeds.len()
    }
}

/// Run `seeds` one at a time through `run` (true for a pass), stopping as
/// soon as `rule` is decided either way. Panics if `seeds` yields fewer than
/// the rule's seeds before it is decided.
pub fn read_in_sequence(
    rule: SeedRule,
    seeds: impl IntoIterator<Item = u64>,
    mut run: impl FnMut(u64) -> bool,
) -> SequentialReading {
    let (mut passes, mut failures, mut ran) = (0, 0, Vec::new());
    let mut seeds = seeds.into_iter();
    loop {
        if let Some(outcome) = rule.decided(passes, failures) {
            return SequentialReading {
                rule,
                outcome,
                passes,
                failures,
                seeds: ran,
            };
        }
        let seed = seeds.next().expect("the rule's seeds are given");
        let passed = run(seed);
        if passed {
            passes += 1;
        } else {
            failures += 1;
        }
        ran.push((seed, passed));
    }
}

/// The summary section of a sequential reading.
pub fn sequential_reading_section(r: &SequentialReading) -> String {
    let outcome = match r.outcome {
        Outcome::Passed => "passed",
        Outcome::Failed => "failed",
        Outcome::Undecided => "undecided",
    };
    let seeds: Vec<String> = (r.seeds.iter())
        .map(|(s, p)| format!("{s}: {}", if *p { "pass" } else { "fail" }))
        .collect();
    format!(
        "## Seeds in sequence (--seeds, --passes-needed)\n\n\
         rule: at least {} of {} seeds pass; the failure bound leaves it {} | \
         outcome: {outcome} after {} seeds ({} passes, {} failures) | {}\n",
        r.rule.needed,
        r.rule.seeds,
        match r.rule.on_failure {
            OnFailure::Fails => "failed",
            OnFailure::Undecided => "undecided",
        },
        r.seeds_run(),
        r.passes,
        r.failures,
        seeds.join(", "),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake reading of seed `s`: passes where `outcomes[s - 1]` is true.
    fn reading(rule: SeedRule, outcomes: &[bool]) -> SequentialReading {
        read_in_sequence(rule, 1..=outcomes.len() as u64, |s| {
            outcomes[(s - 1) as usize]
        })
    }

    #[test]
    fn a_sequential_reading_stops_once_k_passes_are_reached() {
        let rule = SeedRule::at_least(3, 5, OnFailure::Fails);
        let r = reading(rule, &[true, false, true, true, false]);
        assert_eq!(r.outcome, Outcome::Passed);
        assert_eq!(r.seeds_run(), 4);
        assert_eq!(r.passes, 3);
        assert_eq!(r.failures, 1);
        assert_eq!(r.seeds, vec![(1, true), (2, false), (3, true), (4, true)]);
    }

    /// Mode 1's rule declares no refuted line: reaching the failure bound
    /// stops the sequence, undecided. The guild's refutes.
    #[test]
    fn a_sequential_reading_stops_at_the_failure_bound_failed_or_undecided() {
        let outcomes = [false, true, false, false, true];
        let refuting = reading(SeedRule::at_least(3, 5, OnFailure::Fails), &outcomes);
        assert_eq!(refuting.outcome, Outcome::Failed);
        assert_eq!(refuting.seeds_run(), 4);
        let open = reading(SeedRule::at_least(3, 5, OnFailure::Undecided), &outcomes);
        assert_eq!(open.outcome, Outcome::Undecided);
        assert_eq!(open.seeds_run(), 4);
    }

    /// The property behind the early stop: for every sequence of per-seed
    /// outcomes, every "k of n" and both failure meanings, the sequential
    /// reading's outcome is the outcome of running all n, and it runs no
    /// seed after the outcome is decided.
    #[test]
    fn a_sequential_reading_gives_the_full_readings_outcome_on_every_sequence() {
        for n in 1..=7usize {
            for k in 1..=n {
                for on_failure in [OnFailure::Fails, OnFailure::Undecided] {
                    let rule = SeedRule::at_least(k, n, on_failure);
                    for bits in 0..1u32 << n {
                        let outcomes: Vec<bool> = (0..n).map(|i| bits >> i & 1 == 1).collect();
                        let mut ran = 0;
                        let r = read_in_sequence(rule, 1..=n as u64, |s| {
                            ran += 1;
                            outcomes[(s - 1) as usize]
                        });
                        let case = format!("{k} of {n}, {on_failure:?}, {outcomes:?}");
                        assert_eq!(r.outcome, rule.verdict_on_all(&outcomes), "{case}");
                        assert_eq!(ran, r.seeds_run(), "{case}");
                        let before_last = &r.seeds[..r.seeds_run() - 1];
                        let p = before_last.iter().filter(|s| s.1).count();
                        assert_eq!(
                            rule.decided(p, before_last.len() - p),
                            None,
                            "undecided before its last seed: {case}"
                        );
                    }
                }
            }
        }
    }
}
