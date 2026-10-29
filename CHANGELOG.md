# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Changes before this file was started are in the git history and `PROGRESS_LOG.md`.

## [Unreleased]

### Added

- A project README, this changelog and a documentation index (`docs/index.md`), kept
  current by the documentation phases every change now goes through.

### Changed

- Units, weapons, armour and the battle's tuning are JSON files under `content/` (`units/`,
  `weapons/`, `armour/`, `tuning/battle_tuning.json`), checked against the JSON Schema in
  `content/schema/`. A file with a wrong or unknown field fails to load and names the file and
  the field. The Rust core must be built (`bash native/build.sh`) to run the game.
- The terrain library is one file, `content/terrain/terrain_library.json`, in the same schema.
  The map designer's library page saves it directly, without Godot, and refuses an edit that
  removes a terrain or feature a saved map still uses.
- Maps are JSON files, `content/maps/<name>.json`, in the same schema: the nodes, lanes,
  factions, loss groups and the layout in one file. The map designer's Save writes it
  directly, without Godot (Godot is needed only for View in Godot), and refuses a map whose
  terrain isn't in the library or that breaks a map rule, listing why. The map viewer takes a
  map by name (`--map=demo_map`), and the map scene's export button writes JSON.
- The map designer's server is part of the Rust core: start it with
  `native/bin/breach_designer` (built by `bash native/build.sh`) instead of
  `python tools/designer/serve.py`. Python is no longer needed for the designer.

[Unreleased]: https://github.com/SSidey/Breach/commits/main

### Fixed

- A unit making for home on its own (one that came to alone, or a bearer taking the wounded
  home) goes by its own squad's number everywhere in the battle, so the draws its fights make
  and the battle log's entries for it are its own, not those of a squad numbered 0.
