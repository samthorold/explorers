//! Unit anchors: trait-to-flow conversions (#459).
//!
//! Traits are dimensionless (`docs/system-design/trait-space.md`), yet four
//! committed forms read a trait *as* a dimensional flow. Each such conversion
//! carries a unit constant that was previously implicit — a bare `= trait` in
//! the phase code. The dimensional analysis in
//! `docs/research/440-dimensionless-groups.md` (smell S1) names them; this
//! module makes them explicit, citable, and used at the conversion site.
//!
//! Use-dependent wear and repair need no anchor (#763): every use-wear
//! coupling is energy put through a machine, so `use_wear_rate` is a pure
//! number, and repair `ρ · κ · w` is a rate on wear, so `ρ` carries the `1/T`.
//!
//! Every anchor is `1.0` in today's units. Naming them changes no number and
//! no trajectory (guarded by `tests/sim/trait_unit_anchors.rs`). Whether any of
//! them should become a `WorldParameters` field is a later design decision,
//! not one this module makes; see `world-rules.md`, *Unit anchors*.
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
/// effective photosynthetic absorption, and with hyphal uptake on (#727) per
/// unit of contact × effective heterotrophy as well: one anchor for one
/// absorptive surface. Units: `N/T` per trait unit. Read in
/// `phase::nutrient_uptake_demand_after_move`.
pub const AUTOTROPHY_NUTRIENT_UPTAKE_PER_TICK: f32 = 1.0;

/// `u_H`: structure drained from each in-reach target per tick per unit of
/// effective heterotrophy. Units: `E/T` per trait unit (structure is embodied
/// energy). Read in `phase::resolve_drains` for live targets and carcasses.
pub const HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK: f32 = 1.0;
