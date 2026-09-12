# Issue #437 — formal viability A2: permanence of the coupled energy × nutrient (A ⇌ L ⇌ C) reduction, and the lockup repeller

**Status: research finding. Commits nothing.** This note couples the producer↔consumer
energy map of [`432-permanence-pc.md`](432-permanence-pc.md) (A1) to the three-compartment
nutrient cycle that [`viability.md`](../system-design/viability.md) characterises as a flux
balance (`A ⇌ L ⇌ C`, `C* = death_flux / decomp_turnover`), and moves that characterisation
from a *stationarity* statement to a *persistence* one: under what committed-parameter
conditions does living nutrient `L` stay bounded away from zero? It identifies the
**nutrient-lockup boundary** (`L → 0` while `C → N_total`) as a corner of a line of dead
fixed points, states its **repeller condition**, and says exactly which terms of that
condition are endogenous — and therefore which part is a characterisation and which a
gate. It introduces no mechanism, functional form, or parameter; every symbol is a
committed world parameter, a per-cluster trait, or a named lumping of a committed flow. It
introduces **no bound on decomposer reach or carcass richness**. It touches nothing in
`docs/system-design/`, and nothing in the stepper, evaluator, or search.

The numerics extend the throwaway bin
`crates/explorers-sim/src/bin/permanence_prototype.rs` (same **NOT PART OF THE PRODUCTION
STEPPER** banner; the A1 map is reused verbatim, not re-derived).

## TL;DR

With five named stocks — producer structure `P`, consumer structure `C_E` (A1's energy map)
and available `A`, living `L`, carcass-locked `C_N` nutrient (`A + L + C_N = N_total`, four
independent) — coupled through the committed Liebig co-limitation of growth and the
binary-reach drain run over carcasses:

1. **The dead set is a line, not a point.** Every state with `P = C_E = 0` is a fixed point
   (no passive decay), parameterised by how the nutrient splits between `A` and `C_N`. Its
   two ends are the **virgin end** `(0, 0, N_total, 0)` and the **lockup corner**
   `(0, 0, 0, N_total)`. The producer-only face flows into the lockup corner (every death
   strands nutrient, nothing returns it); the consumer-only face flows back toward the
   virgin end (heterotrophs eat the pile and excrete it into `A`, then starve). The lockup
   boundary is therefore a **heteroclinic cycle** along the dead line.
2. **Producers cannot invade the lockup corner, whatever the flux.** Uptake is
   `min(α_P·P, A)` and `A = 0` there, so the Liebig minimum closes growth at zero:
   `λ_P(𝓛) = 1 − μ_P < 1`. Only a heterotroph feeding on the pile can.
3. **Lockup repeller condition.** The corner repels iff the heterotroph's per-tick
   conversion on the whole carcass pile beats its maintenance floor:
   > **`Λ = σ · N_total · min(κ_C·γ·e_C, q/θ_C) / B_C > 1`**, with `σ = h_C·ι/ν`
   — `h_C` the consumer's effective heterotrophy (binary-reach drain per target), `ι` the
   in-reach geometry, `ν` nutrient per carcass (so `N_total/ν` is the carcass count),
   `e_C = base_trophic_efficiency·exp(−trophic_distance_decay·d)` at the consumer↔carcass
   distance, `q` carcass richness (nutrient per unit carcass energy), `θ_C` the consumer's
   stoichiometric demand, `B_C` its maintenance. `Λ` is the carcass-pile twin of A1's
   invasion ratio `I = β·K_P/m`: `K_P` (the producer standing crop) becomes `N_total/ν` (the
   carcass standing pile) and the kernel acquires a Liebig branch in carcass richness.
4. **Persistence of `L`** (uniform persistence in `(P, C_E)`, which bounds
   `L ≥ θ_P·P + θ_C·C_E` away from zero) holds when (i) the producer invades the virgin
   pool, `min(r_P, α_P/θ_P) > μ_P`; (ii) `Λ > 1`; and (iii) the dead-line cycle repels,
   `ln λ_P(virgin) · ln λ_C(𝓛) > ln(1/(1−B_C)) · ln(1/(1−μ_P))`. (i) and (ii) are necessary;
   the product (iii) is what a common average-Lyapunov weight needs and is the classical
   heteroclinic-cycle criterion. Boundary-escape numerics agree with the analytic frontier
   on 239/239 scored cells of a 25×25 sweep (0 disagreements), the frontier being the
   hyperbola `Λ = 1` (`base_trophic_efficiency · N_total = const`).
5. **Which part is a gate.** `Λ` contains `ι`, `ν`, `q`, `e_C`'s distance `d`, and the
   cluster's `h_C, κ_C, θ_C, B_C` — all endogenous or per-cluster. The only pure-parameter
   kills are `base_trophic_efficiency = 0` (then `Λ = 0` for every value of the endogenous
   terms) and the no-heterotroph/no-reach corner (`h_C·ι = 0`): exactly the degenerate
   corners `viability.md`'s decomposer-return finding already names. Everything else is a
   characterisation. **No bound on reach or richness is introduced here.** If the design
   ever commits `ι ≤ ῑ` and a floor `ν ≥ ν̲` on nutrient per carcass, `Λ ≤ Λ̄` becomes a
   config-plus-trait-bound gate; that is the contingency `viability.md` holds in reserve,
   and this note stops there.
6. **The `N_total` energy-death floor is recovered** in the `C_N = 0, A → 0` limit, where
   `L → N_total` and the embodiment-plus-reproduction-gate floor `L ≥ S_min·θ_min + N_repro`
   reads `N_total ≥ S_min·θ_min + N_repro` verbatim; away from that limit the same
   inequality carries `+ A* + C_N*`, the "only raise the real floor" clause.
7. **Two sharpenings of the existing characterisation**, recorded without changing it:
   `decomp_turnover` does *not* contain the trophic kernel (the binary-reach drain removes
   carcass structure at `h_C` per target whether or not the consumer retains it; the kernel
   sits in the decomposer's *viability* `Λ`, not in the per-unit clearance); and a carcass
   created with zero energy but positive nutrient (an agent drained to exactly zero
   structure) is skipped by the drain pass for ever — a genuine irreversible sink in the
   committed stepper that the reduction cannot see (follow-up, not fixed here).

## Reading list this note assumes

A1's map, ratios `ρ = F/B_P`, `I = β·K_P/m`, authority boundary and example10
counter-example ([`432-permanence-pc.md`](432-permanence-pc.md)); `viability.md`'s energy-death
gate, "no clean gate" finding and flux-balance characterisation; Brief F's TL;DR items 1, 2,
4 ([`F-mean-field-operator.md`](F-mean-field-operator.md)); world rules' nutrient stocks,
pools, conservation, flows 2 (uptake), 3 (consumption, on living and dead targets), 6
(death), 7 (kernel), 9 (growth, Liebig); and [`433-energy-bound.md`](433-energy-bound.md)
(no living-energy bound exists — see *What #433 does and does not change* below).

## The coupled reduction

### Stocks

| symbol | what | units | committed source |
|---|---|---|---|
| `P` | producer structure | reference bodies (`M_ref = 1`) | A1 |
| `C_E` | consumer structure | reference bodies | A1 |
| `A` | available pool | nutrient | world rules, *Nutrient pools* |
| `L` | living nutrient = bound `θ_P·P + θ_C·C_E` + free store + earmark `Φ` | nutrient | *Living agents* pool; CONTEXT.md **Nutrient**, **Structure** |
| `C_N` | carcass-locked nutrient | nutrient | *Carcasses* pool |

with `A + L + C_N = N_total` every tick (nutrient conservation; the unavailable pool is
geological and constant, so it is dropped from the ledger). Embodiment gives `L ≥ θ_P·P +
θ_C·C_E`, `θ_X = base_nutrient_ratio + specification_nutrient_coefficient · σ_X` the
cluster's stoichiometric demand per unit structure (`stoichiometric_demand` at unit
structure). The bin carries all five explicitly and checks the ledger rather than defining
one stock as the difference of the others — near the lockup corner `N_total − A − C_N`
loses every digit.

### Per-tick fluxes, each traced

```
U    = min(α_P·P, A)                                      uptake            flow 2, absorb_nutrients
G    = min( r_P·P·(1 − P/K_P)⁺ , (Φ + U)/θ_P )             producer growth   flows 1, 8, 9 (Liebig); A1 face
S    = r_P·P·(P/K_P − 1)⁺                                   shading loss      A1's logistic decline branch
D_L  = h_C·P·C_E                                            living drain      flow 3, A1's bilinear term
D_C  = min( σ·C_E·C_N , C_N/q )                             carcass drain     flow 3 on carcass targets, σ = h_C·ι/ν
ΔC   = min( β·P·C_E + κ_C·γ·e_C·D_C , (θ_P·D_L + q·D_C)/θ_C ) consumer growth  flows 3, 7, 9 (Liebig)
X    = θ_P·D_L + q·D_C − θ_C·ΔC                             excretion → A     flow 3, stoichiometric mismatch
M_P  = μ_P·P + S                                            producer deaths   flow 6 (wear; shading), check_death_thresholds
M_C  = B_C·C_E                                              consumer deaths   A1's removal diagonal (metabolise → death)

P'   = P + G − D_L − M_P
C_E' = C_E + ΔC − M_C
A'   = A − U + X
C_N' = C_N − q·D_C + θ_P·M_P + (M_P/P)·Φ + θ_C·M_C
L'   = N_total − A' − C_N'        (identically θ_P·P' + θ_C·C_E' + Φ')
```

Term by term:

- **Uptake `U`.** `absorb_nutrients` charges each producer a per-body demand equal to its
  effective autotrophy `α_P` and shares the cell pool proportionally when demand exceeds
  it. Well-mixed, that is `min(α_P·P, A)` — linear in `A` below the kink, flat above. The
  whole uptake enters `L` (the κ share as free store, the `1 − κ` share as reproductive
  earmark — both living nutrient).
- **Producer growth `G` (Liebig).** The energy-limited increment is A1's logistic growth
  branch `r_P·P·(1 − P/K_P)` (photosynthesise + metabolise + grow; `r_P = κ_P·γ·(F − B_P)`,
  `K_P = F/B_P`). The nutrient-limited increment is what the *bindable* free store supports
  at demand `θ_P`: `(Φ + U)/θ_P`. The stepper's `grow` takes the smaller (`to_structure =
  energy_limited.min(nutrient_limited)`), and energy that cannot be matched stays in
  reserve rather than burning. Two lumpings, both stated: the free store is pooled across
  the living (the mean field does not resolve which lineage holds it), and the reproductive
  earmark is treated as bindable — the mean field reads reproduction as continuous biomass
  growth, exactly as A1 does in dropping the reproduction thresholds. A stricter same-tick
  reading binds only the κ_P share, replacing `α_P/θ_P` by `κ_P·α_P/θ_P` in the virgin-end
  clause; it does not touch the lockup corner, where this branch is zero either way.
- **Living drain `D_L`.** A1's bilinear term: each consumer body drains each in-reach
  producer at its effective heterotrophy (`resolve_drains`, binary reach, #380). The
  nutrient *bound in the drained structure*, `θ_P·D_L`, travels with it; the target's free
  store and earmark do not.
- **Carcass drain `D_C`.** The *same* pass, run over carcass targets: each consumer body
  drains each in-reach carcass at `h_C` per tick, capped at the carcass's energy. With
  `n = C_N/ν` carcasses in the pile and a fraction `ι` of them in reach, the well-mixed
  drain is `h_C·ι·n·C_E = σ·C_E·C_N`, capped at the pile's energy `C_N/q`. Carcass nutrient
  transfers in proportion to energy drained (`nutrient_fraction = actual_drain / available`),
  so `q·D_C` leaves `C_N`. This is CONTEXT.md's *Decomposition*: consumption where the
  target is dead, no separate capability — which is also why there is **no separate
  decomposer compartment** (below).
- **Consumer growth `ΔC` (Liebig).** Energy-limited: A1's `β·P·C_E` on living prey plus
  `κ_C·γ·e_C·D_C` on carcasses (grow phase × the committed kernel at the consumer↔carcass
  distance). Nutrient-limited: the consumer retains at most its stoichiometric need and
  binds at `θ_C` per unit structure, so what arrives with the drained structure,
  `θ_P·D_L + q·D_C`, supports at most that over `θ_C`.
- **Excretion `X`.** The remainder is excreted at the feeding site (stoichiometric
  mismatch; `nutrient_grid.at_position(...) += excreted` in both drain branches). It goes
  to `A` directly: this is the `C_N → A` route of the characterisation, and the `L → A`
  route (grazing) that Brief F lists as "excretion".
- **Deaths `M_P`, `M_C`.** A consumer body that cannot out-ingest its floor `B_C` declines
  to the death threshold — A1's removal diagonal, here read as the death flux it is: `B_C`
  per body per tick, carrying `θ_C` per body into `C_N` (flow 6: the carcass keeps the
  agent's whole nutrient). Producer deaths are `μ_P·P` (wear-driven intrinsic mortality —
  "somatic wear makes death inevitable" — plus the below-threshold residue of drain kills)
  and the shading branch `S` of A1's logistic; each carries its bound `θ_P` and its share of
  the pooled free store. `μ_P` is a **lumped, endogenous** per-tick rate: A1 nets it inside
  `r_P` (A1's face has no explicit death), so with `μ_P → 0` the energy side is A1 exactly.

### Why five stocks and no decomposer compartment

The orchestrating question was whether decomposer mass should be a fraction of consumer
mass or its own compartment. The committed rules answer it: decomposition *is*
consumption with a dead target, through the same heterotrophy trait, the same reach, the
same drain, the same kernel — a decomposer "is an agent that, in practice, drains carcasses
rather than living prey… a matter of behaviour and circumstance, not a heritable niche"
(CONTEXT.md). In the mean field a consumer body drains *every* target in reach, living or
dead, so the split of its feeding between prey and pile is not a parameter but the ratio of
the two standing stocks `P : ι·C_N/ν`. "Decomposer mass" is therefore the consumer
compartment itself, weighted by that ratio, and the reduction stays at five named stocks
with nothing folded away by fiat. Where a scenario carries a distinct sessile
detritivore guild (example13) the same map applies with that guild's `h_C, κ_C, θ_C, B_C`
and its own distance `d` to the carcass pile — a second consumer compartment would only be
needed to ask *coexistence* questions between guilds, which is the monoculture↔coexistence
mode, not this one.

### The endogenous lumpings, named

| symbol | meaning | why it is endogenous |
|---|---|---|
| `ι` | fraction of the pile within a consumer body's feeding reach | `consumption_reach = h_C·(contact_range_coefficient + body_reach_coefficient·√structure)` — a committed *form* whose magnitude is per-seed (`structure` emergent, coefficients searched); carcass placement emergent |
| `ν` | nutrient per carcass | the dead body's exact content (`nutrient_total` at death) |
| `q` | carcass richness, nutrient per unit carcass energy | `ν / structure-at-death`; ≥ `θ_P` for a producer carcass (bound plus whatever free store rode along) |
| `e_C` | kernel on the pile | `base·exp(−λ·d)` at the distance to *whichever* trait vectors died; for a producer-dominated pile `d` is A1's `d`, so `e_C = e` |
| `μ_P` | producer per-tick mortality | wear, shading, kill residue — realised, not set |

These are exactly the two terms the "no clean gate" finding names (reach, richness) plus
the carcass count and death rate they imply. The bin carries them as `AlcLumping` inputs so
they are visible; it never bounds them.

### What #433 does and does not change

#433 shows no bound on *living energy* exists: the reproductive allocation is uncapped.
This reduction carries structure `P, C_E`, not reserve, and its Liebig branch is the place
the unbounded stock would matter — energy that cannot be matched with nutrient accumulates
in reserve and could later fund a growth burst when nutrient returns. Dropping that stock
makes the reduction *conservative in the lockup direction*: stored energy can only help a
lineage escape once nutrient returns and can never lower `L`. On the nutrient side nothing
is lost — `L ≤ N_total` by conservation, so the reduction's `L` is bounded regardless.

## Boundary structure: the dead line and its heteroclinic cycle

Take `Ω = {P, C_E ≥ 0; A, Φ, C_N ≥ 0; A + L + C_N = N_total}`. The map is continuous on
`Ω`, keeps each face `{P = 0}`, `{C_E = 0}` invariant, and its dead set is

```
𝓓 = {P = 0, C_E = 0}  =  {(0, 0, A, N_total − A) : 0 ≤ A ≤ N_total}
```

**Every point of `𝓓` is a fixed point** — no living body means no uptake, no drain, no
death, and carcasses do not decay (world rules, *No passive decay*). Its ends:

- the **virgin end** `𝓥 = (0, 0, N_total, 0)`: all nutrient available, nothing dead;
- the **lockup corner** `𝓛 = (0, 0, 0, N_total)`: all nutrient in carcasses — the boundary
  the issue names, `L → 0` while `C → N_total`.

**Producer-only face `{C_E = 0}` → `𝓛`.** With no heterotroph, `C_N' − C_N = θ_P·M_P +
(M_P/P)·Φ ≥ θ_P·μ_P·P ≥ 0`: the pile only grows, and `Σ_t P_t < ∞`, so `P → 0`; the dying
fraction takes the free store with it, so `Φ → 0`; and a producer that clears the virgin
clause holds its crop until `A` is drawn down, so `A → 0`. The face's ω-limit is the lockup
corner (the bin's `producer_only_face_silts_into_the_lockup_corner` checks this at three
fluxes: `C_N → N_total` to `10⁻⁶`). This is world rules' "a world without decomposers
accumulates resources in the dead pool until the living system starves", as a limit — and
it is example9's field, whose carcass rain falls outside every decomposer's reach (`ι = 0`
for that pile).

**Consumer-only face `{P = 0}` → toward `𝓥`.** Heterotrophs on the pile drain it, bind
what they need, excrete the rest into `A`, and die back into `C_N` at `B_C` — but every pass
loses energy (`e_C, γ < 1`) and none is replaced, so the pile's energy, and with it (at
fixed richness) its nutrient, runs down; the excreted nutrient accumulates in `A`; the last
heterotroph starves. The face flows along `𝓓` toward the virgin end.

So the boundary ω-limit set is `𝓓` itself, and the lockup boundary is a **heteroclinic
cycle**: `𝓥 → (producer face) → 𝓛 → (consumer face) → 𝓥`. Transverse to `𝓓`, at a point with
available pool `A`:

```
λ_P(A) = 1 + min(r_P, α_P/θ_P) − μ_P     for A > 0    (uptake cap inactive for small P)
λ_P(0) = 1 − μ_P                                       (no pool: Liebig closes growth)
λ_C(A) = 1 + σ·(N_total − A)·min(κ_C·γ·e_C, q/θ_C) − B_C
```

`λ_P` is constant along the open line and drops at the corner; `λ_C` rises linearly toward
the corner. The bin's `lockup_corner_eigenvalues_match_finite_difference_jacobian` checks
the corner pair against a central-difference Jacobian of `step` (four parameter sets,
`10⁻⁶`), including that the off-diagonal entries vanish (the pair is diagonal, as in A1's
origin).

## Theorem and proof sketch — persistence of `L`

**Theorem.** Under A1's admissibility (`0 < B_C < 1`, `0 < μ_P < 1`, orbits in a compact
unsaturated set where neither drain hits its cap) the coupled map is permanent in
`(P, C_E)` — hence `lim inf L ≥ lim inf (θ_P·P + θ_C·C_E) > 0` — if

```
(i)   min(r_P, α_P/θ_P) > μ_P                                      producer invades 𝓥
(ii)  Λ := σ·N_total·min(κ_C·γ·e_C, q/θ_C) / B_C > 1                heterotroph invades 𝓛
(iii) ln λ_P(𝓥) · ln λ_C(𝓛)  >  ln(1/(1 − B_C)) · ln(1/(1 − μ_P))     the cycle repels
```

and (i), (ii) are each necessary.

**Sketch.** Same tool as A1 (Hutson's average Lyapunov function), `V = P^{a₁}·C_E^{a₂}`,
`Ψ = a₁·ln f + a₂·ln g` with `f = P'/P`, `g = C_E'/C_E`. The boundary ω-limit set is `𝓓`,
every point of which is fixed, so the condition is `Ψ > 0` pointwise along `𝓓` for one
common pair `(a₁, a₂)`:

- at `𝓥`: `a₁·ln λ_P(𝓥) + a₂·ln(1 − B_C) > 0` ⟺ `a₁/a₂ > ln(1/(1−B_C)) / ln λ_P(𝓥)`;
- at `𝓛`: `a₁·ln(1 − μ_P) + a₂·ln λ_C(𝓛) > 0` ⟺ `a₁/a₂ < ln λ_C(𝓛) / ln(1/(1−μ_P))`;
- at interior points of `𝓓` (`0 < A < N_total`): `Ψ(A) = a₁·ln λ_P(𝓥) + a₂·ln λ_C(A) ≥
  Ψ(𝓥) > 0`, since `λ_C(A) ≥ 1 − B_C`.

A common ratio exists iff the two bounds on `a₁/a₂` are ordered, which is (iii); (i) and
(ii) make the logs positive. Necessity: if (ii) fails the corner's transverse Jacobian
`diag(1 − μ_P, λ_C(𝓛))` has both entries below 1, so interior points near `𝓛` converge to
`𝓓` (the neutral direction along `𝓓` only moves them to a nearby dead point); if (i) fails
the same happens at `𝓥`, where consumers starve at `1 − B_C` and producers cannot grow.

*Continuity caveat.* The Liebig `min` makes `f` non-differentiable, and at the corner the
ratio `U/P = min(α_P, A/P)` has no limit as `(P, A) → 0`. The argument uses the
direction-independent lower bound `f ≥ 1 − μ_P − h_C·C_E` at `A = 0`, which is what the
corner eigenvalue quotes; a continuous minorant of `Ψ` with positive averages suffices for
the theorem. The regime hypothesis is A1's H2 with the carcass cap added: a consumer mass
above `1/(σ·q)` drains the whole pile in one tick, which is the stepper's one-shot
annihilation and where the mass-action reading stops (the `x` cells below).

*On (iii).* This is the classical criterion for a heteroclinic cycle between two saddles to
be repelling — the product of the expanding rates exceeds the product of the contracting
rates — surfacing here because two *different* compartments do the escaping at the two
ends: the producer at `𝓥`, the heterotroph at `𝓛`. It is not an artefact of the Lyapunov
family: an orbit can shuttle along `𝓓` (producers silt the pool → heterotrophs unlock it →
producers regrow → …) and whether that shuttle spirals in to `𝓓` or out of it is exactly
the balance (iii) states. In every committed example examined (iii) holds by a wide margin
(example10: `7.5 > 0.002`), because `μ_P` and `B_C` are small per tick; it would bind only
for a world whose heterotroph barely clears `Λ = 1` while its producers die fast.

## The lockup repeller condition, in committed symbols

```
Λ  =  (h_C·ι/ν) · N_total · min( κ_C·γ·base·exp(−λ·d_C) ,  q/θ_C ) / B_C
   =  σ·N_total·min(κ_C·γ·e_C, q/θ_C) / B_C                              > 1
```

Reading left to right: (somatic allocation) × (per-target drain) × (reach geometry) ×
(carcasses in the pile) × Liebig-min of (energy conversion through the committed kernel)
and (carcass richness over own demand), over (maintenance floor). Compare `viability.md`'s
decomposer-return floor, `base_trophic_efficiency · (reachable carcass energy per tick) ≥
base_metabolic_rate + maintenance(decomposer body)`: `Λ > 1` is that inequality with its
terms named — the reachable carcass energy per tick is `h_C·ι·N_total/ν` (binary reach:
every in-reach carcass drained at `h_C`, the cap inactive against a full pile), the return
carries `κ_C·γ` and the distance kernel, and the nutrient branch is added.

Two things the form makes visible:

- **`Λ` is `I` with the pile in place of the crop.** A1's `I = κ_C·γ·e·h_C·K_P/B_C` is the
  consumer's conversion at the producer standing crop `K_P` bodies; `Λ` is the same
  conversion at the carcass standing pile `N_total/ν` bodies, with `ι` explicit (A1 sets
  the living-prey indicator to 1 — the same lumping, silently) and the kernel Liebig-capped
  by richness. For a producer-dominated pile `e_C = e` (a carcass keeps its trait vector),
  so `Λ/I = ι·(N_total/ν)/K_P · min(1, q/(κ_C·γ·e·θ_C))`: the pile-to-crop ratio.
- **Carcass *count*, not carcass mass, is what the drain multiplies.** The committed
  binary-reach drain is per target (#380), so at fixed `N_total` many small carcasses are
  cleared faster than few large ones (`ν` small ⟹ `Λ` large). Since the death threshold is
  peak-relative (#433, R12) there is no floor on how small a carcass can be — which is why
  a *lower* bound `ν̲` would be needed for a gate, not just an upper bound on richness.

## Recovery of the `N_total` energy-death floor in the `C_N = 0, A → 0` limit

The energy-death gate (`viability.md`) says a persisting lineage needs one agent both
embodied and reproduction-ready: `N_total ≥ S_min·(base_nutrient_ratio + spec_coeff·σ_min)
+ N_repro_threshold`. In the reduction that requirement sits on the *living* stock: with
`θ_min·S_min` the nutrient bound in the smallest embodied body and `N_repro` the earmark
the reproduction nutrient gate demands,

```
L  ≥  L_floor := θ_min·S_min + N_repro          (embodiment + reproduction nutrient gate)
L  =  N_total − A − C_N                          (conservation)
⟹  N_total  ≥  L_floor + A + C_N
```

In the limit `C_N = 0` (no nutrient stranded — infinite turnover) and `A → 0` (all of it
taken up), `L → N_total` and the inequality reads **`N_total ≥ L_floor`** — the committed
gate, verbatim. The reduction also says what that limit *is* dynamically: with `A = 0`,
`U = min(α_P·P, 0) = 0`, so `G = min(r_P·P·(1 − P/K_P)⁺, Φ/θ_P)` — the producer can bind only
the free store it already holds, and once that is gone growth is zero **for every solar
flux `F`**; the living state is frozen at the nutrient it holds, and the only question left
is the static one on `L`. That is "energy abundance cannot rescue a nutrient-starved
world", read off the map. The bin's `nutrient_starved_face_freezes_growth_regardless_of_flux`
checks `P' ≤ (1 − μ_P)·P` at `F ∈ {1, 10, 10³, 10⁶}` on the `A = Φ = C_N = 0` face.

Away from the limit the floor carries `+ A* + C_N*`: an interior fixed point needs `A* > 0`
(uptake `min(α_P·P*, A*)` must balance deaths `θ_P·μ_P·P*`) and `C_N* > 0` (death is
continuous, turnover finite — the *partial closure* finding), so both terms only raise the
real floor, as the gate says. Below `L_floor` the mean-field biomass has no realisation as
a reproducing lineage: births are zero, `P' ≤ (1 − μ_P)·P → 0`, `L → C_N`, non-permanent —
for every `F`, `h_C`, `e`.

## Stationarity recovered, and sharpened

At an interior fixed point with the carcass cap inactive, `C_N' = C_N` gives

```
q·σ·C_E*·C_N*  =  θ_P·M_P* + (M_P*/P*)·Φ* + θ_C·M_C*
C_N*  =  death_flux / decomp_turnover,     decomp_turnover = q·σ·C_E* = h_C·ι·C_E* / S_d
```

(`S_d = ν/q` the carcass's structure at death). This is `viability.md`'s
`C* = death_flux / decomp_turnover` with the turnover written out: decomposer mass × drain
per target × reach ÷ carcass body energy. **The trophic kernel is not in it.** The
characterisation's parenthetical — turnover = (decomposer mass) · (kernel) · (reach) — puts
`base·exp(−λ·d)` in the clearance, but the committed drain removes carcass structure at
`h_C` per in-reach target whether the consumer retains it or excretes it; the kernel
governs what the consumer *gains*, i.e. whether `C_E* > 0` at all. So the kernel lives in
`Λ`, the decomposer's viability, and the stationarity statement has the same authority
note as before with one term relocated. The persistence statement is what sits on top:
`C_N*` exists iff `C_E* > 0`, and `C_E* > 0` is what (i)–(iii) certify.

## Gate versus characterisation

The issue asks for this to be explicit; here it is, term by term of `Λ > 1`:

| term | status | consequence |
|---|---|---|
| `base_trophic_efficiency`, `trophic_distance_decay`, `growth_efficiency`, `N_total`, `base_metabolic_rate` | world parameters | enter `Λ` monotonically; can be swept |
| `h_C`, `κ_C`, `θ_C`, `B_C` | per-cluster traits (as A1's `I`) | a gate would need a "best heterotroph" bound over the trait box |
| `d_C` (distance to the pile's trait vectors) | per-cluster and per-history | as A1's `d` |
| `ι` (reach geometry) | **endogenous** — committed form, emergent magnitude | *characterisation* |
| `ν`, `q` (carcass count, richness) | **endogenous** — the dead bodies' content | *characterisation* |

**Universal kills (gate-grade).** `base_trophic_efficiency = 0` ⟹ `e_C = 0` ⟹ `Λ = 0`
whatever `ι, ν, q, N_total` and the traits (the bin's
`zero_trophic_efficiency_makes_lockup_certain_for_every_lumping` checks three lumpings,
`N_total` up to `10⁶`); and `h_C·ι = 0` (no heterotroph, or none that ever contacts a
carcass) ⟹ `σ = 0` ⟹ `Λ = 0`. These are the "degenerate corners" the existing finding
names, and nothing here widens them.

**Everything else is a characterisation**, for the reason the existing finding gives:
carcass richness and decomposer reach are set by the realised economy, and `Λ` scales with
`N_total/ν` and `q` without a ceiling — a rich enough, dense enough, in-reach pile makes any
heterotroph viable on it. The condition states whether the *average* world's pile repels a
heterotroph inoculum; it cannot say which seeds nucleate one.

**What would make it a gate, and where this note stops.** If the design committed an upper
bound `ῑ` on achievable reach fraction and a lower bound `ν̲` on nutrient per carcass (the
smallest body that can die — which the peak-relative threshold does not currently supply,
#433), then `Λ ≤ Λ̄ := (h̄_C·ῑ/ν̲)·N_total·min(κ̄_C·γ·base, q̄/θ̲_C)/B̲_C` with the trait
bounds from the search box, and **`Λ̄ ≤ 1` would be a search-box gate** — decidable before a
run, with the same "gated dead that survives a rollout localises a mis-drawn bound"
falsifiability as the existing prefilter. That is precisely the reserve remedy
`viability.md` holds ("a bound on achievable carcass richness or decomposer reach — a knob
it does not currently spend"). **This note introduces no such bound**, and recommends none:
the contingency is to be spent only if genesis finds lockup where the committed parameters
say a heterotroph should be viable — the posture the design already takes.

## Numerics

`AlcMap` in the bin iterates the map above from a point hugging the lockup corner (`P` at
`10⁻³` and `C_E` at `10⁻⁶` of the nutrient scale — and, for `C_E`, of the pile-cap scale
`1/(σ·q)` — with all remaining nutrient in the pile), and reads `lim inf L > 0` as A1 does:
the trough of the bound nutrient `θ_P·P + θ_C·C_E ≤ L` over the last quarter of a
20 000-tick horizon within a factor 10 of its trough over the third quarter and above an
absolute floor. A tick on which either drain hits its cap is reported as having left the
regime. Sweeping `N_total` (25 log-spaced rows, 0.1 to 1000) against
`base_trophic_efficiency` (25 columns), every other committed parameter of example10
fixed and the lumpings at their defaults (`μ_P = 0.02`, `ι = 1`, producer carcasses with no
free store so `q = ν = θ_P`):

```
     N_total  Lam@eff=1     |    |    |    |    |
       0.100      0.203 .........................
       0.147      0.298 .........................
       0.215      0.438 .........................
       0.316      0.643 .........................
       0.464      0.944 .........................
       0.681      1.385 .................?#######
       1.000      2.033 ............#############
       1.468      2.985 ........##############xxx
       2.154      4.381 .....########xxxxxxxxxxxx
       3.162      6.430 ...#####xxxxxxxxxxxxxxxxx
       4.642      9.438 ..###xxxxxxxxxxxxxxxxxxxx
       6.813     13.853 .##xxxxxxxxxxxxxxxxxxxxxx
      10.000     20.333 .##xxxxxxxxxxxxxxxxxxxxxx
      14.678     29.845 ###xxxxxxxxxxxxxxxxxxxxxx
      21.544     43.807 ##xxxxxxxxxxxxxxxxxxxxxxx
      31.623     64.300 ##xxxxxxxxxxxxxxxxxxxxxxx
      46.416     94.380 ##xxxxxxxxxxxxxxxxxxxxxxx
      68.129    138.530 #xxxxxxxxxxxxxxxxxxxxxxxx
     100.000    203.335 #xxxxxxxxxxxxxxxxxxxxxxxx
     146.780    298.455 xxxxxxxxxxxxxxxxxxxxxxxxx
       ...            ...  (all x to N_total = 1000)
  cols: eff = 0.04 .. 1.00 in steps of 0.04; '|' marks 0.2, 0.4, 0.6, 0.8, 1.0
  '#' persistent   '.' not persistent   'x' left the unsaturated regime   '!'/'?' disagreement
  agree = 239, disagree = 0, within 2% of Lambda = 1 (not scored) = 1, left regime = 385
```

- The `.`/`#` frontier is the hyperbola `Λ = 1`, i.e. `base_trophic_efficiency · N_total =
  const` — the pile-side twin of A1's `I = 1` hyperbola in `(ρ, base)`. The single `?` at
  `N_total = 0.68, eff = 0.72` is within the 2 % band (`Λ = 1.00`). The unit test
  `living_nutrient_persists_iff_the_coupled_condition_holds` runs the efficiency sweep at
  `N_total = 2` with 0 disagreements allowed (0.4 s for the whole bin).
- **The `x` region is the "no space" clause made visible.** With `ι = 1` every carcass is
  in every consumer's reach, so at `N_total/ν` carcasses each drained at `h_C`, a consumer
  mass above `1/(σ·q)` (≈ 1.7 bodies here) drains the whole pile in one tick — the cap
  binds, the mass-action reading is no longer faithful, and the condition is silent. It
  grows with `N_total` exactly as A1's `x` region grows with `I`: the well-mixed mean field
  over-serves the heterotroph in proportion to the pile. Where the frontier *is* in regime
  it agrees with the analytic condition on every scored cell.
- **example10 read through the coupled reduction**: `N_total = 50 000`, `Λ ≈ 8·10⁴`,
  (i)–(iii) all hold, numerics leave the regime on tick one. The mean field says a
  heterotroph facing 250 000 carcasses in reach is trivially viable; the committed file's
  consumers die by tick 2 because their reach never meets the sparse crop (A1's standing
  counter-example). The gap localises to `ι` — which is the point of keeping it explicit.

Reproduce:

```
cargo run -p explorers-sim --bin permanence_prototype -- scenarios/example10_predator_prey_hopf.json
cargo test -p explorers-sim --bin permanence_prototype
```

## Side finding — an irreversible sink the reduction cannot see

`resolve_drains`' carcass pass skips any carcass with `energy <= 0.0`, and a carcass's
nutrient transfers in proportion to *energy* drained. An agent killed by a drain that takes
its structure to exactly zero (`total_demand >= available` in the living pass) becomes a
carcass with `energy = 0` and `nutrient = free store + earmark > 0`. That carcass is never
visited again: its nutrient is locked for ever, by construction, with no decomposer able to
reach it at any reach or richness. The reduction (fixed `q`) reads carcass energy and
nutrient as running out together and cannot represent this. It is small per event (the
victim's free store and earmark, never its bound nutrient) but strictly one-way and
cumulative over a long run — a genuine sink that would show as a slow, non-receding rise
in the carcass fraction on any predation-heavy world. Recorded here as a stepper
observation for a follow-up issue; not changed.

## Authority boundary

Inherited from A1 and Brief F's AC3, unchanged:

- **Mean field, deterministic, well-mixed.** The condition says the *average* world's
  lockup corner repels; `δ` is whatever the interior trough is, and a finite population at
  a trough of a few bodies dies by chance. Mean-field persistence is necessary, not
  sufficient, for individual-based persistence; non-persistence is the safe direction.
- **No space.** `ι` is set to 1 in the sweep and is the term that bit on the one example
  examined. The `x` region shows how far the well-mixed reading over-serves the pile.
- **No per-seed claim.** Whether a heterotroph guild nucleates on the pile on a given seed
  is the distributional question `expected-properties.md` reserves for genesis — the same
  boundary that makes `C*` a characterisation makes `Λ` one.
- **Unsaturated regime only.** Where a drain hits its cap the condition is silent.
- **One heterotroph compartment.** Guild coexistence on the pile is outside these
  coordinates.

## Deliverables against the acceptance criteria

- Research doc with the standard header; nothing added to `docs/system-design/` — this file.
- The coupled reduction written out with every term traced to a committed phase/flow —
  *The coupled reduction*, table and term-by-term notes.
- The condition recovers the `N_total` energy-death floor in the `C_N = 0, A → 0` limit —
  *Recovery of the `N_total` energy-death floor*, shown and unit-tested (Liebig freeze).
- The lockup repeller condition stated — `Λ > 1`, *TL;DR* item 3 and *The lockup repeller
  condition*; eigenvalue-checked in the bin.
- Which terms are endogenous, and which part is characterisation vs gate — *Gate versus
  characterisation*, term by term.
- No new bound on reach or richness — none introduced; the contingency is named and stopped at.
- No change to the stepper, evaluator, or search — only the banner-marked throwaway bin
  and its tests are touched.
