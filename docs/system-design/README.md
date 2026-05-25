# System Design

Design documentation for the simulation — our opinion about what stocks, flows, and feedback loops should be present in the game, and why. This layer is built on the ecological ground truths documented in [docs/ecology/](../ecology/) and informs the architectural decisions in [docs/adr/](../adr/), but is distinct from both.

The ecology docs describe observable Earth. This layer describes what we want our world to look like — which ecological principles we carry forward, which we simplify, which feedback mechanisms we consider load-bearing, and what dynamic properties the simulation must exhibit. It is opinionated and detailed.

The ADRs describe how the system design gets implemented in code. They are close to the codebase, specific to data structures and algorithms, and should trace their motivation back to a system design property described here.

## Documents

- [Dynamic Stability](dynamic-stability.md) — the simulation as a stability problem; feedback mechanisms, trade-off pressure, energy flow, and the regime we're designing for

## Relationship to other layers

```
docs/ecology/          Ground truth. Observable Earth ecology.
                       Cross-cutting topics are the foundation.
                       Taxa illustrate agents within those systems.
        │
        ▼
docs/system-design/    Our opinion. What the game should look like.
                       Stocks, flows, feedback loops we include.
                       Which simplifications we make and why.
        │
        ▼
docs/adr/              Implementation choices. Close to code.
                       Data structures, algorithms, protocols.
```

Each layer references the one below it as foundation. No layer reaches up — ecology docs never reference the game, system design docs never prescribe data structures, ADRs trace motivation to system design but don't restate it.
