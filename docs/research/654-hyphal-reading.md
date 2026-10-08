# Issue #654 / #730: hyphal uptake read on the committed atlas by the fixed rule

**Status: measurement, docs only (2026-10-08). Hyphal uptake (#727) was switched on over the
committed atlas (#719's, 78 cells, recipe atlas:31) and read against the switch off by the rule
posted on #654 before the run. Binaries pinned at main 0f097a3 under `target/654/bin`, driver
`target/654/run.sh`.**

> **Correction (2026-10-08, after review).** The first version of this note read the
> switch-off gradient as evidence against the design's premise that no agent is poor in energy
> and rich in nutrient. The ecology literature says otherwise.
> - Consumers keep their body stoichiometry roughly constant and excrete the nutrient they cannot
>   use (Sterner & Elser 2002; [nutrient cycling](../ecology/nutrient-cycling.md)), as flow 3
>   does.
> - Decomposers are limited by carbon, not mineral nutrient, where they coexist with producers
>   (Daufresne & Loreau 2001; [stability and resilience](../ecology/stability-and-resilience.md)).
>
> A high free-nutrient-to-reserve ratio in heterotrophs is what energy limitation looks like, not
> a surplus they can trade, so it is consistent with the design. The failure belongs instead to
> the mechanism. Hyphal uptake rides on effective heterotrophy, with stillness standing in for the
> absorptive habit, which gives every sessile mixotroph its own hyphae. In the literature a
> plant's heterotrophy is carnivory, and a producer reaches hyphae only through a fungal partner:
> it "does not become a decomposer" ([fungi](../ecology/fungi.md), *The producer's route to
> organic nutrient*). That departure is why 98 % of the flow reaches producers. The ratio test
> was also the wrong test of a partner. The literature's exchange runs on complementary
> limitation (Kiers et al. 2011): producers limited by nutrient, the partner limited by carbon.
> The measurements below stand. The sentences this correction supersedes are marked.

## TL;DR

**The rule's partner test fails in both modes, so #654 is re-deferred.** With hyphal uptake on,
heterotrophs by role out-hold producers by role in free nutrient per unit reserve in a bare
majority of worlds. But they already did so with it off, and switching it on does not add
worlds: the paired sign test runs the wrong way (one-sided p = 0.81 decoded, 0.87 at fa 0).
Robustness holds. Two findings the rule did not ask for:

1. **The flow feeds producers, not heterotrophs.** 98 % of hyphal nutrient goes to producers by
   role that carry heterotrophy (the still light-fed mixotrophs). It raises their free nutrient
   faster than the heterotrophs', so the gradient narrows.
2. **It restores flow 1's conditionality.** In every switch-on arm, at every contact distance,
   producer heterotrophy falls where the pool is rich (three tests PASS). With the switch off it
   fails, and runs backwards (pooled ρ +0.087 decoded). It is a readout, not a target, so it
   does not change the verdict.

| rule item | decoded | fa 0 | verdict |
|---|---|---|---|
| 1a. gradient > 1 in a majority of worlds carrying heterotrophs by role | 39 of 73 (53.4 %) | 39 of 76 (51.3 %) | pass |
| 1b. more such worlds than switch-off, paired sign test p < 0.05 | discordant n = 21, k = 9, one-sided p = 0.808 | n = 13, k = 5, p = 0.867 | **FAIL** |
| 2a. per-seed failure rate does not rise (two-proportion, p < 0.05) | 118 → 116 of 780 seeds (15.1 → 14.9 %), z = −0.14, p = 0.56 | (audit is decoded) | holds |
| 2b. seed agreement does not fall | all seeds agree 32 → 25 of 78 cells, z = 1.16, p = 0.12 | | holds |

**Verdict: item 1 fails → re-defer #654, with this reading recorded.**

## 1. What ran

| arm | instrument | settings | wall-clock |
|---|---|---|---|
| census, switch off | `kin_killer_diet` | seeds 1000–1004, T = 2000, all 78 cells; decoded / fa 0 | 728 s / 653 s |
| census, switch on, `d_c = 0.1` | same, `--hyphal-uptake on` | same | 538 s / 726 s |
| census, `d_c = 0.05` | `--contact-distance 0.05` | same | 541 s / 750 s |
| census, `d_c = 0.2` | `--contact-distance 0.2` | same | 629 s / 669 s |
| audit, switch on | `fragility_audit --draws 0 --hyphal-uptake on` | seeds 1000–1009, T = 2000, unperturbed only | 609 s |
| audit, switch off | #719's rows (`target/719/fragility/rows.jsonl`, radius 0) | as #719 ran it | – |

The switch-off census reproduces #719's census on all 78 cells in both modes (identical outcomes,
kin kills and mobile census). The switch-off audit numbers below reproduce #719's published
ones (118 of 780 failing seeds, 31 lockup seeds, 41.0 % of cells agreeing). The switch is
bit-identical when off, as #727 pins.

The census's partner readout (#728) defines the quantities. **Free nutrient** is the unearmarked
store plus the reproductive earmark. **Reserve** is unallocated reserve plus the reproductive
allocation. Both are read on every settled-half role sample, by the sample's role. A run's
**gradient** is the heterotrophs' median free nutrient ÷ reserve over the producers', on a
persisted run with both. A world's gradient is the median over its qualifying runs. No sample had
zero reserve.

## 2. Partner

| arm | worlds carrying heterotrophs by role | gradient > 1 | switch-off baseline | discordant n, k (on > off) | one-sided p | hyphal share |
|---|---:|---:|---:|---|---:|---:|
| decoded, `d_c` 0.1 | 73 | 39 (53.4 %) | 42 of 75 | 21, 9 | 0.808 | 10.0 % |
| fa 0, `d_c` 0.1 | 76 | 39 (51.3 %) | 42 of 77 | 13, 5 | 0.867 | 14.6 % |
| decoded, `d_c` 0.05 | 75 | 37 (49.3 %) | 42 of 75 | 23, 9 | 0.895 | 10.4 % |
| fa 0, `d_c` 0.05 | 76 | 41 (53.9 %) | 42 of 77 | 17, 8 | 0.686 | 11.7 % |
| decoded, `d_c` 0.2 | 77 | 39 (50.6 %) | 42 of 75 | 17, 7 | 0.834 | 12.6 % |
| fa 0, `d_c` 0.2 | 75 | 35 (46.7 %) | 42 of 77 | 21, 7 | 0.961 | 15.1 % |

The failure does not depend on the contact distance. At every `d_c`, fewer worlds move above 1
than move below it.

**The baseline already shows the gradient.** With the switch off, 42 of the 75 (decoded) and
42 of the 77 (fa 0) worlds carrying heterotrophs by role have gradient > 1. ~~That reads against
the design's premise that no agent is poor in energy and rich in nutrient (world-rules.md, *The
partnership needs a partner*).~~ *(Superseded by the correction above: this is energy
limitation, as the literature expects of heterotrophs, not a tradeable surplus.)* Over
qualifying runs, the switch-off quartiles are:

| | decoded, off | decoded, on | fa 0, off | fa 0, on |
|---|---|---|---|---|
| run gradient (q1 / median / q3) | 0.42 / 1.15 / 4.49 | 0.34 / 1.02 / 3.83 | 0.37 / 1.01 / 3.02 | 0.28 / 0.87 / 2.56 |
| heterotrophs' median free N ÷ reserve | 0.75 / 1.66 / 3.53 | 0.82 / 1.99 / 4.50 | 0.80 / 1.75 / 3.60 | 1.03 / 1.85 / 4.02 |
| producers' median free N ÷ reserve | 0.29 / 1.35 / 4.41 | 0.49 / 1.65 / 7.04 | 0.61 / 1.67 / 5.99 | 0.77 / 2.08 / 9.99 |
| heterotroph samples per run | 5 / 21 / 58 | 7 / 19 / 57 | 5 / 20 / 69 | 6 / 17 / 66 |

(Runs: 223 / 229 / 304 / 274.) The run gradient spans an order of magnitude either side of 1, and
heterotrophs by role are thin, with a median of about 20 samples a run against hundreds of
producer samples. *Inference, not decomposed by this run:* the ratio reads reserve as much as
nutrient, so a consumer that lives near its retention buffer reads "nutrient-rich" by holding
little energy rather than much nutrient. The readout separates the arms only weakly, so the rule's
indicator is a noisy test of the partner.

**Where the hyphal nutrient goes** (census section C, settled half, pooled over the 78 cells):

| recipient by role | decoded: share of hyphal N | hyphal share of its pool uptake | fa 0: share of hyphal N | hyphal share of its pool uptake |
|---|---:|---:|---:|---:|
| producer, h_eff > 0.05 (light-fed mixotroph) | 98.0 % | 19.0 % | 97.8 % | 23.1 % |
| producer, h_eff ≤ 0.05 | 0.7 % | 0.1 % | 0.5 % | 0.1 % |
| consumer | 0.3 % | 25.4 % | 0.4 % | 32.7 % |
| decomposer | 0.8 % | 18.4 % | 1.0 % | 25.1 % |
| no role yet | 0.2 % | 5.2 % | 0.2 % | 10.5 % |

The committed atlas's producers carry heterotrophy and are sessile, so they hold full contact and
take nearly all the new flow. Heterotrophs by role do gain nutrient from it: a quarter to a third
of a consumer's pool uptake is hyphal, and 10–15 % of all their nutrient income. But producers'
free nutrient per reserve rises faster than theirs (median 1.35 → 1.65 against 1.66 → 1.99,
decoded), so the gradient the partnership needs narrows. On these worlds the flow serves
mixotrophs, not a fungal partner. That follows from the mechanism, which lets a sessile
mixotroph's heterotrophy absorb from the pool. The literature gives that capability to
absorptive heterotrophs (fungi and bacteria), not to plants (see the correction above).

## 3. Robustness

The audit's unperturbed seeds, 10 per cell, 780 seeds:

| | switch off (#719) | switch on |
|---|---:|---:|
| failing seeds | 118 (15.1 %) | 116 (14.9 %) |
| cells where all 10 seeds agree | 32 (41.0 %) | 25 (32.1 %) |
| mean live fraction | 0.849 | 0.851 |
| live-seed F (median over cells) | 0.334 | 0.328 |
| nutrient lockup | 31 | 28 |
| generalist dominance | 29 | 28 |
| monoculture | 29 | 28 |
| extinction | 23 | 25 |
| bloom stop | 4 | 4 |
| energy death | 2 | 3 |

Neither guard test fires. Seed agreement falls by 7 cells, which is not significant at n = 78.

**Census persistence and lockup** (5 seeds per cell, 390 runs): persisted runs, off → on, go
333 → 338 decoded and 343 → 325 at fa 0. Lockup seeds go 15 → 14 and 8 → 10.

**atlas:31, the recipe:** it lives on all 10 audit seeds both ways, and mean live-seed fitness is
0.737 → 0.701. It persists on all 5 census seeds in every arm. Its run gradients, decoded, go
from 1.15–12.3 to 0.68–4.21. Its refined coexistence and fitness (32 seeds) were not re-measured:
refinement has no switch override, and the audit's 10 seeds cover the same question for a
reading that fails.

## 4. Reported, not required

**Flow 1** (#682's verdict block):

| | off decoded | on decoded | off fa 0 | on fa 0 |
|---|---|---|---|---|
| (1) median per-config ρ | −0.010 PASS | −0.117 PASS | +0.033 FAIL | −0.113 PASS |
| (2) configs with ρ < 0 | 41 of 78, p = 0.73 FAIL | 57 of 78, p < 0.001 PASS | 35 of 78, p = 0.43 FAIL | 54 of 78, p = 0.001 PASS |
| (3) lineage clusters with median ρ < 0 | 3 of 8 FAIL | 7 of 8 PASS | 3 of 8 FAIL | 7 of 8 PASS |
| **conditionality** | **FAIL** | **PASS** | **FAIL** | **PASS** |
| pooled ρ (fallback: ≤ 0) | +0.087 | −0.082 | +0.021 | −0.038 |
| **minority share**, second half | 60.1 % FAIL | 61.4 % FAIL | 59.5 % FAIL | 57.4 % FAIL |

Conditionality passes in all six switch-on arms (`d_c` 0.05, 0.1 and 0.2; decoded and fa 0). The
weakest is (2) at 51 of 78, p = 0.009. The pooled fallback is mixed at the outer contact distances:
+0.006 at 0.05 fa 0, and +0.024 at 0.2 decoded. *Inference:* a sessile mixotroph now draws pool
nutrient through its heterotrophy machinery, so on rich ground the machinery pays through the
pool rather than through grazing. That would turn the backwards pattern #721 left open, and
leaching could not explain, into the domain's carnivorous-plant one. This run does not test the
mechanism. By the design's rule a signature is a readout, not a target, and a pass does not bring
physics out of reserve.

**Mobile autotrophy** (#683's census, runs where mobile producers hold, off → on):

| effective mobility > | decoded | fa 0 |
|---|---|---|
| 0.05 | 138 / 362 → 138 / 360 | 175 / 377 → 161 / 373 |
| 0.3 | 75 / 362 → 73 / 360 | 80 / 377 → 83 / 373 |
| 0.5 | 51 / 362 → 52 / 360 | 50 / 377 → 51 / 373 |

The median producer's effective mobility is 0.345 → 0.327 decoded and 0.191 → 0.198 at fa 0.
Mobile producers are unchanged, as expected: contact is not applied to roots.

## 5. What this does not show

- **Whether a search under the flow would find the partner.** The atlas was chosen without
  hyphae, and its producers are mixotrophs. Under the rule, a fresh search follows only a pass.
- **Whether the ratio is the right test.** The rule fixed free nutrient ÷ reserve before the
  run, and this note reads it as fixed. Section 2's inference suggests reserve dominates it. A
  different partner test would need its own rule, fixed before another run.
- **The mechanism behind the conditionality result.** It is an inference.

## Reproduce

`target/654/run.sh` (pinned binaries in `target/654/bin`, `shasum` kin_killer_diet
aa978b36…, fragility_audit ef8f21e0…). For the sign tests, run
`kin_killer_diet --atlas atlas.json --summary --out target/654/<arm>.jsonl --baseline target/654/off-<mode>.jsonl`
(section N). For the guard, apply the definitions in §3 to `target/654/audit-on.jsonl` against
#719's radius-0 rows.
