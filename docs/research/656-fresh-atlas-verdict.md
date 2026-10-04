# Issue #656: #647's niche tests on a fresh atlas

**Status: measurement, docs only. Searches a fresh atlas under the corrected retention rule (#652)
with `b` in genesis's box (#653), and reads #647's tests on it with #655's instrument. The
instrument commit (`ce68a4a`) extends the fullness `k` grid down to about 0.048 and records an atlas
fingerprint on each row. `explorers-sim`, the search, the evaluator and the prefilter are
unchanged. The committed `atlas.json` is not replaced; §9 recommends whether it should be. This is
the deciding read for #647. The decision stays with #647's owner (§7).**

#647 holds a light-fed mixotroph to two tests ([world rules](../system-design/world-rules.md),
flow 1):

- **conditionality:** its heterotrophy falls where the pool nutrient under it is rich;
- **minority share:** it drains less than half of the carcass pile.

#655 read both tests on the existing atlas, which was searched under the retention defect and at
`b = 0`. Running those worlds at a new rule or a new `b` measures a perturbation. This note reads
the tests on worlds genesis finds under the corrected physics.

## TL;DR

| mode | conditionality: pooled ρ / median per-config ρ (configs ρ < 0) | mixotroph share of carcass structure, whole run / second half | persisted | lockup | `(k, τ)` region |
|---|---|---|---|---|---|
| decoded | **PASS** −0.134 / −0.060 (44 of 79) | **FAIL** 51.4 % / 45.8 % | 355 / 395 | 17 | empty, margin −0.220 |
| fa 0 | **FAIL** −0.048 / +0.051 (33 of 79) | **FAIL** 55.6 % / 48.4 % | 363 / 395 | 18 | empty, margin −0.078 |

(fa 0 = `founder_aggregation = 0`. Each config runs at its own decoded `b`. 79 configs × 5 seeds.)

1. **The fresh atlas sits at middling `b`** (§2). Across its 79 cells `b` runs from 0.10 to 0.96,
   with a median of 0.63 and half the cells between 0.50 and 0.76. No cell is below 0.1. Most of
   the clustering is search geometry: every dimension of the box clusters towards its centre.
   The upward shift and the missing low tail are probably weak selection away from `b ≈ 0`.
2. **The minority share sits at the bar** (§4). It fails on the whole run in both modes, by 1.4
   points decoded and 5.6 points at fa 0. It passes in the second half in both, at 45.8 % and
   48.4 %. On #655's old-atlas runs it was 57–70 %. Decomposers by role now drain 29–32 % of
   carcass structure over the whole run and 35–38 % in the second half, against 18–23 %. The
   per-config share is unrelated to `b` decoded, so the drop comes from the worlds found, not
   from `b`.
3. **Conditionality is absent within worlds** (§3). Decoded it passes by the rule, on a median
   per-config ρ of −0.06. At fa 0 it fails, on a median of +0.05. In both modes the share of
   configs with a negative ρ (44 and 33 of 79) is what a coin would give (two-sided binomial
   p ≈ 0.37 and 0.18). The pooled ρ is negative in both, but that mixes worlds. Higher-`b` configs lean a little
   more negative (Spearman of per-config ρ against `b`: −0.26 decoded, −0.21 at fa 0).
4. **Persistence does not regress** (§5). 89.9 % and 91.9 % of seeds persist, against 84–89 % at
   `b = 0` and 87–90 % at `b = 1` on the old atlas. Lockup is 4.3–4.6 % of seeds, between #655's
   `b = 0` (8–9.5 %) and `b = 1` (1–1.7 %). These are different atlases, so this compares rates,
   not paired seeds.
5. **The #637 region is empty in both modes, even with `k` down to 0.048** (§6). Kept binds.
   Its floor is 37 % decoded and 58–59 % at fa 0, and no `k` that sates 60 % of kin-killing
   mixotrophs leaves heterotrophs by diet 75 % of their production. At low `k`, sated is
   98–100 %, so the extended grid was not where the region was missing.

**Verdict, per #647's mapping** (§7): both tests fail on the strict reading, so the mapping
indicates **both levers**, carcass access and the cross-trait cost. Both failures are marginal.
The minority share fails by a few points on the whole run and passes once the run settles.
Conditionality is not refuted so much as not shown: no within-world relation in either mode. #647's
owner decides. §7 sets out what each decision would rest on.

**#630/#631** (§8): keep deferred, and consider retiring #630 in favour of the design's own fallback.
On the fresh atlas the fullness gate as specified cannot meet both bars, decoded or at fa 0.

**The atlas** (§9): replacing the committed `atlas.json` with this one is recommended, as its own
slice that re-pins `evaluator_pin` and `guild_anchor`. It was not done here.

## 1. What ran

| | |
|---|---|
| Search tree | `68e515b` (main: #652's retention fix, #653's `b` in the box over [0, 1] with `s_ref = 100`, #655's instrument). Release build. |
| Search | CMA-MAE, `--batch 32 --generations 10 --ensemble 5 --max-ticks 2000 --seed 42`, the same settings as #494's atlas. Defaults otherwise: bloom stop at tick 300 (10× founders), refinement of the top 10 at n = 32. |
| Readout tree | `ce68a4a` (branch `issue-656-fresh-atlas`): #655's instrument with the `k` grid extended and an atlas fingerprint on each row. The search code is unchanged from `68e515b`. |
| Diagnostics | `kin_killer_diet` and `role_diet_census --fullness` (ungated, `--satiation-sensitivity 0`) on all 79 cells, seeds 1000–1004 (395 seeds), T = 2000, `EvalConfig::default()`, network off. Decoded and at `--founder-aggregation 0`. No `--uptake-*` flags: the atlas records its 33-dimension box, so each config decodes with its own `b` and `s_ref`. |
| Region | `role_diet_census --summary --fullness-region` on each run's rows. Bars and `τ` grid as #637. The `k` grid is #637's 17 points over [0.25, 20] plus 6 points below, at the same log step, down to 0.048. That gives 23 × 12 = 276 cells per n. |
| Atlas fingerprint | `9c79856550a0151e` (on every `kin_killer_diet` row). |
| Machine | 8-core laptop, one process at a time. |

**Commands** (cwd is the repo root; the readout driver is `target/656/readouts.sh`):

```sh
# search, on 68e515b
env CARGO_TARGET_DIR=target/agent cargo build --release -p explorers-search
target/656/explorers-search --batch 32 --generations 10 --ensemble 5 --max-ticks 2000 \
    --seed 42 --output target/656/atlas.json --recipe-output target/656/recipe.json \
    --checkpoint target/656/search.ckpt

# readouts, on ce68a4a (readouts.sh builds once and copies the bins to target/656/bin)
C=$(seq -s, -f 'atlas:%g' 0 78)
FA0="--founder-aggregation 0"
target/656/bin/kin_killer_diet --atlas target/656/atlas.json --configs $C [$FA0] \
    --out target/656/kkd-{decoded,fa0}.jsonl > target/656/kkd-{decoded,fa0}.md
target/656/bin/role_diet_census --atlas target/656/atlas.json --configs $C [$FA0] \
    --satiation-sensitivity 0 --fullness --out target/656/rd-{decoded,fa0}.jsonl
target/656/bin/role_diet_census --summary --fullness-region \
    --out target/656/rd-{decoded,fa0}.jsonl > target/656/rd-{decoded,fa0}-region.md
```

**Wall clock.** Search 766 s (11 generations, 10 min 57 s, plus refinement). Readouts
(`target/656/times.log`): `kin_killer_diet` 151 s decoded, 140 s fa 0; `role_diet_census
--fullness` 141 s and 125 s; region reads under 1 s. In total 557 s.

**Checks.**

- `role_diet_census`'s rows do not record `b`, so the region could have run on different worlds
  from `kin_killer_diet`. It did not. Its kin-kill counts match `kin_killer_diet`'s exactly:
  18,585 decoded and 33,343 at fa 0. Its region header reads "as decoded, each config its own `b`".
- Every carcass bite is attributed (H's unattributed structure is 0.0 in both modes).
- B's Producer sample count equals A's in both modes (the report's own check).

Populations and pass rules are #655's (§1–2 there). A **light-fed mixotroph** is a Producer-role
agent with effective heterotrophy `h_eff > 0.05`. Conditionality passes when the pooled ρ and the
median per-config ρ of `h_eff` against the pool nutrient at the cell are both below zero. The
minority share passes when light-fed mixotrophs drain under 50 % of carcass structure on the whole
run **and** in the second half.

## 2. The atlas and `b`

| | #494's atlas (committed) | this atlas |
|---|---|---|
| tree | `3465c6d` | `68e515b` |
| retention rule | whole-body demand (defect) | ratio × energy gained (#652) |
| `b` | 0 (not in the box) | searched over [0, 1] |
| box recorded | no | yes, 33 dimensions |
| live cells | 95 | 79 |
| dead configs (of 352) | 68: lockup 52, extinction 9, monoculture 3, energy death 2, generalist dominance 2 | 54: lockup 35, bloom stop 6, monoculture 6, extinction 5, energy death 2 |
| best fitness / QD-score | 0.572 / 36.40 | 0.571 / 18.54 |
| cells with a decomposer or consumer guild | 0 (4 with a decomposer guild on 1 seed of 5) | 0 (none on any seed) |

Fitness is not comparable across the two. The evaluator has changed since `3465c6d`: the trophic
balance left fitness in #602. Nutrient lockup is still the main cliff, at 35 of 54 dead configs
(52 of 68 before). Refinement at n = 32 keeps 9 of the top 10 cells above the plain coexistence
floor. Every top-10 cell reads 0.00 under the guild-aware floors, as in #494.

**`b` across the 79 cells** (`target/656/b-distribution.md`):

| min | p25 | median | p75 | max | mean |
|---:|---:|---:|---:|---:|---:|
| 0.101 | 0.502 | 0.627 | 0.759 | 0.962 | 0.622 |

| `b` in | [0, 0.1) | [0.1, 0.25) | [0.25, 0.5) | [0.5, 0.75) | [0.75, 0.9) | [0.9, 1] |
|---|---:|---:|---:|---:|---:|---:|
| cells | 0 | 2 | 15 | 41 | 19 | 2 |

**How much of that is selection.** CMA-MAE starts from the centre of the box (unit 0.5, which is
`b = 0.5`), so clustering at mid-range may be search geometry. The other 32 dimensions show the
same pull. Across all 33, the median standard deviation of the unit coordinate is 0.238, against
0.289 for a uniform spread. In the median dimension, 68 % of cells sit within 0.25 of the centre;
for `b` it is 71 %. So **the clustering itself is geometry, not selection.** Two things about `b`
are less ordinary:

- **It is shifted up.** Its mean unit coordinate is 0.12 above the centre. That is the third-largest
  shift of the 33, after `mean_kappa` (−0.15) and `solar_flux_magnitude` (+0.14).
- **It has no low tail.** 28 of the 33 dimensions reach below 0.1, but `b` stops at 0.101. Its
  spread is the fifth narrowest.

`b` also correlates weakly with cell fitness (Spearman +0.24, n = 79, p ≈ 0.03). Together these point
to weak selection away from `b ≈ 0` and towards the upper middle. They do not point to selection
towards `b = 1`. With one search seed and 10 generations, this cannot be told apart from where the
emitters happened to wander. The safe reading is that **genesis's worlds sit at middling `b`**, and
neither of #655's endpoints is the right proxy for them.

## 3. Conditionality (G)

| setting | pooled ρ | n | median per-config ρ | configs with ρ < 0 |
|---|---:|---:|---:|---:|
| #655, `b = 0`, decoded | +0.161 | 202,453 | +0.159 | 32 of 94 |
| #655, `b = 0`, fa 0 | +0.258 | 494,377 | +0.252 | 12 of 95 |
| #655, `b = 1`, decoded | −0.147 | 875,944 | −0.074 | 57 of 94 |
| #655, `b = 1`, fa 0 | −0.106 | 1,122,655 | −0.064 | 57 of 95 |
| **fresh, decoded** | **−0.134** | 405,024 | **−0.060** | **44 of 79** |
| **fresh, fa 0** | **−0.048** | 703,608 | **+0.051** | **33 of 79** |

Mean pool nutrient at the cell by `h_eff` bin:

| `h_eff` bin | fresh decoded | fresh fa 0 | #655 b0 decoded | #655 b1 decoded |
|---|---:|---:|---:|---:|
| [0, 0.05) | 703 | 512 | 211 | 728 |
| [0.05, 0.2) | 593 | 434 | 155 | 624 |
| [0.2, 0.5) | 452 | 426 | 192 | 506 |
| [0.5, 1) | 544 | 473 | 240 | 471 |
| ≥ 1 | 532 | 467 | 303 | 394 |

- **The pass decoded is a weak one, and fa 0 fails.** The median config is within 0.06 of zero in
  both modes, on opposite sides of it. Of 79 configs, 44 (decoded) and 33 (fa 0) have a negative ρ.
  Neither split can be told from a coin's. **On these worlds there is no measurable within-world
  relation between a producer's heterotrophy and the nutrient under it.** The pooled ρ is negative
  in both modes. It ranks worlds at different nutrient levels together, which is why the rule also
  asks for the median.
- **The bins are not monotone.** The pool falls from pure producers to `h_eff` 0.2–0.5, then rises
  again for the most heterotrophic producers. The top two bins sit on richer ground than the
  middle one in both modes. That is the pattern #642 found at `b = 0`, now only at the top end.
- **The size effect #655 flagged is still there.** Pure producers sit on the richest pools.
  Under size-scaled uptake small bodies take up little, so a rich pool under the low bins is partly
  a body-size effect. The most heterotrophic producers are still the most nutrient-limited: 65 %
  (decoded) and 60 % (fa 0) at `h_eff ≥ 1`, against 39 % and 35 % for pure producers.
- **`b` helps a little within this atlas.** Per-config ρ falls with the config's `b` (Spearman
  −0.26 decoded, −0.21 at fa 0). Decoded, the 25 configs with `b ≥ 0.7` have a median ρ of −0.18
  (18 negative). At fa 0 they have a median of +0.01. This is the direction #655 found, and it is
  weak.
- **Appetite falls over the run**, as at `b = 1` in #655. Light-fed mixotrophs' mean `h_eff` goes
  from 0.55 to 0.49 decoded and from 0.50 to 0.45 at fa 0, from the first quarter to the last.

## 4. Minority share (H)

Light-fed mixotrophs' share of carcass structure drained:

| setting | whole run | second half |
|---|---:|---:|
| #655, `b = 0`, decoded | 68.9 % | 69.8 % |
| #655, `b = 0`, fa 0 | 69.0 % | 64.4 % |
| #655, `b = 1`, decoded | 63.3 % | 57.0 % |
| #655, `b = 1`, fa 0 | 67.7 % | 62.6 % |
| **fresh, decoded** | **51.4 %** | **45.8 %** |
| **fresh, fa 0** | **55.6 %** | **48.4 %** |

Shares of carcass structure drained by every role (whole run / second half):

| recipient | fresh decoded | fresh fa 0 | #655 range, whole run |
|---|---|---|---|
| light-fed mixotroph | 51.4 / 45.8 | 55.6 / 48.4 | 63–69 |
| pure producer | 0.2 / 0.2 | 0.3 / 0.4 | 0.2–0.7 |
| consumer | 8.0 / 5.7 | 7.1 / 5.0 | 5–8 |
| decomposer | 31.6 / 38.0 | 28.5 / 35.1 | 18–23 |
| no income yet | 8.8 / 10.3 | 8.6 / 11.1 | 5–6 |

- **The share fell by 12–18 points against the old atlas and lands at the bar.** The whole run
  fails by 1.4 points decoded and 5.6 at fa 0. The second half passes by 4.2 and 1.6. Under #655's
  rule the test fails, because it asks for both.
- **The pile shifts to decomposers as the run settles.** Their share rises by 6–7 points from the
  whole run to the second half in both modes. The mixotrophs' share falls by 6–7 points.
- **Per config the picture is the same.** The median config's mixotroph share is 53 % decoded and
  60 % at fa 0. 36 and 21 of 79 configs are below 50 %. The five configs that drain the most carcass
  structure hold about a quarter of it, so no single world carries the result.
- **It is the worlds, not `b`.** Per-config share does not follow `b` decoded (Spearman +0.02). At
  fa 0 it falls a little with `b` (−0.19): 61 % pooled for `b < 0.5`, against 50 % for `b ≥ 0.7`.
  #655 found that `b` alone moved the share by 1–13 points on the old worlds. The fresh worlds
  give a lower share across the whole `b` range.
- **Nutrient follows structure, slightly higher.** Mixotrophs release 53.7 % and 56.8 % of carcass
  nutrient released (whole run). They keep 2.3 % and 2.7 % of what their bites release,
  decomposers 2.3 % and 2.2 %. Carcasses supply 0.6–0.7 % of mixotrophs' nutrient, and pool uptake
  supplies 99 %. The retention fix holds on the new worlds as on the old (#655 §5).
- **Mixotrophy is less common than at `b = 0` and more than at `b = 1`.** Producers with
  `h_eff > 0.05` are 24.3 % and 25.8 % of Producer samples, against 35 % at `b = 0` and 15–21 % at
  `b = 1` on the old atlas. Kin kills per 1000 Producer agent-ticks are 1.92 and 2.00, against
  2.6–2.7 at `b = 0` and 0.6–1.0 at `b = 1`.

## 5. Persistence (E)

| setting | persisted | lockup | other failures |
|---|---:|---:|---|
| #655, `b = 0`, decoded / fa 0 (of 475) | 401 (84.4 %) / 421 (88.6 %) | 45 (9.5 %) / 38 (8.0 %) | |
| #655, `b = 1`, decoded / fa 0 (of 475) | 415 (87.4 %) / 429 (90.3 %) | 8 (1.7 %) / 5 (1.1 %) | |
| **fresh, decoded** (of 395) | **355 (89.9 %)** | **17 (4.3 %)** | extinction 17, monoculture 5, generalist dominance 1 |
| **fresh, fa 0** (of 395) | **363 (91.9 %)** | **18 (4.6 %)** | monoculture 6, extinction 6, generalist dominance 2 |

The instrument prints "no reference" because #642's seed counts belong to the old atlas. There is no
paired baseline on these worlds, so "does not regress" is read as rates. At fa 0, 91.9 % persist,
above all four of #655's settings. Decoded, 89.9 % persist, above three of them and 0.4 points
under `b = 1` at fa 0. Lockup sits between #655's `b = 0` and `b = 1` rates. One
config, `atlas:54` (`b = 0.85`), carries 4 of the 17 decoded lockup seeds. Nothing here suggests
the corrected physics costs persistence on worlds found under it.

## 6. The #637 region

Ungated, bars unchanged (sated ≥ 60 %, kept ≥ 75 %). The grid has 23 `k` values, from 0.048 to 20,
and 12 `τ` values, from 1 to 50:

| setting | result | closest `(k, τ)` | sated (energy side alone) / kept | margin | kin kills / consumer ticks |
|---|---|---|---|---:|---|
| #655, `b = 1`, decoded | 36 of 204 cells | 0.25, 50 (best) | 75.6 % / 89.7 % | +0.147 | 9,436 / 2,021 |
| #655, `b = 1`, fa 0 | empty | 0.748, 50 | 59.3 % / 73.6 % | −0.014 | 21,446 / 5,842 |
| **fresh, decoded** | **empty** | 2.236, 1 | 45.8 % (80.0 %) / 53.0 % | **−0.220** | 18,585 / 5,752 |
| **fresh, fa 0** | **empty** | 2.236, 4.148 | 52.2 % (86.4 %) / 67.5 % | **−0.078** | 33,343 / 11,558 |

n = 2 and n = 8 give the same picture: decoded −0.221 and −0.204, fa 0 −0.078 and −0.075.

- **Kept binds, not sated.** At the grid's lowest `k`, 98–100 % of mixotroph kin kills are sated.
  But kept falls to its floor: 37 % decoded and 58–59 % at fa 0. The floor is the share of
  heterotrophs-by-diet's excess production that comes from their own light, which the gate never
  touches (#637 §2). On #655's `b = 1` decoded run it was about 90 %, and at `b = 1` fa 0 about
  73 %. The heterotrophs by diet on the fresh worlds live far more on their drains.
- **No `k` sates the killers without starving the consumers.** Sated reaches 60 % only at
  `k ≤ 1.7`. There kept is at most 43 % decoded and 65 % at fa 0. Kept reaches 75 % only at
  `k ≥ 5.1` decoded and `k ≥ 2.9` at fa 0, where sated is at most 12 % and 36 %. The two populations' intakes overlap. Kin-killing
  mixotrophs have a median lifetime intake of 3.4 and 3.8 × maintenance, with a p25 of 1.6 and 1.9.
  Heterotrophs by diet have a median of 1.0 and 0.8, with a p75 of 1.6 and 1.4.
- **The extended `k` grid settles #655's open edge.** Below 0.25, sated rises further and kept
  stays flat at its floor, so the region was not hiding below the old grid.
- **Counting only energy-side satiation would not rescue it decoded.** Read off the rounded n = 4
  tables, a bar on energy-side satiation alone still finds no cell decoded. The nearest is about
  `k` 3.9, `τ` 1, at 60 % energy-sated and 71 % kept. At fa 0 a few cells near `k` 3.9 and
  `τ` 1–4 would clear it. That is a different bar, and choosing it is a design decision.

## 7. Verdict on #647

#647's mapping, read on the fresh atlas:

| mode | conditionality | minority share | mapping says |
|---|---|---|---|
| decoded | holds (weakly: median ρ −0.06, 44 of 79 configs negative) | fails on the whole run by 1.4 points; holds in the second half | carcass access |
| fa 0 | fails (median ρ +0.05, 33 of 79 negative) | fails on the whole run by 5.6 points; holds in the second half | cross-trait cost; carcass access |

**Strictly read, both levers are indicated.** The minority share fails in both modes, which points
to carcass access. Conditionality fails at fa 0, which points to the cross-trait cost. Persistence
does not regress.

**Against #655's provisional verdict.** #655 called carcass access the robust result: the share
failed everywhere, by 7–20 points. On the fresh atlas that failure has shrunk to a few points on the
whole run and vanished in the second half. Conditionality was expected to depend on `b`, and the
atlas landed between #655's endpoints. Within worlds it is neither backwards, as at `b = 0`, nor
present, as weakly at `b = 1`.

**How marginal each failure is.**

- **The minority share** is at the bar. It is 51 % and 56 % on the whole run, 46 % and 48 % in the
  second half, and falling as the run settles. The per-config median is 53 % and 60 %. Five seeds
  per config and one search seed could move it by a few points either way.
- **Conditionality** is a null result, not a reversal. The rule passes decoded and fails at fa 0 on
  medians within ±0.06 of zero. The more telling fact is that in neither mode do more configs show
  the relation than chance would. The clause's warrant (#647 decision 3) is the relation being
  present, and on these worlds it is not shown.

**What the owner's decision could look like.** Three readings are defensible on these numbers:

1. **Open both design issues** (the strict reading). The mapping's rule is "fails" and both tests
   fail in at least one mode. This is the reading that needs no judgement call.
2. **Open the cross-trait cost issue; keep carcass access in reserve.** Conditionality is the
   weaker of the two results. It is null in both modes, it is the clause's warrant, and `b` did
   not supply it. The minority share is within noise of passing and passes once settled. This
   reading takes the second half as the operative measure for the share. The test as written asks
   for the whole run too, so this changes how the test is read and should be recorded.
3. **Close #647 with both levers in reserve.** This is defensible only if the owner accepts both
   of the following: (a) the second-half share is the operative measure, as in 2; (b) a null
   conditionality is enough to keep the carnivorous-plant clause, that is, the warrant needs the
   relation not to run backwards rather than to be present. (b) is a weaker standard than flow 1
   states ("falling where the pool under it is rich"), so the design text would need to change to
   match.

This note does not choose. Whichever is chosen, the design docs' *Measured* notes (updated with this
PR) give the numbers the issue would cite.

## 8. #630/#631 (the fullness gate)

**Recommendation: keep both deferred, and consider closing #630 in favour of the design's own
fallback.** #637's convention un-defers #630 only if the region is non-empty both decoded and at
fa 0. On the fresh atlas it is empty in both. The decoded run is further from the bars (−0.22) than
any run before it, and the extended grid shows the region is not hiding at small `k`.

What it implies: **on worlds genesis finds under the corrected physics, the fullness gate as
specified cannot meet both bars.** It cannot sate kin-killing mixotrophs without taking most of the
heterotrophs' production, because their intakes overlap on the maintenance yardstick. #655's
non-empty region at `b = 1` decoded was a property of the old worlds pushed to `b = 1`.

The options, for the gate's owner:

- **Take the fallback the design already names** ([world rules](../system-design/world-rules.md),
  *Values to be measured*): stop asking satiation to sate mixotrophs, and leave their kin-killing to
  recognition and dispersal. #630 would close or be rescoped. #631 measures #630's gate, so it
  would close with it. Kin-killing is lower on these worlds than at `b = 0` on the old atlas
  (1.9–2.0 against 2.6–2.7 per 1000 Producer agent-ticks), which leaves the fallback less to
  handle.
- **Change the bar.** Counting a killer as sated when it is sated on energy alone, the
  carnivorous-plant reading, opens a small region at fa 0 but not decoded (§6). That is a design
  change, and it leans on conditionality, which §3 does not show.
- **Give the gate something to tell the populations apart by.** Any such discriminator is new
  design, and it belongs with #647's levers, not with #630.

## 9. The atlas

**Reproduce:** check out `68e515b`, build `explorers-search` in release, and run the command in §1
(`--seed 42`). The result is `target/656/atlas.json`: 79 cells, a 33-dimension box with
`uptake_structure_exponent` over [0, 1] as its last dimension, provenance seed 42, `max_ticks` 2000,
bloom stop at tick 300 × 10. Its fingerprint is `9c79856550a0151e`. Rollouts skipped: 0. The
search is seeded, but the rollout budget (600 s simulation and 600 s evaluation) is wall-clock.
A rerun on a slower machine that hits the budget would differ. None did here.

**Recommendation: replace the committed `atlas.json` and `recipe.json` with this search's, in a
dedicated slice.** The case:

- The committed atlas was searched under a retention rule that no longer holds, and at a `b` the
  box no longer fixes. Every readout on it since #652 measures a perturbation (#655 §10).
- It predates the recorded search box, so it cannot decode `b`. This one records its box.
- The slice must re-pin `evaluator_pin` (`crates/explorers-search/tests/evaluator_pin.rs`, cells
  re-chosen for the same spread of verdicts) and `guild_anchor`
  (`crates/explorers-genesis/tests/guild_anchor.rs`, the new recipe), as #545 did for #494.

Before replacing, the slice could consider whether one search at 10 generations is enough to
become the baseline. Coverage is 79 of 8000 cells, and the QD-score is half the old one's, though
fitness is not comparable across evaluators (§2). A second search seed would show how much of §2–§6
is this search's particular draw.

## 10. What this does not show

- **One search seed, 10 generations.** The atlas is one draw of what genesis finds. The `b`
  distribution and the readouts could shift with another seed or a longer search.
- **The world is ungated,** as in #637, #642, #645 and #655. A gated world selects differently.
- **It is correlational.** G ranks `h_eff` against the pool. It does not test whether heterotrophy
  responds when nutrient is added. Body size, light and `h_eff` still move together.
- **Five seeds per config.** The minority share's misses (1.4 and 5.6 points) and the persistence
  rates are within the range seed noise has moved similar readouts before (#645 §8).
- **The atlas differs in size and content from the old one** (79 cells against 95). The comparison
  with #655 is across atlases, not paired. "No regression" is read as rates, with no reference
  counts on these worlds.
- **The second-half reading is not the rule.** §7 reports it because the share falls through the
  run. The test as #655 implemented it asks for both halves.

## References

- [655-retention-readouts.md](655-retention-readouts.md): the instrument, the pass rules and the provisional verdict on the old atlas.
- [645-size-scaled-uptake.md](645-size-scaled-uptake.md): `b` and its effect on the old atlas.
- [637-fullness-region.md](637-fullness-region.md): the `(k, τ)` region, its bars and grid.
- [494-guild-atlas.md](494-guild-atlas.md): how the committed atlas was searched and recorded.
- [World rules](../system-design/world-rules.md): flow 1 (the two tests), flow 3 (the retention rule), the need gate, trade-off #5.
