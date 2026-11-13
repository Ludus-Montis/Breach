#!/usr/bin/env bash
# Local runs on the owner's PC (spec 48, Decision 178), in three modes. Build first:
# bash native/build.sh (the pair mode fetches or builds its own two commits).
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
#   inspect and pair pass `laps=1` (5f) where the binary takes it, so the raw output keeps
#   every lap after each tick's table; the records keep the medians and quartiles, not the laps.
# pair (5b): fetches <head>'s and <base>'s (any commit or branch) `release` binaries from the
#   native store over HTTPS, without credentials (spec 49 step 8), or, when either key is
#   missing, builds both in worktrees, never one of each (Q48 point 2); all under bench-pair/
#   (git-ignored). PAIR_STORE_URL names another store, PAIR_TARGET another target (the host's
#   by default: Linux or Windows on x86_64). It runs each side's settled series once per size,
#   then times both in turn
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
PAIR_STORE_URL="${PAIR_STORE_URL:-https://github.com/Ludus-Montis/Breach/releases/download/native-store}"

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

laps_of() { # `laps=1` (spec 48 step 5f) for a binary that takes it; nothing for one built
  # before it, which would refuse it (a pair's base, an older BREACH_TICK_PHASES)
  if "$1" --help | grep -q 'laps=1'; then echo "laps=1"; fi
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
  local rounds="${1:-3}" bin out="bench-grid/inspect" settled ticks laps
  bin="$(tick_phases .)"
  laps="$(laps_of "$bin")"
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
        "$bin" "side=$side" "squads=$squads" "at=$ticks" "repeat=$REPEAT" $laps \
          > "$out/s${side}_q${squads}_r${round}_repeat.txt"
      done
    done
  done
  import_records "$out"
}

store_target() { # the host's store target (spec 49: Linux and Windows on x86_64), or nothing
  if [ -n "${PAIR_TARGET:-}" ]; then echo "$PAIR_TARGET"; return; fi
  case "${PAIR_UNAME:-$(uname -sm)}" in
    "Linux x86_64") echo x86_64-unknown-linux-gnu ;;
    MINGW*x86_64 | MSYS*x86_64 | CYGWIN*x86_64) echo x86_64-pc-windows-msvc ;;
  esac
}

fetch_side() { # fetch_side <ref> <target> <dir>: the key's zip unpacked into <dir>, or fails
  local key zip
  key="$(bash ci/native/store_key.sh "$1" release "$2")"
  zip="$3.zip"
  mkdir -p "$3"
  if ! curl -fsSL -o "$zip" "$PAIR_STORE_URL/$key.zip" 2> /dev/null; then
    echo "$key"
    return 1
  fi
  python3 -c 'import sys, zipfile; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])' "$zip" "$3"
  chmod +x "$3"/* 2> /dev/null || true
  rm -f "$zip"
}

fetch_pair() { # fetch_pair <head> <base> <root>: both sides into <root>/<side>/native/bin/,
  # or neither (prints why and returns 1)
  local target missing side ref
  target="$(store_target)"
  if [ -z "$target" ]; then
    echo "pair: no store target for this host ($(uname -sm)); building both"
    return 1
  fi
  for side in head base; do
    ref="$1"; [ "$side" = base ] && ref="$2"
    if ! missing="$(fetch_side "$ref" "$target" "$3/$side/native/bin")"; then
      echo "pair: the $side's key ($missing) isn't in the store; building both"
      rm -rf "$3/head" "$3/base"
      return 1
    fi
    echo "pair: $side ($ref) from the store"
  done
}

pair() {
  local head="$1" base="$2" rounds="${3:-5}" root="bench-pair" out size label ticks tick
  local -A bins=() laps=()
  rm -rf "$root"
  git worktree prune
  mkdir -p "$root/out"
  out="$root/out"
  local built=
  fetch_pair "$head" "$base" "$root" || built=1
  for side in head base; do
    local ref="$head"; [ "$side" = base ] && ref="$base"
    if [ -n "$built" ]; then
      git worktree add -f --detach "$root/$side" "$ref" > /dev/null
      echo "pair: building $side ($ref, $(git -C "$root/$side" rev-parse --short HEAD))"
      bash "$root/$side/native/build.sh" > "$root/build_$side.log"
    fi
    bins[$side]="$(tick_phases "$root/$side")"
    laps[$side]="$(laps_of "${bins[$side]}")"
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
          "${bins[$side]}" "$size" "at=$tick" "repeat=$REPEAT" ${laps[$side]} \
            > "$out/${label}_t${tick}_${side}_r${round}.txt"
        done
      done
    done
  done
  if [ -n "$built" ]; then
    git worktree remove --force "$root/head"
    git worktree remove --force "$root/base"
  fi
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
