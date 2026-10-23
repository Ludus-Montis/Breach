"""Tests for designer_repo.py (specs/17-local-designer-app.md). Run:

    python -m unittest discover -s tools/designer -p "test_*.py"
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from designer_repo import (  # noqa: E402
    DesignerRepo,
    default_tool,
    find_godot,
    is_valid_name,
    parse_import_output,
    to_native_path,
)

MAP = {"format": "breach-designer-map", "format_version": 1, "map_name": "Test", "nodes": []}


REPO_ROOT = Path(__file__).resolve().parents[2]
BUILT_TOOL = default_tool(REPO_ROOT)


class FakeRunner:
    """Stands in for subprocess.run; records the command and its stdin, runs `effect` (what
    the tool would have done), and returns a canned result."""

    def __init__(self, stdout="", stderr="", returncode=0, raises=None, effect=None):
        self.stdout, self.stderr, self.returncode, self.raises = stdout, stderr, returncode, raises
        self.effect = effect
        self.commands = []
        self.inputs = []

    def __call__(self, command, **kwargs):
        self.commands.append(command)
        self.inputs.append(kwargs.get("input"))
        if self.raises:
            raise self.raises
        if self.effect:
            self.effect(command)
        return subprocess.CompletedProcess(command, self.returncode, self.stdout, self.stderr)


class RepoTestCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()


class NameValidationTest(unittest.TestCase):
    def test_accepts_plain_names(self):
        for name in ("test_map", "demo-map", "Map01"):
            self.assertTrue(is_valid_name(name), name)

    def test_rejects_traversal_and_separators(self):
        for name in ("", "../x", "a/b", "a\\b", "..", "a.b", "a b", "x" * 65):
            self.assertFalse(is_valid_name(name), name)


class SaveMapTest(RepoTestCase):
    def test_given_a_map_when_saved_then_source_json_is_written_and_the_crate_writes_the_map(self):
        runner = FakeRunner(stdout="warning: structures not imported yet\nwrote content/maps/m.json\n")
        repo = DesignerRepo(self.root, None, runner)

        result = repo.save_map("m", MAP)

        source = self.root / "content/maps_src/m.designer.json"
        self.assertEqual(json.loads(source.read_text(encoding="utf-8")), MAP)
        self.assertEqual(result["saved"], "content/maps_src/m.designer.json")
        self.assertTrue(result["import"]["ok"])
        self.assertEqual(result["import"]["warnings"], ["structures not imported yet"])
        content = self.root / "content"
        self.assertEqual(
            runner.commands[0],
            [str(default_tool(self.root)), "map", str(source), str(content / "maps/m.json"), str(content)],
        )

    def test_saved_json_is_pretty_with_lf_endings_and_a_trailing_newline(self):
        DesignerRepo(self.root, "godot", FakeRunner()).save_map("m", MAP)
        raw = (self.root / "content/maps_src/m.designer.json").read_bytes()
        self.assertNotIn(b"\r\n", raw)
        self.assertTrue(raw.endswith(b"}\n"))
        self.assertIn(b'\n  "format": ', raw)

    def test_invalid_name_is_rejected_before_anything_is_written(self):
        repo = DesignerRepo(self.root, "godot", FakeRunner())
        with self.assertRaises(ValueError):
            repo.save_map("../escape", MAP)
        self.assertFalse((self.root / "content").exists())

    def test_body_that_is_not_a_designer_map_is_rejected(self):
        repo = DesignerRepo(self.root, "godot", FakeRunner())
        with self.assertRaises(ValueError):
            repo.save_map("m", {"hello": "world"})

    def test_a_map_the_crate_refuses_is_saved_but_reported(self):
        runner = FakeRunner(stderr="error: no player home\n", returncode=1)
        result = DesignerRepo(self.root, None, runner).save_map("m", MAP)
        self.assertTrue((self.root / "content/maps_src/m.designer.json").is_file())
        self.assertFalse(result["import"]["ok"])
        self.assertEqual(result["import"]["errors"], ["no player home"])

    def test_an_unbuilt_tool_is_reported_not_raised(self):
        runner = FakeRunner(raises=FileNotFoundError())
        result = DesignerRepo(self.root, None, runner).save_map("m", MAP)
        self.assertFalse(result["import"]["ok"])
        self.assertIn("bash native/build.sh", result["import"]["errors"][0])


class ListAndReadMapsTest(RepoTestCase):
    def test_lists_saved_maps_with_their_import_state(self):
        repo = DesignerRepo(self.root, "godot", FakeRunner())
        repo.save_map("b", MAP)
        repo.save_map("a", MAP)
        (self.root / "content/maps").mkdir(parents=True)
        (self.root / "content/maps/a.json").write_text("{}", encoding="utf-8")

        listed = repo.list_maps()

        self.assertEqual([m["name"] for m in listed], ["a", "b"])
        self.assertEqual([m["imported"] for m in listed], [True, False])

    def test_read_map_round_trips_and_missing_map_is_none(self):
        repo = DesignerRepo(self.root, "godot", FakeRunner())
        repo.save_map("m", MAP)
        self.assertEqual(repo.read_map("m"), MAP)
        self.assertIsNone(repo.read_map("absent"))

    def test_no_maps_folder_lists_nothing(self):
        self.assertEqual(DesignerRepo(self.root, None).list_maps(), [])


class ViewMapTest(RepoTestCase):
    def _imported(self, name="m"):
        (self.root / "content/maps").mkdir(parents=True, exist_ok=True)
        (self.root / "content/maps" / f"{name}.json").write_text("{}", encoding="utf-8")

    def test_given_an_imported_map_when_viewed_then_godot_opens_the_viewer_on_it(self):
        launched = []
        self._imported()
        repo = DesignerRepo(self.root, "godot", FakeRunner(), launcher=launched.append)

        result = repo.view_map("m")

        self.assertTrue(result["launched"])
        command = launched[0]
        self.assertEqual(command[0], "godot")
        self.assertNotIn("--headless", command)
        self.assertIn("res://presentation/map_viewer.tscn", command)
        self.assertEqual(command[-1], "--map=m")

    def test_without_godot_or_without_the_map_file_nothing_is_launched(self):
        launched = []
        with self.assertRaises(ValueError):
            DesignerRepo(self.root, None, launcher=launched.append).view_map("m")
        with self.assertRaises(ValueError) as caught:
            DesignerRepo(self.root, "godot", launcher=launched.append).view_map("m")
        self.assertIn("save the map first", str(caught.exception))
        self.assertEqual(launched, [])

    def test_invalid_name_is_rejected(self):
        with self.assertRaises(ValueError):
            DesignerRepo(self.root, "godot", launcher=lambda c: None).view_map("../x")

    def test_a_godot_that_cannot_start_is_reported(self):
        self._imported()

        def fail(command):
            raise OSError("no such file")

        with self.assertRaises(ValueError) as caught:
            DesignerRepo(self.root, "godot", launcher=fail).view_map("m")
        self.assertIn("could not start Godot", str(caught.exception))


class LibraryTest(RepoTestCase):
    def test_library_round_trip_and_unchanged_write_reports_no_change(self):
        repo = DesignerRepo(self.root, None)
        value = [{"id": "player", "display_name": "Player"}]
        self.assertIsNone(repo.read_library("factions"))
        self.assertTrue(repo.write_library("factions", value))
        self.assertFalse(repo.write_library("factions", value))
        self.assertEqual(repo.read_library("factions"), value)
        self.assertTrue((self.root / "content/designer/factions.json").is_file())

    def test_unknown_library_is_rejected(self):
        repo = DesignerRepo(self.root, None)
        for library in ("../factions", "secrets", ""):
            with self.assertRaises(ValueError):
                repo.write_library(library, [])


class TerrainLibraryTest(RepoTestCase):
    """Spec 39 step 2b, read and written by the crate since 2c: the terrain library is
    content/terrain/terrain_library.json, in the crate's schema; the designer reads and writes
    it in its own shape through the crate's tool, and Godot isn't run."""

    def _schema_file(self):
        return self.root / "content/terrain/terrain_library.json"

    def _written(self, command):
        self._schema_file().parent.mkdir(parents=True, exist_ok=True)
        self._schema_file().write_text('{"terrains": []}\n', encoding="utf-8")

    def test_saving_the_terrain_library_gives_the_crate_the_designers_library(self):
        runner = FakeRunner(effect=self._written)
        repo = DesignerRepo(self.root, None, runner)
        value = {"terrains": [{"id": "FIELDS", "label": "Fields"}]}

        result = repo.save_library("terrain", value)

        self.assertTrue(result["changed"])
        self.assertTrue(result["import"]["ok"])
        content = self.root / "content"
        tool = str(default_tool(self.root))
        self.assertEqual(runner.commands[0], [tool, "terrain-write", str(content), str(content / "maps_src")])
        self.assertEqual(json.loads(runner.inputs[0]), value)
        self.assertFalse((self.root / "content/designer/terrain.json").exists())

    def test_an_unchanged_library_reports_no_change(self):
        repo = DesignerRepo(self.root, None, FakeRunner(effect=self._written))
        repo.save_library("terrain", {"terrains": []})
        self.assertFalse(repo.save_library("terrain", {"terrains": []})["changed"])

    def test_a_library_the_crate_refuses_is_reported_and_nothing_changes(self):
        refusal = "error: terrain 'FIELDS' is still used by demo_map - change those maps first\n"
        repo = DesignerRepo(self.root, None, FakeRunner(stderr=refusal, returncode=1))

        result = repo.save_library("terrain", {"terrains": [{"id": "FOREST"}]})

        self.assertFalse(result["changed"])
        self.assertFalse(result["import"]["ok"])
        self.assertIn("terrain 'FIELDS' is still used by demo_map", result["import"]["errors"][0])

    def test_the_designer_reads_its_own_shape_from_the_crate(self):
        self._written(None)
        runner = FakeRunner(stdout='{"terrains": [{"id": "FIELDS", "label": "Fields"}]}\n')
        repo = DesignerRepo(self.root, None, runner)
        self.assertEqual(repo.read_library("terrain")["terrains"][0]["label"], "Fields")
        self.assertEqual(runner.commands[0][1:], ["terrain-read", str(self.root / "content")])

    def test_no_library_file_reads_as_none(self):
        runner = FakeRunner()
        self.assertIsNone(DesignerRepo(self.root, None, runner).read_library("terrain"))
        self.assertEqual(runner.commands, [])


@unittest.skipUnless(BUILT_TOOL.is_file(), "the core isn't built (bash native/build.sh)")
class BuiltToolTest(RepoTestCase):
    """The real tool, on a copy of the repo's terrain library: a library and a map saved,
    read back and refused as the designer would see them."""

    def setUp(self):
        super().setUp()
        terrain = self.root / "content/terrain"
        terrain.mkdir(parents=True)
        library = REPO_ROOT / "content/terrain/terrain_library.json"
        (terrain / "terrain_library.json").write_text(library.read_text(encoding="utf-8"), encoding="utf-8")
        self.repo = DesignerRepo(self.root, None, tool=BUILT_TOOL)

    def test_the_library_reads_back_as_written(self):
        designer = self.repo.read_library("terrain")
        fields = next(t for t in designer["terrains"] if t["id"] == "FIELDS")
        self.assertEqual(fields["label"], "Fields")
        self.assertFalse(self.repo.save_library("terrain", designer)["changed"])

    def test_a_saved_map_is_written_in_the_schema_and_its_terrain_is_held(self):
        demo = json.loads((REPO_ROOT / "content/maps_src/demo_map.designer.json").read_text(encoding="utf-8"))

        result = self.repo.save_map("demo_map", demo)

        self.assertTrue(result["import"]["ok"], result["import"]["errors"])
        written = json.loads((self.root / "content/maps/demo_map.json").read_text(encoding="utf-8"))
        self.assertEqual(written["lanes"][0]["nodes"], ["p", "f", "F", "c"])
        designer = self.repo.read_library("terrain")
        designer["terrains"] = [t for t in designer["terrains"] if t["id"] != "FIELDS"]
        refused = self.repo.save_library("terrain", designer)
        self.assertFalse(refused["import"]["ok"])
        self.assertIn("terrain 'FIELDS' is still used by demo_map", refused["import"]["errors"][0])


class ImportOutputTest(unittest.TestCase):
    def test_parses_warnings_errors_and_written_path(self):
        parsed = parse_import_output(
            "Godot Engine v4.7.2\nwarning: a\nwarning: b\n", "error: bad node\n", 1
        )
        self.assertEqual(parsed["warnings"], ["a", "b"])
        self.assertEqual(parsed["errors"], ["bad node"])
        self.assertFalse(parsed["ok"])

    def test_success_records_the_output_path(self):
        parsed = parse_import_output("wrote content/maps/m.json\n", "", 0)
        self.assertTrue(parsed["ok"])
        self.assertEqual(parsed["output_path"], "content/maps/m.json")

    def test_nonzero_exit_without_error_lines_still_reports_an_error(self):
        parsed = parse_import_output("", "SCRIPT ERROR: Parse Error\n", 1)
        self.assertFalse(parsed["ok"])
        self.assertIn("SCRIPT ERROR: Parse Error", parsed["errors"][0])


class FindGodotTest(RepoTestCase):
    def test_order_is_cli_then_env_then_local_config(self):
        config = self.root / "local_config.json"
        config.write_text(json.dumps({"godot": "from-config"}), encoding="utf-8")
        self.assertEqual(find_godot("from-cli", {"GODOT_BIN": "from-env"}, config), "from-cli")
        self.assertEqual(find_godot(None, {"GODOT_BIN": "from-env"}, config), "from-env")
        self.assertEqual(find_godot(None, {}, config), "from-config")
        self.assertIsNone(find_godot(None, {}, self.root / "absent.json"))

    def test_git_bash_drive_paths_are_converted_on_windows_only(self):
        converted = to_native_path("/f/Code/godot.exe")
        if sys.platform == "win32":
            self.assertEqual(converted, "F:/Code/godot.exe")
        else:
            self.assertEqual(converted, "/f/Code/godot.exe")


if __name__ == "__main__":
    unittest.main()
