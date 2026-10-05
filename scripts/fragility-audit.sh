#!/usr/bin/env bash
# #693: the fragility audit of the committed atlas — for each cell, how often
# its outcome flips across 10 seeds, and across 8 Gaussian jitters of its
# unit vector at radii 0.01, 0.03 and 0.1, rolled out exactly as the search
# rolls out (its evaluator with the bloom stop at 300:10, T = 2000, its
# 600 s + 600 s per-rollout budget) on current main physics.
#
# From the repo root:   scripts/fragility-audit.sh
# Smoke:                env K=target/fragility-smoke C=atlas:0,atlas:1 T=300 D=2 scripts/fragility-audit.sh
#
# Env: ATLAS (default atlas.json), K (output dir, default target/fragility),
# C (configs, default every atlas cell), T (horizon, default 2000), SEED
# (base seed, default 1000), E (ensemble, default 10), D (draws per radius,
# default 8), R (radii, default 0.01,0.03,0.1).
#
# Resumable: a finished step leaves $K/<step>.done and is skipped; an
# interrupted audit resumes from the rows already in $K/rows.jsonl (the bin
# skips them; a killed run loses at most the cell in flight), after dropping
# a torn last line if the kill left one. The bin is built once and copied
# into $K/bin, so a later rebuild cannot swap it mid-run. The summary is
# $K/summary.md (and $K/summary.json). Prints DONE when everything is done.
set -euo pipefail
cd "$(dirname "$0")/.."

ATLAS=${ATLAS:-atlas.json}
K=${K:-target/fragility}
T=${T:-2000}
SEED=${SEED:-1000}
E=${E:-10}
D=${D:-8}
R=${R:-0.01,0.03,0.1}
if [ ! -s "$ATLAS" ]; then
  echo "fragility-audit.sh: no atlas at $ATLAS" >&2
  exit 1
fi
N=$(jq '.cells | length' "$ATLAS")
C=${C:-$(seq -s, -f 'atlas:%g' 0 $((N - 1)))}
BIN=$K/bin
mkdir -p "$K"
LOG=$K/run.log
TIMES=$K/times.log
echo "== $(date) atlas $ATLAS ($N cells), configs $C, T $T, seeds $SEED+$E, draws $D, radii $R, out $K" >>"$LOG"

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
    --bin fragility_audit
  mkdir -p "$BIN"
  cp target/agent/release/fragility_audit "$BIN/"
}

audit() {
  untear "$K/rows.jsonl"
  "$BIN/fragility_audit" --atlas "$ATLAS" --configs "$C" \
    --horizon "$T" --seed "$SEED" --ensemble "$E" --draws "$D" --radii "$R" \
    --out "$K/rows.jsonl" >/dev/null
}

summary() {
  "$BIN/fragility_audit" --atlas "$ATLAS" --summary \
    --out "$K/rows.jsonl" --json "$K/summary.json" >"$K/summary.md"
}

step build build
step audit audit
step summary summary

echo DONE >>"$TIMES"
echo DONE
