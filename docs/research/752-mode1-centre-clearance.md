# Issue #752: first paired readings of the mode-1 centre clearance

**Status: a quick look at the instrument, not the mode-1 reading (that is #755), 2026-10-09.
`reference_mode --clear-at 1500 [--ticks N] [--seed S] [--no-decomposers]`, committed
`recipe.json`, wear 0.2 (the provisional rate). The known state has the problems #751 and #753
record. The readings below show what the paired machinery reports on it, not whether mode 1 is
produced.**

## What the clearance takes out

At tick 1,500 the centre cell holds almost no living agents. The clearance is nearly all
carcass matter:

| seed | community | agents | carcasses | energy | nutrient | centre producers at the fork (control) |
|---|---|---:|---:|---:|---:|---:|
| 1 | with decomposers | 0 | 70 | 1,204.8 | 847.06 | 0 |
| 1 | producers only | 0 | 70 | 1,287.5 | 947.57 | 0 |
| 2 | with decomposers | 1 | 77 | 923.2 | 694.34 | 1 (age 1,493) |
| 3 | with decomposers | 0 | 49 | 505.6 | 359.79 | 0 |
| 4 | with decomposers | 1 | 92 | 994.3 | 666.47 | 1 (age 1,482) |
| 5 | with decomposers | 0 | 36 | 238.9 | 473.77 | 0 |

## What follows (1,500 ticks per arm, seeds 2–5)

- No perturbed arm is recolonised by tick 3,000. Where the control's centre held a producer at the
  fork (seeds 2, 4), the control still holds 2–3 at tick 3,000 and the perturbed centre holds none.
- Where the centre was already empty at the fork (seeds 1, 3, 5), the centre producers are the
  same in both arms, sample for sample. The arms differ only in the carcass pile and the pool it
  leaches into: in the control the pile leaches into the pool, and in the perturbed arm the pool
  holds what it had at the fork until producers draw it down. On seed 5, producers aged 800 and
  more reach both arms' centres from tick ~1,625. On seed 1 (3,000 ticks), two producers reach
  both centres at tick ~4,425.
- Each arm's budget closes with the clearance booked as an outflow. On seed 1, over 3,000 ticks
  per arm, the energy residual is 17 against 427,000 in (4 × 10⁻⁵, f32 rounding of the sim's
  cumulative counters) and the nutrient residual is 1.3 against 18,005.

As #751 found, the control does not persist at the patch scale: the centre holds 0–1 producers at
the fork. So these readings cannot yet tell the response apart from drift (reference-modes.md,
*How a reference mode is read*).
