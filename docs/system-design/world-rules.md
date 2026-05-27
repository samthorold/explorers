# World Rules

The immutable physics of the simulation. These define what the world is made of and how it behaves before any agent strategy, population dynamic, or emergent pattern is considered. An agent dropped into this world faces these constraints unconditionally.

This document is built on the ecological ground truths in [docs/ecology/](../ecology/) — observable properties of real ecosystems that we take as given. It informs but does not prescribe the architectural decisions in [docs/adr/](../adr/).

## Stocks

The world has two currencies. Energy is the primary currency — it powers all processes and is the currency of metabolism. In addition, the world has a single nutrient that agents require alongside energy. An agent's nutrient demand — how much nutrient it needs per unit energy — is derived from its trait vector: more capable agents need more nutrient. Together, energy and nutrient are the building blocks of every agent.

### Energy stocks

Energy flows through the system in one direction: from source to sink. It does not cycle.

Within a living agent, energy exists in two forms: **reserve** (metabolic fuel — the operating account) and **structure** (embodied biomass — the physical body). Reserve fluctuates rapidly as income and costs are applied each tick. Structure accumulates slowly through growth and is depleted by consumption from other agents. The distinction follows Kooijman's (2010) Dynamic Energy Budget theory, where reserve and structure are separate state variables with different dynamics.

| Stock | Type | Description |
|---|---|---|
| **Solar flux** | Source (inexhaustible) | Energy enters the system here. The flux is constant — the same amount of energy is available every tick. All temporal variation in the system is endogenous. |
| **Living agents (reserve)** | Internal | The metabolic fuel held by living organisms. Reserve increases through energy acquisition (photosynthesis, consumption) and decreases through metabolic costs, growth, and reproduction. Reserve is the central clearing account — all energy income and expenditure flows through it. |
| **Living agents (structure)** | Internal | The embodied biomass energy of living organisms. Structure accumulates when an agent allocates reserve surplus to growth (a lossy conversion). Structure is depleted when another agent consumes the body. Structure is not a searched world parameter or a heritable trait — it is a state variable that starts at zero for newborns and grows over the agent's lifetime. Living agents also accumulate somatic wear — see below. |
| **Carcasses** | Internal | Energy held by dead organisms. When an agent dies, its structure (embodied biomass) and nutrient become a carcass at the same location on the surface. A carcass's energy content reflects the agent's accumulated structure at death. Carcasses hold energy indefinitely — there is no passive decay. |
| **Heat** | Sink (inexhaustible) | Energy leaves the system here, permanently. Every lossy process — metabolism, trophic transfer, growth conversion, reproduction inefficiency — sends energy to this sink. It does not return. |

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

> Total system energy = Σ(living agent reserve) + Σ(living agent structure) + Σ(carcass energy)

The change in total system energy per tick equals:

> ΔE = photosynthetic input − total dissipation to heat

Energy is neither created nor destroyed within the system. Every flow is accounted for. Reserve-to-structure conversion (growth) is internal to a living agent and does not change total system energy, but the conversion is lossy — the difference dissipates to heat.

**Nutrient conservation.** For the nutrient:

> Total nutrient = available pool + Σ(living agent nutrient) + Σ(carcass nutrient) + unavailable pool

Nutrient is neither created nor destroyed. It cycles between pools. The unavailable pool changes only through geological-timescale processes (weathering, deposition), not through biological activity.

If either conservation law fails, the world is broken.

### No passive decay

Carcasses do not lose energy or nutrient on their own. Decomposition is always an agentic process — a living agent must actively consume a carcass to return its energy and nutrient to the living system and available pool. This makes decomposer strategies structurally necessary for the world's nutrient cycle. A world without decomposers accumulates resources in the dead pool until the living system starves.

This is not a fragility to be patched with a safety valve. In real ecosystems, what appears to be passive decay is always decomposition by organisms at a finer resolution — bacteria, fungi, invertebrates. The principle is: **all resource transformation requires an agent**.

### Somatic wear

Living agents degrade over time. Every functional capability — photosynthetic apparatus, locomotion machinery, sensory organs, digestive systems — accumulates wear through use and through the baseline cost of maintaining complex machinery. This is the disposable soma principle (Kirkwood 1977): an organism could theoretically maintain its body indefinitely, but the energy required for perfect repair is energy unavailable for reproduction. The body is disposable because investing in immortality is a losing strategy when extrinsic mortality exists.

Wear accumulates per functional trait, not as a single aggregate. An agent that photosynthesizes heavily wears out its photosynthetic apparatus faster than an agent in shade. An agent that sprints wears out its locomotion faster than a sessile one. The rate of wear accumulation emerges from the agent's trait-space position and activity — not from flat rate parameters.

Wear reduces trait effectiveness. Small wear barely affects output — biological systems have redundancy and over-provisioning. But wear compounds, and later increments are increasingly costly. An aging producer captures less light. An aging consumer catches prey less efficiently. An aging agent senses less, moves slower, processes food less effectively.

Agents invest in somatic maintenance to counteract wear. The level of investment is governed by kappa — the heritable fraction of surplus energy allocated to soma vs. reproduction. Some lineages evolve high kappa (long-lived, slow-reproducing), others evolve low kappa (short-lived, fast-reproducing). Somatic maintenance is a whole-organism investment: an agent does not selectively repair one organ while neglecting another. The energy allocated to maintenance competes directly with the energy available for reproduction — this is the core trade-off that Kirkwood identified.

The balance between wear accumulation and repair must produce three properties:
- **Stable equilibrium.** For a given kappa and activity level, an agent reaches a steady-state wear level where repair balances accumulation. At equilibrium, effective traits are degraded but still functional.
- **Activity-dependent equilibrium.** Higher throughput (more photosynthesis, more consumption, more movement) increases the equilibrium wear level. Busier agents age faster, but stabilise at a lower level of function rather than spiralling to death.
- **Trait-space-derived lifespan.** An agent's position on the survive-vs-reproduce axis determines its lifespan. High kappa produces long-lived agents with low equilibrium wear. Low kappa produces short-lived agents with high equilibrium wear. The lifespan gradient is continuous, not binary.

Behavioural traits — kappa, fecundity — do not wear. They are allocation parameters, not physical machinery. An old organism is less capable, not less decisive.

Identity-determining thresholds use an agent's nominal (unworn) traits. An agent born mobile is ecologically mobile for its entire life, even as its effective mobility degrades with age. Wear degrades performance, not identity. An aging wolf does not become a plant.

**Offspring are born with zero wear.** This is the evolutionary rationale for reproduction: it resets the soma. The germline is maintained at higher fidelity than the soma. A parent's worn-out body produces a brand-new body. The information (heritable traits) persists in pristine hardware (the offspring). This asymmetry between germline and soma is what makes the disposable soma trade-off coherent — repair your body, or build a new one.

Grounded in the disposable soma theory (Kirkwood 1977) and antagonistic pleiotropy (Williams 1957). Real organisms senesce — decline in physiological function with age — because energy invested in somatic repair competes with reproduction. The rate of senescence is calibrated to the extrinsic mortality rate: organisms in high-mortality environments evolve faster senescence because investing in longevity beyond the expected lifespan is wasteful (Reznick et al. 2004, Austad 1993). This produces the survivorship curve continuum: Type I (low early mortality, concentrated late-life death — K-strategists with high somatic investment), Type II (constant mortality across age classes), and Type III (high juvenile mortality, low adult mortality — r-strategists with low somatic investment). In the world physics, the position on this continuum emerges from the agent's kappa — the heritable soma-vs-reproduction allocation fraction. The wear/repair mechanic must produce a continuous gradient of lifespans, not a binary outcome.

## Flows

Eight flows move energy and nutrient between stocks. Each flow is a rate — resources per tick — and each is subject to constraints described below. Energy and nutrient travel together through most flows (an agent that eats another agent acquires both energy and nutrient), but they have different fates: energy is progressively dissipated to heat, while nutrient cycles back through consumption of carcasses. Reserve is the central clearing account for energy: all income enters as reserve, all costs are paid from reserve, and surplus reserve can be converted to structure through growth.

### Input flows

**1. Photosynthesis.** Solar → Living agent (reserve). The only way energy enters the living system. Producers absorb energy from the constant solar flux into their reserve. This flow is attenuated by local competition — co-located producers share the available flux, weighted by both autotrophy investment and structure (body size). Bigger producers shade smaller ones, creating a growth feedback loop: an established producer that has accumulated structure captures a larger share of light, grows faster, and shades newcomers further. Any agent with autotrophy investment can photosynthesize — there is no gate. The producer/consumer divide emerges from two structural constraints: superlinear maintenance costs (investing heavily in both autotrophy and heterotrophy is prohibitively expensive) and the nutrient bottleneck (photosynthesis produces energy but not nutrient; an agent that photosynthesizes must also acquire nutrient via uptake from the available pool, which requires sustained substrate contact). Photosynthesis moves energy only — nutrient must be acquired separately through nutrient uptake (flow 2).

Grounded in terrestrial plant photosynthesis. Real producers capture a small fraction of incident solar radiation (1–2% of PAR; Monteith 1972, Zhu et al. 2010), and roughly half of gross primary production is consumed by the producer's own respiration (Gifford 2003). The remainder — net primary production — is the energy base for the rest of the ecosystem. In the world physics, solar flux is the external energy source and light share models spatial competition: an isolated producer receives full flux, a crowded producer receives a fraction weighted by autotrophy investment and accumulated structure. Larger producers shade smaller ones — a size-structured asymmetric competition consistent with real canopy dynamics (Weiner 1990). Photosynthesis produces energy but not nutrient — the producer must acquire nutrient separately through substrate uptake, which requires sustained contact. This couples energy capture to physical stillness: a mobile agent can photosynthesize but cannot access the nutrient needed to reproduce, making mobile photosynthesis a demographic dead end.

**2. Nutrient uptake.** Available pool → Living agent. Agents extract nutrient from the available pool at their location. The uptake rate depends on two factors: the agent's autotrophy investment (which governs both light capture and nutrient extraction — the same sessile infrastructure serves both) and the agent's contact time at its current location. An agent that has remained stationary for many ticks extracts nutrient more effectively than one that just arrived — sustained contact with the substrate is required to establish the interface structures (analogous to roots) through which nutrient is extracted. Uptake follows Michaelis-Menten saturation: `rate = autotrophy × contact_time / (contact_time + k)`, where k is a half-saturation constant (currently 50 ticks). This means uptake increases with contact time but plateaus — an agent that has been stationary for 500 ticks extracts only marginally more than one stationary for 100 ticks. Moving resets contact time. Co-located agents share the available pool proportionally, weighted by their effective uptake rate. Nutrient uptake is the only way nutrient enters the living system.

Grounded in root-mediated nutrient uptake. Real producers extract mineral nutrients from soil solution through root surfaces — a process that requires physical infrastructure (roots, root hairs, mycorrhizal associations) built and maintained over time (Lambers, Chapin & Pons 2008). Uptake is not instantaneous; it depends on the extent of the interface an organism has established with its substrate. In the world physics, contact time models this infrastructure investment: an agent that has remained stationary builds up its interface with the substrate, increasing uptake effectiveness. Moving destroys the interface and resets the process. Uptake saturates — diminishing returns on long residence reflect the depletion of the local available pool around the uptake interface. The available pool is spatially heterogeneous and shared among co-located agents, creating local competition for nutrient independent of competition for light.

### Flows between living agents

**3. Consumption.** Agent (structure) → Living agent (reserve). A consumer eats a target's body through sustained physical contact, draining the target's structure. The target can be a living agent or a carcass — the physics are identical, governed by the same heterotrophy trait. A carcass is an inert agent: it has structure, nutrient, a position, and a trait vector, but it does not act. Decomposition is consumption where the target happens to be dead — it requires no separate capability. The drained structure enters the consumer's reserve, lossy — the fraction retained depends on the trait-space distance between consumer and target (flow 7); the remainder dissipates to heat. Nutrient transfers alongside structure, but the consumer retains only what it needs according to its stoichiometric demand — excess nutrient is excreted immediately to the available pool at the consumption site, never incorporated. The limiting currency (energy or nutrient) constrains how much of the consumed material the consumer can actually use. Consumption of a living target is non-lethal by default; the target survives unless its structure drops below its complexity-dependent death threshold or its reserve reaches zero.

Grounded in animal consumption of living tissue and saprotrophic decomposition. Real consumers face two distinct modes: partial consumption (grazing, browsing) removes tissue but leaves the target alive and capable of regrowth, while lethal consumption removes the target from the living system entirely, creating a carcass (McNaughton 1979; Begon et al. 2006). Both modes operate through the same physical act — the consumer eats the target's body — and both are governed by the same heterotrophic capability. The ecological consequences diverge based on how much structure is removed relative to what the target can survive. In the world physics, this distinction emerges rather than being prescribed: consumption drains target structure, and the target's complexity-dependent death threshold determines whether the interaction is survivable. Whether the target is alive or dead does not change the trait required — heterotrophy applies to all consumption. Trophic transfer efficiency is governed by trait-space distance between consumer and target — consuming a biochemically similar agent yields more usable energy and nutrient than consuming a dissimilar one. In terrestrial ecosystems, direct consumption of living tissue accounts for only 5–15% of net primary production (Cyr & Pace 1993); the majority of energy reaches consumers through the detrital pathway — consumption of carcasses. The world physics should not assume consumption of living targets is the primary energy pathway for all consumer strategies. Real decomposition is dominated by fungi and bacteria (Wilson & Wolkovich 2011). Substrate quality controls decomposition rate: the lignin-to-nitrogen ratio of dead material determines how quickly decomposers can process it (Melillo, Aber & Muratore 1982). In the world physics, trait-space distance between consumer and carcass governs efficiency — a decomposer biochemically similar to its target extracts more usable energy and nutrient.

**4. Reproduction.** Living agent (reserve) → Living agent. Parents invest reserve and nutrient to create offspring. Reproduction requires sufficient nutrient — nutrient limitation blocks reproduction even when energy is abundant. The energy transfer is lossy — a fraction dissipates to heat (flow 8). Both parents survive. Reproduction has two modes, variable fecundity, and spatial dispersal of offspring.

**Sexual reproduction** is the primary mode. Two compatible agents within reproductive range each invest energy. Offspring traits are derived from both parents through crossover (each trait dimension drawn randomly from one parent) with heritable mutation. Compatibility is determined by trait-space distance — agents with similar traits are more likely to mate. Reproductive isolation is an emergent property of trait divergence, not a separate trait. There is no mate selectivity parameter; divergent lineages become reproductively isolated because their trait vectors are distant.

**Asexual reproduction** is governed by an agent's asexual propensity trait — it is not a universal fallback. When an agent has sufficient energy and asexual propensity to reproduce but no compatible mate is available, it can reproduce alone. Offspring traits are the parent's traits plus mutation — no crossover, because there is no second parent. The costs of asexual reproduction are inherent: lower offspring variation (no recombination) and a single parent's energy contribution (less total investment per offspring). These costs create selection pressure: in dense populations where mates are available, sexual reproduction is advantageous because it generates more combinatorial diversity. In sparse populations or for isolated colonizers, asexual reproduction is the only option. Whether a lineage relies primarily on sexual or asexual reproduction is not prescribed — it is an emergent outcome of population density, mate availability, and the fitness value of variation in a given environment.

**Fecundity** — the number of offspring per reproductive event — is a heritable trait. A fixed total energy budget is invested per event; fecundity determines how many offspring share that budget. High fecundity produces many poorly-provisioned offspring (r-strategy). Low fecundity produces few well-provisioned offspring (K-strategy). The actual offspring count for a given event is stochastic — drawn from a Poisson distribution with mean equal to the fecundity trait. This stochasticity means reproductive failure (zero offspring despite energy investment) is possible. The energy cost of reproduction is committed before the outcome is determined — failed reproduction is costly, as it is in nature.

For sexual reproduction, the effective fecundity is the average of the two parents' fecundity traits. Because fecundity is part of the trait vector, it contributes to trait-space distance and therefore to mate compatibility — agents with very different reproductive strategies are less likely to mate. This creates reproductive isolation along the r/K axis without any special mechanism.

**Offspring dispersal** follows a Gaussian kernel centred on the parent's position. Most offspring land nearby; a tail of long-distance dispersers enables colonisation of distant habitat. The dispersal radius is scaled by the parent's reproductive range — wide for sessile agents (spore/seed dispersal) and narrow for mobile agents (who can disperse under their own locomotion after birth). Each offspring in a clutch is placed independently, so siblings from the same event scatter across the dispersal range.

Grounded in life history allocation theory. Real organisms face a fundamental budget problem: energy and nutrient invested in offspring is unavailable for self-maintenance, growth, or future reproduction (Williams 1966). The core axes of variation are offspring number vs. quality (Smith & Fretwell 1974), current vs. future reproduction, and the mode of reproduction itself — sexual (recombination generates variation, but requires mate-finding) vs. asexual (no mate cost, but offspring are less variable). Sessile organisms must disperse gametes or propagules through the environment; mobile organisms can seek mates directly but pay locomotion and predation-risk costs to do so. In the world physics, reproduction is an energy and nutrient transfer from parent to offspring, lossy like all transfers. Fecundity determines how a fixed investment is divided among offspring — many fragile or few robust. Sexual reproduction requires trait-space proximity (compatibility) and physical proximity (contact or spore dispersal scaled by contact time), coupling reproductive success to both ecological similarity and spatial context. Nutrient limitation blocks reproduction even when energy is abundant, reflecting the stoichiometric constraint that real organisms face: building offspring requires specific material ratios, not just calories.

**5. Network redistribution.** Living agent ↔ Living agent. Energy and nutrient move between living agents through network connections (see Topologies below). This flow is cooperative, not adversarial — it is distinct from consumption. It is bidirectional: resources can flow in either direction through a connection, governed by the states of the connected agents. This is the mechanism by which mutualistic relationships become possible. Like all transfers, the energy component is lossy (flow 8).

Grounded in mycorrhizal network exchange. Real ecosystems contain transport infrastructure built by organisms — primarily fungal hyphae and root systems — that move resources between producers (Simard 2018). This is not passive diffusion; it is a market-like exchange governed by local supply-demand ratios (Kiers et al. 2011). Plants allocate more carbon to fungal partners that deliver more nutrient, and fungi allocate more nutrient to plant partners that deliver more carbon. The network transforms point-to-point competition for local resources into a connected system where resources flow along supply-demand gradients. Hub nodes (large, well-established agents) disproportionately influence system-wide distribution. In the world physics, network redistribution is bidirectional and cooperative — distinct from consumption, which is adversarial. The network is built and maintained by agents that invest in it, at an energy cost. Resources flow through network connections independent of surface distance, enabling redistribution between agents that are spatially separated. The network acts as a buffer against local resource shocks, damping oscillations by redistributing from surplus to deficit.

### Flows from living to dead

**6. Death.** Living agent → Carcass. When a living agent dies, its remaining structure and nutrient become a carcass at the same location on the surface. The carcass's energy content is the agent's structure at the moment of death. The nutrient content reflects the nutrient the agent had incorporated.

Death has two triggers — either is sufficient:

- **Reserve depletion.** Reserve reaches zero. The agent can no longer fund metabolism. This is starvation (metabolic costs exceed income) or the terminal consequence of predation draining a target that was already energy-marginal.
- **Structural depletion.** Structure drops below a complexity-dependent threshold. The threshold scales with trait spread: an agent whose investment is concentrated in few dimensions (low complexity) tolerates greater structural damage — like a moss that functions the same at half its mass. An agent whose investment is spread across many dimensions (high complexity) is fragile — specialised organs and systems cannot sustain partial loss. The threshold is derived from the trait vector, not a separate parameter.

Death also has two distal causes. **Extrinsic mortality** — consumption drains structure below the threshold, or competition/shading makes the agent energy-negative until reserve depletes. **Intrinsic mortality** — somatic wear degrades functional traits until the agent can no longer acquire enough energy to cover its metabolic costs, then reserve slowly depletes. In practice these compound: an aging agent with degraded autotrophy that was previously viable becomes energy-negative when a competitor shades it, or an aging consumer whose effective heterotrophy has declined can no longer catch sufficient prey. Somatic wear makes death inevitable on a long enough timeline — even in the absence of predation or competition, an agent whose kappa is too low to keep pace with wear accumulation will eventually degrade to the point of energy bankruptcy.

Grounded in mortality ecology across all kingdoms. Real organisms die from two broad causes: extrinsic mortality (predation, disease, physical disturbance) and intrinsic mortality (senescence — the progressive failure of somatic maintenance; Kirkwood 1977). These causes compound: an aging organism with degraded function that was previously viable becomes fatally vulnerable when conditions shift (Ricklefs 2008). Death transfers embodied energy and nutrient from the living stock to the dead stock — creating the carcass pool that funds decomposition. What the organism accumulated over its lifetime determines the value of the carcass: a long-lived, well-fed agent leaves an energy-rich carcass; a starved juvenile leaves an energy-poor one. In the world physics, death has two triggers — reserve depletion (starvation) and structural depletion below a complexity-dependent threshold — reflecting the distinction between metabolic failure and structural collapse. Both routes produce a carcass that retains the dead agent's structure, nutrient, and trait vector.

### Dissipation

**7. Trophic transfer loss.** Accompanies flows 3, 4, 5, and 9. At every energy conversion — between agents or between reserve and structure within an agent — a fraction is lost to heat. This is not a separate event — it is the inefficiency inherent in every conversion. It is what makes trophic pyramids inevitable: each level of transfer dissipates energy, so less is available at each successive level. The path from target structure → consumer reserve → consumer structure involves two lossy conversions, compounding the loss. Nutrient is not lost to heat — it is either incorporated into the receiving agent or returned to the available pool.

The fraction lost varies with the relationship between consumer and target. Agents that occupy similar positions in trait space are biochemically similar — their structure is built from compatible machinery. Consuming a biochemically similar target is inherently more efficient: less energy is wasted breaking down unfamiliar structures, more of the target's embodied energy and nutrient is usable. Consuming a biochemically dissimilar target is less efficient: more energy is lost in conversion. Trait-space distance between consumer and target determines trophic transfer efficiency. This produces the ecological pattern without a special rule: consumers eating other consumers (small trait distance) capture a high fraction of energy and nutrient per unit consumed, while consumers eating producers (large trait distance) capture a low fraction. The trophic chain's increasing per-interaction efficiency at higher levels — and the corresponding decrease in available biomass — emerges from trait-space geometry.

Grounded in Lindeman's (1942) trophic-dynamic concept. Real trophic transfer efficiency ranges from 5–20% between trophic levels (Welch 1968), decomposed into assimilation efficiency (fraction of consumed energy absorbed) and net production efficiency (fraction of assimilated energy converted to biomass). Assimilation efficiency varies systematically with diet similarity: carnivores assimilate 60–90% of ingested energy because animal tissue is chemically similar to consumer tissue, while herbivores assimilate 15–80% depending on food type (Begon et al. 2006). Net production efficiency varies with thermal strategy: ectotherms convert 25–75% of assimilated energy to biomass, while endotherms rarely exceed 5% due to thermoregulatory costs. In the world physics, trophic transfer efficiency is derived from trait-space distance between consumer and target, producing the ecological pattern without a special rule: consumers eating biochemically similar agents retain a higher fraction than consumers eating dissimilar agents.

**8. Metabolism.** Living agent (reserve) → Heat. Every living agent pays a continuous energy cost from reserve simply to exist. This cost has three components:
- A base rate — the minimum cost of being alive, independent of traits or activity.
- Trait maintenance costs — each capability (autotrophy, heterotrophy, sensing, mobility) costs energy to maintain whether or not it is currently in use. These costs scale superlinearly with investment, making them the mechanism behind the specialist-generalist trade-off: an agent investing in multiple capabilities pays accelerating overhead for all of them.
- Somatic maintenance — governed by kappa, the fraction of surplus energy allocated to soma vs. reproduction. Higher somatic allocation slows aging but competes directly with the energy available for reproduction and growth. This is the mechanism behind the reproduce-vs-survive trade-off.

Metabolism dissipates reserve to heat. It does not release nutrient — nutrient leaves living agents only through death (flow 6). This makes decomposition structurally necessary for nutrient cycling.

Grounded in metabolic theory and DEB theory. Real organisms pay a continuous energy cost to exist — basal metabolic rate scales with body mass as M^(3/4) (Kleiber 1932; West, Brown & Enquist 1997). Metabolic cost has multiple components: basal maintenance, activity costs (locomotion at 10–15× basal rate during active movement; Schmidt-Nielsen 1972), and the cost of maintaining functional machinery whether or not it is in use. DEB theory (Kooijman 2010) formalises maintenance priority: when reserves are insufficient, maintenance takes precedence over growth and reproduction — the organism cannibalises its own capacity to stay alive. In the world physics, metabolism is the sum of base rate and superlinear trait maintenance costs. Each capability costs energy to maintain regardless of use, and the cost accelerates with investment level, creating the specialist-generalist trade-off: agents carrying broad trait investment pay accelerating overhead for machinery spread across many functions.

**9. Growth.** Living agent (reserve) → Living agent (structure). When an agent's reserve income exceeds its metabolic costs, it can allocate surplus reserve to structure — building its body. The conversion is lossy; a fraction dissipates to heat (flow 7). Growth is not a decision — it is an automatic consequence of being well-fed. Structure starts at zero for newborn agents and accumulates over the agent's lifetime. Growth rate depends on the available reserve surplus after metabolism, reproduction, and other costs are paid.

Grounded in DEB theory's reserve-to-structure conversion. Real organisms grow by converting metabolic surplus into embodied biomass — a lossy process constrained by the balance between assimilation and maintenance. DEB theory predicts that growth rate decelerates as body size increases, because maintenance costs scale with existing structure while assimilation scales with surface area — producing the von Bertalanffy growth curve as an emergent property (Kooijman 2010). Growth is not a decision but an automatic consequence of surplus: a well-fed organism grows, a starving organism does not. Structure once built is durable but not free — it must be maintained, creating a feedback where larger agents have higher maintenance costs and need more income to sustain themselves. In the world physics, growth converts reserve surplus to structure after metabolism and reproduction costs are paid. The conversion is lossy, with a fraction dissipated to heat. Structure starts at zero for newborns and accumulates over the agent's lifetime.

### Flow summary

Energy flows one way: source → living system → heat. All energy income enters as reserve. Reserve funds metabolism, growth, and reproduction. Growth converts reserve to structure (the body). Consumption drains structure (from living targets and carcasses alike) into the consumer's reserve. Nutrient cycles: available pool → living agents → carcasses → (via consumption of carcasses) → available pool. Nutrient leaves living agents only through death — metabolism releases reserve to heat but not nutrient.

```
Solar ──photosynthesis──▶ Reserve ──growth──▶ Structure ──death──▶ Carcass
  (source)                  │                    ▲                    │
                        metabolism           consumed by          consumed by
                        reproduction         another agent        another agent
                            │                    │                    │
                            ▼                    ▼                    ▼
                          Heat            Consumer's reserve   Consumer's reserve
                          Offspring        (with trophic loss)  (with trophic loss)
                                                 │
                                            ▲    │
                  Available pool ◀───────── │ ───┘
                       │              nutrient excretion
                  nutrient uptake     (stoichiometric mismatch,
                       │               consumption of carcasses)
                       ▼
                  Living agent (nutrient)
```

## Viability constraints

The flows above describe what can happen. Viability constraints describe what the flows must be capable of producing for the ecology to function. These are not parameter values — they are design requirements on the physics. If any constraint cannot be satisfied by some agent configuration under the maintenance cost landscape, that trophic role is structurally impossible and the ecology is broken.

### Individual balance

Every agent has two currencies — energy and nutrient — each with its own income and cost structure per tick.

**Energy income** enters reserve through acquisition flows: photosynthesis (flow 1) or consumption (flow 3, which covers both living targets and carcasses). Each flow's yield depends on the agent's trait investment (autotrophy or heterotrophy), the availability of the resource it targets, and amplifying factors like accumulated structure.

**Energy cost** leaves reserve through metabolism (flow 8): base rate, trait maintenance, and somatic maintenance (governed by kappa). Cost is determined by the agent's trait vector and activity.

**Nutrient income** enters the agent through uptake from the available pool (flow 2) or alongside energy during consumption (flow 3). Uptake requires sustained substrate contact. Consumption transfers nutrient directly from the target (living or carcass), bypassing the contact-time requirement.

**Nutrient cost** is driven by reproduction. Nutrient limitation blocks reproduction but does not impair other functions. An agent that is nutrient-negative cannot replace itself even if it is energy-positive.

An agent is **viable** when it can achieve positive balance in both currencies over the timescale needed to grow, reproduce, and replace itself. Energy viability without nutrient viability produces agents that survive but cannot reproduce — a demographic dead end. Nutrient viability without energy viability is impossible, since the agent starves before it can reproduce.

### Trophic viability

Each trophic role imposes specific viability requirements across both currencies.

**Producers.** Photosynthetic income must exceed producer metabolic cost. The difference — net primary production — is the energy available to the rest of the ecosystem. If producers barely break even, nothing else can live. Producers acquire nutrient through uptake from the available pool, gated by contact time. A sessile producer-specialist accumulates contact time naturally. The physics must allow such an agent to generate substantial energy surplus while acquiring enough nutrient to reproduce.

**Consumers.** Consumer energy income — whether from grazing living agents or from consuming carcasses — must exceed consumer metabolic cost. Consumer income is downstream of net primary production: consumers can only extract what producers make available as structure. Consumers acquire nutrient alongside energy through consumption, bypassing the contact-time gate. But the nutrient content of prey depends on the prey's own nutrient accumulation — consumers inherit whatever nutrient-to-energy ratio their targets carry. A consumer whose stoichiometric demand exceeds what prey provides is nutrient-limited despite successful feeding.

**Decomposers.** Decomposer energy income from processing carcasses must exceed decomposer metabolic cost. Decomposer income is downstream of mortality: decomposers can only extract what death makes available. Decomposers also acquire nutrient from carcasses, and excrete excess to the available pool — closing the nutrient cycle. A world without viable decomposers locks nutrient in carcasses indefinitely, eventually starving producers of nutrient even while solar flux continues to provide energy.

## Topologies

The world has two topologies. Each topology is a geometry on which agents interact. Different flows operate on different topologies.

### Physical surface

A two-dimensional continuous surface. Movement happens here. Every agent and every carcass has a position on this surface.

The surface is the topology for all spatially-local interactions. Photosynthesis, consumption, nutrient uptake, death, and reproduction all require surface proximity. Sensing, chemotaxis, and light competition also operate on the surface.

### Emergent network

A graph that agents build through their behaviour. Nodes are agents; edges are connections that agents create and maintain. The network is not constrained by surface distance — two agents far apart on the surface can be adjacent on the network if they have built a connection between them.

Network-building is a general capability: any agent can invest in creating connections. The cost structure makes it only worthwhile for certain trait configurations. In real ecosystems, this role is filled by organisms with specific morphology — fungal hyphae, root systems — but in a universal-agent system, which agents build network infrastructure is emergent, not prescribed.

The network enables flows and perception that bypass surface locality. Its topology, density, and reach are all emergent properties.

## Channels

A channel defines what can flow between agents — information, energy, or both — and on which topology. An agent's trait vector determines the range at which it can interact on each channel. Two agents are in **contact** on a channel when they are within the range that their traits allow on that channel's topology.

### Channel types

Channels are distinguished by whether they can carry energy.

**Perception channels** carry information only. They tell an agent what is nearby and in what direction, enabling decision-making, but they do not move energy or nutrient. Looking at food does not feed you; detecting a chemical signal does not transfer resources.

**Physical channels** carry energy and nutrient between agents. They are the pathways through which trophic flows operate. Consumption, reproduction, nutrient uptake, and network redistribution all require a physical channel.

This distinction is a law of physics, not a strategy choice. No amount of trait investment can make a perception channel carry energy. Energy transfer always requires a physical channel — either physical contact on the surface or a connection on the network.

### Channels on the surface

The surface carries both perception and physical channels. Each operates at a range determined by the interacting agents' traits.

- **Surface perception** — detecting agents and carcasses on the surface. Chemical gradients, visual detection, vibration. The range at which an agent perceives its surroundings is determined by its trait investment in sensing. Signals are distance-weighted: closer agents produce stronger signals.
- **Surface contact** — physical interaction on the surface. Required for consumption, reproduction, and nutrient uptake. The range at which an agent can physically interact is determined by its trait vector — not a uniform world parameter. An agent's physical reach is a property of the agent, not the world.

### Channels on the network

The network carries both perception and physical channels.

- **Network perception** — signals propagated through network connections. Chemical alerts, resource-state information. An agent connected to the network can perceive the states of connected agents regardless of surface distance.
- **Network transfer** — resource movement through network connections. Required for network redistribution (flow 5). This is what makes the network more than a signalling system — it is infrastructure that moves energy and nutrient.

### Sustained contact

Many physical interactions are not instantaneous — they require sustained contact over multiple ticks. Nutrient uptake depends on how long an agent has maintained contact with the substrate. Consumption drains structure over time through continued physical contact. The effectiveness of sustained-contact interactions increases with duration, reflecting the time needed to establish the physical interface through which resources transfer. Moving breaks sustained contact and resets the process.

Sustained contact creates a fundamental trade-off on the surface: mobile agents can reach more targets but cannot maintain contact long enough for slow, high-yield interactions. Sessile agents sacrifice reach but gain the deep contact needed for nutrient uptake and sustained feeding.

## Cost structure (trade-offs)

The cost structure is what prevents any agent from being good at everything. The primary mechanism is **superlinear maintenance costs**: the energy cost of maintaining each trait grows faster than linearly with investment, so piling capability into many traits simultaneously becomes prohibitively expensive. This smooth, convex cost landscape, combined with nutrient constraints and structural fragility, produces nine fundamental trade-offs:

**1. Acquire vs. maintain.** Every capability costs energy to maintain whether or not it is currently in use. More capability means more overhead. Superlinear maintenance costs ensure that total effective capability is self-limiting — each additional unit of investment in a trait costs more to maintain than the last.

**2. Sessile vs. mobile.** Sessile and mobile strategies face fundamentally different resource access. The sessile strategy dominates for autotrophs because sunlight is ambient — mobility costs energy for zero additional energy income. A mobile autotroph pays locomotion costs but does not reach more light than a stationary one. The divide is reinforced by contact-time gating: sustained substrate contact is required for both nutrient uptake and spore dispersal, so mobile agents forfeit both. No explicit gate prevents mobile agents from photosynthesizing — the economics of ambient light and the nutrient bottleneck make it unviable.

**3. Reproduce vs. survive.** Energy invested in offspring is energy not available for self-maintenance. Energy allocated to somatic maintenance via kappa (slowing wear) is energy not available for reproduction. This is the disposable soma trade-off: build a new body or repair the current one. Both achieve the same goal — keeping a lineage alive — through different strategies. High kappa produces long-lived agents that reproduce infrequently. Low kappa produces short-lived agents that reproduce early and often. The optimal position on this continuum depends on the extrinsic mortality rate: when predation is high, investing in longevity is wasteful because the agent is likely to be eaten before aging matters.

**4. Few quality vs. many fragile offspring.** A fixed reproductive energy budget can produce few well-provisioned offspring (K-strategy) or many poorly-provisioned offspring (r-strategy). Fecundity determines how many offspring share the budget. High-fecundity offspring start with less energy and are closer to metabolic death — they must establish an energy income quickly or die. Low-fecundity offspring start with more energy and can weather adverse conditions. Neither strategy dominates — their relative success depends on environmental context. In stable, competitive environments, few well-provisioned offspring outcompete many fragile ones. In disturbed or empty environments, many fragile offspring colonise faster.

**5. Specialist vs. generalist.** Superlinear maintenance costs make this trade-off inescapable: the total cost of maintaining two traits at moderate investment exceeds the cost of maintaining one trait at high investment. A specialist concentrating in one capability achieves higher net return in that niche than a generalist spreading investment across many. The generalist can exploit multiple niches but is outcompeted by any specialist within the specialist's niche. This is a mathematical property of convex maintenance costs, not a parameter to be tuned.

**6. Sense vs. save.** Wider sensing range enables better decisions but costs energy through superlinear maintenance. Agents must balance the value of information against its price in metabolic overhead.

**7. Stoichiometric constraint.** Agents need nutrient in amounts determined by their traits — more capable agents demand more nutrient per unit energy. Food sources have varying nutrient-to-energy ratios. An agent consuming nutrient-poor food must eat more to satisfy its nutrient demand, wasting excess energy as heat. An agent consuming nutrient-rich food retains what it needs and excretes the excess nutrient to the available pool. The limiting currency (energy or nutrient) constrains how much of the consumed material the consumer can actually use.

**8. Sexual vs. asexual reproduction.** Sexual reproduction produces more diverse offspring (crossover recombines two parents' traits) but requires a compatible mate — a cost in mate-finding, energy, and the constraint that both parents must be present. Asexual reproduction requires no mate but produces less diverse offspring (parent traits plus mutation only). In dense populations with coevolutionary pressure (predator-prey arms races, competition), variation is valuable and sexual reproduction is favoured. In sparse populations or stable environments, the mate-finding cost outweighs the variation benefit and asexual reproduction is favoured. Which strategy a lineage relies on is emergent, not prescribed.

**9. Complexity vs. structural resilience.** An agent's structural death threshold — the fraction of peak structure below which structural damage is fatal — scales with trait spread. An agent with investment concentrated in few traits (specialist) has a simple body plan that tolerates significant structural loss; a moss at half its mass is still a functioning moss. An agent with investment spread across many traits (generalist) has complex, interdependent systems that cannot sustain partial loss. This makes generalists more vulnerable to consumption — a predator needs to eat less of a complex agent to kill it. Combined with the specialist-generalist trade-off (#5), this means broad trait investment is penalised twice: superlinear maintenance overhead and greater structural fragility.

These trade-offs are the differentiation engine. They do not prescribe what roles emerge — they create the selection pressure that makes role differentiation advantageous. Superlinear maintenance costs guarantee this pressure is always present — it cannot be overcome by acquiring more energy.

## Design constraints on mechanism choice

### Smooth parameter landscape

Every mechanism in the world rules will be placed into a search algorithm (world genesis) that explores parameter space to find initial conditions producing the expected ecological properties. This imposes a meta-constraint on mechanism design: **mechanisms must produce smooth, gradient-rich responses to parameter changes.** Hard thresholds, cliffs, and vast flat regions in parameter space make genesis search intractable — the optimiser needs gradient signal everywhere, not just near the target.

Concretely:
- Prefer continuous costs over binary gates. A trait that becomes gradually less viable as its cost increases is searchable; a trait that is either free or forbidden is not.
- Prefer superlinear scaling over hard ceilings. A cost that grows quadratically with investment creates a smooth trade-off surface; a budget that enforces a hard sum-to-one constraint creates a simplex with edges.
- Prefer independent, individually-tunable knobs over coupled constraints. When one parameter controls multiple trade-offs simultaneously, the search cannot adjust one without disturbing others.
- Avoid mechanisms whose behaviour changes qualitatively at a threshold. Phase transitions in the physics create discontinuities in the fitness landscape that the optimiser must jump across rather than descend.

This constraint does not mean every mechanism must be simple. It means every mechanism must degrade gracefully under perturbation — a small change in parameters should produce a small change in emergent behaviour.
