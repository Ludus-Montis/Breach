# Lane Tile Designer

The map authoring tool (Decision 30, `specs/17-local-designer-app.md`).

```
python tools/designer/serve.py
```

This opens `http://127.0.0.1:8765/`. Python 3.10+ is required, standard library only.

## What Save does

**Save** (Ctrl+S) writes `content/maps_src/<name>.designer.json`, the designer's export.
It then runs the Godot import (`tools/import_designer_map.gd`), which writes
`content/maps/<name>.tres`. The header shows the result. If the import fails, a dialog
lists its errors; the JSON is still saved, but the `.tres` is not updated.

**Open…** (Ctrl+O) lists the saved maps. Any `*.designer.json` export opens, including
older copy-pasted ones.

## Godot

The import needs a Godot 4.7 binary. The server looks for it in this order:
1. `--godot <path>`
2. the `GODOT_BIN` environment variable (the same one `ci/godot/scripts/run_tests.sh` uses)
3. `tools/designer/local_config.json` (gitignored), for example:
   `{"godot": "F:/Code/Godot_v4.7.2-stable_win64.exe"}`

Without Godot, maps still save and the designer says the import didn't run.

## Libraries

The factions, emplacements, room features, room prefabs and structure prefabs libraries
live in `content/designer/*.json`. The terrain library is the game's own file,
`content/terrain/terrain_library.json`, in the Rust crate's schema
(`content/schema/terrain_library.schema.json`, spec 39 step 2b): the server translates it to
and from the designer's shape (`terrain_schema.py`), and refuses an edit that drops a terrain
or feature a saved map uses. The designer writes them as you edit, so commit them along with
your maps. Opening a map adds any library entries it uses that
are missing locally.

To bring libraries across from the old claude.ai designer:
1. Press **Copy libraries JSON** there.
2. In this app, open **Import libraries…** and paste.

## Library

**Library** in the view bar opens a page that browses the content registry
(`content/registry/*.json`, Decision 128). It lists the tags, traits and statuses, each
trait's targets (where its tags agree, narrowed by its "only"), and the content files
using each id. You can search, filter by tag chips or by target, or show only the ids in
use. The registry itself is edited in its JSON files; Godot's tests check content against
it.

## Files

- `index.html`, `designer.css`, `designer.js`: the designer itself.
- `repo.js`: repo mode (Open/Save, import status, library sync). Opening `index.html`
  from disk skips this, and the designer then uses browser storage only.
- `library.html`, `library.js`, `content_library.py`: the Library page and the registry
  reader behind `GET /api/registry`.
- `serve.py`, `designer_repo.py`: the local server.
- `test_*.py`: the server's tests
  (`python -m unittest discover -s tools/designer -p "test_*.py"`).
