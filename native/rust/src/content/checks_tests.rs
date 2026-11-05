//! The content's own checks (spec 39 step 9e-i), each failing `Content::load` naming the file
//! and the field: a unit's sheet and what it carries, an item's, the registry's traits and
//! tags, and the terrain library's ground.

use std::fs;
use std::path::{Path, PathBuf};

use super::tests::{copy_required, scratch};
use super::{Content, LoadError, TERRAIN_LIBRARY};

/// A root with the repo's tuning, terrain library and registry, and `files` written into it.
fn root_with(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = scratch(name);
    copy_required(&root, &["tuning/battle_tuning.json", TERRAIN_LIBRARY]);
    for (path, text) in files {
        let to = root.join(path);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(to, text).unwrap();
    }
    root
}

fn fails(root: &Path) -> LoadError {
    match Content::load(root) {
        Ok(_) => panic!("{} loaded", root.display()),
        Err(error) => error,
    }
}

/// The load fails on `file`, at `field`, with a message holding `says`.
fn assert_fails(root: &Path, file: &str, field: &str, says: &str) {
    let error = fails(root);
    assert_eq!(error.file, root.join(file), "{error}");
    assert_eq!(error.field, field, "{error}");
    assert!(error.message.contains(says), "{error}");
}

#[test]
fn a_unit_with_negative_hp_fails_naming_the_field() {
    let root = root_with("unit-hp", &[("units/grem.json", r#"{"hp": -5}"#)]);
    assert_fails(&root, "units/grem.json", "hp", "must be >= 0, got -5");
}

#[test]
fn a_footprint_is_one_to_eight_slots() {
    let root = root_with("unit-footprint", &[("units/dragon.json", r#"{"footprint_width": 9}"#)]);
    assert_fails(&root, "units/dragon.json", "footprint_width", "must be 1..8, got 9");
    let root = root_with("unit-footprint-0", &[("units/mote.json", r#"{"footprint_depth": 0}"#)]);
    assert_fails(&root, "units/mote.json", "footprint_depth", "must be 1..8, got 0");
}

#[test]
fn a_unit_takes_a_positive_time_to_build_and_strikes_at_a_positive_speed() {
    let root = root_with("unit-build", &[("units/grem.json", r#"{"build_seconds": 0}"#)]);
    assert_fails(&root, "units/grem.json", "build_seconds", "must be > 0");
    let root = root_with("unit-attack-speed", &[("units/grem.json", r#"{"attack_speed": 0}"#)]);
    assert_fails(&root, "units/grem.json", "attack_speed", "must be > 0");
}

#[test]
fn a_units_skills_attributes_and_regeneration_are_not_negative() {
    for field in [
        "cost_food",
        "speed",
        "position_priority",
        "melee_skill",
        "ranged_skill",
        "defence",
        "armour",
        "ward",
        "strength",
        "agility",
        "constitution",
        "willpower",
        "wits",
        "regeneration",
        "regeneration_limit",
    ] {
        let text = format!(r#"{{"{field}": -1}}"#);
        let root = root_with(&format!("unit-{field}"), &[("units/grem.json", &text)]);
        assert_fails(&root, "units/grem.json", field, "must be >= 0");
    }
}

#[test]
fn a_critical_blow_is_at_least_a_hit() {
    let root = root_with("unit-critical", &[("units/grem.json", r#"{"critical": 0.5}"#)]);
    assert_fails(&root, "units/grem.json", "critical", "must be >= 1");
}

#[test]
fn a_trait_level_is_not_negative() {
    let root =
        root_with("unit-trait-level", &[("units/grem.json", r#"{"traits": {"cunning": -1}}"#)]);
    assert_fails(&root, "units/grem.json", "traits.cunning", "must be >= 0");
}

#[test]
fn items_fit_only_the_slots_a_body_has_free() {
    let spear = r#"{"item": {"item_name": "spear", "slots": ["hand", "hand"]}, "damage": 4}"#;
    let one_hand = r#"{"slots": ["hand"], "items": ["weapons/spear"]}"#;
    let root =
        root_with("unit-slots", &[("weapons/spear.json", spear), ("units/grem.json", one_hand)]);
    assert_fails(&root, "units/grem.json", "items", "need slots it hasn't free: hand");
    let two_hands = r#"{"slots": ["hand", "hand"], "items": ["weapons/spear"]}"#;
    let root = root_with(
        "unit-slots-fit",
        &[("weapons/spear.json", spear), ("units/grem.json", two_hands)],
    );
    Content::load(&root).unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn an_innate_weapon_is_part_of_a_body_part_the_unit_has() {
    let fists = r#"{"item": {"item_name": "fists", "slots": ["hand", "hand"], "innate": true}}"#;
    let unit = r#"{"slots": ["hand", "mouth"], "items": ["weapons/fists"]}"#;
    let root =
        root_with("unit-innate", &[("weapons/fists.json", fists), ("units/grem.json", unit)]);
    assert_fails(&root, "units/grem.json", "items[0]", "part of a body part it hasn't");
}

#[test]
fn an_innate_item_takes_no_slot_beside_what_is_carried() {
    let spear = r#"{"item": {"item_name": "spear", "slots": ["hand", "hand"]}}"#;
    let bite = r#"{"item": {"item_name": "bite", "slots": ["mouth"], "innate": true}}"#;
    let unit =
        r#"{"slots": ["hand", "hand", "mouth"], "items": ["weapons/spear", "weapons/bite"]}"#;
    let root = root_with(
        "unit-innate-free",
        &[("weapons/spear.json", spear), ("weapons/bite.json", bite), ("units/grem.json", unit)],
    );
    Content::load(&root).unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn an_items_weight_and_strength_are_not_negative_and_an_innate_one_weighs_nothing() {
    let root = root_with("item-weight", &[("weapons/club.json", r#"{"item": {"weight": -1}}"#)]);
    assert_fails(&root, "weapons/club.json", "item.weight", "must be >= 0");
    let text = r#"{"item": {"strength_requirement": -2}}"#;
    let root = root_with("item-strength", &[("armour/mail.json", text)]);
    assert_fails(&root, "armour/mail.json", "item.strength_requirement", "must be >= 0");
    let text = r#"{"item": {"innate": true, "weight": 1}}"#;
    let root = root_with("item-innate", &[("weapons/bite.json", text)]);
    assert_fails(&root, "weapons/bite.json", "item.weight", "an innate item weighs nothing");
}

#[test]
fn a_weapons_profile_keeps_to_its_bounds() {
    for (text, field, says) in [
        (r#"{"damage": -1}"#, "damage", "must be >= 0"),
        (r#"{"attack_range": -1}"#, "attack_range", "must be >= 0"),
        (r#"{"damage_floor": 1.5}"#, "damage_floor", "must be 0..1"),
        (r#"{"strength_scaling": -0.5}"#, "strength_scaling", "must be >= 0"),
        (r#"{"attack_seconds": 0}"#, "attack_seconds", "must be > 0"),
    ] {
        let root = root_with(&format!("weapon-{field}"), &[("weapons/club.json", text)]);
        assert_fails(&root, "weapons/club.json", field, says);
    }
}

#[test]
fn armour_takes_off_nothing_negative() {
    let root = root_with("armour-ward", &[("armour/mail.json", r#"{"ward": -1}"#)]);
    assert_fails(&root, "armour/mail.json", "ward", "must be >= 0");
    let root = root_with("armour-armour", &[("armour/mail.json", r#"{"armour": -3}"#)]);
    assert_fails(&root, "armour/mail.json", "armour", "must be >= 0");
}

#[test]
fn a_unit_with_an_unknown_trait_or_tag_fails() {
    let root =
        root_with("unit-unknown-trait", &[("units/grem.json", r#"{"traits": {"guile": 1}}"#)]);
    assert_fails(&root, "units/grem.json", "traits.guile", "unknown trait 'guile'");
    let root = root_with("unit-unknown-tag", &[("units/grem.json", r#"{"tags": ["gremlinoid"]}"#)]);
    assert_fails(&root, "units/grem.json", "tags[0]", "unknown tag 'gremlinoid'");
}

#[test]
fn a_trait_sits_only_where_its_tags_agree() {
    // messenger is a leader's: a unit without leadership can't have it, a leader can.
    let root =
        root_with("unit-leader-trait", &[("units/grem.json", r#"{"traits": {"messenger": 1}}"#)]);
    assert_fails(&root, "units/grem.json", "traits.messenger", "can't sit on a unit");
    let text = r#"{"leadership": 10, "traits": {"messenger": 1}}"#;
    let root = root_with("unit-leader-trait-ok", &[("units/captain.json", text)]);
    Content::load(&root).unwrap_or_else(|error| panic!("{error}"));
    // cunning is a creature's, not a weapon's, but a weapon may grant it.
    let text = r#"{"item": {"traits": {"cunning": 1}}}"#;
    let root = root_with("weapon-unit-trait", &[("weapons/club.json", text)]);
    assert_fails(&root, "weapons/club.json", "item.traits.cunning", "can't sit on a weapon");
    let text = r#"{"item": {"grants": {"cunning": 1}}}"#;
    let root = root_with("weapon-grants", &[("weapons/club.json", text)]);
    Content::load(&root).unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn a_tag_is_only_for_its_kind_of_thing() {
    // polearm is an item's tag.
    let root = root_with("unit-item-tag", &[("units/grem.json", r#"{"tags": ["polearm"]}"#)]);
    assert_fails(&root, "units/grem.json", "tags[0]", "tag 'polearm' is for a item, not a unit");
    let text = r#"{"item": {"tags": ["polearm"]}}"#;
    let root = root_with("weapon-item-tag", &[("weapons/pike.json", text)]);
    Content::load(&root).unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn a_materials_trait_keeps_to_the_registry() {
    let library = r#"{"terrains": [{"id": "FIELDS"}],
        "materials": [{"id": "SOIL"}, {"id": "ROCK", "traits": {"cunning": 1}}]}"#;
    let root = root_with("material-trait", &[(TERRAIN_LIBRARY, library)]);
    assert_fails(&root, TERRAIN_LIBRARY, "materials[1].traits.cunning", "can't sit on a material");
}

#[test]
fn the_registry_holds_together() {
    let root = root_with("registry-ok", &[]);
    let content = Content::load(&root).unwrap_or_else(|error| panic!("{error}"));
    assert!(content.registry.traits.iter().any(|entry| entry.id == "cunning"));
    let mut targets = content.registry.targets("cunning");
    targets.sort();
    assert_eq!(targets, ["leader", "unit"]);
    assert_eq!(content.registry.targets("messenger"), ["leader"]);
    assert_eq!(content.registry.targets("siege"), ["weapon"]);
    assert_eq!(content.registry.implicit().get("climber"), Some(&0));
}

#[test]
fn a_trait_tagged_with_a_units_tag_fails_naming_the_registry_file() {
    let traits = r#"{"traits": [{"id": "odd", "tags": ["martial"], "description": "x"}]}"#;
    let root = root_with("registry-trait-tag", &[("registry/traits.json", traits)]);
    assert_fails(&root, "registry/traits.json", "traits[0].tags[0]", "is for a unit, not a trait");
    let traits = r#"{"traits": [{"id": "quiet", "tags": ["behaviour"]}]}"#;
    let root = root_with("registry-description", &[("registry/traits.json", traits)]);
    assert_fails(&root, "registry/traits.json", "traits[0].description", "must not be empty");
    let traits = r#"{"traits": [{"id": "nowhere", "tags": ["behaviour"], "only": ["weapon"],
        "description": "x"}]}"#;
    let root = root_with("registry-targets", &[("registry/traits.json", traits)]);
    assert_fails(&root, "registry/traits.json", "traits[0]", "can sit nowhere");
}

#[test]
fn a_statuss_tags_are_for_statuses() {
    let statuses = r#"{"statuses": [{"id": "dazed", "tags": ["behaviour"]}]}"#;
    let root = root_with("registry-status", &[("registry/statuses.json", statuses)]);
    assert_fails(&root, "registry/statuses.json", "statuses[0].tags[0]", "not a status");
}

/// A library of one terrain, FIELDS, with `fields` (its other fields), and `rest` (the
/// library's other lists), on the materials SOIL and WATER (which flows).
fn library(fields: &str, rest: &str) -> String {
    format!(
        r#"{{"terrains": [{{"id": "FIELDS"{fields}}}], "materials": [{{"id": "SOIL"}},
        {{"id": "WATER", "traits": {{"flows": 1}}}}]{rest}}}"#
    )
}

#[test]
fn the_terrain_librarys_ground_keeps_to_its_bounds() {
    for (fields, field, says) in [
        (r#", "move_cost": -1"#, "terrains[0].move_cost", "must be >= 0"),
        (r#", "bearing": 4, "foundation_max": 2"#, "terrains[0].foundation_max", "2 < bearing 4"),
        (
            r#", "strata": [{"material_id": "SOIL", "min_cells": 4, "max_cells": 2}]"#,
            "terrains[0].strata[0]",
            "min_cells 4 > max_cells 2",
        ),
        (
            r#", "liquids": [{"material_id": "SOIL"}]"#,
            "terrains[0].liquids[0].material_id",
            "'SOIL' doesn't flow",
        ),
        (
            r#", "liquids": [{"material_id": "WATER", "min_cells": 3, "max_cells": 1}]"#,
            "terrains[0].liquids[0]",
            "depth min > max",
        ),
        (
            r#", "liquids": [{"material_id": "WATER", "chance": 1.5}]"#,
            "terrains[0].liquids[0]",
            "chance must be 0 to 1",
        ),
        (r#", "can_be_base": false"#, "terrains", "at least one terrain must have can_be_base"),
    ] {
        let root = root_with("terrain-ground", &[(TERRAIN_LIBRARY, &library(fields, ""))]);
        assert_fails(&root, TERRAIN_LIBRARY, field, says);
    }
}

#[test]
fn the_terrain_librarys_ids_are_there_and_unique() {
    for (rest, field, says) in [
        (
            r#", "features": [{"id": "ford"}, {"id": "ford"}]"#,
            "features[1].id",
            "duplicate feature id 'ford'",
        ),
        (r#", "features": [{"id": ""}]"#, "features[0].id", "must not be empty"),
        (r#", "road_move_multiplier": -1"#, "road_move_multiplier", "must be >= 0"),
    ] {
        let root = root_with("terrain-ids", &[(TERRAIN_LIBRARY, &library("", rest))]);
        assert_fails(&root, TERRAIN_LIBRARY, field, says);
    }
    let twice = r#"{"terrains": [{"id": "FIELDS"}, {"id": "FIELDS"}]}"#;
    let root = root_with("terrain-twice", &[(TERRAIN_LIBRARY, twice)]);
    assert_fails(&root, TERRAIN_LIBRARY, "terrains[1].id", "duplicate terrain id 'FIELDS'");
}

#[test]
fn a_heat_transition_becomes_something_or_gains_a_trait() {
    let library = r#"{"terrains": [{"id": "FIELDS"}],
        "materials": [{"id": "ICE", "heat_transitions": [{"threshold": 2}]}]}"#;
    let root = root_with("terrain-transition", &[(TERRAIN_LIBRARY, library)]);
    assert_fails(
        &root,
        TERRAIN_LIBRARY,
        "materials[0].heat_transitions[0]",
        "becomes nothing and gains no trait",
    );
}

#[test]
fn an_implicit_trait_is_a_units_at_a_level_not_below_zero() {
    let traits = r#"{"traits": [{"id": "rusts", "tags": ["ground"], "default": 0,
        "description": "x"}]}"#;
    let root = root_with("registry-implicit-unit", &[("registry/traits.json", traits)]);
    assert_fails(&root, "registry/traits.json", "traits[0].default", "a unit's");
    let traits = r#"{"traits": [{"id": "plodder", "tags": ["movement"], "default": -1,
        "description": "x"}]}"#;
    let root = root_with("registry-implicit-level", &[("registry/traits.json", traits)]);
    assert_fails(&root, "registry/traits.json", "traits[0].default", "must be >= 0");
}
