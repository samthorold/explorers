# System Design

Design documentation for the simulation — our opinion about what stocks, flows, and feedback loops should be present in the game, and why. This layer is built on the ecological ground truths documented in [docs/ecology/](../ecology/) and informs the architectural decisions in [docs/adr/](../adr/), but is distinct from both.

The ecology docs describe observable Earth. This layer describes what we want our world to look like — which ecological principles we carry forward, which we simplify, which feedback mechanisms we consider load-bearing, and what dynamic properties the simulation must exhibit. It is opinionated and detailed.

The ADRs describe how the system design gets implemented in code. They are close to the codebase, specific to data structures and algorithms, and should trace their motivation back to a system design property described here.

## Documents

- [World Rules](world-rules.md) — the immutable physics of the simulation: stocks (energy and nutrients), flows, topologies, channels, conservation laws, and the cost structure that creates trade-offs
- [Trait Space](trait-space.md) — the six heritable dimensions that define an agent: allocation (kappa), specification (autotrophy, heterotrophy, mobility), and reproduction (fecundity, asexual propensity)
- [Execution Model](execution-model.md) — how the world rules are realised through time: monolithic tick loop, phase ordering, interaction coordination, event recording
- [Expected Properties](expected-properties.md) — the emergent properties we expect from those rules, the failure modes that indicate miscalibration, and the initial conditions that enable or prevent healthy ecology

## How the ecology layer maps here

The cross-cutting topics in the ecology layer each illuminate a different aspect of the world rules:

- **Energy flow** and **nutrient cycling** → what the world is made of. Stocks, flows, conservation laws, multi-currency dynamics, and stoichiometric constraints.
- **Spatial ecology** → the two topologies (physical surface and emergent network) and the two channel types (perception and physical).
- **Life history theory** → the cost structure and trade-offs that agents face within the world rules.
- **Disturbance and succession** → emergent patterns that arise from the rules playing out across different timescales and spatial scales. Not separate mechanisms — consequences of the same physics.

The taxa documents (plants, animals, fungi, bacteria) illustrate how specific lineages exploit the constraints the cross-cutting topics describe. They are not directly referenced here but inform intuition about what kinds of agent strategies the rules should support.

## Relationship to other layers

```
docs/ecology/          Ground truth. Observable Earth ecology.
                       Cross-cutting topics are the foundation.
                       Taxa illustrate agents within those systems.
        │
        ▼
docs/system-design/    Our opinion. What the game world looks like.
                       World rules (immutable physics).
                       Expected properties (what a working world exhibits).
        │
        ▼
docs/adr/              Implementation choices. Close to code.
                       Data structures, algorithms, protocols.
```

Each layer references the one below it as foundation. No layer reaches up — ecology docs never reference the game, system design docs never prescribe data structures, ADRs trace motivation to system design but don't restate it.
