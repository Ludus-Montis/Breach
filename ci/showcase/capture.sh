#!/usr/bin/env bash
# The showcase capture (spec 44 step 3, Decision 169): every scenario on the list
# (content/scenarios/scenarios.json) played on the 2D field under Movie Maker, encoded, cut
# into stills and timed. Run from the repo root by .github/workflows/showcase-capture.yml,
# after the crate is built (native/bin/) and Godot is at $GODOT.
#
# Usage: ci/showcase/capture.sh <pr> <sha> <release> <run-url> <out-dir>
# Writes into <out-dir>: assets/ (pr<N>-<sha7>-<scenario>.webm, four -<moment>.webp stills
# each) and capture.json (the capture's entry in the PR's record).
set -euo pipefail

pr=$1 sha=$2 release=$3 run_url=$4 out=$5
here=$(cd "$(dirname "$0")" && pwd)
GODOT=${GODOT:-/opt/godot/godot}
SCENE=res://presentation/skirmish/formation_2d/formation_field.tscn
FPS=30
MAX_FRAMES=2730          # 90 s of play at 30 fps, and a little over
MAX_BYTES=$((5 * 1024 * 1024))
base="https://github.com/${GITHUB_REPOSITORY:-Ludus-Montis/Breach}/releases/download/$release"
mkdir -p "$out/assets" "$out/work" "$out/entries"
work=$out/work

names=$(python3 -c 'import json; print("\n".join(s["name"] for s in json.load(open(
  "content/scenarios/scenarios.json"))["scenarios"]))')

i=0
for name in $names; do
  i=$((i + 1))
  frames_dir="$work/$name"
  mkdir -p "$frames_dir"
  prefix=$(python3 "$here/entry.py" prefix "$pr" "$sha" "$name")
  echo "== $name: rendering"
  BREACH_COMMIT=$sha xvfb-run -a -s "-screen 0 1280x720x24" \
    "$GODOT" --path . --resolution 1280x720 --fixed-fps "$FPS" --quit-after "$MAX_FRAMES" \
      --write-movie "$frames_dir/frame.png" "$SCENE" -- --capture="$name" \
      > "$work/$name.log" 2>&1 || { tail -n 30 "$work/$name.log"; exit 1; }
  [ -f "$frames_dir/capture.json" ] || { echo "$name wrote no capture.json"; tail -n 30 "$work/$name.log"; exit 1; }
  frames=$(find "$frames_dir" -name 'frame*.png' | wc -l)
  echo "== $name: $frames frames, encoding"
  clip="$out/assets/$prefix.webm"
  ffmpeg -loglevel error -y -framerate "$FPS" -i "$frames_dir/frame%08d.png" \
    -c:v libvpx-vp9 -b:v 0 -crf 36 -row-mt 1 -pix_fmt yuv420p "$clip"
  if [ "$(stat -c %s "$clip")" -gt "$MAX_BYTES" ]; then
    rate=$((MAX_BYTES * 8 * 9 / 10 * FPS / frames))
    ffmpeg -loglevel error -y -framerate "$FPS" -i "$frames_dir/frame%08d.png" \
      -c:v libvpx-vp9 -b:v "$rate" -maxrate "$rate" -row-mt 1 -pix_fmt yuv420p "$clip"
  fi
  python3 "$here/entry.py" stills "$frames_dir/capture.json" "$frames" | while read -r moment frame; do
    ffmpeg -loglevel error -y -i "$(printf '%s/frame%08d.png' "$frames_dir" "$frame")" \
      -c:v libwebp -quality 85 "$out/assets/$prefix-$moment.webp"
  done
  cp "$frames_dir/capture.json" "$work/$name.capture.json"
  rm -rf "$frames_dir"
  echo "$i $name $prefix" >> "$work/list.txt"
done

# The timings come after every movie, so the renders don't share the runner with them.
while read -r i name prefix; do
  echo "== $name: timing, 5 rounds"
  for round in 1 2 3 4 5; do
    native/bin/breach_tick_phases scenario="$name" > "$work/$name.t$round.txt"
  done
  python3 "$here/timings.py" "$work/$name.timings.json" "$work/$name".t*.txt
  python3 "$here/entry.py" entry "$work/$name.capture.json" "$base" "$prefix" \
    "$work/$name.timings.json" > "$out/entries/$(printf '%02d' "$i")-$name.json"
done < "$work/list.txt"

jq -s --arg commit "$sha" --arg date "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg release "$release" --arg run "$run_url" \
  '{commit: $commit, date: $date, release: $release, run: $run, scenarios: .}' \
  "$out"/entries/*.json > "$out/capture.json"
ls -l "$out/assets"
