# Project Vision

What the game is for, so design and engineering choices can be judged against it. The canonical design is in `CONTEXT.md` and `docs/system-design/`.

## The game

The player navigates a complex, initially incomprehensible world of interconnected, adaptive agents. The emotional arc runs from danger and confusion, through wonder, to co-existence: exploitation and mastery lead to ruin, symbiosis leads to success. The aesthetic and emotional reference is *Scavengers Reign*.

Design pillars:
- **No explicit rules of engagement.** The player learns by interacting and watching. No tutorials, HUDs or rule explanations — agents' traits and interactions are read visually (`CONTEXT.md`, *Visual behaviour language*).
- **An open-ended sandbox.** No win state, no objectives.
- **Emergence over script.** The world is an agent-based model of heterogeneous, adaptive agents with Earth-analogous ecology; favour emergent behaviour over scripted content.
- **Technology-agnostic design.** The design is the identity; the implementation is secondary to it.

Judge a design decision by whether it serves the confusion → wonder → symbiosis arc.

### The trespass cycle

The arc isn't one passage through the game; it recurs. The player panics, masters part of the world, overreaches, watches the system collapse around what they missed, and finds symbiosis: **panic → mastery → overreach → collapse → symbiosis**. The game teaches by seduction and consequence: it lets the player feel clever and lets optimisation feel productive, then shows what it cost. The cycle repeats at a smaller scale with each new phenomenon, and exploration is what happens between collapse and symbiosis.

The world is the protagonist, not the player. It runs on a deep internal logic that doesn't care about them; the player is an irritant, a catalyst, at best a symbiont. Surviving, understanding and tending happen together, not as phases: the player survives by understanding and understands by tending, and tending keeps them alive. A mechanic should serve at least two of the three; avoid pure survival (combat, health) and pure puzzle (fixed solutions).

Secondary references: *Nausicaä* (the toxic jungle as an immune response to human damage), *Rain World* (vulnerability in the middle of the food chain), *Annihilation* (the boundary between self and environment dissolving).

## How the owner works

The project owner designs through emotional arcs and complex adaptive systems, thinks in the terms of individual-based ecological modelling, and expects the design to evolve through building. Design concerns are kept separate from technology choices, and the design is settled iteratively (see [delivery workflow](delivery-workflow.md)).
