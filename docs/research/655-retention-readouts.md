# Issue #655: #647's niche tests under the corrected retention rule

**Status: measurement, docs only. Adds a verdict block and two readouts (G, conditionality; H,
niche share by role) to the observer-only diagnostic `kin_killer_diet`, and the uptake-scaling flags
to `role_diet_census` (commit `0675226`). `explorers-sim`, the search, the evaluator and the
prefilter are unchanged. The verdict here is provisional. The deciding read is #656, on a fresh
atlas with `b` in genesis's box.**

#647 settled that a light-fed mixotroph is a niche, not the producers' dominant mode, while it
passes two tests ([world rules](../system-design/world-rules.md), flow 1):

- **conditionality:** its heterotrophy falls where the pool nutrient under it is rich;
- **minority share:** it drains less than half of the carcass pile.

#647 also found that the drain capped a consumer's nutrient retention at its whole-body demand, so
large bodies kept most of a carcass bite. #652 fixed that: every drainer now keeps its ratio × the
energy a bite gains it, and the rest is mineralised. This note measures both tests under the fixed
rule, on the existing atlas, at `b = 0` and `b = 1`. It compares with #642 and #645, which ran
under the old rule, so the effect of the fix and the effect of `b` can be read apart.

## TL;DR

| setting | conditionality: pooled ρ / median per-config ρ | mixotroph share of carcass structure, whole run / second half | persisted (Δ vs #642) | nutrient lockup (Δ vs #642) |
|---|---|---|---|---|
| `b = 0`, decoded | **FAIL** +0.161 / +0.159 | **FAIL** 68.9 % / 69.8 % | 401 (−2) | 45 (+5) |
| `b = 0`, fa 0 | **FAIL** +0.258 / +0.252 | **FAIL** 69.0 % / 64.4 % | 421 (−2) | 38 (−3) |
| `b = 1`, decoded | **PASS** −0.147 / −0.074 | **FAIL** 63.3 % / 57.0 % | 415 (+12) | 8 (−32) |
| `b = 1`, fa 0 | **PASS** −0.106 / −0.064 | **FAIL** 67.7 % / 62.6 % | 429 (+6) | 5 (−36) |

(fa 0 = `founder_aggregation = 0`. `b = 1` runs at `s_ref = 100`. 475 seeds per setting.)

1. **The fix did what it was meant to do to retention** (§5). Light-fed mixotrophs now keep
   1.4–3.5 % of the carcass nutrient their bites release, against 29–48 % before. The carcass
   nutrient they retain falls by 95–96 %. Carcasses now supply 0.4–0.5 % of their nutrient,
   against 8.6–13 %.
2. **It did not move who drains the carcass pile** (§4). Light-fed mixotrophs drain 57–70 % of
   carcass structure under the new rule (whole run or second half), against 56–69 % under the
   old. At `b = 0` their share went up a few points. They still drain the pile for its energy.
3. **The minority share fails in all four settings** (§4). No setting is near 50 %. The closest is
   `b = 1` decoded in the second half, at 57 %.
4. **Conditionality follows `b`, not the fix** (§3). At `b = 0` it runs backwards, as in #642: the
   most heterotrophic producers sit on the richest pools. At `b = 1` it passes, weakly: the median
   per-config ρ is −0.07 and −0.06, and 37–38 configs still show a positive ρ. The bins' mean pools
   at `b = 1` were already in this order under the old rule (#645 §4).
5. **Persistence does not regress** (§6). Against the `b`-matched old runs, persisted seeds move by
   −2 to +3 of 475. Nutrient lockup rises by 5 seeds at `b = 0` decoded, which the strict rule
   counts as rising. The large fall in lockup at `b = 1` is #645's, not the fix's.
6. **The #637 `(k, τ)` region is non-empty only at `b = 1` decoded** (§7): 36 of 204 cells at
   n = 4, all at the grid's lowest `k` (≤ 0.432). The other three settings are empty, two of them
   by under two points.

**Provisional verdict** (§8): the minority share fails everywhere, so #647's mapping points at
**carcass access**. Conditionality fails at `b = 0` and passes at `b = 1`, so whether the
**cross-trait cost** is also needed depends on where genesis puts `b`. #656 decides both.

**#630/#631** (§9): stay deferred. Re-read the region on #656's atlas, and un-defer them only if it
is non-empty there both decoded and at fa 0.

## 1. What ran

| | |
|---|---|
| Tree | `0675226` (branch `issue-655-retention-readouts`: the instrument on top of `b85cd57`, which holds #652's fix and #653). Release build. |
| Diagnostic | `kin_killer_diet` on the atlas (`atlas.json`, 95 configs), seeds 1000–1004 (475 seeds), T = 2000, `EvalConfig::default()`, **ungated**, network off, as in #642/#645. |
| Settings | `b = 0` and `b = 1` (`s_ref = 100`), each decoded and at `--founder-aggregation 0`. |
| Region | `role_diet_census --fullness` on the same four settings, ungated (`--satiation-sensitivity 0`), then `--summary --fullness-region` on each run's rows. The grid and the bars are #637's. |
| Machine | 8-core laptop, one process at a time. |

**Commands** (cwd is the repo root; driver `target/655/run.sh`):

```sh
env CARGO_TARGET_DIR=target/agent cargo build --release -p explorers-search \
    --example kin_killer_diet --bin role_diet_census
C=$(seq -s, -f 'atlas:%g' 0 94)
B1="--uptake-structure-exponent 1 --uptake-reference-structure 100"
FA0="--founder-aggregation 0"

# per setting, e.g. b = 1, fa 0
target/agent/release/examples/kin_killer_diet --atlas atlas.json --configs $C $B1 $FA0 \
    --out target/655/b1-fa0.jsonl > target/655/b1-fa0.md
target/agent/release/role_diet_census --atlas atlas.json --configs $C $B1 $FA0 \
    --satiation-sensitivity 0 --fullness --out target/655/rd-b1-fa0.jsonl
target/agent/release/role_diet_census --summary --fullness-region \
    --out target/655/rd-b1-fa0.jsonl > target/655/rd-b1-fa0-region.md
```

**Wall clock** (`target/655/times.log`; seconds, decoded / fa 0): `kin_killer_diet` b0 32 / 61, b1
99 / 110; `role_diet_census --fullness` b0 28 / 58, b1 90 / 98; region reads under 1 s each. In
total 577 s, under 10 min.

**Checks.**

- Re-reading #645's b0 decoded rows with the new binary (`--summary`) reprints every one of its
  tables unchanged. Only the verdict block, G and H are added. G reads "n/a" on old rows, which carry
  no pool-at-cell samples.
- Re-reading #637's decoded rows with the new `role_diet_census` reproduces #637's closest cells
  exactly (n = 4: k 1.7, τ 17.2, sated 58.3 %, kept 73.4 %).
- b0 fa 0's closest region cell (sated 58.3 %, kept 73.8 %) matches #637's decoded sated share by
  coincidence. It is a different cell (τ = 1, not 17.2) on a different world: 36,163 mixotroph kin
  kills, against #637's 13,045 decoded and 33,983 at fa 0. The kill count matches this note's own
  `kin_killer_diet` b0 fa 0 run, which shares its seeds.

**Populations** are #642's. A **light-fed mixotroph** in G and H is a Producer-role agent (by
recent income) with effective heterotrophy `h_eff > 0.05`. A **pure producer** has `h_eff ≤ 0.05`.
Decomposer, consumer and "no income yet" are recent-income roles.

## 2. The pass rules, as the instrument applies them

- **Conditionality** (G) passes when **both** Spearman correlations of `h_eff` against the pool
  nutrient at the agent's cell are below zero, over second-half Producer-role agent-samples:
  - the **pooled** ρ, all configs and seeds in one ranking;
  - the **median per-config** ρ, over configs with at least 10 samples.

  Both are needed because the pooled ρ mixes worlds at different nutrient levels. A few dense,
  rich worlds could carry it alone.
- **Minority share** (H) passes when light-fed mixotrophs drain **under 50 %** of the attributed
  carcass structure, in the whole run **and** in the second half. Structure is the carcass energy
  drained (`Consumed.energy_delta` on carcass bites). No carcass structure went unattributed in any
  run.
- **No regression** (E) passes when persisted seeds are within 2 % of #642's 403 (decoded) and 423
  (fa 0) of 475, and nutrient-lockup seeds do not rise above #642's 40 and 41. The instrument
  reads "within 2 %" two-sided, so a gain above 2 % prints "no". This note reads a gain as no
  regression.

## 3. Conditionality (G)

| setting | pooled ρ | n | median per-config ρ | configs with ρ < 0 |
|---|---:|---:|---:|---:|
| `b = 0`, decoded | +0.161 | 202,453 | +0.159 | 32 of 94 |
| `b = 0`, fa 0 | +0.258 | 494,377 | +0.252 | 12 of 95 |
| `b = 1`, decoded | −0.147 | 875,944 | −0.074 | 57 of 94 |
| `b = 1`, fa 0 | −0.106 | 1,122,655 | −0.064 | 57 of 95 |

Mean pool nutrient at the cell by `h_eff` bin, new rule, with the old rule (#645) in brackets:

| `h_eff` bin | b0 decoded | b0 fa 0 | b1 decoded | b1 fa 0 |
|---|---:|---:|---:|---:|
| [0, 0.05) | 211 (206) | 125 (113) | 728 (724) | 648 (684) |
| [0.05, 0.2) | 155 (97) | 153 (152) | 624 (682) | 546 (477) |
| [0.2, 0.5) | 192 (161) | 174 (154) | 506 (482) | 498 (438) |
| [0.5, 1) | 240 (212) | 247 (247) | 471 (437) | 434 (412) |
| ≥ 1 | 303 (286) | 280 (274) | 394 (379) | 420 (358) |

- **At `b = 0` conditionality still runs backwards.** The pool under producers rises with their
  heterotrophy above 0.05, as in #642. The fix shifted every bin's pool up a little, and left the
  order alone. At fa 0, 83 of 95 configs show a positive ρ.
- **At `b = 1` it passes, but weakly.** The pooled ρ is about −0.1 to −0.15. The median config is
  near zero (−0.07, −0.06), and four configs in ten still run backwards. The order of the bins'
  pools was the same under the old rule, so the pass comes from `b`, not from the fix. The old rows
  carry no per-sample pool readings, so G cannot be computed on them.
- **Part of the pass may be mechanical.** At `b = 1` small bodies take up little, so nutrient sits
  under pure producers, which are mostly small (#645 §4). A rich pool under the low-`h_eff` bins is
  partly a size effect, not heterotrophy responding to nutrient. The most heterotrophic producers
  are still the most N-limited: 77 % at `h_eff ≥ 1` decoded, against 57 % for pure producers.
- **Appetite falls over the run at `b = 1`, and rises at `b = 0`,** as in #645. Light-fed
  mixotrophs' mean `h_eff` goes from 0.62 to 0.45 decoded and from 0.61 to 0.47 at fa 0, first
  quarter to last, at `b = 1`. At `b = 0` it goes from 0.61 to 0.68 and from 0.55 to 0.63.

## 4. Niche share (H)

Light-fed mixotrophs' share of carcass structure drained, old rule (#642/#645, from section C of
their rows) against new:

| setting | old: whole / second half | new: whole / second half | change, whole / second half |
|---|---|---|---|
| `b = 0`, decoded | 64.0 % / 62.9 % | 68.9 % / 69.8 % | +4.9 / +6.9 |
| `b = 0`, fa 0 | 66.1 % / 61.6 % | 69.0 % / 64.4 % | +2.9 / +2.8 |
| `b = 1`, decoded | 63.7 % / 56.5 % | 63.3 % / 57.0 % | −0.4 / +0.5 |
| `b = 1`, fa 0 | 69.2 % / 64.3 % | 67.7 % / 62.6 % | −1.5 / −1.7 |

Shares of carcass structure drained by every role, new rule, whole run:

| recipient | b0 decoded | b0 fa 0 | b1 decoded | b1 fa 0 |
|---|---:|---:|---:|---:|
| light-fed mixotroph | 68.9 % | 69.0 % | 63.3 % | 67.7 % |
| pure producer | 0.2 % | 0.3 % | 0.2 % | 0.7 % |
| consumer | 5.6 % | 5.2 % | 8.4 % | 7.3 % |
| decomposer | 19.2 % | 19.2 % | 23.0 % | 18.4 % |
| no income yet | 6.2 % | 6.3 % | 5.0 % | 6.0 % |

- **The fix alone does not move the share.** At `b = 0` it rose by 3–7 points. At `b = 1` it moved
  by under 2. Taking most of the nutrient out of a carcass bite did not stop producers draining
  carcasses. They drain them for energy: light-fed mixotrophs' carcass energy gained is about the
  same or higher under the new rule (b0 decoded 4,743 → 5,546).
- **`b` moves it a little.** Under the new rule, `b = 1` takes 5.6 / 12.8 points off the decoded
  share (whole run / second half), and 1.3 / 1.8 at fa 0. Decomposers gain most of it. That is still
  far from a minority.
- **Nutrient released follows structure.** Mixotrophs release 64–69 % of the carcass nutrient
  released, in every setting.
- **Prevalence did not change with the fix.** Producers with `h_eff > 0.05` are 34.9 % and 34.6 % of
  Producer samples at `b = 0` (34.0 %, 34.7 % under the old rule), and 14.6 % and 21.4 % at `b = 1`
  (15.1 %, 21.8 %).

## 5. Retention by role (the old niche metric)

#642's niche metric was the share of carcass nutrient **retained**. Under the new rule most carcass
nutrient is mineralised whoever drains it, so this no longer decides anything. It shows what the fix
did.

Fraction of the carcass nutrient a role's bites release that the role keeps, whole run:

| setting | light-fed mixotroph: old → new | decomposer: old → new | consumer: old → new |
|---|---|---|---|
| `b = 0`, decoded | 29.0 % → 1.4 % | 2.4 % → 0.9 % | 2.9 % → 0.7 % |
| `b = 0`, fa 0 | 36.3 % → 1.5 % | 2.2 % → 1.0 % | 1.7 % → 0.5 % |
| `b = 1`, decoded | 47.3 % → 3.5 % | 7.0 % → 4.1 % | 7.6 % → 3.4 % |
| `b = 1`, fa 0 | 48.3 % → 3.4 % | 6.3 % → 3.1 % | 5.9 % → 2.3 % |

Share of all carcass nutrient retained, whole run:

| setting | light-fed mixotroph: old → new | decomposer: old → new | carcass N retained by mixotrophs: old → new |
|---|---|---|---|
| `b = 0`, decoded | 96.1 % → 76.5 % | 2.2 % → 13.8 % | 101,540 → 4,846 |
| `b = 0`, fa 0 | 97.4 % → 78.2 % | 1.6 % → 13.5 % | 358,593 → 13,366 |
| `b = 1`, decoded | 94.7 % → 65.3 % | 3.2 % → 22.8 % | 73,567 → 3,667 |
| `b = 1`, fa 0 | 94.6 % → 73.8 % | 2.2 % → 15.8 % | 154,787 → 8,360 |

- **The fix cut large mixotrophs' retention sharply, as expected.** What they keep of what they
  release falls by a factor of 13–24. Their retained carcass nutrient falls by 95–96 %.
- **Everyone keeps less, but mixotrophs lose the most.** Decomposers and consumers lose 40–75 %
  of their kept fraction; mixotrophs lose over 90 %. So the retained shares
  converge on the structure shares: mixotrophs' retained share falls to 65–78 %, close to their
  63–69 % of structure. Decomposers' retained share rises from 2–3 % to 14–23 %.
- **Carcasses stop being a nutrient source for mixotrophs.** They supply 0.4–0.5 % of light-fed
  mixotrophs' nutrient acquired (section C), against 8.6–13 % before. Pool uptake supplies 99 %.

## 6. No regression (E)

| setting | persisted, new | #642 (b0) | old rule, same `b` (#645) | lockup, new | #642 | old, same `b` | other failures, new |
|---|---:|---:|---:|---:|---:|---:|---|
| `b = 0`, decoded | 401 | 403 | 403 | 45 | 40 | 40 | extinction 26, monoculture 3 |
| `b = 0`, fa 0 | 421 | 423 | 423 | 38 | 41 | 41 | extinction 10, monoculture 4, generalist dominance 1, energy death 1 |
| `b = 1`, decoded | 415 | 403 | 412 | 8 | 40 | 9 | extinction 22, monoculture 21, generalist dominance 6, energy death 3 |
| `b = 1`, fa 0 | 429 | 423 | 432 | 5 | 41 | 5 | monoculture 28, generalist dominance 9, extinction 4 |

- **Persistence holds.** Against #642, `b = 0` loses 2 seeds in each mode (−0.5 %). `b = 1` gains
  12 and 6. Against the old rule at the same `b`, the fix moves persistence by −2, −2, +3 and −3.
- **Lockup rises by 5 at `b = 0` decoded** (40 → 45), so the strict rule fails there. That is about
  one seed in 95 configs. At fa 0 it falls by 3. At `b = 1` it is 8 and 5, as under the old rule
  (9, 5): the fall from about 40 is `b`'s, found in #645.
- **Monoculture and generalist dominance at `b = 1`** (27 and 37 seeds) were already there under the
  old rule (25 and 35). The fix did not add them.
- **Kin-killing.** Per 1000 Producer agent-ticks, whole run: 2.58 / 2.70 at `b = 0` (old 2.14 /
  2.54) and 0.61 / 0.95 at `b = 1` (old 0.78 / 1.06), decoded / fa 0.

## 7. The #637 region, re-read

`(k, τ)` region for the fullness gate, ungated, n = 4, bars unchanged (sated ≥ 60 %, kept ≥ 75 %):

| setting | result | best or closest `(k, τ)` | sated / kept | margin | mixotroph kin kills / consumer ticks |
|---|---|---|---|---:|---|
| #637, old rule, `b = 0`, decoded | empty | 1.7, 17.2 | 58.3 % / 73.4 % | −0.017 | 13,045 / 2,584 |
| #637, old rule, `b = 0`, fa 0 | empty | 2.236, 2.04 | 52.6 % / 66.7 % | −0.083 | 33,983 / 7,988 |
| `b = 0`, decoded | empty | 1.7, 1 | 46.4 % / 59.4 % | −0.156 | 15,981 / 2,626 |
| `b = 0`, fa 0 | empty | 1.7, 1 | 58.3 % / 73.8 % | −0.017 | 36,163 / 6,395 |
| `b = 1`, decoded | **36 of 204 cells** | 0.25, 50 | 75.6 % / 89.7 % | +0.147 | 9,436 / 2,021 |
| `b = 1`, fa 0 | empty | 0.748, 50 | 59.3 % / 73.6 % | −0.014 | 21,446 / 5,842 |

n = 2 and n = 8 give the same picture (b1 decoded: 36 cells at each; the others empty, margins
within 0.02 of n = 4).

- **The fix alone moves the two `b = 0` settings in opposite directions.** fa 0 gets much closer
  (−0.083 → −0.017). Decoded gets much further (−0.017 → −0.156). The decoded loss is in kept: its
  floor, at the smallest `k`, falls from about 70 % to 48–49 %. The floor is the share of consumers'
  excess production that comes from their own light, which the gate never touches (#637 §2). So the
  heterotrophs by diet in this world are a different mix: 517 agents against 341. The diagnostic
  does not say why.
- **At `b = 1` decoded the region opens, at the edge of the grid.** The 36 feasible cells are every
  `τ` at `k` = 0.25, 0.329 and 0.432. The largest margin is at the lowest `k` and longest `τ` on the
  grid. Kept is 90 % across that corner, so sated binds: the gate must sit very low to sate three
  quarters of the killers.
- **At `b = 1` fa 0 kept binds.** Its floor is about 73 %, just under the bar, so no `k` reaches it.
  The miss is 1.4 points.
- **The energy side is not the obstacle at small `k`.** At `k ≤ 0.75`, 94–100 % of kills are sated
  on energy in every setting. The nutrient side and the kept floor decide the region, as in #637.

## 8. Provisional verdict

#647's mapping, read setting by setting:

| setting | conditionality | minority share | mapping says |
|---|---|---|---|
| `b = 0`, decoded | fails | fails | cross-trait cost; carcass access |
| `b = 0`, fa 0 | fails | fails | cross-trait cost; carcass access |
| `b = 1`, decoded | holds (weakly) | fails | carcass access |
| `b = 1`, fa 0 | holds (weakly) | fails | carcass access |

**Provisionally, the minority share fails, so the outcome is carcass access**: a design issue for an
autotrophy-linked digestion cost, or a universal living–dead asymmetry. This is the robust result.
It fails in every setting, by 7–20 points, and the retention fix did not move it. The fix was the
cheap lever, and it worked on the nutrient (§5) but not on who drains the pile (§4). Mixotrophs
drain carcasses for energy, and nothing in the current physics makes that expensive for a producer.

**Conditionality depends on `b`.** It fails at `b = 0` and passes at `b = 1`. The pass is weak
(median per-config ρ about −0.07), and part of it may be the size effect of §3. So:

- if #656's fresh atlas lands near `b = 0`, both levers are indicated;
- if it lands near `b = 1`, only carcass access is, and conditionality should be read again with the
  size effect in mind.

Neither design issue should open on this note alone. The atlas was searched under the old rule and
at `b = 0`, and the configs here are perturbed, not found. #656 is the deciding read. #647 stays open
for its owner's decision after #656.

## 9. #630/#631 (the fullness gate)

Recommendation: **keep both deferred.** The region is non-empty in one setting of four, and #637's
convention needs it at both decoded and fa 0. At `b = 1` fa 0 it misses by 1.4 points.

What it implies for un-deferring them:

- The region exists only where `b = 1`. Whether the gate has a default depends on `b`, which #653
  put in genesis's box. So #656's atlas is the place to read it.
- **Re-read the region on #656's atlas,** decoded and fa 0, with the bars unchanged. Un-defer #630
  only if it is non-empty in both. #631 follows #630.
- **Extend the `k` grid below 0.25 for that read.** At `b = 1` decoded the best cell is on the grid's
  lower edge, so the true optimum may lie outside it.
- Kin-killing per producer is a quarter of `b = 0`'s at `b = 1` decoded (0.61 against 2.58 per 1000
  agent-ticks). If #656 lands near `b = 1`, the gate has less to do, and #631's bar on kin-grazing
  should be read against that baseline.

## 10. What this does not show

- **The atlas was searched under the old retention rule and at `b = 0`.** Its 95 configs are worlds
  the search found viable under rules that no longer hold. Running them under the new rule, or at
  `b = 1`, measures a perturbation, not what genesis would find. That is why the verdict is
  provisional.
- **The world is ungated,** as in #642, #645 and #637. A gated world selects differently (#624).
- **It is correlational.** G ranks `h_eff` against the pool. It does not test whether a producer
  turns heterotrophy down when nutrient is added. Size, light and `h_eff` still move together, and
  the diagnostic records no structure.
- **The pooled ρ mixes worlds.** Configs at different nutrient levels share one ranking. The median
  per-config ρ guards against that, and at `b = 1` it is much weaker than the pooled one.
- **Five seeds per config.** The lockup rise at `b = 0` decoded (+5) and persistence changes of 2–3
  seeds are within the noise #645 saw (§8 there).
- **The old-rule comparison for G is by bins only.** Old rows carry no per-sample pool readings.
- **The region's kept floor moved for an unmeasured reason** (§7). The diagnostic shows that the
  heterotrophs by diet changed, not why.

## 11. Instrument

In `explorers-search`, commit `0675226`:

- **`examples/kin_killer_diet.rs`**:
  - **Verdict block** at the top of each report: conditionality and minority share pass / fail, and
    persisted and lockup seeds against #642's `b = 0` references (403 / 40 decoded, 423 / 41 fa 0).
  - **G, conditionality**: Spearman ρ (midranks) of Producer `h_eff` against the pool nutrient at the
    agent's cell before the step, second-half sample ticks; pooled, and per config with the median;
    the mean pool by B's `h_eff` bins.
  - **H, niche share**: carcass structure drained, nutrient released and nutrient retained by
    recipient role (C's buckets), whole run and second half, with the share `realised_bites` leaves
    unattributed.
  - Existing sections print unchanged, and pre-#655 rows read back (serde defaults). Unit tests on
    the aggregation run with the workspace (`test = true` on the example).
- **`role_diet_census`**: `--uptake-structure-exponent` and `--uptake-reference-structure`, recorded
  on the row, so the #637 region can be read at `b = 1`.
- **Artifacts** in `target/655/`: `{b0,b1}-{decoded,fa0}.{jsonl,md}`,
  `rd-{b0,b1}-{decoded,fa0}.jsonl`, `rd-*-region.md`, `run.sh`, `run.log`, `times.log`. `--summary`
  reprints a report from the rows.

## References

- [642-light-fed-mixotrophs.md](642-light-fed-mixotrophs.md): the populations, sections 1–3 and A–D, and the old-rule baselines.
- [645-size-scaled-uptake.md](645-size-scaled-uptake.md): `b`, the old-rule `b = 1` runs and section E.
- [637-fullness-region.md](637-fullness-region.md): the `(k, τ)` region, its bars and grid.
- [World rules](../system-design/world-rules.md): flow 1 (the two tests), flow 3 (the retention rule), the need gate, trade-off #5.
