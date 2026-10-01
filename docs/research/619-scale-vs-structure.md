# Issue #619: do need-gating and recognition fail on scale or by design?

**Status: measurement. Adds `--satiation-sensitivity C` and `--recognition-distance D` to the
`reinvasion_barrier` and `role_diet_census` bins. No trajectory changes when the flags are unset.
`explorers-sim`, the search, the evaluator and the prefilter are unchanged.**

#605 and #606 found that need-gating and recognition, at their defaults (`satiation_sensitivity`
c = 0.1, `recognition_distance` D = 0.5), leave infant kin-grazing on `sample:31` where it was, and
that nothing invades by the #587 criterion. #606 (§10) left two explanations open:

- **Scale.** The defaults are mis-sized for the searched worlds. At c = 0.1 an agent holding about 3
  ticks of maintenance still expresses about 0.77, so recognition leaves about 0.27 of capability
  toward an identical newborn. D = 0.5 is only 1.6σ of `sample:31`'s mutation size. If this is the
  cause, putting c and D in #607's search box fixes it.
- **Structure.** The gate reads satiation in ticks of maintenance, but the grow phase mobilises
  reserve above `growth_retention_multiplier` × maintenance. Agents therefore sit at that buffer at
  every c, and a larger c weakens all consumption rather than separating fed from hungry. If this is
  the cause, the design changes first.

This note runs the `sample:31` barrier over a 4 × 3 grid of (c, D), reads the killing grazers'
satiation bands, and runs the role-diet census on the atlas at two cells.

## TL;DR

**Verdict: both, with structure deciding the recommendation.**

1. **Scale is real.** At c ≥ 1 with D ≥ 1, infant kin-grazing on the step lineages halves or better.
   Infant grazed deaths per birth, `step 1×` / `step 2×`, own dispersal:

   | c \ D | 0.5 | 1.0 | 1.5 |
   |---|---|---|---|
   | 0.1 | 0.81 / 0.91 | 0.83 / 0.96 | 0.76 / 0.91 |
   | 0.3 | 0.62 / 0.73 | 0.60 / 0.66 | 0.49 / 0.65 |
   | 1.0 | 0.44 / 0.48 | **0.24 / 0.40** | **0.17 / 0.32** |
   | 3.0 | 0.50 / 0.49 | **0.34 / 0.32** | **0.13 / 0.23** |

   - D alone does nothing at c = 0.1. It helps only once c has pulled expression at the buffer below
     recognition's restraint.
   - `step 2×`'s plain-mode median `r` rises from −0.00173 to between −0.00118 and −0.00001 at
     c ≥ 0.3. `step 1×` improves less, and at D = 1.5 with c ≥ 1 it collapses.
2. **But it works the structural way.**
   - **The killers do not become sated. The sated ones stop killing.** The share of `step 2×`'s kin
     killers holding under 1 tick of reserve rises from 7 % (c = 0.1, D = 0.5) to 75–85 % (c = 3,
     D ≥ 1). The share at ≥ 5 ticks falls from 38 % to 0–1 %. What remains is predation by
     starving grazers, which is what the gate allows by design.
   - **The steps' whole heterotrophic income falls in proportion.** Living plus carcass gain per
     member-tick falls 55–84 % for `step 2×` at c ≥ 1 (0.0152 → 0.0025–0.0068). The kin-grazed energy taken
     off them falls 55–79 % (0.0071 → 0.0015–0.0032). The gate does not single out kin. It damps
     every drain an agent at the retention buffer makes, and recognition then removes the kin share.
3. **Nothing invades by the #587 criterion, in any of the 12 cells or 3 modes.** The best reading is
   `step 2×` at dispersal 4 with c = 3, D = 1.0: median +0.00059, 5/8 seeds positive, interval
   [−0.00277, …]. The founder heterotroph (`full`) still dies out on 8/8 seeds within 7–12 ticks in
   every cell.
4. **The atlas moves much less than `sample:31`** (base → c = 1, D = 1 → c = 3, D = 1.5):
   - Infant producer deaths grazed by kin fall 13 % and 21 % (32,284 → 28,178 → 25,430). The
     kin-grazed share of trait-producer deaths goes 25.7 % → 21.6 % → 19.6 %.
   - Kin kill pairs fall 6 % and 22 %. Non-kin kill pairs do not collapse (15.7k → 21.2k → 15.8k).
   - The share of kin killers holding under 1 tick goes 31 % → 66 % → 92 %.
   - No config, and no seed, holds a consumer or decomposer guild by diet. Generalist dominance goes
     5 → 1 → 3 seeds.
5. **The resident's own consumers do not grow at c ≥ 1.** On all six c ≥ 1 control forks, the
   resident's consumers break even (net −0.0000 to +0.038 E per member-tick), fill no earmark
   (≤ 0.05) and have no births. Their grazing income does not collapse (0.001–0.016 against 0.009).
   This is what a gate that shuts at the retention buffer predicts. It rests on 2–5 seeds and 62–880
   member-ticks per cell, and some c < 1 cells look similar (§4), so it is suggestive only.
6. **Recommendation: a design change before #607**, through grill-with-docs (§6). The gate reads a
   quantity the grow phase holds at the same value for a light-fed mixotroph and a starving consumer.
   Raising c can only move the cut-off. The steps' earmark fill (0.22–1.07) separates them from the
   resident's consumers (0.00–0.05 at c ≥ 1), where reserve does not. If #607 must go first, put
   `satiation_sensitivity` ∈ [0.1, 3] (log scale) and `recognition_distance` ∈ [0.5, 1.2] in the
   box, and read consumer reproduction, not only kin-grazing.

## 1. What ran

| | |
|---|---|
| Tree | `bf9136b` (main, after #612) plus this issue's flags. Release build. |
| Founder aggregation | Pinned to 0 (`--founder-aggregation 0`), as #605/#606. |
| Grid | c ∈ {0.1, 0.3, 1.0, 3.0} × D ∈ {0.5, 1.0, 1.5}. (0.1, 0.5) is the default. |
| `reinvasion_barrier` | `sample:31`, seeds 1000–1007 (8/8 reach tick 1000 in every cell), injection at tick 1000, window 1000, cohort 8. Plain (#587), `--accounting` (#591) and `--dispersal` (#593) modes. |
| `role_diet_census` | Atlas (`atlas.json`, 95 cells), seeds 1000–1004, T = 2000, `EvalConfig::default()`, at the default and at (1.0, 1.0) and (3.0, 1.5). |
| Machine | 8-core laptop, one process at a time. |

**The pins change the whole world, not just the cohort.** The resident runs under the pinned (c, D)
from tick 0, so every cell forks its own resident at tick 1000. At injection the resident has 92–136
agents (100 at the default) and a free pool of 2,579–6,356 N (5,557 N). Its control rate is −0.00023
to −0.00060 (−0.00040). As in #605/#606, compare cells as whole worlds.

**Commands** (cwd is the repo root):

```sh
cargo build --release -p explorers-search --bin reinvasion_barrier --bin role_diet_census

# the grid: for C in 0.1 0.3 1.0 3.0, D in 0.5 1.0 1.5, MODE in "" --accounting --dispersal
./target/release/reinvasion_barrier sample:31 $MODE --founder-aggregation 0 \
    --satiation-sensitivity $C --recognition-distance $D --out target/619/rb-c$C-d$D-<mode>.json

# baseline check, flags unset
./target/release/reinvasion_barrier sample:31 $MODE --founder-aggregation 0 --out target/619/rb-<mode>-unset.json

# atlas: unset, then the two cells
./target/release/role_diet_census --atlas atlas.json --configs atlas:0,…,atlas:94 --founder-aggregation 0 \
    --out target/619/rd-atlas-base.jsonl
./target/release/role_diet_census --atlas atlas.json --configs atlas:0,…,atlas:94 --founder-aggregation 0 \
    --satiation-sensitivity 1.0 --recognition-distance 1.0 --out target/619/rd-atlas-c1.0-d1.0.jsonl
./target/release/role_diet_census --atlas atlas.json --configs atlas:0,…,atlas:94 --founder-aggregation 0 \
    --satiation-sensitivity 3.0 --recognition-distance 1.5 --out target/619/rd-atlas-c3.0-d1.5.jsonl
./target/release/role_diet_census --summary --out target/619/<file>.jsonl
```

**Wall clock:**

| run | time |
|---|---:|
| `reinvasion_barrier` plain / accounting / dispersal, c = 0.1 | 13–15 s / 9–10 s / 16–19 s |
| the same, c = 3.0 | 15–19 s / 11–14 s / 20–25 s |
| the whole 36-run grid | 9 min 01 s (541 s) |
| `role_diet_census`, atlas: default / (1.0, 1.0) / (3.0, 1.5) | 60 s / 64 s / 66 s |

Larger c and D make the `sample:31` worlds a little heavier: the steps' member-ticks rise up to 3×
(`step 2×`: 2,292 → 7,202). Nothing came near needing a cap.

**Reproduction checks.**
- **Unset is the default pin.** With the flags unset, all three modes print exactly the tables of
  the (0.1, 0.5) cell, and the plain artifact's seed records are byte-identical. In the accounted
  modes every field outside the per-member `account` floats is identical. Those floats differ in the
  last ulp, and they differ the same way between two runs of the unset command. The accountant sums
  over `HashMap`s with a random hasher seed (pre-existing; recorded in `known-traps.md`).
- **#612 changes the injected cohorts, not the resident.** Against #606's recognition arm, the
  resident and control lines reproduce digit for digit (population 100, pool 5,557 N, control rate
  −0.00040). The step arms do not. #612 changed `invasion::place_cohort` too: a cohort member whose
  cell cannot cover its bound nutrient now draws the shortfall from the nearest cells, where before
  it drove the cell negative. At tick 1000 many cells are short. So **this note's (0.1, 0.5) cell is
  the baseline**, not #606's tables. For example, `step 2×` at own dispersal: infant grazed per birth
  0.91 (0.93 in #606), median `r` −0.00173 (−0.00173).
- **The atlas is unchanged by #612.** All 95 rows of the unset census are identical to #606's
  recognition-arm atlas rows.

## 2. `sample:31`: kin-grazing and invasion

Own dispersal, uniform placement, `--dispersal` mode. Cells are `step 1×` / `step 2×`.

**Kin-grazed energy** (E per member-tick taken off the lineage by its own members):

| c \ D | 0.5 | 1.0 | 1.5 |
|---|---|---|---|
| 0.1 | 0.0038 / 0.0071 | 0.0040 / 0.0042 | 0.0060 / 0.0022 |
| 0.3 | 0.0041 / 0.0084 | 0.0043 / 0.0115 | 0.0066 / 0.0086 |
| 1.0 | 0.0011 / 0.0032 | 0.0004 / 0.0022 | 0.0002 / 0.0031 |
| 3.0 | 0.0036 / 0.0031 | 0.0002 / 0.0029 | 0.0001 / 0.0015 |

This is noisier than infant grazing per birth (the TL;DR table): it moves 3× across D at c = 0.1,
where infant grazing does not move. Read the per-birth figure first.

**Median `r`, plain mode** (r > 0 seeds in brackets). No arm in any cell or mode invades: no sign-test
interval clears zero, and the highest lower bound anywhere is −0.00047.

| c \ D | 0.5 | 1.0 | 1.5 |
|---|---|---|---|
| 0.1 | −0.00208 (0/8) / −0.00173 (0/8) | −0.00173 (0/8) / −0.00208 (0/8) | −0.00173 (2/8) / −0.00277 (0/8) |
| 0.3 | −0.00118 (1/8) / −0.00069 (1/8) | −0.00118 (0/8) / −0.00073 (0/8) | −0.00098 (0/8) / −0.00118 (0/8) |
| 1.0 | −0.00073 (2/8) / −0.00008 (4/8) | −0.00063 (1/8) / −0.00084 (2/8) | −0.00208 (1/8) / −0.00069 (1/8) |
| 3.0 | −0.00084 (2/8) / −0.00018 (4/8) | −0.00104 (2/8) / −0.00021 (2/8) | −0.00243 (1/8) / −0.00001 (4/8) |

- **The steps get closer to zero at c ≥ 0.3.** None crosses it. The `step 2×` dispersal-4 arm is the
  only one with a positive median in several cells: +0.00002 to +0.00059, 4–5/8 seeds. Its interval
  always includes −0.00277.
- **`step 1×` at D = 1.5 with c ≥ 1 collapses** (−0.00208 / −0.00243). Its births fall and
  its photosynthesis to 0.30–0.33 E per member-tick (births 29 and 46). Its lineage is spared by kin and dies of
  something else (starved deaths are 14–26 of 74–87).
- **The producer cohort (`step 0`) is barely grazed at D = 1.5 with c ≥ 1.** It has 0–2 grazed
  deaths and no kin kills. The producer–consumer separation in the world rules is about 1.4, so D =
  1.5 starts to shelter the producer from the near-producer steps as well as from their own kin. That
  is the prey-sheltering #606 §10 warned of.

## 3. Killing grazers: sated or starving?

Kin kill pairs (grazer and victim both lineage members) of members grazed to death, summed over 8
seeds, own dispersal, by the grazer's satiation in the drain pass (ticks of maintenance in its scarcer
currency). The table gives `step 1×` / `step 2×` pairs, then `step 2×`'s share under 1 tick and its
share at ≥ 5 ticks, then `step 1×`'s share under 1 tick.

| c \ D | 0.5 | 1.0 | 1.5 |
|---|---|---|---|
| 0.1 | 105 / 236; 7 % / 38 % (1×: 8 %) | 117 / 207; 8 % / 33 % (9 %) | 206 / 244; 4 % / 34 % (0 %) |
| 0.3 | 159 / 448; 0 % / 9 % (16 %) | 174 / 383; 8 % / 2 % (3 %) | 158 / 290; 8 % / 0 % (13 %) |
| 1.0 | 151 / 485; 12 % / 6 % (10 %) | 67 / 186; 31 % / 1 % (25 %) | 12 / 110; 59 % / 0 % (75 %) |
| 3.0 | 213 / 310; 32 % / 5 % (27 %) | 68 / 212; 75 % / 1 % (54 %) | 18 / 82; 85 % / 0 % (72 %) |

- **At the default, killers sit at 1–5 ticks**, as #606 found. That is the retention buffer
  (`growth_retention_multiplier` = 2.68 on `sample:31`).
- **Raising c does not move the killers toward sated. It removes the killers at the buffer.** At
  c = 3 an agent at 2.7 ticks expresses 1/(1 + 3 × 2.7) ≈ 0.11, below recognition's ½ restraint, so
  it spares an identical newborn completely. The kills that remain come from grazers under 1 tick,
  which express 0.25–1.
- **Reading against the criterion.**
  - The scale criterion asked for the killers' distribution to shift toward sated. It shifts the
    other way.
  - The structure criterion asked for killers that stay at 1–5 ticks. They do not stay either.
  - The underlying fact matches the structural hypothesis: the population still sits at the
    buffer, and c only decides how much an agent at the buffer expresses. No agent reaches a state
    the gate reads as fed. A larger c makes the buffer itself read as fed, for every agent and
    toward every target.
- **The atlas agrees** (§5): the kin killers' share under 1 tick goes 31 % → 66 % → 92 %.

## 4. Is non-kin grazing damped too?

**The steps.** Their heterotrophic income per member-tick (living plus carcass gain, accounting at
own dispersal), `step 1×` / `step 2×`:

| c \ D | 0.5 | 1.0 | 1.5 |
|---|---|---|---|
| 0.1 | 0.0087 / 0.0152 | 0.0110 / 0.0150 | 0.0115 / 0.0171 |
| 0.3 | 0.0086 / 0.0114 | 0.0077 / 0.0145 | 0.0080 / 0.0133 |
| 1.0 | 0.0034 / 0.0058 | 0.0036 / 0.0056 | 0.0022 / 0.0061 |
| 3.0 | 0.0049 / 0.0068 | 0.0024 / 0.0041 | 0.0023 / 0.0025 |

- **It falls as much as kin-grazing does.** For `step 2×` at c ≥ 1 it falls 55–84 %, against
  55–79 % for the kin-grazed energy. For `step 1×` it falls 44–75 %. Carcass gain, which recognition never touches, falls with it (`step 2×`: 0.0104
  → 0.0020–0.0049).
- **For a light-fed mixotroph that is the intended effect:** its heterotrophy should be latent. It
  does not show that the gate can tell a mixotroph from a consumer.

**The resident's consumers** (control fork, agents of role consumer or decomposer at tick 1000):

| cell | seeds with any | member-ticks | photosynthesis | living gain | net | earmark fill | births |
|---|---:|---:|---:|---:|---:|---:|---:|
| c = 0.1, D = 0.5 | 4 | 546 | 2.088 | 0.0090 | 0.710 | 0.72 | 38 |
| c = 0.1, D = 1.0 / 1.5 | 5 / 3 | 673 / 107 | 0.25 / 0.09 | 0.012 / 0.012 | 0.114 / 0.035 | 0.10 / 0.05 | 2 / 2 |
| c = 0.3, D = 0.5 / 1.0 / 1.5 | 3 / 2 / 3 | 103 / 1,092 / 86 | 0.10–0.30 | 0.004–0.051 | 0.058–0.199 | 0.06–0.19 | 0 / 3 / 5 |
| c = 1.0, D = 0.5 / 1.0 / 1.5 | 2 / 3 / 3 | 87 / 62 / 90 | 0.07–0.08 | 0.003–0.016 | 0.005 / 0.001 / −0.000 | ≤ 0.011 | 0 |
| c = 3.0, D = 0.5 / 1.0 / 1.5 | 2 / 5 / 3 | 654 / 812 / 880 | 0.06–0.11 | 0.001–0.010 | 0.038 / 0.001 / 0.001 | ≤ 0.046 | 0 |

(Medians over the seeds that have any. Net is income − upkeep.)

- **Their grazing income does not collapse.** At c ≥ 1 it is 0.001–0.016 against 0.009 at the
  default.
- **But they break even and do not reproduce.** In all six c ≥ 1 cells, net is within 0.04 E of zero
  per member-tick, the earmark is empty and there are no births. Income equals upkeep: they eat
  until the buffer is refilled, and then expression falls to 0.11–0.27.
- **This is weak evidence.** The default cell's consumers are a different kind of agent: light-fed
  mixotrophs, with photosynthesis 97 % of their income. Some c < 1 cells look similar (net 0.035–0.2,
  0–5 births). Only 2–5 seeds per cell carry any consumers.

## 5. The atlas

`role_diet_census` on the 95 atlas cells, 475 seeds. Each cell reads default (c = 0.1, D = 0.5) →
(c = 1, D = 1) → (c = 3, D = 1.5).

| | default | c = 1, D = 1 | c = 3, D = 1.5 |
|---|---:|---:|---:|
| verdicts: live / lockup / extinction / monoculture / generalist dominance | 416 / 46 / 7 / 1 / 5 | 418 / 52 / 3 / 1 / 1 | 421 / 44 / 3 / 4 / 3 |
| seeds holding a producer / consumer / decomposer guild, by diet | 250 / 0 / 0 | 270 / 0 / 0 | 285 / 0 / 0 |
| configs holding a heterotroph guild by diet on ≥ half their seeds | 0 | 0 | 0 |
| trait-producer deaths | 131,250 | 138,377 | 136,548 |
| infant producer deaths grazed by kin | 32,284 | 28,178 | 25,430 |
| kin-grazed share of trait-producer deaths | 25.7 % | 21.6 % | 19.6 % |
| heterotroph-by-diet deaths grazed by kin | 11,534 of 14,620 | 7,262 of 12,091 | 4,692 of 9,813 |
| light-fed mixotroph deaths / grazed by kin | 10,534 / 6,264 | 21,847 / 16,035 | 24,649 / 18,741 |
| kin kill pairs | 75,942 | 71,091 | 59,551 |
| non-kin kill pairs | 15,745 | 21,162 | 15,835 |
| kin killers under 1 tick | 31 % | 66 % | 92 % |
| tagged consumers that eat as consumers (diet agrees) | 1.3 % (177) | 14.3 % (2,325) | 0.9 % (142) |
| tagged-heterotroph samples with light share ≥ 0.9 | 96.8 % | 90.9 % | 96.6 % |

- **Kin-grazing falls by a fifth at most,** not by half. Non-kin killing does not collapse.
- **(1, 1) moves some tagged consumers onto a consumer diet.** That is 2,325 agent-samples against
  177 at the default. It forms no guild, and at (3, 1.5) it is gone again (142).
- **There are twice as many light-fed mixotrophs,** and kin graze them more. Their heterotrophy
  costs maintenance but now rarely fires, so more of them persist.
- **The producer guild by diet grows** (250 → 270 → 285 seeds).

## 6. Verdict and recommendation

**Verdict: both.**
- **Scale.** The default c = 0.1 puts the gate's half-expression point at 10 ticks, far above the
  1–5-tick buffer where agents live. Raising c to ≥ 1 with D ≥ 1 halves the steps' infant
  kin-grazing on `sample:31`, which the scale branch asked for.
- **Structure.** The same runs show how it gets there.
  - The killers do not become sated. Every agent at the buffer becomes "fed" in the gate's terms,
    and only starving agents kill.
  - The steps' whole heterotrophic income falls in proportion.
  - Nothing invades.
  - On the atlas, infant producer kin-grazing falls by 13–21 % and kin kill pairs by 6–22 %. No
    heterotroph guild appears.
  - The resident's consumers break even at c ≥ 1 and do not reproduce (weak evidence, §4).

  The gate reads a reserve that the grow phase holds at the same buffer for every agent. So no c
  separates a light-fed mixotroph, which should be sated, from an obligate consumer, which should
  stay hungry enough to grow. c only moves one cut-off for both.

**Recommendation for #607: a design change first, through grill-with-docs.** The question to settle
is what need-gating reads as satiation. The data offer two candidates:
- **The reproductive earmark (or the energy mobilised above the buffer) instead of the reserve.** A
  light-fed mixotroph banks surplus there. On `sample:31` the steps' earmark fill is 0.22–1.07 in
  every cell. A consumer that only breaks even does not: the resident's consumers sit at 0.00–0.05 at
  c ≥ 1. That is a contrast the reserve does not show.
- **Satiation measured against the retention buffer**, (reserve − `growth_retention_multiplier` ×
  maintenance) / maintenance, as the issue suggested. On `sample:31` the grow phase leaves only 18 %
  of the excess, so this reads near zero for everyone. Taken alone it would make every agent hungry.
  It is weaker than the earmark.

Either keeps one sensitivity. Once the gate reads such a quantity, c and D belong in #607's box.

**If #607 goes ahead first,** use these ranges:
- `satiation_sensitivity` ∈ [0.1, 3], log-scaled. The kin-grazing response saturates between 1
  and 3.
- `recognition_distance` ∈ [0.5, 1.2]. Keep it under the ~1.4 producer–consumer separation: D = 1.5
  already shelters the producer from the near-producer steps on `sample:31`.

Score consumer reproduction (heterotroph-by-diet births or earmark), not kin-grazing alone. The
search would otherwise find the c ≥ 1 region, where kin-grazing is low because every consumer is
gated at the buffer.

**The smallest measurement that would settle §4.** The evidence that obligate consumers stall at
c ≥ 1 is thin. To settle it, read the satiation and earmark fill of **all** heterotrophs by diet
(not only killers) per tick, at c = 0.1 and c = 1 on the atlas. It needs a census readout and one
atlas pass per c, about 2 min. If consumers at c = 1 sit at the buffer with an empty earmark, the
design change is confirmed as necessary. If they grow, a scale-only #607 is defensible.

## 7. What this does not show

- **One config and 8 seeds for invasion.** Many cells are decided by extinction ties at
  `r = −0.00277`. Cell-to-cell differences of one or two seeds are noise.
- **Different residents.** Every cell's resident evolved under its own (c, D). The step arms are
  read against their own cell's `step 0` and control, not against a common state.
- **The resident consumers are few** (§4).
- **Only two atlas cells, and no LHS draw.** The LHS pass (1.5–3 h per cell) was out of scope for
  this probe.
- **The restraint r = ½ is fixed.** The world rules tie it to the gate's half point. It was not
  varied.
- **The satiation bands are fixed in ticks.** The readout's "hungry (< 10)" column means under half
  expression only at c = 0.1. At c = 1 the half point is 1 tick, and at c = 3 it is ⅓ tick. Read
  the 0–1 band.

## 8. Instrument

- `crates/explorers-search/src/config_source.rs`:
  - `with_consumption_scales(config, Option<c>, Option<D>)` pins a resolved world's
    `satiation_sensitivity` and `recognition_distance`, and leaves everything else as decoded;
  - `parse_non_negative` accepts finite values ≥ 0 (0 is each mechanism's off limit).
- `reinvasion_barrier --satiation-sensitivity C --recognition-distance D`:
  - records each pin on the artifact, left out when unset;
  - `--merge` refuses chunks whose pins differ.
- `role_diet_census --satiation-sensitivity C --recognition-distance D` pins every world in `run_row`
  and records the pins on each row, left out when unset.
- Tests:
  - each pin overrides only its own parameter, and no pin changes nothing;
  - the flags' range is enforced;
  - `reinvasion_barrier`: CLI parsing, the artifact record and round trip, the merge guard, and the
    pin reaching the run;
  - `role_diet_census`: the pins reach the world, the row and the rollout, and an unpinned row
    serialises without them.
- Artifacts are under `target/619/`:
  - `rb-c{C}-d{D}-{plain,accounting,dispersal}.{json,md,err}`;
  - `rb-{plain,accounting,dispersal}-unset.*`;
  - `rd-atlas-{base,c1.0-d1.0,c3.0-d1.5}.jsonl` and their `-summary.md`;
  - `wallclock.txt`, plus `grid.sh` and `atlas.sh`, the loops that ran them.
