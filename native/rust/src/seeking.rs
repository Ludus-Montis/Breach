//! The scrum's seeking on the unit store (spec 35 step 4b): the core's input gathered on the
//! field, and its outcome written into the store's columns there, as ScrumSeekField did in
//! GDScript. GDScript sends only what its rules say of each fighting squad: its foes,
//! whether its units may seek (not ordered to retreat), its frame, its chasers, and each
//! unit's speed. The rest is the field's own: each unit's record (preferred band and
//! footprint, enrolled once), its place in its squad's frame (the store's rank and column,
//! turned to SquadFrame.place_in's point), and the goal its loose entry keeps.

use crate::field::Field;
use crate::maths::{bearing_vector, V2};
use crate::scrum::{PlanIn, PlanOut, AIM, PRESS, STAND, TOUCH};

/// What GDScript's rules say of each fighting squad, in the sync's order.
pub struct SquadSay<'a> {
    pub ids: &'a [i64],
    pub foe_counts: &'a [i32],
    pub foe_ids: &'a [i64],
    /// 1: its units may seek (it isn't ordered to retreat).
    pub seeking: &'a [u8],
    /// Its frame (ScrumStance.frame), 3 per squad: heading, width, centre shift.
    pub frames: &'a [f64],
    /// Its frame's anchor.
    pub anchors: &'a [V2],
    pub chaser_counts: &'a [i32],
    /// Its chasers' unit ids (ScrumPursuit's: they seek no slot).
    pub chaser_ids: &'a [i64],
    /// Each unit's speed, by the sync's flat index.
    pub speeds: &'a [f64],
}

/// The core's input (scrum::PlanIn), gathered on the field.
#[derive(Default)]
pub struct Gathered {
    pub fighters: Vec<i64>,
    pub foe_counts: Vec<i32>,
    pub foe_ids: Vec<i64>,
    pub member_counts: Vec<i32>,
    pub members: Vec<i32>,
    pub may_seek: Vec<u8>,
    pub speeds: Vec<f64>,
    pub anchors: Vec<V2>,
    pub goal_foes: Vec<i64>,
    pub goal_slots: Vec<i32>,
}

impl Gathered {
    pub fn input(&self) -> PlanIn<'_> {
        PlanIn {
            fighters: &self.fighters,
            foe_counts: &self.foe_counts,
            foe_ids: &self.foe_ids,
            member_counts: &self.member_counts,
            members: &self.members,
            may_seek: &self.may_seek,
            speeds: &self.speeds,
            anchors: &self.anchors,
            goal_foes: &self.goal_foes,
            goal_slots: &self.goal_slots,
        }
    }
}

/// ScrumSeekField._gather on the field: each fighting squad's living units in its list's
/// order, its chasers left out. A squad the sync named no living unit of is left out, as
/// GDScript's walk over the synced units never meets it.
pub fn gather(field: &Field, say: &SquadSay) -> Gathered {
    let mut out = Gathered::default();
    let (mut foes_at, mut chasers_at) = (0usize, 0usize);
    for (s, &id) in say.ids.iter().enumerate() {
        let foe_count = say.foe_counts[s] as usize;
        let foes = &say.foe_ids[foes_at..foes_at + foe_count];
        foes_at += foe_count;
        let chaser_count = say.chaser_counts[s] as usize;
        let chasers = &say.chaser_ids[chasers_at..chasers_at + chaser_count];
        chasers_at += chaser_count;
        let Some(sq) = field.squad_index(id) else {
            continue;
        };
        let members = &field.squads[sq as usize].members;
        if members.is_empty() {
            continue;
        }
        out.fighters.push(id);
        out.foe_counts.push(foe_count as i32);
        out.foe_ids.extend_from_slice(foes);
        let front_left = members.iter().any(|&b| field.preferred[b as usize] == 0);
        let frame = &say.frames[3 * s..3 * s + 3];
        let ahead = bearing_vector(frame[0]);
        let before = out.members.len();
        for &body in members {
            let b = body as usize;
            if chasers.contains(&field.unit_id[b]) {
                continue;
            }
            let seeks = say.seeking[s] != 0
                && (field.preferred[b] == 0 || !front_left)
                && field.foot_width[b] == 1
                && field.foot_depth[b] == 1;
            let flat = field.flat_of[b] as usize;
            out.members.push(flat as i32);
            out.may_seek.push(seeks as u8);
            out.speeds.push(say.speeds[flat]);
            out.anchors.push(if seeks {
                place(field, b, say.anchors[s], frame, ahead)
            } else {
                V2::ZERO
            });
            out.goal_foes.push(field.columns.goal_foe[b]);
            out.goal_slots.push(field.columns.goal_slot[b] as i32);
        }
        out.member_counts.push((out.members.len() - before) as i32);
    }
    out
}

/// SquadFrame.place_in: the centre of the unit's place in its squad's frame at `anchor`
/// (`frame` = [heading, width, centre shift], `ahead` its heading's vector), from the
/// store's rank and column and the unit's footprint, in GDScript's precision.
fn place(field: &Field, b: usize, anchor: V2, frame: &[f64], ahead: V2) -> V2 {
    let column = field.columns.column[b] as f64;
    let rank = field.columns.rank[b] as f64;
    let across = column - frame[1] / 2.0 + frame[2] + field.foot_width[b] as f64 / 2.0;
    let back = rank + field.foot_depth[b] as f64 / 2.0;
    let orthogonal = V2::new(ahead.y, -ahead.x);
    anchor.add(orthogonal.scale(-across)).sub(ahead.scale(back))
}

/// ScrumSeekField._write_back on the field: the core's outcome written into each planned
/// unit's loose entry in the store's columns - whether it touches a foe, the slot it makes
/// for and its foe's place, its next point (where it stands, touching or with nowhere to
/// go). `bits` = EntryKeys' [touch, foe_at] key bits. Returns, per fighting squad, 1 if
/// any of its units touches a foe or makes for a slot.
pub fn write_back(
    field: &mut Field,
    gathered: &Gathered,
    out: &PlanOut,
    bits: [i32; 2],
) -> Vec<u8> {
    let mut active = Vec::with_capacity(gathered.fighters.len());
    let mut member = 0usize;
    for &count in &gathered.member_counts {
        let mut any = 0u8;
        for m in member..member + count as usize {
            let b = field.flat[gathered.members[m] as usize] as usize;
            let code = out.codes[m];
            let c = &mut field.columns;
            c.touch[b] = (code == TOUCH) as i32;
            c.keys[b] |= bits[0];
            if code == AIM || code == PRESS {
                c.goal_foe[b] = out.goal_foes[m];
                c.goal_slot[b] = out.goal_slots[m] as i64;
                c.next[b] = out.nexts[m];
                c.foe_at[b] = out.foe_ats[m];
                c.keys[b] |= bits[1];
            } else {
                c.goal_foe[b] = 0;
                c.goal_slot[b] = -1;
                if code != STAND {
                    c.next[b] = c.loose_at[b]; // touching or idle: it stays where it stands
                }
            }
            if code == TOUCH || code == AIM || code == PRESS {
                any = 1;
            }
        }
        member += count as usize;
        active.push(any);
    }
    active
}
