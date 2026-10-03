# Issue #637: the `(k, τ)` region for the fullness gate

**Status: measurement. The region is empty at both settings and at every exponent, so slice 2
(#630) does not start and the orchestrator returns to design (grill-with-docs). Adds a fullness
readout and a `(k, τ)` region evaluator to `role_diet_census` (commit `ad2e8dc`). It is
observer-only and opt-in (`--fullness`): every table the tool printed before is unchanged, and old
rows read back. `explorers-sim`, the search, the evaluator and the prefilter are unchanged.**

#634 found that no per-tick ceiling separates the mixotroph kin killers from the consumers, and
that recent average intake might. #636 settled satiation as **fullness**: a moving average of
intake with clearance time `τ`, read against a ceiling `C = k × m`, with
`E = max(E_E, E_N)` and `E_x = 1 / (1 + (F_x / C_x)^4)`
([world rules](../system-design/world-rules.md), *Capability and expression are decoupled*). #638
started founders at their first tick's light and uptake. The defaults for `(k, τ)` are bounded by
outcome:

- **sated mixotrophs:** at least 60 % of the kin-killing pairs whose killer is a light-fed
  mixotroph are predicted below half expression (`E < 0.5`) at the kill;
- **producing consumers:** heterotrophs by diet keep at least 75 % of their ungated production.

This slice reads both outcomes over a log grid of `(k, τ)` on the ungated atlas, by replaying each
agent's fullness over its own intake series.

## TL;DR

**Verdict: the region is empty at n = 4, both as decoded and at `founder_aggregation = 0`, and at
n = 2 and n = 8 too. No `(k, τ)` on the grid meets both bars. Slice 2 (#630) does not start, and
the orchestrator returns to design.**

| n = 4 | decoded | `founder_aggregation = 0` |
|---|---|---|
| feasible cells | **0 of 204** | **0 of 204** |
| closest cell `(k, τ)` | 1.7, 17.2 | 2.236, 2.04 |
| sated / kept there (bars 60 % / 75 %) | 58.3 % / 73.4 % | 52.6 % / 66.7 % |
| sated on the energy side alone there | 94.0 % | 83.7 % |
| margin `min(sated − 0.60, kept − 0.75)` | −0.017 | −0.083 |
| n: mixotroph kin kills / consumer ticks | 13,045 / 2,584 | 33,983 / 7,988 |

1. **Fullness is a large step from #634, and decoded it is a near miss** (§2). The closest margin
   goes from −0.344 to −0.017 decoded, and from −0.369 to −0.083 at `founder_aggregation = 0`.
2. **The energy side alone does the job; the nutrient side keeps the killers hungry** (§3). At the
   closest decoded cell, 94 % of kills are sated on energy but only 58 % once `E = max(E_E, E_N)`
   lets the emptier nutrient side through. Reading the sated bar on the energy side, with kept
   still read under the co-limited gate, the region is **non-empty** at both settings and every n.
3. **Per-agent average intake does separate the populations** (§4), as #634 suggested. The median
   killer averages 3.0 / 3.4 m over its life, and the median consumer 0.28 / 0.38 m. But the
   killers' lower tail (p10 0.30 / 0.43 m) overlaps the consumers, and a ceiling that leaves
   consumers room cannot sate it.
4. **The exponent barely matters** (§2). Sated does not depend on n at all, and kept moves by a
   point or two between n = 2, 4 and 8. The fixed n = 4 is not what empties the region.
5. **What the data points to is a design question, not a parameter** (§6): whether a killer sated
   on energy but hungry for nutrient counts as a failure of the gate. The world rules already say it
   does not, but the bar counts it as one.

## 1. What ran

| | |
|---|---|
| Tree | `ad2e8dc` (this branch: the instrument on top of `131b848`). Release builds. |
| Census | `role_diet_census` on the atlas (`atlas.json`, 95 cells), seeds 1000–1004, T = 2000, `EvalConfig::default()`, **ungated** (`--satiation-sensitivity 0`), with `--fullness`, both as decoded and at `--founder-aggregation 0`. |
| Region | `role_diet_census --summary --region --fullness-region` on each run's rows: `k` on a log grid over [0.25, 20] (17 points), `τ` on a log grid over [1, 50] ticks (12 points), at n ∈ {2, 4, 8}. 204 cells per n. |
| Energy-side region | `energy_region.py` on the same rows (§3, script below). |
| Machine | 8-core laptop, one process at a time, awake throughout. |

**Commands** (cwd is the repo root; driver `target/637/run.sh`):

```sh
cargo build --release -p explorers-search --bin role_diet_census
C=$(seq -s, -f 'atlas:%g' 0 94)

# FAFLAG in "" "--founder-aggregation 0" -> rd-fadecoded / rd-fa0
./target/release/role_diet_census --atlas atlas.json --configs $C $FAFLAG \
    --satiation-sensitivity 0 --fullness --out target/637/rd-fa<…>.jsonl
./target/release/role_diet_census --summary --region --fullness-region \
    --out target/637/rd-fa<…>.jsonl
```

**Wall clock** (`target/637/times.log`):

| run | time |
|---|---:|
| `role_diet_census --fullness`, atlas, decoded | 28 s |
| `role_diet_census --fullness`, atlas, `founder_aggregation = 0` | 56 s |
| `--summary --region --fullness-region` (both) | < 1 s |
| `cargo test --workspace` at `ad2e8dc` | 936 pass, 0 fail |

Outputs are 6.06 MB decoded and 9.83 MB at `founder_aggregation = 0` (#634's were 3.8 MB and
7.2 MB).

**What the readout does.** Each agent carries a fullness bank: one `F_E` and one `F_N` for each of
the 12 values of `τ`. Each tick after drains, every bank is updated from that tick's intake: light
plus drain energy received, and uptake plus drain nutrient retained. Founders start at their first
tick's light and uptake, and a newborn copies its parent's bank, as the world rules state. The gate
is read from the fullness carried into the tick.

- **Sated** is read on each kin-killing pair whose killer is a light-fed mixotroph, at the kill
  (`E < 0.5`). It is stored per `(k, τ)` only, because `1 / (1 + r^n) < 1/2` exactly when `r > 1`,
  whatever n is. It is also stored on the energy side alone (`E_E < 0.5`).
- **Kept** is read for heterotrophs by diet on every second-half tick:
  `Σ max(0, x_g − 1) / Σ max(0, x − 1)`, where `x = (light + drain energy) / m` and `x_g` is the
  same with the drain energy scaled by that tick's `E`.

## 2. The region

Cells are sated / kept, in %. The grids keep every other `τ` and most `k`, including the rows and
columns of the closest cells. The full maps are in `target/637/rd-fa*-region.txt`. No cell anywhere
on the full grid is feasible, at any n.

**Decoded, n = 4** (13,045 kin kills, 2,584 consumer ticks):

| k \ τ | 1 | 2.037 | 4.148 | 8.447 | 17.203 | 35.036 | 50 |
|---|---:|---:|---:|---:|---:|---:|---:|
| 0.25 | 90/70 | 93/70 | 94/70 | 95/70 | 95/70 | 96/70 | 96/70 |
| 0.432 | 83/70 | 87/70 | 89/70 | 90/70 | 91/70 | 92/70 | 92/70 |
| 0.748 | 71/71 | 75/71 | 78/71 | 80/71 | 82/70 | 83/70 | 83/70 |
| 1.293 | 54/74 | 59/73 | 63/73 | 65/72 | 67/72 | 68/71 | 69/71 |
| 1.7 | 46/77 | 51/76 | 54/75 | 56/74 | **58/73** | 60/73 | 60/73 |
| 2.236 | 38/84 | 42/81 | 45/79 | 47/78 | 49/77 | 51/75 | 51/75 |
| 2.941 | 29/89 | 33/85 | 36/84 | 38/82 | 40/81 | 41/78 | 41/77 |
| 3.867 | 22/93 | 25/89 | 28/87 | 30/87 | 32/86 | 33/82 | 33/81 |
| 6.687 | 9/97 | 11/95 | 13/94 | 14/94 | 15/94 | 15/92 | 15/90 |
| 11.565 | 3/99 | 3/98 | 4/98 | 5/98 | 5/98 | 6/97 | 7/97 |
| 20 | 1/100 | 1/99 | 2/99 | 2/99 | 3/99 | 3/99 | 4/99 |

**`founder_aggregation = 0`, n = 4** (33,983 kin kills, 7,988 consumer ticks):

| k \ τ | 1 | 2.037 | 4.148 | 8.447 | 17.203 | 35.036 | 50 |
|---|---:|---:|---:|---:|---:|---:|---:|
| 0.25 | 92/58 | 94/58 | 95/58 | 95/57 | 96/57 | 97/57 | 97/57 |
| 0.432 | 89/58 | 91/58 | 92/58 | 92/58 | 93/57 | 94/57 | 94/57 |
| 0.748 | 81/58 | 84/58 | 85/58 | 86/58 | 87/58 | 88/58 | 89/58 |
| 1.293 | 67/60 | 71/59 | 73/59 | 75/59 | 77/58 | 78/58 | 79/58 |
| 1.7 | 59/63 | 63/61 | 66/61 | 68/61 | 70/60 | 71/59 | 71/59 |
| 2.236 | 49/69 | **53/67** | 56/66 | 58/65 | 60/64 | 62/62 | 62/61 |
| 2.941 | 38/78 | 42/74 | 45/73 | 48/71 | 50/69 | 51/67 | 52/65 |
| 3.867 | 28/86 | 32/82 | 35/80 | 37/78 | 39/75 | 40/72 | 40/70 |
| 6.687 | 13/95 | 15/91 | 16/89 | 18/87 | 19/85 | 19/82 | 19/80 |
| 11.565 | 5/97 | 6/94 | 7/93 | 7/92 | 8/90 | 8/88 | 8/86 |
| 20 | 2/99 | 2/97 | 3/96 | 3/95 | 4/94 | 4/92 | 4/91 |

(Bold: the closest cell, the largest margin `min(sated − 0.60, kept − 0.75)`.)

**The closest cell at each exponent:**

| n | setting | closest `(k, τ)` | sated | energy side alone | kept | margin |
|---:|---|---|---:|---:|---:|---:|
| 2 | decoded | 1.7, 24.55 | 59.3 % | 94.2 % | 74.4 % | −0.007 |
| 4 | | 1.7, 17.20 | 58.3 % | 94.0 % | 73.4 % | −0.017 |
| 8 | | 1.7, 17.20 | 58.3 % | 94.0 % | 72.9 % | −0.021 |
| 2 | fa 0 | 2.236, 2.91 | 54.2 % | 84.7 % | 68.4 % | −0.066 |
| 4 | | 2.236, 2.04 | 52.6 % | 83.7 % | 66.7 % | −0.083 |
| 8 | | 2.941, 35.04 | 51.2 % | 77.4 % | 66.3 % | −0.088 |

**Reading the map.**

- **`k` trades one outcome for the other; `τ` hardly moves either.** Down the decoded `τ = 17.2`
  column, sated / kept go from 67/72 at k = 1.29 to 58/73 at 1.7 and 49/77 at 2.24. Across a row,
  a longer `τ` sates a few more killers (steady light keeps a long average full) and costs consumers
  a few points (a feast sates for longer). The ridge of best cells is nearly flat in `τ`, which is
  why the closest cell at `founder_aggregation = 0` jumps from `τ` ≈ 2 at n = 4 to 35 at n = 8.
- **The exponent is not the binding choice.** Sated is n-free by construction. Kept differs by a
  point or two between exponents near the ridge. n = 2 is marginally closest, the opposite of what
  the world rules' argument for a steep exponent expects, but the difference is within a point.
- **Kept has a floor.** It never drops below about 70 % decoded or 57 % at
  `founder_aggregation = 0`, even at k = 0.25. `x_g` gates only the drain energy, so a consumer's
  own light is never gated, and a consumer with an empty average still takes a feast whole. The
  readout does not split the two. So the 75 % bar asks less of the drain-borne production than its
  number suggests: at the closest decoded cell, kept sits only 3 points above its floor.
- **Founder aggregation shifts the ridge, as in #634.** At `founder_aggregation = 0`, the floor is
  lower and the killers are harder to sate at a given `k`, so the margin is about five times worse.

## 3. The energy side and the nutrient side

**Sated on the energy side alone** (`E_E < 0.5`, % of kills; n-free):

| k \ τ | decoded: 1 | 4.148 | 17.203 | 50 | fa 0: 1 | 4.148 | 17.203 | 50 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 0.25 | 98 | 100 | 100 | 100 | 99 | 100 | 100 | 100 |
| 0.748 | 95 | 100 | 100 | 100 | 97 | 100 | 100 | 100 |
| 1.293 | 92 | 97 | 99 | 99 | 94 | 98 | 99 | 99 |
| 1.7 | 86 | 91 | 94 | 94 | 89 | 93 | 95 | 95 |
| 2.236 | 76 | 81 | 84 | 85 | 81 | 85 | 87 | 88 |
| 2.941 | 64 | 69 | 73 | 73 | 70 | 75 | 77 | 77 |
| 3.867 | 51 | 56 | 60 | 60 | 58 | 62 | 64 | 65 |
| 6.687 | 29 | 33 | 35 | 34 | 33 | 36 | 38 | 37 |
| 20 | 4 | 5 | 8 | 8 | 5 | 6 | 8 | 9 |

- **The nutrient side is what keeps the killers hungry.** Compare the maps cell by cell. At the
  closest decoded cell, 94 % of kills are sated on energy and 58 % under `E = max(E_E, E_N)`. Over a
  third of the kin kills (36 % decoded, 31 % at fa 0) are made by a killer that is full of light but whose nutrient fullness is
  below its nutrient ceiling. The gap is there at every `k`: even at k = 0.25, co-limited sated is
  90–97 % against energy-side 98–100 %. This is #629's nutrient-side finding in the fullness form:
  most light-fed mixotrophs take up little free nutrient, so their `F_N` sits low against
  `C × η × ratio`.
- **On the energy side, the region is non-empty.** Reading the sated bar as `E_E < 0.5`, and kept
  exactly as before (under the co-limited gate, which is the drain a consumer would actually take):

| n | setting | feasible cells | best `(k, τ)` | energy-sated | co-limited sated | kept | margin |
|---:|---|---:|---|---:|---:|---:|---:|
| 2 | decoded | 34 of 204 | 2.941, 2.04 | 67.3 % | 33.5 % | 82.8 % | +0.073 |
| 4 | | 27 of 204 | 2.941, 4.15 | 69.5 % | 36.3 % | 83.8 % | +0.088 |
| 8 | | 27 of 204 | 2.236, 1.00 | 75.6 % | 37.8 % | 86.1 % | +0.111 |
| 2 | fa 0 | 5 of 204 | 3.867, 4.15 | 61.8 % | 34.9 % | 76.9 % | +0.018 |
| 4 | | 8 of 204 | 3.867, 8.45 | 63.3 % | 37.3 % | 78.0 % | +0.030 |
| 8 | | 9 of 204 | 3.867, 12.05 | 63.9 % | 38.3 % | 79.2 % | +0.039 |

  At n = 4 the feasible cells are k = 1.7 (τ ≤ 2.9), 2.236 (τ ≤ 24.6), 2.941 (every τ) and
  3.867 (τ = 35) decoded, and k = 2.941 (τ ≤ 1.43) and 3.867 (2.9 ≤ τ ≤ 17.2) at
  `founder_aggregation = 0`. The two settings share only k = 2.941 at τ = 1 and 1.43.
- **But those cells leave most kin kills unprevented.** At each best cell above, only 34–38 % of
  kin kills fall below half expression once the nutrient side is let through. The energy reading is
  feasible because it reclassifies the nutrient-hungry killers as acceptable, not because the gate
  stops more kills.

To rerun (pools the stored grids; it reproduces the closest cells above exactly, which checks the
indexing):

```python
# python3 energy_region.py target/637/rd-fa{decoded,0}.jsonl   (cwd: repo root)
import json, sys

def load(path):
    K = T = None; kills = ticks = 0; prod = 0.0; S = SE = PG = None
    for line in open(path):
        for s in json.loads(line).get("seeds", []):
            f = s.get("fullness")
            if not f: continue
            K, T = f["k"], f["tau"]
            kills += f["kills"]; ticks += f["consumer_ticks"]; prod += f["produced"]
            if S is None:
                S = [0] * len(f["sated"]); SE = [0] * len(f["sated"])
                PG = [0.0] * len(f["produced_gated"])
            for i, v in enumerate(f["sated"]): S[i] += v
            for i, v in enumerate(f["sated_energy"]): SE[i] += v
            for i, v in enumerate(f["produced_gated"]): PG[i] += v
    return K, T, kills, ticks, prod, S, SE, PG

K, T, kills, ticks, prod, S, SE, PG = load(sys.argv[1])
nk, nt = len(K), len(T)  # sated: [k][tau]; produced_gated: [n][k][tau]
for e, n in enumerate([2, 4, 8]):
    feas, best = 0, None
    for a in range(nk):
        for b in range(nt):
            i = a * nt + b; se = SE[i] / kills; kp = PG[e * nk * nt + i] / prod
            m = min(se - 0.60, kp - 0.75); feas += m >= 0
            if best is None or m > best[0]:
                best = (m, K[a], T[b], se, S[i] / kills, kp)
    m, k, tau, se, s, kp = best
    print(f"n={n}: {feas} feasible; best k={k:.3f} tau={tau:.2f}: energy-sated {se:.1%}"
          f" co-limited {s:.1%} kept {kp:.1%} margin {m:+.3f}")
```

## 4. Per-agent separation

Each agent's lifetime mean energy intake (light plus drain energy received), per tick, over
maintenance:

| population | setting | agents | p10 | p25 | median | p75 | p90 | mean |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| light-fed mixotroph kin killers | decoded | 2,097 | 0.298 | 1.18 | 3.02 | 6.87 | 13.9 | 6.91 |
| | fa 0 | 5,799 | 0.428 | 1.52 | 3.44 | 7.66 | 17.2 | 7.73 |
| heterotrophs by diet | decoded | 341 | 0.051 | 0.101 | 0.281 | 0.791 | 1.81 | 0.664 |
| | fa 0 | 1,068 | 0.071 | 0.152 | 0.382 | 0.790 | 1.69 | 0.747 |

- **#634's population-mean reading holds per agent.** The median killer averages about 3 m a tick
  over its life, and the median consumer about 0.3 m: a factor of about 10. Per-tick intake could
  not separate them (#634 §4). Averaged intake does, for most agents.
- **The killers' lower tail overlaps the consumers.** A tenth of the killers average under 0.30 /
  0.43 m, which is the consumers' median, and a quarter under 1.2 / 1.5 m, below the consumers' p90.
  A ceiling of k ≈ 1.7–2.2, where consumers keep most of their production, cannot sate that tail.
  This is why co-limited sated stalls in the 50s at the ridge. (A lifetime mean is not the fullness
  at the kill. These are context for the maps, not a second estimate of them.)

## 5. What this does not show

- **No physics change was run.** Every figure is from the ungated world. The maps predict what a
  gate *would* do, not what a gated world evolves into: #624 showed the gated population moves.
- **The production estimate is static.** A gated consumer may recoup intake on later ticks, because
  its prey persists, and that is not seen. So kept is conservative.
- **Kept has a floor** (§2) from the consumers' own light and from feasts taken while empty. The
  readout does not separate the two.
- **The energy-side region (§3) is read off the stored grids,** not from the instrument's own
  evaluator, and it is not a criterion the design states. It is there to show where the nutrient
  side binds.
- **Rows are not byte-identical between runs.** The `gates.diet_fed` samples of #634 are written in
  HashMap order. Every table, including all of the above, is identical.
- **The killers are atlas-wide,** not `sample:31`'s step lineages, and there is no LHS draw.

## 6. Verdict and what it implies

**The region is empty at n = 4, both as decoded and at `founder_aggregation = 0`, and at n = 2
and 8. Slice 2 (#630) does not start, and the orchestrator returns to design (grill-with-docs). The
`(k, τ)` placeholder in the world rules stays open.**

What the data shows:

1. **Fullness is the right kind of state.** It moved the closest margin from −0.34 to −0.02
   decoded. Averaged intake separates most killers from most consumers by a factor of about 10,
   where per-tick intake could not separate them at all.
2. **The co-limited max is what empties the region.** On energy alone the gate sates 84–94 % of
   the kills at the closest cells. `E = max(E_E, E_N)` lets a light-full,
   nutrient-poor killer read hungry. About a third of the mixotroph kin kills at the closest cells
   are of that kind.
3. **The design contradicts itself here.** The world rules call a killer sated on energy but hungry
   for nutrient "a nutrient-starved mixotroph feeding as the design intends (the carnivorous-plant
   pattern), not a failure of the gate", and report the energy-side share for that reason. But the
   sated bar counts every such kill as a failure. Read the design's way, the region is non-empty
   (§3). Read the bar's way, it is empty.
4. **Fixing that reading does not save many kin.** At the energy-side region's best cells, 62–66 %
   of the mixotroph kin kills still go ahead, and 40–50 % of those are by killers full of light but
   hungry for nutrient. Whether that is
   acceptable is the design's question.

**Directions the data points to, for the design session (not decisions):**

- **Restate the sated bar on the energy side,** as the carnivorous-plant clause already implies,
  and take a cell near k ≈ 2.9 with a short τ: at n = 4, k = 2.941 with τ = 1 or 1.43 are the
  only cells in both settings' energy regions. Kin-killing by nutrient-hungry mixotrophs is then left to recognition and dispersal.
- **Change how the nutrient side gates,** for instance by not letting it through for agents whose
  energy fullness is far above its ceiling, or by weighting it. That keeps one bar but changes the
  rules' "hunger follows the emptier currency".
- **The world rules' stated fallback:** stop asking satiation to sate mixotrophs at all, and leave
  their kin-killing to recognition and dispersal.

Separately, the kept floor (§2) means the 75 % bar is softer on the drain-borne production than it
reads. A design session that keeps the bar may want it read on the drain alone.

## 7. Instrument

Commit `ad2e8dc`, in `explorers-search`:

- **`fullness`** (new).
  - `TickIntake` and `tick_intakes` read each agent's tick: light plus drain energy received, and
    uptake plus drain nutrient retained.
  - `FullnessBank` holds one `F_E` and one `F_N` for each `τ` on `tau_grid` (12 log points over
    [1, 50]). Founders start at their first tick's light and uptake (`FullnessBank::at`), and a
    newborn copies its parent's bank.
  - `side_expression` and the co-limited `Expression` are the world rules' gate, at exponent n.
  - `FullnessTracker` follows the banks through a rollout, and `FullnessGrid` accumulates per seed
    over `k_grid` (17 log points over [0.25, 20]) × `τ` × n ∈ {2, 4, 8}: sated and energy-sated
    kills, consumer production and production gated, and per-agent lifetime mean intake for both
    populations. Pooling is a sum.
  - `FullnessRegion` evaluates the bars per n, and the report prints the maps, the feasible region,
    the largest-margin default or "region EMPTY" with the closest cell, the energy-side map and the
    per-agent table.
- **`intake_ceiling`:** the realised-drain booking is factored into `realised_drains`, which also
  gives the nutrient retained (each bite capped at the consumer's demand on the energy gained, as
  the drain pass does). `intake_readings` is unchanged in behaviour.
- **`role_diet`:** `rollout_with_fullness`. `SeedDiet.fullness` is optional, serde-defaulted and
  skipped when absent, so old rows read back and rows without `--fullness` serialise as before.
- **`role_diet_census`:** `--fullness` runs the readout, and `--fullness-region` prints it.
- **Tests:** `a_steady_intake_converges_to_its_rate`,
  `a_single_feast_spikes_then_decays_with_time_constant_tau`,
  `a_founder_starts_at_its_light_and_steady_light_converges_to_the_light_rate`,
  `a_newborn_starts_with_its_parent_s_bank`,
  `expression_follows_the_emptier_side_against_its_own_ceiling`,
  `without_growth_efficiency_the_gate_reads_energy_alone`,
  `being_below_half_expression_does_not_depend_on_the_exponent`,
  `a_consumer_books_drain_energy_received_and_nutrient_retained`,
  `the_grid_counts_hand_computed_kills_and_consumer_ticks`, `pooling_grids_is_a_sum`,
  `the_evaluation_picks_the_largest_margin_default_or_reports_empty`,
  `the_fullness_readout_leaves_the_rollout_identical`,
  `fullness_runs_the_readout_and_its_region_pools_the_rows`.

  The workspace has 936 tests, all passing, and `cargo fmt --check` is clean.
- **Artifacts** are in `target/637/`: `rd-fa{decoded,0}.{jsonl,txt}`, `rd-fa{decoded,0}-region.txt`,
  `run.sh`, `run.log` and `times.log`.
