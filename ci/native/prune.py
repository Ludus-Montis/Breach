#!/usr/bin/env python3
"""The prune of the store of native binaries (spec 49 step 9, Decision 179).

    prune.py [--dry-run]

Run from a full checkout (all history and tags) with GH_TOKEN and GH_REPO set. Keeps:
- forever: the keys of the references (every full commit sha a workflow names, except a
  pinned action's) and of every release, and any asset labelled `release`;
- the keys of the last 20 distinct trees of native/ on main's first-parent history, and any
  asset labelled `main` among them;
- every other key (a PR's, a dispatch's) for 14 days.
Deletes the rest; then, while the store is past 1 GB, the oldest assets it doesn't keep
forever. A key is `<tree of native/>-<toolchain>-<profile>-<target>`, so the prune works by
tree: a key's Linux, Windows and profiling assets live and die together.

store.sh labels each asset with where it was first built (`main <sha>`, `pr <n> <sha>`,
`release <sha>`, `dispatch <sha>`). The upload skips a key already stored, so a key first
built on a PR keeps its `pr` label after the merge: main's history, not the label alone,
says which keys are main's.
"""

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path

STORE = "native-store"
KEEP_MAIN = 20
PR_DAYS = 14
CAP = 1024**3
KEY = re.compile(r"^([0-9a-f]{40})-.+-(release|profiling)-.+\.zip$")
SHA = re.compile(r"\b[0-9a-f]{40}\b")
ROOT = Path(__file__).resolve().parents[2]


@dataclass(frozen=True)
class Asset:
    id: int
    name: str
    label: str
    size: int
    created: datetime


def tree_of(asset):
    match = KEY.match(asset.name)
    return match.group(1) if match else None


def origin(label):
    return (label or "").split(" ", 1)[0]


def plan(assets, forever, main_history, now, cap=CAP, keep_main=KEEP_MAIN):
    """The assets to delete. `forever`: trees kept for good; `main_history`: main's trees
    of native/, newest first."""
    keys = [a for a in assets if tree_of(a)]
    kept_main = set()
    for tree in main_history:
        if len(kept_main) == keep_main:
            break
        kept_main.add(tree)
    on_main = set(main_history) | {tree_of(a) for a in keys if origin(a.label) == "main"}
    for asset in sorted(keys, key=lambda a: a.created, reverse=True):
        if len(kept_main) == keep_main:
            break
        if tree_of(asset) in on_main:
            kept_main.add(tree_of(asset))

    def kept_forever(asset):
        return tree_of(asset) in forever or origin(asset.label) == "release"

    def kept(asset):
        if kept_forever(asset):
            return True
        if tree_of(asset) in on_main:
            return tree_of(asset) in kept_main
        return now - asset.created <= timedelta(days=PR_DAYS)

    gone = [a for a in keys if not kept(a)]
    survivors = [a for a in keys if kept(a)]
    total = sum(a.size for a in assets) - sum(a.size for a in gone)
    for asset in sorted((a for a in survivors if not kept_forever(a)),
                        key=lambda a: a.created):
        if total <= cap:
            break
        gone.append(asset)
        total -= asset.size
    return gone


def reference_shas(workflow_texts):
    return {sha for text in workflow_texts for line in text.splitlines()
            if "uses:" not in line for sha in SHA.findall(line)}


def run(*args, stdin=None):
    return subprocess.run(args, check=True, capture_output=True, text=True,
                          input=stdin, cwd=ROOT).stdout


def native_trees(revisions):
    """The tree of native/ at each revision, in order; one that has none is left out."""
    if not revisions:
        return []
    out = run("git", "cat-file", "--batch-check=%(objectname) %(objecttype)",
              stdin="".join(f"{r}:native\n" for r in revisions))
    return [line.split()[0] for line in out.splitlines() if line.endswith(" tree")]


def gh_api(path, *args):
    return run("gh", "api", f"repos/{os.environ['GH_REPO']}/{path}", *args)


def store_assets():
    release = json.loads(gh_api(f"releases/tags/{STORE}"))
    pages = gh_api(f"releases/{release['id']}/assets?per_page=100", "--paginate",
                   "--jq", ".[]")
    return [Asset(a["id"], a["name"], a.get("label") or "", a["size"],
                  datetime.fromisoformat(a["created_at"].replace("Z", "+00:00")))
            for a in map(json.loads, pages.splitlines())]


def forever_trees():
    texts = [p.read_text() for p in sorted((ROOT / ".github/workflows").glob("*.y*ml"))]
    tags = gh_api("releases?per_page=100", "--paginate", "--jq",
                  f'.[] | select(.tag_name != "{STORE}") | .tag_name').split()
    return set(native_trees(sorted(reference_shas(texts)) + [f"{t}^{{commit}}" for t in tags]))


def main_history():
    commits = run("git", "rev-list", "--first-parent", "origin/main", "--", "native").split()
    return list(dict.fromkeys(native_trees(["origin/main", *commits])))


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dry-run", action="store_true", help="list, delete nothing")
    args = parser.parse_args(argv)
    assets = store_assets()
    gone = plan(assets, forever_trees(), main_history(), datetime.now(timezone.utc))
    lines = [f"- `{a.name}` ({a.label or 'no label'}, {a.size} bytes)" for a in gone]
    kept_bytes = sum(a.size for a in assets) - sum(a.size for a in gone)
    verb = "Would delete" if args.dry_run else "Deleted"
    summary = [f"### Native store prune: {verb.lower()} {len(gone)} of {len(assets)} assets,"
               f" {kept_bytes} bytes kept", *lines]
    for asset in gone if not args.dry_run else []:
        gh_api(f"releases/assets/{asset.id}", "-X", "DELETE")
    print("\n".join(summary))
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as out:
            out.write("\n".join(summary) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
