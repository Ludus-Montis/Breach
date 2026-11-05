//! The content's own rules (spec 39 step 9e-i; GDScript's `validate()`s and ContentRegistry's
//! checks until then): a unit's sheet in bounds and its items fitting its body, an item's
//! weight, strength and attack profile, every trait and tag registered and where it may sit,
//! the registry holding together, and the terrain library's ground. Each gives the first
//! problem as (the field, the message); `Content::load` fails on it, naming the file.

use std::collections::BTreeSet;

use super::{Content, Item, Registry, TerrainLibrary, Unit};

/// A problem: the field (`hp`, `items[1]`, `terrains[0].strata[2]`) and what is wrong.
pub type Problem = (String, String);

/// The most slots a footprint spans each way (Decision 40: an 8x8 dragon fills the widest lane).
const MAX_FOOTPRINT: i64 = 8;

fn problem(field: impl Into<String>, message: impl Into<String>) -> Option<Problem> {
    Some((field.into(), message.into()))
}

/// `field` (named) must be >= 0.
fn not_negative(field: &str, value: f64) -> Option<Problem> {
    (value < 0.0).then(|| (field.into(), format!("must be >= 0, got {value}")))
}

/// The first problem among `checks`.
fn first(checks: impl IntoIterator<Item = Option<Problem>>) -> Option<Problem> {
    checks.into_iter().flatten().next()
}

/// A trait problem on `traits` at `at` (`traits`, `item.grants`).
fn traits_at(
    registry: &Registry,
    at: &str,
    traits: &std::collections::BTreeMap<String, i64>,
    kinds: &[&str],
) -> Option<Problem> {
    if let Some((id, level)) = traits.iter().find(|(_, level)| **level < 0) {
        return problem(format!("{at}.{id}"), format!("level must be >= 0, got {level}"));
    }
    let (id, message) = registry.trait_problems(traits, kinds)?;
    problem(format!("{at}.{id}"), message)
}

/// A tag problem on `tags` at `at`, on a thing of `tagged`.
fn tags_at(registry: &Registry, at: &str, tags: &[String], tagged: &str) -> Option<Problem> {
    let (index, message) = registry.tag_problems(tags, tagged)?;
    problem(format!("{at}[{index}]"), message)
}

impl Content {
    /// The item `path` (`weapons/<id>`, `armour/<id>`) names, if it is here.
    pub fn item(&self, path: &str) -> Option<&Item> {
        match path.split_once('/') {
            Some(("weapons", id)) => self.weapons.get(id).map(|weapon| &weapon.item),
            Some(("armour", id)) => self.armour.get(id).map(|armour| &armour.item),
            _ => None,
        }
    }

    /// The first problem with `unit`'s sheet, its items' fit and its traits and tags. Each
    /// item it names must be here (`check_items` first).
    pub fn unit_problem(&self, unit: &Unit) -> Option<Problem> {
        let counts = [
            ("cost_food", unit.cost_food),
            ("hp", unit.hp),
            ("position_priority", unit.position_priority),
            ("melee_skill", unit.melee_skill),
            ("ranged_skill", unit.ranged_skill),
            ("defence", unit.defence),
            ("armour", unit.armour),
            ("ward", unit.ward),
            ("strength", unit.strength),
            ("agility", unit.agility),
            ("constitution", unit.constitution),
            ("willpower", unit.willpower),
            ("wits", unit.wits),
        ];
        let rates = [
            ("speed", unit.speed),
            ("regeneration", unit.regeneration),
            ("regeneration_limit", unit.regeneration_limit),
        ];
        let footprints =
            [("footprint_depth", unit.footprint_depth), ("footprint_width", unit.footprint_width)];
        let kinds: &[&str] = if unit.leadership > 0 { &["unit", "leader"] } else { &["unit"] };
        first(
            counts
                .iter()
                .map(|(field, value)| not_negative(field, *value as f64))
                .chain(rates.iter().map(|(field, value)| not_negative(field, *value)))
                .chain(footprints.iter().map(|(field, value)| {
                    (!(1..=MAX_FOOTPRINT).contains(value)).then(|| {
                        (field.to_string(), format!("must be 1..{MAX_FOOTPRINT}, got {value}"))
                    })
                }))
                .chain([
                    (unit.build_seconds <= 0.0).then(|| {
                        ("build_seconds".into(), format!("must be > 0, got {}", unit.build_seconds))
                    }),
                    (unit.critical < 1.0).then(|| {
                        ("critical".into(), format!("must be >= 1, got {}", unit.critical))
                    }),
                    (unit.attack_speed <= 0.0).then(|| {
                        ("attack_speed".into(), format!("must be > 0, got {}", unit.attack_speed))
                    }),
                ]),
        )
        .or_else(|| self.fit_problem(unit))
        .or_else(|| traits_at(&self.registry, "traits", &unit.traits, kinds))
        .or_else(|| tags_at(&self.registry, "tags", &unit.tags, "unit"))
    }

    /// Whether `unit`'s items fit its body: each innate one part of body parts it has, and
    /// those it carries in slots it has free.
    fn fit_problem(&self, unit: &Unit) -> Option<Problem> {
        let mut free = unit.slots.clone();
        let mut short = Vec::new();
        for (index, path) in unit.items.iter().enumerate() {
            let item = self.item(path)?;
            if item.innate {
                if !has_slots(&unit.slots, &item.slots) {
                    let message = format!("{}: part of a body part it hasn't", item.item_name);
                    return problem(format!("items[{index}]"), message);
                }
                continue;
            }
            for slot in &item.slots {
                match free.iter().position(|have| have == slot) {
                    Some(at) => {
                        free.remove(at);
                    }
                    None => short.push(slot.clone()),
                }
            }
        }
        if short.is_empty() {
            return None;
        }
        problem("items", format!("its items need slots it hasn't free: {}", short.join(", ")))
    }

    /// The first problem with an item of `kind` (weapon, armour), its fields under `item.`.
    pub fn item_problem(&self, item: &Item, kind: &str) -> Option<Problem> {
        first([
            not_negative("item.weight", item.weight),
            not_negative("item.strength_requirement", item.strength_requirement as f64),
            (item.innate && item.weight > 0.0).then(|| {
                (
                    "item.weight".into(),
                    format!("an innate item weighs nothing, got {}", item.weight),
                )
            }),
        ])
        .or_else(|| traits_at(&self.registry, "item.traits", &item.traits, &[kind]))
        .or_else(|| traits_at(&self.registry, "item.grants", &item.grants, &["unit"]))
        .or_else(|| tags_at(&self.registry, "item.tags", &item.tags, "item"))
    }
}

/// Whether `have` holds every slot `need` names, as many times as it names it.
fn has_slots(have: &[String], need: &[String]) -> bool {
    let mut left = have.to_vec();
    need.iter().all(|slot| match left.iter().position(|kept| kept == slot) {
        Some(at) => {
            left.remove(at);
            true
        }
        None => false,
    })
}

impl super::Weapon {
    /// The first problem with its attack profile (its item's are `Content::item_problem`).
    pub fn problem(&self) -> Option<Problem> {
        first([
            not_negative("damage", self.damage as f64),
            not_negative("attack_range", self.attack_range as f64),
            (!(0.0..=1.0).contains(&self.damage_floor)).then(|| {
                ("damage_floor".into(), format!("must be 0..1, got {}", self.damage_floor))
            }),
            not_negative("strength_scaling", self.strength_scaling),
            (self.attack_seconds <= 0.0).then(|| {
                ("attack_seconds".into(), format!("must be > 0, got {}", self.attack_seconds))
            }),
        ])
    }
}

impl super::Armour {
    /// The first problem with what it takes off a blow.
    pub fn problem(&self) -> Option<Problem> {
        first([not_negative("armour", self.armour as f64), not_negative("ward", self.ward as f64)])
    }
}

impl Registry {
    /// The first problem with the registry itself, as (its file, the field, the message):
    /// each trait's tags a trait's, each trait described and able to sit somewhere, an
    /// implicit one a unit's at a level of 0 or more, each status's tags a status's.
    pub fn problem(&self) -> Option<(&'static str, Problem)> {
        for (at, entry) in self.traits.iter().enumerate() {
            let found = tags_at(self, &format!("traits[{at}].tags"), &entry.tags, "trait")
                .or_else(|| {
                    entry
                        .description
                        .is_empty()
                        .then(|| (format!("traits[{at}].description"), "must not be empty".into()))
                })
                .or_else(|| {
                    let level = entry.default?;
                    let field = format!("traits[{at}].default");
                    if level < 0 {
                        return Some((field, format!("must be >= 0, got {level}")));
                    }
                    let on_units = self.targets(&entry.id).iter().any(|kind| kind == "unit");
                    let message = "an implicit trait is every unit's, so it must be a unit's";
                    (!on_units).then(|| (field, message.into()))
                })
                .or_else(|| {
                    self.targets(&entry.id).is_empty().then(|| {
                        (format!("traits[{at}]"), format!("'{}' can sit nowhere", entry.id))
                    })
                });
            if let Some(found) = found {
                return Some(("traits.json", found));
            }
        }
        for (at, entry) in self.statuses.iter().enumerate() {
            if let Some(found) =
                tags_at(self, &format!("statuses[{at}].tags"), &entry.tags, "status")
            {
                return Some(("statuses.json", found));
            }
        }
        None
    }
}

impl TerrainLibrary {
    /// The first problem with the library beyond a missing material (`missing_material`):
    /// ids there and unique, a base terrain, the ground's bounds, liquids that flow, heat
    /// transitions that do something, and every material's traits a material's.
    pub fn problem(&self, registry: &Registry) -> Option<Problem> {
        let terrain_ids: Vec<&str> = self.terrains.iter().map(|t| t.id.as_str()).collect();
        let feature_ids: Vec<&str> = self.features.iter().map(|f| f.id.as_str()).collect();
        let material_ids: Vec<&str> = self.materials.iter().map(|m| m.id.as_str()).collect();
        first([
            id_problem("terrains", "terrain", &terrain_ids),
            id_problem("features", "feature", &feature_ids),
            id_problem("materials", "material", &material_ids),
            (!self.terrains.iter().any(|terrain| terrain.can_be_base)).then(|| {
                let message =
                    "at least one terrain must have can_be_base, so a map has a base terrain";
                ("terrains".into(), message.into())
            }),
            not_negative("road_move_multiplier", self.road_move_multiplier),
        ])
        .or_else(|| first(self.terrains.iter().enumerate().map(|(at, _)| self.ground_problem(at))))
        .or_else(|| {
            first(
                self.materials
                    .iter()
                    .enumerate()
                    .map(|(at, _)| self.material_problem(at, registry)),
            )
        })
    }

    fn ground_problem(&self, at: usize) -> Option<Problem> {
        let terrain = &self.terrains[at];
        let here = format!("terrains[{at}]");
        if terrain.move_cost < 0.0 {
            return problem(
                format!("{here}.move_cost"),
                format!("must be >= 0, got {}", terrain.move_cost),
            );
        }
        if terrain.foundation_max < terrain.bearing {
            let message =
                format!("foundation_max {} < bearing {}", terrain.foundation_max, terrain.bearing);
            return problem(format!("{here}.foundation_max"), message);
        }
        for (index, body) in terrain.liquids.iter().enumerate() {
            let field = format!("{here}.liquids[{index}]");
            if !self.flows(&body.material_id) {
                let message = format!("liquid body of '{}' doesn't flow", body.material_id);
                return problem(format!("{field}.material_id"), message);
            }
            if body.min_cells < 0 || body.min_cells > body.max_cells {
                return problem(field, format!("liquid '{}': depth min > max", body.material_id));
            }
            let chance = |value: f64| (0.0..=1.0).contains(&value);
            if !(chance(body.chance) && chance(body.surface_chance)) {
                return problem(
                    field,
                    format!("liquid '{}': chance must be 0 to 1", body.material_id),
                );
            }
        }
        for (index, stratum) in terrain.strata.iter().enumerate() {
            if stratum.min_cells < 0 || stratum.min_cells > stratum.max_cells {
                let message = format!(
                    "stratum '{}': min_cells {} > max_cells {}",
                    stratum.material_id, stratum.min_cells, stratum.max_cells
                );
                return problem(format!("{here}.strata[{index}]"), message);
            }
        }
        None
    }

    fn material_problem(&self, at: usize, registry: &Registry) -> Option<Problem> {
        let material = &self.materials[at];
        for (index, transition) in material.heat_transitions.iter().enumerate() {
            if transition.becomes.is_empty() && transition.gains_trait.is_empty() {
                let field = format!("materials[{at}].heat_transitions[{index}]");
                return problem(
                    field,
                    format!("'{}': becomes nothing and gains no trait", material.id),
                );
            }
        }
        traits_at(registry, &format!("materials[{at}].traits"), &material.traits, &["material"])
    }

    /// Whether the material `id` is a liquid: it has the trait "flows" (`MaterialDef.is_liquid`).
    fn flows(&self, id: &str) -> bool {
        self.materials.iter().any(|material| {
            material.id == id && material.traits.get("flows").is_some_and(|level| *level > 0)
        })
    }
}

/// The first id under `list` (`terrains`) that is empty or a duplicate.
fn id_problem(list: &str, kind: &str, ids: &[&str]) -> Option<Problem> {
    let mut seen = BTreeSet::new();
    for (at, id) in ids.iter().enumerate() {
        let field = format!("{list}[{at}].id");
        if id.is_empty() {
            return problem(field, format!("{kind} id must not be empty"));
        }
        if !seen.insert(*id) {
            return problem(field, format!("duplicate {kind} id '{id}'"));
        }
    }
    None
}
