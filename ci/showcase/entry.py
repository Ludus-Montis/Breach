#!/usr/bin/env python3
"""One scenario's part of a showcase capture (spec 44 step 3), from the `capture.json` the
field writes (spec 44 "Settled in 2c"): its asset names, the frames its stills are cut from,
and its entry in the PR's record.

A still is cut at its moment's tick: 10 ticks a second at 30 fps is 3 frames a tick, never
past the movie's last frame. A moment with no tick (no contact) has no still.

Usage:
  entry.py prefix <pr> <sha> <scenario>                 - prints pr<N>-<sha7>-<scenario>
  entry.py stills <capture.json> <frames>               - prints "<moment> <frame>" lines
  entry.py entry <capture.json> <base-url> <prefix> <timings.json>  - prints the entry
"""
import json
import sys

FRAMES_A_TICK = 3
MOMENTS = ("first", "contact", "midpoint", "last")


def prefix(pr: int, sha: str, scenario: str) -> str:
    return f"pr{pr}-{sha[:7]}-{scenario}"


def stills(capture: dict, frames: int) -> list:
    moments = capture["moments"]
    return [
        (moment, min(moments[moment] * FRAMES_A_TICK, frames - 1))
        for moment in MOMENTS
        if moments.get(moment) is not None
    ]


def scenario_entry(capture: dict, base_url: str, name_prefix: str, timed: dict) -> dict:
    behaviours = capture["behaviours"]
    return {
        "name": capture["scenario"],
        "map": capture["map"],
        "seed": capture["seed"],
        "clip": f"{base_url}/{name_prefix}.webm",
        "stills": {
            moment: f"{base_url}/{name_prefix}-{moment}.webp"
            for moment in MOMENTS
            if capture["moments"].get(moment) is not None
        },
        "moments": capture["moments"],
        "ticks": capture["ticks"],
        "seen": {b: tick for b, tick in behaviours.items() if tick is not None},
        "not_seen": [b for b, tick in behaviours.items() if tick is None],
        "missing": capture["missing"],
        "timings": timed,
    }


def assets(made: dict) -> list:
    """The release assets a scenario's entry names: its clip, then its stills."""
    urls = [made["clip"], *(made["stills"][m] for m in MOMENTS if m in made["stills"])]
    return [url.rsplit("/", 1)[1] for url in urls]


def main(arguments: list) -> int:
    command = arguments[0] if arguments else ""
    if command == "prefix" and len(arguments) == 4:
        print(prefix(int(arguments[1]), arguments[2], arguments[3]))
    elif command == "stills" and len(arguments) == 3:
        capture = json.load(open(arguments[1], encoding="utf-8"))
        for moment, frame in stills(capture, int(arguments[2])):
            print(moment, frame)
    elif command == "entry" and len(arguments) == 5:
        capture = json.load(open(arguments[1], encoding="utf-8"))
        timed = json.load(open(arguments[4], encoding="utf-8"))
        print(json.dumps(scenario_entry(capture, arguments[2], arguments[3], timed), indent=1))
    else:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
