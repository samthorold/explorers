//! The genesis evaluator's reading of the running world (#582): the same
//! incremental observation the genesis rollout makes, held by the app beside
//! its ring buffers, so the verdict panel reads the world the way the search
//! would have. Pure observation over `explorers-genesis-eval`; the sim is
//! untouched and nothing here renders.

use explorers_genesis_eval::{
    BloomStop, EvalConfig, FailureMode, FitnessBreakdown, RolloutObservations,
};
use explorers_sim::World;

/// Where the genesis rollout would first have stopped this world: the tick
/// and the gate (extinction, explosion, energy death, nutrient lockup) or
/// the predictive bloom stop that fired on it.
#[derive(Clone, Debug, PartialEq)]
pub struct Stop {
    pub tick: u64,
    pub failure: FailureMode,
}

/// The bloom read behind the predictive bloom stop: how far the world has
/// bloomed over its founders, and whether the rule fired at its tick.
#[derive(Clone, Debug, PartialEq)]
pub struct BloomReading {
    pub founders: usize,
    pub running_peak: usize,
    /// The rule the search applies, `None` if it applies none.
    pub rule: Option<BloomStop>,
    /// Whether the rule fired at its tick; `None` until the world gets there
    /// (or when there is no rule).
    pub fired: Option<bool>,
}

impl BloomReading {
    /// Running peak over founders, the quantity the rule compares with its
    /// factor. 0 for a world with no founders.
    pub fn factor(&self) -> f32 {
        if self.founders == 0 {
            0.0
        } else {
            self.running_peak as f32 / self.founders as f32
        }
    }
}

/// The evaluator's view of one running world, up to its horizon `T`.
///
/// Observation stops at `T`, as the genesis rollout does: the series and
/// snapshots are bounded by the horizon however long the app runs, and the
/// verdict read at `T` is kept as the verdict from then on.
pub struct VerdictObserver {
    config: EvalConfig,
    horizon: u64,
    observations: RolloutObservations,
    /// The living count at tick 0, the bloom stop's reference.
    founders: usize,
    /// The largest living count seen so far, founders included.
    running_peak: usize,
    first_stop: Option<Stop>,
    bloom_fired: Option<bool>,
    /// The verdict read on the world at the horizon tick.
    horizon_verdict: Option<FitnessBreakdown>,
}

impl VerdictObserver {
    /// Start observing `world` (at tick 0) against `config` to `horizon`.
    pub fn new(world: &World, config: EvalConfig, horizon: u64) -> Self {
        let founders = world.agents().len();
        Self {
            observations: RolloutObservations::with_capacity(horizon as usize),
            config,
            horizon,
            founders,
            running_peak: founders,
            first_stop: None,
            bloom_fired: None,
            horizon_verdict: None,
        }
    }

    /// Record one applied step of `world`, as the genesis rollout does after
    /// each `world.step()`, and read the early stops on it in the rollout's
    /// order: the evaluator's incremental gates first, then the bloom stop.
    ///
    /// A no-op once the horizon has been observed; the step that reaches it
    /// also reads the horizon verdict (one clustering pass).
    pub fn observe(&mut self, world: &World) {
        if self.observed_ticks() >= self.horizon {
            return;
        }
        self.observations
            .observe(world, self.config.coexistence_sample_interval);
        let living = world.agents().len();
        self.running_peak = self.running_peak.max(living);
        if let Some(rule) = self.config.bloom_stop
            && world.tick() == rule.tick
        {
            self.bloom_fired = Some(rule.fires(world.tick(), self.founders, self.running_peak));
        }
        if self.first_stop.is_none() {
            // Read off the current parameters: the sliders can move them.
            let sustainable_stock = explorers_genesis_eval::sustainable_stock(world.params());
            let failure = explorers_genesis_eval::early_stop(
                living,
                &self.observations,
                &self.config,
                sustainable_stock,
            )
            .or_else(|| {
                self.config
                    .bloom_stop
                    .filter(|rule| rule.fires(world.tick(), self.founders, self.running_peak))
                    .map(|_| FailureMode::BloomStop)
            });
            self.first_stop = failure.map(|failure| Stop {
                tick: world.tick(),
                failure,
            });
        }
        if self.observed_ticks() == self.horizon {
            self.horizon_verdict = Some(self.evaluate(world));
        }
    }

    /// Ticks observed so far: the world's tick, until it passes the horizon.
    pub fn observed_ticks(&self) -> u64 {
        self.observations.free_energy.len() as u64
    }

    /// The horizon `T` the verdict is read against.
    pub fn horizon(&self) -> u64 {
        self.horizon
    }

    /// The evaluator's verdict at the current tick, read against the horizon
    /// — or, once the horizon has been observed, the verdict read there,
    /// whatever the world has done since. Before the horizon it clusters the
    /// roster and every settled-window snapshot, so it is expensive on a
    /// dense world: call it on demand, not every frame.
    pub fn verdict(&self, world: &World) -> FitnessBreakdown {
        match &self.horizon_verdict {
            Some(verdict) => verdict.clone(),
            None => self.evaluate(world),
        }
    }

    fn evaluate(&self, world: &World) -> FitnessBreakdown {
        explorers_genesis_eval::evaluate_from_log(
            world,
            &self.observations,
            &self.config,
            self.horizon,
        )
    }

    /// The first tick the genesis rollout would have stopped this world, and
    /// why; `None` while it would still be running.
    pub fn first_stop(&self) -> Option<&Stop> {
        self.first_stop.as_ref()
    }

    /// The bloom read so far.
    pub fn bloom(&self) -> BloomReading {
        BloomReading {
            founders: self.founders,
            running_peak: self.running_peak,
            rule: self.config.bloom_stop,
            fired: self.bloom_fired,
        }
    }
}

/// The verdict as the panel reads it: when it was read, the failure mode
/// (or none) and fitness, then the five fitness components.
pub fn verdict_lines(tick: u64, horizon: u64, breakdown: &FitnessBreakdown) -> Vec<String> {
    let failure = match &breakdown.failure {
        Some(mode) => format!("{mode:?}"),
        None => "none (live)".to_owned(),
    };
    vec![
        format!("Read at tick {tick} against T = {horizon}"),
        format!("Failure mode: {failure}"),
        format!("Fitness: {:.3}", breakdown.fitness),
        format!("  oscillation: {:.3}", breakdown.oscillation_strength),
        format!("  clustering: {:.3}", breakdown.clustering_strength),
        format!("  coexistence: {:.3}", breakdown.coexistence_duration),
        format!("  turnover: {:.3}", breakdown.turnover_score),
        format!("  trophic balance: {:.3}", breakdown.trophic_balance_score),
    ]
}

/// Where the search would first have stopped the world, or that it would
/// still be running.
pub fn early_stop_line(stop: Option<&Stop>) -> String {
    match stop {
        Some(stop) => format!(
            "Early stop: the search would stop at tick {} ({:?})",
            stop.tick, stop.failure
        ),
        None => "Early stop: none so far".to_owned(),
    }
}

/// The bloom read and whether the bloom stop fires.
pub fn bloom_line(bloom: &BloomReading) -> String {
    let read = format!(
        "Bloom ×{:.1} (peak {} / {} founders)",
        bloom.factor(),
        bloom.running_peak,
        bloom.founders
    );
    match (bloom.rule, bloom.fired) {
        (None, _) => format!("{read}; no bloom stop"),
        (Some(rule), None) => format!(
            "{read}; bloom stop {}:{} reads at tick {}",
            rule.tick, rule.factor, rule.tick
        ),
        (Some(rule), Some(true)) => format!(
            "{read}; bloom stop {}:{} FIRES at tick {}",
            rule.tick, rule.factor, rule.tick
        ),
        (Some(rule), Some(false)) => format!(
            "{read}; bloom stop {}:{} does not fire at tick {}",
            rule.tick, rule.factor, rule.tick
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use explorers_genesis_eval::DEFAULT_BLOOM_STOP;

    fn bloom(fired: Option<bool>) -> BloomReading {
        BloomReading {
            founders: 10,
            running_peak: 120,
            rule: Some(DEFAULT_BLOOM_STOP),
            fired,
        }
    }

    #[test]
    fn the_bloom_line_says_whether_the_bloom_stop_fires() {
        assert!(bloom_line(&bloom(None)).contains("reads at tick 300"));
        assert!(bloom_line(&bloom(Some(true))).contains("FIRES at tick 300"));
        assert!(bloom_line(&bloom(Some(false))).contains("does not fire"));
        assert!(bloom_line(&bloom(None)).starts_with("Bloom ×12.0"));
    }

    #[test]
    fn the_early_stop_line_names_the_tick_and_the_gate() {
        let stop = Stop {
            tick: 350,
            failure: FailureMode::NutrientLockup,
        };
        assert_eq!(
            early_stop_line(Some(&stop)),
            "Early stop: the search would stop at tick 350 (NutrientLockup)"
        );
        assert_eq!(early_stop_line(None), "Early stop: none so far");
    }

    #[test]
    fn the_verdict_lines_name_the_failure_and_all_five_components() {
        let gated = FitnessBreakdown::gated(
            FailureMode::EnergyDeath,
            400,
            explorers_genesis_eval::guild::RoleGuilds::default(),
        );
        let lines = verdict_lines(400, 2000, &gated);
        assert_eq!(lines[1], "Failure mode: EnergyDeath");
        assert_eq!(lines.len(), 8, "header, mode, fitness and five components");
    }
}
