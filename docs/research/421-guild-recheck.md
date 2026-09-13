# Issue #492 — re-check the four LHS configs with a heterotroph guild at 2000 ticks on `main`

**Status: research run plus a short note. Positive control for #443 §4.6 (on PR #489's
branch) / #490. One additive change to the `role_emergence` bin (per-sample role series
and the #490 guild predicate per run); no stepper, evaluator or search change.** #421's
`role-emergence.json` (2026-06-15, pre-#444 stepper, pre-#474 atlas) found 4 / 198
unselected LHS configs with a median heterotroph count ≥ 5 at `t = 2000` — the only known
candidates for a web where mutual invasibility could pass. Six stepper fixes later, this
note asks one question: is the guild still there on `main`?

## TL;DR

1. **All four still carry a heterotroph guild on `main` (one on a partial read), and it
   is larger, not smaller, than the pre-fix read.** By the #490 predicate
   (`genesis_eval::guild`: role count ≥ 5 on every classification sample over `t ∈ (1000, 2000]` plus ≥ 1 birth naming
   a member), the decomposer guild holds on **7 / 8 seeds of `sample:55`** (median
   terminal P / C / D = 399.5 / 6 / 41; was 42.5 / 5.5 / 20.5), **7 / 8 of `sample:20`**
   (138.5 / 3.5 / 22.5; was 31.5 / 0.5 / 21.5) and **3 / 8 of `sample:127`**
   (15 / 0 / 4; was 3 / 0 / 6). All three now survive 8 / 8 (were 6, 4, 7).
2. **`sample:55` is the only config with a consumer guild** — on 2 / 8 seeds (1006: C 26,
   D 174 at 2000; 1007: C 22, D 123), where the whole web is heterotroph-heavy. On every
   other seed of every config the consumer count sits at 0–15 and dips below 5 at least
   once over the second half. The predicate says no; the hand-read agrees.
3. **`sample:129` is a partial read: decomposer guild on 2 / 2 seeds run.** The 8-seed
   run was stopped at the 46-min cap with no seed complete (single-threaded under memory
   pressure, §1); a 2-seed re-run (seeds 1000–1001, 698 s) has D 11–19 and 6–8 sustained
   over the second half with births, P 466 / 573, C 0–4. Six seeds are unmeasured (§2.4).
4. **The next tier moved up too.** `sample:36` (was median heterotrophs 2–4.5) now reads
   median P / C / D = 26.5 / 2.5 / 22.5 with a decomposer guild on 5 / 8 seeds;
   `sample:111` on 2 / 8 (one seed at D 73); `sample:15` on 1 / 8; `sample:1` on 0 / 8.
5. **What this means for #493 / #494.** The positive control exists: `sample:55` and
   `sample:20` are the configs to spend an invasion run on — decomposer guild on 7 / 8
   seeds, and on `sample:55` a consumer guild on 2. The reachable space does contain
   heterotroph *populations* post-#444; what the atlas lacks is selection for them.

## 1. Method

- Commit `e0aabf6` (`main` after #498), `role_emergence` bin restricted via
  `ROLE_EMERGENCE_CONFIGS=sample:<i>` (the #491 `parse_selector`, `SAMPLE_SEED = 421` —
  the same seed-421 LHS draw #421 used, so `sample:55` is the same unit vector). 8 seeds
  (1000–1007), 2000 ticks, roles classified every 10 ticks via `topology::trophic_roles`,
  exactly as #421.
- **Additive bin change (this PR).** Each run record now carries `role_series` (P / C / D
  on every classification sample, JSON only) and `guild_consumer` / `guild_decomposer`
  — the evaluator's #490 `heterotroph_guilds` called on the run's own event log and a
  roster snapshot at every second-half sample (`tick > 1000`), so the predicate reads the
  same cadence the hand-readable series is on. `false` for any run that did not reach the
  horizon. Existing fields and the classification cadence are unchanged; the CSV gains
  two trailing columns. Driven by one test
  (`run_records_role_series_consistent_with_terminal_counts_and_guild_read`): the series
  is on cadence, ends on the terminal sample, agrees with `final_*`, and the predicate can
  only be true where the series shows the role ≥ `GUILD_MIN_SIZE` on every second-half
  sample.
- **Runtime.** The 8-GB host was under memory pressure from unrelated processes (~5 GB
  swap in use) and the harness killed every multi-threaded background run at ~7–10 min,
  so the expensive configs were run detached with `RAYON_NUM_THREADS=1` (≈ 100 MB RSS).
  Wall-clock per config, single-threaded, sum of seeds: `sample:20` 4077 s (68 min; peak
  population up to 2106 — the 40-min cap was already blown when it reported), `sample:55`
  1218 s (20 min), `sample:127` 2 s, the next tier 10–14 s each. `sample:129` was stopped
  at 46 min with no seed complete, then re-run on 2 seeds (2 threads): 518 s and 698 s per
  seed, so the full 8 would be ~1.5 h single-threaded.
- **Determinism.** `sample:127` run twice: byte-identical after dropping the `wall_time_s`
  column (the artifact carries per-run wall time, so the raw files differ only there;
  sha256 `21690066…` both runs). A 2-seed subset of `sample:55` and `sample:20` was
  re-run and compared to the same rows of the 8-seed run (§2.5).

## 2. Results

Columns: terminal mode; peak living population; terminal P / C / D; consumer and
decomposer counts at `t = 1100, 1200, …, 2000` with the minimum over *every* sample
(every 10 ticks) in the second half in parentheses — the #490 read is "min ≥ 5 and ≥ 1
birth"; and the predicate's verdict.

### 2.1 The four (#443 §4.6's candidates)

| config | old (pre-#444) survive / median P / C / D at 2000 | `main` survive / median P / C / D | guild D (seeds) | guild C (seeds) | verdict |
|---|---|---|---|---|---|
| `sample:55` | 6/8 · 42.5 / 5.5 / 20.5 | 8/8 · 399.5 / 6 / 41 | 7 / 8 | 2 / 8 | **guild present, grown; only consumer guild anywhere** |
| `sample:20` | 4/8 · 31.5 / 0.5 / 21.5 | 8/8 · 138.5 / 3.5 / 22.5 | 7 / 8 | 0 / 8 | **decomposer guild present, grown** |
| `sample:129` | 8/8 · 436 / 0.5 / 19.5 | 2/2 · 519.5 / 0.5 / 11.5 (2 seeds only) | 2 / 2 | 0 / 2 | **decomposer guild present on both seeds run; 6 unmeasured** (§2.4) |
| `sample:127` | 7/8 · 3 / 0 / 6 | 8/8 · 15 / 0 / 4 | 3 / 8 | 0 / 8 | **changed: decomposer guild on a minority of seeds** |


#### `sample:55`

| seed | mode | peak pop | final P / C / D | C at t = 1100 … 2000 (every 100; min over every sample) | D at t = 1100 … 2000 (min) | guild C | guild D |
|---|---|---|---|---|---|---|---|
| 1000 | survived | 1540 | 406 / 5 / 76 | 10 9 13 23 15 12 16 8 25 5 (min 2) | 47 55 55 59 61 70 74 80 82 76 (min 46) | no | yes |
| 1001 | survived | 1175 | 393 / 7 / 14 | 1 3 5 1 4 9 5 6 7 7 (min 0) | 9 13 18 16 14 11 14 14 15 14 (min 6) | no | yes |
| 1002 | survived | 1098 | 371 / 3 / 25 | 5 5 5 4 7 5 7 7 7 3 (min 2) | 19 23 21 22 22 21 22 23 24 25 (min 18) | no | yes |
| 1003 | survived | 918 | 424 / 7 / 57 | 5 4 11 4 8 7 8 10 11 7 (min 2) | 27 31 36 42 47 45 49 50 52 57 (min 15) | no | yes |
| 1004 | survived | 1482 | 442 / 5 / 23 | 7 5 1 0 1 1 4 5 5 5 (min 0) | 7 12 15 14 16 18 17 17 20 23 (min 5) | no | yes |
| 1005 | survived | 1076 | 526 / 1 / 9 | 1 4 1 2 0 3 1 3 2 1 (min 0) | 3 5 6 6 8 6 7 8 8 9 (min 3) | no | no |
| 1006 | survived | 1023 | 202 / 26 / 174 | 43 41 51 38 32 41 29 21 28 26 (min 17) | 144 154 164 170 174 165 170 168 173 174 (min 111) | yes | yes |
| 1007 | survived | 1110 | 326 / 22 / 123 | 14 21 31 21 26 25 30 32 28 22 (min 12) | 46 68 88 101 117 126 117 116 115 123 (min 34) | yes | yes |

#### `sample:20`

| seed | mode | peak pop | final P / C / D | C at t = 1100 … 2000 (every 100; min over every sample) | D at t = 1100 … 2000 (min) | guild C | guild D |
|---|---|---|---|---|---|---|---|
| 1000 | survived | 1439 | 95 / 3 / 45 | 6 4 4 9 11 5 4 0 3 3 (min 0) | 44 42 43 42 43 47 46 46 46 45 (min 40) | no | yes |
| 1001 | survived | 1807 | 135 / 2 / 19 | 7 6 4 1 3 3 4 1 3 2 (min 1) | 17 17 17 16 16 19 19 22 21 19 (min 15) | no | yes |
| 1002 | survived | 1969 | 142 / 1 / 0 | 0 0 0 1 0 0 1 0 0 1 (min 0) | 0 0 1 0 1 1 1 1 0 0 (min 0) | no | no |
| 1003 | survived | 2106 | 121 / 15 / 23 | 4 5 7 3 7 1 2 1 8 15 (min 1) | 7 8 9 10 12 17 17 20 20 23 (min 6) | no | yes |
| 1004 | survived | 2024 | 157 / 4 / 33 | 7 6 6 4 1 5 2 2 4 4 (min 1) | 25 28 28 30 29 28 31 28 32 33 (min 23) | no | yes |
| 1005 | survived | 1867 | 194 / 4 / 13 | 7 1 16 4 6 4 5 8 8 4 (min 0) | 9 8 8 7 8 8 9 9 10 13 (min 6) | no | yes |
| 1006 | survived | 1864 | 19 / 17 / 46 | 25 7 16 24 9 14 8 10 6 17 (min 3) | 60 54 53 51 50 51 47 45 43 46 (min 41) | no | yes |
| 1007 | survived | 1776 | 163 / 3 / 22 | 5 11 7 7 5 2 3 4 1 3 (min 0) | 15 16 19 18 19 21 21 21 22 22 (min 13) | no | yes |

#### `sample:127`

| seed | mode | peak pop | final P / C / D | C at t = 1100 … 2000 (every 100; min over every sample) | D at t = 1100 … 2000 (min) | guild C | guild D |
|---|---|---|---|---|---|---|---|
| 1000 | survived | 29 | 11 / 0 / 7 | 0 0 0 1 1 0 0 1 0 0 (min 0) | 7 8 7 7 7 8 9 7 7 7 (min 7) | no | yes |
| 1001 | survived | 35 | 15 / 1 / 6 | 2 1 1 1 1 1 1 1 1 1 (min 1) | 6 6 6 7 6 6 7 6 6 6 (min 4) | no | no |
| 1002 | survived | 35 | 16 / 1 / 5 | 4 3 1 1 1 1 1 1 1 1 (min 1) | 5 6 6 6 6 5 5 5 6 5 (min 5) | no | yes |
| 1003 | survived | 32 | 17 / 0 / 0 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 0 0 0 0 0 0 0 0 0 0 (min 0) | no | no |
| 1004 | survived | 42 | 13 / 4 / 14 | 8 10 10 11 10 7 6 5 4 4 (min 4) | 13 13 14 13 15 15 14 16 14 14 (min 13) | no | yes |
| 1005 | survived | 31 | 15 / 0 / 3 | 0 1 0 0 0 0 0 0 0 0 (min 0) | 3 3 3 3 3 3 3 3 3 3 (min 3) | no | no |
| 1006 | survived | 37 | 20 / 0 / 0 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 0 0 0 0 0 0 0 0 0 0 (min 0) | no | no |
| 1007 | survived | 21 | 11 / 0 / 0 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 0 0 0 0 0 0 0 0 0 0 (min 0) | no | no |

Reading the series: on `sample:55` and `sample:20` the decomposer count is not a founder
cohort sitting at the floor — it climbs through the second half on most seeds (`sample:55`
1000: 47 → 76; 1003: 27 → 57; 1007: 46 → 123) and the one failing seed on each
(`sample:55` 1005, D 3–9; `sample:20` 1002, D 0–1) is a web that never grew one, not one
that lost it. The consumer count on the same seeds is the opposite shape: 0–15, wandering,
with a second-half minimum of 0–3 on every seed except `sample:55` 1006 / 1007 (min 17 /
12), where P has collapsed to 202 / 326 against D 174 / 123. `sample:127` is a small
world (peak population 21–42) where three seeds hold a decomposer cohort of 5–16 with
births, three hold 3–6 without (1001 dips to 4; 1005 sits at 3), and two never grow one.

### 2.2 The next tier (`sample:36`, `sample:15`, `sample:1`, `sample:111`)

| config | old median heterotrophs | `main` survive / median P / C / D | guild D | guild C |
|---|---|---|---|---|
| `sample:36` | 2–4.5 | 8/8 · 26.5 / 2.5 / 22.5 | 5 / 8 | 0 / 8 |
| `sample:15` | 2–4.5 | 8/8 · 48 / 1 / 2 | 1 / 8 | 0 / 8 |
| `sample:1` | 2–4.5 | 8/8 · 52.5 / 0 / 2 | 0 / 8 | 0 / 8 |
| `sample:111` | 2–4.5 | 8/8 · 17.5 / 0 / 3 | 2 / 8 | 0 / 8 |

`sample:36` is the one that moved up: a decomposer guild of 30–59 on 5 seeds, in a world
of 26–299 agents. `sample:111` seed 1007 carries D 65–101 alone. The other two stay where
#421 had them — one to three heterotroph individuals.


#### `sample:36`

| seed | mode | peak pop | final P / C / D | C at t = 1100 … 2000 (every 100; min over every sample) | D at t = 1100 … 2000 (min) | guild C | guild D |
|---|---|---|---|---|---|---|---|
| 1000 | survived | 94 | 24 / 1 / 30 | 20 17 11 2 1 2 1 0 1 1 (min 0) | 29 34 42 33 31 32 30 30 30 30 (min 25) | no | yes |
| 1001 | survived | 29 | 13 / 0 / 2 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 2 2 2 2 2 2 2 2 2 2 (min 2) | no | no |
| 1002 | survived | 299 | 46 / 2 / 37 | 4 4 0 2 2 0 4 3 4 2 (min 0) | 17 20 24 31 30 33 35 35 34 37 (min 13) | no | yes |
| 1003 | survived | 26 | 0 / 0 / 2 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 2 2 2 2 2 2 2 2 2 2 (min 2) | no | no |
| 1004 | survived | 113 | 39 / 3 / 42 | 1 2 5 3 7 4 3 2 3 3 (min 1) | 33 30 33 33 38 36 38 38 36 42 (min 28) | no | yes |
| 1005 | survived | 108 | 29 / 3 / 59 | 2 4 3 4 6 1 8 4 4 3 (min 1) | 54 54 57 56 56 57 56 55 59 59 (min 50) | no | yes |
| 1006 | survived | 134 | 80 / 7 / 15 | 0 0 0 0 0 1 0 1 2 7 (min 0) | 2 2 2 2 3 5 8 8 9 15 (min 2) | no | no |
| 1007 | survived | 26 | 2 / 3 / 6 | 2 2 2 2 2 3 3 3 3 3 (min 2) | 6 6 6 6 6 6 6 6 6 6 (min 6) | no | yes |

#### `sample:15`

| seed | mode | peak pop | final P / C / D | C at t = 1100 … 2000 (every 100; min over every sample) | D at t = 1100 … 2000 (min) | guild C | guild D |
|---|---|---|---|---|---|---|---|
| 1000 | survived | 93 | 54 / 5 / 6 | 1 3 3 2 1 2 4 4 4 5 (min 1) | 9 8 7 7 7 6 6 7 7 6 (min 6) | no | yes |
| 1001 | survived | 275 | 248 / 1 / 2 | 2 2 2 2 2 2 2 2 3 1 (min 1) | 2 2 3 2 2 2 2 1 1 2 (min 1) | no | no |
| 1002 | survived | 297 | 127 / 0 / 2 | 0 0 0 0 0 1 0 0 0 0 (min 0) | 2 2 2 2 2 2 2 2 2 2 (min 2) | no | no |
| 1003 | survived | 70 | 34 / 1 / 1 | 3 3 3 3 2 2 2 2 1 1 (min 1) | 2 2 2 2 2 2 2 1 1 1 (min 1) | no | no |
| 1004 | survived | 81 | 59 / 1 / 1 | 1 1 2 1 1 1 1 1 1 1 (min 1) | 3 2 2 2 1 1 1 1 1 1 (min 1) | no | no |
| 1005 | survived | 57 | 34 / 0 / 5 | 2 2 2 2 2 2 2 1 1 0 (min 0) | 4 4 4 4 4 4 4 5 5 5 (min 4) | no | no |
| 1006 | survived | 76 | 36 / 0 / 2 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 3 3 3 3 5 3 3 2 2 2 (min 2) | no | no |
| 1007 | survived | 76 | 42 / 1 / 6 | 1 1 2 1 0 1 1 0 0 1 (min 0) | 4 4 4 5 6 5 5 6 6 6 (min 4) | no | no |

#### `sample:1`

| seed | mode | peak pop | final P / C / D | C at t = 1100 … 2000 (every 100; min over every sample) | D at t = 1100 … 2000 (min) | guild C | guild D |
|---|---|---|---|---|---|---|---|
| 1000 | survived | 173 | 58 / 0 / 3 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 3 3 3 3 3 3 3 3 3 3 (min 3) | no | no |
| 1001 | survived | 157 | 47 / 0 / 3 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 5 5 5 5 4 4 4 3 3 3 (min 3) | no | no |
| 1002 | survived | 56 | 8 / 0 / 2 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 2 2 2 2 2 2 2 2 2 2 (min 2) | no | no |
| 1003 | survived | 50 | 10 / 0 / 2 | 1 1 1 1 0 0 0 0 0 0 (min 0) | 1 1 1 1 2 2 2 2 2 2 (min 1) | no | no |
| 1004 | survived | 234 | 157 / 1 / 4 | 1 1 1 1 1 1 1 1 1 1 (min 1) | 4 4 4 4 4 4 4 4 4 4 (min 3) | no | no |
| 1005 | survived | 127 | 72 / 0 / 2 | 2 2 2 2 2 0 0 0 0 0 (min 0) | 1 1 1 1 1 3 2 2 2 2 (min 1) | no | no |
| 1006 | survived | 44 | 9 / 0 / 2 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 3 3 2 2 2 2 2 2 2 2 (min 2) | no | no |
| 1007 | survived | 117 | 101 / 0 / 1 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 3 2 2 2 2 2 2 1 1 1 (min 1) | no | no |

#### `sample:111`

| seed | mode | peak pop | final P / C / D | C at t = 1100 … 2000 (every 100; min over every sample) | D at t = 1100 … 2000 (min) | guild C | guild D |
|---|---|---|---|---|---|---|---|
| 1000 | survived | 27 | 1 / 0 / 2 | 0 0 1 0 1 0 0 0 0 0 (min 0) | 6 4 5 4 3 3 2 2 2 2 (min 2) | no | no |
| 1001 | survived | 25 | 1 / 0 / 1 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 1 1 1 1 1 1 1 1 1 1 (min 1) | no | no |
| 1002 | survived | 27 | 9 / 0 / 0 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 0 0 0 0 0 0 0 0 0 0 (min 0) | no | no |
| 1003 | survived | 27 | 6 / 0 / 4 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 4 4 4 4 4 4 4 4 4 4 (min 4) | no | no |
| 1004 | survived | 107 | 67 / 7 / 14 | 3 2 4 9 11 10 15 9 5 7 (min 1) | 3 2 2 5 10 13 13 16 15 14 (min 2) | no | no |
| 1005 | survived | 192 | 105 / 1 / 11 | 1 2 3 2 2 4 3 2 0 1 (min 0) | 7 7 5 6 7 8 10 10 13 11 (min 5) | no | yes |
| 1006 | survived | 56 | 26 / 0 / 1 | 0 0 0 0 0 0 0 0 0 0 (min 0) | 3 2 1 1 1 1 1 1 1 1 (min 1) | no | no |
| 1007 | survived | 199 | 28 / 4 / 73 | 35 28 23 20 14 14 14 6 6 4 (min 2) | 101 88 90 84 85 87 75 74 77 73 (min 65) | no | yes |

### 2.3 Attribution — what changed the read

Not bisected here. Two things are known to have changed between the runs and both push
the same way: the stepper (#444–#453; #474 §4 shows #444 alone changes 57 % of founders
and #445 changes trajectories) and survival itself — every config now survives 8 / 8,
where `sample:20` survived 4 / 8 before, so the medians are over eight webs rather than a
mix of webs and empty worlds. The direction (larger decomposer guilds, more seeds
carrying one, producers up 3–10×) is consistent with #474's headline that the fixed
physics lifts populations rather than culling them, but which fix did it is a bisect
question, not this note's.

### 2.4 `sample:129`

The 8-seed run was stopped at 46 min single-threaded with no seed complete (the bin's
progress log is per-completed-run, so nothing partial survives a stop). Its pre-fix read
(P 436 at 2000) puts it in `sample:20`'s cost class. The remaining six seeds need a run
on a machine with headroom (~1 h at 8 threads by the per-seed times below).

**The 2-seed partial run completed** (`ROLE_EMERGENCE_SEEDS=2`, 2 threads, 698 s
wall — 518 s and 698 s per seed). Both seeds survive with P 466 / 573 (peak population
1966 / 2009), and both carry a decomposer guild by the predicate (D 11–19 and 6–8 over the
second half, births present); consumers are 0–4. Median terminal P / C / D over the two
= 519.5 / 0.5 / 11.5 against the old 436 / 0.5 / 19.5 on eight seeds. Read as: **guild
still present on the two seeds run, 2 / 2; the other six seeds are unmeasured.**

#### `sample:129`

| seed | mode | peak pop | final P / C / D | C at t = 1100 … 2000 (every 100; min over every sample) | D at t = 1100 … 2000 (min) | guild C | guild D |
|---|---|---|---|---|---|---|---|
| 1000 | survived | 1966 | 466 / 0 / 16 | 2 2 0 1 3 4 0 4 0 0 (min 0) | 11 13 14 14 15 15 18 19 16 16 (min 11) | no | yes |
| 1001 | survived | 2009 | 573 / 1 / 7 | 2 0 0 0 1 0 0 0 0 1 (min 0) | 7 7 6 6 6 7 8 8 8 7 (min 6) | no | yes |

### 2.5 Determinism

- `sample:127`, 8 seeds, two full runs: identical after dropping `wall_time_s`
  (sha256 of the stripped artifact `21690066f5b7…` both times).
- `sample:55` and `sample:20`, `ROLE_EMERGENCE_SEEDS=2` re-run (2 threads) against the
  same seeds' rows of the 8-seed single-threaded run: all four rows identical field for
  field (`role_series`, guild verdicts, milestones, populations) except `wall_time_s`.
  Thread count and subset do not change a row, as the #423 subset-mode contract says.

## 3. What this means for #493 / #494

The amended invasion criterion has a positive control on `main`: `sample:55` (decomposer
guild 7 / 8 seeds, consumer guild 2 / 8, P ≈ 400) and `sample:20` (decomposer 7 / 8).
Spend the invasion run on those two; `sample:127` and `sample:36` are the small-world
variants (P 15–27) if a cheap one is wanted. And the reframing #490 feared is not needed:
the reachable space *does* contain heterotroph populations post-#444 — the atlas has none
because `coexistence_fraction` never selected for them, which is #486's point, not a
stepper limit.
