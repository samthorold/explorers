# Simulation as a standalone crate, Bevy as rendering frontend

The simulation core is a standalone Rust library crate with no game engine dependency. It exposes `fn step(&mut World)` and has no opinion about timing, rendering, or input. Bevy is used solely as a rendering frontend: it reads simulation state each frame, draws it, and forwards player input.

## Considered options

- **Everything in Bevy's ECS.** Agents are Bevy entities, simulation logic runs as Bevy systems. Simpler project structure, but the simulation becomes inseparable from the engine. World genesis (running thousands of headless ticks to reach a quasi-stable ecology, discarding degenerate configurations) would drag in the full engine as a dependency. Testing ecology rules would require engine boilerplate.
- **Simulation as a standalone crate, Bevy as frontend (chosen).** The simulation crate is a pure Rust library. Genesis runs as a CLI tool — thousands of sims in parallel, no renderer. Bevy calls `step()` during play via `FixedUpdate`. The cost is wiring up the bridge between sim state and Bevy, but that bridge is straightforward since all visuals are deterministic functions of agent state.
- **Alternative renderers (macroquad, nannou, raw wgpu).** With the simulation separated, the renderer's job is narrow: draw shapes driven by state, handle input. Bevy is heavier than needed for this alone, but its ecosystem (audio, particles, UI overlays) provides headroom if scope grows. Lighter alternatives would mean rebuilding those capabilities later.

## Consequences

- The project becomes a Cargo workspace with (at minimum) two crates: the simulation library and the Bevy application.
- World genesis tooling can be built and iterated independently of the game.
- Bevy upgrades don't require simulation changes, and vice versa.
