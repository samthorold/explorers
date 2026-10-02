# Instrument Runtimes

Measured wall-clock for the QD search and the research bins, to plan sweeps. Figures are from a laptop; treat them as orders of magnitude.

## Figures

**500-tick horizon (release build):** QD search ~20 min; `permanence_crosscheck`, `energy_bound_check`, `invasion_growth` ~20–30 min each; `eval_scenarios` ~3 min.

**Settled horizon T = 2000** (default since #507):
- Full sweeps over the atlas cells plus 200 LHS configs × 8 seeds: `settling_time` (3000 ticks) ~2.5 h; `energy_death_check` ~1.5 h; `energy_bound_check` ~2 h; `permanence_crosscheck` ~1 h 45 min – 6 h. Dense LHS configs (listed in `docs/research/505*` and `508*`) cost 15–60 min each, because the evaluator's role snapshot is linear in agents × retained edges and one step can outrun the per-run guard, which fires only between steps.
- `role_emergence`, 35 cells × 8 seeds: ~3 h.
- `radius_sweep` (4 baselines × 8 levels × 5 seeds): ~1 h 45 min.
- `guild_census`: atlas ~1 min; 200 LHS configs ~60 min when split over disjoint `--configs` in parallel (one dense config blocks a whole process).
- `role_diet_census`: atlas ~1 min; 200 LHS configs ~1.5–2 h, one config at a time. Ten dense configs (`sample:20`, `24`, `38`, `100`, `129`, `147`, `154`, `165`, `169`, `197`) cost 2–19 min each and dominate; which are heaviest shifts with the physics (#605). Under recognition (#606) `sample:20` took ~65 min and `24`, `154`, `129` and `147` took 18–27 min each. `55`, `61` and `146` went from seconds to 2–9 min, because spared kin keep those worlds alive and dense for longer. These times were measured with three sweeps sharing 8 cores, and one LHS sweep took ~2 h 50 min to ~3 h 30 min under that load. Split `--configs` over two processes so that `sample:20` does not hold the rest. The killing-grazer readout adds ~5 %.
- `reinvasion_barrier` on `sample:31`: 7–14 s per mode at the defaults. With `--satiation-sensitivity 3` it takes 11–25 s, and a 36-run (c, D) grid takes ~9 min (#619). Under the surplus gate (`c = 33`, #624): plain / accounting / dispersal 19–21 s / 12–15 s / 20–25 s, and `role_diet_census` on the atlas 37–42 s decoded, ~78 s at `founder_aggregation = 0`; runs that logged ~1,000 s were the laptop asleep.
- **QD search** (full box, 10 generations, default `--bloom-stop 300:10` since #579): ~7.5–12 min; 20 generations ~30 min. A few dense generations dominate an unstopped search. Neither the search nor refinement has a per-rollout time limit (#562), so one dense rollout can stall a refinement for hours. Pre-#573 checkpoints need `--no-bloom-stop` to resume.

Memory is not the constraint (RSS < 100–600 MB); wall time is.

## Running long sweeps

- **Every long bin is resumable.** Results are written incrementally as JSON lines under `target/` (shared `explorers_search::sweep`), with `--limit` to bound a call. Keep artifacts under `target/`, not the scratchpad, which the OS tmp cleaner empties.
- **Drive long sweeps in-harness as one background process or a resumable loop.** The harness moves any foreground call past ~10 min into the background, and a sub-agent waiting on it may not be woken by the notification. So the orchestrating session drives the sweep and hands the write-up back, rather than a sub-agent "working while waiting".
- **Size a sweep by the terminal roster of its worst cells, never by a pilot's mean.** Worlds that persist with consumers settle to a handful of agents and are nearly free; gated worlds (monoculture, lockup) carry hundreds and dominate the tail. A 4-run pilot once missed a sweep's cost by two orders of magnitude.
- **A sweep that stops producing rows may be asleep, not hung.** On macOS, a laptop idle-sleeping on battery pauses the process, and `Instant` does not advance, so run budgets don't fire either. Check `ps -o cputime,etime` and `pmset -g log` before suspecting a hang. `caffeinate -i -w <pid>` keeps it awake — ask the owner before using it.
