# Issue #822: the pre-registered lean-time reserve reading for mode 1

**Status: done (2026-10-12). The prediction holds in full. No point passes: 0 of 16.** Shrinking alone moves the median founder's death by +3 ticks at every k, within the 5 allowed. The earmark draw delays it by +116 to +124 ticks at k = 0, 3 and 10 and by +36 at k = 30, and both switches together by +119 to +127 and +40, all past the 20 required. Founders *persist* (one alive at tick 50 in 3 of 5 seeds) in all 8 arm × k points that draw the earmark, and in none of the 8 that do not. The guild holds in none of the 80 runs. Producers persist in all 80. By the rule, one follow-up fires, at all 8 earmark points: **the specialist's advantage on litter is grilled next** (world rules, flow 7). Beyond the rule, the earmark buys time more than a lineage: at k ≤ 10 every founder is still dead by tick 144 and the founder line dies out, likely held back by the nutrient earmark; at k = 30 the line survives to the end in all 10 earmark runs, without a guild. No default changed, and no code changed.

## The question

#814's founders die at tick 14–16 at every point, though body reach raises their income more than tenfold. By about tick 5 a founder has mobilised most of its starting reserve, two thirds into the reproductive earmark and the rest into structure, and then starves on its retention buffer while still holding both. #820 designed the *starvation cascade* (world rules, flow 8): the earmark pays upkeep (A), then structure pays it, *shrinking* down to the fragility threshold (B). #821 implemented both, each behind its own switch and off by default. This reading crosses the two switches, a 2×2, on top of #814's four rules, so the only change from #814 is the cascade. The pre-registration is mode 1's *Lean-time reserve* bullet in `docs/system-design/reference-modes.md`, agreed by the owner on 2026-10-12.

## The runs

Release build of `reference_mode` at main after #823 (commit 367666b), frozen as a copy for the sweep. Every run is paired, at 1× litter:

```sh
reference_mode --seed $s --clear-at 1500 --ticks 1500 \
  --size-scaled-base-rate --established-producer-structure 6.937 --living-resistance \
  --litter-half-life 1 --enzyme-half-saturation 1 --body-reach-pieces $k $SWITCHES \
  --out runs/k${k}_${arm}_s${s}/report.json
```

with `$SWITCHES` per arm:

| arm | switches |
|---|---|
| neither | `--starvation-readout` (the readout only, both switches off) |
| earmark | `--starvation-draws-earmark` |
| shrinking | `--starvation-shrinks` |
| both | `--starvation-draws-earmark --starvation-shrinks` |

k = 0, 3, 10 and 30, seeds 1–5: 4 arms × 4 points × 5 seeds = 80 paired runs. Each run seeds 81 decomposer founders (agents at the pure heterotroph vertex), so each arm × k point has 405.

**The founder readout** is `founder_starvation.control` in each `report.json`. Founders die long before the clearing at tick 1,500, so the perturbed arm shares it: the control and perturbed readouts are identical in all 80 runs (checked), and no founder is alive at the end of any run. A founder's reserve *runs out* in the first step it begins with unallocated reserve no greater than that step's upkeep. Its death tick, whether it reproduced before that step, the energy earmark it held then, and the steps it lived with the earmark or burned structure paying some upkeep are recorded per founder. Medians are nearest-rank, as the readout's.

## The prediction and its result

Median death tick among founders that had not reproduced before their reserve first ran out, pooled over seeds 1–5, and its change against the neither arm at the same k:

| k | neither | shrinking (Δ, at most 5?) | earmark (Δ, at least 20?) | both (Δ, at least 20?) |
|---:|---:|---|---|---|
| 0 | 15 | 18 (+3, yes) | 139 (+124, yes) | 142 (+127, yes) |
| 3 | 15 | 18 (+3, yes) | 137 (+122, yes) | 140 (+125, yes) |
| 10 | 14 | 17 (+3, yes) | 130 (+116, yes) | 133 (+119, yes) |
| 30 | 14 | 17 (+3, yes) | 50 (+36, yes) | 54 (+40, yes) |

**Both parts hold at every k.** Seed by seed, against the neither arm of the same seed and k:
- shrinking: +2 to +3 at every point (+2 only at k = 10, seeds 2 and 5);
- earmark: +124 at k = 0, +121 to +122 at k = 3, +115 to +116 at k = 10, and +34 to +35 at k = 30 except seed 4 at +109;
- both: +127, +124 to +125, +118 to +119, and +38 to +40 at k = 30 except seed 4 at +112.

The design's arithmetic for shrinking (about 0.8 of structure, about four ticks of upkeep) fits the 3 ticks seen.

**Founders that reproduced first, outside the prediction:** 0 of 405 at k = 0, 3 and 10 in every arm, and 2 of 405 at k = 30 in every arm (one each in seeds 3 and 5). In every run the median death over all founders equals the median over those that had not reproduced first. The readout records reproduction only before the reserve first ran out; a founder that reproduced later, on its earmark, counts as not having reproduced first.

### Why the founders do not reproduce first

The energy earmark when the reserve ran out is 21.9–22.8 (seed medians), above mode 1's energy threshold of 14.56 (`reproduction_energy_threshold` 14.562065 in `mode1.json`). The stepper's gate also needs the nutrient earmark at `reproduction_nutrient_threshold`, 1.0 (`phase.rs`, the eligible set: `repro_reserve >= reproduction_energy_threshold && repro_nutrient >= reproduction_nutrient_threshold`). From the code, the two earmarks fill differently:
- The energy earmark fills in `grow`, from `(1 - kappa)` of the reserve's mobilised surplus. A founder fills it from its starting reserve without eating anything, which is the 22 seen.
- The nutrient earmark starts at 0.0 for every founder (`lib.rs`, the agent constructors) and fills only in `credit_nutrient`, from `(1 - kappa)` of the nutrient acquired. A pure heterotroph acquires nutrient only by draining.

The founder readout's per-step drain (`report.control.founders.drained`, all founders together) puts the structure a founder drained before its reserve ran out (the first 14 steps) at about 0.09 at k = 0, 0.19 at k = 3, 0.59 at k = 10 and 1.27 at k = 30 (seed 1). Litter carries about 1.07 nutrient per unit of structure (the standing carcasses' 58 nutrient for 54 energy), and about two thirds of what a founder acquires goes to the earmark. That gives a nutrient earmark of roughly 0.06, 0.13, 0.4 and 0.9 by then: below 1.0 at every k, and nearest it at k = 30, where the only founders that reproduced first are.

So the nutrient gate is very likely the block, as #821 guessed. How sure: the code is unambiguous that the energy gate is met and that the nutrient earmark fills only from what a founder drains. The earmark sizes above are an estimate from pooled drain with an assumed nutrient content and kappa, not a measurement: no readout records a founder's nutrient earmark. Not investigated further.

## The verdict, by the fixed rule

| arm | k | founders persist (seeds with one alive at 50) | guild holds | producers persist | point passes | pooled median death | reproduced first | median ticks on earmark / structure |
|---|---:|---|---|---|---|---:|---:|---|
| neither | 0 | no (0/5) | no (0/5) | yes (5/5) | no (0/5) | 15 | 0/405 | 0 / 0 |
| neither | 3 | no (0/5) | no (0/5) | yes (5/5) | no (0/5) | 15 | 0/405 | 0 / 0 |
| neither | 10 | no (0/5) | no (0/5) | yes (5/5) | no (0/5) | 14 | 0/405 | 0 / 0 |
| neither | 30 | no (0/5) | no (0/5) | yes (5/5) | no (0/5) | 14 | 2/405 | 0 / 0 |
| earmark | 0 | **yes (5/5)** | no (0/5) | yes (5/5) | no (0/5) | 139 | 0/405 | 124 / 0 |
| earmark | 3 | **yes (5/5)** | no (0/5) | yes (5/5) | no (0/5) | 137 | 0/405 | 122 / 0 |
| earmark | 10 | **yes (5/5)** | no (0/5) | yes (5/5) | no (0/5) | 130 | 0/405 | 115 / 0 |
| earmark | 30 | **yes (5/5)** | no (0/5) | yes (5/5) | no (0/5) | 50 | 2/405 | 35 / 0 |
| shrinking | 0 | no (0/5) | no (0/5) | yes (5/5) | no (0/5) | 18 | 0/405 | 0 / 3 |
| shrinking | 3 | no (0/5) | no (0/5) | yes (5/5) | no (0/5) | 18 | 0/405 | 0 / 3 |
| shrinking | 10 | no (0/5) | no (0/5) | yes (5/5) | no (0/5) | 17 | 0/405 | 0 / 3 |
| shrinking | 30 | no (0/5) | no (0/5) | yes (5/5) | no (0/5) | 17 | 2/405 | 0 / 3 |
| both | 0 | **yes (5/5)** | no (0/5) | yes (5/5) | no (0/5) | 142 | 0/405 | 125 / 3 |
| both | 3 | **yes (5/5)** | no (0/5) | yes (5/5) | no (0/5) | 140 | 0/405 | 123 / 3 |
| both | 10 | **yes (5/5)** | no (0/5) | yes (5/5) | no (0/5) | 133 | 0/405 | 116 / 3 |
| both | 30 | **yes (5/5)** | no (0/5) | yes (5/5) | no (0/5) | 54 | 2/405 | 36 / 3 |

The ticks on earmark and structure are the median over seeds of each seed's median founder.

**0 of 16 points pass.** No seed of any point passes, so no point reaches 3 of 5.

- **Founders.** They persist at every point that draws the earmark, in 5 of 5 seeds, and at none that does not, where none is alive at tick 20 in any run. With the earmark at k ≤ 10, 80–81 of 81 founders are alive at ticks 20 and 50 and 80–81 at tick 100. At k = 30, 76–81 are alive at tick 20, 38–47 at tick 50 and 34–41 at tick 100.
- **The guild.** It holds in none of the 80 runs. Its fewest members are 0 in 77 runs, 1 in two (neither, k = 30, seed 5; both, k = 30, seed 5) and 2 in one (both, k = 30, seed 1), against the 5 it needs. Its share of the carcass drain is at most 0.025 in 76 runs; the other four are k = 30 runs, at 0.102 (neither, seed 5), 0.144 (shrinking, seed 2), 0.190 (both, seed 1) and 0.227 (both, seed 5).
- **Competition.** The mixotroph share (drain by drainers of autotrophy at least 0.1, over the arm's carcass drain) is 1.000 in every run, as in #814: drainers below autotrophy 0.1 drained nothing in the arm's second half. The guild's share above is through members with autotrophy of at least 0.1.
- **The check.** The guild's living share is 0.000 wherever it drained at all, and undefined ("-") where it drained nothing.
- **Producers.** They persist in all 80 runs: 688–956 at the control's end in the neither and shrinking arms, 938–1,231 in the earmark and both arms.

## What follows, by the rule

1. **Founders persist and many points pass in an earmark arm: does not fire.** Founders persist at every earmark point, but no point passes, in either earmark arm. Lean-time reserve is not shown to be the binding limit, and neither switch is recommended for adoption on this reading.
2. **Founders persist but the guild fails: fires** at all 8 earmark points (earmark and both, k = 0, 3, 10 and 30). Survival was necessary but not sufficient. **The specialist's advantage on litter is grilled next** (world rules, flow 7, with the matched-enzyme kernel held in reserve). #814's second follow-up already named it, since mixotrophs still take all of the carcass drain, and that is unchanged here.
3. **Founders do not persist, and most reproduced before their reserve first ran out: does not fire.** Founders do not persist only in the neither and shrinking arms, and there 0 of 405 (2 of 405 at k = 30) reproduced first.
4. **Founders do not persist, though they drew their earmark down: does not fire.** Founders persist at every point of every arm that draws the earmark.
5. **Producers collapse in any arm: does not fire.** They persist in all 80 runs, under both switches.

These are recorded, not acted on. No grill has run, no issue has been filed, and no default changed.

## Observations beyond the rule, not verdicts

- **At k ≤ 10 the earmark buys time, not a lineage.** "Persist" is read at tick 50. Every founder is dead by tick 141 (earmark) or 144 (both) at k ≤ 10, and the founder line (the founders and their descendants) is extinct at the end of every run there, in every arm. Follow-up 3's mechanism, reproduction timing, is still visibly implicated, though the rule does not fire it: the founders hold an energy earmark above the threshold for more than a hundred ticks without leaving a line, and the nutrient gate (above) is the likely reason. Over its whole life a founder on the earmark drains about 1.0 structure at k = 0, 1.8 at k = 3 and 5.2 at k = 10 (seed 1). By the estimate above it needs about 1.4 to fill a nutrient earmark of 1.0. The average founder reaches that by about tick 105 at k = 3, when upkeep (about 0.19 a tick: the earmark of about 22 lasts 115–125 ticks) has drawn its energy earmark to about 5, well below 14.56; at k = 0 it never does. At k = 10 it reaches it by about tick 34, with an energy earmark of about 19 still above the threshold. So by this estimate k = 10 founders could reproduce on the earmark, and either some did and their line died out, or the estimate is off. It is a rough race, not a measurement: whether any founder reproduced after running out is not recorded, and an extinct line does not say none did.
- **At k = 30 the earmark does leave a line.** With the earmark (either arm) the founder line is alive at the end in all 10 runs, with 8–111 agents. With neither switch or shrinking alone it survives only in seeds 3 and 5 (39 and 45, 40 and 6), the two seeds where a founder reproduced first. In the other three seeds of each earmark arm no founder reproduced first, so founders there reproduced on the earmark, after their reserve ran out. No guild holds in any of these runs, so the line survives as something other than a heterotroph-dominant guild, as #814 found at k = 30.
- **At k = 30 the earmark carries founders only about 35 ticks further.** In four of five seeds the median founder dies at tick 48–49 with the earmark (52–54 with both), against 14 with neither; seed 4 alone is at 123 (126). About half the founders are dead by tick 50 (38–47 of 81 alive), so the point persists by the rule's tick-50 margin. The deaths are bimodal: in seed 5 with both switches, 46 founders die by tick 54 and the other 35 at ticks 116–137. The median time on the earmark is 34–37 ticks at k = 30 (seed 4: 109–110), against 115–125 at k ≤ 10. That fits founders spending their energy earmark on offspring and then starving, since reproduction draws the earmark that would otherwise pay upkeep, but the readout does not record it.
- **The switches apply to producers too.** `S_p` (the control's median established producer structure, second half) is 6.53–8.03 in the neither and shrinking arms and 4.38–5.33 in the earmark and both arms. Producers at the control's end rise from 688–956 to 938–1,231. The earmark draw, not shrinking, makes the difference. Every run here used `b` at #814's `S_p` of 6.937; the earmark arms' own `S_p` would give a larger `b`, which this reading did not iterate.
- **Mixotrophs take essentially all the carcass drain in every run,** as in #814. The cascade changes how long a founder lives, not who drains the litter.
- **At k = 10 with neither switch, some founders die before their reserve runs out:** 65–75 of 81 ran out, against 80–81 of 81 everywhere else. Why is not read here.

## Runtime

80 paired runs, release build, six at a time on 8 cores through `xargs -P 6`, under `caffeinate -i` (the owner approved it), the k = 30 jobs first. No idle sleep: wall-clock was 4–11 % above CPU in every run. The sweep took 3,454 s (57.6 min) of wall-clock. Per run, wall-clock (CPU):
- k = 0: 63–81 s (59–76 s).
- k = 3: 76–103 s (72–96 s).
- k = 10: 129–230 s (120–214 s).
- k = 30: 468–1,109 s (437–1,037 s).

In all it took 19,285 s of CPU (5.4 CPU-hours): 1,327, 1,716, 3,266 and 12,975 s by k. The costliest arm was k = 30 with the earmark draw alone (545–928 s of CPU a run, 3,716 s for its five), and the costliest run k = 30, both switches, seed 5 (1,037 s). Shrinking costs nothing measurable: its arm's totals are within the seed-to-seed spread of the neither arm's (309 against 319 s at k = 0, 3,117 against 2,793 at k = 30). Each `report.json` is about 2 MB at k = 30.

## Reproducing

```sh
cargo build --release -p explorers-reference-modes --bin reference_mode
mkdir -p target/822-sweep/scripts && cp target/release/reference_mode target/822-sweep/
# scripts/run.sh and scripts/analyze.py as in the appendix below
for k in 30 10 3 0; do for a in n e s b; do for s in 1 2 3 4 5; do echo "$k $a $s"; done; done; done > target/822-sweep/jobs.txt
caffeinate -i xargs -P 6 -L 1 sh target/822-sweep/scripts/run.sh < target/822-sweep/jobs.txt
python3 target/822-sweep/scripts/analyze.py        # the verdicts, the prediction and the follow-ups
python3 target/822-sweep/scripts/analyze.py seeds  # the per-seed table below
```

The scripts hard-code `/Users/sam/Projects/explorers/target/822-sweep`; change `cd` in `run.sh` and `ROOT` in `analyze.py` for another checkout.

### `scripts/run.sh`

```sh
#!/bin/sh
# run.sh K_REACH ARM SEED: one paired run, #814's four rules on, plus the starvation-cascade arm
# (n = neither, e = earmark draw, s = shrinking, b = both); skipped if its report exists.
set -e
cd /Users/sam/Projects/explorers/target/822-sweep
k=$1; arm=$2; s=$3
case $arm in
  n) sw="--starvation-readout" ;;
  e) sw="--starvation-draws-earmark" ;;
  s) sw="--starvation-shrinks" ;;
  b) sw="--starvation-draws-earmark --starvation-shrinks" ;;
esac
out=runs/k${k}_${arm}_s${s}
[ -f "$out/report.md" ] && exit 0
mkdir -p "$out"
/usr/bin/time -p ./reference_mode --seed "$s" --clear-at 1500 --ticks 1500 \
  --size-scaled-base-rate --established-producer-structure 6.937 --living-resistance \
  --litter-half-life 1 --enzyme-half-saturation 1 --body-reach-pieces "$k" $sw \
  --out "$out/report.json" > "$out/stdout.txt" 2> "$out/time.txt"
```

### `scripts/analyze.py`

```python
# analyze.py: per-run rows (rows.json), per-arm/point verdicts, the prediction and the follow-ups,
# from target/822-sweep/runs. The founder readout is the control arm's (founders die long before
# the clearing at 1500, so both arms share it).
import json, os, re, sys
ROOT = '/Users/sam/Projects/explorers/target/822-sweep'
KS = [0, 3, 10, 30]; ARMS = ['n', 'e', 's', 'b']; SEEDS = range(1, 6)
ARM_NAME = {'n': 'neither', 'e': 'earmark', 's': 'shrinking', 'b': 'both'}
CENSOR = 10**9  # a founder alive at the end

def median(v):
    v = sorted(v)
    return v[(len(v) + 1) // 2 - 1] if v else None  # nearest rank, as the readout

def ctl_rows(md):
    rows, sec = {}, None
    for line in md.splitlines():
        if line.startswith('## '): sec = line[3:].split(' (')[0].split(',')[0]
        if line.startswith('| control |'):
            rows.setdefault(sec, [c.strip() for c in line.strip('|').split('|')])
    return rows

def row(k, arm, s):
    d = f'{ROOT}/runs/k{k}_{arm}_s{s}'
    if not os.path.exists(f'{d}/report.md'): return None
    a = json.load(open(f'{d}/report.json')); md = open(f'{d}/report.md').read()
    c = a['report']['control']; g = c['guild']; r = ctl_rows(md)
    dr = g['drain']
    share = dr['carcass'] / dr['all_carcass'] if dr['all_carcass'] > 0 else None
    tot = dr['living'] + dr['carcass']; ls = dr['living'] / tot if tot > 0 else None
    acd = c['arm_carcass_drain']; den = acd['mixotroph'] + acd['pure_vertex']
    mx = acd['mixotroph'] / den if den > 0 else None
    f = r['Founders']; lit = c['litter']['second_half']
    fs = a['founder_starvation']['control']['founders']
    t = open(f'{d}/time.txt').read()
    real = float(re.search(r'real ([\d.]+)', t).group(1)); user = float(re.search(r'user ([\d.]+)', t).group(1))
    deaths = [x['death_tick'] if x['death_tick'] is not None else CENSOR for x in fs]
    ran = [x for x in fs if x['ran_out_tick'] is not None]
    return dict(k=k, arm=arm, seed=s, holds=g['holds'], fewest=g['fewest_members'], share=share,
        living_share=ls, mixo_share=mx, persist=g['producers_persist'], passes=g['passes'],
        f20=int(f[2]), f50=int(f[3]), f100=int(f[4]),
        sp_out=lit['established_structure']['median'],
        producers_end=c['final_roles']['producers'],
        n_founders=len(fs), ran_out=len(ran),
        repro_first=sum(x['reproduced_before_running_out'] for x in fs),
        death_med=median(deaths),
        death_med_nrf=median([dt for dt, x in zip(deaths, fs) if not x['reproduced_before_running_out']]),
        deaths_nrf=[dt for dt, x in zip(deaths, fs) if not x['reproduced_before_running_out']],
        earmark_med=median([x['earmark_at_running_out'] for x in ran]),
        on_earmark_med=median([x['ticks_on_earmark'] for x in fs]),
        on_structure_med=median([x['ticks_on_structure'] for x in fs]),
        real=real, user=user)

rows = [x for k in KS for a in ARMS for s in SEEDS if (x := row(k, a, s))]
json.dump(rows, open(f'{ROOT}/rows.json', 'w'), indent=0)
print(len(rows), 'of 80 runs\n')
f3 = lambda v: '-' if v is None else f'{v:.3f}'
dt = lambda v: '-' if v is None else ('alive' if v >= CENSOR else str(v))
R = {(x['k'], x['arm'], x['seed']): x for x in rows}

if 'seeds' in sys.argv:
    print('| k | arm | seed | S_p | founders alive 20 / 50 / 100 | median death (not repro-first) | ran out | reproduced first | median earmark then | median ticks on earmark / structure | guild holds (fewest) | guild share | mixotroph share | living share | producers persist (at end) | pass | runtime s (CPU) |')
    print('|---:|---|---:|---:|---|---:|---:|---:|---:|---|---|---:|---:|---:|---|---|---:|')
    for x in rows:
        print(f"| {x['k']} | {ARM_NAME[x['arm']]} | {x['seed']} | {x['sp_out']:.3f} | {x['f20']} / {x['f50']} / {x['f100']} | {dt(x['death_med'])} ({dt(x['death_med_nrf'])}) | {x['ran_out']}/{x['n_founders']} | {x['repro_first']} | {f3(x['earmark_med'])} | {x['on_earmark_med']} / {x['on_structure_med']} | {'yes' if x['holds'] else 'no'} ({x['fewest']}) | {f3(x['share'])} | {f3(x['mixo_share'])} | {f3(x['living_share'])} | {'yes' if x['persist'] else 'no'} ({x['producers_end']}) | {'yes' if x['passes'] else 'no'} | {x['real']:.0f} ({x['user']:.0f}) |")
    sys.exit()

# Per arm and point, by the fixed rule.
V = {}
print('| arm | k | seeds | founders persist (seeds alive at 50) | guild holds | producers persist | point passes | pooled median death (not repro-first) | reproduced first | median ticks on earmark / structure |')
print('|---|---:|---:|---|---|---|---|---:|---:|---|')
for a in ARMS:
    for k in KS:
        xs = [R[(k, a, s)] for s in SEEDS if (k, a, s) in R]
        if not xs: continue
        alive50 = sum(x['f50'] > 0 for x in xs)
        holds = sum(x['holds'] for x in xs); pers = sum(x['persist'] for x in xs); pas = sum(x['passes'] for x in xs)
        pooled = median([d for x in xs for d in x['deaths_nrf']])
        V[(a, k)] = dict(n=len(xs), fp=alive50 >= 3, holds=holds >= 3, persist=pers >= 3, passes=pas >= 3,
                         pooled=pooled, rf=sum(x['repro_first'] for x in xs), nf=sum(x['n_founders'] for x in xs),
                         ear=median([x['on_earmark_med'] for x in xs]), stru=median([x['on_structure_med'] for x in xs]),
                         collapse=[x['seed'] for x in xs if not x['persist']])
        v = V[(a, k)]
        yn = lambda b, c: f"{'yes' if b else 'no'} ({c}/{len(xs)})"
        print(f"| {ARM_NAME[a]} | {k} | {len(xs)} | {yn(v['fp'], alive50)} | {yn(v['holds'], holds)} | {yn(v['persist'], pers)} | {yn(v['passes'], pas)} | {dt(pooled)} | {v['rf']}/{v['nf']} | {v['ear']} / {v['stru']} |")

print('\n## Prediction')
print('| k | neither | shrinking (Δ, ≤ 5?) | earmark (Δ, ≥ 20?) | both (Δ, ≥ 20?) |')
print('|---:|---:|---|---|---|')
for k in KS:
    if not all((a, k) in V for a in ARMS): continue
    n = V[('n', k)]['pooled']
    cell = lambda a, ok: f"{dt(V[(a, k)]['pooled'])} ({V[(a, k)]['pooled'] - n:+d}, {'yes' if ok(V[(a, k)]['pooled'] - n) else 'no'})"
    print(f"| {k} | {dt(n)} | {cell('s', lambda d: abs(d) <= 5)} | {cell('e', lambda d: d >= 20)} | {cell('b', lambda d: d >= 20)} |")
print('Per seed (median death among founders not reproduced first; Δ vs neither):')
for k in KS:
    for s in SEEDS:
        if not all((k, a, s) in R for a in ARMS): continue
        n = R[(k, 'n', s)]['death_med_nrf']
        print(f"  k={k} s={s}: neither {dt(n)}; " + '; '.join(f"{ARM_NAME[a]} {R[(k, a, s)]['death_med_nrf'] - n:+d}" for a in 'seb'))

print('\n## Follow-ups')
if len(V) < 16: print('(incomplete)')
many = lambda a, f: sum(f(V[(a, k)]) for k in KS if (a, k) in V) >= 2
for a in 'eb':
    if many(a, lambda v: v['fp'] and v['passes']):
        print(f'1 fires: founders persist and many points pass in the {ARM_NAME[a]} arm')
fp_guild = [(ARM_NAME[a], k) for (a, k), v in V.items() if v['fp'] and not v['holds']]
if fp_guild: print('2 fires: founders persist but the guild fails at', fp_guild)
np_rf = [(ARM_NAME[a], k) for (a, k), v in V.items() if not v['fp'] and v['rf'] > v['nf'] / 2]
if np_rf: print('3 fires: founders do not persist and most reproduced first at', np_rf)
np_ear = [(ARM_NAME[a], k) for (a, k), v in V.items() if not v['fp'] and a in 'eb' and v['ear'] > 0]
if np_ear: print('4 fires: founders do not persist though they drew the earmark down at', np_ear)
col = [(ARM_NAME[a], k, v['collapse']) for (a, k), v in V.items() if v['collapse']]
if col: print('5: producers did not persist in (arm, k, seeds):', col)
```

## Appendix: per arm, point and seed

The control arm. `S_p` is the median established producer structure over the control's second half. Founders alive are the readout's counts of the 81 founders. Median death is over all founders, with the median over those that had not reproduced first in brackets. "Ran out" counts founders whose reserve ran out. "Median earmark then" is the energy earmark as that step began, over those that ran out. Guild share is the guild's share of all carcass drain over the arm's second half; mixotroph share is the share of the arm's carcass drain by drainers of autotrophy at least 0.1; living share is the guild's living drain over its total ("-" where it drained nothing). Producers in brackets are the control's at its end. Runtime is wall-clock, CPU in brackets.

| k | arm | seed | S_p | founders alive 20 / 50 / 100 | median death (not repro-first) | ran out | reproduced first | median earmark then | median ticks on earmark / structure | guild holds (fewest) | guild share | mixotroph share | living share | producers persist (at end) | pass | runtime s (CPU) |
|---:|---|---:|---:|---|---:|---:|---:|---:|---|---|---:|---:|---:|---|---|---:|
| 0 | neither | 1 | 6.532 | 0 / 0 / 0 | 15 (15) | 80/81 | 0 | 22.789 | 0 / 0 | no (0) | 0.001 | 1.000 | 0.000 | yes (939) | no | 72 (67) |
| 0 | neither | 2 | 7.414 | 0 / 0 / 0 | 15 (15) | 81/81 | 0 | 22.791 | 0 / 0 | no (0) | 0.000 | 1.000 | - | yes (821) | no | 66 (62) |
| 0 | neither | 3 | 7.064 | 0 / 0 / 0 | 15 (15) | 81/81 | 0 | 22.788 | 0 / 0 | no (0) | 0.000 | 1.000 | - | yes (792) | no | 65 (62) |
| 0 | neither | 4 | 7.613 | 0 / 0 / 0 | 15 (15) | 81/81 | 0 | 22.787 | 0 / 0 | no (0) | 0.000 | 1.000 | - | yes (798) | no | 65 (61) |
| 0 | neither | 5 | 6.955 | 0 / 0 / 0 | 15 (15) | 81/81 | 0 | 22.791 | 0 / 0 | no (0) | 0.000 | 1.000 | - | yes (956) | no | 70 (67) |
| 0 | earmark | 1 | 5.200 | 80 / 80 / 80 | 139 (139) | 80/81 | 0 | 22.789 | 124 / 0 | no (0) | 0.000 | 1.000 | - | yes (976) | no | 74 (70) |
| 0 | earmark | 2 | 5.176 | 81 / 81 / 81 | 139 (139) | 81/81 | 0 | 22.791 | 124 / 0 | no (0) | 0.002 | 1.000 | 0.000 | yes (1068) | no | 81 (76) |
| 0 | earmark | 3 | 4.782 | 81 / 81 / 81 | 139 (139) | 81/81 | 0 | 22.788 | 124 / 0 | no (0) | 0.000 | 1.000 | - | yes (1046) | no | 74 (70) |
| 0 | earmark | 4 | 4.801 | 81 / 81 / 81 | 139 (139) | 81/81 | 0 | 22.787 | 124 / 0 | no (0) | 0.000 | 1.000 | - | yes (1057) | no | 78 (73) |
| 0 | earmark | 5 | 5.006 | 81 / 81 / 81 | 139 (139) | 81/81 | 0 | 22.791 | 124 / 0 | no (0) | 0.000 | 1.000 | - | yes (1041) | no | 79 (74) |
| 0 | shrinking | 1 | 7.007 | 0 / 0 / 0 | 18 (18) | 80/81 | 0 | 22.789 | 0 / 3 | no (0) | 0.000 | 1.000 | - | yes (823) | no | 63 (59) |
| 0 | shrinking | 2 | 7.731 | 0 / 0 / 0 | 18 (18) | 81/81 | 0 | 22.791 | 0 / 3 | no (0) | 0.000 | 1.000 | - | yes (801) | no | 64 (61) |
| 0 | shrinking | 3 | 7.097 | 0 / 0 / 0 | 18 (18) | 81/81 | 0 | 22.788 | 0 / 3 | no (0) | 0.000 | 1.000 | - | yes (948) | no | 71 (66) |
| 0 | shrinking | 4 | 6.810 | 0 / 0 / 0 | 18 (18) | 81/81 | 0 | 22.787 | 0 / 3 | no (0) | 0.001 | 1.000 | 0.000 | yes (897) | no | 66 (63) |
| 0 | shrinking | 5 | 7.249 | 0 / 0 / 0 | 18 (18) | 81/81 | 0 | 22.791 | 0 / 3 | no (0) | 0.000 | 1.000 | - | yes (840) | no | 63 (60) |
| 0 | both | 1 | 5.325 | 80 / 80 / 80 | 142 (142) | 80/81 | 0 | 22.789 | 125 / 3 | no (0) | 0.002 | 1.000 | 0.000 | yes (998) | no | 73 (69) |
| 0 | both | 2 | 4.812 | 81 / 81 / 81 | 142 (142) | 81/81 | 0 | 22.791 | 125 / 3 | no (0) | 0.001 | 1.000 | 0.000 | yes (1143) | no | 76 (72) |
| 0 | both | 3 | 4.611 | 81 / 81 / 81 | 142 (142) | 81/81 | 0 | 22.788 | 125 / 3 | no (0) | 0.000 | 1.000 | - | yes (1054) | no | 72 (69) |
| 0 | both | 4 | 4.847 | 81 / 81 / 81 | 142 (142) | 81/81 | 0 | 22.787 | 125 / 3 | no (0) | 0.000 | 1.000 | - | yes (1063) | no | 66 (63) |
| 0 | both | 5 | 5.323 | 81 / 81 / 81 | 142 (142) | 81/81 | 0 | 22.791 | 125 / 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (1061) | no | 66 (64) |
| 3 | neither | 1 | 6.806 | 0 / 0 / 0 | 15 (15) | 80/81 | 0 | 22.701 | 0 / 0 | no (0) | 0.004 | 1.000 | 0.000 | yes (831) | no | 87 (81) |
| 3 | neither | 2 | 6.984 | 0 / 0 / 0 | 15 (15) | 81/81 | 0 | 22.700 | 0 / 0 | no (0) | 0.000 | 1.000 | - | yes (931) | no | 95 (89) |
| 3 | neither | 3 | 6.986 | 0 / 0 / 0 | 15 (15) | 81/81 | 0 | 22.680 | 0 / 0 | no (0) | 0.004 | 1.000 | 0.000 | yes (898) | no | 96 (90) |
| 3 | neither | 4 | 6.902 | 0 / 0 / 0 | 15 (15) | 81/81 | 0 | 22.693 | 0 / 0 | no (0) | 0.000 | 1.000 | 0.000 | yes (822) | no | 91 (85) |
| 3 | neither | 5 | 7.530 | 0 / 0 / 0 | 15 (15) | 81/81 | 0 | 22.701 | 0 / 0 | no (0) | 0.000 | 1.000 | - | yes (756) | no | 90 (85) |
| 3 | earmark | 1 | 5.106 | 80 / 80 / 80 | 137 (137) | 80/81 | 0 | 22.701 | 122 / 0 | no (0) | 0.000 | 1.000 | - | yes (1073) | no | 92 (86) |
| 3 | earmark | 2 | 4.979 | 81 / 81 / 81 | 137 (137) | 81/81 | 0 | 22.700 | 122 / 0 | no (0) | 0.000 | 1.000 | - | yes (1152) | no | 100 (93) |
| 3 | earmark | 3 | 4.881 | 81 / 81 / 81 | 136 (136) | 81/81 | 0 | 22.680 | 121 / 0 | no (0) | 0.001 | 1.000 | 0.000 | yes (1081) | no | 101 (94) |
| 3 | earmark | 4 | 5.006 | 81 / 81 / 81 | 137 (137) | 81/81 | 0 | 22.693 | 122 / 0 | no (0) | 0.000 | 1.000 | - | yes (1067) | no | 96 (89) |
| 3 | earmark | 5 | 4.585 | 81 / 81 / 81 | 137 (137) | 81/81 | 0 | 22.701 | 122 / 0 | no (0) | 0.008 | 1.000 | 0.000 | yes (1111) | no | 96 (90) |
| 3 | shrinking | 1 | 6.815 | 0 / 0 / 0 | 18 (18) | 80/81 | 0 | 22.701 | 0 / 3 | no (0) | 0.000 | 1.000 | - | yes (795) | no | 76 (72) |
| 3 | shrinking | 2 | 7.199 | 0 / 0 / 0 | 18 (18) | 81/81 | 0 | 22.700 | 0 / 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (867) | no | 94 (87) |
| 3 | shrinking | 3 | 7.347 | 0 / 0 / 0 | 18 (18) | 81/81 | 0 | 22.680 | 0 / 3 | no (0) | 0.000 | 1.000 | - | yes (716) | no | 80 (75) |
| 3 | shrinking | 4 | 7.197 | 0 / 0 / 0 | 18 (18) | 81/81 | 0 | 22.693 | 0 / 3 | no (0) | 0.000 | 1.000 | - | yes (803) | no | 77 (72) |
| 3 | shrinking | 5 | 7.173 | 0 / 0 / 0 | 18 (18) | 81/81 | 0 | 22.701 | 0 / 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (886) | no | 91 (86) |
| 3 | both | 1 | 4.871 | 80 / 80 / 80 | 140 (140) | 80/81 | 0 | 22.701 | 123 / 3 | no (0) | 0.000 | 1.000 | - | yes (1155) | no | 95 (89) |
| 3 | both | 2 | 4.842 | 81 / 81 / 81 | 140 (140) | 81/81 | 0 | 22.700 | 123 / 3 | no (0) | 0.001 | 1.000 | 0.000 | yes (1174) | no | 103 (96) |
| 3 | both | 3 | 4.958 | 81 / 81 / 81 | 139 (139) | 81/81 | 0 | 22.680 | 122 / 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (1066) | no | 94 (88) |
| 3 | both | 4 | 4.766 | 81 / 81 / 81 | 140 (140) | 81/81 | 0 | 22.693 | 123 / 3 | no (0) | 0.000 | 1.000 | - | yes (999) | no | 92 (86) |
| 3 | both | 5 | 4.947 | 81 / 81 / 81 | 140 (140) | 81/81 | 0 | 22.701 | 123 / 3 | no (0) | 0.000 | 1.000 | - | yes (1053) | no | 87 (82) |
| 10 | neither | 1 | 8.030 | 0 / 0 / 0 | 14 (14) | 69/81 | 0 | 22.347 | 0 / 0 | no (0) | 0.000 | 1.000 | - | yes (795) | no | 165 (155) |
| 10 | neither | 2 | 6.709 | 0 / 0 / 0 | 15 (15) | 75/81 | 0 | 22.381 | 0 / 0 | no (0) | 0.000 | 1.000 | 0.000 | yes (954) | no | 197 (183) |
| 10 | neither | 3 | 7.743 | 0 / 0 / 0 | 14 (14) | 65/81 | 0 | 22.336 | 0 / 0 | no (0) | 0.000 | 1.000 | - | yes (769) | no | 129 (120) |
| 10 | neither | 4 | 7.884 | 0 / 0 / 0 | 14 (14) | 75/81 | 0 | 22.346 | 0 / 0 | no (0) | 0.000 | 1.000 | - | yes (688) | no | 176 (163) |
| 10 | neither | 5 | 7.004 | 0 / 0 / 0 | 15 (15) | 70/81 | 0 | 22.381 | 0 / 0 | no (0) | 0.000 | 1.000 | - | yes (902) | no | 188 (174) |
| 10 | earmark | 1 | 4.760 | 80 / 80 / 80 | 130 (130) | 81/81 | 0 | 22.350 | 115 / 0 | no (0) | 0.003 | 1.000 | 0.000 | yes (1061) | no | 230 (214) |
| 10 | earmark | 2 | 4.795 | 81 / 81 / 81 | 130 (130) | 81/81 | 0 | 22.380 | 116 / 0 | no (0) | 0.000 | 1.000 | - | yes (1148) | no | 142 (133) |
| 10 | earmark | 3 | 4.917 | 81 / 80 / 80 | 130 (130) | 81/81 | 0 | 22.357 | 115 / 0 | no (0) | 0.000 | 1.000 | 0.000 | yes (1014) | no | 144 (135) |
| 10 | earmark | 4 | 5.125 | 81 / 81 / 80 | 130 (130) | 81/81 | 0 | 22.347 | 115 / 0 | no (0) | 0.000 | 1.000 | - | yes (983) | no | 160 (150) |
| 10 | earmark | 5 | 4.938 | 81 / 80 / 80 | 130 (130) | 81/81 | 0 | 22.391 | 116 / 0 | no (0) | 0.000 | 1.000 | 0.000 | yes (991) | no | 208 (195) |
| 10 | shrinking | 1 | 6.639 | 0 / 0 / 0 | 17 (17) | 81/81 | 0 | 22.350 | 0 / 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (825) | no | 205 (192) |
| 10 | shrinking | 2 | 7.249 | 0 / 0 / 0 | 17 (17) | 81/81 | 0 | 22.380 | 0 / 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (822) | no | 179 (167) |
| 10 | shrinking | 3 | 7.926 | 0 / 0 / 0 | 17 (17) | 81/81 | 0 | 22.357 | 0 / 3 | no (0) | 0.000 | 1.000 | - | yes (790) | no | 201 (188) |
| 10 | shrinking | 4 | 7.472 | 0 / 0 / 0 | 17 (17) | 81/81 | 0 | 22.347 | 0 / 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (767) | no | 149 (139) |
| 10 | shrinking | 5 | 7.254 | 0 / 0 / 0 | 17 (17) | 81/81 | 0 | 22.391 | 0 / 3 | no (0) | 0.000 | 1.000 | - | yes (872) | no | 220 (207) |
| 10 | both | 1 | 4.581 | 80 / 80 / 80 | 133 (133) | 81/81 | 0 | 22.350 | 116 / 3 | no (0) | 0.001 | 1.000 | 0.000 | yes (1046) | no | 167 (157) |
| 10 | both | 2 | 4.783 | 81 / 81 / 81 | 133 (133) | 81/81 | 0 | 22.380 | 117 / 3 | no (0) | 0.000 | 1.000 | - | yes (1029) | no | 135 (127) |
| 10 | both | 3 | 4.580 | 81 / 81 / 80 | 133 (133) | 81/81 | 0 | 22.357 | 116 / 3 | no (0) | 0.003 | 1.000 | 0.000 | yes (1231) | no | 147 (138) |
| 10 | both | 4 | 4.965 | 81 / 81 / 80 | 133 (133) | 81/81 | 0 | 22.347 | 116 / 3 | no (0) | 0.000 | 1.000 | - | yes (938) | no | 158 (149) |
| 10 | both | 5 | 4.787 | 81 / 80 / 80 | 133 (133) | 81/81 | 0 | 22.391 | 117 / 3 | no (0) | 0.000 | 1.000 | - | yes (1059) | no | 193 (180) |
| 30 | neither | 1 | 8.023 | 0 / 0 / 0 | 14 (14) | 81/81 | 0 | 21.912 | 0 / 0 | no (0) | 0.002 | 1.000 | 0.000 | yes (773) | no | 505 (480) |
| 30 | neither | 2 | 7.339 | 0 / 0 / 0 | 14 (14) | 81/81 | 0 | 21.909 | 0 / 0 | no (0) | 0.019 | 1.000 | 0.000 | yes (842) | no | 512 (487) |
| 30 | neither | 3 | 7.397 | 0 / 0 / 0 | 14 (14) | 81/81 | 1 | 21.913 | 0 / 0 | no (0) | 0.001 | 1.000 | 0.000 | yes (779) | no | 823 (792) |
| 30 | neither | 4 | 7.803 | 0 / 0 / 0 | 14 (14) | 81/81 | 0 | 21.913 | 0 / 0 | no (0) | 0.025 | 1.000 | 0.000 | yes (788) | no | 486 (462) |
| 30 | neither | 5 | 7.644 | 0 / 0 / 0 | 14 (14) | 81/81 | 1 | 21.911 | 0 / 0 | no (1) | 0.102 | 1.000 | 0.000 | yes (747) | no | 601 (573) |
| 30 | earmark | 1 | 4.376 | 78 / 39 / 38 | 49 (49) | 81/81 | 0 | 21.912 | 35 / 0 | no (0) | 0.000 | 1.000 | - | yes (1092) | no | 665 (637) |
| 30 | earmark | 2 | 4.972 | 80 / 38 / 35 | 48 (48) | 81/81 | 0 | 21.909 | 34 / 0 | no (0) | 0.001 | 1.000 | 0.000 | yes (1017) | no | 760 (709) |
| 30 | earmark | 3 | 4.957 | 76 / 38 / 34 | 49 (49) | 81/81 | 1 | 21.913 | 35 / 0 | no (0) | 0.000 | 1.000 | 0.000 | yes (1008) | no | 579 (545) |
| 30 | earmark | 4 | 4.959 | 78 / 43 / 41 | 123 (123) | 81/81 | 0 | 21.913 | 109 / 0 | no (0) | 0.000 | 1.000 | - | yes (1031) | no | 964 (897) |
| 30 | earmark | 5 | 4.568 | 78 / 38 / 35 | 49 (49) | 81/81 | 1 | 21.911 | 35 / 0 | no (0) | 0.000 | 1.000 | 0.000 | yes (1030) | no | 1001 (928) |
| 30 | shrinking | 1 | 7.445 | 0 / 0 / 0 | 17 (17) | 81/81 | 0 | 21.912 | 0 / 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (756) | no | 659 (604) |
| 30 | shrinking | 2 | 7.578 | 0 / 0 / 0 | 17 (17) | 81/81 | 0 | 21.909 | 0 / 3 | no (0) | 0.144 | 1.000 | 0.000 | yes (815) | no | 693 (627) |
| 30 | shrinking | 3 | 7.944 | 0 / 0 / 0 | 17 (17) | 81/81 | 1 | 21.913 | 0 / 3 | no (0) | 0.001 | 1.000 | 0.000 | yes (745) | no | 676 (623) |
| 30 | shrinking | 4 | 7.042 | 0 / 0 / 0 | 17 (17) | 81/81 | 0 | 21.913 | 0 / 3 | no (0) | 0.001 | 1.000 | 0.000 | yes (818) | no | 542 (502) |
| 30 | shrinking | 5 | 7.428 | 0 / 0 / 0 | 17 (17) | 81/81 | 1 | 21.911 | 0 / 3 | no (0) | 0.007 | 1.000 | 0.000 | yes (770) | no | 816 (762) |
| 30 | both | 1 | 4.521 | 79 / 45 / 38 | 54 (54) | 81/81 | 0 | 21.912 | 37 / 3 | no (2) | 0.190 | 1.000 | 0.000 | yes (1151) | no | 677 (633) |
| 30 | both | 2 | 4.696 | 81 / 42 / 35 | 52 (52) | 81/81 | 0 | 21.909 | 35 / 3 | no (0) | 0.023 | 1.000 | 0.000 | yes (1018) | no | 859 (802) |
| 30 | both | 3 | 4.956 | 78 / 45 / 35 | 52 (52) | 81/81 | 1 | 21.913 | 35 / 3 | no (0) | 0.000 | 1.000 | - | yes (1043) | no | 468 (437) |
| 30 | both | 4 | 4.437 | 81 / 47 / 41 | 126 (126) | 81/81 | 0 | 21.913 | 110 / 3 | no (0) | 0.000 | 1.000 | 0.000 | yes (1040) | no | 469 (439) |
| 30 | both | 5 | 4.727 | 80 / 44 / 35 | 53 (53) | 81/81 | 1 | 21.911 | 36 / 3 | no (1) | 0.227 | 1.000 | 0.000 | yes (1010) | no | 1109 (1037) |
