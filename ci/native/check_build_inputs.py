"""Fail if a build input of the native core is set outside native/ (spec 49 step 2).

The store keys a binary by the tree of native/, the pinned toolchain, the profile and the
target (Decision 179). A flag set elsewhere (a workflow's RUSTFLAGS, a script's
--features) would change a build without changing its key, so a stored binary could be
used for code it wasn't built from. This check keeps every such input under native/.
The store's own helper (ci/native/) may set RUSTUP_TOOLCHAIN to the pin, so it is exempt.

Usage: python3 ci/native/check_build_inputs.py   (scans the files git tracks)
"""
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

# Paths that may set build inputs: the crate itself, and the store's helper.
ALLOWED_PREFIXES = ("native/", "ci/native/")

# Files that run or configure a build: workflows, scripts, Cargo configs. Prose (.md) isn't.
SCANNED_SUFFIXES = {".yml", ".yaml", ".sh", ".bash", ".py", ".ps1", ".bat", ".cmd", ".toml"}

BUILD_INPUT = re.compile(
    r"RUSTFLAGS|CARGO_PROFILE_\w*|CARGO_BUILD_\w*|--features\b|target-cpu|RUSTUP_TOOLCHAIN"
)


@dataclass(frozen=True)
class Finding:
    path: str
    line: int
    text: str


def _scanned(rel: str) -> bool:
    return not rel.startswith(ALLOWED_PREFIXES) and Path(rel).suffix in SCANNED_SUFFIXES


def find_build_inputs(root: Path, paths: list[str]) -> list[Finding]:
    """Every line, in the scanned files among `paths`, that sets a native build input."""
    findings = []
    for rel in sorted(p for p in paths if _scanned(p)):
        try:
            text = (root / rel).read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for number, line in enumerate(text.splitlines(), start=1):
            if BUILD_INPUT.search(line):
                findings.append(Finding(rel, number, line.strip()))
    return findings


def tracked_files(root: Path) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files"], cwd=root, check=True, capture_output=True, text=True
    ).stdout
    return out.splitlines()


def main() -> int:
    findings = find_build_inputs(REPO_ROOT, tracked_files(REPO_ROOT))
    for f in findings:
        print(f"{f.path}:{f.line}: sets a native build input outside native/: {f.text}")
    if findings:
        print(
            "Move the setting under native/ (native/build.sh, native/rust/Cargo.toml or "
            "the toolchain pin) so the store's key covers it (Decision 179)."
        )
        return 1
    print("check_build_inputs: no native build input is set outside native/.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
