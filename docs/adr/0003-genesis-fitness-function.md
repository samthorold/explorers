# Geometric-mean fitness function for world genesis evaluation

World genesis evaluates parameterisations by running an ensemble of replicate simulations (same world parameters, different random seeds) and computing a single scalar fitness. A parameterisation is accepted when its median fitness across the ensemble exceeds a threshold. The fitness function is a geometric mean of five normalised criteria, preceded by early-termination failure checks with a uniform grace period — following the pattern-oriented modelling approach (Grimm et al. 2005) where multiple independent patterns must be reproduced simultaneously.

## Revision history

Revised from gated product of three criteria to geometric mean of five criteria. The original product with hard-zero gates produced a flat fitness landscape — every parameterisation scored exactly 0 because (a) failure detectors fired prematurely on initial conditions before evolution could act, and (b) the product of any zero is zero, giving the optimiser no gradient signal. See "Why this changed" below.

## Considered options

- **Weighted sum of criteria.** Assign weights to oscillation, clustering, and coexistence, normalise, and sum. Simple but weights are arbitrary and allow compensation — strong clustering can mask absent oscillations. A sensible world requires all properties, not a high total.
- **Gated product of three criteria (original, superseded).** Failure modes and sanity checks gate to zero. Primary criteria multiplied. Any criterion at zero collapses the entire score. In practice this created a feasibility cliff — the optimiser saw a flat zero landscape with no gradient to follow.
- **Geometric mean of five criteria (chosen).** Failure modes still terminate runs early but failed runs score a small nonzero value proportional to survival duration. Former sanity checks (turnover, trophic balance) become two additional criteria in the geometric mean alongside the original three (oscillation, clustering, coexistence). Zeros are heavily penalised but a single weak criterion doesn't obliterate all signal from the others.
- **Lexicographic ordering.** Rank parameterisations by clustering first, break ties by coexistence, then oscillation. Gives a total ordering but not a scalar, making it incompatible with Bayesian optimisation.

## Why this changed

The original gated product produced fitness = 0.0 for 100% of parameterisations across thousands of ensemble runs. Three interacting causes:

1. **Premature failure detection.** The monoculture detector fired on tick 1 when trait_covariance was low (agents start nearly identical). The generalist dominance detector fired immediately because mean trait values placed all agents above the generalist threshold. These are real failure modes but the detectors couldn't distinguish "degenerate world" from "population that hasn't had time to evolve yet."
2. **Infeasible parameter ranges.** base_metabolic_rate up to 1.0 meant metabolic cost often exceeded energy income, preventing reproduction entirely. trait_covariance down to 0.01 meant agents started as clones.
3. **No gradient signal.** With fitness = 0 everywhere, Bayesian optimisation had nothing to learn from. LHS sensitivity analysis produced all-zero indices. The search was blind.

## The pipeline

Checks are ordered from cheapest to most expensive. Early termination saves compute across the ensemble.

### 1. Failure detection (per tick or periodic, terminates run)

A uniform grace period of 20% of max_ticks applies to all non-catastrophic detectors. Detectors are suppressed during the grace period — initial conditions need time to evolve before they can be meaningfully evaluated.

**Catastrophic (fire immediately, no grace period):**
- **Extinction**: agent count hits zero.
- **Population explosion**: agent count exceeds ceiling.

**Non-catastrophic (fire only after grace period):**
- **Energy death** (periodic): total free energy in living agents trending monotonically downward over a window.
- **Monoculture** (periodic): dip test on trait-distance distribution indicates unimodality.
- **Generalist dominance** (periodic): cluster(s) with high values across multiple energy-acquisition traits dominate the population.

Frozen dynamics (zero births and deaths) was originally a hard failure mode but is now handled entirely by the demographic turnover criterion in the geometric mean. A frozen population gets turnover_score=0, which zeros the geometric mean — no hard termination needed. Removing the hard gate avoids the perverse incentive where the survival floor rewarded frozen populations (long survival, no activity) over populations with active reproduction that exploded (short survival, high activity).

Failed runs score 0. The geometric mean of the five criteria provides the gradient signal for the search — runs that develop ecological structure score higher than those that don't. The survival-fraction floor was removed because it created a perverse incentive: failed runs (with nonzero floor) outscored successful-but-ecologically-dead runs (geometric mean = 0 due to no turnover).

All checks are external observers — the simulation has no knowledge of genesis.

### 2. Criteria (normalised to [0,1], combined via geometric mean)

Five criteria, all normalised to [0,1]:

- **Oscillation strength**: maximum autocorrelation at non-zero lag, averaged across labelled clusters (Bjørnstad & Grenfell 2001). Range naturally [0,1].
- **Clustering strength**: 1 − p-value of the dip test on pairwise trait-distance distribution (Hartigan & Hartigan 1985). Highly multimodal → near 1; unimodal → near 0.
- **Coexistence duration**: fraction of total ticks where ≥2 DBSCAN-labelled clusters coexisted simultaneously.
- **Demographic turnover**: `min(total_births, total_deaths) / max_ticks`, clamped to [0,1]. More turnover is better — saturates quickly.
- **Trophic balance**: `producer_energy / (producer_energy + consumer_energy)`. Values above 0.5 indicate a healthy trophic pyramid.

### 3. Fitness

```
fitness(run) =
  0                                           if any failure mode detected
  geometric_mean(oscillation, clustering,
                 coexistence, turnover,
                 trophic_balance)             otherwise

fitness(parameterisation) = median(fitness across ensemble runs)
```

## Parameter range constraints

Two ranges are tightened from their original values to reduce the fraction of infeasible parameterisations:

- **base_metabolic_rate**: [0.01, 0.5] (was [0.01, 1.0]). Values above 0.5 consistently exceeded energy income, preventing any demographic turnover.
- **trait_covariance**: [0.1, 1.0] (was [0.01, 0.5]). Values below 0.1 produced agents so similar that monoculture was inevitable regardless of evolutionary dynamics.

## Search strategy

Latin hypercube sampling (Thiele et al. 2014) maps the landscape first — identifies sensitive parameters and promising regions. Bayesian optimisation refines within those regions. LHS provides sensitivity analysis as a side effect.

## Consequences

- Genesis tooling lives outside the simulation crate (consistent with ADR-0001). The simulation exposes state; genesis observes and evaluates.
- The geometric mean means a parameterisation cannot score well by excelling at one criterion — all must contribute. But unlike the product, a single weak (but nonzero) criterion doesn't obliterate the signal from strong criteria.
- Grace periods mean the search can explore parameterisations that start uniform but evolve diversity — consistent with genesis testing whether differentiation emerges spontaneously.
- Cluster labelling (DBSCAN) is required for oscillation and coexistence measurement. The dip test gates whether labelling is attempted.
- The median across ensemble runs means outlier runs (one lucky or unlucky seed) don't dominate. A parameterisation must reliably produce sensible worlds.
- The fitness function will need recalibration as new criteria are added (e.g., spatial patterns, environmental cycles). Adding a new factor to the geometric mean is straightforward.
