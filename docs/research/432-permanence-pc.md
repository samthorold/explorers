# Issue #432 — formal viability A1: permanence of the producer↔consumer mean-field map

**Status: research finding. Commits nothing.** This note derives a **permanence**
(uniform persistence) condition for the two-compartment producer↔consumer mean-field
map that [`F-hopf-validation.md`](F-hopf-validation.md) assembled from committed fluxes.
It introduces no mechanism, functional form, or parameter; every symbol below is a
committed world parameter or a lumping of one. It touches nothing in
`docs/system-design/` — the "Direction of travel" note in
[`viability.md`](../system-design/viability.md) asks for affirmative conditions, and this
is one candidate, recorded here so that it can be judged before anything is promoted.
It does not touch the stepper, the phases, the RNG, the evaluator, or the search.

The numerics live in a throwaway bin,
`crates/explorers-sim/src/bin/permanence_prototype.rs`, carrying the same
**NOT PART OF THE PRODUCTION STEPPER** banner as `hopf_prototype.rs`, whose map it reuses
verbatim.

## TL;DR

For the map

```
P' = P · f(P,C),   f = 1 + r_P·(1 − P/K_P) − a·C
C' = C · g(P,C),   g = 1 + β·P − m,             β = κ_C·γ·e·a,  e = base·exp(−λ·d)
```

the extinction boundary `{P = 0} ∪ {C = 0}` is a repeller — the system is **permanent** —
iff

> **(1)** `r_P > 0`  ⟺  `F > B_P`  (the producer face has a positive equilibrium; `r_P = χ_P·γ·(F − B_P)` with the biomass conversion `χ_P > 0` on every committed configuration short of a degenerate corner — see the coefficient table)
> **(2)** `β·K_P > m`  ⟺  `κ_C · γ · base_trophic_efficiency · exp(−trophic_distance_decay·d) · h_C · F  >  B_P · B_C`  (the consumer invades it)

in the committed parameters `F = solar_flux_magnitude`, `B_P`, `B_C` the producer's and
consumer's per-body maintenance (`base_metabolic_rate` plus the form-pinned trait and
structure maintenance), `γ = growth_efficiency`, `κ_C` the consumer's somatic allocation,
`h_C` its effective heterotrophy, `d` the trait-space distance producer↔consumer. Clause
(1) **is** the extinction gate `F ≤ B` of `viability.md`, sharpened by trait maintenance
(shown explicitly below). `κ_P`, the producer's somatic allocation, does **not** appear in
it: the map carries biomass, and the `(1 − κ_P)` share of the surplus becomes offspring
biomass through the committed reproductive branch — an earlier version of this note
lumped `r_P = κ_P·γ·(F − B_P)` and thereby acquired a spurious conjunct `κ_P > 0`, which
[`439-permanence-crosscheck.md`](439-permanence-crosscheck.md) (A3) caught on ten atlas
cells; the correction (#466) is the `χ_P` row of the table below. The Hopf line of #358
(`β·K_P/m = (1+m)/m`) lies strictly inside the permanent region, so the condition admits
the oscillating regime, as it must — permanence constrains the *boundary*, never the
interior attractor. Brute-force boundary-escape numerics agree with the analytic boundary
on 550/550 scored cells of a 25×25 sweep (tolerance stated below), with 0 disagreements.

## The map, and what is reused

The map is `hopf_prototype`'s, term for term — each already traced to a stepper phase in
[`F-hopf-validation.md`](F-hopf-validation.md) (growth diagonal: photosynthesise + metabolise
+ grow; bilinear trophic term: resolve_drains' binary-reach drain; kernel: the committed
`base_trophic_efficiency · exp(−trophic_distance_decay · d)`; removal diagonal: metabolise →
check_death_thresholds). Its coefficients are lumped from committed parameters exactly as
there:

| symbol | lumping | committed source |
|---|---|---|
| `r_P` | `χ_P · γ · (F − B_P)` | producer low-density net income → structure, by **both** of `κ_P`'s branches (below) |
| `K_P` | `F / B_P` | biomass at which the density-dependent light share meets maintenance |
| `a` | `h_C` (effective heterotrophy per reference body) | resolve_drains' per-consumer demand |
| `m` | `B_C` | consumer maintenance floor (metabolise) |
| `β` | `κ_C · γ · base · exp(−λ·d) · a` | grow phase × committed trophic kernel |

with `B_X = base_metabolic_rate + c_photo·α_X^p + c_het·h_X^p + c_mob·μ_X^p + c_struct·M_ref`
(the *form-pinned, coefficients-searched* tier of `viability.md`: the exponent `p` and the
`c_·` coefficients stay symbolic throughout). Reference body mass `M_ref = 1`, as in
`hopf_prototype`.

**The biomass conversion `χ_P` (the #466 correction).** The grow phase (flow 9) mobilises
a body's surplus `F − B_P` and splits it by `κ_P`: the `κ_P` share is converted to
structure at `γ` that tick, the `(1 − κ_P)` share is earmarked as reproductive allocation.
The map's coordinate is biomass, so the earmark is not a loss: at the next reproductive
event (flow 4, `phase::resolve_reproduction`) it becomes offspring, and the offspring's
body is structure the map carries. What the reproductive branch *does* cost, read off the
committed code, is:

- `reproduction_efficiency` — the flat per-event heat on the invested allocation;
- the dispersal propagule share `φ_P = clamp(c_disp · δ_P^{e_disp}, 0, 1)`
  (`dispersal_propagule_cost_fraction` on the producer's dispersal trait `δ_P`;
  `dispersal_propagule_cost_coefficient`, `_exponent`), spent before the budget is divided;
- the **empty brood**: the offspring count is a Poisson draw at `max(fecundity, 0.1)`, and a
  zero draw consumes the whole investment as heat, so a fraction `e^{−f_P}` of events
  provisions nobody;
- `offspring_structure_fraction = s` of each offspring's energy is embodied at birth
  through the same lossy `γ` as in-life growth (`provision_offspring`); the remaining
  `1 − s` is the offspring's reserve, which its own grow phase mobilises and splits by the
  same `κ_P` — the loop closes on itself.

Writing `η_P = reproduction_efficiency · (1 − φ_P) · (1 − e^{−f_P})` for the fraction of the
earmark that reaches an offspring's body, the structure (before `γ`) that one unit of surplus
eventually builds satisfies `χ_P = κ_P + (1 − κ_P)·η_P·(s + (1 − s)·χ_P)`, i.e.

```
χ_P = ( κ_P + (1 − κ_P)·η_P·s ) / ( 1 − (1 − κ_P)·η_P·(1 − s) ),      r_P = χ_P · γ · (F − B_P)
```

`γ` multiplies both branches exactly once (at growth, or at birth / the offspring's growth),
so `r_P` is `γ·(F − B_P)` **up to the reproductive-branch heat**: `χ_P = 1` at `κ_P = 1` and
whenever the reproductive branch is lossless (`η_P = 1`, whatever `s`); otherwise the
reproductive share is strictly less productive than the somatic one and `r_P` is a
`κ_P`-weighted mix, increasing in `κ_P`. At `κ_P = 0`, `χ_P = η_P·s / (1 − η_P·(1 − s)) > 0`
as long as `η_P·s > 0`. Clause (1) is therefore `ρ > 1` together with `χ_P > 0`, and the
second conjunct fails only on a degenerate corner no search box reaches —
`reproduction_efficiency = 0`, or a propagule share of 1, or `offspring_structure_fraction
= 0` with `κ_P = 0` (offspring born as pure reserve that is re-earmarked with loss every
generation and never embodied). On example10 the producer's `fecundity = 0.1` makes the
empty brood the dominant cost (`1 − e^{−0.1} = 0.095`, so `η_P = 0.067`), and with
`κ_P = 0.55`, `s = 0.2`: `χ_P = 0.5697` — the somatic branch carries almost all of `r_P`.
The reproduction *thresholds* still drop out (next-but-one paragraph): they set when the
earmark is spent, not how much of it becomes body.

**The same lumping sits in `β`, uncorrected here.** `β = κ_C·γ·e·a` reads the consumer's
conversion as its somatic share only; by the argument above it should be `χ_C·γ·e·a` with
`χ_C` the consumer's own conversion. It is left as is in this note because `κ_C` runs
through the invasion ratio `I`, A2's `Λ`, the π-forms of
[`440-dimensionless-groups.md`](440-dimensionless-groups.md), every per-cell `I` in A3, and
`hopf_prototype`'s crossing — a separate correction. Its consequence is the symmetric
spurious conjunct `κ_C > 0` on clause (2): the ten atlas cells A3 lists as clause-(1)
failures have `mean_kappa = 0` on the *consumer* centroid too, so after this correction
they clear clause (1) (`r_P` from 0.04 to 0.34) and fail clause (2) with `I = 0` — still
predicted not-permanent, for the same reason in the other compartment.

**One deliberate deviation.** `hopf_prototype` clamps `r_P` at 0 (it only needs the interior
fixed point). The sign of `r_P` *is* the extinction gate, so the boundary analysis keeps it
signed; for `r_P ≤ 0` the producer face is read as density-independent decline
`f = 1 + r_P` (shading can only lower income further, so this bounds the true face factor
from above — using it makes the non-permanence verdict conservative in the right direction).

**Reproduction thresholds drop out.** The map carries *biomass*; whether structure sits in
one body or two is invisible to it (which is exactly why `κ_P` cannot zero `r_P`: it moves
structure between bodies, and `χ_P` accounts for the heat paid on the way).
`reproduction_energy_threshold` and the nutrient earmark enter only through the validity
of the lumping — they set the granularity of the demographic noise the mean field erases
(authority boundary, below) and the energy-death nutrient floor that gates the map's
*existence* (that gate is independent of this one and is not restated).

## Hypotheses

- **(H1) Admissible rates.** `0 < m < 1` (the consumer cannot lose more than its whole body
  in one tick) and `0 < r_P ≤ 2` when `r_P > 0`. Under H1 the face dynamics are simple: on
  `{P = 0}`, `C' = (1 − m)·C → 0` geometrically; on `{C = 0}`, `P' = P + r_P·P·(1 − P/K_P)`
  is the discrete logistic, whose fixed point `K_P` is globally attracting on
  `(0, K_P(1 + r_P)/r_P)` exactly when `0 < r_P ≤ 2`. (For `r_P > 2` see the caveat at the
  end of the proof.)
- **(H2) Unsaturated regime.** Orbits stay in a compact set `Ω ⊂ ℝ²₊` on which `f, g > 0`.
  This is the regime the Hopf validation already assumed for the mass-action term ("total
  demand ≤ available structure"): `f ≤ 0` means the consumers' per-tick demand `a·C`
  exceeds the standing crop's per-tick factor — the stepper's one-shot prey annihilation,
  the second horn of #358's trilemma. On `Ω` the map is continuous, maps the interior to the
  interior, and leaves each face invariant (`P = 0 ⇒ P' = 0`; `C = 0 ⇒ C' = 0`).

## Theorem and proof sketch

**Theorem.** Under H1–H2 the map is permanent — there is `δ > 0` such that every orbit from
the interior satisfies `lim inf P_t ≥ δ` and `lim inf C_t ≥ δ` — iff clauses (1) and (2)
hold (strictly; the equalities are the marginal cases).

**Tool** (average Lyapunov function; Hutson 1984, and Hofbauer–Hutson–Jansen 1987 for
Lotka–Volterra-type difference equations). Let `T: Ω → Ω` be continuous with the boundary
faces invariant, `V: Ω → ℝ₊` continuous with `V > 0` on the interior and `V = 0` on the
boundary, and `Ψ` continuous with `V(Tx)/V(x) = exp Ψ(x)` on the interior. If for every `x`
in the **boundary ω-limit set** `ω(∂Ω)` some finite-time average of `Ψ` along the orbit of
`x` is positive, then `T` is permanent.

**Proof sketch.**

1. *The Lyapunov function.* Take `V = P^a · C^b` with `a, b > 0`. Then, exactly,
   `V(T(P,C)) / V(P,C) = f^a · g^b`, so `Ψ(P,C) = a·ln f(P,C) + b·ln g(P,C)` (well defined
   on `Ω` by H2).
2. *The boundary ω-limit set.* By H1 every orbit on `{P = 0}` converges to `(0,0)` and
   every orbit on `{C = 0}` with `P > 0` converges to `(K_P, 0)`. Hence
   `ω(∂Ω) = {(0,0), (K_P, 0)}` and the average condition reduces to the *value* of `Ψ` at
   two points:
   ```
   Ψ(0,0)   = a·ln(1 + r_P) + b·ln(1 − m)
   Ψ(K_P,0) = a·ln(1)       + b·ln(1 + β·K_P − m)  =  b·ln(1 + β·K_P − m)
   ```
   (`f(K_P, 0) = 1` because `K_P` is a fixed point of the face: the producer contributes
   nothing to escape from the producer-only equilibrium — only the consumer's invasion
   eigenvalue does.)
3. *Sufficiency.* Suppose (1) and (2). `ln(1 − m) < 0` by H1, so pick `a = 1` and
   `b = ln(1 + r_P) / (2·|ln(1 − m)|) > 0`. Then `Ψ(0,0) = ½·ln(1 + r_P) > 0` (by (1)) and
   `Ψ(K_P,0) = b·ln(1 + β·K_P − m) > 0` (by (2)). Both boundary ω-limit points have
   positive `Ψ`, so the theorem applies and the map is permanent.
4. *Necessity.* If `r_P < 0`, then `f(P, C) ≤ 1 + r_P < 1` for all `(P,C)` near the origin
   with `C ≥ 0`, so `(0,0)` attracts an interior neighbourhood: not permanent. If
   `β·K_P < m`, the Jacobian at `(K_P, 0)` is triangular with eigenvalues `1 − r_P` and
   `1 + β·K_P − m`, both of modulus `< 1` under H1, so `(K_P, 0)` is locally asymptotically
   stable and attracts interior points: not permanent. ∎

**Why permanence and not fixed-point stability.** Nothing in steps 1–4 refers to the
interior equilibrium at all. The condition is a statement about the two *boundary* points
and admits any interior attractor — node, spiral, invariant circle, or worse. That is the
property wanted: oscillation is a health criterion, and the Hopf line sits strictly inside
the region the theorem certifies (next section).

**Caveat for `r_P > 2`.** The producer face then carries a period-2 orbit or a chaotic
attractor rather than a fixed point, and `ω(∂Ω)` contains it. Along any bounded face orbit
bounded away from 0, `⟨ln f⟩ = 0`; concavity of `ln` gives `⟨P⟩ ≤ K_P`, and then
`⟨ln g⟩ ≤ ln(1 + β·⟨P⟩ − m) ≤ ln(1 + β·K_P − m)`. So on a fluctuating face the invasion
average is *smaller* than at `K_P`: clause (2) stays **necessary** but is no longer
**sufficient**, and the sharp condition becomes `⟨ln(1 + β·P_t − m)⟩ > 0` along the face
attractor. Every example in this note has `r_P < 2`; the caveat is recorded so the
condition is not over-read.

## Reduction to the extinction gate `F ≤ B` on the `C = 0` face

This is the acceptance criterion the note must show rather than assert. On the face
`{C = 0}` the map is `P' = P · (1 + r_P·(1 − P/K_P))` with

```
r_P = χ_P · γ · (F − B_P),      B_P = B + c_photo·α_P^p + c_struct·M_ref  ≥  B
```

where `B = base_metabolic_rate`, every other term of `B_P` is non-negative (the
form-pinned maintenance is a sum of non-negative powers times non-negative coefficients),
and `χ_P ≥ 0` (a ratio of non-negative terms with a positive denominator). Hence:

```
F ≤ B   ⟹   F ≤ B_P   ⟹   r_P ≤ 0   ⟹   λ_P(0,0) = 1 + r_P ≤ 1
        ⟹   Ψ(0,0) = a·ln(1 + r_P) + b·ln(1 − m) < 0   for every a, b > 0
```

(the second term is strictly negative and the first is `≤ 0`), so no average Lyapunov
function of the form `P^a·C^b` exists and, by step 4, the origin attracts the face: the
producer compartment goes extinct and, its food gone, so does the consumer. The gate
`F ≤ B` is therefore the **corner `B_P = B`** of clause (1) — a producer with no trait or
structure maintenance. Clause (1) is the same gate with the searched maintenance
coefficients left symbolic: `F > B + c_photo·α_P^p + c_struct·M_ref`. Nothing about the
consumer — no efficiency, no kernel, no attack rate — can rescue it, which is exactly the
gate's "for every other parameter and every functional form" clause; and nothing about the
producer's *allocation* can either — `χ_P` scales `r_P` but never flips its sign. The bin's
unit test `condition_fails_on_the_c0_face_whenever_flux_is_at_or_below_base_metabolism`
checks this at `F/B ∈ {0, ¼, ½, 1}` for three efficiencies, asserting `λ_P(0,0) ≤ 1`, both
clauses' conjunction false, and numerical non-permanence; `kappa_zero_producer_with_rho_above_one_keeps_its_face_alive`
and `r_p_mixes_the_somatic_and_reproductive_branches_by_kappa` check the other direction —
that `ρ > 1` keeps the face alive at `κ_P = 0`, that the mix is monotone in `κ_P` between the
reproductive-only and somatic-only limits, that it is `κ_P`-free when the reproductive
branch is lossless, and that a propagule share of 1 genuinely kills the `κ_P = 0` face.

## Boundary equilibria and their eigenvalues

| equilibrium | `(P, C)` | Jacobian | eigenvalues | role in the condition |
|---|---|---|---|---|
| extinction | `(0, 0)` | `diag(1 + r_P, 1 − m)` | `λ_P = 1 + r_P`, `λ_C = 1 − m` | `Ψ(0,0) > 0` needs `λ_P > 1` ⟺ clause (1); `λ_C < 1` always (H1) |
| producer-only | `(K_P, 0)` | `[[1 − r_P, −a·K_P], [0, 1 + β·K_P − m]]` (triangular) | `λ_P = 1 − r_P`, `λ_C = 1 + β·K_P − m` | `Ψ(K_P,0) > 0` ⟺ `λ_C > 1` ⟺ clause (2); `|λ_P| < 1` under H1 |
| interior | `(m/β, (r_P/a)·(1 − m/(β·K_P)))` | `τ = 2 − r_P·P*/K_P`, `Δ = 1 − r_P·P*/K_P + m·r_P·(1 − P*/K_P)` | complex pair, `|λ| = √Δ` (#358) | **none** — permanence does not constrain it; `√Δ = 1` is the Hopf line |

The interior point exists iff clauses (1) and (2) hold (`0 < P* = m/β < K_P` ⟺ `β·K_P > m`;
`C* > 0` ⟺ `r_P > 0`): existence of the coexistence equilibrium and permanence coincide
for this map, as is typical for planar Lotka–Volterra-type systems — but the *proof* of
persistence runs through the boundary, not through that point's stability.

On `scenarios/example10_predator_prey_hopf.json` (committed parameters, current drain form
— `a = 0.55`, so the Hopf crossing is now `base* = 0.3405`, not the pre-#380 `0.4427`
quoted in `F-hopf-validation.md`; `hopf_prototype` reports the same):

```
  point                               P          C   lambda_1   lambda_2
  (0,0)      extinction          0.0000     0.0000     2.3479     0.8870
  (K_P,0)    producer-only      71.1111     0.0000    -0.3479     3.5018
  (P*,C*)    interior            3.0737     2.3449     1.0428     1.0428
```

(`r_P = 1.3479` with `χ_P = 0.5697`; the pre-#466 lumping gave `r_P = 1.3014` — the
reproductive share adds 3.6 % on this low-fecundity producer, and moves nothing but the
`r_P`-dependent entries: `λ_P` at the two boundary points, `C*`, and the interior modulus.
`K_P`, `β`, `I`, the Hopf line, and the sweep below are unchanged.)

`ρ = F/B_P = 71.1 > 1` and `I = β·K_P/m = 23.1 > 1`: the mean field says example10 is
permanent, and (since `I > (1+m)/m = 9.85`) oscillating. See the authority boundary for
what that does and does not mean for the committed file.

## The two dominant dimensionless ratios

Both clauses are ratios of a gain to a maintenance floor:

- **`ρ = F / B_P`** — the *extinction ratio*. Clause (1) is `ρ > 1`. It is also `K_P` in
  units of the reference body.
- **`I = β·K_P / m = κ_C·γ·base·exp(−λ·d)·h_C · F / (B_P·B_C)`** — the *invasion ratio*:
  the consumer's per-tick conversion at the producer's standing crop, over its own floor.
  Clause (2) is `I > 1`. Note `I = ε·ρ` with `ε = κ_C·γ·base·exp(−λ·d)·h_C / B_C`, so in the
  `(ρ, base)` plane the invasion boundary is the hyperbola `base · ρ = const`.

In these coordinates the Hopf validation's crossing `P*/K_P = m/(1+m)` reads
`I = (1 + m)/m`, which is `> 1` for every `m > 0`: **the Hopf line is strictly inside the
permanent region**, at a distance that grows as the consumer's maintenance floor shrinks.

## Numerics

The bin sweeps `ρ` (via `solar_flux_magnitude`, 25 log-spaced rows from 0.5 to 100) against
`base_trophic_efficiency` (25 columns, 0.04 to 1.00), holding every other committed
parameter of example10 fixed. Each cell is classified twice:

- **analytically**, by clauses (1)–(2);
- **by boundary escape**: iterate the map for 20 000 ticks from `(10⁻³·K_P, 10⁻⁶·K_P)` — a
  point hugging both faces, so the orbit must first escape `(0,0)` along the producer face
  and then escape `(K_P, 0)` by consumer invasion, the two ω-limit points of the theorem —
  and read `lim inf > 0` as "the trough has stopped shrinking": the minimum of each
  compartment over the last quarter of the horizon is within a factor 10 of its minimum
  over the third quarter, and above an absolute floor of `10⁻¹⁰⁰` (a geometrically decaying
  compartment loses a factor `≥ (1 − ε)^5000` between the windows; an orbit settled on a
  fixed point or cycle keeps its trough). An orbit on which `f ≤ 0` or `g ≤ 0` has left H2
  and is reported as such rather than scored.

```
       rho  I@eff=1     |    |    |    |    |
      0.50     0.20 .........................
      0.62     0.25 .........................
      0.78     0.32 .........................
      0.97     0.39 .........................
      1.21     0.49 .........................
      1.51     0.61 .........................
      1.88     0.76 .........................
      2.34     0.95 .........................
      2.92     1.19 ....................?####
      3.65     1.48 ................#########
      4.55     1.85 .............############
      5.67     2.31 ..........###############
      7.07     2.88 ........#################
      8.82     3.59 ......###################
     11.00     4.47 .....####################
     13.71     5.58 ....#####################
     17.10     6.95 ...######################
     21.32     8.67 ..#######################
     26.59    10.81 ..####################ooo
     33.16    13.49 .#################oooxxxx
     41.35    16.82 .#############oooxxxxxxxx
     51.57    20.97 .##########oooxxxxxxxxxxx
     64.31    26.15 #########ooxxxxxxxxxxxxxx
     80.19    32.61 #######ooxxxxxxxxxxxxxxxx
    100.00    40.67 ######oxxxxxxxxxxxxxxxxxx
  cols: eff = 0.04 .. 1.00 in steps of 0.04; '|' marks 0.2, 0.4, 0.6, 0.8, 1.0
  '#' permanent, stable interior   'o' permanent, above the Hopf line
  '.' not permanent                'x' Euler map left the unsaturated regime
  '!' / '?' analytic-vs-numeric disagreement
```

**Agreement, with tolerance.** Cells within **2 %** of either analytic boundary
(`|ρ − 1| ≤ 0.02` or `|I − 1| ≤ 0.02`) are not scored: there the invasion growth factor is
within `0.02·m ≈ 0.002` of 1 and a 20 000-tick horizon cannot separate slow growth from slow
decay. Of the 625 cells, 4 are within tolerance (the single `?` at `ρ = 2.92, eff = 0.84`
is one of them: `I = 1.00`), 71 left the unsaturated regime, and the remaining **550 agree,
0 disagree**. The unit test `boundary_escape_numerics_agree_with_analytic_condition` runs
the same check on a finer 40×40 grid (≥ 1000 scored cells, 0 disagreements allowed) in
under half a second.

Read off the picture:

- The `.`/`#` frontier is the hyperbola `I = 1` (`base · ρ = const`), and the whole
  `ρ < 1` band is `.` regardless of efficiency — clause (1) is a vertical wall, clause (2) a
  hyperbola, exactly as derived.
- The `#`/`o` frontier is the Hopf hyperbola `I = (1 + m)/m`, sitting inside the `#`
  region. Permanence certifies both sides of it; the cycle is born inside the permanent
  region, never at its edge.
- The `x` region is a finding in its own right. Above the Hopf line the Euler map's
  invariant circle grows with `I` until the consumer trough's *next* peak demands more than
  the standing crop can supply per tick (`a·C > 1 + r_P(1 − P/K_P)`, `f ≤ 0`) — H2 fails.
  This is not a failure of permanence (the theorem needs H2) but a statement that the
  mass-action reading of `resolve_drains` stops being faithful there: the stepper caps a
  drain at the available structure, which in the mean field is the "dense prey + voracious
  consumer → one-shot annihilation, then predator starvation" horn of the trilemma in
  `F-hopf-validation.md`. Two things are worth separating here. The **permanence condition
  is discretisation-invariant**: the exponential (Ricker) form
  `P' = P·exp(r_P(1 − P/K_P) − a·C)`, `C' = C·exp(β·P − m)` has boundary eigenvalues
  `e^{r_P}`, `e^{−m}`, `e^{β·K_P − m}` whose logs have the same signs as clauses (1)–(2),
  so it certifies the same region. The **Hopf line and the H2 exit are not**: they depend
  on the interior Jacobian, which differs between the additive per-tick form the stepper
  commits and the exponential form (at example10's coefficients the Ricker form's interior
  point is still stable at `eff = 0.8`, where the Euler form has already left H2). So the
  `#`/`.` frontier in the picture is a property of the committed fluxes; the `#`/`o` and
  `o`/`x` frontiers are properties of the committed *per-tick additive* discretisation of
  them, and should be read with that in mind — a point the Hopf validation's own caveat
  about `REFERENCE_BODY_MASS` already makes for the crossing's location.

Reproduce:

```
cargo run -p explorers-sim --bin permanence_prototype -- scenarios/example10_predator_prey_hopf.json
cargo test -p explorers-sim --bin permanence_prototype
```

## Authority boundary — what the condition does and does not claim

This is Brief F's AC3 boundary ([`F-mean-field-operator.md`](F-mean-field-operator.md)),
inherited unchanged and applied to permanence.

**The condition is a statement about a deterministic, well-mixed, continuous-biomass map.**
It says: in the mean field — expectations in place of reproduction and movement draws, an
in-reach indicator replaced by 1, structure a real number — the extinction boundary repels.
It says nothing else. Specifically:

- **No demographic noise.** Permanence bounds `lim inf` away from zero, but `δ` is
  whatever the interior attractor's trough is — and on the growing cycle above the Hopf
  line that trough shrinks with `I` without bound until H2 fails. A finite population at a
  trough of a few bodies dies by chance fluctuation, with probability approaching 1 as the
  trough falls. So mean-field permanence is **necessary, not sufficient,
  for individual-based persistence** — the same conservative asymmetry as the existence
  gates: *mean-field non-permanent ⇒ the expected trajectory of the individual world goes
  extinct; mean-field permanent ⇒ nothing is promised per seed.* The direction that is safe
  is the negative one — the direction a gate uses.
- **No space.** `a·P·C` is an integral over an in-reach indicator that the mean field sets
  to 1. example10 is the standing counter-example: the mean field reads it as permanent
  and oscillating (`ρ = 71`, `I = 23`), and the committed file's consumers go extinct by
  tick 2 because their reach never meets the sparse standing crop
  ([`F-hopf-validation.md`](F-hopf-validation.md), primary fault). The gap between "permanent
  here" and "dead there" localises to the in-reach geometry, exactly as AC5 says a
  disagreement should — it is a diagnostic, not a defect in the condition.
- **No per-seed or distributional claim.** Whether coexistence establishes on a given
  seed, on what fraction of seeds, or with what amplitude, is genesis search's to report.
  The condition's output is a *region* in parameter space where the average world cannot
  hold both compartments, and its complement where it can.
- **Unsaturated regime only (H2).** Where the mean-field orbit would leave H2 the condition
  is silent (the `x` cells); the stepper's drain cap makes that regime a different map
  whose permanence this note does not analyse.
- **Two compartments only.** The map lumps every producer into `P` and every consumer into
  `C`. Trait-distribution branching (monoculture↔coexistence), decomposer guilds, and the
  nutrient ledger are outside its coordinates; the condition is about the
  frozen↔oscillation coordinate's *boundary*, not those modes.

What it *can* be used for, on those terms: the negative direction is a candidate
**affirmative-form gate** in the sense of `viability.md`'s "Direction of travel" — a region
(`ρ ≤ 1` or `I ≤ 1`) where the mean field forbids a producer↔consumer pair from persisting,
decidable from committed parameters plus the two clusters' trait vectors, with the same
"gated dead that survives a rollout localises a mis-drawn gate" falsifiability the existing
prefilter has. Whether to promote it is a system-design decision this note deliberately
does not make; the two known reasons to hesitate are that `I` depends on the consumer's
realised `h_C` and the pair's trait distance `d` (per-cluster inputs, not world
parameters — a gate would need a "best consumer" bound), and that spatial reach, not the
per-tick conversion, was the binding constraint on the one example examined.

## Deliverables against the acceptance criteria

- Research doc with the standard header; nothing added to `docs/system-design/` — this file.
- The condition reduces to `F ≤ B` on the `C = 0` face — shown in *Reduction to the
  extinction gate*, and unit-tested; the `r_P` row counts both of `κ_P`'s branches (#466),
  so clause (1) carries no allocation conjunct.
- Boundary-equilibrium table with eigenvalues — above, symbolic and on example10.
- Numerics agree with the analytic boundary within a stated tolerance — 550/550 scored
  cells, 2 % boundary band, unit-tested on a 40×40 grid.
- Authority boundary — stated.
- No change to the stepper, phases, RNG, evaluator, or search — the only code is the
  throwaway bin, banner-marked not-production, plus its tests.
