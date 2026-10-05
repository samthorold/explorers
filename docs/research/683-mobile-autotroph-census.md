# Issue #683: the census of mobile autotrophy on the committed atlas

**Status: measurement, docs only (2026-10-05). The trigger reading for #648. On the committed
atlas (#677, 99 cells, fingerprint `aa2662b26da489a3`), ungated, it asks whether producers by
role with substantial effective mobility persist and breed on the settled half of a run. That is
the condition on which substrate contact comes out of reserve ([world rules](../system-design/world-rules.md),
flow 2). No code, design or atlas change.**

## TL;DR

**Mobile autotrophy is viable on genesis's worlds, at every threshold read. #648's trigger fires.**

Second half, seeds 1000–1004, T = 2000. "Holds" means the mobile producers number at least 5 on
every second-half sample tick of a run to the horizon, and at least one of them breeds there (the
heterotroph guild's rule, applied to the mobile producers as one population).

| | decoded | `founder_aggregation = 0` |
|---|---:|---:|
| producer agent-samples | 429,127 | 636,320 |
| producer effective mobility, p50 / p90 / max | 0.35 / 1.00 / 2.87 | 0.26 / 0.88 / 3.91 |
| share of producer samples with mobility > 0.3 | 53.0 % | 46.5 % |
| seeds where mobile producers (> 0.3) hold | **49 of 455** | **65 of 458** |
| configs where they hold in ≥ 1 seed / ≥ 3 of 5 seeds (> 0.3) | 32 / 3 | 35 / 10 |
| seeds where they hold, at > 0.5 | 33 of 455 | 41 of 458 |
| seeds where they hold, at > 0.05 | 88 of 455 | 149 of 458 |

- **Most producers move.** The median producer has effective mobility 0.35 (decoded) and 0.26
  (fa 0), and in this world an agent with mobility above zero moves its full stride every tick,
  so realised movement per tick equals effective mobility exactly.
- **Moving producers breed, and in about a tenth of runs they persist as a population.** At the
  proposed threshold of 0.3, mobile producers hold in 11–14 % of the runs that reach the horizon,
  spread over a third of the atlas's worlds. At 0.5 they still hold in 7–9 %.
- **The threshold does not decide the outcome.** The trigger fires at every point of the grid
  (0.05–0.5), decoded and at fa 0.

## 1. What ran

| | |
|---|---|
| Tree | main `eb3dab1` (#690's section L of `kin_killer_diet`). Binary pinned in `target/683/bin` |
| Driver | `scripts/683-census.sh`: committed `atlas.json`, all 99 cells, seeds 1000–1004, T = 2000, `--satiation-sensitivity 0`, decoded and `--founder-aggregation 0` |
| Wall clock | 79 s decoded, 94 s fa 0. No errors |
| Outputs | `target/683/{decoded,fa0}.{md,jsonl}`, 99 rows each |

**Check.** The same rows' flow 1 verdict block (#682) reproduces #670's seed-42 readings exactly
(median per-config ρ −0.021 / −0.026, 52 / 55 of 99 configs negative, settled-half share 61.6 /
59.3 %, persisted 449 / 445 of 495), as it should on the same atlas under the same physics.

## 2. The threshold

The census reports five candidate thresholds on effective mobility (the wear-degraded trait):
0.05, 0.1, 0.2, 0.3 and 0.5.

| effective mobility > | % of producer samples, dec / fa 0 | births, dec / fa 0 | seeds holding, dec / fa 0 |
|---:|---:|---:|---:|
| 0.05 | 73.5 / 68.2 | 38,532 / 47,075 | 88 / 149 |
| 0.1 | 69.7 / 64.3 | 37,180 / 45,113 | 76 / 129 |
| 0.2 | 62.3 / 54.9 | 34,715 / 40,935 | 61 / 91 |
| 0.3 | 53.0 / 46.5 | 31,426 / 36,669 | 49 / 65 |
| 0.5 | 39.2 / 33.2 | 25,673 / 26,623 | 33 / 41 |

**Proposed: 0.3.** A producer above it moves at least 0.3 per tick, every tick. It crosses a
nutrient cell (10 on the search baseline) in about 33 ticks, and its light-competition
neighbourhood far sooner, so it is not anchored to one patch of substrate in any sense the domain
would recognise. 0.3 is also near the
median of the decoded distribution. That puts the cut inside the bulk of the moving producers, not
in a tail that a few worlds could carry. Lower cuts count producers that drift slowly. 0.5 is
stricter and still fires. Since every threshold fires, the choice matters only for how large the
mobile population is said to be, not for the trigger.

## 3. Why the economic divide does not hold here (inference)

The design's divide is economic for now: a moving autotroph pays per-distance movement cost for no
extra light (flow 2, trade-off #2). The stepper charges `distance × movement_cost_coefficient ×
structure`. On the search baseline the coefficient is 0.05, and genesis searches it over [0.001,
0.1]. For a producer of median structure (about 8, #645) moving 0.35 per tick, that is about 0.14
per tick at the baseline coefficient, against a typical metabolic cost of about 2 per tick (#668).
So movement costs a producer a few per cent of its upkeep. Uptake reads only its current cell,
with no contact term, so a moving producer loses nothing else. *This is an inference from the
cost's form and typical magnitudes. The census does not read movement cost per agent.*

## 4. What this means for #648

By flow 2's rule, **substrate contact comes out of reserve**: mobile autotrophy is measured
viable. Where it enters the plan is the owner's call, recorded on #648. The design had held #648
behind #686 so that its effect could be read on its own, not folded into the ungated re-search.

## 5. What this does not show

- **One atlas, searched under the surplus gate at `satiation_sensitivity` 33 and read ungated**,
  as #656, #668 and #670 were. #686 reads the census again on an atlas searched ungated.
- **"Lineage" is the mobile producers as one population**, not a single line of descent. A run
  can hold by turnover among unrelated moving producers.
- **Effective mobility is a trait, and here it is also the behaviour.** Every agent with mobility
  above zero moves its full stride each tick, so the census cannot separate a producer that could
  move from one that does.
- **Movement's cost to producers is inferred, not read** (§3).

## Reproduction

From the repo root, on `eb3dab1` or later: `scripts/683-census.sh`. It is resumable and prints
`DONE`. Section L of `target/683/{decoded,fa0}.md` holds the tables above. Per-config counts come
from `tally.ext.mobile.above[i].guild_seeds` in the JSONL rows (thresholds in grid order).

## References

- [670-cross-trait-verdict.md](670-cross-trait-verdict.md): the same atlas's flow 1 readings.
- [World rules](../system-design/world-rules.md): flow 2 (*Substrate contact is held in
  reserve*), trade-off #2.
