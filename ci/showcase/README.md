# The showcase capture

Spec 44 step 3, Decisions 167 and 169. Every PR that changes the game shows it:
`.github/workflows/showcase-capture.yml` plays every scenario on
[the list](../../content/scenarios/README.md) on the 2D field, and puts a clip, four stills
and timings for each on the PR and on the site's Showcase tab.

## Which PRs

`is_game_change.py`: a PR whose diff touches `presentation/`, `core/`, `native/`,
`content/`, `assets/`, `main.tscn` or `project.godot`, `.md` files excepted. A PR from a fork
is skipped, and says so in the job summary.

## What a capture makes

`capture.sh`, for each scenario in the list's order:
- Movie Maker under xvfb and Mesa, 1280×720 at 30 fps, `--capture=<name>` (90 s of play at
  most), with `--capture-out` beside the frames. The size comes from an `override.cfg`
  written for the run (window override 1280×720): `--resolution` doesn't set Movie Maker's.
- `pr<N>-<sha7>-<scenario>.webm` (VP9; re-encoded at a lower rate if over 5 MB) and
  `pr<N>-<sha7>-<scenario>-<moment>.webp` for `first`, `contact`, `midpoint` and `last`, cut
  at the moment's tick (3 frames a tick; a scenario with no contact has no contact still).
  `<sha7>` is the PR's head commit.
- After every movie, 5 rounds of `breach_tick_phases scenario=<name>`. `timings.py` takes
  the median of the tick's mean, the median of its 95th percentile and each phase's median.

`entry.py` turns each scenario's `capture.json` into its entry: asset URLs, moments, ticks,
the behaviours `seen` (with their first tick), `not_seen`, `missing` (named for and not
seen) and its timings.

## Where it is kept

- The assets go to the pre-release `showcase-YYYY-MM` (UTC month of the capture), made if
  missing.
- Each PR's record, `pr-<N>.json`, goes to the pre-release `showcase-index`. Only that PR's
  runs write it (one concurrency group a PR), so two PRs never overwrite each other.
- `manifest.json`, beside the records, is rebuilt from every record on every run
  (`manifest.py merge`, `index.sh`): `{"version": 1, "captures": [...]}`, newest first. Each
  capture carries its PR's `pr`, `title`, `state` (`open`, `merged` or `closed`), `url` and,
  once closed, `closed_at`, then its `commit`, `date`, `release`, `run` and `scenarios`.
  Two runs finishing together can each rebuild it before seeing the other's record; the
  next run's rebuild puts it right, and the site can rebuild it from the records the same
  way.
- After publishing, the run dispatches the site build (`metrics-pages.yml`).

**Retention** (`manifest.py`): an open PR keeps its last two captures, and a new capture of
a commit already captured replaces it. On `closed`, merged or not, only the last is kept;
no capture is made on close, and a capture still running is cancelled. The assets let go are
deleted from their release.

## The PR comment

One comment, marked `<!-- showcase-capture -->` and edited in place (`comment.py`,
`post_comment.sh`): a row per scenario with its stills, the clip's link, its tick median and
any behaviour it is named for and didn't show ("breaking off: not seen"), then the run and a
link to `#showcase`.

**Advisory:** every step that can fail continues on error. A failure edits the comment to
say so, with the run's link, and the PR's checks stay as they were. The run's logs and each
scenario's `capture.json` are kept as the `showcase-capture` artifact for 14 days.

## Tests

`python3 -m unittest discover -s ci/showcase -p "test_*.py"` (the `tools-tests` pre-commit
hook, and the workflow's `decide` job). The workflow itself is checked by its runs on game
PRs.
