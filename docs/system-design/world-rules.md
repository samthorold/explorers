# World Rules

The immutable physics of the simulation. These define what the world is made of and how it behaves before any agent strategy, population dynamic, or emergent pattern is considered. An agent dropped into this world faces these constraints unconditionally.

This document is built on the ecological ground truths in [docs/ecology/](../ecology/) — observable properties of real ecosystems that we take as given. It informs but does not prescribe the architectural decisions in [docs/adr/](../adr/).

## Stocks

The world has two currencies. Energy is the primary currency — it powers all processes and is the currency of metabolism. In addition, the world has a single nutrient that agents require alongside energy. An agent's nutrient demand — how much nutrient it needs per unit energy — is derived from its trait vector: more capable agents need more nutrient. Together, energy and nutrient are the building blocks of every agent.

### Energy stocks

Energy flows through the system in one direction: from source to sink. It does not cycle.

| Stock | Type | Description |
|---|---|---|
| **Solar flux** | Source (inexhaustible) | Energy enters the system here. The flux is constant — the same amount of energy is available every tick. All temporal variation in the system is endogenous. |
| **Living agents** | Internal | Energy held by living organisms. Every living agent carries an energy balance that increases through acquisition and decreases through costs. Living agents also accumulate somatic wear — see below. |
| **Carcasses** | Internal | Energy held by dead organisms. When an agent dies, its remaining energy becomes a carcass at the same location on the surface. Carcasses hold energy indefinitely — there is no passive decay. |
| **Heat** | Sink (inexhaustible) | Energy leaves the system here, permanently. Every lossy process — metabolism, trophic transfer, reproduction inefficiency — sends energy to this sink. It does not return. |

### Nutrient

Unlike energy, nutrient cycles. It is not consumed and lost — it passes through agents and returns to the environment when agents die and are decomposed. The total amount of nutrient in the system is conserved across all pools (available, living agents, carcasses, unavailable).

The system tracks a single nutrient. This nutrient enters the living system only through uptake from the available pool — a process that requires sustained contact with the substrate (see flow 2). Nutrient leaves living agents only through death.

**Stoichiometric demand.** Different agents need nutrient in different amounts relative to energy, depending on their traits. An agent's nutrient demand is derived from its trait vector — more capable agents (higher trait values) require more nutrient per unit energy. When the nutrient-to-energy ratio in food does not match the consumer's demand, the consumer retains only what it needs and excretes the excess nutrient immediately to the available pool. The limiting currency (energy or nutrient) constrains how much of the consumed material the consumer can actually use. Nutrient limitation blocks reproduction but does not impair other agent functions.

**Differential limitation.** Even with a single nutrient, the system creates differential limitation across the surface. The available pool varies spatially — some locations are nutrient-rich, others nutrient-poor. Agents at nutrient-poor locations may be nutrient-limited (can acquire energy but not enough nutrient to reproduce), while agents at nutrient-rich locations may be energy-limited (have plenty of nutrient but insufficient energy). This spatial variation in which currency is limiting creates context-dependent fitness — different strategies are favoured at different locations.

### Nutrient pools

Nutrients exist in four pools, analogous to the energy stocks:

| Pool | Description |
|---|---|
| **Available** | Nutrient in the environment that is biologically accessible — extractable by agents through nutrient uptake. Distributed spatially across the substrate. Co-located agents share the available pool proportionally (like light competition). |
| **Living agents** | Nutrient incorporated into living agent biomass. Each agent holds nutrient in a ratio determined by its traits. Nutrient leaves living agents only through death — metabolism does not release nutrient. |
| **Carcasses** | Nutrient locked in dead matter. Released back to the available pool through decomposition. The nutrient content of a carcass reflects the content of the agent that died. |
| **Unavailable** | Nutrient locked in forms that are not biologically accessible — bound in rock, occluded in substrate chemistry, or in chemical states that no agent can process. These pools change only through geological-timescale processes. |

### Conservation laws

**Energy conservation.** At any point in time:

> Total system energy = Σ(living agent energy) + Σ(carcass energy)

The change in total system energy per tick equals:

> ΔE = photosynthetic input − total dissipation to heat

Energy is neither created nor destroyed within the system. Every flow is accounted for.

**Nutrient conservation.** For the nutrient:

> Total nutrient = available pool + Σ(living agent nutrient) + Σ(carcass nutrient) + unavailable pool

Nutrient is neither created nor destroyed. It cycles between pools. The unavailable pool changes only through geological-timescale processes (weathering, deposition), not through biological activity.

If either conservation law fails, the world is broken.

### No passive decay

Carcasses do not lose energy or nutrient on their own. Decomposition is always an agentic process — a living agent must actively consume a carcass to return its energy and nutrient to the living system and available pool. This makes decomposer strategies structurally necessary for the world's nutrient cycle. A world without decomposers accumulates resources in the dead pool until the living system starves.

This is not a fragility to be patched with a safety valve. In real ecosystems, what appears to be passive decay is always decomposition by organisms at a finer resolution — bacteria, fungi, invertebrates. The principle is: **all resource transformation requires an agent**.

### Somatic wear

Living agents degrade over time. Every functional capability — photosynthetic apparatus, locomotion machinery, sensory organs, digestive systems — accumulates wear through use and through the baseline cost of maintaining complex machinery. This is the disposable soma principle (Kirkwood 1977): an organism could theoretically maintain its body indefinitely, but the energy required for perfect repair is energy unavailable for reproduction. The body is disposable because investing in immortality is a losing strategy when extrinsic mortality exists.

Wear accumulates per functional trait, not as a single aggregate. An agent that photosynthesizes heavily wears out its photosynthetic apparatus faster than an agent in shade. An agent that sprints wears out its locomotion faster than a sessile one. Each trait wears at a rate determined by two components: a baseline proportional to the trait's magnitude (complex machinery degrades even when idle) and an additional component proportional to the trait's actual metabolic throughput this tick (active use produces damaging byproducts — the oxidative damage model).

Wear reduces trait effectiveness. The mapping is exponential: a small amount of wear barely affects output (biological systems have redundancy and over-provisioning), but wear compounds — later increments are increasingly catastrophic. An aging producer captures less light. An aging consumer catches prey less efficiently. An aging agent senses less, moves slower, processes food less effectively.

Agents invest in somatic maintenance to counteract wear. The level of investment is a heritable trait — some lineages evolve high repair investment (long-lived, slow-reproducing), others evolve low repair (short-lived, fast-reproducing). Somatic maintenance is a whole-organism investment: an agent does not selectively repair one organ while neglecting another. The energy cost of maintenance competes directly with the energy cost of reproduction — this is the core trade-off that Kirkwood identified.

Behavioural traits — mate selectivity, reproductive investment, fecundity — do not wear. They are allocation parameters, not physical machinery. An old organism is less capable, not less decisive.

Identity-determining thresholds use an agent's nominal (unworn) traits. An agent born mobile is ecologically mobile for its entire life, even as its effective mobility degrades with age. Wear degrades performance, not identity. An aging wolf does not become a plant.

**Offspring are born with zero wear.** This is the evolutionary rationale for reproduction: it resets the soma. The germline is maintained at higher fidelity than the soma. A parent's worn-out body produces a brand-new body. The information (heritable traits) persists in pristine hardware (the offspring). This asymmetry between germline and soma is what makes the disposable soma trade-off coherent — repair your body, or build a new one.

## Flows

Eight flows move energy and nutrient between stocks. Each flow is a rate — resources per tick — and each is subject to constraints described below. Energy and nutrient travel together through most flows (an agent that eats another agent acquires both energy and nutrient), but they have different fates: energy is progressively dissipated to heat, while nutrient cycles back through decomposition.

### Input flows

**1. Photosynthesis.** Solar → Living agent. The only way energy enters the living system. Producers absorb energy from the constant solar flux. This flow is attenuated by local competition (producers near other producers share the available flux) and by the fundamental trade-off between sessile and mobile strategies (the ecology establishes that photosynthesis requires being stationary). Photosynthesis moves energy only — nutrient must be acquired separately through nutrient uptake (flow 2).

**2. Nutrient uptake.** Available pool → Living agent. Agents extract nutrient from the available pool at their location. The uptake rate depends on two factors: the agent's nutrient absorption trait (a heritable, maintenance-costing capability) and the agent's contact time at its current location. An agent that has remained stationary for many ticks extracts nutrient more effectively than one that just arrived — sustained contact with the substrate is required to establish the interface structures (analogous to roots) through which nutrient is extracted. Moving resets contact time. Co-located agents share the available pool proportionally, weighted by their effective uptake rate. Nutrient uptake is the only way nutrient enters the living system.

### Flows between living agents

**3. Consumption.** Living agent → Living agent. A consumer drains energy and nutrient from a living target through sustained physical contact. The transfer is lossy — only a fraction of the drained energy reaches the consumer; the remainder dissipates to heat (flow 8). Nutrient transfers alongside energy, but the consumer retains only what it needs according to its stoichiometric demand — excess nutrient is excreted immediately to the available pool at the consumption site, never incorporated. The limiting currency (energy or nutrient) constrains how much of the consumed material the consumer can actually use. Consumption is non-lethal by default; the target survives unless its energy reaches zero.

**4. Reproduction.** Living agent → Living agent. Parents invest energy and nutrient to create offspring. Reproduction requires sufficient nutrient — nutrient limitation blocks reproduction even when energy is abundant. The energy transfer is lossy — a fraction dissipates to heat (flow 8). Both parents survive. Reproduction has two modes, variable fecundity, and spatial dispersal of offspring.

**Sexual reproduction** is the primary mode. Two compatible agents within reproductive range each invest energy. Offspring traits are derived from both parents through crossover (each trait dimension drawn randomly from one parent) with heritable mutation. Compatibility is determined by trait-space distance — agents with similar traits are more likely to mate. This makes reproductive isolation an emergent property of trait divergence.

**Asexual reproduction** is a universal fallback. When an agent has sufficient energy to reproduce but no compatible mate is available, it can reproduce alone. Offspring traits are the parent's traits plus mutation — no crossover, because there is no second parent. The costs of asexual reproduction are inherent: lower offspring variation (no recombination) and a single parent's energy contribution (less total investment per offspring). These costs create selection pressure: in dense populations where mates are available, sexual reproduction is advantageous because it generates more combinatorial diversity. In sparse populations or for isolated colonizers, asexual reproduction is the only option. Whether a lineage relies primarily on sexual or asexual reproduction is not prescribed — it is an emergent outcome of population density, mate availability, and the fitness value of variation in a given environment.

**Fecundity** — the number of offspring per reproductive event — is a heritable trait. A fixed total energy budget is invested per event; fecundity determines how many offspring share that budget. High fecundity produces many poorly-provisioned offspring (r-strategy). Low fecundity produces few well-provisioned offspring (K-strategy). The actual offspring count for a given event is stochastic — drawn from a Poisson distribution with mean equal to the fecundity trait. This stochasticity means reproductive failure (zero offspring despite energy investment) is possible. The energy cost of reproduction is committed before the outcome is determined — failed reproduction is costly, as it is in nature.

For sexual reproduction, the effective fecundity is the average of the two parents' fecundity traits. Because fecundity is part of the trait vector, it contributes to trait-space distance and therefore to mate compatibility — agents with very different reproductive strategies are less likely to mate. This creates reproductive isolation along the r/K axis without any special mechanism.

**Offspring dispersal** follows a Gaussian kernel centred on the parent's position. Most offspring land nearby; a tail of long-distance dispersers enables colonisation of distant habitat. The dispersal radius is scaled by the parent's reproductive range — wide for sessile agents (spore/seed dispersal) and narrow for mobile agents (who can disperse under their own locomotion after birth). Each offspring in a clutch is placed independently, so siblings from the same event scatter across the dispersal range.

**5. Network redistribution.** Living agent ↔ Living agent. Energy and nutrient move between living agents through network connections (see Topologies below). This flow is cooperative, not adversarial — it is distinct from consumption. It is bidirectional: resources can flow in either direction through a connection, governed by the states of the connected agents. This is the mechanism by which mutualistic relationships become possible. Like all transfers, the energy component is lossy (flow 8).

### Flows from living to dead

**6. Death.** Living agent → Carcass. When a living agent's energy reaches zero, it becomes a carcass. All remaining energy and nutrient transfer to the carcass at the agent's location on the surface. The nutrient content of the carcass reflects the content of the agent that died.

Death has two proximate causes. **Extrinsic mortality** — predation drains energy below zero, or starvation when metabolic costs exceed energy income. **Intrinsic mortality** — somatic wear degrades functional traits until the agent can no longer acquire enough energy to cover its metabolic costs. In practice these compound: an aging agent with degraded photosynthetic capacity that was previously viable becomes energy-negative when a competitor shades it, or an aging consumer whose effective consumption rate has declined can no longer catch sufficient prey. Somatic wear makes death inevitable on a long enough timeline — even in the absence of predation or competition, an agent that invests less in somatic maintenance than the rate of wear accumulation will eventually degrade to the point of energy bankruptcy.

### Flows from dead to living

**7. Decomposition.** Carcass → Living agent + Available pool. A decomposer drains energy and nutrient from a carcass through sustained physical contact. The energy transfer is lossy — only a fraction reaches the decomposer; the remainder dissipates to heat (flow 8). Nutrient that the decomposer cannot use (due to stoichiometric mismatch) is excreted immediately to the available pool, closing the nutrient cycle. The carcass is depleted as resources are extracted.

### Dissipation

**8. Trophic transfer loss.** Accompanies flows 3, 4, 5, and 7. At every energy transfer between agents, a fraction is lost to heat. This is not a separate event — it is the inefficiency inherent in every transfer. It is what makes trophic pyramids inevitable: each level of transfer dissipates energy, so less is available at each successive level. Nutrient is not lost to heat — it is either incorporated into the receiving agent or returned to the available pool.

**9. Metabolism.** Living agent → Heat. Every living agent pays a continuous energy cost simply to exist. This cost has three components:
- A base rate — the minimum cost of being alive, independent of traits or activity.
- Trait-dependent costs — each capability (photosynthesis, consumption, decomposition, nutrient absorption) costs energy to maintain whether or not it is currently in use. Sensing and movement also cost energy. These costs are the mechanism behind the specialist-generalist trade-off: an agent investing in multiple capabilities pays overhead for all of them.
- Somatic maintenance — the energy cost of repairing accumulated wear. Higher investment in somatic maintenance slows aging but competes directly with the energy available for reproduction and growth. This is the mechanism behind the reproduce-vs-survive trade-off.

Metabolism dissipates energy to heat. It does not release nutrient — nutrient leaves living agents only through death (flow 6). This makes decomposition structurally necessary for nutrient cycling.

### Flow summary

Energy flows one way: source → living system → heat. Nutrient cycles: available pool → living agents → carcasses → (via decomposition) → available pool. Nutrient leaves living agents only through death — metabolism releases energy to heat but not nutrient.

```
Solar ──photosynthesis──▶ Living Agents ──death──▶ Carcasses
  (source)                 ▲  │  ▲  ↕                │
                           │  │  │  │                 │
              nutrient     │  │  │  network           │
              uptake       │  │  │  redistribution    │
                ▲          │  │  │                    │
                │   decomposition consumption    metabolism
          Available pool   │  │  reproduction         │
                ▲          │  ▼                       │
                │       Carcasses              Heat (sink)
                │                          ◀── trophic loss
                └──── nutrient release
                      (from decomposition,
                       stoichiometric excretion)
```

## Topologies

The world has two topologies. Different flows operate on different topologies.

### Physical surface

A two-dimensional continuous surface. Movement happens here. Every agent and every carcass has a position on this surface.

The surface is the topology for all spatially-local interactions. Agents can only physically contact other agents that are nearby on the surface. Photosynthesis, consumption, decomposition, death, and reproduction all require surface proximity.

### Emergent network

A graph that agents build through their behaviour. Nodes are agents; edges are connections that agents create and maintain. The network is not constrained by surface distance — two agents far apart on the surface can be adjacent on the network if they have built a connection between them.

Network-building is a general capability: any agent can invest in creating connections. The cost structure makes it only worthwhile for certain trait configurations. In real ecosystems, this role is filled by organisms with specific morphology — fungal hyphae, root systems — but in a universal-agent system, which agents build network infrastructure is emergent, not prescribed.

The network enables flows and perception that bypass surface locality. Its topology, density, and reach are all emergent properties.

## Channels

Each topology carries two kinds of channel, distinguished by whether they can carry energy.

### Perception channels (information only)

Perception channels carry information about the state of other agents and carcasses. They do not move energy. Looking at food does not feed you; detecting a chemical signal does not transfer resources. Perception enables decision-making — it tells an agent what is nearby and in what direction — but it has no direct effect on stocks.

Both topologies carry perception channels:
- **Surface perception** — detecting agents and carcasses within a range on the surface (vision, chemotaxis, vibration).
- **Network perception** — detecting signals propagated through network connections (chemical alerts, resource-state information).

### Physical channels (information + energy)

Physical channels can carry energy between agents. They are the pathways through which flows 2, 3, 4, and 6 operate.

Both topologies carry physical channels:
- **Surface contact** — direct physical proximity on the surface. Required for consumption, decomposition, and reproduction.
- **Network transfer** — resource movement through network connections. Required for network redistribution (flow 4). This is what makes the network more than a signaling system — it is infrastructure that moves energy.

### The perception-physical distinction as a world rule

This distinction is a law of physics, not a strategy choice. No amount of trait investment can make a perception channel carry energy. An agent cannot feed through vision or acquire resources through chemotaxis alone. Energy transfer always requires a physical channel — either surface contact or a network connection.

## Cost structure (trade-offs)

The cost structure is what prevents any agent from being good at everything. It is not a rule imposed on agents — it is an emergent consequence of the fact that capabilities cost energy to maintain and that nutrient requirements constrain what agents can efficiently process. Eight fundamental trade-offs arise from this cost structure:

**1. Acquire vs. maintain.** Every energy-acquisition capability costs energy to maintain whether or not it is currently in use. More capability means more overhead.

**2. Sessile vs. mobile.** Sessile and mobile strategies face fundamentally different energy budgets and interaction constraints. The ecology establishes this as a universal property: photosynthesis requires being stationary; consumption requires being mobile (or at least requires prey to come to you). This is the most fundamental differentiation in the system.

**3. Reproduce vs. survive.** Energy invested in offspring is energy not available for self-maintenance. Energy invested in somatic maintenance (slowing wear) is energy not available for reproduction. This is the disposable soma trade-off: build a new body or repair the current one. Both achieve the same goal — keeping a lineage alive — through different strategies. High somatic maintenance investment produces long-lived agents that reproduce infrequently. Low investment produces short-lived agents that reproduce early and often. The optimal position on this continuum depends on the extrinsic mortality rate: when predation is high, investing in longevity is wasteful because the agent is likely to be eaten before aging matters.

**4. Few quality vs. many fragile offspring.** A fixed reproductive energy budget can produce few well-provisioned offspring (K-strategy) or many poorly-provisioned offspring (r-strategy). Fecundity determines how many offspring share the budget. High-fecundity offspring start with less energy and are closer to metabolic death — they must establish an energy income quickly or die. Low-fecundity offspring start with more energy and can weather adverse conditions. Neither strategy dominates — their relative success depends on environmental context. In stable, competitive environments, few well-provisioned offspring outcompete many fragile ones. In disturbed or empty environments, many fragile offspring colonise faster.

**5. Specialist vs. generalist.** Investing in one acquisition strategy costs less overhead than investing in multiple. Specialists pay less and outcompete generalists within their niche; generalists pay more overhead but can exploit multiple niches. The cost structure penalises breadth.

**6. Sense vs. save.** Wider sensing range enables better decisions but costs energy. Agents must balance the value of information against its metabolic price.

**7. Stoichiometric constraint.** Agents need nutrient in amounts determined by their traits — more capable agents demand more nutrient per unit energy. Food sources have varying nutrient-to-energy ratios. An agent consuming nutrient-poor food must eat more to satisfy its nutrient demand, wasting excess energy as heat. An agent consuming nutrient-rich food retains what it needs and excretes the excess nutrient to the available pool. The limiting currency (energy or nutrient) constrains how much of the consumed material the consumer can actually use.

**8. Sexual vs. asexual reproduction.** Sexual reproduction produces more diverse offspring (crossover recombines two parents' traits) but requires a compatible mate — a cost in mate-finding, energy, and the constraint that both parents must be present. Asexual reproduction requires no mate but produces less diverse offspring (parent traits plus mutation only). In dense populations with coevolutionary pressure (predator-prey arms races, competition), variation is valuable and sexual reproduction is favoured. In sparse populations or stable environments, the mate-finding cost outweighs the variation benefit and asexual reproduction is favoured. Which strategy a lineage relies on is emergent, not prescribed.

These trade-offs are the differentiation engine. They do not prescribe what roles emerge — they create the selection pressure that makes role differentiation advantageous.
