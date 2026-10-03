# Issue #634: the `(k_a, k_h)` region for the heterotrophy-scaled intake ceiling

**Status: measurement. The region is empty at both settings, so slice 2 (#630) does not start and
the orchestrator returns to design. Adds per-sample gate records and a `(k_a, k_h)` region
evaluator to `role_diet_census` (commit `b976879`). It is observer-only: every table the tool
printed before is unchanged, and old rows read back. `explorers-sim`, the search, the evaluator and
the prefilter are unchanged.**

#629 showed that a ceiling scaled by the body alone cannot serve both populations. #633 then scaled
the ceiling with heterotrophy, `C = m × (k_a + k_h × h_eff)`
([world rules](../system-design/world-rules.md), *Capability and expression are decoupled*), and
bounded `(k_a, k_h)` by outcome, not by proxy:

- **sated mixotrophs:** at least 60 % of the kin-killing pairs whose killer is a light-fed
  mixotroph are predicted below half expression;
- **producing consumers:** heterotrophs by diet keep at least 75 % of their ungated production.

This slice reads both outcomes over a log grid of `(k_a, k_h)` on the ungated atlas.

## TL;DR

**Verdict: the region is empty at both settings. No `(k_a, k_h)` on the grid meets both bars.
Slice 2 (#630) does not start, and the orchestrator returns to design.**

| | decoded | `founder_aggregation = 0` |
|---|---|---|
| feasible cells | **0 of 306** | **0 of 306** |
| closest cell `(k_a, k_h)` | 0.748, 8.794 | 0.983, 11.565 |
| sated / kept there (bars 60 % / 75 %) | 26.4 % / 40.6 % | 24.7 % / 38.1 % |
| margin `min(sated − 0.60, kept − 0.75)` | −0.344 | −0.369 |
| n: mixotroph kin-kill pairs / diet-fed samples | 13,045 / 267 | 33,983 / 787 |

1. **Raising `k_h` lowers "sated" almost as fast as it raises "kept"** (§2). The heterotrophy term
   lifts both populations' ceilings together.
2. **Heterotrophy does not separate the populations; light does** (§3). The mixotroph kin killers
   have median `h_eff` 0.59 / 0.50, and the diet-fed heterotrophs 0.52 / 0.89. The killers are full
   mixotrophs, not slightly heterotrophic producers. What separates them is light: median light / m
   is 3.9 / 4.4 for the killers and 0.15 / 0.22 for the diet-fed.
3. **Per-tick intake cannot separate them either, because consumers feast and starve** (§4). A
   diet-fed heterotroph's production rides on rare big meals (p90 intake 1.8 / 2.8 m, max 6 / 33 m).
   A killer takes in a steady 4.0 / 4.6 m at the median. A consumer's feast tick looks like a
   mixotroph's ordinary tick. Any per-tick ceiling that lets the feast through also leaves the
   mixotroph room.
4. **Sharpening the saturation does not help** (§5). The best cells are about 28–33 % sated with
   41–45 % kept. The disk form is the worst of the shapes tried.
5. **What does separate them is recent average intake** (§6). Killers average about 4× maintenance
   a tick, and diet-fed heterotrophs about 0.7–1.1× (break-even). That points at a gate with
   per-agent state, which is evidence for the design session, not a decision.

## 1. What ran

| | |
|---|---|
| Tree | `b976879` (this branch: the instrument on top of #633). Release builds. |
| Census | `role_diet_census` on the atlas (`atlas.json`, 95 cells), seeds 1000–1004, T = 2000, `EvalConfig::default()`, **ungated** (`--satiation-sensitivity 0`), both as decoded and at `--founder-aggregation 0`. |
| Region | `role_diet_census --summary --region` on each run's rows: a log grid with `k_a` over [0.25, 20] and `k_h` over {0} ∪ [0.25, 20], 8 points per decade (17 × 18 = 306 cells). |
| Diagnostics | `diagnose.py` on the same rows (§3–5, script below). |
| Machine | 8-core laptop, one process at a time, awake throughout. |

**Commands** (cwd is the repo root; driver `target/634/run.sh`):

```sh
cargo build --release -p explorers-search --bin role_diet_census
C=$(seq -s, -f 'atlas:%g' 0 94)

# FAFLAG in "" "--founder-aggregation 0" -> rd-fadecoded / rd-fa0
./target/release/role_diet_census --atlas atlas.json --configs $C $FAFLAG \
    --satiation-sensitivity 0 --out target/634/rd-fa<…>.jsonl
./target/release/role_diet_census --summary --region --out target/634/rd-fa<…>.jsonl
```

**Wall clock** (`target/634/times.log`):

| run | time |
|---|---:|
| `role_diet_census`, atlas, decoded | 41 s |
| `role_diet_census`, atlas, `founder_aggregation = 0` | 84 s |
| `--summary --region` | < 1 s |
| `cargo test --workspace` at `b976879` | 923 pass, 0 fail |

The gate records keep the census near its #629 time (40–41 s / 81–82 s). Outputs are 3.8 MB
decoded and 7.2 MB at `founder_aggregation = 0`.

## 2. The region

Cells are sated / kept, in %. **Sated** is the share of kin-killing pairs whose killer is a
light-fed mixotroph with predicted `E < 0.5` (bar ≥ 60 %). **Kept** is heterotrophs by diet's
production kept, energy side, static: `Σ max(0, g − 1) / Σ max(0, x − 1)`, where `x` is intake / m
and `g = x·room / (x + room)` with the sample's own `room = max(0, k_a + k_h·h_eff − light/m)`
(bar ≥ 75 %). The grid below keeps every other row and column of the 17 × 18 printout, plus the
rows and columns of the two closest cells; the full maps are in `target/634/rd-fa*-region.txt`. No
cell anywhere on the full grid is feasible.

**Decoded** (n = 13,045 kin-kill pairs, 267 diet-fed samples):

| k_a \ k_h | 0 | 0.329 | 0.569 | 0.983 | 1.7 | 2.941 | 5.085 | 8.794 | 11.565 | 15.209 | 20 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0.25 | 96/0 | 94/0 | 93/0 | 88/0 | 77/0 | 61/1 | 45/18 | 31/37 | 25/50 | 19/61 | 15/70 |
| 0.432 | 93/0 | 91/0 | 89/0 | 84/0 | 72/0 | 57/2 | 43/19 | 29/38 | 23/51 | 18/62 | 14/70 |
| 0.748 | 89/0 | 86/0 | 83/0 | 76/0 | 65/0 | 52/3 | 39/21 | **26/41** | 21/53 | 17/63 | 13/71 |
| 0.983 | 85/0 | 81/0 | 77/0 | 70/0 | 60/0 | 49/5 | 37/22 | 25/42 | 20/54 | 15/63 | 12/71 |
| 1.293 | 78/0 | 73/0 | 70/0 | 63/0 | 54/0 | 44/7 | 33/23 | 23/45 | 18/55 | 14/64 | 10/72 |
| 2.236 | 59/0 | 54/0 | 52/0 | 47/1 | 42/3 | 34/13 | 26/30 | 18/50 | 14/59 | 11/67 | 8/74 |
| 3.867 | 37/3 | 35/4 | 33/6 | 31/9 | 28/16 | 23/29 | 18/43 | 12/58 | 10/64 | 7/71 | 5/76 |
| 6.687 | 18/29 | 17/32 | 17/34 | 15/37 | 14/42 | 12/49 | 10/57 | 6/66 | 5/71 | 4/75 | 3/79 |
| 11.565 | 6/57 | 6/58 | 6/59 | 5/60 | 5/62 | 4/65 | 3/70 | 2/75 | 2/77 | 1/80 | 1/83 |
| 20 | 2/74 | 2/75 | 2/75 | 1/76 | 1/76 | 1/78 | 1/80 | 1/82 | 1/84 | 1/85 | 1/87 |

**`founder_aggregation = 0`** (n = 33,983 kin-kill pairs, 787 diet-fed samples):

| k_a \ k_h | 0 | 0.329 | 0.569 | 0.983 | 1.7 | 2.941 | 5.085 | 8.794 | 11.565 | 15.209 | 20 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0.25 | 98/0 | 96/0 | 95/0 | 91/0 | 83/1 | 70/4 | 54/12 | 37/26 | 30/35 | 23/44 | 17/53 |
| 0.432 | 96/0 | 94/0 | 92/0 | 88/0 | 79/1 | 67/5 | 52/13 | 36/27 | 28/36 | 22/44 | 16/53 |
| 0.748 | 93/0 | 90/0 | 87/0 | 82/0 | 74/1 | 62/6 | 48/14 | 33/29 | 26/37 | 20/45 | 14/54 |
| 0.983 | 90/0 | 86/0 | 83/0 | 78/0 | 69/2 | 58/7 | 45/15 | 31/30 | **25/38** | 19/46 | 13/54 |
| 1.293 | 84/0 | 80/0 | 77/0 | 72/1 | 64/3 | 54/8 | 41/17 | 29/31 | 23/39 | 17/47 | 12/55 |
| 2.236 | 66/1 | 62/1 | 60/2 | 56/3 | 50/6 | 42/14 | 32/23 | 22/35 | 18/42 | 14/49 | 10/57 |
| 3.867 | 42/8 | 40/10 | 38/12 | 36/15 | 32/19 | 27/24 | 21/31 | 15/40 | 12/46 | 9/53 | 7/60 |
| 6.687 | 21/26 | 20/27 | 19/28 | 18/29 | 16/31 | 14/34 | 11/39 | 8/47 | 7/52 | 5/57 | 4/63 |
| 11.565 | 8/40 | 7/40 | 7/41 | 7/42 | 6/43 | 6/46 | 5/50 | 4/56 | 3/59 | 3/64 | 2/68 |
| 20 | 3/54 | 2/55 | 2/55 | 2/56 | 2/57 | 2/59 | 2/61 | 2/65 | 1/68 | 1/71 | 1/75 |

(Bold: the closest cell, the largest margin `min(sated − 0.60, kept − 0.75)`.)

**`k_h = 0`, the body-only ceiling of #629** (`k = k_a`):

| k_a | decoded: sated | decoded: kept | fa 0: sated | fa 0: kept |
|---:|---:|---:|---:|---:|
| 0.25 | 96.1 % | 0.0 % | 98.1 % | 0.0 % |
| 0.432 | 93.4 % | 0.0 % | 96.5 % | 0.0 % |
| 0.748 | 89.5 % | 0.0 % | 93.0 % | 0.0 % |
| 0.983 | 85.3 % | 0.0 % | 89.5 % | 0.0 % |
| 1.293 | 78.0 % | 0.0 % | 84.3 % | 0.0 % |
| 1.7 | 69.1 % | 0.0 % | 76.6 % | 0.1 % |
| 2.236 | 58.8 % | 0.0 % | 66.3 % | 0.7 % |
| 2.941 | 47.5 % | 0.3 % | 54.3 % | 2.2 % |
| 3.867 | 36.8 % | 3.0 % | 42.2 % | 8.4 % |
| 5.085 | 26.6 % | 13.2 % | 30.8 % | 17.5 % |
| 6.687 | 18.2 % | 28.8 % | 20.6 % | 26.2 % |
| 8.794 | 11.6 % | 44.4 % | 12.9 % | 33.5 % |
| 11.565 | 6.0 % | 56.8 % | 7.7 % | 39.6 % |
| 15.209 | 3.4 % | 66.7 % | 4.5 % | 47.0 % |
| 20 | 1.6 % | 74.4 % | 2.5 % | 54.4 % |

**Why this column differs from #629's tables.** The gate's expression at `k_h = 0` is #629's exactly
(tested, §8), but the two outcomes are read on different populations and with a sharper estimate:

- **Sated** counts only kin pairs whose killer is a light-fed mixotroph, as #633's criterion
  states (13,045 of #629's 23,309 kin pairs decoded). Those killers have more light, so more of
  them read sated: 69.1 % at k = 1.7 against #629's 57.9 % over all kin killers.
- **Kept** subtracts each consumer's own light from its room and is exact per sample. #629's
  estimate ignored light and binned. So kept is lower here: 0.0 % at k = 1.7 against #629's 3.1 %.

Neither change moves the verdict; #629's body-only conflict is the `k_h = 0` column.

**Reading the map.**

- **Along any row, raising `k_h` trades one outcome for the other almost one for one.** At
  `k_a = 0.748` decoded, `k_h` going from 5.1 to 8.8 to 15.2 moves sated / kept from 39/21 to 26/41
  to 17/63. Sated + kept stays below 100 in every cell, short of the 135 the two bars need together.
- **The best cells sit at small `k_a` and large `k_h`,** where the ceiling is mostly heterotrophy.
  That is the direction #629 proposed. It helps a little over `k_h = 0` (best margin −0.34 / −0.37
  against about −0.46 / −0.47 on the body-only column), but nowhere near enough.

## 3. Heterotrophy does not separate the populations

The `k_h` term was meant to give consumers a higher ceiling than mixotrophs. It can only do that if
consumers are more heterotrophic. They are not (from the stored gate samples; script in §5):

| population | reading | setting | n | p10 | p25 | median | p75 | p90 | mean |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| mixotroph kin killers | `h_eff` | decoded | 13,045 | 0.107 | 0.325 | 0.587 | 0.989 | 1.29 | 0.674 |
| | | fa 0 | 33,983 | 0.174 | 0.284 | 0.499 | 0.927 | 1.29 | 0.646 |
| heterotrophs by diet | `h_eff` | decoded | 267 | 0.378 | 0.430 | 0.516 | 1.12 | 1.53 | 0.772 |
| | | fa 0 | 787 | 0.381 | 0.457 | 0.889 | 1.25 | 1.65 | 0.924 |
| mixotroph kin killers | light / m | decoded | 13,045 | 1.42 | 2.24 | 3.90 | 7.20 | 12.8 | 6.04 |
| | | fa 0 | 33,983 | 1.59 | 2.60 | 4.44 | 7.76 | 12.9 | 6.73 |
| heterotrophs by diet | light / m | decoded | 267 | 0.025 | 0.052 | 0.147 | 0.576 | 1.40 | 0.504 |
| | | fa 0 | 787 | 0.025 | 0.066 | 0.217 | 0.675 | 1.48 | 0.699 |
| mixotroph kin killers | `P_E` / m | decoded | 13,045 | 0.007 | 0.029 | 0.091 | 0.257 | 0.643 | 0.254 |
| | | fa 0 | 33,983 | 0.005 | 0.027 | 0.096 | 0.272 | 0.637 | 0.253 |
| heterotrophs by diet | `P_E` / m | decoded | 267 | 0 | 0 | 0.043 | 0.261 | 0.927 | 0.237 |
| | | fa 0 | 787 | 0 | 0 | 0.067 | 0.601 | 1.50 | 0.433 |

- **The killers are full mixotrophs.** Their median `h_eff` (0.59 / 0.50) is about the diet-fed
  heterotrophs' (0.52 / 0.89), and decoded it is higher. They are not producers with a trace of
  heterotrophy. So `k_h × h_eff` raises both ceilings by about the same amount, and every step in
  `k_h` that gives consumers room gives killers room too.
- **Light is what separates them,** by a factor of about 20–25 at the median. But light already
  enters the gate, as what fills the ceiling. A ceiling that scales with heterotrophy cannot use it a
  second time.
- **Kin-killing is energetically trivial for the killers.** Their drain potential is a median 0.09
  m against light of 3.9 m. They kill kin on the way, not to eat.

## 4. Per-tick intake cannot separate them: consumers feast and starve

| population | intake / m | n | p10 | p25 | median | p75 | p90 | mean | max |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| mixotroph kin killers | decoded | 13,045 | 1.48 | 2.31 | 4.01 | 7.36 | 13.1 | 6.18 | 165 |
| | fa 0 | 33,983 | 1.68 | 2.67 | 4.56 | 7.94 | 13.2 | 6.88 | 216 |
| heterotrophs by diet | decoded | 267 | 0.044 | 0.096 | 0.301 | 1.08 | 1.78 | 0.705 | 6.12 |
| | fa 0 | 787 | 0.050 | 0.129 | 0.431 | 1.35 | 2.82 | 1.09 | 33.2 |

- **Consumers are feast-and-famine.** The median diet-fed tick takes in a third to a half of
  maintenance. The mean is about break-even (0.70 / 1.09), and production above maintenance comes
  from the tail: p90 1.8 / 2.8 m, max 6 / 33 m.
- **A killer's ordinary tick is a consumer's feast.** The killers' p25 intake (2.3 / 2.7 m) is
  above the consumers' p90. Their median is a steady 4.0 / 4.6 m.
- **So any per-tick ceiling fails one side.** A ceiling low enough to sate the killer (a few m)
  clips exactly the meals that carry consumer production. A ceiling high enough to let a feast
  through leaves the killer room, and with its tiny `P` (§3) it then expresses near 1. This is the
  disk form's arithmetic from #629 §4: the killer reads sated on the energy side only if `room < P`, so only if light
  fills the ceiling to within about 0.1 m of the top.

## 5. Sharper saturation does not help

The disk equation gives up intake well below the ceiling (it halves at `P = room`). A sharper
saturation might let feasts through nearly whole up to the ceiling and still sate the killers. The
check: replace `E = room / (room + P)` with `E = (1 + (P/room)^n)^(−1/n)`, so gated intake is
`g = x / (1 + (x/room)^n)^(1/n)` (n = 1 is the disk form), on a body-only ceiling `C` (ticks of m),
energy and nutrient sides as the gate reads them.

Sated / kept, in %:

| n | setting | C = 1 | 1.5 | 2 | 2.5 | 3 | 4 |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | decoded | 85/0 | 74/0 | 63/0 | 54/0 | 47/0 | 36/4 |
| 2 | | 79/0 | 65/0 | 55/2 | 46/5 | 39/12 | 29/28 |
| 4 | | 78/0 | 64/2 | 53/6 | 44/13 | 38/25 | 28/41 |
| 8 | | 77/0 | 63/3 | 52/9 | 44/17 | 37/29 | 28/45 |
| 1 | fa 0 | 89/0 | 80/0 | 71/0 | 62/1 | 53/2 | 41/9 |
| 2 | | 85/0 | 75/1 | 64/3 | 55/7 | 47/14 | 35/30 |
| 4 | | 84/0 | 73/2 | 63/6 | 53/12 | 45/22 | 33/40 |
| 8 | | 84/0 | 73/3 | 62/8 | 53/14 | 45/25 | 33/45 |

- **The best cells are about 28–33 % sated with 41–45 % kept** (C = 4, n = 8). That is no better
  than the heterotrophy-scaled disk form's closest cell.
- **Where the killers are still mostly sated (C = 2), consumers keep only 6–9 %.** Sharpening moves
  the loss out of the sub-ceiling region, but the feasts are above any ceiling the killers allow.
- **The disk form (n = 1) is the worst shape tried.**

To rerun (reads the gate samples from the census rows; it reproduces §3–5):

```python
# python3 diagnose.py   (cwd: repo root; reads target/634/rd-fa{decoded,0}.jsonl)
import json

def load(fa):
    K, D = [], []
    for line in open(f"target/634/rd-{fa}.jsonl"):
        for s in json.loads(line).get("seeds", []):
            K += s["gates"]["mixotroph_kin_kills"]
            D += s["gates"]["diet_fed"]
    return K, D

def q(v):
    v = sorted(v); n = len(v)
    return (f"n={n} p10 {v[n//10]:.3g} p25 {v[n//4]:.3g} med {v[n//2]:.3g} p75 {v[3*n//4]:.3g}"
            f" p90 {v[9*n//10]:.3g} mean {sum(v)/n:.3g} max {v[-1]:.3g}")

def E1(room, P, n):  # generalised saturation; n = 1 is the disk equation
    if room <= 0: return 0.0
    if P <= 0: return 1.0
    return (1 + (P / room) ** n) ** (-1 / n)

def expr(x, C, n):  # the gate's co-limited expression at body-only ceiling C
    m = x["m"]; e = E1(C * m - x["light"], x["pe"], n)
    if x["eta"] > 0:
        e = max(e, E1(C * m * x["eta"] * x["ratio"] - x["uptake"], x["pn"], n))
    return e

def kept(D, C, n):  # static production kept, energy side, own light in the room
    num = den = 0
    for x in D:
        xi = x["intake"] / x["m"]; g = xi * E1(C - x["light"] / x["m"], xi, n)
        num += max(0, g - 1); den += max(0, xi - 1)
    return num / den

for fa in ["fadecoded", "fa0"]:
    K, D = load(fa); print("==", fa)
    for name, S in [("mixotroph kin killers", K), ("heterotrophs by diet", D)]:
        print(f" {name}:")
        for lab, f in [("h_eff", lambda x: x["h"]), ("light/m", lambda x: x["light"] / x["m"]),
                       ("P_E/m", lambda x: x["pe"] / x["m"]),
                       ("intake/m", lambda x: x["intake"] / x["m"])]:
            print(f"   {lab:9} {q([f(x) for x in S])}")
    for n in [1, 2, 4, 8]:
        print(f"  n={n}: " + "  ".join(
            f"C={C}:{100*sum(expr(x, C, n) < 0.5 for x in K)/len(K):.0f}/{100*kept(D, C, n):.0f}"
            for C in [1.0, 1.5, 2.0, 2.5, 3.0, 4.0]))
```

## 6. Verdict and what it implies

**The region is empty at both settings, so #630 does not start and the orchestrator returns to
design.** The `(k_a, k_h)` placeholder in the world rules stays open.

There are two findings, and together they rule out the whole family, not just this form:

1. **Heterotrophy does not separate the populations (§3).** The kin killers are as heterotrophic as
   the consumers, so a heterotrophy term cannot give one a higher ceiling than the other. Light
   separates them, and light already fills the ceiling.
2. **Per-tick intake does not separate them either (§4–5).** Consumers make their production from
   rare feasts, and a feast tick looks like a mixotroph's ordinary tick. Per-tick intake against
   any ceiling, whether body-scaled, heterotrophy-scaled, disk or sharp, cannot both let the feast
   through and leave the mixotroph no room.

**What does separate them is recent average intake.** Killers average about 4× maintenance every
tick, from steady light. Diet-fed heterotrophs average about 0.7–1.1×, which is break-even.
(Caveat: these are population means over instantaneous samples. The records carry no agent ids, so
per-agent average intake is not measured here.)

**Evidence for the design session.** This is evidence, not a decision. One candidate is satiation
as **gut fullness**:

> fullness = intake accumulated and cleared over a few ticks (a moving average with clearance
> time τ, kept as per-agent state), compared against a ceiling; expression is a steep function of
> fullness, not of this tick's potential `P`.

- Read before feeding, a consumer with an empty gut takes a feast whole. The feast then sates it
  while it clears, which is Holling handling time in another form.
- Steady light keeps a mixotroph full every tick, so it reads sated whatever `P` is in reach.

This reverses the world rules' choice to read this tick's potential with no state ("Expression is
read once per consumer, at the start of drain resolution"). The stated reason is §4: without state,
the gate cannot tell a feast from a steady income. Testing it would need a per-agent fullness
readout: per-agent intake histories at the gate's read point, which the present records (one
instantaneous sample, no agent id) cannot rebuild.

## 7. What this does not show

- **No physics change was run.** Every figure is from the ungated world. The maps predict what a
  gate *would* do, not what a gated world evolves into: #624 showed the gated population moves.
- **The production estimate is static and lone-feeder,** as in #629 §5. A gated consumer may recoup
  intake on later ticks; that is why the bar is 75 %, and the best cells are near 40 %.
- **The killers are atlas-wide,** not `sample:31`'s step lineages.
- **Small consumer samples:** n is 267 decoded and 787 at fa 0.
- **The sharper-saturation check uses a body-only ceiling.** It does not combine sharp saturation
  with `k_h`; given §3, `k_h` lifts both populations together and would not change the picture.
- **Average intake is a population mean,** not per agent (§6).
- **No LHS draw.** The verdict does not hinge on the atlas sample.

## 8. Instrument

Commit `b976879`, in `explorers-search`:

- **`intake_ceiling`.**
  - `GateSample` is one compact, exact agent-sample at the gate's read point: `h_eff`, maintenance,
    light, uptake, `ratio`, growth efficiency, `P_E`, `P_N` and realised energy intake (light plus
    drain income). `GateSample::expression(k_a, k_h)` is #629's `expression_at` at ceiling multiple
    `k_a + k_h × h_eff`. `gated_intake_ticks` is the static `x·room / (x + room)` with the sample's
    own room.
  - `GateSamples` holds `mixotroph_kin_kills` (one sample per kin-killing pair whose killer is a
    light-fed mixotroph, the killer read on the tick of the kill) and `diet_fed` (one per
    second-half agent-sample of a heterotroph by diet).
  - `sated_share` and `production_kept` are the two outcomes, with bars `SATED_BAR = 0.60` and
    `KEPT_BAR = 0.75`. `Region::evaluate` maps them over a grid, `default_point` is the
    largest-margin feasible cell, and `region_report` prints the map, the feasible count, the
    default or "region EMPTY" with the closest cell, and the `k_h = 0` column. `log_grid` builds the
    axes.
- **`role_diet_census`** records `GateSamples` on `SeedDiet` as `gates` (serde-defaulted, so old
  rows read back). `--summary --region` pools them over all configs and prints the region.
- **Tests:**
  - samples on hand-built agents: `a_light_only_producer_samples_its_light_and_a_ceiling_of_k_a`,
    `a_mixotroph_with_small_heterotrophy_has_a_ceiling_just_above_k_a`,
    `a_pure_consumer_samples_its_drain_income_as_intake`,
    `a_nutrient_limited_sample_is_kept_hungry_by_its_nutrient_room`,
    `without_growth_efficiency_a_sample_reads_energy_alone`;
  - the evaluation: `both_outcomes_match_hand_computed_values`,
    `the_region_is_the_feasible_cells_and_the_default_its_largest_margin`,
    `the_log_grid_spans_its_ends_geometrically`,
    `at_k_h_zero_the_gate_reduces_to_the_body_only_ceiling`,
    `the_region_report_names_the_default_or_says_empty`;
  - the census: `rollout_keeps_gate_samples_for_both_populations`,
    `region_evaluates_the_grid_over_pooled_gate_samples`.

  The workspace has 923 tests, all passing.
- **Artifacts** are in `target/634/`: `rd-fa{decoded,0}.{jsonl,txt}`, `rd-fa{decoded,0}-region.txt`,
  `diagnose.py` and `diagnose.txt`, `run.sh` and `times.log`.
