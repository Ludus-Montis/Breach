# The content schema

The simulation's content is JSON in the Rust crate's schema (Decision 153, spec 39 step 2).
The crate's serde structs (`native/rust/src/content/`) are the schema. The files here are
generated from them, for the designer and editors to validate against.

| File | Schema | Holds |
|---|---|---|
| `content/units/<id>.json` | `unit.schema.json` | a unit's sheet (`UnitDef`) |
| `content/weapons/<id>.json` | `weapon.schema.json` | a weapon (`WeaponDef`) |
| `content/armour/<id>.json` | `armour.schema.json` | armour (`ArmourDef`) |
| `content/tuning/battle_tuning.json` | `battle_tuning.schema.json` | the battle's tuning (`BattleTuning`) |
| `content/terrain/terrain_library.json` | `terrain_library.schema.json` | the one terrain library (`TerrainLibraryDef`): terrains, features, materials |
| `content/maps/<name>.json` | `map.schema.json` | a map (`MapDef`): nodes, lanes, edges, factions, loss groups, the layout |

- **Ids.** A file's id is its name without `.json`. A unit names its items by their path under
  `content/` without the extension (`"items": ["weapons/grem_bite"]`).
- **Names.** Every field has the name of its GDScript property. An item's shared fields
  (`ItemDef`: `item_name`, `slots`, `weight`, `tags`, `traits`...) sit under `"item"`.
- **Defaults.** A unit, weapon or armour file lists only the fields it changes. The others take
  the defaults in the schema, which equal GDScript's (`test_crate_content.gd` holds them
  equal). The tuning file names every field. Each terrain, feature, material, stratum,
  liquid body and heat transition in the terrain library lists only what it changes too.
- **Infinity.** A list of reals may hold `"inf"` or `"-inf"`, which JSON numbers can't.
- **Enums by name.** `preferred_position` is `"front"`, `"mid"` or `"back"`; a node's
  `node_type` is `"ORIGIN"`, `"FORT"`... and a plan face's `side` `"north"`, `"west"` or
  `"floor"`.
- **Maps.** Every node is listed once under `nodes`, and a lane names its nodes by id
  (`"nodes": ["p", "f", "F", "c"]`), so a node on several lanes is one node; a node on no lane
  is off-lane. A cell is `[col, row]`, a node's `position` `[x, y]` and a structure cell
  `[x, y, level]`. A plan's keyed cells (`solid_cells`, `fills`, `loads`) are `[cell, value]`
  pairs. `layout` and a node's `plan` and a tile's `bridge` are `null` when there is none.
- **Colours.** A terrain's or material's `color` is `"#rrggbb"` or `"#rrggbbaa"`.
- **The terrain library and the maps** are written by the map designer (`tools/designer/`),
  which edits them in its own shape and saves them in this one through its server, which is
  the crate's (`native/rust/src/designer/`). A map is checked against its rules
  (`native/rust/src/content/map_check.rs`) when it is saved and when it loads. Lists keep the designer's order. `p_f_F_c` is written by
  its map scene's export (`content/maps_src/p_f_F_c.tscn`).
- **Errors.** A field of the wrong type, a field the kind doesn't have, an item that isn't
  there, a material the terrain library names that it doesn't hold, or a node, terrain or
  feature a map names that isn't there fails the load, naming the file and the field
  (`units/grem.json: slots[1]: ...`,
  `terrain/terrain_library.json: terrains[0].strata[1].material_id: ...`,
  `maps/demo_map.json: lanes[0].nodes[1]: ...`).

A change to the structs regenerates the schema: `BLESS_SCHEMA=1 cargo test schema` in
`native/rust/`. A test fails while a checked-in schema is stale. It also updates the designer
in the same PR (`CLAUDE.md`).
