# Issue #593: is self-grazing `sample:31`'s re-invasion barrier?

**Status: measurement. Adds a `--dispersal` mode to the `reinvasion_barrier` bin. No trajectory
changes. `explorers-sim`, the search, the evaluator and the prefilter are unchanged.**

[#591](591-carcass-income-accounting.md) found that a near-producer mixotroph (`step 1×`) injected
onto `sample:31` at tick 1000 has *higher* net energy than an unmodified producer cohort
(`step 0`). It still shrinks faster. The difference was demographic. 148 of its 186 births died
within 50 ticks, grazed to death, and kin took 99 % of the energy grazed off the lineage. #591
inferred the mechanism: heterotrophy grazes the nearest living neighbour, and a newborn dispersed
beside its parent is one. This note tests that inference by moving the newborns out of kin reach.

## TL;DR

1. **The manipulation works as intended.** Raising the injected cohort's `dispersal` from the
   resident's 0.34 to 4 cuts `step 1×`'s infant grazed deaths per birth from 0.80 to 0.21. For
   `step 2×` the fall is from 0.86 to 0.27. Energy grazed by kin falls by about two thirds for
   `step 1×` (0.0084 → 0.0029 E per member-tick). The energy gate is met 3.5 times as often
   (0.031 → 0.107). Dispersal costs nothing on `sample:31`, so every other term is held.
2. **Dispersal helps the steps more than the producer: the interaction is positive and grows
   with heterotrophy.** Paired per seed against `step 0` at the same dispersal:
   - `step 1×` goes from −0.00073 to −0.00033. Over half its gap to the producer closes.
   - `step 2×` goes from −0.00104 to **+0.00062**. At dispersal 4 it beats the producer, with 5/8
     seeds positive, 0/8 extinct, and a median lineage end of 14 against 9.
   - `step 0`'s own gain from dispersal is +0.00033.
3. **Nothing invades by the #587 criterion, `step 0` included.** The criterion is a median
   `r > 0` with the sign interval clear of zero, which at `n = 8` means 8/8 seeds positive. The
   resident itself declines (control rate −0.00058). The best arm, `step 2×` at dispersal 4, has
   a median `r` of +0.00056 on the interval [−0.00208, +0.00142].
4. **Self-grazing is not fully removed.** At dispersal 4, `step 1×` still loses 54 infants to
   grazing (`step 0`: 10). Kin still take most of what is grazed off it (0.0029 of 0.0046 E per
   member-tick). This note does not explain the remainder.
5. **Reading: between 1 and 2 of the issue.** Self-grazing is a real, causal part of the barrier.
   Taking it away closes most of the step's deficit, and at two steps it reverses the deficit. It
   is not sufficient for invasion into a declining resident at this cohort size and seed count.
   The lockup readout moves with it: `step 2×` at dispersal 4 draws the carcass pile down by a
   median 3,645 N against the control, and the free pool gains 292 N. That result comes from a
   single arm and is not monotone across levels (see §4).

## 1. What ran

`cargo run --release -p explorers-search --bin reinvasion_barrier -- sample:31 --dispersal`, on
branch `issue-593-self-grazing-dispersal` from `9a4423c`. Wall clock was about 20 s.

| | |
|---|---|
| Config, seeds, injection | as #587/#591: `sample:31`, seeds 1000–1007 (8/8 reached tick 1000), injection at tick 1000, window 1000, cohort 8 |
| Arms | `step 0`, `step 1×`, `step 2×` (resident producer centroid, heterotrophy raised by `k × mutation_magnitude = k × 0.312`), uniform placement |
| Dispersal | the phenotype's own (the resident centroid, median 0.344 at injection), then 1, 2 and 4. The offspring offset is `Normal(0, dispersal × DISPERSAL_KERNEL_SIGMA)`, with σ = 1 per unit. The step's consumption reach is ≈ 0.28 |
| Pairing | a raised-dispersal cohort is placed exactly where its own-dispersal twin is (same position stream, keyed on seed, phenotype and placement) |
| Accounting | every arm under #591's `LineageAccountant`. The infant, kin-grazing and gate figures are its |

On `sample:31`, `dispersal_propagule_cost_coefficient = 0`, so a wider dispersal kernel costs no
energy. One side effect is not corrected: `dispersal_reach_coefficient = 10` also widens sexual
mate-finding reach. It shows as cross births (with residents): 2 → 38 for `step 0` at dispersal 1,
and 10–30 for the steps at 1. At dispersal 4 there are no cross births on any arm. Offspring
inherit dispersal under mutation, so the raise persists down the lineage.

## 2. Invasion and demography

Counts summed over the 8 seeds. The paired `Δ` columns are the median over seeds of the per-seed
rate difference. The per-member-tick columns are medians over seeds.

| arm | dispersal | median r | r > 0 | Δ vs step 0 | Δ vs own dispersal | births pure / cross | deaths | grazed to death | infant / grazed | infant grazed per birth | grazed by kin (E / member-tick) | energy gate met | lineage end (med / max) | extinct |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| step 0 | 0.34 | −0.00076 | 3/8 | 0 | 0 | 168 / 2 | 172 | 55 | 39 / 21 | 0.12 | 0.0007 | 0.158 | 4.5 / 27 | 2 |
| step 0 | 1 | +0.00001 | 4/8 | 0 | +0.00013 | 214 / 38 | 223 | 59 | 82 / 25 | 0.10 | 0.0015 | 0.156 | 9 / 35 | 1 |
| step 0 | 2 | −0.00030 | 2/8 | 0 | +0.00019 | 263 / 9 | 257 | 60 | 112 / 42 | 0.15 | 0.0001 | 0.199 | 6 / 39 | 1 |
| step 0 | 4 | +0.00009 | 4/8 | 0 | +0.00033 | 314 / 0 | 283 | 21 | 141 / 10 | 0.03 | 0.0000 | 0.226 | 9 / 41 | 1 |
| step 1× | 0.34 | −0.00188 | 0/8 | −0.00073 | 0 | 176 / 10 | 230 | 186 | 153 / 148 | 0.80 | 0.0084 | 0.031 | 1.5 / 7 | 4 |
| step 1× | 1 | −0.00139 | 1/8 | −0.00132 | 0 | 110 / 10 | 151 | 96 | 74 / 66 | 0.55 | 0.0023 | 0.041 | 2 / 21 | 3 |
| step 1× | 2 | −0.00098 | 2/8 | −0.00060 | 0 | 184 / 9 | 211 | 105 | 100 / 64 | 0.33 | 0.0047 | 0.085 | 3 / 24 | 2 |
| step 1× | 4 | −0.00007 | 3/8 | −0.00033 | +0.00049 | 253 / 0 | 252 | 91 | 132 / 54 | 0.21 | 0.0029 | 0.107 | 7.5 / 21 | 2 |
| step 2× | 0.34 | −0.00208 | 1/8 | −0.00104 | 0 | 91 / 15 | 155 | 123 | 93 / 91 | 0.86 | 0.0102 | 0.042 | 1 / 9 | 3 |
| step 2× | 1 | −0.00173 | 0/8 | −0.00167 | 0 | 113 / 30 | 192 | 149 | 123 / 116 | 0.81 | 0.0067 | 0.058 | 1.5 / 7 | 3 |
| step 2× | 2 | −0.00073 | 2/8 | −0.00039 | +0.00150 | 165 / 3 | 194 | 127 | 122 / 97 | 0.58 | 0.0056 | 0.079 | 4 / 10 | 0 |
| step 2× | 4 | +0.00056 | 5/8 | +0.00062 | +0.00192 | 494 / 0 | 445 | 188 | 309 / 135 | 0.27 | 0.0058 | 0.069 | 14 / 33 | 0 |

The rows at the phenotype's own dispersal reproduce #591's figures for `step 0` and `step 1×`
exactly. The dispersal override leaves the own-dispersal arms unchanged.

**Ties.** A lineage extinct inside the window has `r = ln(½ / 8) / 1000 = −0.00277`, and several
seeds share it. That is the lower end of every interval. It is also why many paired `Δ` medians
are exactly 0: on most seeds both lineages died out.

## 3. Energy, briefly

Photosynthesis dominates the income of every arm (0.70–1.15 E per member-tick), as in #591, and
net energy is positive on every arm (+0.44 to +0.74). Dispersal does not change the economics in
one direction. `step 1×`'s carcass gain falls as it disperses (0.0106 → 0.0010 E per member-tick),
because its offspring leave the carcasses near the parent. It still gains. Energy was not what sank
the step, and dispersing it trades a little carcass income for recruitment.

## 4. Lockup readout

Carcass nutrient drawdown at the window end (control − arm, median over seeds):

| | own | 1 | 2 | 4 |
|---|---:|---:|---:|---:|
| step 0 | −231 | −148 | +644 | −540 |
| step 1× | +708 | +460 | +327 | +263 |
| step 2× | +1319 | +940 | −697 | **+3645** |

Against a control pile of about 40,500 N, these are single-digit percentages, of varying sign, on
8 seeds. Only `step 2×` at dispersal 4, the one arm whose lineage grows, stands out. It also moves
the free pool (+292 N, against −3 to +1 elsewhere). One arm is not a trend.

## 5. Reading

Against the issue's three readings:

1. **Self-grazing is the barrier:** partly. Dispersal cuts grazed infant deaths toward `step 0`'s,
   and it helps the steps more than the producer. The interaction is +0.00040 for `step 1×` and
   +0.00166 for `step 2×`, rising with heterotrophy as self-grazing predicts. At two steps it turns
   the deficit into a lead. Neither step *invades* by the #587 sign test.
2. **Real but not sufficient:** this fits best for `step 1×`. The next constraints are the ones
   the issue named:
   - **A declining resident.** No arm, `step 0` included, clears 8/8 positive seeds.
   - **Residual kin grazing.** At dispersal 4, infant grazing per birth is still 7× `step 0`'s
     (0.21 against 0.03).
   - **Not a binding energy gate.** The gate improves with dispersal (0.031 → 0.107) but stays
     below `step 0`'s 0.158–0.226.
3. **Not self-grazing:** refuted. Infant grazing falls monotonically with dispersal for `step 1×`
   (0.80, 0.55, 0.33, 0.21), and the rate follows.

So #591's inference holds causally: kin-blind grazing of newborns is a real part of why selection
cannot climb from producer toward heterotroph on `sample:31`. A lineage that escapes it by
dispersal gets further. That makes the design question the issue kept out of scope a live one:
whether heterotrophy should stay indiscriminate (see §6).

## 6. What this does not show

- **One config, one injection tick, one cohort size, 8 seeds.** The per-level trends are not
  monotone everywhere: `step 2×` is worse at dispersal 1 than at its own. The sign test at `n = 8`
  cannot call a partial effect.
- **Where the residual kin grazing comes from.** At dispersal 4 (σ ≈ 14× reach) a newborn should
  rarely land in a relative's reach. The kin share of what is grazed off `step 1×` falls from 99 %
  to 63 %, but it does not vanish. Candidates, none traced:
  - mobile relatives moving into reach;
  - siblings born in the same tick landing together;
  - density, since the larger lineage is more likely to meet itself.
- **Mate-finding reach.** At dispersal 1 it rises with dispersal (cross births up to 38). At
  dispersal 4 cross births are 0.
- **Whether real selection would raise dispersal.** Dispersal is free here, and the resident sits
  at 0.34 anyway. Why selection has not raised it is not tested.
- **A kin-aware grazing kernel was not tested.** That is a sim change and a design decision.

## 7. Instrument

`crates/explorers-search/src/bin/reinvasion_barrier.rs`:

- `Arm` gains `dispersal_centi: Option<u32>`, which overrides the cohort's dispersal trait in
  hundredths. It is left out of the artifact when `None`, so earlier artifacts read back and merge
  unchanged.
- The cohort position tag ignores the override, so a raised-dispersal cohort is paired with its
  own-dispersal twin.
- `dispersal_arms()` crosses `step 0/1/2` with `DISPERSAL_LEVELS_CENTI = [100, 200, 400]` at
  uniform placement.
- `--dispersal` runs those arms under the energy accountant. It writes
  `target/self-grazing-dispersal.json` and prints the plain and accounting tables, plus the
  dispersal table with the two paired contrasts (`paired_delta`).
- `Injection` records `cohort_positions`.
- The tests:
  - the dispersal smoke test: only dispersal changes, positions are paired, determinism;
  - the arm serialisation round trip;
  - CLI parsing (`--dispersal` is exclusive with `--accounting`).
