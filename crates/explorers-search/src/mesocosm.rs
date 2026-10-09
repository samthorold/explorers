//! Reference-mode mesocosm (#751; reference-modes.md, *Colonisation
//! overshoot*, *Mesocosm*).

use std::collections::HashMap;

use explorers_genesis_eval::income::IncomeLedger;
use explorers_sim::event::EventKind;
use explorers_sim::topology::TrophicRole;
use explorers_sim::{AgentSpec, TraitVector, World, WorldRecipe};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// The mesocosm's side, in nutrient cells: a 5 × 5 torus, so the centre cell
/// has a full ring of neighbours and the wrap is two cells away.
pub const SIDE_CELLS: usize = 5;
/// Producer founders per nutrient cell.
pub const PRODUCERS_PER_CELL: usize = 2;
/// Decomposer founders per nutrient cell.
pub const DECOMPOSERS_PER_CELL: usize = 1;

/// Provisional somatic wear rate for the mesocosm. The calibration sweep
/// (#753, `docs/research/753-mode1-wear-calibration.md`) found no rate that
/// gives producer lifespans of a few hundred ticks and a persisting control,
/// so it stays provisional until the open questions on #751 are settled. Under the committed repair law (flow 9) wear is
/// repaired in full while `wear_rate × autotrophy < kappa`, and runs away to
/// one fixed age at death above it; for the founder producers (kappa ≈ 0.32,
/// autotrophy ≈ 1.07) the edge is ≈ 0.30, where every founder dies before it
/// reproduces (#751). `0.2` sits below it: the founders reproduce, and
/// repair costs them `0.2 × autotrophy` of energy a tick.
pub const PROVISIONAL_WEAR_RATE: f32 = 0.2;

/// Which founders the mesocosm is seeded with. Without decomposers the
/// settled-level prediction is read as a second paired comparison
/// (reference-modes.md, *Colonisation overshoot*, *Mesocosm*, *Community*).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Community {
    ProducersAndDecomposers,
    ProducersOnly,
}

/// Build the mode-1 mesocosm (reference-modes.md, *Colonisation overshoot*,
/// *Mesocosm*): the recipe's world parameters on a [`SIDE_CELLS`]² torus of
/// nutrient cells, with somatic wear at `wear_rate`, seeded with producer and
/// decomposer founders (producers alone with [`Community::ProducersOnly`])
/// and no consumers. The recipe's nutrient is kept at its
/// density per area, so each cell holds what a cell of the recipe's world does.
///
/// Founders take the two trophic vertices the recipe's initial distribution
/// lays its clusters on (`World::new`): all the recipe's trophic investment
/// in autotrophy (a producer) or in heterotrophy (a decomposer), its other
/// traits at their means. They are placed uniformly over the torus from
/// `seed`, so the state is deterministic per seed.
pub fn mode1_mesocosm(
    recipe: &WorldRecipe,
    wear_rate: f32,
    seed: u64,
    community: Community,
) -> World {
    let distribution = recipe
        .initial_distribution
        .as_ref()
        .expect("the mesocosm founders read the recipe's initial distribution");
    let mut params = recipe.parameters.clone();
    let extent = SIDE_CELLS as f32 * params.nutrient_grid_cell_size;
    params.initial_nutrient_pool *= (extent / params.world_extent).powi(2);
    params.world_extent = extent;
    params.wear_rate = wear_rate;

    let mean = distribution.mean_traits;
    let trophic_total = mean.photosynthetic_absorption + mean.heterotrophy;
    let producer = TraitVector {
        photosynthetic_absorption: trophic_total,
        heterotrophy: 0.0,
        ..mean
    };
    let decomposer = TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy: trophic_total,
        ..mean
    };
    let cells = SIDE_CELLS * SIDE_CELLS;
    let decomposers = match community {
        Community::ProducersAndDecomposers => DECOMPOSERS_PER_CELL * cells,
        Community::ProducersOnly => 0,
    };
    let roster = std::iter::repeat_n(producer, PRODUCERS_PER_CELL * cells)
        .chain(std::iter::repeat_n(decomposer, decomposers));
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let half = extent / 2.0;
    let agents: Vec<AgentSpec> = roster
        .map(|traits| AgentSpec {
            position: (rng.random_range(-half..half), rng.random_range(-half..half)),
            reserve: distribution.initial_energy_per_agent,
            traits,
            nutrient: 0.0,
        })
        .collect();
    params.initial_population_size = agents.len() as u32;

    World::from_recipe(
        &WorldRecipe {
            parameters: params,
            initial_distribution: None,
            agents: Some(agents),
            carcasses: None,
            max_ticks: recipe.max_ticks,
        },
        seed,
    )
}

/// Distribution of producer lifespans (ages at death, ticks). Quantiles are
/// nearest-rank; all zero with no deaths.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct LifespanDistribution {
    pub count: usize,
    pub min: u64,
    pub p10: u64,
    pub p25: u64,
    pub median: u64,
    pub p75: u64,
    pub p90: u64,
    pub max: u64,
    pub mean: f64,
}

impl LifespanDistribution {
    pub fn of(lifespans: &[u64]) -> Self {
        if lifespans.is_empty() {
            return Self::default();
        }
        let mut sorted = lifespans.to_vec();
        sorted.sort_unstable();
        let n = sorted.len();
        let rank = |p: f64| sorted[((p * n as f64).ceil() as usize).clamp(1, n) - 1];
        Self {
            count: n,
            min: sorted[0],
            p10: rank(0.10),
            p25: rank(0.25),
            median: rank(0.50),
            p75: rank(0.75),
            p90: rank(0.90),
            max: sorted[n - 1],
            mean: sorted.iter().sum::<u64>() as f64 / n as f64,
        }
    }
}

/// One sample of the centre cell, the patch the mode-1 perturbation clears.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CentreSample {
    /// The world's tick at the sample.
    pub tick: u64,
    /// Agents in the centre cell the income read calls producers.
    pub producers: usize,
    /// Their summed structure.
    pub producer_structure: f32,
    /// Available nutrient in the centre cell's pool.
    pub available_nutrient: f32,
    /// Nutrient held in carcasses standing in the centre cell.
    pub carcass_nutrient: f32,
    /// Producer births in the centre cell since the previous sample: offspring
    /// placed there by a parent the income read called a producer.
    pub producer_births: usize,
    /// Producer deaths in the centre cell since the previous sample.
    pub producer_deaths: usize,
    /// Mean age, in ticks, of the centre's producers (`None` with none).
    pub mean_producer_age: Option<f64>,
}

/// What a mode-1 run reports.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Mode1Report {
    pub samples: Vec<CentreSample>,
    /// Every producer death in the whole world over the run, as its age at
    /// death in ticks: from the run's start for a founder, from its birth for
    /// an agent born in the run.
    pub producer_lifespans: Vec<u64>,
    /// The whole world's living roster at the end, by income role.
    pub final_roles: RoleTally,
    /// The world's energy and nutrient budget over the run.
    pub ledger: Ledger,
}

/// The whole world's energy and nutrient budget over a run, with what a
/// clearance took out as an explicit outflow. The sim's own ledgers close
/// each tick's flows; a clearance happens between ticks, so it is booked
/// here. Energy is open (solar in, dissipation out); nutrient is closed but
/// for the clearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct Ledger {
    /// Energy held by agents and carcasses at the start, before any clearance.
    pub energy_start: f32,
    /// Solar energy fixed over the run.
    pub solar_input: f32,
    /// Energy dissipated over the run.
    pub dissipated: f32,
    /// Energy the clearance removed.
    pub cleared_energy: f32,
    /// Energy held by agents and carcasses at the end.
    pub energy_end: f32,
    /// Nutrient across pool, agents and carcasses at the start, before any
    /// clearance.
    pub nutrient_start: f32,
    /// Nutrient the clearance removed.
    pub cleared_nutrient: f32,
    /// Nutrient across pool, agents and carcasses at the end.
    pub nutrient_end: f32,
}

impl Ledger {
    /// Energy in less energy out and held: zero when the budget closes.
    pub fn energy_residual(&self) -> f32 {
        self.energy_start + self.solar_input
            - self.dissipated
            - self.cleared_energy
            - self.energy_end
    }

    /// Nutrient at the start less nutrient cleared and held: zero when the
    /// budget closes.
    pub fn nutrient_residual(&self) -> f32 {
        self.nutrient_start - self.cleared_nutrient - self.nutrient_end
    }
}

/// The world's stocks and cumulative energy counters at one moment.
#[derive(Clone, Copy)]
struct Stock {
    energy: f32,
    nutrient: f32,
    solar_input: f32,
    dissipated: f32,
}

impl Stock {
    fn of(world: &World) -> Self {
        let params = world.params();
        Self {
            energy: world.free_energy() + world.carcasses().iter().map(|c| c.energy).sum::<f32>(),
            nutrient: world.nutrient_pool()
                + world
                    .agents()
                    .iter()
                    .map(|a| a.nutrient_total(params))
                    .sum::<f32>()
                + world.carcasses().iter().map(|c| c.nutrient).sum::<f32>(),
            solar_input: world.total_solar_input(),
            dissipated: world.dissipated_energy(),
        }
    }
}

/// Living agents by trophic role (the income read); `no_role` have no income
/// yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct RoleTally {
    pub producers: usize,
    pub decomposers: usize,
    pub consumers: usize,
    pub no_role: usize,
}

/// Step `world` for `ticks` ticks, sampling the centre cell at the start and
/// every `sample_every` ticks.
pub fn run_mode1(world: &mut World, ticks: u64, sample_every: u64) -> Mode1Report {
    let mut observer = Observer::new(world);
    let start = Stock::of(world);
    run_arm(
        world,
        &mut observer,
        start,
        Clearance::default(),
        ticks,
        sample_every,
    )
}

/// The mode-1 perturbation read against its paired control (reference-modes.md,
/// *How a reference mode is read*).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PairedReport {
    /// The tick at which the settled state was cloned into the two arms.
    pub fork_tick: u64,
    /// What the perturbed arm's clearance took out of the world.
    pub clearance: Clearance,
    /// The arm whose centre cell was cleared at the fork.
    pub perturbed: Mode1Report,
    /// The arm left alone.
    pub control: Mode1Report,
}

/// Settle `world` for `settle` ticks, clone it into two arms, clear the
/// perturbed arm's centre cell, and run both arms `ticks` further, sampling
/// the centre at the fork and every `sample_every` ticks. Both arms continue
/// from the same cloned state, RNG included, and each arm's observer carries
/// the settle's history, so roles and ages read the same in both. An arm's
/// lifespans are the producer deaths after the fork; its first sample's
/// births and deaths are those since the settle's last sample.
pub fn run_paired(mut world: World, settle: u64, ticks: u64, sample_every: u64) -> PairedReport {
    let mut observer = Observer::new(&mut world);
    for _ in 0..settle {
        world.step();
        observer.observe(&mut world);
    }
    observer.lifespans.clear();
    let fork_tick = world.tick();

    let start = Stock::of(&world);
    let mut control = world.clone();
    let mut control_observer = observer.clone();
    let clearance = clear_centre(&mut world);
    PairedReport {
        fork_tick,
        clearance,
        perturbed: run_arm(
            &mut world,
            &mut observer,
            start,
            clearance,
            ticks,
            sample_every,
        ),
        control: run_arm(
            &mut control,
            &mut control_observer,
            start,
            Clearance::default(),
            ticks,
            sample_every,
        ),
    }
}

/// Step `world` for `ticks` ticks under `observer`, sampling the centre cell
/// at the start and every `sample_every` ticks. `stock` is the world before
/// `cleared` was taken out of it.
fn run_arm(
    world: &mut World,
    observer: &mut Observer,
    stock: Stock,
    cleared: Clearance,
    ticks: u64,
    sample_every: u64,
) -> Mode1Report {
    let start = world.tick();
    let mut samples = vec![observer.sample_centre(world)];
    for _ in 0..ticks {
        world.step();
        observer.observe(world);
        if (world.tick() - start) % sample_every == 0 {
            samples.push(observer.sample_centre(world));
        }
    }
    let mut final_roles = RoleTally::default();
    for agent in world.agents() {
        match observer.income.role(agent.id) {
            Some(TrophicRole::Producer) => final_roles.producers += 1,
            Some(TrophicRole::Decomposer) => final_roles.decomposers += 1,
            Some(TrophicRole::Consumer) => final_roles.consumers += 1,
            None => final_roles.no_role += 1,
        }
    }
    Mode1Report {
        samples,
        producer_lifespans: std::mem::take(&mut observer.lifespans),
        final_roles,
        ledger: {
            let end = Stock::of(world);
            Ledger {
                energy_start: stock.energy,
                solar_input: end.solar_input - stock.solar_input,
                dissipated: end.dissipated - stock.dissipated,
                cleared_energy: cleared.energy,
                energy_end: end.energy,
                nutrient_start: stock.nutrient,
                cleared_nutrient: cleared.nutrient,
                nutrient_end: end.nutrient,
            }
        },
    }
}

/// What a centre-cell clearance took out of the world: the matter that
/// leaves as an outflow (reference-modes.md, *Colonisation overshoot*,
/// *Perturbation*).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct Clearance {
    pub agents: usize,
    pub carcasses: usize,
    /// Energy the removed bodies held: living reserve, structure and
    /// reproductive reserve, and carcass energy.
    pub energy: f32,
    /// Nutrient they held: each agent's free, earmarked and bound nutrient,
    /// and carcass nutrient.
    pub nutrient: f32,
}

/// Clear every agent and carcass standing in the centre cell out of the
/// world. The cell's available pool keeps its nutrient; the removed matter
/// is returned as the clearance's outflow.
pub fn clear_centre(world: &mut World) -> Clearance {
    let grid = world.nutrient_grid().clone();
    let centre = grid.cells().len() / 2;
    let params = world.params().clone();
    let agents = world.retain_agents(|a| grid.cell_index_for(a.position) != centre);
    let carcasses = world.retain_carcasses(|c| grid.cell_index_for(c.position) != centre);
    Clearance {
        agents: agents.len(),
        carcasses: carcasses.len(),
        energy: agents.iter().map(|a| a.energy()).sum::<f32>()
            + carcasses.iter().map(|c| c.energy).sum::<f32>(),
        nutrient: agents
            .iter()
            .map(|a| a.nutrient_total(&params))
            .sum::<f32>()
            + carcasses.iter().map(|c| c.nutrient).sum::<f32>(),
    }
}

/// The event kinds the observer reads: income for the role read, and births
/// and deaths.
const OBSERVED_KINDS: [EventKind; 4] = [
    EventKind::Photosynthesized,
    EventKind::Consumed,
    EventKind::Born,
    EventKind::Died,
];

/// Observer-side state walked along the run's event log. It keeps no history
/// in the world: the log is cut back to what it reads and compacted after
/// every step.
#[derive(Clone)]
struct Observer {
    income: IncomeLedger,
    centre: usize,
    /// Absolute log index of the first event not yet read.
    cursor: usize,
    /// The tick each living agent came onto the roster: the run's start for
    /// those already there, the world's tick after its birth step otherwise.
    birth_tick: HashMap<u64, u64>,
    births: usize,
    deaths: usize,
    lifespans: Vec<u64>,
}

impl Observer {
    fn new(world: &mut World) -> Self {
        world.retain_event_kinds(&OBSERVED_KINDS);
        Self {
            income: IncomeLedger::new(),
            centre: world.nutrient_grid().cells().len() / 2,
            cursor: world.event_log().len(),
            birth_tick: world
                .agents()
                .iter()
                .map(|a| (a.id, world.tick()))
                .collect(),
            births: 0,
            deaths: 0,
            lifespans: Vec::new(),
        }
    }

    /// Read the step just taken, then drop it from the log. Roles are read
    /// before the step's income is booked, as the ledger forgets the dead.
    fn observe(&mut self, world: &mut World) {
        let grid = world.nutrient_grid();
        let after = world.tick();
        for event in world.event_log().since(self.cursor) {
            let in_centre = event
                .position
                .is_some_and(|p| grid.cell_index_for(p) == self.centre);
            match event.kind {
                EventKind::Born => {
                    self.birth_tick.insert(event.source, after);
                    let parent = event.target.and_then(|id| self.income.role(id));
                    if in_centre && parent == Some(TrophicRole::Producer) {
                        self.births += 1;
                    }
                }
                EventKind::Died => {
                    let born = self.birth_tick.remove(&event.source);
                    if self.income.role(event.source) == Some(TrophicRole::Producer) {
                        self.lifespans
                            .push(after - born.expect("every agent has a birth tick"));
                        if in_centre {
                            self.deaths += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        self.income.update(world.event_log());
        self.cursor = world.event_log().len();
        world.compact_event_log_before(self.cursor);
    }

    fn sample_centre(&mut self, world: &World) -> CentreSample {
        let grid = world.nutrient_grid();
        let centre = self.centre;
        let producers: Vec<_> = world
            .agents()
            .iter()
            .filter(|a| grid.cell_index_for(a.position) == centre)
            .filter(|a| self.income.role(a.id) == Some(TrophicRole::Producer))
            .collect();
        CentreSample {
            tick: world.tick(),
            producers: producers.len(),
            producer_structure: producers.iter().map(|a| a.structure).sum(),
            available_nutrient: grid.cells()[centre],
            carcass_nutrient: world
                .carcasses()
                .iter()
                .filter(|c| grid.cell_index_for(c.position) == centre)
                .map(|c| c.nutrient)
                .sum(),
            producer_births: std::mem::take(&mut self.births),
            producer_deaths: std::mem::take(&mut self.deaths),
            mean_producer_age: (!producers.is_empty()).then(|| {
                let ages: u64 = producers
                    .iter()
                    .map(|a| world.tick() - self.birth_tick[&a.id])
                    .sum();
                ages as f64 / producers.len() as f64
            }),
        }
    }
}

/// The human-readable summary of a mode-1 run: the producer-lifespan
/// distribution, whether producers held the centre cell, and the centre-cell
/// time series.
pub fn mode1_summary(header: &str, report: &Mode1Report) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Reference mode 1 mesocosm: centre cell\n\n{header}\n"
    );
    let d = LifespanDistribution::of(&report.producer_lifespans);
    let _ = writeln!(
        out,
        "## Producer lifespans (whole world, age at death in ticks)\n\n\
         deaths {} | min {} | p10 {} | p25 {} | median {} | p75 {} | p90 {} | max {} | mean {:.1}\n",
        d.count, d.min, d.p10, d.p25, d.median, d.p75, d.p90, d.max, d.mean
    );
    let r = report.final_roles;
    let _ = writeln!(
        out,
        "## World roster at the end, by income role\n\n\
         producers {} | decomposers {} | consumers {} | no role yet {}\n",
        r.producers, r.decomposers, r.consumers, r.no_role
    );
    let second_half = &report.samples[report.samples.len() / 2..];
    let min_late = second_half.iter().map(|s| s.producers).min().unwrap_or(0);
    let last = report.samples.last().map_or(0, |s| s.producers);
    let _ = writeln!(
        out,
        "## Centre-cell producers\n\n\
         at the end {last} | fewest over the second half {min_late} | persisted: {}\n",
        if min_late > 0 { "yes" } else { "no" }
    );
    let _ = writeln!(
        out,
        "| tick | producers | structure | pool N | carcass N | births | deaths | mean age |\n\
         |---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    for s in &report.samples {
        let age = s
            .mean_producer_age
            .map_or_else(|| "-".to_string(), |a| format!("{a:.0}"));
        let _ = writeln!(
            out,
            "| {} | {} | {:.1} | {:.2} | {:.2} | {} | {} | {age} |",
            s.tick,
            s.producers,
            s.producer_structure + 0.0,
            s.available_nutrient + 0.0,
            s.carcass_nutrient + 0.0,
            s.producer_births,
            s.producer_deaths
        );
    }
    out
}

/// The human-readable summary of a paired run: the clearance, each arm's
/// budget residuals, lifespans and end roster, and the two arms' centre-cell
/// series side by side as `perturbed / control`.
pub fn paired_summary(header: &str, report: &PairedReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Reference mode 1 mesocosm: centre clearance against its control\n\n{header}\n"
    );
    let c = report.clearance;
    let _ = writeln!(
        out,
        "cleared at tick {}: {} agents, {} carcasses, energy {:.1}, nutrient {:.2} (an outflow)\n",
        report.fork_tick, c.agents, c.carcasses, c.energy, c.nutrient
    );
    let _ = writeln!(
        out,
        "| arm | energy residual | nutrient residual | producer deaths | median lifespan | roster P / D / C / none |\n\
         |---|---:|---:|---:|---:|---|"
    );
    for (name, arm) in [
        ("perturbed", &report.perturbed),
        ("control", &report.control),
    ] {
        let d = LifespanDistribution::of(&arm.producer_lifespans);
        let r = arm.final_roles;
        let _ = writeln!(
            out,
            "| {name} | {:.3} | {:.4} | {} | {} | {} / {} / {} / {} |",
            arm.ledger.energy_residual() + 0.0,
            arm.ledger.nutrient_residual() + 0.0,
            d.count,
            d.median,
            r.producers,
            r.decomposers,
            r.consumers,
            r.no_role
        );
    }
    let _ = writeln!(
        out,
        "\n## Centre cell, perturbed / control\n\n\
         | tick | producers | structure | pool N | carcass N | births | deaths | mean age |\n\
         |---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    let age = |s: &CentreSample| {
        s.mean_producer_age
            .map_or_else(|| "-".to_string(), |a| format!("{a:.0}"))
    };
    for (p, k) in report.perturbed.samples.iter().zip(&report.control.samples) {
        let _ = writeln!(
            out,
            "| {} | {} / {} | {:.1} / {:.1} | {:.2} / {:.2} | {:.2} / {:.2} | {} / {} | {} / {} | {} / {} |",
            p.tick,
            p.producers,
            k.producers,
            p.producer_structure + 0.0,
            k.producer_structure + 0.0,
            p.available_nutrient + 0.0,
            k.available_nutrient + 0.0,
            p.carcass_nutrient + 0.0,
            k.carcass_nutrient + 0.0,
            p.producer_births,
            k.producer_births,
            p.producer_deaths,
            k.producer_deaths,
            age(p),
            age(k)
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn committed_recipe() -> WorldRecipe {
        serde_json::from_str(include_str!("../../../recipe.json")).unwrap()
    }

    #[test]
    fn mode1_mesocosm_is_a_five_by_five_cell_torus_of_producer_and_decomposer_founders_with_wear_on()
     {
        let recipe = committed_recipe();
        let world = mode1_mesocosm(&recipe, 0.01, 7, Community::ProducersAndDecomposers);
        let p = world.params();

        assert_eq!(
            p.world_extent,
            5.0 * recipe.parameters.nutrient_grid_cell_size
        );
        assert_eq!(world.nutrient_grid().cells().len(), 25);
        assert!(p.wear_rate > 0.0);

        let founders = world.agents();
        let producers = founders
            .iter()
            .filter(|a| a.traits.photosynthetic_absorption > 0.0 && a.traits.heterotrophy == 0.0)
            .count();
        let decomposers = founders
            .iter()
            .filter(|a| a.traits.heterotrophy > 0.0 && a.traits.photosynthetic_absorption == 0.0)
            .count();
        assert!(producers > 0 && decomposers > 0);
        assert_eq!(producers + decomposers, founders.len());
    }

    #[test]
    fn a_run_samples_the_centre_cell_every_interval_and_reads_its_pool_and_carcasses() {
        let mut world = mode1_mesocosm(
            &committed_recipe(),
            0.01,
            3,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, 30, 10);

        let ticks: Vec<u64> = report.samples.iter().map(|s| s.tick).collect();
        assert_eq!(ticks, vec![0, 10, 20, 30]);
        let last = report.samples.last().unwrap();
        let centre = world.nutrient_grid().cells().len() / 2;
        assert_eq!(
            last.available_nutrient,
            world.nutrient_grid().cells()[centre]
        );
        let carcass_nutrient: f32 = world
            .carcasses()
            .iter()
            .filter(|c| world.nutrient_grid().cell_index_for(c.position) == centre)
            .map(|c| c.nutrient)
            .sum();
        assert_eq!(last.carcass_nutrient, carcass_nutrient);
    }

    /// A pure autotroph can live only on light, so after a few ticks of light
    /// the income read calls every one a producer, and nothing else is.
    #[test]
    fn centre_producers_are_read_from_income() {
        let mut world = mode1_mesocosm(
            &committed_recipe(),
            0.01,
            5,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, 20, 20);

        let centre = world.nutrient_grid().cells().len() / 2;
        let autotrophs: Vec<_> = world
            .agents()
            .iter()
            .filter(|a| world.nutrient_grid().cell_index_for(a.position) == centre)
            .filter(|a| a.traits.heterotrophy == 0.0 && a.traits.photosynthetic_absorption > 0.0)
            .collect();
        assert!(!autotrophs.is_empty());
        let last = report.samples.last().unwrap();
        assert_eq!(last.producers, autotrophs.len());
        let structure: f32 = autotrophs.iter().map(|a| a.structure).sum();
        assert!((last.producer_structure - structure).abs() < 1e-3);
    }

    /// Heavy wear kills producers within the run; each death's age is
    /// recorded, from the start for a founder and from its birth otherwise.
    #[test]
    fn producer_deaths_record_their_age_at_death() {
        let ticks = 150;
        let mut world = mode1_mesocosm(
            &committed_recipe(),
            0.2,
            11,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, ticks, 10);

        assert!(!report.producer_lifespans.is_empty());
        assert!(
            report
                .producer_lifespans
                .iter()
                .all(|&age| (1..=ticks).contains(&age))
        );
        let centre_deaths: usize = report.samples.iter().map(|s| s.producer_deaths).sum();
        assert!(centre_deaths <= report.producer_lifespans.len());
    }

    /// With reproduction out of reach every producer is a founder, so the
    /// centre's mean producer age is the time since the start.
    #[test]
    fn mean_producer_age_counts_from_the_start_for_founders() {
        let mut recipe = committed_recipe();
        recipe.parameters.reproduction_energy_threshold = f32::MAX;
        let mut world = mode1_mesocosm(&recipe, 0.01, 5, Community::ProducersAndDecomposers);
        let report = run_mode1(&mut world, 30, 10);

        assert_eq!(report.samples[0].mean_producer_age, None);
        for sample in &report.samples[1..] {
            assert!(sample.producers > 0);
            assert_eq!(sample.mean_producer_age, Some(sample.tick as f64));
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_report() {
        let recipe = committed_recipe();
        let run = |seed| {
            run_mode1(
                &mut mode1_mesocosm(&recipe, 0.05, seed, Community::ProducersAndDecomposers),
                60,
                10,
            )
        };
        assert_eq!(run(9), run(9));
        assert_ne!(run(9), run(10));
    }

    /// Clearing the centre takes every agent and carcass standing in it out of
    /// the world, leaves its pool and every other cell alone, and records the
    /// removed energy and nutrient.
    #[test]
    fn clearing_the_centre_removes_its_agents_and_carcasses_and_records_them() {
        let mut world = mode1_mesocosm(
            &committed_recipe(),
            0.2,
            2,
            Community::ProducersAndDecomposers,
        );
        run_mode1(&mut world, 120, 120);
        let grid = world.nutrient_grid().clone();
        let centre = grid.cells().len() / 2;
        let in_centre = |p: (f32, f32)| grid.cell_index_for(p) == centre;
        let params = world.params().clone();
        let agents_before = world.agents().to_vec();
        let carcasses_before = world.carcasses().to_vec();
        let centre_agents: Vec<_> = agents_before
            .iter()
            .filter(|a| in_centre(a.position))
            .collect();
        let centre_carcasses: Vec<_> = carcasses_before
            .iter()
            .filter(|c| in_centre(c.position))
            .collect();
        assert!(!centre_agents.is_empty() && !centre_carcasses.is_empty());

        let cleared = clear_centre(&mut world);

        assert!(world.agents().iter().all(|a| !in_centre(a.position)));
        assert!(world.carcasses().iter().all(|c| !in_centre(c.position)));
        assert_eq!(
            world.agents().len(),
            agents_before.len() - centre_agents.len()
        );
        assert_eq!(
            world.carcasses().len(),
            carcasses_before.len() - centre_carcasses.len()
        );
        assert_eq!(world.nutrient_grid().cells(), grid.cells());
        assert_eq!(cleared.agents, centre_agents.len());
        assert_eq!(cleared.carcasses, centre_carcasses.len());
        let energy: f32 = centre_agents.iter().map(|a| a.energy()).sum::<f32>()
            + centre_carcasses.iter().map(|c| c.energy).sum::<f32>();
        let nutrient: f32 = centre_agents
            .iter()
            .map(|a| a.nutrient_total(&params))
            .sum::<f32>()
            + centre_carcasses.iter().map(|c| c.nutrient).sum::<f32>();
        assert!((cleared.energy - energy).abs() <= 1e-3 * energy);
        assert!((cleared.nutrient - nutrient).abs() <= 1e-3 * nutrient);
    }

    /// The control arm is the unperturbed mesocosm: its series is the tail of
    /// one uninterrupted run over the settle and the arm's span.
    #[test]
    fn the_control_arm_is_the_unperturbed_run_over_the_same_span() {
        let recipe = committed_recipe();
        let (settle, ticks, every) = (60, 60, 10);
        let paired = run_paired(
            mode1_mesocosm(&recipe, 0.2, 6, Community::ProducersAndDecomposers),
            settle,
            ticks,
            every,
        );
        let mut whole = mode1_mesocosm(&recipe, 0.2, 6, Community::ProducersAndDecomposers);
        let uninterrupted = run_mode1(&mut whole, settle + ticks, every);

        assert_eq!(paired.fork_tick, settle);
        let tail = &uninterrupted.samples[(settle / every) as usize..];
        assert_eq!(paired.control.samples, tail);
        assert_eq!(paired.control.final_roles, uninterrupted.final_roles);
        assert!(
            uninterrupted
                .producer_lifespans
                .ends_with(&paired.control.producer_lifespans)
        );
        assert_ne!(paired.perturbed.samples, paired.control.samples);
    }

    /// Each arm's budget closes from the fork, with the clearance as an
    /// outflow: energy at the fork plus solar input is what remains plus what
    /// dissipated plus what was cleared, and nutrient at the fork is what
    /// remains plus what was cleared.
    #[test]
    fn both_arms_conserve_energy_and_nutrient_with_the_clearance_as_an_outflow() {
        let paired = run_paired(
            mode1_mesocosm(
                &committed_recipe(),
                0.2,
                8,
                Community::ProducersAndDecomposers,
            ),
            80,
            80,
            20,
        );
        assert!(paired.clearance.energy > 0.0 && paired.clearance.nutrient > 0.0);
        assert_eq!(
            paired.perturbed.ledger.cleared_energy,
            paired.clearance.energy
        );
        assert_eq!(
            paired.perturbed.ledger.cleared_nutrient,
            paired.clearance.nutrient
        );
        assert_eq!(paired.control.ledger.cleared_energy, 0.0);
        assert_eq!(paired.control.ledger.cleared_nutrient, 0.0);
        for ledger in [&paired.perturbed.ledger, &paired.control.ledger] {
            let energy_budget = ledger.energy_start + ledger.solar_input;
            assert!(
                ledger.energy_residual().abs() <= 1e-4 * energy_budget,
                "{ledger:?}"
            );
            assert!(
                ledger.nutrient_residual().abs() <= 1e-4 * ledger.nutrient_start,
                "{ledger:?}"
            );
        }
    }

    /// The producers-only mesocosm is the full one without its decomposer
    /// founders: the same producers in the same places.
    #[test]
    fn the_producers_only_mesocosm_has_no_decomposer_founders() {
        let recipe = committed_recipe();
        let full = mode1_mesocosm(&recipe, 0.2, 4, Community::ProducersAndDecomposers);
        let bare = mode1_mesocosm(&recipe, 0.2, 4, Community::ProducersOnly);

        let producers = |w: &World| -> Vec<((f32, f32), TraitVector)> {
            w.agents()
                .iter()
                .filter(|a| a.traits.heterotrophy == 0.0)
                .map(|a| (a.position, a.traits))
                .collect()
        };
        assert!(bare.agents().iter().all(|a| a.traits.heterotrophy == 0.0));
        assert_eq!(producers(&bare), producers(&full));
        assert!(full.agents().len() > bare.agents().len());
    }

    #[test]
    fn the_same_seed_gives_the_same_paired_report() {
        let recipe = committed_recipe();
        let run = |seed| {
            let world = mode1_mesocosm(&recipe, 0.2, seed, Community::ProducersOnly);
            run_paired(world, 40, 40, 10)
        };
        assert_eq!(run(3), run(3));
        assert_ne!(run(3), run(4));
    }

    /// The paired summary sets the arms side by side, one row per sampled
    /// tick, with the clearance and each arm's budget residuals.
    #[test]
    fn the_paired_summary_sets_the_arms_side_by_side() {
        let world = mode1_mesocosm(
            &committed_recipe(),
            0.2,
            3,
            Community::ProducersAndDecomposers,
        );
        let paired = run_paired(world, 40, 40, 10);
        let summary = paired_summary("header line", &paired);

        assert!(summary.contains("header line"));
        assert!(summary.contains(&format!(
            "cleared at tick 40: {} agents, {} carcasses",
            paired.clearance.agents, paired.clearance.carcasses
        )));
        assert!(summary.contains("energy residual"));
        let fork = &paired.perturbed.samples[0];
        let control = &paired.control.samples[0];
        assert!(summary.contains(&format!(
            "| 40 | {} / {} |",
            fork.producers, control.producers
        )));
        let rows = summary
            .lines()
            .filter(|l| {
                l.strip_prefix("| ")
                    .is_some_and(|r| r.starts_with(|c: char| c.is_ascii_digit()))
            })
            .count();
        assert_eq!(rows, paired.control.samples.len());
    }

    #[test]
    fn the_lifespan_distribution_reads_quantiles_by_nearest_rank() {
        let d = LifespanDistribution::of(&[50, 10, 40, 20, 30]);
        assert_eq!(d.count, 5);
        assert_eq!((d.min, d.median, d.max), (10, 30, 50));
        assert_eq!((d.p10, d.p90), (10, 50));
        assert_eq!(d.mean, 30.0);
        assert_eq!(LifespanDistribution::of(&[]).count, 0);
    }

    /// Every living agent at the end is tallied by its income role, so a
    /// reader sees whether the heterotroph founders' line stayed decomposers.
    #[test]
    fn the_world_roster_at_the_end_is_tallied_by_role() {
        let mut world = mode1_mesocosm(
            &committed_recipe(),
            0.01,
            4,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, 40, 10);

        let r = report.final_roles;
        assert_eq!(
            r.producers + r.decomposers + r.consumers + r.no_role,
            world.agents().len()
        );
        assert!(r.producers > 0);
    }
}
