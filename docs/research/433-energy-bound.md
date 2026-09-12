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
The [B2 section](#b2--empirical-check-issue-438) at the end checks the lemmas against
2256 real genesis runs and measures the obstruction at population scale.

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

## B2 — empirical check (issue #438; re-run under #476)

**Status: measurement. Commits nothing; changes nothing in the stepper, evaluator, or
search.** `crates/explorers-search/src/bin/energy_bound_check.rs` runs the lemmas
above against real genesis worlds and measures the obstruction. It is a diagnostic,
not a CI gate:

```
cargo run --release -p explorers-search --bin energy_bound_check   # ~20 min, 2256 runs
```

Artifact: `target/energy-bound-check.json` (gitignored). Subset selectors for
development: `ENERGY_BOUND_CONFIGS=atlas:0,sample:12`, `ENERGY_BOUND_SEEDS=2`.

**Change from the previous run.** The numbers below were re-taken under #476 on the
stepper after the six post-atlas fixes (#444–#453, the last of them at `5a7bede`) and on
the atlas #474 regenerated on that stepper (82 live cells, was 56). The 200 `sample:i`
configs are the same seed-421 draw as before, so those labels stay comparable; the
`atlas:i` cells are new. Three things moved, and one explanation covers the direction of
all three. (i) Extinctions fell from 249 of 2048 runs to 76 of 2256 — the founder floor
(#444) means no founder is culled on tick 1 any more (0 of 66 800 founders carry a
negative trait, where before 57 % did), so populations that used to lose most of their
founders before their first real tick now live; #474 reads the same rotation off the
atlas's dead frontier, and #485 records that the metabolic overdraft cap (#445) also
moves trajectories, so the per-cell attribution is not sharper than "the fixed stepper".
(ii) Lemma 1 is less often tight on the count branch (median ratio 1.000 → 0.933;
1563/2048 → 1087/2256 runs at `≥ 0.999`): with the full founder roster alive, producers
more often have a competitor within `r`, so the light share is split rather than taken
whole. (iii) Every tightness ratio in Check 2 roughly doubled (realised Lemma 2 median
0.057 → 0.119; asymptote median 0.0014 → 0.0041) and the obstruction grew (growth median
3.8× → 7.6×; peak at the horizon 704/1799 → 899/2180 survivors) — larger, longer-lived
populations pay more base metabolism against the same throughput and hoard more. No
finding changed: zero violations before, zero now; the allocation is still where living
energy goes; nothing NaN'd, now for the structural reason that the founder domain is
non-negative rather than because the cull leaked in the safe direction. The #444
"two findings" section of the previous version is gone with the artefact it described.

### Design

- **Configs.** The 82 atlas live-cell `unit` vectors, decoded with `decode` over
  `default_ranges` exactly as the search replays them, plus 200 low-discrepancy points
  of the unit cube — the same fixed-seed LHS draw `role_emergence.rs` uses (seed 421;
  the crate's `sobol.rs` is the Saltelli *indices* estimator, not a sequence generator),
  so `sample:i` names the same config in both instruments. Together they cover the
  known-coexisting regimes and the extinction/monoculture regimes the atlas cannot
  replay.
- **Runs.** 8 fixed contiguous seeds (1000–1007) per config, the 500-tick search
  horizon, and `run_single`'s early stops (empty / `> max_population`). Every
  (config, seed) is independent, so the artifact is byte-identical across invocations
  (verified: two full runs, identical SHA-1).
- **Per tick** the instrument reads `N_P(t)` (agents with `photosynthetic_absorption > 0`
  and `structure > 0`) and the id set before `step()`, then `P(t)` as the increment of
  `World::total_solar_input`, `N_s(t)` as the ids still present, and the stocks
  `E_living = Σ(reserve + repro_reserve + structure)`, the allocation
  `Σ repro_reserve`, and `E_tot = E_living + Σ carcass.energy`.
- **Checks** (tolerance `1e-4` relative, as in `prop_energy_bound.rs`):
  1. Lemma 1 per tick: `P(t) / (F·min(N_P(t), m²))`.
  2. The Theorem at every prefix `T ≤ 500`, transient included:
     `B·Σ_{t<T} N_s(t) / (E_tot(0) + T·P_max)`; and the summed Lemma 2 with *realised*
     income, `B·Σ N_s / (E_tot(0) − E_tot(T) + Σ P)` — the tighter first inequality of
     the Theorem's proof, and the one a second energy tap (O3) would trip. The
     asymptotic `N̄_max` is reported as the whole-run mean of `N_s` over `F·m²/B`
     (tightness only — it is not a bound at finite `T`).
  3. The obstruction: `max_t E_living(t)` against the naive cumulative-solar envelope
     `E_tot(0) + t·P_max` at the peak tick, the allocation's share of that peak, and
     the peak over the endowment `E_tot(0)`.
- **NaN guard.** The instrument still counts founders with a negative metabolic trait
  and stops a run's checks at the first NaN in `P`, `E_living` or `E_tot`. Since the
  founder floor (#444) both counters read zero on every run; the guard stays as the
  falsifier of that floor, not as a working exclusion.

### Results (82 + 200 configs × 8 seeds = 2256 runs, horizon 500)

Terminal modes: 2180 survived, 76 extinct, 0 exploded. **No run produced a NaN**,
and **no run violated Lemma 1, the Theorem, or the summed Lemma 2.**

| Check | observed / bound | n | min | q1 | median | q3 | max |
|---|---|---|---|---|---|---|---|
| 1 Lemma 1 | `max_t P(t) / (F·min(N_P, m²))` | 2256 | 0.113 | 0.714 | 0.933 | 1.000 | 1.0001 |
| 2 Theorem | `max_T B·ΣN_s / (E_tot(0) + T·P_max)` | 2256 | 0.000 | 0.0027 | 0.0066 | 0.0148 | 0.116 |
| 2 Lemma 2 summed | `max_T B·ΣN_s / (E_tot(0) − E_tot(T) + ΣP)` | 2256 | 0.000 | 0.061 | 0.119 | 0.200 | 0.759 |
| 2 asymptote | `mean_t N_s / N̄_max` | 2256 | 0.000 | 0.0016 | 0.0041 | 0.0082 | 0.067 |
| 3 envelope | `max_t E_living / (E_tot(0) + t·P_max)` | 2256 | 0.000 | 0.0068 | 0.0126 | 0.024 | 0.946 |
| 3 envelope, survivors only | same | 2180 | 0.000 | 0.0068 | 0.0123 | 0.024 | 0.946 |
| 3 allocation share at peak | `Σ repro_reserve / E_living` | 2256 | 0.000 | 0.120 | 0.357 | 0.628 | 0.9997 |
| 3 allocation share, survivors | same | 2180 | 0.000 | 0.122 | 0.363 | 0.633 | 0.9997 |
| 3 growth | `max_t E_living / E_tot(0)` | 2256 | 0.000 | 2.38 | 7.62 | 27.1 | 40 932 |

**Check 1 — Lemma 1 holds and is tight on the count branch, less often than before.**
The max ratio is 1.0001 (f32 summation of per-agent shares); 1087 of 2256 runs reach
`≥ 0.999` and the median is 0.933. That is the `N_P` branch of the `min`, not the `m²`
branch: in 1698 runs the peak population never exceeds `m²`, so the cap is `F·N_P` and
a ratio of 1 means every producer at that tick had no competitor within `r` and took
the full flux. The drop from a median of 1.000 is the founder floor's doing — the whole
founder roster now lives past tick 1, so a producer more often shares its light — and
the `m²` geometric branch still never binds in this domain, so its non-tightness (noted
under Lemma 1) is invisible here.

**Check 2 — the Theorem holds with two orders of magnitude to spare.** The config-only
form is loose because `P_max = F·m²` assumes every one of `m²` light cells is occupied;
the realised form (summed Lemma 2) is the honest tightness read and its median is
0.119: survivors' base charge `B·N_s` is ~12 % of the energy that actually passed
through the system (was ~6 %), the rest going to trait/structure maintenance, movement,
transfer loss and the stocks. The worst case (0.76, `sample:155` seed 1007, then
`sample:192` seeds 1007 and 1006 at 0.76 and 0.75) is a survivor population that
converts most of its net budget into `B`. The asymptotic `N̄_max = F·m²/B` is nowhere
approached (max 0.067, `sample:70` seed 1005, `N̄_max = 51`, peak population 24 — the
same cell that topped the previous run at 0.079) — the necessary ceiling on sustained
population is not the binding constraint anywhere in the search box, as the Authority
boundary anticipated.

**Check 3 — the obstruction is real and common, and the envelope says nothing.** The
naive cumulative-solar envelope is enormous (`m²` cells × `F` × `t`), so `E_living`
sits at ~1 % of it at the median; the maxima (~0.95, `sample:29` and `sample:69`) are
all peaks at tick 1, where `E_living ≈ E_tot(0)` and the envelope is only one `P_max`
wider. The informative numbers are the other two rows:

- **The reproductive allocation is where living energy goes.** At the peak, the median
  run holds 36 % of `E_living` in `repro_reserve`; the upper quartile holds 63 %;
  815 of 2180 surviving runs hold more than half; the maximum is 99.97 % (`sample:95`).
  Atlas cells (survivor median 0.40) hoard more than the cube sample (0.34).
- **The stock is still rising at the horizon.** 899 of 2180 surviving runs have their
  `E_living` peak at tick 500 itself — the search horizon truncates a growing stock, it
  does not see a plateau. `sample:131` reaches 40 932× its endowment (1 256 747 energy
  from 30.7, 99 % of it allocation, population 3841 < `max_population`), and
  `sample:147` — the previous run's extreme — 20 585× (478 621 from 23.3, 86 %
  allocation, population 953), with no bound in sight.

This is the population-scale version of the lone-producer witness of O1: not an odd
corner but the median behaviour of the search box. Any viability argument that treats
"living energy is bounded by the solar budget" as given is under-committed at flow 9,
exactly as the obstruction section says; and the search's fitness surface is being
read off runs whose dominant energy stock is one the rules never discharge.

### Deliverables against the acceptance criteria

- Deterministic across two runs (identical SHA-1 of `target/energy-bound-check.json`);
  artifact path documented above; not committed.
- Zero violations of any proven bound; nothing to file.
- The three ratio distributions (median, quartiles, max) are the table above.
- The instrument asserts nothing about emergent values; its unit tests cover only the
  pure parts (`bounds`, `check_run`, `distribution`) on synthetic series.
- No change to the stepper, evaluator, or search.
