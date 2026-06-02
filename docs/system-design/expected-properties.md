# Expected Properties

The emergent properties we expect to observe in a world whose rules are correctly calibrated. These are not additional rules — they are predictions. If the [world rules](world-rules.md) are right, these properties should arise from agents operating within them. If these properties are absent, the rules are miscalibrated.

This document serves two purposes: it defines what a well-functioning world looks like, and it defines the failure modes — the observable signs that something is wrong.

## Properties of a well-functioning world

### Trophic structure

Agents differentiate into producers, consumers, and decomposers — the three energy-acquisition strategies corresponding to the three input flows defined in the world rules (photosynthesis, consumption, decomposition). These are not types assigned to agents; they are roles that emerge because the two-currency physics (energy flows one way, matter cycles) requires at least three functional positions to sustain the system, and because functional incompatibility and structural fragility make specialisation — not generalism — the more viable way to fill them. (Per-trait maintenance convexity, sometimes credited with this, is breadth-neutral; see [world rules](world-rules.md), cost structure.) Consumer and decomposer are both expressions of heterotrophy — the distinction is target state (living vs dead), not separate capability. The ecology literature consistently identifies these three roles as the fundamental energy-acquisition strategies in any ecosystem with a detrital pathway.

The trophic structure should exhibit a pyramid shape: more energy (and typically more agents) at the producer level than at the consumer level, and more at the consumer level than at higher levels. This is a direct consequence of trophic transfer loss (flow 7) — energy dissipates at every transfer, so each successive level has less energy available.

### Population oscillations

Populations fluctuate rather than reaching fixed points. Consumer-resource interactions create coupled oscillations — more prey leads to more consumers, which reduces prey, which reduces consumers, which allows prey recovery. These oscillations are the signature of a system with both positive feedbacks (reproductive amplification) and negative feedbacks (resource depletion, metabolic costs) operating simultaneously.

A system that reaches a static equilibrium has either lost its positive feedbacks (frozen dynamics) or its negative feedbacks are overwhelming (all variation is immediately damped). Neither is a healthy state.

### Trait-space clustering

Agents cluster into distinct strategy groups in the trait space, with gaps between clusters. This is analogous to speciation — discrete strategies that are reproductively and functionally distinct from each other. Clustering arises from the interaction between reproductive inheritance (offspring resemble parents), reproductive isolation (agents whose trait-space divergence exceeds a physics-defined reproductive compatibility distance cannot mate), and the cost structure (the fitness landscape has peaks at specialist strategies and valleys at generalist positions). Speciation happens when populations diverge past this compatibility threshold — it is not driven by an explicit selectivity trait but by the accumulation of trait-space distance between lineages.

### Coexistence

Multiple clusters persist simultaneously over extended periods. No single strategy eliminates all others. This requires that different strategies have advantages in different contexts — spatial locations, population densities, successional stages. If one strategy dominates everywhere, the cost structure is not creating enough context-dependent fitness variation.

### Demographic turnover

Agents are born and die at non-trivial rates. The system is not frozen — new agents enter the population, old agents exit, and the composition of the population changes over time. Turnover is what allows adaptation: new trait combinations are tested through reproduction, and unsuccessful ones are removed through death. Death occurs through both extrinsic causes (predation, starvation from competition) and intrinsic causes (somatic wear degrading functional capabilities until the agent can no longer sustain itself). The combination of these mortality sources ensures that no agent persists indefinitely — even in the absence of predation, wear makes eventual death inevitable for agents that invest less in somatic maintenance than the rate of degradation.

### Age structure and survivorship variation

Populations exhibit age distributions, not uniformly young or uniformly old cohorts. Different strategies produce different survivorship patterns: high-fecundity strategies produce many short-lived offspring with high juvenile mortality (most die young, a few survive to reproduce). Low-fecundity strategies produce few long-lived offspring with low juvenile mortality but eventual senescent decline. These patterns emerge from the interaction between fecundity (how much energy each offspring receives), kappa (the allocation parameter controlling how much mobilised energy goes to soma vs reproduction, which determines how fast the agent ages), and extrinsic mortality (predation and competition). The coexistence of different survivorship strategies within the same world is a sign of healthy niche differentiation — r-strategists and K-strategists exploiting different environmental contexts.

### Reproductive mode variation

Both sexual and asexual reproduction occur, with the balance depending on population density, mate availability, and each lineage's evolved asexual propensity. Asexual propensity is an evolvable trait — lineages can evolve toward or away from asexual reproduction over generations. A lineage that has lost asexual capability (low asexual propensity) cannot reproduce without mates, making it vulnerable to extinction in sparse conditions. In dense populations with coevolutionary pressure, sexual reproduction dominates because the variation it generates is competitively valuable. In sparse populations or for colonisers of empty habitat, lineages with high asexual propensity have an advantage because mate-finding is impossible or too costly. The system does not prescribe which mode wins — the balance shifts dynamically as population density, spatial structure, selective pressure, and evolved reproductive strategy change.

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

Free energy trends to zero. Energy accumulates in carcass structure and is not returned to the living system. This occurs when decomposer strategies are not viable — either the cost of decomposition exceeds its energy return, or decomposer populations cannot sustain themselves. The living system starves while energy sits locked in dead matter. Its nutrient-side sibling, **nutrient lockup**, is the same dead-pool sequestration measured on the nutrient pool rather than the energy pool; the two often co-occur but need not (see below).

### Nutrient lockup

Nutrient sequesters irreversibly into the dead pool. The conserved system nutrient — which cycles between the substrate grid, living agents, and carcasses — silts up in carcasses faster than the living decomposers can turn it over, on a rising trend that does not reverse. Producers cannot reproduce without nutrient *even when solar flux provides ample energy*, so the living system can starve for nutrient while its energy budget looks healthy. This is the distinction from energy death: a world whose producers photosynthesise fine (free energy does not collapse) can still lock its nutrient away, because the decomposers that would return it are absent, out of reach, or non-viable. It is the pathology the decomposer role exists to prevent ([trophic roles](../ecology/trophic-roles.md), [world rules](world-rules.md): "a world without decomposers accumulates resources in the dead pool until the living system starves"). The canonical case is a producer→carcass front whose carcass rain falls outside any decomposer's reach (`scenarios/example9_detrital_pathway.json`): energy flow is sustained, but nutrient accumulates unconsumed in the dead pool.

### Population explosion

Unbounded growth. No force limits the system. This occurs when the negative feedbacks (metabolic costs, trophic loss, competition for the energy source) are too weak relative to the energy input rate. Populations grow until the simulation runs out of resources.

### Frozen dynamics

Agents survive but nothing changes. Populations sit at fixed values, no new strategies emerge, demographic turnover drops to near zero. The system has persistence but no process. This occurs when all positive feedbacks are suppressed — reproductive amplification is too slow, oscillations are too heavily damped, or the fitness landscape is too flat for selection to operate.

### Generalist dominance

Agents that invest in all acquisition strategies simultaneously outcompete specialists. Diversity collapses from above — not because one specialist wins, but because a jack-of-all-trades beats every specialist in their own niche. The distinction that matters is *dominance*, not *existence*: real ecosystems carry persistent generalists — mixotrophic protists that photosynthesise and graze, mycorrhizal fungi that both decompose and trade with living partners, insectivorous plants — and a healthy world should too. They persist precisely where the capabilities they combine are physically *compatible* (both sessile, say). Generalist dominance is the failure where a generalist is not merely possible but unbeatable everywhere.

Two forces keep generalists confined to a niche rather than dominating it, and **per-trait maintenance convexity is not one of them** — with independent additive convex costs, spreading a fixed investment across traits is no more expensive than concentrating it, so that mechanism is breadth-neutral (see [world rules](world-rules.md), cost structure, trade-off #5). The forces that actually penalise breadth are **functional incompatibility** (capabilities whose physical demands conflict — autotrophy's substrate-anchored stillness against mobility's movement — cannot both be expressed well by one body) and **structural fragility** (a broad, high-entropy body dies having lost a smaller fraction of its peak structure, so generalists are easier to kill and less buffered against starvation). If generalist dominance is observed, the cause is that these are too weak — or the environment offers too little niche differentiation; the reserve remedy is to commit a cross-trait interaction maintenance term (a cost in the product of co-invested traits), generalising the autotrophy–mobility steepening the world rules already name, rather than steepening the breadth-neutral per-trait costs.

## Relationship between properties and failure modes

| Expected property | Failure mode when absent |
|---|---|
| Trophic structure | Monoculture or generalist dominance |
| Population oscillations | Frozen dynamics |
| Trait-space clustering | Generalist dominance or monoculture |
| Coexistence | Monoculture |
| Demographic turnover | Frozen dynamics or extinction |
| Age structure and survivorship variation | Immortal agents (no wear) or uniform lifespan (no variation in kappa allocation) |
| Reproductive mode variation | Obligate sexuality preventing colonisation, or obligate asexuality preventing adaptation |
| Energy throughput | Energy death or frozen dynamics |
| Energy conservation | Implementation bug (not a calibration issue) |

## How we test for these properties

The properties and failure modes above are claims about a calibrated world. Three complementary lenses test whether a given system design actually meets them, ordered from cheapest to most expensive:

1. **A priori viability** ([viability.md](viability.md)) — nearly free closed-form analysis of the tick-loop map. It rules out parameterizations that *cannot* produce these properties (e.g. the nutrient floor below which no agent can both exist and reproduce) before any simulation runs, and so pre-filters the genesis search.
2. **Diagnostic examples** (`scenarios/`) — a hand-built initial condition that probes a specific failure mode, run headless to observe whether the mode occurs or the system recovers.
3. **Genesis search** — the broad, expensive empirical sweep across parameter space (ensembles of replicate runs).

### Slow sweeps carry a `slow_` marker

The diagnostic-example tests are multi-seed behavioural **sweeps** — each runs a scenario to completion across a spread of seeds, so they dominate the suite's wall-clock cost. They are first-class tests and stay in the default run (they are **not** `#[ignore]`d), but each is tagged with a `slow_` name prefix so the slow sweeps form a selectable category:

- `cargo test --workspace` — runs everything, sweeps included.
- `cargo test slow_` — run only the slow sweeps.
- `cargo test -- --skip slow_` — the tight inner loop: everything except the sweeps (sub-second), leaving the cheap correctness regressions.

Any new slow sweep should adopt the `slow_` prefix to join the category for free; cheap correctness regressions stay unprefixed.

The **example file is the connective tissue**: it is the one concrete object the three lenses share. The same initial condition can be *predicted* by viability, *observed* by a simulation run, and *located* relative to the search — so each example is a unit of cross-validation where an a priori prediction meets an empirical outcome on identical ground. Agreement raises confidence; disagreement localises the fault (a wrong gate, an incomplete gate set, or a miscalibrated rule). For the loop to close, an example should carry the failure mode it probes, viability's prediction, and the observed outcome — not only its initial condition.

### By-construction examples test wiring, not emergence

A diagnostic example can be built two ways, and the difference is load-bearing. An example that sets up an initial condition and *lets* a property appear — or fail to — is evidence about the rules. An example that *guarantees* the property by geometry — seeding agents and nutrients so that, say, detrital share must exceed half by construction — tests only that the mechanism is wired and reachable. It is a plumbing check, not evidence that the role emerges. The trophic roles, decomposers included, are *emergent* outcomes of the economics: correct growth efficiency and an available nutrient pool are enough for detritivore guilds to arise even from random founders, so a decomposer is not a ghost mechanism that has to be hand-placed. A hand-built decomposer scenario therefore earns its keep only as a wiring/viability test; showing the role *emerges* is genesis search's job. The criterion that holds the rules to that claim is **not** the trophic-balance score — that reads the producer-vs-consumer energy share and is blind to the dead-vs-living distinction, because decomposer-ness is not in the trait vector to score (see [trait-space](trait-space.md), "Decomposer is a behavioural role, not a heritable trait"). It is the **energy-death failure gate**: a world whose detrital pathway fails to form locks matter in carcasses, free energy trends to zero, and fitness collapses to zero, so the search culls it (see "Energy death" above). A behavioural role with no axis to reward can only be selected *negatively* — by eliminating the worlds where it fails to appear. The emergence this produces is **distributional**: confirmed across seed ensembles, sporadic per-seed, never guaranteed on a single run — which is the expected signature of an emergent role, not a weakness. A property that only ever appears by construction has not been shown to emerge — and a scaffold that forces it can mask a search that produces nothing but the failure mode (e.g. extinction) on its own.

## What this document does not prescribe

This document describes *what* a working world looks like, not *how* to achieve it. The mechanisms that produce these properties are described in the world rules. The specific parameter values that calibrate the rules correctly are an implementation concern — they live with the code and the genesis search that tunes them, not here.

Disturbance, succession, spatial refugia, patch dynamics — these are patterns that ecologists observe in well-functioning ecosystems. They are not separate mechanisms to design. They arise from the same stocks, flows, and trade-offs described in the world rules, playing out across different timescales and spatial scales. If the world rules are right, these patterns should emerge. If they don't, the problem is in the rules, not in the absence of a disturbance mechanism.

## Initial conditions and world generation

The expected properties above describe a world in motion — agents interacting under the world rules, producing dynamic ecology. But agents do not evolve into a void. They evolve into a **substrate**: the physical medium of the world, with spatially heterogeneous nutrient distributions and terrain properties, shaped by processes far longer than any agent-evolution timescale.

On Earth, the substrate is the product of 4.5 billion years of geology — tectonic uplift, erosion, atmospheric chemistry, hydrological cycles. The spatial distribution of nutrients in rock and soils, water in river systems, and sunlight across latitudes is what creates the niche heterogeneity that supports diverse ecologies. Agents evolved into a world that was already richly structured.

### Two-phase world creation

> **Status:** Not yet implemented — see [#250](https://github.com/samthorold/explorers/issues/250). Current implementation has a `NutrientGrid` but no separate substrate-validation step.

World creation is a two-phase process:

1. **Substrate generation.** The physical medium is procedurally generated — nutrient distributions (both available and unavailable pools), terrain properties, and spatial structure. This phase compresses geological history into an algorithm. The substrate is generated and validated independently of any agents.

2. **Agent genesis.** Agents are seeded onto a validated substrate. World parameters and initial trait distributions are searched through ensemble runs (as described in the existing genesis process). A parameterisation is accepted only when most runs on the given substrate produce sensible worlds.

Separating these phases allows independent validation: a substrate can be assessed for spatial heterogeneity and nutrient adequacy before agents are introduced. If genesis fails, the cause (bad substrate or bad agent parameters) can be diagnosed.

### Substrate failure modes

Certain substrate configurations make it very difficult — or impossible — for the expected ecological properties to emerge:

- **Homogeneous substrate.** If nutrient is uniformly distributed, there is no spatial reason for different strategies to succeed in different places. Coexistence and functional differentiation are harder to achieve because there is less context-dependent fitness variation.
- **Extreme nutrient poverty.** If the substrate contains too little nutrient, the living system cannot sustain demographic turnover and population oscillations — agents cannot reproduce due to nutrient limitation everywhere.
- **Extreme nutrient abundance.** If nutrient is superabundant everywhere, stoichiometric constraint ceases to differentiate strategies. The specialist-generalist trade-off weakens because there is no nutrient limitation to penalise inefficiency.
- **Spatial monotony.** If the substrate has no variation in nutrient availability or terrain properties, the spatial mosaic of successional stages and niche specialisation has no substrate-level driver.

The world rules describe the physics. The expected properties describe what a healthy ecology looks like. The substrate determines whether the physics has enough raw material for the ecology to emerge. How the substrate is generated (procedural algorithms, geological simulation, or hand design) is an implementation concern, but the properties it must have are a system design concern: **the substrate must provide spatially heterogeneous nutrient availability sufficient to create context-dependent fitness variation across the surface.**

### Open questions

**Terrain properties.** The substrate has properties beyond nutrient content — candidates include elevation/topography, accessibility modifiers, light variation, and moisture. Which terrain properties matter and how they affect agent interactions is an open design question.
