#!/usr/bin/env bash
# Local runs on the owner's PC (spec 48, Decision 178), in three modes. Build first:
# bash native/build.sh (the pair mode builds its own two commits).
#
#   bash ci/metrics/local_grid.sh grid [rounds]                (default 3)
#   bash ci/metrics/local_grid.sh inspect [rounds]             (default 3; after grid)
#   bash ci/metrics/local_grid.sh pair <head> <base> [rounds]  (default 5)
#
# grid (step 1, 5b): squad size against squad count. Runs `breach_tick_phases side=<s>
#   squads=<q> fight=10 series=1` for every cell (its settled series, its `inspect:` ticks),
#   the rounds taken in turn (round 1 of every cell, then round 2, ...), into bench-grid/
#   (git-ignored), then imports them as local records under metrics/local/.
# inspect (5b): replays each cell's `inspect:` ticks (round 1's, from bench-grid/) with
#   `at=<ticks> repeat=50`, the rounds in turn, into bench-grid/inspect/, then imports them:
#   one record per (cell, tick). A guide, not the judge (Q47, #909).
# pair (5b): builds <head> and <base> (any commit or branch) in worktrees under bench-pair/
#   (git-ignored), runs each side's settled series once per size, then times both in turn
#   (head first in odd rounds, base first in even ones) at the base's `inspect:` ticks, one
#   tick a run, with repeat=50, as CI's bench-inspect does on one runner. It prints the head's
#   change against the base per size and tick (ci/metrics/local_pair.py). It records nothing.
#   Its sizes are CI's, PAIR_SIZES="preset=4096 preset=25k": each size one battle argument.
#
# A new grid shape is a change to SIDES or SQUADS below.
set -euo pipefail

here="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$here"

SIDES=(3072 6144 12288 24576)
SQUADS=(1 2 4 8 24)
FIGHT=10
REPEAT=50
read -r -a PAIR_SIZES <<< "${PAIR_SIZES:-preset=4096 preset=25k}"

usage() {
  sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//' >&2
  exit 2
}

tick_phases() { # the binary under a checkout: tick_phases <root> (BREACH_TICK_PHASES overrides
  # it for grid and inspect)
  if [ "$1" = . ] && [ -n "${BREACH_TICK_PHASES:-}" ]; then echo "$BREACH_TICK_PHASES"; return; fi
  local bin="$1/native/bin/breach_tick_phases"
  [ -x "$bin" ] || bin="$bin.exe"
  if [ ! -x "$bin" ]; then
    echo "local_grid: no $bin; build it first: bash native/build.sh" >&2
    exit 2
  fi
  echo "$bin"
}

inspect_line() { # the `inspect:` ticks a settled series printed: inspect_line <file>
  sed -n 's/^inspect: \([0-9,]*\).*$/\1/p' "$1" | tr -d '\r' | head -n 1
}

import_records() { # import_records <folder>
  local sha date
  sha="$(git rev-parse HEAD)"
  date="$(date -u +%Y-%m-%d)"
  python3 ci/metrics/local_record.py "$1" --commit "$sha" --date "$date" --fight "$FIGHT"
  echo "local_grid: keep the outputs with: cp -r $1 metrics/raw/local/$date-${sha:0:7}"
}

grid() {
  local rounds="${1:-3}" bin out="bench-grid"
  bin="$(tick_phases .)"
  rm -rf "$out"
  mkdir -p "$out"
  for round in $(seq 1 "$rounds"); do
    for side in "${SIDES[@]}"; do
      for squads in "${SQUADS[@]}"; do
        echo "round $round: side=$side squads=$squads"
        "$bin" "side=$side" "squads=$squads" "fight=$FIGHT" series=1 \
          > "$out/s${side}_q${squads}_r${round}.txt"
      done
    done
  done
  import_records "$out"
}

inspect() {
  local rounds="${1:-3}" bin out="bench-grid/inspect" settled ticks
  bin="$(tick_phases .)"
  rm -rf "$out"
  mkdir -p "$out"
  for round in $(seq 1 "$rounds"); do
    for side in "${SIDES[@]}"; do
      for squads in "${SQUADS[@]}"; do
        settled="bench-grid/s${side}_q${squads}_r1.txt"
        ticks="$( [ -f "$settled" ] && inspect_line "$settled" || true)"
        if [ -z "$ticks" ]; then
          echo "local_grid: no inspect: line in $settled; run the grid first" >&2
          exit 1
        fi
        echo "round $round: side=$side squads=$squads at=$ticks"
        "$bin" "side=$side" "squads=$squads" "at=$ticks" "repeat=$REPEAT" \
          > "$out/s${side}_q${squads}_r${round}_repeat.txt"
      done
    done
  done
  import_records "$out"
}

pair() {
  local head="$1" base="$2" rounds="${3:-5}" root="bench-pair" out size label ticks tick
  local -A bins=()
  rm -rf "$root"
  git worktree prune
  mkdir -p "$root/out"
  out="$root/out"
  for side in head base; do
    local ref="$head"; [ "$side" = base ] && ref="$base"
    git worktree add -f --detach "$root/$side" "$ref" > /dev/null
    echo "pair: building $side ($ref, $(git -C "$root/$side" rev-parse --short HEAD))"
    bash "$root/$side/native/build.sh" > "$root/build_$side.log"
    bins[$side]="$(tick_phases "$root/$side")"
  done
  for size in "${PAIR_SIZES[@]}"; do
    label="${size//=/-}"
    for side in base head; do
      echo "pair: $label, the $side's settled series"
      "${bins[$side]}" "$size" series=1 > "$out/${label}_series_${side}.txt"
    done
    ticks="$(inspect_line "$out/${label}_series_base.txt")"
    if [ -z "$ticks" ]; then
      echo "local_grid: the base printed no inspect: line at $size" >&2
      exit 1
    fi
    for round in $(seq 1 "$rounds"); do
      local order=(head base); [ $((round % 2)) -eq 0 ] && order=(base head)
      for tick in ${ticks//,/ }; do
        for side in "${order[@]}"; do
          echo "pair: $label round $round, tick $tick, $side"
          "${bins[$side]}" "$size" "at=$tick" "repeat=$REPEAT" \
            > "$out/${label}_t${tick}_${side}_r${round}.txt"
        done
      done
    done
  done
  git worktree remove --force "$root/head"
  git worktree remove --force "$root/base"
  python3 ci/metrics/local_pair.py "$out"
}

mode="${1:-grid}"
case "$mode" in
  grid) shift $(( $# > 0 )); grid "$@" ;;
  [0-9]*) grid "$@" ;; # the step 1 form: local_grid.sh [rounds]
  inspect) shift; inspect "$@" ;;
  pair) shift; [ $# -ge 2 ] || usage; pair "$@" ;;
  *) usage ;;
esac
