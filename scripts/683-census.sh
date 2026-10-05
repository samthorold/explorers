#!/usr/bin/env bash
# #683: the census of mobile autotrophy (kin_killer_diet section L) on the
# committed atlas, seeds 1000–1004, T = 2000, ungated (satiation sensitivity
# 0), in two modes: decoded and at founder_aggregation = 0. The reading
# decides whether substrate contact (#648) comes out of reserve
# (world-rules.md flow 2).
#
# From the repo root:   scripts/683-census.sh
# Smoke:                env K=target/683-smoke C=atlas:0 T=200 scripts/683-census.sh
#
# Env: ATLAS (default atlas.json), K (output dir, default target/683), C
# (configs, default atlas:0..atlas:N-1 with N the atlas's cell count), T
# (horizon, default 2000), SEED (base seed, default 1000), E (ensemble,
# default 5).
#
# Resumable: a finished step leaves $K/<step>.done and is skipped; an
# interrupted step resumes from the configs already in its --out JSON lines
# (the example skips them), after dropping a torn last line if the kill left
# one. The example is built once and copied into $K/bin, so a later rebuild
# cannot swap it mid-run. Each mode's report (with section L) is
# $K/<mode>.md, its rows $K/<mode>.jsonl. Prints DONE when everything is done.
set -euo pipefail
cd "$(dirname "$0")/.."

ATLAS=${ATLAS:-atlas.json}
K=${K:-target/683}
T=${T:-2000}
SEED=${SEED:-1000}
E=${E:-5}
if [ ! -s "$ATLAS" ]; then
  echo "683-census.sh: no atlas at $ATLAS" >&2
  exit 1
fi
N=$(jq '.cells | length' "$ATLAS")
C=${C:-$(seq -s, -f 'atlas:%g' 0 $((N - 1)))}
BIN=$K/bin
mkdir -p "$K"
LOG=$K/run.log
TIMES=$K/times.log
echo "== $(date) atlas $ATLAS ($N cells), configs $C, T $T, seeds $SEED+$E, out $K" >>"$LOG"

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

kkd() { # out-stem, flags...
  local s=$1
  shift
  untear "$K/$s.jsonl"
  # Ungated: satiation sensitivity 0.
  "$BIN/kin_killer_diet" --atlas "$ATLAS" --configs "$C" \
    --max-ticks "$T" --seed "$SEED" --ensemble "$E" \
    --satiation-sensitivity 0 ${@+"$@"} \
    --out "$K/$s.jsonl" >"$K/$s.md"
}

FA0=(--founder-aggregation 0)

step build build
step decoded kkd decoded
step fa0 kkd fa0 ${FA0[@]+"${FA0[@]}"}

echo DONE >>"$TIMES"
echo DONE
