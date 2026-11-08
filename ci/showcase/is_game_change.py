#!/usr/bin/env python3
"""Whether a PR changes the game (spec 44 step 3, Decision 167): its diff touches what the
shipped game is built from (`presentation/`, `core/`, `native/`, `content/`, `assets/`,
`main.tscn`, `project.godot`), `.md` files excepted. Docs, specs, tests, CI, tools and
`metrics/` alone don't. Decided by the files changed, never by the commit type.

Usage: is_game_change.py <base-ref>  - prints `game=true|false`, and appends it to
$GITHUB_OUTPUT when set.
"""
import os
import sys
from pathlib import Path

FOLDERS = ("presentation/", "core/", "native/", "content/", "assets/")
FILES = ("main.tscn", "project.godot")


def is_game_change(paths: list) -> bool:
    return any(
        not path.endswith(".md") and (path.startswith(FOLDERS) or path in FILES)
        for path in paths
    )


def output_lines(answer: bool) -> list:
    return [f"game={'true' if answer else 'false'}"]


def main(arguments: list) -> int:
    if not arguments:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "godot" / "scripts"))
    from needs_godot import changed

    lines = output_lines(is_game_change(changed(arguments[0])))
    print("\n".join(lines))
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as out:
            out.write("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
