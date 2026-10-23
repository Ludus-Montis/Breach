"""HTTP-level tests for serve.py (specs/17-local-designer-app.md): a real server on an
ephemeral port over a temp repo, with a fake runner for the crate's tool."""

from __future__ import annotations

import json
import re
import sys
import tempfile
import threading
import unittest
import urllib.error
import urllib.request
from http.server import ThreadingHTTPServer
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from designer_repo import DesignerRepo  # noqa: E402
import serve  # noqa: E402
from serve import API_VERSION, make_handler  # noqa: E402
from test_designer_repo import MAP, FakeRunner  # noqa: E402


class ServeTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.static = self.root / "static"
        self.static.mkdir()
        (self.static / "index.html").write_text("<p>designer</p>", encoding="utf-8")
        self.runner = FakeRunner(stdout="wrote content/maps/m.json\n")
        self.launched = []
        repo = DesignerRepo(self.root, "godot", self.runner, launcher=self.launched.append)
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), make_handler(repo, self.static, log_requests=False))
        self.base = f"http://127.0.0.1:{self.server.server_address[1]}"
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self._tmp.cleanup()

    def request(self, method, path, body=None, headers=None):
        data = json.dumps(body).encode("utf-8") if body is not None else None
        all_headers = {"Content-Type": "application/json"} if data is not None else {}
        all_headers.update(headers or {})
        req = urllib.request.Request(self.base + path, data=data, method=method, headers=all_headers)
        try:
            with urllib.request.urlopen(req) as response:
                return response.status, response.read()
        except urllib.error.HTTPError as error:
            return error.code, error.read()

    def test_serves_the_designer_page(self):
        status, body = self.request("GET", "/")
        self.assertEqual(status, 200)
        self.assertIn(b"designer", body)

    def test_every_script_the_page_loads_is_served(self):
        page = (serve.DESIGNER_DIR / "index.html").read_text(encoding="utf-8")
        for src in re.findall(r'<script src="([^"]+)"', page):
            with self.subTest(src):
                self.assertIn("/" + src, serve.STATIC_FILES)

    def test_health_reports_godot(self):
        status, body = self.request("GET", "/api/health")
        self.assertEqual(status, 200)
        self.assertTrue(json.loads(body)["godot_found"])
        self.assertEqual(json.loads(body)["api_version"], API_VERSION)

    def test_given_a_map_when_put_then_it_is_saved_imported_listed_and_readable(self):
        status, body = self.request("PUT", "/api/maps/m", MAP)
        self.assertEqual(status, 200)
        self.assertTrue(json.loads(body)["import"]["ok"])
        self.assertEqual(len(self.runner.commands), 1)

        status, body = self.request("GET", "/api/maps")
        self.assertEqual([m["name"] for m in json.loads(body)], ["m"])
        status, body = self.request("GET", "/api/maps/m")
        self.assertEqual(json.loads(body), MAP)

    def test_missing_map_is_404(self):
        status, _ = self.request("GET", "/api/maps/absent")
        self.assertEqual(status, 404)

    def test_traversal_names_are_rejected(self):
        status, _ = self.request("PUT", "/api/maps/..%2Fescape", MAP)
        self.assertEqual(status, 400)
        self.assertFalse((self.root / "content").exists())

    def test_put_without_json_content_type_is_rejected(self):
        status, _ = self.request("PUT", "/api/maps/m", MAP, {"Content-Type": "text/plain"})
        self.assertEqual(status, 400)

    def test_cross_origin_requests_are_forbidden(self):
        status, _ = self.request("PUT", "/api/maps/m", MAP, {"Origin": "https://evil.example"})
        self.assertEqual(status, 403)
        status, _ = self.request("GET", "/api/maps", headers={"Host": "evil.example"})
        self.assertEqual(status, 403)

    def test_library_round_trip(self):
        value = [{"id": "player", "display_name": "Player"}]
        self.assertEqual(json.loads(self.request("GET", "/api/libraries/factions")[1]), None)
        status, body = self.request("PUT", "/api/libraries/factions", value)
        self.assertEqual((status, json.loads(body)), (200, {"changed": True, "import": None}))
        self.assertEqual(json.loads(self.request("GET", "/api/libraries/factions")[1]), value)

    def test_view_opens_the_viewer_for_an_imported_map(self):
        (self.root / "content/maps").mkdir(parents=True)
        (self.root / "content/maps/m.json").write_text("{}", encoding="utf-8")

        status, body = self.request("POST", "/api/maps/m/view", {})

        self.assertEqual(status, 200)
        self.assertTrue(json.loads(body)["launched"])
        self.assertEqual(self.launched[0][-1], "--map=m")

    def test_view_of_a_map_that_was_never_imported_is_400(self):
        status, body = self.request("POST", "/api/maps/absent/view", {})
        self.assertEqual(status, 400)
        self.assertIn("save the map first", json.loads(body)["error"])
        self.assertEqual(self.launched, [])

    def test_view_needs_post(self):
        self.assertEqual(self.request("GET", "/api/maps/m/view")[0], 400)

    def test_unknown_api_path_is_404_and_unknown_library_is_400(self):
        self.assertEqual(self.request("GET", "/api/nothing")[0], 404)
        self.assertEqual(self.request("GET", "/api/libraries/secrets")[0], 400)


if __name__ == "__main__":
    unittest.main()
