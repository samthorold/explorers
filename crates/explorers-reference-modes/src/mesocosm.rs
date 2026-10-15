//! Reference-mode mesocosm (#751; reference-modes.md, *Colonisation
//! overshoot*, *Mesocosm*).

use std::collections::HashMap;

use explorers_genesis_eval::guild::{
    Birth, GUILD_MIN_SIZE, RoleSnapshot, role_guilds_from_samples,
};
use explorers_genesis_eval::income::IncomeLedger;
use explorers_sim::event::{Event, EventKind};
use explorers_sim::phase::consumption_reach;
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
    mode1_mesocosm_with_litter(spec, wear, seed, community, 1)
}

/// [`mode1_mesocosm`] standing `litter_x` times the settled litter stock: every
/// cell holds `litter_x` × [`CARCASSES_PER_CELL`] carcasses of the same size,
/// so `litter_x` times the energy and nutrient. A diagnostic instrument
/// setting (#781), not a new default: at 1 it is the committed mesocosm,
/// carcass for carcass. The extra litter is placed after the founders, so
/// the founders stand in the same places at any multiplier.
///
/// Panics if `litter_x` is 0 or the pool cannot cover the litter.
pub fn mode1_mesocosm_with_litter(
    spec: &Mode1Spec,
    wear: Wear,
    seed: u64,
    community: Community,
    litter_x: usize,
) -> World {
    mode1_mesocosm_with_founders(spec, wear, seed, community, litter_x, None)
}

/// [`mode1_mesocosm_with_litter`] with `decomposer_founders` decomposer
/// founders in place of one a cell (#829; reference-modes.md, mode 1,
/// *Decomposer founders*, *Founder density*): an instrument setting, not a
/// new default. `None` is one a cell, the committed mesocosm, state for
/// state. Set, the community with decomposers seeds that many, placed
/// uniformly over the torus like every founder; the producers-only
/// community has none at any count.
///
/// The founding litter is placed before the founders and the producers come
/// first in the roster, so the producers and the 1× litter stand where they
/// do at the default whatever the count. A litter multiplier's extra litter
/// is placed after the founders, so at `litter_x` above 1 its positions
/// follow the count.
///
/// Panics if `decomposer_founders` is `Some(0)`, or as
/// [`mode1_mesocosm_with_litter`].
pub fn mode1_mesocosm_with_founders(
    spec: &Mode1Spec,
    wear: Wear,
    seed: u64,
    community: Community,
    litter_x: usize,
    decomposer_founders: Option<usize>,
) -> World {
    assert!(litter_x >= 1, "the litter multiplier is at least 1");
    assert_ne!(
        decomposer_founders,
        Some(0),
        "a decomposer founder count is at least 1"
    );
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
        Community::ProducersAndDecomposers => {
            decomposer_founders.unwrap_or(DECOMPOSERS_PER_CELL * cells)
        }
        Community::ProducersOnly => 0,
    };
    // Producers found cluster 0 and decomposer founders cluster 1, the
    // marker's unrelated clades (#837); read only under marker isolation.
    let roster = std::iter::repeat_n(
        (producer, MODE1_PRODUCER_CLUSTER),
        PRODUCERS_PER_CELL * cells,
    )
    .chain(std::iter::repeat_n(
        (decomposer, MODE1_DECOMPOSER_CLUSTER),
        decomposers,
    ));
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let half = extent / 2.0;
    let size = params.nutrient_grid_cell_size;
    // One settled stock of litter, `per_cell` carcasses in every cell.
    let litter = |rng: &mut ChaCha8Rng, per_cell: usize| -> Vec<CarcassSpec> {
        (0..cells)
            .flat_map(|cell| std::iter::repeat_n(cell, per_cell))
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
            .collect()
    };
    // Litter is placed before the founders, so both communities stand the
    // same carcasses and the same producers.
    let mut carcasses = litter(&mut rng, CARCASSES_PER_CELL);
    let agents: Vec<AgentSpec> = roster
        .map(|(traits, founding_cluster)| AgentSpec {
            position: (rng.random_range(-half..half), rng.random_range(-half..half)),
            reserve: spec.founder_reserve,
            traits,
            nutrient: 0.0,
            founding_cluster,
        })
        .collect();
    // A multiplier's extra litter comes after the founders, so they stand
    // where they do at 1x.
    carcasses.extend(litter(&mut rng, (litter_x - 1) * CARCASSES_PER_CELL));
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

/// The coefficient of flow 3's body-reach term at which a founder's feeding
/// reach spans `pieces` litter pieces (#795; reference-modes.md, mode 1,
/// *Mesocosm → Parameters → Feeding reach*). Reach is `r = h · (c + b · √S)`
/// for effective heterotrophy `h`, contact range coefficient `c` and structure
/// `S`; a disc of radius `r_k = √(k / (π ρ))` holds `k` pieces on average at
/// litter density `ρ`, so `b = (r_k / h − c) / √S`. Where `r_k ≤ h · c` the
/// contact-only reach already spans `k` pieces, and `b` clamps to 0, the
/// unit-restricted case: returns `(b, clamped)`.
pub fn body_reach_coefficient_for_pieces(
    pieces: f32,
    heterotrophy: f32,
    structure: f32,
    litter_density: f32,
    contact_range_coefficient: f32,
) -> (f32, bool) {
    let r_k = (pieces / (std::f32::consts::PI * litter_density)).sqrt();
    let b = (r_k / heterotrophy - contact_range_coefficient) / structure.sqrt();
    if b > 0.0 { (b, false) } else { (0.0, true) }
}

/// The feeding reach `--body-reach-pieces` set (#795), read from the world
/// the run built.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct BodyReach {
    /// `k`: the litter pieces a decomposer founder's reach is to span at
    /// founding.
    pub pieces: f32,
    /// `ρ`: standing carcasses per unit area at founding.
    pub litter_density: f32,
    /// `h`: the decomposer founders' mean effective heterotrophy at founding,
    /// as the stepper reads it.
    pub founder_heterotrophy: f32,
    /// `S`: their mean founding structure, after nutrient co-limiting.
    pub founder_structure: f32,
    /// `c`: the world's contact range coefficient.
    pub contact_range_coefficient: f32,
    /// `b`: the body-reach coefficient set on the world.
    pub coefficient: f32,
    /// The founders' feeding reach at founding under `b`.
    pub founder_reach: f32,
    /// The litter pieces that reach spans, `π r² ρ`: `k`, unless clamped.
    pub pieces_spanned: f32,
    /// Whether `b` clamped to 0: the contact-only reach already spans `k`.
    pub clamped: bool,
}

/// The decomposer founders' mean effective heterotrophy, under the world's
/// wear steepness, and mean structure after co-limiting, read from a world at
/// founding: the agents at the pure heterotroph vertex. Averaged in f64, so
/// identical founders give their own value back. `None` without founders.
fn decomposer_founders(world: &World) -> Option<(f32, f32)> {
    let steepness = world.params().wear_degradation_steepness;
    let founders: Vec<(f32, f32)> = world
        .agents()
        .iter()
        .filter(|a| a.traits.photosynthetic_absorption == 0.0 && a.traits.heterotrophy > 0.0)
        .map(|a| (a.effective_trait_with_steepness(1, steepness), a.structure))
        .collect();
    if founders.is_empty() {
        return None;
    }
    let mean = |v: fn(&(f32, f32)) -> f32| {
        (founders.iter().map(|f| v(f) as f64).sum::<f64>() / founders.len() as f64) as f32
    };
    Some((mean(|f| f.0), mean(|f| f.1)))
}

/// Set `world`'s body-reach coefficient so that a decomposer founder's
/// feeding reach at founding spans `pieces` litter pieces
/// ([`body_reach_coefficient_for_pieces`]). `h` and `S` are the founders' at
/// the pure heterotroph vertex as built: effective heterotrophy under the
/// world's wear steepness and structure after co-limiting, averaged over the
/// founders. `ρ` is the world's standing carcasses over its area, so it
/// follows the litter multiplier. Call it on a world at founding. `None`,
/// leaving the world untouched, when it has no decomposer founders.
pub fn set_body_reach_pieces(world: &mut World, pieces: f32) -> Option<BodyReach> {
    let (heterotrophy, structure) = decomposer_founders(world)?;
    let litter_density = world.carcasses().len() as f32 / world.params().world_extent.powi(2);
    let c = world.params().contact_range_coefficient;
    let (coefficient, clamped) =
        body_reach_coefficient_for_pieces(pieces, heterotrophy, structure, litter_density, c);
    world.params_mut().body_reach_coefficient = coefficient;
    let founder_reach = consumption_reach(heterotrophy, structure, world.params());
    Some(BodyReach {
        pieces,
        litter_density,
        founder_heterotrophy: heterotrophy,
        founder_structure: structure,
        contact_range_coefficient: c,
        coefficient,
        founder_reach,
        pieces_spanned: std::f32::consts::PI * founder_reach.powi(2) * litter_density,
        clamped,
    })
}

/// `T_p`, the mesocosm's producer lifespan in ticks against which
/// `--litter-half-life` states litter decay (#805; world rules, *Carcass
/// decomposition saturates in its decomposers → How large `k_d` is*): the
/// median lifespan of *established* producers (age at death over 50 ticks) at
/// the committed wear, 229 ticks, as recorded in
/// `docs/research/766-mode1-wear-recalibration.md` (#804 measured 207–238 per
/// seed). Not the median over all producer deaths, about 15 ticks, which
/// seedlings dominate. A constant, because a run's own lifespan is one of its
/// outputs and is not known when `k_d` is set.
pub const ESTABLISHED_PRODUCER_LIFESPAN_TICKS: f32 = 229.0;

/// The decay constant `k_d` at which a carcass under full saturation
/// (`Σc ≫ K`) has a half-life of `lifespans` producer lifespans of
/// `producer_lifespan` ticks: `k_d = ln 2 / (L × T_p)`, per tick.
pub fn carcass_decay_constant_for_half_life(lifespans: f32, producer_lifespan: f32) -> f32 {
    std::f32::consts::LN_2 / (lifespans * producer_lifespan)
}

/// The saturating-decomposition kinetics `--litter-half-life` and
/// `--enzyme-half-saturation` set (#805), read from the world the run built.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct DecompositionKinetics {
    /// `L`: the litter half-life under full saturation, in producer
    /// lifespans.
    pub litter_half_life: f32,
    /// `T_p`: the producer lifespan in ticks
    /// ([`ESTABLISHED_PRODUCER_LIFESPAN_TICKS`]).
    pub producer_lifespan: f32,
    /// `k_d = ln 2 / (L × T_p)`, set on the world, per tick.
    pub decay_constant: f32,
    /// One decomposer founder's drain capability at founding, effective
    /// heterotrophy × `u_H` (mean over the founders): `None` without
    /// founders.
    pub founder_drain_capability: Option<f32>,
    /// `K` in founder drain capabilities: the value given, or the world's
    /// `K` re-expressed when none was. `None` without founders.
    pub half_saturation_founder_capabilities: Option<f32>,
    /// `K` in absolute units (drain capability, structure per tick), set on
    /// the world.
    pub half_saturation: f32,
}

/// Set `world`'s saturating-decomposition kinetics: `k_d` from a litter
/// half-life of `litter_half_life` producer lifespans
/// ([`carcass_decay_constant_for_half_life`] at
/// [`ESTABLISHED_PRODUCER_LIFESPAN_TICKS`]), and, when given, `K` as
/// `half_saturation` decomposer founder drain capabilities, each the
/// founders' mean effective heterotrophy at founding × `u_H` (full hunger and
/// expression: a carcass drain is ungated). Without it the world keeps its
/// `K`. Call it on a world at founding. `None`, leaving the world untouched,
/// when `K` is given and the world has no decomposer founders.
pub fn set_decomposition_kinetics(
    world: &mut World,
    litter_half_life: f32,
    half_saturation: Option<f32>,
) -> Option<DecompositionKinetics> {
    let founder_drain_capability = decomposer_founders(world)
        .map(|(h, _)| h * explorers_sim::units::HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK);
    let absolute = match (half_saturation, founder_drain_capability) {
        (Some(k), Some(c)) => k * c,
        (Some(_), None) => return None,
        (None, _) => world.params().carcass_half_saturation,
    };
    let producer_lifespan = ESTABLISHED_PRODUCER_LIFESPAN_TICKS;
    let decay_constant = carcass_decay_constant_for_half_life(litter_half_life, producer_lifespan);
    let params = world.params_mut();
    params.carcass_decay_constant = decay_constant;
    params.carcass_half_saturation = absolute;
    Some(DecompositionKinetics {
        litter_half_life,
        producer_lifespan,
        decay_constant,
        founder_drain_capability,
        half_saturation_founder_capabilities: half_saturation
            .or(founder_drain_capability.map(|c| absolute / c)),
        half_saturation: absolute,
    })
}

/// The summary's account of the decomposition kinetics set.
pub fn decomposition_kinetics_section(k: &DecompositionKinetics) -> String {
    let in_founders = |v: Option<f32>| v.map_or("-".to_string(), |v| format!("{v:.4}"));
    format!(
        "## Decomposition kinetics (--litter-half-life, --enzyme-half-saturation)\n\n\
         litter half-life L {} producer lifespans | producer lifespan T_p {} ticks | \
         decay constant k_d {:.6} per tick | founder drain capability {} | \
         half-saturation K {} founder capabilities, {:.4} absolute",
        k.litter_half_life,
        k.producer_lifespan,
        k.decay_constant,
        in_founders(k.founder_drain_capability),
        in_founders(k.half_saturation_founder_capabilities),
        k.half_saturation,
    )
}

/// `S_p`, the median structure of an established producer (alive and older
/// than [`ESTABLISHMENT_AGE`] ticks) in the committed mode 1 mesocosm's
/// control, against which `--size-scaled-base-rate` sets `b` (#807; world
/// rules, flow 8, *How large `b` is*). Read by #804
/// (`docs/research/804-mode1-litter-supply.md`): 21.6 pooled over seeds 1–5
/// on the control arms' second halves, 18,274 producer-samples, seed medians
/// 18.1–24.2. A constant, because a run's own producers are one of its
/// outputs and are not known when `b` is set.
pub const ESTABLISHED_PRODUCER_STRUCTURE: f32 = 21.6;

/// The mass-specific base rate at which an agent of structure `s_p` pays
/// under `b · S` what it pays under the per-agent rule:
/// `b = base_metabolic_rate / S_p + structure_maintenance_coefficient`.
pub fn mass_specific_base_rate_for(params: &WorldParameters, s_p: f32) -> f32 {
    params.base_metabolic_rate / s_p + params.structure_maintenance_coefficient
}

/// One agent's per-tick upkeep under one metabolic rule, as #804 reckoned it.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct Upkeep {
    /// The base rate and structure term: `base_metabolic_rate +
    /// structure_maintenance_coefficient · S` per agent, `b · S` size-scaled.
    pub base_and_structure: f32,
    /// What `metabolise` charges: the above, trait maintenance and the
    /// cross-trait cost.
    pub metabolic: f32,
    /// Movement at a full stride, `eff_mobility · u_M ·
    /// movement_cost_coefficient · S`.
    pub movement: f32,
    /// `metabolic + movement`.
    pub total: f32,
}

/// An agent's upkeep at founding under both rules.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct FoundingUpkeep {
    /// Its structure at founding.
    pub structure: f32,
    /// Under the per-agent base rate (`mass_specific_base_rate` 0).
    pub per_agent: Upkeep,
    /// Under the size-scaled base rate `b · S`.
    pub size_scaled: Upkeep,
}

/// What `--size-scaled-base-rate` set (#807), read from the world the run
/// built.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct SizeScaledBaseRate {
    /// `S_p`: [`ESTABLISHED_PRODUCER_STRUCTURE`], or the value given to
    /// [`set_size_scaled_base_rate_at`].
    pub established_producer_structure: f32,
    /// The per-agent base rate `b` replaces.
    pub base_metabolic_rate: f32,
    /// The structure coefficient `b` absorbs.
    pub structure_maintenance_coefficient: f32,
    /// `b`, set on the world, per unit structure per tick.
    pub mass_specific_base_rate: f32,
    /// A decomposer founder's upkeep, each field the mean over the founders:
    /// `None` without them.
    pub decomposer_founder: Option<FoundingUpkeep>,
    /// The producer founder of median per-agent upkeep (the lower median):
    /// `None` without producers.
    pub median_producer: Option<FoundingUpkeep>,
}

fn upkeep(agent: &explorers_sim::Agent, params: &WorldParameters) -> Upkeep {
    let metabolic = explorers_sim::phase::metabolic_cost(agent, params);
    let base_and_structure = if params.mass_specific_base_rate > 0.0 {
        params.mass_specific_base_rate * agent.structure
    } else {
        params.base_metabolic_rate + params.structure_maintenance_coefficient * agent.structure
    };
    let stride = agent.effective_trait_with_steepness(2, params.wear_degradation_steepness)
        * explorers_sim::units::MOBILITY_DISTANCE_PER_TICK;
    let movement = stride * params.movement_cost_coefficient * agent.structure;
    Upkeep {
        base_and_structure,
        metabolic,
        movement,
        total: metabolic + movement,
    }
}

/// Turn the size-scaled base rate on in `world`, at `b` from
/// [`ESTABLISHED_PRODUCER_STRUCTURE`] ([`mass_specific_base_rate_for`]), and
/// record the founders' upkeep under both rules. Call it on a world at
/// founding, with the switch off.
pub fn set_size_scaled_base_rate(world: &mut World) -> SizeScaledBaseRate {
    set_size_scaled_base_rate_at(world, ESTABLISHED_PRODUCER_STRUCTURE)
}

/// [`set_size_scaled_base_rate`] at an `S_p` given explicitly (#814), for a
/// reading that settles `S_p` under the rules it runs with rather than
/// #804's per-agent world. `s_p` is a structure, finite and above 0.
pub fn set_size_scaled_base_rate_at(world: &mut World, s_p: f32) -> SizeScaledBaseRate {
    let per_agent = WorldParameters {
        mass_specific_base_rate: 0.0,
        ..world.params().clone()
    };
    let b = mass_specific_base_rate_for(&per_agent, s_p);
    let size_scaled = WorldParameters {
        mass_specific_base_rate: b,
        ..per_agent.clone()
    };
    let founding = |a: &explorers_sim::Agent| FoundingUpkeep {
        structure: a.structure,
        per_agent: upkeep(a, &per_agent),
        size_scaled: upkeep(a, &size_scaled),
    };
    let decomposers: Vec<FoundingUpkeep> = world
        .agents()
        .iter()
        .filter(|a| a.traits.photosynthetic_absorption == 0.0 && a.traits.heterotrophy > 0.0)
        .map(founding)
        .collect();
    let mean = |v: &dyn Fn(&FoundingUpkeep) -> f32| {
        (decomposers.iter().map(|f| v(f) as f64).sum::<f64>() / decomposers.len() as f64) as f32
    };
    let mean_upkeep = |u: fn(&FoundingUpkeep) -> &Upkeep| Upkeep {
        base_and_structure: mean(&|f| u(f).base_and_structure),
        metabolic: mean(&|f| u(f).metabolic),
        movement: mean(&|f| u(f).movement),
        total: mean(&|f| u(f).total),
    };
    let decomposer_founder = (!decomposers.is_empty()).then(|| FoundingUpkeep {
        structure: mean(&|f| f.structure),
        per_agent: mean_upkeep(|f| &f.per_agent),
        size_scaled: mean_upkeep(|f| &f.size_scaled),
    });
    let mut producers: Vec<FoundingUpkeep> = world
        .agents()
        .iter()
        .filter(|a| a.traits.photosynthetic_absorption > 0.0)
        .map(founding)
        .collect();
    producers.sort_by(|a, b| a.per_agent.total.total_cmp(&b.per_agent.total));
    let median_producer = (!producers.is_empty()).then(|| producers[(producers.len() - 1) / 2]);
    world.params_mut().mass_specific_base_rate = b;
    SizeScaledBaseRate {
        established_producer_structure: s_p,
        base_metabolic_rate: per_agent.base_metabolic_rate,
        structure_maintenance_coefficient: per_agent.structure_maintenance_coefficient,
        mass_specific_base_rate: b,
        decomposer_founder,
        median_producer,
    }
}

/// The summary's account of the size-scaled base rate set.
pub fn size_scaled_base_rate_section(r: &SizeScaledBaseRate) -> String {
    use std::fmt::Write;
    let mut out = format!(
        "## Size-scaled base rate (--size-scaled-base-rate)\n\n\
         b = base_metabolic_rate / S_p + structure_maintenance_coefficient = \
         {} / {} + {} = {:.5} per unit structure per tick\n\n\
         | founder at founding | structure | rule | base and structure | metabolic | movement (full stride) | upkeep |\n\
         |---|---:|---|---:|---:|---:|---:|",
        r.base_metabolic_rate,
        r.established_producer_structure,
        r.structure_maintenance_coefficient,
        r.mass_specific_base_rate,
    );
    for (name, f) in [
        ("decomposer (mean)", r.decomposer_founder),
        ("producer (median)", r.median_producer),
    ] {
        let Some(f) = f else { continue };
        for (rule, u) in [("per agent", f.per_agent), ("size-scaled", f.size_scaled)] {
            write!(
                out,
                "\n| {name} | {:.3} | {rule} | {:.4} | {:.4} | {:.4} | {:.4} |",
                f.structure, u.base_and_structure, u.metabolic, u.movement, u.total
            )
            .unwrap();
        }
    }
    out
}

/// Mode 1's living-tissue resistance `ρ` (world rules, *Consumption is
/// incremental → Living tissue resists drains*, *How large `ρ` is*): mode 1's
/// heterotrophs are decomposers and producer-line mixotrophs, consumers not
/// adapted to living tissue, so the domain closes it to them. The domain's
/// value for those consumers, not a fitted one.
pub const MODE1_LIVING_RESISTANCE: f32 = 1.0;

/// Turn living-tissue resistance on in `world` at [`MODE1_LIVING_RESISTANCE`]
/// and return the `ρ` set.
pub fn set_living_resistance(world: &mut World) -> f32 {
    world.params_mut().living_resistance = MODE1_LIVING_RESISTANCE;
    MODE1_LIVING_RESISTANCE
}

/// The summary section recording the living-tissue resistance a run set.
pub fn living_resistance_section(rho: f32) -> String {
    format!(
        "## Living-tissue resistance (--living-resistance)\n\n\
         ρ = {rho}: every drain on a living target is multiplied by 1 − ρ = {}; \
         carcass drains are unchanged",
        1.0 - rho
    )
}

/// Mode 1's producers' founding cluster under marker isolation (#837).
pub const MODE1_PRODUCER_CLUSTER: u32 = 0;
/// Mode 1's decomposer founders' founding cluster under marker isolation
/// (#837): a separately arriving clade, unrelated to the producers.
pub const MODE1_DECOMPOSER_CLUSTER: u32 = 1;

/// The marker isolation a run set (#837; world rules, flow 4,
/// *Reproductive isolation follows ancestry, through a neutral marker*).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct MarkerIsolation {
    /// `s`: compatibility is `exp(−d² / (2s²))` at marker distance `d`.
    pub compatibility_scale: f32,
}

/// Turn marker isolation on in `spec` at compatibility scale `s`, and return
/// it. It acts at world creation, where each founding cluster's founders
/// take its marker origin, so it is set on the spec the mesocosm is built
/// from, not on a built world.
pub fn set_marker_isolation(spec: &mut Mode1Spec, s: f32) -> MarkerIsolation {
    spec.parameters.marker_isolation = true;
    spec.parameters.compatibility_scale = s;
    MarkerIsolation {
        compatibility_scale: s,
    }
}

/// The summary section recording the marker isolation a run set.
pub fn marker_isolation_section(m: &MarkerIsolation) -> String {
    format!(
        "## Marker isolation (--marker-isolation, --compatibility-scale)\n\n\
         compatibility scale s = {}: a pair mates only past one draw against \
         c = exp(−d² / (2s²)) at marker distance d; producers found cluster {}, \
         decomposer founders cluster {}, 10 s apart (c = e⁻⁵⁰)",
        m.compatibility_scale, MODE1_PRODUCER_CLUSTER, MODE1_DECOMPOSER_CLUSTER
    )
}

/// The founder line at one sample under marker isolation (#837).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct LineSample {
    /// The world's tick at the sample.
    pub tick: u64,
    /// Living members of the founder line.
    pub line_living: usize,
    /// Those with nominal autotrophy below [`MIXOTROPH_AUTOTROPHY`], as
    /// [`DrainSplit`] reads it.
    pub below_autotrophy: usize,
    /// Their share of the line (`None` with the line extinct).
    pub share_below_autotrophy: Option<f64>,
}

/// Pairwise marker compatibility within a set of living agents (#837).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct PairwiseCompatibility {
    pub members: usize,
    pub pairs: usize,
    /// Mean `c` over every pair (`None` with fewer than two members).
    pub mean: Option<f64>,
    /// The lowest `c` of any pair.
    pub lowest: Option<f64>,
}

impl PairwiseCompatibility {
    /// Over every pair of `agents` at compatibility scale `s`.
    pub fn of(agents: &[&explorers_sim::Agent], s: f32) -> Self {
        let mut out = Self {
            members: agents.len(),
            ..Self::default()
        };
        let (mut sum, mut lowest) = (0.0_f64, f64::INFINITY);
        for (i, a) in agents.iter().enumerate() {
            for b in &agents[i + 1..] {
                let c = explorers_sim::marker_compatibility(&a.marker, &b.marker, s);
                sum += c;
                lowest = lowest.min(c);
                out.pairs += 1;
            }
        }
        if out.pairs > 0 {
            out.mean = Some(sum / out.pairs as f64);
            out.lowest = Some(lowest);
        }
        out
    }
}

/// The marker readout of a run or arm under marker isolation (#837;
/// reference-modes.md, mode 1, *Crossing*).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct MarkerReadout {
    /// The founder line at each of the run's or arm's centre samples.
    pub samples: Vec<LineSample>,
    /// Pairwise compatibility within the founder line at the end.
    pub line: PairwiseCompatibility,
    /// Pairwise compatibility within the producers at the end: the living
    /// agents the income read calls producers.
    pub producers: PairwiseCompatibility,
}

/// The summary section of the marker readout, one row per run or arm.
pub fn marker_readout_section(rows: &[(&str, &MarkerReadout)]) -> String {
    let or_dash = |v: Option<f64>| v.map_or_else(|| "-".to_string(), |v| format!("{v:.4}"));
    let mut out = String::from(
        "## Marker readout (#837)\n\n\
         The founder line's living members at each sample, with the share below autotrophy 0.1, \
         then pairwise marker compatibility c at the end within the line and within the \
         producers.\n\n\
         | run | line living by sample (share below autotrophy 0.1) | line members | line mean c | \
         line lowest c | producers | producer mean c | producer lowest c |\n\
         |---|---|---|---|---|---|---|---|\n",
    );
    for (name, r) in rows {
        let series: Vec<String> = (r.samples.iter())
            .map(|s| {
                let share = s
                    .share_below_autotrophy
                    .map_or("-".into(), |v| format!("{v:.2}"));
                format!("{}: {} ({share})", s.tick, s.line_living)
            })
            .collect();
        out.push_str(&format!(
            "| {name} | {} | {} | {} | {} | {} | {} | {} |\n",
            series.join(", "),
            r.line.members,
            or_dash(r.line.mean),
            or_dash(r.line.lowest),
            r.producers.members,
            or_dash(r.producers.mean),
            or_dash(r.producers.lowest),
        ));
    }
    out
}

/// The starvation cascade's switches a run set (#821; world rules, flow 8,
/// *When reserve cannot pay: the starvation cascade*).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct StarvationCascade {
    /// Step 1: upkeep reserve cannot pay is drawn from the energy earmark.
    pub draws_earmark: bool,
    /// Step 2: what is left is paid by burning structure (shrinking).
    pub shrinks: bool,
}

/// Set the starvation cascade's two switches in `world` and return them.
pub fn set_starvation_cascade(
    world: &mut World,
    draws_earmark: bool,
    shrinks: bool,
) -> StarvationCascade {
    let params = world.params_mut();
    params.starvation_draws_earmark = draws_earmark;
    params.starvation_shrinks = shrinks;
    StarvationCascade {
        draws_earmark,
        shrinks,
    }
}

/// The summary section recording the starvation cascade a run set.
pub fn starvation_cascade_section(c: &StarvationCascade) -> String {
    let on = |b: bool| if b { "on" } else { "off" };
    format!(
        "## Starvation cascade (--starvation-draws-earmark, --starvation-shrinks)\n\n\
         upkeep reserve cannot pay is drawn from the energy earmark: {}; \
         what is left is paid by burning structure (shrinking): {}",
        on(c.draws_earmark),
        on(c.shrinks)
    )
}

/// The summary's account of the feeding reach `--body-reach-pieces` set.
pub fn body_reach_section(r: &BodyReach) -> String {
    format!(
        "## Feeding reach (--body-reach-pieces)\n\n\
         pieces k {} | litter density ρ {:.4} per unit area | founder effective heterotrophy h {:.4} | \
         founder structure S {:.4} | contact range coefficient c {:.4} | body reach coefficient b {:.4} | \
         founder reach at founding {:.3} | pieces spanned {:.2} | clamped to 0: {}",
        r.pieces,
        r.litter_density,
        r.founder_heterotrophy,
        r.founder_structure,
        r.contact_range_coefficient,
        r.coefficient,
        r.founder_reach,
        r.pieces_spanned,
        if r.clamped { "yes" } else { "no" }
    )
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

/// Whether an agent is a member of the mesocosm's decomposer guild
/// (reference-modes.md, mode 1, *Community*): the income read calls it a
/// decomposer, and its heterotrophy exceeds its autotrophy. The line is drawn
/// by the agent's own investment, with no threshold of its own, so a
/// producer-line mixotroph living on carcasses carries the role and is out.
pub fn is_guild_member(role: Option<TrophicRole>, traits: &TraitVector) -> bool {
    role == Some(TrophicRole::Decomposer) && traits.heterotrophy > traits.photosynthetic_absorption
}

/// What the guild's members drained (`Consumed` events, structure before
/// trophic efficiency), split between living targets and carcasses, beside
/// the carcass drain by every agent over the same steps.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct GuildDrain {
    /// Drained from living targets by members.
    pub living: f64,
    /// Drained from carcasses by members.
    pub carcass: f64,
    /// Drained from carcasses by every agent, members or not.
    pub all_carcass: f64,
}

impl GuildDrain {
    /// Book one event; `member` says whether its source was a guild member
    /// when it drained. Anything but a drain is ignored.
    pub fn book(&mut self, event: &Event, member: bool) {
        if event.kind != EventKind::Consumed {
            return;
        }
        let drained = event.energy_delta as f64;
        if event.target_was_carcass {
            self.all_carcass += drained;
        }
        if member {
            if event.target_was_carcass {
                self.carcass += drained;
            } else {
                self.living += drained;
            }
        }
    }

    /// The members' share of all the carcass drain (`None` with none drained).
    pub fn carcass_share(&self) -> Option<f64> {
        (self.all_carcass > 0.0).then(|| self.carcass / self.all_carcass)
    }

    /// The living targets' share of the members' drain (`None` with none):
    /// a guild that persists by grazing the stand is not a saprotroph guild.
    pub fn living_share(&self) -> Option<f64> {
        let total = self.living + self.carcass;
        (total > 0.0).then(|| self.living / total)
    }

    fn minus(&self, earlier: &GuildDrain) -> GuildDrain {
        GuildDrain {
            living: self.living - earlier.living,
            carcass: self.carcass - earlier.carcass,
            all_carcass: self.all_carcass - earlier.all_carcass,
        }
    }
}

/// Mode 1's reading (reference-modes.md, mode 1, *Feeding reach → The
/// reading*): whether a heterotroph-dominant decomposer guild holds over the
/// second half of a run or arm, what it drained there, and whether it holds
/// with the producers persisting.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct GuildReadout {
    /// The committed *heterotroph guild* predicate
    /// ([`role_guilds_from_samples`]) over the [`is_guild_member`] agents:
    /// at least `GUILD_MIN_SIZE` on every sample of the second half, and a
    /// birth there with a member as parent.
    pub holds: bool,
    /// The fewest members on any sample of the second half (0 with none).
    pub fewest_members: usize,
    /// The members' drain over the second half's steps.
    pub drain: GuildDrain,
    /// The centre patch's producers persist ([`Mode1Report::producers_persist`]).
    pub producers_persist: bool,
    /// The guild holds and the producers persist.
    pub passes: bool,
}

/// Litter inflow (#804): the structure of the carcasses deaths create, split
/// by whether the dead was a producer (the income read) or anything else.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct LitterInflow {
    /// Carcass structure at death from producer deaths.
    pub producer: f64,
    /// Carcass structure at death from every other death.
    pub other: f64,
}

impl LitterInflow {
    /// Book one event. A `Died` adds the structure its carcass held at death,
    /// read from `at_death` by the carcass's id (a runtime carcass keeps its
    /// agent's id); `producer` says whether the dead was a producer. Anything
    /// else is ignored.
    pub fn book(&mut self, event: &Event, producer: bool, at_death: &HashMap<u64, f32>) {
        if event.kind != EventKind::Died {
            return;
        }
        let structure = *at_death
            .get(&event.source)
            .expect("every death leaves a carcass") as f64;
        if producer {
            self.producer += structure;
        } else {
            self.other += structure;
        }
    }

    /// Producer and other inflow together.
    pub fn total(&self) -> f64 {
        self.producer + self.other
    }
}

/// One sample of the whole world's litter (#804), taken with each
/// [`CentreSample`].
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct LitterSample {
    /// The world's tick at the sample.
    pub tick: u64,
    /// Standing litter: the summed structure (energy) of every carcass, over
    /// the world's nutrient cells.
    pub standing_per_cell: f64,
    /// Carcasses standing, spent ones included: with saturating decomposition
    /// off, a drained carcass stands at structure 0 and is never removed.
    pub carcasses: usize,
    /// Carcasses still holding structure.
    pub nonempty_carcasses: usize,
    /// Litter inflow over the whole world since the previous sample (world
    /// totals, not per cell).
    pub inflow: LitterInflow,
    /// The ticks that inflow covers (0 for a run's first sample).
    pub inflow_ticks: u64,
    /// The structure of each living producer (the income read) older than
    /// [`ESTABLISHMENT_AGE`], in roster order.
    pub established_structures: Vec<f32>,
}

/// Mean, least and greatest of a series.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct Spread {
    pub mean: f64,
    pub min: f64,
    pub max: f64,
}

impl Spread {
    /// The spread of `values` around a `mean` given apart (a rate's mean is
    /// its total over its time, not the mean of its intervals). All zero
    /// with no values.
    fn of(values: &[f64], mean: f64) -> Self {
        if values.is_empty() {
            return Self::default();
        }
        Self {
            mean,
            min: values.iter().copied().fold(f64::INFINITY, f64::min),
            max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        }
    }

    /// The spread of `values` around their own mean.
    fn of_mean(values: &[f64]) -> Self {
        let mean = values.iter().sum::<f64>() / values.len().max(1) as f64;
        Self::of(values, mean)
    }
}

/// Distribution of a structure, quantiles by nearest rank as
/// [`LifespanDistribution`]'s; all zero with no values.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct StructureDistribution {
    pub count: usize,
    pub min: f64,
    pub p10: f64,
    pub p25: f64,
    pub median: f64,
    pub p75: f64,
    pub p90: f64,
    pub max: f64,
    pub mean: f64,
}

impl StructureDistribution {
    pub fn of(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self::default();
        }
        let mut sorted = values.to_vec();
        sorted.sort_unstable_by(f64::total_cmp);
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
            mean: sorted.iter().sum::<f64>() / n as f64,
        }
    }
}

/// The litter reading over a run's or arm's second half (#804).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct LitterWindow {
    /// The ticks of the window's first and last samples: the second half of
    /// the series, as [`Mode1Report::producers_persist`] reads it.
    pub from_tick: u64,
    pub to_tick: u64,
    /// Standing litter per cell over the window's samples.
    pub standing_per_cell: Spread,
    /// Carcasses standing, and those holding structure, over its samples.
    pub carcasses: Spread,
    pub nonempty_carcasses: Spread,
    /// Inflow per cell per tick over the sample intervals ending after the
    /// window's first sample: the mean is the window's total over its time,
    /// the least and greatest are the intervals'.
    pub inflow: Spread,
    pub producer_inflow: Spread,
    pub other_inflow: Spread,
    /// The structure of the established producers alive at each of the
    /// window's samples, pooled: `S_p` (world rules, flow 8, *How large `b`
    /// is*).
    pub established_structure: StructureDistribution,
    /// The median age at death of the run's or arm's producers that died
    /// older than [`ESTABLISHMENT_AGE`] (0 with none).
    pub established_lifespan_median: u64,
}

/// The litter readout of a run or arm (#804): the series at the sample
/// cadence and its second half's reading.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct LitterReadout {
    pub samples: Vec<LitterSample>,
    pub second_half: LitterWindow,
}

impl LitterWindow {
    /// Read the second half of `samples`, with the run's or arm's producer
    /// `lifespans`.
    fn read(samples: &[LitterSample], lifespans: &[u64]) -> Self {
        let from = samples.len() / 2;
        let half = &samples[from..];
        let Some(first) = half.first() else {
            return Self::default();
        };
        let cells = (SIDE_CELLS * SIDE_CELLS) as f64;
        let of = |f: fn(&LitterSample) -> f64| -> Vec<f64> { half.iter().map(f).collect() };
        let intervals = &half[1..];
        let ticks: u64 = intervals.iter().map(|s| s.inflow_ticks).sum();
        let rate = |f: fn(&LitterInflow) -> f64| {
            let rates: Vec<f64> = (intervals.iter())
                .filter(|s| s.inflow_ticks > 0)
                .map(|s| f(&s.inflow) / (cells * s.inflow_ticks as f64))
                .collect();
            let total: f64 = intervals.iter().map(|s| f(&s.inflow)).sum();
            Spread::of(&rates, total / (cells * ticks.max(1) as f64))
        };
        let established: Vec<u64> = (lifespans.iter().copied())
            .filter(|&a| a > ESTABLISHMENT_AGE)
            .collect();
        Self {
            from_tick: first.tick,
            to_tick: half[half.len() - 1].tick,
            standing_per_cell: Spread::of_mean(&of(|s| s.standing_per_cell)),
            carcasses: Spread::of_mean(&of(|s| s.carcasses as f64)),
            nonempty_carcasses: Spread::of_mean(&of(|s| s.nonempty_carcasses as f64)),
            inflow: rate(LitterInflow::total),
            producer_inflow: rate(|i| i.producer),
            other_inflow: rate(|i| i.other),
            established_structure: StructureDistribution::of(
                &(half.iter())
                    .flat_map(|s| s.established_structures.iter().map(|&x| x as f64))
                    .collect::<Vec<_>>(),
            ),
            established_lifespan_median: LifespanDistribution::of(&established).median,
        }
    }
}

/// The founders' fate and income (#781): the agents at the pure heterotroph
/// vertex when the run began (nominal autotrophy 0, heterotrophy above 0),
/// tracked by id. In the mesocosm these are the decomposer founders.
///
/// Income is energy *gained*: each drain's structure times the trophic
/// efficiency the stepper applied to it (flow 7, for the founder's traits
/// against the target's). A `Consumed` event's `energy_delta` is the
/// structure drained, before efficiency (764-founder-death-diagnosis.md,
/// *A reading trap*), so the readout books both and keeps them apart.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct FounderReadout {
    /// Founders at the run's start.
    pub founders: usize,
    /// The world's tick at the run's start.
    pub start_tick: u64,
    /// The world's tick at the run's (or arm's) end.
    pub end_tick: u64,
    /// The world's tick after the step in which each founder died.
    pub death_ticks: Vec<u64>,
    /// Energy the founders gained in each step from the start, while any
    /// founder lived as the step began: entry `i` is the step ending at tick
    /// `start_tick + i + 1`.
    pub gained: Vec<f64>,
    /// The structure they drained in the same steps, before efficiency.
    pub drained: Vec<f64>,
    /// Living agents at the end descended from a founder, founders included:
    /// the founders' line.
    pub line_alive_at_end: usize,
    /// Each founder through the starvation cascade (#821). Not serialised
    /// with the report, so a run's artifact is unchanged unless the
    /// instrument writes it on its own.
    #[serde(skip)]
    pub starvation: StarvationReadout,
}

impl FounderReadout {
    /// Founders alive at the world's tick `tick` (from the start on).
    pub fn alive_at(&self, tick: u64) -> usize {
        self.founders - self.death_ticks.iter().filter(|&&d| d <= tick).count()
    }

    /// Energy gained over the steps ending at ticks `from` through `to`.
    fn window_gain(&self, from: u64, to: u64) -> f64 {
        (from..=to)
            .filter_map(|t| t.checked_sub(self.start_tick + 1))
            .filter_map(|i| self.gained.get(i as usize))
            .sum()
    }

    /// Mean founder income over the steps ending at ticks `from` through
    /// `to`, per living founder-tick: the energy gained over the founders
    /// alive as each step began (`None` with none alive).
    pub fn mean_income(&self, from: u64, to: u64) -> Option<f64> {
        let founder_ticks: usize = (from..=to).map(|t| self.alive_at(t - 1)).sum();
        (founder_ticks > 0).then(|| self.window_gain(from, to) / founder_ticks as f64)
    }

    /// The same gain per founding founder per tick, the dead counting as
    /// earning nothing (`None` with no founders).
    pub fn mean_income_per_founder(&self, from: u64, to: u64) -> Option<f64> {
        let steps = (to + 1 - from) as f64;
        (self.founders > 0).then(|| self.window_gain(from, to) / (self.founders as f64 * steps))
    }
}

/// One decomposer founder through the starvation cascade (#821).
///
/// Its unallocated reserve *runs out* in the first step it begins with
/// reserve no greater than that step's upkeep (`metabolic_cost`, what the
/// stepper's metabolism charges): metabolism leaves the reserve at zero, and
/// any rest of the upkeep falls to the cascade, or with both switches off the
/// founder starves. Upkeep is paid in the third phase of a step, and a
/// decomposer founder's reserve and structure do not change in the two
/// before it, so its state as the step begins is the state metabolism reads.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct FounderStarvation {
    pub id: u64,
    /// The world's tick after the step in which it died (`None` alive at
    /// the end).
    pub death_tick: Option<u64>,
    /// The world's tick after the step in which its unallocated reserve
    /// first ran out (`None` if it never did).
    pub ran_out_tick: Option<u64>,
    /// Whether it had reproduced in a step before that one (before the end,
    /// if its reserve never ran out).
    pub reproduced_before_running_out: bool,
    /// Its energy earmark as that step began.
    pub earmark_at_running_out: Option<f32>,
    /// Steps it lived through in which the earmark paid some of its upkeep.
    pub ticks_on_earmark: u64,
    /// Steps it lived through in which burned structure paid some of it.
    pub ticks_on_structure: u64,
}

/// The decomposer founders through the starvation cascade (#821), by id.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct StarvationReadout {
    pub founders: Vec<FounderStarvation>,
}

/// The nearest-rank median, as [`LifespanDistribution`] reads its quantiles
/// (`None` when empty).
fn nearest_rank_median<T: Copy + PartialOrd>(mut values: Vec<T>) -> Option<T> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    Some(values[values.len().div_ceil(2) - 1])
}

impl StarvationReadout {
    /// Founders that died within the run.
    pub fn died(&self) -> usize {
        self.founders
            .iter()
            .filter(|f| f.death_tick.is_some())
            .count()
    }

    /// Founders whose unallocated reserve ran out within the run.
    pub fn ran_out(&self) -> usize {
        self.founders
            .iter()
            .filter(|f| f.ran_out_tick.is_some())
            .count()
    }

    /// Founders that reproduced before their reserve ran out.
    pub fn reproduced_first(&self) -> usize {
        (self.founders.iter())
            .filter(|f| f.reproduced_before_running_out)
            .count()
    }

    /// Median death tick over the founders that died.
    pub fn median_death_tick(&self) -> Option<u64> {
        nearest_rank_median(self.founders.iter().filter_map(|f| f.death_tick).collect())
    }

    /// Median earmark held as reserve ran out, over the founders whose did.
    pub fn median_earmark_at_running_out(&self) -> Option<f32> {
        nearest_rank_median(
            (self.founders.iter())
                .filter_map(|f| f.earmark_at_running_out)
                .collect(),
        )
    }

    fn median_over_ran_out(&self, ticks: impl Fn(&FounderStarvation) -> u64) -> Option<u64> {
        nearest_rank_median(
            (self.founders.iter())
                .filter(|f| f.ran_out_tick.is_some())
                .map(ticks)
                .collect(),
        )
    }

    /// Median ticks lived on the earmark, over the founders that ran out.
    pub fn median_ticks_on_earmark(&self) -> Option<u64> {
        self.median_over_ran_out(|f| f.ticks_on_earmark)
    }

    /// Median ticks lived on structure, over the founders that ran out.
    pub fn median_ticks_on_structure(&self) -> Option<u64> {
        self.median_over_ran_out(|f| f.ticks_on_structure)
    }
}

/// The founder starvation readout's summary section (#821): one row per run
/// or arm.
pub fn founder_starvation_section(rows: &[(&str, &StarvationReadout)]) -> String {
    let or_dash = |v: Option<String>| v.unwrap_or_else(|| "-".to_string());
    let mut out = String::from(
        "## Founder starvation readout (#821)\n\n\
         A founder's unallocated reserve runs out in the first step it begins with reserve no \
         greater than that step's upkeep. Medians are nearest-rank: the death tick over the \
         founders that died, the rest over the founders whose reserve ran out.\n\n\
         | run | founders | died | median death tick | reserve ran out | reproduced first | \
         median earmark at running out | median ticks on earmark | median ticks on structure |\n\
         |---|---|---|---|---|---|---|---|---|\n",
    );
    for (name, r) in rows {
        out.push_str(&format!(
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.founders.len(),
            r.died(),
            or_dash(r.median_death_tick().map(|t| t.to_string())),
            r.ran_out(),
            r.reproduced_first(),
            or_dash(r.median_earmark_at_running_out().map(|e| format!("{e:.3}"))),
            or_dash(r.median_ticks_on_earmark().map(|t| t.to_string())),
            or_dash(r.median_ticks_on_structure().map(|t| t.to_string())),
        ));
    }
    out
}

/// Why a member of the founder line died (#833; reference-modes.md, mode 1,
/// *Recruitment diagnostic*).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathCause {
    /// By age: the stepper marked the death `Senesced` before its `Died`.
    Senesced,
    /// Not by age, having entered the step with unallocated reserve at or
    /// below that step's upkeep (`metabolic_cost`, as [`FounderStarvation`]
    /// reads running out).
    Reserve,
    /// Neither: it entered the step with reserve above upkeep, so its body
    /// failed (structure below its death threshold) or it was drained to
    /// death. A member born in the step it died in has no entering state and
    /// is counted here.
    Structural,
}

impl DeathCause {
    /// The cause of a death, from whether the stepper marked it senescent and
    /// the member's unallocated reserve and upkeep as the step began (`None`
    /// if it was born in that step).
    pub fn of(senesced: bool, entering: Option<(f32, f32)>) -> Self {
        match entering {
            _ if senesced => DeathCause::Senesced,
            Some((reserve, upkeep)) if reserve <= upkeep => DeathCause::Reserve,
            _ => DeathCause::Structural,
        }
    }
}

/// One decomposer founder's life, tick by tick (#833). Entry `i` of each
/// series is the step ending at tick `first_tick + i + 1`: the state is the
/// founder's as that step began, the upkeep is what that step's metabolism
/// charges (`metabolic_cost`), and the gain and drain are its own in that
/// step (gain after trophic efficiency, drain before it, as
/// [`FounderReadout`] books them).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct FounderLife {
    pub id: u64,
    /// The world's tick as its first recorded step began: the founding.
    pub first_tick: u64,
    /// Unallocated reserve.
    pub reserve: Vec<f32>,
    /// The energy earmark.
    pub repro_reserve: Vec<f32>,
    /// The nutrient earmark.
    pub repro_nutrient: Vec<f32>,
    /// Free nutrient.
    pub nutrient: Vec<f32>,
    pub structure: Vec<f32>,
    pub peak_structure: Vec<f32>,
    /// Mean wear over the functional traits.
    pub wear: Vec<f32>,
    pub upkeep: Vec<f32>,
    pub gained: Vec<f64>,
    pub drained: Vec<f64>,
    /// The world's tick after the step in which it died (`None` alive at the
    /// end, or cleared with the centre patch).
    pub death_tick: Option<u64>,
    pub death_cause: Option<DeathCause>,
}

/// A parent of a line birth (#833).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct LineParent {
    pub id: u64,
    /// Whether it was in the founder line when it reproduced.
    pub in_line: bool,
    /// Its nominal autotrophy and heterotrophy (`None` only if the observer
    /// never saw it standing).
    pub autotrophy: Option<f32>,
    pub heterotrophy: Option<f32>,
}

/// A birth with a parent in the founder line (#833): the offspring joins the
/// line.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct LineBirth {
    /// The world's tick after the step it was born in.
    pub tick: u64,
    pub offspring: u64,
    /// Two parents rather than one.
    pub sexual: bool,
    /// The parent the stepper names first, then the second parent if sexual.
    pub parents: Vec<LineParent>,
    /// The offspring's nominal autotrophy and heterotrophy, and the nutrient
    /// it holds at the end of its birth step (free, earmarked and bound: its
    /// share of the parents' donated earmark). `None` if it died in that
    /// step.
    pub autotrophy: Option<f32>,
    pub heterotrophy: Option<f32>,
    pub nutrient_endowment: Option<f32>,
}

/// A death in the founder line (#833).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct LineDeath {
    pub id: u64,
    /// The world's tick after the step it died in.
    pub tick: u64,
    /// Ticks from its birth, or from the observer's start for a founder.
    pub age: u64,
    pub cause: DeathCause,
    /// Its nominal autotrophy and heterotrophy (`None` only for a member
    /// that died in its birth step, which the observer never saw standing).
    pub autotrophy: Option<f32>,
    pub heterotrophy: Option<f32>,
    /// Its births while in the line.
    pub offspring: u32,
}

/// Carcass drain over one sample interval by lineage (#833): each side
/// split by the drainer's autotrophy as [`DrainSplit`] splits it.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct LineageDrain {
    /// The world's tick at the sample closing the interval.
    pub tick: u64,
    /// Drained by members of the founder line.
    pub founder_line: DrainSplit,
    /// Drained by every other agent.
    pub other: DrainSplit,
}

/// The founder line read for recruitment (#833; reference-modes.md, mode 1,
/// *Recruitment diagnostic*), from the world's founding: a paired arm's
/// carries the settle's history. Autotrophy and heterotrophy are nominal
/// traits (`photosynthetic_absorption` and `heterotrophy`), as
/// [`DrainSplit`] and the heterotroph-dominant count read them.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct RecruitmentReadout {
    /// Each founder's life history, by id.
    pub founders: Vec<FounderLife>,
    /// Every birth with a parent in the line, in order.
    pub births: Vec<LineBirth>,
    /// Every death in the line, in order.
    pub deaths: Vec<LineDeath>,
    /// The carcass drain by lineage at each of this run's or arm's centre
    /// samples, over the interval since the sample before it (the first
    /// sample's, since the settle's last sample in a paired arm).
    pub carcass_drain: Vec<LineageDrain>,
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
    /// The founders' fate and income from the world's founding (#781); a
    /// paired arm's carries the settle's history.
    pub founders: FounderReadout,
    /// The decomposer guild over the run's or arm's second half (#794).
    pub guild: GuildReadout,
    /// The whole world's litter, sampled with the centre patch, and its
    /// second half's reading (#804).
    pub litter: LitterReadout,
    /// The founder line read for recruitment (#833): `None`, and absent from
    /// the artifact, without founders.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recruitment: Option<RecruitmentReadout>,
    /// The marker readout (#837): `None`, and absent from the artifact,
    /// without marker isolation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<MarkerReadout>,
}

impl Mode1Report {
    /// Whether the centre patch held producers on every sample of the second
    /// half of the series.
    pub fn producers_persist(&self) -> bool {
        let second_half = &self.samples[self.samples.len() / 2..];
        second_half.iter().map(|s| s.producers).min().unwrap_or(0) > 0
    }
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
pub fn run_paired(world: World, settle: u64, ticks: u64, sample_every: u64) -> PairedReport {
    run_paired_recording(world, settle, ticks, sample_every, true)
}

/// [`run_paired`], with the recruitment readout (#833) recorded or not: off
/// only to test that it leaves the rest of the report alone.
fn run_paired_recording(
    mut world: World,
    settle: u64,
    ticks: u64,
    sample_every: u64,
    record_recruitment: bool,
) -> PairedReport {
    let mut observer = Observer::new(&mut world);
    observer.record_recruitment = record_recruitment;
    let start = world.tick();
    for _ in 0..settle {
        world.step();
        observer.observe(&mut world);
        let elapsed = world.tick() - start;
        if elapsed < settle && elapsed % sample_every == 0 {
            // The settle's samples before the fork are not reported, but
            // taking them resets the birth and death counts, so the fork's
            // sample counts only those since the settle's last sample, as a
            // run sampled straight through does. The litter inflow
            // likewise.
            observer.sample_centre(&world);
            observer.sample_litter(&world);
            observer.sample_lineage_drain(&world);
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
    // The guild is read over the arm's second half, ticks after `start` from
    // `window_start` on, as the committed predicate reads a run's; its drain
    // over the steps that end there.
    let window_start = ticks / 2 + 1;
    observer.births_log.clear();
    let mut guild_drain_at_window = observer.guild_drain;
    let mut guild_samples: Vec<RoleSnapshot> = vec![(0, observer.guild_members(world))];
    let mut samples = vec![observer.sample_centre(world)];
    let mut litter_samples = vec![observer.sample_litter(world)];
    let mut lineage_drain = vec![observer.sample_lineage_drain(world)];
    let marker_isolation = world.params().marker_isolation;
    let mut line_samples = Vec::new();
    if marker_isolation {
        line_samples.push(observer.sample_line(world));
    }
    for _ in 0..ticks {
        world.step();
        observer.observe(world);
        let elapsed = world.tick() - start;
        if elapsed + 1 == window_start {
            guild_drain_at_window = observer.guild_drain;
        }
        if elapsed % sample_every == 0 {
            samples.push(observer.sample_centre(world));
            litter_samples.push(observer.sample_litter(world));
            lineage_drain.push(observer.sample_lineage_drain(world));
            guild_samples.push((elapsed, observer.guild_members(world)));
            if marker_isolation {
                line_samples.push(observer.sample_line(world));
            }
        }
    }
    let births: Vec<Birth> = std::mem::take(&mut observer.births_log)
        .into_iter()
        .map(|b| Birth {
            tick: b.tick - start,
            ..b
        })
        .collect();
    let guild_holds = role_guilds_from_samples(&guild_samples, &births, ticks).decomposer;
    let fewest_members = (guild_samples.iter())
        .filter(|(t, _)| *t >= window_start)
        .map(|(_, members)| members.len())
        .min()
        .unwrap_or(0);
    let mut final_roles = RoleTally::default();
    for agent in world.agents() {
        match observer.income.role(agent.id) {
            Some(TrophicRole::Producer) => final_roles.producers += 1,
            Some(TrophicRole::Decomposer) => final_roles.decomposers += 1,
            Some(TrophicRole::Consumer) => final_roles.consumers += 1,
            None => final_roles.no_role += 1,
        }
    }
    let mut report = Mode1Report {
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
        founders: FounderReadout {
            end_tick: world.tick(),
            line_alive_at_end: (world.agents().iter())
                .filter(|a| observer.line.contains(&a.id))
                .count(),
            starvation: StarvationReadout {
                founders: observer.starving.values().cloned().collect(),
            },
            ..observer.founders.clone()
        },
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
        guild: GuildReadout::default(),
        litter: LitterReadout::default(),
        recruitment: observer.recruitment(lineage_drain),
        marker: marker_isolation.then(|| observer.marker_readout(world, line_samples)),
    };
    report.litter = LitterReadout {
        second_half: LitterWindow::read(&litter_samples, &report.producer_lifespans),
        samples: litter_samples,
    };
    let producers_persist = report.producers_persist();
    report.guild = GuildReadout {
        holds: guild_holds,
        fewest_members,
        drain: observer.guild_drain.minus(&guild_drain_at_window),
        producers_persist,
        passes: guild_holds && producers_persist,
    };
    report
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

/// A living line member as a step begins: what its metabolism reads, and the
/// stores and body the recruitment readout (#833) records.
#[derive(Clone, Copy)]
struct Stores {
    reserve: f32,
    earmark: f32,
    repro_nutrient: f32,
    nutrient: f32,
    structure: f32,
    peak_structure: f32,
    wear: f32,
    upkeep: f32,
}

impl Stores {
    fn of(a: &explorers_sim::Agent, params: &WorldParameters) -> Self {
        Self {
            reserve: a.reserve,
            earmark: a.repro_reserve,
            repro_nutrient: a.repro_nutrient,
            nutrient: a.nutrient,
            structure: a.structure,
            peak_structure: a.peak_structure,
            wear: a.wear.iter().sum::<f32>() / a.wear.len() as f32,
            upkeep: explorers_sim::phase::metabolic_cost(a, params),
        }
    }
}

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
    /// Each agent's nominal traits, kept while it lives so a drain in the
    /// step it dies is still booked to its side.
    traits: HashMap<u64, TraitVector>,
    /// Carcass-drain energy since the observer began, by drainer autotrophy.
    carcass_drain: DrainSplit,
    /// The guild members' drain since the observer began.
    guild_drain: GuildDrain,
    /// Every birth since the log was last taken (`run_arm` takes it).
    births_log: Vec<Birth>,
    /// The living founders' traits, by id.
    living_founders: HashMap<u64, TraitVector>,
    /// The founders and every agent descended from one.
    line: std::collections::HashSet<u64>,
    /// Traits of the agents and carcasses standing before the step about to
    /// be read, kept while any founder lives: the targets a founder's drain
    /// can name, whose traits set the efficiency it gets.
    agent_traits: HashMap<u64, TraitVector>,
    carcass_traits: HashMap<u64, TraitVector>,
    founders: FounderReadout,
    /// Each founder through the starvation cascade (#821), by id.
    starving: std::collections::BTreeMap<u64, FounderStarvation>,
    /// Each living member of the founder line as the next step begins: its
    /// stores and body, and the upkeep that step's metabolism reads.
    stores: HashMap<u64, Stores>,
    /// Whether the recruitment readout (#833) is recorded: always, with
    /// founders, but for the test that it leaves the world alone.
    record_recruitment: bool,
    /// Each founder's life history (#833), by id.
    lives: std::collections::BTreeMap<u64, FounderLife>,
    /// The line's births (#833), in order.
    line_births: Vec<LineBirth>,
    /// Births to each living line member so far (#833).
    line_offspring: HashMap<u64, u32>,
    /// The line's deaths (#833), in order.
    line_deaths: Vec<LineDeath>,
    /// Carcass drain by lineage since the last sample (#833).
    lineage_drain: LineageDrain,
    /// Litter inflow since the last litter sample, and that sample's tick.
    litter_inflow: LitterInflow,
    litter_sampled_at: u64,
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
            traits: world.agents().iter().map(|a| (a.id, a.traits)).collect(),
            carcass_drain: DrainSplit::default(),
            guild_drain: GuildDrain::default(),
            births_log: Vec::new(),
            living_founders: HashMap::new(),
            line: std::collections::HashSet::new(),
            agent_traits: HashMap::new(),
            carcass_traits: HashMap::new(),
            founders: FounderReadout::default(),
            starving: std::collections::BTreeMap::new(),
            stores: HashMap::new(),
            record_recruitment: true,
            lives: std::collections::BTreeMap::new(),
            line_births: Vec::new(),
            line_offspring: HashMap::new(),
            line_deaths: Vec::new(),
            lineage_drain: LineageDrain::default(),
            litter_inflow: LitterInflow::default(),
            litter_sampled_at: world.tick(),
        }
        .with_founders(world)
    }

    /// Take the agents at the pure heterotroph vertex as the founders.
    fn with_founders(mut self, world: &World) -> Self {
        self.living_founders = (world.agents().iter())
            .filter(|a| a.traits.photosynthetic_absorption == 0.0 && a.traits.heterotrophy > 0.0)
            .map(|a| (a.id, a.traits))
            .collect();
        self.line = self.living_founders.keys().copied().collect();
        self.founders = FounderReadout {
            founders: self.living_founders.len(),
            start_tick: world.tick(),
            ..FounderReadout::default()
        };
        self.starving = (self.living_founders.keys())
            .map(|&id| {
                (
                    id,
                    FounderStarvation {
                        id,
                        ..FounderStarvation::default()
                    },
                )
            })
            .collect();
        self.lives = (self.living_founders.keys())
            .map(|&id| {
                (
                    id,
                    FounderLife {
                        id,
                        first_tick: world.tick(),
                        ..FounderLife::default()
                    },
                )
            })
            .collect();
        self.remember_targets(world);
        self.remember_stores(world);
        self
    }

    /// Remember each living line member's stores, body and upkeep, as the
    /// next step's metabolism will read them.
    fn remember_stores(&mut self, world: &World) {
        self.stores.clear();
        for a in world.agents() {
            if self.line.contains(&a.id) {
                (self.stores).insert(a.id, Stores::of(a, world.params()));
            }
        }
    }

    /// Mark the founders whose unallocated reserve runs out in the step just
    /// taken: those that began it with reserve no greater than its upkeep.
    fn read_running_out(&mut self, after: u64) {
        for (id, s) in &self.stores {
            let Some(f) = self.starving.get_mut(id) else {
                continue;
            };
            if f.ran_out_tick.is_none() && s.reserve <= s.upkeep {
                f.ran_out_tick = Some(after);
                f.earmark_at_running_out = Some(s.earmark);
            }
        }
    }

    /// Count the step just taken against each founder that lived through it
    /// on the cascade: on the earmark if the earmark paid some of its upkeep,
    /// on structure if burned structure did.
    fn read_cascade_ticks(&mut self, world: &World) {
        let p = world.params();
        for (id, s) in &self.stores {
            let (reserve, earmark, upkeep) = (s.reserve, s.earmark, s.upkeep);
            if reserve > upkeep || !self.living_founders.contains_key(id) {
                continue;
            }
            let f = self.starving.get_mut(id).expect("every founder is read");
            let shortfall = upkeep - reserve.max(0.0);
            let from_earmark = if p.starvation_draws_earmark {
                earmark.max(0.0)
            } else {
                0.0
            };
            if p.starvation_draws_earmark && earmark > 0.0 && shortfall > 0.0 {
                f.ticks_on_earmark += 1;
            }
            if p.starvation_shrinks && shortfall > from_earmark {
                f.ticks_on_structure += 1;
            }
        }
    }

    /// Remember the traits of what stands now, for the next step's founder
    /// drains; forget them once no founder lives.
    fn remember_targets(&mut self, world: &World) {
        self.agent_traits.clear();
        self.carcass_traits.clear();
        if self.living_founders.is_empty() {
            return;
        }
        (self.agent_traits).extend(world.agents().iter().map(|a| (a.id, a.traits)));
        (self.carcass_traits).extend(world.carcasses().iter().map(|c| (c.id, c.traits)));
    }

    /// Read the step just taken, then drop it from the log. Roles are read
    /// before the step's income is booked, as the ledger forgets the dead.
    fn observe(&mut self, world: &mut World) {
        let grid = world.nutrient_grid();
        let after = world.tick();
        // The stepper marks a senescent death with `Senesced` just before its
        // `Died`, in the same step.
        let mut senesced = std::collections::HashSet::new();
        let founders_began = !self.living_founders.is_empty();
        let (mut gained, mut drained) = (0.0_f64, 0.0_f64);
        // Each founder's own gain and drain in this step (#833).
        let mut income: HashMap<u64, (f64, f64)> = HashMap::new();
        self.read_running_out(after);
        // Agents born in this step are on the roster now; those that died in
        // it are still in the map from the step before.
        (self.traits).extend(world.agents().iter().map(|a| (a.id, a.traits)));
        // The carcasses this step's deaths left, by id, for the litter inflow.
        let died: std::collections::HashSet<u64> = (world.event_log().since(self.cursor).iter())
            .filter(|e| e.kind == EventKind::Died)
            .map(|e| e.source)
            .collect();
        let at_death: HashMap<u64, f32> = if died.is_empty() {
            HashMap::new()
        } else {
            (world.carcasses().iter())
                .filter(|c| died.contains(&c.id))
                .map(|c| (c.id, c.energy_at_death))
                .collect()
        };
        for event in world.event_log().since(self.cursor) {
            let member = (self.traits.get(&event.source))
                .is_some_and(|t| is_guild_member(self.income.role(event.source), t));
            self.guild_drain.book(event, member);
            let producer = self.income.role(event.source) == Some(TrophicRole::Producer);
            self.litter_inflow.book(event, producer, &at_death);
            let in_centre = event
                .position
                .is_some_and(|p| in_centre_patch(grid.cell_index_for(p)));
            match event.kind {
                EventKind::Born => {
                    self.births_log.extend(Birth::of(event));
                    self.birth_tick.insert(event.source, after);
                    let parents = [event.target, event.second_parent];
                    if parents.iter().flatten().any(|p| self.line.contains(p)) {
                        if self.record_recruitment {
                            self.record_line_birth(world, event, after);
                        }
                        self.line.insert(event.source);
                    }
                    for parent in parents.iter().flatten() {
                        if let Some(f) = self.starving.get_mut(parent) {
                            if f.ran_out_tick.is_none() {
                                f.reproduced_before_running_out = true;
                            }
                        }
                    }
                    let parent = event.target.and_then(|id| self.income.role(id));
                    if in_centre && parent == Some(TrophicRole::Producer) {
                        self.births += 1;
                    }
                }
                EventKind::Senesced => {
                    senesced.insert(event.source);
                }
                EventKind::Consumed if self.living_founders.contains_key(&event.source) => {
                    let consumer = &self.living_founders[&event.source];
                    let target = event.target.expect("a drain names its target");
                    let target = if event.target_was_carcass {
                        &self.carcass_traits[&target]
                    } else {
                        &self.agent_traits[&target]
                    };
                    let efficiency = explorers_sim::trophic_transfer_efficiency(
                        consumer,
                        target,
                        world.params(),
                    );
                    gained += (event.energy_delta * efficiency) as f64;
                    drained += event.energy_delta as f64;
                    let own = income.entry(event.source).or_default();
                    own.0 += (event.energy_delta * efficiency) as f64;
                    own.1 += event.energy_delta as f64;
                    if event.target_was_carcass {
                        self.book_carcass_drain(event.source, event.energy_delta);
                    }
                }
                EventKind::Consumed if event.target_was_carcass => {
                    self.book_carcass_drain(event.source, event.energy_delta);
                }
                EventKind::Died => {
                    let traits = self.traits.remove(&event.source);
                    let cause = DeathCause::of(
                        senesced.contains(&event.source),
                        (self.stores.get(&event.source)).map(|s| (s.reserve, s.upkeep)),
                    );
                    if self.record_recruitment && self.line.contains(&event.source) {
                        let born = self.birth_tick[&event.source];
                        self.line_deaths.push(LineDeath {
                            id: event.source,
                            tick: after,
                            age: after - born,
                            cause,
                            autotrophy: traits.map(|t| t.photosynthetic_absorption),
                            heterotrophy: traits.map(|t| t.heterotrophy),
                            offspring: self.line_offspring.remove(&event.source).unwrap_or(0),
                        });
                    }
                    if self.living_founders.remove(&event.source).is_some() {
                        self.founders.death_ticks.push(after);
                        if let Some(f) = self.starving.get_mut(&event.source) {
                            f.death_tick = Some(after);
                        }
                        if let Some(life) = self.lives.get_mut(&event.source) {
                            life.death_tick = Some(after);
                            life.death_cause = Some(cause);
                        }
                    }
                    self.line.remove(&event.source);
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
        if founders_began {
            self.founders.gained.push(gained);
            self.founders.drained.push(drained);
        }
        if self.record_recruitment {
            self.record_lives(world, &died, &income);
        }
        self.read_cascade_ticks(world);
        self.remember_stores(world);
        self.remember_targets(world);
        self.income.update(world.event_log());
        self.cursor = world.event_log().len();
        world.compact_event_log_before(self.cursor);
    }

    /// Add the step just taken to each founder's life history: every founder
    /// that began it standing and is still standing or died in it (a founder
    /// cleared with the centre patch leaves no `Died`, and its history ends).
    fn record_lives(
        &mut self,
        world: &World,
        died: &std::collections::HashSet<u64>,
        income: &HashMap<u64, (f64, f64)>,
    ) {
        let standing: std::collections::HashSet<u64> = (world.agents().iter())
            .filter(|a| self.lives.contains_key(&a.id))
            .map(|a| a.id)
            .collect();
        for (id, life) in &mut self.lives {
            let Some(s) = self.stores.get(id) else {
                continue;
            };
            if !standing.contains(id) && !died.contains(id) {
                continue;
            }
            life.reserve.push(s.reserve);
            life.repro_reserve.push(s.earmark);
            life.repro_nutrient.push(s.repro_nutrient);
            life.nutrient.push(s.nutrient);
            life.structure.push(s.structure);
            life.peak_structure.push(s.peak_structure);
            life.wear.push(s.wear);
            life.upkeep.push(s.upkeep);
            let (g, d) = income.get(id).copied().unwrap_or_default();
            life.gained.push(g);
            life.drained.push(d);
        }
    }

    /// Record a birth with a parent in the line, before the offspring joins
    /// it, and count it against each parent in the line.
    fn record_line_birth(&mut self, world: &World, event: &Event, after: u64) {
        let parents: Vec<LineParent> = [event.target, event.second_parent]
            .into_iter()
            .flatten()
            .map(|id| {
                let traits = self.traits.get(&id);
                LineParent {
                    id,
                    in_line: self.line.contains(&id),
                    autotrophy: traits.map(|t| t.photosynthetic_absorption),
                    heterotrophy: traits.map(|t| t.heterotrophy),
                }
            })
            .collect();
        for p in parents.iter().filter(|p| p.in_line) {
            *self.line_offspring.entry(p.id).or_default() += 1;
        }
        let child = (world.agents().iter()).find(|a| a.id == event.source);
        self.line_births.push(LineBirth {
            tick: after,
            offspring: event.source,
            sexual: event.second_parent.is_some(),
            parents,
            autotrophy: child.map(|a| a.traits.photosynthetic_absorption),
            heterotrophy: child.map(|a| a.traits.heterotrophy),
            nutrient_endowment: child.map(|a| a.nutrient_total(world.params())),
        });
    }

    /// Take the carcass drain by lineage since the last sample, closed at
    /// the world's tick now.
    fn sample_lineage_drain(&mut self, world: &World) -> LineageDrain {
        LineageDrain {
            tick: world.tick(),
            ..std::mem::take(&mut self.lineage_drain)
        }
    }

    /// The recruitment readout so far (#833), with the run's or arm's
    /// lineage drain samples: `None` without founders.
    fn recruitment(&self, carcass_drain: Vec<LineageDrain>) -> Option<RecruitmentReadout> {
        (self.record_recruitment && self.founders.founders > 0).then(|| RecruitmentReadout {
            founders: self.lives.values().cloned().collect(),
            births: self.line_births.clone(),
            deaths: self.line_deaths.clone(),
            carcass_drain,
        })
    }

    /// Book a carcass drain to the drainer's side of [`MIXOTROPH_AUTOTROPHY`].
    /// With the recruitment readout, book it by lineage too (#833).
    fn book_carcass_drain(&mut self, drainer: u64, drained: f32) {
        let drained = drained as f64;
        let lineage = if self.line.contains(&drainer) {
            &mut self.lineage_drain.founder_line
        } else {
            &mut self.lineage_drain.other
        };
        if self.traits[&drainer].photosynthetic_absorption < MIXOTROPH_AUTOTROPHY {
            self.carcass_drain.pure_vertex += drained;
            if self.record_recruitment {
                lineage.pure_vertex += drained;
            }
        } else {
            self.carcass_drain.mixotroph += drained;
            if self.record_recruitment {
                lineage.mixotroph += drained;
            }
        }
    }

    /// The founder line's living members now (#837), and those below
    /// autotrophy 0.1.
    fn sample_line(&self, world: &World) -> LineSample {
        let line: Vec<_> = (world.agents().iter())
            .filter(|a| self.line.contains(&a.id))
            .collect();
        let below = (line.iter())
            .filter(|a| a.traits.photosynthetic_absorption < MIXOTROPH_AUTOTROPHY)
            .count();
        LineSample {
            tick: world.tick(),
            line_living: line.len(),
            below_autotrophy: below,
            share_below_autotrophy: (!line.is_empty()).then(|| below as f64 / line.len() as f64),
        }
    }

    /// The marker readout (#837) at the end of a run or arm, with its line
    /// samples.
    fn marker_readout(&self, world: &World, samples: Vec<LineSample>) -> MarkerReadout {
        let s = world.params().compatibility_scale;
        let line: Vec<_> = (world.agents().iter())
            .filter(|a| self.line.contains(&a.id))
            .collect();
        let producers: Vec<_> = (world.agents().iter())
            .filter(|a| self.income.role(a.id) == Some(TrophicRole::Producer))
            .collect();
        MarkerReadout {
            samples,
            line: PairwiseCompatibility::of(&line, s),
            producers: PairwiseCompatibility::of(&producers, s),
        }
    }

    /// The living guild members, read into the decomposer role for the
    /// committed guild predicate.
    fn guild_members(&self, world: &World) -> HashMap<u64, TrophicRole> {
        (world.agents().iter())
            .filter(|a| is_guild_member(self.income.role(a.id), &a.traits))
            .map(|a| (a.id, TrophicRole::Decomposer))
            .collect()
    }

    /// Sample the whole world's litter, taking the inflow since the last
    /// litter sample.
    fn sample_litter(&mut self, world: &World) -> LitterSample {
        let tick = world.tick();
        let established_structures: Vec<f32> = (world.agents().iter())
            .filter(|a| self.income.role(a.id) == Some(TrophicRole::Producer))
            .filter(|a| tick - self.birth_tick[&a.id] > ESTABLISHMENT_AGE)
            .map(|a| a.structure)
            .collect();
        let carcasses = world.carcasses();
        LitterSample {
            tick,
            standing_per_cell: carcasses.iter().map(|c| c.energy as f64).sum::<f64>()
                / (SIDE_CELLS * SIDE_CELLS) as f64,
            carcasses: carcasses.len(),
            nonempty_carcasses: carcasses.iter().filter(|c| c.energy > 0.0).count(),
            inflow: std::mem::take(&mut self.litter_inflow),
            inflow_ticks: tick - std::mem::replace(&mut self.litter_sampled_at, tick),
            established_structures,
        }
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

/// The ticks at which the founder readout counts founders alive (#781).
pub const FOUNDER_CHECKPOINTS: [u64; 3] = [20, 50, 100];
/// The ticks over which the founder readout reads mean income (#779's
/// pre-registered window): the steps ending at ticks 5 through 20.
pub const FOUNDER_INCOME_WINDOW: (u64, u64) = (5, 20);

/// The header of the founder readout's table (#781): one row per arm, from
/// [`founder_row`].
const FOUNDER_HEADER: &str = "| founders | alive at 20 | alive at 50 | alive at 100 | alive at end | line alive at end | mean income, ticks 5–20, per living founder-tick: gained (drained) | gained per founder-tick, the dead at 0 |";

/// One arm's founder readout. A checkpoint past the arm's end reads `-`.
fn founder_row(f: &FounderReadout) -> String {
    let checkpoints: Vec<String> = FOUNDER_CHECKPOINTS
        .iter()
        .map(|&t| {
            if t <= f.end_tick {
                f.alive_at(t).to_string()
            } else {
                "-".to_string()
            }
        })
        .collect();
    let (from, to) = FOUNDER_INCOME_WINDOW;
    let drained = FounderReadout {
        gained: f.drained.clone(),
        ..f.clone()
    };
    let income = match (f.mean_income(from, to), drained.mean_income(from, to)) {
        (Some(g), Some(d)) => format!("{g:.3} ({d:.3})"),
        _ => "-".to_string(),
    };
    let per_founder = f
        .mean_income_per_founder(from, to)
        .map_or_else(|| "-".to_string(), |g| format!("{g:.3}"));
    format!(
        "| {} | {} | {} | {} | {income} | {per_founder} |",
        f.founders,
        checkpoints.join(" | "),
        f.alive_at(f.end_tick),
        f.line_alive_at_end
    )
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

/// The header of the guild reading's table (#794): one row per arm, from
/// [`guild_row`].
const GUILD_HEADER: &str = "| guild holds | fewest members | guild share of carcass drain | guild drain: living / carcass (living share) | producers persist | pass |";

/// One arm's guild reading over its second half.
fn guild_row(g: &GuildReadout) -> String {
    let yes = |b: bool| if b { "yes" } else { "no" };
    let share = |s: Option<f64>| s.map_or_else(|| "-".to_string(), |s| format!("{s:.3}"));
    format!(
        "| {} | {} | {} | {:.1} / {:.1} ({}) | {} | {} |",
        yes(g.holds),
        g.fewest_members,
        share(g.drain.carcass_share()),
        g.drain.living,
        g.drain.carcass,
        share(g.drain.living_share()),
        yes(g.producers_persist),
        yes(g.passes)
    )
}

/// The title of the guild reading's section.
fn guild_title() -> String {
    format!(
        "## Heterotroph-dominant decomposer guild, second half (decomposer role, heterotrophy > autotrophy; at least {GUILD_MIN_SIZE} on every sample and a member's birth)"
    )
}

/// The title of the litter reading's section (#804).
const LITTER_TITLE: &str =
    "## Litter supply, second half (whole world, carcass structure per nutrient cell)";

/// The header of the litter reading's table: one row per arm, from
/// [`litter_row`].
const LITTER_HEADER: &str = "| ticks | standing litter per cell: mean (min–max) | inflow per cell per tick: mean (min–max) | from producer deaths | from other deaths | carcasses: all / holding structure (means) |";

/// One arm's litter reading over its second half.
fn litter_row(w: &LitterWindow) -> String {
    let s = |s: &Spread, digits: usize| {
        format!(
            "{:.digits$} ({:.digits$}–{:.digits$})",
            s.mean + 0.0,
            s.min + 0.0,
            s.max + 0.0
        )
    };
    format!(
        "| {}–{} | {} | {} | {} | {} | {:.0} / {:.0} |",
        w.from_tick,
        w.to_tick,
        s(&w.standing_per_cell, 2),
        s(&w.inflow, 4),
        s(&w.producer_inflow, 4),
        s(&w.other_inflow, 4),
        w.carcasses.mean,
        w.nonempty_carcasses.mean
    )
}

/// The title of the established producers' section (#804).
fn established_title() -> String {
    format!(
        "## Established producers, second half (producers alive older than {ESTABLISHMENT_AGE} ticks, structure pooled over the samples: S_p)"
    )
}

/// The header of the established producers' table: one row per arm, from
/// [`established_row`].
const ESTABLISHED_HEADER: &str = "| producer-samples | structure min | p10 | p25 | median | p75 | p90 | max | mean | established lifespan median |";

/// One arm's established producers over its second half.
fn established_row(w: &LitterWindow) -> String {
    let d = &w.established_structure;
    format!(
        "| {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {} |",
        d.count,
        d.min,
        d.p10,
        d.p25,
        d.median,
        d.p75,
        d.p90,
        d.max,
        d.mean,
        w.established_lifespan_median
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
    let _ = writeln!(
        out,
        "{}\n\n{GUILD_HEADER}\n|---|---:|---:|---:|---|---|\n{}\n",
        guild_title(),
        guild_row(&report.guild)
    );
    let _ = writeln!(
        out,
        "## Founders (pure heterotroph vertex at founding, by id)\n\n{FOUNDER_HEADER}\n|---:|---:|---:|---:|---:|---:|---:|---:|\n{}\n",
        founder_row(&report.founders)
    );
    let w = &report.litter.second_half;
    let _ = writeln!(
        out,
        "{LITTER_TITLE}\n\n{LITTER_HEADER}\n|---|---:|---:|---:|---:|---:|\n{}\n\n{}\n\n{ESTABLISHED_HEADER}\n|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n{}\n",
        litter_row(w),
        established_title(),
        established_row(w)
    );
    let second_half = &report.samples[report.samples.len() / 2..];
    let min_late = second_half.iter().map(|s| s.producers).min().unwrap_or(0);
    let last = report.samples.last().map_or(0, |s| s.producers);
    let _ = writeln!(
        out,
        "## Centre-patch producers\n\n\
         at the end {last} | fewest over the second half {min_late} | persisted: {}\n",
        if report.producers_persist() {
            "yes"
        } else {
            "no"
        }
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
        "\n{}\n\n| arm {GUILD_HEADER}\n|---|---|---:|---:|---:|---|---|",
        guild_title()
    );
    for (name, arm) in [
        ("perturbed", &report.perturbed),
        ("control", &report.control),
    ] {
        let _ = writeln!(out, "| {name} {}", guild_row(&arm.guild));
    }
    let _ = writeln!(
        out,
        "\n## Founders (pure heterotroph vertex at founding, by id)\n\n| arm {FOUNDER_HEADER}\n|---|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    for (name, arm) in [
        ("perturbed", &report.perturbed),
        ("control", &report.control),
    ] {
        let _ = writeln!(out, "| {name} {}", founder_row(&arm.founders));
    }
    let arms = [
        ("perturbed", &report.perturbed),
        ("control", &report.control),
    ];
    let _ = writeln!(
        out,
        "\n{LITTER_TITLE}\n\n| arm {LITTER_HEADER}\n|---|---|---:|---:|---:|---:|---:|"
    );
    for (name, arm) in arms {
        let _ = writeln!(out, "| {name} {}", litter_row(&arm.litter.second_half));
    }
    let _ = writeln!(
        out,
        "\n{}\n\n| arm {ESTABLISHED_HEADER}\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
        established_title()
    );
    for (name, arm) in arms {
        let _ = writeln!(out, "| {name} {}", established_row(&arm.litter.second_half));
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

    /// The diagnostic litter multiplier (#781) stands `N` times the settled
    /// stock in every cell, as `N` times the carcasses of the same size, on
    /// the same founders in the same places, with total nutrient at founding
    /// still the pool.
    #[test]
    fn a_litter_multiplier_scales_every_cells_carcass_count_energy_and_nutrient() {
        let spec = Mode1Spec::committed();
        let x = 10;
        let base = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            7,
            Community::ProducersAndDecomposers,
        );
        let world = mode1_mesocosm_with_litter(
            &spec,
            mesocosm_wear(),
            7,
            Community::ProducersAndDecomposers,
            x,
        );
        let grid = world.nutrient_grid();
        for cell in 0..grid.cells().len() {
            let standing: Vec<_> = world
                .carcasses()
                .iter()
                .filter(|c| grid.cell_index_for(c.position) == cell)
                .collect();
            assert_eq!(standing.len(), x * CARCASSES_PER_CELL);
            let energy: f32 = standing.iter().map(|c| c.energy).sum();
            let nutrient: f32 = standing.iter().map(|c| c.nutrient).sum();
            assert!((energy - x as f32 * CARCASS_ENERGY_PER_CELL).abs() < 1e-2);
            assert!((nutrient - x as f32 * CARCASS_NUTRIENT_PER_CELL).abs() < 1e-2);
        }
        let placed = |w: &World| -> Vec<((f32, f32), TraitVector)> {
            w.agents().iter().map(|a| (a.position, a.traits)).collect()
        };
        assert_eq!(placed(&world), placed(&base));
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
    }

    /// At 1× the multiplier is the committed mesocosm, carcass for carcass:
    /// the default run is unchanged.
    #[test]
    fn a_litter_multiplier_of_one_is_the_committed_mesocosm() {
        let spec = Mode1Spec::committed();
        for community in [Community::ProducersAndDecomposers, Community::ProducersOnly] {
            let base = mode1_mesocosm(&spec, mesocosm_wear(), 7, community);
            let one = mode1_mesocosm_with_litter(&spec, mesocosm_wear(), 7, community, 1);
            let carcasses = |w: &World| -> Vec<_> {
                (w.carcasses().iter())
                    .map(|c| (c.id, c.position, c.energy, c.nutrient, c.traits))
                    .collect()
            };
            let agents = |w: &World| -> Vec<_> {
                (w.agents().iter())
                    .map(|a| (a.id, a.position, a.reserve, a.structure, a.traits))
                    .collect()
            };
            assert_eq!(carcasses(&one), carcasses(&base));
            assert_eq!(agents(&one), agents(&base));
            assert_eq!(one.nutrient_grid().cells(), base.nutrient_grid().cells());
        }
    }

    /// The agents at the pure heterotroph vertex at founding: the decomposer
    /// founders.
    fn decomposer_founder_count(world: &World) -> usize {
        (world.agents().iter())
            .filter(|a| a.traits.photosynthetic_absorption == 0.0 && a.traits.heterotrophy > 0.0)
            .count()
    }

    /// The decomposer founders' count (#829) seeds that many decomposer
    /// founders, below and above one a cell, beside the same producers.
    #[test]
    fn a_decomposer_founder_count_seeds_that_many_decomposer_founders() {
        let spec = Mode1Spec::committed();
        for n in [1, 5, 10, 20, 81, 200] {
            let world = mode1_mesocosm_with_founders(
                &spec,
                mesocosm_wear(),
                7,
                Community::ProducersAndDecomposers,
                1,
                Some(n),
            );
            assert_eq!(decomposer_founder_count(&world), n, "n {n}");
            assert_eq!(
                world.agents().len(),
                PRODUCERS_PER_CELL * SIDE_CELLS * SIDE_CELLS + n
            );
        }
    }

    fn carcass_state(w: &World) -> Vec<(u64, (f32, f32), f32, f32, TraitVector)> {
        (w.carcasses().iter())
            .map(|c| (c.id, c.position, c.energy, c.nutrient, c.traits))
            .collect()
    }

    fn agent_state(w: &World) -> Vec<(u64, (f32, f32), f32, f32, TraitVector)> {
        (w.agents().iter())
            .map(|a| (a.id, a.position, a.reserve, a.structure, a.traits))
            .collect()
    }

    /// Unset, the decomposer founders' count (#829) is one a cell: the world
    /// is the committed mesocosm's, state for state, for the same seed, in
    /// both communities and at any litter multiplier. A count given to the
    /// producers-only community seeds no decomposers.
    #[test]
    fn an_unset_decomposer_founder_count_is_the_committed_mesocosm() {
        let spec = Mode1Spec::committed();
        for community in [Community::ProducersAndDecomposers, Community::ProducersOnly] {
            for x in [1, 3] {
                let base = mode1_mesocosm_with_litter(&spec, mesocosm_wear(), 7, community, x);
                let unset =
                    mode1_mesocosm_with_founders(&spec, mesocosm_wear(), 7, community, x, None);
                let one_a_cell = mode1_mesocosm_with_founders(
                    &spec,
                    mesocosm_wear(),
                    7,
                    community,
                    x,
                    Some(DECOMPOSERS_PER_CELL * SIDE_CELLS * SIDE_CELLS),
                );
                for w in [&unset, &one_a_cell] {
                    assert_eq!(carcass_state(w), carcass_state(&base));
                    assert_eq!(agent_state(w), agent_state(&base));
                    assert_eq!(w.nutrient_grid().cells(), base.nutrient_grid().cells());
                    assert_eq!(w.params(), base.params());
                }
            }
        }
        let bare = mode1_mesocosm_with_founders(
            &spec,
            mesocosm_wear(),
            7,
            Community::ProducersOnly,
            1,
            Some(10),
        );
        assert_eq!(decomposer_founder_count(&bare), 0);
    }

    /// The decomposer founders' count (#829) moves neither the producers nor
    /// the founding litter: the litter is placed before the founders and the
    /// producers come first in the roster, so both stand as at one a cell,
    /// state for state. Total nutrient at founding is still the pool.
    #[test]
    fn the_decomposer_founder_count_moves_neither_producers_nor_litter() {
        let spec = Mode1Spec::committed();
        let base = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            7,
            Community::ProducersAndDecomposers,
        );
        let producers = |w: &World| -> Vec<_> {
            let mut p = agent_state(w);
            p.retain(|a| a.4.photosynthetic_absorption > 0.0);
            p
        };
        assert_eq!(
            producers(&base).len(),
            PRODUCERS_PER_CELL * SIDE_CELLS * SIDE_CELLS
        );
        for n in [5, 10, 20, 200] {
            let world = mode1_mesocosm_with_founders(
                &spec,
                mesocosm_wear(),
                7,
                Community::ProducersAndDecomposers,
                1,
                Some(n),
            );
            assert_eq!(producers(&world), producers(&base), "n {n}");
            assert_eq!(carcass_state(&world), carcass_state(&base), "n {n}");
            let params = world.params().clone();
            let total = world.nutrient_pool()
                + (world.agents().iter())
                    .map(|a| a.nutrient_total(&params))
                    .sum::<f32>()
                + world.carcasses().iter().map(|c| c.nutrient).sum::<f32>();
            let pool = params.initial_nutrient_pool;
            assert!(
                (total - pool).abs() <= 1e-4 * pool,
                "n {n}: {total} vs {pool}"
            );
        }
    }

    /// `--body-reach-pieces` (#795) reads h and S as the decomposer founders'
    /// means and ρ from the litter, none of which the founders' count (#829)
    /// changes: identical founders give their own h and S back at any count,
    /// and the founding litter is unmoved. So `k` converts to the same
    /// coefficient and founder reach at every count.
    #[test]
    fn the_body_reach_conversion_does_not_depend_on_the_founder_count() {
        let spec = Mode1Spec::committed();
        let build = |n: Option<usize>| {
            mode1_mesocosm_with_founders(
                &spec,
                mesocosm_wear(),
                1,
                Community::ProducersAndDecomposers,
                1,
                n,
            )
        };
        let base = set_body_reach_pieces(&mut build(None), 137.7).unwrap();
        for n in [5, 10, 20] {
            let mut world = build(Some(n));
            let reach = set_body_reach_pieces(&mut world, 1_377.0 / n as f32).unwrap();
            assert_eq!(
                reach.founder_heterotrophy, base.founder_heterotrophy,
                "n {n}"
            );
            assert_eq!(reach.founder_structure, base.founder_structure, "n {n}");
            assert_eq!(reach.litter_density, base.litter_density, "n {n}");
            if n == 10 {
                assert_eq!(reach, base);
            }
        }
    }

    /// The founder readout (#781) takes the founders the world was built
    /// with, at any count (#829): it counts and reads income over those, not
    /// one a cell.
    #[test]
    fn the_founder_readout_counts_the_founders_the_count_seeded() {
        let spec = Mode1Spec::committed();
        let n = 10;
        let mut world = mode1_mesocosm_with_founders(
            &spec,
            mesocosm_wear(),
            4,
            Community::ProducersAndDecomposers,
            1,
            Some(n),
        );
        let report = run_mode1(&mut world, 20, 10);
        let f = &report.founders;
        assert_eq!(f.founders, n);
        assert_eq!(f.alive_at(0), n);
        assert_eq!(f.starvation.founders.len(), n);
        let (from, to) = FOUNDER_INCOME_WINDOW;
        let per_founder = f.mean_income_per_founder(from, to).unwrap();
        // `gained` stops once every founder is dead.
        let gained: f64 = (from - 1..to)
            .filter_map(|i| f.gained.get(i as usize))
            .sum();
        let steps = (to + 1 - from) as f64;
        assert!((per_founder - gained / (n as f64 * steps)).abs() < 1e-12);
        let summary = mode1_summary("n founders", &report);
        assert!(summary.contains(&format!("| {n} | ")), "{summary}");
    }

    #[test]
    #[should_panic(expected = "a decomposer founder count is at least 1")]
    fn a_decomposer_founder_count_of_zero_is_refused() {
        mode1_mesocosm_with_founders(
            &Mode1Spec::committed(),
            mesocosm_wear(),
            7,
            Community::ProducersAndDecomposers,
            1,
            Some(0),
        );
    }

    /// The founders are the agents at the pure heterotroph vertex at
    /// founding, tracked by id (#781): the readout counts those ids alive at
    /// every tick, read independently off the roster of the same run.
    #[test]
    fn the_founder_readout_counts_the_founding_decomposers_alive_by_id() {
        let spec = Mode1Spec::committed();
        let build = || {
            mode1_mesocosm(
                &spec,
                mesocosm_wear(),
                4,
                Community::ProducersAndDecomposers,
            )
        };
        let ticks = 30;
        let report = run_mode1(&mut build(), ticks, 10);

        let mut world = build();
        let founders: std::collections::HashSet<u64> = world
            .agents()
            .iter()
            .filter(|a| a.traits.photosynthetic_absorption == 0.0 && a.traits.heterotrophy > 0.0)
            .map(|a| a.id)
            .collect();
        assert_eq!(
            founders.len(),
            DECOMPOSERS_PER_CELL * SIDE_CELLS * SIDE_CELLS
        );
        let f = &report.founders;
        assert_eq!(f.founders, founders.len());
        assert_eq!(f.alive_at(0), founders.len());
        let mut fell = false;
        for _ in 0..ticks {
            world.step();
            let alive = world
                .agents()
                .iter()
                .filter(|a| founders.contains(&a.id))
                .count();
            assert_eq!(f.alive_at(world.tick()), alive, "tick {}", world.tick());
            fell |= alive < founders.len();
        }
        assert!(fell, "some founder dies within the run");

        let bare = run_mode1(
            &mut mode1_mesocosm(&spec, mesocosm_wear(), 4, Community::ProducersOnly),
            5,
            5,
        );
        assert_eq!(bare.founders.founders, 0);
    }

    /// Founder income is energy *gained*: each drain's structure times the
    /// trophic efficiency the stepper applied to it, read independently off a
    /// full event log. A `Consumed` event's `energy_delta` is the structure
    /// drained, before efficiency, so reading it as income would overstate
    /// income about threefold (764-founder-death-diagnosis.md, *A reading
    /// trap*): the readout keeps the two apart.
    #[test]
    fn founder_income_is_drained_structure_times_the_trophic_efficiency_applied() {
        let spec = Mode1Spec::committed();
        let build = || {
            mode1_mesocosm(
                &spec,
                mesocosm_wear(),
                4,
                Community::ProducersAndDecomposers,
            )
        };
        let ticks = 20;
        let report = run_mode1(&mut build(), ticks, 10);

        let mut world = build();
        let founders: HashMap<u64, TraitVector> = world
            .agents()
            .iter()
            .filter(|a| a.traits.photosynthetic_absorption == 0.0)
            .map(|a| (a.id, a.traits))
            .collect();
        let (mut gained, mut drained) = (vec![], vec![]);
        for _ in 0..ticks {
            let agents: HashMap<u64, TraitVector> =
                world.agents().iter().map(|a| (a.id, a.traits)).collect();
            let carcasses: HashMap<u64, TraitVector> =
                world.carcasses().iter().map(|c| (c.id, c.traits)).collect();
            let cursor = world.event_log().len();
            world.step();
            let (mut g, mut d) = (0.0_f64, 0.0_f64);
            for e in world.event_log().since(cursor) {
                let Some(consumer) = founders.get(&e.source) else {
                    continue;
                };
                if e.kind != EventKind::Consumed {
                    continue;
                }
                let target = e.target.unwrap();
                let target = if e.target_was_carcass {
                    carcasses[&target]
                } else {
                    agents[&target]
                };
                let eff =
                    explorers_sim::trophic_transfer_efficiency(consumer, &target, world.params());
                g += (e.energy_delta * eff) as f64;
                d += e.energy_delta as f64;
            }
            gained.push(g);
            drained.push(d);
        }
        let f = &report.founders;
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
        let (sum_g, sum_d): (f64, f64) = (gained.iter().sum(), drained.iter().sum());
        assert!(sum_d > 0.0);
        assert!(
            close(f.gained.iter().sum(), sum_g),
            "{:?} vs {gained:?}",
            f.gained
        );
        assert!(close(f.drained.iter().sum(), sum_d));
        // The founders drain producer litter, which they keep about 0.31 of.
        assert!(
            sum_g < 0.5 * sum_d,
            "income is not the drain: {sum_g} vs {sum_d}"
        );

        // Mean income over ticks 5–20 per living founder-tick: the gain in the
        // steps ending at ticks 5 through 20 over the founders alive as each
        // step began.
        let window_gain: f64 = gained[4..20].iter().sum();
        let founder_ticks: usize = (5..=20).map(|t| f.alive_at(t - 1)).sum();
        let mean = f.mean_income(5, 20).unwrap();
        assert!(close(mean, window_gain / founder_ticks as f64), "{mean}");
        let per_founder = f.mean_income_per_founder(5, 20).unwrap();
        assert!(close(
            per_founder,
            window_gain / (16.0 * founders.len() as f64)
        ));
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
            litter: LitterReadout::default(),
            founders: FounderReadout::default(),
            guild: GuildReadout::default(),
            recruitment: None,
            marker: None,
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
            litter: LitterReadout::default(),
            founders: FounderReadout::default(),
            guild: GuildReadout::default(),
            recruitment: None,
            marker: None,
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

    /// Both summaries carry the founder readout (#781) per arm: founders
    /// alive at ticks 20, 50 and 100 and at the end, the line alive at the
    /// end, and mean income over ticks 5–20 as energy gained, with the
    /// structure drained beside it in brackets.
    #[test]
    fn the_summaries_report_the_founder_readout() {
        // 4 founders; one dies in the step ending at tick 10, one at 60.
        // Each living founder gains 0.1 a tick and drains 0.3.
        let mut gained = vec![];
        let mut drained = vec![];
        for t in 1..=200u64 {
            let alive = 4 - (t > 10) as usize - (t > 60) as usize;
            gained.push(0.1 * alive as f64);
            drained.push(0.3 * alive as f64);
        }
        let founders = FounderReadout {
            founders: 4,
            start_tick: 0,
            end_tick: 200,
            death_ticks: vec![10, 60],
            gained,
            drained,
            line_alive_at_end: 5,
            starvation: StarvationReadout::default(),
        };
        // Ticks 5–20: alive at the start of each step 4 (steps 5–10) then 3.
        let mean = founders.mean_income(5, 20).unwrap();
        assert!((mean - 0.1).abs() < 1e-12);
        let mut report = Mode1Report {
            samples: vec![],
            producer_lifespans: vec![],
            senescent_lifespans: vec![],
            final_roles: RoleTally::default(),
            heterotroph_dominant: 0,
            carcass_drain: DrainSplit::default(),
            arm_carcass_drain: DrainSplit::default(),
            ledger: Ledger::default(),
            litter: LitterReadout::default(),
            founders,
            guild: GuildReadout::default(),
            recruitment: None,
            marker: None,
        };
        let line = "| 4 | 3 | 3 | 2 | 2 | 5 | 0.100 (0.300) | 0.084 |";
        let summary = mode1_summary("h", &report);
        assert!(summary.contains(line), "{summary}");

        let control = report.clone();
        report.founders.end_tick = 40;
        let paired = PairedReport {
            fork_tick: 0,
            clearance: Clearance::default(),
            perturbed: report,
            control,
        };
        let summary = paired_summary("h", &paired);
        assert!(summary.contains(&format!("| control {line}")), "{summary}");
        // Checkpoints past an arm's end are not read.
        assert!(
            summary.contains("| perturbed | 4 | 3 | - | - | 3 | 5 | 0.100 (0.300) | 0.084 |"),
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

    fn traits(autotrophy: f32, heterotrophy: f32) -> TraitVector {
        TraitVector {
            photosynthetic_absorption: autotrophy,
            heterotrophy,
            ..Mode1Spec::committed().decomposer
        }
    }

    /// The guild's members are agents the income read calls decomposers whose
    /// heterotrophy exceeds their autotrophy (reference-modes.md, mode 1,
    /// *Community*). A producer-line mixotroph living on carcasses carries
    /// the decomposer role and is out.
    #[test]
    fn a_guild_member_is_a_decomposer_whose_heterotrophy_exceeds_its_autotrophy() {
        let decomposer = Some(TrophicRole::Decomposer);
        assert!(is_guild_member(decomposer, &traits(0.0, 1.07)));
        assert!(is_guild_member(decomposer, &traits(0.15, 0.9)));
        // A producer-line mixotroph draining carcasses.
        assert!(!is_guild_member(decomposer, &traits(0.9, 0.2)));
        assert!(!is_guild_member(decomposer, &traits(0.5, 0.5)));
        // Heterotroph-dominant by investment, but not living on carcasses.
        assert!(!is_guild_member(
            Some(TrophicRole::Producer),
            &traits(0.0, 1.0)
        ));
        assert!(!is_guild_member(
            Some(TrophicRole::Consumer),
            &traits(0.0, 1.0)
        ));
        assert!(!is_guild_member(None, &traits(0.0, 1.0)));
    }

    /// The guild's drain splits between living targets and carcasses, and its
    /// carcass drain is read as a share of all the carcass drain.
    #[test]
    fn the_guild_drain_splits_living_targets_from_carcasses() {
        use explorers_sim::event::{Event, EventLog};
        let event = |seq: u64, kind: EventKind, source: u64, carcass: bool, drained: f32| Event {
            tick: 0,
            seq,
            kind,
            source,
            target: Some(100 + seq),
            energy_delta: drained,
            position: None,
            target_was_carcass: carcass,
            second_parent: None,
            nutrient_delta: 0.0,
        };
        let mut log = EventLog::new();
        for e in [
            event(0, EventKind::Consumed, 1, true, 5.0),
            event(1, EventKind::Consumed, 2, false, 2.0),
            event(2, EventKind::Consumed, 1, false, 1.0),
            event(3, EventKind::Consumed, 3, true, 3.0),
            event(4, EventKind::Consumed, 3, false, 7.0),
            event(5, EventKind::Photosynthesized, 1, false, 4.0),
        ] {
            log.append(e).unwrap();
        }
        let members = [1, 2];
        let mut drain = GuildDrain::default();
        for e in log.since(0) {
            drain.book(e, members.contains(&e.source));
        }
        assert_eq!(
            drain,
            GuildDrain {
                living: 3.0,
                carcass: 5.0,
                all_carcass: 8.0,
            }
        );
        assert_eq!(drain.carcass_share(), Some(5.0 / 8.0));
        assert_eq!(drain.living_share(), Some(3.0 / 8.0));
        assert_eq!(GuildDrain::default().carcass_share(), None);
        assert_eq!(GuildDrain::default().living_share(), None);
    }

    /// A run reads the guild over its second half: the verdict is the
    /// committed predicate over its members, the drain is booked over the
    /// same window, and the pass needs the producers to persist too.
    #[test]
    fn a_run_reads_the_guild_over_its_second_half() {
        let mut world = mode1_mesocosm(
            &Mode1Spec::committed(),
            mesocosm_wear(),
            4,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, 60, 10);
        let g = &report.guild;
        assert_eq!(g.producers_persist, report.producers_persist());
        assert_eq!(g.passes, g.holds && g.producers_persist);
        if g.fewest_members < explorers_genesis_eval::guild::GUILD_MIN_SIZE {
            assert!(!g.holds);
        }
        assert!(g.drain.carcass <= g.drain.all_carcass);
        assert!(g.drain.all_carcass > 0.0);
        assert!(g.drain.all_carcass <= report.arm_carcass_drain.total() + 1e-9);
    }

    /// Both summaries carry the guild reading per arm: the verdict, its
    /// fewest members, its share of the carcass drain, its living / carcass
    /// drain split, whether the producers persist, and the combined pass.
    #[test]
    fn the_summaries_report_the_guild_reading() {
        let mut report = Mode1Report {
            samples: vec![],
            producer_lifespans: vec![],
            senescent_lifespans: vec![],
            final_roles: RoleTally::default(),
            heterotroph_dominant: 0,
            carcass_drain: DrainSplit::default(),
            arm_carcass_drain: DrainSplit::default(),
            ledger: Ledger::default(),
            litter: LitterReadout::default(),
            founders: FounderReadout::default(),
            guild: GuildReadout {
                holds: true,
                fewest_members: 6,
                drain: GuildDrain {
                    living: 1.0,
                    carcass: 3.0,
                    all_carcass: 12.0,
                },
                producers_persist: true,
                passes: true,
            },
            recruitment: None,
            marker: None,
        };
        let line = "| yes | 6 | 0.250 | 1.0 / 3.0 (0.250) | yes | yes |";
        let summary = mode1_summary("h", &report);
        assert!(summary.contains(GUILD_HEADER), "{summary}");
        assert!(summary.contains(line), "{summary}");

        let control = report.clone();
        report.guild = GuildReadout::default();
        let paired = PairedReport {
            fork_tick: 0,
            clearance: Clearance::default(),
            perturbed: report,
            control,
        };
        let summary = paired_summary("h", &paired);
        assert!(summary.contains(&format!("| control {line}")), "{summary}");
        assert!(
            summary.contains("| perturbed | no | 0 | - | 0.0 / 0.0 (-) | no | no |"),
            "{summary}"
        );
    }

    /// The conversion (#795): at the coefficient it gives, a reach disc of
    /// `h · (c + b · √S)` holds k carcasses at density ρ; with `r_k ≤ h · c`
    /// it clamps to 0, the world's contact-only reach.
    #[test]
    fn body_reach_coefficient_spans_k_pieces_and_clamps_at_contact_range() {
        let mut params = Mode1Spec::committed().parameters;
        let (h, s, rho) = (1.07, 9.0, 0.17);
        let c = params.contact_range_coefficient;
        for k in [3.0, 10.0, 30.0] {
            let (b, clamped) = body_reach_coefficient_for_pieces(k, h, s, rho, c);
            assert!(!clamped && b > 0.0, "k {k}: b {b}");
            params.body_reach_coefficient = b;
            let r = consumption_reach(h, s, &params);
            let held = std::f32::consts::PI * r * r * rho;
            assert!((held - k).abs() < 1e-3 * k, "k {k}: the disc holds {held}");
        }
        // h · c = 1.07 · 1.28 ≈ 1.37 spans π · 1.37² · 0.17 ≈ 1.0 pieces.
        let at_contact = std::f32::consts::PI * (h * c).powi(2) * rho;
        for k in [0.5, at_contact] {
            assert_eq!(
                body_reach_coefficient_for_pieces(k, h, s, rho, c),
                (0.0, true),
                "k {k}"
            );
        }
    }

    /// `set_body_reach_pieces` reads h and S from the decomposer founders the
    /// world built (effective heterotrophy, co-limited structure) and ρ from
    /// its litter, so ρ follows `--litter-x`, and sets the world's coefficient.
    #[test]
    fn set_body_reach_pieces_reads_the_built_founders_and_litter() {
        let spec = Mode1Spec::committed();
        for x in [1, 10] {
            let mut world = mode1_mesocosm_with_litter(
                &spec,
                mesocosm_wear(),
                1,
                Community::ProducersAndDecomposers,
                x,
            );
            let founder = world
                .agents()
                .iter()
                .find(|a| a.traits.photosynthetic_absorption == 0.0)
                .unwrap()
                .clone();
            let k_steep = world.params().wear_degradation_steepness;
            // 30 pieces, since at 10x litter about 10 is the contact-only reach.
            let reach = set_body_reach_pieces(&mut world, 30.0).unwrap();
            assert_eq!(reach.pieces, 30.0);
            assert_eq!(
                reach.founder_heterotrophy,
                founder.effective_trait_with_steepness(1, k_steep)
            );
            assert_eq!(reach.founder_structure, founder.structure);
            let area = spec.parameters.world_extent.powi(2);
            let rho = (x * CARCASSES_PER_CELL * SIDE_CELLS * SIDE_CELLS) as f32 / area;
            assert!((reach.litter_density - rho).abs() < 1e-6, "x {x}");
            assert_eq!(world.params().body_reach_coefficient, reach.coefficient);
            assert!(!reach.clamped);
            assert_eq!(
                reach.founder_reach,
                consumption_reach(
                    reach.founder_heterotrophy,
                    reach.founder_structure,
                    world.params()
                )
            );
            assert!((reach.pieces_spanned - 30.0).abs() < 3e-2, "{reach:?}");
        }
        // At 1x, about one piece is the contact-only reach: the clamp applies
        // and the world keeps its coefficient of 0.
        let mut world = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            1,
            Community::ProducersAndDecomposers,
        );
        let reach = set_body_reach_pieces(&mut world, 1.0).unwrap();
        assert!(reach.clamped, "{reach:?}");
        assert_eq!(world.params().body_reach_coefficient, 0.0);
        // Without decomposer founders there is no founder reach to set.
        let mut world = mode1_mesocosm(&spec, mesocosm_wear(), 1, Community::ProducersOnly);
        assert!(set_body_reach_pieces(&mut world, 10.0).is_none());
    }

    /// `set_size_scaled_base_rate` (#807; world rules, flow 8, *How large `b`
    /// is*) sets `b = base_metabolic_rate / S_p +
    /// structure_maintenance_coefficient`, `S_p` the established producers'
    /// median structure #804 read, and records the decomposer founder's and
    /// the median producer founder's upkeep at founding under both rules.
    /// #804 worked the founder's out by hand: 0.407 a tick today, 0.121
    /// size-scaled.
    #[test]
    fn set_size_scaled_base_rate_sets_b_and_records_the_upkeep() {
        assert_eq!(ESTABLISHED_PRODUCER_STRUCTURE, 21.6);
        let spec = Mode1Spec::committed();
        let mut world = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            1,
            Community::ProducersAndDecomposers,
        );
        let before = world.params().clone();
        assert_eq!(before.mass_specific_base_rate, 0.0);
        let r = set_size_scaled_base_rate(&mut world);
        let b = before.base_metabolic_rate / ESTABLISHED_PRODUCER_STRUCTURE
            + before.structure_maintenance_coefficient;
        assert_eq!(
            r.established_producer_structure,
            ESTABLISHED_PRODUCER_STRUCTURE
        );
        assert_eq!(r.mass_specific_base_rate, b);
        assert_eq!(world.params().mass_specific_base_rate, b);
        assert!((b - 0.02496).abs() < 1e-4, "b {b}");

        let founder = r.decomposer_founder.expect("decomposer founders");
        assert!(
            (founder.per_agent.total - 0.407).abs() < 1e-3,
            "{founder:?}"
        );
        assert!(
            (founder.size_scaled.total - 0.121).abs() < 1e-3,
            "{founder:?}"
        );
        assert!((founder.per_agent.base_and_structure - 0.347).abs() < 1e-3);
        assert!((founder.size_scaled.base_and_structure - 0.061).abs() < 1e-3);
        assert_eq!(founder.per_agent.movement, founder.size_scaled.movement);
        assert!((founder.per_agent.movement - 0.0165).abs() < 1e-3);

        // The median producer founder, smaller than S_p, pays less.
        let producer = r.median_producer.expect("producer founders");
        assert!(producer.structure < ESTABLISHED_PRODUCER_STRUCTURE);
        assert!(
            producer.size_scaled.total < producer.per_agent.total,
            "{producer:?}"
        );
        let section = size_scaled_base_rate_section(&r);
        assert!(section.contains("## Size-scaled base rate"), "{section}");

        // Producers only: no decomposer founder to record.
        let mut world = mode1_mesocosm(&spec, mesocosm_wear(), 1, Community::ProducersOnly);
        assert!(
            set_size_scaled_base_rate(&mut world)
                .decomposer_founder
                .is_none()
        );
    }

    /// `set_size_scaled_base_rate_at` (#814) sets `b` from an `S_p` given
    /// explicitly, so a reading can settle `S_p` under the rules it runs
    /// with, and records that `S_p`. At [`ESTABLISHED_PRODUCER_STRUCTURE`] it
    /// is `set_size_scaled_base_rate`.
    #[test]
    fn set_size_scaled_base_rate_at_takes_s_p_explicitly() {
        let spec = Mode1Spec::committed();
        let build = || {
            mode1_mesocosm(
                &spec,
                mesocosm_wear(),
                1,
                Community::ProducersAndDecomposers,
            )
        };
        let mut world = build();
        let before = world.params().clone();
        let r = set_size_scaled_base_rate_at(&mut world, 11.9);
        let b = before.base_metabolic_rate / 11.9 + before.structure_maintenance_coefficient;
        assert_eq!(r.established_producer_structure, 11.9);
        assert_eq!(r.mass_specific_base_rate, b);
        assert_eq!(world.params().mass_specific_base_rate, b);
        let founder = r.decomposer_founder.expect("decomposer founders");
        // A smaller S_p raises b, so the founder pays more than at 21.6.
        let mut default = build();
        let d = set_size_scaled_base_rate(&mut default);
        assert!(b > d.mass_specific_base_rate);
        assert!(founder.size_scaled.total > d.decomposer_founder.unwrap().size_scaled.total);
        assert_eq!(founder.per_agent, d.decomposer_founder.unwrap().per_agent);

        let mut at_constant = build();
        assert_eq!(
            set_size_scaled_base_rate_at(&mut at_constant, ESTABLISHED_PRODUCER_STRUCTURE),
            d
        );
        assert_eq!(at_constant.params(), default.params());
    }

    /// Ticks until a carcass drained every tick by one unit-heterotrophy
    /// drainer under `params` holds half its structure at death, by the
    /// stepper's own drain pass.
    fn drained_half_life(params: &WorldParameters) -> u64 {
        use explorers_sim::spatial::{NutrientGrid, SpatialGrid};
        let extent = params.world_extent;
        let site = (extent / 2.0, extent / 2.0);
        let heterotroph = TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 1.0,
            mobility: 0.0,
            kappa: 0.5,
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        };
        let mut agents = vec![explorers_sim::Agent {
            id: 0,
            position: site,
            reserve: 100.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 5.0,
            traits: heterotroph,
            wear: Default::default(),
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
            marker: Default::default(),
        }];
        let structure = 1000.0;
        let mut carcasses = vec![explorers_sim::Carcass {
            id: 1,
            position: site,
            energy: structure,
            energy_at_death: structure,
            nutrient: 0.0,
            traits: TraitVector {
                photosynthetic_absorption: 0.5,
                heterotrophy: 0.0,
                ..heterotroph
            },
        }];
        let mut grid = SpatialGrid::new(extent, extent / 10.0);
        grid.insert(0, site);
        let mut nutrient_grid = NutrientGrid::new(extent, extent / 10.0, 0.0);
        for tick in 1..=100_000 {
            explorers_sim::phase::resolve_drains(
                &mut agents,
                &mut carcasses,
                &grid,
                params,
                &mut nutrient_grid,
            );
            agents[0].reserve = 100.0;
            if carcasses[0].energy <= structure / 2.0 {
                return tick;
            }
        }
        panic!("the carcass never halved");
    }

    /// The `k_d` conversion (#805; world rules, *Carcass decomposition
    /// saturates in its decomposers → How large `k_d` is*): a litter
    /// half-life of L producer lifespans is `k_d = ln 2 / (L × T_p)`, so
    /// under full saturation (`Σc ≫ K`, here K a millionth of the drainer's
    /// capability) a carcass drained every tick halves in L × `T_p` ticks.
    #[test]
    fn litter_half_life_under_full_saturation_is_l_producer_lifespans() {
        let t_p = ESTABLISHED_PRODUCER_LIFESPAN_TICKS;
        assert_eq!(t_p, 229.0);
        let mut params = Mode1Spec::committed().parameters;
        params.carcass_half_saturation =
            1e-6 * explorers_sim::units::HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK;
        for l in [0.5, 1.0, 4.0] {
            let k_d = carcass_decay_constant_for_half_life(l, t_p);
            assert!(
                (k_d - std::f32::consts::LN_2 / (l * t_p)).abs() < 1e-9,
                "L {l}: k_d {k_d}"
            );
            params.carcass_decay_constant = k_d;
            let half_life = drained_half_life(&params) as f32;
            assert!(
                (half_life - l * t_p).abs() <= 1.0,
                "L {l}: half-life {half_life} ticks, expected {}",
                l * t_p
            );
        }
    }

    /// `set_decomposition_kinetics` (#805) sets `k_d` from L and `T_p`, and
    /// `K` from K founder capabilities: one decomposer founder's drain
    /// capability at founding, effective heterotrophy × `u_H`, read from the
    /// world built. Without K the world keeps its default `K`, recorded in
    /// founder capabilities too.
    #[test]
    fn set_decomposition_kinetics_reads_the_founders_drain_capability() {
        let spec = Mode1Spec::committed();
        let mut world = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            1,
            Community::ProducersAndDecomposers,
        );
        let founder = world
            .agents()
            .iter()
            .find(|a| a.traits.photosynthetic_absorption == 0.0)
            .unwrap()
            .clone();
        let capability = founder
            .effective_trait_with_steepness(1, world.params().wear_degradation_steepness)
            * explorers_sim::units::HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK;
        let kinetics = set_decomposition_kinetics(&mut world, 2.0, Some(3.0)).unwrap();
        assert_eq!(kinetics.litter_half_life, 2.0);
        assert_eq!(
            kinetics.producer_lifespan,
            ESTABLISHED_PRODUCER_LIFESPAN_TICKS
        );
        assert_eq!(
            kinetics.decay_constant,
            carcass_decay_constant_for_half_life(2.0, ESTABLISHED_PRODUCER_LIFESPAN_TICKS)
        );
        assert_eq!(kinetics.founder_drain_capability, Some(capability));
        assert_eq!(kinetics.half_saturation_founder_capabilities, Some(3.0));
        assert_eq!(kinetics.half_saturation, 3.0 * capability);
        assert_eq!(
            world.params().carcass_decay_constant,
            kinetics.decay_constant
        );
        assert_eq!(
            world.params().carcass_half_saturation,
            kinetics.half_saturation
        );

        // L alone: K stays the default u_H, about 0.93 founder capabilities.
        let mut world = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            1,
            Community::ProducersAndDecomposers,
        );
        let kinetics = set_decomposition_kinetics(&mut world, 1.0, None).unwrap();
        assert_eq!(
            kinetics.half_saturation,
            explorers_sim::DEFAULT_CARCASS_HALF_SATURATION
        );
        assert_eq!(
            world.params().carcass_half_saturation,
            kinetics.half_saturation
        );
        let in_founders = kinetics.half_saturation_founder_capabilities.unwrap();
        assert!(
            (in_founders - kinetics.half_saturation / capability).abs() < 1e-6,
            "{kinetics:?}"
        );

        // Without decomposer founders, L still converts; K cannot.
        let mut world = mode1_mesocosm(&spec, mesocosm_wear(), 1, Community::ProducersOnly);
        let kinetics = set_decomposition_kinetics(&mut world, 1.0, None).unwrap();
        assert_eq!(kinetics.founder_drain_capability, None);
        assert_eq!(kinetics.half_saturation_founder_capabilities, None);
        let mut world = mode1_mesocosm(&spec, mesocosm_wear(), 1, Community::ProducersOnly);
        assert!(set_decomposition_kinetics(&mut world, 1.0, Some(1.0)).is_none());
        assert_eq!(world.params().carcass_decay_constant, 0.0);
    }

    /// Litter inflow (#804): a death adds the structure its carcass held at
    /// death, booked to producers when the dead was a producer and to the
    /// others otherwise. Drains, births and every other event add nothing.
    #[test]
    fn a_death_adds_its_carcass_structure_at_death_to_the_litter_inflow() {
        use explorers_sim::event::{Event, EventLog};
        let event = |seq: u64, kind: EventKind, source: u64| Event {
            tick: 0,
            seq,
            target_was_carcass: matches!(kind, EventKind::Consumed),
            kind,
            source,
            target: None,
            energy_delta: 9.0,
            position: None,
            second_parent: None,
            nutrient_delta: 0.0,
        };
        let mut log = EventLog::new();
        for e in [
            event(0, EventKind::Consumed, 1),
            event(1, EventKind::Senesced, 1),
            event(2, EventKind::Died, 1),
            event(3, EventKind::Born, 4),
            event(4, EventKind::Died, 2),
            event(5, EventKind::Died, 3),
        ] {
            log.append(e).unwrap();
        }
        // Agents 1 and 3 were producers; their carcasses held 4.0 and 0.5 at
        // death, agent 2's held 1.5.
        let at_death = HashMap::from([(1, 4.0_f32), (2, 1.5), (3, 0.5)]);
        let producers = [1, 3];
        let mut inflow = LitterInflow::default();
        for e in log.since(0) {
            inflow.book(e, producers.contains(&e.source), &at_death);
        }
        assert_eq!(
            inflow,
            LitterInflow {
                producer: 4.5,
                other: 1.5,
            }
        );
        assert_eq!(inflow.total(), 6.0);
    }

    /// A run samples the world's litter beside the centre patch (#804): the
    /// standing carcass structure per cell, the carcasses standing and those
    /// still holding structure, the inflow since the previous sample, and
    /// the established producers alive. With no carcass ever removed, the
    /// inflow over the run is the structure at death of every carcass that
    /// was not standing at the start.
    #[test]
    fn a_run_samples_the_worlds_litter_and_its_inflow() {
        let mut world = mode1_mesocosm(
            &Mode1Spec::committed(),
            mesocosm_wear(),
            3,
            Community::ProducersAndDecomposers,
        );
        let seeded: std::collections::HashSet<u64> =
            world.carcasses().iter().map(|c| c.id).collect();
        let report = run_mode1(&mut world, 120, 10);
        let litter = &report.litter;
        assert_eq!(litter.samples.len(), report.samples.len());
        for (l, c) in litter.samples.iter().zip(&report.samples) {
            assert_eq!(l.tick, c.tick);
            assert!(l.nonempty_carcasses <= l.carcasses);
        }
        let first = &litter.samples[0];
        assert_eq!(
            (first.inflow, first.inflow_ticks),
            (LitterInflow::default(), 0)
        );
        assert!(litter.samples[1..].iter().all(|s| s.inflow_ticks == 10));

        let cells = (SIDE_CELLS * SIDE_CELLS) as f64;
        let last = litter.samples.last().unwrap();
        let standing: f64 = world.carcasses().iter().map(|c| c.energy as f64).sum();
        assert!((last.standing_per_cell - standing / cells).abs() < 1e-6);
        assert_eq!(last.carcasses, world.carcasses().len());
        assert_eq!(
            last.nonempty_carcasses,
            world.carcasses().iter().filter(|c| c.energy > 0.0).count()
        );

        let inflow: f64 = litter.samples.iter().map(|s| s.inflow.total()).sum();
        let created: f64 = (world.carcasses().iter())
            .filter(|c| !seeded.contains(&c.id))
            .map(|c| c.energy_at_death as f64)
            .sum();
        assert!(inflow > 0.0);
        assert!(
            (inflow - created).abs() < 1e-3 * created,
            "{inflow} vs {created}"
        );
        assert!(litter.samples.iter().any(|s| s.inflow.producer > 0.0));

        // Established producers: alive older than ESTABLISHMENT_AGE, so none
        // before then, and never more than the world's producers.
        for s in &litter.samples {
            if s.tick <= ESTABLISHMENT_AGE {
                assert!(s.established_structures.is_empty());
            }
        }
        assert!(!last.established_structures.is_empty());
        assert!(last.established_structures.iter().all(|&x| x > 0.0));
    }

    /// The second half's litter reading (#804): standing litter and carcass
    /// counts over the second half's samples, inflow per cell per tick over
    /// the intervals ending after its first sample, and the structure of the
    /// established producers pooled over its samples (`S_p`).
    #[test]
    fn the_litter_window_reads_the_second_half() {
        let mut world = mode1_mesocosm(
            &Mode1Spec::committed(),
            mesocosm_wear(),
            3,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, 120, 10);
        let (samples, w) = (&report.litter.samples, &report.litter.second_half);
        let half = &samples[samples.len() / 2..];
        let cells = (SIDE_CELLS * SIDE_CELLS) as f64;
        let standing: Vec<f64> = half.iter().map(|s| s.standing_per_cell).collect();
        let mean = standing.iter().sum::<f64>() / standing.len() as f64;
        assert!((w.standing_per_cell.mean - mean).abs() < 1e-9);
        assert_eq!(
            w.standing_per_cell.min,
            standing.iter().copied().fold(f64::INFINITY, f64::min)
        );
        assert_eq!(
            w.standing_per_cell.max,
            standing.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        );
        assert_eq!(
            (w.from_tick, w.to_tick),
            (half[0].tick, half.last().unwrap().tick)
        );

        let intervals = &half[1..];
        let ticks: u64 = intervals.iter().map(|s| s.inflow_ticks).sum();
        let total: f64 = intervals.iter().map(|s| s.inflow.total()).sum();
        let producer: f64 = intervals.iter().map(|s| s.inflow.producer).sum();
        assert!((w.inflow.mean - total / (cells * ticks as f64)).abs() < 1e-9);
        assert!((w.producer_inflow.mean - producer / (cells * ticks as f64)).abs() < 1e-9);
        assert!(w.inflow.min <= w.inflow.mean && w.inflow.mean <= w.inflow.max);

        let pooled: Vec<f64> = (half.iter())
            .flat_map(|s| s.established_structures.iter().map(|&x| x as f64))
            .collect();
        let s_p = &w.established_structure;
        assert_eq!(*s_p, StructureDistribution::of(&pooled));
        assert!(s_p.count > 0);
        assert!(s_p.min <= s_p.p25 && s_p.p25 <= s_p.median && s_p.median <= s_p.p75);
        assert!(s_p.p75 <= s_p.max && s_p.median > 0.0);
        let lifespans: Vec<u64> = (report.producer_lifespans.iter().copied())
            .filter(|&a| a > ESTABLISHMENT_AGE)
            .collect();
        assert_eq!(
            w.established_lifespan_median,
            LifespanDistribution::of(&lifespans).median
        );
    }

    /// Both summaries carry the litter reading per arm (#804): standing
    /// litter, inflow by source and carcass counts, and the established
    /// producers' structure and lifespan.
    #[test]
    fn the_summaries_report_the_litter_reading() {
        let spread = |mean, min, max| Spread { mean, min, max };
        let mut report = Mode1Report {
            samples: vec![],
            producer_lifespans: vec![],
            senescent_lifespans: vec![],
            final_roles: RoleTally::default(),
            heterotroph_dominant: 0,
            carcass_drain: DrainSplit::default(),
            arm_carcass_drain: DrainSplit::default(),
            ledger: Ledger::default(),
            founders: FounderReadout::default(),
            guild: GuildReadout::default(),
            recruitment: None,
            marker: None,
            litter: LitterReadout {
                samples: vec![],
                second_half: LitterWindow {
                    from_tick: 750,
                    to_tick: 1500,
                    standing_per_cell: spread(54.0, 50.5, 58.25),
                    carcasses: spread(1400.0, 1377.0, 1420.0),
                    nonempty_carcasses: spread(900.4, 850.0, 950.0),
                    inflow: spread(0.125, 0.05, 0.2),
                    producer_inflow: spread(0.12, 0.05, 0.19),
                    other_inflow: spread(0.005, 0.0, 0.01),
                    established_structure: StructureDistribution {
                        count: 4000,
                        min: 1.0,
                        p10: 2.0,
                        p25: 4.0,
                        median: 8.0,
                        p75: 12.0,
                        p90: 16.0,
                        max: 30.0,
                        mean: 9.5,
                    },
                    established_lifespan_median: 229,
                },
            },
        };
        let litter = "| 750–1500 | 54.00 (50.50–58.25) | 0.1250 (0.0500–0.2000) | 0.1200 (0.0500–0.1900) | 0.0050 (0.0000–0.0100) | 1400 / 900 |";
        let established =
            "| 4000 | 1.00 | 2.00 | 4.00 | 8.00 | 12.00 | 16.00 | 30.00 | 9.50 | 229 |";
        let summary = mode1_summary("h", &report);
        for line in [LITTER_HEADER, ESTABLISHED_HEADER, litter, established] {
            assert!(summary.contains(line), "{line}\n{summary}");
        }

        let control = report.clone();
        report.litter = LitterReadout::default();
        let paired = PairedReport {
            fork_tick: 0,
            clearance: Clearance::default(),
            perturbed: report,
            control,
        };
        let summary = paired_summary("h", &paired);
        for line in [litter, established] {
            assert!(summary.contains(&format!("| control {line}")), "{summary}");
        }
        assert!(
            summary.contains(
                "| perturbed | 0 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0 |"
            ),
            "{summary}"
        );
    }

    /// #821's founder starvation readout, read independently off a full
    /// event log and the founders' states before each step. A founder's
    /// unallocated reserve *runs out* in the first step it begins with
    /// reserve no greater than that step's upkeep: metabolism leaves it at
    /// zero, and any rest falls to the cascade (or it starves). The earmark
    /// at that moment is its earmark as that step began. A tick on the
    /// earmark (on structure) is a step after which the founder lives and in
    /// which the earmark (structure) paid some of its upkeep.
    #[test]
    fn the_founder_starvation_readout_follows_each_founder_through_the_cascade() {
        let spec = Mode1Spec::committed();
        let build = |earmark: bool, shrinks: bool| {
            let mut world = mode1_mesocosm(
                &spec,
                mesocosm_wear(),
                4,
                Community::ProducersAndDecomposers,
            );
            let set = set_starvation_cascade(&mut world, earmark, shrinks);
            assert_eq!((set.draws_earmark, set.shrinks), (earmark, shrinks));
            world
        };
        for (earmark, shrinks) in [(false, false), (true, false), (false, true), (true, true)] {
            let ticks = 80;
            let report = run_mode1(&mut build(earmark, shrinks), ticks, 10);

            let mut world = build(earmark, shrinks);
            let mut expected: std::collections::BTreeMap<u64, FounderStarvation> = (world
                .agents()
                .iter())
            .filter(|a| a.traits.photosynthetic_absorption == 0.0 && a.traits.heterotrophy > 0.0)
            .map(|a| {
                (
                    a.id,
                    FounderStarvation {
                        id: a.id,
                        ..FounderStarvation::default()
                    },
                )
            })
            .collect();
            for _ in 0..ticks {
                let before: HashMap<u64, (f32, f32, f32, f32)> = (world.agents().iter())
                    .filter(|a| expected.contains_key(&a.id))
                    .map(|a| {
                        let upkeep = explorers_sim::phase::metabolic_cost(a, world.params());
                        (a.id, (a.reserve, a.repro_reserve, a.structure, upkeep))
                    })
                    .collect();
                let cursor = world.event_log().len();
                world.step();
                let after = world.tick();
                for (id, &(reserve, earmark_held, _, upkeep)) in &before {
                    let f = expected.get_mut(id).unwrap();
                    if f.ran_out_tick.is_none() && reserve <= upkeep {
                        f.ran_out_tick = Some(after);
                        f.earmark_at_running_out = Some(earmark_held);
                    }
                }
                for e in world.event_log().since(cursor) {
                    if e.kind != EventKind::Born {
                        continue;
                    }
                    for parent in [e.target, e.second_parent].into_iter().flatten() {
                        if let Some(f) = expected.get_mut(&parent) {
                            if f.ran_out_tick.is_none() {
                                f.reproduced_before_running_out = true;
                            }
                        }
                    }
                }
                for (id, &(reserve, earmark_held, structure, upkeep)) in &before {
                    let f = expected.get_mut(id).unwrap();
                    let Some(now) = world.agents().iter().find(|a| a.id == *id) else {
                        f.death_tick = Some(after);
                        continue;
                    };
                    if reserve > upkeep {
                        continue;
                    }
                    let shortfall = upkeep - reserve.max(0.0);
                    let from_earmark = if earmark { earmark_held.max(0.0) } else { 0.0 };
                    if earmark && earmark_held > 0.0 && shortfall > 0.0 {
                        f.ticks_on_earmark += 1;
                    }
                    if shrinks && shortfall > from_earmark {
                        f.ticks_on_structure += 1;
                        assert!(
                            now.structure < structure,
                            "a structure tick burns structure"
                        );
                    }
                }
            }
            let expected: Vec<FounderStarvation> = expected.into_values().collect();
            let readout = &report.founders.starvation;
            assert_eq!(readout.founders, expected, "arm ({earmark}, {shrinks})");
            assert!(readout.ran_out() > 0, "some founder runs out of reserve");
            let lived_on = |f: &FounderStarvation| (f.ticks_on_earmark, f.ticks_on_structure);
            let total: (u64, u64) = expected
                .iter()
                .map(lived_on)
                .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
            assert_eq!(total.0 > 0, earmark, "earmark ticks iff the switch is on");
            assert_eq!(total.1 > 0, shrinks, "structure ticks iff the switch is on");
        }
    }

    /// The readout's summary: nearest-rank medians, the death tick over the
    /// founders that died and the rest over those whose reserve ran out, and
    /// the count that reproduced first.
    #[test]
    fn the_starvation_readout_summarises_as_medians_and_counts() {
        let f = |id, death, ran_out, first, earmark, on_e, on_s| FounderStarvation {
            id,
            death_tick: death,
            ran_out_tick: ran_out,
            reproduced_before_running_out: first,
            earmark_at_running_out: earmark,
            ticks_on_earmark: on_e,
            ticks_on_structure: on_s,
        };
        let r = StarvationReadout {
            founders: vec![
                f(1, Some(30), Some(5), true, Some(2.0), 20, 4),
                f(2, Some(10), Some(6), false, Some(16.0), 3, 1),
                f(3, None, Some(7), false, Some(9.0), 8, 0),
                f(4, None, None, true, None, 0, 0),
            ],
        };
        assert_eq!(r.died(), 2);
        assert_eq!(r.ran_out(), 3);
        assert_eq!(r.reproduced_first(), 2);
        assert_eq!(r.median_death_tick(), Some(10));
        assert_eq!(r.median_earmark_at_running_out(), Some(9.0));
        assert_eq!(r.median_ticks_on_earmark(), Some(8));
        assert_eq!(r.median_ticks_on_structure(), Some(1));
        let section = founder_starvation_section(&[("run", &r)]);
        assert!(
            section.contains("| run | 4 | 2 | 10 | 3 | 2 | 9.000 | 8 | 1 |"),
            "{section}"
        );
        let empty = founder_starvation_section(&[("run", &StarvationReadout::default())]);
        assert!(
            empty.contains("| run | 0 | 0 | - | 0 | 0 | - | - | - |"),
            "{empty}"
        );
    }

    /// Mean wear over an agent's functional traits.
    fn mean_wear(a: &explorers_sim::Agent) -> f32 {
        a.wear.iter().sum::<f32>() / a.wear.len() as f32
    }

    /// Each founder's life history, read independently off a full event log
    /// of `world` stepped `ticks` times: its state as every step it lives
    /// through begins, the upkeep that step charges, its own gain and drain
    /// in it, and the tick and cause of its death.
    fn replay_founder_lives(mut world: World, ticks: u64) -> Vec<FounderLife> {
        let founders: HashMap<u64, TraitVector> = (world.agents().iter())
            .filter(|a| a.traits.photosynthetic_absorption == 0.0 && a.traits.heterotrophy > 0.0)
            .map(|a| (a.id, a.traits))
            .collect();
        let mut expected: std::collections::BTreeMap<u64, FounderLife> = (founders.keys())
            .map(|&id| {
                (
                    id,
                    FounderLife {
                        id,
                        first_tick: 0,
                        ..FounderLife::default()
                    },
                )
            })
            .collect();
        for _ in 0..ticks {
            let agents: HashMap<u64, TraitVector> =
                world.agents().iter().map(|a| (a.id, a.traits)).collect();
            let carcasses: HashMap<u64, TraitVector> =
                world.carcasses().iter().map(|c| (c.id, c.traits)).collect();
            let before: HashMap<u64, explorers_sim::Agent> = (world.agents().iter())
                .filter(|a| founders.contains_key(&a.id))
                .map(|a| (a.id, a.clone()))
                .collect();
            let cursor = world.event_log().len();
            world.step();
            let after = world.tick();
            let mut income: HashMap<u64, (f64, f64)> = HashMap::new();
            let mut senesced = std::collections::HashSet::new();
            for e in world.event_log().since(cursor) {
                if e.kind == EventKind::Senesced {
                    senesced.insert(e.source);
                }
                let Some(consumer) = founders.get(&e.source) else {
                    continue;
                };
                if e.kind != EventKind::Consumed {
                    continue;
                }
                let target = e.target.unwrap();
                let target = if e.target_was_carcass {
                    carcasses[&target]
                } else {
                    agents[&target]
                };
                let eff =
                    explorers_sim::trophic_transfer_efficiency(consumer, &target, world.params());
                let entry = income.entry(e.source).or_default();
                entry.0 += (e.energy_delta * eff) as f64;
                entry.1 += e.energy_delta as f64;
            }
            for (id, a) in &before {
                let life = expected.get_mut(id).unwrap();
                let upkeep = explorers_sim::phase::metabolic_cost(a, world.params());
                life.reserve.push(a.reserve);
                life.repro_reserve.push(a.repro_reserve);
                life.repro_nutrient.push(a.repro_nutrient);
                life.nutrient.push(a.nutrient);
                life.structure.push(a.structure);
                life.peak_structure.push(a.peak_structure);
                life.wear.push(mean_wear(a));
                life.upkeep.push(upkeep);
                let (g, d) = income.get(id).copied().unwrap_or_default();
                life.gained.push(g);
                life.drained.push(d);
                if !world.agents().iter().any(|now| now.id == *id) {
                    life.death_tick = Some(after);
                    life.death_cause = Some(if senesced.contains(id) {
                        DeathCause::Senesced
                    } else if a.reserve <= upkeep {
                        DeathCause::Reserve
                    } else {
                        DeathCause::Structural
                    });
                }
            }
        }
        expected.into_values().collect()
    }

    /// The readout follows each founder tick by tick, as the replay reads it.
    #[test]
    fn the_recruitment_readout_follows_each_founder_tick_by_tick() {
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
        let expected = replay_founder_lives(build(), ticks);
        let readout = report.recruitment.as_ref().expect("founders are followed");
        assert_eq!(readout.founders, expected);
        assert!(expected.iter().any(|f| f.death_tick.is_some()));
        assert!(expected.iter().any(|f| f.gained.iter().sum::<f64>() > 0.0));
        assert_eq!(
            (run_mode1(
                &mut mode1_mesocosm(&spec, mesocosm_wear(), 4, Community::ProducersOnly),
                5,
                5,
            ))
            .recruitment,
            None,
            "no founders, no readout"
        );
    }

    /// A death is `senesced` when the stepper marked it so, else `reserve`
    /// when the member entered the step with unallocated reserve at or below
    /// its upkeep, else `structural`.
    #[test]
    fn the_cause_of_death_is_senesced_then_reserve_then_structural() {
        assert_eq!(DeathCause::of(true, Some((0.0, 1.0))), DeathCause::Senesced);
        assert_eq!(DeathCause::of(true, Some((5.0, 1.0))), DeathCause::Senesced);
        assert_eq!(DeathCause::of(false, Some((1.0, 1.0))), DeathCause::Reserve);
        assert_eq!(DeathCause::of(false, Some((0.5, 1.0))), DeathCause::Reserve);
        assert_eq!(
            DeathCause::of(false, Some((1.5, 1.0))),
            DeathCause::Structural
        );
        assert_eq!(DeathCause::of(false, None), DeathCause::Structural);

        // A senescence hazard well above the mesocosm's: founders die of
        // all three causes, each read as the replay reads it.
        let spec = Mode1Spec::committed();
        let build = || {
            let wear = Wear {
                senescence_hazard: 0.3,
                ..mesocosm_wear()
            };
            mode1_mesocosm(&spec, wear, 4, Community::ProducersAndDecomposers)
        };
        let ticks = 150;
        let report = run_mode1(&mut build(), ticks, 10);
        let expected = replay_founder_lives(build(), ticks);
        let founders = report.recruitment.expect("founders are followed").founders;
        assert_eq!(founders, expected);
        for cause in [
            DeathCause::Senesced,
            DeathCause::Reserve,
            DeathCause::Structural,
        ] {
            assert!(
                founders.iter().any(|f| f.death_cause == Some(cause)),
                "some founder dies {cause:?}"
            );
        }
        let json = serde_json::to_value(&founders[0]).unwrap();
        assert!(json["death_cause"].is_string(), "{json}");
    }

    /// The founder line's births and deaths, read independently off a full
    /// event log of `world` stepped `ticks` times. The line is the founders
    /// and every agent born to a living member; traits are nominal.
    fn replay_line(mut world: World, ticks: u64) -> (Vec<LineBirth>, Vec<LineDeath>) {
        let mut line: std::collections::HashSet<u64> = (world.agents().iter())
            .filter(|a| a.traits.photosynthetic_absorption == 0.0 && a.traits.heterotrophy > 0.0)
            .map(|a| a.id)
            .collect();
        let mut born_at: HashMap<u64, u64> = line.iter().map(|&id| (id, 0)).collect();
        let mut offspring: HashMap<u64, u32> = HashMap::new();
        let mut traits: HashMap<u64, TraitVector> = HashMap::new();
        let (mut births, mut deaths) = (vec![], vec![]);
        for _ in 0..ticks {
            traits.extend(world.agents().iter().map(|a| (a.id, a.traits)));
            let entering: HashMap<u64, (f32, f32)> = (world.agents().iter())
                .filter(|a| line.contains(&a.id))
                .map(|a| {
                    let upkeep = explorers_sim::phase::metabolic_cost(a, world.params());
                    (a.id, (a.reserve, upkeep))
                })
                .collect();
            let cursor = world.event_log().len();
            world.step();
            let after = world.tick();
            traits.extend(world.agents().iter().map(|a| (a.id, a.traits)));
            let mut senesced = std::collections::HashSet::new();
            for e in world.event_log().since(cursor) {
                match e.kind {
                    EventKind::Senesced => {
                        senesced.insert(e.source);
                    }
                    EventKind::Born => {
                        let parents: Vec<u64> =
                            [e.target, e.second_parent].into_iter().flatten().collect();
                        if !parents.iter().any(|p| line.contains(p)) {
                            continue;
                        }
                        let child = world.agents().iter().find(|a| a.id == e.source);
                        births.push(LineBirth {
                            tick: after,
                            offspring: e.source,
                            sexual: e.second_parent.is_some(),
                            parents: (parents.iter())
                                .map(|&p| LineParent {
                                    id: p,
                                    in_line: line.contains(&p),
                                    autotrophy: traits.get(&p).map(|t| t.photosynthetic_absorption),
                                    heterotrophy: traits.get(&p).map(|t| t.heterotrophy),
                                })
                                .collect(),
                            autotrophy: child.map(|a| a.traits.photosynthetic_absorption),
                            heterotrophy: child.map(|a| a.traits.heterotrophy),
                            nutrient_endowment: child.map(|a| a.nutrient_total(world.params())),
                        });
                        for p in &parents {
                            if line.contains(p) {
                                *offspring.entry(*p).or_default() += 1;
                            }
                        }
                        line.insert(e.source);
                        born_at.insert(e.source, after);
                    }
                    EventKind::Died if line.remove(&e.source) => {
                        let t = traits.get(&e.source);
                        deaths.push(LineDeath {
                            id: e.source,
                            tick: after,
                            age: after - born_at[&e.source],
                            cause: DeathCause::of(
                                senesced.contains(&e.source),
                                entering.get(&e.source).copied(),
                            ),
                            autotrophy: t.map(|t| t.photosynthetic_absorption),
                            heterotrophy: t.map(|t| t.heterotrophy),
                            offspring: offspring.get(&e.source).copied().unwrap_or(0),
                        });
                    }
                    _ => {}
                }
            }
        }
        (births, deaths)
    }

    /// Every birth with a parent in the founder line records its tick, its
    /// offspring, whether it was sexual, each parent's id, membership of the
    /// line and traits, and the offspring's traits and nutrient endowment.
    #[test]
    fn a_line_birth_records_each_parents_membership_of_the_line() {
        let spec = Mode1Spec::committed();
        let build = || {
            mode1_mesocosm(
                &spec,
                mesocosm_wear(),
                4,
                Community::ProducersAndDecomposers,
            )
        };
        let ticks = 150;
        let report = run_mode1(&mut build(), ticks, 10);
        let (births, _) = replay_line(build(), ticks);
        let readout = report.recruitment.expect("founders are followed");
        assert_eq!(readout.births, births);
        assert!(!births.is_empty(), "the founders reproduce");
        for b in &births {
            assert_eq!(b.parents.len(), 1 + b.sexual as usize);
            assert!(b.parents.iter().any(|p| p.in_line));
        }
        // Crossing with a parent outside the line, and mating within it.
        let sexual = |both_in: bool| {
            (births.iter())
                .filter(|b| b.sexual)
                .any(|b| b.parents.iter().all(|p| p.in_line) == both_in)
        };
        assert!(
            sexual(false),
            "a sexual birth with a parent outside the line"
        );
        assert!(sexual(true), "a sexual birth within the line");
    }

    /// Each line member that dies records its tick, age, cause, traits and
    /// offspring count; a founder's death in the line agrees with its life
    /// history.
    #[test]
    fn a_line_death_records_its_age_cause_traits_and_offspring() {
        let spec = Mode1Spec::committed();
        let build = || {
            mode1_mesocosm(
                &spec,
                mesocosm_wear(),
                4,
                Community::ProducersAndDecomposers,
            )
        };
        let ticks = 150;
        let report = run_mode1(&mut build(), ticks, 10);
        let (_, deaths) = replay_line(build(), ticks);
        let readout = report.recruitment.expect("founders are followed");
        assert_eq!(readout.deaths, deaths);
        assert!(deaths.iter().any(|d| d.offspring > 0), "a parent dies");
        let founder_ids: std::collections::HashSet<u64> =
            readout.founders.iter().map(|f| f.id).collect();
        assert!(
            deaths.iter().any(|d| !founder_ids.contains(&d.id)),
            "a descendant dies"
        );
        for f in &readout.founders {
            let death = deaths.iter().find(|d| d.id == f.id);
            assert_eq!(death.map(|d| d.tick), f.death_tick);
            assert_eq!(death.map(|d| d.cause), f.death_cause);
        }
    }

    /// The carcass drain is split per sample interval by lineage, founder
    /// line or not, and by the drainer's autotrophy, as [`DrainSplit`]
    /// splits it: over the samples the four parts sum to the run's drain.
    #[test]
    fn the_lineage_drain_split_sums_to_the_carcass_drain() {
        let spec = Mode1Spec::committed();
        let mut world = mode1_mesocosm(
            &spec,
            mesocosm_wear(),
            4,
            Community::ProducersAndDecomposers,
        );
        let report = run_mode1(&mut world, 150, 10);
        let drain = report
            .recruitment
            .expect("founders are followed")
            .carcass_drain;
        assert_eq!(
            drain.iter().map(|d| d.tick).collect::<Vec<_>>(),
            report.samples.iter().map(|s| s.tick).collect::<Vec<_>>(),
            "sampled with the centre patch"
        );
        assert_eq!(drain[0].founder_line.total() + drain[0].other.total(), 0.0);
        let sum = |side: fn(&DrainSplit) -> f64| -> f64 {
            drain
                .iter()
                .map(|d| side(&d.founder_line) + side(&d.other))
                .sum()
        };
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
        let whole = report.carcass_drain;
        assert!(close(sum(|d| d.pure_vertex), whole.pure_vertex));
        assert!(close(sum(|d| d.mixotroph), whole.mixotroph));
        let line: f64 = drain.iter().map(|d| d.founder_line.total()).sum();
        let other: f64 = drain.iter().map(|d| d.other.total()).sum();
        assert!(line > 0.0 && other > 0.0, "{line} {other}");
    }

    /// The recruitment readout only reads: a paired run with it records
    /// every other field of its report exactly as the same run without it.
    #[test]
    fn the_recruitment_readout_leaves_every_other_field_of_the_report_alone() {
        let spec = Mode1Spec::committed();
        let run = |record| {
            let world = mode1_mesocosm(
                &spec,
                mesocosm_wear(),
                4,
                Community::ProducersAndDecomposers,
            );
            serde_json::to_value(run_paired_recording(world, 60, 60, 10, record)).unwrap()
        };
        let (mut with, without) = (run(true), run(false));
        for arm in ["perturbed", "control"] {
            assert!(without[arm].get("recruitment").is_none());
            let readout = with[arm]
                .as_object_mut()
                .unwrap()
                .remove("recruitment")
                .expect("the readout is recorded");
            assert!(!readout["births"].as_array().unwrap().is_empty());
        }
        assert_eq!(with, without);
    }

    /// Under marker isolation (#837) mode 1's producers found cluster 0 and
    /// its decomposer founders cluster 1: unrelated clades, 10 s apart in
    /// the marker.
    #[test]
    fn marker_isolation_founds_producers_and_decomposer_founders_as_two_clusters() {
        let mut spec = Mode1Spec::committed();
        let set = set_marker_isolation(&mut spec, 100.0);
        assert_eq!(set.compatibility_scale, 100.0);
        let world = mode1_mesocosm_with_founders(
            &spec,
            mesocosm_wear(),
            3,
            Community::ProducersAndDecomposers,
            1,
            Some(10),
        );
        assert!(world.params().marker_isolation);
        assert_eq!(world.params().compatibility_scale, 100.0);
        let (mut producers, mut decomposers) = (0, 0);
        for a in world.agents() {
            if a.traits.photosynthetic_absorption > 0.0 {
                producers += 1;
                assert_eq!(a.marker, explorers_sim::marker_origin(0, 100.0));
            } else {
                decomposers += 1;
                assert_eq!(a.marker, explorers_sim::marker_origin(1, 100.0));
            }
        }
        assert_eq!((producers, decomposers), (PRODUCERS_PER_CELL * 81, 10));
    }

    /// The marker readout (#837) is written only under marker isolation, so
    /// a default report is unchanged. Under it, each sample counts the
    /// founder line and its share below autotrophy 0.1, and the end reads
    /// pairwise compatibility within the line and within the producers.
    #[test]
    fn the_marker_readout_is_recorded_only_under_marker_isolation() {
        let build = |on: bool| {
            let mut spec = Mode1Spec::committed();
            if on {
                set_marker_isolation(&mut spec, 100.0);
            }
            mode1_mesocosm_with_founders(
                &spec,
                mesocosm_wear(),
                2,
                Community::ProducersAndDecomposers,
                1,
                Some(10),
            )
        };
        let plain = run_mode1(&mut build(false), 60, 20);
        assert!(plain.marker.is_none());
        assert!(
            serde_json::to_value(&plain)
                .unwrap()
                .get("marker")
                .is_none()
        );

        let report = run_mode1(&mut build(true), 60, 20);
        let marker = report.marker.as_ref().expect("the readout is recorded");
        assert_eq!(marker.samples.len(), report.samples.len());
        let first = &marker.samples[0];
        assert_eq!((first.tick, first.line_living), (0, 10));
        assert_eq!(first.below_autotrophy, 10);
        assert_eq!(first.share_below_autotrophy, Some(1.0));
        for c in [&marker.line, &marker.producers] {
            if c.members >= 2 {
                assert_eq!(c.pairs, c.members * (c.members - 1) / 2);
                let (mean, lowest) = (c.mean.unwrap(), c.lowest.unwrap());
                assert!(lowest <= mean && mean <= 1.0 && lowest > 0.0, "{c:?}");
            }
        }
        assert!(marker.producers.members > 100);
        // Unrelated clades do not cross: every line birth's parents are in
        // the line.
        let recruitment = report.recruitment.as_ref().unwrap();
        assert!(
            recruitment
                .births
                .iter()
                .all(|b| b.parents.iter().all(|p| p.in_line))
        );
    }
}
