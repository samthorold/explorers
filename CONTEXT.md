# Explorers

An ecology-driven game where a foreign entity navigates an alien world of interconnected, adaptive agents. No explicit rules. The emotional arc moves from danger and confusion to wonder and co-existence. Exploitation leads to ruin; symbiosis leads to success.

## Language

### Core

**Agent**:
The fundamental unit of the simulation. Everything in the world is an agent: organisms, carcasses, the player. There is no inert backdrop and no fixed types — an agent's role (producer, herbivore, decomposer) is a human interpretation of its position in trait space.
_Avoid_: entity, creature, organism (when referring to the simulation abstraction)

**Trait vector**:
A multi-dimensional vector of continuous values that defines an agent's identity and determines all behaviour. Genesis-minimal dimensions (8): photosynthetic absorption, consumption rate, scavenging rate, mobility, chemotaxis sensitivity, mate selectivity, sensing range, reproductive investment. Deferred dimensions: social weight (herding — emergent flavour, not structurally necessary for genesis criteria), chemical signature (player interaction and species recognition, not needed for genesis fitness).
_Avoid_: stats, attributes, genome (genome implies a genotype/phenotype distinction that doesn't exist here)

**Energy**:
The universal currency of the simulation. Enters the world only through solar flux. Flows between agents through consumption, reproduction, and decomposition. Energy conversion is lossy at every trophic transfer — consumers capture only a fraction of the energy they drain (per Lindeman 1942's trophic efficiency principle). The remainder is dissipated. Metabolic cost also dissipates energy. The system is open: solar flux is the sole tap, metabolic dissipation and transfer loss are the drains. Carrying capacity and trophic pyramid structure emerge from this energy budget rather than being imposed.
_Avoid_: health, mana, resources

**Tick**:
The discrete time step of the simulation. Each tick, all agents sense their neighbourhood, select actions, and update state.

### Energy flow

**Solar flux**:
A continuous, uniform energy field that all agents are exposed to. The sole external energy input to the world. An agent's photosynthetic absorption trait determines how much flux it converts to energy.
_Avoid_: sunlight, light level, radiation

**Consumption**:
An agent draining energy from a living agent over time through sustained contact. The consumer's consumption rate trait determines drain speed. The target survives unless its energy reaches zero — grazing is non-lethal by default.
_Avoid_: eating, attacking, harvesting

**Carcass**:
A dead agent. Retains the dead agent's energy, locked until a decomposer processes it. Emits a chemical signal detectable by agents with scavenging affinity. No passive decay — energy stays locked indefinitely without decomposition.
_Avoid_: corpse, remains, resource node

**Decomposition**:
An agent draining energy from a **carcass**. Functionally identical to **consumption** but targets carcasses rather than living agents, governed by the scavenging rate trait.
_Avoid_: recycling, decay (decay implies passive process)

**Metabolic cost**:
The energy an agent expends per **tick**. Derived from activity, not fixed: base cost scales with body size, plus costs for movement, sensing, and environmental mismatch (dormant until spatial gradients are added).
_Avoid_: upkeep, energy drain, maintenance

### Movement and sensing

**Chemotaxis**:
Movement biased toward a detected signal gradient. An agent's chemotaxis sensitivity trait determines how strongly it steers toward the signal source. Targets depend on what the agent can consume: producers emit signals attractive to consumers, carcasses emit signals attractive to scavengers.
_Avoid_: pathfinding, tracking, homing

**Social foraging**:
Attraction toward nearby feeding agents. An agent's social weight trait determines the strength of this pull. Combined with chemotaxis and a random exploration component to produce the agent's movement vector. Herding is an emergent property of high social weight across a local population.
_Avoid_: flocking, herding (as a mechanic — herding is emergent, not prescribed)

**Sensing range**:
The radius within which an agent detects other agents. An evolvable trait with metabolic cost — wider sensing costs more energy per tick. Signals are distance-weighted: closer agents produce stronger signals.
_Avoid_: vision, awareness radius

**Distance-weighted detection**:
The sensing model. Agents detect others within their sensing range, but signal strength falls off with distance. No persistent chemical field in the environment — gradients are computed from the positions of nearby emitters. A computationally cheap approximation of diffusion.

### Reproduction and evolution

**Sexual reproduction**:
Two agents within sensing range whose trait vectors are within a compatibility distance produce an offspring via budding. Both parents survive. Each parent invests energy according to their own reproductive investment trait. Offspring receives the sum of both investments scaled by reproduction efficiency (remainder dissipated). Offspring traits are produced by uniform crossover (each dimension independently selected from one parent, per Gavrilets 2004) plus Gaussian mutation. Uniform crossover is deliberately chosen over arithmetic mean (Dieckmann & Doebeli 1999) because recombination works against speciation — clusters that persist despite recombination are ecologically reinforced, not just reproductively isolated.
_Avoid_: mating, breeding (too specific to animal analogues)

**Mate selectivity**:
A trait dimension controlling the maximum trait distance at which an agent will reproduce with another. High selectivity = narrow compatibility = stronger speciation pressure.
_Avoid_: choosiness, pickiness

**Reproductive investment**:
A trait dimension controlling how much energy a parent transfers to offspring at birth. High investment = fewer, fitter offspring (K-strategy). Low investment = many, fragile offspring (r-strategy).
_Avoid_: brood size, litter size

**Speciation**:
The divergence of agent populations into distinct clusters in trait space that no longer interbreed due to trait distance exceeding mate selectivity thresholds. Not designed — emerges from selection and reproductive dynamics.
_Avoid_: species (as a designed concept — there are no species definitions, only emergent clusters)

### World parameters

**World parameters**:
The constants that define the physics of the simulation. Searched by genesis across ensemble runs. Not visible to agents, not evolvable. Distinct from the trait vector, which evolves within a run.

**Solar flux magnitude**:
Energy input rate per tick per unit photosynthetic absorption. The sole tap — controls how much energy enters the system.

**Consumption efficiency**:
Energy transferred per tick per unit consumption rate on sustained contact with a living agent.

**Decomposition efficiency**:
Energy transferred per tick per unit scavenging rate on contact with a carcass.

**Base metabolic rate**:
Energy cost per tick, scaled by activity (body size, movement, sensing). The sole drain alongside reproduction.

**Movement cost coefficient**:
Energy cost per unit distance moved per tick. Makes mobility expensive — creates the core trade-off between sessile photosynthesis and mobile consumption.

**Sensing cost coefficient**:
Energy cost per tick per unit sensing range. Makes wide awareness expensive.

**Reproduction efficiency**:
Fraction of energy invested by the parent that the offspring actually receives. The remainder is dissipated. Reproduction is lossy like all energy transfers.

**Reproduction energy threshold**:
Minimum energy an agent must have to attempt reproduction. Below this, agents prioritise survival.

**Mutation rate**:
Probability of each trait dimension mutating per reproduction event.

**Mutation magnitude**:
Standard deviation of the Gaussian perturbation applied to a mutated trait dimension.

**Contact radius**:
Distance threshold below which two agents are considered in contact. Governs consumption, decomposition, and reproduction. A world parameter, uniform for all agents.

**World extent**:
Spatial dimensions of the world. Interacts with population size to determine density. Toroidal topology during genesis (no edges, no boundary effects). Play-time topology — where the player can move beyond the genesis world — is a separate design problem (see future ADR).

**Initial population size**:
Number of agents at tick zero.

### Initial trait distribution

**Initial trait distribution**:
The starting configuration of agents in trait space. Searched by genesis alongside world parameters, but secondary — if world parameters are correct, many initial distributions should converge to sensible ecologies (the ensemble tests this).

**Mean trait vector**:
Centre of the initial population in trait space.

**Trait covariance**:
Whether initial trait dimensions are independent or correlated (e.g., high mobility correlated with high consumption). Controls whether the founding population starts as a single cloud or an elongated structure in trait space.

**Initial cluster count**:
Whether genesis seeds one uniform population or multiple pre-differentiated groups. Seeding multiple clusters tests whether the world parameters sustain diversity; seeding one tests whether differentiation emerges spontaneously.

**Initial energy per agent**:
Starting energy budget. Interacts with metabolic rate to determine how long agents survive before they must acquire energy.

### Measurement

**Trait-distance distribution**:
The distribution of pairwise distances between all agents in trait space. A uniform population produces a unimodal distribution. A population with emergent clusters produces a multimodal distribution — short within-cluster distances and long between-cluster distances. The primary tool for detecting whether clustering exists.

**Dip test**:
A statistical test for multimodality in a distribution (Hartigan & Hartigan 1985). Applied to the trait-distance distribution, it yields a single scalar (the dip statistic) indicating how strongly the population departs from unimodality. Parameter-free. Used as the accept/reject signal for trait-space clustering during world genesis.
_Avoid_: cluster count (the question during genesis is whether clustering exists, not how many)

**Cluster labelling**:
Identifying and tracking specific clusters in trait space over time. Performed via DBSCAN (density-based, no preset cluster count; Ester et al. 1996) once the dip test confirms clustering exists. Required for all downstream measurements: oscillation detection per cluster, coexistence duration between clusters, trophic pyramid by cluster energy. Variance-ratio / gap statistic (scalar measure of clustering strength vs. uniform expectation) is an alternative approach.

### Player

**Player**:
A foreign entity dropped into the world. Has a mutable trait vector that shifts through interactions. Starts as an outsider; the world reacts to them based on trait compatibility.
_Avoid_: character, avatar

**Touch**:
Direct physical contact between the player and another agent. The strongest form of interaction. Modifies both parties' traits.

**Presence**:
The passive effect of the player being near agents. A weaker form of interaction. Strength depends on the agent's chemotaxis sensitivity and the player's chemical signature.

**Field of view**:
The player perceives only their local surroundings. Once they move on, they lose sight of that area, but the simulation continues there. No persistent map.

**Symbiosis**:
Interactions where trait compatibility produces mutual benefit. The ecology responds to a symbiotic player by offering more interactions, approaching rather than fleeing, stabilising locally.

**Exploitation**:
One-sided interactions that benefit the player at an agent's expense. The ecology responds defensively: organisms withdraw symbiotic responses, activate defensive behaviours, and the player becomes increasingly isolated.

### Simulation

**World genesis**:
The process of generating a playable world. A parameterisation is evaluated as an ensemble of replicate runs (same parameters, different random seeds). Each run simulates a random initial population forward (off-screen). Degenerate runs are detected and terminated early. A parameterisation is accepted only when most runs in the ensemble produce sensible worlds. The player drops into a world with history.

**Degenerate configuration**:
A simulation outcome that fails to produce a functioning ecology. Six canonical failure modes: extinction (all agents die), monoculture (trait space collapses to a single cluster), energy death (free energy trends irreversibly toward zero), population explosion (unbounded growth), frozen dynamics (no turnover despite agents surviving), generalist dominance (one or more clusters with high values across multiple energy-acquisition traits outcompete specialists — indicates missing trade-off pressure in the model).
_Avoid_: bad run, failed world (too vague)

**Sensible world**:
A world that exhibits the positive patterns expected of a functioning ecology. Primary criteria: endogenous population oscillations between trophic levels (Lotka & Volterra; verified in ABMs by DeAngelis & Grimm 2014), trait-space clustering with gaps (emergent speciation), and coexistence duration (multiple clusters persisting simultaneously over extended periods, per Chesson 2000's coexistence theory). Secondary criteria (sanity checks): trophic pyramid (energy decreasing at higher trophic levels, Lindeman 1942) and demographic turnover (non-trivial birth and death rates). Evaluation follows the pattern-oriented modelling approach (Grimm et al. 2005): a parameterisation is accepted only when multiple independent patterns are reproduced simultaneously across an ensemble of runs.
_Avoid_: balanced world, stable world (stability is not the goal — dynamic persistence is)

**World recipe**:
The output artifact of world genesis. A combination of **world parameters**, an **initial distribution**, and a **max ticks** count — the minimal specification needed to deterministically create a world given a seed. Does not contain a seed (each playthrough generates a fresh one). Max ticks is the tick count at which genesis certified the ecology as sensible — the app fast-forwards to this point before the player enters. The simulation is deterministic: the same recipe + seed always produces the same world history.
_Avoid_: save file, world state, snapshot (a recipe is instructions for creating a world, not a captured moment of one)

**Death**:
When an agent's energy reaches zero. The agent becomes a **carcass**.

## Example dialogue

> **Dev:** An agent with high photosynthetic absorption and low mobility — is that a plant?
>
> **Domain:** We'd say it's occupying a producer-like niche in trait space. There are no plants. If it also has moderate scavenging rate, it's a producer that supplements with decomposition — something that doesn't map to any Earth category.
>
> **Dev:** What happens when a herbivore eats a producer?
>
> **Domain:** A consumer drains energy from it through consumption. The producer doesn't die unless its energy hits zero — it can recover if the consumer moves on. If it does die, it becomes a carcass, and its energy is locked until a decomposer finds it.
>
> **Dev:** What if all the decomposers die out?
>
> **Domain:** Energy accumulates in carcasses with no way back into the flux. Producers starve for lack of recycled energy — wait, no. Producers get energy from solar flux directly. But the total energy in the living system shrinks as more gets locked in carcasses. Eventually populations decline because there's less free energy circulating. It's a slow death, not a sudden collapse.
>
> **Dev:** How do species form?
>
> **Domain:** They don't — not by design. Agents reproduce sexually with a trait-distance compatibility check. Over time, clusters form in trait space that stop interbreeding because their trait distance exceeds their mate selectivity threshold. We call that speciation, but there's no species registry. It's emergent.

## Design decisions

### The world is a complex adaptive system

The world is an agent-based model in the tradition of computational ecology (Grimm & Railsback 2005, *Individual-based Modeling and Ecology*; DeAngelis & Grimm 2014). All entities are heterogeneous, adaptive agents interacting locally in 2D continuous space. Population dynamics emerge from individual agent behaviour — there is no top-down control of species populations or ecosystem balance.

### Earth-analogous ecology

The simulation follows real ecological principles: energy conservation, trophic levels (Lindeman 1942), nutrient cycling, carrying capacity. The forms may be alien but the dynamics are grounded. This allows us to draw on established ABM literature and ecological models (the ODD protocol, Grimm et al. 2006, 2010, provides the standard description format; pattern-oriented modelling, Grimm et al. 2005, provides the validation approach). Alien mechanics can be layered on once the base ecology produces coherent dynamics.

### No explicit rules for the player

The player receives no tutorial, no HUD, no rule explanations. The world communicates through visible agent behaviour — movement patterns, colour, shape, pulsation, spatial relationships. The player learns by observing and interacting.

### No goal, no win state

The game is open-ended. There is no objective, quest, or ending. The experience is being in this world — watching its dynamics, participating in its ecology, deepening understanding. Engagement comes from the world being intrinsically interesting, not from extrinsic rewards.

### Observation is participation

The player can never passively observe. Their presence affects nearby agents based on trait-dependent sensitivity. Even standing still is an interaction. The player is always a disturbance — the question is whether they become a welcome one.

### The ecology is robust, the player is fragile

Exploitation does not collapse the ecosystem. The world was functioning before the player arrived and will continue after they die. Exploitative play causes the local ecology to withdraw — defensive responses activate, symbiotic offers cease, and the player is left isolated with no way to sustain themselves. The punishment for exploitation is exclusion, not apocalypse.

### Player verbs: move and touch (v1)

The initial verb set is minimal: move through the world and make physical contact with agents. Richer verbs (carry, transform) may emerge through symbiotic relationships in future iterations — the world grants capability as the player integrates.

### No environmental cycles (v1)

No imposed day/night or seasonal cycles initially. All temporal patterns emerge from agent interactions. Environmental oscillators can be added later if the simulation lacks rhythm.

### Visual behaviour language

Agent traits map to visual properties. Interactions produce visible effects. The player learns to read the ecology by watching how things look and behave. No text, no numbers, no labels.
