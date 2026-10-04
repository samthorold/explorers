# Issue #642: what light-fed mixotrophs eat, and what they are

**Status: measurement, docs only. Adds an observer-only diagnostic, the example
`kin_killer_diet` in `explorers-search`, and a per-bite form of the realised-drain booking
(`intake_ceiling::realised_bites`; `realised_drains` now sums it). `explorers-sim`, the search, the
evaluator and the prefilter are unchanged. No design decision is taken here: the open questions in
§6 go to follow-up issues.**

#637 found the `(k, τ)` region for the fullness gate empty. About a third of the mixotroph kin
kills there were by killers full of light but hungry for nutrient, and the world rules call that
"the carnivorous-plant pattern". That raised three questions the region maps could not answer:

- what do these killers eat apart from their kin, and could they reach anything else?
- is light-fed mixotrophy a conditional strategy, used where nutrient is short, as carnivory is in
  the domain? Or is it a bypass: a route around decomposition?
- how common is it, and who processes the carcass pile?

## TL;DR

1. **Kin kills are spatial** (§2). At 80 % (decoded) and 85 % (`founder_aggregation = 0`) of
   light-fed mixotroph kin kills, kin are the only living things in feeding reach. For killers
   sated on energy but hungry for nutrient it is 89 % and 92 %. There is on average less than 0.1
   living non-kin agent in reach.
2. **Carcasses are the nutrient source; living kin are not** (§1). Of the nutrient these agents
   retain from drains, carcasses supply 88–94 % and living kin 5–8 %. Living kin still give 38–44 %
   of their drain energy. A carcass bite carries about 9–14 nutrient per unit of structure, against
   about 0.8 for a living bite, because a carcass holds its dead agent's whole free store
   and earmark. Drains are a minority route overall: pool uptake still supplies 86–90 % of these
   agents' nutrient over the run.
3. **Producers have absorbed the decomposer niche** (§4, C). Light-fed mixotrophs retain 96–97 % of
   all carcass nutrient retained, and decomposers 1.5–2.4 %. About two thirds of the carcass
   nutrient released by bites is released by light-fed mixotrophs, and 55–58 % of what is excreted
   back to the pool. Consumers and decomposers together are under 1 % of agents (§3, A).
4. **Mixotrophy is not conditional on poor ground** (§3, B). The most heterotrophic producers
   (`h_eff ≥ 1`) sit on the richest pools (286 and 274, against 206 and 113 for pure producers).
   They are still the most nutrient-limited (65 % and 60 %, against 30 %), because their light
   income doubles while their uptake stays flat. Uptake does not scale with structure, and light
   share does.
5. **Appetite rises over the run** (§5, D). Light-fed mixotrophs' mean effective heterotrophy rises
   from 0.61 to 0.69 decoded and from 0.55 to 0.62 at `founder_aggregation = 0`, between the first
   and last quarters. In the domain's closest experiment, viscous populations evolved *lower*
   cannibalism (Boots et al. 2021).

On land the domain shows the reverse on every count (§6): mixotrophy is rare and confined to
bright, wet, nutrient-poor ground; detritus goes mainly through specialist decomposers; uptake
capacity grows with root mass; and producers reach organic nutrient through mycorrhizal partners,
not by eating. Whether the world should look like that is the design's question (§7).

## 1. What ran

| | |
|---|---|
| Tree | `e59842e` plus the uncommitted diagnostic, committed with this note. Release builds. |
| Diagnostic | `kin_killer_diet` (example in `explorers-search`) on the atlas (`atlas.json`, 95 configs), seeds 1000–1004 (475 seeds), T = 2000, `EvalConfig::default()`, **ungated** (satiation sensitivity 0; recognition at the baseline `D = 0.5`), both as decoded and at `--founder-aggregation 0`. |
| Rollout | Mirrors `role_diet::rollout_with_fullness` step for step (the same retention, observations, early stops and ledger); observer-only. |
| Machine | 8-core laptop, one process at a time. |

**Commands** (cwd is the repo root; drivers `target/kkd/run.sh` for §§1–3 of the output and
`target/kkd2/run.sh` for the full output; sections 1–3 are identical between the two runs):

```sh
cargo build --release -p explorers-search --example kin_killer_diet
C=$(seq -s, -f 'atlas:%g' 0 94)
./target/release/examples/kin_killer_diet --atlas atlas.json --configs $C \
    --out target/kkd2/decoded.jsonl > target/kkd2/decoded.md
./target/release/examples/kin_killer_diet --atlas atlas.json --configs $C \
    --founder-aggregation 0 --out target/kkd2/fa0.jsonl > target/kkd2/fa0.md
```

**Wall clock** (`target/kkd*/times.log`):

| run | time |
|---|---:|
| sections 1–3, decoded (`target/kkd`) | 53 s |
| sections 1–3, `founder_aggregation = 0` (`target/kkd`) | 99 s |
| sections 1–3 and A–D, decoded (`target/kkd2`) | 47 s |
| sections 1–3 and A–D, `founder_aggregation = 0` (`target/kkd2`) | 99 s |

**Populations and definitions.**

- **Light-fed mixotroph** (§§1–3): an agent with heterotrophy above zero whose **trophic role** at
  the start of the tick is producer (light at least half its recent income). This is #634/#637's
  population. Sections A–D use a stricter cut, a producer with effective heterotrophy
  `h_eff > 0.05` (`effective_trait_with_steepness(1, wear_degradation_steepness)`), because about
  a sixth of section 1's population carries only a trace of heterotrophy (`h_eff ≤ 0.05`). A **pure
  producer** is a producer with `h_eff ≤ 0.05`.
- **Kin**: parent, offspring or sibling (`DietLedger::is_kin`). A **kin kill** is counted once per
  killing pair, as #637 counts them.
- **Fullness state** at #637's closest decoded cell (`k = 1.7`, `τ = 17.2`, n = 4), from the
  fullness carried into the tick: (i) sated on energy, hungry for nutrient (`E_E < 0.5 ≤ E_N`);
  (ii) hungry on energy (`E_E ≥ 0.5`); (iii) sated on both.
- **Reach**: inside the killer's `phase::consumption_reach` at drain time. With the baseline's
  `body_reach_coefficient = 0`, that is `h_eff × contact_range_coefficient`.
- **Nutrient bound / retained** per bite (`realised_bites`): the nutrient the bite releases, and
  `min(released, demand × energy gained)`, as the drain pass computes it. The rest is excreted to
  the cell by the drain rule; the diagnostic infers that excretion, it does not read the grid.

## 2. Diet and reach (sections 1–2)

**Diet** (% of bites / % of drain energy gained / % of drain nutrient retained):

| target | decoded, whole run | fa 0, whole run | decoded, second half | fa 0, second half |
|---|---:|---:|---:|---:|
| kin, living | 31.6 / 41.7 / 8.0 | 37.1 / 40.4 / 6.4 | 29.4 / 43.8 / 7.3 | 37.8 / 37.9 / 5.3 |
| non-kin, living (all roles) | 12.5 / 4.9 / 3.4 | 7.9 / 3.0 / 1.2 | 3.3 / 1.3 / 0.4 | 4.4 / 2.1 / 0.6 |
| carcass | 55.9 / 53.4 / 88.5 | 55.0 / 56.7 / 92.3 | 67.3 / 54.9 / 92.3 | 57.8 / 60.1 / 94.1 |
| bites (n) | 209,796 | 420,464 | 53,419 | 98,247 |

Nutrient per unit of structure drained, drain-weighted: about 0.8 for every living target (kin
0.80 / 0.78; non-kin producers 0.85 / 0.76) and 10.6 / 12.8 for carcasses (9.0 / 13.7 in the second
half). The difference is the physics, not prey choice: a living bite releases only the nutrient
bound in the structure removed, while a carcass holds all its dead agent's nutrient, free store and
earmark included (world rules, flows 3 and 6).

By fullness state the pattern is the same. Carcasses are 44–70 % of bites in every state and
period; living kin 22–54 %; living non-kin at most 16 % over the whole run and 7 % in the second
half. The sated-on-energy, hungry-for-nutrient killers take 60 % / 59 % of their bites from
carcasses over the run.

**Reach at each kin kill** (% of kin kills, whole run):

| fullness state | decoded: kills | only kin in reach | non-kin living in reach | carcass in reach | fa 0: kills | only kin in reach | non-kin living in reach | carcass in reach |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| (i) E-sated, N-hungry | 4,657 | 88.6 | 1.5 | 11.0 | 8,465 | 92.4 | 0.4 | 7.3 |
| (ii) E-hungry | 784 | 88.0 | 3.2 | 10.3 | 1,857 | 95.2 | 0.3 | 4.5 |
| (iii) sated both | 7,604 | 73.8 | 4.7 | 24.5 | 23,661 | 81.6 | 1.8 | 17.3 |
| all | 13,045 | 79.9 | 3.5 | 18.8 | 33,983 | 85.0 | 1.3 | 14.1 |

Mean in reach at a kin kill: 2.10 / 1.59 living kin, 0.07 / 0.02 living non-kin, and 0.57 / 0.18
carcasses, most of them kin (0.49 / 0.14). The second half is the same: 81.2 % / 85.2 % of kin
kills have only kin in reach.

- **The killers are not choosing kin.** Kin are what is there. Where anything else is in reach, it
  is almost always a carcass, and usually a kin carcass.
- **Mobility and dispersal barely separate killers** (section 3). Median mobility of kin killers
  is 0.28 against 0.52 for light-fed mixotrophs that never kill kin decoded, and 0.30 against 0.30
  at fa 0. Median dispersal is 0.94 against 1.09, and 1.13 against 1.22. Killers are a little more
  sessile and a little less dispersive, which fits a spatial cause, but the distributions overlap
  almost entirely.

## 3. Prevalence and conditionality (sections A–B)

**A. Prevalence** (second-half sample ticks, every living agent, decoded / fa 0):

| group | % of agent-samples |
|---|---:|
| producer (all) | 99.1 / 99.0 |
| light-fed mixotroph (producer, `h_eff > 0.05`) | 33.7 / 34.4 |
| pure producer (`h_eff ≤ 0.05`) | 65.3 / 64.6 |
| consumer | 0.1 / 0.2 |
| decomposer | 0.3 / 0.4 |
| no income yet | 0.6 / 0.5 |

Among producers, 34.0 % / 34.7 % have `h_eff > 0.05`, 29.3 % / 26.1 % above 0.2 and 18.6 % / 16.1 %
above 0.5. Producer `h_eff` p90 is 0.865 / 0.815, against a median of 0.635 / 0.525 for consumers
and 0.425 / 0.445 for decomposers: the upper tenth of producers carry more heterotrophy than the median
heterotroph.

**B. Producers by `h_eff`** (second half; per agent-sample means; decoded, then fa 0):

| `h_eff` bin | samples | births / 1000 agent-ticks | % N-limited | pool N at cell | light E | uptake N | carcass-drain N | N from carcass % | deaths | mean age at death |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| [0, 0.05) | 128,462 | 5.62 | 30.4 | 206.0 | 5.10 | 0.277 | 0.000 | 0.0 | 5,249 | 99.6 |
| [0.05, 0.2) | 9,269 | 4.91 | 37.4 | 97.3 | 6.75 | 0.236 | 0.055 | 18.8 | 496 | 98.3 |
| [0.2, 0.5) | 20,734 | 6.40 | 42.7 | 161.3 | 8.64 | 0.198 | 0.041 | 16.9 | 388 | 120.7 |
| [0.5, 1) | 20,770 | 4.63 | 53.3 | 211.8 | 9.03 | 0.154 | 0.029 | 15.8 | 140 | 293.1 |
| ≥ 1 | 15,536 | 6.28 | 64.5 | 285.7 | 10.21 | 0.212 | 0.031 | 12.5 | 76 | 448.2 |
| *fa 0:* [0, 0.05) | 322,977 | 4.09 | 29.7 | 113.4 | 4.56 | 0.198 | 0.000 | 0.1 | 9,718 | 133.9 |
| [0.05, 0.2) | 42,852 | 7.41 | 32.8 | 152.2 | 5.40 | 0.213 | 0.054 | 20.0 | 1,728 | 99.9 |
| [0.2, 0.5) | 49,556 | 6.66 | 29.8 | 153.9 | 8.38 | 0.242 | 0.084 | 25.6 | 825 | 148.3 |
| [0.5, 1) | 45,179 | 6.22 | 39.7 | 246.7 | 9.49 | 0.234 | 0.072 | 23.1 | 347 | 292.4 |
| ≥ 1 | 34,287 | 5.64 | 60.2 | 274.2 | 10.79 | 0.263 | 0.071 | 20.8 | 164 | 312.0 |

(% N-limited: the census's `Surplus` nutrient below energy. Earmark fill, the energy the grow phase
adds to the reproductive allocation, rises from 1.43 to 2.45 decoded and from 1.51 to 2.35 at
fa 0. The full table, with living-drain income, is in `target/kkd2/*.md`.)

- **The most heterotrophic producers stand on the richest ground.** At `h_eff ≥ 1` the pool under
  them is 286 / 274, against 206 / 113 for pure producers. Decoded the gradient is not monotone (the
  `[0.05, 0.2)` bin sits on the poorest cells, 97); at fa 0 it rises bin by bin.
- **They are nutrient-limited anyway, because light scales with size and uptake does not.** Light
  income doubles across the bins (5.1 → 10.2, 4.6 → 10.8). Uptake stays at about 0.15–0.28 a tick
  in every bin. In the code, light share is weighted by effective autotrophy × structure
  (`phase::photosynthesise`), and uptake demand is effective autotrophy × a fixed anchor, with no
  structure term (`phase::absorb_nutrients`). So the bigger, better-lit producer gains energy it
  cannot match with nutrient, whatever its pool, and reads nutrient-limited. Heterotrophy rises
  with it. This is correlational (§8); size, light and `h_eff` move together.
- **Carcasses are a minority of even the most heterotrophic producer's nutrient**: 12.5–25.6 % per
  bin. Uptake is the main route throughout.
- **The births advantage is mixed.** Decoded, no bin of mixotrophs breeds clearly faster than pure
  producers (4.6–6.4 against 5.6 per 1000 agent-ticks). At fa 0 every mixotroph bin breeds
  38–81 % faster (5.6–7.4 against 4.1).
- **Lifespan rises with `h_eff`** from `[0.2, 0.5)` up (to 448 / 312 ticks at `h_eff ≥ 1`, against
  100 / 134 for pure producers), but deaths in the top bins are few (76 / 164) and agents alive at
  the end are censored. Long-lived agents are also the large ones.

## 4. Who processes the detritus (section C)

**Carcass nutrient by recipient, whole run** (decoded / fa 0; recipient by role at the start of the
tick):

| recipient | carcass bites | % of carcass N released | % of carcass N retained | own retention (retained / released) |
|---|---:|---:|---:|---:|
| light-fed mixotroph | 89,687 / 224,881 | 65.6 / 65.1 | 96.1 / 97.4 | 29 % / 36 % |
| decomposer | 28,558 / 57,443 | 17.8 / 17.1 | 2.2 / 1.6 | 2.4 % / 2.2 % |
| consumer | 7,866 / 15,873 | 6.5 / 6.7 | 1.0 / 0.5 | |
| no income yet | 8,557 / 18,726 | 9.8 / 10.7 | 0.5 / 0.3 | |
| pure producer | 27,574 / 6,209 | 0.3 / 0.3 | 0.3 / 0.2 | |

The second half is the same: light-fed mixotrophs retain 95.3 % / 97.5 % of carcass nutrient
retained, and decomposers 2.4 % / 1.5 %.

- **Light-fed mixotrophs release most of the carcass nutrient, and so do most of the
  mineralising.** Of the carcass nutrient bites release, 80 % / 76 % is excreted to the pool,
  because the consumer keeps only `demand × energy gained`. Light-fed mixotrophs' bites account for
  58 % / 55 % of that excretion, decomposers' for 22 %. They release 3.4 / 2.8 times what they keep.
  (Inferred from the drain rule; see §1.)
- **Decomposers keep almost nothing of what they drain.** They gain 7 % / 11 % of the structure
  they drain as energy, against 14 % / 18 % for light-fed mixotrophs, and their nutrient need is
  set by that energy. This fits the trait-distance kernel pricing a producer carcass for
  near-producers (#587), but this diagnostic does not separate it from body size.
- **Pool uptake is how nutrient enters life.** It supplies 96.7 % / 94.2 % of all nutrient
  acquired by living agents over the run (94.5 % / 89.6 % in the second half). Living drains supply
  0.5–0.8 %, carcass drains 2.8–9.5 %.
- **There is no network route.** Network redistribution is enabled (connection cap > 0) in 0 of
  the 475 seeds' configs.

## 5. Appetite over time (section D)

Mean `h_eff` of light-fed mixotrophs (producer, `h_eff > 0.05`) by quarter of the run (decoded /
fa 0):

| quarter | mean `h_eff` | median | % > 0.5 | all producers: % > 0.5 |
|---|---:|---:|---:|---:|
| Q1 | 0.606 / 0.548 | 0.455 / 0.395 | 46.2 / 39.8 | 13.7 / 13.4 |
| Q2 | 0.659 / 0.585 | 0.515 / 0.425 | 51.2 / 43.7 | 13.5 / 13.1 |
| Q3 | 0.688 / 0.609 | 0.555 / 0.435 | 54.5 / 45.7 | 18.2 / 15.6 |
| Q4 | 0.694 / 0.622 | 0.565 / 0.455 | 55.1 / 46.8 | 19.2 / 16.5 |

Heterotrophy among producers does not decline in these kin-clustered populations; it rises. This
is the opposite of the *Plodia* result, where lines kept in viscous food evolved lower cannibalism
(Boots et al. 2021; [cannibalism and kin](../ecology/cannibalism-and-kin.md)). The quarters are
population snapshots, so the rise mixes selection with survivorship of the large, long-lived,
heterotrophic agents of §3.

## 6. What the domain shows, and where the world departs from it

The ecology is in [trophic roles](../ecology/trophic-roles.md) (*Mixotrophy*, *Why decomposition
falls to specialists*), [plants](../ecology/plants.md) (*Root allocation and nutrient uptake*),
[fungi](../ecology/fungi.md) (*Mycorrhizal strategy*) and [cannibalism and
kin](../ecology/cannibalism-and-kin.md) (*Sessile and viscous consumers*). In brief:

| | domain | this world (ungated atlas) |
|---|---|---|
| How common producer mixotrophy is | On land, rare: carnivorous plants are a few hundred species, confined to bright, wet, nutrient-poor habitats (Givnish et al. 1984; Ellison & Gotelli 2001). In plankton, widespread (Stoecker 1998; Zubkov & Tarran 2008). | A third of all agents; a sixth carry `h_eff > 0.5`. |
| When it is expressed | When the limiting nutrient is short; less when nutrient is added (Nygaard & Tobiesen 1993; Ellison & Gotelli 2002). | On the richest pools, because light outgrows a fixed uptake (§3). |
| What it eats | Prey richer in the limiting nutrient and, in plankton, smaller than the mixotroph: insects, bacteria. Not its own kin, and not other plants. | Carcasses for nutrient; living kin for energy (§2). |
| Who processes detritus | Specialists. Most net primary production enters the detrital pathway, which runs mainly through microbes and detritivores (Cyr & Pace 1993). Producers take up mainly mineral nutrient. | Light-fed mixotrophs, 96–97 % of carcass nutrient retained (§4). |
| A producer's answer to nutrient shortage | More root (Poorter et al. 2012) and mycorrhizal partners, which reach organic nitrogen for it (Read & Perez-Moreno 2003; Smith & Read 2008). | Uptake cannot grow with the body, and the network is off; drains are the only lever left. |
| Kin cannibalism in sessile, viscous populations | Selected against (Boots et al. 2021). No clear natural analogue of a light-fed producer killing its kin. | Rises over the run (§5). |

Three departures in the physics line up with these:

- **Uptake does not scale with structure.** Light share does (`photosynthesise`: effective
  autotrophy × structure). Uptake demand is effective autotrophy × `u_A`, with no structure term and
  no contact or mobility check (`absorb_nutrients`). A producer that grows gains light it cannot
  match with nutrient. In the domain, uptake capacity grows with root mass, and allocation shifts
  toward roots under nutrient shortage.
- **The carcass drain has no decomposer term.** It is gated only by effective heterotrophy, reach
  and expression, with no autotrophy term and no carcass-specific efficiency (`resolve_drains`).
  There is no passive decay (`carcass_has_no_passive_decay`). A carcass carries all its nutrient,
  so it is the richest food in the world, and the agents nearest it, closest to it in trait space
  and most nutrient-limited are the light-fed producers beside it.
- **The mycorrhizal route is off.** Network redistribution is implemented and default-disabled
  (world rules, flow 5). It is not in `default_ranges` and its cap is 0 in every atlas config, so a
  producer cannot trade energy for nutrient with a decomposer.

## 7. Open design questions

Stated as questions for the design session; none is decided here.

1. **Should light-fed mixotrophy be the dominant producer mode?** The world rules intend the
   carnivorous-plant pattern: well-lit producers that consume for nutrient and cut back when it is
   plentiful. Here it is a third of all agents, not confined to poor ground, and rising. Is that the
   pattern working, or a sign that something the domain has is missing?
2. **Should producers hold the decomposer niche?** The design keeps "decomposer" behavioural (trait
   space), and expects a detrital guild of ordinary heterotrophs. The guild that forms here is light-
   fed producers. Is that an acceptable reading of "behavioural", or the by-construction route
   around decomposition the design wants to avoid?
3. **Should uptake scale with the body?** Light scales with structure; uptake does not. A root term
   would let a large producer answer nutrient shortage the way plants do. Would it remove the
   nutrient hunger that drives most of this, and what would it do to the detrital pathway?
4. **Should the autotrophy–mobility incompatibility be physical?** The world rules say movement
   breaks the substrate contact uptake needs. In the code uptake ignores mobility, and on the
   search baseline `mobility_maintenance_cost` is 0, so the incompatibility rests on per-distance
   movement cost alone.
5. **Should the network be on for genesis?** It is the domain's route by which producers reach
   organic nutrient without eating. It has never been switched on in the search.
6. **Is #637's "sated mixotrophs" bar still the right target?** Kin are killed because they are the
   only thing in reach, and the killers eat carcasses for nutrient. A gate that sates the killers on
   energy would leave the nutrient side, and the carcass route, where it is.

## 8. What this does not show

- **The world is ungated.** Every figure is at satiation sensitivity 0. A gated world selects
  differently (#624).
- **It is correlational.** Bins of `h_eff` are compared, not interventions. `h_eff`, size and light
  income move together, so section B cannot say which one drives nutrient limitation, births or
  lifespan.
- **Lifespan is censored.** Agents alive at T = 2000 are excluded, and the long-lived are the large
  and the heterotrophic, so the top bins' mean age at death is biased in an unknown direction.
- **Excretion is inferred.** The diagnostic books retained nutrient by the drain rule
  (`min(released, demand × energy gained)`) and reads the remainder as excreted. It does not read
  the nutrient grid.
- **Reach is geometric only.** "In reach" is `consumption_reach` at drain time. It does not ask
  whether a target would have been drained (expression, recognition, competition for the target).
- **The atlas is one search box.** `body_reach_coefficient`, `mobility_maintenance_cost` and the
  network parameters are fixed at the baseline (0), so none of their effects is seen.
- **Section 3 compares agents, not ticks.** A kin killer is any light-fed mixotroph that killed kin
  once.

## 9. Instrument

In `explorers-search`, committed with this note:

- **`examples/kin_killer_diet.rs`** (new, throwaway): §§1–3 (diet by target and fullness state,
  reach at each kin kill, traits of killers) and A–D (prevalence and investment, producers by
  `h_eff`, carcass processing and nutrient into life by route, producer heterotrophy by quarter).
  `--summary` reprints a report from the rows; `--crosscheck` runs `role_diet::rollout_with_fullness`
  per seed and prints its kin-kill count beside this tool's.
- **`intake_ceiling::realised_bites`**: the per-bite form of `realised_drains`, which now sums it.
  Behaviour of `realised_drains` and its callers is unchanged.
- **Artifacts** in `target/kkd/` and `target/kkd2/`: `{decoded,fa0}.{jsonl,md}`, `preview-*`,
  `run.sh`, `run.log`, `times.log`.

## References

Ecology references are listed in the linked ecology docs. Those cited here:

- Boots, M. et al. (2021). Experimental evidence that local interactions select against selfish behaviour. *Ecology Letters*.
- Cyr, H. & Pace, M.L. (1993). Magnitude and patterns of herbivory in aquatic and terrestrial ecosystems. *Nature* 361: 148–150.
- Ellison, A.M. & Gotelli, N.J. (2001). Evolutionary ecology of carnivorous plants. *Trends in Ecology & Evolution* 16(11): 623–629.
- Ellison, A.M. & Gotelli, N.J. (2002). Nitrogen availability alters the expression of carnivory in the northern pitcher plant, *Sarracenia purpurea*. *PNAS* 99(7): 4409–4412.
- Givnish, T.J., Burkhardt, E.L., Happel, R.E. & Weintraub, J.D. (1984). Carnivory in the bromeliad *Brocchinia reducta*, with a cost/benefit model for the general restriction of carnivorous plants to sunny, moist, nutrient-poor habitats. *American Naturalist* 124(4): 479–497.
- Nygaard, K. & Tobiesen, A. (1993). Bacterivory in algae: a survival strategy during nutrient limitation. *Limnology and Oceanography* 38(2): 273–279.
- Poorter, H. et al. (2012). Biomass allocation to leaves, stems and roots: meta-analyses of interspecific variation and environmental control. *New Phytologist* 193(1): 30–50.
- Read, D.J. & Perez-Moreno, J. (2003). Mycorrhizas and nutrient cycling in ecosystems – a journey towards relevance? *New Phytologist* 157(3): 475–492.
- Smith, S.E. & Read, D.J. (2008). *Mycorrhizal Symbiosis*. 3rd ed. Academic Press.
- Stoecker, D.K. (1998). Conceptual models of mixotrophy. *Journal of Eukaryotic Microbiology* 45(3): 255–261.
- Zubkov, M.V. & Tarran, G.A. (2008). High bacterivory by the smallest phytoplankton in the North Atlantic Ocean. *Nature* 455: 224–226.
