# Research Brief F — Mean-field operator as affirmative viability theory

**Status: research finding. Commits nothing.** This is the Cluster-F deep-dive for epic
[#345](https://github.com/) (issue [#348](https://github.com/)). It is a recommendation, not an
implementation, and deliberately lives outside `docs/system-design/` — the system-design layer is
self-justifying and commits the design; this layer records investigation that *informs* a future
commitment. Nothing here introduces a mechanism, a functional form, or a parameter into the world
rules. It proposes an *analysis lens*: a reduced operator derived **from** the committed map `T`,
whose spectrum makes the dynamics failure modes computable. Any operator term named here is a
*reading* of an existing phase, never a new physics.

This brief is the affirmative theory `viability.md` explicitly points toward. That document calls
today's gates a *"negative-space sketch"* of an *"eventual affirmative theory"* and files
*monoculture↔coexistence, frozen↔oscillation, population explosion, generalist dominance, and
nutrient lockup* as *"bifurcation and stability territory… beyond cheap a priori reach (for now)."*
F is the proposal for how that reach is bought: discretise `T` into a transition operator on a
density over (space × trait), then read its fixed points, leading eigenvalue, eigenvalue-pair
crossings, and invasion fitness as the failure modes named qualitatively today.

## TL;DR

1. **The central question — which reduced coordinates each failure mode lives on — has a clean,
   per-mode answer, and it is not the same answer for every mode.** The 7-dim trait space will not
   densely bin (the curse of dimensionality: a 10-bin grid over 7 traits is 10⁷ cells, mostly
   empty). But no single failure mode needs all seven. Extinction/explosion/energy-death/lockup are
   **scalar-density** modes (they live on total living mass and total available-pool nutrient, with
   *no* trait-resolution requirement). Monoculture↔coexistence is the **only** genuinely
   distribution-shaped mode and lives on the 3-dim specification subspace (autotrophy × heterotrophy
   × mobility). Frozen↔oscillation lives on a 2-compartment coupling (living mass ↔ available pool,
   or producer mass ↔ consumer mass) — a 2×2 Jacobian, not a trait grid.
2. **Recommendation: hybrid, not a single discretisation.** A **scalar-compartment ODE** (living
   energy, available nutrient, carcass-locked nutrient) carries the existence/flux modes and the
   Hopf onset; **moment closure** (per-compartment trait mean + covariance, the
   Price-equation / quantitative-genetics view) carries the *direction* of monoculture-vs-coexistence
   cheaply; a **coarse 2-3 trait bin** is reserved only for confirming *multi-peakedness* once the
   moment view flags positive disruptive selection. Binning the full 7-dim space is rejected outright.
3. **Every tick phase maps to a standard operator term**, in the phase order `execution-model.md`
   fixes: photosynthesise+absorb+metabolise+grow → a **selection/growth** diagonal; mutation at
   reproduction → a **mutation–diffusion** kernel in trait space; dispersal → a **convolution** in
   physical space; resolve_drains → the **bilinear (quadratic) trophic** term that makes the operator
   nonlinear (and is the seat of the Hopf); death → a **removal/relabel** operator that is also the
   *source term* of the carcass compartment. This is the operator decomposition the AC asks for, and
   it lands exactly on the real phase names in `phase.rs`.
4. **The nutrient-lockup generalisation is the single sharpest prize.** `viability.md`'s energy-death
   gate is a *static* `N_total` floor that it says *"deliberately sets aside"* carcass-locked nutrient
   and is therefore "necessary, not sufficient." F replaces it with a **three-compartment flux
   balance** (available ⇌ living ⇌ carcass) whose stationary carcass fraction is a closed-form ratio
   of death flux to decomposer turnover. *That* is the dynamical generalisation viability.md says it
   "cannot yet reach" — and it is reachable because every flux is already a committed rule.
5. **Authority boundary is hard and non-negotiable.** Mean-field drops demographic stochasticity and
   per-seed realisations. F arbitrates **existence and stability** (does a non-trivial attractor
   exist; is it a point or a limit cycle; is the trivial state global). F **must never** arbitrate
   the **distributional** modes — above all the *sporadic-per-seed decomposer guild*, which
   `expected-properties.md` explicitly calls *"distributional… confirmed across seed ensembles,
   sporadic per-seed, never guaranteed on a single run."* Mean-field erases exactly the rare-event,
   single-realisation structure that emergence is. This mirrors the existence-gate vs. dynamics split
   `viability.md` already draws, one level up: F is allowed to deepen the *dynamics* half, never to
   claim the *distributional* half.

## What the map actually is today (the baseline this brief reasons from)

All citations are against `crates/explorers-sim/` at the time of writing.

- **`T` is `World::step()`** (`lib.rs:986+`), a fixed-order discrete map `state_{t+1} = T(state_t)`
  exactly as `viability.md` formalises it. The phase order (confirmed in `step`): build grid →
  photosynthesise (`phase.rs:19`) → absorb_nutrients (`phase.rs:88`) → metabolise (`phase.rs:156`) →
  grow (`phase.rs:189`) → resolve_drains (`phase.rs:386`, coordinated pass 1; deaths marked) →
  remove drain-dead → resolve_reproduction (`phase.rs:910`, coordinated pass 2) → move_agents
  (`phase.rs:752`) → apply_wear (`phase.rs:309`) → check_death_thresholds (`phase.rs:687`) → two
  conservation ledgers (energy + closed nutrient).
- **State is a point cloud, not a density.** `Vec<Agent>` + `Vec<Carcass>`. Each `Agent` carries
  `reserve, structure, peak_structure, nutrient, position, traits, wear[3],
  repro_reserve, repro_nutrient`; each `Carcass` carries the **exact `TraitVector`** of the agent
  that died (`lib.rs:683+`; CONTEXT.md, *Carcass*, *Decomposition*). F's job is to lift this cloud
  to a density `n(x, θ)` over physical position `x` and trait vector `θ`, plus the scalar carcass and
  available-nutrient fields.
- **Trait vector is 7-dim, three layers** (CONTEXT.md, *Trait vector*): allocation (kappa);
  specification (autotrophy, heterotrophy, mobility); reproduction (fecundity, asexual propensity,
  dispersal). *Roles are read from θ, never assigned* — "producer / consumer / decomposer" are
  regions of trait space, not types. This is what forces a trait-space density rather than a
  per-species compartment model.
- **Trophic transfer is a committed bilinear form.** `trophic_transfer_efficiency =
  base_trophic_efficiency · exp(−trophic_distance_decay · d)` (`lib.rs:768`), `d` = trait-space
  distance consumer→target — now owned by `world-rules.md` flow 7 and promoted into system design
  (#294). Drain demand is a **binary-reach drain** (#380): while a target is within feeding reach the
  consumer drains it at its full effective heterotrophy `eff` each tick — no contact-duration term.
  This is the *kernel coefficient* of F's nonlinear term — already in closed form, which is exactly
  why F is feasible.
- **Nutrient is closed and conserved** (`lib.rs` nutrient ledger, balanced every tick to 5e-3
  relative). Energy is open: solar flux in, dissipation + transfer-loss out. This closure is the
  backbone of the lockup flux balance.
- **Randomness enters only at reproduction and movement** (per brief A: asexual roll, per-trait
  mutation, dispersal placement, sexual tie-break, random-walk step). Every other phase is
  deterministic. Mean-field replaces these *draws* with their *expectations* — which is precisely
  where F sheds its authority over the distributional modes (below).

## AC1 — Which reduced coordinates each failure mode lives on (the central question)

**Lead finding: the implicated coordinate set is per-mode, and most modes are far lower-dimensional
than the 7-dim trait space.** This is what dissolves the curse-of-dimensionality objection — you
never discretise all seven at once because no single failure mode is governed by all seven. The
table is the spine of the brief; each row is justified beneath it.

| Failure mode | Lives on (reduced coordinate) | Operator object that decides it | Discretisation | Recommendation |
|---|---|---|---|---|
| **Extinction** | total living energy mass `M` (scalar) | leading eigenvalue of `T` linearised at `n≡0` | scalar compartment | **scalar ODE** — no trait resolution needed |
| **Energy death / nutrient lockup** | available-pool nutrient `A`, carcass-locked nutrient `C` (2 scalars) + decomposer turnover | 3-compartment flux balance `A⇌M⇌C`; stationary `C` fraction | scalar compartments + 1 turnover coefficient | **scalar ODE** (the affirmative generalisation of the `N_total` gate) |
| **Population explosion** | total living mass `M` (scalar) | sign of leading eigenvalue as `M→∞` (density-dependence of net growth) | scalar compartment | **scalar ODE** — read whether per-capita growth ever turns negative |
| **Frozen ↔ oscillation** | a 2-compartment coupling: (producer mass ↔ consumer mass), or (living mass ↔ available pool) | complex eigenvalue pair of the 2×2 (or low-block) Jacobian crossing the imaginary axis → **Hopf** | low-block (2-4) Jacobian | **scalar/low-block ODE** — *not* a trait grid |
| **Monoculture ↔ coexistence** | the 3-dim **specification** subspace (autotrophy × heterotrophy × mobility) | single- vs multi-peak stationary `n(θ)`; **invasion fitness** sign (adaptive dynamics) → evolutionary branching | **moment closure** (mean+cov) to *detect* branching; coarse 2-3 trait bin only to *confirm* peaks | **moment closure first, bin to confirm** |
| **Generalist dominance** | same 3-dim specification subspace, read along the diagonal (autotrophy≈heterotrophy≈mobility) vs. the axes | stability of the *differentiated* (off-diagonal) state vs. the diagonal attractor | moment closure (covariance structure: diagonal vs. axis-aligned) | **moment closure** — see authority caveat below |

### Why these coordinates, mode by mode

- **Extinction, energy death, population explosion, nutrient lockup are scalar-density modes.** None
  of them asks *what shape* the trait distribution has — they ask whether *mass exists at all*,
  whether *the available pool is being starved*, or whether *mass grows without bound*. The extinction
  gate viability.md already states (`F ≤ B` ⇒ no producer net-positive) is exactly the statement that
  the leading eigenvalue of the linearisation at `n≡0` is `≤ 1` — F generalises the two-number gate
  into "is the trivial fixed point the global attractor." None of these needs trait resolution, so
  binning is the wrong tool: a scalar (or three-scalar) compartment ODE carries them with no
  dimensionality cost at all.
- **Frozen↔oscillation lives on a coupling, not a grid.** `expected-properties.md` (Population
  oscillations) defines the healthy state as coupled producer↔consumer (or mass↔resource)
  fluctuation — positive feedback (reproductive amplification) against negative feedback (resource
  depletion, metabolic cost). That is a predator-prey loop, and its onset is a **Hopf bifurcation**:
  a complex-conjugate eigenvalue pair of the *low-block* Jacobian crossing the imaginary axis (in
  discrete time, crossing the unit circle). "Frozen" is a stable node (all eigenvalues inside);
  "oscillation" is the limit cycle past the crossing. The governing object is a 2×2–4×4 Jacobian
  block, **not** a trait histogram — so again binning the 7-dim space is the wrong instrument.
- **Monoculture↔coexistence is the one genuinely distribution-shaped mode**, and it lives on the
  **specification subspace** (autotrophy, heterotrophy, mobility) because that is the subspace that
  defines trophic *role* (CONTEXT.md: role is read from these three). Coexistence = a *multi-peak*
  stationary `n(θ)` (producers at high-autotrophy, consumers at high-heterotrophy, separated by gaps
  — exactly the dip-test / DBSCAN signature, CONTEXT.md). Monoculture = a *single* peak. The
  transition between them is **evolutionary branching** in the adaptive-dynamics sense (Geritz et
  al.): the resident is at a singular trait point where the second derivative of **invasion fitness**
  turns positive — disruptive selection splits the peak. The decisive cheap object is the *sign of
  invasion fitness* of a nearby mutant against the resident density, which moment closure delivers
  without ever forming the full histogram.
- **Generalist dominance lives on the same specification subspace, read as a direction.** A
  generalist sits near the *diagonal* (autotrophy≈heterotrophy≈mobility all moderate); specialists
  sit near the *axes*. `viability.md`'s resolved finding is sharp here and constrains F: there is *no
  static gate*, because the anti-generalist force is **structural fragility** (#9, a mortality term)
  and **functional incompatibility** (#2, the sessile-autotrophy/mobility morphological clash) —
  *both dynamic*. F can in principle read this as the stability of the off-diagonal (differentiated)
  attractor vs. the diagonal one, via the covariance structure under moment closure. **But** see the
  authority caveat: fragility acts through a *peak-relative structural death threshold*
  (`phase.rs:687`, `lib.rs:731`) whose bite depends on the realised structure trajectory, which a
  mean field smooths — so F's verdict here is the weakest and most provisional of the set.

### The discretisation recommendation, stated plainly

**Reject dense binning of the 7-dim trait space.** Adopt a **three-layer hybrid**, each layer doing
only what its mode needs:

1. **Scalar / low-block compartment ODEs** (living energy `M`, available nutrient `A`,
   carcass-locked nutrient `C`; optionally split `M` into producer/consumer blocks). Carries
   extinction, energy death, population explosion, **nutrient lockup**, and the **Hopf** onset. No
   trait resolution → no dimensionality cost. This is most of the value.
2. **Moment closure on the 3-dim specification subspace** — track per-compartment trait **mean** and
   **covariance** (the Price-equation / quantitative-genetics view). The mutation term feeds the
   covariance; the selection gradient moves the mean; the *sign of the leading covariance eigenvalue's
   growth* (disruptive vs. stabilising selection) is the cheap branching detector for
   monoculture↔coexistence and the diagonal-vs-axis read for generalist dominance. Closed at second
   order (Gaussian closure); the known failure of moment closure is *near* the branching point, where
   the distribution goes bimodal and a unimodal moment model is by construction blind to the second
   peak — which is exactly why layer 3 exists.
3. **A coarse 2-3 trait bin (≤ 8 bins per axis on autotrophy × heterotrophy, optionally × mobility),
   used only to confirm multi-peakedness** once layer 2 flags disruptive selection. This is a
   *targeted* discretisation of the *implicated* subspace, invoked rarely, never the 7-dim grid.

The justification for the split is precisely the central-question finding: **only one mode is
distribution-shaped, so only one mode pays for trait resolution, and even it gets a 3-dim subspace,
not seven.**

## AC2 — Each tick phase as an operator term

The operator `𝒯 = T` acts on the density `n(x, θ)` plus scalar fields `A(x)` (available pool) and a
carcass density `c(x, θ)`. In the phase order `step()` fixes, each phase is a standard term. (Terms
are written as the *expectation* of the corresponding phase — that substitution is the mean-field
approximation and the source of the authority boundary in AC3.)

1. **Photosynthesise + absorb + metabolise + grow → the selection / growth diagonal `G(θ; M, A)`.**
   - photosynthesise (`phase.rs:19`): income = `flux · structure-weighted light share` within the
     light-competition radius. The share denominator is a **local spatial convolution** of
     producer weight `eff_autotrophy · structure` — i.e. a **density-dependent**, competition term:
     per-capita income *falls* as co-located producer mass rises. This density-dependence is what
     bounds growth and is read directly in the **population-explosion** test (does per-capita net ever
     go negative as `M→∞`).
   - absorb (`phase.rs:88`): uptake from the local pool `A`, shared proportionally — couples `n` to
     the `A` field (the nutrient side of the flux balance).
   - metabolise (`phase.rs:156`): the per-agent cost floor `B` plus superlinear trait maintenance —
     the negative diagonal. The extinction gate's `F − B` lives here.
   - grow (`phase.rs:189`): kappa split + Liebig co-limitation of structure on free nutrient.
   - **Net:** these four assemble into a per-`θ` net per-capita growth rate `g(θ; M, A)` — the
     *diagonal* of the linearised operator. Its value at `n≡0` (uncrowded, full pool) is the
     **leading-eigenvalue** input for extinction/explosion; its *gradient in θ* is the **selection
     gradient** that moves the moment-closure mean.
2. **Mutation at reproduction → the mutation–diffusion kernel `𝔻_θ`.** resolve_reproduction
   (`phase.rs:910`) applies per-trait Gaussian mutation (rate `mutation_rate`, sd `mutation_magnitude`)
   and, sexually, uniform crossover. In density form this is **convolution of `n` with a mutation
   kernel in trait space** — to second order, a diffusion term `½·σ²·∇²_θ n`. This is the term that
   *feeds the trait covariance* under moment closure and supplies the variation that branching needs.
   (Crossover is harder: uniform crossover is a *mixing* operator, not a diffusion; under Gaussian
   closure it relaxes toward the population mean. Honest caveat — recorded.)
3. **Dispersal → the spatial convolution `𝔻_x`.** Offspring placement (dispersal kernel widening with
   the dispersal trait) is a **convolution of the birth source in physical space**. On the toroidal
   genesis world (CONTEXT.md, *World extent*) this is a clean circular convolution — diagonal in the
   spatial Fourier basis, which is what makes a spectral treatment of the spatial part cheap.
4. **resolve_drains → the bilinear (quadratic) trophic term `B(n, n)` and `B(n, c)` — the nonlinear
   heart.** resolve_drains (`phase.rs:386`) is, per target, a segment reduction over in-reach
   consumers with edge weight `demand(consumer) · trophic_eff(consumer, target)`, where
   `trophic_eff = base_trophic_efficiency · exp(−trophic_distance_decay · d)` (`lib.rs:768`). In
   density form this is a **bilinear integral operator**: the flux from trait-region θ′ into θ is
   `∫ K(θ, θ′) n(θ) n(θ′) dθ′`, kernel `K` = the committed exp-decay-in-trait-distance form times the
   spatial in-reach indicator (the binary-reach drain, #380 — no contact-duration factor). **This quadratic term is what makes
   `𝒯` nonlinear, and therefore what makes the Jacobian density-dependent — it is the seat of the
   Hopf bifurcation** (the predator-prey coupling) and the engine of disruptive selection (a consumer
   peak feeds on the producer peak, deepening the gap). The living pass `B(n,n)` and the carcass pass
   `B(n,c)` share the *same* kernel — the code runs the same proportional-split algorithm over living
   and carcass targets — which is the operator-level statement of "decomposition is consumption of a
   carcass" (CONTEXT.md).
5. **Death → the removal / relabel operator `R` and the carcass source.** check_death_thresholds
   (`phase.rs:687`) + the structural threshold (`lib.rs:731`) + reserve-zero starvation are a
   **density sink** on `n` — and, because a carcass keeps the *exact* trait vector, the **same flux is
   the source term of the carcass density `c`** (a relabel θ-preservingly from `n` to `c`, not a
   deletion). Decomposition `B(n,c)` then drains `c` back to the pool `A`. **This `n → c → A` chain is
   the entire nutrient-lockup flux balance** (AC's lockup mode), and its closed form is in AC4.

**Assembled:** `n_{t+1} = 𝔻_x 𝔻_θ [ (I + G(θ;M,A)) n_t + B(n_t, n_t) + B(n_t, c_t) − R n_t ]`, with
`c` and `A` carried as coupled fields. Every coefficient (`flux`, `B`, the maintenance exponent,
`base_trophic_efficiency`, `trophic_distance_decay`, `mutation_magnitude`, dispersal width, the death
threshold's fragility scaling) is a **committed world parameter or a promoted functional form** — F
introduces none. This is the AC2 deliverable: the five named terms (selection / mutation-diffusion /
dispersal-convolution / bilinear trophic / removal) each land on a real phase.

## AC4 — Nutrient lockup as a flux balance (the affirmative generalisation)

This is the sharpest single result and deserves its own statement, because `viability.md` names this
exact gap. The energy-death gate there is *static*:

```
N_total ≥ structure_min·(base_nutrient_ratio + spec_coeff·σ_min) + N_repro_threshold
```

and viability.md says of it: *"it ignores nutrient locked in carcasses in transit (death flux ÷
decomposition rate)… clearing it is necessary, not sufficient"*, and that nutrient lockup *"is a
dynamics failure — turnover- and reach-dependent — not decidable from `N_total` alone."* That
parenthetical, "death flux ÷ decomposition rate", **is** the mean-field quantity. F closes it.

With closed nutrient conserved across three compartments — **available pool `A`**, **living-bound `L`**
(free store + earmark + structure-bound), **carcass-locked `C`** — the mean-field flux balance is:

```
dL/dt = uptake(A, producers) + ingest(C+L drained) − death_flux(L)
dC/dt = death_flux(L) − decomp_flux(C; decomposer mass, reach)
dA/dt = decomp_flux(C) + excretion − uptake(A, producers)
       with  A + L + C = N_total  (closed)
```

At stationarity `dC/dt = 0` gives the **carcass-locked fraction in closed form**:

```
C* = death_flux / decomp_turnover         (the "death flux ÷ decomposition rate" viability.md names)
```

where `decomp_turnover` is the per-unit-carcass decomposition rate = (decomposer mass) × (the
committed `base_trophic_efficiency · exp(−trophic_distance_decay · d)` kernel at the
decomposer-on-carcass distance) × the in-reach geometry. **Nutrient lockup is then a single
computable inequality**: lockup occurs when `C*` is so large that the residual available pool `A* =
N_total − L_min − C*` drops below the reproduction floor — i.e. the *available* pool, not `N_total`,
is what must clear the gate. This is the dynamical floor viability.md says it "cannot yet reach,"
and it is reachable precisely because every flux above is a committed rule. Two consequences worth
flagging to a follow-up:

- **It predicts the `example9_detrital_pathway` verdict.** That scenario's field-wide carcass rain is
  unconsumed (decomposer out of reach of the ring), so `decomp_turnover → 0` for the field and `C* →`
  all the field's death flux: F predicts **`nutrient_lockup`**, which is the **8/8 verdict the
  evaluator now reports** (#342). This is a live cross-check the AC5 next-move can formalise.
- **It is an *existence/stability* statement, not a distributional one** — it tells you *whether* a
  world locks up on average, never *which seeds* sprout a decomposer guild. That second question is
  off-limits to F (AC3).

## AC3 — F's authority boundary (what it can and cannot arbitrate)

This boundary is load-bearing and must be stated as sharply as the gates themselves. It mirrors,
one level down, the **existence-gate vs. dynamics** split `viability.md` already draws: viability
today owns *existence* and defers *dynamics*; F is the tool that extends the reach into *dynamics* —
**but only the deterministic, mean-field part of dynamics**, never the distributional part.

**What F CAN arbitrate (the deterministic existence/stability skeleton):**

- **Existence:** is the trivial fixed point `n≡0` the global attractor? (extinction) — generalising
  viability's `F ≤ B` two-number gate to "leading eigenvalue ≤ 1 everywhere."
- **Boundedness:** does per-capita net growth turn negative at high density, or stay positive?
  (population explosion) — read off the density-dependence of the selection diagonal.
- **Local stability of the non-trivial attractor:** node vs. limit cycle, via the Jacobian spectrum —
  the **frozen↔oscillation Hopf** crossing.
- **Multiplicity / branching of the stationary trait distribution:** single- vs. multi-peak via
  invasion-fitness sign — the **monoculture↔coexistence** direction (with generalist dominance as the
  diagonal-vs-axis special case, provisionally — its fragility/incompatibility forces are mortality-
  and morphology-mediated and smooth poorly, so F's verdict here is the weakest).
- **Flux balance of the closed nutrient cycle:** the **nutrient-lockup** stationary carcass fraction.

**What F CANNOT arbitrate (and must never claim):**

- **The sporadic-per-seed decomposer guild.** `expected-properties.md` is explicit: decomposer
  emergence is *"distributional: confirmed across seed ensembles, sporadic per-seed, never guaranteed
  on a single run — the expected signature of an emergent role."* A behavioural role read from θ
  (CONTEXT.md, *Decomposition*: "a matter of behaviour and circumstance, not a heritable niche")
  appears in a *fraction* of seeds. Mean-field replaces the reproduction/movement *draws* with their
  *expectations* — it computes the *average* world and erases exactly the rare-event, single-seed
  structure in which a guild does or does not nucleate. **F can say whether the average world locks
  up; it cannot say a decomposer guild appears in 71/120 worlds.** That number is genesis search's
  to report (and it does: scenarios/README, "71/120 viable random worlds produced decomposers").
- **Demographic stochasticity near extinction.** A mean field cannot see a small population die out
  by chance fluctuation when its *expected* growth is positive — the deterministic operator predicts
  persistence, the stochastic individual world goes extinct. So F's extinction verdict is *necessary
  (mean-field-dead ⇒ dead) but not sufficient* — the same conservative asymmetry viability's existence
  gates already carry, now inherited explicitly.
- **Per-seed realisation of any quantity** the validation triad reports as an *ensemble distribution*
  (scenarios/README: `observed.json` is "an ensemble distribution, not a single seed (#314)"). F
  predicts the distribution's *deterministic skeleton* (where the attractor is), never its *spread*.

**One-line boundary:** F is the arbiter of *existence and stability*; it is **never** the arbiter of
the *distributional* modes. Where brief A *preserves* per-seed distributional emergence (it runs the
real individual physics, just faster), **F deliberately erases it** — that erasure is the price of
the closed-form spectrum and the precise edge of F's authority. The two are complementary by exactly
this asymmetry: A is the safe-but-expensive distributional witness; F is the cheap-but-blind
deterministic skeleton. Disagreement between them is *diagnostic*, never a defect in either (AC5).

## AC5 — Falsifiability on a `scenarios/` example (validation-triad cross-check)

A predicted bifurcation must be made falsifiable on the one shared object the validation triad turns
on — the **example file** (`viability.md`, *Place in the validation triad*; scenarios/README). The
mechanism is: F emits a *prediction* on a scenario's initial condition; the headless run *observes*
the outcome on the *same* file; genesis *locates* it. The scenario `metadata` block already carries
`prediction` (`live`/`dead`/`undecided`) and `probes` (the failure mode) — F's prediction slots
straight into that half, and `observed.json` (computed by the same evaluator genesis uses) is the
other half.

Concretely, two cross-checks are writable today against existing files:

1. **Nutrient-lockup prediction on `example9_detrital_pathway.json`.** F's flux balance (AC4) predicts
   the *field* locks up because the producer ring's carcass rain is out of the decomposer's reach
   (`decomp_turnover → 0` for the field, `C* →` the full death flux). The file's **observed verdict is
   `nutrient_lockup` (8/8)** (scenarios/README, #342). **Agreement → confidence; disagreement →
   localises the fault**, exactly as viability.md prescribes: if F said "lockup" and the run showed a
   healthy pool, the operator's decomposition-reach geometry is wrong; if F said "fine" and the run
   locked up, F's `decomp_turnover` overestimates decomposer reach. This is the *cheapest* place to
   test the affirmative lockup theory and it has a ready-made ground truth.

2. **A Hopf prediction on a new minimal predator-prey scenario.** Author a small two-cluster file
   (a producer cluster + a consumer cluster within trophic-transfer range) and have F compute the
   2-block Jacobian's leading eigenvalue pair as a *named world parameter is swept* (e.g.
   `trophic_distance_decay` or `base_trophic_efficiency`, the kernel coefficients). F predicts the
   sweep value at which the pair crosses the unit circle — a **falsifiable bifurcation point**: below
   it the headless run should freeze to a fixed population, above it it should oscillate (the
   `oscillation` score in `observed.json`, computed by the same evaluator). The example carries F's
   predicted crossing value in `metadata.rationale`; the run is swept across it; agreement confirms
   the Hopf reading, a shifted crossing localises the spatial in-reach geometry (the binary-reach
   drain carries no contact-duration term to mis-model, #380). This is the canonical "predicted bifurcation made falsifiable
   on a single example" the AC asks for, and it reuses the existing `--scenario … --trace` headless
   harness and `eval_scenarios` evidence pipeline with no new machinery.

In both, F is held to the *existence/stability* claim only — neither cross-check asks F to predict a
per-seed decomposer guild, respecting AC3.

## AC5 — Recommended next move

**Is there something actionable? Yes — and it is a theory-then-thin-build sequence, sharply scoped by
the central-question finding so it does not over-reach into a 7-dim solver.**

1. **Ready to write now — the nutrient-lockup flux-balance theory note (theory issue).** AC4 is the
   highest-value, lowest-risk piece: it closes a gap `viability.md` *explicitly names as open*, every
   flux is already committed, and it has a ready cross-check (`example9`, 8/8 lockup). The follow-up
   issue is a **system-design pass** (route through `/grill-with-docs`) that promotes the
   three-compartment `A⇌L⇌C` flux balance and the `C* = death_flux / decomp_turnover` stationary
   condition into `viability.md` as the **affirmative nutrient-lockup gate** — the dynamical sibling
   of the static `N_total` floor. This is a *docs/theory* deliverable, no code, and it is the natural
   first reorientation of viability "toward the affirmative" that the doc's *Direction of travel*
   section calls for. → theory issue, `ready-for-agent`.

2. **Ready to spike next — a scalar/low-block compartment prototype to validate the Hopf reading
   (thin build).** A small, *throwaway* (`explorers-app`-adjacent or a standalone analysis bin)
   integrator of the 2-4 compartment ODE + 2-block Jacobian, validated against a vectorised (brief A)
   or existing headless rollout on the **predator-prey scenario of AC5 cross-check 2**. Its only job
   is to confirm that the predicted unit-circle crossing matches the observed freeze→oscillate
   transition on one example — the validation-triad meeting point. It is *not* a genesis objective
   yet and *not* the 7-dim solver. Scoped this way it is low-risk and decides whether the operator
   reading is trustworthy before any larger investment. → build issue, blocked on (1)'s flux-balance
   note for the lockup compartment definitions.

3. **Deferred until (2) validates — moment-closure branching detector as a genesis *objective*.** The
   monoculture↔coexistence invasion-fitness sign (the only distribution-shaped mode) is the richest
   prize but the one that touches the most uncertain modelling (Gaussian closure blind exactly at the
   branching point; generalist-dominance fragility smooths poorly). It should follow only once the
   scalar/Hopf skeleton is trusted, and it is where F's output could become the *distance-to-
   bifurcation* objective the epic's interlock describes (F sharpens the objective → A makes it cheap
   → B/QD maps it → E amortises it). Explicitly **not** agent-ready until (1) and (2) land. → recorded
   as a follow-up, not a current issue.

**The go/no-go gate:** promote the affirmative lockup gate (1) *only if* its prediction matches
`example9`'s observed `nutrient_lockup`; build the compartment prototype (2) *only* to test the Hopf
crossing on one example; and **gate every step on the authority boundary** — at no point does F
claim a per-seed distributional verdict. If F's existence/stability skeleton and A/B's observed
boundary ever disagree, that disagreement is the *diagnostic signal the validation triad prizes*, not
a failure of F — it localises whether the missing physics is in the operator term, the gate, or the
rule.
