# ci/github: the GitHub sync

Stdlib Python run by the `github-sync` workflow (`.github/workflows/github-sync.yml`), with
`GITHUB_TOKEN`. Cloud agents can't create labels or milestones, so this does (spec 36,
Decision 148); it also keeps an issue per spec step (spec 40, Decision 155). Every run
creates only what is missing, so a second run changes nothing. It never edits a body,
renames or deletes.

| When | What `sync.py` does |
|---|---|
| Push to `main` changing `specs/**`, or by hand | The labels in `labels.FIXED`; per `specs/NN-*.md` a milestone `Spec NN: <title>`, a `raise-when:spec-NN` label and a `step:NN-<id>` label per distinct `### Step <id>` heading; a parent issue `Spec NN: <title>` with a sub-issue `Spec NN step <id>: <title>` per step id, in the milestone. A step whose `step:` label is on a merged PR is created closed. |
| A step's heading is gone | Its issue, if the sync made it and it is open, is closed as not planned with a comment naming the spec's steps now. |
| A PR changing `specs/NN-*.md` | The labels, and the spec's milestone (created if missing) on the PR and on each issue its body `Closes`, `Fixes` or `Resolves`, unless one is set. With several specs, the lowest-numbered. No step issues. |
| By hand with `migrate` | `migrate_questions.py` moves `OPEN_QUESTIONS.md`'s open entries to issues labelled `question`. |

The sync knows its own issues by a marker line in the body, `<!-- github-sync: spec NN -->`
or `<!-- github-sync: step NN-<id> -->`. A step heading is a contract: renaming one closes
its issue and opens another. Writes are paced a second apart (GitHub's secondary rate
limit), so the first backfill takes a few minutes.

Files: `sync.py` (plan and run), `step_issues.py` (the parent and step issues),
`spec_pr.py` (a spec PR's milestone), `labels.py`, `github_client.py` (the REST calls),
`fake_client.py` (the tests' stand-in). Tests: `python3 -m unittest discover -s ci/github
-p "test_*.py"`.
