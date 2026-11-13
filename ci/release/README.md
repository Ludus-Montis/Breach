# Releases

Spec 49, Decision 180. Versions are `0.x.y`, starting at `0.0.1`; the major stays 0.

## How to release

1. Nothing to do on each merge: the `release-pr` workflow (`.github/workflows/release-pr.yml`)
   runs on every push to `main` and keeps one PR open from `chore/release`, titled
   `chore(release): <version>`.
2. Review that PR. Its one change is `CHANGELOG.md`: `[Unreleased]` becomes
   `[<version>] - <date>` under a fresh empty `[Unreleased]`, and the compare links at the
   bottom are rebuilt. No version is written anywhere else (not in `Cargo.toml`).
3. Merge it. That is the release; spec 49 step 11 tags `v<version>` on the merge and
   publishes it with the native binaries.

With `[Unreleased]` empty there is no release PR, and an open one is closed.

## How the version is chosen

`ci/release/release_pr.py plan` reads the last `v*` tag, `CHANGELOG.md`'s `[Unreleased]` and
the commits since the tag:

- no tag yet: `0.0.1`;
- a breaking change bumps the minor (`0.x.y` to `0.x+1.0`): `**BREAKING:**` in an
  `[Unreleased]` entry, or a commit since the tag with `!` before its colon (`feat!:`) or
  `BREAKING CHANGE` in its message;
- an `### Added` entry also bumps the minor;
- anything else bumps the patch.

The tests: `python3 -m unittest discover -s ci/release -p "test_*.py"`, run by `release-pr`
on a PR that touches `ci/release/`.

## Gotchas

- The PR is opened with the workflow's own token, so no CI (and no `docs-gate`) runs on it;
  `release-pr` runs `check-docs.sh changelog` itself before pushing, and the PR body says so.
- The repo's Actions settings must allow GitHub Actions to create pull requests (Settings,
  Actions, General, "Allow GitHub Actions to create and approve pull requests"), or the
  workflow's `gh pr create` is refused.
- Until step 11 tags a merged release, the last tag stays missing, so the next release PR is
  `0.0.1` again.
