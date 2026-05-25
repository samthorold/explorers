# Dynamic Stability

The simulation must produce ecologies that persist without collapsing — not through static equilibrium but through continuous fluctuation. Populations rise and fall, clusters form and dissolve, energy flows shift — yet the system as a whole endures.

This document is our opinion about what dynamic properties the simulation must exhibit and which ecological mechanisms produce them. It is built on the ground truths in [docs/ecology/](../ecology/) — observable properties of real ecosystems that we take as given. It informs but does not prescribe the architectural decisions in [docs/adr/](../adr/).

CONTEXT.md calls the goal a "sensible world" and explicitly avoids the word "stable." That's deliberate. The system we're designing is dissipative — it requires constant energy input (solar flux) to maintain structure, and it maintains that structure through ongoing activity, not by reaching rest. The analogy is a whirlpool, not a rock.

## The stability problem

A complex adaptive system with reproducing, evolving agents can fail in six ways (CONTEXT.md's degenerate configurations):

1. **Extinction** — all agents die. The system has no persistence.
2. **Monoculture** — trait space collapses. The system has persistence but no diversity.
3. **Energy death** — free energy trends to zero. The system's currency drains away.
4. **Population explosion** — unbounded growth. No force limits the system.
5. **Frozen dynamics** — agents survive but nothing happens. Persistence without process.
6. **Generalist dominance** — agents that do everything outcompete specialists. Diversity collapses from above.

Each failure mode is the absence of a specific feedback mechanism. The design problem is ensuring that all six feedbacks are present and that none overwhelms the others.

## Energy as the stability substrate

The simulation has a single currency (energy) and a single source (solar flux). This is the foundational design choice — everything else follows from it.

### Why single-currency

Real ecosystems run on multiple currencies (carbon, nitrogen, phosphorus) whose interactions produce much of their complexity. The [nutrient cycling](../ecology/nutrient-cycling.md) reference describes how these currencies interact — differential limitation, stoichiometric mismatch, nutrient-specific feedback loops, and alternative stable states driven by nutrient ratios. We deliberately simplify to one currency for two reasons:

1. **Tractability.** Genesis must search a parameter space. Every currency adds dimensions to that space and interaction terms between currencies. A single currency lets us validate the core stability mechanisms before layering complexity.
2. **Sufficiency.** The primary stability mechanisms — trophic efficiency loss, metabolic cost, carrying capacity — operate through energy alone. A single energy currency correctly produces trophic pyramid structure, consumer-resource oscillations, density-dependent regulation, and the structural necessity of decomposers. These are the dynamics we need for the basic persistence problem.

### What we lose

The [nutrient cycling](../ecology/nutrient-cycling.md) reference documents a gradient from single-currency to multi-currency dynamics. Our choice of energy-only sits at the simplest end. We lose:

- **Differential limitation** — producers nitrogen-limited while decomposers are carbon-limited, creating cross-trophic competition. With one currency, there's only one limiting factor.
- **Stoichiometric mismatch** — consumer-resource elemental mismatches driving consumption rates and waste composition (Sterner & Elser 2002). With one currency, food is food.
- **Litter quality effects** — decomposition rate depending on who produced the dead matter. With one currency, all carcasses decompose at the same rate given the same decomposer.
- **Nutrient-ratio regime shifts** — transitions between nitrogen-limited and phosphorus-limited states that maintain diversity through temporal heterogeneity.

The trade-off is real: single-currency systems lack the stoichiometric constraints that prevent certain failure modes in nature. Generalist dominance, in particular, may be harder to suppress without the cost structures that multiple currencies impose. If genesis struggles to suppress generalists, a second currency (analogous to nitrogen limitation) is the first place to look. The ecology docs describe the full gradient — energy only, energy + one nutrient, full C/N/P, C/N/P + water — and most individual-based ecology models use at least two currencies for this reason.

### The open thermodynamic system

Energy enters through solar flux, flows through agents via consumption and reproduction, and exits through metabolic dissipation and trophic transfer loss. The system is open — it cannot reach thermodynamic equilibrium while flux continues. This openness is what makes dynamic stability possible: the system is perpetually driven, and its structure is maintained by the flow, not despite it.

The critical design parameters are the tap and the drains:

- **Tap**: solar flux magnitude × number of producers × their photosynthetic absorption (attenuated by the mobility sigmoid and local competition).
- **Drains**: base metabolic rate + trait maintenance costs + movement costs + sensing costs + trophic transfer loss (consumption and reproduction efficiency < 1.0).

If the tap exceeds total drains, energy accumulates and populations grow. If drains exceed the tap, energy depletes and populations decline. Dynamic stability requires these to oscillate around a balance point — not to sit at one.

## Feedback mechanisms

The simulation's stability depends on the interplay of negative feedbacks (which resist change and push toward balance) and positive feedbacks (which amplify change and drive dynamics). A system with only negative feedback is frozen. A system with only positive feedback explodes. Dynamic stability emerges when both are present and neither dominates globally.

### Negative feedbacks (stabilising)

**Carrying capacity through light competition.** Producers share solar flux within a light competition radius. As producer density increases locally, per-agent energy intake drops. This is density-dependent negative feedback — the classic logistic growth mechanism. It prevents producer populations from growing without bound and creates spatial selection pressure (dispersed producers outperform clustered ones).

**Trophic efficiency loss.** Every energy transfer — consumption, decomposition, reproduction — is lossy. Consumers capture only a fraction of their prey's energy. This is Lindeman's (1942) ten-percent law operationalised: each trophic level can support roughly an order of magnitude less biomass than the one below it. The trophic pyramid emerges from this loss, not from any imposed population cap. It is self-enforcing — more consumers mean less energy per consumer, which means fewer consumers survive to reproduce.

**Metabolic cost of traits.** Every energy-acquisition trait (photosynthetic absorption, consumption rate, scavenging rate) costs energy to maintain whether or not it's used. This is the key anti-generalist mechanism. An agent with high values across all three traits pays triple maintenance but can only exploit one niche at a time in any given spatial context. Specialists pay less overhead and outcompete generalists in their niche. This feedback converts the theoretical advantage of generalism into a practical disadvantage.

**Movement and sensing costs.** Mobility and wide sensing range cost energy per tick. These costs enforce the fundamental trade-off between sessile photosynthesis (cheap to maintain, spatially constrained) and mobile consumption (expensive to maintain, spatially flexible). Without these costs, all agents would evolve maximum mobility and maximum sensing, collapsing the producer-consumer distinction.

### Positive feedbacks (destabilising, diversity-generating)

**Predator-prey oscillations.** More prey → more energy for consumers → more consumers reproduce → consumer population grows → prey declines → less energy for consumers → consumer population crashes → prey recovers. This is the Lotka-Volterra cycle. It is inherently unstable in isolation (the oscillations can grow without bound in simple models) but is damped by the negative feedbacks above. The simulation needs these oscillations to exist (they're a genesis fitness criterion) but not to diverge.

**Reproductive amplification.** Successful agents (those with enough energy) reproduce, creating more agents with similar traits. If a trait configuration is locally advantageous, it amplifies. This is natural selection operating as a positive feedback on trait-space density. It drives cluster formation (speciation) but can also drive monoculture if one configuration dominates everywhere.

**Energy lock-up in carcasses.** When agents die, their energy is locked in carcasses until decomposed. If decomposers decline, energy accumulates in the dead pool, reducing the energy available to the living system, which causes more deaths, which creates more carcasses. This is a positive feedback toward energy death — it only reverses if decomposer populations recover. The simulation deliberately has no passive decay, making decomposers load-bearing for the energy cycle.

### The feedback balance

The negative feedbacks set limits. The positive feedbacks generate dynamics within those limits. Dynamic stability is the regime where:

- Populations oscillate but don't diverge (negative feedbacks damp the Lotka-Volterra cycle).
- Trait clusters form and persist but don't freeze (reproductive amplification drives speciation; metabolic costs prevent any one cluster from dominating all niches).
- Energy circulates rather than accumulating or depleting (the tap-drain balance oscillates around zero net flow, with carcass lock-up as a buffer that decomposers continuously process).

The genesis fitness function (ADR-0003) directly encodes this regime: oscillation strength, clustering strength, coexistence duration, demographic turnover, and trophic balance are all measurements of the feedback balance being in the right regime.

## Life history strategies as emergent allocation

The [life history theory](../ecology/life-history-theory.md) reference describes how organisms allocate limited energy among competing demands — growth, maintenance, reproduction, dispersal, defense. In our simulation, these allocation trade-offs map onto the trait vector:

- **Reproductive investment** maps directly onto the r/K continuum. High-investment agents produce fewer, better-provisioned offspring with higher initial survival. Low-investment agents produce many fragile offspring. Neither strategy dominates — their relative success depends on environmental context (predation pressure, resource density, local disturbance).
- **Mate selectivity** controls speciation pressure. High selectivity reduces mating frequency but narrows the genetic distance between parents, producing more similar offspring. This is the mechanism by which trait-space clusters become reproductively isolated.
- **Mutation rate and magnitude** function as bet-hedging parameters. High mutation variance is diversified bet-hedging — the lineage spreads offspring across phenotype space, ensuring some suit whatever conditions arise. Low mutation variance is conservative bet-hedging — offspring cluster near proven parental phenotypes. In a stable world, low variance wins. In a fluctuating world, high variance persists through environmental shifts.

These are not prescribed strategies — they emerge from the interaction between the energy budget, the trait vector, and population-level competition. The ecology tells us this is sufficient: explicit energy budgets with competing demands and heritable allocation parameters produce life history evolution without pre-programming.

## Trade-off pressure as the differentiation engine

The simulation's most important system design property is that no agent can be good at everything. This isn't a rule — it's an emergent consequence of the cost structure:

1. **The mobility sigmoid** gates photosynthetic gain. High-mobility agents get negligible photosynthesis. This creates the producer-consumer divide — the most fundamental differentiation in the system.
2. **Trait maintenance costs** penalise breadth. Carrying high values in multiple acquisition traits costs energy whether or not those traits are exercised. Specialists pay less overhead than generalists.
3. **Movement and sensing costs** make mobility expensive. Sessile agents save energy; mobile agents spend it. The savings fund photosynthetic investment; the spending funds prey-seeking capability.
4. **Reproduction efficiency loss** means offspring are always energetically cheaper than their parents' investment. K-strategists (high reproductive investment) produce fewer, fitter offspring. r-strategists (low investment) produce many fragile ones. Neither strategy dominates — their relative success depends on environmental context (predation pressure, resource density, disturbance).

These trade-offs are the mechanism by which the simulation avoids both monoculture and generalist dominance. They don't prescribe what roles emerge — they create the selection pressure that makes role differentiation advantageous.

### What happens when trade-offs are too weak

If trait maintenance costs are too low, generalists pay little penalty for breadth. They can photosynthesize, consume, and decompose, occupying all niches simultaneously. The result is generalist dominance — a single cluster in trait space with high values everywhere.

If the mobility sigmoid is too gradual, the producer-consumer divide blurs. Agents can move freely while still photosynthesizing effectively. The fundamental niche separation that drives trophic structure disappears.

If metabolic costs are too low overall, energy accumulates in agents. Populations grow because agents rarely die of starvation. The negative feedback of resource limitation weakens, and the system trends toward population explosion.

Genesis searches for the parameter regime where all trade-offs bite hard enough to force differentiation but not so hard that only one strategy survives.

## Spatial dynamics and stability

The ecology docs on spatial ecology and disturbance describe how space is not just a container but a mechanism. Spatial structure creates stability properties that well-mixed (non-spatial) models lack:

**Local interaction creates spatial refugia.** Because agents sense and interact locally, prey populations can persist in areas away from consumers. This spatial decoupling prevents the global synchronisation that causes simultaneous extinction in non-spatial predator-prey models. The light competition radius and sensing range are the key parameters — they determine how "local" local interaction is.

**Dispersal-competition trade-offs.** Agents that invest in mobility can reach unoccupied patches but pay movement costs. Agents that stay put save energy but face local competition. This trade-off mirrors the colonisation-competition trade-off in succession ecology and contributes to the coexistence of multiple strategies.

**Patch dynamics.** Local extinction and recolonisation create a mosaic of patches at different stages of ecological development. Some patches are producer-dominated (early succession analogue), others have established consumer-producer cycles. This spatial heterogeneity stabilises the system at the landscape scale even when individual patches are unstable.

The simulation's toroidal topology during genesis eliminates edge effects, ensuring that spatial dynamics are driven by agent interactions rather than boundary conditions.

## The role of genesis

Genesis is not just a parameter search — it's a stability filter. The fitness function (ADR-0003) asks: does this parameterisation produce the feedback balance described above? The five criteria map directly to stability properties:

| Criterion | Stability property |
|---|---|
| Oscillation strength | Positive feedbacks are present (populations cycle rather than freezing) |
| Clustering strength | Trade-off pressure produces niche differentiation |
| Coexistence duration | Negative feedbacks prevent competitive exclusion |
| Demographic turnover | The system is metabolically active (not frozen) |
| Trophic balance | Energy flows downhill through trophic levels (pyramid structure maintained) |

The geometric mean ensures all properties must be present simultaneously — a parameterisation cannot compensate for missing oscillations with strong clustering. This mirrors the ecological reality that functioning ecosystems exhibit all of these properties at once.

The ensemble approach (multiple runs per parameterisation) tests robustness: a parameterisation that produces dynamic stability in most runs, regardless of initial conditions and random seed, has the right feedback balance built into its physics rather than depending on lucky starting conditions.

## From system design to architecture

This document describes what the simulation must achieve (dynamic stability through feedback balance) and why (the ecology tells us which feedbacks matter). The ADRs describe how specific mechanisms are implemented:

- **ADR-0002** (universal agent, emergent roles) implements the trade-off engine — one trait vector, continuous space, role as interpretation.
- **ADR-0003** (genesis fitness) implements the stability filter — the five criteria that encode the feedback balance.
- **ADR-0004** (minimal tick loop) implements the feedback mechanisms — the sense-decide-act cycle where all feedbacks operate.
- **ADR-0005** (spore dispersal) implements a spatial dispersal mechanism that contributes to patch dynamics.
- **ADR-0006** (event-driven simulation) implements the causal infrastructure — events, projections, and broadcasts that let agents perceive and respond to the consequences of feedback.

Future ADRs should trace their motivation back to a specific stability property or feedback mechanism described here. If a new mechanism doesn't serve stability (either by adding a missing feedback, strengthening a weak one, or preventing a failure mode), it needs justification for why it belongs in the simulation at all.

## Open questions

**Is single-currency sufficient to suppress generalist dominance?** Trait maintenance costs are the primary anti-generalist mechanism. If genesis consistently finds generalist-dominated parameterisations despite high maintenance costs, the system may need a second currency (analogous to nitrogen limitation) that imposes additional specialisation pressure. The ecology docs on nutrient cycling describe how stoichiometric constraints enforce specialisation in real ecosystems.

**What is the role of disturbance?** The simulation currently has no exogenous disturbance — all dynamics are endogenous (agent-agent interactions). The ecology docs on disturbance and succession argue that intermediate disturbance maintains diversity by preventing competitive exclusion from running to completion. If the simulation produces worlds where one cluster eventually excludes all others despite correct feedback balance, exogenous disturbance (spatial perturbations, energy pulses) may be needed as a diversity-maintenance mechanism.


**Does the event-driven architecture change the stability properties?** ADR-0006 introduces stigmergy — agents responding to traces of past activity rather than live state. This adds temporal delay to feedback loops (agents act on stale information). Delay in feedback loops can either stabilise (damping oscillations) or destabilise (causing overshoot). The interaction between projection decay rates and the feedback mechanisms described here is an open design question that genesis must search.
