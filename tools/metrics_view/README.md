# Tick history page

Draws the bench history under `metrics/` (spec 43 step 7, Decision 162): the whole tick of each
merge to `main` that touched the simulation, per harness, and where its time goes by phase.
Static HTML and JS, no build step, reading only `metrics/`.

## See it

- **On the web:** <https://ludus-montis.github.io/Breach/>, published by the `metrics-pages`
  workflow each time `metrics/` changes on `main` (first, once: Settings → Pages → Source:
  GitHub Actions).
- **From disk:** `python3 tools/metrics_view/build_index.py` writes `records.json` beside the
  page (git-ignored); then open `tools/metrics_view/index.html`. A browser that won't read a
  file beside the page offers a folder picker instead: choose the repo's `metrics` folder.

## What it shows

- **Whole tick, ms (log scale):** one line per harness along `main`'s first-parent line:
  `tick_phases.gd` and `formation_bench.gd` (GDScript era, against `eee3e81`) and
  `breach_tick_phases` (cargo era, against the bridge `8ca4227`). A line breaks where a commit
  has no number (the harness failed there, or didn't run), and the eras are never joined;
  the bridge is marked, with the factor between the two `tick_phases`. Points are fresh
  records. **Show harvested points** adds, hollow, the ones read from old logs, placed at their
  commit or at the `main` commit they were compared with: hidden by default, since they were
  mostly branch comparisons with other args and no shared reference. Hover a point for its
  numbers, click it for the rest.
- **By phase:** a harness's records stacked by the tick's top-level phases (a phase timed more
  finely is summed into its parent), with the reference's whole tick as a grey dash.
- **A point's details:** links to its PR and its record, and a table comparing it with the
  commit before it on its line (a harvested point: with its commit's fresh record) at the
  deepest phase level both timed. A phase only one side timed more finely is named, not
  compared; a phase one side didn't measure is a gap, never 0.

## Files

- `metrics_view.js`: the logic (lines, the bridge, the comparison), tested with Node by
  `test/metrics_view.test.js` (`node --test "tools/metrics_view/test/*.test.js"`).
- `metrics_page.js`, `metrics_view.css`, `index.html`: the drawing.
- `build_index.py`: lists `metrics/` (not `raw/`, not `schema/`) into `records.json`, tested by
  `tools/test_metrics_index.py`.
