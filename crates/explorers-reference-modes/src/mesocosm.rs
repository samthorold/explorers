//! Reference-mode mesocosm (#751; reference-modes.md, *Colonisation
//! overshoot*, *Mesocosm*).

use std::collections::HashMap;

use explorers_genesis_eval::income::IncomeLedger;
use explorers_sim::event::EventKind;
use explorers_sim::topology::TrophicRole;
use explorers_sim::{AgentSpec, CarcassSpec, TraitVector, World, WorldParameters, WorldRecipe};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// The mesocosm's side, in nutrient cells: a 9 × 9 torus, so the centre patch
/// has a full ring of neighbouring cells and the wrap is three cells away.
pub const SIDE_CELLS: usize = 9;
/// The centre patch's side, in nutrient cells: the 3 × 3 block of cells at
/// the torus's centre is the patch mode 1 clears and reports.
pub const PATCH_CELLS: usize = 3;
/// Producer founders per nutrient cell.
pub const PRODUCERS_PER_CELL: usize = 2;
/// Decomposer founders per nutrient cell.
pub const DECOMPOSERS_PER_CELL: usize = 1;

/// Standing carcasses per nutrient cell at founding: a colonised stand has
/// litter, so the decomposer founders have something to eat until producer
/// deaths take over (#764). The stock is a settled recipe world's: the
/// committed recipe run from its own initial distribution, seeds 1–10, read
/// every 50 ticks over ticks 1,500–2,000 and scaled to one cell's area,
/// stands 16.9 carcasses holding 54.0 energy and 57.9 nutrient per cell
/// (`docs/research/764-mesocosm-standing-carcasses.md`). Each cell gets that
/// stock as equal carcasses of producer litter.
pub const CARCASSES_PER_CELL: usize = 17;
/// Carcass energy standing in each cell at founding; see [`CARCASSES_PER_CELL`].
pub const CARCASS_ENERGY_PER_CELL: f32 = 54.0;
/// Carcass nutrient standing in each cell at founding, drawn from the cell's
/// pool; see [`CARCASSES_PER_CELL`].
pub const CARCASS_NUTRIENT_PER_CELL: f32 = 58.0;

/// The mesocosm's somatic wear: the four parameters of the wear law (world
/// rules, *Somatic wear*), which together set producer lifespans.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct Wear {
    /// Baseline accumulation per unit of nominal trait, per tick.
    pub wear_rate: f32,
    /// Accumulation per unit of energy put through a functional trait.
    pub use_wear_rate: f32,
    /// Repair rate `ρ`: wear relaxes at `ρ · κ` per tick.
    pub repair_rate: f32,
    /// Senescence hazard `η`: a living agent dies with probability `η · w`.
    pub senescence_hazard: f32,
}

/// The mode-1 mesocosm's own parameters, stated in full in the committed
/// `mode1.json` (#782): nothing is derived from another world at run time.
/// The spec is their only home. It was generated once from the committed
/// recipe with the adjustments the mesocosm then made at run time:
///
/// - **Extent**: [`SIDE_CELLS`] nutrient cells a side, with the recipe's
///   nutrient pool kept at its density per area, so each cell holds what a
///   cell of the recipe's world does.
/// - **Trophic decay 0.60** (#772). The recipe's 2.38 lies outside flow 7's
///   domain bound (world rules, flow 7, *What the two parameters stand for,
///   and their domain bounds*: decay in about [0.36, 0.98]); the spec keeps
///   the recipe's `base_trophic_efficiency` (0.78, in bound). At 0.60 the
///   kernel's factor at the reference distance `√2` is about 0.43, near the
///   bound's midpoint, and a decomposer founder at the heterotroph vertex
///   assimilates `0.78 · exp(−0.60 · 1.52) ≈ 0.31` of the producer litter it
///   drains, against 0.021 at the recipe's decay (reference-modes.md, mode 1,
///   *Parameters*).
/// - **Wear** ([`Mode1Spec::wear`]): #766's calibration
///   (`docs/research/766-mode1-wear-recalibration.md`), which supersedes the
///   old repair law's (#753). Wear comes from use alone: a producer wears
///   with the energy it captures, relaxes towards `w* = a / (ρ · κ)` over
///   about 95 ticks, and dies senescent at `η · w`. Established producers
///   (past [`ESTABLISHMENT_AGE`]) live a median of about 230 ticks, under the
///   leaching half-life of 277, and the unperturbed centre patch held
///   producers through every second-half sample in 10 seeds of 10.
///   Calibrated with the decomposer founders dying out early (#764), so it
///   may need revisiting once they persist.
/// - **Founders**: the two trophic vertices the recipe's initial
///   distribution lays its clusters on (`World::new`): all its mean trophic
///   investment in autotrophy (a producer) or in heterotrophy (a decomposer),
///   its other traits at their means, each with the recipe's energy per agent.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mode1Spec {
    /// Every world parameter, the mesocosm's extent, nutrient pool, wear and
    /// trophic decay among them.
    pub parameters: WorldParameters,
    /// The producer founders' traits: all trophic investment in autotrophy.
    pub producer: TraitVector,
    /// The decomposer founders' traits: all trophic investment in heterotrophy.
    pub decomposer: TraitVector,
    /// Each founder's starting reserve.
    pub founder_reserve: f32,
    pub max_ticks: u64,
}

impl Mode1Spec {
    /// The committed spec's path, `crates/explorers-reference-modes/mode1.json`.
    pub const COMMITTED_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/mode1.json");

    /// The committed spec, compiled in.
    pub fn committed() -> Self {
        serde_json::from_str(include_str!("../mode1.json")).expect("mode1.json parses")
    }

    /// The four parameters of the wear law the spec carries: the mesocosm's
    /// calibrated wear, and the instrument's default.
    pub fn wear(&self) -> Wear {
        let p = &self.parameters;
        Wear {
            wear_rate: p.wear_rate,
            use_wear_rate: p.use_wear_rate,
            repair_rate: p.repair_rate,
            senescence_hazard: p.senescence_hazard,
        }
    }
}

/// Which founders the mesocosm is seeded with. Without decomposers the
/// settled-level prediction is read as a second paired comparison
/// (reference-modes.md, *Colonisation overshoot*, *Mesocosm*, *Community*).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Community {
    ProducersAndDecomposers,
    ProducersOnly,
}

/// Build the mode-1 mesocosm (reference-modes.md, *Colonisation overshoot*,
/// *Mesocosm*) from its spec: the spec's world parameters on a
/// [`SIDE_CELLS`]² torus of nutrient cells, with somatic wear under `wear`,
/// seeded with producer and decomposer founders (producers alone with
/// [`Community::ProducersOnly`]) and no consumers, on a stand of litter: every
/// cell holds [`CARCASSES_PER_CELL`] standing carcasses. The founders' and the
/// litter's nutrient is drawn from the spec's pool, so total nutrient at
/// founding is that pool.
///
/// Founders take the spec's two trophic vertices, [`Mode1Spec::producer`]
/// and [`Mode1Spec::decomposer`], each with [`Mode1Spec::founder_reserve`].
/// They are placed uniformly over the torus from `seed`, so the state is
/// deterministic per seed. The mesocosm makes no parameter adjustment of its
/// own beyond setting `wear` and counting its founders.
///
/// Panics if the spec's extent is not [`SIDE_CELLS`] nutrient cells.
pub fn mode1_mesocosm(spec: &Mode1Spec, wear: Wear, seed: u64, community: Community) -> World {
    let mut params = spec.parameters.clone();
    let extent = SIDE_CELLS as f32 * params.nutrient_grid_cell_size;
    assert_eq!(
        params.world_extent, extent,
        "the spec's world_extent is SIDE_CELLS x nutrient_grid_cell_size"
    );
    params.wear_rate = wear.wear_rate;
    params.use_wear_rate = wear.use_wear_rate;
    params.repair_rate = wear.repair_rate;
    params.senescence_hazard = wear.senescence_hazard;

    let (producer, decomposer) = (spec.producer, spec.decomposer);
    let cells = SIDE_CELLS * SIDE_CELLS;
    let decomposers = match community {
        Community::ProducersAndDecomposers => DECOMPOSERS_PER_CELL * cells,
        Community::ProducersOnly => 0,
    };
    let roster = std::iter::repeat_n(producer, PRODUCERS_PER_CELL * cells)
        .chain(std::iter::repeat_n(decomposer, decomposers));
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let half = extent / 2.0;
    let size = params.nutrient_grid_cell_size;
    // Litter is placed before the founders, so both communities stand the
    // same carcasses and the same producers.
    let carcasses: Vec<CarcassSpec> = (0..cells)
        .flat_map(|cell| std::iter::repeat_n(cell, CARCASSES_PER_CELL))
        .map(|cell| {
            let (col, row) = ((cell % SIDE_CELLS) as f32, (cell / SIDE_CELLS) as f32);
            CarcassSpec {
                position: (
                    -half + (col + rng.random_range(0.0..1.0)) * size,
                    -half + (row + rng.random_range(0.0..1.0)) * size,
                ),
                energy: CARCASS_ENERGY_PER_CELL / CARCASSES_PER_CELL as f32,
                traits: producer,
                nutrient: CARCASS_NUTRIENT_PER_CELL / CARCASSES_PER_CELL as f32,
            }
        })
        .collect();
    let agents: Vec<AgentSpec> = roster
        .map(|traits| AgentSpec {
            position: (rng.random_range(-half..half), rng.random_range(-half..half)),
            reserve: spec.founder_reserve,
            traits,
            nutrient: 0.0,
        })
        .collect();
    params.initial_population_size = agents.len() as u32;

    let mut world = World::from_recipe(
        &WorldRecipe {
            parameters: params,
            initial_distribution: None,
            agents: Some(agents),
            carcasses: Some(carcasses),
            max_ticks: spec.max_ticks,
        },
        seed,
    );
    // The litter's nutrient comes out of the pool after the founders have
    // bound theirs, as theirs does (world rules, *Founders bind nutrient at
    // world creation*), so total nutrient at founding is the initial pool.
    let litter: Vec<_> = world
        .carcasses()
        .iter()
        .map(|c| (c.position, c.nutrient))
        .collect();
    for (position, nutrient) in litter {
        let shortfall = world.nutrient_grid_mut().draw_nearest(position, nutrient);
        assert_eq!(shortfall, 0.0, "the pool covers the mesocosm's litter");
    }
    world
}

/// The autotrophy at which a carcass drainer counts as a mixotroph rather
/// than a specialist at the pure heterotroph vertex (#772). #764's
/// pre-registered reading splits decomposer income here: the settled recipe
/// world's heterotrophs carry autotrophy 0.02–0.18, and flow 7's bound says a
/// mixotroph with 0.1–0.2 autotrophy sits a little nearer producer litter.
pub const MIXOTROPH_AUTOTROPHY: f32 = 0.1;

/// Energy drained from carcasses (the `Consumed` events on a carcass, before
/// trophic efficiency), split by the draining agent's nominal autotrophy at
/// [`MIXOTROPH_AUTOTROPHY`]. Counted by income, not by role: every drain is
/// booked, whatever the drainer's role.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct DrainSplit {
    /// Drained by agents with autotrophy below [`MIXOTROPH_AUTOTROPHY`]: the
    /// pure-vertex side.
    pub pure_vertex: f64,
    /// Drained by agents with autotrophy at or above it: mixotrophs.
    pub mixotroph: f64,
}

impl DrainSplit {
    /// Both sides together.
    pub fn total(&self) -> f64 {
        self.pure_vertex + self.mixotroph
    }

    /// The mixotroph side's share of the total (`None` with nothing drained).
    pub fn mixotroph_share(&self) -> Option<f64> {
        let total = self.total();
        (total > 0.0).then(|| self.mixotroph / total)
    }

    fn minus(&self, earlier: &DrainSplit) -> DrainSplit {
        DrainSplit {
            pure_vertex: self.pure_vertex - earlier.pure_vertex,
            mixotroph: self.mixotroph - earlier.mixotroph,
        }
    }
}

/// The establishment cut-off, in ticks: a producer that dies older than this
/// had established, so lifespans read over those are not swamped by seedling
/// deaths. #753 read its past-seedling median at the same cut-off.
pub const ESTABLISHMENT_AGE: u64 = 50;

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

/// One sample of the centre patch, the [`PATCH_CELLS`]² block of cells the
/// mode-1 perturbation clears.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CentreSample {
    /// The world's tick at the sample.
    pub tick: u64,
    /// Agents in the centre patch the income read calls producers.
    pub producers: usize,
    /// Their summed structure.
    pub producer_structure: f32,
    /// Available nutrient in the centre patch's pool.
    pub available_nutrient: f32,
    /// Nutrient held in carcasses standing in the centre patch.
    pub carcass_nutrient: f32,
    /// Producer births in the centre patch since the previous sample:
    /// offspring placed there by a parent the income read called a producer.
    pub producer_births: usize,
    /// Producer deaths in the centre patch since the previous sample.
    pub producer_deaths: usize,
    /// Agents in the whole world the income read calls decomposers.
    pub decomposers: usize,
    /// Mean age, in ticks, of the centre patch's producers (`None` with none).
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
    /// The lifespans among those of producers that died senescent (the
    /// stepper's `Senesced` event) rather than of starvation or structural
    /// failure.
    pub senescent_lifespans: Vec<u64>,
    /// The whole world's living roster at the end, by income role.
    pub final_roles: RoleTally,
    /// Living agents at the end whose nominal traits are heterotroph-dominant
    /// (heterotrophy above autotrophy), whatever their income (#772).
    pub heterotroph_dominant: usize,
    /// Carcass-drain energy by the drainer's autotrophy over the whole
    /// horizon: from the world's founding, so a paired arm's carries the
    /// settle's history.
    pub carcass_drain: DrainSplit,
    /// The same over this run or arm alone (after the fork for a paired arm).
    pub arm_carcass_drain: DrainSplit,
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

/// Step `world` for `ticks` ticks, sampling the centre patch at the start and
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
    /// The arm whose centre patch was cleared at the fork.
    pub perturbed: Mode1Report,
    /// The arm left alone.
    pub control: Mode1Report,
}

/// Settle `world` for `settle` ticks, clone it into two arms, clear the
/// perturbed arm's centre patch, and run both arms `ticks` further, sampling
/// the centre at the fork and every `sample_every` ticks. Both arms continue
/// from the same cloned state, RNG included, and each arm's observer carries
/// the settle's history, so roles and ages read the same in both. An arm's
/// lifespans are the producer deaths after the fork; its first sample's
/// births and deaths are those since the settle's last sample.
pub fn run_paired(mut world: World, settle: u64, ticks: u64, sample_every: u64) -> PairedReport {
    let mut observer = Observer::new(&mut world);
    let start = world.tick();
    for _ in 0..settle {
        world.step();
        observer.observe(&mut world);
        let elapsed = world.tick() - start;
        if elapsed < settle && elapsed % sample_every == 0 {
            // The settle's samples before the fork are not reported, but
            // taking them resets the birth and death counts, so the fork's
            // sample counts only those since the settle's last sample, as a
            // run sampled straight through does.
            observer.sample_centre(&world);
        }
    }
    observer.lifespans.clear();
    observer.senescent_lifespans.clear();
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

/// Step `world` for `ticks` ticks under `observer`, sampling the centre patch
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
    let drain_at_start = observer.carcass_drain;
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
        senescent_lifespans: std::mem::take(&mut observer.senescent_lifespans),
        final_roles,
        heterotroph_dominant: world
            .agents()
            .iter()
            .filter(|a| a.traits.heterotrophy > a.traits.photosynthetic_absorption)
            .count(),
        carcass_drain: observer.carcass_drain,
        arm_carcass_drain: observer.carcass_drain.minus(&drain_at_start),
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

/// What a centre-patch clearance took out of the world: the matter that
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

/// Whether the mesocosm's nutrient cell `cell` (row-major) lies in the centre
/// patch, the [`PATCH_CELLS`]² block around the torus's centre cell.
fn in_centre_patch(cell: usize) -> bool {
    let lo = SIDE_CELLS / 2 - PATCH_CELLS / 2;
    let within = |i: usize| (lo..lo + PATCH_CELLS).contains(&i);
    within(cell % SIDE_CELLS) && within(cell / SIDE_CELLS)
}

/// Clear every agent and carcass standing in the centre patch out of the
/// world. The patch's available pool keeps its nutrient; the removed matter
/// is returned as the clearance's outflow.
pub fn clear_centre(world: &mut World) -> Clearance {
    let grid = world.nutrient_grid().clone();
    let in_patch = |p| in_centre_patch(grid.cell_index_for(p));
    let params = world.params().clone();
    let agents = world.retain_agents(|a| !in_patch(a.position));
    let carcasses = world.retain_carcasses(|c| !in_patch(c.position));
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

/// The event kinds the observer reads: income for the role read, births,
/// and deaths with their senescent cause.
const OBSERVED_KINDS: [EventKind; 5] = [
    EventKind::Photosynthesized,
    EventKind::Consumed,
    EventKind::Born,
    EventKind::Senesced,
    EventKind::Died,
];

/// Observer-side state walked along the run's event log. It keeps no history
/// in the world: the log is cut back to what it reads and compacted after
/// every step.
#[derive(Clone)]
struct Observer {
    income: IncomeLedger,
    /// Absolute log index of the first event not yet read.
    cursor: usize,
    /// The tick each living agent came onto the roster: the run's start for
    /// those already there, the world's tick after its birth step otherwise.
    birth_tick: HashMap<u64, u64>,
    births: usize,
    deaths: usize,
    lifespans: Vec<u64>,
    senescent_lifespans: Vec<u64>,
    /// Each agent's nominal autotrophy, kept while it lives so a drain in the
    /// step it dies is still booked to its side.
    autotrophy: HashMap<u64, f32>,
    /// Carcass-drain energy since the observer began, by drainer autotrophy.
    carcass_drain: DrainSplit,
}

impl Observer {
    fn new(world: &mut World) -> Self {
        world.retain_event_kinds(&OBSERVED_KINDS);
        Self {
            income: IncomeLedger::new(),
            cursor: world.event_log().len(),
            birth_tick: world
                .agents()
                .iter()
                .map(|a| (a.id, world.tick()))
                .collect(),
            births: 0,
            deaths: 0,
            lifespans: Vec::new(),
            senescent_lifespans: Vec::new(),
            autotrophy: world
                .agents()
                .iter()
                .map(|a| (a.id, a.traits.photosynthetic_absorption))
                .collect(),
            carcass_drain: DrainSplit::default(),
        }
    }

    /// Read the step just taken, then drop it from the log. Roles are read
    /// before the step's income is booked, as the ledger forgets the dead.
    fn observe(&mut self, world: &mut World) {
        let grid = world.nutrient_grid();
        let after = world.tick();
        // The stepper marks a senescent death with `Senesced` just before its
        // `Died`, in the same step.
        let mut senesced = std::collections::HashSet::new();
        // Agents born in this step are on the roster now; those that died in
        // it are still in the map from the step before.
        self.autotrophy.extend(
            world
                .agents()
                .iter()
                .map(|a| (a.id, a.traits.photosynthetic_absorption)),
        );
        for event in world.event_log().since(self.cursor) {
            let in_centre = event
                .position
                .is_some_and(|p| in_centre_patch(grid.cell_index_for(p)));
            match event.kind {
                EventKind::Born => {
                    self.birth_tick.insert(event.source, after);
                    let parent = event.target.and_then(|id| self.income.role(id));
                    if in_centre && parent == Some(TrophicRole::Producer) {
                        self.births += 1;
                    }
                }
                EventKind::Senesced => {
                    senesced.insert(event.source);
                }
                EventKind::Consumed if event.target_was_carcass => {
                    let drained = event.energy_delta as f64;
                    let autotrophy = self.autotrophy[&event.source];
                    if autotrophy < MIXOTROPH_AUTOTROPHY {
                        self.carcass_drain.pure_vertex += drained;
                    } else {
                        self.carcass_drain.mixotroph += drained;
                    }
                }
                EventKind::Died => {
                    self.autotrophy.remove(&event.source);
                    let born = self.birth_tick.remove(&event.source);
                    if self.income.role(event.source) == Some(TrophicRole::Producer) {
                        let age = after - born.expect("every agent has a birth tick");
                        self.lifespans.push(age);
                        if senesced.contains(&event.source) {
                            self.senescent_lifespans.push(age);
                        }
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
        let in_patch = |p| in_centre_patch(grid.cell_index_for(p));
        let producers: Vec<_> = world
            .agents()
            .iter()
            .filter(|a| in_patch(a.position))
            .filter(|a| self.income.role(a.id) == Some(TrophicRole::Producer))
            .collect();
        CentreSample {
            tick: world.tick(),
            producers: producers.len(),
            producer_structure: producers.iter().map(|a| a.structure).sum(),
            available_nutrient: (grid.cells().iter().enumerate())
                .filter(|&(cell, _)| in_centre_patch(cell))
                .map(|(_, n)| n)
                .sum(),
            carcass_nutrient: world
                .carcasses()
                .iter()
                .filter(|c| in_patch(c.position))
                .map(|c| c.nutrient)
                .sum(),
            producer_births: std::mem::take(&mut self.births),
            producer_deaths: std::mem::take(&mut self.deaths),
            decomposers: world
                .agents()
                .iter()
                .filter(|a| self.income.role(a.id) == Some(TrophicRole::Decomposer))
                .count(),
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

/// The header of the decomposer reading's table (#764, #772): one row per
/// arm, from [`decomposer_row`].
const DECOMPOSER_HEADER: &str = "| decomposers persist | decomposers at end | heterotroph-dominant at end | carcass drain, whole horizon: autotrophy < 0.1 / >= 0.1 (mixotroph share) | carcass drain, this arm |";

/// One arm's decomposer reading: whether any agent the income read calls a
/// decomposer is alive at the end, how many, the heterotroph-dominant agents
/// alive at the end, and the carcass-drain split over the whole horizon and
/// over the arm.
fn decomposer_row(report: &Mode1Report) -> String {
    let split = |d: &DrainSplit| {
        let share = d
            .mixotroph_share()
            .map_or_else(|| "-".to_string(), |s| format!("{s:.3}"));
        format!("{:.1} / {:.1} ({share})", d.pure_vertex, d.mixotroph)
    };
    let decomposers = report.final_roles.decomposers;
    format!(
        "| {} | {decomposers} | {} | {} | {} |",
        if decomposers > 0 { "yes" } else { "no" },
        report.heterotroph_dominant,
        split(&report.carcass_drain),
        split(&report.arm_carcass_drain)
    )
}

/// The human-readable summary of a mode-1 run: the producer-lifespan
/// distribution, whether producers held the centre patch, and the patch's
/// time series beside the world's decomposers.
pub fn mode1_summary(header: &str, report: &Mode1Report) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Reference mode 1 mesocosm: centre patch\n\n{header}\n"
    );
    let d = LifespanDistribution::of(&report.producer_lifespans);
    let _ = writeln!(
        out,
        "## Producer lifespans (whole world, age at death in ticks)\n\n\
         deaths {} | min {} | p10 {} | p25 {} | median {} | p75 {} | p90 {} | max {} | mean {:.1}\n",
        d.count, d.min, d.p10, d.p25, d.median, d.p75, d.p90, d.max, d.mean
    );
    let established = |ages: &[u64]| -> Vec<u64> {
        ages.iter()
            .copied()
            .filter(|&a| a > ESTABLISHMENT_AGE)
            .collect()
    };
    let e = LifespanDistribution::of(&established(&report.producer_lifespans));
    let senescent = established(&report.senescent_lifespans).len();
    let _ = writeln!(
        out,
        "established (age > {ESTABLISHMENT_AGE}): deaths {} | median {} | senescent {senescent} ({:.2})\n",
        e.count,
        e.median,
        senescent as f64 / e.count.max(1) as f64
    );
    let r = report.final_roles;
    let _ = writeln!(
        out,
        "## World roster at the end, by income role\n\n\
         producers {} | decomposers {} | consumers {} | no role yet {}\n",
        r.producers, r.decomposers, r.consumers, r.no_role
    );
    let _ = writeln!(
        out,
        "## Decomposers and carcass-drain income\n\n{DECOMPOSER_HEADER}\n|---|---:|---:|---:|---:|\n{}\n",
        decomposer_row(report)
    );
    let second_half = &report.samples[report.samples.len() / 2..];
    let min_late = second_half.iter().map(|s| s.producers).min().unwrap_or(0);
    let last = report.samples.last().map_or(0, |s| s.producers);
    let _ = writeln!(
        out,
        "## Centre-patch producers\n\n\
         at the end {last} | fewest over the second half {min_late} | persisted: {}\n",
        if min_late > 0 { "yes" } else { "no" }
    );
    let _ = writeln!(
        out,
        "| tick | producers | structure | pool N | carcass N | births | deaths | mean age | decomposers (world) |\n\
         |---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    for s in &report.samples {
        let age = s
            .mean_producer_age
            .map_or_else(|| "-".to_string(), |a| format!("{a:.0}"));
        let _ = writeln!(
            out,
            "| {} | {} | {:.1} | {:.2} | {:.2} | {} | {} | {age} | {} |",
            s.tick,
            s.producers,
            s.producer_structure + 0.0,
            s.available_nutrient + 0.0,
            s.carcass_nutrient + 0.0,
            s.producer_births,
            s.producer_deaths,
            s.decomposers
        );
    }
    out
}

/// The human-readable summary of a paired run: the clearance, each arm's
/// budget residuals, lifespans and end roster, and the two arms' centre-patch
/// series side by side as `perturbed / control`.
pub fn paired_summary(header: &str, report: &PairedReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Reference mode 1 mesocosm: centre patch clearance against its control\n\n{header}\n"
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
        "\n## Decomposers and carcass-drain income\n\n| arm {DECOMPOSER_HEADER}\n|---|---|---:|---:|---:|---:|"
    );
    for (name, arm) in [
        ("perturbed", &report.perturbed),
        ("control", &report.control),
    ] {
        let _ = writeln!(out, "| {name} {}", decomposer_row(arm));
    }
    let _ = writeln!(
        out,
        "\n## Centre patch, perturbed / control\n\n\
         | tick | producers | structure | pool N | carcass N | births | deaths | mean age | decomposers (world) |\n\
         |---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    let age = |s: &CentreSample| {
        s.mean_producer_age
            .map_or_else(|| "-".to_string(), |a| format!("{a:.0}"))
    };
    for (p, k) in report.perturbed.samples.iter().zip(&report.control.samples) {
        let _ = writeln!(
            out,
            "| {} | {} / {} | {:.1} / {:.1} | {:.2} / {:.2} | {:.2} / {:.2} | {} / {} | {} / {} | {} / {} | {} / {} |",
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
            age(k),
            p.decomposers,
            k.decomposers
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mesocosm's calibrated wear, as the committed spec carries it.
    fn mesocosm_wear() -> Wear {
        Mode1Spec::committed().wear()
    }

    /// The committed spec states the mesocosm's parameters in full (#782):
    /// trophic transfer inside flow 7's domain bound (decay 0.60, #772, at the
    /// recipe's base efficiency 0.78), a 9-cell extent, and #766's calibrated
    /// wear.
    #[test]
    fn the_committed_spec_pins_the_mesocosms_parameters() {
        let spec = Mode1Spec::committed();
        let p = &spec.parameters;
        assert_eq!(p.trophic_distance_decay, 0.60);
        assert!((p.base_trophic_efficiency - 0.78).abs() < 0.005);
        assert_eq!(
            p.world_extent,
            SIDE_CELLS as f32 * p.nutrient_grid_cell_size
        );
        assert_eq!(SIDE_CELLS, 9);
        assert_eq!(
            spec.wear(),
            Wear {
                wear_rate: 0.0,
                use_wear_rate: 0.0007,
                repair_rate: 0.03,
                senescence_hazard: 0.022,
            }
        );
    }

    /// The mesocosm sets all four parameters of the wear law, whatever the
    /// spec carries: the instrument's wear overrides reach the world.
    #[test]
    fn the_mesocosm_runs_under_the_wear_it_is_given() {
        let wear = Wear {
            wear_rate: 0.03,
            use_wear_rate: 0.004,
            repair_rate: 0.5,
            senescence_hazard: 0.002,
        };
        let world = mode1_mesocosm(&Mode1Spec::committed(), wear, 7, Community::ProducersOnly);
        let p = world.params();
        assert_eq!(
            (
                p.wear_rate,
                p.use_wear_rate,
                p.repair_rate,
                p.senescence_hazard
            ),
            (0.03, 0.004, 0.5, 0.002)
        );
    }

    /// The mesocosm runs the spec's parameters as they stand (#782): it makes
    /// no adjustment of its own but to count its founders. Trophic transfer
    /// runs inside flow 7's domain bound (#772): decay 0.60 at base
    /// efficiency 0.78.
    #[test]
    fn the_mesocosm_runs_the_specs_parameters_as_they_stand() {
        let spec = Mode1Spec::committed();
        for community in [Community::ProducersAndDecomposers, Community::ProducersOnly] {
            let world = mode1_mesocosm(&spec, spec.wear(), 7, community);
            let p = world.params();
            assert_eq!(p.trophic_distance_decay, 0.60);
            assert!((p.base_trophic_efficiency - 0.78).abs() < 0.005);
            let expected = WorldParameters {
                initial_population_size: world.agents().len() as u32,
                ..spec.parameters.clone()
            };
            assert_eq!(*p, expected);
        }
    }

    /// The spec must describe the mesocosm's own geometry: its extent is
    /// [`SIDE_CELLS`] nutrient cells.
    #[test]
    #[should_panic(expected = "SIDE_CELLS")]
    fn a_spec_whose_extent_is_not_nine_cells_is_refused() {
        let mut spec = Mode1Spec::committed();
        spec.parameters.world_extent = 10.0 * spec.parameters.nutrient_grid_cell_size;
        mode1_mesocosm(&spec, spec.wear(), 7, Community::ProducersOnly);
    }

    #[test]
    fn mode1_mesocosm_is_a_nine_by_nine_cell_torus_of_producer_and_decomposer_founders_with_wear_on()
     {
        let spec = Mode1Spec::committed();
        let world = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            7,
            Community::ProducersAndDecomposers,
        );
        let p = world.params();

        assert_eq!(
            p.world_extent,
            9.0 * spec.parameters.nutrient_grid_cell_size
        );
        assert_eq!(world.nutrient_grid().cells().len(), 81);
        assert!(p.wear_rate + p.use_wear_rate > 0.0 && p.senescence_hazard > 0.0);

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

    /// A colonised stand has litter: every cell stands the settled recipe
    /// world's carcass stock at founding, in both communities, as producer
    /// litter.
    #[test]
    fn every_cell_stands_the_settled_carcass_stock_at_founding() {
        let spec = Mode1Spec::committed();
        for community in [Community::ProducersAndDecomposers, Community::ProducersOnly] {
            let world = mode1_mesocosm(&spec, mesocosm_wear(), 7, community);
            let grid = world.nutrient_grid();
            let producer = world.agents()[0].traits;
            for cell in 0..grid.cells().len() {
                let standing: Vec<_> = world
                    .carcasses()
                    .iter()
                    .filter(|c| grid.cell_index_for(c.position) == cell)
                    .collect();
                assert_eq!(standing.len(), CARCASSES_PER_CELL);
                let energy: f32 = standing.iter().map(|c| c.energy).sum();
                let nutrient: f32 = standing.iter().map(|c| c.nutrient).sum();
                assert!((energy - CARCASS_ENERGY_PER_CELL).abs() < 1e-3);
                assert!((nutrient - CARCASS_NUTRIENT_PER_CELL).abs() < 1e-3);
                assert!(standing.iter().all(|c| c.traits == producer));
            }
        }
    }

    /// The standing carcasses' nutrient comes out of the pool, as the
    /// founders' does (world rules, *Founders bind nutrient at world
    /// creation*): total nutrient at founding is the world's initial pool, and
    /// a run's budget closes from tick 0.
    #[test]
    fn conservation_holds_from_tick_0_with_the_standing_carcasses() {
        let spec = Mode1Spec::committed();
        for community in [Community::ProducersAndDecomposers, Community::ProducersOnly] {
            let mut world = mode1_mesocosm(&spec, mesocosm_wear(), 3, community);
            let params = world.params().clone();
            let total = world.nutrient_pool()
                + world
                    .agents()
                    .iter()
                    .map(|a| a.nutrient_total(&params))
                    .sum::<f32>()
                + world.carcasses().iter().map(|c| c.nutrient).sum::<f32>();
            let pool = params.initial_nutrient_pool;
            assert!((total - pool).abs() <= 1e-4 * pool, "{total} vs {pool}");

            let ledger = run_mode1(&mut world, 100, 50).ledger;
            assert_eq!(ledger.nutrient_start, total);
            let energy_budget = ledger.energy_start + ledger.solar_input;
            assert!(ledger.energy_residual().abs() <= 1e-4 * energy_budget);
            assert!(ledger.nutrient_residual().abs() <= 1e-4 * pool);
        }
    }

    #[test]
    fn a_run_samples_the_centre_patch_every_interval_and_reads_its_pool_and_carcasses() {
        let mut world = mode1_mesocosm(
            &Mode1Spec::committed(),
            mesocosm_wear(),
            3,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, 30, 10);

        let ticks: Vec<u64> = report.samples.iter().map(|s| s.tick).collect();
        assert_eq!(ticks, vec![0, 10, 20, 30]);
        let last = report.samples.last().unwrap();
        let grid = world.nutrient_grid();
        let pool: f32 = (0..grid.cells().len())
            .filter(|&i| {
                let (col, row) = (i % SIDE_CELLS, i / SIDE_CELLS);
                (3..=5).contains(&col) && (3..=5).contains(&row)
            })
            .map(|i| grid.cells()[i])
            .sum();
        assert!((last.available_nutrient - pool).abs() <= 1e-4 * pool);
        let carcass_nutrient: f32 = world
            .carcasses()
            .iter()
            .filter(|c| in_centre_block(grid, c.position))
            .map(|c| c.nutrient)
            .sum();
        assert!((last.carcass_nutrient - carcass_nutrient).abs() <= 1e-4 * carcass_nutrient);
    }

    /// A pure autotroph can live only on light, so after a few ticks of light
    /// the income read calls every one a producer, and nothing else is. With
    /// reproduction out of reach no mutant offspring blurs the founders.
    #[test]
    fn centre_patch_producers_are_read_from_income() {
        let mut spec = Mode1Spec::committed();
        spec.parameters.reproduction_energy_threshold = f32::MAX;
        let mut world = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            5,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, 20, 20);

        let autotrophs: Vec<_> = world
            .agents()
            .iter()
            .filter(|a| in_centre_block(world.nutrient_grid(), a.position))
            .filter(|a| a.traits.heterotrophy == 0.0 && a.traits.photosynthetic_absorption > 0.0)
            .collect();
        assert!(!autotrophs.is_empty());
        let last = report.samples.last().unwrap();
        assert_eq!(last.producers, autotrophs.len());
        let structure: f32 = autotrophs.iter().map(|a| a.structure).sum();
        assert!((last.producer_structure - structure).abs() < 1e-3);
    }

    /// Producers die within the run; each death's age is
    /// recorded, from the start for a founder and from its birth otherwise.
    #[test]
    fn producer_deaths_record_their_age_at_death() {
        let ticks = 150;
        let mut world = mode1_mesocosm(
            &Mode1Spec::committed(),
            mesocosm_wear(),
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

    /// A producer death the stepper marks `Senesced` is recorded among the
    /// senescent lifespans as well as among all of them; with no hazard,
    /// none is.
    #[test]
    fn senescent_producer_deaths_are_recorded_apart() {
        let spec = Mode1Spec::committed();
        let run = |senescence_hazard| {
            let wear = Wear {
                senescence_hazard,
                ..mesocosm_wear()
            };
            run_mode1(
                &mut mode1_mesocosm(&spec, wear, 11, Community::ProducersOnly),
                100,
                50,
            )
        };

        let report = run(0.2);
        assert!(!report.senescent_lifespans.is_empty());
        assert!(report.senescent_lifespans.len() < report.producer_lifespans.len());
        let mut all = report.producer_lifespans.clone();
        for age in &report.senescent_lifespans {
            let i = all.iter().position(|a| a == age).expect("a producer death");
            all.swap_remove(i);
        }

        let report = run(0.0);
        assert!(!report.producer_lifespans.is_empty());
        assert!(report.senescent_lifespans.is_empty());
    }

    /// The summary reads lifespans over producers past establishment, so
    /// seedling deaths do not swamp the median, and the senescent share of
    /// their deaths.
    #[test]
    fn the_summary_reports_established_lifespans_and_their_senescent_share() {
        let report = Mode1Report {
            samples: vec![],
            producer_lifespans: vec![3, 8, 60, 90, 120, 400, 10],
            senescent_lifespans: vec![8, 90, 400],
            final_roles: RoleTally::default(),
            heterotroph_dominant: 0,
            carcass_drain: DrainSplit::default(),
            arm_carcass_drain: DrainSplit::default(),
            ledger: Ledger::default(),
        };
        let summary = mode1_summary("h", &report);
        assert_eq!(ESTABLISHMENT_AGE, 50);
        assert!(
            summary.contains("established (age > 50): deaths 4 | median 90 | senescent 2 (0.50)"),
            "{summary}"
        );
    }

    /// With reproduction out of reach every producer is a founder, so the
    /// centre's mean producer age is the time since the start.
    #[test]
    fn mean_producer_age_counts_from_the_start_for_founders() {
        let mut spec = Mode1Spec::committed();
        spec.parameters.reproduction_energy_threshold = f32::MAX;
        let mut world = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            5,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, 30, 10);

        assert_eq!(report.samples[0].mean_producer_age, None);
        for sample in &report.samples[1..] {
            assert!(sample.producers > 0);
            assert_eq!(sample.mean_producer_age, Some(sample.tick as f64));
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_report() {
        let spec = Mode1Spec::committed();
        let run = |seed| {
            run_mode1(
                &mut mode1_mesocosm(
                    &spec,
                    mesocosm_wear(),
                    seed,
                    Community::ProducersAndDecomposers,
                ),
                60,
                10,
            )
        };
        assert_eq!(run(9), run(9));
        assert_ne!(run(9), run(10));
    }

    /// The centre patch: the 3 × 3 block of cells around the torus's centre
    /// cell, read from row and column.
    fn in_centre_block(grid: &explorers_sim::spatial::NutrientGrid, p: (f32, f32)) -> bool {
        let cell = grid.cell_index_for(p);
        let (col, row) = (cell % SIDE_CELLS, cell / SIDE_CELLS);
        (3..=5).contains(&col) && (3..=5).contains(&row)
    }

    /// Clearing the centre patch takes every agent and carcass standing in
    /// its 3 × 3 block of cells out of the world, leaves the pool and every
    /// other cell alone, and records the removed energy and nutrient.
    #[test]
    fn clearing_the_centre_patch_removes_its_agents_and_carcasses_and_records_them() {
        let mut world = mode1_mesocosm(
            &Mode1Spec::committed(),
            mesocosm_wear(),
            2,
            Community::ProducersAndDecomposers,
        );
        run_mode1(&mut world, 120, 120);
        let grid = world.nutrient_grid().clone();
        let in_centre = |p: (f32, f32)| in_centre_block(&grid, p);
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
        let spec = Mode1Spec::committed();
        let (settle, ticks, every) = (60, 60, 10);
        let paired = run_paired(
            mode1_mesocosm(
                &spec,
                mesocosm_wear(),
                6,
                Community::ProducersAndDecomposers,
            ),
            settle,
            ticks,
            every,
        );
        let mut whole = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            6,
            Community::ProducersAndDecomposers,
        );
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
                &Mode1Spec::committed(),
                mesocosm_wear(),
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
        let spec = Mode1Spec::committed();
        let full = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            4,
            Community::ProducersAndDecomposers,
        );
        let bare = mode1_mesocosm(&spec, mesocosm_wear(), 4, Community::ProducersOnly);

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
        let spec = Mode1Spec::committed();
        let run = |seed| {
            let world = mode1_mesocosm(&spec, mesocosm_wear(), seed, Community::ProducersOnly);
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
            &Mode1Spec::committed(),
            mesocosm_wear(),
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

    /// Each sample counts the whole world's decomposers by income role, so a
    /// reader sees how long the decomposer founders' line lasts.
    #[test]
    fn each_sample_counts_the_worlds_decomposers() {
        let mut world = mode1_mesocosm(
            &Mode1Spec::committed(),
            mesocosm_wear(),
            4,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, 4, 2);
        assert_eq!(report.samples[0].decomposers, 0, "no income read yet");
        assert!(report.samples[1].decomposers > 0);
        let last = report.samples.last().unwrap();
        assert_eq!(last.decomposers, report.final_roles.decomposers);

        let mut bare = mode1_mesocosm(
            &Mode1Spec::committed(),
            mesocosm_wear(),
            4,
            Community::ProducersOnly,
        );
        let report = run_mode1(&mut bare, 4, 2);
        assert!(report.samples.iter().all(|s| s.decomposers == 0));
    }

    /// Both summaries read the centre patch and carry the world's decomposers
    /// in each row.
    #[test]
    fn the_summaries_report_the_centre_patch_and_the_worlds_decomposers() {
        let spec = Mode1Spec::committed();
        let mut world = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            4,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, 4, 2);
        let summary = mode1_summary("h", &report);
        assert!(summary.contains("centre patch"));
        let s = &report.samples[1];
        assert!(summary.contains(&format!("| 2 | {} |", s.producers)));
        assert!(summary.contains(&format!(" | {} |\n", s.decomposers)));
        assert!(s.decomposers > 0);

        let paired = run_paired(
            mode1_mesocosm(
                &spec,
                mesocosm_wear(),
                4,
                Community::ProducersAndDecomposers,
            ),
            2,
            2,
            2,
        );
        let summary = paired_summary("h", &paired);
        assert!(summary.contains("centre patch"));
        let (p, k) = (&paired.perturbed.samples[1], &paired.control.samples[1]);
        assert!(summary.contains(&format!(" | {} / {} |\n", p.decomposers, k.decomposers)));
    }

    /// Every living agent at the end is tallied by its income role, so a
    /// reader sees whether the heterotroph founders' line stayed decomposers.
    #[test]
    fn the_world_roster_at_the_end_is_tallied_by_role() {
        let mut world = mode1_mesocosm(
            &Mode1Spec::committed(),
            mesocosm_wear(),
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

    /// Carcass-drain energy is booked by the draining agent's autotrophy,
    /// split at [`MIXOTROPH_AUTOTROPHY`]: read independently off a full event
    /// log of the same run, with each drainer's traits from the roster.
    #[test]
    fn carcass_drain_energy_is_split_by_the_drainers_autotrophy() {
        let spec = Mode1Spec::committed();
        let build = || {
            mode1_mesocosm(
                &spec,
                mesocosm_wear(),
                4,
                Community::ProducersAndDecomposers,
            )
        };
        let ticks = 40;
        let report = run_mode1(&mut build(), ticks, 10);

        let mut world = build();
        let mut traits: HashMap<u64, TraitVector> = HashMap::new();
        let mut expected = DrainSplit::default();
        for _ in 0..ticks {
            traits.extend(world.agents().iter().map(|a| (a.id, a.traits)));
            let cursor = world.event_log().len();
            world.step();
            traits.extend(world.agents().iter().map(|a| (a.id, a.traits)));
            for e in world.event_log().since(cursor) {
                if e.kind == EventKind::Consumed && e.target_was_carcass {
                    let drained = e.energy_delta as f64;
                    if traits[&e.source].photosynthetic_absorption < 0.1 {
                        expected.pure_vertex += drained;
                    } else {
                        expected.mixotroph += drained;
                    }
                }
            }
        }
        assert_eq!(MIXOTROPH_AUTOTROPHY, 0.1);
        assert!(expected.pure_vertex > 0.0);
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
        assert!(
            close(report.carcass_drain.pure_vertex, expected.pure_vertex)
                && close(report.carcass_drain.mixotroph, expected.mixotroph),
            "{:?} vs {expected:?}",
            report.carcass_drain
        );
        assert_eq!(report.arm_carcass_drain, report.carcass_drain);
    }

    /// A paired arm's drain split covers the whole horizon, the settle's
    /// history included, and separately its own span after the fork: the
    /// control's whole-horizon split is the uninterrupted run's.
    #[test]
    fn a_paired_arms_drain_split_carries_the_settles_history() {
        let spec = Mode1Spec::committed();
        let build = || {
            mode1_mesocosm(
                &spec,
                mesocosm_wear(),
                6,
                Community::ProducersAndDecomposers,
            )
        };
        let (settle, ticks) = (30, 30);
        let paired = run_paired(build(), settle, ticks, 10);
        let settled = run_mode1(&mut build(), settle, 10);
        let whole = run_mode1(&mut build(), settle + ticks, 10);

        let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
        let c = paired.control;
        assert!(close(
            c.carcass_drain.pure_vertex,
            whole.carcass_drain.pure_vertex
        ));
        assert!(close(
            c.carcass_drain.mixotroph,
            whole.carcass_drain.mixotroph
        ));
        assert!(close(
            c.arm_carcass_drain.pure_vertex,
            whole.carcass_drain.pure_vertex - settled.carcass_drain.pure_vertex
        ));
        let p = paired.perturbed;
        assert!(close(
            p.carcass_drain.pure_vertex - p.arm_carcass_drain.pure_vertex,
            settled.carcass_drain.pure_vertex
        ));
    }

    /// Both summaries carry #764's reading per arm: whether decomposers
    /// persist, the heterotroph-dominant agents alive at the end, and the
    /// carcass-drain split over the whole horizon and over the arm.
    #[test]
    fn the_summaries_report_the_decomposer_reading() {
        let mut report = Mode1Report {
            samples: vec![],
            producer_lifespans: vec![],
            senescent_lifespans: vec![],
            final_roles: RoleTally {
                producers: 5,
                decomposers: 2,
                consumers: 0,
                no_role: 0,
            },
            heterotroph_dominant: 3,
            carcass_drain: DrainSplit {
                pure_vertex: 30.0,
                mixotroph: 10.0,
            },
            arm_carcass_drain: DrainSplit {
                pure_vertex: 1.0,
                mixotroph: 3.0,
            },
            ledger: Ledger::default(),
        };
        let line = "| yes | 2 | 3 | 30.0 / 10.0 (0.250) | 1.0 / 3.0 (0.750) |";
        let summary = mode1_summary("h", &report);
        assert!(summary.contains(line), "{summary}");

        let control = report.clone();
        report.final_roles.decomposers = 0;
        report.arm_carcass_drain = DrainSplit::default();
        let paired = PairedReport {
            fork_tick: 0,
            clearance: Clearance::default(),
            perturbed: report,
            control,
        };
        let summary = paired_summary("h", &paired);
        assert!(summary.contains(&format!("| control {line}")), "{summary}");
        assert!(
            summary.contains("| perturbed | no | 0 | 3 | 30.0 / 10.0 (0.250) | 0.0 / 0.0 (-) |"),
            "{summary}"
        );
    }

    /// The end roster also counts the living agents whose nominal traits are
    /// heterotroph-dominant (heterotrophy above autotrophy), whatever their
    /// income, so a reader sees whether the decomposer founders' side of
    /// trait space is still alive (#772).
    #[test]
    fn the_end_roster_counts_heterotroph_dominant_agents_by_traits() {
        let mut world = mode1_mesocosm(
            &Mode1Spec::committed(),
            mesocosm_wear(),
            4,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, 5, 5);
        let expected = world
            .agents()
            .iter()
            .filter(|a| a.traits.heterotrophy > a.traits.photosynthetic_absorption)
            .count();
        assert!(expected > 0);
        assert_eq!(report.heterotroph_dominant, expected);

        let mut bare = mode1_mesocosm(
            &Mode1Spec::committed(),
            mesocosm_wear(),
            4,
            Community::ProducersOnly,
        );
        assert_eq!(run_mode1(&mut bare, 5, 5).heterotroph_dominant, 0);
    }
}
