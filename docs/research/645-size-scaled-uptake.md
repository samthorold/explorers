# Issue #645: size-scaled uptake on the atlas

**Status: measurement, docs only. Adds two flags and a persistence readout to the observer-only
diagnostic `kin_killer_diet` (`explorers-search`). `explorers-sim`, the search, the evaluator and the
prefilter are unchanged. No design decision is taken here: the recommendation in §9 and the
questions in §10 go to genesis and #647.**

#642 found that light-fed mixotrophs are a third of all agents on the ungated atlas, retain 96–97 %
of all carcass nutrient retained, and kill their kin because kin are what is in reach. It traced
the nutrient hunger behind this to one asymmetry: light share scales with structure, and uptake did
not. #644 added the missing term, latent and default off. Uptake demand becomes
`effective autotrophy × u_A × (structure / s_ref)^b`. This note switches it on across the atlas and
asks whether letting uptake grow with the body ends large producers' nutrient shortage, and with it
their carcass dependence and kin-killing.

## TL;DR

1. **It thins mixotrophy; it does not end its carcass dependence** (§3, §4). Producers with
   `h_eff > 0.05` fall from 34 % to 15 % (decoded) and from 35 % to 22 % (fa 0) at `b = 1`. Those
   with `h_eff > 0.5` fall from 19 % to 5 % and from 16 % to 8 %. But carcasses still supply
   85–92 % of the nutrient light-fed mixotrophs retain from drains, against 88–92 % at `b = 0`.
   The most heterotrophic producers are not relieved either: at `h_eff ≥ 1`, 76–83 % are N-limited
   at `b = 1` (decoded), against 65 % at `b = 0`, and their uptake barely moves.
2. **Appetite now falls over the run** (§6). At `b ≥ 2/3`, light-fed mixotrophs' mean `h_eff` falls
   from the first quarter to the last: 0.61 → 0.47 decoded and 0.61 → 0.48 at fa 0, at `b = 1`.
   At `b = 0` it rose (0.61 → 0.69, 0.55 → 0.62). At `b = 1/3` it still rises.
3. **The detrital position stays with producers** (§5). Light-fed mixotrophs retain 92–97 % of
   carcass nutrient retained in every setting. Decomposers' share rises from 2.2 % to 2.9–4.6 %
   (decoded) and from 1.6 % to 1.4–3.5 % (fa 0). The decomposer guild stays at 0.2–0.5 % of agents.
   What changes is the heterotrophs' own budgets: small bodies take up little, so decomposers draw
   45–68 % of their nutrient from carcasses at `b ≥ 2/3` (10 % at `b = 0`), and consumers 43–61 %
   from living prey (12 %).
4. **Kin-killing per producer halves or more; absolute kin-killing does not fall** (§7). Per 1000
   Producer agent-ticks over the whole run: 2.14 → 0.74–0.78 decoded, 2.54 → 1.01–1.06 at fa 0, at
   `b = 1`. But worlds get denser, so in the second half absolute kin kills double decoded
   (2,592 → 5,091 / 5,139) at `b = 1`, and rise by 18–29 % at fa 0. Per light-fed mixotroph the rate barely
   moves. The fall per producer is mostly fewer mixotrophs, not gentler ones.
5. **Persistence holds, but the failure mode changes** (§8). Persisted seeds go from 403 to
   405–416 of 475 (decoded) and from 423 to 422–435 (fa 0) at `b ≥ 2/3`. Nutrient lockup falls from
   40–41 seeds to 5–9 at `b = 1`; monoculture and generalist dominance rise from 3 to 25–35. The
   atlas was searched at `b = 0`, so these are configs tuned for the other rule.
6. **The exponent drives the result, not the total supply** (§9). At each `b`, the default
   reference (`s_ref = 100`) and the demand-preserving one give results closer to each other than to
   the neighbouring `b`.

**Recommendation** (§10): put `uptake_structure_exponent` in genesis's `default_ranges` over
`[0, 1]`, and keep `uptake_reference_structure` fixed at 100. Size-scaled uptake does not answer
#647 on its own: producers still hold the decomposer niche at every `b`.

## 1. What ran

| | |
|---|---|
| Tree | `fedc4b8` (branch `issue-645-uptake-measurement`), release build. |
| Diagnostic | `kin_killer_diet` on the atlas (`atlas.json`, 95 configs), seeds 1000–1004 (475 seeds), T = 2000, `EvalConfig::default()`, **ungated** as in #642, both as decoded and at `--founder-aggregation 0`. |
| Settings | 7 × 2 runs; the flags override `uptake_structure_exponent` (`b`) and `uptake_reference_structure` (`s_ref`) on every decoded config. |
| Check | At `b = 0` every table reproduces #642's byte for byte (sections 1–3 and A–D); only the new header line, the kin-kill rate and section E are added. |
| Machine | 8-core laptop, one process at a time. |

**Settings.** #644 chose `s_ref = 100`. Over the atlas at `b = 0`, the reference that leaves total
producer demand unchanged is about 40 at `b = 1/3`, 85 at `b = 2/3` and 140 at `b = 1` (world rules,
*Size-scaled uptake*). So the runs form two series. The first moves `b` at the default reference,
which also changes total demand. The second moves `b` at fixed total demand, which isolates the
reallocation of demand toward large bodies.

| label | `b` | `s_ref` | total demand vs `b = 0` |
|---|---:|---:|---:|
| b0 | 0 | 100 | 1 |
| b⅓·100 | 1/3 | 100 | 0.74× |
| b⅔·100 | 2/3 | 100 | 0.90× |
| b1·100 | 1 | 100 | 1.40× |
| b⅓·40 | 1/3 | 40 | ≈ 1 |
| b⅔·85 | 2/3 | 85 | ≈ 1 |
| b1·140 | 1 | 140 | ≈ 1 |

Tables below list them in that order: b0, then the `s_ref = 100` series, then the demand-preserving
series.

**Commands** (cwd is the repo root; driver `target/645/run.sh`):

```sh
cargo build --release -p explorers-search --example kin_killer_diet
C=$(seq -s, -f 'atlas:%g' 0 94)
# per setting, e.g. b = 1, s_ref = 140; repeat with --founder-aggregation 0 for fa0
./target/release/examples/kin_killer_diet --atlas atlas.json --configs $C \
    --uptake-structure-exponent 1 --uptake-reference-structure 140 \
    --out target/645/b1-s140-decoded.jsonl > target/645/b1-s140-decoded.md
```

`b = 1/3` and `2/3` are passed as `0.33333334` and `0.6666667`. b0 passes no uptake flag.

**Wall clock** (`target/645/times.log`; seconds, decoded / fa 0): b0 49 / 101; b⅓·100 148 / 162;
b⅔·100 248 / 243; b1·100 168 / 172; b⅓·40 98 / 168; b⅔·85 220 / 297; b1·140 137 / 172. In total
2,383 s, about 40 min. Runs at `b > 0` take 2–3× longer than b0, because the worlds hold more
agents (§3).

**Populations and definitions** are #642's (its §1). In brief: a **light-fed mixotroph** in
sections 1–2 and the kin-kill counts is a Producer-role agent with heterotrophy above zero. Sections
A–D use the stricter `h_eff > 0.05`. A **pure producer** is a producer with `h_eff ≤ 0.05`. **% N-limited** is
the census's `Surplus` nutrient below energy. Section E is new: each seed's verdict from the census
(`role_diet::census_failure`), exactly as `role_diet` reads it.

## 2. Diet and reach (sections 1–2)

These barely move, so they are summarised. Carcasses supply 85–92 % of the drain nutrient light-fed
mixotrophs retain in every setting (88.5 / 92.3 % at b0; 88.3 / 88.1 % at b1·100; 85.5 / 87.4 % at
b1·140; decoded / fa 0). Living kin give 35–45 % of their drain energy throughout. At 83–87 % of kin
kills, kin are the only living things in reach, against 80 / 85 % at b0. Size-scaled uptake does not
change what a light-fed mixotroph eats, or why it eats kin.

## 3. Prevalence (section A)

Second-half sample ticks, every living agent:

| setting | producers, % `h_eff > 0.05` | % `h_eff > 0.5` | producer `h_eff` p90 | consumer % | decomposer % | producer samples |
|---|---:|---:|---:|---:|---:|---:|
| *decoded* b0 | 34.0 | 18.6 | 0.865 | 0.1 | 0.3 | 194,771 |
| b⅓·100 | 22.5 | 10.1 | 0.505 | 0.1 | 0.4 | 365,503 |
| b⅔·100 | 20.9 | 7.1 | 0.345 | 0.1 | 0.4 | 529,274 |
| b1·100 | 15.1 | 4.9 | 0.185 | 0.1 | 0.2 | 829,695 |
| b⅓·40 | 28.2 | 14.2 | 0.645 | 0.1 | 0.5 | 262,788 |
| b⅔·85 | 18.9 | 6.7 | 0.315 | 0.1 | 0.3 | 567,637 |
| b1·140 | 15.6 | 5.2 | 0.195 | 0.1 | 0.2 | 894,396 |
| *fa 0* b0 | 34.7 | 16.1 | 0.815 | 0.2 | 0.4 | 494,851 |
| b⅓·100 | 27.7 | 11.6 | 0.595 | 0.1 | 0.3 | 605,140 |
| b⅔·100 | 25.2 | 9.9 | 0.495 | 0.1 | 0.4 | 826,497 |
| b1·100 | 21.8 | 7.7 | 0.395 | 0.1 | 0.2 | 1,094,106 |
| b⅓·40 | 31.0 | 13.0 | 0.675 | 0.1 | 0.4 | 517,107 |
| b⅔·85 | 22.7 | 8.6 | 0.425 | 0.1 | 0.4 | 832,082 |
| b1·140 | 21.2 | 7.3 | 0.365 | 0.1 | 0.2 | 1,230,501 |

- **Mixotrophy thins monotonically with `b`.** The share above 0.5 falls by a factor of 3.6–3.8
  decoded and 2.1–2.2 at fa 0 at `b = 1`. The upper tenth of producers no longer carries more
  heterotrophy than the median heterotroph (decoded p90 0.19 at `b = 1`, against consumer and
  decomposer medians of about 0.4–0.6).
- **The decomposer guild does not grow as a share.** Decomposers stay at 0.2–0.5 % of agents and
  consumers at 0.1 %. In absolute samples decomposers roughly quadruple decoded (507 → 1,939–2,094 at
  `b = 1`), but only because the worlds hold up to 4.6× more producers.
- **Worlds get denser.** Second-half producer samples rise 4.3–4.6× decoded and 2.2–2.5× at fa 0
  at `b = 1`. Births per 1000 producer agent-ticks rise from 5.6 to 7.0–10.2 decoded.

## 4. Conditionality (section B)

Second half, per agent-sample means. Pool is the nutrient-grid cell under the agent; uptake is pool
uptake per tick; "N from carcass" is the carcass-drain share of nutrient acquired.

| setting | all producers: % N-limited | pure: % N-limited | pure: pool | pure: uptake | `h_eff ≥ 1`: % N-limited | `≥ 1`: pool | `≥ 1`: light E | `≥ 1`: uptake | `≥ 1`: N from carcass % |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| *decoded* b0 | 37.2 | 30.4 | 206 | 0.277 | 64.5 | 286 | 10.2 | 0.212 | 12.5 |
| b⅓·100 | 26.3 | 20.3 | 281 | 0.236 | 61.3 | 263 | 11.1 | 0.188 | 9.1 |
| b⅔·100 | 37.6 | 34.3 | 492 | 0.239 | 67.3 | 291 | 10.1 | 0.186 | 9.6 |
| b1·100 | 58.8 | 58.7 | 724 | 0.153 | 76.1 | 379 | 10.3 | 0.222 | 4.8 |
| b⅓·40 | 28.6 | 24.6 | 302 | 0.334 | 54.4 | 228 | 10.8 | 0.190 | 10.5 |
| b⅔·85 | 31.4 | 27.5 | 435 | 0.231 | 64.5 | 320 | 10.3 | 0.207 | 8.1 |
| b1·140 | 65.5 | 65.7 | 801 | 0.118 | 82.9 | 532 | 9.5 | 0.257 | 4.8 |
| *fa 0* b0 | 33.0 | 29.7 | 113 | 0.198 | 60.2 | 274 | 10.8 | 0.263 | 20.8 |
| b⅓·100 | 28.7 | 24.7 | 291 | 0.239 | 58.4 | 227 | 10.7 | 0.221 | 11.5 |
| b⅔·100 | 34.1 | 31.4 | 405 | 0.210 | 59.4 | 239 | 10.4 | 0.219 | 9.7 |
| b1·100 | 51.9 | 51.3 | 684 | 0.165 | 64.0 | 358 | 10.5 | 0.284 | 6.0 |
| b⅓·40 | 28.0 | 24.2 | 193 | 0.248 | 49.3 | 181 | 11.0 | 0.184 | 12.7 |
| b⅔·85 | 32.9 | 30.2 | 356 | 0.196 | 56.5 | 218 | 11.1 | 0.219 | 23.0 |
| b1·140 | 57.6 | 57.1 | 709 | 0.129 | 69.0 | 481 | 9.8 | 0.330 | 6.9 |

(The full table, with every bin, births and lifespans, is in `target/645/*.md`.)

- **Large heterotrophic producers are not relieved.** At `h_eff ≥ 1` the N-limited share falls
  clearly only at `b = 1/3` with the demand-preserving reference (64.5 → 54.4 %; 60.2 → 49.3 % at
  fa 0). At `b = 1` it rises, to 76–83 %
  decoded and 64–69 % at fa 0. Their uptake per tick stays at 0.18–0.33, as at b0 (0.21 / 0.26),
  and their light income stays at about 10. Either these agents are not much larger than `s_ref`,
  or something besides demand caps their uptake. The diagnostic does not record structure, so it
  cannot say which (§11).
- **At `b = 1` limitation spreads to the small.** Pure producers go from 30 % to 51–66 % N-limited,
  while the pool under them triples or more (206 → 724–801 decoded, 113 → 684–709 at fa 0). Their
  uptake falls (0.28 → 0.12–0.15 decoded). This is what the rule says a small body does at `b = 1`:
  a producer at the median structure (about 8) has about a twelfth of its size-blind demand at
  `s_ref = 100`. Nutrient sits in the pool because small bodies cannot take it up.
- **Mixotrophy becomes relatively conditional on poor ground, but not on poor ground in absolute
  terms.** At b0 the most heterotrophic producers sat on the richest pools. With `b > 0` the order
  has turned by `b = 1/3` in both modes (fa 0: 227 against 291), and at `b = 1` the `h_eff ≥ 1` bin
  sits on 379 / 358 against 724 / 684 for pure producers. In absolute terms the picture is mixed. At
  `b = 1/3` and `2/3` the top bin's pool is similar to b0's or poorer (fa 0: 181–239 against 274),
  while pure producers' pools get richer. At `b = 1` every bin's pool is richer than at b0.
  Throughout, the heterotrophic bins remain the most N-limited.
- **Carcasses matter less to every producer bin.** At `h_eff ≥ 1`, carcass drains supply 4.8–6.9 %
  of nutrient at `b = 1`, against 12.5 / 20.8 % at b0. Across all producers the carcass share falls
  from 4.8 to 2.8–3.6 % decoded, and from 10.4 to 4.1–6.5 % at fa 0. (fa 0's b⅔·85, at 23.0 %, is
  the exception, on 26,490 samples.) Uptake still supplies most of every bin's nutrient, as at b0.
- **Light income falls for pure producers** (5.1 → 2.3 decoded, 4.6 → 2.4 at fa 0, at `b = 1`),
  because the denser worlds share the same flux among more bodies.

## 5. Who processes the detritus (section C)

Whole run, recipient by role at the start of the tick:

| setting | % of carcass N retained: light-fed mixotroph | decomposer | consumer | decomposers' own N from carcass % | consumers' own N from living drains % | all agents: N from carcass % |
|---|---:|---:|---:|---:|---:|---:|
| *decoded* b0 | 96.1 | 2.2 | 1.0 | 9.9 | 12.4 | 2.8 |
| b⅓·100 | 95.7 | 2.9 | 0.7 | 20.7 | 20.8 | 2.8 |
| b⅔·100 | 94.6 | 3.3 | 0.9 | 45.2 | 45.6 | 2.8 |
| b1·100 | 94.7 | 3.2 | 1.1 | 64.2 | 56.2 | 2.7 |
| b⅓·40 | 95.8 | 2.9 | 0.6 | 21.2 | 18.5 | 2.8 |
| b⅔·85 | 94.5 | 3.6 | 0.9 | 46.3 | 43.5 | 2.7 |
| b1·140 | 92.0 | 4.6 | 1.7 | 67.4 | 60.5 | 2.3 |
| *fa 0* b0 | 97.4 | 1.6 | 0.5 | 9.0 | 12.3 | 5.2 |
| b⅓·100 | 96.7 | 1.9 | 0.5 | 24.1 | 26.0 | 4.4 |
| b⅔·100 | 94.4 | 3.5 | 0.7 | 53.5 | 45.0 | 4.0 |
| b1·100 | 94.6 | 2.2 | 0.8 | 63.0 | 57.0 | 3.5 |
| b⅓·40 | 97.6 | 1.4 | 0.5 | 18.7 | 22.6 | 4.6 |
| b⅔·85 | 95.8 | 2.4 | 0.8 | 45.7 | 44.8 | 4.4 |
| b1·140 | 95.6 | 2.6 | 0.8 | 67.8 | 57.7 | 3.4 |

The second half is the same: light-fed mixotrophs retain 91–98 % and decomposers 1.5–5.1 %.

- **Carcass processing does not move to decomposers.** Their share of carcass nutrient retained rises
  by one to two points at most. Light-fed mixotrophs hold over 92 % in every setting.
- **Heterotrophs now live by heterotrophy.** At b0, decomposers and consumers took 80–90 % of their
  nutrient by pool uptake, like producers. With `b > 0` their small bodies take up little, so
  decomposers get most of theirs from carcasses and consumers from living prey. That is closer to
  the domain's division of labour, but it is a change in the heterotrophs' budgets, not in who
  handles the carcass pile.
- **Less nutrient passes through carcasses in total.** Carcass nutrient released by bites falls from
  534k to 216k (b1·100) and 163k (b1·140) over the whole run decoded, and pool uptake carries 96–97 %
  of nutrient into life throughout. The carcass share of all nutrient acquired falls at fa 0 (5.2 →
  3.4–3.5 % at `b = 1`) and is flat decoded.

## 6. Appetite over time (section D)

| setting | light-fed mixotrophs' mean `h_eff`, Q1 → Q4 | all producers, % `h_eff > 0.5`, Q1 → Q4 |
|---|---|---|
| *decoded* b0 | 0.606 → 0.694 | 13.7 → 19.2 |
| b⅓·100 | 0.575 → 0.616 | 11.6 → 10.4 |
| b⅔·100 | 0.550 → 0.491 | 10.2 → 7.5 |
| b1·100 | 0.614 → 0.470 | 14.6 → 4.8 |
| b⅓·40 | 0.580 → 0.649 | 12.0 → 16.2 |
| b⅔·85 | 0.550 → 0.501 | 9.9 → 6.7 |
| b1·140 | 0.629 → 0.450 | 16.1 → 5.0 |
| *fa 0* b0 | 0.548 → 0.622 | 13.4 → 16.5 |
| b⅓·100 | 0.540 → 0.581 | 11.8 → 12.6 |
| b⅔·100 | 0.542 → 0.526 | 12.3 → 10.3 |
| b1·100 | 0.610 → 0.483 | 16.1 → 7.4 |
| b⅓·40 | 0.539 → 0.584 | 11.8 → 14.1 |
| b⅔·85 | 0.530 → 0.525 | 11.6 → 9.3 |
| b1·140 | 0.638 → 0.492 | 18.3 → 7.0 |

The first quarter is about the same in every setting, because the founders are the same. From
there, at `b = 1/3` heterotrophy still rises, though less than at b0. At `b = 2/3` it is flat to
falling, and at `b = 1` it falls clearly: the share of producers above 0.5 drops by a half to two
thirds. That is the direction of the *Plodia* result (Boots et al. 2021; [cannibalism and
kin](../ecology/cannibalism-and-kin.md)), which b0 contradicted. As in #642, the quarters are
population snapshots, so selection and survivorship are mixed.

## 7. Kin kills, rate and absolute

Kin kills by light-fed mixotrophs (section 1's population), one per killing pair. The rate is per
1000 Producer-role agent-ticks. "Per mixotroph" divides second-half kin kills by Producer
agent-ticks × section 1's share of producers (from the sample ticks, so approximate).

| setting | rate, whole run | rate, second half | kin kills, whole run | kin kills, second half | Producer agent-ticks, whole / second half (M) | per mixotroph, second half |
|---|---:|---:|---:|---:|---:|---:|
| *decoded* b0 | 2.14 | 1.33 | 13,045 | 2,592 | 6.10 / 1.95 | 3.3 |
| b⅓·100 | 1.50 | 0.97 | 13,740 | 3,552 | 9.14 / 3.65 | 3.7 |
| b⅔·100 | 1.19 | 0.93 | 14,888 | 4,919 | 12.47 / 5.30 | 3.3 |
| b1·100 | 0.78 | 0.61 | 11,667 | 5,091 | 15.05 / 8.30 | 3.0 |
| b⅓·40 | 1.65 | 1.12 | 12,561 | 2,938 | 7.61 / 2.63 | 3.4 |
| b⅔·85 | 1.09 | 0.83 | 13,903 | 4,683 | 12.71 / 5.67 | 3.4 |
| b1·140 | 0.74 | 0.57 | 11,412 | 5,139 | 15.36 / 8.94 | 2.9 |
| *fa 0* b0 | 2.54 | 1.41 | 33,983 | 6,957 | 13.39 / 4.95 | 3.5 |
| b⅓·100 | 1.78 | 0.98 | 28,428 | 5,914 | 15.93 / 6.06 | 2.9 |
| b⅔·100 | 1.51 | 0.90 | 29,301 | 7,418 | 19.41 / 8.27 | 2.9 |
| b1·100 | 1.06 | 0.75 | 23,736 | 8,177 | 22.30 / 10.95 | 2.7 |
| b⅓·40 | 2.13 | 1.29 | 30,999 | 6,651 | 14.53 / 5.18 | 3.6 |
| b⅔·85 | 1.44 | 0.84 | 28,727 | 6,993 | 19.97 / 8.33 | 3.0 |
| b1·140 | 1.01 | 0.73 | 23,567 | 8,972 | 23.31 / 12.31 | 2.7 |

The three columns answer different questions:

- **Per producer, kin-killing falls by 58–65 % at `b = 1`**, monotonically in `b`. A producer
  picked at random is far less likely to be killing kin.
- **In absolute terms, it does not fall.** Over the whole run, decoded kin kills are flat
  (11,412–14,888 against 13,045), and at fa 0 they fall by up to 31 %. In the second half they rise
  in every decoded setting, to about twice b0 at `b = 1`, and at fa 0 they rise at `b ≥ 2/3` by up
  to 29 %. The worlds hold more producers: 2.5× (decoded) and 1.7× (fa 0) the agent-ticks over the
  run, and 4.3–4.6× and 2.2–2.5× in the second half. A denser world has more kin within reach of one
  another.
- **Per light-fed mixotroph, the rate barely moves**: 2.9–3.7 against 3.3 decoded, and 2.7–3.6
  against 3.5 at fa 0. The fall per producer is mostly a fall in the share of producers that are
  mixotrophs. Those that remain kill kin about as often as before.

## 8. Persistence and failure modes (section E)

Census verdicts over 475 seeds:

| setting | persisted | extinction | energy death | nutrient lockup | monoculture | generalist dominance |
|---|---:|---:|---:|---:|---:|---:|
| *decoded* b0 | 403 | 28 | 1 | 40 | 1 | 2 |
| b⅓·100 | 405 | 28 | 1 | 40 | 0 | 1 |
| b⅔·100 | 416 | 25 | 1 | 27 | 5 | 1 |
| b1·100 | 412 | 26 | 3 | 9 | 18 | 7 |
| b⅓·40 | 399 | 21 | 0 | 50 | 4 | 1 |
| b⅔·85 | 405 | 24 | 1 | 32 | 9 | 4 |
| b1·140 | 408 | 28 | 3 | 5 | 20 | 11 |
| *fa 0* b0 | 423 | 8 | 0 | 41 | 1 | 2 |
| b⅓·100 | 422 | 7 | 0 | 38 | 5 | 3 |
| b⅔·100 | 432 | 2 | 0 | 24 | 14 | 3 |
| b1·100 | 432 | 3 | 0 | 5 | 26 | 9 |
| b⅓·40 | 412 | 7 | 0 | 52 | 2 | 2 |
| b⅔·85 | 422 | 5 | 1 | 36 | 9 | 2 |
| b1·140 | 435 | 2 | 0 | 7 | 25 | 6 |

- **The net count barely moves; the composition does.** At `b = 1` nutrient lockup nearly
  vanishes (40 → 5–9, 41 → 5–7), and monoculture and generalist dominance rise from 3 seeds to
  25–31 decoded and 31–35 at fa 0. The census checks the dead-pool gate first, so some of these are
  worlds that would have locked up and now fail the diversity check instead.
- **Lockup worlds are rescued.** Configs that locked up at b0 persist at `b ≥ 2/3`: decoded
  atlas:22 goes from 1 of 5 seeds to 5 in all four such settings, atlas:3 from 2 to 4–5, and
  atlas:39 from 1 to 4 at `s_ref = 100`. At fa 0 atlas:22 goes from 1 to 5, and atlas:12 from 2 to 5
  at `b = 1`.
- **Monoculture spreads across configs, with a few repeat offenders.** At b1·100, 18 configs
  (decoded) and 23 (fa 0) have a monoculture or generalist-dominance seed, against at most 3 at b0.
  atlas:62, :18 and :50 have monoculture seeds in four or more of the six `b > 0` settings in at
  least one mode, and at worst lose two to four persisted seeds against b0.
- **Per config, verdicts change in about 40 % of configs.** At b1·100, 23 configs gain persisted
  seeds and 20 lose them decoded; 22 gain and 16 lose at fa 0. Most changes are one seed. Any
  parameter change sends each seed down a different stochastic path, so a one-seed change is
  within noise. Five configs lose two seeds or more at b1·100 in each mode.
- **`b = 1/3` at the demand-preserving reference is the worst setting for lockup** (50 / 52
  seeds) and for persistence (399 / 412). It is also the setting that keeps the most mixotrophy
  (§3).

## 9. Reallocation against total supply

The two series separate the reallocation of demand toward large bodies (`b`) from the change in
total demand (`s_ref` at fixed `b`). The demand-preserving series holds supply at b0's. The
`s_ref = 100` series supplies 0.74×, 0.90× and 1.40× as much.

- **`b` is what matters.** At each `b`, the pair of runs agree more with each other than with the
  next `b`. Producers above 0.5: 10.1 / 14.2 at `b = 1/3`, 7.1 / 6.7 at 2/3, 4.9 / 5.2 at 1
  (decoded). Kin-kill rate: 1.50 / 1.65, 1.19 / 1.09, 0.78 / 0.74. Lockup seeds: 40 / 50, 27 / 32,
  9 / 5. The appetite trend turns down at `b = 2/3` in both series.
- **Supply moves things a little, the way one expects.** At `b = 1/3`, the 0.74× supply of
  `s_ref = 100` thins mixotrophy further than the demand-preserving 40 (22.5 against 28.2 % above
  0.05). At `b = 1`, the 1.40× supply of `s_ref = 100` leaves fewer producers N-limited than the
  demand-preserving 140 (58.8 against 65.5 % decoded; 51.9 against 57.6 % at fa 0). At `b = 2/3`
  the two references are close (100 against 85), and so are the results.

So the effects are from reallocation toward large bodies, not from a change in how much nutrient
producers can take up in total.

## 10. Recommendation for genesis's search box

**Put `uptake_structure_exponent` in `default_ranges` over `[0, 1]`. Keep
`uptake_reference_structure` out of the box, fixed at 100.**

- **Why `b` belongs in the box.** It changes the world's ecology, not only its numbers: mixotrophy
  thins, appetite turns down, lockup gives way to monoculture, and heterotrophs come to live by
  heterotrophy. All of it is monotone in `b`. Persistence on configs tuned for `b = 0` is about
  equal or better at `b ≥ 2/3`, so the search has no reason to avoid it. What it costs is a different
  failure mode, and the census already scores monoculture and generalist dominance as failures,
  so the search can select against them. The atlas was searched at `b = 0` (§11): only a search
  with `b` in the box can tell whether worlds exist that keep the lockup relief without the
  diversity collapse.
- **Why `[0, 1]`.** The range keeps `b = 0`, so every current world stays reachable. Every effect
  measured here is still moving at 1, and the domain's root-to-shoot scaling sits in this range
  (world rules, *Size-scaled uptake*). Above 1, a large body would take up more per unit structure
  than a small one. Nothing here supports that.
- **Why not `s_ref`.** At fixed `b` it moves results little compared with `b` itself (§9), and it
  would add a dimension the search would spend samples on. 100 is the value #644 chose, and
  at it total demand stays within 0.74–1.40× of b0's over the whole range.
- **What the box change costs.** `decode` gains a 33rd raw coordinate (24 world parameters).
  Existing atlases record the box they were searched under and keep decoding over it
  ([genesis search](../system-design/genesis-search.md), *The search box*). A new atlas is needed
  to see `b` in action.

Not recommended: changing the default of `b`. Its default stays 0 until a search with `b` in the
box shows which values genesis's worlds settle on.

## 11. What this means for #647

#647 asks whether light-fed mixotrophs should hold the decomposer niche. Size-scaled uptake was the
cheapest candidate cause from #642's §7. It is not sufficient.

- **Producers still hold the decomposer niche.** At every `b`, light-fed mixotrophs retain over
  92 % of carcass nutrient retained, and carcasses still supply 85–92 % of their drain nutrient.
  There are fewer of them, but the carcass pile is still theirs. The remaining causes from #642's
  §6 are untouched here: the carcass drain has no decomposer term and no autotrophy cost, a
  carcass carries all its nutrient, and the network route is off.
- **Mixotrophy is more conditional, but not yet conditional on nutrient.** It is rarer, it now
  declines over the run, and at `b > 0` the most heterotrophic producers stand on poorer pools than
  pure producers. But that is relative: pure producers' ground got richer, while the top bin's
  pool fell only at `b ≤ 2/3`. The heterotrophic bins remain the most N-limited, and the most
  heterotrophic producers' uptake does not rise. The domain pattern, mixotrophy used where
  nutrient is short and dropped where it is plentiful, is not shown.
- **Kin-killing is not solved by thinning.** Each remaining mixotroph kills kin about as often as
  before, and denser worlds hold more of them in absolute terms. The spatial cause of #642's §2 is
  unchanged.

If #647 decides producers should not hold the niche, `b > 0` is a useful companion to a direct
change in the carcass drain (a decomposer term, or an autotrophy cost on carcass efficiency). On its
own it is not that change.

## 12. What this does not show

- **The world is ungated.** Every figure is at satiation sensitivity 0, as in #642. A gated world
  selects differently (#624).
- **It is correlational.** Section B compares bins of `h_eff`, not interventions. `h_eff`, size and
  light still move together. The diagnostic records no structure, so it cannot say whether the
  `h_eff ≥ 1` bin is large, nor why its uptake did not rise.
- **The atlas was searched at `b = 0`.** Its 95 configs are worlds the search found viable under
  size-blind uptake. Switching `b` on in them measures a perturbation, not the worlds genesis would
  find with `b` in the box. Persistence and failure modes in §8 are for those configs only.
- **Lifespans are censored.** Agents alive at T = 2000 are excluded. They are many more at `b > 0`,
  because the worlds are denser, so section B's ages are not comparable across settings and are not
  reported.
- **Excretion is inferred** and **reach is geometric only**, as in #642 (§8 there).
- **Five seeds per config.** Per-config verdict changes of one seed are within noise (§8).

## 13. Instrument

In `explorers-search`, committed before this note (`fedc4b8`):

- **`examples/kin_killer_diet.rs`**: `--uptake-structure-exponent <b>` and
  `--uptake-reference-structure <s_ref>`, applied to every decoded config
  (`config_source::with_uptake_scaling`). Rows record the effective `b` and `s_ref`, and the
  summary states them in its header. New: kin kills per 1000 Producer-role agent-ticks (whole run
  and second half), and section E, the census's verdict per seed, pooled and per config.
- **`role_diet::census_failure`**: the census's verdict factored out of the rollout, so the
  diagnostic reads failure modes exactly as the census does.
- **Tests**: `cargo test --workspace`, 948 passed, 0 failed. At `b = 0` the output is
  byte-identical to #642's tables, apart from the added lines.
- **Artifacts** in `target/645/`: `<setting>-{decoded,fa0}.{jsonl,md}`, `preview/`, `run.sh`,
  `run.log`, `times.log`. `--summary` reprints a report from the rows.

## References

- Boots, M. et al. (2021). Experimental evidence that local interactions select against selfish behaviour. *Ecology Letters*.
- Poorter, H. et al. (2012). Biomass allocation to leaves, stems and roots: meta-analyses of interspecific variation and environmental control. *New Phytologist* 193(1): 30–50.
