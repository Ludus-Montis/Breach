---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 44: The project site and its Showcase tab

## Purpose

The owner asked for visual progress to be visible on every PR: a CI job that runs the
skirmish and shows its frames on the PR, so a PR shows the game and not only test counts,
and an agent can critique the frames. The captures are also to be seen on the Pages site,
as a tab beside the metrics, newest first, with the game viewable there. The Pages site
becomes the project's site, Metrics and Showcase its first two tabs.

**Settled by Decisions 166** (the site: one shell, one folder a tab, `#name` routes) and
**167** (the capture: what, which PRs, where it is kept, what is kept, where it shows).
Nothing here changes a simulation outcome.

## Status

Agreed in the design session of 2026-11-08. Six steps, each its own PR, built with
`/run-phase` as "spec 44 step N".
- **Step 1 is a go/no-go spike.** If it fails, stop and raise a `question` issue: the
  Decisions assume the real game renders deterministically in CI.
- Steps 2 and 3 follow in order. Step 4 may come any time; step 5 needs 3 and 4.
- **Step 6 is optional.** If the owner doesn't run it, it moves to its own spec.

## Pre-mortem (asked at the start of the round)

- **Riskiest assumption:** that Godot 4.7.2 renders the real skirmish on a GPU-less runner
  (xvfb, Mesa lavapipe for Forward+, or the OpenGL fallback), with the Rust extension loaded,
  in minutes, and the same frames on every run. Cheapest test: step 1.
- **Second:** that a release asset plays in a `<video>` on the Pages site and shows inline in
  a PR comment. Also step 1.
- **Regrets guarded against:** storage growth (release assets, not git); noisy comparisons
  (Movie Maker's fixed rate, a fixed seed, checked in step 1); a capture path that drifts
  from the game (the real scene, `--capture`); a clip that says nothing (fixed moments);
  CI minutes and PR noise (game paths only, cancel on push, one comment edited in place);
  breaking today's metrics URL (the root opens Metrics).

## Steps

### Step 1, the spike

A hand-started workflow (`.github/workflows/showcase-spike.yml`), deleted by step 3.
- Installs Godot 4.7.2 and xvfb with Mesa, builds the crate, imports the project, and runs
  the skirmish scene with `--write-movie` for 300 frames at 30 fps, twice.
- **Records under "Measured":** the renderer and driver Godot reports; the wall time of the
  import, the build and each run; whether the two runs' PNG frames are byte-identical (if
  not, the largest per-pixel difference and where); one frame, attached to the PR.
- **Playback check:** uploads a short WebM and one WebP still to a test pre-release
  `showcase-spike`. Records whether the WebM plays in a `<video>` on a GitHub Pages page in
  Chrome and Firefox, and whether the still shows inline in a PR comment. Deletes the
  pre-release after.
- **Go:** the scene renders as it does on a desktop (the owner judges the frame), a run
  takes under 5 minutes, and frames are identical or within a tolerance recorded here.
  **Fallbacks if playback fails:** the site copies the last 100 clips into its own build.

### Step 2, `--capture` on the skirmish scene

The real skirmish scene, started with `-- --capture`, plays the fixed scenario hands-free and
quits.
- **The scenario:** one map from `content/` and one wave, seed 1, the scene's default camera.
  The step chooses the map whose wave reaches first contact within 5 seconds and ends within
  30 seconds of sim time, and records the choice here. The wave ends when one side is gone
  or at a tick cap of 30 seconds.
- **The moments:** it writes `capture.json` beside the movie: the ticks of the first tick,
  first contact (the first tick with damage), the midpoint and the last tick, the tick
  count, the map, the wave, the seed and the commit.
- No other behaviour of the scene changes without `--capture`.

### Step 3, the `showcase-capture` workflow

`.github/workflows/showcase-capture.yml`, on `pull_request` (opened, synchronize, reopened,
closed).
- **Which PRs:** `ci/showcase/is_game_change.py` (Decision 167's paths). A PR from a fork is
  skipped and says so in the job summary.
- **Capture:** xvfb, `--write-movie` with step 2's scenario at 960×540 and 30 fps; ffmpeg
  makes `pr<N>-<sha7>.webm` (VP9, at most 5 MB) and four `pr<N>-<sha7>-<moment>.webp`
  stills from the recorded ticks.
- **Publish:** uploads them to the pre-release `showcase-YYYY-MM` (created if missing), and
  updates `manifest.json` on the pre-release `showcase-index`: per capture the PR, its title,
  state, commit, date, asset URLs, moments and tick count.
- **Retention:** an open PR keeps its last two captures; on `closed`, merged or not, only
  the last is kept (no new capture is made on close).
- **The PR comment:** one comment, found by a hidden marker and edited in place, with the
  stills, the clip's link and a link to `#showcase`. Ends with the attribution footer.
- **Advisory:** a failed capture edits the comment to say so and links the run; the job
  doesn't fail the PR's checks.
- **Concurrency:** one group per PR, a new push cancelling the old run. After publishing, it
  dispatches the site build.

### Step 4, the site shell

Decision 166's shell, with the tick history page moved in unchanged.
- `tools/site/index.html`, `site.js`, `site.css`: a tab bar from one list of tabs, routing
  by `#name`, the root and unknown names opening Metrics.
- The tick history page loads inside its tab from its own folder (`tools/metrics_view/`),
  each tab's page loaded on first view.
- `metrics-pages.yml` becomes `site-pages.yml`: it builds every tab's data, on pushes to
  `main` that change a tab's data or code, on `workflow_dispatch` (the capture's
  dispatch), and as before one deploy at a time.
- `tools/metrics_view/README.md` and `docs/` say where the site and its tabs live.

### Step 5, the Showcase tab

`tools/showcase_view/`, built from `manifest.json` by the site workflow.
- **Timeline, newest first:** a card for each PR's latest capture: number and title (linked),
  state (open captures marked), commit, date, the four stills, and a player that opens the
  clip.
- **Compare:** a toggle puts a capture beside the one before: the PR's previous capture while
  it is open, the previous merged PR's once merged. Stills side by side at the same moments,
  clips started together.
- **Filters:** open, merged, closed unmerged; all on by default.
- Phone width works: one column, the player full width.

### Step 6, an agent's critique (optional)

After a capture, an agent reviews the four stills against the comparison's (step 5's rule)
and appends a short critique to the PR comment: what changed, what reads poorly, what looks
broken. Advisory only. Needs the owner to add an API key secret; the step says which. If not
run, this step moves to its own spec.

## Not in this spec

- **A replay viewer of the simulation's state** (Q41, #828): smaller than clips, but it draws
  the simulation, not the game.
- **A playable web build** on the site.
- **More scenarios** per capture (archers, forts, retreats): a later spec, using step 2's
  `--capture` with a scenario argument.
- **Further tabs** (Q38's series, Gate 1, the designer): each a folder, per Decision 166.

## Measured

### Step 1, the spike

`.github/workflows/showcase-spike.yml` runs on its own PR and writes these into its job
summary and its PR comment (marked `<!-- showcase-spike -->`). Copy them here once the run
is in, with the owner's judgement of the frame and the go or no-go.
- The playback check stands in for Pages with a page served over http on the runner: a
  `<video>` without `crossorigin` loads a cross-origin source the same from either origin.
  The owner confirms on the real site in step 5.
- The test pre-release `showcase-spike` is deleted when the spike's PR closes, so the PR
  comment's still shows until then.

First run (commit ee35018, `formation_skirmish.tscn`, 300 frames at 30 fps):
- **Renderer:** `Vulkan 1.4.335 - Forward+ - Using Device #0: Unknown - llvmpipe (LLVM 21.1.8,
  256 bits)`: Forward+ on Mesa's software Vulkan, no fallback needed.
- **Wall time:** crate build 211 s (the longest part), import 6 s, runs 13 s and 11 s: about
  1.2 s of wall time per second of play.
- **Frames:** both runs byte-identical, all 300 frames.
- **Size:** frames came out 1152×648, not the 960×540 asked for: the window's stretch
  setting scales them. Step 3 sets the size it needs.
- **Clip:** VP9 WebM, 119 KB for 10 s.
- **Playback:** the first check timed out in both browsers because it listened for
  `canplay` after the clip was already ready. Fixed; checked again by hand in Chromium, the
  WebM plays (10.0 s) and the WebP shows in an `<img>`. Release assets are served as
  `application/octet-stream`, which browsers play and show anyway.
- **Owner's view of the frame:** the scene captured is the wrong one to show. The lane feel
  test's camera fits the whole map, too small to read, and in 10 s with no orders nothing
  reaches a fight. The owner is most interested in what the 2D field shows: fording,
  flanking, engaging, breaking off, cornering and routing. The capture's scenario, its tie
  to the bench's scenarios, and the field as a stand-in for Q25's game-like map (#280) go
  to a design session before step 2.

## Scenarios

### Step 1
- Given the spike runs twice on one commit, then it records the renderer, the times and
  whether the frames are identical, and the owner judges one frame.

### Step 2
- Given `--capture`, when the scene starts, then it plays the fixed wave without input,
  writes `capture.json` with four moments in order, and quits.
- Given the same commit twice, then `capture.json` is identical.
- Given no `--capture`, then the scene behaves as before.

### Step 3
- Given a PR touching only `docs/`, `specs/`, `tests/`, `ci/` or `metrics/`, then no capture runs.
- Given a PR touching `presentation/` or `native/`, then a capture is published and the PR has
  one comment with four stills.
- Given a second push, then the comment is edited, not added, and the PR keeps two captures.
- Given the PR closes, then only its last capture remains.
- Given the capture fails, then the comment says so and the PR's checks stay as they were.

### Step 4
- Given the root URL or `#unknown`, then Metrics opens; given `#showcase`, then Showcase opens.
- Given the tick history page, then it draws as it did before the move.

### Step 5
- Given a manifest, then cards are newest first and open captures are marked.
- Given compare on an open PR with two captures, then its two captures show side by side;
  on a merged PR, then the previous merged PR's capture is beside it.

### Step 6
- Given a capture, then the PR comment gains a critique, and nothing else changes.

## Test-first order

1. Step 1: no test; the spike is checked by its run, recorded under "Measured".
2. Step 2: red on `capture.json`'s moments from a short fixed run (gdUnit, `-a` the one file),
   and on the scene unchanged without the argument; then `--capture`.
3. Step 3: red on `is_game_change.py` against fixture path lists, and on the retention rule
   and the manifest update (Python, fixture manifests); then the workflow, checked by a run.
4. Step 4: red on the routing (JS logic, tested as `trace_view.js` is); then the shell.
5. Step 5: red on the timeline order and the compare pairing from a fixture manifest (JS
   logic); then the tab.
6. Step 6: red on the comment update keeping the stills and adding the critique (Python);
   then the job.

## Rubric answers

- **single-noun-phrase:** the spike; the capture argument; the capture workflow; the site
  shell; the Showcase tab; the critique.
- **ocp-extension-point:** a new page is a folder and a line in the shell's list; a new
  scenario is a `--capture` argument value.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the Showcase tab reads only `manifest.json`; the workflow only the capture's
  output.
- **dip-direction:** the tabs depend on their built data, never on how it is stored.
- **general-over-special:** general: the files changed decide capture; one scenario at the
  same moments; one retention rule; every page a tab (Decisions 166, 167).
- **order-independent:** not applicable; no simulation outcome changes.
- **bounded-work:** not applicable to the tick; one capture job per push to a game PR,
  cancelled by the next.
