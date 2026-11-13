---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 49: A store of native binaries, and versioned releases

## Purpose

CI builds the same native code many times: `bench`, `bench-inspect` and `bench-series`
rebuild the references (`8ca4227`, `eee3e81`) on every run, and `sim-checks`' `base-build`
rebuilds the same `main` commit for every open PR. The owner's PC builds both sides of a local
pair from scratch (`local_grid.sh pair`, spec 48 step 5b), 2 to 4 minutes a side. And the
project has no versions: no tags, and `CHANGELOG.md` holds only `[Unreleased]`. Q48 (#949).

**Settled by Decision 179 (the store):**
- a binary is keyed by the git tree id of `native/`, the pinned Rust toolchain, the Cargo
  profile and the target, so most merges reuse the last build;
- the toolchain is pinned in `native/rust/rust-toolchain.toml`, and a CI check keeps every
  build input under `native/`;
- the store is assets on one long-lived pre-release, `native-store`;
- a key is filled for Linux and Windows in parallel; a reader downloads its key, else builds
  that OS itself and uploads it;
- only trusted runs write; references and releases are kept forever, the last 20 keys from
  `main`, PR-only keys for 14 days, under a 1 GB cap.

**Settled by Decision 180 (releases):**
- versions are 0.x.y from `0.0.1`: a breaking change or an `Added` entry bumps the minor,
  anything else the patch;
- a workflow keeps a release PR (`chore/release`) current, and the owner releases by merging it;
- a release ships `native/bin/` for Linux and Windows, from the store, and is kept forever.

## Status

Agreed in the design session of 2026-11-12. Eleven steps, each its own PR, built with
`/run-phase` as "spec 49 step N".
- Steps 1, 2 and 10 can be built side by side.
- Step 3 needs 1; step 4 (the riskiest assumption's test) needs 2 and 3.
- No workflow reads the store before step 4 passes: steps 5 to 8 need 4.
- Step 9 (the prune) needs 3. Step 11 (the release) needs 3 and 10.

## Pre-mortem

- **Regret in a month:** a stored binary is used for code it wasn't built from, because the key
  missed an input (a flag in a workflow, a newer compiler), and a `bench` reports a change
  that isn't there with nothing failing. Second: the release machinery costs more upkeep than
  it gives before there is anything to ship.
- **Riskiest assumption:** that everything which changes a build lives under `native/`, so two
  builds with one key time the same.
- **Cheapest test:** steps 1 and 2 make the assumption checkable (the pin, and a check that no
  build input lives outside `native/`), and step 4 builds the reference twice under one key on
  separate runs and benches one against the other: no change beyond noise. Scheduled as steps
  1, 2 and 4, before any reader.

## Steps

### Step 1, the pin and the key

**Needs:** none
**Touches:** `native/rust/rust-toolchain.toml` (new), `ci/native/store_key.sh` (new),
`ci/native/test_store_key.sh` or a Python equivalent (new), `native/README.md`

- **The pin:** `rust-toolchain.toml` names the stable version CI's runners use today (read
  with `rustc -V` on a run), with the `rustfmt` and `clippy` components CI's `rust` job uses.
- **The key:** `store_key.sh <commit> <profile> <target>` prints
  `<tree id of native/>-<toolchain>-<profile>-<target>`. The toolchain is the pinned one of
  the commit running the script (the head), not of the commit being keyed, so a reference that
  predates the pin is keyed and built with the head's compiler.

Scenarios:
- Given two commits that differ only outside `native/`, then their keys are equal.
- Given a change to a `.rs` file, `Cargo.lock`, `Cargo.toml`'s profiles, `native/build.sh` or
  the pin, then the key changes.
- Given the same commit keyed for `release` and `profiling`, or for two targets, then the
  keys differ.

### Step 2, build inputs stay under `native/`

**Needs:** none
**Touches:** `ci/native/check_build_inputs.py` (new) and its test, `.github/workflows/ci.yml`
(a step that runs it), `ci/godot/README.md` or `native/README.md` (what it checks)

- **The check** fails if a workflow or script outside `native/` sets `RUSTFLAGS`,
  `CARGO_PROFILE_*`, `CARGO_BUILD_*`, `--features`, `target-cpu`, or `RUSTUP_TOOLCHAIN`, except
  in the store's helper (`ci/native/`). It names the file and line.

Scenarios:
- Given a workflow that sets `RUSTFLAGS`, then the check fails and names it.
- Given today's workflows, then the check passes (or the step moves the flag under `native/`).

### Step 3, the store and its fill

**Needs:** 1
**Touches:** `.github/workflows/native-store.yml` (new), `ci/native/store.sh` (new: fetch, or
build and upload), its test, `ci/metrics/README.md` or `native/README.md` (the store)

- **The store** is the pre-release `native-store`, made by the workflow if missing. An asset is
  `<key>.zip`: `native/bin/` whole for `release`, `breach_tick_phases` alone for `profiling`.
- **`store.sh fetch <commit> <profile> <target> <dir>`** downloads and unzips the key into
  `<dir>`; if it is missing, builds it (`native/build.sh`, or `cargo build --profile
  profiling --bin tick_phases`) with `RUSTUP_TOOLCHAIN` set to the head's pin, and uploads it
  when the run may write. An upload that finds the asset already there skips.
- **May write:** a push to `main`, a same-repo PR, a release, `workflow_dispatch`. A fork's PR
  never.
- **The fill workflow** runs on pushes to `main`, same-repo PRs and `workflow_dispatch` with a
  list of commits, as a job per OS in parallel (`ubuntu-latest`, `windows-latest`), each
  filling `release` for its target. It fills `profiling` on Linux only.
- **The size:** the PR reports one key's zip sizes per OS, so the 1 GB cap can be checked
  against 20 keys.

Scenarios:
- Given a key in the store, then `fetch` downloads it and builds nothing.
- Given a key missing, then `fetch` builds it and, on a run that may write, uploads it.
- Given a fork's PR, then `fetch` builds and does not upload.

### Step 4, two builds of one key time the same

**Needs:** 2, 3
**Touches:** this spec's `## Measured` section

No code: a measured verdict.
- Fill the reference `8ca4227` twice under its key on separate runs (`workflow_dispatch`, the
  second with the asset deleted first), keep both zips, and run `bench` with one as head and
  the other as base, `rounds` of 5.
- **Passing:** the tick and every phase show no change beyond noise. If it fails, the owner
  decides before any reader is built.

### Step 5, `bench` reads the store

**Needs:** 4
**Touches:** `.github/workflows/bench.yml`, `ci/metrics/README.md`

- Head, base and the reference come from `store.sh fetch` (`release`), and the sampling
  profile's binary from its `profiling` key, instead of building in worktrees.

Scenarios:
- Given a `bench` run whose base and reference are stored, then the run builds only the head
  (or nothing, when the head's `native/` matches a stored key), and its summary is as before.

### Step 6, `bench-inspect` and `bench-series` read the store

**Needs:** 4
**Touches:** `.github/workflows/bench-inspect.yml`, `.github/workflows/bench-series.yml`,
`ci/metrics/README.md`

- Every side they build (the head, the base, the references, each past commit of a backfill)
  comes from `store.sh fetch`.

Scenarios:
- Given a backfill over commits that share a `native/` tree, then each tree is built once.

### Step 7, `sim-checks` reads the store

**Needs:** 4
**Touches:** `.github/workflows/sim-checks.yml`, `ci/godot/README.md`

- `base-build` and the digest's build fetch their keys; the PR's own build fetches its key
  too, so a PR that doesn't touch `native/` builds nothing.

Scenarios:
- Given a PR that doesn't touch `native/`, then `sim-checks` downloads `main`'s binaries and
  the digests run as before.

### Step 8, a local pair downloads its binaries

**Needs:** 4
**Touches:** `ci/metrics/local_grid.sh`, `ci/metrics/README.md` (Local runs)

- `local_grid.sh pair <head> <base>` fetches each side's Windows (or Linux) `release` key from
  the store over HTTPS, without credentials (the repo is public), and builds only a side whose
  key is missing, as now. Both sides come from the store, or both are built: never one of
  each (Q48 point 2).

Scenarios:
- Given both keys stored, then `pair` builds nothing.
- Given one key missing, then `pair` builds both locally and says why.

### Step 9, the prune

**Needs:** 3
**Touches:** `.github/workflows/native-store-prune.yml` (new), `ci/native/prune.py` (new) and
its test

- **Scheduled** (daily) and by hand. Keeps: the keys of the references (`8ca4227`,
  `eee3e81`, read from the workflows that name them) and of every release, forever; the last
  20 distinct keys built from `main`; PR-only keys for 14 days. Deletes the rest, then, past
  1 GB, the oldest unkept keys first.
- An asset records where it was first built (`main` or a PR, and the commit) in its label, so
  the prune can tell them apart.

Scenarios:
- Given 25 `main` keys, a reference key and a release key, then the 5 oldest `main` keys are
  deleted and the reference and release keys stay.
- Given a PR-only key 15 days old, then it is deleted.

**Built (2026-11-12):** `store.sh`'s upload labels the asset `main <sha>`, `pr <n> <sha>`,
`release <sha>` or `dispatch <sha>`. The label alone can't say which keys are `main`'s:
the upload skips a key already stored, so a key first built on a PR keeps its `pr` label
after the merge. So `main`'s keys are the trees of `native/` on its first-parent history,
newest first, plus any labelled `main`. The 20 kept are 20 distinct trees, each with all
its assets (Linux, Windows, profiling). The cap then deletes the oldest assets not kept
forever. The references are every full sha a workflow names outside a `uses:` line.

### Step 10, the release PR

**Needs:** none
**Touches:** `ci/release/release_pr.py` (new) and its test, `.github/workflows/release-pr.yml`
(new), `CHANGELOG.md` (compare links section, if missing), `AI_First_Development_Kit`'s or
`docs/`'s note on how to release

- **On each push to `main`**, the workflow runs `release_pr.py` and opens or updates the PR
  from `chore/release`. With no tag yet, the version is `0.0.1`. After that, a breaking change
  (`**BREAKING:**` in `[Unreleased]`, or `!` or `BREAKING CHANGE` in a commit since the last
  tag) or an `Added` entry bumps the minor (`0.x+1.0`), anything else the patch. The major
  stays 0.
- **The PR's change:** `[Unreleased]` becomes `[x.y.z] - <date>`, a fresh empty `[Unreleased]`
  above it, and the compare links at the bottom. Nothing else; no version in `Cargo.toml`.
- With `[Unreleased]` empty, no PR (an open one is closed).
- A PR opened with the workflow's token doesn't trigger CI; the workflow runs
  `check-docs.sh changelog` itself and says so in the PR body.

Scenarios:
- Given no tags and some entries, then the PR is for `0.0.1`.
- Given `v0.0.1` and an `Added` entry, then `0.1.0`; given only `Fixed`, then `0.0.2`.
- Given a commit since the tag with `feat!:`, then the minor bumps, never the major.

### Step 11, the release

**Needs:** 3, 10
**Touches:** `.github/workflows/release.yml` (new), `ci/release/` (notes from the changelog),
`docs/` (how to download a release)

- **When the release PR merges,** CI tags `vx.y.z` on the merge commit and makes a GitHub
  release whose notes are that version's changelog section, with the `release` keys for Linux
  and Windows attached (from the store, filled first if missing), named
  `breach-native-<version>-<target>.zip`.
- The release's keys are kept forever (step 9 reads the releases).

Scenarios:
- Given the release PR merged, then a tag and a release exist with both zips, and the notes
  match the changelog section.

## Measured

Nothing yet: step 4's verdict goes here.
