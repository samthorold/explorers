# System Design

Design documentation for the simulation — our opinion about what stocks, flows, and feedback loops should be present in the game, and why. This layer is built on the ecological ground truths documented in [docs/ecology/](../ecology/) and is encoded by the implementation, but is distinct from both.

The ecology docs describe observable Earth. This layer describes what we want our world to look like — which ecological principles we carry forward, which we simplify, which feedback mechanisms we consider load-bearing, and what dynamic properties the simulation must exhibit. It is opinionated and detailed, and it is **self-justifying**: each mechanism is described together with the reason it has that shape, including why tempting alternatives fail. A reader should understand why the design looks the way it does today from these documents alone — no separate record of past decisions is needed.

The implementation (the code) encodes this design. It is close to the codebase — specific to data structures, algorithms, and parameter values — and traces its motivation back to a system design property described here without restating it.

## Documents

- [World Rules](world-rules.md) — the immutable physics of the simulation: stocks (energy and nutrients), flows, topologies, channels, conservation laws, and the cost structure that creates trade-offs
- [Trait Space](trait-space.md) — the seven heritable dimensions that define an agent: allocation (kappa), specification (autotrophy, heterotrophy, mobility), and reproduction (fecundity, asexual propensity, dispersal)
- [Execution Model](execution-model.md) — how the world rules are realised through time: monolithic tick loop, phase ordering, interaction coordination, event recording
- [Expected Properties](expected-properties.md) — the emergent properties we expect from those rules, the failure modes that indicate miscalibration, and the initial conditions that enable or prevent healthy ecology
- [Viability](viability.md) — the *a priori* lens: closed-form gates on the world parameters that decide which parameterizations cannot produce the expected properties at all, before any simulation runs

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
docs/system-design/    Our opinion. What the game world looks like, and why.
                       World rules (immutable physics).
                       Expected properties (what a working world exhibits).
                       Self-justifying: rationale lives here, in the present tense.
        │
        ▼
the code               Implementation. The encoding of the design.
                       Data structures, algorithms, parameter values.
```

There are three layers — the domain (observable Earth ecology), the system design (our opinion, encoded by any implementation), and the implementation itself (the code). There is no separate decision-record layer: the reasons a design has its shape are written into the system-design documents in the present tense, so understanding the present never requires reconstructing a history of choices.

Each layer references the one below it as foundation. No layer reaches up — ecology docs never reference the game, and system design docs never prescribe data structures. The implementation traces its motivation to the system design but doesn't restate it.
