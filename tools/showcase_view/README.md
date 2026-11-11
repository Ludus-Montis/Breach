# The Showcase tab

The site's first tab, at `#showcase` and the root URL (spec 44 step 5; Decisions 167, 169
and 170): every PR that changes the game, as [the showcase capture](../../ci/showcase/README.md)
recorded it, newest first.

## What it shows

- **A card a PR,** its latest capture: number and title (linked), its state (`open` marked),
  the commit and the date.
- **A row a scenario,** named as the bench names it: the four stills (first, contact,
  midpoint, last, with their ticks), the clip's player, and that commit's timings for the
  scenario: the tick's median and 95th percentile, its phases' medians under "Phases". Then
  every behaviour: seen (with its first tick), or not seen. One the scenario is named for and
  didn't show is in red.
- **Compare** puts the card beside the capture before it: the PR's own previous capture while
  it is open; once it is closed, merged or not, the capture of the PR merged last before it
  closed (`closed_at`). Scenarios pair by name: the stills side by side, the clips playing
  together, each timing with its change in per cent. A scenario in only one of the two is
  shown alone and says so. No capture before it: no Compare button.
- **Filters:** open, merged, closed unmerged (all on), and one scenario.
- At phone width it is one column, the player full width, the timings under the stills.

## Files

- `showcase_view.js`: the logic, no DOM: the timeline, `partner`, `pairs`, `timingRows`,
  `behaviours`, the filters. Tested with Node by `test/showcase_view.test.js`
  (`node --test "tools/showcase_view/test/*.test.js"`, in the `tools-tests` pre-commit hook).
- `showcase_page.js`, `index.html`, `showcase_view.css`: the drawing.

## Its data

`manifest.json` in the same folder, nothing else. The site build
(`.github/workflows/site-pages.yml`) downloads it from the `showcase-index` pre-release,
because release downloads send no CORS headers; with none there yet, it writes an empty one.
The stills and clips load from their release URLs directly. Each capture dispatches the site
build, so the tab follows within minutes.

To see it from disk: build `_site` as the workflow does, put a `manifest.json` in
`_site/showcase/`, and serve the folder (`python3 -m http.server`).
