#!/usr/bin/env python3
"""Check the metrics records (spec 43 step 2, Decision 162).

Every `*.json` under the metrics folder, apart from `raw/` (source material, not records)
and `schema/` (the schemas themselves), is a record. Each must:
- be named `<date>-<run id>[-<n>].json` (`2026-11-07-19203344.json`, `...-2.json`);
- name its schema version in `schema`, one that has a file `<schema>.schema.json` in the
  schema folder, and validate against it;
- write a gap as `{"na": <reason>}`, never `0`: a measure's `values` or `median` of `0`
  fails, in `results` and in `reference.results` alike (an importer that found nothing
  writes a gap instead).

Usage: check_records.py [metrics-dir] [schema-dir]   (defaults: metrics, metrics/schema)
Prints one line per problem, naming the file and the field, and exits 1 if there is any.
Needs `jsonschema` (`pip install jsonschema`).
"""
import json
import re
import sys
from pathlib import Path

NAME = re.compile(r"^\d{4}-\d{2}-\d{2}-\d+(-\d+)?\.json$")
NOT_RECORDS = ("raw", "schema")


def records(root: Path) -> list:
    """Every record file under the metrics folder, in a stable order."""
    return sorted(
        path for path in root.rglob("*.json")
        if path.relative_to(root).parts[0] not in NOT_RECORDS
    )


def zero_gaps(record: dict) -> list:
    """The fields where a measure was written 0 in a gap's place."""
    found = []
    sections = [("results", record.get("results"))]
    reference = record.get("reference")
    if isinstance(reference, dict):
        sections.append(("reference/results", reference.get("results")))
    for section, results in sections:
        if not isinstance(results, dict) or "na" in results:
            continue
        for path, measure in results.items():
            if not isinstance(measure, dict):
                continue
            values = measure.get("values")
            if isinstance(values, list) and any(v == 0 for v in values):
                found.append(f"{section}/{path}: values holds 0; write a gap, {{\"na\": ...}}")
            if measure.get("median") == 0 and not isinstance(measure.get("median"), bool):
                found.append(f"{section}/{path}: median is 0; write a gap, {{\"na\": ...}}")
    return found


def schema_errors(record: dict, schema_dir: Path, validators: dict) -> list:
    """The record's schema problems, each as `<field>: <message>`."""
    version = record.get("schema")
    if not isinstance(version, str):
        return ["schema: missing; a record names its schema version"]
    if version not in validators:
        schema_file = schema_dir / f"{version}.schema.json"
        if not re.fullmatch(r"[a-z0-9-]+", version) or not schema_file.is_file():
            return [f"schema: unknown version {version!r}"]
        import jsonschema

        schema = json.loads(schema_file.read_text(encoding="utf-8"))
        validators[version] = jsonschema.Draft202012Validator(schema)
    errors = sorted(validators[version].iter_errors(record), key=lambda e: list(e.path))
    return [f"{'/'.join(str(p) for p in error.path) or '(record)'}: {error.message}"
            for error in errors]


def check_file(path: Path, root: Path, schema_dir: Path, validators: dict) -> list:
    shown = path.relative_to(root.parent)
    problems = []
    if not NAME.match(path.name):
        problems.append("file name: not <date>-<run id>[-<n>].json")
    try:
        record = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        return [f"{shown}: {p}" for p in problems + [f"not JSON ({error})"]]
    if not isinstance(record, dict):
        return [f"{shown}: {p}" for p in problems + ["(record): not a JSON object"]]
    problems += schema_errors(record, schema_dir, validators)
    problems += zero_gaps(record)
    return [f"{shown}: {p}" for p in problems]


def check_tree(root: Path, schema_dir: Path) -> list:
    validators = {}
    problems = []
    for path in records(root):
        problems += check_file(path, root, schema_dir, validators)
    return problems


def main(arguments: list) -> int:
    root = Path(arguments[0]) if arguments else Path("metrics")
    schema_dir = Path(arguments[1]) if len(arguments) > 1 else root / "schema"
    files = records(root) if root.is_dir() else []
    problems = check_tree(root, schema_dir) if files else []
    for problem in problems:
        print(problem)
    print(f"{len(files)} record(s) checked, {len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
