//! What a unit strikes with and wears, as the fight reckons it (Decisions 119 and 120, spec
//! 39 step 8c-iii; GDScript's UnitArms and UnitDef's reads of its items), read from its
//! definition in the crate's content: the weapons it can use (those it carries, and its
//! innate ones whose body parts no carried item holds), each weapon's damage with its
//! wielder's strength over the weapon's need, the skill it wields them at, how far their
//! damage may fall short, its armour and ward, its load and the traits its items grant.

use crate::content::{Armour, BattleTuning, Content, Item, Unit, Weapon};

#[cfg(test)]
pub(crate) mod tests;

/// One item a unit carries: a weapon or armour.
enum Carried<'a> {
    Weapon(&'a Weapon),
    Armour(&'a Armour),
}

impl<'a> Carried<'a> {
    fn item(&self) -> &'a Item {
        match self {
            Carried::Weapon(weapon) => &weapon.item,
            Carried::Armour(armour) => &armour.item,
        }
    }
}

/// A unit's definition with its items found in the content, in the order it lists them.
pub struct Arms<'a> {
    unit: &'a Unit,
    items: Vec<Carried<'a>>,
    tuning: &'a BattleTuning,
}

impl<'a> Arms<'a> {
    /// `unit`'s arms, its items (`weapons/<id>`, `armour/<id>`) found in `content`; the
    /// loader has checked each is there.
    pub fn of(content: &'a Content, unit: &'a Unit) -> Arms<'a> {
        let items = unit
            .items
            .iter()
            .filter_map(|path| match path.split_once('/') {
                Some(("weapons", id)) => content.weapons.get(id).map(Carried::Weapon),
                Some(("armour", id)) => content.armour.get(id).map(Carried::Armour),
                _ => None,
            })
            .collect();
        Arms { unit, items, tuning: &content.tuning }
    }

    /// The level of a rated trait it has, its own or one an item it carries grants, the
    /// higher (UnitDef.trait_level). One it doesn't list it has at the registry's default,
    /// which is 0 for every implicit trait (content/registry/traits.json: climber, swimmer).
    pub fn trait_level(&self, id: &str) -> i64 {
        let own = self.unit.traits.get(id).copied().unwrap_or(0);
        let granted = self.items.iter().filter_map(|c| c.item().grants.get(id).copied());
        granted.fold(own, i64::max)
    }

    /// The traits its items grant, item by item in the order it lists them.
    pub fn granted(&self) -> Vec<&'a str> {
        let grants = self.items.iter().flat_map(|carried| carried.item().grants.keys());
        grants.map(String::as_str).collect()
    }

    /// The weapons it can use, in the order it lists them: those it carries, and its innate
    /// ones whose body parts no carried item holds (fists are no use with a spear in both
    /// hands).
    pub fn weapons(&self) -> Vec<&'a Weapon> {
        let free = self.free_slots();
        let usable = |weapon: &&Weapon| !weapon.item.innate || has_slots(&free, &weapon.item.slots);
        self.items
            .iter()
            .filter_map(|carried| match carried {
                Carried::Weapon(weapon) => Some(*weapon),
                Carried::Armour(_) => None,
            })
            .filter(usable)
            .collect()
    }

    /// The hardest-hitting ranged weapon (the first of the hardest), or none for a
    /// melee-only unit.
    pub fn ranged_weapon(&self) -> Option<&'a Weapon> {
        let mut best: Option<&Weapon> = None;
        for weapon in self.weapons() {
            if weapon.attack_range != 0 && best.is_none_or(|b| weapon.damage > b.damage) {
                best = Some(weapon);
            }
        }
        best
    }

    /// Seconds between its melee blows: its melee weapons strike together, as often as the
    /// slowest of them allows, scaled by its attack speed (a second without weapons).
    pub fn melee_seconds(&self) -> f64 {
        let longest = self.used(false).iter().fold(0.0, |most: f64, w| most.max(w.attack_seconds));
        (if longest > 0.0 { longest } else { 1.0 }) / self.unit.attack_speed
    }

    /// Whether it can parry (Decision 118): it holds a melee weapon, not an innate one, or a
    /// shield.
    pub fn can_parry(&self) -> bool {
        self.used(false).iter().any(|weapon| !weapon.item.innate)
            || self.items.iter().any(|carried| carried.item().tags.iter().any(|t| t == "shield"))
    }

    /// [(damage, damage type, magical), ...] for the weapons it strikes with: all its melee
    /// weapons together, or its best ranged one.
    pub fn parts(&self, ranged: bool) -> Vec<(f64, &'a str, bool)> {
        let parts = self.used(ranged).into_iter();
        parts.map(|w| (self.damage(w), w.damage_type.as_str(), w.magical)).collect()
    }

    /// The least share of its damage a blow may deal before skill lifts it: its weapons'
    /// floors, weighted by their damage (1 with no weapons).
    pub fn floor_share(&self, ranged: bool) -> f64 {
        let mut total = 0.0;
        let mut weighted = 0.0;
        for weapon in self.used(ranged) {
            total += weapon.damage as f64;
            weighted += weapon.damage as f64 * weapon.damage_floor;
        }
        if total > 0.0 {
            weighted / total
        } else {
            1.0
        }
    }

    /// Its melee or ranged skill as it wields its weapons: shifted by its proficiency with
    /// the least familiar of them (mastered with none), and lowered for the most strength
    /// any of them asks that it lacks.
    pub fn skill(&self, ranged: bool) -> i64 {
        let trained = if ranged { self.unit.ranged_skill } else { self.unit.melee_skill };
        let mut least = 2;
        let mut short = 0;
        for weapon in self.used(ranged) {
            least = least.min(self.proficiency(weapon));
            short = short.max(weapon.item.strength_requirement - self.unit.strength);
        }
        let tuning = self.tuning;
        let shift = [tuning.item_untrained_skill, 0.0, tuning.item_mastered_skill][least];
        let skill = trained as f64 + shift - short as f64 * tuning.item_weak_skill;
        (skill.round() as i64).max(0)
    }

    /// The weight of what it carries (innate parts weigh nothing).
    pub fn carried_weight(&self) -> f64 {
        self.items.iter().fold(0.0, |carried, item| carried + item.item().weight)
    }

    /// Its encumbrance (Decision 120): 0 none, 1 mild, 2 steep, 3 immobile.
    pub fn load_stage(&self) -> usize {
        let hauler = self.trait_level("hauler");
        stage_of(self.carried_weight(), self.unit.strength, hauler, self.tuning)
    }

    /// Whether it carries more than ground_mode_load_share of the way through its first
    /// encumbrance band, so it neither swims nor climbs (TerrainWalker).
    pub fn too_heavy_for_modes(&self) -> bool {
        let tuning = self.tuning;
        let limit = self.unit.strength as f64 * tuning.load_per_strength;
        let easy = limit * tuning.load_easy;
        self.carried_weight() > easy + (limit - easy) * tuning.ground_mode_load_share
    }

    /// Its armour (against mundane blows) and its ward (against magical ones): its own and
    /// what it wears.
    pub fn protection(&self) -> (i64, i64) {
        let mut out = (self.unit.armour, self.unit.ward);
        for carried in &self.items {
            if let Carried::Armour(armour) = carried {
                out = (out.0 + armour.armour, out.1 + armour.ward);
            }
        }
        out
    }

    /// The weapons its blows or shots come from: all its melee ones, or its best ranged one.
    fn used(&self, ranged: bool) -> Vec<&'a Weapon> {
        if ranged {
            return self.ranged_weapon().into_iter().collect();
        }
        self.weapons().into_iter().filter(|weapon| weapon.attack_range == 0).collect()
    }

    /// A weapon's damage with its wielder's strength over the weapon's need.
    fn damage(&self, weapon: &Weapon) -> f64 {
        let over = (self.unit.strength - weapon.item.strength_requirement).max(0);
        weapon.damage as f64 + weapon.strength_scaling * over as f64
    }

    /// 0 untrained, 1 trained, 2 mastered: its best level in the weapon's tags; trained in an
    /// innate weapon or one with no tags.
    fn proficiency(&self, weapon: &Weapon) -> usize {
        if weapon.item.innate || weapon.item.tags.is_empty() {
            return 1;
        }
        let levels = weapon.item.tags.iter().map(|t| self.unit.proficiencies.get(t).copied());
        levels.map(Option::unwrap_or_default).max().unwrap_or(0).clamp(0, 2) as usize
    }

    /// The slots its body has left once its carried items take theirs.
    fn free_slots(&self) -> Vec<&'a str> {
        let mut free: Vec<&str> = self.unit.slots.iter().map(String::as_str).collect();
        for carried in &self.items {
            if carried.item().innate {
                continue;
            }
            for slot in &carried.item().slots {
                if let Some(at) = free.iter().position(|f| f == slot) {
                    free.remove(at);
                }
            }
        }
        free
    }
}

/// The stage of `carried` load for a unit of `strength` and hauler level `hauler`
/// (UnitArms.stage_of).
pub fn stage_of(carried: f64, strength: i64, hauler: i64, tuning: &BattleTuning) -> usize {
    let limit = strength as f64 * tuning.load_per_strength;
    let most = tuning.load_most + hauler as f64 * tuning.load_hauler;
    if carried <= limit * tuning.load_easy {
        0
    } else if carried <= limit {
        1
    } else if carried <= limit * most {
        2
    } else {
        3
    }
}

/// Whether `have` holds every slot `need` names, as many times as it names it.
fn has_slots(have: &[&str], need: &[String]) -> bool {
    let mut left = have.to_vec();
    need.iter().all(|slot| match left.iter().position(|l| l == slot) {
        Some(at) => {
            left.remove(at);
            true
        }
        None => false,
    })
}
