"""The release PR's version and changelog (spec 49 step 10, Decision 180).

Versions are 0.x.y from 0.0.1. With no tag yet the version is 0.0.1. After that, a breaking
change (`**BREAKING:**` in `[Unreleased]`, or `!` or `BREAKING CHANGE` in a commit since the
last tag) or an `Added` entry bumps the minor, anything else the patch. The major stays 0.
With `[Unreleased]` empty there is no release.

    release_pr.py plan [CHANGELOG.md]           prints `version=<x.y.z>`, or `version=` for none
    release_pr.py apply <version> <date> <repo url> [CHANGELOG.md]
                                                 `[Unreleased]` becomes `[x.y.z] - <date>` under a
                                                 fresh empty `[Unreleased]`, with the compare
                                                 links at the bottom rebuilt
"""
import re
import subprocess
import sys
from pathlib import Path

UNRELEASED = "## [Unreleased]"
VERSION_HEADING = re.compile(r"^## \[(\d+\.\d+\.\d+)\]", re.MULTILINE)
LINK_LINE = re.compile(r"^\[(?:Unreleased|\d+\.\d+\.\d+)\]: \S+[ \t]*\n?", re.MULTILINE)
BREAKING_COMMIT = re.compile(r"\A[a-z]+(\([^)]*\))?!:|BREAKING[ -]CHANGE", re.MULTILINE)
ENTRY = re.compile(r"^[-*] ")


def unreleased_lines(text):
    """The lines of the `[Unreleased]` section, without link lines."""
    lines = text.splitlines()
    start = lines.index(UNRELEASED) + 1
    end = next((i for i in range(start, len(lines)) if lines[i].startswith("## ")), len(lines))
    return [line for line in lines[start:end] if not LINK_LINE.match(line + "\n")]


def has_added_entry(lines):
    category = None
    for line in lines:
        if line.startswith("### "):
            category = line[4:].strip()
        elif category == "Added" and ENTRY.match(line):
            return True
    return False


def is_breaking(lines, commits):
    if any("**BREAKING:**" in line for line in lines):
        return True
    return any(BREAKING_COMMIT.search(message) for message in commits)


def next_version(last_tag, text, commits):
    """The release's version, or None when `[Unreleased]` holds no entry."""
    lines = unreleased_lines(text)
    if not any(ENTRY.match(line) for line in lines):
        return None
    if not last_tag:
        return "0.0.1"
    _, minor, patch = (int(part) for part in last_tag.lstrip("v").split("."))
    if is_breaking(lines, commits) or has_added_entry(lines):
        return f"0.{minor + 1}.0"
    return f"0.{minor}.{patch + 1}"


def compare_links(versions, repo):
    """Keep a Changelog's links, newest first: each version against the one before it."""
    links = [f"[Unreleased]: {repo}/compare/v{versions[0]}...HEAD"]
    for newer, older in zip(versions, versions[1:]):
        links.append(f"[{newer}]: {repo}/compare/v{older}...v{newer}")
    links.append(f"[{versions[-1]}]: {repo}/releases/tag/v{versions[-1]}")
    return links


def rewrite(text, version, date, repo):
    body = LINK_LINE.sub("", text)
    body = body.replace(UNRELEASED + "\n", f"{UNRELEASED}\n\n## [{version}] - {date}\n", 1)
    body = re.sub(r"\n{3,}", "\n\n", body).rstrip("\n")
    links = compare_links(VERSION_HEADING.findall(body), repo)
    return body + "\n\n" + "\n".join(links) + "\n"


def git(*args):
    result = subprocess.run(["git", *args], capture_output=True, text=True)
    return result.stdout if result.returncode == 0 else None


def last_tag_and_commits():
    tag = (git("describe", "--tags", "--abbrev=0", "--match", "v[0-9]*") or "").strip() or None
    if not tag:
        return None, []
    log = git("log", f"{tag}..HEAD", "--format=%B%x00") or ""
    return tag, [message.strip() for message in log.split("\0") if message.strip()]


def main(argv):
    if argv[:1] == ["plan"]:
        path = Path(argv[1] if len(argv) > 1 else "CHANGELOG.md")
        tag, commits = last_tag_and_commits()
        print(f"version={next_version(tag, path.read_text(encoding='utf-8'), commits) or ''}")
        return 0
    if argv[:1] == ["apply"] and len(argv) >= 4:
        path = Path(argv[4] if len(argv) > 4 else "CHANGELOG.md")
        text = path.read_text(encoding="utf-8")
        path.write_text(rewrite(text, argv[1], argv[2], argv[3]), encoding="utf-8")
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
