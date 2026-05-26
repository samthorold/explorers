# Explorers

An ecology-driven game where a foreign entity navigates an alien world of interconnected, adaptive agents. No explicit rules. The emotional arc moves from danger and confusion to wonder and co-existence. Exploitation leads to ruin; symbiosis leads to success.

## Language

### Core

**Agent**:
The fundamental unit of the simulation. Everything in the world is an agent: organisms, carcasses. There is no inert backdrop and no fixed types — an agent's role (producer, consumer, decomposer) is derived from its position in trait space. The derivation has mechanical consequences: trait values determine which capabilities an agent can exercise, but the labels are always a reading of the trait vector, never an assigned type. The trait budget constraint (L1, sum to 1.0 across budget traits) ensures that investing in one capability reduces others, driving role differentiation.
_Avoid_: entity, creature, organism (when referring to the simulation abstraction)

**Trait vector**:
A multi-dimensional vector of continuous values that defines an agent's identity and determines all behaviour. The vector has two tiers. **Budget traits** (7 dimensions) sum to 1.0 (L1 budget constraint) — investing more in one capability necessarily reduces others. This shared budget is the primary mechanism that prevents generalist dominance and drives specialisation. Budget traits: photosynthetic absorption, consumption rate, scavenging rate, nutrient absorption, mobility, chemotaxis sensitivity, somatic maintenance. **Unconstrained traits** (4 dimensions) govern reproduction strategy and perception, not resource acquisition: mate selectivity, sensing range, reproductive investment, fecundity. These are not under the L1 budget — reproduction strategy is a separate axis from feeding strategy. Deferred dimensions: social weight (herding — emergent flavour, not structurally necessary for genesis criteria), chemical signature (species recognition, not needed for genesis fitness).
_Avoid_: stats, attributes, genome (genome implies a genotype/phenotype distinction that doesn't exist here)

**Substrate**:
The physical medium of the world — what agents live on and in. Holds nutrients in spatially heterogeneous distributions. Has material properties beyond nutrient content (terrain characteristics that affect agent interactions — specific properties are an open design question). Generated procedurally before agents are seeded. In ecological literature, substrate consistently refers to the physical medium or material, not the nutrients themselves.
_Avoid_: terrain (too specific to one property), environment (too broad), map (implies player-facing representation)

**Energy**:
The universal currency of the simulation. Enters the world only through solar flux. Flows between agents through consumption, reproduction, and decomposition. Energy conversion is lossy at every trophic transfer — consumers capture only a fraction of the energy they drain (per Lindeman 1942's trophic efficiency principle). The remainder is dissipated. Metabolic cost also dissipates energy. The system is open: solar flux is the sole tap, metabolic dissipation and transfer loss are the drains. Carrying capacity and trophic pyramid structure emerge from this energy budget rather than being imposed. Energy exists in two forms within a living agent: **reserve** and **structure**.
_Avoid_: health, mana, resources

**Reserve**:
An agent's metabolic fuel — the operating account through which all energy flows. Photosynthesis and decomposition income enter as reserve. Metabolic costs, growth, and reproduction are paid from reserve. Reserve fluctuates each tick as income and costs are applied. Death occurs when reserve reaches zero (starvation). Distinct from structure: reserve is the fuel gauge, not the body.
_Avoid_: energy (when specifically meaning the metabolic balance), stamina

**Structure**:
An agent's embodied biomass energy — the physical body built up over its lifetime. Structure accumulates when an agent allocates reserve surplus to growth (a lossy conversion). Consumption by another agent drains the target's structure (eating the body). Death transfers structure to the carcass — this is what makes carcasses energy-rich and decomposers energetically viable. Death also occurs when structure drops below a complexity-dependent threshold: agents with trait budget spread across many dimensions (high complexity) are more fragile to structural damage than specialists concentrated in few traits. Grounded in DEB theory's (Kooijman 2010) distinction between reserve and structure.
_Avoid_: biomass (too overloaded in ecology), body size, HP

**Nutrient**:
A cycling resource that agents require alongside energy. Unlike energy, nutrients are conserved — they cycle between pools rather than flowing from source to sink. The system tracks a single nutrient alongside energy. An agent's nutrient demand (how much nutrient it needs per unit energy) is derived from its trait vector — more capable agents need more nutrient. Nutrient limitation blocks reproduction but does not impair other functions.
_Avoid_: resource (too generic), mineral (too specific to one real-world nutrient)

**Available pool**:
The portion of nutrient at a location that is biologically accessible — extractable by agents through nutrient uptake. Distinguished from the unavailable pool (locked in rock or occluded forms, accessible only on geological timescales). The available pool at a location is finite and shared proportionally among co-located agents attempting uptake.
_Avoid_: substrate (substrate is the physical medium, not the nutrient stock)

**Tick**:
The discrete time step of the simulation. Each tick, all agents sense their neighbourhood, select actions, and update state.

### Energy flow

**Solar flux**:
The sole external energy input to the world. Agents compete locally for flux — each producer shares available light with other producers within a **light competition radius**, weighted by photosynthetic absorption. An isolated producer receives full flux; a producer in a crowded area receives a fraction. The producer/consumer divide is not enforced by a gate on photosynthesis — it emerges from two structural constraints: the trait budget (investing in photosynthetic absorption leaves less for mobility and consumption) and contact-time nutrient uptake (mobile agents cannot extract nutrients from the substrate, so photosynthesis without nutrient access is a dead end for reproduction).
_Avoid_: sunlight, light level, radiation

**Consumption**:
An agent draining structure and nutrient from a living agent over time through sustained contact — the consumer is eating the target's body. The consumer's consumption rate trait determines drain speed. The drained structure enters the consumer's reserve (with trophic transfer loss). The consumer retains only the nutrient it needs (per its stoichiometric demand) and excretes excess immediately to the available pool. The target survives unless its structure drops below its complexity-dependent death threshold or its reserve reaches zero — grazing is non-lethal by default.
_Avoid_: eating, attacking, harvesting

**Carcass**:
A dead agent. Retains the dead agent's structure (embodied biomass energy) and nutrient, locked until a decomposer processes it. Emits a chemical signal detectable by agents with scavenging affinity. No passive decay — structure and nutrient stay locked indefinitely without decomposition. A carcass's energy content reflects the agent's accumulated structure at death — old, well-fed agents leave energy-rich carcasses; heavily grazed or starved agents leave energy-poor ones.
_Avoid_: corpse, remains, resource node

**Decomposition**:
An agent draining structure and nutrient from a **carcass**. Functionally identical to **consumption** but targets carcasses rather than living agents, governed by the scavenging rate trait. The extracted structure enters the decomposer's reserve (with trophic transfer loss). Nutrient that exceeds the decomposer's stoichiometric demand is excreted immediately to the available pool, closing the nutrient cycle.
_Avoid_: recycling, decay (decay implies passive process)

**Nutrient uptake**:
An agent extracting nutrient from the **available pool** at its location. The only way nutrient enters the living system. Rate follows Michaelis-Menten saturation: `nutrient_absorption × contact_time / (contact_time + k)`, where k is a half-saturation constant (50 ticks). Uptake increases with contact time but plateaus — diminishing returns on long residence. Moving resets contact time. This creates a physical basis for the producer/consumer divide: sessile agents accumulate contact time and extract nutrients efficiently; mobile agents cannot.
_Avoid_: feeding (feeding is consumption of other agents), mining (implies deliberate extraction from rock)

**Contact time**:
The number of consecutive ticks an agent has spent at its current location. Governs nutrient uptake effectiveness — longer contact time means more effective extraction from the available pool. Resets when the agent moves. This is a world physics principle: sustained contact with the substrate is required to establish the interface structures (analogous to roots) needed for nutrient extraction.
_Avoid_: residence time (ecological term with different meaning — how long a nutrient stays in a pool)

**Stoichiometric mismatch**:
The difference between the nutrient ratio in an agent's food and the ratio the agent needs. When a consumer eats prey whose nutrient ratio doesn't match its demand, it retains only what it needs and excretes the excess nutrient immediately to the available pool. The limiting currency (energy or nutrient) constrains how much of the consumed material the consumer can actually use.
_Avoid_: waste, inefficiency (mismatch is not inefficiency — it is a physical constraint)

**Gross primary production**:
The total photosynthetic income across all producers per tick — the sum of all photosynthesis (flow 1) in the system. The raw energy tap before producers pay their own costs. Sets the ceiling on how much energy the rest of the ecosystem can access.
_Avoid_: GPP (abbreviation obscures meaning in a glossary)

**Autotrophic respiration**:
The total metabolic cost of all producers per tick. Producers pay to exist — base metabolism, trait maintenance, somatic maintenance, sensing — before any energy is available to consumers or decomposers. A producer population that barely covers its own costs leaves nothing for the rest of the food web regardless of how much solar flux enters the system.
_Avoid_: producer overhead, self-consumption

**Net primary production**:
Gross primary production minus autotrophic respiration. The energy actually available to consumers, decomposers, and the detrital pool — the portion of photosynthetic income that producers do not spend on themselves. This is the quantity that constrains whether consumer strategies are viable: if net primary production is low relative to consumer metabolic costs, no amount of consumption efficiency or predation skill makes consumers energy-positive. The trophic pyramid sits on this base.
_Avoid_: NPP (abbreviation), surplus (implies waste — NPP is the functional output of the producer base)

**Metabolic cost**:
The energy an agent expends per **tick**. Comprises a base rate, plus costs for movement, sensing, trait maintenance, and **somatic maintenance**. Each energy-acquisition trait (photosynthetic absorption, consumption rate, scavenging rate, nutrient absorption) costs energy to maintain whether used or not. Agents that carry traits they never exercise pay for the biological machinery, creating selection pressure toward specialisation. Somatic maintenance — repairing accumulated **wear** — is an additional cost that competes directly with reproduction. Metabolism costs energy only — nutrients are not released as metabolic waste. Nutrients leave living agents only through death.
_Avoid_: upkeep, energy drain, maintenance

**Somatic wear**:
The cumulative degradation of an agent's functional traits over its lifetime. Each functional trait (the seven budget traits excluding somatic maintenance) accumulates wear independently through two mechanisms: baseline degradation proportional to the trait's magnitude (complex machinery degrades even when idle) and use-dependent degradation proportional to the trait's metabolic throughput that tick (active use produces damaging byproducts). Wear reduces a trait's effective output via exponential decay — small wear barely affects performance, but wear compounds and later increments are increasingly catastrophic. An aging producer captures less light. An aging consumer catches prey less efficiently. Behavioural traits (mate selectivity, reproductive investment, fecundity) do not wear — they are allocation parameters, not physical machinery. Offspring are born with zero wear. Grounded in the disposable soma theory (Kirkwood 1977).
_Avoid_: aging (too vague — wear is per-trait, not a single scalar), damage (implies a discrete event, not continuous accumulation)

**Somatic maintenance**:
A budget trait controlling how much energy an agent invests in repairing accumulated **wear**. Higher investment slows degradation across all functional traits — somatic maintenance is a whole-organism investment, not selective repair. Repair effectiveness decays exponentially with current wear — heavily degraded traits are harder to repair than lightly worn ones. The energy cost of somatic maintenance competes directly with reproduction — this is the core survive-vs-reproduce trade-off. High somatic maintenance produces long-lived agents that reproduce infrequently. Low somatic maintenance produces short-lived agents that reproduce early and often.
_Avoid_: healing, regeneration (these imply discrete repair events, not continuous investment)

### Movement and sensing

**Chemotaxis**:
Movement biased toward a detected signal gradient. An agent's chemotaxis sensitivity trait determines how strongly it steers toward the signal source. Targets depend on what the agent can consume: producers emit signals attractive to consumers, carcasses emit signals attractive to scavengers.
_Avoid_: pathfinding, tracking, homing

**Social foraging**:
Attraction toward nearby feeding agents. An agent's social weight trait determines the strength of this pull. Combined with chemotaxis and a random exploration component to produce the agent's movement vector. Herding is an emergent property of high social weight across a local population.
_Avoid_: flocking, herding (as a mechanic — herding is emergent, not prescribed)

**Sensing range**:
The radius within which an agent detects other agents. An evolvable trait with metabolic cost — wider sensing costs more energy per tick. Signals are distance-weighted: closer agents produce stronger signals. For sessile agents, sensing range also determines **spore dispersal** radius — the distance over which they can reproduce without physical contact.
_Avoid_: vision, awareness radius

**Distance-weighted detection**:
The sensing model. Agents detect others within their sensing range, but signal strength falls off with distance. No persistent chemical field in the environment — gradients are computed from the positions of nearby emitters. A computationally cheap approximation of diffusion.

### Reproduction and evolution

**Sexual reproduction**:
Two agents whose trait vectors are within a compatibility distance produce an offspring via budding. Both parents survive. Each parent invests energy according to their own reproductive investment trait. Offspring receives the sum of both investments scaled by reproduction efficiency (remainder dissipated). Offspring traits are produced by uniform crossover (each dimension independently selected from one parent, per Gavrilets 2004) plus Gaussian mutation. Uniform crossover is deliberately chosen over arithmetic mean (Dieckmann & Doebeli 1999) because recombination works against speciation — clusters that persist despite recombination are ecologically reinforced, not just reproductively isolated. Sexual reproduction requires physical contact on the surface — both agents must be within physical interaction range as determined by their traits.
_Avoid_: mating, breeding (too specific to animal analogues)

**Mate selectivity**:
A trait dimension controlling the maximum trait distance at which an agent will reproduce with another. High selectivity = narrow compatibility = stronger speciation pressure.
_Avoid_: choosiness, pickiness

**Reproductive investment**:
A trait dimension controlling how much energy a parent transfers to offspring at birth. High investment = fewer, fitter offspring (K-strategy). Low investment = many, fragile offspring (r-strategy).
_Avoid_: brood size, litter size

**Fecundity**:
An unconstrained trait controlling the number of offspring per reproductive event. A fixed total energy budget is invested per event; fecundity determines how many offspring share that budget. High fecundity produces many poorly-provisioned offspring (r-strategy). Low fecundity (or zero) produces few well-provisioned offspring (K-strategy). The actual offspring count is stochastic — drawn from a Poisson distribution with mean equal to the fecundity trait. For sexual reproduction, the effective fecundity is the average of both parents'. Because fecundity is part of the trait vector, it contributes to trait-space distance and therefore to mate compatibility — agents with very different reproductive strategies are less likely to mate.
_Avoid_: clutch size, litter size (too specific to animal analogues)

**Asexual reproduction**:
A universal fallback when an agent has sufficient energy to reproduce but no compatible mate is available. Offspring traits are the parent's traits plus mutation — no crossover, because there is no second parent. The costs are inherent: lower offspring variation (no recombination) and a single parent's energy contribution. In dense populations where mates are available, sexual reproduction is advantageous because it generates more combinatorial diversity. In sparse populations or for isolated colonizers, asexual reproduction is the only option. Whether a lineage relies primarily on sexual or asexual reproduction is emergent, not prescribed.
_Avoid_: cloning (implies exact replication — mutation still applies)

**Spore dispersal**:
A reproduction mechanism that bypasses the requirement for physical contact — the specific topology and mechanics are TBD.
_Avoid_: pollination (implies a specific biological mechanism)

**Speciation**:
The divergence of agent populations into distinct clusters in trait space that no longer interbreed due to trait distance exceeding mate selectivity thresholds. Not designed — emerges from selection and reproductive dynamics.
_Avoid_: species (as a designed concept — there are no species definitions, only emergent clusters)

### World parameters

**World parameters**:
The constants that define the physics of the simulation. Searched by genesis across ensemble runs. Not visible to agents, not evolvable. Distinct from the trait vector, which evolves within a run.

**Solar flux magnitude**:
Total energy available per tick within a **light competition radius**. Divided among local producers proportional to their photosynthetic absorption. The sole tap — controls how much energy enters the system.

**Light competition radius**:
The radius within which producers compete for solar flux. Producers outside this radius do not affect each other's energy intake. A world parameter searched by genesis. Interacts with world extent and population density to determine how crowded the light environment is.

**Base metabolic rate**:
Fixed energy cost per tick, independent of traits or activity. The floor of metabolic cost — trait maintenance, movement, and sensing costs are added on top.

**Trait maintenance cost**:
Energy cost per tick per unit of each energy-acquisition trait. Three coefficients (one each for photosynthetic absorption, consumption rate, scavenging rate) are world parameters searched by genesis. An agent with high consumption rate pays for maintaining the machinery to consume whether or not it finds prey. Drives evolutionary specialisation — generalists pay more overhead than specialists.

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

### Infrastructure

**Event**:
A record of something that happened in the simulation. Events are defined by a single enumeration — the vocabulary of the entire system. The event log is the source of truth for the simulation's causal history. Events record topology changes: births, deaths, trophic transfers, mate selections. Not a complete state record — full energy reconstruction requires replaying from seed.
_Avoid_: message, signal (events are facts, not communications)

**Broadcast**:
The notification mechanism of the discrete event simulation. When an event occurs, the DES delivers it individually to each agent within sensing range. Broadcast is infrastructure — the dispatch protocol, not an ecological concept. Two filtering layers: spatial (sensing range) and subscription (agents may NACK an event type to unsubscribe). Each agent responds with a struct containing an ACK/NACK and a vector of events. On ACK, the DES queues the returned events. On NACK, the DES ignores returned events and stops delivering that event type to that agent. NACKs are per-agent and do not inherit to offspring.
_Avoid_: using "broadcast" to describe ecological phenomena like spore dispersal

**Projection**:
A left fold over the event log that produces derived data for a specific consumer. Multiple projections can exist for different purposes (agent perception, genesis evaluation, replay tooling). Spatial projections decay over time and provide agents with navigational context. The DES computes projections and hands the data to agents alongside broadcasts — agents do not query projections themselves.
_Avoid_: view, cache (a projection is a derived model, not a performance optimisation)

### Simulation

**World genesis**:
The process of generating a playable world. A parameterisation is evaluated as an ensemble of replicate runs (same parameters, different random seeds). Each run simulates a random initial population forward (off-screen). Degenerate runs are detected and terminated early. A parameterisation is accepted only when most runs in the ensemble produce sensible worlds. The player drops into a world with history.

**Degenerate configuration**:
A simulation outcome that fails to produce a functioning ecology. Six canonical failure modes: extinction (all agents die), monoculture (trait space collapses to a single cluster), energy death (free energy trends irreversibly toward zero), population explosion (unbounded growth), frozen dynamics (no turnover despite agents surviving), generalist dominance (one or more clusters with high values across multiple energy-acquisition traits outcompete specialists — indicates missing trade-off pressure in the model).
_Avoid_: bad run, failed world (too vague)

**Sensible world**:
A world that exhibits the positive patterns expected of a functioning ecology. Five criteria evaluated via arithmetic mean: endogenous population oscillations between trophic levels (Lotka & Volterra; verified in ABMs by DeAngelis & Grimm 2014), trait-space clustering with gaps (emergent speciation), coexistence duration (multiple clusters persisting simultaneously over extended periods, per Chesson 2000's coexistence theory), demographic turnover (non-trivial birth and death rates), and trophic balance (energy decreasing at higher trophic levels, Lindeman 1942). Arithmetic mean is used rather than geometric mean because the genesis optimiser requires gradient signal even when some criteria score zero — geometric mean produces zero gradient in all directions when any single criterion is zero, creating a flat fitness landscape that the optimiser cannot descend. Evaluation follows the pattern-oriented modelling approach (Grimm et al. 2005): a parameterisation is accepted only when multiple independent patterns are reproduced simultaneously across an ensemble of runs.
_Avoid_: balanced world, stable world (stability is not the goal — dynamic persistence is)

**World recipe**:
The output artifact of world genesis. A combination of **world parameters**, an **initial distribution**, and a **max ticks** count — the minimal specification needed to deterministically create a world given a seed. Does not contain a seed (each playthrough generates a fresh one). Max ticks is the tick count at which genesis certified the ecology as sensible — the app fast-forwards to this point before the player enters. The simulation is deterministic: the same recipe + seed always produces the same world history.
_Avoid_: save file, world state, snapshot (a recipe is instructions for creating a world, not a captured moment of one)

**Death**:
When an agent's reserve reaches zero (starvation, predation) or its structure drops below its complexity-dependent threshold (structural damage from consumption). The agent becomes a **carcass**, retaining its remaining structure and nutrient.

## Example dialogue

> **Dev:** An agent with high photosynthetic absorption and low mobility — is that a plant?
>
> **Domain:** It's a producer — its trait budget is concentrated in photosynthetic absorption and nutrient absorption, leaving little for mobility or consumption. We derive that label from its traits, not assign it. Because it stays put, it accumulates contact time and extracts nutrients efficiently from the substrate. If it also has moderate scavenging rate, it's a producer that supplements with decomposition — but the trait budget means that scavenging investment comes at the cost of its other capabilities.
>
> **Dev:** What happens when a herbivore eats a producer?
>
> **Domain:** A consumer drains structure from it — eating the body. The drained structure enters the consumer's reserve with trophic loss. The producer doesn't die unless its structure drops below its death threshold or its reserve hits zero — it can recover if the consumer moves on, regrowing structure from reserve surplus. If it does die, its remaining structure and nutrient become a carcass, locked until a decomposer finds it.
>
> **Dev:** What if all the decomposers die out?
>
> **Domain:** Energy accumulates in carcass structure with no way back into the living system. Nutrient locks in carcasses too — and that's the real bottleneck. Producers get energy from solar flux directly, so energy input continues. But nutrient is conserved and cycling — every death locks nutrient in carcasses. Without decomposers to release it, the available pool depletes. Eventually producers can't reproduce even though they have plenty of energy. It's a slow death through nutrient starvation, not energy starvation.
>
> **Dev:** How do species form?
>
> **Domain:** They don't — not by design. Agents reproduce sexually with a trait-distance compatibility check. Over time, clusters form in trait space that stop interbreeding because their trait distance exceeds their mate selectivity threshold. We call that speciation, but there's no species registry. It's emergent.

## Design decisions

### The world is a complex adaptive system

The world is an agent-based model in the tradition of computational ecology (Grimm & Railsback 2005, *Individual-based Modeling and Ecology*; DeAngelis & Grimm 2014). All entities are heterogeneous, adaptive agents interacting locally in 2D continuous space. Population dynamics emerge from individual agent behaviour — there is no top-down control of species populations or ecosystem balance.

### Earth-analogous ecology

The simulation follows real ecological principles: energy conservation, trophic levels (Lindeman 1942), nutrient cycling, carrying capacity. The forms may be alien but the dynamics are grounded. This allows us to draw on established ABM literature and ecological models (the ODD protocol, Grimm et al. 2006, 2010, provides the standard description format; pattern-oriented modelling, Grimm et al. 2005, provides the validation approach). Alien mechanics can be layered on once the base ecology produces coherent dynamics.

### No environmental cycles (v1)

No imposed day/night or seasonal cycles initially. All temporal patterns emerge from agent interactions. Environmental oscillators can be added later if the simulation lacks rhythm.

### Visual behaviour language

Agent traits map to visual properties. Interactions produce visible effects. The player learns to read the ecology by watching how things look and behave. No text, no numbers, no labels.
