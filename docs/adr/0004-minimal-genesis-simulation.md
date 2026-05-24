# Sense-decide-act tick loop with 8-dimensional trait vector for genesis-minimal simulation

The minimal simulation capable of producing the three genesis fitness criteria (oscillations, clustering, coexistence) uses a sense-decide-act tick loop with double-buffered state and an 8-dimensional evolvable trait vector. Two trait dimensions (social weight, chemical signature) are deferred — they add behavioural richness but are not structurally necessary for trophic differentiation, population oscillations, or emergent speciation.

## Considered options

- **Sequential agent update.** Each agent senses, decides, and acts in turn within a single pass. Simpler to implement — no double-buffering needed. But results depend on iteration order: the first agent sees a different world than the last. Shuffling mitigates but doesn't eliminate the bias. Reproducibility requires storing the shuffle permutation alongside the seed. Harder to parallelise.
- **Sense-decide-act with double-buffered state (chosen).** All agents sense (reading last tick's state), then all decide, then all act (writing to next tick's state). No order-dependence between agents within a phase. Results are reproducible given a seed — only initial positions and mutation are stochastic. Parallelises trivially per phase if genesis throughput demands it. The cost is maintaining two copies of agent state, which is negligible at expected population sizes.

## The tick loop

```
1. Sense     — build each agent's neighbourhood
               (who is within sensing range, at what distance, distance-weighted signal strength)

2. Decide    — each agent selects actions based on traits + neighbourhood:
               - movement vector (chemotaxis toward food signal + random exploration)
               - consumption target (living agent in contact, if consumption rate > 0)
               - decomposition target (carcass in contact, if scavenging rate > 0)
               - reproduction partner (compatible agent within sensing range,
                 if energy > reproduction threshold)

3. Act       — apply all decisions simultaneously:
               a. Move all agents, pay movement cost (distance × movement cost coefficient)
               b. Photosynthesise: gain energy (photosynthetic absorption × solar flux magnitude)
               c. Consume: drain energy from living targets (consumption rate × consumption efficiency)
               d. Decompose: drain energy from carcasses (scavenging rate × decomposition efficiency)
               e. Pay base metabolic cost (scaled by sensing range)
               f. Death: agents at zero energy become carcasses (energy locked)
               g. Reproduce: valid pairs produce offspring via crossover + mutation,
                  parents transfer energy per reproductive investment trait,
                  offspring receives fraction per reproduction efficiency (remainder dissipated)
```

## Genesis-minimal trait vector (8 dimensions)

| Dimension | Role in genesis fitness |
|---|---|
| Photosynthetic absorption | Defines producer niche; energy input pathway |
| Consumption rate | Defines consumer niche; creates trophic coupling for oscillations |
| Scavenging rate | Defines decomposer niche; energy recycling prevents energy death |
| Mobility | Producer/consumer trade-off — sessile photosynthesis vs. mobile consumption |
| Chemotaxis sensitivity | Steering strength toward food signals; under real selection pressure for foraging success |
| Sensing range | Required for all local interaction; metabolic cost creates trade-off |
| Mate selectivity | Maximum trait distance for reproduction; drives emergent speciation (clustering with gaps) |
| Reproductive investment | Energy transferred to offspring; K/r trade-off affects oscillation character |

## Deferred dimensions

- **Social weight**: attraction toward feeding neighbours. Produces emergent herding. Adds behavioural richness but herding is not required for any of the three genesis criteria.
- **Chemical signature**: identity signal detectable by other agents. Required for player-ecology interaction and species-level recognition. Not needed for genesis fitness evaluation.

## Consequences

- The simulation crate's `TraitVector` expands from 2 to 8 dimensions.
- Double-buffered state means `World::step` reads from current state and writes to a new state, then swaps. No in-place mutation during a tick.
- Genesis evaluation observes the state between ticks — it never needs to see intermediate phase results.
- Adding social weight and chemical signature later extends the trait vector without changing the tick loop structure. The sense and decide phases already handle arbitrary neighbourhood signals.
- The movement model initially has two components (chemotaxis + random exploration). Social foraging adds a third component when social weight is promoted.
