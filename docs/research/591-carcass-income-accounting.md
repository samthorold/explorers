# Issue #591: where does `sample:31`'s near-producer carcass income go?

**Status: measurement. Adds a lib module (`explorers_search::energy_accounting`) and an
`--accounting` mode on the `reinvasion_barrier` bin. No trajectory changes. `explorers-sim`, the
search, the evaluator and the prefilter are unchanged.**

[#587](587-reinvasion-barrier.md) injected phenotypes onto `sample:31` at tick 1000. It found that
a mixotroph one mutation step toward heterotrophy (`step 1×`, `e = 0.29`, carcass-intake ceiling
≈ 0.10 E/tick on one carcass) shrinks faster than an unmodified producer cohort (`step 0`). Every
phenotype also did worse when placed on the pile.

[#589](589-heterotroph-margin.md)'s closed form says that on one carcass in reach the step more
than pays for its base and heterotrophy maintenance. This note books the realised energy of every
injected lineage member on every tick. The aim is to find where the promised income goes.

## TL;DR

1. **Reach, not presence, closes the gap between the closed form and the carcass intake.**
   - A `step 1×` member has a median of 13 (uniform) to 18 (pile) energy-holding carcasses in its
     nutrient-grid cell. It has only 0.15 carcasses per tick inside its consumption reach
     (`0.35 × 0.81 ≈ 0.28` length units).
   - Its realised carcass ceiling is therefore 0.011 E/tick, not the 0.10 the closed form sets at
     `k = 1`.
   - Sharing (≤ 0.00003 E/tick) and exhaustion (0.0004–0.0005, about 4 % of the ceiling) are
     negligible. Wear costs nothing.
   - Kept carcass income is 0.0106 (uniform) and 0.0108 (pile) E/tick.
   - The pile does not raise the realised `k`: 0.145 on the pile against 0.146 off it.
2. **What does reach does pay.**
   - Realised `k` (0.15) sits at #589's break-even `k* = 0.14`.
   - The carcass income covers about 73 % of #587's two terms (base + heterotrophy, 0.0146 E/tick).
   - It covers the step's own extra heterotrophy maintenance (0.0029) 3.6 times over.
   - Measured against the producer, the step gains energy from carcasses. The kernel is not
     the problem.
3. **The terms the closed form omits are 14–23 times its two terms. They are paid by
   photosynthesis, not by carcasses.**
   - `step 1×` upkeep is 0.20–0.34 E/tick: structure maintenance, photosynthesis maintenance,
     growth loss and movement. Against it, photosynthesis earns 0.71–1.08 E/tick.
   - Carcass income is about 1 % of the step's income.
   - The step's net energy (income − upkeep) is positive: +0.68 uniform and +0.53 pile. That is
     *higher* than the unmodified producer (+0.44 uniform, +0.09 pile).
4. **The step does not lose on energy. It loses its young.** This sink is not on the issue's
   list:
   - Of `step 1×`'s 230 deaths over 8 seeds (uniform), 186 are members grazed to death, and 16
     starved.
   - 153 of the deaths are infants (≤ 50 ticks after birth), and 148 of those infants were grazed
     to death.
   - 99 % of the structure grazed off the lineage is grazed by other members.
   - The unmodified producer loses 55 of 172 deaths to grazing, and 21 of its 39 infant deaths.
   - Raising heterotrophy raises the drain on the nearest living targets, and a newborn dispersed
     beside its parent and siblings is one. So the step eats its own offspring.
   - Births do not lag in number (186 against 170). Recruitment does: the energy earmark reaches
     the reproduction threshold on 3 % of member-ticks, against 16 % for the producer.
5. **The pile is worse for every phenotype because it costs photosynthesis.**
   - On the pile the unmodified producer earns 0.19 E/tick of light against 0.90 off it (the
     unshaded ceiling is 4.88). Its energy gate is met on 1 % of member-ticks against 16 %.
   - `step 1×` loses less (0.71 against 1.08).
   - For the full heterotroph, the pile adds kin grazing (0.044 E/tick grazed off members, nearly
     all of it by members) and no more carcass income.
6. **Reading: none of the three as posed, with the second closest.**
   - The carcass route delivers what reach allows, which is about the closed form's break-even.
   - Sharing and exhaustion do not matter (not reading 1).
   - The cost of the step is not on the autotrophic side off the pile. The step photosynthesises
     as well as the producer (not reading 2, except for the pile).
   - The omitted upkeep is real and large, but photosynthesis pays for it, so folding it into
     `heterotroph_margin` would not explain the lockup (not reading 3).
   - What sinks the step is demographic: self-grazing kills its infants.
7. **The contrast case refutes the issue's expectation.** On #462's live baselines (`sample:110`,
   `18`, `188`) the resident consumers do not live on carcasses.
   - Their realised `k` is 0–0.01 and their carcass income 0–0.0035 E/tick.
   - Their photosynthesis is 4.5–9.2 E/tick, at or near the unshaded ceiling.
   - They persist as mixotrophs on light in sparse worlds of 4–18 agents. The emergent-terms
     reading of #589 ("carcasses in reach at once") does not hold there either.

## 1. What ran

`cargo run --release -p explorers-search --bin reinvasion_barrier -- sample:31 --accounting` (and
the same command for `sample:110`, `sample:18`, `sample:188`). The branch is
`issue-591-carcass-income-accounting` from `c0e0ccd`. Each 8-seed run took about 7 s wall clock
(`sample:31`) or under 1 s (the contrast configs).

| | |
|---|---|
| Config, seeds, injection | as #587: `sample:31`, seeds 1000–1007 (8/8 reached the injection tick), injection at tick 1000, window 1000, cohort 8 |
| Arms | `step 0` (the resident producer centroid, heterotrophy 0.036), `step 1×` (heterotrophy 0.348), `full` (the founder heterotroph centroid, heterotrophy 1.086, photosynthesis 0), each `uniform` and `pile` |
| Contrast | the control fork's own consumers: agents in the consumer or decomposer role at the injection tick, with their descendants. On `sample:31` only one seed has any |
| Lineage read | identical to #587. Every rate, birth and death count in the plain table matches #587's artifact, so the accountant is observer-side only |
| Unit | per **member-tick** (a member alive at a tick's start). Tables give the median over seeds of each seed's per-member-tick mean, over seeds with any member-tick. Counts are summed over seeds |

`sample:31`: world extent 39.5, nutrient cells 10 × 10 (16 cells), contact range coefficient 0.81,
body-reach coefficient 0, so reach is `h_eff × 0.81`. Solar flux 4.88, light radius 5.4, energy
reproduction threshold 42.4, nutrient threshold 1.

## 2. The account

`LineageAccountant` steps the fork one tick at a time. It snapshots every member (reserve,
structure, earmarks, wear, traits, position) and every carcass before the step. It then reads the
tick's events: `Photosynthesized`, `Metabolized`, `Grew`, `Consumed` (both directions), `Moved`,
`Reproduced` and `Born`. The grow phase emits no event for the earmark or for repair, so the
accountant replays it from the tick-start state, the photosynthesis and the metabolic charge.

| term | how it is booked |
|---|---|
| carcass ceiling | `h · u_H · e_c` per carcass reached (nominal `h`, each carcass's own `e_c`) |
| wear gap | `(h − h_eff) · u_H · e_c`, with `h_eff` from the replayed post-repair wear |
| exhaustion gap | `(d − min(d, E_c)) · e_c`: the carcass held less than the eater's demand `d` |
| sharing gap | `(min(d, E_c) − drain) · e_c`: co-consumers' proportional split |
| carcass gain | `drain · e_c`; the four above close to it exactly |
| photosynthesis | the event, against the unshaded ceiling (the flux: all the light, no neighbour) |
| maintenance | the six flow-8 terms at the tick-start state, scaled pro rata when a starving member's charge is capped |
| growth loss, repair, movement | `Grew` (`to_structure / growth_efficiency − to_structure`), the replay, `Moved` |
| grazed | structure drained off a member alive, and the part drained by other members |
| earmark fill, reproduction outlay | replayed `(1 − κ)` surplus, and the whole earmark of a member that reproduced |
| deaths | what the member held at death: to its carcass, and the rest dissipated (of which the stranded earmark) |
| births | the newborn's energy |

**Reconciliation.** Over every tick, the lineage's stock change equals the booked terms. The
booked photosynthesis, carcass gain, living gain, total outgo and birth endowment also equal the
stepper's own energy ledger restricted to the members. On the runs here the worst per-arm residual
is `2.8e-6` of throughput and the worst ledger gap `1.3e-5`. Two tests pin this in
`energy_accounting::tests` at `1e-4` of throughput: a constructed pile world (sharing and
exhaustion by construction) and a `sample:31` fork with deaths, births and reproduction.

A dying member's reserve at the moment of death is not observable. Its holdings are therefore
booked as the death terms, and the residual measures surviving members only.

## 3. `sample:31`: income

Median over seeds, per member-tick. `k*` is #589's break-even count on the resident producer's
carcass, `(base + h^x·c_H) / (h·u_H·e)`.

| arm | member-ticks | `k*` | `k` reached | carcasses in cell | P(k ≥ 1) | carcass ceiling | wear gap | exhaustion gap | sharing gap | carcass gain | living gain | photosynthesis (ceiling 4.88) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| step 0 / uniform | 6355 | 0.50 | 0.005 | 12.7 | 0.005 | 0.0003 | 0 | 0.0000 | 0 | 0.0003 | 0.0002 | 0.900 |
| step 0 / pile | 484 | 0.50 | 0.000 | 17.6 | 0.000 | 0 | 0 | 0 | 0 | 0 | 0 | 0.192 |
| step 1× / uniform | 2042 | 0.14 | 0.146 | 13.0 | 0.131 | 0.0108 | 0 | 0.0004 | 0.0000 | 0.0106 | 0.0040 | 1.082 |
| step 1× / pile | 708 | 0.14 | 0.145 | 17.9 | 0.114 | 0.0113 | 0 | 0.0005 | 0.0000 | 0.0108 | 0.0041 | 0.712 |
| full / uniform | 52 | 1.54 | 0.475 | 10.5 | 0.318 | 0.0142 | 0 | 0.0011 | 0 | 0.0126 | 0.0074 | 0 |
| full / pile | 50 | 1.54 | 0.500 | 17.1 | 0.391 | 0.0100 | 0 | 0.0012 | 0.0000 | 0.0082 | 0.0265 | 0 |

Co-consumers per carcass contact are 0.00–0.01 on every arm: nobody shares. Pooled over seeds
instead of taking medians, the reading is the same. For `step 1×` it gives `k` 0.114 / 0.201
(uniform / pile), carcass gain 0.010 / 0.016, sharing 0.00003 and exhaustion 0.0006–0.0007.

## 4. `sample:31`: upkeep and net

| arm | base | heterotrophy | #587 terms | photo maint. | asexual | structure | maintenance | growth loss | movement | upkeep | income | net | grazed (by kin) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| step 0 / uniform | 0.0117 | 0.0002 | 0.0119 | 0.042 | 0.003 | 0.121 | 0.176 | 0.077 | 0.098 | 0.362 | 0.902 | +0.437 | 0.0007 (0.0007) |
| step 0 / pile | 0.0117 | 0.0001 | 0.0118 | 0.040 | 0.003 | 0.020 | 0.077 | 0.006 | 0.020 | 0.105 | 0.192 | +0.092 | 0.0006 (0.0000) |
| step 1× / uniform | 0.0117 | 0.0029 | 0.0146 | 0.040 | 0.003 | 0.125 | 0.191 | 0.084 | 0.069 | 0.335 | 1.090 | +0.683 | 0.0085 (0.0084) |
| step 1× / pile | 0.0117 | 0.0030 | 0.0147 | 0.040 | 0.003 | 0.042 | 0.106 | 0.064 | 0.040 | 0.204 | 0.732 | +0.529 | 0.0052 (0.0048) |
| full / uniform | 0.0112 | 0.0216 | 0.0328 | 0 | 0.003 | 0.019 | 0.054 | 0.000 | 0.018 | 0.073 | 0.027 | −0.045 | 0.0055 (0.0000) |
| full / pile | 0.0113 | 0.0218 | 0.0331 | 0 | 0.003 | 0.019 | 0.055 | 0.000 | 0.019 | 0.073 | 0.034 | −0.039 | 0.0443 (0.0439) |

Upkeep is maintenance + growth loss + repair + movement. Repair and mobility maintenance are zero
on every arm. The medians of `net` are per-seed medians, not the difference of the columns.

## 5. `sample:31`: reproduction and death

Counts are summed over the 8 seeds. The energy gate is `earmark ≥ 42.4`, the nutrient gate
`nutrient earmark ≥ 1`. "Met" is the fraction of member-ticks at or above the gate.

| arm | earmark fill | earmark level | energy gate met | nutrient gate met | births | deaths | starved | grazed to death | infant deaths / grazed | stranded earmark |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| step 0 / uniform | 0.478 | 47.0 | 0.158 | 0.618 | 170 | 172 | 32 | 55 | 39 / 21 | 0.081 |
| step 0 / pile | 0.448 | 19.8 | 0.012 | 0.229 | 58 | 94 | 29 | 9 | 12 / 1 | 0.263 |
| step 1× / uniform | 0.840 | 20.4 | 0.031 | 0.741 | 186 | 230 | 16 | 186 | 153 / 148 | 0.169 |
| step 1× / pile | 0.716 | 19.9 | 0.012 | 0.809 | 66 | 125 | 24 | 81 | 63 / 59 | 0.228 |
| full / uniform | 1.938 | 10.2 | 0 | 0 | 0 | 64 | 57 | 6 | 0 / 0 | 1.938 |
| full / pile | 2.018 | 10.1 | 0 | 0 | 0 | 64 | 46 | 12 | 0 / 0 | 2.018 |

**The step.** `step 1×` fills its earmark faster than the producer (0.84 against 0.48 E per
member-tick) and gives birth as often (186 against 170). Four in five of those births die within
50 ticks, grazed to death, almost always by kin. A member's death strands its earmark. So the
earmark's mean level stays at half the threshold (20 against 42.4), and the energy gate is met on
3 % of member-ticks. The nutrient gate is not binding (74–81 % met). The energy the step spends on
reproduction goes into infants that its own lineage grazes.

A grazed-to-death member is one that was drained alive in the tick it died and was not starving:
the graze took its structure below the fragility threshold (`fragility × peak structure`). How
much energy was grazed (0.0085 E/tick) understates the cost, because a small graze kills a
newborn.

**The full heterotroph** is #587's story made explicit. Its `κ = 0.012` moves 1.94–2.02 E per
member-tick into an earmark that never reaches the threshold and is stranded at death. Its upkeep
(0.073) is more than twice its income (0.027–0.034), and #587's two terms alone (0.033) exceed that
income. Its realised `k` (0.48–0.50) is a third of its `k* = 1.54`. On the pile, the heterotrophs
graze each other (0.044 E/tick grazed, nearly all by kin), which adds deaths, not income.

## 6. The contrast: #462's live baselines

The same command on the three baselines. The row is the control fork's resident consumers (median
3, 2 and 3 at injection), with their descendants, over the window.

| config | seeds | member-ticks | `k` reached | carcasses in cell | carcass gain | living gain | photosynthesis (ceiling) | #587 terms | upkeep | net | births / deaths (infant, grazed) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `sample:110` | 8 | 2682 | 0.006 | 2.3 | 0.0026 | 0.0017 | 4.49 (4.61) | 0.534 | 3.17 | +1.00 | 62 / 66 (62, 28) |
| `sample:18` | 8 | 1452 | 0.000 | 0.0 | 0 | 0 | 9.20 (9.21) | 0.560 | 6.14 | +2.83 | 54 / 56 (54, 54) |
| `sample:188` | 8 | 2595 | 0.010 | 1.9 | 0.0035 | 0.0009 | 9.10 (9.44) | 0.540 | 7.43 | +1.55 | 230 / 237 (217, 156) |

In these worlds the "consumers" are mixotrophs that live on nearly unshaded light. Photosynthesis
is 97–100 % of the ceiling, in worlds of 4–18 agents. Their carcass and prey income is three
orders of magnitude smaller. They clear #589's negative margins (−0.39 to −0.47) because the
margin counts no photosynthesis, not because many carcasses sit in reach.

Their populations hold level (control rates −0.00013 to 0) by replacing infant deaths: 54 of 56
deaths on `sample:18` are infants grazed to death. Injected `step 1×` and `full` cohorts do not
invade there either (0/8 seeds positive on every arm and placement).

## 7. Reading

The issue's question was where the income the closed form promises goes, and three candidate
readings. The answers, sink by sink:

1. **Shared drain:** no. Co-consumers per carcass contact are ≤ 0.01. The sharing gap is under
   0.1 % of the ceiling.
2. **Reach, not presence:** yes. This is the whole gap between the `k = 1` ceiling and the realised
   carcass income. The pile puts 13–18 carcasses in the member's cell, but its reach disc is about
   0.25 % of the cell's area. Realised `k` is 0.15 and does not rise on the pile.
3. **Lost photosynthesis:** off the pile, no. The step photosynthesises at least as well as the
   producer. On the pile, yes, for every phenotype: shading takes the producer's light income from
   0.90 to 0.19 E/tick. That is #587's "the pile is worse for everyone".
4. **Finite carcass energy:** barely. The exhaustion gap is about 4 % of the ceiling for the step
   and about 8–12 % for the full heterotroph.
5. **Omitted upkeep:** large (14–23 times #587's terms), but photosynthesis pays it and the step's
   net is positive. Folding it into `heterotroph_margin` would make the margin more negative
   everywhere without separating anything. #589 already found that the margin predicts nothing,
   and it would still omit photosynthesis.
6. **Not listed in the issue: self-grazing.** Heterotrophy is indiscriminate. A near-producer that
   steps up its drain grazes its nearest living neighbours, and a dispersed newborn is one. This is
   what separates `step 1×` from `step 0`: 148 of its 186 births die grazed within 50 ticks,
   against 21 of 170 for the producer.

So the carcass economy is not crowded out, and the kernel is not what stops the near-producer. The
step's carcass income matches the break-even the closed form sets, via a realised `k` that happens
to equal `k*`. The step's cost is demographic. Nothing here bears on the #465 carcass floor:
exhaustion is not binding. The reach bound (`ῑ`) is the term that matters on the income side. A
larger realised `k` would take a larger reach, i.e. more heterotrophy, which also means more
self-grazing.

## 8. What this does not show

- One lockup config, one injection tick and one cohort size, as #587. The contrast configs were run
  with `sample:31`'s arms (their own resident centroids).
- Medians of per-seed means over seeds with member-ticks. Seeds where the lineage dies within a few
  ticks weigh the same as seeds where it persists. Pooled figures are quoted where they differ.
- "Grazed by kin" is measured in energy. Which grazer killed a given infant is not traced. The
  infant counts pair a death with any graze in its tick. That the killers are kin is inferred from
  kin taking 99 % of the energy grazed off `step 1×` members.
- A dying member's reserve at death is booked as a death term, not observed. The residual
  measures surviving members only.
- Nutrient flows are not booked. The nutrient gate is read only as met or not.

## 9. Instrument

- `crates/explorers-search/src/energy_accounting.rs` holds `LineageAccountant` (`new`, `step`,
  `account`, `lineage`, `alive`) and `EnergyAccount`. The accountant widens the fork's event
  retention to the kinds it reads, and compacts the log behind each tick.
- The tests are the two reconciliation pins (the constructed pile world and a `sample:31` fork),
  the photosynthesis-ledger match, and the carcass gap closure.
- `crates/explorers-search/src/bin/reinvasion_barrier.rs` gains `--accounting`. It runs the six
  named arms under the accountant, books the control fork's resident consumers, adds `k*` and the
  account to each record, and prints three tables with the worst residual and ledger gap. It writes
  `target/carcass-income-accounting.json`, chunks and merges like the plain mode, and leaves the
  plain mode's output unchanged.
- The mode's smoke test checks the arms, the accounts, the contrast account and determinism.
