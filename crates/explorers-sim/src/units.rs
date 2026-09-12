//! Unit anchors: trait-to-flow conversions (#459) and use-wear couplings (#460).
//!
//! Traits are dimensionless (`docs/system-design/trait-space.md`), yet five
//! committed forms read a trait *as* a dimensional flow. Each such conversion
//! carries a unit constant that was previously implicit — a bare `= trait` in
//! the phase code. The dimensional analysis in
//! `docs/research/440-dimensionless-groups.md` (smell S1) names them; this
//! module makes them explicit, citable, and used at the conversion site.
//!
//! A second family (smell S2) sits in use-dependent wear: `use_wear_rate`
//! multiplies three usages of different dimension — energy captured, energy
//! drained, distance moved — with one coefficient. The three unit constants
//! that make that product homogeneous were likewise implicit; they are named
//! below and multiplied in at `phase::apply_wear` / `soa::apply_wear_soa`.
//!
//! Every anchor is `1.0` in today's units. Naming them changes no number and
//! no trajectory (guarded by `tests/trait_unit_anchors.rs`). Whether any of
//! them should become a `WorldParameters` field — or whether `use_wear_rate`
//! should split — is a later design decision, not one this module makes; see
//! `world-rules.md`, *Unit anchors* and *Somatic wear*.
//!
//! Base units follow the research note: energy `E`, nutrient `N`, length `L`,
//! tick `T`.

/// `u_M`: distance an agent moves per tick per unit of effective mobility.
/// Units: `L/T` per trait unit. Read in `phase::move_agents`.
pub const MOBILITY_DISTANCE_PER_TICK: f32 = 1.0;

/// `u_D`: standard deviation of the offspring placement kernel
/// `Normal(0, σ)` per unit of the seed parent's dispersal trait.
/// Units: `L` per trait unit. Read in `phase::resolve_reproduction` (both the
/// asexual and the sexual placement step).
pub const DISPERSAL_KERNEL_SIGMA: f32 = 1.0;

/// `u_A`: nutrient an agent demands from its cell per tick per unit of
/// effective photosynthetic absorption. Units: `N/T` per trait unit. Read in
/// `phase::absorb_nutrients`.
pub const AUTOTROPHY_NUTRIENT_UPTAKE_PER_TICK: f32 = 1.0;

/// `u_H`: structure drained from each in-reach target per tick per unit of
/// effective heterotrophy. Units: `E/T` per trait unit (structure is embodied
/// energy). Read in `phase::resolve_drains` for live targets and carcasses.
pub const HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK: f32 = 1.0;

/// `u_R`: base wear repair per functional trait per tick per unit kappa,
/// before the `exp(−repair_decay · wear)` attenuation. Units: `E/T` per unit
/// kappa (wear is repaired 1:1 with energy). Read in `phase::grow` and its
/// SoA twin `soa::grow`.
pub const KAPPA_REPAIR_PER_TICK: f32 = 1.0;

/// `u_WA`: wear accrued on the autotrophy apparatus per unit of
/// `use_wear_rate` per unit of energy captured (functional trait 0). Wear is
/// an energy (repaired 1:1 in `grow`), so this coupling is dimensionless:
/// `E/E`. Read in `phase::apply_wear` and `soa::apply_wear_soa`.
pub const USE_WEAR_PER_ENERGY_CAPTURED: f32 = 1.0;

/// `u_WH`: wear accrued on the heterotrophy apparatus per unit of
/// `use_wear_rate` per unit of energy drained from targets (functional trait
/// 1). Dimensionless: `E/E`. Read in `phase::apply_wear` and
/// `soa::apply_wear_soa`.
pub const USE_WEAR_PER_ENERGY_DRAINED: f32 = 1.0;

/// `u_WM`: wear accrued on the locomotion apparatus per unit of
/// `use_wear_rate` per unit of distance moved (functional trait 2). This is
/// the coupling that breaks `use_wear_rate`'s homogeneity: `E/L`, where the
/// other two are `E/E`. Read in `phase::apply_wear` and `soa::apply_wear_soa`.
pub const USE_WEAR_PER_DISTANCE_MOVED: f32 = 1.0;

/// The three use-wear anchors indexed by functional trait, in the order
/// `usage` is laid out (`[energy captured, energy drained, distance moved]`).
pub const USE_WEAR_ANCHORS: [f32; crate::FUNCTIONAL_TRAIT_COUNT] = [
    USE_WEAR_PER_ENERGY_CAPTURED,
    USE_WEAR_PER_ENERGY_DRAINED,
    USE_WEAR_PER_DISTANCE_MOVED,
];
