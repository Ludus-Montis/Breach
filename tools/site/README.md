# The project site

The shell of <https://ludus-montis.github.io/Breach/> (Decision 166, spec 44 step 4): a tab
bar, one tab a page, routed by `#name`. The root URL and any unknown name open the first tab,
Showcase (Decision 170). Old links to the tick history page land on Showcase; it is at
`#metrics`.

## Tabs

| Tab | Route | Its folder | Its data |
|---|---|---|---|
| Showcase | `#showcase` (and the root) | [`tools/showcase_view/`](../showcase_view/README.md) | `manifest.json`, downloaded from the `showcase-index` pre-release |
| Metrics | `#metrics` | [`tools/metrics_view/`](../metrics_view/README.md) | `records.json`, built from `metrics/` |

Each tab is a self-contained folder: its own HTML, JS, CSS and data, never another tab's. The
shell loads a tab's page in its own frame the first time the tab is shown, and keeps it while
you switch away. A tab's links open over the whole site (`<base target="_top">`), not inside
the frame.

## Adding a tab

1. Its folder, with an `index.html` that works on its own.
2. A line in `TABS` in `site.js`: `{ name, title, page: '<name>/index.html' }`.
3. A step in `.github/workflows/site-pages.yml` copying the folder to `_site/<name>/` and
   building its data there, and the folder's path under the workflow's `paths`.

## Files

- `index.html`, `site.css`: the bar and the frames.
- `site.js`: the list of tabs and `route(hash, tabs)`, tested with Node by
  `test/site.test.js` (`node --test "tools/site/test/*.test.js"`, in the `tools-tests`
  pre-commit hook), and the bar's drawing.

## Built and published

`.github/workflows/site-pages.yml` builds `_site` (the shell at the root, each tab in its
folder) and deploys it, on each push to `main` that changes a tab's data or code, and on
`workflow_dispatch`, which the showcase capture starts after each publish
(`ci/showcase/index.sh`). One deploy at a time. To see it from disk, copy the files as the
workflow does and serve the folder (`python3 -m http.server`): a frame won't load from
`file://` in every browser.
