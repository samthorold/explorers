#!/usr/bin/env bash
# #700: the calibration read that sets the top of carcass leaching's rate λ
# (world-rules.md, *Carcass energy decays only through agents; carcass
# nutrient leaches*), and a first λ sweep, on the committed atlas: ungated,
# decoded at founder_aggregation = 0, seeds 1000–1004, T = 2000. Both phases
# run kin_killer_diet, whose section M is the readout (time to first drain,
# λ_max candidates, the sweep's quantities).
#
# Phase 1, the calibration at λ = 0:
#   scripts/700-leaching.sh
# Phase 2, the sweep over a λ list (λ = 0 reuses phase 1's rows):
#   env PHASE=2 LAMBDAS="0 0.004 0.008 0.016" scripts/700-leaching.sh
# Smoke:
#   env K=target/700-smoke C=atlas:0,atlas:1 T=300 scripts/700-leaching.sh
#   env K=target/700-smoke C=atlas:0,atlas:1 T=300 PHASE=2 LAMBDAS="0 0.05" scripts/700-leaching.sh
#
# Env: ATLAS (default atlas.json), K (output dir, default target/700), C
# (configs, default every atlas cell), T (horizon, default 2000), SEED (base
# seed, default 1000), E (ensemble, default 5), PHASE (1 or 2, default 1),
# LAMBDAS (phase 2's λ list, space-separated, required there).
#
# Outputs: one directory per λ, $K/lambda-<λ>/ (phase 1 is $K/lambda-0),
# holding rows.jsonl (one row per config; per seed `tally.leach_runs`, per
# carcass `[died, drained | null, bitten]`) and report.md (section M at the
# end). Phase 1 also prints M1–M2 (time to first drain, the λ_max readings)
# to $K/calibration.md; phase 2 writes the sweep table over every λ in the
# list to $K/sweep.md.
#
# Resumable: a finished λ leaves $K/lambda-<λ>.done and is skipped; an
# interrupted λ resumes from the configs already in its rows.jsonl (the
# example skips them), after dropping a torn last line if the kill left one.
# The example is built once and copied into $K/bin, so a later rebuild cannot
# swap it mid-run (delete $K/build.done to rebuild). Prints DONE when the
# phase is done.
set -euo pipefail
cd "$(dirname "$0")/.."

ATLAS=${ATLAS:-atlas.json}
K=${K:-target/700}
T=${T:-2000}
SEED=${SEED:-1000}
E=${E:-5}
PHASE=${PHASE:-1}
LAMBDAS=${LAMBDAS:-}
if [ ! -s "$ATLAS" ]; then
  echo "700-leaching.sh: no atlas at $ATLAS" >&2
  exit 1
fi
if [ "$PHASE" != 1 ] && [ "$PHASE" != 2 ]; then
  echo "700-leaching.sh: PHASE must be 1 or 2, got $PHASE" >&2
  exit 1
fi
if [ "$PHASE" = 2 ] && [ -z "$LAMBDAS" ]; then
  echo "700-leaching.sh: phase 2 needs LAMBDAS, e.g. LAMBDAS=\"0 0.01 0.02 0.04\"" >&2
  exit 1
fi
N=$(jq '.cells | length' "$ATLAS")
C=${C:-$(seq -s, -f 'atlas:%g' 0 $((N - 1)))}
BIN=$K/bin
mkdir -p "$K"
LOG=$K/run.log
TIMES=$K/times.log
echo "== $(date) phase $PHASE, atlas $ATLAS ($N cells), configs $C, T $T, seeds $SEED+$E, λ ${LAMBDAS:-0}, out $K" >>"$LOG"

# Drop an unterminated last line (a row cut off mid-append).
untear() {
  local f=$1
  if [ -s "$f" ] && [ "$(tail -c1 "$f" | od -An -tx1 | tr -d ' ')" != "0a" ]; then
    echo "untear $f" >>"$LOG"
    perl -0777 -i -pe 's/[^\n]*\z//' "$f"
  fi
}

# step <name> <command...>: run once, time it, mark it done.
step() {
  local name=$1
  shift
  if [ -e "$K/$name.done" ]; then
    echo "skip $name (done)" >>"$LOG"
    return
  fi
  echo "== $(date) $name: $*" >>"$LOG"
  local t0
  t0=$(date +%s)
  "$@" 2>>"$LOG"
  echo "$name: $(($(date +%s) - t0)) s" >>"$TIMES"
  touch "$K/$name.done"
}

build() {
  env CARGO_TARGET_DIR=target/agent cargo build --release -p explorers-search \
    --example kin_killer_diet
  mkdir -p "$BIN"
  cp target/agent/release/examples/kin_killer_diet "$BIN/"
}

# Is this λ zero (0, 0.0, 0e0, …)?
is_zero() {
  awk -v l="$1" 'BEGIN { exit !(l + 0 == 0) }'
}

# The directory stem for a λ: phase 1's for any zero.
stem() {
  if is_zero "$1"; then
    echo lambda-0
  else
    echo "lambda-$1"
  fi
}

# run_lambda <λ>: every config × seed at this λ.
run_lambda() {
  local l=$1 d
  d=$K/$(stem "$l")
  mkdir -p "$d"
  untear "$d/rows.jsonl"
  "$BIN/kin_killer_diet" --atlas "$ATLAS" --configs "$C" \
    --max-ticks "$T" --seed "$SEED" --ensemble "$E" \
    --founder-aggregation 0 --leaching-rate "$l" \
    --out "$d/rows.jsonl" >"$d/report.md"
}

# M1–M2 of phase 1's report.
calibration() {
  awk '/^#### M1\./ { on = 1 } /^#### M3\./ { on = 0 } on' \
    "$K/lambda-0/report.md" >"$K/calibration.md"
}

sweep() {
  local files=() l
  for l in $LAMBDAS; do
    files+=("$K/$(stem "$l")/rows.jsonl")
  done
  local list
  list=$(
    IFS=,
    echo "${files[*]+"${files[*]}"}"
  )
  "$BIN/kin_killer_diet" --leaching-sweep "$list" >"$K/sweep.md"
}

step build build
step lambda-0 run_lambda 0
if [ "$PHASE" = 1 ]; then
  calibration
  cat "$K/calibration.md"
else
  for l in $LAMBDAS; do
    step "$(stem "$l")" run_lambda "$l"
  done
  sweep
  cat "$K/sweep.md"
fi

echo "DONE phase $PHASE" >>"$TIMES"
echo DONE
