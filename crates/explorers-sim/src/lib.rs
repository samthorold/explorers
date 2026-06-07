pub mod energy_ledger;
pub mod event;
pub mod keyed_rng;
pub mod nutrient_ledger;
pub mod phase;
pub mod soa;
pub mod spatial;
pub mod topology;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};

pub fn toroidal_distance(a: (f32, f32), b: (f32, f32), extent: f32) -> f32 {
    let (dx, dy) = toroidal_displacement(a, b, extent);
    (dx * dx + dy * dy).sqrt()
}

pub fn toroidal_displacement(from: (f32, f32), to: (f32, f32), extent: f32) -> (f32, f32) {
    let mut dx = to.0 - from.0;
    let mut dy = to.1 - from.1;
    if dx > extent / 2.0 {
        dx -= extent;
    } else if dx < -extent / 2.0 {
        dx += extent;
    }
    if dy > extent / 2.0 {
        dy -= extent;
    } else if dy < -extent / 2.0 {
        dy += extent;
    }
    (dx, dy)
}

/// Wrap a position into the toroidal world.
pub fn wrap_position(pos: (f32, f32), extent: f32) -> (f32, f32) {
    let half = extent / 2.0;
    let x = (pos.0 + half).rem_euclid(extent) - half;
    let y = (pos.1 + half).rem_euclid(extent) - half;
    (x, y)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraitVector {
    pub photosynthetic_absorption: f32,
    /// Investment in consumption machinery — covers both predation (living targets)
    /// and decomposition (carcasses). Target state determines which, not separate traits.
    #[serde(alias = "consumption_rate")]
    pub heterotrophy: f32,
    pub mobility: f32,
    /// DEB-theory allocation parameter: fraction of mobilised energy routed to soma
    /// vs reproduction. High kappa = long-lived, slow-reproducing. Range 0.0–1.0.
    #[serde(
        alias = "somatic_maintenance",
        alias = "reproductive_investment",
        default = "default_kappa"
    )]
    pub kappa: f32,
    #[serde(default)]
    pub fecundity: f32,
    /// Probability of reproducing asexually (0.0–1.0). High values enable
    /// reproduction without a mate; low values require sexual pairing.
    #[serde(default)]
    pub asexual_propensity: f32,
    /// Investment in offspring dispersal mechanisms (spores, seeds, fruits).
    /// Higher values widen the Gaussian kernel for offspring placement.
    /// Independent of mobility and fecundity.
    #[serde(default)]
    pub dispersal: f32,
}

impl TraitVector {
    pub fn distance(&self, other: &TraitVector) -> f32 {
        let d0 = self.photosynthetic_absorption - other.photosynthetic_absorption;
        let d1 = self.heterotrophy - other.heterotrophy;
        let d2 = self.mobility - other.mobility;
        let d3 = self.kappa - other.kappa;
        let d4 = self.fecundity - other.fecundity;
        let d5 = self.asexual_propensity - other.asexual_propensity;
        let d6 = self.dispersal - other.dispersal;
        (d0 * d0 + d1 * d1 + d2 * d2 + d3 * d3 + d4 * d4 + d5 * d5 + d6 * d6).sqrt()
    }

    pub fn get(&self, index: usize) -> f32 {
        match index {
            0 => self.photosynthetic_absorption,
            1 => self.heterotrophy,
            2 => self.mobility,
            3 => self.kappa,
            4 => self.fecundity,
            5 => self.asexual_propensity,
            6 => self.dispersal,
            _ => unreachable!(),
        }
    }

    pub fn set(&mut self, index: usize, value: f32) {
        match index {
            0 => self.photosynthetic_absorption = value,
            1 => self.heterotrophy = value,
            2 => self.mobility = value,
            3 => self.kappa = value,
            4 => self.fecundity = value,
            5 => self.asexual_propensity = value,
            6 => self.dispersal = value,
            _ => unreachable!(),
        }
    }

    /// Number of trait dimensions.
    pub const NUM_DIMS: usize = 7;
}

fn default_kappa() -> f32 {
    0.5
}
fn default_wear_rate() -> f32 {
    0.1
}
fn default_wear_degradation_steepness() -> f32 {
    1.0
}
fn default_somatic_maintenance_cost_coefficient() -> f32 {
    0.1
}
fn default_use_wear_rate() -> f32 {
    0.01
}
fn default_structure_maintenance_coefficient() -> f32 {
    0.01
}
fn default_repair_decay() -> f32 {
    1.0
}
fn default_trophic_distance_decay() -> f32 {
    1.0
}

fn zero_traits() -> TraitVector {
    TraitVector {
        photosynthetic_absorption: 0.0,
        heterotrophy: 0.0,
        mobility: 0.0,
        kappa: 0.0,
        fecundity: 0.0,
        asexual_propensity: 0.0,
        dispersal: 0.0,
    }
}
fn default_base_nutrient_ratio() -> f32 {
    0.1
}
fn default_specification_nutrient_coefficient() -> f32 {
    0.2
}
fn default_sensing_range_coefficient() -> f32 {
    10.0
}
fn default_reproductive_compatibility_distance() -> f32 {
    2.0
}
fn default_reproduction_nutrient_threshold() -> f32 {
    1.0
}
fn default_maintenance_cost_exponent() -> f32 {
    2.0
}
fn default_nutrient_grid_cell_size() -> f32 {
    10.0
}
fn default_growth_retention_multiplier() -> f32 {
    2.0
}
fn default_reserve_mobilisation_rate() -> f32 {
    1.0
}
fn default_offspring_structure_fraction() -> f32 {
    0.2
}
fn default_asexual_propensity_maintenance_cost() -> f32 {
    0.01
}
fn default_dispersal_propagule_cost_exponent() -> f32 {
    2.0
}

/// Split a per-agent initial energy budget into a (reserve, structure, heat)
/// triple using the same reserve/structure provisioning that reproduction
/// applies to newborns: a fraction `offspring_structure_fraction` of the
/// budget is committed to structure via a lossy `growth_efficiency`
/// conversion; the rest becomes reserve. Heat is the unconverted remainder.
/// Conservation: reserve + structure + heat = budget.
pub fn provision_initial_reserve_structure(
    budget: f32,
    params: &WorldParameters,
) -> (f32, f32, f32) {
    let frac = params.offspring_structure_fraction.clamp(0.0, 1.0);
    let structure_share = budget * frac;
    let structure = structure_share * params.growth_efficiency;
    let heat = structure_share - structure;
    let reserve = budget - structure_share;
    (reserve, structure, heat)
}

/// Fraction of the post-efficiency reproductive energy budget spent at a
/// reproduction event on building propagule structures (spores, plumes, fruit),
/// as a function of the dispersal trait. Rises superlinearly with dispersal
/// (`coefficient * dispersal^exponent`) to enforce anti-generalist economics,
/// and is clamped to [0, 1]. The spent fraction dissipates as heat rather than
/// provisioning offspring, so higher dispersal leaves less for each offspring —
/// the dispersal analogue of the fecundity quantity/quality trade-off, charged
/// only at the reproduction event (never per tick). A zero coefficient disables
/// the cost.
pub fn dispersal_propagule_cost_fraction(dispersal: f32, params: &WorldParameters) -> f32 {
    if params.dispersal_propagule_cost_coefficient <= 0.0 {
        return 0.0;
    }
    let d = dispersal.max(0.0);
    let frac = params.dispersal_propagule_cost_coefficient
        * d.powf(params.dispersal_propagule_cost_exponent);
    frac.clamp(0.0, 1.0)
}

/// How a newborn (offspring or tick-0 seeded agent) is provisioned across both
/// currencies, with birth structure co-limited by the agent's donated nutrient
/// (ADR-0003 embodiment).
pub struct OffspringProvision {
    pub structure: f32,
    pub reserve: f32,
    /// Free nutrient store after the bound birth nutrient is deducted.
    pub free_nutrient: f32,
    /// Energy dissipated to heat by the lossy reserve-to-structure conversion.
    pub heat: f32,
}

/// Provision a newborn's structure, reserve, and nutrient from its energy and
/// nutrient budgets. Birth structure binds nutrient (`structure × demand`), so
/// it is co-limited (Liebig's law of the minimum): the structure built is the
/// smaller of what the structure-energy share affords and what the nutrient
/// budget supports. Unmatched structure-energy stays in the newborn's reserve
/// rather than burning, mirroring in-life growth.
///
/// Conservation: `reserve + structure + heat = energy_budget` (energy) and
/// `free_nutrient + structure × demand = nutrient_budget` (nutrient).
pub fn provision_offspring(
    traits: &TraitVector,
    structure_energy_share: f32,
    energy_budget: f32,
    nutrient_budget: f32,
    params: &WorldParameters,
) -> OffspringProvision {
    let efficiency = params.growth_efficiency;
    let energy_limited = (structure_energy_share * efficiency).max(0.0);
    let ratio = stoichiometric_demand(traits, 1.0, params); // demand per unit structure
    let nutrient_limited = if ratio > 0.0 {
        (nutrient_budget / ratio).max(0.0)
    } else {
        f32::INFINITY
    };
    let structure = energy_limited.min(nutrient_limited);
    // Energy actually spent building that structure; unmatched energy stays in
    // reserve. Guard efficiency == 0 (no structure built).
    let energy_spent = if efficiency > 0.0 {
        structure / efficiency
    } else {
        0.0
    };
    let heat = energy_spent - structure;
    let reserve = energy_budget - energy_spent;
    let free_nutrient = (nutrient_budget - structure * ratio).max(0.0);
    OffspringProvision {
        structure,
        reserve,
        free_nutrient,
        heat,
    }
}

/// Draw the nutrient bound in each seeded agent's birth structure from the
/// available pool at the agent's location (ADR-0003 embodiment). Seeded agents
/// are born with structure, which binds `structure × demand`; sourcing it from
/// the pool keeps total system nutrient at world creation equal to the initial
/// pool (nutrient is conserved, never conjured into the living system).
fn bind_seed_structure_nutrient(
    agents: &[Agent],
    nutrient_grid: &mut spatial::NutrientGrid,
    params: &WorldParameters,
) {
    for agent in agents {
        let bound = agent.bound_nutrient(params);
        if bound > 0.0 {
            *nutrient_grid.at_position(agent.position) -= bound;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldParameters {
    pub solar_flux_magnitude: f32,
    /// Peak trophic transfer efficiency when consumer and target are identical
    /// in trait space (distance = 0). Replaces the old flat consumption/decomposition
    /// efficiency parameters.
    #[serde(alias = "consumption_efficiency")]
    pub base_trophic_efficiency: f32,
    /// Exponential decay rate for trophic efficiency as trait-space distance increases.
    /// Higher values penalise biochemical dissimilarity more steeply.
    #[serde(default = "default_trophic_distance_decay")]
    pub trophic_distance_decay: f32,
    pub reproduction_efficiency: f32,
    pub base_metabolic_rate: f32,
    pub movement_cost_coefficient: f32,
    /// Sensing range = mobility * sensing_range_coefficient.
    /// Subordinate to mobility: mobile agents perceive farther.
    #[serde(default = "default_sensing_range_coefficient")]
    pub sensing_range_coefficient: f32,
    pub reproduction_energy_threshold: f32,
    /// Minimum reproductive-nutrient earmark an agent must hold to reproduce.
    /// Mirrors `reproduction_energy_threshold` for the nutrient currency: the
    /// reproduction gate reads `repro_nutrient`, never the free store, so growth
    /// can never pin an agent on this gate.
    #[serde(default = "default_reproduction_nutrient_threshold")]
    pub reproduction_nutrient_threshold: f32,
    pub mutation_rate: f32,
    pub mutation_magnitude: f32,
    #[serde(alias = "contact_radius")]
    pub contact_range_coefficient: f32,
    pub world_extent: f32,
    pub initial_population_size: u32,
    pub light_competition_radius: f32,
    pub photo_maintenance_cost: f32,
    pub heterotrophy_maintenance_cost: f32,
    /// Total nutrient seeded into the world's spatial grid at genesis. A value
    /// of 0.0 is a silent kill switch — no nutrient exists, so no agent clears
    /// the reproduction nutrient gate and the nutrient economy never starts.
    /// Required (no serde default) so an under-specified recipe fails loudly
    /// rather than fabricating a corpse-world. (issue #327)
    pub initial_nutrient_pool: f32,
    /// Fraction of surplus reserve converted to structure each tick (0.0–1.0).
    /// A value of 0.0 is a silent kill switch — no agent builds structure, so
    /// biomass, carcass energy, and the embodiment subsystem all go dark.
    /// Required (no serde default) so an under-specified recipe fails loudly.
    /// (issue #327)
    pub growth_efficiency: f32,
    #[serde(default = "default_wear_rate")]
    pub wear_rate: f32,
    #[serde(default = "default_wear_degradation_steepness")]
    pub wear_degradation_steepness: f32,
    /// Legacy field retained for backward-compatible JSON parsing.
    /// Somatic maintenance is now derived from kappa allocation.
    #[serde(default = "default_somatic_maintenance_cost_coefficient")]
    pub somatic_maintenance_cost_coefficient: f32,
    #[serde(default = "default_use_wear_rate")]
    pub use_wear_rate: f32,
    #[serde(default = "default_structure_maintenance_coefficient")]
    pub structure_maintenance_coefficient: f32,
    #[serde(default = "default_repair_decay")]
    pub repair_decay: f32,
    /// Base nutrient-to-energy ratio per unit structure.
    #[serde(default = "default_base_nutrient_ratio")]
    pub base_nutrient_ratio: f32,
    /// How much each unit of specification investment adds to the nutrient ratio.
    #[serde(default = "default_specification_nutrient_coefficient")]
    pub specification_nutrient_coefficient: f32,
    /// Trait-space distance threshold for sexual reproduction compatibility.
    /// A world parameter, not a per-agent trait. Speciation emerges when clusters
    /// diverge beyond this fixed threshold.
    #[serde(default = "default_reproductive_compatibility_distance")]
    pub reproductive_compatibility_distance: f32,
    /// Per-tick energy cost of maintaining mobility machinery, paid whether or not
    /// the agent moves. Distinct from movement_cost_coefficient (per-distance cost).
    #[serde(default)]
    pub mobility_maintenance_cost: f32,
    /// Exponent applied to each specification trait before multiplying by its
    /// maintenance cost coefficient.  Values > 1 make costs superlinear,
    /// enforcing the specialist-generalist trade-off.
    #[serde(default = "default_maintenance_cost_exponent")]
    pub maintenance_cost_exponent: f32,
    /// Cell size for the spatial nutrient grid. Nutrient is distributed across
    /// a grid of cells; co-located agents share their cell's pool proportionally.
    #[serde(default = "default_nutrient_grid_cell_size")]
    pub nutrient_grid_cell_size: f32,
    /// Multiplier applied to per-tick metabolic cost to compute the retention
    /// buffer in the grow phase. Surplus available for kappa-allocation is
    /// `reserve - retention`, where `retention = metabolic_cost * multiplier`.
    /// Default 2.0 preserves historical behaviour.
    #[serde(default = "default_growth_retention_multiplier")]
    pub growth_retention_multiplier: f32,
    /// Reserve mobilisation rate `f` (flow 9): the fraction of the above-buffer
    /// reserve excess that is mobilised — and so split by kappa into soma and
    /// reproductive allocation — each tick. `mobilised = f * (reserve - buffer)`;
    /// the remainder stays in reserve to be mobilised on later ticks. This is
    /// DEB's energy conductance: a large reserve is drawn down as a *rate*, never
    /// the whole stock in one step. The bounded rate makes reserve a feast-famine
    /// buffer — it lets a consumer (discrete-meal income) survive the gaps between
    /// meals instead of having its provisioning liquidated on its first well-fed
    /// tick. Default 1.0 is the degenerate one-tick liquidation (`mobilised =
    /// reserve - buffer`) — a no-op against existing recipes; search moves to
    /// `f < 1` where buffering produces more viable consumer ecologies.
    #[serde(default = "default_reserve_mobilisation_rate")]
    pub reserve_mobilisation_rate: f32,
    /// Fraction of each offspring's per-offspring energy share (after
    /// reproduction_efficiency loss) that is committed to structure rather
    /// than reserve at birth. The structure commitment is converted from
    /// energy via `growth_efficiency` (same lossy conversion as in-life
    /// growth), with the unconverted remainder dissipated as heat. The
    /// remaining `(1 - offspring_structure_fraction)` share becomes the
    /// newborn's starting reserve. Default 0.2 leaves most of the
    /// investment as metabolic fuel while ensuring newborns are not
    /// degenerately structure-zero at tick 0.
    #[serde(default = "default_offspring_structure_fraction")]
    pub offspring_structure_fraction: f32,
    /// Per-tick energy cost of maintaining asexual-reproduction machinery, paid
    /// in the metabolise phase whether or not the trait fires. Charged
    /// superlinearly via `maintenance_cost_exponent`, like the other
    /// maintenance costs. This is deliberately the only reproduction trait
    /// with a standing maintenance cost: it realises "machinery, not fallback"
    /// economically, keeping a directional selection gradient on
    /// `asexual_propensity` alive even when the trait rarely fires — without
    /// any threshold or gate.
    #[serde(default = "default_asexual_propensity_maintenance_cost")]
    pub asexual_propensity_maintenance_cost: f32,
    /// Coefficient on the dispersal propagule cost charged **at a reproduction
    /// event** (not per tick). At a reproduction event, a fraction
    /// `(coefficient * dispersal^exponent)` of the post-efficiency reproductive
    /// energy budget is spent building propagule structures (spores, plumes,
    /// fruit) before the remainder is divided among offspring — so higher
    /// dispersal leaves less to provision each offspring. The spent energy
    /// dissipates as heat (energy conserved). This is the dispersal analogue of
    /// the fecundity quantity/quality trade-off, expressed only when the agent
    /// reproduces. Default 0.0 disables the cost (backward-compatible).
    #[serde(default)]
    pub dispersal_propagule_cost_coefficient: f32,
    /// Exponent applied to the dispersal trait in the propagule cost. Values > 1
    /// make the cost superlinear (anti-generalist economics): broadcasting widely
    /// is disproportionately expensive. Default 2.0.
    #[serde(default = "default_dispersal_propagule_cost_exponent")]
    pub dispersal_propagule_cost_exponent: f32,
    /// Coefficient on the dispersal contribution to sexual **mate-finding reach**.
    /// Mate-search reach = `effective_mobility * sensing_range_coefficient +
    /// dispersal * dispersal_reach_coefficient`. This lets a sessile (mobility-0)
    /// agent broadcast gametes far enough to pair with a compatible neighbour:
    /// dispersal is the sessile solution to mate-finding, mirroring how it scatters
    /// offspring. Reach gates eligibility only — it does not move offspring (that
    /// remains governed by the dispersal trait at the placement step). Unlike the
    /// mobility term, dispersal does not wear, so this contribution is age-stable.
    /// Default 0.0 disables it (backward-compatible: existing recipes keep the
    /// pure-mobility reach).
    #[serde(default)]
    pub dispersal_reach_coefficient: f32,
    /// Coefficient on the **structure** contribution to consumption (feeding)
    /// reach. Consumption reach = `eff_heterotrophy * (contact_range_coefficient
    /// + body_reach_coefficient * sqrt(structure))`. Feeding reach, like
    /// reproductive reach, has two physical solutions — move the organism
    /// (mobility/locomotion, folded into the contact-range term) or extend the
    /// body through the substrate. A sessile heterotroph (a mycelium) forages by
    /// growing its body *through* the medium: the body is the feeding organ, so a
    /// larger body touches more. `sqrt(structure)` is used because a disc's
    /// radius scales with the square root of its area/mass — sublinear, so reach
    /// does not run away with body size. The whole reach is modulated by
    /// `eff_heterotrophy`, so a large autotroph's bulk (heterotrophy ~ 0) gains
    /// no foraging reach while a growing mycelium does. Default 0.0 disables it
    /// (backward-compatible: existing recipes keep the pure contact-range reach).
    #[serde(default)]
    pub body_reach_coefficient: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InitialDistribution {
    pub mean_traits: TraitVector,
    pub trait_covariance: f32,
    pub initial_cluster_count: u32,
    pub initial_energy_per_agent: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentSpec {
    #[serde(
        deserialize_with = "deserialize_position",
        serialize_with = "serialize_position"
    )]
    pub position: (f32, f32),
    /// Starting reserve for this agent. Accepts "energy" in JSON for backward compatibility.
    #[serde(alias = "energy")]
    pub reserve: f32,
    pub traits: TraitVector,
    #[serde(default)]
    pub nutrient: f32,
}

/// A standing carcass seeded directly into the world from a `WorldRecipe`
/// (issue #311). Mirrors `AgentSpec` so the recipe format stays decoupled from
/// the runtime `Carcass` type: it carries the carcass's position, embodied
/// energy, nutrient, and the trait vector of the agent it "came from" (which
/// governs distance-dependent trophic efficiency for whoever decomposes it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CarcassSpec {
    #[serde(
        deserialize_with = "deserialize_position",
        serialize_with = "serialize_position"
    )]
    pub position: (f32, f32),
    /// Embodied biomass energy of the carcass (mirrors a dead agent's structure).
    pub energy: f32,
    pub traits: TraitVector,
    #[serde(default)]
    pub nutrient: f32,
}

/// First id assigned to seeded carcasses. Runtime carcasses reuse the dead
/// agent's id (in the `0..next_agent_id` range, which only grows), and seeded
/// agents take `0..pop_size`. Seeding carcass ids from the very top of the u64
/// range keeps them permanently disjoint from any current or future agent id,
/// so a seeded carcass can never be confused with a living agent or a
/// runtime-generated carcass (issue #311).
const SEEDED_CARCASS_ID_BASE: u64 = u64::MAX / 2;

fn deserialize_position<'de, D>(deserializer: D) -> Result<(f32, f32), D::Error>
where
    D: serde::Deserializer<'de>,
{
    let arr: [f32; 2] = Deserialize::deserialize(deserializer)?;
    Ok((arr[0], arr[1]))
}

fn serialize_position<S>(pos: &(f32, f32), serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(2))?;
    seq.serialize_element(&pos.0)?;
    seq.serialize_element(&pos.1)?;
    seq.end()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldRecipe {
    pub parameters: WorldParameters,
    #[serde(default)]
    pub initial_distribution: Option<InitialDistribution>,
    #[serde(default)]
    pub agents: Option<Vec<AgentSpec>>,
    /// Standing carcasses seeded directly into the world (issue #311), so a
    /// scenario can place a decomposer on a detrital deposit at tick 0 rather
    /// than waiting for agents to die. Only honoured on the `agents` branch.
    #[serde(default)]
    pub carcasses: Option<Vec<CarcassSpec>>,
    pub max_ticks: u64,
}

/// Number of functional traits that accumulate somatic wear.
/// Indices: 0=photosynthetic_absorption, 1=heterotrophy, 2=mobility.
pub const FUNCTIONAL_TRAIT_COUNT: usize = 3;

#[derive(Clone, Debug)]
pub struct Agent {
    pub id: u64,
    pub position: (f32, f32),
    pub reserve: f32,
    pub structure: f32,
    /// Developmental high-water mark: `max(structure ever held)`. The death
    /// threshold is a *fraction of peak structure* (world-rules #9), so this is
    /// the body the agent measures its own structural loss against. It is
    /// physiological state — DEB's maximum structural length — not analysis or
    /// classification state (ADR-0001): the stepper legitimately owns it.
    /// Invariant: `peak_structure >= structure`, maintained at birth/seeding
    /// (= initial structure) and after the growth flow (the only place structure
    /// increases). Structure only ever decreases via grazing/metabolism, so no
    /// peak update is needed there.
    pub peak_structure: f32,
    pub nutrient: f32,
    pub traits: TraitVector,
    /// Per-functional-trait somatic wear accumulation.
    pub wear: [f32; FUNCTIONAL_TRAIT_COUNT],
    /// Reproduction reserve: accumulates (1-kappa) fraction of surplus each tick.
    /// Reproduction draws from this, not from reserve.
    pub repro_reserve: f32,
    /// Reproductive-nutrient earmark: accumulates the (1-kappa) share of each
    /// tick's nutrient uptake. Off-limits to growth. Reproduction gates on and
    /// donates from this, mirroring `repro_reserve` for energy.
    pub repro_nutrient: f32,
}

/// Maps a functional trait index (0–2) to its position in the TraitVector.
/// 0=photosynthetic_absorption, 1=heterotrophy, 2=mobility.
pub const FUNCTIONAL_TRAIT_INDICES: [usize; FUNCTIONAL_TRAIT_COUNT] = [0, 1, 2];

impl Agent {
    /// Construct an agent at birth/seeding, establishing the peak-structure
    /// high-water mark at the initial structure so the agent is born above its
    /// own (peak-relative) death threshold. wear starts at zero and the
    /// reproductive earmarks start empty — the universal birth state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u64,
        position: (f32, f32),
        reserve: f32,
        structure: f32,
        nutrient: f32,
        traits: TraitVector,
    ) -> Self {
        Agent {
            id,
            position,
            reserve,
            structure,
            peak_structure: structure,
            nutrient,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        }
    }

    /// Record the agent's structure into its peak high-water mark. Call after
    /// any flow that *increases* structure (growth). Cheap and idempotent.
    pub fn record_peak_structure(&mut self) {
        if self.structure > self.peak_structure {
            self.peak_structure = self.structure;
        }
    }

    /// Total energy held by this agent (reserve + structure + repro_reserve).
    pub fn energy(&self) -> f32 {
        self.reserve + self.structure + self.repro_reserve
    }

    /// Nutrient bound in this agent's structure: matter locked into the body at
    /// the stoichiometric demand (`structure × demand`), released only when the
    /// structure is grazed or returned to a carcass at death (ADR-0003).
    pub fn bound_nutrient(&self, params: &WorldParameters) -> f32 {
        stoichiometric_demand(&self.traits, self.structure, params)
    }

    /// Total nutrient held by this agent: the free store, the
    /// reproductive-nutrient earmark, and the nutrient bound in its structure.
    /// The conserved nutrient quantity for the ledger and every death path.
    pub fn nutrient_total(&self, params: &WorldParameters) -> f32 {
        self.nutrient + self.repro_nutrient + self.bound_nutrient(params)
    }

    /// Credit acquired nutrient, split by kappa, mirroring the energy allocation.
    /// The kappa share feeds the free store; the (1 - kappa) share feeds the
    /// reproductive-nutrient earmark. The split is route-agnostic (ADR-0004): it
    /// applies to every unit of nutrient acquired — autotrophic pool uptake
    /// (flow 2) and the nutrient ingested by consumption (flow 3) alike — so a
    /// heterotroph funds reproduction from ingested biomass exactly as a producer
    /// funds it from absorbed nutrient.
    pub fn credit_nutrient(&mut self, amount: f32) {
        let kappa = self.traits.kappa.clamp(0.0, 1.0);
        self.nutrient += amount * kappa;
        self.repro_nutrient += amount * (1.0 - kappa);
    }

    /// Returns the nominal trait value for a given functional trait index (0–5).
    pub fn nominal_functional_trait(&self, ft_index: usize) -> f32 {
        self.traits.get(FUNCTIONAL_TRAIT_INDICES[ft_index])
    }

    /// Returns the effective (wear-degraded) trait value using the given steepness k.
    /// effective = nominal * exp(-k * wear)
    pub fn effective_trait_with_steepness(&self, ft_index: usize, k: f32) -> f32 {
        let nominal = self.nominal_functional_trait(ft_index);
        nominal * (-k * self.wear[ft_index]).exp()
    }

    /// Returns the effective trait value using a default steepness of 1.0.
    /// For production use, prefer the World method that supplies the world parameter.
    pub fn effective_trait(&self, ft_index: usize) -> f32 {
        self.effective_trait_with_steepness(ft_index, 1.0)
    }

    /// Returns a TraitVector with functional traits degraded by wear.
    /// Behavioural traits (kappa, fecundity, asexual_propensity, dispersal) are passed through unchanged.
    pub fn effective_traits(&self, k: f32) -> TraitVector {
        let mut t = self.traits;
        for ft in 0..FUNCTIONAL_TRAIT_COUNT {
            let effective = self.effective_trait_with_steepness(ft, k);
            t.set(FUNCTIONAL_TRAIT_INDICES[ft], effective);
        }
        t
    }
}

pub struct Carcass {
    pub id: u64,
    pub position: (f32, f32),
    pub energy: f32,
    pub nutrient: f32,
    /// Original agent's trait vector, used for distance-dependent trophic efficiency.
    pub traits: TraitVector,
}

/// Complexity-dependent structural fragility — the *fraction of peak structure*
/// below which structural damage is fatal (world-rules #9, energy-flow.md:103).
///
/// Returns the normalised entropy of the L1-normalised trait vector, in [0, 1):
/// - Specialist (budget concentrated in few traits) → ~0 (robust: a moss at
///   half its mass is still a functioning moss)
/// - Generalist (budget spread across many traits) → ~1 (fragile: complex,
///   interdependent systems cannot sustain partial loss)
///
/// This is the *shape* of the trait vector only — it is size-independent. The
/// absolute death threshold is this fragility scaled by the agent's own
/// peak-structure high-water mark (see `death_threshold`): death-by-structural-
/// loss is catabolising your own body past a survivable *fraction* of what you
/// built, not falling below an absolute floor a newborn never had a chance to
/// clear.
pub fn structural_fragility(traits: &TraitVector) -> f32 {
    let mut values = [0.0_f32; TraitVector::NUM_DIMS];
    let mut sum = 0.0_f32;
    for dim in 0..TraitVector::NUM_DIMS {
        let v = traits.get(dim).max(0.0);
        values[dim] = v;
        sum += v;
    }
    if sum < 1e-10 {
        return 0.0;
    }
    // Normalised entropy: H / ln(N)
    let n = TraitVector::NUM_DIMS as f32;
    let ln_n = n.ln();
    let mut entropy = 0.0_f32;
    for &v in &values {
        let p = v / sum;
        if p > 1e-10 {
            entropy -= p * p.ln();
        }
    }
    entropy / ln_n
}

/// Complexity- and size-dependent death threshold: the structure level below
/// which an agent dies, relative to the body it actually built.
///
/// `threshold = structural_fragility(traits) × peak_structure` — a *fraction of
/// peak structure* (world-rules #9). A newborn or seed has
/// `peak_structure == birth structure` and `fragility < 1`, so it is born above
/// its own threshold regardless of size: small is not the same as dying.
pub fn death_threshold(traits: &TraitVector, peak_structure: f32) -> f32 {
    structural_fragility(traits) * peak_structure
}

/// Stoichiometric nutrient demand: structure × trait-derived ratio.
///
/// `demand = structure × (base_nutrient_ratio + specification_nutrient_coefficient × specification_sum)`
/// where `specification_sum = autotrophy + heterotrophy + mobility`.
///
/// Larger agents need more nutrient. More specification investment means
/// proportionally more nutrient per unit biomass.
pub fn stoichiometric_demand(
    traits: &TraitVector,
    structure: f32,
    params: &WorldParameters,
) -> f32 {
    let specification_sum = traits.photosynthetic_absorption.max(0.0)
        + traits.heterotrophy.max(0.0)
        + traits.mobility.max(0.0);
    let ratio =
        params.base_nutrient_ratio + params.specification_nutrient_coefficient * specification_sum;
    structure * ratio
}

/// Distance-dependent trophic transfer efficiency.
///
/// `efficiency = base_trophic_efficiency × exp(−trophic_distance_decay × trait_distance)`
///
/// Biochemically similar agents (close in trait space) convert more efficiently.
/// At distance 0 the efficiency equals `base_trophic_efficiency`.
pub fn trophic_transfer_efficiency(
    consumer: &TraitVector,
    target: &TraitVector,
    params: &WorldParameters,
) -> f32 {
    let distance = consumer.distance(target);
    params.base_trophic_efficiency * (-params.trophic_distance_decay * distance).exp()
}

#[allow(dead_code)]
pub struct World {
    params: WorldParameters,
    agents: Vec<Agent>,
    carcasses: Vec<Carcass>,
    dissipated_energy: f32,
    total_solar_input: f32,
    nutrient_grid: spatial::NutrientGrid,
    seed: u64,
    rng: ChaCha8Rng,
    tick: u64,
    last_tick_births: usize,
    last_tick_deaths: usize,
    next_agent_id: u64,
    event_log: event::EventLog,
    next_seq: u64,
    ledger: energy_ledger::EnergyLedger,
    nutrient_ledger: nutrient_ledger::NutrientLedger,
}

impl World {
    pub fn new(params: WorldParameters, distribution: InitialDistribution, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let extent = params.world_extent;
        let pop_size = params.initial_population_size as usize;
        let n_clusters = (distribution.initial_cluster_count as usize).max(1);

        let pos_dist = rand::distr::Uniform::new(-extent / 2.0, extent / 2.0).unwrap();
        let trait_dist = Normal::new(0.0_f32, distribution.trait_covariance).unwrap();

        let mean = &distribution.mean_traits;
        let trophic_total = mean.photosynthetic_absorption + mean.heterotrophy;

        let cluster_centroids: Vec<TraitVector> = (0..n_clusters)
            .map(|c| {
                let (photo, hetero) = if n_clusters == 1 || trophic_total <= 0.0 {
                    (mean.photosynthetic_absorption, mean.heterotrophy)
                } else {
                    match c % 2 {
                        0 => (trophic_total, 0.0),
                        _ => (0.0, trophic_total),
                    }
                };
                TraitVector {
                    photosynthetic_absorption: photo,
                    heterotrophy: hetero,
                    mobility: mean.mobility,
                    kappa: mean.kappa,
                    fecundity: mean.fecundity,
                    asexual_propensity: mean.asexual_propensity,
                    dispersal: mean.dispersal,
                }
            })
            .collect();

        let (seed_reserve, seed_structure, seed_heat) =
            provision_initial_reserve_structure(distribution.initial_energy_per_agent, &params);
        let agents: Vec<Agent> = (0..pop_size)
            .map(|id| {
                let x = pos_dist.sample(&mut rng);
                let y = pos_dist.sample(&mut rng);
                let centroid = &cluster_centroids[id % n_clusters];
                Agent {
                    id: id as u64,
                    position: (x, y),
                    reserve: seed_reserve,
                    structure: seed_structure,
                    // Seed structure is the high-water mark: a fresh seed is
                    // born above its own peak-relative death threshold (#312).
                    peak_structure: seed_structure,
                    nutrient: 0.0,
                    traits: TraitVector {
                        photosynthetic_absorption: centroid.photosynthetic_absorption
                            + trait_dist.sample(&mut rng),
                        heterotrophy: centroid.heterotrophy + trait_dist.sample(&mut rng),
                        mobility: centroid.mobility + trait_dist.sample(&mut rng),
                        kappa: (centroid.kappa + trait_dist.sample(&mut rng)).clamp(0.0, 1.0),
                        fecundity: centroid.fecundity + trait_dist.sample(&mut rng),
                        asexual_propensity: (centroid.asexual_propensity
                            + trait_dist.sample(&mut rng))
                        .clamp(0.0, 1.0),
                        dispersal: (centroid.dispersal + trait_dist.sample(&mut rng)).max(0.0),
                    },
                    wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                    repro_reserve: 0.0,
                    repro_nutrient: 0.0,
                }
            })
            .collect();

        let mut nutrient_grid = spatial::NutrientGrid::new(
            params.world_extent,
            params.nutrient_grid_cell_size,
            params.initial_nutrient_pool,
        );
        // Seeded structure binds nutrient (ADR-0003 embodiment): draw each
        // agent's bound birth nutrient from the available pool at its location so
        // total system nutrient at creation equals the initial pool — nutrient is
        // not conjured into the living system.
        bind_seed_structure_nutrient(&agents, &mut nutrient_grid, &params);
        let initial_dissipation = seed_heat * pop_size as f32;
        Self {
            params,
            agents,
            carcasses: Vec::new(),
            dissipated_energy: initial_dissipation,
            total_solar_input: 0.0,
            nutrient_grid,
            seed,
            rng,
            tick: 0,
            last_tick_births: 0,
            last_tick_deaths: 0,
            next_agent_id: pop_size as u64,
            event_log: event::EventLog::new(),
            next_seq: 0,
            ledger: energy_ledger::EnergyLedger::new(),
            nutrient_ledger: nutrient_ledger::NutrientLedger::new(),
        }
    }

    pub fn from_recipe(recipe: &WorldRecipe, seed: u64) -> Self {
        if let Some(ref agents) = recipe.agents {
            let params = recipe.parameters.clone();
            let rng = ChaCha8Rng::seed_from_u64(seed);
            let pop_size = agents.len();
            let mut initial_dissipation = 0.0_f32;
            let sim_agents: Vec<Agent> = agents
                .iter()
                .enumerate()
                .map(|(i, spec)| {
                    let (reserve, structure, heat) =
                        provision_initial_reserve_structure(spec.reserve, &params);
                    initial_dissipation += heat;
                    Agent {
                        id: i as u64,
                        position: spec.position,
                        reserve,
                        structure,
                        peak_structure: structure,
                        nutrient: spec.nutrient,
                        traits: spec.traits,
                        wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                        repro_reserve: 0.0,
                        repro_nutrient: 0.0,
                    }
                })
                .collect();
            let mut nutrient_grid = spatial::NutrientGrid::new(
                params.world_extent,
                params.nutrient_grid_cell_size,
                params.initial_nutrient_pool,
            );
            // Seeded structure binds nutrient (ADR-0003): draw it from the pool so
            // total system nutrient at creation equals the initial pool.
            bind_seed_structure_nutrient(&sim_agents, &mut nutrient_grid, &params);
            // Materialise any seeded carcasses (issue #311). Their ids come from a
            // high range disjoint from agent ids (current and future), so a seeded
            // carcass can never collide with a living agent or a runtime carcass.
            let seeded_carcasses: Vec<Carcass> = recipe
                .carcasses
                .iter()
                .flatten()
                .enumerate()
                .map(|(i, spec)| Carcass {
                    id: SEEDED_CARCASS_ID_BASE + i as u64,
                    position: spec.position,
                    energy: spec.energy,
                    nutrient: spec.nutrient,
                    traits: spec.traits,
                })
                .collect();
            Self {
                params,
                agents: sim_agents,
                carcasses: seeded_carcasses,
                dissipated_energy: initial_dissipation,
                total_solar_input: 0.0,
                nutrient_grid,
                seed,
                rng,
                tick: 0,
                last_tick_births: 0,
                last_tick_deaths: 0,
                next_agent_id: pop_size as u64,
                event_log: event::EventLog::new(),
                next_seq: 0,
                ledger: energy_ledger::EnergyLedger::new(),
                nutrient_ledger: nutrient_ledger::NutrientLedger::new(),
            }
        } else if let Some(ref distribution) = recipe.initial_distribution {
            Self::new(recipe.parameters.clone(), distribution.clone(), seed)
        } else {
            panic!("WorldRecipe must have either 'agents' or 'initial_distribution'");
        }
    }

    pub fn add_agent(&mut self, mut agent: Agent) {
        agent.id = self.next_agent_id;
        self.next_agent_id += 1;
        self.agents.push(agent);
    }

    pub fn add_carcass(&mut self, carcass: Carcass) {
        self.carcasses.push(carcass);
    }

    pub fn step(&mut self) {
        use energy_ledger::EnergyEndpoint;

        let extent = self.params.world_extent;

        // Snapshot pre-tick state for energy ledger conservation verification
        let pre_agent_energy: std::collections::HashMap<u64, f32> = self
            .agents
            .iter()
            .map(|a| (a.id, a.reserve + a.structure + a.repro_reserve))
            .collect();
        let pre_carcass_energy: std::collections::HashMap<u64, f32> =
            self.carcasses.iter().map(|c| (c.id, c.energy)).collect();

        // Snapshot pre-tick nutrient per pool for the nutrient ledger.
        // Nutrient is a closed resource: the ledger verifies that the total
        // across all pools (grid cells + living agents + carcasses) at tick
        // end equals the total at tick start.
        let pre_grid_nutrient: f32 = self.nutrient_grid.total();
        let pre_agent_nutrient: f32 = self
            .agents
            .iter()
            .map(|a| a.nutrient_total(&self.params))
            .sum();
        let pre_carcass_nutrient: f32 = self.carcasses.iter().map(|c| c.nutrient).sum();

        // Snapshot trait vectors for ledger efficiency calculations
        let pre_agent_traits: std::collections::HashMap<u64, TraitVector> =
            self.agents.iter().map(|a| (a.id, a.traits)).collect();
        let pre_carcass_traits: std::collections::HashMap<u64, TraitVector> =
            self.carcasses.iter().map(|c| (c.id, c.traits)).collect();

        // Build spatial grid once at start of tick
        let cell_size = self.params.light_competition_radius.max(1.0);
        let mut grid = crate::spatial::SpatialGrid::new(extent, cell_size);
        for (i, a) in self.agents.iter().enumerate() {
            grid.insert(i as u64, a.position);
        }

        // Autonomous phases in order
        let mut events = Vec::new();

        // 1. Photosynthesise
        let photo_events = phase::photosynthesise(&mut self.agents, &grid, &self.params);
        let solar_this_tick: f32 = photo_events.iter().map(|e| e.energy_delta).sum();
        self.total_solar_input += solar_this_tick;
        events.extend(photo_events);

        // 2. Absorb nutrients
        let nutrient_events =
            phase::absorb_nutrients(&mut self.agents, &mut self.nutrient_grid, &self.params);
        events.extend(nutrient_events);

        // 3. Metabolise
        let (metab_events, dissipated) = phase::metabolise(&mut self.agents, &self.params);
        self.dissipated_energy += dissipated;
        events.extend(metab_events);

        // 4. Grow
        let (grow_events, grow_dissipated) = phase::grow(&mut self.agents, &self.params);
        self.dissipated_energy += grow_dissipated;
        events.extend(grow_events);

        // 5. Resolve drains (coordinated pass 1)
        let drain_result = phase::resolve_drains(
            &mut self.agents,
            &mut self.carcasses,
            &grid,
            &self.params,
            &mut self.nutrient_grid,
        );
        self.dissipated_energy += drain_result.dissipated;
        events.extend(drain_result.events);

        // Mark deaths from drain resolution
        let drain_dead_ids: std::collections::HashSet<u64> =
            drain_result.dead_agents.iter().copied().collect();
        for agent in self.agents.iter_mut() {
            if drain_dead_ids.contains(&agent.id) {
                // Dissipate remaining reserve and repro_reserve (not captured by carcass)
                self.dissipated_energy += agent.reserve.max(0.0) + agent.repro_reserve.max(0.0);
                events.push(event::Event {
                    tick: 0,
                    seq: 0,
                    kind: event::EventKind::Died,
                    source: agent.id,
                    target: None,
                    energy_delta: 0.0,
                    position: Some(agent.position),
                    target_was_carcass: false,
                });
                agent.reserve = 0.0; // mark for removal
                agent.repro_reserve = 0.0;
            }
        }
        // Add new carcasses from drain kills (not in this tick's spatial grid)
        for c in drain_result.new_carcasses {
            self.carcasses.push(c);
        }

        // Remove drain-killed agents before further phases
        self.agents.retain(|a| !drain_dead_ids.contains(&a.id));

        // 6. Resolve reproduction (coordinated pass 2)
        // Rebuild grid after removing dead agents (indices changed)
        let mut repro_grid = crate::spatial::SpatialGrid::new(extent, cell_size);
        for (i, a) in self.agents.iter().enumerate() {
            repro_grid.insert(i as u64, a.position);
        }
        let repro_result = phase::resolve_reproduction(
            &mut self.agents,
            &drain_dead_ids,
            &repro_grid,
            &self.params,
            self.seed,
            self.tick,
        );
        self.dissipated_energy += repro_result.dissipated;
        self.last_tick_births = repro_result.offspring.len();
        events.extend(repro_result.events);
        // Add offspring to world with unique IDs
        for mut child in repro_result.offspring {
            child.id = self.next_agent_id;
            self.next_agent_id += 1;
            self.agents.push(child);
        }

        // 7. Move agents. Movement is the final repositioning phase: it runs
        // after all energy-affecting phases (so movement energy is bounded by
        // what remains) but before wear and the death check, so this tick's own
        // movement is charged as mobility use-wear within the same tick. Wear and
        // the death check are autonomous (they don't read the spatial grid), so
        // positions staying stable across them is unaffected by moving first.
        let mut move_grid = crate::spatial::SpatialGrid::new(extent, cell_size);
        for (i, a) in self.agents.iter().enumerate() {
            move_grid.insert(i as u64, a.position);
        }
        let move_result = phase::move_agents(
            &mut self.agents,
            &self.carcasses,
            &move_grid,
            &self.params,
            self.seed,
            self.tick,
        );
        self.dissipated_energy += move_result.dissipated;
        // move_result.move_distance is aligned by index with self.agents at move
        // time; no agents are added or removed between move and wear, so the
        // index -> id mapping is stable.
        let move_distance_by_id: std::collections::HashMap<u64, f32> = self
            .agents
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                let dist = move_result.move_distance[i];
                (dist > 0.0).then_some((a.id, dist))
            })
            .collect();
        events.extend(move_result.events);

        // 8. Wear: collect per-agent usage from earlier phases
        let mut usage_data: std::collections::HashMap<u64, [f32; FUNCTIONAL_TRAIT_COUNT]> =
            std::collections::HashMap::new();
        // Autotrophy usage: energy captured via photosynthesis
        for ev in events
            .iter()
            .filter(|e| e.kind == event::EventKind::Photosynthesized)
        {
            let entry = usage_data
                .entry(ev.source)
                .or_insert([0.0; FUNCTIONAL_TRAIT_COUNT]);
            entry[0] += ev.energy_delta;
        }
        // Heterotrophy usage: energy drained via consumption
        for ev in events
            .iter()
            .filter(|e| e.kind == event::EventKind::Consumed)
        {
            let entry = usage_data
                .entry(ev.source)
                .or_insert([0.0; FUNCTIONAL_TRAIT_COUNT]);
            entry[1] += ev.energy_delta;
        }
        // Mobility usage: distance moved during this tick's movement phase.
        for (&id, &dist) in &move_distance_by_id {
            let entry = usage_data
                .entry(id)
                .or_insert([0.0; FUNCTIONAL_TRAIT_COUNT]);
            entry[2] += dist;
        }
        let wear_events = phase::apply_wear(&mut self.agents, &self.params, &usage_data);
        events.extend(wear_events);

        // 9. Check death thresholds
        let (death_events, threshold_carcasses, death_dissipated) =
            phase::check_death_thresholds(&mut self.agents, &self.params);
        let threshold_deaths = threshold_carcasses.len();
        self.dissipated_energy += death_dissipated;
        events.extend(death_events);
        for c in threshold_carcasses {
            self.carcasses.push(c);
        }

        // Remove dead agents (those with reserve <= 0 or structure below threshold)
        self.agents.retain(|a| a.reserve > 0.0);

        // --- Energy ledger: record all flows for conservation verification ---
        // Built from event data and state snapshots, after all phases complete.
        self.ledger.clear();

        let post_agent_energy: std::collections::HashMap<u64, f32> = self
            .agents
            .iter()
            .map(|a| (a.id, a.reserve + a.structure + a.repro_reserve))
            .collect();
        let post_carcass_energy: std::collections::HashMap<u64, f32> =
            self.carcasses.iter().map(|c| (c.id, c.energy)).collect();

        // Collect all agent IDs that exist at start or end of tick
        let mut all_agent_ids = std::collections::HashSet::new();
        for &id in pre_agent_energy.keys() {
            all_agent_ids.insert(id);
        }
        for &id in post_agent_energy.keys() {
            all_agent_ids.insert(id);
        }
        let mut all_carcass_ids = std::collections::HashSet::new();
        for &id in pre_carcass_energy.keys() {
            all_carcass_ids.insert(id);
        }
        for &id in post_carcass_energy.keys() {
            all_carcass_ids.insert(id);
        }

        // Record endowments (pre-tick energy for entities that existed at tick start)
        for (&id, &energy) in &pre_agent_energy {
            if energy > 0.0 {
                self.ledger
                    .record(EnergyEndpoint::Endowment, EnergyEndpoint::Agent(id), energy);
            }
        }
        for (&id, &energy) in &pre_carcass_energy {
            if energy > 0.0 {
                self.ledger.record(
                    EnergyEndpoint::Endowment,
                    EnergyEndpoint::Carcass(id),
                    energy,
                );
            }
        }

        // Record solar input per agent
        for ev in events
            .iter()
            .filter(|e| e.kind == event::EventKind::Photosynthesized)
        {
            self.ledger.record(
                EnergyEndpoint::SolarTap,
                EnergyEndpoint::Agent(ev.source),
                ev.energy_delta,
            );
        }

        // Record consumption flows (target -> consumer with trophic loss)
        for ev in events
            .iter()
            .filter(|e| e.kind == event::EventKind::Consumed)
        {
            let target_id = ev.target.unwrap();
            let drain = ev.energy_delta;
            let is_carcass = pre_carcass_energy.contains_key(&target_id)
                && !pre_agent_energy.contains_key(&target_id);
            // Compute distance-dependent trophic efficiency from trait vectors
            let consumer_traits = pre_agent_traits
                .get(&ev.source)
                .copied()
                .unwrap_or_else(|| {
                    // Offspring born this tick — look up from current agents
                    self.agents
                        .iter()
                        .find(|a| a.id == ev.source)
                        .map(|a| a.traits)
                        .unwrap_or_else(zero_traits)
                });
            let target_traits = if is_carcass {
                pre_carcass_traits
                    .get(&target_id)
                    .copied()
                    .unwrap_or_else(zero_traits)
            } else {
                pre_agent_traits
                    .get(&target_id)
                    .copied()
                    .unwrap_or_else(zero_traits)
            };
            let eff = trophic_transfer_efficiency(&consumer_traits, &target_traits, &self.params);
            let gained = drain * eff;
            let lost = drain - gained;
            let target_ep = if is_carcass {
                EnergyEndpoint::Carcass(target_id)
            } else {
                EnergyEndpoint::Agent(target_id)
            };
            if gained > 0.0 {
                self.ledger
                    .record(target_ep.clone(), EnergyEndpoint::Agent(ev.source), gained);
            }
            if lost > 0.0 {
                self.ledger
                    .record(target_ep, EnergyEndpoint::Dissipation, lost);
            }
        }

        // Record death transfers (agent -> carcass)
        for &id in &all_carcass_ids {
            if !pre_carcass_energy.contains_key(&id) && pre_agent_energy.contains_key(&id) {
                // New carcass from a pre-tick agent that died
                let carcass_energy = post_carcass_energy.get(&id).copied().unwrap_or(0.0);
                if carcass_energy > 0.0 {
                    self.ledger.record(
                        EnergyEndpoint::Agent(id),
                        EnergyEndpoint::Carcass(id),
                        carcass_energy,
                    );
                }
            }
        }

        // Record offspring birth endowments
        for (&id, &energy) in &post_agent_energy {
            if !pre_agent_energy.contains_key(&id) && energy > 0.0 {
                self.ledger
                    .record(EnergyEndpoint::Endowment, EnergyEndpoint::Agent(id), energy);
            }
        }

        // For each pre-tick agent: compute residual dissipation
        // (everything not accounted for by consumption, death, or retained energy)
        for (&id, &_pre_energy) in &pre_agent_energy {
            let total_in = self.ledger.net_received(&EnergyEndpoint::Agent(id));
            let total_out = self.ledger.net_sent(&EnergyEndpoint::Agent(id));
            let post_energy = post_agent_energy.get(&id).copied().unwrap_or(0.0);
            let unaccounted = total_in - total_out - post_energy;
            if unaccounted > 1e-6 {
                self.ledger.record(
                    EnergyEndpoint::Agent(id),
                    EnergyEndpoint::Dissipation,
                    unaccounted,
                );
            }
        }

        // For pre-tick carcasses: compute residual dissipation
        for (&id, &_pre_energy) in &pre_carcass_energy {
            let total_in = self.ledger.net_received(&EnergyEndpoint::Carcass(id));
            let total_out = self.ledger.net_sent(&EnergyEndpoint::Carcass(id));
            let post_energy = post_carcass_energy.get(&id).copied().unwrap_or(0.0);
            let unaccounted = total_in - total_out - post_energy;
            if unaccounted > 1e-6 {
                self.ledger.record(
                    EnergyEndpoint::Carcass(id),
                    EnergyEndpoint::Dissipation,
                    unaccounted,
                );
            }
        }

        // --- Nutrient ledger: verify the closed-system conservation invariant ---
        // Nutrient cycles between three pool categories (grid, agents, carcasses)
        // and is never created or destroyed. The ledger endows each category
        // with its pre-tick total and reconciles the post-tick deltas, so any
        // net creation or destruction trips assert_balanced().
        let pre_pools = nutrient_ledger::PoolTotals {
            grid: pre_grid_nutrient,
            agents: pre_agent_nutrient,
            carcasses: pre_carcass_nutrient,
        };
        let post_pools = nutrient_ledger::PoolTotals {
            grid: self.nutrient_grid.total(),
            agents: self
                .agents
                .iter()
                .map(|a| a.nutrient_total(&self.params))
                .sum(),
            carcasses: self.carcasses.iter().map(|c| c.nutrient).sum(),
        };
        self.nutrient_ledger
            .build_from_pool_totals(pre_pools, post_pools);
        // Verify conservation eagerly in debug builds; in release the check is
        // compiled out, keeping the ledger an orthogonal, zero-cost wrapper
        // (mirrors the energy ledger's disable-in-release approach).
        #[cfg(debug_assertions)]
        self.nutrient_ledger.assert_balanced();

        self.last_tick_deaths = drain_dead_ids.len() + threshold_deaths;

        // Append events to log
        for mut ev in events {
            ev.tick = self.tick;
            ev.seq = self.next_seq;
            self.next_seq += 1;
            let _ = self.event_log.append(ev);
        }

        self.tick += 1;
    }

    pub fn params(&self) -> &WorldParameters {
        &self.params
    }

    pub fn params_mut(&mut self) -> &mut WorldParameters {
        &mut self.params
    }

    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    /// Test support (#376): deliberately permute the in-memory order of the
    /// `agents` slice without changing any agent's identity or state. The
    /// keyed-stateless RNG keys every stochastic outcome on stable agent id (and
    /// the symmetric ordered pair for sexual reproduction), and newborn ids are
    /// assigned in a canonical, world-state-derived order — so the trajectory
    /// must be invariant under this permutation. The trajectory-level shuffle
    /// test calls this between steps; a divergence means iteration order leaked
    /// back into determinism. The permutation is a deterministic rotate-by-`k`,
    /// so the test itself stays reproducible.
    pub fn permute_agent_order_for_test(&mut self, k: usize) {
        if !self.agents.is_empty() {
            let n = self.agents.len();
            // Reverse, then rotate by k: a non-cyclic permutation that changes
            // both adjacency and absolute position of every agent in the slice.
            self.agents.reverse();
            self.agents.rotate_left(k % n);
        }
    }

    pub fn carcasses(&self) -> &[Carcass] {
        &self.carcasses
    }

    pub fn nutrient_pool(&self) -> f32 {
        self.nutrient_grid.total()
    }

    pub fn nutrient_grid(&self) -> &spatial::NutrientGrid {
        &self.nutrient_grid
    }

    pub fn nutrient_grid_mut(&mut self) -> &mut spatial::NutrientGrid {
        &mut self.nutrient_grid
    }

    pub fn dissipated_energy(&self) -> f32 {
        self.dissipated_energy
    }

    /// Free (non-carcass-locked) energy: the living system's instantaneous
    /// energy stock — every living agent's reserve + structure + repro_reserve
    /// summed across the population. Energy locked in carcasses is excluded;
    /// it re-enters this stock only when a decomposer consumes the carcass.
    /// Instantaneous read of public state — the world stays history-free; the
    /// per-tick trend an evaluator needs is built by sampling this each tick.
    pub fn free_energy(&self) -> f32 {
        self.agents.iter().map(|a| a.energy()).sum()
    }

    /// Producer (autotroph) share of the living energy: the energy-weighted mean
    /// of each agent's autotrophy fraction `a = photo / (photo + hetero)`, where
    /// `photo = photosynthetic_absorption` and `hetero = heterotrophy`. An agent
    /// with `photo + hetero == 0` contributes `a = 0`. Zero when there are no
    /// agents or total energy is zero.
    ///
    /// Being a ratio it is scale-invariant — independent of population size or
    /// absolute energy. Sampled once per tick by callers, its detrended series
    /// carries the producer↔consumer rhythm the genesis oscillation descriptor
    /// reads (issue #392). Instantaneous read of public state; the world stays
    /// history-free, mirroring `free_energy`.
    pub fn producer_energy_share(&self) -> f32 {
        let mut weighted = 0.0_f32;
        let mut total = 0.0_f32;
        for agent in &self.agents {
            let e = agent.energy();
            let photo = agent.traits.photosynthetic_absorption;
            let hetero = agent.traits.heterotrophy;
            let denom = photo + hetero;
            let autotrophy = if denom > 0.0 { photo / denom } else { 0.0 };
            weighted += e * autotrophy;
            total += e;
        }
        if total > 0.0 { weighted / total } else { 0.0 }
    }

    /// The dead pool's share of the conserved system nutrient: carcass-locked
    /// nutrient over the total across grid, living agents, and carcasses. Zero
    /// when no nutrient is present anywhere. Total nutrient is conserved, so the
    /// fraction's trend isolates *where* the nutrient sits, not how much there is
    /// — a normalized, scenario-independent read. Sampled each tick by callers,
    /// it lets the genesis evaluator detect nutrient sequestered into carcasses
    /// that the living decomposer population cannot turn over (issue #342).
    /// Instantaneous read of public state; the world stays history-free.
    pub fn carcass_locked_nutrient_fraction(&self) -> f32 {
        let carcasses: f32 = self.carcasses.iter().map(|c| c.nutrient).sum();
        let agents: f32 = self
            .agents
            .iter()
            .map(|a| a.nutrient_total(&self.params))
            .sum();
        let total = self.nutrient_grid.total() + agents + carcasses;
        if total > 0.0 { carcasses / total } else { 0.0 }
    }

    pub fn total_solar_input(&self) -> f32 {
        self.total_solar_input
    }

    pub fn last_tick_births(&self) -> usize {
        self.last_tick_births
    }

    pub fn last_tick_deaths(&self) -> usize {
        self.last_tick_deaths
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn event_log(&self) -> &event::EventLog {
        &self.event_log
    }

    pub fn energy_ledger(&self) -> &energy_ledger::EnergyLedger {
        &self.ledger
    }

    pub fn nutrient_ledger(&self) -> &nutrient_ledger::NutrientLedger {
        &self.nutrient_ledger
    }

    /// Emit an event to the log. Retained for coordinated phases (not yet implemented).
    #[allow(dead_code)]
    fn emit(
        &mut self,
        kind: event::EventKind,
        source: u64,
        target: Option<u64>,
        energy_delta: f32,
        position: Option<(f32, f32)>,
    ) {
        let seq = self.next_seq;
        self.next_seq += 1;
        let _ = self.event_log.append(event::Event {
            tick: self.tick,
            seq,
            kind,
            source,
            target,
            energy_delta,
            position,
            target_was_carcass: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_traits() -> TraitVector {
        TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 0.0,
            mobility: 0.0,
            kappa: 0.0,
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        }
    }

    fn test_params() -> WorldParameters {
        WorldParameters {
            asexual_propensity_maintenance_cost: 0.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            dispersal_reach_coefficient: 0.0,
            body_reach_coefficient: 0.0,
            solar_flux_magnitude: 10.0,
            base_trophic_efficiency: 0.5,
            trophic_distance_decay: 0.0,
            reproduction_efficiency: 0.7,
            base_metabolic_rate: 0.1,
            movement_cost_coefficient: 0.05,
            sensing_range_coefficient: 10.0,
            reproduction_energy_threshold: 50.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_range_coefficient: 5.0,
            world_extent: 100.0,
            initial_population_size: 10,
            light_competition_radius: 1000.0,
            photo_maintenance_cost: 0.0,
            heterotrophy_maintenance_cost: 0.0,
            initial_nutrient_pool: 0.0,
            growth_efficiency: 0.0,
            wear_rate: 0.0,
            wear_degradation_steepness: 0.0,
            somatic_maintenance_cost_coefficient: 0.0,
            use_wear_rate: 0.0,
            structure_maintenance_coefficient: 0.0,
            repair_decay: 0.0,
            base_nutrient_ratio: 0.1,
            specification_nutrient_coefficient: 0.2,
            reproductive_compatibility_distance: 2.0,
            mobility_maintenance_cost: 0.0,
            maintenance_cost_exponent: 1.0,
            nutrient_grid_cell_size: 10.0,
            growth_retention_multiplier: 2.0,
            reserve_mobilisation_rate: 1.0,
            offspring_structure_fraction: 0.2,
        }
    }

    fn test_distribution() -> InitialDistribution {
        InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.5,
                heterotrophy: 0.3,
                mobility: 0.4,
                kappa: 0.5,
                fecundity: 0.0,
                asexual_propensity: 0.0,
                dispersal: 0.0,
            },
            trait_covariance: 0.1,
            initial_cluster_count: 1,
            initial_energy_per_agent: 100.0,
        }
    }

    #[test]
    fn propagule_cost_fraction_is_zero_when_coefficient_disabled() {
        let params = WorldParameters {
            dispersal_propagule_cost_coefficient: 0.0,
            ..test_params()
        };
        assert_eq!(dispersal_propagule_cost_fraction(3.0, &params), 0.0);
    }

    #[test]
    fn propagule_cost_fraction_is_superlinear_and_clamped() {
        let params = WorldParameters {
            dispersal_propagule_cost_coefficient: 0.1,
            dispersal_propagule_cost_exponent: 2.0,
            ..test_params()
        };
        let f1 = dispersal_propagule_cost_fraction(1.0, &params);
        let f2 = dispersal_propagule_cost_fraction(2.0, &params);
        // Superlinear: doubling the trait more than doubles the fraction.
        assert!(
            f2 > 2.0 * f1,
            "fraction should rise superlinearly: f1={f1}, f2={f2}"
        );
        // Clamped to at most 1.0 even for large dispersal.
        assert_eq!(dispersal_propagule_cost_fraction(100.0, &params), 1.0);
    }

    #[test]
    fn trait_vector_has_named_accessors() {
        let traits = TraitVector {
            photosynthetic_absorption: 0.1,
            heterotrophy: 0.2,
            mobility: 0.4,
            kappa: 0.8,
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        };
        assert_eq!(traits.photosynthetic_absorption, 0.1);
        assert_eq!(traits.heterotrophy, 0.2);
        assert_eq!(traits.mobility, 0.4);
        assert_eq!(traits.kappa, 0.8);
        assert_eq!(traits.get(2), 0.4); // index 2 is mobility
    }

    #[test]
    fn trait_vector_has_fecundity_dimension() {
        let traits = TraitVector {
            fecundity: 3.5,
            ..zero_traits()
        };
        assert_eq!(traits.fecundity, 3.5);
        assert_eq!(traits.get(4), 3.5);
    }

    #[test]
    fn trait_vector_has_kappa_dimension() {
        let traits = TraitVector {
            kappa: 0.7,
            ..zero_traits()
        };
        assert_eq!(traits.kappa, 0.7);
        assert_eq!(traits.get(3), 0.7);
        assert_eq!(TraitVector::NUM_DIMS, 7);
    }

    #[test]
    fn world_created_with_params_has_correct_population_size() {
        let world = World::new(test_params(), test_distribution(), 42);
        assert_eq!(world.agents().len(), 10);
    }

    #[test]
    fn free_energy_is_living_stock_excluding_carcasses() {
        let world = World::new(test_params(), test_distribution(), 42);
        let expected: f32 = world.agents().iter().map(|a| a.energy()).sum();
        assert_eq!(world.free_energy(), expected);
        // Adding a carcass must not change free energy — carcass energy is locked.
        let mut world = world;
        let before = world.free_energy();
        world.add_carcass(Carcass {
            id: 9999,
            position: (0.0, 0.0),
            energy: 500.0,
            nutrient: 0.0,
            traits: zero_traits(),
        });
        assert_eq!(
            world.free_energy(),
            before,
            "carcass energy is not free energy"
        );
    }

    #[test]
    fn carcass_locked_nutrient_fraction_is_dead_pool_share_of_total() {
        // Mirror of free_energy, on the nutrient side: an instantaneous read of
        // the dead pool's share of the conserved system nutrient (grid + living
        // agents + carcasses). This is the per-tick signal the genesis evaluator
        // samples to detect nutrient sequestered into carcasses that the living
        // decomposer population cannot turn over (issue #342).
        let mut world = World::new(test_params(), test_distribution(), 42);
        let grid = world.nutrient_pool();
        let agents: f32 = world
            .agents()
            .iter()
            .map(|a| a.nutrient_total(world.params()))
            .sum();
        // No carcasses yet: the dead pool holds none of the system's nutrient.
        assert_eq!(world.carcass_locked_nutrient_fraction(), 0.0);

        // Lock a known amount of nutrient into a carcass; the fraction becomes
        // that share of the (now larger) system total.
        world.add_carcass(Carcass {
            id: 9999,
            position: (0.0, 0.0),
            energy: 0.0,
            nutrient: 40.0,
            traits: zero_traits(),
        });
        let expected = 40.0 / (grid + agents + 40.0);
        assert!(
            (world.carcass_locked_nutrient_fraction() - expected).abs() < 1e-6,
            "fraction = carcass nutrient / total system nutrient, got {}",
            world.carcass_locked_nutrient_fraction()
        );
    }

    #[test]
    fn producer_energy_share_is_energy_weighted_autotrophy_fraction() {
        // Mirror of free_energy on the trophic-rhythm side (issue #392): an
        // instantaneous, scale-invariant read of how much of the living energy
        // sits in autotrophs. Per agent the autotrophy fraction is
        // photo / (photo + hetero); the world weights it by the agent's energy.
        let params = WorldParameters {
            initial_population_size: 0,
            ..test_params()
        };
        let mut world = World::new(params, test_distribution(), 42);

        // No agents → no living energy → share is 0.
        assert_eq!(world.producer_energy_share(), 0.0);

        // A pure producer (hetero = 0) and a pure consumer (photo = 0), equal
        // energy: the energy-weighted mean autotrophy fraction is 0.5.
        let producer_traits = TraitVector {
            photosynthetic_absorption: 0.8,
            heterotrophy: 0.0,
            ..zero_traits()
        };
        let consumer_traits = TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 0.6,
            ..zero_traits()
        };
        world.add_agent(Agent::new(0, (0.0, 0.0), 10.0, 0.0, 0.0, producer_traits));
        world.add_agent(Agent::new(0, (0.0, 0.0), 10.0, 0.0, 0.0, consumer_traits));
        assert!(
            (world.producer_energy_share() - 0.5).abs() < 1e-6,
            "equal-energy producer+consumer → 0.5, got {}",
            world.producer_energy_share()
        );
    }

    #[test]
    fn producer_energy_share_all_producer_and_all_consumer_and_zero_energy() {
        let params = WorldParameters {
            initial_population_size: 0,
            ..test_params()
        };
        let producer = TraitVector {
            photosynthetic_absorption: 0.9,
            heterotrophy: 0.1,
            ..zero_traits()
        };
        let consumer = TraitVector {
            photosynthetic_absorption: 0.1,
            heterotrophy: 0.9,
            ..zero_traits()
        };

        // All-producer (the autotrophy fraction is 0.9 each) → 0.9.
        let mut all_prod = World::new(params.clone(), test_distribution(), 42);
        all_prod.add_agent(Agent::new(0, (0.0, 0.0), 5.0, 0.0, 0.0, producer));
        all_prod.add_agent(Agent::new(0, (0.0, 0.0), 20.0, 0.0, 0.0, producer));
        assert!((all_prod.producer_energy_share() - 0.9).abs() < 1e-6);

        // All-consumer → 0.1.
        let mut all_cons = World::new(params.clone(), test_distribution(), 42);
        all_cons.add_agent(Agent::new(0, (0.0, 0.0), 5.0, 0.0, 0.0, consumer));
        all_cons.add_agent(Agent::new(0, (0.0, 0.0), 20.0, 0.0, 0.0, consumer));
        assert!((all_cons.producer_energy_share() - 0.1).abs() < 1e-6);

        // An agent with photo + hetero == 0 contributes autotrophy fraction 0;
        // an agent with zero energy contributes nothing. A single zero-energy
        // agent → total energy 0 → share 0.
        let mut zero_energy = World::new(params, test_distribution(), 42);
        zero_energy.add_agent(Agent::new(0, (0.0, 0.0), 0.0, 0.0, 0.0, producer));
        assert_eq!(zero_energy.producer_energy_share(), 0.0);
    }

    #[test]
    fn agent_and_carcass_have_nutrient_field() {
        let agent = Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 100.0,
            structure: 0.0,
            peak_structure: 0.0,
            nutrient: 5.0,
            traits: zero_traits(),
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        };
        assert_eq!(agent.nutrient, 5.0);
        let carcass = Carcass {
            id: 0,
            position: (0.0, 0.0),
            energy: 50.0,
            nutrient: 3.0,
            traits: zero_traits(),
        };
        assert_eq!(carcass.nutrient, 3.0);
    }

    #[test]
    fn reproduction_nutrient_threshold_has_serde_default() {
        // The reproduction nutrient gate's threshold is a new world parameter
        // with a serde default, so older recipes that omit it still parse.
        let params: WorldParameters = serde_json::from_str(
            r#"{
                "solar_flux_magnitude": 10.0,
                "base_trophic_efficiency": 1.0,
                "reproduction_efficiency": 0.7,
                "base_metabolic_rate": 0.0,
                "movement_cost_coefficient": 0.0,
                "reproduction_energy_threshold": 50.0,
                "mutation_rate": 0.0,
                "mutation_magnitude": 0.0,
                "contact_range_coefficient": 5.0,
                "world_extent": 100.0,
                "initial_population_size": 0,
                "light_competition_radius": 1000.0,
                "photo_maintenance_cost": 0.0,
                "heterotrophy_maintenance_cost": 0.0,
                "initial_nutrient_pool": 100.0,
                "growth_efficiency": 0.5
            }"#,
        )
        .expect("params omitting reproduction_nutrient_threshold should deserialise");
        assert_eq!(
            params.reproduction_nutrient_threshold,
            default_reproduction_nutrient_threshold()
        );
    }

    #[test]
    fn dispersal_reach_coefficient_has_serde_default() {
        // The dispersal contribution to mate-finding reach is a new world
        // parameter with a serde default of 0.0, so existing recipes/scenarios
        // that omit it deserialise unchanged and keep the pure-mobility reach.
        let params: WorldParameters = serde_json::from_str(
            r#"{
                "solar_flux_magnitude": 10.0,
                "base_trophic_efficiency": 1.0,
                "reproduction_efficiency": 0.7,
                "base_metabolic_rate": 0.0,
                "movement_cost_coefficient": 0.0,
                "reproduction_energy_threshold": 50.0,
                "mutation_rate": 0.0,
                "mutation_magnitude": 0.0,
                "contact_range_coefficient": 5.0,
                "world_extent": 100.0,
                "initial_population_size": 0,
                "light_competition_radius": 1000.0,
                "photo_maintenance_cost": 0.0,
                "heterotrophy_maintenance_cost": 0.0,
                "initial_nutrient_pool": 100.0,
                "growth_efficiency": 0.5
            }"#,
        )
        .expect("params omitting dispersal_reach_coefficient should deserialise");
        assert_eq!(params.dispersal_reach_coefficient, 0.0);
    }

    #[test]
    fn body_reach_coefficient_has_serde_default() {
        // The structure contribution to consumption (feeding) reach is a new
        // world parameter with a serde default of 0.0, so existing
        // recipes/scenarios that omit it deserialise unchanged and keep the
        // pure contact-range feeding reach.
        let params: WorldParameters = serde_json::from_str(
            r#"{
                "solar_flux_magnitude": 10.0,
                "base_trophic_efficiency": 1.0,
                "reproduction_efficiency": 0.7,
                "base_metabolic_rate": 0.0,
                "movement_cost_coefficient": 0.0,
                "reproduction_energy_threshold": 50.0,
                "mutation_rate": 0.0,
                "mutation_magnitude": 0.0,
                "contact_range_coefficient": 5.0,
                "world_extent": 100.0,
                "initial_population_size": 0,
                "light_competition_radius": 1000.0,
                "photo_maintenance_cost": 0.0,
                "heterotrophy_maintenance_cost": 0.0,
                "initial_nutrient_pool": 100.0,
                "growth_efficiency": 0.5
            }"#,
        )
        .expect("params omitting body_reach_coefficient should deserialise");
        assert_eq!(params.body_reach_coefficient, 0.0);
    }

    #[test]
    fn growth_efficiency_is_required() {
        // growth_efficiency = 0.0 is a silent kill switch: no agent builds
        // structure, so biomass, carcass energy, and the embodiment subsystem
        // all go dark. It must be a required field — a recipe that omits it is
        // under-specified and must fail loudly rather than fabricate a
        // corpse-world. (issue #327)
        let result: Result<WorldParameters, _> = serde_json::from_str(
            r#"{
                "solar_flux_magnitude": 10.0,
                "base_trophic_efficiency": 1.0,
                "reproduction_efficiency": 0.7,
                "base_metabolic_rate": 0.0,
                "movement_cost_coefficient": 0.0,
                "reproduction_energy_threshold": 50.0,
                "mutation_rate": 0.0,
                "mutation_magnitude": 0.0,
                "contact_range_coefficient": 5.0,
                "world_extent": 100.0,
                "initial_population_size": 0,
                "light_competition_radius": 1000.0,
                "photo_maintenance_cost": 0.0,
                "heterotrophy_maintenance_cost": 0.0,
                "initial_nutrient_pool": 100.0
            }"#,
        );
        let err = result.expect_err("params omitting growth_efficiency must fail to deserialise");
        assert!(
            err.to_string().contains("growth_efficiency"),
            "error should name the missing field, got: {err}"
        );
    }

    #[test]
    fn initial_nutrient_pool_is_required() {
        // initial_nutrient_pool = 0.0 is a silent kill switch: no nutrient
        // exists anywhere, so no agent clears the reproduction nutrient gate and
        // the nutrient economy never starts. It must be a required field so an
        // under-specified recipe fails loudly. (issue #327)
        let result: Result<WorldParameters, _> = serde_json::from_str(
            r#"{
                "solar_flux_magnitude": 10.0,
                "base_trophic_efficiency": 1.0,
                "reproduction_efficiency": 0.7,
                "base_metabolic_rate": 0.0,
                "movement_cost_coefficient": 0.0,
                "reproduction_energy_threshold": 50.0,
                "mutation_rate": 0.0,
                "mutation_magnitude": 0.0,
                "contact_range_coefficient": 5.0,
                "world_extent": 100.0,
                "initial_population_size": 0,
                "light_competition_radius": 1000.0,
                "photo_maintenance_cost": 0.0,
                "heterotrophy_maintenance_cost": 0.0,
                "growth_efficiency": 0.5
            }"#,
        );
        let err =
            result.expect_err("params omitting initial_nutrient_pool must fail to deserialise");
        assert!(
            err.to_string().contains("initial_nutrient_pool"),
            "error should name the missing field, got: {err}"
        );
    }

    #[test]
    fn agent_carries_repro_nutrient_earmark() {
        let agent = Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 0.0,
            structure: 0.0,
            peak_structure: 0.0,
            nutrient: 5.0,
            traits: zero_traits(),
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 7.0,
        };
        assert_eq!(agent.repro_nutrient, 7.0);
    }

    #[test]
    fn nutrient_total_counts_free_earmark_and_bound() {
        // Embodiment (ADR-0003): an agent's conserved nutrient total is its free
        // store + reproductive earmark + the nutrient bound in its structure
        // (structure * demand). The bound portion is matter locked into the body.
        let params = conservation_params();
        let agent = Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 0.0,
            structure: 4.0,
            peak_structure: 4.0,
            nutrient: 5.0,
            traits: zero_traits(),
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 7.0,
        };
        // zero traits -> ratio = base_nutrient_ratio = 0.1; bound = 4.0 * 0.1 = 0.4
        let bound = stoichiometric_demand(&agent.traits, agent.structure, &params);
        assert!((bound - 0.4).abs() < 1e-6, "bound = structure * ratio");
        assert!(
            (agent.nutrient_total(&params) - (5.0 + 7.0 + 0.4)).abs() < 1e-6,
            "nutrient_total = free + earmark + bound, got {}",
            agent.nutrient_total(&params)
        );
    }

    #[test]
    fn carcass_has_no_passive_decay() {
        // System design WR-3: carcasses are inert entities that hold the dead
        // agent's structure and nutrient at the death location indefinitely —
        // energy and nutrient leave a carcass ONLY via consumption (Flow 3).
        // With no living consumers in the world, a carcass's structure (energy)
        // and nutrient must be unchanged after stepping the world.
        let params = WorldParameters {
            initial_population_size: 0,
            ..test_params()
        };
        let mut world = World::new(params, test_distribution(), 42);
        assert!(
            world.agents().is_empty(),
            "world should have no living agents"
        );

        let initial_energy = 50.0_f32;
        let initial_nutrient = 3.0_f32;
        world.add_carcass(Carcass {
            id: 1,
            position: (50.0, 50.0),
            energy: initial_energy,
            nutrient: initial_nutrient,
            traits: zero_traits(),
        });

        for _tick in 0..100 {
            world.step();
        }

        assert_eq!(world.carcasses().len(), 1, "carcass should persist");
        let carcass = &world.carcasses()[0];
        assert_eq!(
            carcass.energy, initial_energy,
            "carcass structure must not decay without consumption"
        );
        assert_eq!(
            carcass.nutrient, initial_nutrient,
            "carcass nutrient must not decay without consumption"
        );
    }

    #[test]
    fn world_has_nutrient_pool() {
        let mut params = test_params();
        params.initial_nutrient_pool = 100.0;
        let world = World::new(params, test_distribution(), 42);
        assert_eq!(world.nutrient_pool(), 100.0);
    }

    #[test]
    fn step_does_not_panic() {
        let mut world = World::new(test_params(), test_distribution(), 42);
        world.step();
        world.step();
        assert!(world.tick() == 2);
    }

    #[test]
    fn structural_fragility_is_size_independent_and_ordered() {
        // structural_fragility is the normalised entropy of the trait vector,
        // in [0, 1): a pure specialist concentrates its budget and is robust
        // (fragility ~0); a uniform generalist spreads it and is fragile
        // (fragility -> 1). It is size-independent — it depends only on the
        // *shape* of the trait vector, never on the agent's structure — because
        // it is the fraction of *peak structure* below which damage is fatal.
        let specialist = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let generalist = TraitVector {
            photosynthetic_absorption: 0.1,
            heterotrophy: 0.1,
            mobility: 0.1,
            kappa: 0.1,
            fecundity: 0.1,
            asexual_propensity: 0.1,
            dispersal: 0.1,
        };
        let spec = structural_fragility(&specialist);
        let gen_f = structural_fragility(&generalist);
        assert!(
            spec >= 0.0 && spec < 1.0,
            "specialist fragility {spec} out of [0,1)"
        );
        assert!(
            gen_f >= 0.0 && gen_f < 1.0,
            "generalist fragility {gen_f} out of [0,1)"
        );
        assert!(
            gen_f > spec,
            "generalist fragility ({gen_f}) should exceed specialist ({spec})"
        );

        // Scaling the trait budget up or down must not change fragility — only
        // the shape matters.
        let bigger = TraitVector {
            photosynthetic_absorption: 0.2,
            heterotrophy: 0.2,
            mobility: 0.2,
            kappa: 0.2,
            fecundity: 0.2,
            asexual_propensity: 0.2,
            dispersal: 0.2,
        };
        assert!(
            (structural_fragility(&bigger) - gen_f).abs() < 1e-6,
            "fragility must be size-independent"
        );
    }

    #[test]
    fn newborns_and_seeds_are_born_viable_across_the_trait_range() {
        // Root fix for the "born below the death threshold, dead on arrival"
        // class of bugs (#310, #312): with the peak-relative threshold, an agent
        // whose peak structure equals its birth structure is always above its own
        // threshold, because fragility < 1. This must hold for *any* trait shape
        // and *any* birth size — even a tiny seed and the most uniform generalist.
        let shapes = [
            // pure specialist
            TraitVector {
                photosynthetic_absorption: 1.0,
                ..zero_traits()
            },
            // uniform generalist (maximal fragility)
            TraitVector {
                photosynthetic_absorption: 0.2,
                heterotrophy: 0.2,
                mobility: 0.2,
                kappa: 0.2,
                fecundity: 0.2,
                asexual_propensity: 0.2,
                dispersal: 0.2,
            },
            // fragile decomposer (example6)
            TraitVector {
                photosynthetic_absorption: 0.45,
                heterotrophy: 0.5,
                mobility: 0.2,
                kappa: 0.5,
                fecundity: 0.2,
                asexual_propensity: 0.3,
                dispersal: 0.1,
            },
        ];
        for traits in shapes {
            for birth_structure in [1e-4_f32, 0.01, 0.5, 5.0, 50.0] {
                let threshold = death_threshold(&traits, birth_structure);
                assert!(
                    threshold < birth_structure,
                    "newborn (structure {birth_structure}, fragility {}) must be \
                     born above its own threshold ({threshold})",
                    structural_fragility(&traits)
                );
            }
        }
    }

    #[test]
    fn generalist_dies_at_a_larger_fraction_of_peak_than_specialist() {
        // World-rules #9 preserved as a *relative* loss property: a complex,
        // interdependent generalist cannot sustain partial loss, so it dies having
        // lost a *smaller fraction of its peak* — i.e. its death threshold is a
        // *larger fraction* of peak structure than a specialist's. The threshold
        // is now relative to the body actually built, not an absolute floor.
        let specialist = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let generalist = TraitVector {
            photosynthetic_absorption: 0.2,
            heterotrophy: 0.2,
            mobility: 0.2,
            kappa: 0.2,
            fecundity: 0.2,
            asexual_propensity: 0.2,
            dispersal: 0.2,
        };
        let peak = 10.0;
        let spec_fraction = death_threshold(&specialist, peak) / peak;
        let gen_fraction = death_threshold(&generalist, peak) / peak;
        assert!(
            gen_fraction > spec_fraction,
            "generalist must die at a larger fraction of peak ({gen_fraction}) \
             than specialist ({spec_fraction})"
        );
    }

    #[test]
    fn pure_producer_simulation_runs_without_panic() {
        // A simulation of pure producers: photosynthesise, metabolise, grow, wear, die
        let params = WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.5,
            growth_efficiency: 0.5,
            wear_rate: 0.01,
            wear_degradation_steepness: 1.0,
            repair_decay: 1.0,
            initial_population_size: 0,
            // Producers need environmental nutrient to bootstrap structure: growth
            // is nutrient-co-limited, and photosynthesis needs structure > 0.
            initial_nutrient_pool: 100.0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        // Manually add pure producers, seeded with a little nutrient so they can
        // lay down initial structure on the first tick.
        for i in 0..10 {
            world.add_agent(Agent {
                id: 0, // will be reassigned
                position: (i as f32 * 5.0 - 25.0, 0.0),
                reserve: 50.0,
                structure: 0.0,
                peak_structure: 0.0,
                nutrient: 5.0,
                traits: TraitVector {
                    photosynthetic_absorption: 0.8,
                    kappa: 0.7,
                    ..zero_traits()
                },
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 0.0,
                repro_nutrient: 0.0,
            });
        }

        // Run for 100 ticks
        for _tick in 0..100 {
            world.step();
            // Some agents may die, that is fine
            if world.agents().is_empty() {
                // Producers died, simulation still did not panic
                break;
            }
        }

        // Verify events were logged
        assert!(world.event_log().len() > 0, "should have logged events");

        // Verify photosynthesis events exist
        let photo_events = world
            .event_log()
            .by_kind(&event::EventKind::Photosynthesized);
        assert!(
            !photo_events.is_empty(),
            "should have photosynthesis events"
        );

        // Verify metabolism events exist
        let metab_events = world.event_log().by_kind(&event::EventKind::Metabolized);
        assert!(!metab_events.is_empty(), "should have metabolism events");
    }

    #[test]
    fn use_dependent_wear_accumulates_through_world_step() {
        // A producer with nonzero use_wear_rate should accumulate more autotrophy
        // wear when it photosynthesises than baseline alone.
        let params = WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.1,
            growth_efficiency: 0.5,
            wear_rate: 0.01,
            use_wear_rate: 0.02,
            wear_degradation_steepness: 1.0,
            repair_decay: 0.0, // no repair, so wear only accumulates
            initial_population_size: 0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        // Add a single producer with structure (so it can photosynthesise)
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 0.0,
            traits: TraitVector {
                photosynthetic_absorption: 0.8,
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        });

        world.step();

        // After one tick, the producer should have photosynthesised and accumulated
        // both baseline and use-dependent wear on autotrophy.
        assert!(!world.agents().is_empty(), "agent should survive one tick");
        let agent = &world.agents()[0];
        // Baseline wear: 0.01 * 0.8 = 0.008
        // Use-dependent wear: 0.02 * energy_captured (should be > 0 since it photosynthesised)
        // Total should exceed baseline alone
        let baseline_only = 0.01 * 0.8;
        assert!(
            agent.wear[0] > baseline_only,
            "autotrophy wear ({}) should exceed baseline-only ({baseline_only}) due to photosynthesis use-wear",
            agent.wear[0]
        );
        // Heterotrophy and mobility wear should be baseline only (no usage)
        assert!(
            (agent.wear[1]).abs() < 1e-6,
            "heterotrophy wear should be zero (no heterotrophy trait)"
        );
        assert!(
            (agent.wear[2]).abs() < 1e-6,
            "mobility wear should be zero (no mobility trait)"
        );
    }

    #[test]
    fn use_dependent_wear_heterotrophy_through_world_step() {
        // A consumer that drains a target should accumulate extra heterotrophy wear.
        let params = WorldParameters {
            solar_flux_magnitude: 0.0, // no photosynthesis
            base_metabolic_rate: 0.0,  // no metabolism drain
            growth_efficiency: 0.0,
            wear_rate: 0.01,
            use_wear_rate: 0.02,
            wear_degradation_steepness: 1.0,
            repair_decay: 0.0, // no repair
            contact_range_coefficient: 10.0,
            base_trophic_efficiency: 0.5,
            trophic_distance_decay: 0.0,
            initial_population_size: 0,
            movement_cost_coefficient: 0.0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        // Consumer
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 0.0,
            peak_structure: 0.0,
            nutrient: 0.0,
            traits: TraitVector {
                heterotrophy: 0.6,
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        });
        // Target with structure to drain
        world.add_agent(Agent {
            id: 0,
            position: (1.0, 0.0), // within contact_radius=10
            reserve: 50.0,
            structure: 20.0,
            peak_structure: 20.0,
            nutrient: 0.0,
            traits: TraitVector {
                photosynthetic_absorption: 0.5,
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        });

        world.step();

        // Find the consumer (agent with heterotrophy)
        let consumer = world.agents().iter().find(|a| a.traits.heterotrophy > 0.3);
        assert!(consumer.is_some(), "consumer should survive");
        let consumer = consumer.unwrap();
        // Heterotrophy wear should exceed baseline due to consumption
        let baseline_only = 0.01 * 0.6;
        assert!(
            consumer.wear[1] > baseline_only,
            "heterotrophy wear ({}) should exceed baseline ({baseline_only}) due to consumption",
            consumer.wear[1]
        );
    }

    #[test]
    fn use_dependent_wear_mobility_through_world_step() {
        // A mobile agent that moves should accumulate extra mobility wear beyond
        // baseline. Movement runs before wear within each tick, so the distance
        // moved on a given tick is folded into THAT tick's mobility usage.
        let params = WorldParameters {
            solar_flux_magnitude: 0.0, // no photosynthesis
            base_metabolic_rate: 0.5,  // small drain so reserve is retained, not dumped to repro
            growth_efficiency: 0.0,
            wear_rate: 0.01,
            use_wear_rate: 0.02,
            wear_degradation_steepness: 1.0,
            repair_decay: 0.0,              // no repair
            movement_cost_coefficient: 0.0, // free movement so the agent survives
            initial_population_size: 0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        // A mobile agent: nonzero mobility drives a random-walk move each tick.
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 0.0,
            traits: TraitVector {
                mobility: 0.5,
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        });

        // Two ticks: each tick's move is charged as mobility use-wear that same
        // tick (move runs before wear), so wear[2] exceeds two ticks of baseline.
        world.step();
        world.step();

        let agent = world
            .agents()
            .iter()
            .find(|a| a.id == 0)
            .expect("mobile agent should survive two ticks");
        // Two ticks of baseline-only wear: 2 * wear_rate * mobility = 2 * 0.01 * 0.5
        let baseline_only = 2.0 * 0.01 * 0.5;
        assert!(
            agent.wear[2] > baseline_only,
            "mobility wear ({}) should exceed two ticks of baseline ({baseline_only}) due to movement use-wear",
            agent.wear[2]
        );
    }

    #[test]
    fn movement_wear_charged_in_same_tick() {
        // With the tick loop ordered move -> wear -> check death thresholds, an
        // agent's movement on a given tick is folded into THAT tick's mobility
        // use-wear (no one-tick lag). After a single step in which a mobile agent
        // moves a nonzero distance, wear[2] exceeds one tick of baseline-only wear.
        let params = WorldParameters {
            solar_flux_magnitude: 0.0, // no photosynthesis
            base_metabolic_rate: 0.5,
            growth_efficiency: 0.0,
            wear_rate: 0.01,
            use_wear_rate: 0.02,
            wear_degradation_steepness: 1.0,
            repair_decay: 0.0,              // no repair
            movement_cost_coefficient: 0.0, // free movement so the agent survives
            initial_population_size: 0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 0.0,
            traits: TraitVector {
                mobility: 0.5,
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        });

        // A single tick: the agent moves and the move-wear is charged this tick.
        world.step();

        let agent = world
            .agents()
            .iter()
            .find(|a| a.id == 0)
            .expect("mobile agent should survive one tick");
        // One tick of baseline-only wear: wear_rate * mobility = 0.01 * 0.5.
        // Under the old one-tick lag this is exactly what wear[2] would be after
        // the first step; charging the move this tick pushes it strictly higher.
        let baseline_only = 0.01 * 0.5;
        assert!(
            agent.wear[2] > baseline_only,
            "mobility wear ({}) should exceed one tick of baseline ({baseline_only}) because this tick's move is charged this tick",
            agent.wear[2]
        );
    }

    #[test]
    fn no_budget_normalization_exists() {
        // Budget normalization was removed per system design:
        // superlinear maintenance costs are the limiter, not algebraic normalization.
        // This test confirms the method no longer exists on TraitVector.
        let traits = TraitVector {
            photosynthetic_absorption: 2.0,
            heterotrophy: 3.0,
            kappa: 0.8,
            ..zero_traits()
        };
        // Traits retain their raw values — no normalization
        assert_eq!(traits.photosynthetic_absorption, 2.0);
        assert_eq!(traits.heterotrophy, 3.0);
        assert_eq!(traits.kappa, 0.8);
    }

    #[test]
    fn effective_trait_degrades_with_wear() {
        let mut agent = Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 10.0,
            structure: 0.0,
            peak_structure: 0.0,
            nutrient: 0.0,
            traits: TraitVector {
                photosynthetic_absorption: 1.0,
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        };
        let nominal = agent.effective_trait_with_steepness(0, 1.0);
        assert!((nominal - 1.0).abs() < 1e-6);

        agent.wear[0] = 1.0;
        let degraded = agent.effective_trait_with_steepness(0, 1.0);
        assert!(degraded < nominal);
        assert!(degraded > 0.0);
    }

    #[test]
    fn stoichiometric_demand_increases_with_specification() {
        let params = test_params();
        let structure = 5.0;
        // Low specification: only autotrophy
        let low_spec = TraitVector {
            photosynthetic_absorption: 0.3,
            ..zero_traits()
        };
        // High specification: autotrophy + heterotrophy + mobility
        let high_spec = TraitVector {
            photosynthetic_absorption: 0.3,
            heterotrophy: 0.4,
            mobility: 0.3,
            ..zero_traits()
        };
        let demand_low = stoichiometric_demand(&low_spec, structure, &params);
        let demand_high = stoichiometric_demand(&high_spec, structure, &params);
        assert!(
            demand_high > demand_low,
            "more specification investment should yield higher demand: low={}, high={}",
            demand_low,
            demand_high
        );

        // Non-specification traits (fecundity, mate_selectivity etc.) should NOT affect demand
        let with_repro = TraitVector {
            photosynthetic_absorption: 0.3,
            fecundity: 5.0,
            ..zero_traits()
        };
        let demand_repro = stoichiometric_demand(&with_repro, structure, &params);
        assert!(
            (demand_repro - demand_low).abs() < 1e-6,
            "non-specification traits should not affect demand: low={}, with_repro={}",
            demand_low,
            demand_repro
        );
    }

    #[test]
    fn stoichiometric_demand_zero_traits_base_ratio_only() {
        // With zero specification, demand = structure * base_ratio
        let params = test_params();
        let traits = zero_traits();
        let demand = stoichiometric_demand(&traits, 10.0, &params);
        let expected = 10.0 * params.base_nutrient_ratio;
        assert!(
            (demand - expected).abs() < 1e-6,
            "zero-spec demand should be structure * base_ratio: got {}, expected {}",
            demand,
            expected
        );
    }

    #[test]
    fn stoichiometric_demand_scales_with_structure() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            heterotrophy: 0.3,
            mobility: 0.2,
            ..zero_traits()
        };
        // Zero structure → zero demand (newborns)
        assert_eq!(stoichiometric_demand(&traits, 0.0, &params), 0.0);
        // Positive structure → positive demand
        let demand_small = stoichiometric_demand(&traits, 1.0, &params);
        let demand_large = stoichiometric_demand(&traits, 10.0, &params);
        assert!(
            demand_small > 0.0,
            "positive structure should yield positive demand"
        );
        assert!(
            demand_large > demand_small,
            "larger structure should yield larger demand"
        );
        // Demand is proportional to structure
        let ratio = demand_large / demand_small;
        assert!(
            (ratio - 10.0).abs() < 1e-4,
            "demand should scale linearly with structure"
        );
    }

    #[test]
    fn toroidal_distance_wraps_correctly() {
        let d = toroidal_distance((-48.0, 0.0), (48.0, 0.0), 100.0);
        assert!(
            (d - 4.0).abs() < 1e-3,
            "toroidal distance should be 4.0, got {}",
            d
        );
    }

    #[test]
    fn world_from_recipe_with_agents() {
        let recipe = WorldRecipe {
            parameters: test_params(),
            initial_distribution: None,
            agents: Some(vec![AgentSpec {
                position: (0.0, 0.0),
                reserve: 50.0,
                traits: zero_traits(),
                nutrient: 0.0,
            }]),
            carcasses: None,
            max_ticks: 100,
        };
        let world = World::from_recipe(&recipe, 42);
        assert_eq!(world.agents().len(), 1);
    }

    #[test]
    fn world_from_recipe_seeds_carcasses() {
        // A WorldRecipe can seed standing carcasses directly (issue #311): the
        // recipe carries a `carcasses` deposit and `from_recipe` materialises it
        // into the world alongside the seeded agents.
        let recipe = WorldRecipe {
            parameters: test_params(),
            initial_distribution: None,
            agents: Some(vec![AgentSpec {
                position: (0.0, 0.0),
                reserve: 50.0,
                traits: zero_traits(),
                nutrient: 0.0,
            }]),
            carcasses: Some(vec![CarcassSpec {
                position: (1.0, 2.0),
                energy: 40.0,
                nutrient: 3.0,
                traits: zero_traits(),
            }]),
            max_ticks: 100,
        };
        let world = World::from_recipe(&recipe, 42);
        assert_eq!(
            world.carcasses().len(),
            1,
            "seeded carcass must materialise"
        );
        let c = &world.carcasses()[0];
        assert_eq!(c.position, (1.0, 2.0));
        assert_eq!(c.energy, 40.0);
        assert_eq!(c.nutrient, 3.0);
    }

    #[test]
    fn seeded_carcass_ids_never_collide_with_agent_ids() {
        // Correctness: runtime carcasses reuse the dead agent's id, and seeded
        // agents take ids 0..pop_size with next_agent_id continuing from there.
        // Seeded-carcass ids must sit in a disjoint range so they cannot collide
        // with any current or future agent id (issue #311).
        let recipe = WorldRecipe {
            parameters: test_params(),
            initial_distribution: None,
            agents: Some(vec![
                AgentSpec {
                    position: (0.0, 0.0),
                    reserve: 50.0,
                    traits: zero_traits(),
                    nutrient: 0.0,
                },
                AgentSpec {
                    position: (1.0, 1.0),
                    reserve: 50.0,
                    traits: zero_traits(),
                    nutrient: 0.0,
                },
            ]),
            carcasses: Some(vec![
                CarcassSpec {
                    position: (2.0, 2.0),
                    energy: 10.0,
                    nutrient: 1.0,
                    traits: zero_traits(),
                },
                CarcassSpec {
                    position: (3.0, 3.0),
                    energy: 10.0,
                    nutrient: 1.0,
                    traits: zero_traits(),
                },
            ]),
            max_ticks: 100,
        };
        let mut world = World::from_recipe(&recipe, 7);
        // Drive enough births/deaths that next_agent_id advances well past pop_size.
        for _ in 0..50 {
            world.step();
        }
        let carcass_ids: std::collections::HashSet<u64> =
            world.carcasses().iter().map(|c| c.id).collect();
        let agent_ids: std::collections::HashSet<u64> =
            world.agents().iter().map(|a| a.id).collect();
        assert!(
            carcass_ids.is_disjoint(&agent_ids),
            "seeded-carcass ids must never collide with agent ids: carcasses {carcass_ids:?}, agents {agent_ids:?}"
        );
    }

    #[test]
    fn carcass_spec_round_trips_through_serde() {
        let spec = CarcassSpec {
            position: (4.5, -2.5),
            energy: 12.0,
            nutrient: 0.5,
            traits: zero_traits(),
        };
        let json = serde_json::to_string(&spec).expect("serialize");
        let back: CarcassSpec = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(spec, back);
        // `nutrient` defaults when omitted, mirroring AgentSpec.
        let minimal = r#"{"position":[1.0,2.0],"energy":5.0,"traits":{"photosynthetic_absorption":0.0,"heterotrophy":0.0,"mobility":0.0,"kappa":0.5,"fecundity":0.0,"asexual_propensity":0.0,"dispersal":0.0}}"#;
        let parsed: CarcassSpec = serde_json::from_str(minimal).expect("parse minimal");
        assert_eq!(parsed.nutrient, 0.0);
    }

    #[test]
    fn high_entropy_generalist_seed_survives_tick_zero() {
        // #312 regression (folded into #313): a small, high-entropy generalist
        // seed used to be born below the *absolute* structural death threshold and
        // die on arrival — `structure_at_birth < normalised_entropy × trait_budget`.
        // With the peak-relative threshold, its peak structure is its birth
        // structure and fragility < 1, so it is born viable and survives the death
        // check on the very first tick. No seed gate is needed.
        let params = WorldParameters {
            // Tiny base metabolism keeps a retention buffer (so reserve stays
            // positive); the trait-maintenance superlinear drains are zeroed so the
            // *only* death path under test is the structural threshold — not
            // starvation. The point is structural viability at birth.
            solar_flux_magnitude: 0.0,
            base_metabolic_rate: 0.001,
            somatic_maintenance_cost_coefficient: 0.0,
            structure_maintenance_coefficient: 0.0,
            photo_maintenance_cost: 0.0,
            heterotrophy_maintenance_cost: 0.0,
            mobility_maintenance_cost: 0.0,
            asexual_propensity_maintenance_cost: 0.0,
            wear_rate: 0.0,
            use_wear_rate: 0.0,
            movement_cost_coefficient: 0.0,
            growth_efficiency: 0.5,
            offspring_structure_fraction: 0.2,
            initial_nutrient_pool: 1000.0,
            ..test_params()
        };
        // A uniform generalist (maximal fragility) with a *small* energy budget,
        // so its seeded structure is tiny — exactly the dead-on-arrival profile.
        let generalist = TraitVector {
            photosynthetic_absorption: 0.5,
            heterotrophy: 0.2,
            mobility: 0.2,
            kappa: 0.2,
            fecundity: 0.2,
            asexual_propensity: 0.2,
            dispersal: 0.2,
        };
        let recipe = WorldRecipe {
            parameters: params,
            initial_distribution: None,
            agents: Some(vec![AgentSpec {
                position: (0.0, 0.0),
                reserve: 1.0, // small budget -> tiny seeded structure
                traits: generalist,
                nutrient: 100.0,
            }]),
            carcasses: None,
            max_ticks: 10,
        };
        let mut world = World::from_recipe(&recipe, 42);
        // Born alive at world creation.
        assert_eq!(world.agents().len(), 1, "seed must be created alive");
        let seed_structure = world.agents()[0].structure;
        assert!(seed_structure > 0.0, "seed should have positive structure");
        // The seed's peak high-water mark equals its birth structure.
        assert!(
            (world.agents()[0].peak_structure - seed_structure).abs() < 1e-9,
            "seed peak structure must equal its birth structure"
        );
        // Survives the death check on the very first tick.
        world.step();
        assert_eq!(
            world.agents().len(),
            1,
            "high-entropy generalist seed must survive its first tick (#312)"
        );
    }

    // --- Energy ledger conservation tests ---

    /// Helper: conservation params with all phases active.
    fn conservation_params() -> WorldParameters {
        WorldParameters {
            asexual_propensity_maintenance_cost: 0.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            dispersal_reach_coefficient: 0.0,
            body_reach_coefficient: 0.0,
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.5,
            growth_efficiency: 0.5,
            wear_rate: 0.01,
            wear_degradation_steepness: 1.0,
            repair_decay: 1.0,
            somatic_maintenance_cost_coefficient: 0.05,
            structure_maintenance_coefficient: 0.01,
            movement_cost_coefficient: 0.1,
            sensing_range_coefficient: 10.0,
            reproduction_energy_threshold: 20.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.1,
            mutation_magnitude: 0.05,
            initial_population_size: 0,
            initial_nutrient_pool: 100.0,
            base_trophic_efficiency: 0.5,
            trophic_distance_decay: 0.0,
            contact_range_coefficient: 10.0,
            world_extent: 50.0,
            light_competition_radius: 100.0,
            photo_maintenance_cost: 0.05,
            heterotrophy_maintenance_cost: 0.05,
            use_wear_rate: 0.01,
            base_nutrient_ratio: 0.1,
            specification_nutrient_coefficient: 0.2,
            reproductive_compatibility_distance: 2.0,
            mobility_maintenance_cost: 0.0,
            maintenance_cost_exponent: 1.0,
            nutrient_grid_cell_size: 10.0,
            growth_retention_multiplier: 2.0,
            reserve_mobilisation_rate: 1.0,
            offspring_structure_fraction: 0.2,
        }
    }

    /// Create a mixed population: producers and heterotrophs.
    fn seed_mixed_population(world: &mut World) {
        // Producers (sessile, photosynthetic)
        for i in 0..10 {
            world.add_agent(Agent {
                id: 0,
                position: (i as f32 * 5.0 - 25.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 10.0,
                traits: TraitVector {
                    photosynthetic_absorption: 0.6,
                    kappa: 0.7,
                    fecundity: 1.0,
                    ..zero_traits()
                },
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 5.0,
                repro_nutrient: 0.0,
            });
        }
        // Heterotrophs (mobile, consume living and dead)
        for i in 0..10 {
            world.add_agent(Agent {
                id: 0,
                position: (i as f32 * 10.0 - 25.0, 10.0),
                reserve: 40.0,
                structure: 3.0,
                peak_structure: 3.0,
                nutrient: 5.0,
                traits: TraitVector {
                    heterotrophy: 0.4,
                    mobility: 0.3,
                    kappa: 0.5,
                    fecundity: 1.0,
                    ..zero_traits()
                },
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 5.0,
                repro_nutrient: 0.0,
            });
        }
    }

    #[test]
    fn single_tick_energy_conservation_pure_producer() {
        // A single producer: solar input = energy retained + dissipated.
        // The ledger should verify conservation after each tick.
        let params = WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 1.0,
            initial_population_size: 0,
            growth_efficiency: 0.0,
            movement_cost_coefficient: 0.0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0, // nonzero structure required for light capture
            nutrient: 0.0,
            traits: TraitVector {
                photosynthetic_absorption: 0.8,
                kappa: 0.7,
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        });

        world.step();

        // The ledger should be balanced — if step() records flows correctly,
        // assert_balanced() should not panic.
        world.energy_ledger().assert_balanced();

        // Solar input should be positive
        assert!(
            world.energy_ledger().total_solar_input() > 0.0,
            "solar input should be positive"
        );
    }

    #[test]
    fn energy_conservation_500_ticks_mixed_population() {
        // Run 500 ticks with all phases active (photosynthesis, nutrient uptake,
        // metabolism, growth, drains, reproduction, wear, death, movement).
        // assert_balanced() must pass on every tick.
        let params = conservation_params();
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        seed_mixed_population(&mut world);

        for _tick in 0..500 {
            world.step();
            // assert_balanced() panics if conservation is violated
            world.energy_ledger().assert_balanced();

            if world.agents().is_empty() {
                // All agents died — conservation still held until here
                break;
            }
        }

        // Verify cumulative conservation: total solar = dissipated + retained
        let total_solar = world.total_solar_input();
        let total_dissipated = world.dissipated_energy();
        let retained_agents: f32 = world
            .agents()
            .iter()
            .map(|a| a.reserve + a.structure + a.repro_reserve)
            .sum();
        let retained_carcasses: f32 = world.carcasses().iter().map(|c| c.energy).sum();
        // Initial endowment energy
        let initial_energy: f32 = 50.0 * 10.0  // producers reserve
            + 5.0 * 10.0   // producer structure
            + 5.0 * 10.0   // producer repro_reserve
            + 40.0 * 10.0  // heterotrophs reserve
            + 3.0 * 10.0   // heterotroph structure
            + 5.0 * 10.0; // heterotroph repro_reserve
        let total_input = initial_energy + total_solar;
        let total_output = total_dissipated + retained_agents + retained_carcasses;
        let diff = (total_input - total_output).abs();
        let tolerance = total_input * 1e-3; // 0.1% tolerance for f32
        assert!(
            diff < tolerance,
            "cumulative energy conservation violated: input={total_input}, output={total_output}, diff={diff}"
        );
    }

    #[test]
    fn world_creation_binds_seed_structure_nutrient_from_the_pool() {
        // At world creation, seeded agents are born with structure, which binds
        // nutrient (structure * demand). That bound nutrient must be drawn from
        // the available pool so total system nutrient at tick 0 equals the
        // initial pool — nutrient is not conjured into the living system.
        let mut params = conservation_params();
        params.initial_population_size = 8;
        params.initial_nutrient_pool = 500.0;
        let dist = InitialDistribution {
            mean_traits: TraitVector {
                photosynthetic_absorption: 0.5,
                ..zero_traits()
            },
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let world = World::new(params.clone(), dist, 42);

        // Some agents must actually carry bound nutrient for this to be a real
        // test (seeded structure > 0 and positive demand).
        let total_bound: f32 = world
            .agents()
            .iter()
            .map(|a| a.bound_nutrient(&params))
            .sum();
        assert!(
            total_bound > 0.0,
            "seeded agents should carry bound nutrient"
        );

        let total_system_nutrient: f32 = world.nutrient_pool()
            + world
                .agents()
                .iter()
                .map(|a| a.nutrient_total(&params))
                .sum::<f32>()
            + world.carcasses().iter().map(|c| c.nutrient).sum::<f32>();
        assert!(
            (total_system_nutrient - params.initial_nutrient_pool).abs() < 1e-2,
            "total system nutrient at creation must equal the initial pool: got {}, expected {}",
            total_system_nutrient,
            params.initial_nutrient_pool
        );
    }

    #[test]
    fn nutrient_conservation_across_ticks() {
        // Total nutrient (available pool + living agents + carcasses) must be
        // constant across all ticks.
        let params = conservation_params();
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        seed_mixed_population(&mut world);

        // Compute initial total nutrient
        let initial_nutrient: f32 = world.nutrient_pool()
            + world
                .agents()
                .iter()
                .map(|a| a.nutrient_total(world.params()))
                .sum::<f32>()
            + world.carcasses().iter().map(|c| c.nutrient).sum::<f32>();

        for t in 0..200 {
            world.step();

            let current_nutrient: f32 = world.nutrient_pool()
                + world
                    .agents()
                    .iter()
                    .map(|a| a.nutrient_total(world.params()))
                    .sum::<f32>()
                + world.carcasses().iter().map(|c| c.nutrient).sum::<f32>();

            let diff = (current_nutrient - initial_nutrient).abs();
            let tolerance = initial_nutrient.abs().max(1.0) * 1e-4;
            assert!(
                diff < tolerance,
                "nutrient conservation violated at tick {t}: initial={initial_nutrient}, current={current_nutrient}, diff={diff}"
            );

            if world.agents().is_empty() {
                break;
            }
        }
    }

    // --- Nutrient ledger conservation tests ---

    #[test]
    fn nutrient_ledger_balanced_after_uptake() {
        // A producer absorbs nutrient from the grid. The nutrient ledger must
        // remain balanced: nutrient moved grid -> agent, none created or lost.
        let params = conservation_params();
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 0.0,
            traits: TraitVector {
                photosynthetic_absorption: 0.6,
                kappa: 0.7,
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        });

        world.step();

        world.nutrient_ledger().assert_balanced();
    }

    #[test]
    fn nutrient_ledger_balanced_after_consumption_with_excretion() {
        // A heterotroph consumes a nutrient-rich neighbour. Some nutrient is
        // retained, the excess is excreted back to the grid. Total nutrient
        // (grid + agents + carcasses) is unchanged: the ledger stays balanced.
        let params = conservation_params();
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        // Consumer: heterotroph, co-located with target.
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 40.0,
            structure: 3.0,
            peak_structure: 3.0,
            nutrient: 0.0,
            traits: TraitVector {
                heterotrophy: 0.5,
                kappa: 0.5,
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        });
        // Nutrient-rich target.
        world.add_agent(Agent {
            id: 0,
            position: (0.5, 0.0),
            reserve: 10.0,
            structure: 10.0,
            peak_structure: 10.0,
            nutrient: 20.0,
            traits: TraitVector {
                photosynthetic_absorption: 0.1,
                kappa: 0.7,
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        });

        world.step();

        world.nutrient_ledger().assert_balanced();
    }

    #[test]
    fn nutrient_ledger_balanced_after_death_to_carcass() {
        // An agent dies (starvation: zero reserve, no income) and its nutrient
        // transfers to a carcass. Total nutrient is conserved across the
        // death→carcass transition.
        let params = WorldParameters {
            base_metabolic_rate: 100.0, // guarantee starvation this tick
            ..conservation_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 1.0, // tiny reserve — wiped out by metabolism
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 12.0,
            traits: TraitVector {
                heterotrophy: 0.3,
                kappa: 0.7,
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        });

        world.step();

        // The agent should have died and become a carcass holding its nutrient.
        assert!(world.agents().is_empty(), "agent should have starved");
        assert!(!world.carcasses().is_empty(), "a carcass should exist");
        world.nutrient_ledger().assert_balanced();
    }

    #[test]
    fn nutrient_ledger_balanced_after_reproduction() {
        // An agent reproduces asexually, donating nutrient to its offspring.
        // Donated nutrient leaves the parent and enters the offspring; total
        // nutrient across all living agents is conserved.
        let params = WorldParameters {
            base_metabolic_rate: 0.0, // preserve repro_reserve through metabolism
            reproduction_energy_threshold: 5.0,
            reproduction_nutrient_threshold: 1.0,
            ..conservation_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 60.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 20.0,
            traits: TraitVector {
                photosynthetic_absorption: 0.3,
                kappa: 0.7,
                fecundity: 5.0,
                asexual_propensity: 1.0, // guaranteed asexual reproduction
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            // Funded above the #310 viability gate (fecundity 5 × death threshold).
            repro_reserve: 350.0,
            repro_nutrient: 10.0, // earmark above threshold so reproduction fires
        });

        world.step();

        assert!(
            world.last_tick_births() > 0,
            "reproduction should produce births this tick"
        );
        world.nutrient_ledger().assert_balanced();
    }

    #[test]
    fn fed_producer_reproduces_and_is_not_pinned_on_nutrient_gate() {
        // Regression for #269. Under the build-permit model, the per-tick order
        // absorb -> grow -> reproduce let growth greedily convert each tick's
        // nutrient up to the permit ceiling, leaving a well-fed agent pinned
        // *exactly* on the old reproduction gate (nutrient == structure * demand)
        // — never above it, never with free nutrient to donate. So a fed
        // population never reproduced. With nutrient uptake split by kappa, the
        // (1 - kappa) share lands in the repro_nutrient earmark that growth
        // cannot touch, so the earmark accumulates past the threshold and
        // reproduction fires.
        let params = WorldParameters {
            // Abundant light and nutrient: a genuinely fed steady state.
            solar_flux_magnitude: 50.0,
            initial_nutrient_pool: 1000.0,
            base_metabolic_rate: 0.1,
            growth_efficiency: 0.5, // growth is active and would consume the permit
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.0,
            ..conservation_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 7);
        // A single sessile autotroph with kappa < 1 (so some uptake is
        // earmarked), positive fecundity and a guaranteed-asexual draw.
        world.add_agent(Agent {
            id: 0,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 0.0,
            traits: TraitVector {
                photosynthetic_absorption: 0.8,
                kappa: 0.6,
                fecundity: 2.0,
                asexual_propensity: 1.0,
                ..zero_traits()
            },
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 0.0,
            repro_nutrient: 0.0,
        });

        let mut total_births = 0;
        for _ in 0..200 {
            world.step();
            total_births += world.last_tick_births();
            world.nutrient_ledger().assert_balanced();
            if world.agents().is_empty() {
                break;
            }
        }

        assert!(
            total_births > 0,
            "a fed producer must reproduce — the #269 nutrient-gate pinning is resolved"
        );
    }

    #[test]
    fn nutrient_ledger_balanced_500_ticks_mixed_population() {
        // Run a mixed population through many ticks, exercising every nutrient
        // path (uptake, consumption + excretion, carcass consumption, death,
        // reproduction). The nutrient ledger must stay balanced on every tick.
        let params = conservation_params();
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        seed_mixed_population(&mut world);

        for _tick in 0..500 {
            world.step();
            world.nutrient_ledger().assert_balanced();
            if world.agents().is_empty() {
                break;
            }
        }
    }

    #[test]
    fn no_negative_energy_balance_in_ledger() {
        // No agent or carcass endpoint should have negative net balance
        // in the ledger at any tick boundary.
        let params = conservation_params();
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        seed_mixed_population(&mut world);

        for _tick in 0..100 {
            world.step();
            // assert_balanced() checks no negative balances
            world.energy_ledger().assert_balanced();
            if world.agents().is_empty() {
                break;
            }
        }
    }

    #[test]
    fn topology_projection_works_with_event_log() {
        // TopologyProjection should process the event log from World::step()
        // without panicking, correctly reading Consumed, Reproduced, Died events.
        let params = conservation_params();
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        seed_mixed_population(&mut world);

        for _ in 0..50 {
            world.step();
            if world.agents().is_empty() {
                break;
            }
        }

        let mut topo = crate::topology::TopologyProjection::new();
        topo.update(world.event_log());

        // Should have processed events without panic
        // Verify trophic_roles() returns a map (may be empty if all initial agents died)
        let _roles = topo.trophic_roles(world.agents());
        // Computing roles without panic is the key test

        // Died events should remove agents from active set
        let died_events = world.event_log().by_kind(&event::EventKind::Died);
        for ev in &died_events {
            assert!(
                !topo.active_agents().contains(&ev.source),
                "dead agent {} should not be in active set",
                ev.source
            );
        }

        // Incremental update should not panic
        topo.update(world.event_log());

        // Verify event log has expected event types
        let log = world.event_log();
        assert!(
            !log.by_kind(&event::EventKind::Photosynthesized).is_empty(),
            "should have photosynthesis events"
        );
        assert!(
            !log.by_kind(&event::EventKind::Metabolized).is_empty(),
            "should have metabolism events"
        );
    }

    // --- Trophic transfer efficiency tests ---

    #[test]
    fn trophic_efficiency_equals_base_when_traits_identical() {
        let params = WorldParameters {
            base_trophic_efficiency: 0.7,
            trophic_distance_decay: 2.0,
            ..test_params()
        };
        let traits = TraitVector {
            heterotrophy: 0.5,
            ..zero_traits()
        };
        let eff = trophic_transfer_efficiency(&traits, &traits, &params);
        assert!(
            (eff - 0.7).abs() < 1e-6,
            "identical traits should yield base efficiency, got {}",
            eff
        );
    }

    #[test]
    fn trophic_efficiency_decreases_with_trait_distance() {
        let params = WorldParameters {
            base_trophic_efficiency: 0.8,
            trophic_distance_decay: 1.0,
            ..test_params()
        };
        let consumer = TraitVector {
            heterotrophy: 0.5,
            ..zero_traits()
        };
        let near_target = TraitVector {
            heterotrophy: 0.6,
            ..zero_traits()
        };
        let far_target = TraitVector {
            heterotrophy: 0.5,
            photosynthetic_absorption: 1.0,
            mobility: 1.0,
            ..zero_traits()
        };
        let eff_near = trophic_transfer_efficiency(&consumer, &near_target, &params);
        let eff_far = trophic_transfer_efficiency(&consumer, &far_target, &params);
        assert!(
            eff_near > eff_far,
            "near target should have higher efficiency: near={}, far={}",
            eff_near,
            eff_far
        );
        assert!(
            eff_near < 0.8,
            "non-identical traits should be below base: {}",
            eff_near
        );
        assert!(eff_far > 0.0, "efficiency should be positive: {}", eff_far);
    }

    #[test]
    fn trophic_efficiency_zero_decay_gives_flat_efficiency() {
        // With decay=0, efficiency is always base regardless of distance
        let params = WorldParameters {
            base_trophic_efficiency: 0.6,
            trophic_distance_decay: 0.0,
            ..test_params()
        };
        let consumer = zero_traits();
        let far_target = TraitVector {
            photosynthetic_absorption: 10.0,
            heterotrophy: 10.0,
            mobility: 10.0,
            ..zero_traits()
        };
        let eff = trophic_transfer_efficiency(&consumer, &far_target, &params);
        assert!(
            (eff - 0.6).abs() < 1e-6,
            "zero decay should yield flat base efficiency, got {}",
            eff
        );
    }

    #[test]
    fn trophic_efficiency_higher_decay_penalises_distance_more() {
        let consumer = TraitVector {
            heterotrophy: 0.5,
            ..zero_traits()
        };
        let target = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let low_decay = WorldParameters {
            base_trophic_efficiency: 0.8,
            trophic_distance_decay: 0.5,
            ..test_params()
        };
        let high_decay = WorldParameters {
            base_trophic_efficiency: 0.8,
            trophic_distance_decay: 3.0,
            ..test_params()
        };
        let eff_low = trophic_transfer_efficiency(&consumer, &target, &low_decay);
        let eff_high = trophic_transfer_efficiency(&consumer, &target, &high_decay);
        assert!(
            eff_low > eff_high,
            "higher decay should reduce efficiency more: low_decay={}, high_decay={}",
            eff_low,
            eff_high
        );
    }

    #[test]
    fn energy_conservation_with_distance_dependent_efficiency() {
        // Full tick loop with nonzero trophic_distance_decay. Energy ledger
        // must balance on every tick despite variable per-event efficiency.
        let params = WorldParameters {
            base_trophic_efficiency: 0.7,
            trophic_distance_decay: 1.5,
            ..conservation_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);
        seed_mixed_population(&mut world);

        for _tick in 0..200 {
            world.step();
            world.energy_ledger().assert_balanced();
            if world.agents().is_empty() {
                break;
            }
        }

        // Cumulative conservation
        let total_solar = world.total_solar_input();
        let total_dissipated = world.dissipated_energy();
        let retained_agents: f32 = world
            .agents()
            .iter()
            .map(|a| a.reserve + a.structure + a.repro_reserve)
            .sum();
        let retained_carcasses: f32 = world.carcasses().iter().map(|c| c.energy).sum();
        let initial_energy: f32 =
            50.0 * 10.0 + 5.0 * 10.0 + 5.0 * 10.0 + 40.0 * 10.0 + 3.0 * 10.0 + 5.0 * 10.0;
        let total_input = initial_energy + total_solar;
        let total_output = total_dissipated + retained_agents + retained_carcasses;
        let diff = (total_input - total_output).abs();
        let tolerance = total_input * 2e-3; // 0.2% tolerance for f32 accumulation over 200 ticks
        assert!(
            diff < tolerance,
            "cumulative energy conservation violated with distance-dependent efficiency: \
             input={total_input}, output={total_output}, diff={diff}"
        );
    }

    #[test]
    fn dying_agent_takes_final_step_before_death_check() {
        // With the tick loop ordered move -> wear -> check death thresholds, a
        // mobile agent destined to die from wear this tick takes one final step
        // before the death check. The agent still dies; its final move only
        // repositions a corpse no subsequent tick will read, so it is immaterial.
        let params = WorldParameters {
            solar_flux_magnitude: 0.0, // no photosynthesis income
            base_metabolic_rate: 0.0,  // no metabolism drain
            growth_efficiency: 0.0,
            wear_rate: 10.0, // very high baseline wear
            use_wear_rate: 0.0,
            wear_degradation_steepness: 1.0,
            repair_decay: 0.0, // no repair
            contact_range_coefficient: 5.0,
            movement_cost_coefficient: 0.0,
            initial_population_size: 0,
            ..test_params()
        };
        let dist = InitialDistribution {
            mean_traits: zero_traits(),
            trait_covariance: 0.0,
            initial_cluster_count: 1,
            initial_energy_per_agent: 50.0,
        };
        let mut world = World::new(params, dist, 42);

        // Agent with mobility (would move) and structure just above death threshold.
        // High wear_rate will degrade traits, pushing structure below threshold → death.
        // We give it a generalist trait spread so its death_threshold is meaningful,
        // and structure just barely above that threshold.
        let traits = TraitVector {
            mobility: 0.5,
            photosynthetic_absorption: 0.3,
            heterotrophy: 0.3,
            kappa: 0.3,
            fecundity: 0.3,
            asexual_propensity: 0.3,
            dispersal: 0.3,
            ..zero_traits()
        };
        // Death threshold for this generalist is significant. The threshold is a
        // fraction of *peak* structure, so we model an agent that grew to a peak
        // of 10.0 and has since been worn down below its peak-relative threshold.
        // We set reserve to a small positive value so the agent doesn't die from
        // starvation before wear runs, but the structure is below threshold.
        let peak = 10.0;
        let threshold = death_threshold(&traits, peak);
        let mut agent = Agent::new(0, (0.0, 0.0), 0.01, threshold * 0.5, 0.0, traits);
        agent.peak_structure = peak; // had grown to its peak, now worn down
        world.add_agent(agent);

        let agent_id = world.agents().last().unwrap().id;

        world.step();

        // The agent should still have died from structure below the threshold.
        let died_events: Vec<_> = world
            .event_log()
            .by_kind(&event::EventKind::Died)
            .into_iter()
            .filter(|e| e.source == agent_id)
            .collect();
        assert!(
            !died_events.is_empty(),
            "agent should have died from structure below death threshold"
        );

        // The agent is removed from the living population after the death check.
        assert!(
            world.agents().iter().all(|a| a.id != agent_id),
            "dead agent should be removed from the living population"
        );

        // It took one final step before the death check: the move phase now runs
        // ahead of wear/death, so a Moved event for the dying agent is expected
        // (and immaterial — it only repositions a corpse).
        let moved_events: Vec<_> = world
            .event_log()
            .by_kind(&event::EventKind::Moved)
            .into_iter()
            .filter(|e| e.source == agent_id)
            .collect();
        assert!(
            !moved_events.is_empty(),
            "dying agent should take one final step before the death check (move runs before wear/death)"
        );
    }
}
