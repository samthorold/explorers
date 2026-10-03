# Issue #629: the intake-ceiling window for `k`

**Status: measurement. The window is open by the stated criterion but fails the outcome it was meant
to secure, so slice 2 (#630) does not start and the orchestrator returns to design. Adds an
intake-ceiling readout to `role_diet_census` (commit `6bcf913`). It is observer-only: every table
the tool printed before is unchanged, and old rows read back. `explorers-sim`, the search, the
evaluator and the prefilter are unchanged.**

#628 settled satiation as intake against a body-scaled ceiling `k × metabolic_cost`, using Holling's
disk equation ([world rules](../system-design/world-rules.md), *Capability and expression are
decoupled*). Its default `k` comes from a window read on the ungated atlas. The upper bound keeps
light-fed mixotrophs sated, `k ≤ 2 × p25(light / m)`. The lower bound keeps consumers producing,
`k ≥ p75(intake / m)` over heterotrophs by diet. The default is the window's geometric midpoint.
This slice reads that window, and it also reads two things the window is supposed to secure: the
killers' predicted expression and consumer production.

## TL;DR

**Verdict: the window is open by the stated criterion, decoded [1.05, 2.76] with default k = 1.70,
and at `founder_aggregation = 0` [1.36, 2.69] with 1.91. It fails by the outcome it was meant to
secure. Slice 2 (#630) does not start, and the orchestrator returns to design.**

| | decoded | `founder_aggregation = 0` |
|---|---|---|
| light-fed mixotrophs, light / m: p25 / median / p75 (n) | 1.380 / 2.020 / 3.080 (79,080) | 1.344 / 1.989 / 3.090 (197,114) |
| heterotrophs by diet, intake / m: p25 / median / p75 (n) | 0.096 / 0.299 / 1.052 (267) | 0.128 / 0.427 / 1.358 (787) |
| **energy window** | **1.052 ≤ k ≤ 2.759** | **1.358 ≤ k ≤ 2.687** |
| **default k** (geometric midpoint) | **1.704** | **1.910** |
| nutrient-side window | empty (upper ≤ 0, lower ≥ 18.7) | empty (upper ≤ 0, lower ≥ 52.9) |

1. **The window is open on the energy side,** which is the side the design states its criterion
   on. The nutrient-side window is empty at both settings, because most light-fed mixotrophs take up
   no free nutrient. It does not decide the verdict by itself (§3).
2. **At the default, the killers are sated only partly.** At k = 1.704, 58 % of kin-killing pairs
   (decoded) and 63 % (`founder_aggregation = 0`) have a predicted expression below 0.5. Ungated,
   every killer expresses 1. Raising `k` loses them fast: 41 % at k = 2.76, 21 % at 5 and 6 % at 10.
3. **At the default, consumers keep about 3 % of their production.** This is a static estimate from
   the ungated intake distribution (§5). Consumer production comes from the heavy tail of meals,
   worth several ticks of maintenance.
   The disk form also halves intake at P = room, not only above the ceiling. Keeping at least half
   of production needs k ≈ 8–10.
4. **One maintenance-scaled `k` cannot serve both populations.** Mixotroph killers read sated only at
   k ≲ 2, and consumers keep half their production only at k ≈ 10. This is the same population
   conflict as the surplus gate (#624 §6), in a new form (§6).

## 1. What ran

| | |
|---|---|
| Tree | `6bcf913` (this branch: the instrument on top of #628). Release builds. |
| Census | `role_diet_census` on the atlas (`atlas.json`, 95 cells), seeds 1000–1004, T = 2000, `EvalConfig::default()`, **ungated** (`--satiation-sensitivity 0`), both as decoded and at `--founder-aggregation 0`. |
| Killer passes | Decoded at `--intake-ceiling-k` ∈ {1.704, 2.76, 5, 10}. At `founder_aggregation = 0`, k = 1.704 only: the driver passed the *decoded* default to both passes, not that setting's own 1.910. |
| Machine | 8-core laptop, one process at a time, awake throughout. |

`--intake-ceiling-k` changes no physics. The world stays ungated, and the flag only bands the
killers by the expression they *would* have at that `k`. So the window tables of every k-pass are
identical, line for line, to the ungated pass at the same founder aggregation. That doubles as a
determinism check.

**Commands** (cwd is the repo root; driver `target/629/run.sh`):

```sh
cargo build --release -p explorers-search --bin role_diet_census
C=$(seq -s, -f 'atlas:%g' 0 94)

# FAFLAG in "" "--founder-aggregation 0" -> rd-fadecoded / rd-fa0
./target/release/role_diet_census --atlas atlas.json --configs $C $FAFLAG \
    --satiation-sensitivity 0 --out target/629/rd-fa<…>.jsonl
./target/release/role_diet_census --summary --out target/629/rd-fa<…>.jsonl

# killer passes: K = 1.704 (both settings, from run.sh); K in 2.76 5 10 (decoded, run by hand)
./target/release/role_diet_census --atlas atlas.json --configs $C $FAFLAG \
    --satiation-sensitivity 0 --intake-ceiling-k $K --out target/629/rd-fa<…>-k<…>.jsonl
./target/release/role_diet_census --summary --out target/629/rd-fa<…>-k<…>.jsonl
```

**Wall clock** (`target/629/times.log`):

| run | time |
|---|---:|
| `role_diet_census`, atlas, decoded (ungated, or at k = 1.704) | 40–41 s |
| `role_diet_census`, atlas, `founder_aggregation = 0` (ungated, or at k = 1.704) | 81–82 s |
| `--summary` | < 1 s |
| decoded passes at k = 2.76, 5, 10 | not logged (the same work as a decoded pass) |
| `cargo test --workspace` at `6bcf913` | 911 pass, 0 fail |

## 2. The energy window

The census reads at the intake gate's read point: once per agent at the start of drain resolution,
after photosynthesis and uptake, with what is in reach fixed. The readings are over second-half
agent-samples, in ticks of maintenance `m` (the drain-time metabolic cost). Light-fed mixotrophs are
income producers (#599) with heterotrophy above zero. Heterotrophs by diet are the deaths table's
diet group.

| population | reading | setting | n | p25 | median | p75 |
|---|---|---|---:|---:|---:|---:|
| light-fed mixotrophs | light / m | decoded | 79,080 | 1.380 | 2.020 | 3.080 |
| | | fa 0 | 197,114 | 1.344 | 1.989 | 3.090 |
| heterotrophs by diet | intake / m (light + drain income) | decoded | 267 | 0.096 | 0.299 | 1.052 |
| | | fa 0 | 787 | 0.128 | 0.427 | 1.358 |
| heterotrophs by diet | drain potential P_E / m | decoded | 267 | 0.000 | 0.042 | 0.265 |
| | | fa 0 | 787 | 0.000 | 0.067 | 0.598 |

- **Upper bound** `2 × p25(light / m)`: 2.759 decoded, 2.687 at `founder_aggregation = 0`.
- **Lower bound** `p75(intake / m)`: 1.052 and 1.358.
- **Default** `√(lower × upper)`: **1.704** and **1.910**.
- **Live configs only** (decoded, 87 configs): 1.002 ≤ k ≤ 2.741, default 1.658. Dead seeds do not
  move it.

The two populations are about a factor of 7 apart at their medians. Mixotrophs get about 2 ticks of
maintenance a tick from light. Diet-fed heterotrophs take in 0.3–0.4 ticks at the median, and more
than half of them have almost nothing in reach (P_E median 0.04–0.07). So the window exists because
consumers are *poor* at the median, not because they are sated.

## 3. The nutrient side

The census reads the nutrient side the same way. Uptake and intake go through `η·ratio`, so they
read in the same ticks-of-maintenance units as the energy side.

| population | reading | setting | p25 | median | p75 |
|---|---|---|---:|---:|---:|
| light-fed mixotrophs | uptake (matched) / m | decoded | 0.000 | 0.000 | 0.408 |
| | | fa 0 | 0.000 | 0.000 | 0.495 |
| heterotrophs by diet | nutrient intake (matched) / m | decoded | 2.665 | 7.445 | 18.701 |
| | | fa 0 | 4.686 | 13.696 | 52.880 |
| heterotrophs by diet | P_N (matched) / m | decoded | 0.000 | 5.872 | 15.622 |
| | | fa 0 | 0.000 | 7.726 | 29.348 |

**The nutrient window is empty at both settings.** The upper bound is 0, because more than half of
the light-fed mixotrophs take up no free nutrient on a given tick. The lower bound is 18.7 and 52.9.
This agrees with #622: in this run's own surplus table, 26–34 % of light-fed mixotrophs sit at zero
surplus, and 92–93 % of those have no free nutrient.

**Why this does not decide the verdict.** The design states the window criterion on the energy side.
The nutrient side enters expression only through `E = max(E_E, E_N)`. The hungrier currency sets
`E`, so a mixotroph with no uptake has its whole nutrient room open. It reads sated only if its
nutrient drain potential is also small against that room. For consumers, diet brings far more
nutrient than energy in matched units, so their energy side is the hungrier and governs. The killer
bands in §4 use the full `max` form, so they already include whatever the nutrient side contributes.
They do not separate killers held hungry by energy room from those held hungry by nutrient room;
that split is a cheap readout if design needs it.

## 4. Predicted kin-killer expression

These are the killing grazers' kill pairs on the atlas (all configs), banded by the grazer's
predicted expression at `k`, read at the start of drain resolution ("Killing grazers at the intake
gate (#629)"). `E < 0.5` (the sated side) is the two lowest bands. Ungated, every killer expresses
E = 1.

| k | setting | kin pairs | E < 0.1 | 0.1–0.5 | 0.5–0.9 | ≥ 0.9 | **kin E < 0.5** | **non-kin E < 0.5** |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 1.704 | decoded | 23,309 | 7,018 | 6,474 | 6,242 | 3,575 | **57.9 %** | **55.9 %** |
| 1.704 | fa 0 | 56,495 | 22,340 | 13,083 | 14,746 | 6,326 | **62.7 %** | **56.1 %** |
| 2.76 | decoded | 23,309 | 4,453 | 5,047 | 7,552 | 6,257 | 40.8 % | 40.6 % |
| 5 | decoded | 23,309 | 1,892 | 3,024 | 7,906 | 10,487 | 21.1 % | 24.4 % |
| 10 | decoded | 23,309 | 463 | 891 | 6,327 | 15,628 | 5.8 % | 9.2 % |

(Non-kin pairs: 24,017 decoded and 12,595 at fa 0.)

- **At the default, a little over half of the kin killers would sit on the sated side.** For
  comparison, the surplus gate at its default sated 68 % / 62 % of `sample:31`'s step killers
  (#624 condition 1), on a different population. This table is atlas-wide, not `sample:31`'s step
  lineages.
- **The share falls steeply with `k`.** It falls by about a third at the top of the window (2.76) and
  by a factor of 10 at k = 10.
- **Why the killers need a small `k`.** A killer's kin-kill potential is small, because a newborn is
  tiny. Under the disk form, `E = room / (room + P) < 0.5` needs `room < P`. So the killer reads
  sated only when light fills its ceiling to within one small `P` of the top, `k·m − light < P`. With
  light at about 2 m for a typical mixotroph, that means k ≲ 2. Each step up in `k` opens room that
  a small `P` cannot cover.

## 5. Consumer production kept (static estimate)

The lower bound assumes "the plateau rarely binds", and that this protects consumer production. The
census gives the ungated intake distribution for heterotrophs by diet, which supports a static
estimate of what a ceiling at `k` would leave them.

**The estimate.** Treat each sample's realised intake `x` (ticks of maintenance per tick) as the
potential of a lone feeder with no light. Gated intake is then `g(x) = x·k / (x + k)`, the disk
equation with room `k`. Production is the part above maintenance, `max(0, x − 1)` ungated and
`max(0, g(x) − 1)` gated. Each bin counts at its geometric midpoint.

| k | decoded: intake kept | decoded: **production kept** | fa 0: intake kept | fa 0: **production kept** |
|---:|---:|---:|---:|---:|
| 1.7 | 53.7 % | **3.1 %** | 41.5 % | **3.0 %** |
| 1.9 | 56.1 % | 5.2 % | 43.6 % | 5.1 % |
| 2.76 | 63.8 % | 15.7 % | 50.8 % | 14.3 % |
| 5 | 74.8 % | 37.2 % | 61.9 % | 31.5 % |
| 10 | 84.8 % | 60.4 % | 73.3 % | 51.3 % |
| 20 | 91.5 % | 77.2 % | 82.4 % | 67.7 % |

Ungated means: decoded intake 0.705 and production 0.243 over 267 samples; at fa 0, 1.096 and 0.580
over 787 samples. The share of intake that comes from samples above `k` is 40.8 % at k = 1.7 and
23.7 % at 2.76 decoded, and 65.7 % at k = 1.7 at fa 0.

- **The lower bound protects the wrong quantity.** p75 of intake is about 1 tick of maintenance,
  which is roughly break-even. A consumer's production comes from the minority of meals worth
  several ticks of maintenance (up to about 6 decoded and about 35 at fa 0). The p75 criterion says
  nothing about those meals. Only 10 % (decoded) and 19 % (fa 0) of the samples sit above the
  default, but they carry 41 % and 66 % of the intake.
- **The disk form bites below the ceiling too.** `g(x) = x/2` at `x = k`, so even meals at the ceiling
  lose half. A meal of 3 m under k = 1.7 yields 1.09 m, which is barely above maintenance.
- **Keeping at least half of production needs k ≈ 8–10.** That is 5× the window's top.

**Caveats.**
- **The estimate is static.** A gated consumer whose prey persists can recoup intake on later ticks,
  so the real loss is likely smaller. It is not likely 30× smaller.
- **Lone feeder.** Co-feeders are ignored.
- **Light is ignored.** Diet-fed heterotrophs have little light, but any light they have would cut
  their room further.
- **Small samples.** n is 267 decoded and 787 at fa 0.

To rerun (reads the census rows directly; it reproduces the table above):

```python
# python3 intake_kept.py target/629/rd-fadecoded.jsonl target/629/rd-fa0.jsonl
import json, sys

LOW, PER_DECADE, LOG_BINS = 1e-3, 20, 160  # SurplusDistribution: [below 1e-3, 160 log bins, >= 1e5]

for path in sys.argv[1:]:
    zero, bins = 0, [0] * (LOG_BINS + 2)
    for line in open(path):
        for seed in json.loads(line)["seeds"]:
            h = (seed.get("intake") or {}).get("heterotroph_intake")
            if h:
                zero += h["zero"]
                bins = [a + b for a, b in zip(bins, h["bins"] or [0] * len(bins))]
    xs = [(0.0, zero), (LOW / 2, bins[0])]
    xs += [(LOW * 10 ** ((i - 0.5) / PER_DECADE), bins[i]) for i in range(1, LOG_BINS + 1)]
    xs.append((LOW * 10 ** (LOG_BINS / PER_DECADE), bins[-1]))
    n = sum(c for _, c in xs)
    mean = lambda f: sum(f(x) * c for x, c in xs) / n
    intake, prod = mean(lambda x: x), mean(lambda x: max(0.0, x - 1))
    print(f"{path}: n {n}, mean intake {intake:.3f}, mean production {prod:.3f}")
    for k in (1.7, 1.9, 2.76, 5, 10, 20):
        g = lambda x: x * k / (x + k)  # lone feeder, no light: room = k
        print(f"  k {k:>5}: intake kept {100 * mean(g) / intake:5.1f} %,"
              f" production kept {100 * mean(lambda x: max(0.0, g(x) - 1)) / prod:5.1f} %,"
              f" intake from samples above k {100 * mean(lambda x: x * (x > k)) / intake:5.1f} %")
```

## 6. Verdict and what it implies

**The window is open by the criterion and fails by the outcome.** Per the issue, an open window
would set the default `k`. But the criterion was a proxy for two outcomes, and at the default it
secures neither well:

1. **Consumers: about 3 % of production kept at k = 1.70.** The lower bound read p75 of intake on the
   assumption that "the plateau rarely binds" protects production. It does not. Production lives in
   the heavy tail of meals, and the disk form halves intake already at P = room. Keeping half of
   production needs k ≈ 8–10.
2. **Killers: k must stay ≲ 2.** A mixotroph's kin-kill potential is small, so it reads sated only
   when light nearly fills its ceiling. At k = 10, 6 % of kin killers are on the sated side.

One maintenance-scaled ceiling cannot be about 2 for mixotrophs and about 10 for consumers. This is
the surplus gate's population conflict (#624 §6) in a new form. Slice 2 (#630) does not start, and
the `k` placeholder in the world rules stays open.

**Evidence for the design session.** This is evidence, not a decision. The ceiling has to scale
differently for the two acquisition routes. One candidate:

> ceiling = photosynthetic capacity + `k_h` × heterotrophic capability

- A mixotroph's ceiling would be mostly its own light capacity, which its light fills, so it reads
  sated.
- A consumer's ceiling would be about `k_h` targets' worth of drain, which is Holling handling. With
  one target in reach, `E = k_h / (k_h + 1)`, so a lone consumer does not throttle itself.

This candidate is not measured here. The readings this instrument already takes (`P`, light,
intake, per agent at the gate's read point) are what it needs to test it.

## 7. What this does not show

- **No physics change was run.** Every figure is from the ungated world. The killer bands and the
  production estimate predict what a gate *would* do. They do not show what a gated world evolves
  into: #624 showed the gated population moves, and more agents live as consumers there.
- **The killers are atlas-wide,** not `sample:31`'s step lineages, which #624's conditions 1–2 read.
- **The fa 0 killer pass used k = 1.704,** the decoded default, not that setting's own 1.910.
- **The killer bands do not split energy room from nutrient room** (§3).
- **The production estimate is static and lone-feeder** (§5 caveats), on 267 / 787 samples.
- **No LHS draw.** The verdict does not hinge on the atlas sample.

## 8. Instrument

Commit `6bcf913`, in `explorers-search`:

- **`intake_ceiling` (new module).** `intake_readings` reads every drain-time agent at the start of
  drain resolution from the `PreStep` replay of phases 1–4. `PreStep` now also keeps per-agent
  photosynthesis and uptake and the pre-step carcasses. The readings are:
  - **light and uptake**;
  - **realised drain income**, from the tick's `Consumed` events;
  - **drain potential `P`**, which mirrors the drain pass's reach and per-target demand at expression
    1 with no recognition restraint, each target capped at what it holds, before co-feeders split it.
    `P_E` is the structure taken × transfer efficiency. `P_N` is the nutrient bound in that
    structure.

  Ticks of maintenance divide by the drain-time metabolic cost. Nutrient is divided by `η·ratio`
  first. `IntakeReading::expression_at(k)` and `predicted_expression` give the disk-form `E`.
  `IntakeCensus` pools the distributions, using `SurplusDistribution`'s log bins, and the energy and
  nutrient `Window`s.
- **`role_diet_census`** records `IntakeCensus` on `SeedDiet`. The field is serde-defaulted, so old
  rows read back. The tool prints the quartile table and both window lines (or "window EMPTY"). With
  `--intake-ceiling-k K`, it bands the killing grazers by predicted expression at K (in
  `GrazerHunger`).
- **Tests:**
  - readings on hand-built agents: `a_light_only_agent_reads_its_photosynthesis_in_ticks_of_maintenance`,
    `a_drain_only_agent_reads_its_drain_income_and_matching_potential`,
    `a_light_and_drain_agent_counts_every_route`,
    `a_nutrient_limited_agent_is_kept_hungry_by_its_nutrient_room`,
    `without_growth_efficiency_the_nutrient_side_is_absent`,
    `realised_drains_are_bounded_by_and_mostly_equal_the_potential`;
  - the window: `the_window_reads_both_populations`,
    `the_window_line_reports_bounds_default_or_empty`, `an_empty_window_proposes_no_default`;
  - the disk equation: `expression_is_the_disk_equation_on_the_energy_side`,
    `a_full_room_expresses_nothing_and_an_empty_reach_expresses_fully`,
    `the_hungrier_nutrient_side_sets_expression`;
  - the census: `rollout_records_intake_by_population`,
    `rollout_bands_killing_grazers_by_predicted_intake_expression`,
    `an_intake_ceiling_k_reaches_the_kills_and_the_row`.

  The workspace has 911 tests, all passing.
- **Artifacts** are in `target/629/`:
  - `rd-fa{decoded,0}{,-k}.{jsonl,txt}` and `-summary.txt`;
  - `rd-fadecoded-k{2.76,5,10}.*`;
  - `hint-fa{decoded,0}.json`, the pooled heterotroph-by-diet intake histograms, identical to
    pooling the `.jsonl` rows;
  - `run.sh` and `times.log`.
