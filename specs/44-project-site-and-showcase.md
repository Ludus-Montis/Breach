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
**167** (the capture: what, which PRs, where it is kept, what is kept, where it shows), and
**169**, which amends 167's "what is captured": named scenarios on the 2D field, sharing
their names with the bench, each up to 90 s. Nothing here changes a simulation outcome.

## Status

Agreed in the design session of 2026-11-08. Steps 2, 3 and 5 were rewritten after the
spike, in a second design session the same day (Decision 169). Six steps, each its own PR,
built with `/run-phase` as "spec 44 step N".
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

### Round 2, the capture's scenario (after the spike)

- **Regret in a month:** the clips no longer show what their names promise. An order log
  replays by tick: `retreat B` at tick 318 assumes contact came first. A change that moves
  contact later turns "breaking off" into a wave that walks up and turns round, and nobody
  notices, because the capture is advisory.
- **Riskiest assumption:** that a fixed order log keeps showing the behaviours it was
  recorded for (fording, flanking, engaging, breaking off, cornering, routing) as the
  simulation changes.
- **Cheapest test:** step 2 opens with it. It replays the digest's five field logs
  (`native/rust/src/digest/runs.rs`, `FIELD_LOGS`, owner-played feel-test logs from earlier
  builds) headless on today's `main` (`field_play::actions::replay`), and records under
  "Measured" which of the behaviours each was played to show still happen. After that, every
  capture checks its scenarios the same way (Decision 169: a behaviour not seen is reported).

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

### Step 2, the scenarios and `--capture` on the 2D field

Decision 169. The 2D field (`presentation/skirmish/formation_2d/formation_field.tscn`),
started with `-- --capture=<name>`, plays one named scenario hands-free and quits. The field
stands in until Q25's game-like map (#280) can play a scenario.
- **First, the pre-mortem's test:** replay the digest's five field logs headless on `main`
  and record under "Measured" which behaviours each still shows. A log that still shows its
  behaviours can seed a scenario's draft.
- **The list:** `content/scenarios/`, one record file a scenario (`<name>.record`, the
  FormationRecord text, Decision 115, whose header carries the seed) and one
  `scenarios.json` naming each scenario's map (`field` for now), its record and the
  behaviours it must show. It is a list of fixtures, not game content: nothing in the shipped
  game reads it, so neither the content schema nor the designer changes. It sits under
  `content/` so that a changed scenario is a game change (Decision 167's paths) and is
  captured again.
- **The first scenarios:** two or three, drafted by this step, named `field_<what it
  shows>` and covering the six behaviours between them: fording, flanking, engaging,
  breaking off, cornering and routing. Each reaches first contact within 15 s of play and
  ends within 90 s. The orders are the field's own: send, send together, wait in the wood,
  route via C, retreat, tend, hurry, and the kingdom's pursue. The owner approves them by
  watching step 3's captures, and may replace a record with one played and copied out of
  the field.
- **The behaviours a scenario shows** are read from the replay. Most come from the event
  log (`events.rs`: `engaged`, `flanked`, `disengaged`/`withdrawing`, a morale band's rout,
  `fled_home`). Behaviours that no event marks are read from where units stand: fording is a
  unit on a `ford` cell. The step settles how cornering is read and records it here.
- **The scenario's end:** when one side is gone, every player squad is home, or at 90 s of
  play, whichever comes first.
- **The camera:** fixed, framing the whole field at a size that reads (the field is 128
  cells wide at 8 px). The step sets the movie's size to fit and records it here. The spike's
  frames came out 1152×648, too small for the field.
- **What it writes:** `capture.json` beside the movie: the scenario's name, map, seed and
  record, and the commit. The ticks of the four moments (first tick, first contact (the
  first tick with damage), midpoint, last tick), the tick count, and each behaviour with the
  first tick it was seen, or `null`.
- **The bench:** `breach_tick_phases scenario=<name>` times the same replay headless, every
  tick from the first to the scenario's end, printed as its `series=1` prints, so
  `bench_compare.py` and the `bench` workflow read it unchanged. It reads the same
  `content/scenarios/` list.
- No other behaviour of the field changes without `--capture`.

**Split** (cascade #841's standing rule, too big for one session):
- **2a:** the pre-mortem's replay, the list (`content/scenarios/`, read by
  `native/rust/src/scenarios.rs`), how each behaviour is read (`scenarios/seen.rs`) and the
  first three scenarios, checked by `cargo test scenarios::`.
- **2b:** `breach_tick_phases scenario=<name>`, timing `scenarios::Replay` tick by tick.
- **2c:** `--capture=<name>` on the field and `capture.json`, through the same `Replay`.

**Settled in 2a:**
- **Cornering** is read from the events: a squad fighting two different foes at once (squads
  it has engaged, or been flanked by), counted since it last disengaged or withdrew.
- **Fording** is a living unit standing in water deeper than 0 (`Ground::depth_at`); on the
  field only the ford can be stood in.
- **First contact** is the first tick a `hit` does damage (dmg above 0).
- **The end:** a side no longer stands after first contact (the digest's `both_stand`, which
  is also "every player squad home"), then TAIL (150) ticks so a rout plays out; or 90 s
  (900 ticks), whichever comes first.
- A scenario reaching contact within 15 s can't send full waves: grems take 2 s each across
  four builders, and a wave walks about 12 s to the line. The drafts send A partly built at
  tick 40 (25 for the break-off).

**Settled in 2b:**
- `breach_tick_phases scenario=<name>` (`scenarios::timed`) takes only `threaded` and `repo`
  beside it (`series` is accepted and changes nothing); any other argument exits 2. Its header
  line is "<name> on <map>, seed <seed>: <ms> ms a tick", to two decimals: the field's tick is
  about 0.05 ms, which one decimal prints as 0.0.
- The phases' series is built by `Series::push`, shared with `series=1`.
- A scenario's ticks timed are its ticks played (`watch`'s), from tick 0 to its end; checked
  by `cargo test scenarios::`.

**Settled in 2c:**
- The field plays a scenario through the same watch as the bench's `Replay`
  (`scenarios::Watch`, split out of it): `FieldSim::scenario` puts it on the field's worker, and
  `FieldState.open_scenario`, `ended` and `capture` read it. Checked by `cargo test`: the field
  on its worker ends on the replay's tick and sees what it sees.
- The scene takes it as `restart(..., scenario)` (`FieldCapture` does), not a new method: the
  scene is at the ISP check's seven public methods and its 300-line limit.
- In a capture each frame plays the ticks its clock owes and waits for them (`run_ticks`),
  never dropping time, so the frames don't depend on the worker's speed.
- **The movie's size:** 1280×720 (`--resolution 1280x720`; the scene also sizes the window).
  The project's 1152×648 canvas scales by 10/9, so the whole field (128 by 64 cells) shows at
  about 9 px a cell, with the HUD above it.
- **`capture.json`** (`scenarios::capture`): `scenario`, `map`, `seed`, `record`, `commit`
  (`BREACH_COMMIT`, else `GITHUB_SHA`), `moments` (`first` 0, `contact` or null, `midpoint`
  halfway from contact (or tick 0) to the last, `last`), `ticks`, `behaviours` (all six, a tick
  or null) and `missing`. Written beside `--write-movie`'s file, or to `--capture-out=<path>`.
- A name not on the list prints the error naming it and exits 2.

### Step 3, the `showcase-capture` workflow

`.github/workflows/showcase-capture.yml`, on `pull_request` (opened, synchronize, reopened,
closed).
- **Which PRs:** `ci/showcase/is_game_change.py` (Decision 167's paths). A PR from a fork is
  skipped and says so in the job summary.
- **Capture:** for each scenario on step 2's list, xvfb, `--write-movie` on the field with
  `--capture=<name>` at step 2's size and 30 fps, at most 90 s. ffmpeg makes
  `pr<N>-<sha7>-<scenario>.webm` (VP9, at most 5 MB) and four
  `pr<N>-<sha7>-<scenario>-<moment>.webp` stills from the recorded ticks.
- **Timings:** for each scenario, `breach_tick_phases scenario=<name>` with 5 rounds on the
  same commit and runner, after the movies. Its tick median and 95th percentile, and its
  phases' medians, go into the manifest.
- **Publish:** uploads them to the pre-release `showcase-YYYY-MM` (created if missing), and
  updates `manifest.json` on the pre-release `showcase-index`: per capture the PR, its title,
  state, commit and date; per scenario its asset URLs, moments, tick count, the behaviours
  seen and not seen, and its timings.
- **Retention:** an open PR keeps its last two captures; on `closed`, merged or not, only
  the last is kept (no new capture is made on close).
- **The PR comment:** one comment, found by a hidden marker and edited in place, with a row
  per scenario: its stills, the clip's link, its tick median, and any behaviour not seen
  ("flanked: not seen"). Then a link to `#showcase`. Ends with the attribution footer.
- **Advisory:** a failed capture edits the comment to say so and links the run; the job
  doesn't fail the PR's checks.
- **Concurrency:** one group per PR, a new push cancelling the old run. After publishing, it
  dispatches the site build.

**Settled in 3** (`ci/showcase/README.md`):
- **Each PR's record is its own file,** `pr-<N>.json` on `showcase-index`, written only by
  that PR's runs, and `manifest.json` is rebuilt from every record on every run. One shared
  file edited by every PR would lose a capture whenever two runs overlap; GitHub's
  concurrency keeps only one waiting run a group, so one global group would drop runs too.
- **`manifest.json`** is `{"version": 1, "captures": [...]}`, newest first. Each capture
  carries its PR's `pr`, `title`, `state` (`open`, `merged`, `closed`), `url` and, once
  closed, `closed_at` (for step 5's "the previous merged PR"), then `commit` (the PR's head),
  `date`, `release`, `run` and `scenarios`. A scenario has `name`, `map`, `seed`, `clip`,
  `stills` (by moment), `moments`, `ticks`, `seen` (behaviour to first tick), `not_seen`,
  `missing` and `timings` (`rounds`, `tick_median_ms`, `tick_p95_ms`, `phases_median_ms`).
- **The timings** are medians over the 5 rounds: of the tick's mean, of its 95th percentile,
  and of each phase's mean.
- **The comment's "not seen"** names the behaviours the scenario is named for and didn't
  show (`missing`); the manifest keeps every behaviour not seen.
- **A clip over 5 MB** is encoded again at the rate that fits.
- **The movie's size and `capture.json`'s place, found by the hand check:** `--resolution`
  doesn't set Movie Maker's size (it recorded 1152×648), so `capture.sh` writes an
  `override.cfg` (window override 1280×720) for the run, and the 1152×648 canvas scales by
  10/9 as 2c intended. Godot keeps `--write-movie` out of `OS.get_cmdline_args()`, so 2c's
  "beside the movie" falls back to `user://`; `capture.sh` passes `--capture-out`.
- **A new capture of a commit already captured** (a re-run) replaces it, keeping two
  different commits.
- **Advisory:** each step that can fail continues on error, and the next runs only if it
  succeeded, so the job never goes red; the last step edits the comment on any failure.
- **Checked by a run on the first game PR after this one:** this PR changes only `ci/` and
  `.github/`, so its own run decides "not a game change". `capture.sh` was checked by hand
  (the PR's description says how).

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

**Settled in 4** (`tools/site/README.md`):
- The built site puts the shell at the root and each tab in its folder (`_site/metrics/`). A
  tab's page loads in an `<iframe>`, created on the tab's first view and kept after, so the
  tick history page moves in unchanged. Its one new line is `<base target="_top">`, so its PR
  and record links open over the site rather than inside the frame (GitHub refuses framing).
- The shipped list of tabs has Metrics only. Step 5 adds Showcase's line with its folder;
  until then `#showcase` opens Metrics, as any unknown name does. The routing test checks
  `#showcase` against a fixture list with both tabs.

### Step 5, the Showcase tab

`tools/showcase_view/`, built from `manifest.json` by the site workflow (Decisions 167 and
169).
- **Timeline, newest first:** a card for each PR's latest capture: number and title (linked),
  state (open captures marked), commit and date. Under it, a row per scenario, named as the
  bench names it.
- **A scenario's row:** the four stills, a player that opens the clip (up to 90 s), and
  beside them that commit's tick timings for the same scenario: the tick's median and 95th
  percentile, and its phases' medians on demand. The behaviours it was named for are listed,
  each marked seen (with its tick) or not seen.
- **Compare:** a toggle puts a capture beside the one before: the PR's previous capture while
  it is open, the previous merged PR's once merged. Paired by scenario: stills side by side at
  the same moments, clips started together, and each timing with its change in per cent. A
  scenario in only one of the two is shown alone and says so.
- **Filters:** open, merged, closed unmerged; all on by default. A scenario filter shows one
  scenario's rows only.
- Phone width works: one column, the player full width, the timings under the stills.

### Step 6, an agent's critique (optional)

After a capture, an agent reviews the four stills against the comparison's (step 5's rule)
and appends a short critique to the PR comment: what changed, what reads poorly, what looks
broken. Advisory only. Needs the owner to add an API key secret; the step says which. If not
run, this step moves to its own spec.

## Not in this spec

- **A replay viewer of the simulation's state** (Q41, #828): smaller than clips, but it draws
  the simulation, not the game.
- **A playable web build** on the site.
- **More scenarios** (archers, forts, the bench's `4096` and `25k` battles once a map can
  draw them): each is an entry in step 2's list. A new one needs no new step.
- **Q25's game-like map** (#280): when it can play a named scenario with `--capture`, the
  capture moves to it from the field (Decision 169).
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

Final run (commit 669e278, the spike's PR comment):
- **Wall time:** build 330 s, import 7 s, runs 20 s and 18 s, 300 frames each.
- **Frames:** byte-identical.
- **Playback, both browsers:** the WebM plays in a `<video>` in Chromium 131.0.6778.33 and
  in Firefox 132.0 (1152×648, 10.0 s). The WebP shows in an `<img>` in both.
- **PR comment:** the WebP still shows inline, through github.com, which answers 200
  `application/octet-stream`.
- **Go:** the capture renders, quickly and deterministically, and plays. The scene was the
  wrong one (below), settled by Decision 169.
- **Owner's view of the frame:** the scene captured is the wrong one to show. The lane feel
  test's camera fits the whole map, too small to read, and in 10 s with no orders nothing
  reaches a fight. The owner is most interested in what the 2D field shows: fording,
  flanking, engaging, breaking off, cornering and routing. Settled in the design session
  that followed (Decision 169): named scenarios on the field, sharing their names with the
  bench, each up to 90 s; step 2 rewritten.

### Step 2a, the replay and the first scenarios

The digest's five field logs replayed on `main` (cb931b2) with `scenarios::watch`, all six
behaviours read, to 90 s (none ended sooner). Ticks of first contact, then each behaviour's
first tick:

| Log | Contact | Fording | Flanking | Engaging | Breaking off | Cornering | Routing |
|---|---|---|---|---|---|---|---|
| field0 (A via C, A+B at 158) | 373 | 214 | 369 | 389 | - | 509 | 461 |
| field1 (B at 120, retreat 318) | - | 176 | - | - | - | - | - |
| field2 (A via C at 87, retreat 229) | 215 | - | - | 212 | 229 | - | - |
| field3 (field0, v1, tending) | 373 | 214 | 369 | 389 | - | 509 | 461 |
| field4 (field1, v1, tending) | - | 176 | - | - | - | - | - |

- field0 and field3 still show fording, flanking, engaging, cornering and routing; field2
  still engages and breaks off. field1 and field4 no longer reach a fight before B is
  retreated at 318, so they show only the ford: a breaking-off they were played to show is
  gone. The pre-mortem's regret is real, and the capture's "not seen" report is needed.
- None reaches contact within 15 s, so none seeds a scenario as it is.

The first scenarios (`content/scenarios/`), as drafted, on the same commit:

| Scenario | Orders | Contact | Shows (tick) | Ends |
|---|---|---|---|---|
| `field_ford_and_flank` | seed 492625; B waits in the wood; A at 40, B at 90 | 142 | fording 146, engaging 139, flanking 299, cornering 299 | 900 |
| `field_waves_rout` | seed 1; A at 40, 100, 160, 220 and 280 | 142 | engaging 139, cornering 201, routing 454 (also breaking off 474, fording 502) | 605 |
| `field_break_off` | seed 1; A at 25, retreated at 140 | 128 | engaging 124, breaking off 140 | 291 |

## Scenarios

### Step 1
- Given the spike runs twice on one commit, then it records the renderer, the times and
  whether the frames are identical, and the owner judges one frame.

### Step 2
- Given the digest's five field logs replayed on `main`, then the behaviours each still
  shows are recorded under "Measured".
- Given `--capture=<name>` on the field, when the scene starts, then it plays that scenario's
  record without input, writes `capture.json` with four moments in order and each behaviour's
  first tick or `null`, and quits by 90 s of play.
- Given the same commit twice, then `capture.json` is identical.
- Given each scenario on the list, then every behaviour it is named for is seen.
- Given `breach_tick_phases scenario=<name>`, then it times the same ticks the capture played
  and prints them as `series=1` does.
- Given a name not on the list, then the scene and the bench exit with an error naming it.
- Given no `--capture`, then the field behaves as before.

### Step 3
- Given a PR touching only `docs/`, `specs/`, `tests/`, `ci/` or `metrics/`, then no capture runs.
- Given a PR touching `presentation/`, `native/` or `content/scenarios/`, then a capture is
  published with a clip, four stills and timings per scenario, and the PR has one comment.
- Given a scenario whose behaviour is no longer seen, then the comment says so and the PR's
  checks stay as they were.
- Given a second push, then the comment is edited, not added, and the PR keeps two captures.
- Given the PR closes, then only its last capture remains.
- Given the capture fails, then the comment says so and the PR's checks stay as they were.

### Step 4
- Given the root URL or `#unknown`, then Metrics opens; given `#showcase`, then Showcase opens.
- Given the tick history page, then it draws as it did before the move.

### Step 5
- Given a manifest, then cards are newest first, open captures are marked, and each card has
  a row per scenario with its stills, player, timings and behaviours.
- Given compare on an open PR with two captures, then its two captures show side by side,
  paired by scenario, with each timing's change; on a merged PR, then the previous merged
  PR's capture is beside it.
- Given a scenario in only one of the two captures, then it shows alone and says so.

### Step 6
- Given a capture, then the PR comment gains a critique, and nothing else changes.

## Test-first order

1. Step 1: no test; the spike is checked by its run, recorded under "Measured".
2. Step 2: the digest logs' replay first, recorded. Then red on the scenario list's reading
   and each scenario's behaviours seen (`cargo test` in `native/rust/`), and on
   `scenario=` in `breach_tick_phases`. Then red on `capture.json` from a short fixed run
   (gdUnit, `-a` the one file) and on the field unchanged without the argument; then
   `--capture`.
3. Step 3: red on `is_game_change.py` against fixture path lists, and on the retention rule
   and the manifest update (Python, fixture manifests); then the workflow, checked by a run.
4. Step 4: red on the routing (JS logic, tested as `trace_view.js` is); then the shell.
5. Step 5: red on the timeline order, the compare pairing by scenario and the timing changes
   from a fixture manifest (JS logic); then the tab.
6. Step 6: red on the comment update keeping the stills and adding the critique (Python);
   then the job.

## Rubric answers

- **single-noun-phrase:** the spike; the capture argument; the capture workflow; the site
  shell; the Showcase tab; the critique.
- **ocp-extension-point:** a new page is a folder and a line in the shell's list; a new
  scenario is an entry in `content/scenarios/`, captured and timed with no code change.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the Showcase tab reads only `manifest.json`; the workflow only the capture's
  output.
- **dip-direction:** the tabs depend on their built data, never on how it is stored.
- **general-over-special:** general: the files changed decide capture; every scenario on one
  list is captured, timed and checked the same way, at the same moments; one retention rule;
  every page a tab (Decisions 166, 167, 169).
- **pre-mortem:** round 2's riskiest assumption (a fixed record keeps showing its
  behaviours) is tested first in step 2, then checked on every capture.
- **order-independent:** not applicable; no simulation outcome changes.
- **bounded-work:** not applicable to the tick; one capture job per push to a game PR,
  cancelled by the next.
