# Gated-product fitness function for world genesis evaluation

World genesis evaluates parameterisations by running an ensemble of replicate simulations (same world parameters, different random seeds) and computing a single scalar fitness. A parameterisation is accepted when its median fitness across the ensemble exceeds a threshold. The fitness function is a gated product of three normalised criteria, preceded by early-termination failure checks — following the pattern-oriented modelling approach (Grimm et al. 2005) where multiple independent patterns must be reproduced simultaneously.

## Considered options

- **Weighted sum of criteria.** Assign weights to oscillation, clustering, and coexistence, normalise, and sum. Simple but weights are arbitrary and allow compensation — strong clustering can mask absent oscillations. A sensible world requires all three properties, not a high total.
- **Gated product of criteria (chosen).** Failure modes gate to zero with early termination. Sanity checks (turnover, trophic pyramid) gate to zero. Primary criteria (oscillation strength, clustering strength, coexistence duration) are each normalised to [0,1] and multiplied. Any criterion near zero tanks the score — all must be present. No weight tuning required.
- **Lexicographic ordering.** Rank parameterisations by clustering first, break ties by coexistence, then oscillation. Gives a total ordering but not a scalar, making it incompatible with Bayesian optimisation.

## The pipeline

Checks are ordered from cheapest to most expensive. Early termination saves compute across the ensemble.

### 1. Failure detection (per tick or periodic, terminates run → score 0)

- **Extinction**: agent count hits zero.
- **Population explosion**: agent count exceeds ceiling.
- **Energy death** (periodic): total free energy in living agents trending monotonically downward over a window.
- **Frozen dynamics** (periodic): zero births and zero deaths over a window.
- **Monoculture** (periodic, after sufficient ticks): dip test on trait-distance distribution indicates unimodality.
- **Generalist dominance** (periodic): cluster(s) with high values across multiple energy-acquisition traits dominate the population.

All checks are external observers — the simulation has no knowledge of genesis.

### 2. Sanity checks (binary gate, run end)

- **Demographic turnover**: birth count and death count both positive over evaluation window.
- **Trophic pyramid**: total energy in producer-like clusters > total energy in consumer-like clusters (using trophic coordinates on the energy-acquisition simplex).

Fail either → score 0.

### 3. Primary criteria (normalised to [0,1], multiplied)

- **Oscillation strength**: maximum autocorrelation at non-zero lag, averaged across labelled clusters (Bjørnstad & Grenfell 2001). Range naturally [0,1].
- **Clustering strength**: 1 − p-value of the dip test on pairwise trait-distance distribution (Hartigan & Hartigan 1985). Highly multimodal → near 1; unimodal → near 0.
- **Coexistence duration**: fraction of total ticks where ≥2 DBSCAN-labelled clusters coexisted simultaneously.

### 4. Fitness

```
fitness(run) =
  0                                           if any failure mode detected
  0                                           if any sanity check fails
  oscillation * clustering * coexistence      otherwise

fitness(parameterisation) = median(fitness across ensemble runs)
```

## Search strategy

Latin hypercube sampling (Thiele et al. 2014) maps the landscape first — identifies sensitive parameters and promising regions. Bayesian optimisation refines within those regions. LHS provides sensitivity analysis as a side effect.

## Consequences

- Genesis tooling lives outside the simulation crate (consistent with ADR-0001). The simulation exposes state; genesis observes and evaluates.
- The product formulation means a parameterisation cannot score well by excelling at one criterion — all three must be present. This is deliberately conservative.
- Cluster labelling (DBSCAN) is required for oscillation and coexistence measurement. The dip test gates whether labelling is attempted.
- The median across ensemble runs means outlier runs (one lucky or unlucky seed) don't dominate. A parameterisation must reliably produce sensible worlds.
- The fitness function will need recalibration as new criteria are added (e.g., spatial patterns, environmental cycles). Adding a new factor to the product is straightforward but changes the scale.
