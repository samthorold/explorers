//! Trait-to-flow unit anchors (#459).
//!
//! Traits are dimensionless (`docs/system-design/trait-space.md`), yet five
//! committed forms read a trait *as* a dimensional flow. Each such conversion
//! carries a unit constant that was previously implicit — a bare `= trait` in
//! the phase code. The dimensional analysis in
//! `docs/research/440-dimensionless-groups.md` (smell S1) names them; this
//! module makes them explicit, citable, and used at the conversion site.
//!
//! Every anchor is `1.0` in today's units. Naming them changes no number and
//! no trajectory (guarded by `tests/trait_unit_anchors.rs`). Whether any of
//! them should become a `WorldParameters` field is a later design decision,
//! not one this module makes — see `world-rules.md`, *Unit anchors*.
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
