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

- **Ids.** A file's id is its name without `.json`. A unit names its items by their path under
  `content/` without the extension (`"items": ["weapons/grem_bite"]`).
- **Names.** Every field has the name of its GDScript property. An item's shared fields
  (`ItemDef`: `item_name`, `slots`, `weight`, `tags`, `traits`...) sit under `"item"`.
- **Defaults.** A unit, weapon or armour file lists only the fields it changes. The others take
  the defaults in the schema, which equal GDScript's (`test_crate_content.gd` holds them
  equal). The tuning file names every field.
- **Infinity.** A list of reals may hold `"inf"` or `"-inf"`, which JSON numbers can't.
- **Enums by name.** `preferred_position` is `"front"`, `"mid"` or `"back"`.
- **Errors.** A field of the wrong type, a field the kind doesn't have, or an item that isn't
  there fails the load, naming the file and the field (`units/grem.json: slots[1]: ...`).

A change to the structs regenerates the schema: `BLESS_SCHEMA=1 cargo test schema` in
`native/rust/`. A test fails while a checked-in schema is stale. It also updates the designer
in the same PR (`CLAUDE.md`).
