# Explorers

An ecology-driven game where a foreign entity navigates an alien world of interconnected, adaptive agents. No explicit rules. The emotional arc moves from danger and confusion to wonder and co-existence. Exploitation leads to ruin; symbiosis leads to success.

## Glossary

- **Agent** — the fundamental unit of the simulation. Everything in the world is an agent: organisms, environmental features, substances. There is no inert backdrop.
- **Trait vector** — a multi-dimensional vector of continuous values that defines an agent's identity: metabolism, sensitivity, reactivity, mobility, chemical signature, etc. Determines what actions an agent can perform and how it responds to others.
- **Action** — something an agent does: emit, absorb, grow, contract, move, pulse. Available actions and their intensity are determined by the agent's trait vector.
- **Response** — how an agent reacts to another agent's action, determined by the responding agent's traits relative to the acting agent's traits.
- **Player** — a foreign entity dropped into the world. Has a mutable trait vector that shifts through interactions. Starts as an outsider; the world reacts to them based on trait compatibility.
- **Touch** — direct physical contact between the player and another agent. The strongest form of interaction. Modifies both parties' traits.
- **Presence** — the passive effect of the player being near agents. A weaker form of interaction. How strongly an agent reacts to the player's presence depends on the agent's sensitivity trait and the player's current trait signature.
- **Field of view** — the player perceives only their local surroundings. Once they move on, they lose sight of that area, but the simulation continues there. No persistent map.
- **Symbiosis** — interactions where trait compatibility produces mutual benefit. The ecology responds to a symbiotic player by offering more interactions, approaching rather than fleeing, stabilising locally.
- **Exploitation** — one-sided interactions that benefit the player at an agent's expense. The ecology responds defensively: organisms withdraw symbiotic responses, activate defensive behaviours, and the player becomes increasingly isolated.
- **Energy** — the universal currency of the simulation. Flows through interactions. Agents that deplete their energy die. Agents that accumulate enough energy can reproduce.
- **Death** — when an agent's energy reaches zero. Dead agents become resources available to other agents (recycling).
- **Reproduction** — when an agent's energy exceeds a threshold and conditions are met (e.g. compatible partner nearby). Offspring inherit parent traits with mutation, enabling evolution.
- **Tick** — the discrete time step of the simulation. Each tick, all agents evaluate their neighbourhood, select actions, and update state.
- **World genesis** — the process of generating a playable world. A random initial population is simulated forward (off-screen) until it reaches a quasi-stable attractor. Degenerate configurations are discarded automatically by validation rules that inspect the resulting ecology. The player drops into a world with history.

## Design decisions

### The world is a complex adaptive system

The world is an agent-based model in the tradition of computational ecology. All entities are heterogeneous, adaptive agents interacting locally in 2D continuous space. Population dynamics emerge from individual agent behaviour — there is no top-down control of species populations or ecosystem balance.

### Earth-analogous ecology

The simulation follows real ecological principles: energy conservation, trophic levels (producers, consumers, decomposers), nutrient cycling, carrying capacity. The forms may be alien but the dynamics are grounded. This allows us to draw on established ABM literature and ecological models. Alien mechanics can be layered on once the base ecology produces coherent dynamics.

### No explicit rules for the player

The player receives no tutorial, no HUD, no rule explanations. The world communicates through visible agent behaviour — movement patterns, colour, shape, pulsation, spatial relationships. The player learns by observing and interacting.

### No goal, no win state

The game is open-ended. There is no objective, quest, or ending. The experience is being in this world — watching its dynamics, participating in its ecology, deepening understanding. Engagement comes from the world being intrinsically interesting, not from extrinsic rewards.

### Observation is participation

The player can never passively observe. Their presence affects nearby agents based on trait-dependent sensitivity. Even standing still is an interaction. The player is always a disturbance — the question is whether they become a welcome one.

### The ecology is robust, the player is fragile

Exploitation does not collapse the ecosystem. The world was functioning before the player arrived and will continue after they die. Exploitative play causes the local ecology to withdraw — defensive responses activate, symbiotic offers cease, and the player is left isolated with no way to sustain themselves. The punishment for exploitation is exclusion, not apocalypse.

### Player verbs: move and touch (v1)

The initial verb set is minimal: move through the world and make physical contact with agents. Richer verbs (carry, transform) may emerge through symbiotic relationships in future iterations — the world grants capability as the player integrates.

### No environmental cycles (v1)

No imposed day/night or seasonal cycles initially. All temporal patterns emerge from agent interactions. Environmental oscillators can be added later if the simulation lacks rhythm.

### Visual behaviour language

Agent traits map to visual properties. Interactions produce visible effects. The player learns to read the ecology by watching how things look and behave. No text, no numbers, no labels.
