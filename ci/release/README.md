# Releases

Spec 49, Decision 180. Versions are `0.x.y`, starting at `0.0.1`; the major stays 0.

## How to release

1. Nothing to do on each merge: the `release-pr` workflow (`.github/workflows/release-pr.yml`)
   runs on every push to `main` and keeps one PR open from `chore/release`, titled
   `chore(release): <version>`.
2. Review that PR. Its one change is `CHANGELOG.md`: `[Unreleased]` becomes
   `[<version>] - <date>` under a fresh empty `[Unreleased]`, and the compare links at the
   bottom are rebuilt. No version is written anywhere else (not in `Cargo.toml`).
3. Merge it. That is the release: the `release` workflow (`.github/workflows/release.yml`)
   tags `v<version>` on the merge commit and publishes a GitHub release whose notes are that
   version's changelog section, with the native binaries attached ("Download a release").

With `[Unreleased]` empty there is no release PR, and an open one is closed.

## Download a release

Each release on the repository's Releases page carries `native/bin/` for both platforms:

- `breach-native-<version>-x86_64-unknown-linux-gnu.zip` (Linux);
- `breach-native-<version>-x86_64-pc-windows-msvc.zip` (Windows).

Unzip the one for your platform into `native/bin/` of a checkout of that tag
(`git checkout v<version>`), in place of `bash native/build.sh release`. The zips are the
merge commit's `release` keys from the store of native binaries (`ci/native/store.sh`,
Decision 179), built and stored first if missing; a release's keys are never pruned.

The workflow releases the newest `## [x.y.z]` section of `CHANGELOG.md` that has no
`vx.y.z` tag yet (`ci/release/release.py plan`), so rerunning it releases nothing twice.
If a release failed partway, run `release` by hand on `main` (`workflow_dispatch`).

## How the version is chosen

`ci/release/release_pr.py plan` reads the last `v*` tag, `CHANGELOG.md`'s `[Unreleased]` and
the commits since the tag:

- no tag yet: `0.0.1`;
- a breaking change bumps the minor (`0.x.y` to `0.x+1.0`): `**BREAKING:**` in an
  `[Unreleased]` entry, or a commit since the tag with `!` before its colon (`feat!:`) or
  `BREAKING CHANGE` in its message;
- an `### Added` entry also bumps the minor;
- anything else bumps the patch.

The tests (`release_pr.py`'s and `release.py`'s): `python3 -m unittest discover -s ci/release
-p "test_*.py"`, run by `release-pr` on a PR that touches `ci/release/`.

## Gotchas

- The PR is opened with the workflow's own token, so no CI (and no `docs-gate`) runs on it;
  `release-pr` runs `check-docs.sh changelog` itself before pushing, and the PR body says so.
- The repo's Actions settings must allow GitHub Actions to create pull requests (Settings,
  Actions, General, "Allow GitHub Actions to create and approve pull requests"), or the
  workflow's `gh pr create` is refused.
- The notes stop at the next `## ` heading or the first link line (`[x]: …`), so the compare
  links stay at the bottom of `CHANGELOG.md`.
