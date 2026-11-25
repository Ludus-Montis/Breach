# ci-review: a Claude PR review step for CI (study exercise)

A study exercise for the Claude Certified Architect exam, task statement 3.6 (Claude Code in
CI). It builds the `claude -p` command a CI job would run, reads the one JSON object that
command prints, and posts only the findings that are new since the last review.

Nothing here calls the API. The tests run on canned output in `fixtures/`, and
`example-workflow.yml` is a reference that sits outside `.github/workflows/`, so it never runs.

| File | What it does |
|---|---|
| `schema.json` | The JSON schema the review must match (`--json-schema`) |
| `prompt.py` | The prompt: review criteria, plus the earlier findings on a re-review |
| `command.py` | The `claude -p` argument list: read-only tools, a turn bound, JSON output |
| `envelope.py` | Reads the printed result object; a failed run becomes a structured error |
| `rereview.py` | Splits a re-review into new findings to post, earlier ones still open, fixed and unmentioned |

Run the tests with `python3 -m unittest test_ci_review -v` from this folder. The lines
marked `STUDY 3.6` carry the exam topic; the `Study*` tests cover them.
