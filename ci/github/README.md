# ci/github: the GitHub sync

Stdlib Python run by the `github-sync` workflow (`.github/workflows/github-sync.yml`), with
`GITHUB_TOKEN`. Cloud agents can't create labels or milestones, so this does (spec 36,
Decision 148); it also keeps an issue per spec step (spec 40, Decision 155). Every run
creates only what is missing, so a second run changes nothing. It never edits a body,
renames or deletes.

| When | What `sync.py` does |
|---|---|
| Push to `main` changing `specs/**` or the sync itself, or by hand | The labels in `labels.FIXED`; per `specs/NN-*.md` a milestone `Spec NN: <title>`, a `raise-when:spec-NN` label and a `step:NN-<id>` label per distinct `### Step <id>` heading; a parent issue `Spec NN: <title>` with a sub-issue `Spec NN step <id>: <title>` per step id, in the milestone. A step whose `step:` label is on a merged PR is created closed. |
| Several issues carry one marker | The lowest-numbered is the sync's own; each later one that is open is closed as a duplicate with a comment naming the kept one (spec 40 step 4). |
| A kept step isn't under its spec's kept parent | It is attached there (`replace_parent`), open or closed; the sync reads each kept step's parent once a run and writes only where it differs. |
| After every run | Each sync parent with step sub-issues is open while any step is, and closed when all are: as completed if any step completed, else as not planned. A parent with no steps is left to the owner. Each `Spec NN: <title>` milestone is open while any issue or PR in it is, and closed when none is; no other milestone is touched (spec 40 step 5). |
| An issue is closed or reopened | The same rule for that issue's parent (if it is a sync step) and for its milestone and the parent's. Nothing is created, no labels touched. |
| A step's heading is only under the spec's `## Built` | A step split while it was built (7b): the sync only sees it once its PR has merged, and that PR carried the parent step's label. Its issue, if open, is closed as completed with a comment naming the PR whose merge first added the heading (`git log -S`, so the workflow fetches the whole history). A heading under `## Steps` is never closed this way. |
| A step's heading is gone | Its issue, if the sync made it and it is open, is closed as not planned with a comment naming the spec's steps now. |
| A PR changing `specs/NN-*.md` | The labels, and the spec's milestone (created if missing) on the PR and on each issue its body `Closes`, `Fixes` or `Resolves`, unless one is set. With several specs, the lowest-numbered. No step issues. |
| By hand with `migrate` | `migrate_questions.py` moves `OPEN_QUESTIONS.md`'s open entries to issues labelled `question`. |

The sync knows its own issues by a marker line in the body, `<!-- github-sync: spec NN -->`
or `<!-- github-sync: step NN-<id> -->`. Each run prints how many issues, PRs and markers it
read before it writes; before creating anything for a spec it also reads that spec's
milestone, and counts the markers there as its own too. A step heading is a contract: renaming one closes
its issue and opens another. Writes are paced a second apart (GitHub's secondary rate
limit), so the first backfill takes a few minutes.

Files: `sync.py` (plan and run), `step_issues.py` (the parent and step issues),
`open_while_open.py` (a parent and its milestone follow their items),
`spec_pr.py` (a spec PR's milestone), `labels.py`, `github_client.py` (the REST calls),
`fake_client.py` (the tests' stand-in). Tests: `python3 -m unittest discover -s ci/github
-p "test_*.py"`.

## The Project's Status

`project_status.py`, run by the `project-status` workflow
(`.github/workflows/project-status.yml`) when a PR is opened, reopened, marked ready,
turned back into a draft, or gains or loses the `handoff` label (spec 40 step 3). It sets
the PR's Status in the owner's Project, and the same Status on each issue the PR closes:

| PR | Status |
|---|---|
| Draft labelled `handoff` | Next |
| Other draft | In progress |
| Ready for review | In review |

Merged or closed is left to the Project's built-in workflows (Done). It reads the PR with
`GITHUB_TOKEN` and writes the Project with a token minted each run from the owner's
GitHub App: repo variables `PROJECT_APP_ID`, `PROJECT_OWNER` and `PROJECT_NUMBER`, and the
secret `PROJECT_APP_PRIVATE_KEY` (spec 40, "The owner's setup"). Until they are set, the job
says so and passes. An item not yet in the Project is added. Option names ignore case; one missing from
the Project fails the job naming it. Files: `project_status.py` (the rule, the gate and
apply), `graphql_client.py` (the GraphQL calls).

## The Project backfill

`project_backfill.py`, run by hand by the `project-backfill` workflow
(`.github/workflows/project-backfill.yml`, spec 40 step 6). It lists the open issues and PRs
with `GITHUB_TOKEN` (REST, no search), lists the Project's items once (paged) with the App's
token, and adds each open issue or PR that isn't there. Status is left to the Project's
built-in workflow (Inbox). It prints how many it added and how many were already there, so a
second run adds nothing. Skipped like `project-status` until the App is set up. Files:
`project_backfill.py` (the gate, the REST read and apply); it reuses `graphql_client.py`.
