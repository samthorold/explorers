# Spike #354 — CPU-SIMD SoA inner stepper: measure the single-rollout latency win

**Status: research finding. Commits nothing to the design.** This is the measurement-first
follow-up to [Research Brief A](A-vectorised-stepper.md). Brief A argued the *outer* loop (config ×
seed) was the free, fork-less win (shipped in #353) and that the *inner* SoA/SIMD stepper was the
order-of-magnitude prize but carried a second-source-of-truth tax. This spike tests the narrowest,
safest slice of that prize — can the *elementwise* phases be vectorised **bit-identically** to the
scalar stepper, and is the win worth having? — and reports the numbers.

The spike ships real code: an SoA agent store (`crates/explorers-sim/src/soa.rs`) and SoA
implementations of the three provably-elementwise phases, guarded by a differential bit-identity test
(`crates/explorers-sim/tests/soa_differential.rs`) and benchmarked by a bin
(`crates/explorers-sim/src/bin/bench_soa.rs`). It changes **no committed semantics**: no RNG change,
no `N_max`, no arena cap. The AoS `Vec<Agent>` stepper remains the one source of truth; the SoA store
is an additive mirror used only by the new phases and their tests.

## TL;DR

1. **The bit-identity thesis is confirmed for the elementwise phases.** `metabolise`, `grow`, and
   `apply_wear` ported to an SoA layout are **bit-identical** to the scalar versions — every agent
   field, every event, every dissipated total, asserted by `f32::to_bits` equality over the full
   example4 and example9 trajectories (200 ticks each, on the live evolving population). Because the
   SoA loop visits agents in the same index order and performs the same floating-point operations in
   the same order, there is no reordering to break — identity holds **by construction**, not by luck.
2. **But the elementwise phases own a *negligible* share of the rollout floor.** On example9 (the
   thriving world, the actual latency target) the three phases are **~0.2 % of a tick**. On example4
   (a small world) they are **~1.9 %**. The ~3.5–5 s single-rollout floor is **not** in the cheap
   phases — it is in the spatial-query and reduction phases (`photosynthesise`, `resolve_drains`,
   `move`), exactly the ones this spike (by design) did *not* vectorise.
3. **At these population scales SoA is a net loss, not a win.** Measured SoA elementwise time is
   **0.6×–0.8×** (i.e. *slower*) than scalar, because the per-tick SoA build + write-back cost
   dominates the few microseconds of arithmetic, and peak populations (26 / 97 agents) are far too
   small for SIMD lane parallelism to amortise anything. Even integrated (paying the conversion once
   per tick rather than per phase), best-case savings would be a fraction of 0.2 % of the tick.
4. **The reductions verdict: a fixed-order SIMD reduction *can* stay bit-identical, but the binding
   constraint is the gather order, not the reduction.** See below. The honest read is that bit-exact
   vectorisation of these phases is *possible* but *hard*, and — given (2) — not where the time is
   anyway.
5. **Go/No-Go on the follow-up (the hard part: reductions + `resolve_reproduction` + births): NO-GO
   as a latency play.** The measured floor does not live in the phases an SoA/SIMD rewrite would make
   bit-identical cheaply, and the phases where it *does* live are precisely the ones that either force
   the second source of truth (the reductions, the greedy matcher) or are irreducibly serial
   (reproduction). The spike's own numbers retire the latency rationale. (A *throughput* rewrite is a
   separate question; #353's rayon already owns that lever.)

## What was built

- **`AgentSoA`** (`crates/explorers-sim/src/soa.rs`) — a dynamically-sized Structure-of-Arrays mirror
  of `Vec<Agent>`: every field a parallel `Vec` column, with `from_agents` / `to_agents` round-trip,
  `push_agent`, and `swap_remove`. No fixed capacity, no `N_max`, no mask-flip death — births and
  deaths are ordinary column `push` / `swap_remove`, exactly as the AoS store does today.
- **`metabolise_soa`, `grow_soa`, `apply_wear_soa`** — the three elementwise phases ported to operate
  over the columns. The arithmetic is the *same scalar expression* transcribed over column reads
  (one source of truth for the physics: the maintenance-cost expression is factored into a single
  helper shared by metabolise and grow, mirroring the scalar code). The chosen vectorisation strategy
  is **autovectorisation off a clean SoA loop**, deliberately *not* `std::simd` (nightly-gated) or
  `wide`/`pulp` (a dependency and a *second* expression of the math, which would cut against the
  spike's "transparent optimisation, not a second source of truth" thesis). A clean column loop lets
  the optimiser vectorise the same scalar expression the scalar stepper runs.

## AC2 — Bit-identity of the elementwise phases (confirmed)

The differential test (`tests/soa_differential.rs`) is the determinism guard's green bar. For each
phase it runs the scalar AoS version and the SoA version on the same input and asserts:

- every agent field bit-identical (`to_bits()`), including `position`, `reserve`, `structure`,
  `peak_structure`, `nutrient`, `repro_reserve`, `repro_nutrient`, all 7 traits, all 3 wear lanes;
- the returned `Event` stream identical (kind, source, `energy_delta` by bits);
- the returned `dissipated` total identical by bits.

It runs this two ways: (a) on a *warmed* example4 population (50 ticks in, so wear/structure/earmarks
are non-trivial), and (b) over the **full 200-tick trajectory** of both example4 and example9,
re-checking every live agent every tick as the population grows, shrinks, and reorders via births and
deaths. **All six tests pass.** The full workspace suite (411 prior tests + 6 new) stays green and
unchanged — no exact-outcome or scenario test was touched.

This is the spike's central positive result: for the per-agent phases, the SoA rewrite is genuinely a
*transparent optimisation*. There is no determinism-guard stop here — nothing forced a tolerance,
RNG, or ordering change.

## AC3 — The reductions verdict (`photosynthesise`, `resolve_drains`)

**Can a fixed-order SIMD reduction stay bit-identical? Yes in principle — but that is not the binding
constraint, and the constraint that *is* binding makes it expensive.**

The non-associativity of floating-point addition is real but tractable: a tree/pairwise SIMD
reduction differs from a sequential `+=` accumulation, but you can *pin the reduction order* (a
strided lane layout with a fixed combine order) to reproduce the sequential result bit-for-bit. The
reduction itself is not the obstacle. Two deeper obstacles are:

1. **The summation order is the *gather* order, and the gather order is the spatial grid's bucket
   order plus a dedup `HashSet`.** In `photosynthesise` the per-producer `total_weight` accumulates
   over `grid.query_radius(...)` results (`phase.rs:46–67`), de-duplicated through a
   `std::collections::HashSet` (`phase.rs:47–51`). In `resolve_drains`, `total_demand` is
   `drain.consumers.iter().map(...).sum()` (`phase.rs:486`), and `consumers` is built in
   grid-query-then-HashSet-dedup order (`phase.rs:431–473`). **To stay bit-identical, a vectorised
   reduction must consume the addends in *exactly that order*.** That couples the SIMD kernel to the
   grid's internal iteration order and the hash iteration order — fragile, and it pins the entire
   spatial-index implementation as a bit-level contract. It is doable (materialise the gathered list
   in scalar order, then reduce it in that fixed order), but the SIMD win is then fighting a
   gather-bound, irregular-fan-in workload, which is the unfavourable end of the SIMD range.

2. **These phases are scatter/segment-reductions with variable fan-in, not flat elementwise maps.**
   `resolve_drains` is two passes (living, then carcass) of: per-target gather of in-range consumers,
   a `total_demand` segment-sum, a proportional split, and a *scatter* of drains onto targets and
   gains onto consumers — with within-pass death marking. The fan-in is variable per target. This is
   a real segment-reduction kernel, not a `vmap`; getting it bit-identical means reproducing the
   scalar gather/scatter order exactly.

**Verdict:** bit-identity for these phases is *achievable* (fixed-order reduction over a
scalar-materialised gather list) but **not a transparent optimisation** the way the elementwise
phases are — it ties the kernel to the spatial index's iteration order as a correctness contract, and
the workload is gather/scatter-bound. Crucially, per AC4 below, **this difficulty buys almost
nothing**, because the elementwise phases the spike *did* make bit-identical are already a rounding
error in the tick budget — and a full tick-cost breakdown would be needed to know whether the
reductions are even the dominant cost (the spatial *queries* themselves, with their HashSet dedup, are
a strong suspect). Either way, this is not free.

## AC4 — Benchmark table (single rollout, scalar vs SoA elementwise)

Measured via `cargo run --release -p explorers-sim --bin bench_soa` (Apple Silicon, `--release`).
`rollout(s)` is the full scalar `World::step()` loop wall-time (the latency floor). `elem_scalar(s)`
and `elem_soa(s)` are the summed wall-time of the three elementwise phases over the rollout, scalar
vs SoA; `elem_soa(s)` includes the `AgentSoA` build + write-back each tick (the standalone, pessimistic
cost). `elem%tick` is the elementwise phases' share of the tick. `soa_x` is scalar/SoA (>1 = SoA
faster).

| scenario | ticks | peak_pop | final_pop | rollout(s) | elem_scalar(s) | elem_soa(s) | elem%tick | soa_x |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| example4.json | 1000 | 26 | 11 | 0.076 | 0.0015 | 0.0024 | **1.9 %** | 0.61× |
| example9_detrital_pathway.json | 2000 | 97 | 87 | 4.995 | 0.0110 | 0.0134 | **0.2 %** | 0.82× |

(Numbers vary run-to-run by a few percent; the orders of magnitude are stable. example9's ~5 s here
matches the ~3.5 s order-of-magnitude floor the issue cited — both say "one rollout is seconds,
dominated by agent count.")

**Reading the table:**
- The elementwise phases own **0.2 % of the example9 tick**. Even a *perfect, free, infinitely-fast*
  vectorisation of all three would shave 0.2 % off the rollout floor. By Amdahl's law the ceiling on
  this spike's lever is ~0.2 %.
- SoA is **slower** at these scales (0.6×–0.8×), dominated by the build/write-back conversion. The
  populations (tens of agents) are nowhere near where SIMD lane-parallelism amortises the column
  setup. The thrashing-thriving world never exceeds ~100 agents.
- The floor lives in the **other** phases. Profiling the remaining ~99.8 % is the natural next
  measurement, but it is structurally the spatial-query and reduction phases — the ones AC3 shows are
  hard to vectorise bit-identically.

## AC5 — Go/No-Go on the follow-up

**Recommendation: NO-GO on the SoA/SIMD inner-stepper rewrite as a single-rollout *latency* play.**

The spike was the cheap, safe experiment designed to either justify or retire the bigger rewrite from
*measured numbers*. The numbers retire it, for a simple reason:

- The latency floor is **not** in the phases that vectorise transparently. The elementwise phases —
  the only ones that are bit-identical for free — are ~0.2 % of a thriving tick. Vectorising them
  perfectly is invisible.
- The floor **is** in the phases that resist: the spatial-query/reduction phases. Making *those*
  bit-identical (AC3) is achievable only by pinning the SIMD kernel to the spatial index's iteration
  order — not transparent, and the workload is gather/scatter-bound (the unfavourable SIMD regime).
  And `resolve_reproduction` (the largest single serial phase, out of spike scope) is irreducibly
  serial with a greedy global matcher and RNG consumption.
- So buying the latency win means taking on exactly the second-source-of-truth tax Brief A warned
  about — for phases where even the bit-exact path is hard — to attack a floor whose cheap, safe
  fraction is 0.2 %.

**What this does *not* close off:**
- **The SoA store itself is sound and cheap to keep.** It round-trips bit-identically and could host
  future per-agent vectorised work or cache-friendlier layouts if a phase ever grows hot. It carries
  no semantic delta. Keeping it as a tested, additive module costs little.
- **If single-rollout latency must drop, the lever is algorithmic, not SIMD.** The right next
  measurement is a per-phase tick-cost breakdown of example9 (which phase owns the 99.8 %?). The
  prime suspects are the spatial queries with their `HashSet` dedup (`photosynthesise`,
  `resolve_drains`, `move`) — a better spatial index or dedup strategy would likely beat any SIMD
  rewrite of the elementwise phases, at no determinism cost. That is a cleaner follow-up than the SoA
  inner stepper.
- **Throughput** (many configs × seeds) is already owned by #353's rayon and is unaffected by this
  verdict.

In one line: **the elementwise SoA rewrite is bit-identical and correct, but the rollout floor isn't
there — so the inner-stepper SIMD rewrite is a no-go for latency, and the next move (if latency
matters) is to profile and attack the spatial-query phases algorithmically, not to add a second
source of truth.**
