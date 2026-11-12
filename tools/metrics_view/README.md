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
  own chart (step 8c), and the harvested 25k runs, which that chart shows. **Show harvested points** adds, hollow, the ones read from old logs, placed at their
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
  and the quadratic share `c·x²/t`. A commit with no 25k set breaks the lines, and so does
  the first set marked `series=settled` (spec 47 step 2b): from there the series runs until the
  field settles, a different measure from the one before. Step 8d adds:
  - **× the bridge:** a toggle that shows each point over the bridge timed beside it in the
    same job (p95 over its p95, p99 over its p99, the estimate over its mean). The goal lines
    are in ms only and hide; a point with no bridge is left out, and a note counts them.
  - **CPU shapes:** each point's mark is its runner's CPU, one shape per CPU, the same shape
    the CPU toggles show beside each name. Unticking a CPU hides its points; the fit and the
    miss still use every record of the commit.
  - **A point's details:** click a point for that commit's fitted curve over size, in ms, with
    its grid points and the measured 24,576, and the fit's `b`, `c` and miss.
  - **Show harvested points** adds the harvested 25k runs (`preset=25k`, or 24,576 a side),
    hollow and never fitted, at their commit's or base's place; the columns then start at the
    first of them. They are no longer on the 4,096 chart.
- **On track (the whole-tick chart, in ms):** for each commit with a 25k set, the dotted lines
  are the 4-lane tick at which its lanes fit predicts Decision 159's 50 ms (target) and 100 ms
  (interim) at 24 lanes, `goal × (b·4096 + c·4096²) / (b·24576 + c·24576²)`, drawn against the
  25k set's measured 4-lane point (a series from contact, so not the `breach_tick_phases`
  line's battle). They break at a commit with no 25k set, and show in ms only: the 4-lane grid
  shape is timed without the bridge, so it has no ratio.
- **By phase:** a harness's records stacked by the tick's top-level phases (a phase timed more
  finely is summed into its parent). The eight costliest phases over the harness's ten newest
  records get a colour each, so a phase no longer timed (a retired name) gives up its colour.
  Two more parts close each bar: **other phases**, the timed phases without a colour (its
  tooltip lists each, in ms), and **untimed**, the tick less every timed phase (work outside
  any lap, 0 to 2% of a `breach_tick_phases` tick; all of a harness that times only the tick).
  In ms the reference's whole tick is a grey dash per record,
  in × the reference one grey line at 1, in % of tick none. Click a phase in the legend, "other
  phases" and "untimed" included, to hide or show it in every unit; the stack closes up and no
  other phase's value or share changes.
- **A tick replayed (spec 47 step 5b, Decision 177):** small multiples, one per top-level phase
  and size (4,096 and 24,576 a side), along `main`'s first-parent line from the first commit
  with repeat records (`metrics/inspect/`, series `inspect`, written by `bench-inspect` at each
  merge, step 5a). Each inspection tick's repeats are dots; the same phase's lap from the whole
  tick is the line: the 25k set's record of the same commit and battle (`side=4096 squads=4`,
  `side=24576 squads=24`, the settled series' mean tick). A panel's title gives the gap, the
  repeats' median over the lap at its latest commit with both. **% of tick** is the default
  (the owner, 2026-11-11): each side over its own tick, so each record's runner cancels. In
  **× the reference** the two sides have different yardsticks (a repeat's is the bridge on
  `bench`'s battle, `side=4096 squads=4 fight=3`; a set record's the bridge on its own shape),
  so read each side's trend there, not the gap; **ms** holds the runners as well. ⚠ marks a
  panel holding a phase the repeats under-report, whose cost builds up across ticks (`strays`,
  `groups`, `bodies: pushes`, `stamina`; Q47, #909): there the whole tick is the judge. The
  CPU toggles apply.
- **A point's details:** links to its PR and its record, and a table, in the phase chart's
  unit, comparing it with the commit before it on its line among the CPUs shown (the title
  names both records' CPUs) (a harvested point: with its commit's fresh record) at the
  deepest phase level both timed. A phase only one side timed more finely is named, not
  compared; a phase one side didn't measure is a gap, never 0.
- **The owner's PC (local runs, spec 48 step 2, Decision 178):** a section below CI's, shown
  when the bundle holds records with `series` `local` (`metrics/local/`). It is headed by the
  machine the records name (CPU, cores, RAM, OS) and the run's commit and date, and is in raw
  ms on log scales. A run is the records of one date and commit; a picker lists them when
  there are several, newest first, and the run before the picked one is drawn dotted.
  - **Two groupings:** the tick (or a phase) against units a side, a curve per squad count; or
    against squad size, a curve per total. **Per squad** divides each value by the cell's
    squad count, so a phase whose cost is the square of a squad's size draws every total on
    one line of slope 2.
  - **Each point** is the cell's median, with a bar for its rounds' range.
  - **The phase picker:** the whole tick (default) or any phase the run times. With the tick,
    the three costliest phases at the largest cell are drawn thin.
  - **Decision 159's lines:** 50 ms target and 100 ms interim, at 24,576 a side on CI's
    4-thread runner, a guide here; drawn with the whole tick only, not per squad.
  - **The repeats (spec 48 step 5c):** a cell's repeat records (harness `tick_phases-repeat`,
    from `local_grid.sh inspect`) are hollow points just left of the cell's point, one per tick
    they replay, each with a bar; hovering names the tick and the repeat count. The bar spans
    the rounds' quartiles, the lowest `q1` to the highest `q3`, where the record keeps them
    (step 5e), and the rounds' medians where it doesn't; the hover text says which. They
    are not cells of the series. Beside them the legend gives Q47's caveat (#909): the repeats
    under-report phases whose cost builds up across ticks (strays, groups, bodies: pushes,
    stamina), and the whole tick is the judge.
  - CI's charts never draw a local record: their filters keep `series` `bench`.

## Files

- `metrics_view.js`: the logic (lines, the bridge, the comparison, the units, the phase stack
  and the CPU filter), tested with Node by
  `test/metrics_view.test.js` (`node --test "tools/metrics_view/test/*.test.js"`).
- `metrics_25k.js`: the 25k chart's logic (the fit, the miss, the lines, × the bridge, the CPU marks,
  the harvested 25k runs and the 4,096 chart's "on track" lines), tested by
  `test/metrics_25k.test.js` against the fixtures `ci/metrics/bench_set.py` uses
  (`ci/metrics/fixtures/bench_fit.json`).
- `metrics_inspect.js`: the repeats' panels (the pairing with the 25k set, the units, the
  gap and Q47's caveat), tested by `test/metrics_inspect.test.js`.
- `metrics_local.js`: the local section's logic (runs, curves by squad count or squad size,
  per squad, the phases, the costliest three and the repeats' points), tested by `test/metrics_local.test.js` on
  the owner's 2026-11-11 grid under `metrics/local/`.
- `metrics_page.js`, `metrics_25k_page.js`, `metrics_inspect_page.js`, `metrics_local_page.js`,
  `metrics_view.css`, `index.html`: the drawing.
- `build_index.py`: lists `metrics/` (not `raw/`, not `schema/`) into `records.json`, tested by
  `tools/test_metrics_index.py`.
