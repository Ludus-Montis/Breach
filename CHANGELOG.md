# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Changes before this file was started are in the git history and `PROGRESS_LOG.md`.

## [Unreleased]

### Added

- A project README, this changelog and a documentation index (`docs/index.md`), kept
  current by the documentation phases every change now goes through.
- Each design round now opens with a pre-mortem (what we'd regret in a month, the riskiest
  assumption, the cheapest test of it), recorded in the round's spec, with its test scheduled
  as a step (Decision 168).
- The tick history page has a 25k chart: each merge's tick at 24,576 a side (p95, with p99 as
  a band) for 24 lanes and for one squad, beside the estimate fitted from the smaller sizes,
  with misses beyond 10% ringed and the 50 ms and 100 ms goals drawn.

### Changed

- The tick history page now shows each record as × the reference timed beside it by default,
  so a slower or faster CI runner no longer reads as a change; "ms" is a click away, and the
  phase chart also offers % of tick. Each runner CPU has a checkbox to keep to one machine, and
  each phase in the legend can be hidden.
- In a crowded scrum, units now settle who makes for which slot together rather than one at a
  time: where several want the same slot the best-placed takes it and the others look again,
  and a few more wait a body's breadth short for a tick before finding a slot of their own.
- The 2D feel test's battle plays on its own thread: the frame no longer freezes while a tick
  runs, and orders are still taken between ticks. When the battle can't keep up and time is
  dropped, the status row says `SIMULATION BEHIND`.
- Units, weapons, armour and the battle's tuning are JSON files under `content/` (`units/`,
  `weapons/`, `armour/`, `tuning/battle_tuning.json`), checked against the JSON Schema in
  `content/schema/`. A file with a wrong or unknown field fails to load and names the file and
  the field. The Rust core must be built (`bash native/build.sh`) to run the game.
- Content that breaks its rules fails to load and names the file and the field: a unit's
  sheet out of bounds or its items not fitting its body, an item's weight or attack profile
  out of bounds, a trait or tag that isn't in `content/registry/` or can't sit there, and a
  terrain library with a repeated id, a stratum or liquid out of bounds or a liquid that
  doesn't flow. These were checked only by Godot's tests before.
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
- A group whose stragglers fall far behind and go on as a group of their own gives that
  group its own number everywhere in the battle as well, so its draws and its battle log
  entries are its own, not those of a squad numbered 0.
