# Instrument Runtimes

Measured wall-clock for the QD search and the research bins, to plan sweeps. Figures are from a laptop; treat them as orders of magnitude.

## Figures

**500-tick horizon (release build):** QD search ~20 min; `permanence_crosscheck`, `energy_bound_check`, `invasion_growth` ~20–30 min each; `eval_scenarios` ~3 min.

**Settled horizon T = 2000** (default since #507):
- Full sweeps over the atlas cells plus 200 LHS configs × 8 seeds: `settling_time` (3000 ticks) ~2.5 h; `energy_death_check` ~1.5 h; `energy_bound_check` ~2 h; `permanence_crosscheck` ~1 h 45 min – 6 h. Dense LHS configs (listed in `docs/research/505*` and `508*`) cost 15–60 min each, because the evaluator's role snapshot is linear in agents × retained edges and one step can outrun the per-run guard, which fires only between steps.
- `role_emergence`, 35 cells × 8 seeds: ~3 h.
- `radius_sweep` (4 baselines × 8 levels × 5 seeds): ~1 h 45 min.
- `guild_census`: atlas ~1 min; 200 LHS configs ~60 min when split over disjoint `--configs` in parallel (one dense config blocks a whole process).
- `role_diet_census`: atlas ~1 min; 200 LHS configs ~90 min.
- **QD search** (full box, 10 generations, default `--bloom-stop 300:10` since #579): ~7.5–12 min; 20 generations ~30 min. A few dense generations dominate an unstopped search. Neither the search nor refinement has a per-rollout time limit (#562), so one dense rollout can stall a refinement for hours. Pre-#573 checkpoints need `--no-bloom-stop` to resume.

Memory is not the constraint (RSS < 100–600 MB); wall time is.

## Running long sweeps

- **Every long bin is resumable.** Results are written incrementally as JSON lines under `target/` (shared `explorers_search::sweep`), with `--limit` to bound a call. Keep artifacts under `target/`, not the scratchpad, which the OS tmp cleaner empties.
- **Drive long sweeps in-harness as one background process or a resumable loop.** The harness moves any foreground call past ~10 min into the background, and a sub-agent waiting on it may not be woken by the notification. So the orchestrating session drives the sweep and hands the write-up back, rather than a sub-agent "working while waiting".
- **Size a sweep by the terminal roster of its worst cells, never by a pilot's mean.** Worlds that persist with consumers settle to a handful of agents and are nearly free; gated worlds (monoculture, lockup) carry hundreds and dominate the tail. A 4-run pilot once missed a sweep's cost by two orders of magnitude.
- **A sweep that stops producing rows may be asleep, not hung.** On macOS, a laptop idle-sleeping on battery pauses the process, and `Instant` does not advance, so run budgets don't fire either. Check `ps -o cputime,etime` and `pmset -g log` before suspecting a hang. `caffeinate -i -w <pid>` keeps it awake — ask the owner before using it.
