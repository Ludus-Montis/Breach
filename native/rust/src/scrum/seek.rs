//! The scrum's seek, walk and facing over the roster (FormationScrum._seek, spec 39 step
//! 8b5-xiii). The field is brought to the scrum's snapshot of the listed squads (`sync`), a
//! unit new to it enrolled with its record; each fighting squad's say is read where the
//! crate holds it: its foes, whether its units may seek (it isn't ordered to retreat), its
//! frame (its stance's, else its own), its chasers, each unit's speed. Then the seek
//! (`seeking::seek`), the walk (`walk_store::walk`, over the bodies lying on the field),
//! each step paid for the ground it crosses (`mover::pay`), and the facing
//! (`face_store::face`): the phases ScrumSeekField, ScrumWalkField and ScrumFaceField ran
//! from GDScript, bit for bit.

use crate::contest;
use crate::entries::Key;
use crate::face_store;
use crate::facing;
use crate::field::{self, Enrol, Field, SyncIn};
use crate::ground::{Ground, Walker};
use crate::locks::{self, DESTROYED, FIGHTING, ROUTING};
use crate::maths::V2;
use crate::mover;
use crate::moving::pass::Bodies;
use crate::roster::{Member, Roster};
use crate::scrum::{Params, AIM, PRESS, TOUCH};
use crate::seeking::{self, SquadSay};
use crate::squads::Squads;
use crate::walk_store::{self, WalkSay};
use crate::walking;

/// The tuning the three phases read.
pub struct Rules {
    pub tick: i64,
    pub seed: i64,
    /// combat_contest_die.
    pub die: i64,
    /// reach_contact, scrum_leash.
    pub reach: f64,
    pub leash: f64,
    /// The seconds a tick lasts, and the cells a tick at speed 1.
    pub seconds: f64,
    pub pace: f64,
    /// scrum_crowding: the share of its pace a unit keeps in the press, out of a pursuit.
    pub crowding: f64,
    /// bodies_steer_look, bodies_steer_clear, bodies_ground_drag, bodies_ground_block.
    pub look: f64,
    pub clear: f64,
    pub drag: f64,
    pub block: f64,
    /// The least dot for a point to be in front (ScrumReach.in_front).
    pub front: f64,
}

/// FormationScrum._seek over the roster's squads on `ground` (none: open). Returns the ids
/// of the fighting squads active this tick: one of whose units touches a foe or makes for a
/// slot (the stall's `active`).
pub fn step(
    squads: &Squads,
    field: &mut Field,
    roster: &Roster,
    ground: Option<&Ground>,
    rules: &Rules,
) -> Vec<i64> {
    let synced = sync(squads, field, roster, rules.seed);
    let say = Say::of(squads, field, roster, &synced);
    let params = Params {
        tick: rules.tick,
        seed: rules.seed,
        die: rules.die,
        reach: rules.reach,
        leash: rules.leash,
    };
    let mut within = |flat: i32, p: V2| match ground {
        Some(g) => g.crossing(&synced[flat as usize].walker, p, p) > 0.0,
        None => true,
    };
    let bits = [Key::Touch.bit(), Key::FoeAt.bit()];
    let (gathered, out, active) =
        seeking::seek(field, &say.squad_say(), &params, &mut within, bits);
    let planned: Vec<i64> = (0..gathered.fighters.len())
        .flat_map(|n| {
            let foes = gathered.foe_counts[n] as i64;
            [gathered.fighters[n], foes, gathered.member_counts[n] as i64]
        })
        .collect();
    let slots: Vec<usize> =
        gathered.members.iter().map(|&flat| field.flat[flat as usize] as usize).collect();
    let from: Vec<V2> = slots.iter().map(|&s| field.columns.loose_at[s]).collect();
    let units = (&planned[..], &gathered.members[..]);
    walk(squads, field, roster, units, (&out.codes, &out.nexts), (&gathered.speeds, rules));
    if ground.is_some() {
        let walker = |m: usize| &synced[gathered.members[m] as usize].walker;
        pay(field, (&slots, &from), &out.codes, walker, ground);
    }
    let facing = facing::Params {
        reach: rules.reach,
        front: rules.front,
        seconds: rules.seconds,
        pace: rules.pace,
        crowding: rules.crowding,
    };
    let valued = Key::Toward.valued();
    let (foe_ids, members) = (&gathered.foe_ids, &gathered.members);
    face_store::face(field, &planned, foe_ids, members, &gathered.speeds, valued, &facing);
    (0..gathered.fighters.len()).filter(|&n| active[n] == 1).map(|n| gathered.fighters[n]).collect()
}

/// BodyFieldSync.scrum: the field brought to the listed squads' living units, in the lists'
/// order, those new to it enrolled with their record. Returns each synced unit's member, by
/// its flat index.
fn sync<'r>(squads: &Squads, field: &mut Field, roster: &'r Roster, seed: i64) -> Vec<&'r Member> {
    let mut ids = Vec::with_capacity(roster.listed.len());
    let mut factions = Vec::with_capacity(roster.listed.len());
    let mut flags = Vec::with_capacity(roster.listed.len());
    let mut counts = Vec::with_capacity(roster.listed.len());
    let mut members = Vec::new();
    for listed in &roster.listed {
        ids.push(listed.id);
        factions.push(format!("#{}", listed.faction)); // the field tells factions by name
        flags.push(match squads.state[listed.squad] {
            ROUTING => field::ROUTING,
            DESTROYED => field::DESTROYED,
            _ => 0,
        });
        counts.push(listed.living.len() as i32);
        members.extend(listed.living.iter());
    }
    let unit_ids: Vec<i64> = members.iter().map(|m| m.id).collect();
    let input = SyncIn {
        squad_ids: &ids,
        squad_factions: &factions,
        squad_flags: &flags,
        squad_counts: &counts,
        unit_ids: &unit_ids,
        mode: field::SCRUM,
        bases: &[],
    };
    let unknown = field.sync(seed, &input);
    if !unknown.is_empty() {
        enrol(field, &unknown, &members, seed);
    }
    members
}

/// BodyFieldSync._enrol: what the field needs once about each new unit.
fn enrol(field: &mut Field, flat: &[i32], members: &[&Member], seed: i64) {
    let new: Vec<&Member> = flat.iter().map(|&f| members[f as usize]).collect();
    let radii: Vec<f64> = new.iter().map(|m| m.radius()).collect();
    let areas: Vec<f64> = new.iter().map(|m| (m.foot.0 * m.foot.1) as f64).collect();
    let draws: Vec<i64> = new.iter().map(|m| contest::draw(seed, m.id)).collect();
    let initiatives: Vec<i64> =
        new.iter().map(|m| field.columns.sheet[m.slot].initiative).collect();
    let records: Vec<i64> = new.iter().flat_map(|m| [m.preferred, m.foot.0, m.foot.1]).collect();
    let paces: Vec<f64> = new.iter().flat_map(|m| [m.turn_rate, m.backward_pace]).collect();
    field.enrol(&Enrol {
        flat,
        radii: &radii,
        areas: &areas,
        draws: &draws,
        initiatives: &initiatives,
        records: &records,
        paces: &paces,
    });
}

/// ScrumSeekField._say: what the rules say of each fighting squad, in the lists' order, and
/// each synced unit's speed.
#[derive(Default)]
struct Say {
    ids: Vec<i64>,
    foe_counts: Vec<i32>,
    foe_ids: Vec<i64>,
    seeking: Vec<u8>,
    frames: Vec<f64>,
    anchors: Vec<V2>,
    chaser_counts: Vec<i32>,
    chaser_ids: Vec<i64>,
    speeds: Vec<f64>,
}

impl Say {
    fn of(squads: &Squads, field: &Field, roster: &Roster, synced: &[&Member]) -> Say {
        let listed: Vec<usize> = roster.listed.iter().map(|l| l.squad).collect();
        let factions: Vec<u32> = roster.listed.iter().map(|l| l.faction).collect();
        let mut say = Say::default();
        for (n, &s) in listed.iter().enumerate() {
            if squads.state[s] != FIGHTING {
                continue;
            }
            let foes = locks::foe_ids(squads, s, &listed, &factions, factions[n]);
            say.ids.push(squads.id[s]);
            say.foe_counts.push(foes.len() as i32);
            say.foe_ids.extend_from_slice(&foes);
            say.seeking.push((squads.order[s] != locks::RETREAT) as u8);
            let (frame, anchor) = frame(squads, s);
            say.frames.extend_from_slice(&frame[1..]);
            say.anchors.push(anchor);
            say.chaser_counts.push(squads.chasers[s].len() as i32);
            say.chaser_ids.extend(squads.chasers[s].iter().map(|c| c.unit));
        }
        say.speeds = synced.iter().map(|m| field.columns.speed[m.slot]).collect();
        say
    }

    fn squad_say(&self) -> SquadSay<'_> {
        SquadSay {
            ids: &self.ids,
            foe_counts: &self.foe_counts,
            foe_ids: &self.foe_ids,
            seeking: &self.seeking,
            frames: &self.frames,
            anchors: &self.anchors,
            chaser_counts: &self.chaser_counts,
            chaser_ids: &self.chaser_ids,
            speeds: &self.speeds,
        }
    }
}

/// ScrumStance.frame: the squad's frame, [0 (its pace's place), heading, width, centre
/// shift], and its anchor: its stance's, else its own.
fn frame(squads: &Squads, s: usize) -> ([f64; 4], V2) {
    let width = squads.width[s] as f64;
    match squads.stance[s] {
        Some(stance) => ([0.0, stance.heading, width, 0.0], stance.anchor),
        None => ([0.0, squads.heading[s], width, squads.centre_shift[s]], squads.position[s]),
    }
}

/// ScrumWalkField.walk: each planned squad's pace (crowded, out of a pursuit) and frame, the
/// bodies lying on the field, and every planned unit's step.
fn walk(
    squads: &Squads,
    field: &mut Field,
    roster: &Roster,
    (planned, members): (&[i64], &[i32]),
    (codes, nexts): (&[u8], &[V2]),
    (speeds, rules): (&[f64], &Rules),
) {
    let mut squad_floats = Vec::with_capacity(planned.len() / 3 * 4);
    let mut squad_anchors = Vec::with_capacity(planned.len() / 3);
    for triple in planned.chunks(3) {
        let s = roster.listed[roster.by_id(triple[0]).expect("a planned squad is listed")].squad;
        let crowding = if squads.pursuit[s].is_none() { rules.crowding } else { 1.0 };
        let (mut frame, anchor) = frame(squads, s);
        frame[0] = rules.pace * crowding;
        squad_floats.extend_from_slice(&frame);
        squad_anchors.push(anchor);
    }
    let lying = Bodies::of(squads, field, roster);
    let say = WalkSay {
        squad_floats: &squad_floats,
        squad_anchors: &squad_anchors,
        lying_ats: &lying.ats,
        lying_radii: &lying.radii,
        lying_areas: &lying.areas,
    };
    let params = walking::Params {
        seed: rules.seed,
        seconds: rules.seconds,
        look: rules.look,
        clear: rules.clear,
        drag: rules.drag,
        block: rules.block,
    };
    let bits = [Key::Toward.bit(), Key::Toward.valued()];
    walk_store::walk(field, planned, &say, members, codes, nexts, speeds, bits, &params);
}

/// ScrumWalkField.pay: each unit that stepped, its step (from where it stood, `from`) paid
/// for the ground it crosses; one walking back to its place stands where it got.
fn pay<'w>(
    field: &mut Field,
    (slots, from): (&[usize], &[V2]),
    codes: &[u8],
    walker: impl Fn(usize) -> &'w Walker,
    ground: Option<&Ground>,
) {
    for (m, &slot) in slots.iter().enumerate() {
        if codes[m] == TOUCH {
            continue; // it didn't step
        }
        let c = &mut field.columns;
        let paid = mover::pay(ground, walker(m), from[m], c.loose_at[slot]);
        c.loose_at[slot] = paid;
        if codes[m] != AIM && codes[m] != PRESS {
            c.next[slot] = paid;
        }
    }
}

#[cfg(test)]
mod tests;
