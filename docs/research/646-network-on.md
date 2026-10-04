# Issue #646: the atlas with network redistribution on

**Status: measurement, docs only. Adds a raw readout field to `Event` (`nutrient_delta`) and splits
the network's nutrient leg into its own `Redistributed` event in `explorers-sim`; neither touches
state, and every pin and proptest is unchanged. Adds network flags, a network-route table and a
connection census to the observer-only diagnostic `kin_killer_diet` (`explorers-search`). The
search, the evaluator and the prefilter are unchanged. No design decision is taken here: the
recommendation in §10 and the questions in §11 go to genesis and #647.**

#642 found that light-fed mixotrophs retain 96–97 % of carcass nutrient and decomposers about 2 %,
and that producers have no partnered route to organic nutrient. In the domain, producers reach
organic N and P through mycorrhizal partners, not by eating detritus ([fungi](../ecology/fungi.md)).
The sim has a mycorrhizal counterpart, network redistribution (world rules, flow 5; #249), but it
is off everywhere. Its design example is a producer rich in energy and poor in nutrient, linked to
a decomposer poor in energy and rich in nutrient, each gaining the currency it lacks. This note
switches the network on across the atlas and asks whether that partnership forms, and whether it
changes who processes detritus.

## TL;DR

1. **The intended partnership does not form** (§4, §9). Producer–producer links are 98.0–98.6 %
   of all connections in every setting; producer–decomposer links are 0.5–0.9 %, at or below what
   pairing by headcount would give. Net nutrient from decomposers to producers is about zero
   (−22k to +1.5k over the whole run, against producers' 7–17 M of non-network nutrient acquired:
   at most 0.2 %, and mostly negative). The network is not a mycorrhizal route from detritus to
   producers.
2. **What the network does instead** (§4). Gross network N is large (3.7–20 M over the run, a
   third to over half of all N entering agents), but 84–91 % of it is producer↔producer churn. Net, producers
   are donors. The net recipients are agents with no income yet (newborns, mostly) and consumers.
   Energy does flow down to heterotrophs, as the design intends for half the trade: decomposers'
   net network energy is 12–32× what they gain from all their drains. Nothing comes back.
3. **Carcass processing barely moves** (§3). Light-fed mixotrophs retain 93.7–95.4 % of carcass
   nutrient retained decoded (96.1 % off) and 94.5–95.7 % at fa 0 (97.4 %). Decomposers go from
   2.2 % to 2.9–4.6 % and from 1.6 % to 2.6–3.8 %. The decomposer guild stays at 0.4–0.7 % of
   agents. No decomposer guild appears.
4. **Mixotrophy thins and appetite turns down; kin-killing per mixotroph barely moves** (§2, §6,
   §7). Producers with `h_eff > 0.05` fall from 34 % to 13–25 % (decoded) and from 35 % to 17–28 %
   (fa 0). Light-fed mixotrophs' mean `h_eff` now falls over the run in every setting (0.61 → 0.69
   off; 0.52 → 0.41 at c4-lowfast). Kin kills per 1000 producer agent-ticks fall from 2.14 to
   0.70–1.48 decoded and from 2.54 to 0.98–1.81 at fa 0. But producer agent-ticks rise 2.2–5.4×,
   so absolute kin kills rise (13,045 → 18,403–26,008 decoded), and per light-fed mixotroph the
   second-half rate moves from 3.3 to 2.6–4.0 (decoded) and from 3.5 to 2.3–3.5 (fa 0).
5. **Persistence falls, through nutrient lockup** (§8). Persisted seeds fall from 403 to 306–340
   of 475 (decoded) and from 423 to 254–303 (fa 0). Nutrient lockup goes from 40–41 seeds to
   108–164, and monoculture from 1 to 4–56. Every setting is worse than off; 18–49 configs lose
   two seeds or more.
6. **Producers stop being nutrient-limited and start being crowded** (§5). The share of producers
   that are N-limited falls from 37 % to 2–11 % (decoded), and light per producer falls from 6.4 to
   1.2–3.0. The most heterotrophic producers (`h_eff ≥ 1`) stay 51–60 % N-limited.

**Recommendation** (§10): keep the network out of genesis's `default_ranges`. As built, it levels
nutrient among neighbouring producers rather than trading it across guilds, and on the atlas it
costs persistence in every setting. What it would need to form the intended partnership is a
design question for #647 (§11).

## 1. What ran

| | |
|---|---|
| Tree | `6fe3aec` (branch `issue-646-network-measurement`), release build. |
| Diagnostic | `kin_killer_diet` on the atlas (`atlas.json`, 95 configs), seeds 1000–1004 (475 seeds), T = 2000, `EvalConfig::default()`, **ungated** as in #642 and #645, both as decoded and at `--founder-aggregation 0`. Uptake scaling off (`b = 0`). |
| Settings | Off plus six network settings, × 2 modes; the flags set all five network parameters on every decoded config. |
| Check | With the network off, every table is byte-identical to #645's `b = 0` run (`diff target/645/b0-{decoded,fa0}.md target/646/off-{decoded,fa0}.md` differs only in the new header line naming the network setting), and so to #642's. |
| Machine | 8-core laptop, one process at a time. |

**Grid.** Transfer efficiency is 0.9 in every setting. Costs are set against the atlas's metabolic
cost (median about 2 per tick) and reserve surplus (median about 0–20): "low" creation and
maintenance costs are a few percent of a tick's metabolism, "moderate" about half. c2-test uses
the values of the mutualism capstone test in `explorers-sim` (`lib.rs`, #414).

| setting | connection cap | creation cost | maintenance cost | redistribution rate |
|---|---:|---:|---:|---:|
| off | 0 | – | – | – |
| c1-low | 1 | 0.1 | 0.01 | 0.1 |
| c1-mod | 1 | 1.0 | 0.1 | 0.3 |
| c2-test | 2 | 0.2 | 0.05 | 0.4 |
| c4-low | 4 | 0.1 | 0.01 | 0.1 |
| c4-lowfast | 4 | 0.1 | 0.01 | 0.3 |
| c4-mod | 4 | 1.0 | 0.1 | 0.3 |

The cap bounds the connections an agent *builds*, not its degree: an agent can also be the partner
of others' connections, so mean live connections per agent can exceed the cap (§9).

**Commands** (cwd is the repo root; driver `target/646/run.sh`):

```sh
cargo build --release -p explorers-search --example kin_killer_diet
C=$(seq -s, -f 'atlas:%g' 0 94)
# per setting, e.g. c4-lowfast; repeat with --founder-aggregation 0 for fa0
./target/release/examples/kin_killer_diet --atlas atlas.json --configs $C \
    --network-connection-cap 4 --network-creation-cost 0.1 --network-maintenance-cost 0.01 \
    --network-redistribution-rate 0.3 --network-transfer-efficiency 0.9 \
    --out target/646/c4-lowfast-decoded.jsonl > target/646/c4-lowfast-decoded.md
```

Off passes no network flag.

**Wall clock** (`target/646/times.log`; seconds, decoded / fa 0): off 52 / 108; c1-low 107 / 146;
c1-mod 110 / 154; c2-test 252 / 281; c4-low 278 / 342; c4-lowfast 616 / 549; c4-mod 152 / 189. In
total 3,336 s, about 56 min. Runs with the network on take 1.4–12× longer than off. The worlds hold
more agents (§2), and the network pass scans the connection list per agent.

**Populations and definitions** are #642's and #645's. A **light-fed mixotroph** in sections 1–2
and the kin-kill counts is a Producer-role agent with heterotrophy above zero; sections A–D use
`h_eff > 0.05`. Roles are the recent-income role (`income.rs`), which **does not count network
income** (§12). New here: the network tables under section C (network N and E in and out per
recipient role, and donor → recipient flow matrices, read off the `Redistributed` events) and
section F (connections on sample ticks by endpoint roles and kinship). Exact flows between roles
below are summed from the rows (`network_n_flow`, `network_e_flow`); the summaries print them as
rounded percentages.

## 2. Prevalence (section A)

Second-half sample ticks, every living agent:

| setting | producers, % `h_eff > 0.05` | % `h_eff > 0.5` | consumer % | decomposer % | decomposer samples | producer samples |
|---|---:|---:|---:|---:|---:|---:|
| *decoded* off | 34.0 | 18.6 | 0.1 | 0.3 | 507 | 194,771 |
| c1-low | 24.5 | 9.1 | 0.4 | 0.5 | 2,794 | 534,441 |
| c1-mod | 23.6 | 8.5 | 0.4 | 0.6 | 3,588 | 632,222 |
| c2-test | 16.7 | 5.4 | 0.4 | 0.6 | 7,497 | 1,217,950 |
| c4-low | 15.7 | 4.4 | 0.4 | 0.6 | 7,413 | 1,172,888 |
| c4-lowfast | 13.0 | 3.7 | 0.3 | 0.6 | 10,792 | 1,876,944 |
| c4-mod | 18.8 | 6.3 | 0.3 | 0.4 | 3,593 | 792,065 |
| *fa 0* off | 34.7 | 16.1 | 0.2 | 0.4 | 1,955 | 494,851 |
| c1-low | 28.2 | 8.8 | 0.5 | 0.6 | 5,941 | 979,353 |
| c1-mod | 28.3 | 9.2 | 0.5 | 0.5 | 5,492 | 1,069,130 |
| c2-test | 22.5 | 7.0 | 0.4 | 0.7 | 11,763 | 1,637,049 |
| c4-low | 20.0 | 5.4 | 0.5 | 0.7 | 11,059 | 1,634,255 |
| c4-lowfast | 17.1 | 4.4 | 0.4 | 0.6 | 15,462 | 2,376,876 |
| c4-mod | 24.4 | 7.6 | 0.4 | 0.6 | 7,646 | 1,322,001 |

- **No decomposer guild appears.** Decomposers double as a share at most (0.3 → 0.4–0.6 %
  decoded). In absolute samples they rise up to 21× decoded, but producers rise up to 9.6× in the
  same second half: the worlds are much denser.
- **Mixotrophy thins, roughly with network capacity.** The share above 0.05 falls most at cap 4
  with cheap links (13.0 / 17.1 %) and least at cap 1 (24.5 / 28.2 %). Above 0.5 it falls by a
  factor of 2–5 decoded. This is as strong as `b = 1` in #645 (15.1 / 21.8 %).
- **Consumers triple as a share** (0.1 → 0.3–0.4 % decoded), still a sliver.

## 3. Who processes the detritus (section C)

Whole run unless marked, recipient by role at the start of the tick. "Own N from carcass" is the
carcass-drain share of the role's non-network nutrient (main C table).

| setting | % of carcass N retained: light-fed mixotroph | decomposer | consumer | second half: mixotroph / decomposer | decomposers' own N from carcass % | carcass N retained, all (k) |
|---|---:|---:|---:|---|---:|---:|
| *decoded* off | 96.1 | 2.2 | 1.0 | 95.3 / 2.4 | 9.9 | 106 |
| c1-low | 95.4 | 2.9 | 1.3 | 95.4 / 3.3 | 7.6 | 178 |
| c1-mod | 95.0 | 3.5 | 1.2 | 94.1 / 4.3 | 11.6 | 238 |
| c2-test | 94.3 | 3.9 | 1.5 | 93.8 / 4.6 | 7.1 | 226 |
| c4-low | 94.6 | 3.4 | 1.6 | 94.2 / 3.7 | 6.1 | 195 |
| c4-lowfast | 93.7 | 4.6 | 1.4 | 93.1 / 5.4 | 6.8 | 240 |
| c4-mod | 95.0 | 3.6 | 1.2 | 95.2 / 3.7 | 9.6 | 209 |
| *fa 0* off | 97.4 | 1.6 | 0.5 | 97.5 / 1.5 | 9.0 | 368 |
| c1-low | 95.3 | 2.7 | 1.5 | 94.3 / 3.7 | 12.1 | 518 |
| c1-mod | 95.7 | 2.6 | 1.1 | 95.2 / 3.1 | 12.0 | 558 |
| c2-test | 94.5 | 3.8 | 1.3 | 94.0 / 4.3 | 10.8 | 532 |
| c4-low | 95.7 | 2.9 | 1.2 | 94.1 / 4.6 | 8.6 | 483 |
| c4-lowfast | 95.0 | 3.6 | 1.2 | 93.7 / 5.0 | 8.5 | 533 |
| c4-mod | 95.5 | 3.0 | 1.0 | 94.6 / 3.7 | 11.0 | 524 |

- **Carcass processing does not move to decomposers.** Their share rises by one to three points;
  light-fed mixotrophs keep 93–96 %. This is the same size of shift as #645's (2.2 → 2.9–4.6 %).
- **More carcass nutrient is processed in total** (106k → 178–240k decoded), because there are more
  bodies. The split stays the same.
- **Decomposers do not live more by decomposition.** Their own carcass share stays at 6–12 %,
  against 10 % off. Pool uptake still gives them most of their non-network nutrient. (Contrast
  #645, where small bodies taking up little pushed it to 45–68 %.)
- **Mixotrophs' diet shifts a little toward kin** (section 1). Carcasses supply 75–83 % of the drain
  nutrient light-fed mixotrophs retain (88.5 % off) decoded and 81–86 % at fa 0 (92.3 %); living
  kin supply 51–56 % of their drain energy (42 / 40 % off). At 76–85 % of their kin kills only kin
  are in reach, as off (80 / 85 %).

## 4. Producers' nutrient by route, and the network, net of churn

The network tables under section C count network N *in* (received) and *out* (given) per role.
Gross "in" is dominated by back-and-forth between producers, so the readout below is the net
column and the donor → recipient flows.

| setting | gross network N (M) | producer↔producer share of gross % | decomposer → producer N | producer → decomposer N | net decomposer → producer | net as % of producers' non-network N | decomposers' net network E | decomposers' drain E (carcass + living) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| *decoded* c1-low | 3.65 | 91.3 | 14,918 | 26,030 | −11,112 | −0.15 | 34,078 | 2,023 |
| c1-mod | 4.87 | 85.9 | 18,737 | 28,455 | −9,718 | −0.12 | 73,265 | 2,781 |
| c2-test | 13.50 | 86.4 | 62,497 | 60,994 | +1,503 | +0.01 | 115,224 | 3,561 |
| c4-low | 7.86 | 89.9 | 44,560 | 50,229 | −5,669 | −0.06 | 51,555 | 2,353 |
| c4-lowfast | 15.45 | 86.1 | 77,420 | 77,449 | −29 | −0.00 | 98,247 | 3,771 |
| c4-mod | 7.64 | 86.7 | 26,474 | 37,343 | −10,869 | −0.12 | 79,457 | 2,667 |
| *fa 0* c1-low | 5.45 | 90.1 | 30,240 | 48,863 | −18,623 | −0.16 | 68,981 | 5,744 |
| c1-mod | 7.08 | 84.0 | 29,840 | 51,619 | −21,779 | −0.18 | 93,667 | 5,785 |
| c2-test | 17.75 | 84.5 | 95,532 | 96,846 | −1,314 | −0.01 | 140,206 | 7,875 |
| c4-low | 10.22 | 88.8 | 58,694 | 69,869 | −11,175 | −0.08 | 84,018 | 5,491 |
| c4-lowfast | 20.23 | 85.1 | 116,410 | 116,643 | −233 | −0.00 | 143,602 | 8,258 |
| c4-mod | 11.47 | 84.2 | 53,161 | 64,750 | −11,589 | −0.09 | 113,038 | 6,827 |

"Producer" is light-fed mixotrophs plus pure producers. Over configs, net decomposer → producer N
is positive in 20–40 of 95.

- **There is no mycorrhizal route.** Net nutrient from decomposers to producers is zero to within
  0.2 % of what producers acquire otherwise, and in eleven of twelve runs it runs the other way.
  Decomposers have no nutrient surplus to send: carcasses give them 2–5 % of carcass N retained
  (§3), and on net nutrient flows to them over the network in eight of twelve runs.
- **Half the trade happens: energy.** Producers send energy down the reserve gradient to
  heterotrophs (36–171k producer → decomposer, 2–28k back). Decomposers' net network energy is
  12–32× the energy they gain from carcasses and living prey together; consumers' is 15–32×. By
  energy, the network-on "decomposers" and "consumers" live on producer subsidy. The role ledger
  cannot see this (§12), so they are still classified by their small drain income.
- **The gross figure is churn.** Producers' "% N from network" is 18–56 % gross, but their net is
  negative in every run: they give more than they get. The flow matrix puts 84–91 % of gross
  network N on producer↔producer links, moving free nutrient back and forth between neighbours of
  similar standing.
- **The net recipients are newborns and consumers.** Agents with no income yet, mostly newborns,
  net 113k–1.09M N over the run; the network supplies 58–91 % of the N they acquire (gross). Consumers net 16–45k. Flow
  follows the absolute free store, so nutrient moves from large, established bodies to small new
  ones. Since 28–69 % of links are between kin (§9), much of this is plausibly parental
  provisioning after birth, but the diagnostic does not split recipients by kinship.

## 5. Conditionality (section B)

Second half, per agent-sample means. Section B's nutrient routes exclude the network.

| setting | all producers: % N-limited | pure: % N-limited | all producers: light E | `h_eff ≥ 1`: % N-limited | `≥ 1`: pool | `≥ 1`: uptake | `≥ 1`: N from carcass % |
|---|---:|---:|---:|---:|---:|---:|---:|
| *decoded* off | 37.2 | 30.4 | 6.38 | 64.5 | 286 | 0.212 | 12.5 |
| c1-low | 10.8 | 7.2 | 3.01 | 56.6 | 313 | 0.329 | 15.3 |
| c1-mod | 8.3 | 5.2 | 2.73 | 52.3 | 270 | 0.352 | 17.2 |
| c2-test | 3.4 | 1.7 | 1.64 | 52.4 | 531 | 0.345 | 23.9 |
| c4-low | 4.1 | 2.1 | 1.65 | 58.3 | 337 | 0.289 | 16.4 |
| c4-lowfast | 2.2 | 1.0 | 1.23 | 51.1 | 574 | 0.375 | 25.7 |
| c4-mod | 5.6 | 3.0 | 2.19 | 59.8 | 323 | 0.273 | 18.8 |
| *fa 0* off | 33.0 | 29.7 | 5.90 | 60.2 | 274 | 0.263 | 20.8 |
| c1-low | 12.4 | 8.7 | 3.01 | 59.2 | 252 | 0.267 | 23.8 |
| c1-mod | 10.1 | 6.7 | 2.79 | 56.2 | 256 | 0.287 | 20.0 |
| c2-test | 4.5 | 2.5 | 1.85 | 53.4 | 352 | 0.283 | 21.2 |
| c4-low | 5.8 | 3.1 | 1.86 | 56.6 | 355 | 0.331 | 19.9 |
| c4-lowfast | 3.0 | 1.5 | 1.41 | 51.8 | 401 | 0.300 | 25.1 |
| c4-mod | 6.4 | 4.0 | 2.31 | 54.8 | 266 | 0.303 | 19.2 |

- **Producers become energy-limited, not nutrient-limited.** Pure producers go from 30 % N-limited
  to 1–9 %. Light per producer falls by half to four fifths, because the denser worlds share the
  same flux among more bodies. The network evens out free nutrient between neighbours, so few
  producers are short of it.
- **The most heterotrophic producers are not relieved.** At `h_eff ≥ 1`, 51–60 % are still
  N-limited, they still stand on the richest pools, and carcasses supply a larger share of their
  non-network nutrient than off decoded (15–26 % against 12.5 %); at fa 0 it is about the same
  (19–25 % against 20.8 %). The domain's conditional
  pattern (mixotrophy where nutrient is short) is not shown, as in #642 and #645.

## 6. Appetite over time (section D)

| setting | light-fed mixotrophs' mean `h_eff`, Q1 → Q4 | all producers, % `h_eff > 0.5`, Q1 → Q4 |
|---|---|---|
| *decoded* off | 0.606 → 0.694 | 13.7 → 19.2 |
| c1-low | 0.538 → 0.521 | 10.2 → 10.1 |
| c1-mod | 0.545 → 0.502 | 10.9 → 9.4 |
| c2-test | 0.517 → 0.435 | 8.7 → 5.9 |
| c4-low | 0.530 → 0.410 | 9.2 → 4.4 |
| c4-lowfast | 0.521 → 0.405 | 8.1 → 3.7 |
| c4-mod | 0.525 → 0.464 | 9.5 → 6.7 |
| *fa 0* off | 0.548 → 0.622 | 13.4 → 16.5 |
| c1-low | 0.494 → 0.457 | 10.0 → 9.4 |
| c1-mod | 0.494 → 0.475 | 10.6 → 9.9 |
| c2-test | 0.479 → 0.439 | 9.0 → 7.6 |
| c4-low | 0.477 → 0.425 | 8.8 → 5.7 |
| c4-lowfast | 0.467 → 0.391 | 8.3 → 4.6 |
| c4-mod | 0.488 → 0.455 | 9.8 → 8.1 |

Off, heterotrophy rises over the run. With the network on it falls in every setting, most at cap
4 with cheap links (0.52 → 0.41; producers above 0.5 fall by more than half). Unlike #645, the
first quarter already differs from off (0.47–0.55 against 0.55–0.61): the network acts from the
first ticks. The quarters are population snapshots, so selection and survivorship are mixed.

## 7. Kin kills, rate and absolute

Kin kills by light-fed mixotrophs (section 1's population), one per killing pair. The rate is per
1000 Producer-role agent-ticks. "Per mixotroph" divides second-half kin kills by Producer
agent-ticks × section 1's share of producers (from the sample ticks, so approximate), as in #645.

| setting | rate, whole run | rate, second half | kin kills, whole run | kin kills, second half | Producer agent-ticks, whole / second half (M) | per mixotroph, second half |
|---|---:|---:|---:|---:|---:|---:|
| *decoded* off | 2.14 | 1.33 | 13,045 | 2,592 | 6.10 / 1.95 | 3.3 |
| c1-low | 1.37 | 0.99 | 18,403 | 5,284 | 13.43 / 5.33 | 3.1 |
| c1-mod | 1.48 | 1.24 | 21,584 | 7,810 | 14.57 / 6.31 | 4.0 |
| c2-test | 1.06 | 0.83 | 26,008 | 10,114 | 24.42 / 12.16 | 3.7 |
| c4-low | 0.88 | 0.56 | 20,298 | 6,564 | 22.95 / 11.71 | 2.6 |
| c4-lowfast | 0.70 | 0.47 | 23,127 | 8,884 | 33.05 / 18.73 | 2.8 |
| c4-mod | 1.09 | 0.73 | 19,473 | 5,779 | 17.80 / 7.91 | 2.9 |
| *fa 0* off | 2.54 | 1.41 | 33,983 | 6,957 | 13.39 / 4.95 | 3.5 |
| c1-low | 1.73 | 1.12 | 41,844 | 10,998 | 24.17 / 9.79 | 3.3 |
| c1-mod | 1.81 | 1.22 | 45,916 | 13,025 | 25.39 / 10.69 | 3.5 |
| c2-test | 1.27 | 0.76 | 45,445 | 12,473 | 35.65 / 16.35 | 2.7 |
| c4-low | 1.27 | 0.74 | 44,131 | 12,020 | 34.76 / 16.32 | 2.9 |
| c4-lowfast | 0.98 | 0.51 | 44,842 | 12,154 | 45.63 / 23.73 | 2.3 |
| c4-mod | 1.48 | 0.92 | 43,597 | 12,201 | 29.50 / 13.20 | 3.0 |

- **Per producer, kin-killing falls by 31–67 %** (whole run, decoded), most at cap 4 with cheap
  links.
- **In absolute terms it rises.** Whole-run kin kills rise 41–99 % decoded and 23–35 % at fa 0; in
  the second half they double to quadruple decoded. Producer agent-ticks rise 2.2–5.4× (decoded)
  and 1.8–3.4× (fa 0) over the run.
- **Per light-fed mixotroph, the rate moves little.** At cap 4 it falls by a tenth to a third
  (3.3 → 2.6–2.9 decoded, 3.5 → 2.3–3.0 at fa 0); at cap 1 and c2-test it is flat or higher
  (c1-mod decoded 4.0). As in #645, the fall per producer is mostly fewer mixotrophs.

## 8. Persistence and failure modes (section E)

Census verdicts over 475 seeds; the last two columns count configs whose persisted seeds change
against off.

| setting | persisted | extinction | energy death | nutrient lockup | monoculture | generalist dominance | configs losing ≥ 1 / ≥ 2 | configs gaining |
|---|---:|---:|---:|---:|---:|---:|---|---:|
| *decoded* off | 403 | 28 | 1 | 40 | 1 | 2 | – | – |
| c1-low | 338 | 14 | 0 | 115 | 4 | 4 | 43 / 18 | 8 |
| c1-mod | 340 | 13 | 0 | 109 | 11 | 2 | 40 / 18 | 6 |
| c2-test | 306 | 11 | 0 | 130 | 21 | 7 | 52 / 37 | 11 |
| c4-low | 335 | 12 | 0 | 108 | 18 | 2 | 46 / 22 | 10 |
| c4-lowfast | 307 | 14 | 1 | 109 | 35 | 9 | 56 / 31 | 8 |
| c4-mod | 319 | 16 | 0 | 121 | 14 | 5 | 53 / 27 | 10 |
| *fa 0* off | 423 | 8 | 0 | 41 | 1 | 2 | – | – |
| c1-low | 303 | 5 | 0 | 141 | 16 | 10 | 54 / 30 | 3 |
| c1-mod | 293 | 3 | 0 | 154 | 16 | 9 | 58 / 32 | 3 |
| c2-test | 256 | 3 | 0 | 164 | 37 | 15 | 69 / 48 | 5 |
| c4-low | 276 | 5 | 0 | 149 | 36 | 9 | 60 / 47 | 4 |
| c4-lowfast | 254 | 3 | 0 | 146 | 56 | 16 | 70 / 49 | 7 |
| c4-mod | 280 | 3 | 0 | 142 | 39 | 11 | 59 / 41 | 4 |

- **Every setting loses persistence**, by 63–97 seeds decoded and 120–169 at fa 0. Even the
  cheapest, slowest network (c1-low) costs 65 / 120 seeds.
- **Nutrient lockup is the main cause.** It roughly triples to quadruples (40 → 108–130; 41 →
  141–164). 24–40 configs that had no lockup seed off now have one. Lockup is the dead pool's
  share of nutrient trending high; with more bodies dying and the carcass split unchanged (§3),
  carcass nutrient piles up faster than its processors turn it over.
- **Monoculture and generalist dominance rise** (3 → 8–44 seeds decoded, 3 → 25–72 at fa 0), as
  denser producer worlds lose diversity. **Extinction halves** (28 → 11–16; 8 → 3–5): the network
  props up worlds that would otherwise die out, and they fail later by lockup instead.
- **It is not one-seed noise.** 18–49 configs lose two seeds or more, against 3–11 that gain any.
  The atlas was searched with the network off, so these configs are tuned for the other rule
  (§12).

## 9. Who connects to whom (section F)

Mean live connections per agent-sample = 2 × connections / living agents. Endpoint roles and kin
shares are second half, on sample ticks.

| setting | mean live connections, whole / second half | formed, whole run | % producer–producer | % producer–decomposer | % producer–consumer | % kin, all | % kin, producer–decomposer |
|---|---|---:|---:|---:|---:|---:|---:|
| *decoded* c1-low | 1.53 / 1.55 | 96,721 | 98.5 | 0.6 | 0.6 | 69.4 | 50.0 |
| c1-mod | 1.37 / 1.43 | 72,334 | 98.4 | 0.6 | 0.6 | 65.9 | 45.3 |
| c2-test | 2.91 / 3.10 | 253,794 | 98.4 | 0.7 | 0.4 | 43.2 | 28.7 |
| c4-low | 5.06 / 5.51 | 365,378 | 98.2 | 0.7 | 0.5 | 32.6 | 20.9 |
| c4-lowfast | 5.33 / 5.67 | 525,796 | 98.4 | 0.6 | 0.4 | 28.3 | 19.5 |
| c4-mod | 2.94 / 3.34 | 172,161 | 98.6 | 0.5 | 0.4 | 40.0 | 25.5 |
| *fa 0* c1-low | 1.52 / 1.56 | 135,186 | 98.2 | 0.8 | 0.8 | 66.9 | 46.6 |
| c1-mod | 1.36 / 1.43 | 106,428 | 98.3 | 0.6 | 0.7 | 66.5 | 51.7 |
| c2-test | 2.78 / 2.93 | 306,251 | 98.0 | 0.9 | 0.6 | 47.9 | 32.8 |
| c4-low | 4.70 / 5.18 | 473,512 | 98.0 | 0.8 | 0.6 | 37.3 | 26.5 |
| c4-lowfast | 4.96 / 5.42 | 621,607 | 98.0 | 0.7 | 0.4 | 32.6 | 21.8 |
| c4-mod | 2.81 / 3.07 | 235,086 | 98.1 | 0.6 | 0.5 | 46.6 | 36.1 |

- **The network is a producer mat.** 98 % of links join two producers in every setting.
  Producer–decomposer links are 0.5–0.9 %. With decomposers at 0.4–0.7 % of agents, random pairing
  by headcount would give about 1 %, so decomposers are if anything under-connected. Most
  decomposers can afford few links of their own (formation is surplus-gated), and producers fill
  their cap with whoever they touch.
- **Formation follows contact, and contact is kin.** At cap 1, two thirds of links are between
  kin; the cheapest neighbour to wire to is the one beside you, and a sessile producer's neighbours
  are its parent, offspring and siblings. The deterministic tie-break takes the lowest partner id,
  which favours older neighbours. Higher caps reach past the nearest kin, and the kin share falls
  to 28–47 %.
- **The cap binds.** Mean degree sits near or above the cap (1.4–1.6 at cap 1, 4.7–5.7 at cap 4
  with cheap links). Moderate costs (c4-mod) roughly halve the degree at cap 4, but do not change
  who links to whom.

## 10. Recommendation for genesis's search box

**Keep the network out of `default_ranges`.** (`network_parameters_are_not_searched` in
`crates/explorers-search/src/search.rs` stays as is.)

- **It does not do the job it was designed for.** The design's case for the network is the
  producer–decomposer trade (flow 5). On the atlas, at every cap and cost tried, that trade does
  not form: links follow contact, contact is producer to producer and mostly kin, and net nutrient
  from decomposers to producers is zero. Putting five parameters in the box would ask the search to
  tune a mechanism whose intended behaviour the physics does not currently produce.
- **What it does do is not neutral.** It levels free nutrient among neighbouring producers,
  provisions newborns, and subsidises heterotrophs with producer energy. That thins mixotrophy and
  turns appetite down, as #645's `b` does, but it also makes the worlds denser and roughly triples
  nutrient lockup. On the atlas it costs persistence in every setting, by 16–40 %.
- **A search could still find worlds it suits.** The atlas was searched with the network off, so
  this is a perturbation of worlds tuned without it (§12). The lockup cost might be searched away.
  But the search would then be finding worlds for a nutrient-levelling producer mat, not for
  mutualism, and nothing in the evaluator rewards the partnership.
- **If it is put in the box anyway**, the measured settings suggest a small box: connection cap
  over {0, …, 4} (0 keeps every current world reachable), redistribution rate over [0, 0.3],
  creation and maintenance costs fixed at the low values (0.1, 0.01; moderate costs moved little
  except degree), and transfer efficiency fixed at 0.9. That is two new dimensions, not five. It is
  not recommended until #647 settles what the network is for.

Not recommended: changing any network default. Every default stays 0.

## 11. What this means for #647

#647 asks whether light-fed mixotrophs should hold the decomposer niche, and whether to wait for
#645 and #646, which might remove the pattern without a design change. Neither does.

- **Producers still hold the decomposer niche, with either mechanism.** #645's size-scaled uptake
  and #646's network both thin mixotrophy (to 13–28 % of producers with the network, 15–22 % at `b = 1`) and both turn
  appetite down over the run. Neither moves carcass processing: light-fed mixotrophs retain over
  92 % of carcass nutrient retained in every setting of both. The remaining causes from #642's §6
  are untouched by both: the carcass drain has no decomposer term and no autotrophy cost, and a
  carcass carries all its nutrient.
- **They differ on persistence.** `b > 0` relieved lockup (40 → 5–9 seeds) at the cost of more
  monoculture. The network worsens lockup (40 → 108–130) and monoculture both. They have not been
  run together.
- **Kin-killing is not solved by thinning, again.** Per light-fed mixotroph the rate moves by a
  third at most, and denser worlds hold more kin kills in absolute terms. The spatial cause (#642,
  §2) is unchanged: at 76–85 % of kin kills only kin are in reach.
- **The network as built cannot deliver the mycorrhizal route.** If #647 decides producers should
  reach organic nutrient through partners rather than by eating carcasses, flow 5 would need to
  change. Open questions for the grill, not settled here:
  1. **Partner choice.** Should formation pick partners by complementarity (the builder rich in
     one currency, the partner rich in the other) rather than by contact and lowest id? In the
     domain the plant does not wire to whatever touches its root; it sanctions partners that do
     not deliver (Kiers et al. 2011), the refinement flow 5 defers.
  2. **Formation on a gradient.** Should a connection form only where a currency gradient runs
     both ways across it? Today a builder links to any contacted neighbour it can afford, so links
     between near-identical kin, with no trade to make, dominate.
  3. **What the gradient reads.** Flow follows the absolute free store, so nutrient moves from big,
     established bodies to small, new ones regardless of need. Should it follow a ratio (store
     against demand, or nutrient against energy), so that an N-limited producer is a recipient
     even when its store is large?
  4. **Can a decomposer be nutrient-rich at all?** The partnership needs a decomposer with
     nutrient to spare. Here decomposers retain 2–5 % of carcass nutrient and are not richer in
     free nutrient than their producer partners. A network change alone cannot fix that; it
     depends on #647's answer about the carcass drain.
  5. **Persistence under subsidy.** Heterotrophs take 12–34× their drain energy from the network
     and return no nutrient, the free-rider case flow 5 says the builder should shed under stress.
     At these costs producers are not stressed enough to shed them. Is blunt selection on
     maintenance enough, or does the network need the deferred per-partner ranking?

## 12. What this does not show

- **The world is ungated.** Every figure is at satiation sensitivity 0, as in #642 and #645. A
  gated world selects differently (#624).
- **The atlas was searched with the network off.** Its 95 configs are worlds viable without it.
  Switching it on measures a perturbation, not the worlds genesis would find with it in the box.
  Persistence and failure modes in §8 are for those configs only.
- **Transfer efficiency is fixed at 0.9.** Lower efficiency would tax the energy subsidy to
  heterotrophs, and might change who gains from a link; it was not varied.
- **Roles are blind to network income.** The recent-income role ledger (`income.rs`) counts light,
  pool uptake and drains, not network transfers. A "decomposer" here is an agent whose small drain
  income is mostly carcass, even when most of its energy comes over the network (§4). Section B's
  nutrient routes and "% N-limited" also leave the network out.
- **It is correlational.** Settings change density, light per producer and the mixotroph share
  together. The kin share of network nutrient flow to newborns is inferred, not measured.
- **Gross and net.** Gross network N counts churn between producers; only the net columns and the
  flow matrices support a statement about routes.
- **Lifespans are censored** at T = 2000 and the worlds are much denser, so section B's ages are not
  compared.
- **Five seeds per config.** Per-config changes of one seed are within noise (§8 counts ≥ 2).

## 13. Instrument

Committed before this note (`6fe3aec`):

- **`explorers-sim`**: `Event` gains a raw `nutrient_delta` field (0.0 on every event except the
  network's nutrient leg). The nutrient leg of flow 5 is now its own `Redistributed` event, with
  source = nutrient donor and target = recipient, so its direction is unambiguous when energy and
  nutrient flow opposite ways along a connection. The energy leg is unchanged. Neither touches
  state. Two phase-level network unit tests now assert the split events, and one new hand-built
  network world tests the readout.
- **`examples/kin_killer_diet.rs`**: `--network-connection-cap`, `--network-creation-cost`,
  `--network-maintenance-cost`, `--network-redistribution-rate` and
  `--network-transfer-efficiency`, applied to every decoded config (`config_source::with_network`).
  The header names the setting. With the network on, section C gains a network-route table per
  window (N and E in, out and net per recipient role; donor → recipient flow matrices for N and E),
  and section F counts connections on sample ticks by endpoint roles and kinship, with mean live
  connections, formed and dissolved, and a per-config table.
- **Tests**: `cargo test --workspace`, 951 passed, 0 failed. No pin or proptest changed. With the
  network off the output is byte-identical to #645's `b = 0` run apart from the header line.
- **Artifacts** in `target/646/`: `<setting>-{decoded,fa0}.{jsonl,md,log}`, `preview/` (a 10-config
  decoded preview), `run.sh`, `run.log`, `times.log`. `--summary --out <file>` reprints a report
  from the rows.

## References

- Kiers, E. T. et al. (2011). Reciprocal rewards stabilize cooperation in the mycorrhizal symbiosis. *Science* 333: 880–882.
- Simard, S. W. (2018). Mycorrhizal networks facilitate tree communication, learning, and memory. In *Memory and Learning in Plants*, Springer.
