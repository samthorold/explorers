# Issue #587: is `sample:31`'s lockup a heterotroph re-invasion barrier?

**Status: measurement. Adds a research bin (`reinvasion_barrier`) and moves
`invasion_growth`'s invasion machinery into a shared lib module; changes no
trajectory.** `sample:31` is the bloom-then-lockup cell that is `ρ ∧ I`'s one
strict false positive at `T = 2000` ([`viability.md`](../system-design/viability.md),
[`509-viability-at-settled-horizon.md`](509-viability-at-settled-horizon.md)). A
throwaway probe found its founder heterotrophs (10 of 31) gone by tick 100–150,
before any carcass pile. Producers kept breeding through lockup, and no
heterotroph reappeared. This note asks whether a heterotroph can establish on
the pile once it exists (the phenotype question) and whether the small steps
selection would have to climb pay (the path question).

## TL;DR

1. **The second reading holds: the full heterotroph fails even on the pile.**
   Injected at tick 1000 onto the top carcass-nutrient cells, the founder
   heterotroph centroid dies out on 8/8 seeds at a median of 8 ticks, with
   no birth. It does the same when spread uniformly (9 ticks). Neither the
   reach nor the placement is the problem. The pile does not feed a
   heterotroph in this world. That is an **economics** problem, not a
   selection-path one.
2. **Transfer efficiency sets the economics.** A pure heterotroph sits at
   trait distance ≈ 1.5 from the resident producer. That is also the trait
   vector every producer carcass keeps. It therefore keeps `e ≈ 0.02` of what
   it drains, so its carcass-intake ceiling is `heterotrophy × u_H × e ≈ 0.022`
   E/tick with a carcass in reach every tick. Its trait-maintenance terms alone
   cost ≈ 0.034 E/tick before structure and somatic upkeep. Its `κ ≈ 0.01`
   sends ~80 % of the founder reserve into a reproductive earmark on tick 1.
   The earmark never reaches the 42 E threshold, and the soma starves in about
   six ticks. The real founder heterotrophs with photosynthesis below 0.01 die by tick
   10 in the same way. Every founder heterotroph that lasted to tick 20 or
   beyond carried some photosynthesis.
3. **The small steps do not invade either, and heterotrophy makes them worse,
   not better.** A resident producer cohort with heterotrophy raised by 1, 2 or
   4 mutation magnitudes has a negative median rate on every arm (at most 1/8
   seeds positive). An unmodified resident-producer cohort does better on the
   same resident (`step 0`: 3/8 seeds positive, median `−0.00076` against
   `−0.0019` for one step, uniform). The steps eat the pile: one step drains
   2,496 carcass meals over 8 seeds and draws the pile down by a median of
   708 N against the control. They lose on the balance.
4. **This is not a re-invasion barrier in the issue's sense.** The recalcitrance
   / defence reserve lever is not the target this cell points to. The pile
   pays in proportion to how close the eater is to a producer in trait space,
   so only near-producers profit from it, and the residents already are
   near-producers.

## 1. What ran

`cargo run --release -p explorers-search --bin reinvasion_barrier -- sample:31`
on branch `issue-587-reinvasion-barrier` (from `26b2436`), ~30 s wall clock for the whole run.

| | |
|---|---|
| Config | `sample:31` (the shared `config_source` key) |
| Seeds | 1000–1007 (the `invasion_growth` block); 8/8 reached the injection tick |
| Injection tick / window | 1000 / 1000 (the window ends at the settled horizon `T = 2000`) |
| Cohort | 8 agents, provisioned as founders (`place_cohort`) |
| Phenotypes | `full` = the founder heterotroph centroid `World::new` seeds (photo 0, heterotrophy 1.086, the other traits at the mean; κ 0.012). `step k` = the resident producer centroid at tick 1000 (`topology::trophic_roles`), heterotrophy + `k × mutation_magnitude` (0.312), for `k` = 1, 2, 4. **`step 0`** = the resident producer centroid itself, added as a baseline (see §3) |
| Placements | `uniform` over the world; `pile` = uniform within the 2 nutrient-grid cells (of 16) holding the most carcass nutrient, dealt round-robin from the top cell |
| Control | the resident forked and stepped through the window uninjected |

At the injection tick (medians over seeds) the resident is 98 agents, all
producers by role and none a nominal heterotroph (heterotrophy > photo; max
heterotrophy 0.66). The pile holds 24,441 N in carcasses and the pool holds
6,487 N. The two pile cells hold 28 % of the carcass nutrient. By the window
end the control has fallen to 45 agents, carcass N has grown to 40,475 and the
pool to 152 N. The resident's own per-capita rate over the window is
`−0.00058`. The lockup world is itself in decline, and every injected rate
should be read against that.

## 2. Result

Rate `r = ln(max(N_end, ½) / 8) / 1000`. An extinct lineage reads
`−0.00277`. The interval is the sign test's (`[min, max]` at `n = 8`), and
*invades* means median > 0 with the interval clear of zero, as in
`invasion_growth`. Diet and deaths are summed over the 8 seeds. The
carcass-drawdown column is the median of control − arm carcass N at the
window end, and the pool-gain column the median of arm − control pool N.

| arm | median r | interval | r > 0 | invades | lineage end (med / max) | extinct (med tick) | births pure / cross | meals living / carcass | deaths drained / undrained | carcass drawdown | pool gain |
|---|---:|---|---:|---|---:|---:|---:|---:|---:|---:|---:|
| full / uniform | −0.00277 | [−0.00277, −0.00277] | 0/8 | no | 0 / 0 | 8 (9) | 0 / 0 | 89 / 241 | 7 / 57 | −612 | −1 |
| full / pile | −0.00277 | [−0.00277, −0.00277] | 0/8 | no | 0 / 0 | 8 (8) | 0 / 0 | 76 / 273 | 14 / 50 | −398 | −152 |
| step 0 (resident) / uniform | −0.00076 | [−0.00277, 0.00122] | 3/8 | no | 4.5 / 27 | 2 (313) | 168 / 2 | 880 / 1024 | 56 / 116 | −231 | −3 |
| step 0 (resident) / pile | −0.00277 | [−0.00277, 0.00041] | 1/8 | no | 0 / 12 | 5 (220) | 55 / 3 | 66 / 448 | 9 / 85 | −76 | −4 |
| step 1× / uniform | −0.00188 | [−0.00277, −0.00013] | 0/8 | no | 1.5 / 7 | 4 (552) | 176 / 10 | 992 / 2496 | 186 / 44 | 708 | −1 |
| step 1× / pile | −0.00277 | [−0.00277, −0.00139] | 0/8 | no | 0 / 2 | 5 (193) | 65 / 1 | 382 / 1539 | 81 / 44 | 21 | −1 |
| step 2× / uniform | −0.00208 | [−0.00277, 0.00012] | 1/8 | no | 1 / 9 | 3 (244) | 91 / 15 | 615 / 1462 | 123 / 32 | 1319 | 1 |
| step 2× / pile | −0.00277 | [−0.00277, −0.00139] | 0/8 | no | 0 / 2 | 5 (159) | 55 / 2 | 272 / 891 | 77 / 40 | 437 | 33 |
| step 4× / uniform | −0.00277 | [−0.00277, −0.00139] | 0/8 | no | 0 / 2 | 6 (186) | 52 / 2 | 318 / 1025 | 77 / 38 | 1560 | 0 |
| step 4× / pile | −0.00277 | [−0.00277, −0.00208] | 0/8 | no | 0 / 1 | 6 (74) | 32 / 1 | 248 / 657 | 69 / 26 | 271 | 120 |

The economics read is set out below. `e` is `trophic_transfer_efficiency`
of the arm's traits against the resident producer centroid, which is also
the trait vector a producer carcass keeps. The ceiling is the most carcass
energy the phenotype can keep per tick with a carcass in reach
(`heterotrophy × u_H × e`, the binary-reach drain). "Kept" is the drained
carcass energy × that seed's `e`. It is an estimate, because each carcass
carries its own dead agent's traits, not the centroid's.

| arm | `e` on resident producer (median) | intake ceiling (E/tick) | carcass E drained | carcass E kept |
|---|---:|---:|---:|---:|
| full | 0.021 | 0.022 | 238 / 273 (uniform / pile) | 5 / 6 |
| step 0 | 0.558 | 0.024 | 268 / 217 | 150 / 121 |
| step 1× | 0.290 | 0.103 | 897 / 527 | 260 / 153 |
| step 2× | 0.151 | 0.101 | 873 / 561 | 132 / 85 |
| step 4× | 0.041 | 0.053 | 1170 / 736 | 48 / 30 |

## 3. Reading

**The full heterotroph cannot live on this pile, and placement does not
matter.** On the pile it eats: 273 carcass meals from 64 founders. It keeps
about 2 % of what it drains. The death column shows starvation or the
structure threshold (50 of 64 undrained), not predation. The ceiling is
0.022 E/tick. Its heterotrophy and base-metabolic maintenance alone is 0.034
E/tick (`0.0117 + 1.086^1.80 × 0.0194`, the recipe's coefficients), before
the structure and somatic terms. With `κ = 0.012`, the first tick moves 10.3
of its 12.9 E reserve into a reproductive earmark that cannot reach the
42.4 E threshold (a probe on seed 1000 traced reserve 12.9 → 2.5 → 0.6 → 0.2
→ 0.1 → 0.04, dead at tick 6, earmark 12.5 stranded). In an exploratory
check, not part of the committed bin, raising the full phenotype's κ to 0.2,
0.5 and 0.8 on the same forks only stretched survival to at most 35 ticks.
No birth occurred on any seed or placement. κ is not the lever. The transfer
efficiency is.

**This is also why the founders died, and it has nothing to do with the
pile.** Protocol checks with the same bin (`--t-inj 1 --window 300` and
`--t-inj 250 --window 300`) inject the full cohort before any pile exists,
and it dies out on 8/8 seeds by tick 6–8. Among the real founders of seeds
1000–1002 (30 nominal heterotrophs), every one with photosynthesis below
0.01 died by tick 10. Every one that lasted to tick 20 or beyond was a
mixotroph (photosynthesis 0.007–0.30), and so were the few that reached
tick 84–193. So the probe's "founder heterotrophs die on the
living-prey economy" is really "the pure-heterotroph corner of this config
has no economy at all". A pure heterotroph was never viable here, before or
after the pile.

**The small steps are near-producers eating the pile, and heterotrophy costs
them.** One step keeps `e = 0.29` and has the highest ceiling (0.10 E/tick).
Its lineages breed (176 pure births) and drain the most carcass energy, and
they pull the pile down against the control (median 708 N). They still shrink
faster than an unmodified producer cohort in the same world (0/8 positive
against 3/8 for `step 0`). Four steps sit at `e = 0.04`: more drain, less
kept, and extinction by tick 74–186. The kernel makes the steps pay less the
further they climb. The heterotrophy maintenance and the photosynthetic
dilution they carry are not repaid by the carcass income at any step size.
The unmodified `step 0` cohort does not invade either, but the resident it
enters is itself declining (`−0.00058` per tick). It is the only phenotype
whose seeds straddle zero on both placements.

**The `pile` placement is worse than `uniform` for every phenotype, the
resident baseline included.** So the pile cells are a worse neighbourhood
overall (plausibly crowded, and so shaded by living producers), not a
heterotroph-specific trap. That fits the economic reading: being next to the
carcasses buys nothing a phenotype cannot convert.

**Which reading holds.** The full heterotroph fails even on the pile, so the
answer is the issue's second reading. The pile does not feed a heterotroph
in this world. That is an economics problem (transfer efficiency above all,
then κ and maintenance), not a selection-path one. The first reading (full
invades, steps do not) is excluded, and so is the third (steps invade). The
recalcitrance / defence asymmetry lever (`trait-space.md`, "Decomposer is a
behavioural role") is not what this cell asks for. A recalcitrance *discount*
makes the dead harder to drain, which would not rescue a phenotype already
keeping 2 %. The binding term is the trait-distance kernel pricing the dead
producer: whoever would decompose it must stand near a producer in trait
space. On `sample:31` that means the resident mixotrophic producers, which
do eat the pile (the probe's 10–30 E per 50 ticks) and cannot keep up with
it.

## 4. What this does not show

- One config. `sample:31` has `trophic_distance_decay = 2.09` and
  `κ_C = 0.01`. Other lockup cells (`atlas:2`, `sample:154`, `158`, `192`) may
  differ. The bin takes any `config_source` key.
- One injection tick (1000) and one cohort size (8). The protocol checks at
  ticks 1 and 250 cover only the full phenotype, over a 300-tick window.
- `pile` means the two top carcass-nutrient cells of a 4 × 4 grid, which hold
  28 % of the carcass nutrient. Within a cell the positions are uniform, not
  placed on carcasses.
- The κ sweep and the founder-death trace were throwaway diagnostics on this
  branch, run once and not committed. Their numbers are quoted above as run.
- The maintenance figure counts only the trait and base terms. The kept-energy
  column uses the centroid's `e` for every carcass.

## 5. Instrument

`crates/explorers-search/src/bin/reinvasion_barrier.rs` takes the config key
(default `sample:31`), `--t-inj` (default 1000) and `--window` (default 1000).
It chunks with `--seed-from A --seeds N` and recombines with `--merge F…`
(byte-identical to the whole run). It writes `target/reinvasion-barrier.json`.
It reuses `invasion_growth`'s fork, founder provisioning, lineage by `Born`,
diet split, rate and sign test. Those now live in `explorers_search::invasion`,
moved unchanged, and `invasion_growth` imports them. The lineage also tallies
the energy its members drain. A separate bin, rather than a mode on
`invasion_growth`, keeps that bin's atlas-wide role / arm protocol and its
artifact untouched. This question has a different unit (one config, explicit
phenotypes × placements, a baseline arm) and a different summary. Its tests
are smoke tests only: forks and determinism, the full phenotype equals
`World::new`'s heterotroph cluster, small-step arithmetic, pile placement on
a seeded carcass layout, the summary / merge round trip, and the CLI.
`explorers-sim` is unchanged.
