# Issue #736 / #743: the ECM partnership read on the committed atlas by the fixed rule

**Status: measurement, docs only (2026-10-09). Routed surplus (#738) and farthest-first formation
(#739) were switched on through the network at #646's six settings, over the committed atlas
(#719's, 78 cells). Each setting was read against the network off by the rule posted on #736
before the run. Binaries are pinned at main df135e9 under `target/736/bin`, and the driver is
`target/736/run.sh`.**

## TL;DR

**No setting passes all four items. Items 3 and 4 fail in every setting, so by the rule #736 is
re-deferred.** The literature's preconditions hold, and the trade runs. The pairs do not form,
and the network costs robustness.

- **Item 1 holds, and it held before any trade.** Producers by role are more nutrient-limited, and
  decomposers by role more energy-limited, in 76–92 % of worlds carrying both. With the network
  off it is 90.3 % (decoded) and 92.2 % (fa 0). That is the complementary limitation of Daufresne
  & Loreau (2001), with no exchange needed to produce it.
- **Item 2 holds: routed surplus makes the trade run.** Net nutrient flows from decomposers to
  producers in 67.5–90.8 % of worlds that form producer–decomposer links. #646 measured ≈ 0. The
  gradient flow still runs the other way in every arm, and routed surplus outweighs it 1.5–5.9
  times.
- **Item 3 fails: pairs form below chance.** Producer–decomposer links are *below* pairing by
  headcount in 63–73 of 78 worlds in every arm (one-sided p = 1.0000). The median world forms them
  at 0.57–0.69 × the headcount share. Only 2.0–3.1 % of producers with spare connection capacity
  have a decomposer within reach.
- **Item 4 fails: the network costs robustness.** The per-seed failure rate rises from 118 of 780
  seeds to 209–333 (z = 5.7–12.0). Seed agreement falls from 32 of 78 cells to 9–14 (z = 3.2–4.2).
  The added failures are mostly monoculture (29 → 86–246 seeds), plus nutrient lockup in the
  low-rate settings.

| setting | 1. complementary limitation | 2. trade runs | 3. pairs > headcount | 4. robustness guard | all four |
|---|---|---|---|---|---|
| c1-low | pass / pass | pass / pass | **fail** / **fail** | **fail** | no |
| c1-mod | pass / pass | pass / pass | **fail** / **fail** | **fail** | no |
| c2-test | pass / pass | pass / pass | **fail** / **fail** | **fail** | no |
| c4-low | pass / pass | pass / pass | **fail** / **fail** | **fail** | no |
| c4-lowfast | pass / pass | pass / pass | **fail** / **fail** | **fail** | no |
| c4-mod | pass / pass | pass / pass | **fail** / **fail** | **fail** | no |

(Census items are given as decoded / fa 0. The audit runs decoded.)

**Verdict: fails on items 3 and 4, not on 3 alone. That falls under "other fails", so #736 is
re-deferred with this reading recorded.** The network stays default-disabled, and switching it on
as physics is not proposed.

## 1. What ran

The settings are #646's six, with transfer efficiency 0.9 in all:

| setting | cap | creation cost | maintenance cost | redistribution rate |
|---|---:|---:|---:|---:|
| c1-low | 1 | 0.1 | 0.01 | 0.1 |
| c1-mod | 1 | 1.0 | 0.1 | 0.3 |
| c2-test | 2 | 0.2 | 0.05 | 0.4 |
| c4-low | 4 | 0.1 | 0.01 | 0.1 |
| c4-lowfast | 4 | 0.1 | 0.01 | 0.3 |
| c4-mod | 4 | 1.0 | 0.1 | 0.3 |

| arm | instrument | settings | wall-clock (decoded / fa 0) |
|---|---|---|---|
| census, network off | `kin_killer_diet` | seeds 1000–1004, T = 2000, all 78 cells | 1111 s / 990 s |
| census, c1-low | same, `--network-*` | same | 1464 s / 1270 s |
| census, c1-mod | | | 1421 s / 1429 s |
| census, c2-test | | | 2009 s / 1728 s |
| census, c4-low | | | 2413 s / 2076 s |
| census, c4-lowfast | | | 3383 s / 2662 s |
| census, c4-mod | | | 945 s / 1683 s |
| audit, each on setting | `fragility_audit --draws 0 --network-*` | seeds 1000–1009, T = 2000, unperturbed only | 1372, 1409, 1974, 2451, 2862, 1712 s (same order) |
| audit, network off | #719's rows (`target/719/fragility/rows.jsonl`, radius 0) | as #719 ran it | – |

Three lanes ran in parallel on 8 cores (decoded census, fa 0 census, audit), taking 3¼–3½ h each,
so the per-arm times are under contention. The network-off census reproduces #654's off arms on
all 78 cells in both modes, with identical outcomes and kin kills: the network is bit-identical
when off, as #738 and #739 pin. The guard's off-arm numbers reproduce #719's and #654's (118 of
780 failing seeds, 32 of 78 cells agreeing). Run on #654's switch-on audit, the guard script gives
#654's published z = −0.14 and 1.16.

The census defines the readouts: section O (#740) for item 1 and section P (#742) for items 2 and
3. A **world** is one config, pooled over its five seeds, and everything is read over the settled
half (tick ≥ T/2 + 1). Roles are the sample's recent-income role.

## 2. Item 1: complementary limitation

Each settled-half growth event is classed by Liebig's law, by replaying the grow phase. It is
nutrient-limited if growth bound the whole free store, and energy-limited otherwise. A world passes
when producers by role have the larger nutrient-limited share and decomposers by role the larger
energy-limited share. The two conditions are complements, so they are one check.

| arm | decoded: worlds passing / carrying both | fa 0 |
|---|---:|---:|
| network off (reported only) | 65 / 72 (90.3 %) | 71 / 77 (92.2 %) |
| c1-low | 59 / 76 (77.6 %) | 70 / 78 (89.7 %) |
| c1-mod | 62 / 76 (81.6 %) | 65 / 76 (85.5 %) |
| c2-test | 62 / 75 (82.7 %) | 64 / 78 (82.1 %) |
| c4-low | 56 / 74 (75.7 %) | 60 / 78 (76.9 %) |
| c4-lowfast | 59 / 74 (79.7 %) | 58 / 76 (76.3 %) |
| c4-mod | 60 / 76 (78.9 %) | 59 / 75 (78.7 %) |

The item holds in every arm. It is strongest with the network off. That matches the literature's
order of causes: complementary limitation is the precondition for the trade, not its product. The
network brings more worlds to carry both roles (up to 78 of 78), and slightly fewer of them pass.
This run does not decompose why.

## 3. Item 2: the trade runs

The net is nutrient from decomposers by role to producers by role, less the reverse. It counts the
gradient flow (`Redistributed`) and routed surplus (`SurplusRouted`) separately, summed over the
worlds that form producer–decomposer links.

| arm | worlds with net > 0 / forming P–D links | gradient-flow net N | routed-surplus net N | sum |
|---|---:|---:|---:|---:|
| c1-low, decoded | 52 / 76 (68.4 %) | −289 695 | +448 901 | +159 206 |
| c1-low, fa 0 | 52 / 77 (67.5 %) | −350 355 | +526 355 | +176 000 |
| c1-mod, decoded | 56 / 75 (74.7 %) | −204 241 | +349 033 | +144 792 |
| c1-mod, fa 0 | 54 / 76 (71.1 %) | −264 833 | +455 546 | +190 713 |
| c2-test, decoded | 68 / 75 (90.7 %) | −149 006 | +626 167 | +477 161 |
| c2-test, fa 0 | 68 / 77 (88.3 %) | −198 038 | +675 844 | +477 807 |
| c4-low, decoded | 61 / 75 (81.3 %) | −227 211 | +687 756 | +460 545 |
| c4-low, fa 0 | 61 / 77 (79.2 %) | −317 456 | +770 775 | +453 319 |
| c4-lowfast, decoded | 65 / 74 (87.8 %) | −135 412 | +795 416 | +660 004 |
| c4-lowfast, fa 0 | 69 / 76 (90.8 %) | −236 692 | +875 187 | +638 496 |
| c4-mod, decoded | 65 / 75 (86.7 %) | −168 178 | +472 894 | +304 716 |
| c4-mod, fa 0 | 62 / 75 (82.7 %) | −176 635 | +527 202 | +350 567 |

The item holds in every arm. The gradient flow alone still carries nutrient from producers to
decomposers, as #646 found, because a homeostatic decomposer's store gives it nothing to send. The
nutrient leg is the routed flux, as world-rules.md flow 5 argues.

## 4. Item 3: pairs exist

For each world, the observed producer–decomposer share of live connections is compared with
2 · p_P · p_D over the role headcounts on the same sample ticks. The sign test is exact over the
discordant worlds.

| arm | above / below / equal (of 78) | n, k | one-sided p | median observed ÷ headcount share | P–D share of all links | decomposer in reach (reported only) |
|---|---|---|---:|---:|---:|---:|
| c1-low, decoded | 13 / 63 / 2 | 76, 13 | 1.0000 | 0.68 | 2.07 % | 2.4 % |
| c1-low, fa 0 | 10 / 68 / 0 | 78, 10 | 1.0000 | 0.69 | 1.68 % | 2.1 % |
| c1-mod, decoded | 9 / 68 / 1 | 77, 9 | 1.0000 | 0.64 | 1.36 % | 2.0 % |
| c1-mod, fa 0 | 4 / 73 / 1 | 77, 4 | 1.0000 | 0.61 | 1.35 % | 2.4 % |
| c2-test, decoded | 7 / 68 / 3 | 75, 7 | 1.0000 | 0.66 | 1.08 % | 2.4 % |
| c2-test, fa 0 | 9 / 69 / 0 | 78, 9 | 1.0000 | 0.66 | 1.19 % | 2.8 % |
| c4-low, decoded | 7 / 69 / 2 | 76, 7 | 1.0000 | 0.65 | 1.01 % | 2.7 % |
| c4-low, fa 0 | 5 / 73 / 0 | 78, 5 | 1.0000 | 0.65 | 0.99 % | 2.8 % |
| c4-lowfast, decoded | 5 / 70 / 3 | 75, 5 | 1.0000 | 0.62 | 0.97 % | 2.4 % |
| c4-lowfast, fa 0 | 4 / 72 / 2 | 76, 4 | 1.0000 | 0.62 | 0.99 % | 3.1 % |
| c4-mod, decoded | 5 / 72 / 1 | 77, 5 | 1.0000 | 0.57 | 0.95 % | 2.5 % |
| c4-mod, fa 0 | 4 / 71 / 3 | 75, 4 | 1.0000 | 0.58 | 0.91 % | 2.1 % |

The item fails in every arm, and fails the other way: pairs form *less* often than headcount
predicts (two-sided p < 0.0001 everywhere). The P–D share of links is about 1–2 %.
Farthest-first ranking (#739) cannot choose a partner that is not within reach, and in 97–98 % of
samples a producer with a free slot has no decomposer within reach. *Inference, not decomposed by
this run:* decomposers are scarce (the headcount share is 1.3–2.9 %, so p_D is around 1 %), and
they gather at carcasses rather than among producers. The pairs fall below even the scarce
headcount, so scarcity alone does not account for the failure. A headcount baseline assumes a
well-mixed world, and this one probably is not. This run does not separate the two causes.

## 5. Item 4: robustness guard

These are the unperturbed seeds (radius 0, 10 seeds × 78 cells) against #719's network-off rows.
The two-proportion z test is pooled and one-sided in the harmful direction.

| arm | failing seeds of 780 | z, p (failure rises) | cells where all seeds agree | z, p (agreement falls) | live-seed F (median) |
|---|---:|---|---:|---|---:|
| network off (#719) | 118 (15.1 %) | – | 32 | – | 0.334 |
| c1-low | 209 (26.8 %) | 5.66, < 0.0001 | 14 | 3.16, 0.0008 | 0.401 |
| c1-mod | 244 (31.3 %) | 7.56, < 0.0001 | 10 | 3.97, < 0.0001 | 0.404 |
| c2-test | 310 (39.7 %) | 10.90, < 0.0001 | 9 | 4.18, < 0.0001 | 0.405 |
| c4-low | 275 (35.3 %) | 9.16, < 0.0001 | 9 | 4.18, < 0.0001 | 0.404 |
| c4-lowfast | 333 (42.7 %) | 12.01, < 0.0001 | 10 | 3.97, < 0.0001 | 0.395 |
| c4-mod | 248 (31.8 %) | 7.77, < 0.0001 | 11 | 3.76, 0.0001 | 0.396 |

Failing verdicts, seeds of 780:

| arm | monoculture | nutrient lockup | generalist dominance | extinction | bloom stop | energy death |
|---|---:|---:|---:|---:|---:|---:|
| network off (#719) | 29 | 31 | 29 | 23 | 4 | 2 |
| c1-low | 86 | 67 | 36 | 13 | 4 | 3 |
| c1-mod | 125 | 51 | 48 | 11 | 6 | 3 |
| c2-test | 187 | 42 | 59 | 11 | 7 | 4 |
| c4-low | 165 | 43 | 44 | 10 | 11 | 2 |
| c4-lowfast | 246 | 18 | 48 | 8 | 11 | 2 |
| c4-mod | 152 | 39 | 39 | 9 | 6 | 3 |

The item fails in every setting. Most of the added failures are monoculture, and they grow with the
rate and the cap. The bloom stop barely moves, so the audit's dense-world cut-off does not explain
the rise. Live seeds score higher (0.40 against 0.33), so the network helps the worlds that survive
it and fails more of them. The census agrees: pooled persistence falls from 85.4 % / 87.9 % with
the network off to 52.3–75.1 % with it on, and census monoculture rises from 15 / 20 seeds to
38–143. *Inference:* with only 1–2 % of links joining a producer to a decomposer, the network mostly pools
energy among neighbouring producers, which levels the reserve differences selection would act on.
#646's producer–producer reading saw the same. This run does not test it.

## 6. Reported only

**Flow 1 signatures** (census, with `--atlas`; network off → on):

| arm | conditionality | pooled ρ | minority share |
|---|---|---|---|
| off, decoded / fa 0 | FAIL / FAIL | +0.087 / +0.021 | 60.1 % / 59.5 % FAIL |
| c1-low | PASS / FAIL ((2) 47 of 78, p = 0.089) | −0.034 / −0.042 | 56.8 % / 55.6 % FAIL |
| c1-mod | PASS / PASS | −0.013 / −0.031 | 54.0 % / 57.5 % FAIL |
| c2-test | PASS / PASS | +0.007 / −0.010 | 54.2 % / 56.6 % FAIL |
| c4-low | PASS / PASS | +0.039 / −0.046 | 54.5 % / 55.6 % FAIL |
| c4-lowfast | PASS / PASS | +0.027 / −0.004 | 53.3 % / 53.8 % FAIL |
| c4-mod | PASS / PASS | +0.022 / −0.030 | 56.5 % / 56.7 % FAIL |

Conditionality passes in 11 of the 12 on arms and fails in both off arms, as #654's hyphal arms
also showed. The minority share fails everywhere. In absolute terms, decomposers drain 99–153 % more
carcass structure per seed than with the network off, and light-fed mixotrophs 55–97 % more. By the
design's rule a signature is a readout, not a target, and a pass does not bring physics out of
reserve.

**Guild outcomes and lockup** (census, seeds of 390, decoded / fa 0):

| arm | persisted | monoculture | nutrient lockup |
|---|---|---|---|
| off | 333 / 343 | 15 / 20 | 15 / 8 |
| c1-low | 293 / 270 | 38 / 59 | 29 / 27 |
| c1-mod | 271 / 263 | 65 / 69 | 25 / 21 |
| c2-test | 243 / 229 | 87 / 104 | 22 / 15 |
| c4-low | 265 / 240 | 80 / 93 | 22 / 19 |
| c4-lowfast | 235 / 204 | 120 / 143 | 9 / 13 |
| c4-mod | 266 / 256 | 76 / 78 | 22 / 14 |

## 7. What this does not show

- **Whether a world with decomposers near producers would pair.** The atlas was searched without
  the network, and its decomposers are about 1 % of agents. Item 3's failure is a fact about these
  worlds, and it is consistent with spatial segregation. The rule did not ask for a spatial
  baseline, and this note does not substitute one.
- **The mechanism behind the monoculture rise.** That is an inference (section 5).
- **Whether a partner-only formation rule would fix item 4.** #739's ranking deliberately has no
  gate (world-rules.md flow 5). Any change to it is a design question, to be grilled against
  docs/ecology/ first.

## Reproduce

Run `target/736/run.sh decoded|fa0|audit`. The pinned binaries are in `target/736/bin`
(`shasum` kin_killer_diet cd0838df…, fragility_audit 3f5fa64d…).

- **Items 1–3:** run
  `kin_killer_diet --summary --atlas atlas.json --out target/736/<arm>.jsonl --baseline target/736/off-<mode>.jsonl`
  and read sections O and P.
- **Item 4:** run
  `python3 -I target/736/guard.py target/719/fragility/rows.jsonl target/736/audit-<setting>.jsonl`.
