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
    find_godot,
    is_valid_name,
    parse_import_output,
    to_native_path,
)

MAP = {"format": "breach-designer-map", "format_version": 1, "map_name": "Test", "nodes": []}


class FakeRunner:
    """Stands in for subprocess.run; records the command and returns a canned result."""

    def __init__(self, stdout="", stderr="", returncode=0, raises=None):
        self.stdout, self.stderr, self.returncode, self.raises = stdout, stderr, returncode, raises
        self.commands = []

    def __call__(self, command, **kwargs):
        self.commands.append(command)
        if self.raises:
            raise self.raises
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
    def test_given_a_map_when_saved_then_source_json_is_written_and_import_runs(self):
        runner = FakeRunner(stdout="warning: tiles not imported yet\nwrote res://content/maps/m.tres\n")
        repo = DesignerRepo(self.root, "godot", runner)

        result = repo.save_map("m", MAP)

        source = self.root / "content/maps_src/m.designer.json"
        self.assertEqual(json.loads(source.read_text(encoding="utf-8")), MAP)
        self.assertEqual(result["saved"], "content/maps_src/m.designer.json")
        self.assertTrue(result["import"]["ok"])
        self.assertEqual(result["import"]["warnings"], ["tiles not imported yet"])
        command = runner.commands[0]
        self.assertEqual(command[:3], ["godot", "--headless", "--path"])
        self.assertIn("res://tools/import_designer_map.gd", command)
        self.assertEqual(
            command[-2:], ["res://content/maps_src/m.designer.json", "res://content/maps/m.tres"]
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

    def test_without_godot_the_map_still_saves_and_import_reports_why_it_did_not_run(self):
        runner = FakeRunner()
        result = DesignerRepo(self.root, None, runner).save_map("m", MAP)
        self.assertTrue((self.root / "content/maps_src/m.designer.json").is_file())
        self.assertFalse(result["import"]["ran"])
        self.assertIn("GODOT_BIN", result["import"]["errors"][0])
        self.assertEqual(runner.commands, [])

    def test_missing_godot_binary_is_reported_not_raised(self):
        runner = FakeRunner(raises=FileNotFoundError())
        result = DesignerRepo(self.root, "nope.exe", runner).save_map("m", MAP)
        self.assertFalse(result["import"]["ok"])
        self.assertIn("nope.exe", result["import"]["errors"][0])


class ListAndReadMapsTest(RepoTestCase):
    def test_lists_saved_maps_with_their_import_state(self):
        repo = DesignerRepo(self.root, "godot", FakeRunner())
        repo.save_map("b", MAP)
        repo.save_map("a", MAP)
        (self.root / "content/maps").mkdir(parents=True)
        (self.root / "content/maps/a.tres").write_text("x", encoding="utf-8")

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
        (self.root / "content/maps" / f"{name}.tres").write_text("x", encoding="utf-8")

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
        self.assertEqual(command[-1], "--map=res://content/maps/m.tres")

    def test_without_godot_or_without_the_tres_nothing_is_launched(self):
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
    """Spec 39 step 2b: the terrain library is content/terrain/terrain_library.json, in the
    crate's schema; the designer reads and writes it in its own shape, and Godot isn't run."""

    def _schema_file(self):
        return self.root / "content/terrain/terrain_library.json"

    def test_saving_the_terrain_library_writes_the_crates_schema(self):
        runner = FakeRunner()
        repo = DesignerRepo(self.root, "godot", runner)

        result = repo.save_library("terrain", {"terrains": [{"id": "FIELDS", "label": "Fields"}]})

        self.assertTrue(result["changed"])
        self.assertTrue(result["import"]["ok"])
        written = json.loads(self._schema_file().read_text(encoding="utf-8"))
        self.assertEqual(written["terrains"][0]["display_name"], "Fields")
        self.assertEqual(runner.commands, [])
        self.assertFalse((self.root / "content/designer/terrain.json").exists())

    def test_the_designer_reads_its_own_shape_back(self):
        repo = DesignerRepo(self.root, None)
        repo.save_library("terrain", {"terrains": [{"id": "FIELDS", "label": "Fields"}]})
        self.assertEqual(repo.read_library("terrain")["terrains"][0]["label"], "Fields")

    def test_an_unchanged_library_reports_no_change(self):
        repo = DesignerRepo(self.root, None)
        repo.save_library("terrain", {"terrains": [{"id": "FIELDS"}]})
        self.assertFalse(repo.save_library("terrain", {"terrains": [{"id": "FIELDS"}]})["changed"])

    def test_dropping_a_terrain_a_saved_map_uses_is_refused_and_nothing_is_written(self):
        repo = DesignerRepo(self.root, None)
        repo.save_library("terrain", {"terrains": [{"id": "FIELDS"}]})
        repo.save_map("demo_map", dict(MAP, default_terrain="FIELDS"))
        before = self._schema_file().read_text(encoding="utf-8")

        result = repo.save_library("terrain", {"terrains": [{"id": "FOREST"}]})

        self.assertFalse(result["changed"])
        self.assertFalse(result["import"]["ok"])
        self.assertIn("terrain 'FIELDS' is still used by demo_map", result["import"]["errors"][0])
        self.assertEqual(self._schema_file().read_text(encoding="utf-8"), before)

    def test_saving_a_map_runs_only_the_map_import(self):
        runner = FakeRunner(stdout="wrote x\n")
        repo = DesignerRepo(self.root, "godot", runner)
        repo.save_library("terrain", {"terrains": [{"id": "FIELDS"}]})

        repo.save_map("m", MAP)

        self.assertEqual(len(runner.commands), 1)
        self.assertIn("res://tools/import_designer_map.gd", runner.commands[0])


class ImportOutputTest(unittest.TestCase):
    def test_parses_warnings_errors_and_written_path(self):
        parsed = parse_import_output(
            "Godot Engine v4.7.2\nwarning: a\nwarning: b\n", "error: bad node\n", 1
        )
        self.assertEqual(parsed["warnings"], ["a", "b"])
        self.assertEqual(parsed["errors"], ["bad node"])
        self.assertFalse(parsed["ok"])

    def test_success_records_the_output_path(self):
        parsed = parse_import_output("wrote res://content/maps/m.tres\n", "", 0)
        self.assertTrue(parsed["ok"])
        self.assertEqual(parsed["output_path"], "res://content/maps/m.tres")

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
