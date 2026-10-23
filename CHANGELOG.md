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
  terrain isn't in the library. The map viewer takes a map by name (`--map=demo_map`), and the
  map scene's export button writes JSON.

[Unreleased]: https://github.com/SSidey/Breach/commits/main
