"""Local server for the Lane Tile Designer (specs/17-local-designer-app.md, Decision 30).

    python tools/designer/serve.py [--port 8765] [--godot <path>] [--no-browser]

Serves the designer from tools/designer/ on 127.0.0.1 only and gives it a small JSON API.
Saving a map writes content/maps_src/<name>.designer.json, then runs the Rust crate's
designer tool (native/bin/breach_designer, built by bash native/build.sh), which writes
content/maps/<name>.json in the crate's schema, so the map the game loads is always current.
Godot is only needed to open the map viewer. Python standard library only.

API:
    GET  /api/health                  repo path, whether Godot is configured and the tool built
    GET  /api/maps                    [{name, modified, imported}]
    GET  /api/maps/<name>             the saved export JSON
    PUT  /api/maps/<name>             save + import -> {saved, import:{ran, ok, warnings, errors, output_path}}
    POST /api/maps/<name>/view        open content/maps/<name>.json in the Godot map viewer
    GET  /api/libraries/<library>     content/designer/<library>.json (null if absent); "terrain"
                                      is content/terrain/terrain_library.json, in the designer's shape
    PUT  /api/libraries/<library>     write it -> {changed, import}; "terrain" is written in the
                                      crate's schema, and refused (import.ok false) if it drops a
                                      terrain or feature a saved map uses
    GET  /api/registry                the tags, traits and statuses (content/registry), each
                                      trait's targets, and where each is used (Decision 128)
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import threading
import webbrowser
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlparse

sys.path.insert(0, str(Path(__file__).resolve().parent))
from designer_repo import DesignerRepo, find_godot  # noqa: E402
from content_library import read_registry  # noqa: E402

DESIGNER_DIR = Path(__file__).resolve().parent
REPO_ROOT = DESIGNER_DIR.parents[1]
STATIC_FILES = {
    "/": ("index.html", "text/html; charset=utf-8"),
    "/index.html": ("index.html", "text/html; charset=utf-8"),
    "/designer.css": ("designer.css", "text/css; charset=utf-8"),
    "/designer.js": ("designer.js", "text/javascript; charset=utf-8"),
    "/repo.js": ("repo.js", "text/javascript; charset=utf-8"),
    "/load_paths.js": ("load_paths.js", "text/javascript; charset=utf-8"),
    "/planner_tools.js": ("planner_tools.js", "text/javascript; charset=utf-8"),
    "/planner_draw.js": ("planner_draw.js", "text/javascript; charset=utf-8"),
    "/planner_examples.js": ("planner_examples.js", "text/javascript; charset=utf-8"),
    "/planner_subnodes.js": ("planner_subnodes.js", "text/javascript; charset=utf-8"),
    "/planner.js": ("planner.js", "text/javascript; charset=utf-8"),
    "/library.html": ("library.html", "text/html; charset=utf-8"),
    "/library.js": ("library.js", "text/javascript; charset=utf-8"),
}
MAX_BODY_BYTES = 16 * 1024 * 1024
## Bumped whenever the API gains or changes an endpoint; repo.js compares it with its own
## REQUIRED_API so a server started before an update says "restart serve.py".
API_VERSION = 5


def make_handler(repo: DesignerRepo, static_dir: Path = DESIGNER_DIR, log_requests: bool = True):
    write_lock = threading.Lock()

    class Handler(BaseHTTPRequestHandler):
        server_version = "BreachDesigner/1"

        def log_message(self, fmt, *args):  # quieter than the default per-request line
            if log_requests and not self.path.startswith("/api/libraries"):
                sys.stderr.write("designer: " + (fmt % args) + "\n")

        def do_GET(self):
            if not self._local_request():
                return
            path = urlparse(self.path).path
            if path in STATIC_FILES:
                self._send_static(*STATIC_FILES[path])
                return
            self._api("GET", path)

        def do_PUT(self):
            if not self._local_request():
                return
            self._api("PUT", urlparse(self.path).path)

        def do_POST(self):
            if not self._local_request():
                return
            self._api("POST", urlparse(self.path).path)

        def _api(self, method, path):
            parts = [p for p in path.split("/") if p]
            try:
                if parts == ["api", "health"] and method == "GET":
                    self._send_json(
                        {
                            "repo": str(repo.root),
                            "godot": repo.godot,
                            "godot_found": bool(repo.godot),
                            "tool_found": repo.tool.is_file(),
                            "api_version": API_VERSION,
                        }
                    )
                elif parts == ["api", "registry"] and method == "GET":
                    self._send_json(read_registry(repo.root))
                elif parts == ["api", "maps"] and method == "GET":
                    self._send_json(repo.list_maps())
                elif len(parts) == 4 and parts[:2] == ["api", "maps"] and parts[3] == "view":
                    if method != "POST":
                        raise ValueError("use POST to open the viewer")
                    self._send_json(repo.view_map(parts[2]))
                elif len(parts) == 3 and parts[:2] == ["api", "maps"] and method != "POST":
                    self._maps(method, parts[2])
                elif len(parts) == 3 and parts[:2] == ["api", "libraries"] and method != "POST":
                    self._libraries(method, parts[2])
                else:
                    self._send_json({"error": "not found"}, HTTPStatus.NOT_FOUND)
            except ValueError as error:
                self._send_json({"error": str(error)}, HTTPStatus.BAD_REQUEST)

        def _maps(self, method, name):
            if method == "GET":
                found = repo.read_map(name)
                if found is None:
                    self._send_json({"error": f"no map named {name!r}"}, HTTPStatus.NOT_FOUND)
                else:
                    self._send_json(found)
                return
            body = self._read_body()
            with write_lock:
                self._send_json(repo.save_map(name, body))

        def _libraries(self, method, library):
            if method == "GET":
                self._send_json(repo.read_library(library))
                return
            body = self._read_body()
            with write_lock:
                self._send_json(repo.save_library(library, body))

        def _local_request(self) -> bool:
            """Blocks DNS-rebinding and cross-site writes: only this machine's own pages."""
            host = (self.headers.get("Host") or "").split(":")[0]
            origin = self.headers.get("Origin")
            origin_host = urlparse(origin).hostname if origin else None
            if host in ("127.0.0.1", "localhost") and origin_host in (None, "127.0.0.1", "localhost"):
                return True
            self._send_json({"error": "forbidden"}, HTTPStatus.FORBIDDEN)
            return False

        def _read_body(self):
            if "application/json" not in (self.headers.get("Content-Type") or ""):
                raise ValueError("Content-Type must be application/json")
            length = int(self.headers.get("Content-Length") or 0)
            if length <= 0 or length > MAX_BODY_BYTES:
                raise ValueError(f"body must be 1..{MAX_BODY_BYTES} bytes")
            try:
                return json.loads(self.rfile.read(length).decode("utf-8"))
            except json.JSONDecodeError as error:
                raise ValueError(f"body is not valid JSON: {error}") from error

        def _send_static(self, filename, content_type):
            payload = (static_dir / filename).read_bytes()
            self._send(payload, content_type, HTTPStatus.OK)

        def _send_json(self, value, status=HTTPStatus.OK):
            payload = json.dumps(value).encode("utf-8")
            self._send(payload, "application/json; charset=utf-8", status)

        def _send(self, payload, content_type, status):
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(payload)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(payload)

    return Handler


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Serve the Lane Tile Designer for this repo.")
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--godot", help="Godot 4 executable (else GODOT_BIN, else local_config.json)")
    parser.add_argument("--no-browser", action="store_true", help="don't open a browser tab")
    args = parser.parse_args(argv)

    godot = find_godot(args.godot, dict(os.environ), DESIGNER_DIR / "local_config.json")
    repo = DesignerRepo(REPO_ROOT, godot)
    server = ThreadingHTTPServer(("127.0.0.1", args.port), make_handler(repo))
    url = f"http://127.0.0.1:{server.server_address[1]}/"
    print(f"Lane Tile Designer: {url}  (repo {REPO_ROOT})")
    if not repo.tool.is_file():
        print(f"Designer tool: not built ({repo.tool}) - run bash native/build.sh, or saves fail.")
    if godot:
        print(f"Godot: {godot} - View in Godot opens the map viewer")
    else:
        print(
            "Godot: not configured - maps save, but View in Godot can't open the viewer.\n"
            '       Pass --godot <path>, set GODOT_BIN, or add {"godot": "<path>"} to\n'
            "       tools/designer/local_config.json (gitignored)."
        )
    print("Ctrl+C to stop.")
    if not args.no_browser:
        webbrowser.open(url)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
