# Issue #546 — decomposer guilds inside gated worlds: mostly a straddle of the producer / heterotroph line, not a separate diet

**Status: measurement. Adds the `guild_structure` instrument (explorers-search) and
widens four read-only accessors (below) so it reads the evaluator's own values; changes no
stepper, evaluator, gate, threshold or search behaviour. The gate decision this feeds stays
open on #546.** Stepper, evaluator and search at `c86c456` (the #494 census baseline).

## TL;DR

Of the 21 LHS (config, seed) pairs the #494 census read a decomposer guild on at `T = 2000`,
**all 21 reproduce** their census verdict and `has_decomposer_guild = true`. None are
excluded. Under the rule below:

| group | A (straddle) | B (behavioural guild) | ambiguous |
|---|---|---|---|
| `monoculture` (12) | **5** | **2** | **5** |
| `generalist_dominance` (3) | 1 | 1 | 1 |
| live controls (6) | 1 | 5 | 0 |

1. **The diet side never sits on a knife edge.** On all 21 seeds the decomposers' median
   detrital reliance is **1.00**. They eat carcasses and nothing else, well clear of the 0.5
   cut. The consumer / decomposer split is real everywhere. Any artefact is on the
   producer / heterotroph side.
2. **On that side, gated decomposers sit near the photo = hetero line.** Mean heterotrophy
   share of the decomposers is 0.56–0.83 in the 12 monoculture seeds (median 0.63), against
   0.63–0.96 in the controls (median 0.85). In the five ambiguous monoculture seeds, 23–44 % of
   decomposers sit within 0.1 of the line. So the decomposers there are the heterotroph tail of
   one continuum running up from the line. They are not a separate cluster.
3. **The monoculture gate is not marginal.** All 12 monoculture seeds read
   `clustering_strength` 0.00–0.37 against the 0.5 threshold (margin −0.13 to −0.50). The
   `sample:20` "knife edge" is not one at the gate either: its live seed (1004) passes at
   0.64, but its own decomposers straddle the line (61 % within 0.1). It classifies A.
4. **The clean live guilds are tiny worlds.** Five controls read B. Three of them (`sample:10`/1002,
   `sample:127`/1000, `sample:188`/1000) hold 13–18 agents at `T`. That is below
   `ROSTER_FLOOR` (20), so the roster gates never read them. The other two (`sample:127`/1002,
   /1004) hold 23 and 31. All five are sharply bimodal: producers at heterotrophy share ≈ 0,
   decomposers at 0.74–0.96, `clustering_strength` 1.0. No gated seed looks like that.
5. **Which direction the numbers support:** the **"mostly (A)"** follow-up. Only 2 / 12
   monoculture seeds (`sample:15`/1000, `sample:36`/1000) meet the B condition. Both are
   borderline: 9 % and 14 % of decomposers near the line, against the 20 % bound. Widen "near"
   from ±0.1 to ±0.2, which is the generalist gate's own band at `generalist_threshold = 0.3`,
   and the monoculture split becomes 10 A / 0 B / 2 ambiguous. The generalist-dominance B
   (`sample:45`/1001) has decomposers at heterotrophy share 0.60–0.67. That is inside the
   generalist band, so by that gate's own definition they are generalists. The choice between
   the two directions stays with the maintainer.

## What was measured

`guild_structure` rolls out each listed pair on the census's path: `search::decode` over
`default_ranges`, horizon 2000, `EvalConfig::default()`, early-stop carry off. It runs through
`explorers_genesis::rollout`, which is `run_single` refactored to also return the terminal world
and `RolloutObservations`, so the verdict is bit-for-bit the census's. Every read is an
existing evaluator or topology function:

| read | source |
|---|---|
| verdict, guild flags | the rollout's `RunResult` / `FitnessBreakdown` |
| monoculture margin | `clustering_strength(terminal roster) − clustering_threshold` |
| trophic position | `trophic_coordinates` (heterotrophy share) per living agent |
| terminal roles | `TopologyProjection::trophic_roles_of` on the rollout's own projection (`RolloutObservations::topology`), the same read the evaluator scores balance with |
| settled-window role counts | `RolloutObservations::role_snapshots` over `(T/2, T]` at the evaluator's 10-tick cadence, the samples the guild predicate reads (100 per seed) |
| detrital reliance | `TopologyProjection::detrital_reliance`, the quantity `trophic_roles_of` cuts at `DETRITAL_RELIANCE_THRESHOLD` |
| generalist share | `generalist_energy_share`, the quantity `is_generalist_dominant` compares against `generalist_dominance_fraction` |

Accessors added (no behaviour change; the existing functions now call them):
`TopologyProjection::detrital_reliance` and `pub DETRITAL_RELIANCE_THRESHOLD`;
`RolloutObservations::topology()`; `generalist_energy_share`; `explorers_genesis::rollout` /
`Rollout`; `pub Cliff::from_failure`.

## Classification rule

Read over the **terminal decomposers** (the topology's role read at `T`). An agent sits "near
the line" when its heterotrophy share is within 0.1 of 0.5. Decomposers are heterotrophs, so
this means share in (0.5, 0.6].

- **A (straddle):** at least half the decomposers sit near the line.
- **B (behavioural guild):** at most a fifth sit near the line, **and** the decomposers' median
  detrital reliance is ≥ 0.75.
- **ambiguous:** otherwise, or no terminal decomposers.

The rule was fixed before the run and applied identically to gated seeds and live controls. It
reads the decomposers rather than the whole roster because reading A is a claim about the
decomposers: that they tip across the line from the producers.

## Per-seed table

`cs − thr` is terminal `clustering_strength` minus `clustering_threshold` (0.5); negative means
monoculture. "het" is the heterotrophy share over the whole terminal roster; "near" is the
share of agents within 0.1 of the line. P / C / D are producer / consumer / decomposer counts,
median over the 100 settled-window samples, with the decomposer min–max. "dec" columns are over
the terminal decomposers; "rel" is their detrital reliance. "gen" is the generalist energy share
(the GD gate fires above 0.5 on rosters of at least 20). All 21 reproduce their census verdict
and guild.

| config / seed | verdict | cs − thr | roster | het p10/p50/p90 | near | P / C / D (window median; D min–max) | dec n | dec het mean | dec near | dec rel p10/p50/p90 | gen | reading |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| sample:15/1000 | monoculture | −0.408 | 64 | 0.00/0.13/0.79 | 0.08 | 56 / 1 / 8 (5–11) | 11 | 0.83 | 0.09 | 1.00/1.00/1.00 | 0.16 | B |
| sample:20/1000 | monoculture | −0.429 | 101 | 0.09/0.45/0.76 | 0.39 | 77 / 4 / 35 (33–41) | 36 | 0.68 | 0.36 | 1.00/1.00/1.00 | 0.45 | ambiguous |
| sample:20/1001 | monoculture | −0.364 | 135 | 0.18/0.36/0.49 | 0.36 | 126 / 0 / 13 (13–14) | 13 | 0.56 | 0.85 | 0.97/1.00/1.00 | 0.60 | A |
| sample:20/1003 | monoculture | −0.469 | 96 | 0.31/0.62/0.76 | 0.27 | 32 / 5 / 68 (61–74) | 61 | 0.67 | 0.23 | 1.00/1.00/1.00 | 0.61 | ambiguous |
| sample:36/1000 | monoculture | −0.404 | 64 | 0.43/0.68/0.87 | 0.20 | 14 / 4 / 44 (26–51) | 50 | 0.72 | 0.14 | 1.00/1.00/1.00 | 0.61 | B |
| sample:36/1002 | monoculture | −0.409 | 91 | 0.35/0.49/0.79 | 0.44 | 78 / 3 / 35 (15–43) | 41 | 0.71 | 0.24 | 0.99/1.00/1.00 | 0.64 | ambiguous |
| sample:36/1004 | monoculture | −0.333 | 82 | 0.27/0.42/0.65 | 0.41 | 60 / 2 / 24 (14–27) | 24 | 0.63 | 0.50 | 0.96/1.00/1.00 | 0.82 | A |
| sample:38/1000 | monoculture | −0.404 | 326 | 0.28/0.35/0.48 | 0.23 | 380 / 2 / 20 (13–24) | 23 | 0.57 | 0.83 | 0.96/1.00/1.00 | 0.82 | A |
| sample:100/1000 | monoculture | −0.128 | 473 | 0.45/0.54/0.68 | 0.76 | 334 / 20 / 242 (114–300) | 297 | 0.59 | 0.69 | 1.00/1.00/1.00 | 0.79 | A |
| sample:129/1003 | monoculture | −0.500 | 647 | 0.00/0.25/0.53 | 0.19 | 692 / 5 / 78 (70–81) | 79 | 0.58 | 0.71 | 0.85/1.00/1.00 | 0.36 | A |
| sample:129/1004 | monoculture | −0.500 | 565 | 0.00/0.27/0.46 | 0.15 | 696 / 2 / 33 (30–37) | 34 | 0.62 | 0.44 | 0.99/1.00/1.00 | 0.51 | ambiguous |
| sample:165/1000 | monoculture | −0.500 | 265 | 0.00/0.18/0.36 | 0.05 | 301 / 0 / 7 (6–9) | 7 | 0.60 | 0.43 | 0.98/1.00/1.00 | 0.21 | ambiguous |
| sample:45/1001 | generalist_dominance | +0.444 | 476 | 0.00/0.64/0.67 | 0.10 | 80 / 128 / 216 (92–301) | 301 | 0.64 | 0.10 | 1.00/1.00/1.00 | 0.59 | B |
| sample:96/1000 | generalist_dominance | +0.261 | 57 | 0.32/0.48/0.66 | 0.46 | 41 / 3 / 21 (19–24) | 20 | 0.63 | 0.40 | 0.88/1.00/1.00 | 0.85 | ambiguous |
| sample:100/1002 | generalist_dominance | +0.104 | 467 | 0.35/0.45/0.54 | 0.73 | 475 / 8 / 66 (29–105) | 105 | 0.53 | 0.98 | 1.00/1.00/1.00 | 0.98 | A |
| sample:10/1002 | live | +0.500 | 15 | 0.00/0.01/0.84 | 0.13 | 12 / 1 / 5 (5–5) | 5 | 0.74 | 0.20 | 1.00/1.00/1.00 | 0.33 | B |
| sample:20/1004 | live | +0.141 | 186 | 0.00/0.33/0.56 | 0.26 | 206 / 5 / 30 (27–34) | 31 | 0.63 | 0.61 | 0.67/1.00/1.00 | 0.38 | A |
| sample:127/1000 | live | +0.500 | 18 | 0.07/0.18/0.85 | 0.00 | 11 / 0 / 7 (7–9) | 7 | 0.85 | 0.00 | 1.00/1.00/1.00 | 0.00 | B |
| sample:127/1002 | live | +0.500 | 23 | 0.00/0.00/0.96 | 0.00 | 17 / 2 / 5 (5–7) | 7 | 0.96 | 0.00 | 1.00/1.00/1.00 | 0.00 | B |
| sample:127/1004 | live | +0.500 | 31 | 0.00/0.92/0.96 | 0.00 | 13 / 9 / 14 (13–17) | 14 | 0.94 | 0.00 | 1.00/1.00/1.00 | 0.00 | B |
| sample:188/1000 | live | +0.500 | 13 | 0.13/0.63/0.90 | 0.08 | 6 / 1 / 7 (5–9) | 5 | 0.84 | 0.00 | 1.00/1.00/1.00 | 0.21 | B |

The full per-seed records are in the JSON-lines output: 10-bin histograms of the heterotrophy
share for the roster and for the decomposers, quantiles, min / median / max of every role count,
consumer-guild flags and fitness. Two seeds also hold a consumer guild: `sample:45`/1001 and
`sample:100`/1000.

### Generalist dominance

The three GD seeds read generalist energy shares of 0.59, 0.85 and 0.98 against
`generalist_dominance_fraction` 0.5. None is marginal on the highest two. All three pass the
monoculture gate (margin +0.10 to +0.44), so there the trait distribution is not collapsed.
What is collapsed is the band around the line, into which the decomposers fall: their
heterotrophy share is 0.50–0.67 at p10–p90 on all three.

### Sensitivity to the band

The ±0.1 band is the rule's only free choice. The reliance clause never binds, since median
reliance is 1.00 everywhere. Widening the band to ±0.2 (heterotrophy share < 0.7, the
generalist gate's own band) moves the counts to:

| group | A | B | ambiguous |
|---|---|---|---|
| monoculture (12) | 10 | 0 | 2 (`sample:15`/1000, `sample:36`/1002) |
| generalist_dominance (3) | 3 | 0 | 0 |
| live controls (6) | 1 | 4 | 1 (`sample:10`/1002) |

These counts come from the decomposer histograms in the output file.

## What the numbers say about the follow-up

- **For "mostly (A)":** most gated guilds are decomposers drawn from the heterotroph side of
  a single trait continuum that runs up from the photo = hetero line. The monoculture gate fires
  far from its threshold on all of them. The stated rule gives no majority-B group, and the
  generalist band gives none at all. The controls that do read B are small, sharply bimodal
  worlds. None looks like a gated seed.
- **For "mostly (B)":** the diet split is fully real on every seed (reliance 1.00). At the
  stated band, 2 monoculture seeds (`sample:15`/1000, `sample:36`/1000) and one GD seed carry
  a decomposer guild whose members mostly sit away from the line. A per-cliff guild tally on
  the dead frontier would record these without touching the gate.

On balance the numbers lean to the first. A decomposer guild inside a gated world is mostly
the carcass-feeding tail of a mixotroph monoculture. It is not a separated behavioural guild
that the gate is wrongly removing. This note does not choose between the two directions.

## Reproduce

```
cargo build --release -p explorers-search --bin guild_structure
./target/release/guild_structure --configs sample:20                    # ~7.3 min
./target/release/guild_structure --configs sample:129                   # ~6.5 min
./target/release/guild_structure --configs sample:165,sample:96         # ~6.3 min
./target/release/guild_structure --configs sample:10,sample:15,sample:36,sample:38,sample:45,sample:100,sample:127,sample:188   # ~6.0 min
./target/release/guild_structure --summary
```

Output: `target/546/guild-structure.jsonl`, one row per config holding its listed seeds, in the
shared sweep shape. It is resumable, skips configs already present, and takes `--limit`.
`guild_structure` with no flags runs all 12 configs in one command. That took **1557 s (26 min)**
wall clock on the 8 GB machine, sequential across configs, with each config's seeds in parallel.
Per-config times: `sample:20` 437 s (4 seeds), `sample:129` 389 s, `sample:165` 341 s,
`sample:100` 173 s, `sample:38` 165 s, `sample:96` 36 s, all others under 10 s. The split above
keeps each command under 10 minutes.
