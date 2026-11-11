# Tick history page

Draws the bench history under `metrics/` (spec 43 steps 7 and 9, Decisions 162 and 172): the
whole tick of each merge to `main` that touched the simulation, per harness, and where its time
goes by phase, by default as × the reference so the runner's speed cancels out.
Static HTML and JS, no build step, reading only `metrics/`.

## See it

- **On the web:** the Metrics tab of the project site, <https://ludus-montis.github.io/Breach/#metrics>
  (`#metrics`, and the root; [`tools/site/`](../site/README.md)), published by the `site-pages`
  workflow each time `metrics/` or the page changes on `main` (first, once: Settings → Pages →
  Source: GitHub Actions). The page is served from the site's `metrics/` folder and opens in
  the shell's frame; its links open over the whole site.
- **From disk:** `python3 tools/metrics_view/build_index.py` writes `records.json` beside the
  page (git-ignored); then open `tools/metrics_view/index.html`. A browser that won't read a
  file beside the page offers a folder picker instead: choose the repo's `metrics` folder.

## What it shows

The page opens with **How to read this**: what fresh and harvested records, the reference, the
bridge, the battle measured (4,096 a side for `tick_phases`, 1,024 for `formation_bench`; the
25k set has its own chart) and a gap mean.

- **CPUs:** a row of checkboxes under the header, one per runner CPU the records name
  (`machine.cpu`, "unknown" without one), all on. Untick one to take its records out of every
  chart and the compare; the whole-tick line joins the points left.
- **Units:** the whole-tick chart is in **× the reference** (default: a record's tick over the
  reference's tick timed in the same job) or **ms**, on a log scale in both. The phase chart is
  in × the reference (default), ms, or **% of tick**. A record with no reference is left out in
  × the reference, and a note under the chart counts them. Tooltips give a point's CPU, its ms
  and its ratio.
- **Whole tick (log scale):** one line per harness along `main`'s first-parent line:
  `tick_phases.gd` and `formation_bench.gd` (GDScript era, against `eee3e81`) and
  `breach_tick_phases` (cargo era, against the bridge `8ca4227`). A line breaks where a commit
  has no number (the harness failed there, or didn't run), and the eras are never joined;
  the bridge is marked, with the factor between the two `tick_phases`. Points are fresh
  records, apart from the 25k set's (`series=1`, spec 43 step 8a), which are left for their
  own chart (step 8c). **Show harvested points** adds, hollow, the ones read from old logs, placed at their
  commit or at the `main` commit they were compared with: hidden by default, since they were
  mostly branch comparisons with other args and no shared reference. Hover a point for its
  numbers, click it for the rest.
- **24,576 (the 25k set, spec 43 step 8c, Decision 165):** below the whole tick, in ms on a log
  scale, from the first commit with a 25k set (`series=1` records). Sizes are a side, mirrored.
  One line per shape, 24 lanes of 1,024 and one squad of 24,576: the p95 as the line, the p99
  as a thin band above it. Dotted, each shape's estimate from that commit's grid: the
  non-negative fit of `b·x + c·x²` on the smaller sizes' mean ticks, taken at 24,576. A merge
  whose estimate misses the measured mean by more than 10% is ringed. Decision 159's goals are
  the dashed lines at 50 ms (target) and 100 ms (interim). Hover a point for its CPU, the miss
  and the quadratic share `c·x²/t`. A commit with no 25k set breaks the lines. Still to come
  (step 8d): × the bridge, the 4,096 chart's "on track" lines, a point's fitted curve, CPU
  shapes and the harvested 25k runs.
- **By phase:** a harness's records stacked by the tick's top-level phases (a phase timed more
  finely is summed into its parent). In ms the reference's whole tick is a grey dash per record,
  in × the reference one grey line at 1, in % of tick none. Click a phase in the legend, "other"
  included, to hide or show it in every unit; the stack closes up and no other phase's value
  or share changes.
- **A point's details:** links to its PR and its record, and a table, in the phase chart's
  unit, comparing it with the commit before it on its line among the CPUs shown (the title
  names both records' CPUs) (a harvested point: with its commit's fresh record) at the
  deepest phase level both timed. A phase only one side timed more finely is named, not
  compared; a phase one side didn't measure is a gap, never 0.

## Files

- `metrics_view.js`: the logic (lines, the bridge, the comparison, the units, the phase stack
  and the CPU filter), tested with Node by
  `test/metrics_view.test.js` (`node --test "tools/metrics_view/test/*.test.js"`).
- `metrics_25k.js`: the 25k chart's logic (the fit, the miss, the lines), tested by
  `test/metrics_25k.test.js` against the fixtures `ci/metrics/bench_set.py` uses
  (`ci/metrics/fixtures/bench_fit.json`).
- `metrics_page.js`, `metrics_25k_page.js`, `metrics_view.css`, `index.html`: the drawing.
- `build_index.py`: lists `metrics/` (not `raw/`, not `schema/`) into `records.json`, tested by
  `tools/test_metrics_index.py`.
