# Genesis Search

The empirical lens on the world rules — the broad, expensive sweep of world-parameter space that
[viability](viability.md) calls *lens 3* of the validation triad. [Viability](viability.md) decides in
closed form which parameterizations *cannot* work; this is where the parameterizations that *might* work
are actually found, by running them.

This document fixes the **shape of what genesis search produces**, and why it has that shape. It does
not prescribe an optimiser — the search method is implementation and is free to change. What a
replacement implementation must preserve is the contract stated here.

## Genesis illuminates; it does not optimise

The genesis search returns an **[atlas](../../CONTEXT.md)** — a map from world-parameter space onto the
failure-mode coordinates — **not a single tuned [recipe](../../CONTEXT.md)**. A playable recipe is still
drawn *from* the atlas, but it is a projection of the map, never the map's purpose.

This is the load-bearing choice, and it follows from three properties of the objective, not from a
preference for one optimiser over another:

- **The space is high-dimensional.** The search varies ~30 world parameters and initial-distribution
  fields at once. A scalar surrogate optimiser (Gaussian-process Bayesian optimisation) is reliable only
  in low dimensions and degrades badly here — so badly that any such method needs a manual prefilter to
  fix the dimensions that "don't matter" before it can run at all. That prefilter is a global, one-shot
  guess that discards any dimension whose importance is *conditional* on another.
- **The objective is gated.** Most of parameter space is a flat zero-fitness desert: the
  [degenerate configurations](../../CONTEXT.md) collapse fitness to zero around a thin viable manifold.
  An optimiser climbing a scalar gets **no gradient across the desert** — which direction leads to life
  is invisible from fitness alone. This is the textbook deceptive-landscape failure.
- **What the designer wants is plural.** The goal is not *one* best world but a map of *which regions
  avoid which failure modes* — so the designer can see the structure of viability, and so a downstream
  surrogate can be trained on the whole manifold, not just its peak. A single maximised number cannot
  answer a plural question.

A **quality-diversity** search dissolves all three at once. It selects on *novelty in the failure-mode
coordinates*, not on fitness rank — so a config that scores zero but lands in an unexplored region is
*kept and bred from*, giving the search stepping-stones across the desert an optimiser cannot find. Its
native output *is* the atlas. And a covariance-adapting emitter learns the relevant subspace as it
moves, retiring the manual dimension-fixing prefilter the surrogate optimiser needed. The specific
emitter and archive are implementation; the illumination contract is the design.

## The search box: the full box by default, a narrowed box held in reserve

A config is a point of the unit cube, and `decode` maps it onto 34 raw fields — 25 world parameters
and 9 founder-distribution fields — each over a range, linearly unless the coordinate's scale says otherwise. The set of ranges is the **search
box**. The full box, `default_ranges()`, spans every field's plausible range, and the search runs over
it by default.

The 33rd coordinate is the **uptake structure exponent** `b`, linear over `[0, 1]` (#653). The range
must hold `b = 0` exactly, the size-blind uptake every earlier world ran, so a log scale is out; above
1 a large body would take up more per unit structure than a small one. Its partner, the uptake
reference structure, stays out of the box at 100. The reasons are in world-rules.md flow 2
(*Size-scaled uptake*). The 32-coordinate box before it is the **size-blind box**
(`size_blind_ranges()`): `decode` reads a box without `b`'s coordinate as `b = 0`, so a unit vector
drawn under it still names the world it always named.

The **cross-trait cost** `c_AH` is not in the box. Its default 0 holds in every world genesis searches, so the box is the 33-coordinate **untaxed box** (`untaxed_ranges()`), and the leaching rate is held outside it (below). The term is latent (world-rules.md trade-off #5): it was specified as the lever for a signature that is now reported rather than required, genesis did not select on it when it was searched, and a dimension the search does not select on only adds noise. If it comes out of reserve, its range is linear over `[0, 0.14]`. That range holds `c_AH = 0` exactly, which rules out a log scale. Its top is measured, not copied from the per-trait costs: it is the `c_AH` at which a typical light-fed mixotroph that drains pays about twice its median drain income ([`668-cross-trait-calibration.md`](../research/668-cross-trait-calibration.md)). `decode` reads a box without `c_AH`'s coordinate as `c_AH = 0`, so an atlas searched under the untaxed box names the same worlds whether or not the coordinate exists. The box with `c_AH` as its 34th coordinate is the **taxed box** (`taxed_ranges()`), the one it is searched over if it comes out of reserve. *Current state (#716, #687, 2026-10-07): the search defaults to the untaxed box, with `λ` held fixed outside it (below). The committed atlas (#687) is #719's seed-42 search under that box, so each of its 78 cells and the committed recipe (atlas:31, cell [5, 19, 1]) run at `c_AH = 0` and `λ = 0.0025`. #677's atlas, committed until #687, was searched under the taxed box and records it, so its cells keep their own `c_AH`, with `λ = 0`. #686 and #711 searched without `c_AH`.*

The **leaching rate** `λ` is not in the box. Genesis's worlds run at a fixed `λ = 0.0025`; world-rules.md (*Carcass energy decays only through agents; carcass nutrient leaches*, *The rate*) gives the value and why it is fixed rather than searched. The stepper's own default stays `λ = 0`, so a world that names no rate, and every atlas searched before leaching, keeps running as it was named. An atlas therefore records the fixed rate its worlds ran at, beside its box, as `"fixed": {"leaching_rate": 0.0025}`, and a reader decodes each atlas at the rate it records, at the coordinate's value when its box has a `λ` coordinate, or at 0 when it records neither. The record is written only when the search held something fixed, so an atlas from before it is written and fingerprinted exactly as before; the fingerprint eats the record when there is one, since the same cells at another rate are other worlds. A search checkpoint stamps the fixed rate with its box, so a checkpoint from a search that held another rate, or none, is refused rather than resumed into a search under this one. #686's and #711's atlases were searched with `λ` as a box coordinate on a square scale, `λ = 0.01 · u²`, the **leached box** (`leached_ranges()`): the untaxed box with `λ` as its 34th coordinate. A coordinate's scale is part of the box, and the atlas records it with each range as `"scale": "square"`, writing nothing for a linear range, so a linear box is written exactly as before scales existed and its fingerprint is unchanged. A range recorded without a scale is linear. *Current state (#716, 2026-10-07): implemented. The default box is the untaxed box, every world a search under it decodes runs at `GENESIS_LEACHING_RATE` = 0.0025, and its atlas records it. #677's atlas records neither a `λ` coordinate nor a rate and decodes at `λ = 0`; #686's and #711's decode at their own coordinate's `λ`. All three decode bit for bit to the worlds they named before (a digest of the decoded worlds pins atlases cut to the taxed, untaxed and leached boxes). Recipes written from new atlases carry `leaching_rate: 0.0025` explicitly. Since #687 the committed atlas is one of them: it records `"fixed": {"leaching_rate": 0.0025}`, and the committed recipe carries the rate.*

**`decode` reads a coordinate by its name, not its position.** Each range carries the name of the field it decodes to, and `decode` looks every field up by that name. A field the box has no coordinate for keeps the known-viable baseline's value, which for the fields later boxes added (`b`, `c_AH`, `λ`) is the latent 0 every earlier world ran. A box cannot be identified by its length: the taxed box and the leached box both have 34 coordinates, and the 34th is `c_AH` in one and `λ` in the other. Read by position, every cell of #677's atlas would have decoded with its `c_AH` coordinate as a leaching rate and no cross-trait cost. Read by name, atlases under every box an atlas has recorded (the taxed, untaxed, leached and size-blind boxes) decode to the same worlds bit for bit, which a digest of their decoded worlds pins. A search checkpoint stamps its box the same way, scales included, so a checkpoint from the taxed box is refused by a search under the leached box (the 34th coordinate's name differs) rather than resumed into it.

A **narrowed box** (`narrowed_ranges()`) keeps the same raw coordinates and narrows
only the box. It is opt-in: it keeps outcome prediction but costs the illumination (below).

- **The core stays at full width.** `light_competition_radius`, `world_extent`,
  `solar_flux_magnitude`, `initial_population_size`, `contact_range_coefficient`, `mean_kappa`,
  `mean_mobility` and `reproduction_energy_threshold` hold the signal. On a held-out LHS draw an
  eight-dim raw set matches the full 32 on every target, and greedy selection on either draw picks the
  same eight on its own. Raw32 loses 0.13 R² on the live fraction between draws, and the core does not.
  `trait_covariance` and `mean_heterotrophy` also stay wide, for the bloom-onset margin.
- **The other 23 fields are narrowed, not frozen.** Each one's range is cut to a band of
  `NARROWED_BAND_FRACTION` (0.25) of its full span. The band is centred on the value `decode` inherits
  for that field from the known-viable baseline. The five founder-distribution fields inherit no value,
  so their band is centred on the midpoint of the full range. A band that would cross a full-range
  bound is slid back inside it, keeping its width. The uptake structure exponent's centre is the
  baseline's 0, so its band is the bottom quarter of `[0, 1]`. Likewise a narrowing of the taxed box
  would give `c_AH` the bottom quarter of `[0, 0.14]`, and one of the leached box would give `λ` the
  bottom quarter of `[0, 0.01]`, `[0, 0.0025]`, still on the square scale: the band is cut in `λ`, as
  every band is cut in its field, so it would span the bottom half of the coordinate `u`. The narrowed
  box holds `λ` fixed outside it, as the full box does. An effect that cannot be detected at n ≈ 200 is not
  proven to be zero, so the band keeps some width and a weak effect stays findable. Carrying these
  fields at full width would spend a covariance-adapting emitter's evaluations on directions with no
  detectable effect. The width is a judgement call that the data does not settle, so it is set in one
  constant.
- **The coordinates stay raw.** A reduced `decode` over composite groups is rejected. The composites
  do not beat their own ingredients held out, and a search over them would need an inverse map back to
  raw fields.

The evidence for the core is [`462-held-out-check.md`](../research/462-held-out-check.md).

**Why the narrowed box is not the default.** The held-out check selected the core on outcome
prediction (live, lockup, bloom, onset), not on the behaviour axes the archive is binned on. Run as the
default (#559), the narrowed box illuminated less than the full box on both search seeds tried (batch
32, 10 generations, ensemble 5, `T = 2000`):

| box, seed | cells | QD-score | best / median fitness | clustering bins | dead: nutrient-lockup |
|---|---|---|---|---|---|
| full, 42 | 95 | 36.4 | 0.572 / 0.424 | 5 | 52 |
| full, 43 | 93 | 36.6 | 0.606 / 0.416 | 7 | 48 |
| narrowed, 42 | 78 | 28.3 | 0.497 / 0.390 | 2 | 88 |
| narrowed, 43 | 84 | 33.8 | 0.713 / 0.434 | 4 | 85 |

The full box is stable across seeds. The narrowed box loses 10–18 % of coverage, fills fewer bins of
the clustering axis, and records about 1.7× as many nutrient-lockup dead configs on both seeds. The
cause is the narrowing itself, not where the bands sit. On the two LHS draws, bands centred on the
range midpoints predict the same lockup as bands centred on the baseline. Narrowing the 22 dims around
either centre raises predicted lockup and lowers the live fraction, and the cost is spread thinly
across all 22. It starts at a band width of 0.75, and no dim carries more than about 0.02 of it. No
narrowed dim is a strong behaviour driver either: clustering and carcass follow the core. So no
re-centred, wider or selective narrowing is worth promoting
([`561-narrowing-behaviour-axes.md`](../research/561-narrowing-behaviour-axes.md)). The one gain, a
higher best elite on seed 43, is the concentration a smaller box buys.

**A unit vector names a world only together with its box.** The same `unit` decodes to different
worlds under the full and the narrowed box. So the atlas records the box it was searched under
(`search_box`), and every reader of atlas cells decodes them over that box: the recipe projection, the
re-projection, and the research instruments (`atlas:i`). A reader that brings a box of its own is
refused with an error when that box differs from the atlas's. It never silently decodes the wrong
worlds. An atlas written before the box was recorded reads as the size-blind box, which is the box it
was searched under. An atlas that recorded the untaxed box, such as #663's (committed until #677),
decodes over it with `c_AH = 0`, and one that recorded the taxed box, such as #677's (committed
until #687), decodes with its own `c_AH` and `λ = 0`. The instruments' LHS draws (`sample:i`, `sample@S:i`) sample a
whole box, not the search's box, and that box is the size-blind one (`config_source::sample_box()`): recorded rows and
research notes name worlds by `sample:i`, and a draw over the 33-coordinate box would have given every
one of them a nonzero `b`, and would have redrawn every other coordinate too. So every sample world
keeps `b = 0`, `c_AH = 0` and `λ = 0`, and the instruments see them only through atlas cells or an
explicit pin. The draws hold nothing fixed (`config_source::sample_fixed()`), so a sample world does
not run at genesis's fixed leaching rate either: `sample:i` names a fixed world, and it keeps naming
the world recorded rows ran.

## The three behaviour axes are the failure-mode coordinates

The atlas bins each surviving world on three **[behaviour axes](../../CONTEXT.md)**, each in `[0, 1]`.
They are not chosen by eye — each is the coordinate a [dynamics failure mode](viability.md) lives on, so
the atlas's cells correspond to distinct ecological regimes:

| Axis | What it measures | The mode it indexes |
|---|---|---|
| **oscillation strength** | endogenous population rhythm between trophic levels | **frozen dynamics** (low) ↔ healthy oscillation (mid) |
| **clustering strength** | multimodality of the trait-distance distribution (the dip statistic) | **monoculture** (low) ↔ **coexistence** (high) |
| **carcass-locked fraction** | the dead pool's share of conserved nutrient (the trailing-window mean the [lockup gate](viability.md) reads) | healthy throughput (low) ↔ **nutrient lockup** (high) |

All three are observables the **evaluator already computes** while scoring a run — they are read off the
evaluator's output, never re-derived by the search. The third axis is deliberately the carcass-locked
fraction and **not** the producer-vs-heterotroph energy share: that share cannot see the
dead-vs-living distinction, so it would put a healthy world and an about-to-lock-up world in the *same*
cell (and it is a reported observable, not a fitness term — [expected properties](expected-properties.md),
*Trophic structure*). The carcass-locked fraction is the cheap
observable of the [flux-balance order parameter `C*`](viability.md) the lockup cliff actually lives on.

## The dead frontier is the atlas's most valuable layer

A world that hits a terminal gate has no meaningful behaviour coordinate (a world extinct by tick three
has no oscillation), so it gets **no cell**. Instead it is tallied to the **[dead frontier](../../CONTEXT.md)**,
keyed by which failure mode it hit. The frontier is *where parameter space stops being survivable, and
which cliff it hits when it does* — the structure the designer most wants to see, and the negative-space
sketch the [viability](viability.md) gates aim to predict in closed form.

The frontier holds **two kinds of entry**, and the distinction is load-bearing:

- **A priori deaths** — configs the [viability](viability.md) prefilter gates *before any rollout*. The
  two committed closed-form gates (extinction's flux floor `F ≤ B`; energy death's nutrient floor on
  `N_total`) run first and route provably-dead configs straight to the frontier, spending no ensemble on
  them. This is the lens-1 → lens-3 interlock viability already names as its payoff: it shrinks the
  search box so budget concentrates on the survivable region.
- **Observed deaths** — configs a rollout actually carried into a cliff.

Because the prefilter and the rollout both verdict the *same* config, they **must agree**: a config
viability calls extinct must, when run, be extinct. The search keeps a sampling cross-check that
prefiltered-dead configs would also die if simulated — so every genesis sweep is a continuous
falsification test of the two gates, exactly the cross-validation the [validation triad](viability.md)
turns on. Disagreement is diagnostic, not a nuisance: it localises a mis-drawn gate.

The **nutrient-lockup cliff cannot be prefiltered** — viability shows it has no cheap a priori gate (it
is turnover- and decomposer-mass-dependent, emergent per seed). So the lockup layer of the frontier is
populated only by *running* worlds that survive yet strand their nutrient. Reaching that thin
high-carcass region is the emitter's job (directed exploration along the carcass axis), not the
prefilter's. The atlas maps the lockup cliff by running, because the physics forbids gating it.

**An unfinished rollout is no verdict (#562).** Every seed rollout in the search and in refinement runs
under the research sweeps' two wall-clock budgets (`--run-timeout-secs` on the step loop,
`--eval-timeout-secs` on the terminal evaluation; 600 s each by default). A seed that exhausts either is
*unfinished*. It is not a death, so it never reaches the frontier. It is not live, so it never reaches a
cell. It is counted (`rollouts_unfinished` on the atlas, per cell in refinement). A config is read off its
finished seeds, and a config none of whose seeds finished is placed nowhere. Dense worlds are the
tall-bloom region the search must still visit, which is why the budget excludes them rather than
failing them, and why the defaults sit far above a normal rollout's cost. The search is deterministic in
`(config, seed)` only while no budget fires: wall clock decides which seeds are unfinished, so an atlas
with a non-zero `rollouts_unfinished` is not guaranteed to reproduce. One no budget touched is
byte-identical to an unbudgeted search.

## The atlas maps the settled community, not the founder bloom

Every rollout the search scores starts from a founder cohort and passes through a **pioneer bloom** —
the producers colonise the empty world, peak, and then fall back as their own mortality feeds the
carcass flux that the heterotroph niche lives on ([disturbance and succession](../ecology/disturbance-and-succession.md)).
The heterotroph guilds are a **later successional stage**: on the strongest known guild configs the
producers fall 2–3× and the decomposers rise 2–18× *after* the bloom peaks, and a guild that reads on
7 / 8 seeds over the settled phase reads on 0 / 8 at peak bloom
([443 §6.2](../research/443-invasion-growth.md)).

The atlas characterises the **settled community** — the post-bloom stage the player is dropped into —
not the bloom. The reason is what the axes are for: they are existence/stability coordinates, and the
existence or stability of a colonisation transient is not the thing the design maps; the failure modes
the frontier tallies (frozen, monoculture, lockup) are properties of where the ecology *settles*, and
a bloom-stage read cannot tell "the search selects against guilds" from "the guild forms after the
search stops looking". A bloom-stage map is therefore a map of the wrong object, however cheap. The
rollout horizon is consequently a **design quantity set by the ecology's own settling time**, not a
search tuning knob.

**One rollout, one settled window.** Each rollout runs to the horizon `T`; the terminal gates
(extinction, energy death, lockup) fire whenever they fire along the way, so the dead frontier is the
frontier of the *whole* trajectory — a world that blooms and then locks up is on the lockup cliff,
which a bloom-length rollout could not see. Every behaviour axis and every reported distribution (the
guild fractions, the coexistence fraction) is read off the **settled window `(T/2, T]`** — the same
window the guild predicate is defined on — not off the bloom or the whole run. The window changes
*when* each axis is read, not *what* it reads: `clustering_strength` stays the existence read on the
final snapshot (now a settled tick; the existence-vs-persistence pair with `coexistence_duration` in
[expected properties](expected-properties.md) is deliberate and a window-typical dip would blur it);
`oscillation_strength` reads the window's share series (a thousand settled ticks instead of a few
hundred of bloom — the demographic-pulsing contamination #358 found at the short horizon is a
bloom-stage artefact); the carcass-locked fraction stays the trailing mean the lockup gate reads, so
axis and gate cannot diverge — if the dead pool has not settled by `T − window` the horizon is too
short, not the read. The two halves are not separable: a bloom-length rollout with a "longer guild-only read" bolted on still has to run
the world to `T` to see the guild, saves only the per-tick observation (which is not where the cost
is), and leaves the axes reading the bloom while the guild reads the settled stage — a map whose
coordinates and whose reported distributions describe different objects.

**The frontier costs a bloom, the atlas costs the horizon.** A rollout that hits a terminal gate is
tallied and stopped where it dies; only a world still alive past the bloom is carried to `T`. Nothing
is *scored* before `T` — an early stop is a frontier entry, never a cell — so the map is the same map
as if every rollout ran the full horizon, with one stated exception, the bloom stop below; the search
simply does not pay a settled-community price for a world that has no settled community. This is the tick-0 prefilter's interlock moved along the
trajectory, and it is why the longer horizon does not force the map to get coarser: cutting
generations or batch to hold wall-clock trades the atlas's resolution for its correctness, and
shrinking the seed ensemble makes the selection signal over seeds noisier exactly where
[the projection](#the-recipe-is-a-projection-of-the-atlas) already struggles. Neither is taken.

The two dead-pool gates are read *incrementally* for this: on the series-so-far, exactly the
horizon definitions with the reference starting at the grace, at the lockup-window cadence, so a
world that collapses at tick `t` and stays collapsed stops at the first window boundary past
`t + window` (`early_stop` in the evaluator; extinction and explosion read the count every tick).
But the gates as defined are **not proven irreversible** — a world flagged at tick 600 might
recover by `T` — so the stop carries the same falsification interlock the prefilter has: a
configurable fraction of stopped rollouts (`early_stop_crosscheck_fraction`, default 5 %, drawn
deterministically from the rollout seed) is carried to `T` anyway and re-verdicted on the full
series, and every stopped-dead / alive-at-`T` disagreement is surfaced beside the prefilter and
bifurcation disagreements (`early_stop_disagreements`), never swallowed. The carry changes only
what is *recorded*: a carried rollout's own verdict stays the gate's, so the atlas is the same map
at any carry fraction — while no budget fires. A carried rollout runs to `T`, so it can exhaust a
rollout budget a stopped one would not, and an unfinished seed drops out of its config's verdict
([573 §5](../research/573-bloom-stop-trial.md): carrying every stop moved one or two frontier
configs and no cells). A non-empty list localises a gate firing on a reversible collapse — the
evidence for tightening the gate's definition, not for lengthening the grace. Carried in full on
two searches, the gates' own list is not empty: of 1261 carried stops, 13 gate stops read alive at `T`.

**One predictive stop: the bloom stop (#573).** The gates stop a world that *has* collapsed. The
**bloom stop** stops one that very likely *will*. At tick 300, a rollout whose running peak
(founders included) has reached 10 × its founders is stopped and tallied on its own cliff,
`bloom_stop`, never as an observed death (`--bloom-stop T:F`, default `300:10`; `--no-bloom-stop`
turns it off). It is read after the gates, so a gate firing on the same tick keeps its verdict,
and the carry samples it like any gate. It exists because the tall blooms are where the search's
wall clock goes. On an LHS draw, a bloom past 5× founders by tick 300 ended in lockup or another
failure 97 % of the time ([554 §4, §7](../research/554-peak-timing-early-stop.md)). In the
search it halves the time to an identical 10-generation atlas, so a search reaches about twice
as far in the same time ([573](../research/573-bloom-stop-trial.md)). The factor is 10, not 5,
because at 5 it stopped every seed of a config that was live on 8 of 8, one that blooms 5–8×
and settles. **The cost is a stated blind spot:** the atlas does not hold worlds that bloom past
10× by tick 300 and then persist. The one such world found carried thousands of agents at `T`
(1200–4700), the dense region the rollout budgets already cannot finish. The atlas records the
rule it ran under (`provenance.bloom_stop`), and the refinement never applies it, since it
re-reads cells that are already live.

**The gates' reference excludes the founder transient, by a measured tick count.** Lockup is read as
a trailing window against the trajectory's earlier history (the low the dead pool once reached);
energy death is read against a history-free reference, the config's sustainable stock ([expected
properties](expected-properties.md#energy-death)), but its trailing window is likewise taken from the
grace on. Tick 0 is not part of either read: the founder
cohort is *provisioned*, so its stock is an artefact of the founder budget rather than anything the
ecology produced, and for the first few tens of ticks — until photosynthetic income overtakes the
provisioning — the series describes the budget, not the world. The reference therefore starts after a
**grace** of fixed tick count, sized from the *provisioning transient* — the tick at which the living
stock first exceeds its tick-0 value, measured across the atlas's live cells and a low-discrepancy
sample of the search box, taken at an upper quantile — and recorded here beside the measurement that
produced it. It is an ecological constant, not a fraction of the horizon: a longer rollout does not
make the founder budget last longer, and a grace scaled to the horizon would blind the gates to a
world that genuinely dies early. It is also not per-run: a world that never recovers its founder stock
would then never acquire a reference and would pass as live. When the stepper changes materially the
transient is re-measured, the same way [viability](viability.md)'s tightness claims are re-checked
against the atlas and the search box.

**The horizon is the settling time, measured the same way.** `T` is set so that the settled window
`(T/2, T]` opens *after* succession has run: the measurement is, per run, the tick at which the
producer count last leaves the band it holds over the run's final stretch (succession is the producer
ceding stock to the heterotroph niche, so the producer series is where it reads), taken at an upper
quantile across the atlas's live cells and a low-discrepancy sample of the search box, with `T/2`
placed above it. The evidence that it is a succession timescale and not a threshold is
[443 §6.2–6.3](../research/443-invasion-growth.md): at 1000 ticks decomposers are still rising
2–18× on the strongest guild configs, at 2000 the control arm only drifts, and the guild's own
doubling time where it lives is ~700 ticks. A round number chosen for convenience would carry no
procedure to re-run when the stepper changes; this one is re-measured with the grace, off the same
long rollouts, since both are transient ticks of the same trajectory.

*Current values — measured by `crates/explorers-search/src/bin/settling_time.rs` (one resumable
JSON-lines row per config; the summary restates the procedure beside the quantiles) over the 82 atlas
live cells and the 200-point LHS sample of the search box, 8 seeds each, at a 3000-tick horizon: 2256
runs, 1480 live (mode `none`, reached the horizon), the rest tallied by mode
([505](../research/505-settling-time.md)). Both quantiles are nearest-rank over the live runs, atlas
and sample pooled.* **Grace = 260 ticks** (`EvalConfig::grace_ticks`): the 90th percentile of the
provisioning-transient tick over the 1357 live runs whose stock re-exceeds the founder budget, rounded
up to 10 — the distribution is 50 / 90 / 95 / max = **2 / 253 / 514 / 2586** pooled (atlas, n = 401:
1 / 198 / 419 / 1885; sample, n = 956: 3 / 296 / 551 / 2586); 123 live runs (8 %) never re-exceed
tick 0, the case that rules out a per-run grace. **Horizon: `T = 2000` stands as the working value;
the settling read did not measure it.** The producer-settling tick (last tick outside ±20 % of the
final-500-tick mean) reads 50 / 90 / 95 / max = **1677 / 2981 / 3000 / 3000** pooled over the 1407
live runs with producers in the tail (atlas, n = 396: 2021 / 2989 / 3000 / 3000; sample, n = 1011:
1527 / 2973 / 3000 / 3000), so the rule *smallest multiple of 500 with `T/2` above the 90th
percentile* returns 6000 — but the statistic is saturated, not the ecology unsettled: 29 % of live
runs leave the band *inside* the 500-tick tail the band is defined from (the median settled producer
count is 3, so ±20 % is narrower than one agent and every birth or death breaks it), the upper
quantiles sit at the horizon on every sub-population (tail count ≥ 20: 90th percentile 3000), and
the reading would move with any horizon it was run at. `SearchConfig::max_ticks` is therefore not
moved on it; the band statistic is re-designed (a band in absolute agents, or a smoothed series)
before `T` is read again, and #492 / 443 §6.3 remain the evidence the working value rests on.*

## Genesis selects for worlds that are sensible across initial conditions

A world's quality is a property of its ensemble, not of a draw. CONTEXT.md's *sensible world* is
accepted "only when multiple patterns are reproduced simultaneously across an ensemble of runs", so a
world that lives and coexists on some seeds and dies on others is not sensible however well its good
seeds score. Genesis's target is therefore **robustness across initial conditions**: how reliably the
world's parameters produce a sensible world, as well as how good that world is when they do.

This is the robustness the search can afford, and it stands for the others. Across #677's atlas,
the one #693 audited, a world whose seeds disagree on its verdict is the same world that flips when its parameters
are jittered (Spearman ρ +0.80 between seed-only and jitter flip rates over 99 worlds). Worlds whose
seeds all agree barely move under jitter: 3–10 % of verdicts flip, against 21–34 % for the rest. So
robustness to parameters adds little beyond robustness to initial conditions, at about 25× the
cost ([693-fragility-audit.md](../research/693-fragility-audit.md)). Robustness to perturbation
mid-run is the one the game turns on, since the player's actions are perturbations. It is read on the
atlas's worlds as a readout, not searched. A perturbation can only be applied to a world that
settles first, so a world must first be robust across initial conditions, and pulse response is
measured on top of that.

Selecting on robustness is not new to genesis. It is where the projection already looks: the recipe
pick reads a coexistence-fraction floor at a refined ensemble because a straddler can top the
leaderboard on a lucky draw (*The recipe is a projection of the atlas*). The fragility audit shows
the straddler is not an edge case of the pick. 42 of #677's atlas's 99 worlds have seeds that
disagree, the seed-to-seed fitness spread (median sd 0.149) is close to the median cell fitness
(0.191), and flip rate rises with fitness (ρ +0.27). A search that ranks on a 5-seed median keeps
fragile worlds that drew well.

**Robustness is required of the world the player is given, and read of the map.** The player gets one
world, the recipe, and replays it on new seeds, so that world must coexist on nearly every initial
condition (*The recipe is a projection of the atlas*). The atlas only has to hold enough robust worlds
for the pick to draw from, and it does: #677's atlas, both robustness-scored ones (#686,
#711) and the committed one (#719's) each hold 32–57 worlds that live on all 10 fresh audit seeds. Robustness across the whole map
is not a target, because the map exists to show the cliffs too. A requirement that most of its worlds
agree across seeds asks every region to be robust, which no region near a cliff can be. It is also
nearly out of reach at 10 seeds whatever the score: a world that lives with probability p agrees on
all 10 with probability p¹⁰ + (1 − p)¹⁰, which is 0.60 at p = 0.95. The share of the atlas whose
seeds all agree, its mean live fraction and its flip rates under jitter are therefore **readouts**
that describe how robust the map is, not bars a search must clear. The score still steers the search
toward robust worlds (below), because a pick needs robust candidates near the top of the fitness
order, where it looks first.

**A cell's fitness is its live fraction squared times its live seeds' mean fitness**: `L² · F`,
where `L` is the share of its seed ensemble whose runs pass every gate (extinction, energy death,
lockup, monoculture, generalist dominance, bloom stop) and `F` is the mean fitness of those live
seeds. A cell with no live seed scores 0. The score has two factors because a world's quality has
two parts: how reliably its parameters produce a sensible world, and how good that world is when
they do. The power on `L` sets how much reliability counts against quality. At power 1 the score is
the plain mean over seeds with gated seeds at 0, the expected fitness, which trades the two parts
one for one. A world live on 9 of 10 seeds then needs only 11 % more `F` to beat one live on all 10.

**The power is 2 because the plain mean lets quality buy fragility.** Across a searched atlas, `L`
and `F` are uncorrelated, but `F` varies far more: 5.5× the variance of `L` on a log scale, with `F`
spanning 0.09–0.44 between deciles while `L` sits in 0.7–1.0. So the mean ranks cells almost wholly
by `F`, and a niche keeps whichever world drew the highest `F`, however often it dies. Equal leverage
for the two factors on the log score needs a power of `√(var log F / var log L)`, which measures 2.3.
That is measured on an atlas a power-1 search had already narrowed in `L`, so 2 is used, the round
value just below it. At power 2, one dead seed in ten costs about a fifth of the score and three
cost half. A higher power puts reliability first and drifts toward dull worlds that never die, which
the game and flow 1's signatures cannot use. The score stays smooth in `L`, as the smooth-landscape
constraint requires, so the emitters see a gradient toward robustness rather than a cliff. It is an
aggregation over seeds, not a new criterion, so it stays inside the authority boundary below. A world
that lives on 3 of 5 seeds at 0.30 scores 0.108, and one that lives on all 5 at 0.25 scores 0.25.
The tempting alternatives each fail:

- **The plain mean over seeds** (power 1) is the expected fitness. It is the natural reading of
  "how reliably, times how good", but `F`'s spread swamps `L`'s, so a search ranked on it climbs
  `F` and leaves the archive no more robust than an unscored one.
- **The median over seeds** ignores how the losing seeds lose. It reaches 0 only once most seeds
  die, so a world that dies on 2 of 5 scores as if it never did.
- **A lower tail of the seeds** (the mean of the worst few) punishes fitness spread among live seeds
  as well as death, and it is flat at 0, with no gradient, once that many seeds die. Robustness here
  means the verdict holding across seeds, which `L` measures directly.
- **The mean less a multiple of the seed spread** mixes the noise of a world that dies with the noise
  of one that lives unevenly, and its multiple has no natural scale.
- **A floor on the live fraction** is a cliff in parameter space, which the smooth-landscape
  constraint forbids ([world rules](world-rules.md), *Smooth parameter landscape*).
- **The live fraction as a behaviour axis** would make fragility a niche the archive preserves,
  the opposite of selecting against it.

Whether a cell is live or dead stays with its majority of seeds, so the dead frontier keeps its
meaning: a cell is on a cliff when most of its seeds fall off it. The projection's coexistence floor
and refinement stay, as a second, stricter guard on the one world the player is given. 

**A cell's recorded fitness rests on 10 seeds.** Seed-to-seed fitness sd is about 0.15 on
#677's atlas, against a median cell fitness of 0.19, so the mean's standard error is about 0.067 at
5 seeds, 0.047 at 10 and 0.033 at 20. Ten seeds halve the variance and resolve the live fraction to a
tenth, and keep a search near half an hour. Twenty would double the cost again for a third less error,
and the one world given to the player already gets that precision from refinement at 32 seeds. How a
search avoids paying for 10 seeds on a hopeless candidate is implementation. It may, for example,
evaluate 5 and add the other 5 only to a candidate that could enter or replace an archive elite. But
no cell's recorded fitness rests on fewer than 10 seeds, since a 5-seed estimate is the lucky draw
this section exists to discount.

A live cell's descriptors come from its **median-fitness live seed**. The gated seeds score 0 and sit
at the bottom of the fitness order, so the median of the whole ensemble can be a dead seed, or a live
one well below the live seeds' typical world. The median of the live seeds is the world the cell is
when it lives. A dead cell's descriptors are never read. "Most" is strictly more than half, so a 5–5
split stays live and pays for its dead half in the mean. A dead cell is tallied on the cliff most of
its dead seeds hit.

Adaptive top-up is opt-in (`--top-up-screen 5`), not the default. A config's seeds roll out in
parallel, so 10 seeds cost far less than twice the wall clock of 5, and the mean over 10 keeps a
search inside the half hour this section budgets. Top-up saves only part of the remainder: a 5 + 5
top-up runs as two sequential waves, each paying for its slowest seed. And its saving comes from
deciding on the screen alone. A config whose screen is dead goes to the dead frontier on 5 seeds, and
a live one is turned away on a 5-seed estimate — the lucky draw the mean over 10 exists to discount.
No recorded cell rests on the screen, but the frontier and the emitter's signal do.

*Current state (#710, 2026-10-06): the search scores a cell by `L² · F` over its 10 seeds (`live_fraction_score` and `LIVE_FRACTION_POWER` in `qd.rs`, reached only through `config_eval_from_ensemble`). Refinement reduces through the same function, so the recipe's refined fitness is `L² · F` too. The atlas records it as `provenance.scoring.aggregation: {"live_fraction_power": 2}`. An atlas recording `mean_over_seeds` was ranked by the plain mean (#699), and one without `scoring` by the median seed of 5; both read as before. A checkpoint is schema version 3, so a mean-scored search's checkpoint (version 2) is refused rather than resumed into an `L² · F` search. #711 and #719 searched under it, and the committed atlas is #719's (#687). #686's atlas, searched under the mean, failed its robustness rule: all seeds agree in 43.4 % of worlds, against 57.6 % on #677's atlas, then committed ([686-robust-atlas.md](../research/686-robust-atlas.md)). The variance figures above are measured on that atlas's audit seeds.*
*Measured (#710): re-reduced from the fragility audit's radius-0 rows (10 seeds per cell), `L² · F` reorders the cells a mean-ranked search kept only a little. On the committed atlas (99 cells, 42 with a dead seed) Spearman ρ against the mean is 0.960 and 9 of the top 10 stay; the median cell moves from 0.200 to 0.178, the best (0.445) is unchanged, and the recipe's cell (atlas:90, live on all 10) keeps 0.255 and rises from rank 33 to 28. On #686's atlas (83 cells, 47 with a dead seed) ρ is 0.963 and 8 of the top 10 stay; the median moves from 0.214 to 0.200. Both atlases were selected under the median seed or the mean, so this reads how the score re-ranks worlds already chosen, not what a search under it would choose.*
*Measured (#711, 2026-10-07; [711-robust-score-atlas.md](../research/711-robust-score-atlas.md)): the seed-42 search rerun with only the score changed moves robustness the right way, though not to the bar #711 set. Against the mean-scored #686 atlas, the share of worlds whose 10 fresh seeds all agree rises from 43.4 % to 47.1 %, the mean live fraction from 0.864 to 0.887, and the median live-seed fitness holds (0.281 → 0.276). Both stay below the committed atlas (57.6 %, 0.900) and the 70 % bar. Under a binomial model, a world living with probability p agrees on all 10 seeds with probability p¹⁰ + (1 − p)¹⁰, so 70 % of worlds agreeing needs most of them at p ≥ 0.97. The search took 29 min, against 55, because it no longer climbs into the dense worlds near the cliffs.*
*Current state (#699, 2026-10-06): the search scores a cell by the mean over a 10-seed ensemble
(`config_eval_from_ensemble` and `SEARCH_ENSEMBLE_SIZE` in `qd.rs`). The atlas records its scoring as
`provenance.scoring` (`mean_over_seeds`, the ensemble size, and the top-up screen when one ran). An
atlas without it, such as #677's (committed until #687), was ranked on the median-fitness seed of 5.
#686's atlas was searched under the mean; the committed one (#687, #719's) under `L² · F`.*
*Measured (#699): re-reduced on the fragility audit's 10 seeds per cell (seeds 1000–1009, radius 0;
[693-fragility-audit.md](../research/693-fragility-audit.md)), the committed atlas's 99 cells move
from a median cell fitness of 0.130 under the median seed of the first 5 to 0.200 under the mean of
10 (best 0.424 → 0.445; per-cell change median +0.014, median |Δ| 0.066, range −0.171 to +0.240).
The ranking mostly holds (Spearman ρ 0.84 between the two reductions; 6 of the top 10 cells stay
there), 1 cell goes dead and 2 come alive, and the recipe's cell (atlas:90) rises from 0.102 to 0.255.
On 32 identical bootstrap configs at `T = 2000`, 10 seeds took 392 s against 280 s for 5 (×1.40),
and 300 s with a 5-seed top-up screen (×1.07). A standard search (`--batch 32 --generations 10
--max-ticks 2000`) is therefore expected at ~18–27 min from the 13–19 min it took at 5 seeds, or
~14–20 min with top-up, and its dense-generation tail grows by the same factor. The top-up run's dead
frontier matched the 5-seed run's exactly (5 lockup, 1 energy death), where the full 10 seeds read
4 lockup and 1 bloom stop.*

## Authority boundary: the heterotroph guilds are reported, never optimised

A **decomposer** is a behavioural role — a **trophic role**, read from an agent's recent realised income — confirmed across seed
ensembles but **sporadic per seed** — a persistent guild forms in only a fraction of surviving runs
([expected properties](expected-properties.md); CONTEXT.md, *Decomposition*). The atlas therefore records
it as a **per-cell distribution** — the fraction of a cell's seed ensemble that holds a
**[heterotroph guild](../../CONTEXT.md)** of that role, with the sample count — and **never** as a
behaviour axis or a fitness term. The same read is reported for the **consumer** role
(`consumer_fraction` beside `decomposer_fraction`). The heterotroph share of living energy and of
income, by trophic role, is reported per seed under the same boundary (#602): the evaluator reads it
off the terminal roster (`FitnessBreakdown::heterotroph_shares` — consumer and decomposer shares of
energy and of recent income, over the agents with a role, `None` where no terminal roster was read),
and each atlas cell records its ensemble's per-seed reads as `heterotroph_shares` beside the guild
fractions. An atlas or checkpoint written before it reads back with the list empty.

A guild is a *population*, not a role tag on one agent (#490; the #443 re-read found the atlas's resident
"guild" to be a single sessile mixotroph on most seeds, a quarter of them sterile at the `kappa` clamp).
Membership is each agent's **trophic role** — read from recent realised income, never from the trait
vector — taken on the rollout's roster snapshots (the `coexistence_sample_interval` cadence)
over the **second half** of the run. The guild holds when the role's count reaches `GUILD_MIN_SIZE = 5`
on **every** such sample *and* at least one `Born` event in that window names a member as a parent —
sustained size rules out the lone long-lived individual, recruitment rules out a sterile founder cohort
sitting at exactly the floor. Both values are starting points, not settled thresholds.

The income read is the evaluator's own ledger (`explorers_genesis_eval::income`), built from the
rollout's `Photosynthesized` and `Consumed` events and never stored in the world. Income decays with a
half-life of `INCOME_HALF_LIFE = 50` ticks, so a role follows a diet change within a few hundred ticks
while averaging over the tick-to-tick flicker of feeding; the half-life is a starting
value too. An agent with no income yet — a newborn — has no role and is left out of every role count.
The same read buckets the heterotroph shares, so the evaluator has one role read.

Membership reads income, not traits, because a guild is a population occupying a network position, and
a trait read does not find one. Among agents whose heterotrophy exceeds their autotrophy, about 97 % take
at least 90 % of their income from light, and a trait-read decomposer guild appears on seeds where no
agent lives on carcasses ([596-role-tag-vs-diet.md](../research/596-role-tag-vs-diet.md)). A light-fed
mixotroph carries heterotrophy it mostly cannot use (its kin are spared by recognition, world rules,
*Recognition*), so a trait read would count agents that consume little.

**Decided (#494): the guild stays reported.** It is not a binning axis and it does not fold into
`coexistence_fraction`. The settled-horizon atlas was regenerated with the read reported
([494-guild-atlas.md](../research/494-guild-atlas.md)):
- No live cell reaches a guild fraction ≥ 0.5.
- Living worlds hold a decomposer guild at the same 0.9 % per seed whether the search selected them or
  not, so the objective is not selecting against guilds.
- Only 1 of 141 live unselected configs is a guild cell. A binning axis would sit on a near-flat
  signal, and a guild-aware floor would clear none of the refined top-10.
- The guilds the gates remove are mostly a mixotroph straddle, not a second trophic level (#546).

Revisit if a stepper or threshold change makes live guild cells common enough to climb.

To put the fold option to that decision on one fixed atlas (#494, #538), the projection can read a
**guild-aware floor**: `explorers-search --coexistence-floor decomposer|consumer|either`, with the full run
or with `--reproject`, counts a refinement seed as coexisting only if it also holds that guild. It is a
comparison instrument, not a change to this boundary. The default is `plain`, the refinement always
reports the refined fraction under all four floors from the same seeds, and the floor changes only which
cell the recipe is picked from (and so where refinement in fitness order stops), never the atlas, its
binning or its fitness.

This is the same existence-vs-distributional boundary [viability](viability.md) already respects when it
makes `C*` a *characterisation* rather than a gate: the atlas maps the existence/stability skeleton of
parameter space (the three axes are all existence/stability quantities), and it may not collapse a
distributional emergence into a coordinate, because a behaviour axis is a *point* per config while the
guild's truth is a *fraction of seeds*. The boundary is enforced mechanically: the guild signal rides on
the evaluator's output as a reported observable, alongside the other non-fitness readings, and is never
summed into fitness nor binned on.

The same boundary decides what a **terminal gate** zeroes. A gated world has no meaningful behaviour
*coordinate* (above), so every scored descriptor is zeroed and the world gets no cell — but an
observable is not a coordinate. A world gated on its **terminal** read ran the whole horizon, so its
settled window `(T/2, T]` was classified like any other and the guild predicate is as computable, and
as true, there as on a live world; zeroing it was a measurement artefact that hid real guilds inside
monoculture and lockup cells (#527, measured in
[519-guild-read-at-settled-horizon.md](../research/519-guild-read-at-settled-horizon.md)). So the guild
flags are carried through the horizon gate and the scored descriptors are not. A rollout stopped
**before** the horizon is the other case: it has no settled window to read, so its flags report `false`
and "no guild" is indistinguishable from "not read" — accepted rather than made tri-state.

Carrying the flags through is not a reason to **let the world through**. The terminal gates stay blind to
the guild: a `monoculture` or `generalist_dominance` world that holds a decomposer guild is still a
failed world. Measured at `T = 2000` (#546,
[546-guild-in-monoculture.md](../research/546-guild-in-monoculture.md)), the guilds inside gated worlds
are mostly the heterotroph tail of one trait continuum straddling the photo = hetero line. The role
read's producer / heterotroph cut splits a mixotroph monoculture into a "producer" and a "decomposer"
population; it is not a second trophic level. Only 2 of 12 monoculture-gated guild seeds read as a
separate, diet-specialised cluster, both borderline, and none at the generalist gate's own ±0.2 band.
The gate fires well clear of its threshold on every one (`clustering_strength` 0.00–0.37 against 0.5).
The clean guild worlds are sharply bimodal and live. So a gated world's guild flag is reported, but
it is not evidence that the gate misfired, and the atlas's gated-guild count overstates the real
guilds the gates remove.

A **further** reported per-seed distribution rides under the *same* boundary: the **coexistence
fraction** — the share of a cell's seed ensemble that lands in the coexisting regime (alive, and either
clustering or coexisting; the `||` is the #359 small-N disjunction so clustering's silent zero below
n≈4 does not under-count). Like the decomposer fraction it is recorded with the sample count and is
**never** a behaviour axis nor a fitness term; the monoculture↔coexistence *axis* is still the
representative seed's `clustering_strength` (the median-fitness live seed). Note that a coexisting seed need not hold a guild: `coexistence_fraction`
reads separated trait clusters, which one heterotrophy-dominant individual already supplies — the guild
fractions are what say whether a second trophic *level* is present. What the fraction adds is visibility into how a cell's ensemble splits
across the regime — the raw material the projection reads (below).

Because each cell's elite is selected on a noisy estimate over seeds, a lucky elite can misrepresent its
cell. The archive tolerates this descriptor noise by design (a soft per-cell acceptance threshold rather
than a single sticky occupant) rather than pretending each cell is a noise-free point. The coexistence
fraction makes that noise *visible*: a cell whose representative seed coexists on a lucky draw but whose
ensemble mostly monocultures reads a low fraction, and the projection (below) now reads it.

## Predicted bifurcation coordinates and the cross-check

The atlas's three behaviour axes are *observed* — read off a rollout's per-seed breakdown. Two of the
dynamics failure modes also admit a cheap *closed-form prediction* of which side of their bifurcation a
config sits on, lifted from the two validated research spikes ([#358 Hopf](../research/F-hopf-validation.md),
[#359 branching](../research/F-branching-validation.md)) into `crates/explorers-search/src/bifurcation.rs`:

- **`oscillation_distance` = `|λ| − 1`** of the living-mass↔available-pool 2×2 Jacobian (Brief F AC1's
  self-contained coupling) — `< 0` frozen, `> 0` limit cycle.
- **`branching_distance` = `D`**, the adaptive-dynamics invasion margin of a rare heterotroph into the
  founder monoculture — `> 0` coexistence, `< 0` monoculture.

Both are **descriptors, not objectives, and not binning axes.** This is the deliberate choice, and it is
the one the committed research permits. Both #358 and #359 returned a *Qualified GO* that explicitly
gates objective-promotion on work this design does **not** yet do: hardening the genesis observables
(#358 — `oscillation_strength` is flat and demographic-pulsing-dominated at search scale; #359 —
`clustering_strength` silently zeroes below n=4) and putting spatial in-reach geometry plus a wear
penalty into the objective's environment. So the readings enter as per-cell descriptors plus a
predicted-vs-observed cross-check, never summed into fitness — exactly as [viability](viability.md) keeps
`C*` a *characterisation* rather than a gate. **Objective-promotion remains gated** on that
observable-hardening (#358/#359); the cross-check's regime tag (below) makes the gate's status legible on
every sweep.

**Reduced coordinates for a single-founder config.** A QD config carries one founder *mean* trait vector
— no producer/consumer pair (that is emergent). So branching `D` is computed on the founder mean
directly (sweep a rare heterotroph against the founder-as-producer in the founder-monoculture
environment), and the Hopf reading uses the living-biomass↔available-pool coupling rather than the
prototype's producer↔consumer pair (which needs two clusters a config lacks, and whose observable #358
showed decouples at scale anyway). Both reuse the committed kernel (`trophic_transfer_efficiency`) and
`TraitVector::distance` verbatim.

**Conductance-aware under flow 9.** The [reserve mobilisation rate](world-rules.md) `f` (flow 9) sets
how fast above-buffer reserve is mobilised into the growth flow. Both readings are aware of it, and the
asymmetry between them is a direct consequence of reserve conservation. At any reserve fixed point the
mobilised flow per tick *equals* net income `I − b` (inflow must balance outflow when reserve returns to
the same value each tick), so `f` only sets where reserve sits (`R* = ρb + (I − b)/f`, larger as `f → 0`)
and how fast it relaxes (`~1/f`), **not** the steady-state throughput. The selection diagonals are
steady-state objects — invasion fitness and Jacobian eigenvalues are asymptotic, read after the reserve
transient — so the **branching margin `D` and the interior fixed-point location are f-invariant**: a
naïve scalar `f` on `net_energy` would be wrong, because the throughput the operator reads is net income
regardless of `f`. The **oscillation reading is not** f-invariant in the same way: the conductance rate
turns reserve into a genuine slow state on the consumer's income→structure pathway (the flow `f`
governs), so the Hopf coupling is a three-compartment available-pool↔reserve↔living-mass map whose third
eigenvalue (`≈ 1 − f`) and lagged structure-building shift the **Hopf boundary** with `f` even though the
fixed point does not move. The reduction is exact at `f = 1` (reserve is slaved within one tick and the
map collapses to the two-compartment pool↔living-mass form). The reserve buffer sits only on the consumer
growth pathway: the producer's photosynthetic income refills reserve every tick, so the pool-fill
diagonal is f-invisible (flow 9 names this producer/consumer asymmetry), and maintenance stays lumped on
the living-mass decay term rather than drawn from reserve, which is what keeps the `f = 1` reduction
exact. Because the oscillation observable is `WeakObservable` at current scale, the predicted-vs-observed
crosscheck across `f` is exercised for the branching axis (its realised steady-state rate is directly
measurable and reads f-invariant); for the oscillation axis the rollout crosscheck stays gated on the
hardened cycle-detector, and the descriptor is held to its analytic invariants (fixed-point
f-invariance, exact `f = 1` reduction, a monotone boundary shift in `f`).

**The cross-check, regime-tagged.** For every *live* config the predicted sign is compared against the
observed behaviour-axis boundary, and every disagreement is surfaced (`bifurcation_disagreements`),
never swallowed — the validation-triad cross-check both spikes prize. Each disagreement carries a
**regime**:

- `Validated` — the observable is trustworthy here, so the disagreement implicates F's spectral reading.
- `WeakObservable` — the observable is known-weak here, so the disagreement localises to the
  observable (or its geometry), not F. The **branching** axis is `WeakObservable` exactly on #359's
  small-N borderline (`clustering_strength == 0` while `coexistence_duration > 0`), else `Validated`
  (wear is off for every searched config, satisfying #359's other validated-regime condition). The
  **oscillation** axis is *constant* `WeakObservable` at current genesis scale — #358's verdict is that
  `oscillation_strength` cannot adjudicate the Hopf crossing — and flips to `Validated` only once a
  hardened cycle-detector lands (a separate issue).

**Authority boundary.** Like the three axes and `C*`, these readings arbitrate **existence/stability
only**. They never read the heterotroph guilds or any per-seed distributional property (the decomposer and
consumer fractions, the coexistence fraction), are never summed into fitness, and are never a binning axis — the
same existence-vs-distributional boundary the rest of the atlas respects.

## The recipe is a projection of the atlas

A single playable [world recipe](../../CONTEXT.md) is still drawn from the search, because the app needs
one world to drop the player into. It is the elite of the highest-fitness live cell **whose refined coexistence fraction
clears 0.90**: at least 29 of 32 independent seeds coexist (`COEXISTENCE_FLOOR`). That is CONTEXT.md's
bar, *"accepted only when most runs in the ensemble produce sensible worlds"*, read for the one world a
player replays. The player meets the world one playthrough at a time, in an open sandbox with no reset,
so one playthrough in ten failing to coexist is the most the world can afford. The floor reads
coexistence rather than the live fraction, because a world that lives as a monoculture is alive but is
not the world the game needs. At n = 32 the floor passes a world that coexists 95 % of the time with
probability 0.93, and one at 80 % with probability 0.09. A world at exactly 90 % passes only 60 % of
the time, so the floor in effect asks for about 95 %. A floor at a half, the bar's literal reading,
would hand over a world that fails to coexist on half of its playthroughs.

**The pick refines in fitness order until a world clears.** The search refines live cells in order of
recorded fitness and stops at the first whose refined fraction clears the floor, up to a cap of about
40 cells. That keeps the pick the highest-fitness world that clears while bounding its cost. On
#711's atlas, the first cell to clear was the 7th in fitness order. A fixed top-K would find none
whenever every robust world sits just below the cut. When no refined cell clears, the pick falls back
to the refined cell with the **highest refined coexistence fraction**, with a warning, so the player is
given the most robust world found rather than the one that drew the highest fitness.

The why is #401: the 5-seed median that ranked cells then is high-variance near the monoculture↔coexistence
bifurcation, so a **straddler** — a cell that coexists on only a minority of initial conditions — can win
a lucky draw and top the leaderboard while its typical outcome is monoculture (the live #401 leader scored
fitness 0.67 yet re-evaluated to median 0 over an independent 8-seed ensemble, coexisting on ~3/8 ICs).
This is **selection only** — not binning, not fitness: the straddler is still a recorded cell with its
real fitness; only the *recipe pick* reads the floor, so the atlas map stays untouched and now visibly
honest. The atlas's honest stance is that *many* worlds across the manifold are viable, so any cell's
elite is reachable as a recipe, not only the projected one. "The best recipe" is one pick from a map, not
the search's output.

*Current state (#715, 2026-10-07): the projection runs this rule. `COEXISTENCE_FLOOR` is 0.90, tested exactly in integers (`clears_coexistence_floor`: 29 of 32 clears, 28 does not); refinement runs in recorded-fitness order and stops at the first cell that clears, up to `REFINE_CAP` = 40 (`--refine-cap`, which replaces `--refine-top-k`); and the fallback is the refined cell with the highest refined coexistence, a tie going to the higher fitness. A cell's refinement seeds depend on its rank alone, so the cells #711's top-10 refinement read are read again on the same seeds. The search log names the pick and how many cells were refined to reach it. #711's recipe, picked under the old rule, coexists on 0.88 of its refined seeds; *Measured* (#715): `--reproject` of #711's atlas under this rule refines 7 cells and picks the seventh, [12, 19, 1] (refined coexistence 0.97, 31 of 32; refined fitness 0.33 against the old pick's 0.41), in 103 s; #686's top 10 hold five above 0.90. The committed recipe (#687) is this rule's pick on #719's atlas: its top cell, [5, 19, 1] (atlas:31), cleared the floor on the first refinement, with refined coexistence 32/32 and refined fitness 0.704.*

**Gated elite refinement hardens the pick (#404).** The floor above reads the *same* in-run
ensemble that ranks the cell, and that estimate is itself high-variance near the bifurcation — so a lucky
draw can both top the leaderboard *and* clear the floor (the #401 leader read 0.60 = 3/5 in-run yet
re-evaluated to ~3/8 over an independent draw). Before projecting, the search therefore **re-evaluates live
cells** at a **larger, independent ensemble** (`REFINE_ENSEMBLE_SIZE`, ≫ the search's 10), in fitness
order as above, and applies the floor to that **refined** fraction. The refinement seeds are deterministic but drawn
far above any seed the search used (offset `2^40`), so the re-evaluation is an *independent* draw, not a
re-read of the in-run seeds, and a fixed `(atlas, seed)` refines bit-reproducibly. The pick is the
highest **recorded**-fitness refined cell whose refined fraction clears the floor. This stays inside the authority boundary: refinement
**never** rewrites the atlas map's binning or per-cell fitness — the recorded fitness remains the ranking
key, the refined fraction feeds only the pick, and the straddler stays a recorded cell. Its cost is
bounded by the cap and logged, including the live cells that were not refined. The refinement size is
a *separator*, not an estimator: at n = 32 the 0.90 floor tells a world at p ≈ 0.95 from one at
p ≈ 0.80 with under 10 % error either way, and a sequential (SPRT) alternative was evaluated and not
adopted — the arithmetic is in [`docs/research/434-ensemble-confidence.md`](../research/434-ensemble-confidence.md).

**The projection re-runs without the search (#531).** Because the pick is a function of `(atlas, seed)`
and the projection settings alone, the atlas records the search's seed and horizon (`provenance`), and
`explorers-search --reproject ATLAS` runs refinement and projection against an atlas file with no search.
It yields exactly the recipe the run that wrote the atlas projected under the same `--refine-cap` /
`--refine-ensemble`, so a refinement that dies costs minutes rather than a regeneration, and projection
settings can be compared on one fixed atlas instead of across two searches' draws.
