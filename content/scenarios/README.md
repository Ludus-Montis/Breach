# Scenarios

The named scenarios the showcase capture plays and the bench times (spec 44 step 2,
Decision 169). They are fixtures, not game content: nothing in the shipped game reads this
folder, so neither the content schema nor the designer covers it. It sits under `content/` so
that a changed scenario is a game change and is captured again.

## The list

`scenarios.json` holds one entry a scenario, in the order they are captured:

| Field | What it is |
|---|---|
| `name` | The scenario's name, shared by the capture and the bench. A field scenario is named `field_<what it shows>`. |
| `map` | The map it plays on. Only `field` (the 2D field, `formation_field.tscn`) for now. |
| `record` | The record file in this folder: a FormationRecord (Decision 115), whose header carries the seed. |
| `shows` | The behaviours it must show: any of `fording`, `flanking`, `engaging`, `breaking_off`, `cornering`, `routing`. |

The crate reads it (`native/rust/src/scenarios.rs`) and refuses an unknown map or behaviour, a
record that doesn't read, or a name listed twice.

## How each behaviour is read

From the replay, tick by tick (`native/rust/src/scenarios/seen.rs`):
- **engaging:** an `engaged` event; **flanking:** a `flanked` event;
- **breaking off:** a `disengaged` or `withdrawing` event;
- **routing:** a squad's morale falling to its routing band, or a `routed` event;
- **fording:** a living unit standing in water (on the field, only the ford can be stood in);
- **cornering:** a squad fighting two different foes at once (engaged with, or flanked by),
  since it last broke off.

First contact is the first tick a blow does damage.

## Rules a scenario keeps

The crate's tests (`cargo test scenarios::` in `native/rust/`) play every scenario and check:
- every behaviour it is named for is seen;
- first contact comes within 15 s of play (tick 150);
- it ends within 90 s: when a side no longer stands after first contact (then 150 ticks
  more, so a rout plays out), or at 90 s.

## Timing one

`native/bin/breach_tick_phases scenario=<name>` times a scenario's every tick, pass by pass,
printed as its `series=1` prints (`native/README.md`, "Tick phases"); the `bench` workflow runs
it with `args` `scenario=<name>`.

## Capturing one

The 2D field started with `-- --capture=<name>` plays the scenario hands-free, every tick it
owes waited for, so two runs on one commit give the same frames. When the scenario ends it
writes `capture.json` and quits (`presentation/skirmish/formation_2d/field_capture.gd`):

    godot --path . --resolution 1280x720 --fixed-fps 30 --write-movie /tmp/c/frame.png \
      res://presentation/skirmish/formation_2d/formation_field.tscn -- --capture=field_break_off

`capture.json` goes beside the movie (or to `--capture-out=<path>`) and holds the scenario's
name, map, seed and record, the commit (`BREACH_COMMIT`, else `GITHUB_SHA`), the ticks of its
four moments (`first`, `contact`, `midpoint`, `last`), the ticks played, each behaviour's first
tick or `null`, and `missing`, the behaviours it is named for and didn't show. A name not on
the list exits 2 with an error naming it.

## Adding one

Write the record (or copy one out of the field after playing it), add an entry to
`scenarios.json`, and run the tests above. No code changes.
