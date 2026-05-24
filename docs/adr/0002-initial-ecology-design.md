# Emergent trophic roles from a universal agent and closed energy loop

The initial ecology uses a single universal agent type whose role (producer, consumer, decomposer) emerges from its position in a continuous trait vector — there are no fixed types or labels. Energy enters only through uniform solar flux, flows through consumption and reproduction, and is locked in carcasses on death until a decomposer-like agent extracts it. There is no passive decay: decomposer-role agents are load-bearing for energy recycling.

## Considered options

- **Fixed agent types (producer, herbivore, decomposer) with hardcoded behaviour.** Simpler to implement and debug. Each type has its own update logic. But it contradicts the "everything is an agent" principle, prevents novel strategies from emerging, and means world genesis is tuning three separate populations rather than searching for parameterisations where trophic differentiation arises spontaneously.
- **Universal agent with emergent roles (chosen).** One agent type, one trait vector, one update loop. Role is a human interpretation of trait-space position. Omnivores, producer-decomposer hybrids, and novel niches can emerge. World genesis searches for trait configurations where recognisable trophic structure self-organises — a much stronger validation signal. The cost is that the system may fail to differentiate at all under poor parameterisation, but that failure is itself useful information for automated search.
- **Soft roles (trait-influenced but type-constrained).** A middle ground where agents have an inherited role tendency but traits shift behaviour partially across boundaries. Adds complexity without the full payoff of either approach.

## Consequences

- The trait vector must contain dimensions sufficient to express all ecological behaviours: photosynthetic absorption, consumption rate, scavenging rate, mobility, chemotaxis sensitivity, social weight, mate selectivity, sensing range, reproductive investment, chemical signature.
- Metabolic cost is derived from activity (movement, sensing, size) rather than being a fixed per-tick tax or an independent trait. Environmental mismatch cost is architecturally present but dormant until spatial gradients are added.
- Reproduction is sexual with trait-distance compatibility, enabling emergent speciation. Budding model — both parents survive, offspring receives energy determined by the parent's reproductive investment trait.
- Consumption drains energy over time rather than killing instantly — agents survive grazing unless energy reaches zero.
- No passive carcass decay means decomposer extinction leads to energy lock-up and systemic decline. This is an intentional fragility: degenerate configurations should fail fast during world genesis parameter search.
- Sensing uses distance-weighted detection (no diffusion fields) to keep per-tick computation cheap for genesis throughput.
- Herbivore-like agents move via a weighted combination of chemotaxis (toward food signal), social foraging (toward feeding neighbours), and random exploration. Weights are trait-determined and heterogeneous, so herding is emergent rather than prescribed.
