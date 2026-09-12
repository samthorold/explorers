# Issue #440 — formal viability E1: dimensionless groups (Buckingham-π) over the committed rules

**Status: research finding. Commits nothing.** This note assigns units to every field of
`WorldParameters` and `InitialDistribution`, runs the Buckingham-π reduction over the
committed functional forms, states the count of independent dimensionless groups against
the dimensions of the genesis search box, and rewrites every gate and bound in
[`viability.md`](../system-design/viability.md) and the A1/A2/B1 research notes in those
groups. It introduces no mechanism, functional form, or parameter; it changes nothing in
the stepper, the evaluator, or the search. The one promoted artefact is a short
"Dimensionless groups" section in `viability.md` carrying the unit-table summary and the
group list. Three dimensional-homogeneity smells are named here and filed separately; a
π-space search is filed as deferred work.

Units are read from the code that realises each rule (`crates/explorers-sim/src/phase.rs`,
`lib.rs`, `spatial.rs`; the prefilter in `crates/explorers-search/src/prefilter.rs`) where
[`world-rules.md`](../system-design/world-rules.md) is silent on a coefficient's units.

## TL;DR

1. **Four base units** suffice: energy `E`, nutrient `N`, length `L`, and the tick `T`.
   Traits are dimensionless numbers; counts are dimensionless. Every field of
   `WorldParameters` and `InitialDistribution` is in the unit table below.
2. **Rank 4, `k = 49`.** Of the 44 world-parameter fields, one is dead (the legacy
   `somatic_maintenance_cost_coefficient`, read by no phase) and one cannot be assigned
   consistent units (`use_wear_rate`, smell S2); the remaining 42 plus the 10
   initial-distribution fields are 52 dimensionally homogeneous quantities. The tick is
   committed (`Δt = 1`), so the time scale is not free; three fields — `base_metabolic_rate`
   `B`, `world_extent` `L`, `reproduction_nutrient_threshold` `N_r` — set the energy,
   length and nutrient scales. The committed map therefore depends on at most
   **`k = 52 − 3 = 49` independent dimensionless groups**: 23 fields that are already
   dimensionless, and 26 ratios built on `(B·τ, L, N_r, τ)`.
3. **Search box: 32 raw dimensions, 29 intrinsic groups.** Fifteen of the 32 searched
   parameters are dimensional; twelve intrinsic ratios can be formed among them, and with
   the 17 already-dimensionless searched fields that is 29 groups. The remaining three
   searched directions are the energy scale (`B`), the length scale (`L`) and the nutrient
   scale (`base_nutrient_ratio` against the fixed `N_r`). Under dimensionally homogeneous
   rules these would be pure redundancy — invisible to the map — and the box would be
   29-dimensional. Under the *committed* rules they are physical, but only through ratios
   to four un-searched baseline constants (`asexual_propensity_maintenance_cost`,
   `reproduction_nutrient_threshold`, `nutrient_grid_cell_size`,
   `dispersal_reach_coefficient`) and through the hidden unit anchors of smell S1.
4. **Every gate is expressible in the groups.** Extinction: `π_F = F/B ≤ 1`. Energy
   death: `π_N ≥ π_ρ · ŝ_min · (1 + π_ρs · σ_min) + 1`. A1: `ρ = π_F / b_P`,
   `I = κ_C·γ·e·ĥ_C·π_F / (b_P·b_C)`. A2: `Λ = ĥ_C·ι·(π_N/ν̂)·min(κ_C·γ·e_C, q/θ_C) / b_C`.
   B1: `N̄_max = π_F·m²`, `m = ⌊√2/π_r⌋ + 1`. Two of these carry a named offending
   constant: the prefilter's energy-death floor uses `STRUCTURE_MIN = 1.0` energy with no
   committed source (smell S3), and A1's `I` and A2's `Λ` are dimensionless only because
   the heterotrophy drain anchor `u_H` is silently `1 E/T` per trait unit (smell S1).
5. **Three smells, one deferral.** S1 — five trait-to-flow conversions carry implicit
   unit constants equal to 1 (mobility → distance, dispersal → kernel width, autotrophy →
   nutrient uptake, heterotrophy → structure drain, kappa → repair), so the trait unit is
   secretly pinned to `L/T`, `L`, `N/T`, `E/T` and `E/T` at once and trait-space distance
   `d` sums quantities of different dimension. S2 — `use_wear_rate` multiplies energy
   captured, energy drained *and* distance moved with one coefficient. S3 — the
   energy-death prefilter's `STRUCTURE_MIN = 1.0` is an energy constant the design never
   committed. Deferred — search the atlas in π-space.

## Conventions

- **Base units.** `E` energy, `N` nutrient, `L` length, `T` tick. `1` marks a
  dimensionless quantity; `count` is dimensionless but integer-valued.
- **The tick is committed.** The execution model fixes `Δt = 1 tick`; there is no
  half-tick and no other time constant. "Per-tick fractions" (`reserve_mobilisation_rate`,
  `structure_maintenance_coefficient`, `network_redistribution_rate`) are written `1/T`
  so that the unit table is honest about what they multiply; numerically they are
  already dimensionless because `τ = 1`. Per-event probabilities (`mutation_rate`,
  `asexual_propensity`) are `1`.
- **Traits are dimensionless** in `[0, 1]`-ish, per [`trait-space.md`](../system-design/trait-space.md).
  Where the stepper uses a trait *as* a dimensional quantity, the table says so and the
  hidden constant is named in smell S1. Trait-space distance `d` and the trait
  thresholds compared to it (`reproductive_compatibility_distance`, `1/trophic_distance_decay`,
  `mutation_magnitude`, `trait_covariance`) are in trait units, i.e. `1`.
- **Reference scales** for the groups: `τ = 1 tick`; energy `ε := B·τ` (one tick of base
  metabolism — the one cost every agent pays, and the denominator of the extinction gate);
  length `L` (the extent, the only length every other length is compared to on the torus);
  nutrient `N_r := reproduction_nutrient_threshold` (the nutrient the reproduction gate
  reads). A hat marks a quantity in these units: `x̂ = x/ε`, `x/L`, `x/N_r`, `x·τ`.
- **Structure is energy.** `structure` is embodied energy (world rules, *Energy stocks*),
  so anything "per unit structure" is per `E`.

## Unit table — `WorldParameters`

Forty-four fields, in struct order (`crates/explorers-sim/src/lib.rs`). The "form"
column is the committed use the unit is read from; the "group" column names the
dimensionless group the field contributes (defined in the next section).

| # | field | units | form it is read from | group |
|---|---|---|---|---|
| 1 | `solar_flux_magnitude` `F` | `E/T` | income = `F · w_i/W_i` per light neighbourhood (flow 1; `photosynthesise`) | `π_F = F/B` |
| 2 | `base_trophic_efficiency` | `1` | efficiency ceiling (flow 7) | itself |
| 3 | `trophic_distance_decay` `λ` | `1` (per trait-distance) | `exp(−λ·d)`, `d` in trait units | itself |
| 4 | `reproduction_efficiency` | `1` | fraction of investment received (flow 4) | itself |
| 5 | `base_metabolic_rate` `B` | `E/T` | charged every tick (flow 8; `metabolise`) | **energy scale** |
| 6 | `movement_cost_coefficient` `c_mv` | `1/L` | `cost = distance · c_mv · structure` (`move_agents`): `L · (1/L) · E = E` per tick | `π_mv = c_mv·L` |
| 7 | `sensing_range_coefficient` `s_c` | `L` (per trait unit) | `sensing = eff_mobility · s_c` (`move_agents`) | `π_sn = s_c/L` |
| 8 | `reproduction_energy_threshold` `E_r` | `E` | allocation `≥ E_r` (flow 4) | `π_Er = E_r/ε` |
| 9 | `reproduction_nutrient_threshold` `N_r` | `N` | earmark `≥ N_r` (flow 4) | **nutrient scale** |
| 10 | `mutation_rate` | `1` | per-dimension probability per birth | itself |
| 11 | `mutation_magnitude` | `1` (trait units) | `Normal(0, magnitude)` added to a trait | itself |
| 12 | `contact_range_coefficient` | `L` (per trait unit) | `reach = eff_het · (contact + body·√structure)` (flow 3; `consumption_reach`) | `π_c = contact/L` |
| 13 | `world_extent` `L` | `L` | torus side | **length scale** |
| 14 | `initial_population_size` | `count` | founders at tick 0 | itself |
| 15 | `light_competition_radius` `r` | `L` | strict `< r` neighbourhood (flow 1) | `π_r = r/L` |
| 16 | `photo_maintenance_cost` `c_a` | `E/T` (per trait^p) | `c_a · α^p` per tick (flow 8) | `π_a = c_a/B` |
| 17 | `heterotrophy_maintenance_cost` `c_h` | `E/T` (per trait^p) | `c_h · h^p` per tick | `π_h = c_h/B` |
| 18 | `initial_nutrient_pool` `N_total` | `N` | total conserved nutrient at genesis | `π_N = N_total/N_r` |
| 19 | `growth_efficiency` `γ` | `1` | reserve → structure conversion (flow 9) | itself |
| 20 | `wear_rate` | `E/T` (per trait unit) | baseline wear `= wear_rate · nominal` per tick; wear is repaired 1:1 with energy (`grow`), so wear is `E` | `π_w = wear_rate/B` |
| 21 | `wear_degradation_steepness` `k` | `1/E` | `effective = nominal · exp(−k · wear)` | `π_k = k·ε` |
| 22 | `somatic_maintenance_cost_coefficient` | — | **dead**: legacy serde field, read by no phase | none |
| 23 | `use_wear_rate` | **inhomogeneous** | multiplies energy captured (`ft 0`), energy drained (`ft 1`) *and* distance moved (`ft 2`) (`apply_wear`) | **smell S2** |
| 24 | `structure_maintenance_coefficient` `c_s` | `1/T` | `c_s · structure` per tick (flow 8): `E` per `E` per tick | `π_s = c_s·τ` |
| 25 | `repair_decay` | `1/E` | `repair = kappa · exp(−decay · wear)` (`grow`) | `π_rd = repair_decay·ε` |
| 26 | `base_nutrient_ratio` `ρ_b` | `N/E` | `demand = structure · (ρ_b + ρ_s · σ)` (`stoichiometric_demand`) | `π_ρ = ρ_b·ε/N_r` |
| 27 | `specification_nutrient_coefficient` `ρ_s` | `N/E` (per trait unit) | as above, `σ = α + h + μ` | `π_ρs = ρ_s/ρ_b` |
| 28 | `reproductive_compatibility_distance` | `1` (trait units) | `d ≤ distance` gates mating (flow 4) | itself |
| 29 | `mobility_maintenance_cost` `c_μ` | `E/T` (per trait^p) | `c_μ · μ^p` per tick | `π_μ = c_μ/B` |
| 30 | `maintenance_cost_exponent` `p` | `1` | exponent on each trait | itself |
| 31 | `nutrient_grid_cell_size` | `L` | cells of side `cell`; `N_total` spread uniformly over `⌈L/cell⌉²` cells (`NutrientGrid::new`) | `π_cell = cell/L` |
| 32 | `growth_retention_multiplier` | `T` | `buffer = metabolic_cost · multiplier`: `(E/T) · T = E` (flow 9) | `π_ret = mult/τ` |
| 33 | `reserve_mobilisation_rate` `f` | `1/T` | `mobilised = f · (reserve − buffer)` per tick (flow 9) | `π_f = f·τ` |
| 34 | `offspring_structure_fraction` | `1` | share of per-offspring energy committed to structure | itself |
| 35 | `asexual_propensity_maintenance_cost` `c_x` | `E/T` (per trait^p) | `c_x · asexual^p` per tick | `π_x = c_x/B` |
| 36 | `dispersal_propagule_cost_coefficient` | `1` (per trait^exp) | `fraction = coeff · dispersal^exp` of the reproductive budget (flow 4) | itself |
| 37 | `dispersal_propagule_cost_exponent` | `1` | exponent on dispersal | itself |
| 38 | `dispersal_reach_coefficient` | `L` (per trait unit) | `mate reach = eff_mobility · s_c + dispersal · coeff` | `π_dr = coeff/L` |
| 39 | `body_reach_coefficient` | `L/E^½` (per trait unit) | `body · √structure` inside feeding reach | `π_body = body·√ε/L` |
| 40 | `network_connection_cap` | `count` | connections per builder (flow 5) | itself |
| 41 | `network_creation_cost` | `E` | paid once from surplus | `π_nc = cost/ε` |
| 42 | `network_maintenance_cost` | `E/T` | paid per connection per tick | `π_nm = cost/B` |
| 43 | `network_redistribution_rate` | `1/T` | `flow = rate · (donor − recipient)` per tick | `π_nr = rate·τ` |
| 44 | `network_transfer_efficiency` | `1` | received fraction of energy sent | itself |

## Unit table — `InitialDistribution`

| field | units | form it is read from | group |
|---|---|---|---|
| `mean_traits.photosynthetic_absorption` | `1` (trait) | founder centroid; light weight `α · structure` (flow 1); **also** nutrient uptake demand `= eff_α` per tick (`absorb_nutrients`) — anchor `u_A = 1 N/T` per trait unit (S1) | itself |
| `mean_traits.heterotrophy` | `1` (trait) | founder centroid; per-target drain `= eff_h` per tick in structure units (`resolve_drains`) — anchor `u_H = 1 E/T` per trait unit (S1) | itself |
| `mean_traits.mobility` | `1` (trait) | founder centroid; `distance moved = eff_μ` per tick (`move_agents`) — anchor `u_M = 1 L/T` per trait unit (S1) | itself |
| `mean_traits.kappa` | `1` | somatic fraction of mobilised surplus; **also** `base_repair = kappa` energy per tick per functional trait (`grow`) — anchor `u_R = 1 E/T` per unit kappa (S1) | itself |
| `mean_traits.fecundity` | `count` | Poisson mean offspring per event | itself |
| `mean_traits.asexual_propensity` | `1` | per-tick probability | itself |
| `mean_traits.dispersal` | `1` (trait) | `σ` of the offspring placement kernel `Normal(0, dispersal)` — anchor `u_D = 1 L` per trait unit (S1); also `dispersal^exp` in the propagule cost and `dispersal · dispersal_reach_coefficient` in mate reach | itself |
| `trait_covariance` | `1` (trait units) | standard deviation of the founder draw `Normal(0, trait_covariance)` per dimension (`World::new`); a scalar σ, not a covariance matrix — a misnomer, not a unit problem | itself |
| `initial_cluster_count` | `count` | trophic centroids at genesis | itself |
| `initial_energy_per_agent` `E_0` | `E` | founder endowment, provisioned as reserve + structure (`provision_initial_reserve_structure`) | `π_E0 = E_0/ε` |

## Buckingham-π

**Quantities.** 52 dimensionally homogeneous fields (42 world parameters after excluding
the dead #22 and the inhomogeneous #23; 10 initial-distribution fields) plus the committed
tick `Δt = 1 T`.

**Dimension matrix.** Rows `(E, N, L, T)`; the four base units all appear independently:
`E` alone (`E_r`, `E_0`, `network_creation_cost`), `N/E` (`ρ_b`, `ρ_s`) and `N` alone
(`N_r`, `N_total`), `L` alone (five lengths) and `1/L` (`c_mv`), `T` through every rate.
The matrix has **rank 4**.

**Count.** Buckingham gives `(52 + 1) − 4 = 49` independent groups. Equivalently: the
tick fixes the time scale, and three fields fix the other three scales (`B` for energy,
`L` for length, `N_r` for nutrient), leaving `52 − 3 = 49`. They decompose as:

- **23 fields already dimensionless** — #2, #3, #4, #10, #11, #14, #19, #28, #30, #34,
  #36, #37, #40, #44, the seven trait means, `trait_covariance`, `initial_cluster_count`.
- **26 constructed groups**, on the reference scales `(ε = B·τ, L, N_r, τ)`:

| axis | groups |
|---|---|
| energy rates over `B` (7) | `π_F = F/B`, `π_a = c_a/B`, `π_h = c_h/B`, `π_μ = c_μ/B`, `π_x = c_x/B`, `π_w = wear_rate/B`, `π_nm = network_maintenance_cost/B` |
| energy stocks over `ε` (3) | `π_Er = E_r/ε`, `π_E0 = E_0/ε`, `π_nc = network_creation_cost/ε` |
| inverse energies times `ε` (2) | `π_k = k·ε`, `π_rd = repair_decay·ε` |
| per-tick rates times `τ` (3) | `π_s = c_s·τ`, `π_f = f·τ`, `π_nr = network_redistribution_rate·τ` |
| times over `τ` (1) | `π_ret = growth_retention_multiplier/τ` |
| lengths over `L` (5) | `π_r = r/L`, `π_c = contact_range_coefficient/L`, `π_sn = sensing_range_coefficient/L`, `π_dr = dispersal_reach_coefficient/L`, `π_cell = nutrient_grid_cell_size/L` |
| mixed length (2) | `π_mv = c_mv·L`, `π_body = body_reach_coefficient·√ε/L` |
| nutrient (3) | `π_N = N_total/N_r`, `π_ρ = ρ_b·ε/N_r`, `π_ρs = ρ_s/ρ_b` |

Readings: `π_F` is flux in base-metabolisms; `π_Er` and `π_E0` are the reproduction
threshold and the founder endowment in *ticks of base metabolism*; `π_s` is the fraction of
the body paid per tick; `π_ret` is the buffer in ticks; `π_r` sets the light-neighbourhood
count `m² ≈ 2/π_r²` of B1; `π_mv` is the fraction of the body spent to cross the world;
`π_N` is the total nutrient in reproduction earmarks; `π_ρ` is the nutrient bound in one
tick's worth of base-metabolic body, in earmarks.

**Against the search box.** `default_ranges()` (`crates/explorers-search/src/search.rs`)
searches **32** dimensions: 15 dimensional (`F, B, c_a, c_h` in `E/T`; `E_r, E_0` in `E`;
`ρ_b, ρ_s` in `N/E`; `c_mv` in `1/L`; `s_c, contact, L, r` in `L`; `mult` in `T`; `f` in
`1/T`) and 17 dimensionless (#2, #3, #4, #10, #11, #14, #28, #30, #34, six trait
means — `fecundity` is not searched — `trait_covariance`, `initial_cluster_count`). Within the box the dimensional
fifteen form 12 intrinsic groups (`π_F, π_a, π_h, π_Er, π_E0, π_ρs, π_mv, π_sn, π_c, π_r,
π_ret, π_f`), so the box spans **29 intrinsic groups**. Its other three directions are the
scales `B`, `L` and `ρ_b`:

- under homogeneous rules they would be null — scaling every energy, every length, or
  every nutrient together leaves the map invariant, and the box would be 29-dimensional
  with three wasted axes;
- under the committed rules they act, but only through ratios to un-searched baseline
  constants — `π_x = c_x/B` (`c_x = 0.01` fixed), `π_ρ = ρ_b·ε/N_r` (`N_r = 1` fixed),
  `π_cell = cell/L` (`cell = 10` fixed), `π_dr = coeff/L` (`coeff = 10` fixed) — and
  through the hidden anchors of S1 (`u_M·τ/L`, `u_D/L`, `u_A·τ/N_r`, `u_H·τ/ε`), which pin
  the trait unit to each scale.

So the box is honestly 29 intrinsic knobs plus three scale ratios that the design never
chose to expose as ratios. That is the finding the deferred π-space issue acts on. (The
un-searched, non-default network, wear and body-reach groups are all zero on the
baseline; they add nothing to the box's effective dimension.)

## Gates and bounds in π

Notation for per-cluster lumpings, as in A1/A2: `b_X = B_X/B` is a body's maintenance in
base-metabolisms, `B_X = B + c_a·α_X^p + c_h·h_X^p + c_μ·μ_X^p + c_s·M_ref`, so
`b_X = 1 + π_a·α_X^p + π_h·h_X^p + π_μ·μ_X^p + π_s·m̂_ref` with `m̂_ref = M_ref/ε`;
`ĥ_C = h_C·u_H/B` is the consumer's per-target drain in base-metabolisms;
`e = base_trophic_efficiency·exp(−λ·d)`.

**Extinction (viability.md, form-free).** `F ≤ B` ⟺

> `π_F ≤ 1`.

**Energy death (viability.md, fully pinned).**
`N_total ≥ S_min·(ρ_b + ρ_s·σ_min) + N_r` ⟺

> `π_N ≥ π_ρ · ŝ_min · (1 + π_ρs·σ_min) + 1`, with `ŝ_min = S_min/ε`.

The gate as written is scale-free once `S_min` is a committed energy. It is not: B1 shows
no minimal body exists under the peak-relative death threshold, and the prefilter
(`fails_energy_death_gate`) substitutes `STRUCTURE_MIN = 1.0` — one unit of energy with
no committed source — so the implemented floor is `π_N ≥ π_ρ/ε·(…) + 1`, i.e. it moves
with `B` for no physical reason. **Offending constant: the prefilter's `STRUCTURE_MIN`
(smell S3).** With `S_min → 0` the gate reduces to `π_N ≥ 1` (`N_total ≥ N_r`), which is
what the committed rules actually support.

**A1 permanence (`432-permanence-pc.md`).** Clause (1) `ρ = F/B_P > 1` ⟺

> `ρ = π_F / b_P > 1`

— the extinction gate sharpened by the searched maintenance groups (`b_P = 1` is the
corner `π_F ≤ 1`). Clause (2) `I = β·K_P/m = κ_C·γ·e·h_C·F/(B_P·B_C) > 1` ⟺

> `I = κ_C · γ · e · ĥ_C · π_F / (b_P · b_C) > 1`.

`I` is dimensionless in raw parameters only because `h_C` is read as a drain of
`u_H = 1 E/T` per trait unit — the anchor is load-bearing (S1). The `κ_P` factor in
`r_P = κ_P·γ·(F − B_P)` that #439 flags as a lumping error is a pure number and does not
affect the dimensional form; its consequence — the spurious conjunct `κ_P > 0` on clause
(1) — is inherited unchanged and is A4's to fix, not this note's.

**A2 lockup repeller (`437-permanence-alc.md`).**
`Λ = (h_C·ι/ν) · N_total · min(κ_C·γ·e_C, q/θ_C) / B_C > 1` ⟺

> `Λ = ĥ_C · ι · (π_N / ν̂) · min(κ_C·γ·e_C, q/θ_C) / b_C > 1`, `ν̂ = ν/N_r`.

`ι` is a geometry fraction (`1`); `q` and `θ_C` are both `N/E`, so their ratio is
homogeneous; `ν` is nutrient per carcass (`N`). Same anchor dependence as `I`.

**B1 energy bound (`433-energy-bound.md`).** Lemma 1: `P_max = F·m²`,
`m = ⌊√2·L/r⌋ + 1` ⟺

> `P̂_max = π_F · m²`, `m = ⌊√2/π_r⌋ + 1`.

Theorem: `N̄_max = F·m²/B` ⟺ **`N̄_max = π_F · m²`** — already a pure number, as a
count must be. Conditional bound under the named (uncommitted) holding cost `h`:
`E_living ≤ max(E_living(0), P_max/h')` ⟺ `Ê_living ≤ max(π_E0·N_0, π_F·m²/(h'·τ))`, with
`h'·τ = min(π_s, h·τ)`. Transient `t_0 = E_tot(0)/(B·ε_tol)` ⟺ `t_0/τ = Ê_tot(0)/ε̂_tol`.

**Decomposer-return floor and `C*` (viability.md, characterisations).**
`base·(reachable carcass energy per tick) ≥ B + maintenance(body)` ⟺
`base·R̂ ≥ b_D` with `R̂` the reachable carcass energy per tick in base-metabolisms; and
`C* = death_flux/decomp_turnover` (`N`) ⟺ `Ĉ* = C*/N_r`, with the lockup condition
`π_N − L̂_min − Ĉ* ≥` the energy-death floor above. Both stay characterisations for the
reasons viability.md gives; nondimensionalising changes nothing about their authority.

## Dimensional-homogeneity smells

**S1 — hidden unit anchors in trait-to-flow conversions.** Five committed forms use a
dimensionless trait *as* a dimensional quantity with an implicit constant of 1:

| conversion | code | implicit constant |
|---|---|---|
| distance moved per tick `= eff_mobility` | `move_agents` | `u_M = 1 L/T` per trait unit |
| offspring kernel `σ = dispersal` | `resolve_reproduction` (both modes) | `u_D = 1 L` per trait unit |
| nutrient uptake demand `= eff_autotrophy` per tick | `absorb_nutrients` | `u_A = 1 N/T` per trait unit |
| structure drained per target per tick `= eff_heterotrophy` | `resolve_drains` | `u_H = 1 E/T` per trait unit |
| repair per functional trait per tick `= kappa · exp(−decay·wear)` | `grow` | `u_R = 1 E/T` per unit kappa |

Consequences: the trait unit is pinned simultaneously to `L/T`, `L`, `N/T`, `E/T`, so the
Euclidean trait-space distance `d` (which drives `e`, mating compatibility and the
branching descriptor) adds a length-like coordinate (`mobility`, `dispersal`) to
energy-rate-like ones (`heterotrophy`) and pure fractions (`kappa`, `asexual_propensity`);
and no rescaling of `E`, `L` or `N` is a symmetry of the map even where every explicit
parameter scales along. `world-rules.md` does not state these constants; `trait-space.md`
calls traits dimensionless. Filed as #459 — resolved by naming the five anchors as explicit constants
(`crates/explorers-sim/src/units.rs`, each `1.0`) and documenting them in `world-rules.md`
(*Unit anchors*) and `viability.md` (*Dimensionless groups*); no number changed. Whether to
parameterise any anchor, and what `d` then means, remains a system-design decision.

**S2 — `use_wear_rate` mixes energy and length.** `apply_wear` computes
`use_rate · usage[ft]` with `usage = [energy captured, energy drained, distance moved]`.
One coefficient is therefore `wear/E` for autotrophy and heterotrophy and `wear/L` for
mobility; no single unit satisfies both. The field is `0` on every committed recipe and
on the search baseline, so nothing runs on it today — it is a smell in the rule, not a
live defect. Filed as #460 (`needs-triage`).

**S3 — the energy-death prefilter's `STRUCTURE_MIN = 1.0` is an uncommitted energy
constant.** `viability.md` writes the gate with a symbolic `structure_min`; B1 proves no
minimal viable body exists under the peak-relative death threshold; the prefilter fills
the symbol with `1.0` energy. The gate is therefore not scale-free (`ŝ_min = 1/ε`), and
the only value the committed rules support is `S_min → 0`, i.e. `N_total ≥ N_r`. Filed
as #461 (`needs-triage`); the fix is a decision about the gate (drop the term, or commit a
floor), not a physics change.

Two non-smells recorded for completeness: `somatic_maintenance_cost_coefficient` is dead
(read by no phase; retained for serde compatibility), and `trait_covariance` is a
standard deviation, not a covariance — a naming issue with no dimensional content.

## Deferred

**Search the atlas in π-space.** The box spends three of its 32 axes on scale directions
that are meaningful only through ratios to un-searched constants and hidden anchors
(above). A π-space search would parameterise the 29 intrinsic groups directly (plus the
three scale ratios explicitly, if kept), so that CMA-MAE's covariance adapts along the
axes the physics actually reads and every atlas cell is portable across energy, length
and nutrient scales. Filed as #462 (`deferred`); it depends on S1 (#459) being decided,
since the hidden anchors are what make the raw scales physical today.

## Deliverables against the acceptance criteria

- Every world-parameter and initial-distribution field appears in the unit table — 44 + 10
  rows above, including the dead field and the inhomogeneous one.
- Independent groups listed with `k` against the searched count — `k = 49` over the
  committed rules; the 32-dimensional box spans 29 intrinsic groups plus three scale
  directions.
- Every existing gate is expressed in the groups — extinction, energy death, A1 `ρ`/`I`,
  A2 `Λ`, B1 `P_max`/`N̄_max`/conditional `E_max`, the decomposer-return floor and `C*`;
  the two offending constants (`STRUCTURE_MIN`, the `u_H` anchor) are named and filed.
- `viability.md` gains one short "Dimensionless groups" section — the unit summary and
  the group list; no mechanism, form, or parameter added.
- Deferred π-space search issue filed (#462); three smells filed `needs-triage`
  (#459, #460, #461).
- No code change.
