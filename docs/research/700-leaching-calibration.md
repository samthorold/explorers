# Issue #700: calibrating the top of the leaching rate's range, and a first λ sweep

**Status: measurement, docs only (2026-10-06). The calibration read that sets the top of the
leaching rate λ's search range ([world rules](../system-design/world-rules.md), *Carcass energy
decays only through agents; carcass nutrient leaches*, *The form*), and a first look at what
leaching does to lockup, on the committed atlas. No code, design or atlas change.**

## TL;DR

**Proposed λ_max = 0.01, from t* ≈ 70 ticks: the median time for a drainer to find a carcass it
did not kill. Leaching removes nutrient lockup, but most of the effect is there by a sixteenth of
that top. Leaching also adds monocultures.**

| λ | persisted (of 495) | nutrient lockup | monoculture | carcass-locked, gate window | leached % of carcass N out |
|---:|---:|---:|---:|---:|---:|
| 0 | 455 | 19 | 6 | 0.142 | 0 |
| 0.000625 (λ_max/16) | 457 | 7 | 17 | 0.106 | 52.0 |
| 0.00125 (λ_max/8) | 465 | 5 | 12 | 0.086 | 63.7 |
| 0.0025 (λ_max/4) | 468 | 2 | 12 | 0.067 | 71.0 |
| 0.005 (λ_max/2) | 469 | 2 | 10 | 0.053 | 75.9 |
| 0.01 (λ_max) | 469 | 1 | 13 | 0.047 | 79.8 |

1. **#700's literal reading of t* measures kills, not search.** The median time to first drain
   over every drained carcass is 1 tick. About a third of carcasses were bitten to death, drained
   while living in the step they died, and the drainer already on them bites again the next tick.
   Leaving those out, the median over drained carcasses is 84 ticks over the whole run and 68 on
   the settled half. The per-config median of 99 configs is 75 (interquartile range 49–110). λ_max
   = ln 2 / 68 ≈ 0.0102, rounded to **0.01**.
2. **Leaching ends lockup.** Nutrient-lockup seeds fall from 19 (in 14 configs) to 1 (in 1
   config) at λ_max. At λ_max/4, 16 of the 19 persist.
3. **The response is steeply concave.** At λ_max/16, a half-life of about 1100 ticks, lockup has
   already fallen by 63 % and leaching already carries half of the carcass nutrient that leaves.
   A linear box on [0, 0.01] spends almost all its draws where lockup is already gone. That is a
   question for #701 (§5).
4. **Leaching adds monocultures.** Monoculture seeds rise from 6 to 10–17 at every λ > 0. Part
   of this is lockup seeds that now stop on monoculture instead. Part is 7–13 seeds that persisted
   at λ = 0. Persisted seeds net out at +14, not +19.

## 1. What ran

| | |
|---|---|
| Tree | main `46efe00` (#704: section M of `kin_killer_diet`, `scripts/700-leaching.sh`). Binary pinned in `target/700/bin` |
| Worlds | committed `atlas.json` (#677, 99 cells, fingerprint `aa2662b26da489a3`): ungated, decoded at `founder_aggregation = 0`, seeds 1000–1004, T = 2000 |
| Phase 1 | `scripts/700-leaching.sh`: λ = 0, 90 s |
| Phase 2 | `env PHASE=2 "LAMBDAS=0 0.000625 0.00125 0.0025 0.005 0.01" scripts/700-leaching.sh`: 104–164 s per λ, slower as λ rises |
| Outputs | `target/700/lambda-<λ>/{rows.jsonl,report.md}`, `calibration.md`, `sweep.md`. No errors |

#700 asked for λ_max/4, /2 and 1. λ_max/16 and /8 were added once λ_max/4 showed the effect
nearly saturated.

## 2. Time to first drain (λ = 0)

For each carcass formed in a run: the ticks from its death to the first bite any drainer takes.
A carcass never drained is censored at the run's end. *Bitten to death* means the carcass was
drained while living in the step it died, so its drainer was already on it. The settled half is
deaths after tick 1000.

| carcasses | window | n | never drained | p25 | median t* | p75 | p90 | KM median |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| all | whole run | 177,442 | 53.7 % | 1 | 1 | 56 | 261 | – |
| all | settled half | 56,453 | 61.0 % | 1 | 1 | 41 | 202 | – |
| not bitten to death | whole run | 124,094 | 74.0 % | 19 | 84 | 246 | 503 | – |
| not bitten to death | settled half | 42,623 | 79.6 % | 14 | 68 | 195 | 381 | – |
| bitten to death | whole run | 53,348 | 6.5 % | 1 | 1 | 1 | 3 | 1 |

The Kaplan–Meier median, which counts undrained carcasses as censored, is unread on every set
except the bitten: more than half are never drained, so the estimated undrained share never falls
to one half.

## 3. Choosing t*

The design sets the top at "the rate at which a typical carcass leaches half its store in the
time a drainer takes to reach it". The readings:

| reading | t* | λ_max |
|---|---:|---:|
| median over drained, all carcasses (#700's literal reading) | 1 | 0.69 |
| median over drained, not bitten to death, whole run | 84 | 0.0083 |
| **median over drained, not bitten to death, settled half** | **68** | **0.0102** |
| per-config median of the above (whole run), median over 99 configs | 75 | 0.0092 |
| KM median, undrained censored (any set) | – | – |

**The not-bitten, settled-half median is the reading.**

- **Leave out the bitten.** A carcass bitten to death has nothing to reach: its drainer is on it
  at death, and the 1-tick gap is the order of a step, not a search. At 0.69 a carcass would leach
  half its store every tick. That is not slow leaching. It would empty every carcass's soluble
  store before any drainer could matter, and so wipe out the split the design rests on.
- **Use the settled half.** The settled half is the regime the gates and descriptors read. Its 68
  is close to the whole run's 84 and the per-config 75, so the choice moves λ_max by under 25 %.
- **The censored reading can't be used, and the median is the right side of it.** Three quarters
  of not-bitten carcasses are never drained, so the median over drained ones understates how long
  a carcass waits. ln 2 / t* is therefore an upper bound on the rate the rule asks for. That is
  the right side to err on for the top of a range.

## 4. The sweep

Verdicts are the census's, per seed. *Carcass-locked* is the dead pool's share of conserved
nutrient, read off the lockup gate's own series on every run, as its trailing mean over the gate
window and as its settled-half mean, averaged over runs. *Leached %* splits carcass nutrient out
by route. The *light-fed mixotroph share* is #683/#642's light-fed mixotrophs' share of attributed
second-half carcass structure drained (`kin_killer_diet` section H).

| λ | persisted | lockup | extinction | generalist dominance | monoculture | carcass-locked, gate / settled | leached %, whole / settled | light-fed mixotroph % | t* not bitten |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 455 | 19 | 9 | 6 | 6 | 0.142 / 0.113 | 0 / 0 | 67.4 | 84 |
| 0.000625 | 457 | 7 | 10 | 4 | 17 | 0.106 / 0.090 | 52.0 / 63.4 | 56.4 | 112 |
| 0.00125 | 465 | 5 | 9 | 4 | 12 | 0.086 / 0.076 | 63.7 / 73.3 | 52.0 | 112 |
| 0.0025 | 468 | 2 | 9 | 4 | 12 | 0.067 / 0.059 | 71.0 / 78.2 | 56.1 | 114 |
| 0.005 | 469 | 2 | 9 | 5 | 10 | 0.053 / 0.048 | 75.9 / 80.8 | 54.8 | 123 |
| 0.01 | 469 | 1 | 7 | 5 | 13 | 0.047 / 0.040 | 79.8 / 84.0 | 54.4 | 121 |

**Lockup, seed by seed.** Of the 19 seeds that lock up at λ = 0:

| λ | still lockup | now persist | now monoculture | now extinction |
|---:|---:|---:|---:|---:|
| 0.000625 | 7 | 10 | 2 | 0 |
| 0.00125 | 4 | 13 | 2 | 0 |
| 0.0025 | 2 | 16 | 1 | 0 |
| 0.005 | 2 | 15 | 1 | 1 |
| 0.01 | 1 | 15 | 3 | 0 |

Configs with any lockup seed: 14 at λ = 0 (16, 29, 39, 43, 49, 53, 58, 60, 70, 78, 80, 82, 88,
89); 6 at λ_max/16; 4 at λ_max/8; 2 at λ_max/4 and λ_max/2; 1 (89) at λ_max. The configs that
still lock up somewhere at λ ≥ λ_max/4 are 49, 82, 88 and 89. At λ_max/8, one seed that persisted at λ = 0 locks up instead.

**Monoculture.** Seeds that persisted at λ = 0 and stop on monoculture at λ > 0: 13, 8, 8, 7 and
8 across the five λ values. Lockup-to-monoculture seeds add 1–3 more. The new monocultures fall in
configs 8, 18, 52, 61, 69, 73, 75 and 90, worlds with no lockup at λ = 0. Leaching returns
nutrient to the cell where the carcass lies, so a likely cause is a single producer guild taking
that pulse. This sweep does not show the cause. Lockup configs 43, 53, 58, 82, 88 and 89 show
monoculture seeds at some λ > 0, which may only be a later gate firing once lockup no longer
stops the run first.

**Readouts.**

- *Light-fed mixotrophs' share of carcass structure.* It falls from 67 % to 52–56 % at any
  λ > 0 and does not trend with λ beyond that. Leaching takes the nutrient that made carcasses
  rich. A plausible reading, not tested here, is that this makes carcasses less worth draining
  for light-fed mixotrophs in particular.
- *Time to first drain.* For not-bitten carcasses it lengthens with λ, from 84 to 112–123
  ticks. Read as a direction only; this sweep does not show why.
- *The never-drained share* stays at 53–54 % at every λ. Leaching does not change how many
  carcasses drainers find, only what is left in the ones they don't.

## 5. For #701: the box's range

The rule gives 0.01, and the design searches λ linearly from 0, with 0 held exactly. The sweep
shows the effect nearly saturated by λ_max/4. By λ_max/4, lockup has fallen 89 % and the carcass-locked
fraction 53 %. Going on to λ_max removes one more lockup seed. That is because the carcasses
that matter for lockup are the 54 % that no drainer ever finds. Their clock is the run's horizon,
not a drainer's reach, and over 2000 ticks even a 1100-tick half-life empties most of their
soluble store.

This bears on #686's read ("if robustness-scored worlds pile toward λ > 0, leaching buys
robustness"). On [0, 0.01], about 94 % of the box lies above λ_max/16, where lockup is already
mostly relieved. Piling toward λ > 0 then says little beyond "not at 0".

Three options, for the owner:

- **(a) Keep 0.01, the rule's reading.** It is simple and faithful to the design, but most of the
  box is saturated.
- **(b) Lower the top to about 0.0025**, where lockup is 89 % relieved. This spends the box on the
  transition. The trade-off: it departs from the rule's "time to reach" reading in favour of where
  the response saturates.
- **(c) Keep 0.01 but search a concave transform that holds 0**, for example λ = λ_max · u² with u
  linear on [0, 1]. Half the draws then fall below λ_max/4.

My recommendation is **(c)** if the box machinery takes a per-dimension transform cheaply, else
**(b)**. Either keeps 0 exact, as the design requires, and puts the search's resolution where the
worlds differ. Settle this with `grill-with-docs` before #701.

## 6. Limits

- **The atlas was searched under different physics.** The committed atlas came from the old
  surplus gate with c_AH in a 34-dimension box, and every λ here is applied to those worlds.
  Whether worlds searched with λ in the box settle where these did is #686's question.
- **The sweep shows lockup relief, not robustness.** Every row is 5 seeds per world, unperturbed.
  Whether leaching narrows the seed-to-seed spread #693 found needs that audit rerun at λ > 0.
- **The monoculture rise is counted, not explained.** If #686's worlds show it too, it needs its
  own look.
