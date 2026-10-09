# Issue #757: inventory of departures from the plain physics

**Status: read-only inventory, docs only (2026-10-09). It decides nothing.** It lists the
mechanisms, switches, pinned parameters, constants, instruments and design-doc history blocks that
an audit against the purpose-first design ([README](../system-design/README.md), *Purpose before
structure*; [reference modes](../system-design/reference-modes.md)) should look at, with what each
was for. Every **class** below is a *suggestion* for the owner's grill:

- **keep**: serves a reference mode or a load-bearing loop, or is a faithful domain flow;
- **delete**: remove the code and the switch together;
- **hold**: tied to a named future mode or an open decision (e.g. the network for mutualism);
- **re-ground**: it belongs in the physics but is switched off or pinned, like wear.

For design-doc history blocks (section 6) the classes read as **move** (to `docs/research/`),
**trim** (to a pointer) or **keep** (it states design rationale).

**How sizes were measured.** Rust LOC are `wc -l` of whole files, or line ranges of the
non-test functions that carry a mechanism (function extents from a `^fn`…`^}` scan, branch lines
counted by eye); test lines are excluded unless stated. Design-doc sizes are in characters where a
paragraph is one physical line (world-rules, viability, trait-space, expected-properties), with a
rough 100-column line equivalent (`~Nl`). All sizes are approximate.

**How origins were found.** `git log -S <symbol>` for the oldest commit, then
`gh issue view N --json title,state`. Where the number found is a PR rather than an issue, or the
attribution rests on a single commit, the row says *uncertain*.

## Summary

| Category | Rows |
|---|---|
| 1. Default-off switches and latent mechanisms | 11 |
| 2. Physics parameters pinned or held outside the search box | 16 |
| 3. Mechanisms reversed or left vestigial | 14 |
| 4. Constants with no stated derivation | 24 |
| 5. Instruments (binaries 24, instrument-only modules 14, census sections 31) | 69 |
| 6. Measurement history inline in the design docs (world-rules 21, genesis-search 16, viability 9, others 8) | 54 |

### Suggested *re-ground* (input to the grill)

- **Somatic wear** (`wear_rate`, with `repair_decay` and `wear_degradation_steepness` inert while it
  is 0): 0 in the search baseline and recipe, against world-rules *Wear is always on*. Tracked as
  **#756**. Also `genesis-search.md` §*Predicted bifurcation coordinates* still says "wear is off for
  every searched config" as #359's validity condition.
- **Use-wear** (`use_wear_rate`): 0 everywhere; the design has it, its unit inhomogeneity is an open
  decision. Rides with #756.
- **Mobility maintenance cost** (`mobility_maintenance_cost`): 0 everywhere, although the design
  prices each capability's maintenance and routes sensing cost through it. Autotrophy–mobility
  incompatibility then rests on movement cost alone (642 note); see open #648.
- **Dispersal propagule cost** (`dispersal_propagule_cost_coefficient`): 0 everywhere, although
  world-rules describes it as charged; wider dispersal is free (593 note).
- **Body reach** (`body_reach_coefficient`): 0 everywhere; the term is in the flow-3 reach formula.
  Weaker case: the design treats it as a reserve lever and viability.md rejects pinning it above 0.
- **Serde defaults that disagree with the baseline**: `wear_rate` (default 0.1 vs 0),
  `use_wear_rate` (0.01 vs 0), `dispersal_reach_coefficient` (0 vs 10). A recipe omitting a field
  gets different physics from a searched world.

### Suggested *delete* (input to the grill)

Code and parameters:
- `somatic_maintenance_cost_coefficient`: a legacy parameter no stepper code reads (app slider only).
- The cross-trait cost `c_AH` (*delete or hold*): latent at 0 since #716; #670 found it does not
  deliver flow 1's tests. `taxed_ranges()` survives only to decode #677's atlas.
- Dead code in the sim: `#[allow(dead_code)] fn emit` (#61), `Agent::effective_trait` (no
  non-test caller), `resolve_drains` without deficits (tests only), `sensing_throughput` /
  `detected_count` (read only by a proptest).
- The SoA spike `soa.rs` (380) with `bench_soa` (147): #354's verdict was "the floor isn't there",
  yet the module is kept in parity by `tests/soa_differential.rs`.
- Serde back-compat aliases for renamed fields (#127, #191, #196, #198, #220) and the
  withdrawn-`satiation_sensitivity` load test, if old recipes need not load.
- The dead `bayesopt.rs` / `gp.rs` / `sobol.rs` modules (521 LOC, #11): only `pub mod` references.

Instruments whose reading is closed (bin LOC, plus modules only they use):
- `energy_bound_check` (1141, #433), `energy_death_check` (879, #508), `guild_census` (656, #494),
  `guild_structure` (862, #546), `heterotroph_margin` (553 + 333, #589), `invasion_growth`
  (1862, #443), `radius_sweep` (649, #462), `reinvasion_barrier` (1941 + `energy_accounting` 860,
  #587/#591/#593), `role_emergence` (966, #421), `branching_detector` (1083, #359),
  `hopf_prototype` (587, #358), `bench_query_radius` (337, #423; delete or hold),
  `step_profile` (106, #423; delete or hold).
- `role_diet_census` (1358, #596 and the gate readings) and its census sections: every reading it
  served is closed. Its library modules (`role_diet`, `intake_ceiling`, `fullness`,
  `grazer_hunger`, ~5,600 LOC) are entangled with the live `kin_killer_diet` census, so they are
  *hold* until that census's few uses are cut loose.
- Closed `kin_killer_diet` sections I, J, K (#668/#681), and 2, 3, D (#642).

Docs:
- Stale descriptions of withdrawn gates: `viability.md` dimensionless groups still list the need
  gate `1/(1 + π_sat·s)` and the intake-ceiling `k`, `τ`; `CONTEXT.md` *Intake ceiling* says the
  multiple "is a world parameter searched by genesis"; *Fullness* and *clearance time* describe a
  gate that never landed (#630 closed, withdrawn by PR #679).
- The largest history block in any design doc: the withdrawn fullness-gate section of
  `world-rules.md` (*Capability and expression*, ~22k chars), plus ~45 smaller *Measured* /
  *Current state* blocks to move to `docs/research/` (section 6).

## 1. Default-off world switches and latent mechanisms

| Mechanism (fields) | Location | Origin | Purpose | State | Sim LOC | Measured verdict | Suggested |
|---|---|---|---|---|---|---|---|
| Network, flow 5 (`network_connection_cap` as master switch, `network_creation_cost`, `network_maintenance_cost`, `network_redistribution_rate`, `network_transfer_efficiency`) | sim `lib.rs:573-600`; `phase.rs` `maintain_connections`, `form_connections`, `redistribute`, `PartnerPayments`, `route_or_excrete`; `lib.rs` `Connection`, step block, `seed_connection`, `prune_dead_connections` | Epic #249; slices #409–#413 (exact slice range uncertain); routed surplus #738; farthest-first formation #739 | Trait-free mycorrhizal links that level energy and nutrient down gradients and route excreted nutrient to paying partners | **Off**: all 0 in recipe and baseline; a test pins it out of the box (`network_parameters_are_not_searched`); instruments override with `--network-*` | ~400 (+ ~980 test lines) | 646: levels nutrient among producers and costs persistence; 736 (#743): ECM reading fails items 3–4, re-deferred | **hold** (mutualism, the named next mode; #736 open) |
| Hyphal uptake (`hyphal_uptake` bool, `contact_distance`) | `lib.rs:650-671`; `phase.rs` `substrate_contact`, branch in `nutrient_uptake_demand_after_move`, move-distance plumbing | #727 (impl); decision #654 open | Heterotrophs take pool nutrient, scaled by substrate contact | **Off** (false); `contact_distance` inert, never searched; absent from recipe (serde fills it) | ~25 | 654 (#730): item 1 fails, 39 of 73; re-deferred | **hold** (#654, prerequisite of the partnership) |
| Size-scaled uptake (`uptake_structure_exponent` b, `uptake_reference_structure` s_ref) | `lib.rs:615-629`, const at 686; `phase.rs` `nutrient_uptake_demand_after_move` | #644; into the box by #653 | Uptake demand scales as (structure/s_ref)^b, like root mass | **On**: b searched over [0, 1], recipe 0.37; s_ref fixed at 100 outside the box | ~5 | 645: put b in the box; does not end producers' hold on the decomposer niche | **keep** (faithful flow; s_ref see §4) |
| Cross-trait cost `c_AH` (`cross_trait_cost`) | `lib.rs:630-639`; `phase.rs` `with_cross_trait_cost`, `cross_trait_charge`; `soa.rs` | #667; into box #669; out of box #701/#716 | Superlinear maintenance on mixotrophs (trade-off 5) | **Latent**: 0 in recipe and baseline; `taxed_ranges()` [0, 0.14] kept only to decode #677's atlas | ~16 (+ `taxed_ranges`) | 668: c_max 0.14; 670: does not deliver flow 1's tests | **delete** or **hold** (owner's call; the design names trade-off 5) |
| Recognition (`recognition_distance` D, `RECOGNITION_RESTRAINT` = 1) | `lib.rs:601-614`; `phase.rs:339-368`, call in `resolve_drains_routed` | #604 (PR #615); restraint fixed by #684 | Spare living targets that resemble the consumer | **On**, D = 0.5 fixed outside the box (#725) | ~26 | 606: live counts barely move; generalist dominance rises; within-cluster predation does not fall | **keep** (live rule) — but its 606 verdict is weak; grill whether it earns its place |
| Body reach (`body_reach_coefficient`) | `lib.rs:558-572`; one term in `phase.rs` `consumption_reach` | PR #317 (issue uncertain) | Feeding reach grows with √structure (mycelium as foraging organ) | **Off**: 0 in recipe, baseline and every scenario; never searched | 1 (5-line fn) | none dedicated; 642 lists it "effects unseen"; viability.md rejects pinning it > 0 | **re-ground** or **delete** |
| Dispersal propagule cost (`dispersal_propagule_cost_coefficient`, `_exponent`) | `lib.rs:232-240, 530-545`; two calls in `phase.rs` `resolve_reproduction` | PR #286 (issue uncertain) | Energy for propagule structures, taken from the reproductive budget | **Off**: coefficient 0; exponent 2.0 inert | ~20 | 593 notes wider dispersal is free | **re-ground** |
| Dispersal mate reach (`dispersal_reach_coefficient`) | `lib.rs:546-557`; `phase.rs` `resolve_reproduction` | PR #287 (issue uncertain) | Sessile agents find mates by broadcast gametes | **On** at 10 (baseline, recipe); serde default 0 (off); not searched | ~3 | 593: also widens mate reach | **keep**; fix the serde-default mismatch |
| Leaching (`leaching_rate` λ) | `lib.rs:640-649`; `phase.rs` `leach_carcasses` | #698; fixed outside the box by #716 | Leachable carcass nutrient returns to the cell pool | **On** at 0.0025 in genesis and recipe; stepper default 0; `leached_ranges()` kept for old atlases | ~32 | 700: λ_max 0.01; 719: atlas passes at fixed λ | **keep** (mode 4 depends on it) |
| Founder aggregation (`InitialDistribution::founder_aggregation`) | `lib.rs:698-721`, `DEFAULT_FOUNDER_AGGREGATION` 0.8 | #601 (PR #611); held fixed #725 | Founders seeded as patches | **On**, 0.8, not searched | small | none | **keep** (derivation stated, expected-properties) |
| No Cargo `[features]` / `cfg(feature)` | — | — | — | none exist in `crates` | 0 | — | — |

## 2. Physics parameters pinned to 0, neutral, or held outside the search box

Box: `untaxed_ranges()` in `crates/explorers-search/src/search.rs` (33 coordinates). Fields outside
it take `viable_baseline()` (`search.rs:~653-707`, #334 fixing #326), copied from the
example4/example9 template. `crates/explorers-genesis/src/lib.rs:~398-442` is a test fixture, not a
baseline. Network, hyphal, cross-trait, body reach, propagule cost, recognition, λ and s_ref are in
§1. The 22 searched physics fields (solar flux, trophic efficiency, … offspring structure fraction)
have no departure and are not listed.

| Field | Sim use | Origin | Recipe / search | Design says the flow exists? | State | Sim LOC | Suggested |
|---|---|---|---|---|---|---|---|
| `wear_rate` | `phase.rs` `apply_wear`; `soa.rs` `apply_wear_soa` | PR #145 (issue uncertain); instrument condition #359 | **0 / fixed 0**; serde default 0.1; app 0.1; mode-1 mesocosm sets its own (provisional 0.2, #753) | **Yes**: world-rules *Wear is always on* (#750) | **Off against the design.** Baseline-zeroing tracked as **#756** (open) | wear machinery ~200 in total (apply, SoA twin, repair in `grow`, usage collection, effective-trait fns) | **re-ground** (#756) |
| `use_wear_rate` | same functions | PR #147 (issue uncertain) | 0 / fixed 0; serde default 0.01 | Yes (world-rules, use-wear) | Off; unit inhomogeneity an open decision | inside the ~200 above | **re-ground** (with #756) |
| `wear_degradation_steepness` | effective traits in photosynthesis, uptake, drains, movement, reproduction | PR #145 (issue uncertain) | 1.0 / fixed | Yes | Neutral, **inert while wear is 0** | ~22 | **keep** (comes back with wear) |
| `repair_decay` | repair loop in `grow`, `grow_soa` | PR #148 (issue uncertain) | 1.0 / fixed | Yes (flow 9 repair) | **Inert while wear is 0** | ~47 | **keep** (comes back with wear) |
| `somatic_maintenance_cost_coefficient` | **none**; app slider only | PR #146; made legacy by kappa allocation #198 (uncertain) | 0.1 / fixed | Its own doc comment calls it legacy | **Vestigial** | 0 (field + default fn) | **delete** |
| `mobility_maintenance_cost` | `phase.rs` `metabolic_cost`, `soa.rs` | PR #214 (issue uncertain) | **0 / fixed 0**; all scenarios 0 | **Yes**: every capability has a maintenance cost; sensing cost is paid through it | **Off against the design** | ~2 | **re-ground** (ties to #648) |
| `body_reach_coefficient` | see §1 | PR #317 | 0 / fixed 0 | Yes (flow-3 reach formula) | Off | 1 | **re-ground** or **delete** |
| `dispersal_propagule_cost_coefficient` (+ exponent 2.0) | see §1 | PR #286 | 0 / fixed 0 | **Yes** (described as charged) | Off; exponent inert | ~20 | **re-ground** |
| `structure_maintenance_coefficient` | `metabolic_cost`, `soa.rs` | PR #148 (uncertain) | 0.01 / fixed | Yes (flow 8) | On, not searched; value not derived | 2 | **keep**; derive the value (§4) |
| `asexual_propensity_maintenance_cost` | `metabolic_cost`, `soa.rs` | PR #255 (uncertain) | 0.01 / fixed | Yes (trade-off 8) | On, not searched; "small", no value given | 2 | **keep**; derive (§4) |
| `growth_efficiency` | `grow`, `retention_deficit`, provisioning, `grow_soa` | PR #128 (uncertain); no serde default since #327 | 0.3 / fixed | Yes | On, not searched | spread | **keep**; derive (§4) |
| `reproduction_nutrient_threshold` | gate in `resolve_reproduction` | PR #275 (uncertain) | 1.0 / fixed | Yes (flow 4 earmark); doubles as the nutrient unit `N_r` | On | 1 | **keep** (unit choice) |
| `initial_nutrient_pool` | `World::new`, `from_recipe` | PR #116 (uncertain); required since #327 | 50000 / fixed | Yes | On, not searched | small | **keep**; derive (§4) |
| `nutrient_grid_cell_size` | `World::new`, `from_recipe` | PR #219 (uncertain) | 10 / fixed | Yes; sets the patch scale reference modes read at | On, not searched | small | **keep**; derive (§4) |
| `uptake_reference_structure` s_ref | see §1 | #644 | 100 / fixed outside the box | Yes (named, no value) | On | — | **keep**; derive (§4) |
| `recognition_distance` | see §1 | #604 | 0.5 / fixed (#725) | Yes, with rationale | On | — | **keep** |

## 3. Mechanisms added and later reversed, or left vestigial

| Mechanism | What remains (location, size) | Added by | Reversed by | Purpose | State | Suggested |
|---|---|---|---|---|---|---|
| Need-gated consumption, `1/(1+c·s)` | No stepper code. `satiation_sensitivity` removed from params, search, app and flags. Back-compat test `lib.rs:~2313-2328` (old recipes with the field still load). Readings in `grazer_hunger.rs` (1375). Note 605 | #600/#610, #603/#614; measured #605, #619 | Superseded by the surplus gate (#621/#623); parameter removed #684 (PR #692) | Sate light-fed mixotrophs so their heterotrophy idles | Removed; survives as instrument + load test | **delete** the load test if old recipes need not load; instrument code see §5 |
| Surplus / satiation gate | Removed from the stepper (`flat_satiation_limit` deleted). Readings in `role_diet.rs` (1446), `role_diet_census` (1358), `grazer_hunger.rs`. Notes 622, 624 | #621, #622, #623 (PR #626) | #624 verdict fail; removed #684 (PR #692) | Same aim, satiation read as surplus | Vestigial (research only) | **delete** (with §5 instruments) |
| Intake ceiling `C = k × maintenance`, then `(k_a, k_h)` | Never in the stepper. `intake_ceiling.rs` (1632; `realised_bites` still used by `kin_killer_diet`, `fullness`, `role_diet`). Spec in world-rules *Capability and expression*. `CONTEXT.md` *Intake ceiling* | #628, #629, #633 | #629 failed by outcome; #634 region empty; folded into the fullness withdrawal (PR #679) | Body-scaled ceiling for the gate | Research only; docs and CONTEXT out of date | **delete** (keep `realised_bites` if the census needs it) |
| Fullness gate (moving-average intake over clearance time τ) | `fullness.rs` (1163; `FullnessBank`, `FullnessTracker`, grids). `CONTEXT.md` *Fullness*, *clearance time*. world-rules record block (~22k chars, §6) | #636, #637, #638; #630 (gate) and #631 closed | Withdrawn by PR #679 (per #662's closing rule; #670) | Gate drains on recent fullness | Research only; doc section kept on purpose as a record | **delete** code; **move** the doc record (§6) |
| Stale docs describing withdrawn gates | `viability.md` dimensionless groups (need gate `1/(1 + π_sat·s)`; `k`, `τ` as parameter dimensions); `CONTEXT.md` *Intake ceiling* ("searched by genesis on a log scale"), *Fullness* | #622–#637 era | #684's commit message says the `π_sat` group was dropped, but the text survived | — | **Wrong**: describes gates that do not exist | **delete** the stale text |
| Ungating (#684) | `RECOGNITION_RESTRAINT = 1.0` (`phase.rs:~331-339`, a const, not a parameter); `resolve_drains_with_deficits` | #680 (design via #662) | — | Expression = capability × max(0, 1 − w(d)) | Live | **keep**; the const could fold away (r = 1 is a no-op multiplier) |
| Trait-read role tag (heterotrophy > autotrophy) | `role_diet.rs::trait_tag`; `topology::DETRITAL_RELIANCE_THRESHOLD = 0.5` still used by `role_diet` and `tests/headless_decomposer.rs`; `trophic_balance_*` fields in `role_diet` | #267/#268 era | #596 measured it; #599/#609 replaced it with income roles; #613 retired trophic balance from fitness | Classify trophic role | Tag research-only; threshold live in the topology projection | **delete** the tag; grill the threshold |
| Self-grazing dispersal test | `--dispersal` mode in `reinvasion_barrier.rs` | #593 (PR #594) | Measurement closed | Move newborns out of kin reach to test #591's inference | Vestigial flag | **delete** (with the bin) |
| Somatic maintenance trait cost | `somatic_maintenance_cost_coefficient` (see §2) | PR #146 | kappa allocation #198 (uncertain) | Old somatic maintenance cost | Dead parameter | **delete** |
| Serde back-compat aliases | `consumption_efficiency`→`base_trophic_efficiency` (#196), `contact_radius`→`contact_range_coefficient` (#220), `consumption_rate`→`heterotrophy` (#191), `somatic_maintenance`/`reproductive_investment`→`kappa` (#198), `energy`→`reserve` (#127) | various | — | Load old recipes | Vestigial | **delete** if old recipes need not load |
| Dead code in the sim | `#[allow(dead_code)] fn emit` (`lib.rs:~2050`, "for coordinated phases (not yet implemented)", #61); `#[allow(dead_code)]` on `World`; `Agent::effective_trait` (no non-test caller); `resolve_drains` without deficits (tests only); `sensing_throughput` / `detected_count` (#175; read only by `tests/prop_symmetry.rs`) | #61, #173, #175 | — | — | Vestigial | **delete** |
| SoA / SIMD stepper spike | `crates/explorers-sim/src/soa.rs` (380) + `bin/bench_soa.rs` (147); kept in parity by `tests/soa_differential.rs` (e.g. it carries the cross-trait cost) | #354 (PR #356) | Verdict: "the floor isn't there" | Vectorised stepper | Research only, still maintained | **delete** |
| Dead search modules | `bayesopt.rs` (187), `gp.rs` (144), `sobol.rs` (190): only `pub mod` references | #11 (PR #19) | Superseded by QD (#365) | Bayesian optimisation search | Dead | **delete** |
| Earlier removals (context only) | Contact-duration feeding ramp (#382), contact-time uptake gate (#216), L1 budget (#191), offspring-count floor (#232) | — | Removed | — | Gone; recorded in world-rules and trait-space | — (nothing remains) |

## 4. Constants with no stated derivation

"Derivation" means the design docs (`docs/system-design/*.md`) give a reason for the value, not
only the name or units. Rows with a stated derivation are listed for completeness and marked.

| Constant | Value | Location | Derivation in the design? | Origin | Suggested |
|---|---|---|---|---|---|
| Unit anchors u_M, u_D, u_A, u_H, u_R | 1.0 each | `explorers-sim/src/units.rs:27-51` | Named; "1 in today's units"; exposing them deferred | #459 | **keep** (unit choices); state them as such |
| Use-wear units u_WA, u_WH, u_WM | 1.0 | `units.rs:57-73` | Named; inhomogeneity an open decision | #460 | **re-ground** with use-wear |
| `DEFAULT_CONTACT_DISTANCE` | 0.1·u_M | `lib.rs:671` | Partly: fixed in u_M units, no reason for 0.1 | #727 | **hold** (with hyphal uptake) |
| `DEFAULT_UPTAKE_REFERENCE_STRUCTURE` s_ref | 100 | `lib.rs:677-686` | Code comment only (atlas demand-neutral calculation); world-rules names s_ref, no value | #644, #653 | **keep**; move the derivation to the design |
| `DETRITAL_RELIANCE_THRESHOLD` | 0.5 | `topology.rs:27` | No | #267 (uncertain) | grill (see §3) |
| `growth_efficiency` | 0.3 | baseline, recipe | No (used, never valued) | PR #128 (uncertain) | **keep**; derive |
| `structure_maintenance_coefficient` | 0.01 | baseline, recipe | No (units and role only) | PR #148 (uncertain) | **keep**; derive |
| `asexual_propensity_maintenance_cost` | 0.01 | baseline, recipe | No ("small") | PR #255 (uncertain) | **keep**; derive |
| `nutrient_grid_cell_size` | 10 | baseline, recipe | No (units only, `π_cell`) | PR #219 (uncertain) | **keep**; derive (it fixes the patch scale) |
| `initial_nutrient_pool` | 50000 | baseline, recipe | No (units only, `π_N`) | PR #116 (uncertain) | **keep**; derive |
| `dispersal_reach_coefficient` | 10 | baseline, recipe | No | PR #287 (uncertain) | **keep**; derive |
| Founder `fecundity` | 0.35 | `search.rs:~786` in `decode` ("inherits the known-viable template value") | No | uncertain | grill: search it or derive it |
| `trait_covariance` floor | 0.1 (recipe sits on the floor of [0.1, 1.0]) | `search.rs:~376` | Range kept wide (genesis-search), bound not derived | uncertain | grill |
| `wear_degradation_steepness`, `repair_decay` | 1.0, 1.0 | baseline | No | PR #145, #148 (uncertain) | **re-ground** with wear |
| Poisson fecundity floor `.max(0.1)` | 0.1 | `phase.rs:~1863` (asexual), `~2168` (sexual) | No (trait-space calls the mean a search parameter) | #199 | grill |
| Mutation clamps: kappa and asexual propensity to [0, 1] (`dim == 3 \|\| dim == 5`), other traits floored at 0 | — | `phase.rs:~1938`, `~2302` | Implied by trait ranges | #199 | **keep**; index-based dims are fragile |
| Random walk: U(0, 2π) angle, U(0, 1) magnitude, added 1:1 to the chemotaxis vector | — | `phase.rs:~1614-1690` | **No**: execution-model defers to world-rules, which does not specify it | #175 | grill (consumer movement carries mode 3) |
| Movement epsilons `1e-6` (no-direction, no-move-no-cost) | 1e-6 | `phase.rs:~1635-1709` | No | #175 | **keep** (numerical) |
| `structural_fragility`: normalised entropy of the L1-normalised trait vector | — | `lib.rs:~959-985` | Form stated (trade-off 9); the entropy choice is not derived | uncertain | grill |
| Network flow forms: energy `gradient/(1+eff)`, nutrient capped at `gradient/2` | — | `phase.rs:~545`, `~580` | Only "never overshoots" in a code doc | #410, #411 | **hold** (with the network) |
| `PRODUCER_LIGHT_SHARE` (observer) | 0.5 | `genesis-eval/src/income.rs:29` | No | #599/#609 | **keep**; state it |
| `ROSTER_FLOOR` (observer) | 20 | `genesis-eval/src/lib.rs:129` | No | uncertain | **keep**; state it |
| `NARROWED_BAND_FRACTION` (observer) | 0.25 | `search.rs` | Own comment: "a judgement call" | #559 | grill |
| `STRUCTURE_MIN` (prefilter) | 1.0 | `prefilter.rs:40` | "Uncommitted 1 E anchor" (world-rules unit anchors) | #480 | **hold** (#465 permanence gate) |

Stated derivations, for completeness (not departures): `DEFAULT_FOUNDER_AGGREGATION` 0.8
(expected-properties, *Founder placement*), `recognition_distance` 0.5 and `RECOGNITION_RESTRAINT`
1 (world-rules, *Recognition*), `leaching_rate` 0.0025 = λ_max/4 (world-rules, *Carcass energy …
leaches*), cross-trait exponent p/2 (trade-off 5), √structure in feeding reach (flow 3),
`SUSTAINABLE_FRACTION` 0.1, `GUILD_MIN_SIZE` 5, `INCOME_HALF_LIFE` 50, bloom stop 300:10
(genesis-search / expected-properties).

## 5. Instruments

### 5a. Binaries

"Live refs" means a reference from `docs/system-design/`, `docs/agents/` (other than the runtime
table in `instrument-runtimes.md`), `CONTEXT.md`, an external test or a script. Every bin also has
in-file tests, which do not count. Open issues touched: #648, #654, #736, #465, #756.

| Binary | Path | LOC (+ modules only it uses) | Origin | Purpose | Live refs | Reading closed? | Suggested |
|---|---|---|---|---|---|---|---|
| `explorers-search` (main) | search `src/main.rs` | 596 (+ shared infra: qd 4550, search 1578, checkpoint 791, atlas_file 621, bifurcation 806, prefilter 223, config_source 757, sweep 425, lhs 65) | #11; QD #365; prefilter #370; bifurcation #372; checkpoint #530; reproject #535 | Genesis atlas search, resume, reproject | genesis-search, viability; tests | n/a (production) | **keep** |
| `reference_mode` | search `bin/reference_mode.rs` | 144 (+ mesocosm 1024) | #751 (PR #758), #752, #753 | Mode-1 mesocosm: build, settle, perturb, report the centre patch | reference-modes, CONTEXT, runtimes; `reference_mode_cli` test | **Live** (mode 1) | **keep** |
| `export_recipe` | search `bin/export_recipe.rs` | 140 (+ recipe_export 288) | #581 (PR #584) | Export an atlas cell as an app recipe | app, fragility docs | n/a (tool) | **keep** |
| `fragility_audit` | search `bin/fragility_audit.rs` | 609 (+ fragility 1661) | #693 (PR #694); network overrides #747 | Per-cell outcome-flip rate under small perturbations | CONTEXT, world-rules, 736; tests; script | #693 closed; used by open #654, #736 | **keep** |
| `settling_time` | search `bin/settling_time.rs` | 957 | #505 (PR #512) | Provisioning transient and settling tick; sets T and grace | genesis-search §horizon | #505 closed; named re-measure tool for T | **keep** |
| `eval_scenarios` | genesis-eval `bin/eval_scenarios.rs` | 194 | #293 (PR #301) | Cross-validate example verdicts | runtimes; sim test comment | ongoing | **keep** |
| `permanence_crosscheck` | search `bin/permanence_crosscheck.rs` | 2271 | #439 (PR #458) | Cross-check A1/A2 permanence predictions | viability cites results | #439 closed; permanence gate deferred (#465 open) | **hold** (#465) |
| `permanence_prototype` | sim `bin/permanence_prototype.rs` | 1559 | #432 (PR #448) | A1 permanence condition of the mean-field map | viability (result, not name) | #432 closed | **hold** (#465) |
| `probe_generalist` | genesis-eval `bin/probe_generalist.rs` | 105 | #325 (PR #378) | Do fragility + incompatibility confine generalists | viability cites it | #325 closed | **hold** |
| `role_diet_census` | search `bin/role_diet_census.rs` | 1358 (+ role_diet 1446, intake_ceiling 1632, fullness 1163, grazer_hunger 1375, shared with `kin_killer_diet`) | #596 (PR #597); then #605, #606, #619, #622, #624, #629, #634, #637, #655, #656, #668, #684 | Role tag vs diet, plus the gate-calibration readouts | runtimes only | Yes, every reading; gates withdrawn | **delete** the bin; modules **hold** until untangled from the census |
| `energy_bound_check` | search `bin/` | 1141 | #433 (PR #457) | Empirical check of bound B1 | runtimes only | Yes (433 note) | **delete** |
| `energy_death_check` | search `bin/` | 879 | #508 (PR #516) | Zero-false-positive check before promoting the energy-death read | runtimes only | Yes (promoted) | **delete** (keep the shared `sweep.rs`) |
| `guild_census` | search `bin/` | 656 | #494 (PR #544; issue attribution uncertain) | Guild fractions, atlas vs LHS | runtimes only | Yes (494 note) | **delete** |
| `guild_structure` | search `bin/` | 862 | #546 (PR #547) | Straddle vs behavioural decomposer guild in gated worlds | none | Yes (546 note) | **delete** |
| `heterotroph_margin` | search `bin/` | 553 (+ heterotroph_margin.rs 333) | #589 (PR #590) | Heterotroph carcass-income margin over the box | none by name | Yes | **delete** |
| `invasion_growth` | search `bin/` | 1862 | #443 (PR #488); config_source #498 | Mutual-invasibility growth rates | runtimes only | Yes ("question was mis-posed") | **delete** (`invasion.rs` 545 goes if its other users go) |
| `radius_sweep` | search `bin/` | 649 | #462 (PR #521) | Single-factor light-radius sweep | runtimes only | Yes (462 notes) | **delete** |
| `reinvasion_barrier` | search `bin/` | 1941 (+ energy_accounting 860) | #587 (PR #588); `--accounting` #591; `--dispersal` #593 (uncertain) | Inject heterotroph phenotypes on sample:31's carcass pile | known-traps, runtimes | Yes | **delete** (with its known-traps entry) |
| `role_emergence` | search `bin/` | 966 | #421 (PR #422) | Timing of trophic-role emergence | runtimes only | Yes (421 note) | **delete** |
| `step_profile` | search `bin/` | 106 | #423 (PR #424) | Profile the step-loop hot path | none | Yes | **delete** or **hold** (cheap perf tool) |
| `bench_query_radius` | sim `bin/` | 337 | #423 (PR #424) | Benchmark `SpatialGrid::query_radius` | none | Yes | **delete** or **hold** |
| `bench_soa` | sim `bin/` | 147 | #354 (PR #356) | SoA spike benchmark | none | Yes ("floor not there") | **delete** (with `soa.rs`, §3) |
| `branching_detector` | sim `bin/` | 1083 | #359 (PR #361) | Moment-closure branching spike (cluster F) | none (productised in `bifurcation.rs`, #372) | Yes | **delete** |
| `hopf_prototype` | sim `bin/` | 587 | #358 (PR #360) | Compartment-ODE Hopf spike | none | Yes | **delete** |

Not a binary but the live census: `crates/explorers-search/examples/kin_killer_diet.rs` (6826 LOC,
+ `leaching.rs` 564 and `flow1_verdict.rs` 473 used only by it). Origin #642 (PR #643), then a
section per reading (5c). Live for open #736, #654, #648. **keep**, with its closed sections cut.

### 5b. Library modules that exist for an instrument

| Module | LOC | Origin | Used by | Suggested |
|---|---|---|---|---|
| `energy_accounting.rs` | 860 | #591 (PR #592) | `reinvasion_barrier` only | **delete** |
| `heterotroph_margin.rs` | 333 | #589 (PR #590) | its bin | **delete** |
| `invasion.rs` | 545 | #587 (PR #588) | `invasion_growth`, `reinvasion_barrier`, `role_diet_census`, `energy_accounting` | **delete** if those go, else **hold** |
| `role_diet.rs` | 1446 | #596 (PR #597) | `role_diet_census`; `kin_killer_diet` (`rollout_with_fullness`, `census_failure`); `fragility` (`failure_label`) | **hold** (untangle) |
| `intake_ceiling.rs` | 1632 | #629 (PR #632); region #634 | role_diet, fullness, grazer_hunger, census (`realised_bites`) | **hold** → **delete** after untangling |
| `fullness.rs` | 1163 | #637 (PR #641) | role_diet, role_diet_census, kin_killer_diet | **hold** → **delete** after untangling |
| `grazer_hunger.rs` | 1375 | #606 (PR #617) | many; census section O uses `PreStep`, `GrowthLimit` | **hold** |
| `leaching.rs` | 564 | #700 (PR #704) | census section M only | **hold** (λ fixed by #716; reading closed) |
| `flow1_verdict.rs` | 473 | #682 (PR #689) | census only | **hold** |
| `fragility.rs` | 1661 | #693 (PR #694) | `fragility_audit` | **keep** |
| `mesocosm.rs` | 1024 | #751 (PR #758) | `reference_mode` | **keep** |
| `recipe_export.rs` | 288 | #581 (PR #584) | `export_recipe` | **keep** |
| `bifurcation.rs` | 806 | #372/#374 | QD descriptor in `qd.rs` (live) | **keep** |
| `bayesopt.rs` + `gp.rs` + `sobol.rs` | 521 | #11 (PR #19) | nothing | **delete** (§3) |

### 5c. Census sections

`role_diet_census` sections live in its bin plus `role_diet`, `intake_ceiling`, `fullness`;
`kin_killer_diet` sections in the example. Sizes are not split per section.

| Section | Instrument | Origin | State | Suggested |
|---|---|---|---|---|
| Trait tag × diet role confusion | role_diet_census | #596 | closed; tag retired (#599, #602) | **delete** |
| Light-share deciles of tagged heterotrophs | role_diet_census | #596 | closed | **delete** |
| Guild fractions under both reads | role_diet_census | #596 | closed | **delete** |
| Deaths: infant / grazed / grazed-by-kin | role_diet_census | #596; #605 (uncertain) | closed | **delete** |
| Grazed pairs + drain-time satiation; birth trait distance | role_diet_census | #606 | closed | **delete** |
| Surplus satiation by role | role_diet_census | #622 | closed; gate removed #684 | **delete** |
| Intake-ceiling window (`--intake-ceiling-k`) | role_diet_census | #629 | closed (fails by outcome) | **delete** |
| Gate records + `--region` (k_a, k_h) | role_diet_census | #634 | closed (region empty) | **delete** |
| `--fullness` / `--fullness-region` (n, k, τ) | role_diet_census | #637; grid #656 | closed (empty) | **delete** |
| Persistence readout (pooled failure labels) | role_diet_census | #645 | closed | **delete** |
| 1 Diet by target bucket | kin_killer_diet | #642 | closed; C reuses its buckets | **hold** |
| 2 Reach at each kin kill | kin_killer_diet | #642 | closed | **delete** |
| 3 Mobility / dispersal of kin killers | kin_killer_diet | #642 | closed | **delete** |
| A Prevalence / h_eff by role | kin_killer_diet | #642 | closed | **hold** |
| B Producer samples by h_eff bins | kin_killer_diet | #642 | G reads B | **hold** |
| C Carcass processing by route / recipient (network columns #646) | kin_killer_diet | #642; #646 | H and P read it | **keep** |
| D Producer heterotrophy by quarter | kin_killer_diet | #642 | closed | **delete** |
| E Persistence (verdict per seed) | kin_killer_diet | #645 | read by all later sections | **keep** |
| F Live connections by endpoint role / kinship | kin_killer_diet | #646 | P reads it; #736 open | **keep** |
| Verdict block (#647 tests vs #642 ref; atlas fingerprint) | kin_killer_diet | #655, #656 | closed | **hold** |
| G Conditionality ρ(h_eff, pool N) | kin_killer_diet | #655 | feeds flow-1 verdicts | **hold** |
| H Niche share (carcass drained by bucket) | kin_killer_diet | #655 | feeds flow-1 verdicts | **hold** |
| I c_AH calibration (c_max) | kin_killer_diet | #668 | closed | **delete** |
| J Carcass N retained by mixotrophs | kin_killer_diet | #668 | closed | **delete** |
| K c_AH charge by role (`--crosscheck`) | kin_killer_diet | #681 | closed | **delete** |
| Flow-1 verdicts header (`--baseline`) | kin_killer_diet | #682 | world-rules flow 1 | **hold** |
| L Mobile autotrophy | kin_killer_diet | #683 | #648 open | **keep** (hold with #648) |
| M Carcass leaching M1–M4 (`--leaching-sweep`) | kin_killer_diet | #700 | closed; λ fixed (#716) | **hold** |
| N Partner gradient + hyphal share (`--hyphal-uptake`) | kin_killer_diet | #728 | #654, #736 open | **keep** |
| O Growth limitation by role | kin_killer_diet | #740 | #736 open | **keep** |
| P Producer–decomposer trade and pairing | kin_killer_diet | #742 | #736 open | **keep** |

(The shared network override flags in `config_source.rs`, #646/#747 and possibly #741, serve the
census and `fragility_audit`; **keep** while #736 is open.)

## 6. Measurement history inline in the design docs

Issue-reference counts per file (`grep -oE '#[0-9]+'`, minus trade-off references #2/#5/#8/#9):
world-rules 116 (45 distinct), genesis-search 90 (28), viability 26 (20), expected-properties 8,
trait-space 2, execution-model 1, README 0, reference-modes 0. "Note" says whether a
`docs/research/` note already holds the content: **Y** yes, **P** partly, **N** no (a note would be
needed before moving).

### world-rules.md

| Section | What it narrates | Issues | Size | Note | Suggested |
|---|---|---|---|---|---|
| *Capability and expression* — the withdrawn fullness gate ("*The record: … withdrawn*") | Full spec of a withdrawn gate, with eight embedded *Measured* blocks | ~30 refs (#605–#670, #630/#631) | ~21.8k chars, ~40 lines — the largest block in any design doc | Y (605, 606, 619, 622, 624, 629, 634, 637, 642, 655, 668, 670) | **move**; keep ~3 sentences on the constraints it set |
| Photosynthesis — mixotroph tests | *Current state* cross-trait (#667/#669); *Measured* #655/#656, #668, #670; demotion (#693); hyphal (#654) | 9 | ~2.8k chars | Y (655, 656, 662, 668, 670) | **move**; keep one sentence + pointer |
| Cost structure — 5. Specialist vs generalist | c_AH range on #663's atlas; *Current state* (#701, #687); *Measured* #668, #670; "since #684" | 10 | ~2.2k chars | Y (668, 670) | **move**; keep "latent, [0, 0.14] if revived" |
| Carcass leaching — *the rate* | *Measured (#686, #711)* λ cell shares; *Current state (#716)* box/atlas history | 7 | ~1.6k chars | Y for Measured; N for Current state | **move**; leave "λ fixed at 0.0025, not searched" |
| Carcass leaching — *form / rate* | #670 carcass bite; λ spread; *Measured (#700)* lockup seeds 19→7→2→1 | 4 | ~0.7k chars | Y (700, 686, 711) | **move** Measured; keep the λ_max/4 rationale |
| Carcass leaching — last sentence | #677 atlas fragility, ρ +0.37 | 1 | ~250 chars | Y (693) | **trim** |
| Sessile vs mobile / substrate contact | *Measured (#683)* mobile autotrophy viable; *Measured (#654 grill)*; #648 | 4 | ~1.3k chars | Y (683); P (#654 grill) | **move**; keep the trigger rule |
| Consumption — retention rule | *Measured (#668)* 7–8× carcass nutrient; *Measured (#670)* | 2 | ~0.9k chars | Y (668, 670) | **move** |
| Recognition | *Measured (#606; #624 re-measures)* offspring-spared shares; #619 D = 1.4; c_AH, λ noise | 6 | ~1k chars | Y (606, 619) | **move**; keep the default-D rationale |
| Recognition — near-identical non-kin | *Measured (#606; #624)* hunger the ordinary state | 2 | ~0.5k chars | Y (606) | **move** |
| Hyphal uptake — *Measured (#730)* | partner test fails, 39 of 73 | 3 | ~1k chars | Y (654) | **move** |
| Hyphal uptake — why it failed | Sterner & Elser rationale; "open under #654" | 1 | ~1k chars | P | **keep** rationale; drop narration |
| Hyphal uptake — read by rule (#654) | | 1 | ~200 chars | Y | **trim** |
| Hyphal uptake — **Status: Implemented (#727)** | implementation status | 1 | ~0.6k chars | N | **trim** to "latent, default off" |
| *The partnership's partner is the decomposer* | *Measured (#743)* ECM reading, four result bullets | 4 | ~1.3k chars, 7 lines | Y (736) | **move**; one-line pointer |
| Redistribution split — **Status (#738)** | inert while the network is off | 1 | ~0.7k chars | N | **trim** |
| Network flow — **Status (#249)**; event legs (#646) | implementation status | 2 | ~0.6k chars | P (646) | **trim**; keep the event-legs design |
| *Capability and expression … ungated* (second paragraph) | #656, #668, #670 read worlds; #624 fails consumers 18–39×; #686 | 6 | ~0.8k chars | Y (624) | **trim** to a pointer |
| Feeding reach — hard predicate | proptest divergence (#453, #477), 8e-5, distances | 2 | ~0.45k chars | N | **keep** (rationale); drop refs |
| Light-competition membership | #548 worked example (tick-7 reserve fork) | 1 | ~0.9k chars | N | **trim** the example |
| Founders bind nutrient; size-scaled uptake; unit anchors | single refs (#612, #642, #461) in rationale | 3 | refs | — | **keep**; drop refs |

### genesis-search.md

| Section | What it narrates | Issues | Lines | Note | Suggested |
|---|---|---|---|---|---|
| Scoring — four stacked dated blocks | *Current state (#710)*, *Measured (#710)* ρ 0.960; *Measured (#711)* 43.4→47.1 %; *Current state (#699)*; *Measured (#699)* wall-clock | ~20 refs | 20 (~4.2k chars) | Y (711); N (#710, #699) | **move**; keep "scored by L²·F over 10 seeds" |
| *Why the narrowed box is not the default* | #559 run table + cause analysis | 1 | 22 | Y (561) | **trim** to a result line + pointer |
| *The horizon is the settling time* — *Current values — measured* | grace 260; settling quantiles; T = 2000 | 1 | 21 | Y (505) | **move** quantiles; keep the values |
| After the pick rule — *Current state / Measured (#715)* | reproject #711; committed recipe atlas:31 32/32, 0.704 | 5 | ~12 | N | **move**; keep the constants |
| *Decided (#494): the guild stays reported* | result bullets 0.9 %, 1 of 141 | 2 | 11 | Y (494) | **trim** to the decision |
| Terminal gate zeroes — *Measured at T = 2000 (#546)* | 2 of 12 monoculture seeds | 1 | 10 | Y (546) | **trim** |
| Search box — leaching | #686/#711 history; *Current state (#716)* | 5 | ~10 | N | **move** history; keep the decode rule |
| Search box — cross-trait | *Current state (#716, #687)* | 6 | ~5 | N | **trim** |
| One predictive stop: the bloom stop (#573) | 97 %, factor 10, blind spot | 1 | 16 | Y (554, 573) | **trim** numbers |
| Early-stop carry | 1261 carried stops, 13 read alive | 0 | ~4 | P (573) | **move** |
| Genesis selects for robust | #677 / #693: Spearman +0.80, 25× cost; 42 of 99; 32–57 robust worlds | 5 | ~12 | Y (693); P | **trim** to one sentence + pointer |
| Pick refines in fitness order; gated refinement | "7th on #711's atlas"; #401 numbers | 2 | ~6 | P / N | **trim** |
| Predicted bifurcation coordinates | #358/#359 *Qualified GO* verdicts as gating reason; "wear is off for every searched config" | ~14 | ~10 scattered | Y (F- notes) | **keep** the gating rationale; **re-ground** the wear clause with #756 |
| A unit vector names a world only with its box | #663/#677/#687 commit history as examples | 3 | ~3 | N | **trim** |
| Atlas maps the settled community; 1 rollout; power is 2; 10 seeds; guild authority boundary | rationale citing 443, 596, measured 5.5× variance, sd 0.15 | several | ~15 | Y / P | **keep** (rationale); drop refs |
| Headings with issue numbers (#562, #404, #531, #653; guild floor #494/#538; #527) | refs only | 8 | — | — | **trim** refs |

### viability.md

| Section | What it narrates | Issues | Size | Note | Suggested |
|---|---|---|---|---|---|
| Permanence — *Where the IBM contradicts it — the failure cells* | per-cell failures (sample:12/31/154/158/192, atlas:2), confusion counts, κ_C re-take | #482 + inline | ~10.3k chars, ~100l | Y (509, 432, 437, 439) | **move** detail; keep the authority-boundary conclusion |
| Findings and follow-ups (status list) | restates C*, N̄_max, permanence cross-check numbers | — | ~6.1k chars | P (433, 509) | **trim** each bullet to status + pointer |
| Generalist dominance — *Empirical confirmation (#325)* | example12 shares 33–56 %, 0 survivors | 1 | ~2k chars | N | **move** |
| Generalist dominance — *Re-validated (#380, fixes #379)* | contact-duration ramp confound history | 2 (×8) | ~1.7k chars | N | **move** |
| Generalist dominance — conclusion | median 20 % → 44 % watch item | — | ~0.8k chars | N | **keep** the conclusion; move numbers |
| Sustained population — *Tightness* | 2256-run check, 2189 runs, median 1.000 | — | ~1.6k chars | Y (509, 433) | **trim** + pointer |
| Partial closure — *Empirical confirmation (#390, extends #136)* | example13 8/8, fitness 0.686 | 2 | ~1k chars | N | **move** |
| Lockup flux balance — cross-check | example9 lockup 8/8 | — | ~0.6k chars | N | **keep** (falsifiable prediction) |
| Dimensionless groups — "added in #653 / #669" | box history; **and** the stale need-gate / `k`, `τ` text (§3) | 2 | ~250 chars | N | **trim**; delete the stale text |

### expected-properties.md, trait-space.md, execution-model.md

| File — section | What it narrates | Issues | Size | Note | Suggested |
|---|---|---|---|---|---|
| expected-properties — Generalist dominance | "(#662)" + *Measured (#656)* 46–48 % | 2 | ~360 chars | Y (656, 662) | **trim** |
| expected-properties — *What the detector measures* | refs | #486, #599 | refs | N | drop refs |
| expected-properties — By-construction examples | "retired, #602" | 1 | ref | N | drop ref |
| expected-properties — Two-phase world creation | **Status: Not yet implemented — #250** (open, deferred) | 1 | ~190 chars | N | **keep** as a tracking pointer or move to the tracker |
| expected-properties — Founder placement | "Concretely (#601)", "(#612)" | 2 | refs | N | drop refs |
| trait-space — Decomposer is a behavioural role | *Measured (#655, #656)* 1–4 % kept, 46–48 % shares | 2 | ~0.5k chars | Y (655, 656) | **trim** |
| trait-space — reserve lever | sample:31 measured case, e ≈ 0.02 | — | ~0.7k chars | Y (587, 589) | **trim** |
| execution-model — Event vocabulary, *Redistributed* | "(#646)" | 1 | ref | — | drop ref |

History with no research note yet (a note is needed before moving it): #710, #699, #715, #716
(genesis-search); #325, #380/#379, #390/#136 (viability); #453/#477, #548 (world-rules, better
trimmed in place); the 1261/13 early-stop carry result (genesis-search).
