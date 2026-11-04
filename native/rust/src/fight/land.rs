//! The fight's blows landed (BlowLanding.land, spec 39 step 8b5-xviii), with what each reads
//! read where the crate holds it: each unit's arms from its sheet and its words (its weapons'
//! parts, the damage types it is weak to, resists, is immune to and that stop its
//! regeneration, coded afresh for the tick in the order first met, its agility), and with
//! rolls on, each striker's formation's morale band (`shocks::morale`, `bands::band`) and
//! how many foes press each target (`blows::pressed`), on the field as the scrum's search
//! synced it this tick, or synced here (`scrum::seek::sync`) when it had no need to. Then
//! `landing::land`. How each landed goes back to GDScript, which builds the events.

use std::collections::HashMap;

use crate::bands::{self, BandTuning};
use crate::blows;
use crate::field::Field;
use crate::ground::Ground;
use crate::landing::{self, Arms, LandingTuning, Part};
use crate::roster::Roster;
use crate::scrum::seek;
use crate::shocks;
use crate::squads::Squads;
use crate::words;

/// What the landing reads of the tick and of BattleTuning.
pub struct Rules {
    pub tuning: LandingTuning,
    /// [the battle's seed, the tick], or None for no rolls (every blow a plain hit).
    pub rolls: Option<(i64, i64)>,
    /// reach_contact: the foes within it that fight a target press it.
    pub reach: f64,
    pub bands: BandTuning,
}

/// A blow or shot to land, by its striker's and its target's store slots: its damage before
/// it lands, whether it is a shot, and (a blow) whether it falls on a flank.
#[derive(Clone, Copy, Debug)]
pub struct Struck {
    pub striker: usize,
    pub target: usize,
    pub damage: i64,
    pub ranged: bool,
    pub flank: bool,
}

/// An agility not set (UnitDef.AVERAGE).
const AVERAGE: f64 = 10.0;

/// BlowLanding.land over the roster's squads: per blow, the damage it deals and how it
/// landed (`landing`'s codes). `synced`: the scrum's search synced the field this tick.
pub fn land(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    struck: &[Struck],
    rules: &Rules,
    synced: bool,
    ground: Option<&Ground>,
) -> Vec<(i64, u8)> {
    if struck.is_empty() {
        return vec![];
    }
    let mut pressing = vec![(None, 0); struck.len()];
    if let Some((seed, _)) = rules.rolls {
        if !synced {
            seek::sync(squads, field, roster, seed);
        }
        pressing = pressed(squads, field, roster, struck, rules);
    }
    let mut arms = ArmsOf::default();
    let mut blows = Vec::with_capacity(struck.len());
    for (blow, &(band, pressed)) in struck.iter().zip(&pressing) {
        blows.push(landing::Blow {
            striker: arms.index(field, blow.striker),
            target: arms.index(field, blow.target),
            nominal: blow.damage,
            ranged: blow.ranged,
            flank: !blow.ranged && blow.flank,
            band,
            pressed,
        });
    }
    landing::land(&mut field.columns, &arms.arms, &blows, &rules.tuning, rules.rolls, ground)
}

/// ScrumBlowsField.pressed: per blow, its striker's formation's morale band (None: not on
/// the field) and how many units touching its target have it as theirs, by the units'
/// targets now.
fn pressed(
    squads: &mut Squads,
    field: &Field,
    roster: &Roster,
    struck: &[Struck],
    rules: &Rules,
) -> Vec<(Option<usize>, i64)> {
    let targets: Vec<i64> = roster
        .listed
        .iter()
        .flat_map(|l| &l.living)
        .map(|m| field.columns.target_id[m.slot])
        .collect();
    let id = |slot: usize| field.unit_at(slot as u32).unwrap_or(0);
    let asked: Vec<i64> = struck.iter().flat_map(|b| [id(b.striker), id(b.target)]).collect();
    let counts = blows::pressed(field, &targets, &asked, rules.reach);
    let mut bands_of: HashMap<usize, usize> = HashMap::new();
    counts
        .chunks(2)
        .map(|pair| {
            let band = usize::try_from(pair[0]).ok().map(|place| {
                *bands_of.entry(place).or_insert_with(|| {
                    let listed = &roster.listed[place];
                    let morale = shocks::morale(squads, &field.columns, listed, &rules.bands);
                    bands::band(morale, &rules.bands) as usize
                })
            });
            (band, i64::from(pair[1]))
        })
        .collect()
}

/// The arms of the units a tick's blows strike with or land on, each once, in the order
/// first met (BlowLanding._Arms); damage types coded afresh, in the order first met, so a
/// tick's blows may name 64 of them however many words the lexicon holds.
#[derive(Default)]
struct ArmsOf {
    arms: Vec<Arms>,
    at: HashMap<usize, usize>,
    codes: HashMap<u32, u32>,
}

impl ArmsOf {
    /// The unit's index, its arms read the first time it is met.
    fn index(&mut self, field: &Field, slot: usize) -> usize {
        if let Some(&at) = self.at.get(&slot) {
            return at;
        }
        let (sheet, said) = (&field.columns.sheet[slot], &field.columns.words[slot]);
        let agility = field.lexicon.find("agility").and_then(|code| {
            said.attributes.iter().find(|(word, _)| *word == code).map(|&(_, level)| level)
        });
        let mut arms = Arms {
            id: field.unit_at(slot as u32).unwrap_or(0),
            slot,
            melee_skill: sheet.melee_skill as f64,
            ranged_skill: sheet.ranged_skill as f64,
            critical: sheet.critical,
            melee_floor: sheet.melee_floor,
            ranged_floor: sheet.ranged_floor,
            max_stamina: sheet.max_stamina,
            agility: agility.unwrap_or(AVERAGE),
            dodging: sheet.dodging,
            defence: sheet.defence,
            armour: sheet.armour,
            ward: sheet.ward,
            wounding: sheet.wounding as i32,
            parries: sheet.parries,
            weak: self.bits(&said.weaknesses),
            resists: self.bits(&said.resistances),
            immune: self.bits(&said.immunities),
            stops: self.bits(&said.regeneration_stops),
            ..Arms::default()
        };
        arms.melee_parts = said.melee_parts.iter().map(|p| self.part(p)).collect();
        arms.ranged_parts = said.ranged_parts.iter().map(|p| self.part(p)).collect();
        self.at.insert(slot, self.arms.len());
        self.arms.push(arms);
        self.arms.len() - 1
    }

    fn part(&mut self, part: &words::Part) -> Part {
        Part { amount: part.damage, kind: self.code(part.kind), magical: part.magical }
    }

    fn bits(&mut self, kinds: &[u32]) -> u64 {
        kinds.iter().fold(0, |bits, &kind| bits | 1u64.checked_shl(self.code(kind)).unwrap_or(0))
    }

    fn code(&mut self, word: u32) -> u32 {
        let next = self.codes.len() as u32;
        let code = *self.codes.entry(word).or_insert(next);
        debug_assert!(code < 64, "a tick's blows name at most 64 damage types");
        code
    }
}
