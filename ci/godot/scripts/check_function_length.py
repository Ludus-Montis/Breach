#!/usr/bin/env python3
"""srp-size (function-length half only - file-length is enforced by gdlint's
`max-file-lines` in .gdlintrc instead of being duplicated here), per
AI_First_Development_Kit/principles/solid-mechanical.md (S) and
AI_First_Development_Kit/config/thresholds.yaml (solid_mechanical.srp.max_function_lines).

Heuristic limits (see ci/godot/README.md): function-length is measured from a top-level
`func` line to the next top-level declaration (func/static func/class_name/signal/var/
const at zero indent) or end of file. A `static func` ends the function before it but
isn't measured itself yet - a blind spot like the inner-class one. Functions nested inside an inner `class` block are not measured
separately - this is a known blind spot, flagged rather than silently wrong, since
GDScript's indentation-only nesting makes a fully general parse more than this check
warrants before a real static-analysis tool is adopted.

Deliberately dependency-free (no pyyaml) so this hook doesn't need
`additional_dependencies` in .pre-commit-config.yaml - thresholds.yaml is simple enough
to read with a regex.
"""
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(
    subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
)
THRESHOLDS_FILE = REPO_ROOT / "AI_First_Development_Kit" / "config" / "thresholds.yaml"
EXCLUDED_DIRS = {".claude", "addons", "AI_First_Development_Kit"}


def read_threshold(key: str, default: int) -> int:
    text = THRESHOLDS_FILE.read_text(encoding="utf-8")
    match = re.search(rf"^\s*{re.escape(key)}:\s*(\d+)\s*$", text, re.MULTILINE)
    return int(match.group(1)) if match else default


def iter_gd_files():
    for path in REPO_ROOT.rglob("*.gd"):
        relative = path.relative_to(REPO_ROOT)
        if relative.parts and relative.parts[0] in EXCLUDED_DIRS:
            continue
        yield path


def check_function_lengths(path: Path, max_function_lines: int, violations: list):
    lines = path.read_text(encoding="utf-8").splitlines()
    func_start = None
    for index, line in enumerate(lines):
        is_top_level_decl = re.match(r"^(static func|func|class_name|signal|var|const|class)\b", line)
        if func_start is not None and (is_top_level_decl or index == len(lines) - 1):
            end = index if is_top_level_decl else index + 1
            length = end - func_start
            if length > max_function_lines:
                func_name = lines[func_start].strip()
                violations.append(
                    f"{path.relative_to(REPO_ROOT)}:{func_start + 1}: '{func_name}' "
                    f"is {length} lines (max {max_function_lines})"
                )
            func_start = None
        if re.match(r"^func\b", line):
            func_start = index


def main() -> int:
    max_function_lines = read_threshold("max_function_lines", 40)

    violations = []
    for path in iter_gd_files():
        check_function_lengths(path, max_function_lines, violations)

    if violations:
        print("check_function_length: srp-size (function) violations found:")
        for violation in violations:
            print(f"  {violation}")
        return 1

    print("check_function_length: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
