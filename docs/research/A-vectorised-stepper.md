# Research Brief A — Vectorise the inner stepper (SoA, masked arena, batched rollouts)

**Status: research finding. Commits nothing.** This is the Cluster-A deep-dive for epic
[#345](https://github.com/) (issue [#346](https://github.com/)). It is a recommendation, not an
implementation, and deliberately lives outside `docs/system-design/` — the system-design layer is
self-justifying and commits the design; this layer records investigation that *informs* a future
commitment. Any design delta named here (an `N_max` physics term, a determinism note, the
implementation-language fork) becomes real only when it is committed in `world-rules.md` /
`execution-model.md` through a deliberate design pass.

## TL;DR

1. **The fork (one physics implementation vs. two) is being posed one step too early.** The outer
   loop — `vmap(vmap(...))` over (config × seed) — is *embarrassingly parallel and currently runs
   sequentially*. Claiming CPU parallelism (rayon over the ensemble and the search batch) is a
   ~cores× win, **bit-identical** to today, with **zero divergence risk and one source of truth**.
   It should happen first and is independently shippable.
2. The inner `scan(step)` vectorisation (SoA + GPU) is the genuine order-of-magnitude-beyond prize,
   but it is the part that **touches committed semantics** (determinism, a new `N_max` fecundity
   ceiling) and creates the second-source-of-truth liability. It is **not ready for an agent to
   build** — it needs a design pass to commit those deltas and to settle the language fork.
3. **The free win is real but half a win.** A carcass already carries the dead agent's exact trait
   vector, so death becomes a per-row mask flip on a fixed arena. But *births still need slot
   allocation*. The dynamic-cardinality obstacle is halved, not removed.
4. **When the inner stepper is built, prefer Rust-native GPU over JAX** — it preserves the
   single-language / "unify the canonical model" direction the design is already travelling. JAX is
   only the better bet if differentiability becomes a first-class need, which the epic explicitly
   rules out.

## What the code actually is today (the baseline this brief reasons from)

All citations are against `crates/explorers-sim/` at the time of writing.

- **Representation is Array-of-Structs.** `Vec<Agent>` plus a parallel `Vec<Carcass>`
  (`lib.rs:780–781`). `Agent` (`lib.rs:551–578`) holds `reserve, structure, peak_structure,
  nutrient, position, traits, contact_time, wear[3], repro_reserve, repro_nutrient`. `TraitVector`
  is a 7-field value struct (`lib.rs:42–69`), copied into every agent and every carcass.
- **`Carcass` carries the full `TraitVector`** (`lib.rs:683–690`) — identical layout to a living
  agent's traits. This is the free win, confirmed.
- **The tick is a single `World::step()`** (`lib.rs:986–1389`) whose phase order exactly matches
  `execution-model.md`: grid → photosynthesise → absorb → metabolise → grow → resolve_drains →
  (retain dead) → resolve_reproduction → move → wear → check_death → (retain dead) → ledgers →
  event log.
- **Deaths are `Vec::retain`** (`lib.rs:1087`, `lib.rs:1188`); **births are `Vec::push`**
  (`lib.rs:1106–1110`). Cardinality is fully dynamic. There is **no arena cap** in the sim; the
  *eval* layer has a coarse `max_population` early-break per rollout (`genesis/src/lib.rs:49–51`).
- **RNG is a single sequential `ChaCha8Rng::seed_from_u64(seed)`** per world
  (`lib.rs:798–799`), threaded through the tick. It is consumed only in **reproduction** (asexual
  propensity roll `phase.rs:964`; per-trait mutation `phase.rs:1053–1055`; dispersal placement
  `phase.rs:1068–1069`; sexual seed-parent tie-break `phase.rs:1297`) and **movement** (random-walk
  angle/magnitude `phase.rs:834–837`). All other phases consume no randomness.
- **Per-seed runs are already independent.** `run_single` builds a fresh `World::new(…, seed)` per
  seed (`genesis/src/lib.rs:32`), so each seed owns its own RNG stream. This is what makes the CPU
  parallelism win bit-identical.
- **The ensemble runs sequentially** — `(0..ensemble_size).map(…)` (`genesis/src/lib.rs:75–80`).
  No `rayon`, no threads, no `par_iter` anywhere in the workspace.
- **Two conservation ledgers** run every tick: energy (solar-in = dissipation + retained,
  `energy_ledger.rs`) and nutrient (closed: endowed pools = retained pools to `5e-3` relative,
  `nutrient_ledger.rs:92–172`).

## AC1 — The fork: one physics implementation vs. two

**Recommendation: do not fork yet. Stage it.**

The fork only bites for the *inner* `scan(step)` vectorisation. The `vmap(vmap(...))` outer two
loops — config and seed — are independent rollouts and parallelise on the CPU with no new physics
implementation at all. That reframes the decision:

| Path | Speedup | Sources of truth | Divergence risk | Touches committed semantics |
|---|---|---|---|---|
| **rayon over (config × seed)** | ~cores× (10–50× typical) | **one** | **none** (bit-identical) | no |
| **JAX SoA/GPU inner stepper** | + ~10–100× on top | **two** (Rust + JAX) | high; weak oracle | yes (RNG, `N_max`) |
| **Rust-native GPU (cubecl/wgpu/burn)** | + ~10–100× on top | one language, two kernels | high; weak oracle | yes (RNG, `N_max`) |

When the inner stepper *is* built, the recommendation is **Rust-native GPU**, for three reasons:

1. **It preserves the direction the design is already travelling.** `viability.md`'s standing
   follow-up is to *"unify the canonical parameter/form model across genesis, scenarios, and
   viability."* A JAX stepper adds a *third* model in a *second language*, pulling hard against that.
   Rust-native keeps one language and lets the trait/parameter structs and scalar physics helpers be
   *shared*, not transcribed — shrinking the divergence surface to the kernels themselves.
2. **The differential-testing oracle is weak, which argues against any second implementation.**
   Counter-based RNG (below) means bit-identical replay against the current stepper is *gone by
   construction*; the only available oracle is *ensemble-statistical equivalence* (same seed →
   statistically-equivalent ensemble, not the same trajectory). Detecting a subtle divergence in one
   of ~40 parameters through noisy ensemble statistics is genuinely hard, and the test is a
   permanent CI tax. The fewer implementations that must agree, the better — and one-language
   minimises the part that can drift.
3. **The autodiff pull that would justify JAX is explicitly out of scope.** The epic rules out full
   differentiable simulation ("Gradients are not all you need"), and brief E's inner emulator (E2)
   "drifts exactly near bifurcations." So the one thing JAX is uniquely good at is not a need here.
   *If* that changes — if E2 or a differentiable objective becomes load-bearing — re-open the fork;
   until then it is a liability, not a feature.

The honest cost statement: a second stepper is a **permanent maintenance tax on a part of the system
that is under active iteration** (recent commits keep changing the physics: nutrient-lockup failure
mode #344, obligate decomposers #341, trophic forms #294). Every such change would land twice and be
re-validated through a noisy oracle. That tax is the spine of the decision, and it is why the
recommendation is to extract the *free, fork-less* throughput first and only pay the tax if the CPU
ceiling proves insufficient for the interactivity goal (user story 3).

## AC2 — The coordinated-phase seam: what vectorises, what resists

Classifying every phase by how it maps to SoA:

**Cleanly elementwise (trivial vmap / per-row SoA):**
- **metabolise** (`phase.rs:156–184`) — fixed per-agent costs, no grid read.
- **grow** (`phase.rs:189–301`) — kappa split + Liebig co-limit; per-agent, nutrient-aware, no grid.
- **wear** (`phase.rs:309–349`) — reads a precomputed per-agent usage map; per-row given the map.
- **death checks** — masked threshold tests; a per-row flag flip, not a removal (see AC's free win).

**Gather-then-elementwise (a neighbour reduction feeds a per-row update):**
- **photosynthesise** (`phase.rs:19–84`) — structure-weighted light split among co-located
  producers. This is a clean **segment reduction**: bucket producers by light-competition cell, sum
  `eff_photo · structure` per cell, then each agent's income is its weighted share. Vectorises well.
- **move** (`phase.rs:752–892`) — chemotaxis reads living neighbours via the grid (`phase.rs:787`)
  *and linearly scans all carcasses* (`phase.rs:812–828`). A neighbour gather + reduction, then a
  per-row position update. Vectorises, but the all-carcass scan needs the carcasses in the spatial
  index too (today they are not).

**Sparse scatter-gather with a barrier (the real engineering, but tractable):**
- **resolve_drains** (`phase.rs:386–681`) — two passes (living, then carcass), each: per-target
  gather of in-range consumers (variable fan-in), `total_demand` per target (**segment reduction**),
  proportional split, then **scatter** drains onto targets and gains onto consumers. The
  sustained-contact term (`demand ∝ contact_time/(contact_time+K)`, `phase.rs:460–465`) and
  trait-distance trophic efficiency (`phase.rs:466–470`) are per-(consumer,target) edge weights —
  standard for a segment-reduction kernel. Within-pass death marking is a mask flip. **The two-pass
  "deaths immediately reflected" rule maps to a hard barrier between the drain pass and the
  reproduction pass** — but it is a *per-lane* barrier (each seed syncs independently), so it costs
  nothing across the batch.

**Resists clean vectorisation (the genuine obstacle):**
- **resolve_reproduction** (`phase.rs:910–1435`) — two problems:
  1. **Greedy global mate-matching by trait-space distance** (`phase.rs:1130–1196`): collect
     compatible candidate pairs, sort by trait distance, assign each agent at most once. Greedy
     matching is serially dependent and **orthogonal to the spatial grid** (it is a trait-space
     nearest-neighbour problem). It does not data-parallelise without *changing the matching rule*
     (a parallel approximate matching would alter the physics → divergence). Realistic options: a
     serial per-lane matching kernel (cheap if eligible sets are small), or an explicitly-committed
     approximate matcher (a design change, not a transparent optimisation).
  2. **The asexual→sexual ordering** (asexual fires first and removes winners from the sexual pool,
     `phase.rs:1120–1124`) is a sequential dependency within the phase.
  3. **Births need slot allocation** — the dynamic-cardinality residual (see free win).

**Seam summary:** 4 phases are free, 2 are gather-then-elementwise, drains are a well-understood
segment-reduction+scatter, and **reproduction is the one phase that genuinely resists** — and it
happens to be the phase that also owns RNG consumption and birth allocation. Reproduction is where
the engineering risk, the determinism question, and the cardinality problem all converge. Any spike
should target reproduction *first*, because the rest is comparatively routine.

## AC3 — Feasibility + speedup of batching over (config × seed); commitments touched

- **CPU (rayon over config × seed):** trivially feasible — swap `.map` for `.par_iter` in
  `run_ensemble` and over the LHS/Sobol/BO batch in search. Near-ideal scaling (rollouts are fully
  independent). **~cores× — 10–50× on typical hardware. Touches no design commitment** (it only
  parallelises; per-seed determinism is preserved exactly). Bit-identical to today.
- **GPU (SoA inner stepper, batched over config × seed × within-rollout):** feasible with real
  engineering, dominated by the coordinated phases. Expected **another ~10–100× on top of a
  saturated CPU**, but **Amdahl-bounded by the irregular phases** — the greedy mate-match and the
  proportional-split scatter set the floor, and an individual-based world with variable fan-in and
  dynamic births is a scatter-heavy, branch-heavy workload (the less-favourable end of the GPU
  range, unlike a regular grid CA). **Commitments touched:** determinism (RNG must go counter-based;
  bit-identical replay against the current stepper is lost) and a new `N_max` arena cap (a
  density-dependent fecundity ceiling absent from the world rules — see AC4).

The practical sequencing consequence: **rayon first also measures the ceiling.** Once cores are
saturated you know empirically whether single-config-ensemble latency is still the bottleneck — i.e.
whether the GPU prize is worth its permanent tax — instead of guessing.

## AC4 — Determinism, and `N_max`-saturation as the population-explosion proxy

**Determinism.** `architecture.md` states the load-bearing invariant plainly: *"Determinism per
seed is the load-bearing invariant — everything above it relies on being able to replay a run
exactly."* The current single sequential `ChaCha8Rng` cannot be threaded through a `vmap`/parallel
arena. Counter-based RNG (Philox/Random123) keyed by `(seed, tick, agent_id, phase)` restores
**seed-reproducibility under parallelism** and therefore *satisfies the invariant as stated* — same
seed → same run, within the new implementation. What it does **not** preserve is **bit-identical
identity with the current Rust stepper**. Two concrete consequences:

1. The CPU rayon win is **exempt** — each seed already owns its own `ChaCha8Rng`
   (`genesis/src/lib.rs:32`), so parallelising across seeds changes nothing about any single seed's
   stream. Rayon is bit-identical; only the GPU rewrite breaks replay.
2. Tests that assert *exact* outcomes — e.g. the genesis determinism test comparing
   `termination_tick` across repeats (`genesis/src/lib.rs:218–253`), and the headless scenario
   tests — **would not transfer** to a vectorised stepper. They would have to be re-expressed as
   ensemble-statistical assertions. This is part of the differential-testing tax and a concrete
   reason the rewrite is not a transparent optimisation.

**`N_max` saturation as a free explosion signal — confirmed, and must be surfaced.** A fixed-capacity
arena introduces a density-dependent fecundity ceiling that the world rules do not contain: once the
arena is full, births that cannot get a slot are dropped or deferred — *that is a new physics term*,
and it bites exactly in the **population-explosion** regime, which `viability.md` files as a dynamics
failure "beyond cheap a priori reach." Two implications:

- **It must never be silent.** A silent cap would corrupt the physics in precisely the regime you
  most want to characterise, and would *throw away* a cheap signal. The recommendation is to emit a
  per-rollout `arena_saturation` metric (peak fill fraction, ticks-at-cap) into the
  `FitnessBreakdown` / `FailureMode` path and treat sustained saturation as a population-explosion
  vote — a *finer-grained, per-tick* refinement of the coarse `max_population` early-break that the
  eval layer already uses (`genesis/src/lib.rs:50`).
- This is a genuine **bonus** of the SoA arena that partially offsets the second-source-of-truth
  cost: the cap you are forced to introduce doubles as a detector for a failure mode that currently
  has no static gate.

## The free win — quantified

Confirmed by the code: `Carcass` carries the full `TraitVector` (`lib.rs:683–690`), and drain
resolution runs the *same* proportional-split algorithm over living and carcass targets
(`phase.rs:421–541` vs `563–673`). So on a masked SoA arena:

- **Death = a per-row `kind` flip** (living → carcass), keeping the row's trait vector and position
  in place. No remove, no reallocate, no compaction. This replaces *two* AoS operations — the
  `retain` removal (`lib.rs:1087`, `1188`) and the separate `Vec<Carcass>` push — with one mask
  write.
- **But births still need a slot.** A new agent needs a free row, an id, and trait mutation. The
  arena therefore needs a slot allocator (free-list or bump-with-cap). The free-list is fed by
  **fully-decomposed carcasses** (energy and nutrient drained to zero → row reclaimable), which is
  clean, but the arena must budget living + carcasses-in-transit + birth headroom together — and
  *that* budget is the `N_max` whose saturation is the explosion signal above.

**Verdict:** the free win removes the death half of dynamic cardinality entirely and for free. The
birth half remains and is exactly where `N_max` (and its new physics term) enters. So: *halved, not
solved* — and the residual half is load-bearing, because it is where the one genuine semantic change
lives.

## What A cannot over-reach (authority boundary)

Unlike the F/E cluster, A is a *throughput* bet: it runs the **actual individual-based physics**,
per seed, just faster and in parallel. It therefore **preserves per-seed distributional emergence** —
the sporadic-per-seed decomposer guild (`trait-space.md`, "Decomposer is a behavioural role")
survives untouched, because A simulates the same stochastic individual events. The *only* things A
perturbs are (1) RNG-stream identity — runs differ from the current stepper but remain valid draws
from the same distribution — and (2) the `N_max` ceiling in the explosion regime. A never erases the
distributional layer the way a mean-field operator (F) would. This is A's strength and the reason it
is the safe member of its cluster to build.

## AC5 — Recommended next move

**Are we ready for something actionable? Yes for the throughput win; not yet for the rewrite.**

1. **Ready for an agent now — parallelise the outer loop with rayon.** `par_iter` over the ensemble
   in `run_ensemble`, and over the LHS/Sobol/BO batch in search. Bit-identical, one source of truth,
   no design delta, ~cores× wall-clock. It directly serves "interactive rather than batch" (user
   story 3), is independent of the fork, and *measures the ceiling* that decides whether the rewrite
   is worth it. Tracer-bullet, low risk. → issue with `ready-for-agent`.

2. **Not yet agent-ready — the vectorised SoA/GPU stepper needs a design pass first.** It is blocked
   on *committing* three deltas in system design, not on engineering: (a) the determinism note
   (counter-based RNG; replay-identity with the current stepper is deliberately abandoned), (b) the
   `N_max` arena cap as a real, measured physics term with arena-saturation surfaced as a
   population-explosion signal, and (c) the language fork (recommended: Rust-native GPU). Because it
   *changes committed semantics*, an agent must not build it silently. → issue routed through
   `/grill-with-docs` to land those commitments, after which a scoped spike (target reproduction
   first — the phase where matching, RNG, and births all converge) becomes writable as a build issue.

The go/no-go gate between them: **build the rewrite only if, after rayon saturates the cores,
single-config-ensemble latency is still the thing blocking interactive iteration.** If CPU
parallelism alone makes the loop interactive, the second source of truth is a tax with no buyer.
