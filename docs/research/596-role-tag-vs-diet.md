# Issue #596: the trait-read role tag against realised diet

**Status: measurement. Adds `explorers_search::role_diet` and the `role_diet_census` bin. No
trajectory changes. `explorers-sim`, the search, the evaluator, the prefilter and `CONTEXT.md`
are unchanged.**

The role tag (`CONTEXT.md`, "trophic role"; `TopologyProjection::trophic_roles_of`) reads an agent
as a heterotroph by *trait*: heterotrophy > photosynthetic absorption. Among heterotrophs it then
splits consumer from decomposer by realised diet. [#591](591-carcass-income-accounting.md) found
that the tagged "consumers" of three live baselines take 97–100 % of the unshaded light ceiling
and about a thousandth of that from prey and carcasses. This note asks how general that is and
what it does to the readouts that use the tag.

> **After #599.** The evaluator now reads the trophic role from recent realised income
> (`explorers_genesis_eval::income`), and `TopologyProjection::trophic_roles_of` is gone. The census
> keeps the retired tag as `role_diet::trait_tag` (same rule), shares the income rule
> (`Income::role`), and its diet read now leaves an agent with no income out of the role counts
> instead of letting it keep its tag, as the evaluator does. The figures below were taken before
> that change; a re-run may move the diet guild counts by the handful of seeds where a no-income
> heterotroph by trait sat at a guild's floor.

## TL;DR

1. **The heterotroph tag almost never means heterotrophy.** On live configs:

   | | atlas (92 configs, 460 seeds) | LHS draw (141 configs, 705 seeds) |
   |---|---:|---:|
   | tagged-heterotroph agent-samples taking ≥ 90 % of lifetime income from light | 97.2 % | 96.7 % |
   | tagged consumers that are consumers by diet | 1.4 % | 1.4 % |
   | tagged decomposers that are decomposers by diet | 1.9 % | 1.1 % |

   The rest are producers by diet. The producer tag is right 99.2 % (atlas) and 96.8 % (LHS) of
   the time.
2. **By diet, no decomposer guild exists anywhere.** Over all 1,475 seeds (95 atlas cells + 200
   LHS configs × 5), the trait tag reads a decomposer guild on 25 seeds; the diet read reads one
   on **0**. Consumer guilds are 4 seeds under the tag and 3 under diet, with the membership
   partly switching. The committed atlas's four non-zero `decomposer_fraction` cells (0.2 each)
   are tag artefacts on this evidence.
3. **The trophic-balance fitness term is mostly measuring the tag's mistake.** The term is the
   producer share of living energy at the horizon, bucketed by the tag. Under the tag, it is
   below 0.9 on 140 of 427 live atlas seeds and 257 of 695 live LHS seeds (below 0.5 on 31 and
   111). Under diet it is **never** below 0.9 on either. Every atlas move is upward, by up to 1.0.
   So the spread in this term across the search box comes almost entirely from light-fed agents
   being counted off the pyramid base.
4. **#589's joins stand. Its explanation of why "consumers" persist does not.**
   - The "consumers" outcome #509 recorded and #462/#589 read is the trait split, counted at T
     (`permanence_crosscheck::compartments`). It counts light-fed agents.
   - #589's null result (the heterotroph margin predicts nothing) is untouched, and if anything
     explained: the outcome it was tested against is not heterotrophy.
   - Its account that consumers "persist on emergent reach and carcass-richness terms" is wrong
     for nearly all of them. They persist on light.
5. **Kin grazing of infants is the ordinary mortality regime, not a feature of a few worlds.**
   Across the atlas:
   - 84 % of trait-producer deaths are infants (≤ 50 ticks).
   - 30 % of trait-producer deaths are grazed alive in their death tick, and kin (parent,
     offspring or sibling) did 78 % of that grazing. So 24 % of all producer deaths are
     kin-grazed.
   - Trait heterotrophs with a heterotroph diet die as infants 98.5 % of the time. 76 % of them
     are grazed, 88 % of those by kin.

   The LHS draw is denser. There 73 % of producer deaths are grazed, but kin do only 29 % of it.

## 1. What ran

`role_diet_census --atlas atlas.json` (95 cells) and `role_diet_census` (the 200-config LHS draw,
seed 421). The branch is `issue-596-role-tag-vs-diet` from `13a8014`.

| | |
|---|---|
| Rollout | `role_diet::rollout`, the genesis rollout step for step. It uses the same event retention (plus `Photosynthesized`), the same `RolloutObservations`, the same early stops and the same horizon verdict. `EvalConfig::default()` (no bloom stop), T = 2000 |
| Seeds | 5 per config (the search's ensemble), block 1000–1004, as `guild_census` |
| Income | per agent, lifetime: light from `Photosynthesized`; living and carcass from `Consumed` (split by `target_was_carcass`), as energy *drained* before the trophic efficiency. The drained figure is an upper bound on heterotrophic income, so the light share is a lower bound |
| Diet role | light ≥ 0.5 of income → producer. Otherwise consumer or decomposer at the tag's own detrital-reliance threshold (0.5). No income yet → none (the guild re-read keeps the tag for these) |
| Samples | the evaluator's own second-half role snapshots, every 10 ticks. The confusion counts agent-samples |
| Deaths | every death of the run. Infant ≤ 50 ticks. Grazed = a living-target `Consumed` on it in its death tick. Kin = parent, offspring or sibling (a shared parent), from `Born` |
| Live config | most of its seeds end with no failure verdict |
| Wall clock | atlas 49 s. LHS 92 min, with one dense config at a time |

The rollout is pinned against `explorers_genesis::rollout` on real seeds: the same verdict, the
same termination tick, the same consumer and decomposer guild, and the tag balance equal to the
evaluator's `trophic_balance_score` (`role_diet::tests`).

## 2. The confusion table (live configs, agent-samples)

**Atlas:**

| tag \ diet | producer | consumer | decomposer | no income | total | agrees |
|---|---:|---:|---:|---:|---:|---:|
| producer | 395,786 | 528 | 1,060 | 1,747 | 399,121 | 99.2 % |
| consumer | 12,473 | 176 | 0 | 337 | 12,986 | 1.4 % |
| decomposer | 28,095 | 42 | 550 | 0 | 28,687 | 1.9 % |

**LHS draw:**

| tag \ diet | producer | consumer | decomposer | no income | total | agrees |
|---|---:|---:|---:|---:|---:|---:|
| producer | 482,775 | 3,969 | 2,630 | 9,580 | 498,954 | 96.8 % |
| consumer | 20,938 | 304 | 0 | 514 | 21,756 | 1.4 % |
| decomposer | 37,984 | 70 | 425 | 0 | 38,479 | 1.1 % |

All configs, live or not, give the same picture. On the LHS draw with failing configs
included, 2.7 % of tagged consumers and 1.8 % of tagged decomposers agree with their diet.

The decomposer row is the starker one. The tag calls an agent a decomposer when most of what it
has *drained* came from carcasses. A light-fed agent that has drained a few crumbs, mostly from
carcasses, qualifies, whatever that is against its light income.

The few trait producers that are heterotrophs by diet (1–3 %) are mostly shaded producers or
infants whose small light income is outweighed by a graze.

## 3. How light-fed

The light share of lifetime income, for trait-tagged heterotroph agent-samples with income:

| light share | atlas | LHS |
|---|---:|---:|
| 0.0–0.1 | 0.8 % | 0.6 % |
| 0.1–0.5 | 1.0 % | 0.7 % |
| 0.5–0.9 | 1.0 % | 2.0 % |
| 0.9–1.0 | **97.2 %** | **96.7 %** |

The distribution is bimodal, with almost nothing in between. The 0.5 threshold is not doing the
work: moving the cut anywhere between 0.1 and 0.9 moves at most 2 % of samples.

## 4. The guild reads

Per seed, over all 1,475:

| guild | seeds under the tag | seeds under diet |
|---|---:|---:|
| consumer | 4 | 3 |
| decomposer | 25 | 0 |

The decomposer seeds lost are on `sample:10`, `15`, `20` (4/5), `36` (3/5), `38`, `45`, `96`,
`100` (2/5), `127` (3/5), `129` (2/5), `165` and `188`, and on `atlas:29`, `41`, `83` and `91`.

The consumer read moves between configs. It loses `atlas:59`, `sample:45` and `sample:96`, and
gains `sample:165` and `sample:197`. `sample:100` keeps its one.

The diet read adds a producer guild on 8 atlas and 12 LHS live configs. There the tag had counted
light-fed agents out of the producer role, and so below `GUILD_MIN_SIZE` on some sample.

The committed atlas (`atlas.json`, the search's own seeds) records a `decomposer_fraction` of 0.2
on four cells and a `consumer_fraction` of 0 everywhere. On this evidence its decomposer
fractions are the tag's artefact. Its consumer fractions were already zero.

## 5. Trophic balance

The evaluator's balance term, `trophic_balance_score`, is the producer share of living energy at
the horizon, with each agent bucketed by the tag. Recomputed per live seed with diet roles:

| | atlas (427 live seeds) | LHS (695 live seeds) |
|---|---:|---:|
| below 0.9, by the tag | 140 | 257 |
| below 0.9, by diet | 0 | 0 |
| below 0.5, by the tag | 31 | 111 |
| below 0.5, by diet | 0 | 0 |
| seeds that move by > 0.05 | 171 (all upward) | 283 (5 fall slightly) |

The median is 1.0 under both reads, because most live worlds end with producers only. Among the
atlas seeds that move, the mean goes from 0.69 by the tag to 1.00 by diet.

So by diet, every live world at the horizon is essentially all producer energy. What the balance
term currently varies with is how much of that producer energy sits in agents whose heterotrophy
trait edges past their autotrophy.

## 6. Deaths

Summed over every run, all configs:

**Atlas:**

| agents | deaths | infant | grazed | infant grazed (by kin) | grazed by kin |
|---|---:|---:|---:|---:|---:|
| trait producers | 115,201 | 83.9 % | 30.4 % | 32,562 (25,957) | 77.7 % of grazed |
| trait heterotrophs, light-fed | 5,827 | 82.4 % | 45.9 % | 2,394 (1,585) | 67.7 % |
| trait heterotrophs, heterotroph by diet | 9,879 | 98.5 % | 75.8 % | 7,471 (6,568) | 87.8 % |
| trait heterotrophs, no income yet | 2,121 | 93.6 % | 13.9 % | 294 (13) | 4.4 % |

**LHS draw:**

| agents | deaths | infant | grazed | infant grazed (by kin) | grazed by kin |
|---|---:|---:|---:|---:|---:|
| trait producers | 411,350 | 96.2 % | 72.6 % | 294,518 (84,746) | 28.7 % of grazed |
| trait heterotrophs, light-fed | 6,378 | 82.9 % | 55.9 % | 3,225 (1,940) | 61.7 % |
| trait heterotrophs, heterotroph by diet | 14,811 | 99.1 % | 84.6 % | 12,521 (9,914) | 79.2 % |
| trait heterotrophs, no income yet | 2,401 | 96.9 % | 22.8 % | 547 (11) | 2.0 % |

Deaths are dominated by infants in every bucket.

- **Atlas.** Grazing kills about a third of producers, and mostly their own kin do it. This is
  #591's `sample:31` finding at the scale of the atlas.
- **LHS draw.** Mortality is heavier and less kin-directed: 73 % of producer deaths are grazed,
  29 % of that by kin. It holds the dense and failing configs the search's live cells exclude,
  where many unrelated grazers crowd each other.
- **Heterotroph-by-diet deaths.** 99 % are infants. An agent of that age has a diet of a few
  ticks, a bite or two outweighing a shaded start, so this bucket is mostly newborns caught in
  mutual grazing with their family rather than established heterotrophs.

## 7. Reading

- **The trait-read heterotroph tag does not identify heterotrophs.** In live worlds about 97 % of
  what it tags as consumer or decomposer lives on light. What it identifies is agents whose
  heterotrophy trait edges past their autotrophy. Those are mixotrophs by genotype and producers
  by economy.
- **Readouts built on the tag:**
  - *Heterotroph guild fractions* (atlas, evaluator; reported only): the decomposer guild
    disappears under diet and the consumer guild is noise at 3–4 seeds in 1,475. The atlas's
    guild observables currently report the tag, not heterotrophy.
  - *Trophic balance* (a fitness term the search optimises): under diet the term is ≈ 1 on every
    live seed. Its current spread comes from the tag.
  - *#509/#462/#589 "consumers"* (the trait count at T): mostly light-fed agents. #589's null
    stands. Its sentence on how consumers persist is corrected here: they persist on light. The
    same sentence in `viability.md` is corrected alongside.
- **What changes, and what doesn't.** Whether the tag, the guild read or the balance term should
  read diet is a design question for `grill-with-docs`, as the issue says. Nothing here changes
  them. It is added to the pinned design agenda.

## 8. What this does not show

- **The 0.5 light-share cut.** The distribution is bimodal (§3), so the reading does not hinge on
  it.
- **Drained against kept.** Heterotrophic income is booked as energy drained, before the transfer
  efficiency. That is conservative, and only makes the light share an understatement.
- **Lifetime income, not recent.** A lifetime ledger dilutes a diet change late in life. This
  does not matter for the finding: the light-fed share of tagged heterotrophs is the same across
  the whole second half.
- **The census's seeds, not the search's.** The atlas does not record which seeds produced a
  cell. Rows re-measure a cell at n = 5, as `guild_census` does.
- **Kin by pedigree depth one.** A cousin or grandparent grazer reads as non-kin, so the kin
  shares are lower bounds. Which grazer delivered the killing drain is not singled out when
  several grazed in the tick. A death counts as kin-grazed if any grazer was kin.

## 9. Instrument

- `crates/explorers-search/src/role_diet.rs`:
  - `Income`, `diet_role` and `PRODUCER_LIGHT_SHARE`;
  - `DietLedger`, which books income per agent, descent, age and the tick's living grazers, and
    attributes deaths into a `DeathTable`;
  - `Confusion`, `SeedDiet` and `rollout`, the genesis rollout with the census alongside.
- Tests:
  - the diet-role arithmetic;
  - the ledger: income, descent, kin, and a kin-grazed infant death;
  - the rollout pinned against `explorers_genesis::rollout`: verdict, termination tick, both
    heterotroph guilds, the trophic-balance term, and determinism.
- `crates/explorers-search/src/bin/role_diet_census.rs`:
  - config selection and seeds as `guild_census`, with seeds in parallel;
  - JSON lines to `target/role-diet-census.jsonl` (`--out`), resumable, with `--limit` and
    `--summary`;
  - a summary with the confusion table, light-share deciles, guild cross, trophic balance and the
    death table per source (all and live configs), plus a per-config table;
  - tests of the pool arithmetic and the CLI.
- Artifacts: `target/role-diet-atlas.jsonl` and `target/role-diet-sample.jsonl`, with their
  `*-summary.md`.
