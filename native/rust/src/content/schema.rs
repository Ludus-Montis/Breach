//! The JSON Schema of each kind of content file, generated from the structs, checked in
//! under `content/schema/` for the designer to validate against. A test fails if a checked-in
//! schema isn't what the structs generate: `BLESS_SCHEMA=1 cargo test schema` rewrites them.

use schemars::{schema_for, Schema};

use super::{Armour, BattleTuning, TerrainLibrary, Unit, Weapon};

/// Each kind's schema file name and its schema, in name order.
pub fn schemas() -> Vec<(&'static str, Schema)> {
    vec![
        ("armour.schema.json", schema_for!(Armour)),
        ("battle_tuning.schema.json", schema_for!(BattleTuning)),
        ("terrain_library.schema.json", schema_for!(TerrainLibrary)),
        ("unit.schema.json", schema_for!(Unit)),
        ("weapon.schema.json", schema_for!(Weapon)),
    ]
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;

    #[test]
    fn the_checked_in_schemas_are_what_the_structs_generate() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/schema");
        let bless = std::env::var("BLESS_SCHEMA").is_ok_and(|value| value == "1");
        for (name, schema) in schemas() {
            let text = serde_json::to_string_pretty(&schema).unwrap() + "\n";
            let path = dir.join(name);
            if bless {
                fs::create_dir_all(&dir).unwrap();
                fs::write(&path, &text).unwrap();
            }
            let checked_in = fs::read_to_string(&path).unwrap_or_default();
            assert!(checked_in == text, "{name} is stale: BLESS_SCHEMA=1 cargo test schema");
        }
    }
}
