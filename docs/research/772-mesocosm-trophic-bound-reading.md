# Issue #772: the mode-1 mesocosm under flow 7's bound, and #764's reading

**Status: measurement (2026-10-10). The mode-1 mesocosm (`explorers_search::mesocosm`) now runs
trophic transfer inside flow 7's domain bound: the recipe's `base_trophic_efficiency` (0.78) and
`trophic_distance_decay` 0.60 (`MESOCOSM_TROPHIC_DISTANCE_DECAY`), against the recipe's 2.38.
#764's pre-registered reading (#771) was run on it. Verdict: row 2 of the table.**

## TL;DR

- **Row 2 applies: decomposers persist, but almost all decomposer income goes to mixotrophs.**
  Agents the income read calls decomposers are alive at tick 3,000 in the control arm of all 5
  seeds (8–10 of them). Pooled over the 5 control arms, agents with autotrophy ≥ 0.1 took **0.986**
  of all carcass-drain energy over ticks 0–3,000. Over ticks 1,500–3,000 they took all of it (1.000
  in every seed and arm). The table's reading is *the residual inversion matters*, and the next
  step it names is to grill the trait-dependent kernel.
- **Caveat for the owner: the pure-vertex founders' line died out anyway.** Every pure-vertex drain
  happens in the first 25–50 ticks (supplementary runs below). By tick 50 no heterotroph-dominant
  agent is alive in any seed. In 4 of 5 seeds none is alive at tick 3,000 either (seed 2's control
  has 11). The persisting decomposers are agents that sit on the producer side of trait space and
  draw most of their drained income from carcasses. The founders died before those mixotroph
  drainers had taken more than a few percent of the litter, so the founders did not lose a
  competition to them. That is row 3's description of the founders ("they still die out"). The
  verdict is row 2 because the pre-registered operationalisation counts persistence by income role
  over all agents, not by founder line. It was fixed before the runs and is not changed here.
- **Efficiency check.** A founder at the recipe's heterotroph vertex sits at trait distance 1.52
  from producer litter, so it assimilates `0.78 · exp(−0.60 · 1.52) ≈ 0.31` of what it drains,
  against `0.78 · exp(−2.38 · 1.52) ≈ 0.021` before.

## Method (pre-registered, verbatim)

The operationalisation was fixed by the orchestrator before any run:

- **Run per seed:** `reference_mode --seed S --clear-at 1500 --ticks 1500` (release build, all
  other flags default = #770 wear defaults, committed recipe.json). Settle 1500 ticks, fork into
  control and perturbed arms, each run to absolute tick 3000. That is "paired, 3000 ticks". Run
  seeds 1–5 in parallel (background; each ~2 min or more — follow instrument-runtimes.md "Running
  long sweeps"; keep artifacts under target/reference-mode/772/).
- **Decomposers persist (per arm):** at least one living agent whose income role is Decomposer at
  tick 3000.
- **Heterotroph-dominant alive at 3000 (per arm):** count of living agents with traits heterotrophy
  > photosynthetic_absorption at tick 3000.
- **Decomposer income share by role (per arm):** sum of carcass-drain energy over the whole horizon
  (ticks 0–3000; for an arm, the settle's history plus that arm's post-fork drains) taken by agents,
  split by the draining agent's autotrophy (photosynthetic_absorption) < 0.1 ("pure-vertex side")
  versus ≥ 0.1 ("mixotroph"). Report both shares. Also report the same over ticks 1500–3000 as
  supplementary only.
- **Seed-level reading uses the control arm** (the unperturbed world). Report the perturbed arm
  too; if it would give a different row, note it, but the row is read from the control arm.
- **Row assignment:** Count seeds whose control arm has decomposers persisting. If ≥ 3 of 5
  persist: row 2 if, pooled over the persisting seeds, the mixotroph (autotrophy ≥ 0.1) share of
  decomposer drain income is > 0.5; otherwise row 1 if heterotroph-dominant agents are alive at
  tick 3000 in at least 3 of 5 seeds (in the same seeds that persist); if neither holds, AMBIGUOUS.
  If ≥ 3 of 5 die out: row 3. Anything else that does not fit cleanly: AMBIGUOUS — record it as
  ambiguous and stop.

The readouts (`Mode1Report::heterotroph_dominant`, `carcass_drain`, `arm_carcass_drain`) read the
event log: a `Consumed` event on a carcass carries the energy drained (before trophic efficiency),
and the observer books it to the drainer's nominal autotrophy, split at `MIXOTROPH_AUTOTROPHY =
0.1`. The split counts every carcass drain, whatever the drainer's role. Persistence reads
`final_roles.decomposers`, the income read's tally at the end.

## Results

Ticks 0–3,000 for each arm. "Drain" is carcass-drain energy, autotrophy < 0.1 / ≥ 0.1.
"Producers" is the whole world's income-role producers at tick 3,000, for context.

| seed | arm | decomposers persist | decomposers at 3000 | heterotroph-dominant at 3000 | drain 0–3000 | share 0–3000 (< 0.1 / ≥ 0.1) | drain 1500–3000 (supplementary) | producers at 3000 |
|---:|---|---|---:|---:|---:|---:|---:|---:|
| 1 | control | yes | 10 | 0 | 486.6 / 35,429.5 | 0.014 / 0.986 | 0.0 / 23,295.8 | 225 |
| 1 | perturbed | yes | 11 | 0 | 486.6 / 39,808.7 | 0.012 / 0.988 | 0.0 / 27,674.9 | 182 |
| 2 | control | yes | 10 | 11 | 468.7 / 32,107.8 | 0.014 / 0.986 | 0.0 / 24,548.2 | 202 |
| 2 | perturbed | yes | 8 | 0 | 468.7 / 31,615.2 | 0.015 / 0.985 | 0.0 / 24,055.6 | 195 |
| 3 | control | yes | 9 | 0 | 504.9 / 44,435.0 | 0.011 / 0.989 | 0.0 / 32,083.6 | 153 |
| 3 | perturbed | yes | 8 | 0 | 504.9 / 35,620.9 | 0.014 / 0.986 | 0.0 / 23,269.4 | 150 |
| 4 | control | yes | 9 | 0 | 542.6 / 24,323.5 | 0.022 / 0.978 | 0.0 / 18,393.5 | 216 |
| 4 | perturbed | yes | 6 | 0 | 542.6 / 21,746.5 | 0.024 / 0.976 | 0.0 / 15,816.4 | 181 |
| 5 | control | yes | 8 | 0 | 426.4 / 36,454.3 | 0.012 / 0.988 | 0.0 / 25,876.1 | 171 |
| 5 | perturbed | yes | 14 | 0 | 426.4 / 33,026.5 | 0.013 / 0.987 | 0.0 / 22,448.3 | 189 |

**Row assignment.**
1. Control arms with decomposers persisting: 5 of 5 (≥ 3), so rows 1 and 2 are in play.
2. Pooled over those 5, the mixotroph share of carcass-drain energy over ticks 0–3,000 is 0.986,
   which is > 0.5. So **row 2** applies.
3. The perturbed arms give the same row: all 5 persist, and every share is ≥ 0.976.

Heterotroph-dominant agents are alive at tick 3,000 in 1 of 5 control arms (seed 2) and in no
perturbed arm. So row 1 would not have applied even without the mixotroph share. Seed 2's 11
heterotroph-dominant agents drain nothing on the pure-vertex side after the fork, so their
autotrophy is ≥ 0.1.

## Supplementary: when the pure-vertex side stops draining

These runs are outside the pre-registration. Each is an unperturbed `reference_mode --seed S --ticks
T --sample-every T`, read at its end. The columns are heterotroph-dominant agents alive, income-role
decomposers alive, and cumulative carcass drain by autotrophy < 0.1 / ≥ 0.1.

| seed | tick 25 | tick 50 | tick 100 | tick 200 |
|---:|---|---|---|---|
| 1 | 1, 1, 485.5 / 33.0 | 0, 0, 486.6 / 41.3 | 0, 2, 486.6 / 81.0 | 0, 1, 486.6 / 180.0 |
| 2 | 0, 0, 468.7 / 1.2 | 0, 0, 468.7 / 1.7 | 0, 0, 468.7 / 68.9 | 0, 2, 468.7 / 180.3 |
| 3 | 0, 0, 504.9 / 0.0 | 0, 0, 504.9 / 19.4 | 0, 1, 504.9 / 41.7 | 0, 0, 504.9 / 154.7 |
| 4 | 0, 0, 542.6 / 1.9 | 0, 0, 542.6 / 7.0 | 0, 0, 542.6 / 16.0 | 0, 0, 542.6 / 28.0 |
| 5 | 1, 1, 425.4 / 13.1 | 0, 0, 426.4 / 45.3 | 0, 1, 426.4 / 96.5 | 0, 1, 426.4 / 221.4 |

- **At 0.31 efficiency the founders still last under 50 ticks.** The pure-vertex side's whole-run
  drain (426–543) is booked by tick 50 in every seed. #764 found the founders gone by tick 18 at
  0.021 efficiency (seed 1), so the efficiency gain buys them a few ticks at most.
- **The founders did not lose to mixotrophs.** By tick 25 the mixotroph side has drained 0–33
  energy, against 425–543 on the pure-vertex side. The mixotroph decomposers come later, from the
  producer line, and they carry the detrital pathway from about tick 100 on.
- So the founders' fate reads like row 3: efficiency was not what killed them. Reach, metabolism and
  wear are #764's other candidates. This is a reading of the supplementary runs, not the verdict,
  and the decay is not retuned here.

## Runtime

The five paired runs (4,500 ticks each: 1,500 settle plus 1,500 per arm) ran in parallel on 8
cores, release build. Wall-clock per run: seed 1 291 s, seed 2 267 s, seed 3 254 s, seed 4 268 s,
seed 5 304 s. The world now holds about 150–225 producers and 8–14 decomposers at tick 3,000.

## Reproducing

```sh
cargo build --release -p explorers-search --bin reference_mode
for s in 1 2 3 4 5; do
  target/release/reference_mode --seed $s --clear-at 1500 --ticks 1500 \
    --out target/reference-mode/772/s$s.json &
done; wait
# supplementary
for s in 1 2 3 4 5; do for t in 25 50 100 200; do
  target/release/reference_mode --seed $s --ticks $t --sample-every $t \
    --out target/reference-mode/772/early-s$s-t$t.json
done; done
```

Each `.md` summary beside its JSON carries the *Decomposers and carcass-drain income* table per arm.
