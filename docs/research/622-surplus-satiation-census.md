# Issue #622: surplus satiation by role on the atlas

**Status: measurement; proposes `satiation_sensitivity = 33` (set in #623). Adds a surplus-satiation readout to `role_diet_census`
(`explorers_search::grazer_hunger::Surplus`, `SurplusDistribution`;
`explorers_search::role_diet::SurplusByRole`). No trajectory changes: with the new `surplus` field
removed, every atlas row is byte-identical to `main`'s (`f02b023`) at both founder-aggregation
settings. `explorers-sim`, the search, the evaluator and the prefilter are unchanged.**

#621 settled satiation as **surplus above the retention buffer**, co-limited by matching free
nutrient, read once per consumer after metabolism and before growth (world rules, *Capability and
expression are decoupled*):

`s = max(0, min(reserve − buffer, free_nutrient / (growth_efficiency × ratio))) / metabolic_cost`,
with `buffer = growth_retention_multiplier × metabolic_cost`.

The design (as first written) set the default `satiation_sensitivity` to `c = 1 / s₂₅`, where `s₂₅`
is the 25th percentile of the surplus of **light-fed mixotrophs** on the atlas: income-role producers
(light ≥ half of recent income, #599) with heterotrophy above zero. This note measures that
distribution. The criterion as written gives `s₂₅ = 0`, so it was amended (§4) to read the 25th
percentile over light-fed mixotrophs **with positive surplus**.

## TL;DR

1. **Proposed default: `satiation_sensitivity = 33`** (`1 / s₂₅⁺`, `s₂₅⁺ = 0.030` ticks over the
   light-fed mixotrophs with `s > 0`, decoded atlas; exact read 33.03). The cross-check at
   `founder_aggregation = 0` gives `s₂₅⁺ = 0.074`, `c = 13.4`. Live configs only: 38.2 decoded, 13.8
   at `founder_aggregation = 0`.

   | light-fed mixotrophs | samples | at s = 0 | s₂₅ | median | s₇₅ | s₂₅ (s > 0) | 1 / s₂₅ (s > 0) |
   |---|---:|---:|---:|---:|---:|---:|---:|
   | atlas, decoded | 84,403 | 29.0 % | 0 | 0.049 | 0.48 | 0.030 | **33.0** |
   | atlas, `founder_aggregation = 0` | 206,933 | 26.5 % | 0 | 0.134 | 0.94 | 0.074 | 13.4 |

2. **Why the criterion reads only positive surplus.** Over all light-fed mixotrophs `s₂₅ = 0`:
   29.0 % sit at `s = 0` (26.5 % at `founder_aggregation = 0`), so `c = 1 / s₂₅` has no finite
   value. About 90 % of those zeros hold no free nutrient before growth. Their energy above the
   buffer is large: on the energy side alone only 2.7 % are at or below the buffer. They are
   nutrient-short producers, which the design deliberately leaves at full capability at any `c` (the
   carnivorous-plant case). They cannot be sated, so they cannot set the scale at which sated
   agents half-express.
3. **Light-fed mixotrophs read *less* sated than heterotrophs, not more.** Their median surplus is
   0.05–0.13 ticks. Consumers by income have a median of 0.8, and decomposers by income 0 (decoded)
   or 1.0 (`founder_aggregation = 0`). At `c = 33` the median consumer expresses about
   `1 / (1 + 33 × 0.86) ≈ 0.03`. Only heterotrophs at `s = 0` (between meals, 26–57 % of their
   samples) feed near full capability. Slice 2 should measure what that does to heterotroph
   income.
4. **Founder aggregation moves `c` by about 2.5×** (33 → 13): the mixotrophs' positive tail sits
   about twice as high at `founder_aggregation = 0`. The zero share and the heterotroph rows barely
   move.

## 1. What ran

- **Census:** `role_diet_census` on the atlas (`atlas.json`, 95 cells), seeds 1000–1004, T = 2000,
  `EvalConfig::default()`. Once with worlds as decoded and once with `--founder-aggregation 0` (the
  #605/#606 convention).
- **Tree:** `main` at `f02b023` plus this branch's observer-only readout.
- **Machine:** 8-core laptop, release build.

```sh
cargo build --release -p explorers-search --bin role_diet_census
C=$(seq -s, -f 'atlas:%g' 0 94)
./target/release/role_diet_census --atlas atlas.json --configs "$C" --out target/622/atlas.jsonl
./target/release/role_diet_census --atlas atlas.json --configs "$C" --founder-aggregation 0 --out target/622/atlas-fa0.jsonl
./target/release/role_diet_census --summary --out target/622/atlas.jsonl
```

**Wall clock:**

| run | `main` (no readout) | with the readout | final re-run |
|---|---:|---:|---:|
| atlas, decoded | 26.5 s | 26.3 s | 40.9 s |
| atlas, `founder_aggregation = 0` | 59.0 s | 58.2 s | 94.4 s |

The readout costs nothing measurable. It replays three phases only on the evaluator's second-half
sample ticks. The final re-run, which added only the positive-surplus percentile to the summary,
was slower because the laptop was under other load. Its rows are identical.

**Reproduction check.** The same commands ran on `main` (`f02b023`). The branch's rows with
`.seeds[].surplus` deleted (`jq -c 'del(.seeds[].surplus)'`) are byte-identical to `main`'s, for
all 95 rows at both settings.

## 2. The readout

- **`PreStep::metabolised_agents`** replays the tick's first three phases (photosynthesise, absorb
  nutrients, metabolise) on a copy of the pre-step state. `drain_time_agents`, the #606 drain-pass
  read, is now that roster grown. A test pins `grow(metabolised) == drain_time` bit for bit, and
  the #606 drain replay test still reproduces the stepper's living drains bit for bit.
- **`Surplus::of`** computes both sides independently of the stepper, which does not read surplus
  yet. The energy side is `(reserve − buffer) / m`, negative below the buffer. The nutrient side is
  `N / (η · ratio) / m`, with no buffer. `ticks()` is `max(0, min(energy, nutrient))`. Tests use
  hand-built agents above, at and below the buffer. They also cover a nutrient-limited agent far
  above its buffer, which reads its nutrient ticks unshifted, and one with no free nutrient, which
  reads 0.
- **`SurplusDistribution`** counts exact zeros apart, splits them into "no free nutrient" and "at
  or below the buffer", and counts nutrient-limited samples. It bins the rest at 20 log bins per
  decade from 10⁻³ to 10⁵ ticks. Percentiles interpolate within a bin, which is accurate to about
  12 %.
- **What is sampled:** on each of the evaluator's second-half role-snapshot ticks, every agent
  alive at the sample is read from the metabolised replay of the step that led to it. Agents born
  in that step have no pre-growth state and are left out. The role is the evaluator's
  recent-income role at the sample (#599), not the census's lifetime diet. A light-fed mixotroph
  is an income producer with `traits.heterotrophy > 0`, which is effective heterotrophy above
  zero, since wear only scales it.
- **Rows:** `SeedDiet.surplus` (`#[serde(default)]`) holds the four role distributions (producer,
  consumer, decomposer, no role), the light-fed mixotrophs, and their energy side alone. Old rows
  read back with an empty readout.

## 3. Distributions

Atlas, all configs (95 configs, 475 seeds). `s` is in ticks of maintenance. "No free nutrient" is
the share of the zeros held there by nutrient rather than by the buffer.

**Decoded:**

| agents | samples | at s = 0 | of which no free nutrient | nutrient-limited | s₂₅ | median | s₇₅ |
|---|---:|---:|---:|---:|---:|---:|---:|
| light-fed mixotrophs | 84,403 | 29.0 % | 90.7 % | 44.6 % | 0 | 0.049 | 0.480 |
| … energy side alone | 84,403 | 2.7 % | — | — | 0.077 | 2.604 | 18.911 |
| producers by income | 215,208 | 27.6 % | 88.3 % | 34.1 % | 0 | 0.221 | 2.256 |
| consumers by income | 252 | 25.8 % | 9.2 % | 17.1 % | 0 | 0.862 | 3.758 |
| decomposers by income | 1,374 | 56.7 % | 1.4 % | 5.2 % | 0 | 0 | 2.051 |
| heterotrophs by income (both) | 1,626 | 51.9 % | 2.0 % | 7.0 % | 0 | 0 | 2.216 |

**`founder_aggregation = 0`:**

| agents | samples | at s = 0 | of which no free nutrient | nutrient-limited | s₂₅ | median | s₇₅ |
|---|---:|---:|---:|---:|---:|---:|---:|
| light-fed mixotrophs | 206,933 | 26.5 % | 90.4 % | 37.6 % | 0 | 0.134 | 0.938 |
| … energy side alone | 206,933 | 2.5 % | — | — | 0.168 | 1.959 | 12.558 |
| producers by income | 480,469 | 28.9 % | 89.7 % | 34.3 % | 0 | 0.204 | 1.843 |
| consumers by income | 587 | 24.9 % | 6.2 % | 12.6 % | 0.001 | 0.789 | 3.245 |
| decomposers by income | 2,294 | 36.3 % | 4.6 % | 9.3 % | 0 | 1.011 | 6.630 |
| heterotrophs by income (both) | 2,881 | 33.9 % | 4.8 % | 10.0 % | 0 | 0.950 | 5.938 |

Live configs only (87 decoded, 89 at `founder_aggregation = 0`) agree to within two points. For
example, light-fed mixotrophs decoded: 78,312 samples, 27.6 % at zero, median 0.048, `s₇₅` 0.445.
The full tables are in `target/622/atlas*.md`.

**Finer percentiles of the light-fed mixotrophs** (from the JSON rows):

| | s₂₅ | s₃₀ | s₃₅ | s₄₀ | s₅₀ | s₇₅ |
|---|---:|---:|---:|---:|---:|---:|
| decoded | 0 | 0 | 0.001 | 0.006 | 0.049 | 0.48 |
| … energy side alone | 0.077 | 0.19 | 0.46 | 1.0 | 2.6 | 18.9 |
| `founder_aggregation = 0` | 0 | 0.001 | 0.008 | 0.031 | 0.134 | 0.94 |
| … energy side alone | 0.168 | 0.33 | 0.58 | 0.91 | 1.96 | 12.6 |

The zeros are spread across the atlas, not confined to a few cells. All 95 configs carry light-fed
mixotroph samples. 29 configs have more than 40 % of them at zero, and 5 have more than 90 %.

**Heterotrophs by income are scarce.** There are 1.6–2.9k samples against 85–207k mixotroph
samples, mostly decomposers, so their percentiles are noisy. Half the decoded decomposer samples
sit at zero, and almost all of those are at or below the buffer. That is the "between meals"
reading the design predicts.

## 4. The default: `c = 1 / s₂₅` over light-fed mixotrophs with positive surplus

The design picks `c = 1 / s₂₅` so that three quarters of the light-fed mixotrophs sit at or past
half expression, where recognition spares identical kin completely. Read over all of them, that is
unattainable:

- **No finite `c` achieves it.** 26–29 % of light-fed mixotrophs have `s = 0` and express full
  capability at any `c`, so at most 71–74 % can ever reach half expression.
- **The zeros are nutrient-hungry producers.** They hold energy above the buffer but no free
  nutrient before growth. The design means them to feed at full capability: "a producer saturated
  with light but short of nutrient has a real reason to drain". *Inference:* they are
  nutrient-limited growers. Each tick's growth binds their free nutrient into structure, so the
  store they carry into the next grow phase is near zero whenever that tick's uptake was small.

**Chosen criterion (the owner's call, option 1 of three):** `c = 1 / s₂₅⁺`, the 25th percentile over
light-fed mixotrophs with `s > 0`. These are the mixotrophs the gate can sate, and three quarters of
them sit at or past half expression at that `c`. The census prints it as "Proposed default
satiation_sensitivity" (`SurplusByRole::proposed_sensitivity`).

| | s₂₅⁺ | c = 1 / s₂₅⁺ |
|---|---:|---:|
| decoded atlas, all configs (**the default**) | 0.0303 | **33.0** |
| decoded atlas, live configs | 0.0262 | 38.2 |
| `founder_aggregation = 0`, all configs (cross-check) | 0.0744 | 13.4 |
| `founder_aggregation = 0`, live configs | 0.0723 | 13.8 |

**Proposed default `satiation_sensitivity = 33`.** It is read on the decoded atlas because that is
the world as the search runs it. The `founder_aggregation = 0` value is 2.5× lower. The percentile
read is accurate to about 12 % (one log bin), so 33 is as precise as the figure supports.

The other options were not taken:
- Anchor on the mixotroph median (`c ≈ 7.5–20`).
- Revisit the nutrient side's yardstick, for example count nutrient against this tick's uptake
  rather than the stored free nutrient, which growth empties.

**What this costs heterotrophs** (for #623 to measure, not settled here):
- **A consumer at its median surplus (0.86 ticks) expresses about 3 % of capability at `c = 33`.**
  The positive surplus on this yardstick is small for every role, so a `c` set where mixotrophs
  half-express gates any heterotroph holding a surplus hard.
- **Feeding is left to heterotrophs between meals.** Those at `s = 0` (26–57 % of their samples)
  still feed at full capability.
- **The energy side alone would separate the roles better.** Its median is 2.0–2.6 ticks for
  mixotrophs, against a co-limited 0.8–1.0 for heterotrophs. But the design deliberately
  co-limits, and an energy-only gate would silence the carcass recycling the nutrient side exists
  to keep open.

## 5. What this does not show

- **The LHS draw was not run** (it takes 1.5–3 h). The atlas is the population the design names.
- **Surplus is read under the current physics.** The stepper still gates on the old
  reserve-in-ticks satiation at the decoded `c`. Once slice 2 switches the gate, the distribution
  will move, because consumption, and so the nutrient a mixotroph ingests, changes with it.
- **The role is recent income (half-life 50 ticks).** A mixotroph whose light share sits near ½ can
  flicker between roles from sample to sample.
