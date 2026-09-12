# Issue #433 — formal viability B1: the living-energy dissipativity bound, and the rule that blocks it

**Status: research finding. Commits nothing.** This note derives what the committed
energy budget *does* bound — a config-only cap on per-tick solar income, a one-step
dissipativity recursion on total system energy, and a closed-form ceiling on the
long-run mean number of surviving agents — and then shows that the bound the issue asks
for, `E_living(t) ≤ max(E_living(0), E_max)` with `E_max` a function of the committed
parameters, **does not exist under the committed rules**. The obstruction is named
precisely (the reproductive allocation is an untaxed, uncapped stock whose only outflow
is a reproductive event that the rules do not guarantee), exhibited by a one-agent
witness, and the commitment that would restore the bound is stated with the closed form
it would yield. It introduces no mechanism, functional form, or parameter; it touches
nothing in `docs/system-design/` and nothing in the stepper, evaluator, or search. The
finding feeds [`viability.md`](../system-design/viability.md)'s **Open** tier.

Every lemma is checked against the real stepper by
`crates/explorers-sim/tests/prop_energy_bound.rs` (256 proptest cases over the search
domain per property, plus the deterministic witness), so the proof stays falsifiable.

## TL;DR

Let `F = solar_flux_magnitude`, `L = world_extent`, `r = light_competition_radius`,
`B = base_metabolic_rate`, and `m = ⌊√2·L / r⌋ + 1`.

1. **Per-tick solar income is capped** (Lemma 1): total photosynthetic income in any
   tick is `P(t) ≤ P_max := F · min(N_P(t), m²)`, where `N_P(t)` is the number of
   embodied producers. Config-only: `P_max ≤ F·m²`. Requires `r > 0`.
2. **Total energy is dissipative per survivor** (Lemma 2): with `E_tot` = living
   (reserve + reproductive allocation + structure) + carcass energy and `N_s(t)` the
   agents alive at the start of tick `t` that are still alive at its end,
   `E_tot(t+1) ≤ E_tot(t) + P(t) − B·N_s(t)`.
3. **Sustainable count** (Theorem): the long-run mean number of tick-survivors obeys
   `(1/T)·Σ_{t<T} N_s(t) ≤ P_max/B + E_tot(0)/(B·T)` — so past
   `t_0 = E_tot(0)/(B·ε)` the mean is within `ε` of **`N̄_max = F·m²/B`**. On the
   `lib.rs` default (`r = 1000 > L`, so `m = 1`) that is simply `F/B`. This needs no
   "minimal viable body": the death threshold is peak-relative, so no such body exists
   in the committed rules, and the count bound goes through metabolism instead.
4. **No `E_max` exists.** A lone producer with `kappa < 1`, `asexual_propensity = 0`
   and no mate gains a fixed positive amount of living energy every tick, forever
   (≈ 5.6 energy/tick on the viable baseline). `E_living(t) → ∞` while `N = 1`. The
   obstructing rule is **flow 9's reproductive allocation**: it is filled every tick by
   the `(1 − kappa)` share of mobilised surplus, is charged nothing to hold, has no cap,
   and is drawn down only by a reproductive event (flow 4) that the rules never force.
   Three further gaps are recorded (the untaxed carcass pool; negative founder traits
   turning maintenance into income under an odd exponent, #444; `r = 0`), but the first
   alone is sufficient and survives every parameter choice in the search box.
5. **Commitment required.** A per-unit holding cost `h > 0` on *every* living stock
   (the analogue of `structure_maintenance_coefficient`, extended to reserve and the
   allocation) makes living energy dissipative in itself and yields
   `E_living(t) ≤ max(E_living(0), P_max/h)` for all `t ≥ 0` (`t_0 = 0`) in any world
   whose carcass pool cannot feed back into the living stock; the general case leaves
   the carcass-to-living inflow as a separate term, blocked by the same "no passive
   decay" endogeneity that already blocks the nutrient-lockup gate.

## Committed rules relied upon

Every step below cites one of these. `WR` = [world rules](../system-design/world-rules.md),
`EM` = [execution model](../system-design/execution-model.md); the code column is where
the rule is realised (`phase.rs` / `lib.rs` are in `crates/explorers-sim/src/`).

| # | Rule | Source | Code |
|---|---|---|---|
| R1 | Solar flux is the sole external energy source; it is constant at `F` per tick per light-competition neighbourhood. The pre-tick endowment is the only other input, booked once at world creation. | WR *Energy stocks*; EM *Energy conservation verification* | `phase::photosynthesise`; `provision_initial_reserve_structure` (`lib.rs`) |
| R2 | Light share = `own_weight / Σ weights of producers within r`, weight = `effective_autotrophy × structure`; income = `F × share`. Zero structure means zero light. | WR flow 1 | `photosynthesise`, lines 36–69 |
| R3 | The surface is a torus of extent `L`; distance is toroidal; "within" is strict (`< r`). | WR *Physical surface*; EM *Positions are stable within a tick* | `toroidal_distance`; `photosynthesise` line 62 |
| R4 | Phase order: photosynthesise → absorb → metabolise → grow → drains → deaths → reproduction → move → wear → death check; the population at a tick's start is exactly the agents that ended the previous tick alive. | EM *The tick loop* | `World::step` |
| R5 | Metabolism charges `B + Σ c_k·trait_k^p + c_s·structure` from reserve to heat every tick, for every living agent, with all coefficients `≥ 0` and traits in the non-negative trait domain. | WR flow 8 | `phase::metabolise` |
| R6 | Growth mobilises `f × (reserve − buffer)⁺`, splits it by kappa; the kappa share funds repair and lossy growth; the `(1 − kappa)` share is added to the reproductive allocation, which is reserve for conservation but "is not available to fund metabolism, growth, or repair". | WR flow 9 | `phase::grow`, line 525 |
| R7 | Reproduction requires `allocation ≥ reproduction_energy_threshold` **and** `repro_nutrient ≥ reproduction_nutrient_threshold`, then either an asexual roll against `asexual_propensity` or a compatible mate within reach; it consumes the whole allocation; every offspring energy comes from it, lossily. | WR flow 4 | `phase::resolve_reproduction`, lines 1159–1173, 1223–1268 |
| R8 | Consumption moves structure from a target (living or carcass) to a consumer's reserve at efficiency `base·exp(−decay·d) ≤ base ≤ 1`; the rest is heat. Proportional split never exceeds the target's stock. | WR flows 3, 7; EM *Pass 1 — Drains* | `phase::resolve_drains` |
| R9 | Death moves structure to a carcass and dissipates reserve and allocation; carcasses do not decay. | WR flow 6, *No passive decay* | `check_death_thresholds`; drain-death path in `World::step` |
| R10 | Movement, repair, growth conversion, reproduction inefficiency, propagule cost, network creation/maintenance/transfer loss are all non-negative flows to heat. | WR flows 4, 5, 7, 8, 9 | `move_agents`, `grow`, `resolve_reproduction`, `form/maintain_connections`, `redistribute` |
| R11 | Conservation: `ΔE_system = photosynthetic input − total dissipation`; no other source, no other sink. | WR *Conservation laws*; EM *Energy conservation verification* | `EnergyLedger::assert_balanced` (`energy_ledger.rs` ~line 160) |
| R12 | The structural death threshold is `fragility(traits) × peak_structure` — peak-relative, with no absolute floor; offspring are born at their own peak. | WR trade-off #9, flow 9 | `death_threshold` (`lib.rs`) |

## Notation

- `N(t)`: agents alive at the start of tick `t`; `N_P(t) ⊆ N(t)` those with
  `effective_autotrophy > 0` and `structure > 0` (the embodied producers of R2).
- `N_s(t)`: the agents in `N(t)` still alive at the end of tick `t` (tick survivors).
- `E_living(t) = Σ_{i∈N(t)} (reserve_i + allocation_i + structure_i)`; `E_carc(t)` the
  carcass pool; `E_tot = E_living + E_carc`.
- `P(t)`: total photosynthetic income booked in tick `t`; `D(t)`: total heat.

## Lemma 1 — per-tick solar income is capped by a config-only quantity

**Claim.** If `r > 0`, then for every tick
`P(t) ≤ F · min(N_P(t), m²)`, `m = ⌊√2·L/r⌋ + 1`.

**Proof.** By R2, producer `i` receives `F · w_i / W_i` with `w_i > 0` and
`W_i = Σ_{j : d(i,j) < r} w_j ≥ w_i`, the sum over embodied producers within toroidal
distance `r` (R3), self included. Two bounds:

*(a)* Each share is `≤ 1`, so `P(t) ≤ F·N_P(t)`.

*(b)* Tile the torus into `m × m` axis-aligned square cells of side `s = L/m`. Because
`m > √2·L/r`, `s < r/√2`, so any two points of one cell are at Euclidean distance
`< s·√2 < r`, and toroidal distance is never larger than Euclidean distance (R3), so they
are within `r` of each other under the strict test at `photosynthesise` line 62. Hence
for every producer `i` in cell `c`, `W_i ≥ S_c := Σ_{j∈c} w_j`, and

```
Σ_{i∈c} w_i / W_i  ≤  Σ_{i∈c} w_i / S_c  =  1.
```

Summing over the `m²` cells, `Σ_i w_i/W_i ≤ m²`, so `P(t) ≤ F·m²`. ∎

The spatial-grid query at line 49 returns a superset of the producers within `r`
(`SpatialGrid::query_radius` scans `⌈r/cell⌉ + 1` cells per side and distance-filters
with `≤ r`), and the phase re-filters with `< r`, so `W_i` is exactly the R2 sum; the
`seen` set removes the double-returns noted in the memory file. The constant `m²` is not
tight (a packing argument would sharpen it; the scaling `(L/r)²` would not change).

*Numbers.* `lib.rs` default (`L = 100`, `r = 1000`): `m = 1`, `P_max = F = 10` — the
whole world is one light neighbourhood. Viable baseline (`L = 100`, `r = 8`): `m = 18`,
`P_max = 324·F = 3240`. Search box (`L ∈ [20,100]`, `r ∈ [1,20]`): `m ≤ 142`.

This is the bounding statement viability.md's Open tier lacks for "the photosynthesis
income magnitude": net primary production per tick is at most `F·m² − B·N_P`.

## Lemma 2 — total energy is dissipative per surviving agent

**Claim.** `E_tot(t+1) ≤ E_tot(t) + P(t) − B·N_s(t)`.

**Proof.** By R11, `E_tot(t+1) = E_tot(t) + P(t) − D(t)`, since the only input after
tick 0 is solar (R1; the endowment is booked once) and every other flow is either
internal to `E_tot` (growth, consumption, reproduction, redistribution, death's
structure-to-carcass move — R6, R8, R9, R10) or goes to heat. So it suffices to show
`D(t) ≥ B·N_s(t)`.

Take `i ∈ N_s(t)`. By R4 it was present at `metabolise`, which subtracted its full cost
`c_i ≥ B` from its reserve (R5; `phase.rs` line 170 is unconditional arithmetic). Because
`i` ends the tick alive, its reserve is `> 0` after every later phase; the energy that
left `i` during the tick is therefore *at least* `c_i` more than what arrived, and that
difference can only have gone to heat or to other agents' reserves/carcasses — both of
which are already counted on the right-hand side of the ledger, so the `c_i` is a net
loss of `E_tot`. Every other heat term is `≥ 0` (R10). Summing over survivors,
`D(t) ≥ Σ_{i∈N_s(t)} c_i ≥ B·N_s(t)`. ∎

*On #445.* An agent that starves this tick is charged its full `c_i` even though it
held less; the accumulator `World::dissipated_energy` over-counts by the overdraft. The
lemma counts only survivors, whose charge is genuinely paid, so the bound is unaffected
in direction and in value. Non-survivors contribute nothing to the right-hand side; their
energy goes to heat and carcass (R9), never back into `E_tot`.

*Precondition R5 — the non-negative trait domain.* The property test first failed on a
case where `World::new` had seeded a founder with `heterotrophy = −2.13` (#444) under
`maintenance_cost_exponent = 3`: `(−2.13)³ × c_h` is negative, the agent's metabolic
cost fell below `B`, and metabolism *paid* it. That is a second energy tap outside R1.
It is reachable only through the #444 seeding gap (mutation floors every trait at 0),
and only under an odd or non-integer exponent (non-integer gives NaN instead). Recorded
as obstruction O3 below; the test pins the exponent even.

## Theorem — what the committed budget does bound

Summing Lemma 2 over `t = 0 … T−1` and using Lemma 1:

```
B · Σ_{t<T} N_s(t)  ≤  E_tot(0) − E_tot(T) + Σ_{t<T} P(t)  ≤  E_tot(0) + T·P_max.
```

**(i) Long-run mean survivor count.**
`(1/T)·Σ_{t<T} N_s(t) ≤ P_max/B + E_tot(0)/(B·T)`. For any tolerance `ε`, once
`T ≥ t_0 := E_tot(0)/(B·ε)` the mean number of agents that survive a tick is at most
`F·m²/B + ε`. **`N̄_max = F·m²/B`.** The instantaneous `N(t)` is *not* bounded — a
hoarded allocation (below) can be discharged into a Poisson-sized brood of newborns that
each live a few ticks — so the count claim is necessarily time-averaged. Because the
death threshold is peak-relative (R12) and per-offspring energy is
`allocation·efficiency·(1 − propagule)/count` with unbounded Poisson `count` (R7), there
is no config-only "minimal viable body energy" to divide `E_max` by; the metabolic route
above replaces the issue's suggested corollary and needs no such floor.

**(ii) Throughput budget.** The same sum applied to every non-negative heat term in
R5/R10 gives a family of time-average bounds: mean survivor structure
`≤ P_max/c_s` (via `c_s·structure` in R5), mean total trait maintenance `≤ P_max`, mean
movement spend `≤ P_max`, etc. Each is a statement about *flows*, not stocks.

**(iii) What does not follow.** Lemma 2 drains `B` *per survivor*, not per unit of
energy. A bound on the stock `E_living` needs `N_s(t)` to be large whenever `E_living(t)`
is — a per-agent energy ceiling — or a drain proportional to the stock. The committed
rules provide neither for reserve and the reproductive allocation (the only per-unit
charge is `c_s` on structure, R5). The next section shows this gap is real, not an
artefact of the proof.

## The requested bound does not exist: the obstruction

**Obstruction O1 — the reproductive allocation is an untaxed, uncapped stock with no
guaranteed outflow.** Consider one producer with `photosynthetic_absorption = 1`,
`kappa = 0.5`, `asexual_propensity = 0`, `dispersal = 0`, `mobility = 0`, alone on the
viable baseline (`F = 10`, `B = 0.3`, `r = 8`). Each tick:

- it receives the full flux `F` (R2, `W_i = w_i`);
- it pays `c = B + c_a + c_s·structure < F` (R5);
- `grow` mobilises the surplus above its retention buffer and adds the `(1 − kappa)`
  half to `repro_reserve` (R6, line 525), which nothing charges (R5 has no term in it),
  nothing caps, and nothing spends: reproduction needs an asexual roll that never
  succeeds at propensity 0 or a mate that does not exist (R7, line 1211 and the sexual
  search), so the allocation is never drawn down. Its nutrient earmark clears the
  threshold too, so the energy gate is the only one in play and it is *passed*
  (allocation `>` `reproduction_energy_threshold = 15` from tick 3 on, nutrient earmark `>` its threshold from tick 2) without effect;
- the kappa half goes to structure, whose `c_s` maintenance rises until growth's
  co-limitation and the shrinking surplus settle it — so the *taxed* stock saturates;
- wear is `0` on the baseline, and where it is not, repair is funded first from the
  soma share (R6) and world-rules require an equilibrium "degraded but still
  functional" — so wear does not force death either.

`E_living(t)` therefore increases by `≈ (1 − kappa)·(F − c(t))` every tick, converging
to a positive constant: the witness test measures **+281.7 energy over ticks 50–100 and
+504.0 over ticks 100–200** (≈ 5.6 → 5.0 per tick), with `N = 1` throughout and the
allocation past the reproduction threshold at every step. For any candidate `E_max`
there is a `t` with `E_living(t) > max(E_living(0), E_max)`. No bound of the requested
form exists under the committed rules.

The obstruction is *not* an income term unbounded by flux (Lemma 1 rules that out); it is
a **drain term that does not scale with the stock**. It survives every point of the
genesis search box: `mean_asexual_propensity` may be `0`, `mean_kappa` may be `< 1`, and
a lineage's asexual propensity can mutate to `0` (R7's clamp) and its compatible mates
can be out of reach at any time. The lone producer is only the cleanest witness; a
population in which some lineage is reproductively isolated hoards the same way.

**Obstruction O2 — the carcass pool is untaxed and never decays** (R9). Even if O1 were
closed, `E_tot` is unbounded: a producer population whose dead are outside any
decomposer's reach deposits structure into carcasses every tick and nothing removes it.
This is the energy-side twin of the nutrient-lockup finding in viability.md — the same
endogenous "achievable decomposer reach / carcass richness" term that blocks a clean
lockup gate blocks a bound on `E_tot`. For `E_living` alone O2 matters only through the
carcass-to-living inflow `C_in(t)` (R8), which the rules bound per consumer per tick by
`base_trophic_efficiency × effective_heterotrophy` but not in aggregate.

**Obstruction O3 — negative founder traits under an odd exponent** (#444). With a
founder trait `< 0` and `maintenance_cost_exponent` odd, R5's per-trait term is negative
and metabolism becomes a source. Reachable from the search's `InitialDistribution`
(`mean_* = 0`, `trait_covariance > 0`) with `maintenance_cost_exponent = 3`. Closed by
the #444 fix (floor founder traits at 0, as mutation already does); until then Lemma 2
holds only on the non-negative trait domain the world rules assume.

**Obstruction O4 — `light_competition_radius = 0`.** With `r = 0` the strict test at
line 62 is never true, every producer receives the full `F`, and `P(t) = F·N_P(t)` with
no config-only cap (Lemma 1's `m` is undefined). The search box keeps `r ≥ 1`, so this is
a legal-parameter corner rather than a live risk; it is the one point where the issue's
"income term not bounded by flux" reading does apply.

## What commitment would restore the bound

The minimal change is to make **living energy dissipative in itself**: a per-unit
holding cost `h > 0` per tick on reserve and on the reproductive allocation — the same
shape as the committed `c_s·structure` term in R5, extended to the other two living
stocks. (Equivalent alternatives: a cap on the allocation with overflow dissipated, or a
forced reproductive discharge at the cap — but both still leave unallocated reserve
untaxed and so bound only the allocation; the holding cost is the one that closes O1 for
every living stock at once.) Nothing here is committed; this is the closed form the
commitment *would* buy, so that it can be judged.

With `h' = min(h, c_s) > 0`, Lemma 2's argument gives, for survivors,
`D(t) ≥ h'·E_living(t)`, hence

```
E_living(t+1)  ≤  (1 − h')·E_living(t) + P_max + C_in(t).
```

In any world where `C_in ≡ 0` — no carcass pool, or no consumer able to reach it — this
is a contraction toward `P_max/h'`, and by induction

> **`E_living(t) ≤ max(E_living(0), P_max/h')` for all `t ≥ 0`**, i.e. `t_0 = 0`:
> if `E_living(t) ≥ P_max/h'` the right-hand side is `≤ E_living(t)`; otherwise it is
> `< P_max/h'`.

With `E_max = F·m²/h'`, the count corollary sharpens from a time average to an
instantaneous statement for the agents that survive a tick: every survivor paid `≥ B`,
and the budget it paid from was at most `E_living(t) + P_max`, so
`N_s(t) ≤ (E_max + P_max)/B` — still without any minimal-body floor.

The general case keeps `C_in(t)` as a separate term. Bounding it needs a bound on
aggregate consumer reach or on carcass richness — exactly the reserve knob viability.md
declines to spend for the lockup gate — so a bound on `E_living` in a world with a
reachable carcass pool is contingent on that same future commitment, not on anything
new. Per consumer the term is already self-limiting: intake `e·h_i` against maintenance
`B + c_h·h_i^p` nets at most `max_h(e·h − c_h·h^p) − B` per tick, a closed form in
committed parameters; what is missing is a bound on how many consumers a hoarded pool
can stand up at once.

## Authority boundary

- The lemmas are exact consequences of R1–R12 as realised in the stepper; the property
  tests exercise them over the search domain (`support::world_case_integer_exponent`,
  worlds `≤ 30` extent, `≤ 40` founders, `≤ 20` ticks) and mutation-checks confirm each
  test fails on a perturbed bound.
- The theorem's `N̄_max` is a *necessary* ceiling on sustained population, not a
  prediction of it — an ecology can sit far below `F·m²/B` (most do, since every real
  agent pays more than `B`).
- The obstruction is a statement about the *rules*, decided by a single witness; whether
  real genesis worlds hoard to any meaningful degree is an empirical question this note
  does not answer. What it does say is that "living energy is bounded by the solar
  budget" is not a theorem of the current design, and any viability argument that
  assumes it is under-committed at flow 9.

## Deliverables against the acceptance criteria

- Research doc with the standard header; nothing added to `docs/system-design/` — this
  file.
- Bound or obstruction: **obstruction**, stated precisely as O1 (flow 9's reproductive
  allocation: untaxed, uncapped, discharged only by an unguaranteed event), with a
  one-agent witness; the partial results that *do* hold are Lemma 1 (`P_max = F·m²`),
  Lemma 2, and the count theorem `N̄_max = F·m²/B`.
- Transient stated: `t_0 = E_tot(0)/(B·ε)` for the mean-count bound; `t_0 = 0` for the
  conditional bound under the named commitment.
- Rules relied upon enumerated in one place: the R1–R12 table.
- No change to the stepper, evaluator, or any committed document; the only code is
  `crates/explorers-sim/tests/prop_energy_bound.rs` (three tests, ~0.2 s).
