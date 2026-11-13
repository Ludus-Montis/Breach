"""The release a push to main makes (spec 49 step 11, Decision 180).

Merging the release PR (step 10) puts a new `## [x.y.z] - <date>` section at the top of
CHANGELOG.md. The version to release is the newest such section with no `v<x.y.z>` tag yet,
so a rerun after the tag exists releases nothing. The notes are that section's body, up to
the next `## ` heading or the first link line.

    release.py plan [CHANGELOG.md]               prints `version=<x.y.z>`, or `version=` for none
    release.py notes <version> [CHANGELOG.md]    prints the version's notes
    release.py asset <version> <target>          prints the zip's name for that target
"""
import re
import subprocess
import sys
from pathlib import Path

VERSION_HEADING = re.compile(r"^## \[(\d+\.\d+\.\d+)\]", re.MULTILINE)
LINK_LINE = re.compile(r"^\[[^\]]+\]: \S+")


def version_to_release(text, tags):
    """The newest version section, when its tag doesn't exist yet; else None."""
    versions = VERSION_HEADING.findall(text)
    if not versions or f"v{versions[0]}" in tags:
        return None
    return versions[0]


def notes(text, version):
    """The body of `## [version]`, without its heading, the next section or the links."""
    lines = text.splitlines()
    prefix = f"## [{version}]"
    start = next((i for i, line in enumerate(lines) if line.startswith(prefix)), None)
    if start is None:
        raise KeyError(version)
    body = []
    for line in lines[start + 1 :]:
        if line.startswith("## ") or LINK_LINE.match(line):
            break
        body.append(line)
    return "\n".join(body).strip("\n") + "\n"


def asset_name(version, target):
    return f"breach-native-{version}-{target}.zip"


def tags():
    result = subprocess.run(["git", "tag", "--list", "v*"], capture_output=True, text=True)
    return set(result.stdout.split()) if result.returncode == 0 else set()


def main(argv):
    if argv[:1] == ["plan"]:
        text = Path(argv[1] if len(argv) > 1 else "CHANGELOG.md").read_text(encoding="utf-8")
        print(f"version={version_to_release(text, tags()) or ''}")
        return 0
    if argv[:1] == ["notes"] and len(argv) >= 2:
        text = Path(argv[2] if len(argv) > 2 else "CHANGELOG.md").read_text(encoding="utf-8")
        sys.stdout.write(notes(text, argv[1]))
        return 0
    if argv[:1] == ["asset"] and len(argv) == 3:
        print(asset_name(argv[1], argv[2]))
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
