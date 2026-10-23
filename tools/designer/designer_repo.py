"""Repo-side operations behind the designer's local server (specs/17-local-designer-app.md).

Everything here touches only three fixed folders under the repo root, by whitelisted
name, so a request can never reach an arbitrary path:

- content/maps_src/<name>.designer.json  - the designer's saved map (its export JSON)
- content/maps/<name>.json               - the map in the Rust crate's schema, which the game
                                           loads (spec 39 step 2c)
- content/designer/<library>.json        - the designer's shared libraries
- content/terrain/terrain_library.json   - the terrain library, in the crate's schema

The crate's tool (native/bin/breach_designer, from bash native/build.sh) reads the designer's
map and terrain library into the schema and back (native/rust/src/designer/); Godot is only
needed to open the map viewer.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
from pathlib import Path

NAME_PATTERN = re.compile(r"^[A-Za-z0-9_-]{1,64}$")
LIBRARIES = (
    "factions",
    "default_relations",
    "terrain",
    "emplacements",
    "room_features",
    "room_prefabs",
    "structure_prefabs",
)
MAP_SUFFIX = ".designer.json"
VIEWER_SCENE = "res://presentation/map_viewer.tscn"
TOOL_TIMEOUT_SECONDS = 60


def is_valid_name(name: str) -> bool:
    return bool(NAME_PATTERN.match(name or ""))


def write_json(path: Path, value) -> bool:
    """Writes pretty JSON with LF endings; returns False (and leaves the file) if unchanged."""
    text = json.dumps(value, indent=2, ensure_ascii=False) + "\n"
    if path.exists() and path.read_text(encoding="utf-8") == text:
        return False
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(text)
    return True


def read_json(path: Path):
    if not path.is_file():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def to_native_path(raw: str) -> str:
    """Git Bash exports paths like /f/Code/godot.exe; Windows needs F:/Code/godot.exe."""
    match = re.match(r"^/([A-Za-z])/(.*)$", raw)
    if os.name == "nt" and match:
        return f"{match.group(1).upper()}:/{match.group(2)}"
    return raw


def find_godot(cli_value: str | None, env: dict, config_path: Path) -> str | None:
    """First of: --godot, GODOT_BIN, local_config.json's "godot". None if none is set."""
    config = read_json(config_path) if config_path.is_file() else None
    for candidate in (cli_value, env.get("GODOT_BIN"), (config or {}).get("godot")):
        if candidate:
            return to_native_path(candidate)
    return None


def default_tool(root: Path) -> Path:
    """The crate's designer tool that bash native/build.sh puts in native/bin/."""
    return root / "native" / "bin" / ("breach_designer.exe" if os.name == "nt" else "breach_designer")


def parse_import_output(stdout: str, stderr: str, returncode: int) -> dict:
    """Reads the crate tool's `warning:` / `error:` / `wrote` lines."""
    warnings, errors, wrote = [], [], None
    for line in (stdout + "\n" + stderr).splitlines():
        line = line.strip()
        if line.startswith("warning: "):
            warnings.append(line[len("warning: "):])
        elif line.startswith("error: "):
            errors.append(line[len("error: "):])
        elif line.startswith("wrote "):
            wrote = line[len("wrote "):]
    if returncode != 0 and not errors:
        tail = [x for x in (stderr or stdout).strip().splitlines() if x.strip()][-8:]
        errors.append(f"the designer tool exited with code {returncode}: " + (" | ".join(tail) or "no output"))
    return {
        "ran": True,
        "ok": returncode == 0 and not errors,
        "warnings": warnings,
        "errors": errors,
        "output_path": wrote,
    }


class DesignerRepo:
    """The designer's view of one repo checkout."""

    def __init__(self, root: Path, godot: str | None, runner=subprocess.run, launcher=None, tool=None):
        self.root = root
        self.godot = godot
        self.tool = Path(tool) if tool else default_tool(root)
        self._runner = runner
        self._launcher = launcher or _launch_detached
        self.content = root / "content"
        self.maps_src = self.content / "maps_src"
        self.maps_out = self.content / "maps"
        self.libraries = self.content / "designer"
        self.terrain_library = self.content / "terrain" / "terrain_library.json"

    def list_maps(self) -> list:
        if not self.maps_src.is_dir():
            return []
        out = []
        for path in sorted(self.maps_src.glob("*" + MAP_SUFFIX)):
            name = path.name[: -len(MAP_SUFFIX)]
            if is_valid_name(name):
                out.append(
                    {
                        "name": name,
                        "modified": path.stat().st_mtime,
                        "imported": (self.maps_out / f"{name}.json").is_file(),
                    }
                )
        return out

    def read_map(self, name: str):
        self._require_name(name)
        return read_json(self.maps_src / f"{name}{MAP_SUFFIX}")

    def save_map(self, name: str, export_data: dict) -> dict:
        """Writes the source JSON, then has the crate write content/maps/<name>.json from it."""
        self._require_name(name)
        if not isinstance(export_data, dict) or export_data.get("format") != "breach-designer-map":
            raise ValueError('body is not a designer map (format must be "breach-designer-map")')
        source = self.maps_src / f"{name}{MAP_SUFFIX}"
        write_json(source, export_data)
        return {
            "saved": source.relative_to(self.root).as_posix(),
            "import": self.run_import(name),
        }

    def run_import(self, name: str) -> dict:
        self._require_name(name)
        source = self.maps_src / f"{name}{MAP_SUFFIX}"
        out = self.maps_out / f"{name}.json"
        imported = self._run_tool(["map", str(source), str(out), str(self.content)])
        imported.pop("stdout", None)
        return imported

    def _run_tool(self, args: list, stdin: str | None = None) -> dict:
        try:
            done = self._runner(
                [str(self.tool)] + args,
                capture_output=True,
                text=True,
                input=stdin,
                timeout=TOOL_TIMEOUT_SECONDS,
            )
        except FileNotFoundError:
            return self._failed(f"the designer tool isn't built ({self.tool}): run bash native/build.sh")
        except subprocess.TimeoutExpired:
            return self._failed(f"the designer tool timed out after {TOOL_TIMEOUT_SECONDS}s")
        parsed = parse_import_output(done.stdout or "", done.stderr or "", done.returncode)
        parsed["stdout"] = done.stdout or ""
        return parsed

    def view_map(self, name: str) -> dict:
        """Opens the Godot map viewer (specs/18) on content/maps/<name>.json."""
        self._require_name(name)
        if not self.godot:
            raise ValueError("Godot binary not configured, so the viewer can't be opened")
        if not (self.maps_out / f"{name}.json").is_file():
            raise ValueError(f"content/maps/{name}.json doesn't exist yet - save the map first")
        command = [self.godot, "--path", str(self.root), VIEWER_SCENE, "--", f"--map={name}"]
        try:
            self._launcher(command)
        except OSError as error:
            raise ValueError(f"could not start Godot ({self.godot}): {error}") from error
        return {"launched": True, "map": f"content/maps/{name}.json"}

    def read_library(self, library: str):
        """A library, or None if it has no file yet. The terrain library is read by the crate
        into the designer's shape."""
        self._require_library(library)
        if library != "terrain":
            return read_json(self.libraries / f"{library}.json")
        if not self.terrain_library.is_file():
            return None
        read = self._run_tool(["terrain-read", str(self.content)])
        if not read["ok"]:
            raise ValueError("; ".join(read["errors"]))
        return json.loads(read["stdout"])

    def write_library(self, library: str, value) -> bool:
        self._require_library(library)
        if library == "terrain":
            return self.save_library(library, value)["changed"]
        return write_json(self.libraries / f"{library}.json", value)

    def save_library(self, library: str, value) -> dict:
        """write_library; the crate writes the terrain library in its schema, and refuses it
        (writing nothing) if it drops a terrain or feature a saved map uses, or doesn't fit."""
        if library != "terrain":
            return {"changed": self.write_library(library, value), "import": None}
        before = self.terrain_library.read_bytes() if self.terrain_library.is_file() else None
        written = self._run_tool(
            ["terrain-write", str(self.content), str(self.maps_src)], json.dumps(value)
        )
        after = self.terrain_library.read_bytes() if self.terrain_library.is_file() else None
        written.pop("stdout", None)
        return {"changed": written["ok"] and after != before, "import": written}

    @staticmethod
    def _failed(message: str) -> dict:
        return {"ran": False, "ok": False, "warnings": [], "errors": [message], "output_path": None}

    @staticmethod
    def _require_name(name: str) -> None:
        if not is_valid_name(name):
            raise ValueError(f"invalid map name {name!r}: use letters, digits, '_' and '-' only")

    @staticmethod
    def _require_library(library: str) -> None:
        if library not in LIBRARIES:
            raise ValueError(f"unknown library {library!r}")


def _launch_detached(command: list) -> None:
    """Starts Godot without tying it to the server (closing either leaves the other)."""
    flags = 0
    if os.name == "nt":
        flags = subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP
    subprocess.Popen(  # noqa: S603 - fixed argv, whitelisted map name
        command,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        creationflags=flags,
        start_new_session=os.name != "nt",
    )
