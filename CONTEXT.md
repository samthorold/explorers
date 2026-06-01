# Explorers

An ecology-driven game where a foreign entity navigates an alien world of interconnected, adaptive agents. No explicit rules. The emotional arc moves from danger and confusion to wonder and co-existence. Exploitation leads to ruin; symbiosis leads to success.

## Language

### Core

**Agent**:
The fundamental unit of the simulation. Everything in the world is an agent: organisms, carcasses. There is no inert backdrop and no fixed types — an agent's role (producer, consumer, decomposer) is derived from its position in trait space. The derivation has mechanical consequences: trait values determine which capabilities an agent can exercise, but the labels are always a reading of the trait vector, never an assigned type. Superlinear maintenance costs ensure that investing heavily across many traits is prohibitively expensive, driving role differentiation.
_Avoid_: entity, creature, organism (when referring to the simulation abstraction)

**Trait vector**:
A seven-dimensional vector of continuous values that defines an agent's identity and determines all behaviour. Organised in three layers. **Allocation** (1 dimension): kappa — the fraction of mobilised energy routed to soma vs reproduction. **Specification** (3 dimensions): autotrophy, heterotrophy, mobility — these determine how an agent acquires energy and moves through the world. Autotrophy and heterotrophy are independent dimensions, not a spectrum — an agent can invest in both, though superlinear maintenance costs make broad investment expensive. **Reproduction** (3 dimensions): fecundity, asexual propensity, dispersal. All six traits are independently evolvable, each carrying its own cost — there is no shared budget or constraint that forces them to sum to a fixed value. The anti-generalist mechanism is economic, not algebraic: every trait makes itself progressively more expensive as it grows — whether through a superlinear maintenance cost paid each tick or through an emergent trade-off that bites harder at higher investment — so spreading investment across many traits costs more than concentrating it. Some capabilities are subordinate to trait dimensions rather than independent traits: sensing is subordinate to mobility (mobile agents sense; sessile agents do not need to), nutrient uptake is implicit in autotrophy. Some quantities are derived from the trait vector rather than stored in it: mate selectivity emerges from trait-space divergence evaluated against a physics-defined **reproductive compatibility distance**, not from an explicit trait. Structure (body size) factors into light competition — bigger producers shade smaller ones. Deferred dimensions: social weight (herding — emergent flavour, not structurally necessary for genesis criteria), chemical signature (species recognition, not needed for genesis fitness).
_Avoid_: stats, attributes, genome (genome implies a genotype/phenotype distinction that doesn't exist here)

**Kappa**:
The DEB-derived allocation parameter (Kooijman 2010) governing the fraction of mobilised energy routed to soma versus reproduction. High kappa directs more energy to somatic maintenance and growth — producing long-lived, slow-reproducing agents. Low kappa directs more energy to reproduction — producing short-lived, prolific agents. Kappa is an evolvable trait, not a fixed parameter. Kappa governs how much energy is available for all somatic functions (maintenance, growth, activity costs) versus reproduction. Kappa is a **flow-allocation rule applied to surplus**, not a spend-time partition: in the grow phase, whatever surplus reserve an agent has after retaining for next-tick metabolism is split immediately — kappa × surplus is committed to soma (spent that tick on repair and growth) and (1 − kappa) × surplus is committed to a **reproductive allocation** that earmarks but does not separately store the energy. This DEB-style flow allocation decouples reproduction timing from current-tick reserve fluctuations: an agent accumulates its committed reproductive allocation over many ticks before any single reproductive event fires. The same kappa governs the parallel split of **nutrient**: the (1 − kappa) share of each tick's nutrient income — whether absorbed from the pool (autotrophs) or ingested by consumption (heterotrophs) — is earmarked as **reproductive nutrient** and the kappa share is left as free nutrient for growth to bind into structure. The split is route-agnostic, exactly as on the energy side. One allocation gene moves both currencies in lockstep — there is no separate nutrient-allocation trait.
_Avoid_: allocation ratio (too generic), maintenance trait (kappa governs more than maintenance), spend-time partition (kappa allocates flow at the moment of surplus, not at the moment of spending)

**Autotrophy**:
Investment in photosynthetic machinery — the specification trait governing an agent's capacity to convert solar flux into energy. Implicitly requires nutrient uptake from the substrate: the biological machinery for photosynthesis and nutrient extraction are coupled (analogous to leaves requiring roots). An agent with high autotrophy and low mobility is a producer. Autotrophy and heterotrophy are independent dimensions, not a spectrum — an agent can invest in both, though superlinear maintenance costs make this expensive.
_Avoid_: photosynthetic absorption (old term), plant-ness

**Heterotrophy**:
Investment in consumption machinery — the specification trait governing an agent's capacity to drain structure and nutrient from other agents, both living and dead. A single trait covers both predation and decomposition. Whether an agent functions as a consumer or decomposer depends on what it eats (living agents or carcasses), not on separate traits. Trophic transfer efficiency between consumer and target is governed by trait-space distance, not by distinct consumption and scavenging machinery.
_Avoid_: consumption rate, scavenging rate (old terms that implied separate machinery)

**Asexual propensity**:
An evolvable reproduction trait controlling an agent's capacity to reproduce without a mate. Not a universal fallback — agents with low asexual propensity cannot reproduce alone even when no compatible mate is available. High asexual propensity enables colonisation of empty niches and reproduction in sparse populations. The trait is subject to selection pressure: lineages in dense, well-mixed populations may evolve low asexual propensity because sexual reproduction provides recombination benefits, while isolated or pioneer lineages may evolve high asexual propensity.
_Avoid_: cloning ability, parthenogenesis (too specific to real-world mechanisms)

**Substrate**:
The physical medium of the world — what agents live on and in. Holds nutrients in spatially heterogeneous distributions. Has material properties beyond nutrient content (terrain characteristics that affect agent interactions — specific properties are an open design question). Generated procedurally before agents are seeded. In ecological literature, substrate consistently refers to the physical medium or material, not the nutrients themselves.
_Avoid_: terrain (too specific to one property), environment (too broad), map (implies player-facing representation)

**Energy**:
The universal currency of the simulation. Enters the world only through solar flux. Flows between agents through consumption and reproduction. Energy conversion is lossy at every trophic transfer — consumers capture only a fraction of the energy they drain (per Lindeman 1942's trophic efficiency principle). The remainder is dissipated. Metabolic cost also dissipates energy. The system is open: solar flux is the sole tap, metabolic dissipation and transfer loss are the drains. Carrying capacity and trophic pyramid structure emerge from this energy budget rather than being imposed. Energy exists in two forms within a living agent: **reserve** and **structure**.
_Avoid_: health, mana, resources

**Reserve**:
An agent's metabolic fuel — the operating account through which all energy flows. Photosynthesis and consumption income enter as reserve. Metabolic costs, growth, and reproduction are paid from reserve. Reserve fluctuates each tick as income and costs are applied. Death occurs when reserve reaches zero (starvation). Distinct from structure: reserve is the fuel gauge, not the body. Reserve has an earmarked sub-account, the **reproductive allocation**, which accumulates the (1 − kappa) share of surplus committed by kappa in the grow phase; it is still part of reserve (and still part of the agent's energy total for conservation purposes) but is reserved for reproduction and is not available to fund metabolism. Energy still exists in only two forms — reserve and structure — the reproductive allocation is a labelled fraction of reserve, not a third form.
_Avoid_: energy (when specifically meaning the metabolic balance), stamina

**Structure**:
An agent's embodied biomass energy — the physical body built up over its lifetime. Structure accumulates when an agent allocates reserve surplus to growth (a lossy conversion), and growth also **binds nutrient**: building a unit of structure consumes free nutrient from the agent's store and locks it into the body at the agent's stoichiometric ratio (`bound nutrient = structure × demand`). Growth is therefore **co-limited** — an agent builds structure only up to whichever of energy or free nutrient is scarcer (Liebig's law of the minimum), throttling smoothly toward zero as free nutrient runs out (no hard gate); energy that cannot be matched with nutrient stays in reserve rather than being burned. The nutrient bound in structure is matter — it is not available for anything else while the agent lives, and is released only when the structure is removed: grazed away (to the consumer) or returned to the carcass at death (to decomposers). Consumption by another agent drains the target's structure and the nutrient bound in it. Death transfers structure to the carcass — this is what makes carcasses energy-rich and decomposers energetically viable. Death also occurs when structure drops below a complexity-dependent threshold: agents with investment spread across many specification traits (high complexity) are more fragile to structural damage than specialists concentrated in few traits. Grounded in DEB theory's (Kooijman 2010) distinction between reserve and structure.
_Avoid_: biomass (too overloaded in ecology), body size, HP

**Nutrient**:
A cycling resource agents require alongside energy. Unlike energy, nutrient is conserved — it cycles between pools rather than flowing from source to sink — and it is **matter embodied in biomass**: a living agent holds nutrient in two places, a **free store** (unbound, available) and the amount **bound in its structure** (`structure × demand`, locked into the body until that structure is removed). An agent's demand (nutrient per unit structure) is derived from its trait vector — more capable agents need more. Within a living agent nutrient takes the same shape as energy: the free store mirrors **reserve**, and a **reproductive nutrient** earmark mirrors the **reproductive allocation**. Nutrient **co-limits growth** (growth binds free nutrient into the body, so it stalls when free nutrient runs out — Liebig's law of the minimum) and **gates reproduction** (a reproductive event requires the reproductive-nutrient earmark to have accumulated past the **reproduction nutrient threshold**). The earmark is fed the (1 − kappa) share of each tick's nutrient income — whatever the route, pool uptake (autotrophs) or consumption (heterotrophs) — and is off-limits to growth, so growth can never starve reproduction of nutrient, and reproduction is never starved by growth. Metabolism does not touch nutrient — nutrient leaves a living agent only by grazing (the bound portion of the structure eaten) or death (everything).
_Avoid_: resource (too generic), mineral (too specific to one real-world nutrient)

**Reproductive nutrient**:
The nutrient analogue of the **reproductive allocation** — an earmarked sub-account of an agent's free nutrient store, accumulated tick over tick from the (1 − kappa) share of each tick's nutrient income, whether absorbed from the pool (autotrophs) or ingested by consumption (heterotrophs). Off-limits to growth (growth binds only unearmarked free nutrient), it is the nutrient a parent provisions to offspring at a reproductive event, and the **reproduction nutrient threshold** gates reproduction on it. Mirrors reproductive allocation one-to-one: the same kappa fills both earmarks, both are donated to offspring and divided by the realised offspring count — so a high-fecundity agent under-provisions each offspring in *both* currencies (the Smith & Fretwell quantity/quality trade-off operating on both axes). Still part of the agent's nutrient total for conservation — a labelled fraction of the free store, not a separate pool.
_Avoid_: reproductive nutrient pool (it is an earmark, not a distinct stock)

**Available pool**:
The portion of nutrient at a location that is biologically accessible — extractable by agents through nutrient uptake. Distinguished from the unavailable pool (locked in rock or occluded forms, accessible only on geological timescales). The available pool at a location is finite and shared proportionally among co-located agents attempting uptake.
_Avoid_: substrate (substrate is the physical medium, not the nutrient stock)

**Tick**:
The discrete time step of the simulation. Each tick, all agents sense their neighbourhood, select actions, and update state.

### Energy flow

**Solar flux**:
The sole external energy input to the world. Agents compete locally for flux — each producer shares available light with other producers within a **light competition radius**, weighted by autotrophy. Structure (body size) factors into light competition — bigger producers shade smaller ones. An isolated producer receives full flux; a producer in a crowded area receives a fraction. The producer/consumer divide is not enforced by a gate on photosynthesis — it emerges from superlinear maintenance costs: investing heavily in both autotrophy and mobility is prohibitively expensive, and nutrient uptake is implicit in autotrophy (the same sessile infrastructure serves both), so mobile photosynthesis is an economic dead end.
_Avoid_: sunlight, light level, radiation

**Consumption**:
An agent draining structure and nutrient from a living agent over time through sustained contact — the consumer is eating the target's body. The consumer's heterotrophy trait determines drain speed. The drained structure enters the consumer's reserve (with trophic transfer loss). Only the nutrient **bound in the structure removed** transfers (proportional to structure drained); a living target's free nutrient store and reproductive-nutrient earmark are not touched — they stay with it and are released only at death. The consumer retains only the nutrient it needs (per its stoichiometric demand, spanning soma and reproduction) and excretes the excess immediately to the available pool; the retained nutrient is split by **kappa** exactly as pool uptake is — the (1 − kappa) share earmarked as **reproductive nutrient** — so a consumer provisions offspring from ingested nutrient, the heterotroph's route to the earmark. The target survives unless its structure drops below its complexity-dependent death threshold or its reserve reaches zero — grazing is non-lethal by default.
_Avoid_: eating, attacking, harvesting

**Carcass**:
An inert agent. A living agent becomes a carcass on death — it retains its structure (embodied biomass energy), all its nutrient (free store, reproductive earmark, and the nutrient that was bound in its structure), position, and trait vector, but no longer acts. Structure and nutrient stay locked until another agent consumes the carcass. No passive decay. A carcass's energy content reflects the agent's accumulated structure at death — old, well-fed agents leave energy-rich carcasses; heavily grazed or starved agents leave energy-poor ones.
_Avoid_: corpse, remains, resource node

**Decomposition**:
**Consumption** where the target is a **carcass**. Not a separate mechanism — the same heterotrophy trait and physics apply (trait-space distance governs trophic transfer efficiency, stoichiometric mismatch causes nutrient excretion). The term is useful because it names the ecological role: decomposers are agents whose trait-space position makes them efficient consumers of carcasses. Decomposition closes the nutrient cycle by returning carcass nutrient to the available pool.
_Avoid_: recycling, decay (decay implies passive process)

**Nutrient uptake**:
An agent extracting nutrient from the **available pool** at its location — the autotroph's route for nutrient to enter a living agent (heterotrophs acquire nutrient by consumption instead). Rate depends on the agent's autotrophy investment — the same sessile infrastructure (analogous to roots) that captures light also extracts nutrients from the substrate. In terrestrial ecology, nutrient extraction requires physical infrastructure embedded in the substrate — roots, hyphae, membrane surfaces — that is fundamentally incompatible with mobility. The expected property that mobile agents cannot efficiently extract nutrients emerges from the superlinear cost of investing in both autotrophy and mobility, not from a separate gating mechanism. Co-located agents share the available pool proportionally, weighted by their effective uptake rate. Each tick's uptake is split immediately by **kappa** — the (1 − kappa) share earmarked as **reproductive nutrient**, the remainder added to the free store available for growth.
_Avoid_: feeding (feeding is consumption of other agents), mining (implies deliberate extraction from rock)

**Stoichiometric mismatch**:
The difference between the nutrient ratio in an agent's food and the ratio the agent needs. When a consumer eats prey whose nutrient ratio doesn't match its demand, it retains only what it needs and excretes the excess nutrient immediately to the available pool. The limiting currency (energy or nutrient) constrains how much of the consumed material the consumer can actually use.
_Avoid_: waste, inefficiency (mismatch is not inefficiency — it is a physical constraint)

**Gross primary production**:
The total photosynthetic income across all producers per tick — the sum of all photosynthesis (flow 1) in the system. The raw energy tap before producers pay their own costs. Sets the ceiling on how much energy the rest of the ecosystem can access.
_Avoid_: GPP (abbreviation obscures meaning in a glossary)

**Autotrophic respiration**:
The total metabolic cost of all producers per tick. Producers pay to exist — base metabolism, trait maintenance (superlinear in each specification trait), somatic maintenance (governed by kappa) — before any energy is available to consumers or decomposers. A producer population that barely covers its own costs leaves nothing for the rest of the food web regardless of how much solar flux enters the system.
_Avoid_: producer overhead, self-consumption

**Net primary production**:
Gross primary production minus autotrophic respiration. The energy actually available to consumers, decomposers, and the detrital pool — the portion of photosynthetic income that producers do not spend on themselves. This is the quantity that constrains whether consumer strategies are viable: if net primary production is low relative to consumer metabolic costs, no amount of consumption efficiency or predation skill makes consumers energy-positive. The trophic pyramid sits on this base.
_Avoid_: NPP (abbreviation), surplus (implies waste — NPP is the functional output of the producer base)

**Metabolic cost**:
The energy an agent expends per **tick**. Comprises a base rate, plus costs for movement, sensing, trait maintenance, and somatic maintenance (governed by **kappa**). Each specification trait (autotrophy, heterotrophy, mobility) costs energy to maintain whether used or not — and each trait's maintenance cost grows superlinearly with its value, so agents that invest broadly pay disproportionately more than specialists. Agents that carry traits they never exercise pay for the biological machinery, creating selection pressure toward specialisation. Somatic maintenance — repairing accumulated **wear** — is funded from the soma fraction of mobilised energy as determined by kappa, competing directly with reproduction. Metabolism costs energy only — nutrients are not released as metabolic waste. Nutrients leave living agents only through death.
_Avoid_: upkeep, energy drain, maintenance

**Somatic wear**:
The cumulative degradation of an agent's functional traits over its lifetime. Each specification trait (autotrophy, heterotrophy, mobility) accumulates wear independently through use and through the baseline cost of maintaining complex machinery. Wear reduces a trait's effective output — small wear barely affects performance, but wear compounds and later increments are increasingly costly. An aging producer captures less light. An aging consumer drains prey less efficiently. The rate of wear accumulation and the shape of degradation emerge from the agent's trait-space position and activity, not from flat rate parameters. Allocation and reproduction traits (kappa, fecundity, asexual propensity) do not wear — they are allocation parameters, not physical machinery. Offspring are born with zero wear. Grounded in the disposable soma theory (Kirkwood 1977).
_Avoid_: aging (too vague — wear is per-trait, not a single scalar), damage (implies a discrete event, not continuous accumulation)

**Somatic maintenance**:
The energy an agent invests in repairing accumulated **wear**. Governed by **kappa** — the fraction of mobilised energy routed to soma determines how much energy is available for maintenance and growth. Higher kappa means more energy for somatic maintenance, slowing degradation across all specification traits — somatic maintenance is a whole-organism investment, not selective repair. The energy routed to soma competes directly with reproduction — this is the core survive-vs-reproduce trade-off. High kappa produces long-lived agents that reproduce infrequently. Low kappa produces short-lived agents that reproduce early and often. The balance between wear accumulation and repair determines an agent's lifespan — this balance must permit a stable equilibrium where agents are degraded but functional, not a guaranteed death spiral or effective immortality.
_Avoid_: healing, regeneration (these imply discrete repair events, not continuous investment)

### Movement and sensing

**Chemotaxis**:
Movement biased toward a detected signal gradient. An agent's mobility trait governs how strongly it can steer toward a signal source — chemotaxis is subordinate to mobility, not an independent trait dimension. Targets depend on what the agent can consume: producers emit signals attractive to consumers, carcasses emit signals attractive to decomposers.
_Avoid_: pathfinding, tracking, homing

**Social foraging**:
Attraction toward nearby feeding agents. An agent's social weight trait determines the strength of this pull. Combined with chemotaxis and a random exploration component to produce the agent's movement vector. Herding is an emergent property of high social weight across a local population.
_Avoid_: flocking, herding (as a mechanic — herding is emergent, not prescribed)

**Sensing range**:
The radius within which an agent detects other agents. Subordinate to mobility — not an independent trait dimension. An agent's effective sensing range derives from its mobility investment. Signals are distance-weighted: closer agents produce stronger signals.
_Avoid_: vision, awareness radius

**Distance-weighted detection**:
The sensing model. Agents detect others within their sensing range, but signal strength falls off with distance. No persistent chemical field in the environment — gradients are computed from the positions of nearby emitters. A computationally cheap approximation of diffusion.

### Reproduction and evolution

**Sexual reproduction**:
Two agents whose trait vectors are within the **reproductive compatibility distance** produce an offspring via budding. Both parents survive. Each parent invests energy from the reproduction fraction of mobilised energy (1 − kappa). Offspring receives the sum of both investments scaled by reproduction efficiency (remainder dissipated). Offspring traits are produced by uniform crossover (each dimension independently selected from one parent, per Gavrilets 2004) plus Gaussian mutation. Uniform crossover is deliberately chosen over arithmetic mean (Dieckmann & Doebeli 1999) because recombination works against speciation — clusters that persist despite recombination are ecologically reinforced, not just reproductively isolated. Sexual reproduction requires physical contact on the surface — both agents must be within physical interaction range as determined by their traits.
_Avoid_: mating, breeding (too specific to animal analogues)

**Mate selectivity**:
Not an explicit trait dimension. Reproductive isolation emerges from trait-space divergence evaluated against the **reproductive compatibility distance** — a world parameter, not a per-agent trait. Agents whose trait vectors are within the compatibility distance can mate; those further apart cannot. Speciation pressure comes from the physics of the compatibility threshold interacting with trait-space clustering, not from individual choosiness.
_Avoid_: choosiness, pickiness, compatibility trait

**Reproductive investment**:
The energy a parent transfers to offspring at birth, drawn from the parent's accumulated **reproductive allocation** — the earmarked share of reserve built up over many ticks from the (1 − kappa) fraction of each tick's surplus. Not an independent trait dimension — the rate at which the allocation fills is governed by kappa, and fecundity determines how that amount is divided among offspring at the reproductive event. High kappa (more to soma) fills the allocation slowly; low kappa fills it quickly. Reproduction draws from this accumulated allocation, not from unallocated reserve, so an agent that is energy-marginal for metabolism can still reproduce if its allocation is full, and an agent flush with unallocated reserve cannot reproduce until kappa has routed surplus into the allocation. A parallel transfer of **reproductive nutrient** accompanies the energy — drawn from the parent's nutrient earmark and divided among offspring the same way — so each offspring is provisioned in both currencies.
_Avoid_: brood size, litter size

**Fecundity**:
A reproduction trait controlling the number of offspring per reproductive event. The reproduction fraction of mobilised energy (1 − kappa) is the total energy budget invested per event; fecundity determines how many offspring share that budget. High fecundity produces many poorly-provisioned offspring (r-strategy). Low fecundity produces few well-provisioned offspring (K-strategy). The actual offspring count is stochastic — drawn from a Poisson distribution with mean equal to the fecundity trait. For sexual reproduction, the effective fecundity is the average of both parents'. Because fecundity is part of the trait vector, it contributes to trait-space distance and therefore to reproductive compatibility — agents with very different reproductive strategies are less likely to be within the **reproductive compatibility distance**.
_Avoid_: clutch size, litter size (too specific to animal analogues)

**Asexual reproduction**:
Reproduction without a mate, governed by an agent's **asexual propensity** trait. Not a universal fallback — an agent must have evolved sufficient asexual propensity to reproduce alone. Offspring traits are the parent's traits plus mutation — no crossover, because there is no second parent. The costs are inherent: lower offspring variation (no recombination) and a single parent's energy contribution. In dense populations where compatible mates are available, sexual reproduction is advantageous because it generates more combinatorial diversity. Whether a lineage relies primarily on sexual or asexual reproduction is emergent, driven by the evolved asexual propensity of its members.
_Avoid_: cloning (implies exact replication — mutation still applies)

**Dispersal**:
A reproduction trait controlling how far offspring land from the parent. Represents investment in dispersal structures and mechanisms — spores, seeds, fruits, explosive pods. Higher dispersal investment produces a wider dispersal kernel. Independent of fecundity (a dandelion has both high fecundity and high dispersal; a coconut has low fecundity and moderate dispersal). Independent of mobility (sessile organisms disperse via propagule structures; mobile organisms' offspring may disperse under their own locomotion — both are expressions of the same trait). The dispersal-fecundity trade-off is budgetary, enforced through superlinear maintenance costs on both dimensions, not a mechanical coupling. Dispersal does double duty: the same propagule investment also sets a low-mobility agent's mate-finding reach — see **Reproductive reach**.
_Avoid_: spore dispersal (too specific to one mechanism), range (overloaded term)

**Reproductive reach**:
The spatial distance over which two agents can find each other to mate — the spatial counterpart to the **reproductive compatibility distance** (which is the trait-space counterpart). Derived, not a trait or a world parameter: it combines an agent's mobility-derived perception with its dispersal-derived gamete broadcast, added smoothly with no floor or ceiling. Mate-finding has two physical solutions — move the organism or move its gametes — so a mobile agent reaches mates by locomotion (a wolf) and a sessile agent by broadcasting gametes (pollen, spores). A mobility-0 agent is therefore not reproductively isolated as long as it invests in dispersal. A sexual event requires both axes: partners within reproductive reach (space) *and* within the compatibility distance (trait space).
_Avoid_: mating range (overloaded), sensing range (reach is broader than perception — gamete broadcast extends it)

**Feeding reach**:
The spatial distance over which a consumer can drain a target's body (a living agent or a carcass). Derived, not a trait or a world parameter, and — like **reproductive reach** — it has two physical solutions, because feeding has two ways to close the gap to food: move the organism, or extend the body to the food. A mobile consumer reaches food by locomotion (folded into the **contact-range** term). A sessile consumer cannot travel, so it extends its **body** through the substrate instead: a decomposer's mycelium *is* its foraging organ, so growing bigger lets it touch more. Feeding reach is therefore derived from **both** the contact-range coefficient and a structure-derived body-extent term, added smoothly with no floor or ceiling, and the whole reach is modulated by **effective heterotrophy** — the foraging body is the *heterotrophic* body, so a large autotroph's bulk earns it no feeding reach while a growing mycelium's does. A mobility-0 consumer is therefore not feeding-isolated as long as it invests in heterotrophy and grows. This mirrors how structure already governs **light competition** (bigger producers shade more) — body size is spatially consequential on both the autotrophic and heterotrophic axes. The body-extent term uses the square root of structure (a disc's radius scales with the square root of its area), so reach grows sublinearly and does not run away with body size.
_Avoid_: contact radius (a uniform world value — feeding reach is a per-agent derived property), grazing range (overloaded)

**Speciation**:
The divergence of agent populations into distinct clusters in trait space that no longer interbreed due to trait distance exceeding the **reproductive compatibility distance**. Not designed — emerges from selection and reproductive dynamics.
_Avoid_: species (as a designed concept — there are no species definitions, only emergent clusters)

### World parameters

**World parameters**:
The constants that define the physics of the simulation. Searched by genesis across ensemble runs. Not visible to agents, not evolvable. Distinct from the trait vector, which evolves within a run.

**Solar flux magnitude**:
Total energy available per tick within a **light competition radius**. Divided among local producers proportional to their autotrophy and structure (body size). The sole tap — controls how much energy enters the system.

**Light competition radius**:
The radius within which producers compete for solar flux. Producers outside this radius do not affect each other's energy intake. A world parameter searched by genesis. Interacts with world extent and population density to determine how crowded the light environment is.

**Reproductive compatibility distance**:
The trait-space distance threshold within which two agents can sexually reproduce. A world parameter, not a trait — individual agents do not evolve their own selectivity. Mate selectivity is derived: it emerges from the interaction between trait-space clustering and this physics-defined threshold. If two agents' trait vectors are within the compatibility distance, they can mate; otherwise they cannot. This is the mechanism through which speciation occurs — clusters that diverge beyond the compatibility distance become reproductively isolated. Searched by genesis.
_Avoid_: mate selectivity threshold (selectivity implies a per-agent choice — this is a world constant)

**Base metabolic rate**:
Fixed energy cost per tick, independent of traits or activity. The floor of metabolic cost — trait maintenance, movement, and sensing costs are added on top.

**Trait maintenance cost**:
Energy cost per tick per unit of each specification trait, growing superlinearly with the trait's value. Coefficients (one each for autotrophy, heterotrophy, mobility) are world parameters searched by genesis. An agent with high heterotrophy pays for maintaining the machinery to consume whether or not it finds prey. The superlinear shape is the primary anti-generalist mechanism — an agent investing moderately in two traits pays more total maintenance than one investing heavily in a single trait.

**Movement cost coefficient**:
Energy cost per unit distance moved per tick. Makes mobility expensive — creates the core trade-off between sessile photosynthesis and mobile consumption.

**Sensing cost coefficient**:
Energy cost per tick for sensing, derived from mobility investment. Makes wide awareness expensive — but since sensing is subordinate to mobility, this cost is part of the mobility maintenance cost, not a separate parameter.

**Body reach coefficient**:
The weight on the structure-derived body-extent term of **feeding reach**: `consumption_reach = effective_heterotrophy × (contact_range_coefficient + body_reach_coefficient × √structure)`. It sets how strongly a sessile consumer's growth extends its reach to food — the body-as-foraging-organ channel (mycelium through substrate). The square-root form keeps the contribution sublinear; modulation by effective heterotrophy keeps it a *heterotrophic* channel (autotrophic bulk earns no feeding reach). A world parameter searched by genesis, mirroring the **dispersal reach coefficient** (the sessile solution to mate-finding). Default 0.0 disables it — existing recipes keep the pure contact-range feeding reach.
_Avoid_: mycelium coefficient (too literal — the mechanism is general to all consumers, not decomposer-only)

**Growth retention multiplier**:
The size of the retention buffer in the grow phase, expressed as a multiple of an agent's per-tick metabolic cost. Surplus eligible for kappa-allocation is `reserve − (metabolic_cost × multiplier)`. A higher multiplier makes agents hoard more reserve against future metabolism before committing any to soma or reproductive allocation, slowing both growth and reproductive accumulation. A world parameter searched by genesis. Default 2.0.
_Avoid_: retention factor (too generic), metabolic safety margin (implies a hard floor — this is a smooth flow-allocation buffer, not a starvation guard)

**Reproduction efficiency**:
Fraction of energy invested by the parent that the offspring actually receives. The remainder is dissipated. Reproduction is lossy like all energy transfers.

**Offspring structure fraction**:
Fraction of each offspring's per-offspring energy share (after reproduction-efficiency loss) that is committed to **structure** at birth rather than to **reserve**. The structure commitment goes through the same lossy reserve-to-structure conversion as in-life growth (`growth_efficiency`); the unconverted remainder is dissipated to heat. The remaining `(1 − offspring_structure_fraction)` becomes the newborn's starting reserve. The same provisioning is applied to seeded agents at world creation, so tick-0 agents are not degenerately disadvantaged against the structural death threshold. Conservation across reproduction: parents' committed investment = sum of offspring reserve + sum of offspring structure + heat. A world parameter searched by genesis. Default 0.2 — most of the per-offspring share is reserve (metabolic fuel for the first ticks of life), with enough committed to structure that newborns are not born structure-zero.
_Avoid_: birth weight (too phenotypic), structure share (ambiguous between birth-time and growth-time allocation)

**Reproduction energy threshold**:
Minimum **reproductive allocation** an agent must have accumulated to attempt reproduction. Below this, agents prioritise survival.

**Reproduction nutrient threshold**:
Minimum **reproductive nutrient** an agent must have accumulated to attempt reproduction — the nutrient-axis mirror of the **reproduction energy threshold**. A reproductive event requires both thresholds to be met. Replaces the former body-support nutrient gate (`nutrient ≥ structure × demand`), which embodiment makes redundant: the body's nutrient is bound by construction, so the only nutrient question left at reproduction is whether enough has been earmarked to provision offspring.

**Mutation rate**:
Probability of each trait dimension mutating per reproduction event.

**Mutation magnitude**:
Standard deviation of the Gaussian perturbation applied to a mutated trait dimension.

**World extent**:
Spatial dimensions of the world. Interacts with population size to determine density. Toroidal topology during genesis (no edges, no boundary effects). Play-time topology — where the player can move beyond the genesis world — is a separate design problem, not yet addressed in the system design.

**Initial population size**:
Number of agents at tick zero.

### Initial trait distribution

**Initial trait distribution**:
The starting configuration of agents in trait space. Searched by genesis alongside world parameters, but secondary — if world parameters are correct, many initial distributions should converge to sensible ecologies (the ensemble tests this).

**Mean trait vector**:
Centre of the initial population in trait space.

**Trait covariance**:
Whether initial trait dimensions are independent or correlated (e.g., high mobility correlated with high heterotrophy). Controls whether the founding population starts as a single cloud or an elongated structure in trait space.

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

### Simulation

**Intent**:
A thin declaration emitted by an agent each tick, naming what it wants to do without specifying magnitude or outcome. The interaction coordinator receives intents and computes what actually happens by applying world physics. An intent carries the verb (consume, decompose, reproduce, redistribute), the source agent, a target (for directed verbs), the agent's position, and its traits. Agents self-filter — they only emit intents they believe they can afford. Multiple intents per tick are permitted; the energy budget is the only constraint on how many actions fire.
_Avoid_: action, command, request (intent emphasises that the outcome is not guaranteed — the coordinator arbitrates)

**World genesis**:
The process of generating a playable world. A parameterisation is evaluated as an ensemble of replicate runs (same parameters, different random seeds). Each run simulates a random initial population forward (off-screen). Degenerate runs are detected and terminated early. A parameterisation is accepted only when most runs in the ensemble produce sensible worlds. The player drops into a world with history.

**Degenerate configuration**:
A simulation outcome that fails to produce a functioning ecology. Six canonical failure modes: extinction (all agents die), monoculture (trait space collapses to a single cluster), energy death (free energy trends irreversibly toward zero), population explosion (unbounded growth), frozen dynamics (no turnover despite agents surviving), generalist dominance (one or more clusters with high values across multiple specification traits outcompete specialists — indicates superlinear maintenance costs are too weak to enforce trade-offs).
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

> **Dev:** An agent with high autotrophy and low mobility — is that a plant?
>
> **Domain:** It's a producer — its specification traits are concentrated in autotrophy, leaving little investment in heterotrophy or mobility. We derive that label from its traits, not assign it. Because it invests in autotrophy, it extracts nutrients from the substrate — the same sessile infrastructure serves both light capture and nutrient extraction. Its low mobility means it pays no movement overhead, which is why sessile autotrophy dominates: sunlight is ambient, so mobility adds cost for zero additional energy income. If it also has moderate heterotrophy, it's a producer that supplements with consumption — but superlinear maintenance costs mean that spreading investment across both autotrophy and heterotrophy is progressively more expensive than specialising.
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
> **Domain:** They don't — not by design. Agents reproduce sexually when their trait-space distance is within the reproductive compatibility distance — a world parameter, not a per-agent trait. Over time, clusters form in trait space that stop interbreeding because their trait distance exceeds that threshold. We call that speciation, but there's no species registry. It's emergent.

## Design decisions

### The world is a complex adaptive system

The world is an agent-based model in the tradition of computational ecology (Grimm & Railsback 2005, *Individual-based Modeling and Ecology*; DeAngelis & Grimm 2014). All entities are heterogeneous, adaptive agents interacting locally in 2D continuous space. Population dynamics emerge from individual agent behaviour — there is no top-down control of species populations or ecosystem balance.

### Earth-analogous ecology

The simulation follows real ecological principles: energy conservation, trophic levels (Lindeman 1942), nutrient cycling, carrying capacity. The forms may be alien but the dynamics are grounded. This allows us to draw on established ABM literature and ecological models (the ODD protocol, Grimm et al. 2006, 2010, provides the standard description format; pattern-oriented modelling, Grimm et al. 2005, provides the validation approach). Alien mechanics can be layered on once the base ecology produces coherent dynamics.

### No environmental cycles (v1)

No imposed day/night or seasonal cycles initially. All temporal patterns emerge from agent interactions. Environmental oscillators can be added later if the simulation lacks rhythm.

### Visual behaviour language

Agent traits map to visual properties. Interactions produce visible effects. The player learns to read the ecology by watching how things look and behave. No text, no numbers, no labels.
