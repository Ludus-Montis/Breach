# Breach

[![Standard Readme](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg)](https://github.com/RichardLitt/standard-readme)

A reverse tower defence: you are the overlord, sending waves against a kingdom's forts.

Breach is built in Godot 4.7 (GDScript) with a Rust core for the simulation's hot paths. The
simulation is deterministic and lives in `sim/`, the scenes in `presentation/`, content and
its schema in `content/`, and a local web app for designing maps in `tools/designer/`.

## Table of Contents

- [Background](#background)
- [Install](#install)
- [Usage](#usage)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

## Background

The design is in
[Breach — Reverse Tower Defense Design Spec.md](Breach%20—%20Reverse%20Tower%20Defense%20Design%20Spec.md),
its addenda, and its Decision ledger ([DECISIONS_INDEX.md](DECISIONS_INDEX.md)). Work is built
in numbered rounds under [`specs/`](specs/), following the
[AI-First Development Kit](AI_First_Development_Kit/README.md).

## Install

You need [Godot 4.7.2](https://godotengine.org/) (the version CI pins). On Linux, this fetches
it and prints the `GODOT_BIN` to export:

```sh
bash ci/godot/scripts/get_godot.sh
```

Build the Rust core before the first run: the game and the tests load their units, weapons and
tuning through it (`content/*.json`, in the crate's schema). You need Rust from rustup
(platform notes in [native/README.md](native/README.md#building)):

```sh
bash native/build.sh
```

For the commit checks, install [pre-commit](https://pre-commit.com/) and `gdtoolkit`, then:

```sh
pre-commit install
```

## Usage

Open the project in Godot and run it (the main scene is `main.tscn`), or:

```sh
"$GODOT_BIN" --path .
```

Run the test suite (gdUnit4):

```sh
bash ci/godot/scripts/run_tests.sh
```

Design a map in the browser (Python 3.10+, opens `http://127.0.0.1:8765/`):

```sh
python tools/designer/serve.py
```

## Documentation

Everything beyond this page is indexed in [`docs/`](docs/index.md). Notable changes are in
[CHANGELOG.md](CHANGELOG.md).

## Contributing

Every change goes on a branch and through a pull request, which the maintainer merges. Tests
come first, commit messages follow [Conventional Commits](https://www.conventionalcommits.org),
and a change is done only when its documentation is
([documentation phases](AI_First_Development_Kit/principles/documentation-phases.md)). Agents
start from [CLAUDE.md](CLAUDE.md).

## License

No licence has been chosen yet. All rights reserved © Simeon Sidey.
