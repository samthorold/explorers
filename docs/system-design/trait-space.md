# Trait Space

The heritable dimensions that define an agent. These are the axes of variation that evolution operates on — the properties that make one lineage different from another. This document describes what the dimensions are and why they exist. It does not prescribe data structures, algorithms, or parameter values — those are implementation concerns that live with the code. It does not describe the underlying ecological principles — those are in [docs/ecology/](../ecology/).

This document is built on the ecological ground truths in [docs/ecology/](../ecology/) and the world rules in [world-rules.md](world-rules.md). It informs but does not prescribe how the implementation encodes these dimensions in code.

## Framework: physics vs. trait vs. derived

Not every agent property needs its own dimension. Three categories determine where a property belongs:

**Physics.** Universal rules, same for all agents, not evolvable. World parameters. These are knobs for genesis search, not part of the agent's heritable identity. Example: reproductive compatibility distance (the trait-space distance threshold below which two agents can mate). All agents face the same threshold — it is a property of the world, not the organism.

**Trait.** An explicit dimension in the trait vector. Heritable, mutable, heterogeneous across the population. Each trait is independently evolvable — selection can adjust one without mechanically forcing changes in another. Example: fecundity, asexual propensity.

**Derived.** Heterogeneous across agents but computed from existing traits rather than stored as a separate axis. Derived properties vary across the population — they are not universal like physics — but they do not need independent evolvability because the variation they produce follows deterministically from the trait dimensions that already exist. Example: mate selectivity (reproductive isolation emerges from trait-space divergence measured against the physics-defined compatibility distance — no separate "choosiness" axis is needed).

The criterion for promoting a derived property to a trait: **does independent evolvability of this property produce emergent dynamics that derived heterogeneity cannot?** If two agents with identical traits should be able to differ in a property and have that difference be heritable, it must be a trait. If the property is fully determined by the trait vector and world physics, it is derived.

## Dimensions

Seven dimensions in three layers. The layers reflect different aspects of an agent's life history: how it allocates energy, how it acquires energy and moves, and how it reproduces.

### Allocation layer

**Kappa (soma vs. reproduction).** Following DEB theory's kappa rule (Kooijman 2010), this trait governs the fraction of energy surplus directed to soma (maintenance + growth) versus reproduction. Within the somatic branch, maintenance has priority — if income does not cover maintenance, the organism degrades. Growth only happens with surplus after maintenance.

High kappa = long-lived, slow-reproducing. Low kappa = short-lived, fast-reproducing. This is the core survive-vs-reproduce trade-off — the disposable soma principle expressed as a single continuous parameter. Kappa does not determine the total energy budget; it determines how the budget is split once metabolic costs are paid.

### Specification layer

Three dimensions describe how an agent acquires energy and whether it moves.

**Autotrophy.** Investment in photosynthetic machinery. Determines an agent's capacity to capture energy from solar flux. Autotrophy implicitly requires nutrient uptake from the abiotic substrate — a photosynthesizer needs matter (nutrients) to build its body, and sunlight provides only energy. Nutrient uptake is a requirement of autotrophy, not an independent trait.

**Heterotrophy.** Investment in consumption machinery. Determines an agent's capacity to consume biomass from other agents, living or dead. Heterotrophs acquire both energy and nutrients from prey. There is no separate "scavenging" trait — consuming a living agent and consuming a carcass use the same machinery. Whether an agent is a predator, grazer, or decomposer depends on target availability, body size ratios, and other traits — not separate consumption capabilities.

Autotrophy and heterotrophy are two independent dimensions, not a single spectrum. An agent can invest in both (mixotrophy) — and real ecosystems carry persistent mixotrophs, so this is a strategy to confine to a niche, not forbid. Each trait carries a superlinear *per-trait* maintenance cost, but that cost is breadth-neutral: paying for two traits independently is no dearer than paying for one (see [world-rules](world-rules.md), cost structure). What keeps mixotrophy a niche strategy rather than a dominant one is functional incompatibility between the capabilities combined and structural fragility — not the per-trait maintenance bill. Two independent dimensions produce smoother parameter landscapes for genesis search than a single spectrum with a mixotroph valley in the middle.

**Mobility.** How much an agent moves. Movement costs energy proportional to use, and movement cost scales with structure — bigger organisms cost more to move. Sessile autotrophy emerges as the dominant producer strategy not from a budget constraint but from pure energy economics: sunlight is ambient, so movement costs energy for zero additional energy income. Mobility is valuable when your resource is unevenly distributed — prey moves, is patchy.

### Reproduction layer

Three dimensions describe how an agent reproduces.

**Fecundity.** How many offspring share the reproductive energy budget (determined by kappa). High fecundity = many poorly-provisioned offspring (r-strategy). Low fecundity = few well-provisioned offspring (K-strategy). The actual offspring count per event is stochastic (Poisson with mean = fecundity trait). This is the Smith & Fretwell (1974) offspring number-vs-quality trade-off and is independent of kappa — kappa determines the total budget, fecundity determines how it is divided.

**Asexual propensity.** Capacity to reproduce without a mate. This is evolvable machinery, not a universal fallback. A lineage that has specialised into sexual reproduction has lost asexual capability — like wolves, which cannot reproduce asexually even if no mates are available. This distinction matters: asexual reproduction is not a free backup mode that all agents can access.

Selection dynamics: in sparse populations, asexual capability is hugely valuable — it enables colonisation and persistence when mates are unavailable. In dense populations with available mates, sexual reproduction is favoured because recombination generates combinatorial diversity. When mates are available and sexual reproduction is the norm, asexual machinery becomes unused overhead — maintained at a cost for a capability that is never exercised. Whether a lineage is primarily sexual or asexual is emergent from these selection pressures, not prescribed.

The cost is literal: asexual propensity carries a small superlinear maintenance cost, paid every tick regardless of whether the asexual path is exercised. It is the *only* reproduction trait that does — and deliberately so. Fecundity and dispersal express their trade-offs on *every* reproduction event (offspring fragility, spatial competition), so they are never under-policed: selection always sees them. Asexual propensity is different because it can sit high but **dormant** — a lineage with plentiful mates reproduces sexually and almost never invokes the asexual path, so the trait's only mechanical disadvantage (asexual offspring inherit parent traits plus mutation, with no recombination) applies to vanishingly few events. As propensity falls, asexual reproduction becomes rarer still, and the no-recombination penalty flattens toward zero before propensity reaches it. Without a standing cost, an unused propensity would simply drift, and the "lost capability" outcome would never reliably emerge. The maintenance cost keeps a directional gradient alive even when the trait never fires, carrying propensity all the way to zero — where the capability is genuinely gone. This realises "machinery, not fallback" as a smooth economic gradient, with no threshold or gate: an agent at propensity 0.01 *can* still reproduce asexually ~1% of the time; it is the cost that drives specialised-sexual lineages down to true 0, not a cutoff.

**Dispersal.** Investment in offspring dispersal capability — the structures and mechanisms that move propagules away from the parent. This is a life history trait: a dandelion's parachute seeds, an oak's heavy acorns, a fern's wind-borne spores, a coconut palm's buoyant fruit are all expressions of different dispersal investment strategies. Higher dispersal investment produces a wider dispersal kernel (offspring land farther from the parent on average). Lower investment produces offspring that land near the parent.

Dispersal is independent of fecundity. An organism can produce many far-dispersing offspring (dandelion — high fecundity, high dispersal) or few nearby offspring (coconut — low fecundity, moderate dispersal). The dispersal-fecundity trade-off is not a mechanical coupling but a budgetary pressure paid **at the reproduction event**, not as a standing per-tick maintenance cost — consistent with asexual propensity being the *only* reproduction trait that costs every tick. Both draw on the reproductive energy budget: fecundity divides it across more offspring (each gets less), and dispersal spends part of it building the propagule structures that carry offspring (pollen, plumes, fruit), leaving less to provision each one. The cost rises superlinearly with investment, so broadcasting both widely and prolifically is disproportionately expensive — the anti-generalist economics, expressed when the agent reproduces rather than every tick.

Dispersal is also independent of mobility. Sessile organisms disperse offspring through propagule structures (spores, seeds); mobile organisms' offspring may disperse under their own locomotion. Both modes are expressions of the same trait — investment in getting offspring away from the parent. The trait governs how far offspring land, regardless of the physical mechanism.

Dispersal does double duty: it also sets **mate-finding reach for low-mobility agents**. Mate-finding has the same physical solution as offspring placement — move reproductive material across space — so the propagule investment that scatters a sessile agent's offspring also carries its gametes to a stationary partner (pollen, spores, broadcast spawning). An agent's *reproductive reach* is therefore derived from both mobility and dispersal: a mobile agent finds mates through its mobility-derived perception, a sessile agent through gamete broadcast, and the two add smoothly (see [world-rules](world-rules.md), reproductive reach). The consequence is that a mobility-0 agent is **not** reproductively isolated as long as it invests in dispersal — a wolf (high mobility, negligible dispersal) finds mates by moving, a producer (mobility 0, high dispersal) finds them by broadcast. Reproductive reach is the *spatial* axis of mating; the reproductive compatibility distance is the orthogonal *trait-space* axis — both must be satisfied for a sexual event.

## No L1 budget constraint

The old design used a budget constraint (traits summing to 1.0) to prevent generalist dominance. The redesign drops that hard simplex for a smooth landscape: each trait carries an independent superlinear *per-trait* maintenance cost. The motivation is **smoothness, not an anti-generalist effect** — and it is worth being precise about what per-trait convexity does and does not do, because an earlier version of this section claimed the opposite. This means:

- **Per-trait convexity is breadth-neutral; it caps extremity, not breadth.** An agent can invest in both autotrophy and heterotrophy; it pays superlinear maintenance on each, independently. Crucially, this does *not* reward concentration: a specialist at investment `2t` pays `c·(2t)^p = 2^p·c·t^p`, while a generalist split across two traits at `t` each pays `2·c·t^p` — and since `2^p > 2` for `p > 1`, the *generalist* pays less. Convexity penalises piling investment into a single trait, making each trait self-limiting at a finite best value; it gives no maintenance-cost advantage to specialists over generalists.
- **Specialisation is rewarded elsewhere — by incompatibility and fragility, not by the maintenance bill.** The selection pressure toward specialisation comes from functional incompatibility between certain capabilities (autotrophy's substrate-anchored stillness versus mobility's movement) and from structural fragility (a broad, high-entropy body dies after a smaller structural loss — [world-rules](world-rules.md) trade-off #9). Per-trait maintenance contributes the *smoothness*; those two contribute the *anti-generalist* pressure; and a cross-trait interaction maintenance term is held in reserve should they prove too weak.
- **The parameter landscape is smooth.** No simplex edges, no corners, no forbidden regions. Every point in trait space is reachable and has a well-defined fitness. Moving in any direction from any point produces a continuous change in maintenance cost.
- **Genesis search has gradient signal everywhere.** The superlinear cost surface provides gradient information at every point in the search space. There are no plateaus where the optimiser cannot distinguish between configurations, and no cliffs where a small parameter change produces a qualitative shift.

This directly implements the smooth parameter landscape constraint described in [world-rules.md](world-rules.md#design-constraints-on-mechanism-choice).

## Structure in light competition

Light competition should be weighted by structure (body size), not just photosynthetic absorption trait. In real canopy competition, bigger producers shade smaller ones regardless of photosynthetic efficiency. A large, moderately efficient producer captures more total light than a small, highly efficient one because it intercepts more of the incoming flux.

This creates a feedback loop: capture light, gain energy, grow, win more light — but at higher maintenance cost. Growth becomes a strategic investment for producers, not just a passive consequence of being well-fed. The energy cost of maintaining a large body (structure-dependent maintenance) counterbalances the competitive advantage of size.

The r/K spectrum for producers emerges from this feedback. Grow big and dominate the canopy — high light income, high maintenance, slow reproduction (high kappa). Stay small and reproduce fast before being shaded out — low light income per individual, low maintenance, fast reproduction (low kappa). Neither strategy dominates; their relative success depends on population density, disturbance frequency, and nutrient availability.

## What this document does not prescribe

This document describes the trait dimensions and their ecological motivation. It does not prescribe:

- **Data structures or algorithms.** How traits are stored, how maintenance costs are computed, how mutations are applied — those are implementation concerns that live with the code.
- **Parameter values.** The exponent of superlinear scaling, the Poisson mean for fecundity, the specific maintenance cost per trait unit — those are genesis search parameters.
- **Ecological principles.** The underlying biology that motivates these design choices is documented in [docs/ecology/](../ecology/). This document is our opinion about how to use that biology, not a restatement of it.
