#!/usr/bin/env bash
# Traces the first runs whose outcomes changed (specs/37-outcome-trace.md, step 3): each on
# the base's worktree and on the branch, from 20 ticks before its first difference to 40
# after, then diffed with trace_diff.py. Run by sim-checks' `digests` job when
# check_outcomes.py --changed wrote any run.
#
#   trace_changed.sh <changed file> <base worktree> <out dir> [most runs, default 3]
#
# Writes <out dir>/<run>.base.jsonl, .head.jsonl, .diff.txt and .diff.json per run, spaces in
# the run's label as underscores. Each side traces with its own core's digest,
# native/bin/breach_digest (Decision 151: outcomes are guarded on Rust); a base whose digest
# can't trace (before spec 39 step 9d-iii, when the trace was the GDScript digest's) says so in
# the run's .diff.txt. Never fails: the trace shows a change, check_outcomes judges it.
set -uo pipefail

changed="$1"
base="$2"
out="$3"
most="${4:-3}"
head_dir="$(pwd)"
mkdir -p "$out"

# The run's trace from `dir`'s own digest (the digests job gives the base's worktree its
# core). A digest that doesn't know trace= refuses it and writes nothing.
trace() {
	local dir="$1" run="$2" from="$3" to="$4" file="$5"
	(cd "$dir" && native/bin/breach_digest "only=$run" "trace=$file" \
		"from=$from" "to=$to" > /dev/null 2>&1)
}

head -n "$most" "$changed" | while IFS=$'\t' read -r run tick; do
	[ -n "$run" ] || continue
	name="${run// /_}"
	from=$((tick > 20 ? tick - 20 : 1))
	to=$((tick + 40))
	trace "$head_dir" "$run" "$from" "$to" "$head_dir/$out/$name.head.jsonl"
	trace "$base" "$run" "$from" "$to" "$head_dir/$out/$name.base.jsonl"
	if [ ! -s "$out/$name.base.jsonl" ]; then
		echo "$run: the base's digest can't trace (before spec 39 step 9d-iii)" > "$out/$name.diff.txt"
		continue
	fi
	python3 ci/godot/scripts/trace_diff.py "$out/$name.base.jsonl" "$out/$name.head.jsonl" \
		--json "$out/$name.diff.json" > "$out/$name.diff.txt"
	sed -i "1s/^/$run: /" "$out/$name.diff.txt"
done
exit 0
