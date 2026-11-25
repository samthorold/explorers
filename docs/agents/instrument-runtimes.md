# Instrument Runtimes

Measured cost of the reference-mode runs and the other bins still in the workspace, to plan sweeps. Figures are from a laptop (8 cores, release builds); treat them as orders of magnitude. Each setting has its latest figure; superseded measurements are in git history and in the research notes.

**Archived bins.** The QD search and its research bins (`settling_time`, `energy_death_check`, `energy_bound_check`, `permanence_crosscheck`, `invasion_growth`, `role_emergence`, `radius_sweep`, `guild_census`, `role_diet_census`, `kin_killer_diet`, `reinvasion_barrier`, `fragility_audit` and the rest of `explorers-search`) left the workspace with the crate; their measured runtimes live in this file at the tag `atlas-search-archive`, alongside the code (`docs/agents/architecture.md`).

## Figures

**`reference_mode`** (`cargo run --release -p explorers-reference-modes --bin reference_mode`): the mode-1 mesocosm (9 × 9 nutrient cells, 243 founders, 1,377 standing carcasses at founding). Costs are per run: wall-clock for the runs of under a minute (#796–#804), CPU (`user` + `sys`) from #814 on, where wall-clock matched CPU within 5–20 % when the runs were held awake (below). "Paired" is `--clear-at 1500 --ticks 1500`. "Four rules" was `--size-scaled-base-rate --living-resistance --litter-half-life L --enzyme-half-saturation K --established-producer-structure 6.937`; since #862 the size-scaled base rate at `S_p` 6.937 is the world's own, so it is `--living-resistance --litter-half-life L --enzyme-half-saturation K`, and `--decomposer-mesocosm` sets them at the decomposer mesocosm's declared L 1 and K 1 (#884). `k` is `--body-reach-pieces`.

| Setting | Cost per run | Note |
|---|---|---|
| Defaults, paired, alone | 18 s | #797 (no research note) |
| `--litter-x 10`, paired, alone | 49 s | #797 |
| Pilots, alone: `--litter-x 10 --clear-at 100 --ticks 100`; `--clear-at 400 --ticks 400` | 2.7 s; 2.8 s | #797 |
| Litter readout, paired, body reach 0, five at a time | 20–22 s | `docs/research/804-mode1-litter-supply.md` |
| `k` = 0, 1, 3 / 10 / 30, paired, five at a time | 18–23 s / 11–18 s / 2–3 s | `docs/research/796-body-reach-sweep.md` |
| Four rules, L = 0.5–4, K = 1–10; `k` = 0 / 3 / 10 / 30 | 38–58 s / 44–84 s / 82–220 s / 220–780 s | `docs/research/814-mode1-combined-reading.md` |
| Four rules (L = K = 1) × starvation cascade; `k` = 0 / 3 / 10 / 30 | 59–76 s / 72–96 s / 120–214 s / 437–1,037 s | `docs/research/822-mode1-lean-time-reserve.md` |
| Four rules (L = K = 1), catchment `k` = 131.6 / 387.0 / 1,083.7 (`R` = 170 / 500 / 1,400) | 1,142–1,556 s / 1,869–2,616 s / 2,652–3,161 s | `docs/research/826-mode1-catchment.md` |
| #826's settings with `--decomposer-founders` 20 / 10 / 5 (`k` = 68.85 / 137.7 / 275.4), with the `recruitment` readout | 855–1,118 s / 1,463–1,756 s / 1,644–2,016 s | `docs/research/833-mode1-recruitment-diagnostic.md` |
| #829's command at `n` = 10 / 5 with `--marker-isolation --compatibility-scale 100` (since #861 marker isolation runs in every world at `s` = 100 and `--marker-isolation` is refused, so #829's command alone is this run) | 1,319–1,636 s / 1,413–1,799 s | `docs/research/837-mode1-crossing.md` |
| #837's command plus `--growth-dilutes-wear` (the flag is gone since #858, which puts dilution in every world), `n` = 10 / 5 | 1,404–1,620 s / 1,563–1,925 s | `docs/research/840-mode1-ageing.md` |

**Reading by the ladder** (#857; `end-state.md`, *The decomposer guild*, *Its ladder*). Three options make the expensive rungs fail fast. None has its own measured figure yet; the costs below are derived from the rows above.

| Option | What it costs | Note |
|---|---|---|
| `--early-window C` with `--ticks 100` (rung 2: `C` = 50) | One run of 100 ticks: about the pilots' 2.7 s per 200 ticks at the defaults, more under the four rules and body reach, whose early ticks run on the founding pile | Reads one run, not paired; refuses `--clear-at` |
| `--abort-on-line-extinction` | Nothing measurable: one roster scan per tick while the line lives. It saves the rest of a run whose line dies, which at the four rules is most of a 1,500–3,000 s run | Also stops a paired settle (both arms abort at once) or one arm |
| `--seeds N --passes-needed K` | Between K and N runs, one at a time: rung 3 (3 of 5) costs 3 to 5 runs, not 5. Add `--failures-undecided` where the rule's failure bound declares no refuted line (mode 1) | Writes `mode1-sequence.json`; a run of the sequence is byte for byte its seed's single run |

The sequence runs its seeds one after another in one process, so it is a long foreground job at rung 3: drive it as below, in the background, and let the abort shorten the failures.

What sets the cost: the carcass pile and the producer stand, not the living roster. Body reach can collapse the stand and make a run cheaper (#796) or draw the litter down and keep the founder line alive, making it far dearer (#814 at `k` = 30). CPU tracks reach, not the founder count (#829). Readouts cost nothing measurable. A `report.json` is 0.3–2.3 MB.

Memory is not the constraint (RSS < 100–600 MB); time is.

## Running long sweeps

- **Make a long sweep resumable.** Write results incrementally as JSON lines under `target/`, with a `--limit` to bound a call, so an interrupted sweep picks up where it stopped (the archived search bins shared `explorers_search::sweep` for this; restore it from the tag if a new sweep needs it). Keep artifacts under `target/`, not the scratchpad, which the OS tmp cleaner empties.
- **Drive long sweeps in-harness as one background process or a resumable loop.** The harness moves any foreground call past ~10 min into the background, and a sub-agent waiting on it may not be woken by the notification. So the orchestrating session drives the sweep and hands the write-up back, rather than a sub-agent "working while waiting". Five or six runs at a time on 8 cores (`xargs -P 5`), worst cells first, is the usual shape.
- **Size a sweep by its worst cells, never by a pilot's mean.** Pilot the largest reach, as a full paired run or at least several hundred ticks: a short pilot's early ticks run on a small carcass pile and understate the settled cost (#814's pilot missed `k` = 30 by more than ten times, #826's by 3–10 times).
- **A sweep that stops producing rows may be asleep, not hung.** On macOS, a sleeping laptop pauses the process, and `Instant` does not advance, so run budgets don't fire either. Check `ps -o cputime,etime` and `pmset -g log` before suspecting a hang.
- **Hold the laptop awake on mains power, and ask the owner first.** `caffeinate -i` only prevents idle sleep, not system or lid sleep (#837 stalled overnight under it). `caffeinate -s -w <xargs pid>` holds the system awake, but only on AC power; on battery it does nothing beyond `-i` (#840's battery runs took 2.4–2.7 times their CPU in wall-clock, against 1.01 on mains). Plug in, keep the lid open, and use `-s` for an unattended sweep. A paused sweep loses no work: `kill -STOP` the run processes and `kill -CONT` them later. Report CPU as the cost whenever wall-clock was interrupted.
