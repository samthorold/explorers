# Ecology Reference

Reference documentation on Earth's ecology, written from a systems perspective — stocks, flows, feedback loops, and residence times. This is descriptive: how Earth works, observed and documented. It is not prescriptive and contains no design decisions about the game or simulation. For the simulation's own language, see [CONTEXT.md](../../CONTEXT.md). For design decisions, see [docs/system-design/](../system-design/). For implementation choices, see [docs/adr/](../adr/).

## Reading order

The cross-cutting topics describe system-level properties that emerge regardless of which organisms are present. They are the closest thing to ground truth — observable dynamics of any sufficiently complex ecology. The taxa documents describe organisms that operate within those systems, illustrating how specific lineages exploit the constraints and flows the cross-cutting topics describe.

## Cross-cutting topics

- [Nutrient Cycling](nutrient-cycling.md) — carbon, nitrogen, phosphorus as a complete system; stoichiometric constraints; multi-currency dynamics
- [Life History Theory](life-history-theory.md) — energy allocation trade-offs, r/K selection, reproductive strategies, senescence, bet-hedging, dispersal-fecundity trade-offs
- [Spatial Ecology](spatial-ecology.md) — patch dynamics, dispersal, movement ecology, spatial pattern formation, spatial competition, spatial feedbacks
- [Disturbance and Succession](disturbance-and-succession.md) — disturbance regimes, successional models, alternative stable states, resilience, gap dynamics, recovery trajectories

## Taxa

- [Plants](plants.md) — primary producers, energy gateway, sessile constraint, light competition, succession
- [Animals](animals.md) — consumers, predator-prey dynamics, trophic cascades, mobility, invertebrate infrastructure
- [Fungi](fungi.md) — decomposition, mycorrhizal networks, parasitic regulation, nutrient loop closure
- [Bacteria and Archaea](bacteria-and-archaea.md) — nutrient transformation, chemoautotrophy, nitrogen cycle, microbial loop

## On ABM and computational literature

Several documents include sections reviewing agent-based modeling (ABM) and computational ecology literature. These sections describe findings from published research — what models revealed about ecological dynamics, which simplifications preserved or destroyed emergent properties, where individual-based approaches produced different results from equation-based ones. They are included because they deepen understanding of the ecology itself.

They are not suggestions for how to architect the game. The system design layer ([docs/system-design/](../system-design/)) is where ecological understanding gets translated into design choices. These sections stay in the ecology layer because they illuminate the ecology, not because they prescribe implementation.

## Gaps and next steps

### Missing taxa

**Protists and algae.** The most significant gap. Protists include the only true mixotrophs — organisms that photosynthesize and consume prey simultaneously. They are the real-world proof that mixed functional strategies work under the right constraints. Algae (both micro and macro) dominate aquatic primary production and have different spatial dynamics than terrestrial plants — no structural investment in height, rapid turnover, suspension in a fluid medium. A protists-and-algae doc should cover: mixotrophy as a real strategy with real trade-offs, the phytoplankton-zooplankton interface, algal bloom dynamics as a stock-and-flow phenomenon, and computational literature on plankton community models.

**Lichens and symbiotic composites.** Lichens are fungi + algae/cyanobacteria operating as a single functional unit. They represent stable mutualistic coupling between a producer and a decomposer — a composite that occupies functional space neither partner could alone.
