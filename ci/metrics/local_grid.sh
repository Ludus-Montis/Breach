#!/usr/bin/env bash
# The local grid (spec 48 step 1, Decision 178): squad size against squad count on the
# owner's PC. Runs `breach_tick_phases side=<s> squads=<q> fight=10` for every cell, the
# rounds taken in turn (round 1 of every cell, then round 2, ...), into bench-grid/
# (git-ignored), then imports them as local records under metrics/local/.
#
#   bash ci/metrics/local_grid.sh [rounds]     (default 3; build first: bash native/build.sh)
#
# A new grid shape is a change to SIDES or SQUADS below.
set -euo pipefail

rounds="${1:-3}"
here="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$here"

SIDES=(3072 6144 12288 24576)
SQUADS=(1 2 4 8 24)
FIGHT=10

bin="native/bin/breach_tick_phases"
[ -x "$bin" ] || bin="$bin.exe"
if [ ! -x "$bin" ]; then
  echo "local_grid: no $bin; build it first: bash native/build.sh" >&2
  exit 2
fi

out="bench-grid"
rm -rf "$out"
mkdir -p "$out"
for round in $(seq 1 "$rounds"); do
  for side in "${SIDES[@]}"; do
    for squads in "${SQUADS[@]}"; do
      echo "round $round: side=$side squads=$squads"
      "$bin" "side=$side" "squads=$squads" "fight=$FIGHT" > "$out/s${side}_q${squads}_r${round}.txt"
    done
  done
done

sha="$(git rev-parse HEAD)"
date="$(date -u +%Y-%m-%d)"
python3 ci/metrics/local_record.py "$out" --commit "$sha" --date "$date" --fight "$FIGHT"
echo "local_grid: keep the outputs with: cp -r $out metrics/raw/local/$date-${sha:0:7}"
