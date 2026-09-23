# Issue #519 — the decomposer guild at `T = 2000`: the community thinned, and the read is masked on exactly the worlds that still hold one

**Status: measurement. Runs the existing `role_emergence` instrument over a
selected cell set; changes no stepper, evaluator or search behaviour, and
proposes no threshold change.** #519 asked which of two things moved when the
decomposer-guild observable fell from 172/180 cells at the 500-tick cross-check
([439](439-permanence-crosscheck.md)) to 7/161 at the settled horizon
([509](509-viability-at-settled-horizon.md)): **the read** (#490's predicate too
strict for a settled community of ~3 producers) or **the community** (founder-bloom
guilds thinning to lone individuals). The answer is *the community* — and the
investigation turned up a second, separate fault in the read that the issue did
not propose and that matters more for #494. Stepper and search at `3180536`.

## TL;DR

1. **Among persisting seeds it is the community, decisively.** Over 189
   crosscheck-persisting seeds, the failing clause of the #490 predicate is
   **size** on 166 and **recruitment on none**. Median decomposer count over the
   settled window is **1**; only 12 % of persisting seeds ever reach the floor of
   5 on even one sample. A settled persisting community holds a decomposer
   *individual*, which is [#443](443-invasion-growth.md)/#489's finding holding
   at `T = 2000`.
2. **The read is not flaky.** `role_emergence` and `permanence_crosscheck`,
   independent instruments over the same configs, seeds and horizon, agree on
   **189 of 189** persisting seeds. Whatever else is wrong, the guild predicate
   reproduces.
3. **But the observable is zeroed by design on every gated seed, and that is
   where the guilds are.** All 14 cross-instrument disagreements are seeds the
   evaluator gated (12 monoculture, 1 lockup, 1 generalist dominance), where
   `FitnessBreakdown::gated` sets every descriptor to zero, `has_decomposer_guild`
   included. Read directly off the role snapshots, those same seeds carry a
   median of **20–42 decomposers** over the settled window (max 371) against the
   persisting seeds' **1**. The guild is not absent from the settled horizon; it
   is absent from the *live* half of it.
4. **No threshold rescues the persisting worlds.** Lowering `GUILD_MIN_SIZE`
   from 5 to 1 takes the sustained-guild rate from 8 % to 51 % of persisting
   seeds — but a "guild" of one agent sustained is the individual the observable
   exists to distinguish. The predicate's floor is not what broke.
5. **For #494**: at `T = 2000` a guild axis would be empty among live cells and
   populated only among cells routed to the dead frontier, which are never
   binned. On this evidence the guild cannot be an atlas binning axis at the
   settled horizon — not because the axis is uninteresting, but because the
   evaluator discards the reading on every world that would populate it.

## 1. What ran

| | |
|---|---|
| instrument | `crates/explorers-search/src/bin/role_emergence.rs`, unchanged, in subset mode (`ROLE_EMERGENCE_CONFIGS`); it records per-sample role series (`role_series`, every 10 ticks) and the #490 predicate per role, which is exactly what #519 asked to dump |
| cells | 35: **all 15** cells whose persisting seeds carry the guild in `permanence-crosscheck.jsonl`, plus **20 no-guild persisting-with-consumers cells** drawn stratified across the median-peak-population range (6 → 1184), so the no-guild sample is not just the cheap small worlds |
| seeds | 8 per cell (`1000..1007`), horizon 2000 — the same seed block, horizon and `decode` path as the #509 sweep, so a (cell, seed) row is directly comparable between the two instruments |
| output | `target/role-emergence.json`, 280 runs, SHA-1 `e05e13a61a66141f603e2c23a56f67e34a2c1078` (not committed — `target/` is build output) |
| cost | **3 h wall clock**, ~10 400 s CPU across ~8 threads, ~600 MB peak |
| reference | `target/permanence-crosscheck.jsonl` (#509), SHA-1 `e92eb3709796251ea6e3c0dd6d506b777b16b731` |
| tool | a throwaway Python script over the two JSON artifacts; not committed (the [474](474-atlas-regen-post-fix.md) precedent). §2's clause rule is the whole method |

**Clause attribution.** #490's predicate holds when the role count reaches
`GUILD_MIN_SIZE = 5` on *every* sample of `(T/2, T]` **and** at least one birth
in the window names a member. Per seed, reading the window counts off
`role_series`: `max < 5` → **size** (never reaches the floor); `max ≥ 5 > min` →
**sustained** (reaches it, dips below); `min ≥ 5` with the predicate still false
→ **recruitment** (sits at the floor throughout, no qualifying birth).

**On the cell counts.** #519 quotes 7 guild-positive of 161 persisting-with-
consumers cells; those are scoped to [509](509-viability-at-settled-horizon.md)'s
`I ≤ 1` subset. Over the whole file the figures are 15 of 234, and the
guild-positive cells with `I ≤ 1` number exactly 7. The two reconcile; this note
works over the whole file and says so where the subset matters.

## 2. Which clause fails

Crosscheck-persisting seeds only (mode `none` at `T = 2000`), n = 189:

| clause | decomposer guild | consumer guild |
|---|---|---|
| holds | 16 | 16 |
| **size** (never reaches 5) | **166** | 171 |
| sustained (reaches 5, dips) | 7 | 2 |
| **recruitment** (at floor, no birth) | **0** | **0** |

The clause #519 named as the likely culprit — recruitment, "no births in the
window" — **never fires**. The communities do not fail to breed; they fail to be
populations at all.

Window decomposer counts over the same 189 seeds: median minimum **1**, median
maximum **1**. The share reaching a given count on at least one sample: ≥ 1 62 %,
≥ 2 35 %, ≥ 3 21 %, ≥ 5 12 %, ≥ 10 3 %.

**Threshold sensitivity.** Share of persisting seeds sustaining ≥ k on *every*
window sample — i.e. what #490 would read at `GUILD_MIN_SIZE = k`:

| k | decomposer | consumer |
|---|---|---|
| 1 | 51 % | 31 % |
| 2 | 26 % | 16 % |
| 3 | 15 % | 5 % |
| 4 | 11 % | 1 % |
| 5 (committed) | 8 % | 1 % |

The read *is* threshold-sensitive — 8 % → 51 % between k = 5 and k = 1 — but the
floor that would restore the 500-tick picture is k = 1, which reads a single
sustained agent as a guild and erases the distinction the observable was built
for (#490: "a heterotroph *population* — sustained size and recruitment — as
distinct from a heterotroph *individual* carrying a role tag"). **This is a
designer's call, not a bug**: the numbers are above, the choice is not this
note's to make. What the numbers do settle is that no defensible floor turns the
persisting communities into guilds.

## 3. The read reproduces — and then is zeroed where it matters

Both instruments ran the same 35 cells × 8 seeds at the same horizon, so their
guild flags are comparable seed by seed. Over 271 comparable (non-timeout) seeds:

| crosscheck mode | `role_emergence` guild | crosscheck guild | n |
|---|---|---|---|
| persisting | false | false | 173 |
| persisting | **true** | **true** | **16** |
| gated | false | false | 68 |
| gated | **true** | **false** | **14** |

**Zero disagreement on persisting seeds.** The #490 predicate is reproducible
across two independently written rollout drivers; the 500 → 2000 collapse is not
an instrument defect in that sense.

**Every disagreement is a gated seed.** `FitnessBreakdown::gated`
(`crates/explorers-genesis-eval/src/lib.rs`) returns zero fitness *and every
descriptor zeroed* — `has_decomposer_guild: false` among them — on the stated
reasoning that "a degenerate world has no meaningful behaviour coordinate (it is
routed to the dead frontier by cliff, not binned)". That reasoning holds for a
*behaviour coordinate*. It does not hold for a **reported observable** whose
whole purpose is to say what the community contained, and the consequence is
severe:

| | persisting seeds (n = 189) | gated seeds read as guilds (n = 14) |
|---|---|---|
| median final population | 8 | 86 |
| median final decomposers | 1 | 40 |
| median window decomposer count (min / max) | 1 / 1 | 20 / 42 |
| largest window decomposer count | 39 | **371** |

The 14 are 12 monoculture, 1 nutrient-lockup, 1 generalist-dominance, across six
cells (`sample:100`, `sample:36`, `sample:96`, `sample:127`, `atlas:12`,
`atlas:56`). `sample:100` ends with 83–371 decomposers on 6 of its
8 seeds and is recorded in `permanence-crosscheck.jsonl` as a cell with **no
guild**, because every one of those seeds was gated as monoculture.

So the honest statement of the 172/180 → 7/161 collapse has two terms, and #519
proposed only one of them:

- the settled *live* community really did thin to individuals (§2); **and**
- the settled *gated* communities that still hold guilds are recorded as holding
  none, because the verdict zeroes the observable before it is written down.

A plausible reading of why the 500-tick cross-check saw 172/180 — the
post-bloom successional stage `genesis-search.md` describes (producers fall
2–3×, decomposers rise 2–18× after the peak) is still running at 500 ticks, in
worlds large enough to carry a guild and not yet gated — is **not tested here**
(§5).

## 4. What this settles for #519 and #494

- **#519's question is answered: the community.** Among live settled worlds the
  guild signature is gone because the population is gone, not because the
  predicate is strict; the recruitment clause never fires and the size clause
  explains 166 of 189.
- **A second defect is now on the table, and it is the one that blocks #494.**
  The guild observable is unreadable on gated worlds by construction. Deciding
  whether the guild becomes a binning axis or folds into `coexistence_fraction`
  on the current data would be deciding it on a read that is blind to every
  world that still has a guild. Filing this separately is the next step (the
  fix is small — carry the guild read through `gated`, or read guilds outside
  the verdict path — but it is an evaluator change, so it needs its own issue,
  a red test and the designer's agreement that a reported observable should
  survive a failure verdict).
- **For the atlas**: on this evidence a guild axis at `T = 2000` would be empty
  where the atlas bins and populated where it does not. That is an argument
  against the axis *as things stand*, and it would need re-making after the
  gating fix.

## 5. What this does not settle

- **Why 500 ticks read 172/180.** The successional explanation above is
  consistent with everything here but untested: it needs the same cells read at
  `T = 500` *and* their gate status at 500, which this run does not record.
  Until then the size of the drop that is community-thinning versus the size
  that is gating-mask is unquantified — this note shows both are real, not their
  proportions.
- **Whether the 14 gated guilds are ecologically interesting.** A monoculture
  verdict with 40 decomposers on the roster may be a genuine brown-web community
  the roster gate mislabels, or a producer collapse with a decomposer bloom
  feeding on the corpses. Nothing here distinguishes them; `sample:100`'s 371
  decomposers against 311 producers would be the place to look.
- **The threshold.** §2 gives the sensitivity; it does not propose a value, and
  `GUILD_MIN_SIZE = 5` is untouched.
- **Cell coverage.** 35 of 234 persisting-with-consumers cells, chosen to cover
  all 15 guild-positive ones plus a stratified 20; the per-clause proportions are
  over-weighted toward guild-positive cells by construction and should not be
  read as box-wide rates.
- **The consumer guild** is reported alongside throughout but was not #519's
  question; its numbers are weaker in the same direction.
