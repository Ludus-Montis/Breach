# Spec 48 step 4: does the owner's PC track CI? (2026-11-12)

Four commit pairs, each timed on the owner's PC (i7-12700K, 20 threads, 32 GB, Windows) with
the whole tick, `breach_tick_phases preset=<size> fight=10`, head and base in turn for 5 rounds
(head first in odd rounds), each commit built from its own worktree with `native/build.sh`.
`summary.txt` is `ci/godot/scripts/bench_compare.py --head head_*.txt --base base_*.txt`.

The repeat mode (`local_grid.sh pair`) couldn't be used: every commit here predates it, so
the base prints no `inspect:` ticks. Each folder is `<head>-vs-<base>-<size>`. In
`bfce23c-vs-5ea4c29-25k` the head is the older commit: CI's change is from `bfce23c` to
`5ea4c29`, so its percentages read inverted. The verdict is in `specs/48-local-bench-series.md`,
`## Measured`.
