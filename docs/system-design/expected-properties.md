# Expected Properties

The emergent properties we expect to observe in a world whose rules are correctly calibrated. These are not additional rules — they are predictions. If the [world rules](world-rules.md) are right, these properties should arise from agents operating within them. If these properties are absent, the rules are miscalibrated.

This document serves two purposes: it defines what a well-functioning world looks like, and it defines the failure modes — the observable signs that something is wrong.

## Properties of a well-functioning world

### Trophic structure

Agents differentiate into producers, consumers, and decomposers — the three energy-acquisition strategies corresponding to the three input flows defined in the world rules (photosynthesis, consumption, decomposition). These are not types assigned to agents; they are roles that emerge because the cost structure makes specialisation more viable than generalism. The ecology literature consistently identifies these three roles as the fundamental energy-acquisition strategies in any ecosystem with a detrital pathway.

The trophic structure should exhibit a pyramid shape: more energy (and typically more agents) at the producer level than at the consumer level, and more at the consumer level than at higher levels. This is a direct consequence of trophic transfer loss (flow 7) — energy dissipates at every transfer, so each successive level has less energy available.

### Population oscillations

Populations fluctuate rather than reaching fixed points. Consumer-resource interactions create coupled oscillations — more prey leads to more consumers, which reduces prey, which reduces consumers, which allows prey recovery. These oscillations are the signature of a system with both positive feedbacks (reproductive amplification) and negative feedbacks (resource depletion, metabolic costs) operating simultaneously.

A system that reaches a static equilibrium has either lost its positive feedbacks (frozen dynamics) or its negative feedbacks are overwhelming (all variation is immediately damped). Neither is a healthy state.

### Trait-space clustering

Agents cluster into distinct strategy groups in the trait space, with gaps between clusters. This is analogous to speciation — discrete strategies that are reproductively and functionally distinct from each other. Clustering arises from the interaction between reproductive inheritance (offspring resemble parents), mate selectivity (reproduction is more likely between similar agents), and the cost structure (the fitness landscape has peaks at specialist strategies and valleys at generalist positions).

### Coexistence

Multiple clusters persist simultaneously over extended periods. No single strategy eliminates all others. This requires that different strategies have advantages in different contexts — spatial locations, population densities, successional stages. If one strategy dominates everywhere, the cost structure is not creating enough context-dependent fitness variation.

### Demographic turnover

Agents are born and die at non-trivial rates. The system is not frozen — new agents enter the population, old agents exit, and the composition of the population changes over time. Turnover is what allows adaptation: new trait combinations are tested through reproduction, and unsuccessful ones are removed through death.

### Energy throughput

Energy flows through the system at meaningful rates. It enters via photosynthesis, passes through living agents and carcasses, and exits via metabolism and trophic loss. The rate of throughput — not just the total amount of stored energy — is what characterises an active system. A system can have correct energy conservation but zero throughput (all energy locked in carcasses, no decomposers). Throughput requires that all flows are active, not just that the accounting holds.

### Energy conservation

The accounting holds. Total energy in the system (living agents + carcasses) changes by exactly the difference between photosynthetic input and total dissipation. No energy appears from nowhere; no energy vanishes without being accounted for. This is not an emergent property in the usual sense — it is a correctness criterion for the simulation itself. If conservation fails, the implementation is broken.

## Failure modes

Each failure mode is the absence of one or more expected properties. They define what a miscalibrated world looks like.

### Extinction

All agents die. The system has no persistence. This occurs when drains persistently exceed the tap — metabolic costs are too high relative to photosynthetic input, or consumption pressure overwhelms producer populations before they can reproduce.

### Monoculture

Trait space collapses to a single cluster. The system has persistence but no diversity. One strategy outcompetes all others everywhere. This occurs when the cost structure does not create enough context-dependent fitness variation — a single strategy dominates across all spatial locations and population densities.

### Energy death

Free energy trends to zero. Energy accumulates in the carcass stock and is not returned to the living system. This occurs when decomposer strategies are not viable — either the cost of decomposition exceeds its energy return, or decomposer populations cannot sustain themselves. The living system starves while energy sits locked in dead matter.

### Population explosion

Unbounded growth. No force limits the system. This occurs when the negative feedbacks (metabolic costs, trophic loss, competition for the energy source) are too weak relative to the energy input rate. Populations grow until the simulation runs out of resources.

### Frozen dynamics

Agents survive but nothing changes. Populations sit at fixed values, no new strategies emerge, demographic turnover drops to near zero. The system has persistence but no process. This occurs when all positive feedbacks are suppressed — reproductive amplification is too slow, oscillations are too heavily damped, or the fitness landscape is too flat for selection to operate.

### Generalist dominance

Agents that invest in all acquisition strategies simultaneously outcompete specialists. Diversity collapses from above — not because one specialist wins, but because a jack-of-all-trades beats every specialist in their own niche. This occurs when trait maintenance costs are too low, so the overhead of breadth does not outweigh the benefit of flexibility.

## Relationship between properties and failure modes

| Expected property | Failure mode when absent |
|---|---|
| Trophic structure | Monoculture or generalist dominance |
| Population oscillations | Frozen dynamics |
| Trait-space clustering | Generalist dominance or monoculture |
| Coexistence | Monoculture |
| Demographic turnover | Frozen dynamics or extinction |
| Energy throughput | Energy death or frozen dynamics |
| Energy conservation | Implementation bug (not a calibration issue) |

## What this document does not prescribe

This document describes *what* a working world looks like, not *how* to achieve it. The mechanisms that produce these properties are described in the world rules. The specific parameter values that calibrate the rules correctly are an implementation concern — they belong in the ADR layer, not here.

Disturbance, succession, spatial refugia, patch dynamics — these are patterns that ecologists observe in well-functioning ecosystems. They are not separate mechanisms to design. They arise from the same stocks, flows, and trade-offs described in the world rules, playing out across different timescales and spatial scales. If the world rules are right, these patterns should emerge. If they don't, the problem is in the rules, not in the absence of a disturbance mechanism.

## Initial conditions and world generation

The expected properties above describe a world in motion — agents interacting under the world rules, producing dynamic ecology. But agents do not evolve into a void. They evolve into a **substrate**: a spatial distribution of nutrients, topography, and environmental heterogeneity that has been shaped by processes far longer than any agent-evolution timescale.

On Earth, the substrate is the product of 4.5 billion years of geology — tectonic uplift, erosion, atmospheric chemistry, hydrological cycles. The spatial distribution of phosphorus in rock, nitrogen in soils, water in river systems, and sunlight across latitudes is what creates the niche heterogeneity that supports diverse ecologies. Agents evolved into a world that was already richly structured.

The expected properties of a well-functioning world depend on the quality of its initial conditions. Certain substrate configurations make it very difficult — or impossible — for the expected ecological properties to emerge:

- **Homogeneous substrate.** If nutrients are uniformly distributed, there is no spatial reason for different strategies to succeed in different places. Coexistence and functional differentiation are harder to achieve because there is less context-dependent fitness variation.
- **Extreme nutrient poverty.** If the substrate contains too little of a limiting nutrient, the energy throughput of the living system may be too low to sustain demographic turnover and population oscillations.
- **Extreme nutrient abundance.** If all nutrients are superabundant everywhere, stoichiometric constraint ceases to differentiate strategies. The specialist-generalist trade-off weakens because there is no nutrient limitation to penalise inefficiency.
- **Spatial monotony.** If the substrate has no variation in nutrient availability, topography, or accessibility, the spatial mosaic of successional stages and niche specialisation has no substrate-level driver.

The world rules describe the physics. The expected properties describe what a healthy ecology looks like. The initial conditions — how the substrate is generated — determine whether the physics has enough raw material for the ecology to emerge. How the substrate is generated (procedural algorithms, geological simulation, or hand design) is an implementation concern, but the properties it must have are a system design concern: **the substrate must provide spatially heterogeneous nutrient availability sufficient to create context-dependent fitness variation across the surface.**
