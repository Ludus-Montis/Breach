# Outcome trace page

Plays a run's base and head traces side by side, so an outcome change can be seen and
signed off (`specs/37-outcome-trace.md`, step 4). Static HTML and JS, no build, no server.

## See an outcome change

1. On a PR whose digests changed, open the `sim-checks` run and download the
   `sim-digests-<set>` artifact for each set that changed. Its `traces/` folder holds, for up
   to 3 runs, `<run>.base.jsonl`, `<run>.head.jsonl`, `<run>.diff.txt` and `<run>.diff.json`.
2. Open `tools/trace_view/index.html` in a browser, straight from disk.
3. Load the run's base trace, head trace and diff JSON.

The page opens at the first difference. Each unit is a dot with a tick for its facing,
coloured by its squad's faction; a hollow dot is down. The units the diff names at a tick are
ringed in red. Click a unit to see its fields on both sides, with the differing ones marked.
The scrubber, Play and the step buttons move both sides together.

To trace a run yourself:

    godot --headless --path . --script res://tools/formation_digest.gd -- \
        "only=mirror_headon s1 rtrue" trace=base.jsonl from=100 to=160
    python3 ci/godot/scripts/trace_diff.py base.jsonl head.jsonl --json diff.json

## Files

- `trace_view.js`: loading, indexing by tick and which units to ring; tested with Node by
  `test/trace_view.test.js` (`node --test "tools/trace_view/test/*.test.js"`).
- `trace_page.js`: the canvases and controls.
