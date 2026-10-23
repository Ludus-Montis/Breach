//! Breach's native core (Decision 129: Rust for the simulation's hot core, through
//! GDExtension with godot-rust). One class, `BodyField`: the bodies' state, owned here and
//! kept across ticks, and the hot passes that run on it - body parting (UnitBodies), the
//! scrum's slot search (ScrumSeek, SlotSearch, ScrumSlots, ScrumNear) and its facing
//! (FormationScrum._faces, UnitShuffle, UnitMotion). GDScript keeps the rules and stays the
//! reference; the switch is `BREACH_NATIVE` (NativeKernels).

mod blows;
#[cfg(test)]
mod build;
mod columns;
pub mod content;
pub mod designer;
mod det_math;
mod face_store;
mod facing;
mod field;
mod field_parting;
mod ghash;
mod grid;
pub mod ground;
mod maths;
mod motion;
mod mover;
mod ordered;
mod parting;
mod pathing;
mod paths;
mod route;
mod scrum;
mod seeking;
mod snapshot;
mod squads;
mod steer;
mod walk_store;
mod walking;

use std::time::Instant;

use godot::prelude::*;

use field::{Enrol, Field, SyncIn};
use maths::V2;
use parting::Tuning;
use squads::Squads;

struct BreachNative;

#[gdextension]
unsafe impl ExtensionLibrary for BreachNative {}

/// The bodies on a battle's field (see native/README.md for the boundary).
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct BodyField {
    field: Field,
    /// The squad store (spec 39 step 1, SquadColumnsNative).
    squads: Squads,
    /// Microseconds of native work in the last call (the bench reads it).
    usec: i64,
    /// The last part()'s moved bodies (flat indices), points and nexts, for commit_part().
    parted: (Vec<i32>, Vec<V2>, Vec<V2>),
}

#[godot_api]
impl BodyField {
    /// Brings the field to the squads as they stand, flat over the squads in list order:
    /// per squad its id, faction, flags (1 routing, 2 destroyed) and how many living units
    /// follow; per unit its id and - when not empty - its base (fleeing). `mode` says which
    /// snapshot: 1 the scrum's (loose or not, and its bearing), 2 the bodies' (loose or
    /// fleeing, and a loose one's next point). How each stands, its next point and bearing
    /// are read here from the store's columns (spec 35 step 4e), and where it stands by its
    /// flags: its loose place, its offset, or its position.
    /// Returns the flat indices of units new to the field: enrol() them before any pass.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn sync(
        &mut self,
        seed: i64,
        squad_ids: PackedInt64Array,
        squad_factions: PackedStringArray,
        squad_flags: PackedByteArray,
        squad_counts: PackedInt32Array,
        unit_ids: PackedInt64Array,
        mode: i64,
        bases: PackedVector2Array,
    ) -> PackedInt32Array {
        let began = Instant::now();
        let factions: Vec<String> =
            squad_factions.as_slice().iter().map(|s| s.to_string()).collect();
        let bases = v2s(&bases);
        let input = SyncIn {
            squad_ids: squad_ids.as_slice(),
            squad_factions: &factions,
            squad_flags: squad_flags.as_slice(),
            squad_counts: squad_counts.as_slice(),
            unit_ids: unit_ids.as_slice(),
            mode: mode as u8,
            bases: &bases,
        };
        let unknown = self.field.sync(seed, &input);
        self.usec = began.elapsed().as_micros() as i64;
        PackedInt32Array::from(unknown.as_slice())
    }

    /// Enrols the units sync() reported new (by flat index): radius, footprint area,
    /// ScrumContest.draw and initiative - GDScript's own values, handed over once - and
    /// each one's record, 3 per unit: preferred position, footprint width and depth, and
    /// its paces, 2 per unit: turn rate and backward pace.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn enrol(
        &mut self,
        flat: PackedInt32Array,
        radii: PackedFloat64Array,
        areas: PackedFloat64Array,
        draws: PackedInt64Array,
        initiatives: PackedInt64Array,
        records: PackedInt64Array,
        paces: PackedFloat64Array,
    ) {
        self.field.enrol(&Enrol {
            flat: flat.as_slice(),
            radii: radii.as_slice(),
            areas: areas.as_slice(),
            draws: draws.as_slice(),
            initiatives: initiatives.as_slice(),
            records: records.as_slice(),
            paces: paces.as_slice(),
        });
    }

    /// The record enrolled for the unit in `slot`: [preferred position, footprint width,
    /// depth, turn rate, backward pace] (tests).
    #[func]
    fn record(&self, slot: i64) -> PackedFloat64Array {
        let s = slot as usize;
        let f = &self.field;
        PackedFloat64Array::from(&[
            f.preferred[s] as f64,
            f.foot_width[s] as f64,
            f.foot_depth[s] as f64,
            f.turn_rate[s],
            f.backward_pace[s],
        ])
    }

    /// UnitBodies' passes on the field. `tuning` = [passes, resist, brush]; `way.call(draw,
    /// other draw)` gives the way two bodies on each other part. Returns [moved (flat
    /// indices), their points (a fleeing one's offset), their nexts, loosened (flat indices,
    /// in order), stats = [usec, passes that moved something, pairs pushed]]; commit_part()
    /// then writes the moved into the store's columns.
    #[func]
    fn part(&mut self, tuning: PackedFloat64Array, way: Callable, threaded: bool) -> VarArray {
        let t = tuning.as_slice();
        let tuning = Tuning { passes: t[0] as i64, resist: t[1], brush: t[2] };
        let mut ask = |a: i64, b: i64| -> V2 {
            let w: Vector2 = way.callv(&varray![a, b]).to();
            V2::new(w.x, w.y)
        };
        let began = Instant::now();
        let out = field_parting::part_field(&mut self.field, &tuning, threaded, &mut ask);
        self.usec = began.elapsed().as_micros() as i64;
        let result = varray![
            &PackedInt32Array::from(out.moved.as_slice()),
            &packed(&out.points),
            &packed(&out.nexts),
            &PackedInt32Array::from(out.loosened.as_slice()),
            &PackedInt64Array::from(&[self.usec, out.passes, out.pairs]),
        ];
        self.parted = (out.moved, out.points, out.nexts);
        result
    }

    /// Writes the last part()'s moved bodies into the unit store's columns, by how each
    /// stands now (a fleeing one's offset, a loose one's point and next, else its position):
    /// called once GDScript has given the bodies it made loose their entries (spec 35,
    /// Decision 144).
    #[func]
    fn commit_part(&mut self) {
        let (moved, points, nexts) = std::mem::take(&mut self.parted);
        field_parting::commit(&mut self.field, &moved, &points, &nexts);
    }

    /// ScrumSeek.plan on the field, as of a sync of the scrum's snapshot, reading and
    /// writing the unit store's columns there (spec 35 step 4b). `ints` = [tick, seed,
    /// combat_contest_die, EntryKeys' touch bit, its foe_at bit], `floats` = [reach_contact,
    /// scrum_leash]. Per fighting squad, in the sync's order: its id, how many foes, their
    /// squad ids, whether its units may seek (1: not ordered to retreat), its frame
    /// ([heading, width, centre shift] and its anchor), how many chasers and their unit ids;
    /// per unit of the sync (flat index) its speed. `within.call(flat index, point)` is the
    /// ground's say (an invalid Callable: open ground). Each unit's place, goal and record
    /// are read on the field, and its outcome written into its loose entry's columns there.
    /// Returns, for the walk and the facing: [the squads planned (those of `squads` the
    /// sync named a living unit of), their units' flat indices, units per squad, codes (0
    /// touch, 1 stand, 2 aim, 3 press, 4 idle), nexts, foe_ats, active (per squad: 1 if any
    /// unit touches a foe or makes for a slot)].
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn seek(
        &mut self,
        ints: PackedInt64Array,
        floats: PackedFloat64Array,
        squads: PackedInt64Array,
        foe_counts: PackedInt32Array,
        foe_ids: PackedInt64Array,
        seeking: PackedByteArray,
        frames: PackedFloat64Array,
        anchors: PackedVector2Array,
        chaser_counts: PackedInt32Array,
        chaser_ids: PackedInt64Array,
        speeds: PackedFloat64Array,
        within: Callable,
    ) -> VarArray {
        let began = Instant::now();
        let i = ints.as_slice();
        let f = floats.as_slice();
        let params = scrum::Params { tick: i[0], seed: i[1], die: i[2], reach: f[0], leash: f[1] };
        let anchors = v2s(&anchors);
        let say = seeking::SquadSay {
            ids: squads.as_slice(),
            foe_counts: foe_counts.as_slice(),
            foe_ids: foe_ids.as_slice(),
            seeking: seeking.as_slice(),
            frames: frames.as_slice(),
            anchors: &anchors,
            chaser_counts: chaser_counts.as_slice(),
            chaser_ids: chaser_ids.as_slice(),
            speeds: speeds.as_slice(),
        };
        let gathered = seeking::gather(&self.field, &say);
        let input = gathered.input();
        let ground = within.is_valid();
        let members = &gathered.members;
        let mut ask = |member: usize, p: V2| -> bool {
            let flat = members[member] as i64;
            !ground || within.callv(&varray![flat, Vector2::new(p.x, p.y)]).to::<bool>()
        };
        let out = scrum::plan(&self.field, &input, &params, &mut ask);
        let active =
            seeking::write_back(&mut self.field, &gathered, &out, [i[3] as i32, i[4] as i32]);
        self.usec = began.elapsed().as_micros() as i64;
        varray![
            &PackedInt64Array::from(gathered.fighters.as_slice()),
            &PackedInt32Array::from(gathered.members.as_slice()),
            &PackedInt32Array::from(gathered.member_counts.as_slice()),
            &PackedByteArray::from(out.codes.as_slice()),
            &packed(&out.nexts),
            &packed(&out.foe_ats),
            &PackedByteArray::from(active.as_slice()),
        ]
    }

    /// FormationScrum's facing on the field, after the walk, for the units the tick's
    /// seek() planned (`squads` = per fighting squad [id, foes, units], `foe_ids`,
    /// `members` and `speeds` as seek's). `floats` = [reach_contact, the front's least
    /// dot (ScrumReach.in_front), tick seconds, pace (cells a tick at speed 1),
    /// scrum_crowding]; `valued` = EntryKeys' bit saying a unit's walk chose a point to
    /// look towards. Where each unit stands, faces and makes for (next, foe_at, toward)
    /// are read from the unit store's columns, its turn rate and backward pace from its
    /// record, and its bearing after its turn is written into the column (spec 35 step
    /// 4d). Returns every unit's bearing after its turn (tests).
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn face(
        &mut self,
        floats: PackedFloat64Array,
        squads: PackedInt64Array,
        foe_ids: PackedInt64Array,
        members: PackedInt32Array,
        speeds: PackedFloat64Array,
        valued: i64,
    ) -> PackedFloat64Array {
        let began = Instant::now();
        let f = floats.as_slice();
        let params =
            facing::Params { reach: f[0], front: f[1], seconds: f[2], pace: f[3], crowding: f[4] };
        let triples = squads.as_slice();
        let foe_counts: Vec<i32> = triples.chunks(3).map(|c| c[1] as i32).collect();
        let member_counts: Vec<i32> = triples.chunks(3).map(|c| c[2] as i32).collect();
        let members = members.as_slice();
        let gathered = face_store::gather(&self.field, members, valued as i32);
        let input = facing::FaceIn {
            foe_counts: &foe_counts,
            foe_ids: foe_ids.as_slice(),
            member_counts: &member_counts,
            members,
            ats: &gathered.ats,
            motion: &gathered.motion,
            speeds: speeds.as_slice(),
            flags: &gathered.flags,
            nexts: &gathered.nexts,
            foe_ats: &gathered.foe_ats,
            towards: &gathered.towards,
        };
        let out = facing::face(&mut self.field, &input, &params);
        face_store::write_back(&mut self.field, members, &out);
        self.usec = began.elapsed().as_micros() as i64;
        PackedFloat64Array::from(out.as_slice())
    }

    /// ScrumBlows.blows' search on the field, as of a sync of the scrum's snapshot:
    /// `floats` = [reach_contact, the front's least dot (ScrumReach.in_front)], `rules` per
    /// squad in the sync's order (1 fighting, 2 ordered to retreat). Returns per unit that
    /// touches a foe it strikes, in the sync's order: [its flat index, the foe's, flags (1
    /// it retreats and has turned away, 2 the blow falls on the foe's flank)].
    #[func]
    fn blows(&mut self, floats: PackedFloat64Array, rules: PackedByteArray) -> PackedInt32Array {
        let began = Instant::now();
        let f = floats.as_slice();
        let params = blows::Params { reach: f[0], front: f[1] };
        let picks = blows::picks(&self.field, rules.as_slice(), &params);
        let mut out = Vec::with_capacity(picks.len() * 3);
        for pick in &picks {
            out.extend_from_slice(&[pick.flat, pick.foe, pick.flags as i32]);
        }
        self.usec = began.elapsed().as_micros() as i64;
        PackedInt32Array::from(out.as_slice())
    }

    /// BlowLanding's counts on the field, as of the same sync as blows(): `targets` is
    /// every unit's target id by flat index, `asked` [striker id, target id] per blow.
    /// Returns per blow [the striker's squad (its place in the sync, -1: none), how many
    /// units touching the target (within `reach`) have it as theirs].
    #[func]
    fn pressed(
        &mut self,
        reach: f64,
        targets: PackedInt64Array,
        asked: PackedInt64Array,
    ) -> PackedInt32Array {
        let began = Instant::now();
        let out = blows::pressed(&self.field, targets.as_slice(), asked.as_slice(), reach);
        self.usec = began.elapsed().as_micros() as i64;
        PackedInt32Array::from(out.as_slice())
    }

    /// FormationScrum's walk on the field, before the facing, for the units the tick's
    /// seek() planned (`squads` and `members` as seek's; where each stands as that
    /// call's sync left it). `floats` = [tick seconds, bodies_steer_look,
    /// bodies_steer_clear, bodies_ground_drag, bodies_ground_block]; `bits` = EntryKeys'
    /// [toward's key bit, its valued bit]; per squad `squad_floats` = [pace (cells a tick at
    /// speed 1), heading, width, centre shift] and its frame's anchor; per unit: seek()'s
    /// code and next point, its speed; the bodies lying on the ground and their [radius,
    /// footprint area]. Each unit's bearing, rank and column are read from the unit store's
    /// columns, its turn rate, backward pace and footprint from its record (spec 35 step
    /// 4c), and where it ends and looks is written into its loose entry's columns there.
    /// Returns [where each unit stands after its step, where it looks: its next point
    /// making for a slot, UnitShuffle's walking back to its place] (tests).
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn walk(
        &mut self,
        seed: i64,
        floats: PackedFloat64Array,
        squads: PackedInt64Array,
        squad_floats: PackedFloat64Array,
        squad_anchors: PackedVector2Array,
        members: PackedInt32Array,
        codes: PackedByteArray,
        nexts: PackedVector2Array,
        speeds: PackedFloat64Array,
        bits: PackedInt32Array,
        lying_ats: PackedVector2Array,
        lying_sizes: PackedFloat64Array,
    ) -> VarArray {
        let began = Instant::now();
        let f = floats.as_slice();
        let params = walking::Params {
            seed,
            seconds: f[0],
            look: f[1],
            clear: f[2],
            drag: f[3],
            block: f[4],
        };
        let member_counts: Vec<i32> = squads.as_slice().chunks(3).map(|c| c[2] as i32).collect();
        let sizes = lying_sizes.as_slice();
        let lying_radii: Vec<f64> = sizes.chunks(2).map(|c| c[0]).collect();
        let lying_areas: Vec<f64> = sizes.chunks(2).map(|c| c[1]).collect();
        let (anchors, nexts, lying) = (v2s(&squad_anchors), v2s(&nexts), v2s(&lying_ats));
        let (members, codes) = (members.as_slice(), codes.as_slice());
        let (motion, frames) = walk_store::gather(&self.field, members, codes);
        let input = walking::WalkIn {
            member_counts: &member_counts,
            squad_floats: squad_floats.as_slice(),
            squad_anchors: &anchors,
            members,
            codes,
            nexts: &nexts,
            speeds: speeds.as_slice(),
            motion: &motion,
            frames: &frames,
            lying_ats: &lying,
            lying_radii: &lying_radii,
            lying_areas: &lying_areas,
        };
        let out = walking::walk(&self.field, &input, &params);
        let b = bits.as_slice();
        walk_store::write_back(
            &mut self.field,
            members,
            codes,
            &out.ats,
            &out.towards,
            [b[0], b[1]],
        );
        self.usec = began.elapsed().as_micros() as i64;
        varray![&packed(&out.ats), &packed(&out.towards)]
    }

    /// DetMath's `op` ("sin", "cos", "asin", "acos", or "atan2" of a and b) over the
    /// inputs, as the core computes it (the suite proves it against the GDScript).
    #[func]
    fn det_math(op: GString, a: PackedFloat64Array, b: PackedFloat64Array) -> PackedFloat64Array {
        let f: fn(f64, f64) -> f64 = match op.to_string().as_str() {
            "sin" => |x, _| det_math::sin(x),
            "cos" => |x, _| det_math::cos(x),
            "asin" => |x, _| det_math::asin(x),
            "acos" => |x, _| det_math::acos(x),
            "atan2" => det_math::atan2,
            _ => return PackedFloat64Array::new(),
        };
        let (a, b) = (a.as_slice(), b.as_slice());
        let out: Vec<f64> =
            (0..a.len()).map(|i| f(a[i], b.get(i).copied().unwrap_or(0.0))).collect();
        PackedFloat64Array::from(out.as_slice())
    }

    /// The squad store (spec 39 step 1) takes a new squad: returns its slot, kept for the
    /// battle, every column blank there.
    #[func]
    fn squad_open(&mut self, squad_id: i64) -> i64 {
        self.squads.open(squad_id) as i64
    }

    /// An int column's value in a squad's slot, by the column's index (SquadColumns).
    #[func]
    fn squad_int(&mut self, slot: i64, column: i64) -> i64 {
        self.squads.int(slot as usize, column as usize)
    }

    #[func]
    fn put_squad_int(&mut self, slot: i64, column: i64, value: i64) {
        self.squads.set_int(slot as usize, column as usize, value);
    }

    /// A real column's value in a squad's slot.
    #[func]
    fn squad_real(&mut self, slot: i64, column: i64) -> f64 {
        self.squads.real(slot as usize, column as usize)
    }

    #[func]
    fn put_squad_real(&mut self, slot: i64, column: i64, value: f64) {
        self.squads.set_real(slot as usize, column as usize, value);
    }

    /// The point column's value in a squad's slot.
    #[func]
    fn squad_point(&mut self, slot: i64, column: i64) -> Vector2 {
        let p = self.squads.point(slot as usize, column as usize);
        Vector2::new(p.x, p.y)
    }

    #[func]
    fn put_squad_point(&mut self, slot: i64, column: i64, value: Vector2) {
        self.squads.set_point(slot as usize, column as usize, V2::new(value.x, value.y));
    }

    /// A squad's members: its units' unit-store slots, in its list's order.
    #[func]
    fn squad_members(&self, slot: i64) -> PackedInt32Array {
        PackedInt32Array::from(self.squads.members[slot as usize].as_slice())
    }

    #[func]
    fn put_squad_members(&mut self, slot: i64, members: PackedInt32Array) {
        self.squads.members[slot as usize] = members.as_slice().to_vec();
    }

    /// The crate's whole state after `tick` of the battle seeded `seed` (spec 39 step 1,
    /// `snapshot`): the units, the squads, the tick and the seed.
    #[func]
    fn snapshot(&self, tick: i64, seed: i64) -> PackedByteArray {
        PackedByteArray::from(snapshot::write(tick, seed, &self.field, &self.squads).as_slice())
    }

    /// Squads the squad store holds.
    #[func]
    fn squad_count(&self) -> i64 {
        self.squads.count() as i64
    }

    /// Reads back a state `snapshot` wrote, in place of this one; returns its [tick, seed],
    /// or nothing (the state unchanged) if the bytes aren't a state.
    #[func]
    fn restore(&mut self, bytes: PackedByteArray) -> PackedInt64Array {
        match snapshot::read(bytes.as_slice()) {
            Ok(state) => {
                self.field = state.field;
                self.squads = state.squads;
                PackedInt64Array::from(&[state.tick, state.seed])
            }
            Err(_) => PackedInt64Array::new(),
        }
    }

    /// Bodies on the field.
    #[func]
    fn count(&self) -> i64 {
        self.field.count() as i64
    }

    /// The unit store (spec 35, UnitStoreNative) takes a unit: returns its slot, the
    /// lowest free one if it had none. A held unit keeps its slot off the field.
    #[func]
    fn hold(&mut self, unit_id: i64) -> i64 {
        self.field.hold(unit_id) as i64
    }

    /// The unit store lets a unit go; its slot is free for the next.
    #[func]
    fn release(&mut self, unit_id: i64) {
        self.field.release(unit_id);
    }

    /// The store's slot of a unit, -1 if it doesn't hold it.
    #[func]
    fn slot_of(&self, unit_id: i64) -> i64 {
        self.field.slot_of(unit_id).map_or(-1, |s| s as i64)
    }

    /// The unit the store holds in a slot, -1 for a free one.
    #[func]
    fn unit_at(&self, slot: i64) -> i64 {
        if slot < 0 {
            return -1;
        }
        self.field.unit_at(slot as u32).unwrap_or(-1)
    }

    /// Units the store holds.
    #[func]
    fn held_count(&self) -> i64 {
        self.field.held_count() as i64
    }

    /// The units the store holds, by slot.
    #[func]
    fn held_units(&self) -> PackedInt64Array {
        PackedInt64Array::from(self.field.held_units().as_slice())
    }

    /// The unit store's point column `name` (UnitColumns: "position", "at", "offset",
    /// "next", "foe_at", "toward"), by slot: one copy.
    #[func]
    fn column_points(&mut self, name: GString) -> PackedVector2Array {
        match self.field.columns.points(&name.to_string()) {
            Some(column) => column.iter().map(|p| Vector2::new(p.x, p.y)).collect(),
            None => PackedVector2Array::new(),
        }
    }

    /// Replaces a point column with `values` (by slot, as long as the column).
    #[func]
    fn commit_points(&mut self, name: GString, values: PackedVector2Array) {
        let Some(column) = self.field.columns.points(&name.to_string()) else { return };
        if values.len() != column.len() {
            godot_error!("BodyField.commit_points: a column keeps its length");
            return;
        }
        for (slot, p) in values.as_slice().iter().enumerate() {
            column[slot] = V2 { x: p.x, y: p.y };
        }
    }

    /// One slot's value in a point column.
    #[func]
    fn point(&mut self, slot: i64, name: GString) -> Vector2 {
        match self.field.columns.points(&name.to_string()) {
            Some(column) => {
                let p = column[slot as usize];
                Vector2::new(p.x, p.y)
            }
            None => Vector2::ZERO,
        }
    }

    /// Sets one slot's value in a point column.
    #[func]
    fn put_point(&mut self, slot: i64, name: GString, value: Vector2) {
        if let Some(column) = self.field.columns.points(&name.to_string()) {
            column[slot as usize] = V2 { x: value.x, y: value.y };
        }
    }

    /// The unit store's 32-bit int column `name` ("flags", "touch", "keys", "rank", "column"),
    /// by slot: one
    /// copy.
    #[func]
    fn column_ints(&mut self, name: GString) -> PackedInt32Array {
        match self.field.columns.ints(&name.to_string()) {
            Some(column) => PackedInt32Array::from(column.as_slice()),
            None => PackedInt32Array::new(),
        }
    }

    /// Replaces a 32-bit int column with `values` (by slot, as long as the column).
    #[func]
    fn commit_ints(&mut self, name: GString, values: PackedInt32Array) {
        let Some(column) = self.field.columns.ints(&name.to_string()) else { return };
        if values.len() != column.len() {
            godot_error!("BodyField.commit_ints: a column keeps its length");
            return;
        }
        column.copy_from_slice(values.as_slice());
    }

    /// The unit store's 64-bit int column `name` ("goal_foe", "goal_slot", "caught",
    /// "held_by"), by slot: one copy.
    #[func]
    fn column_longs(&mut self, name: GString) -> PackedInt64Array {
        match self.field.columns.longs(&name.to_string()) {
            Some(column) => PackedInt64Array::from(column.as_slice()),
            None => PackedInt64Array::new(),
        }
    }

    /// Replaces a 64-bit int column with `values` (by slot, as long as the column).
    #[func]
    fn commit_longs(&mut self, name: GString, values: PackedInt64Array) {
        let Some(column) = self.field.columns.longs(&name.to_string()) else { return };
        if values.len() != column.len() {
            godot_error!("BodyField.commit_longs: a column keeps its length");
            return;
        }
        column.copy_from_slice(values.as_slice());
    }

    /// The unit store's float column `name` ("stuck", "closest", "along", "fanned"), by
    /// slot: one copy.
    #[func]
    fn column_reals(&mut self, name: GString) -> PackedFloat64Array {
        match self.field.columns.reals(&name.to_string()) {
            Some(column) => PackedFloat64Array::from(column.as_slice()),
            None => PackedFloat64Array::new(),
        }
    }

    /// Replaces a float column with `values` (by slot, as long as the column).
    #[func]
    fn commit_reals(&mut self, name: GString, values: PackedFloat64Array) {
        let Some(column) = self.field.columns.reals(&name.to_string()) else { return };
        if values.len() != column.len() {
            godot_error!("BodyField.commit_reals: a column keeps its length");
            return;
        }
        column.copy_from_slice(values.as_slice());
    }

    /// One slot's value in an int column, 32- or 64-bit.
    #[func]
    fn int_of(&mut self, slot: i64, name: GString) -> i64 {
        let name = name.to_string();
        if let Some(column) = self.field.columns.ints(&name) {
            return column[slot as usize] as i64;
        }
        match self.field.columns.longs(&name) {
            Some(column) => column[slot as usize],
            None => 0,
        }
    }

    /// Sets one slot's value in an int column, 32- or 64-bit.
    #[func]
    fn put_int(&mut self, slot: i64, name: GString, value: i64) {
        let name = name.to_string();
        if let Some(column) = self.field.columns.ints(&name) {
            column[slot as usize] = value as i32;
        } else if let Some(column) = self.field.columns.longs(&name) {
            column[slot as usize] = value;
        }
    }

    /// One slot's value in a float column.
    #[func]
    fn real_of(&mut self, slot: i64, name: GString) -> f64 {
        match self.field.columns.reals(&name.to_string()) {
            Some(column) => column[slot as usize],
            None => 0.0,
        }
    }

    /// Sets one slot's value in a float column.
    #[func]
    fn put_real(&mut self, slot: i64, name: GString, value: f64) {
        if let Some(column) = self.field.columns.reals(&name.to_string()) {
            column[slot as usize] = value;
        }
    }

    /// A unit fell (UnitHealth, spec 35 step 5b): onto the store's fell queue.
    #[func]
    fn fell(&mut self, unit_id: i64) {
        self.field.fell(unit_id);
    }

    /// The units fallen since deaths last took them, in the order they fell; the queue is
    /// left empty.
    #[func]
    fn take_fallen(&mut self) -> PackedInt64Array {
        PackedInt64Array::from(self.field.take_fallen().as_slice())
    }

    /// Every column blank in `slot` (a slot the store has just given).
    #[func]
    fn blank(&mut self, slot: i64) {
        self.field.columns.blank(slot as usize);
    }

    /// Microseconds of native work in the last sync, part, seek, walk or face.
    #[func]
    fn usec(&self) -> i64 {
        self.usec
    }

    /// Where every body on the field stands, by the last sync's flat indices (tests).
    #[func]
    fn points(&self) -> PackedVector2Array {
        self.field
            .flat
            .iter()
            .map(|&b| {
                let p = self.field.at[b as usize];
                Vector2::new(p.x, p.y)
            })
            .collect()
    }

    /// Godot's hash() of an Array of ints and Strings as the core computes it (the suite
    /// proves it against the engine's own).
    #[func]
    fn godot_hash(keys: VarArray) -> i64 {
        let texts: Vec<String> = keys.iter_shared().map(|v| v.to_string()).collect();
        let mut out = Vec::with_capacity(texts.len());
        for (v, text) in keys.iter_shared().zip(&texts) {
            out.push(match v.get_type() {
                VariantType::STRING => ghash::Key::Str(text),
                _ => ghash::Key::Int(v.to::<i64>()),
            });
        }
        ghash::hash(&out) as i64
    }
}

fn v2s(points: &PackedVector2Array) -> Vec<V2> {
    points.as_slice().iter().map(|p| V2::new(p.x, p.y)).collect()
}

fn packed(points: &[V2]) -> PackedVector2Array {
    points.iter().map(|p| Vector2::new(p.x, p.y)).collect()
}

/// The simulation's content as the crate loaded it (spec 39 step 2, Decision 153), handed to
/// GDScript as its definitions (`UnitDef`, `MapDef`...), filled field for field by name, until
/// their last reader ports. Each is made once and shared, as `load()` shared a `.tres`. Also
/// what reads and writes a map file and reads the designer's exports for GDScript. CrateContent
/// is GDScript's way in.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct ContentStore {
    content: Option<content::Content>,
    made: std::collections::BTreeMap<String, Gd<Object>>,
}

#[godot_api]
impl ContentStore {
    /// Loads the content under `root` (an OS path to the repo's `content/`), once: "" when
    /// loaded, else why not.
    #[func]
    fn load(&mut self, root: GString) -> GString {
        if self.content.is_some() {
            return GString::new();
        }
        match content::Content::load(std::path::Path::new(&root.to_string())) {
            Ok(loaded) => {
                self.content = Some(loaded);
                GString::new()
            }
            Err(error) => GString::from(&error.to_string()),
        }
    }

    /// The unit `id`, its items the shared ones; null if there is none.
    #[func]
    fn unit(&mut self, id: GString) -> Variant {
        let key = format!("units/{id}");
        if let Some(made) = self.made.get(&key) {
            return made.to_variant();
        }
        let Some(unit) = self.content.as_ref().and_then(|c| c.units.get(&id.to_string())) else {
            return Variant::nil();
        };
        let mut fields = json_fields(unit);
        let paths = fields.remove("items");
        let made = made_from("UnitDef", &fields);
        let items: VarArray = paths
            .as_ref()
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .map(|path| self.item(path.as_str().unwrap_or_default()))
            .collect();
        made.get("items").call("assign", &[items.to_variant()]);
        self.made.insert(key, made.clone());
        made.to_variant()
    }

    /// The weapon `id`; null if there is none.
    #[func]
    fn weapon(&mut self, id: GString) -> Variant {
        self.item(&format!("weapons/{id}"))
    }

    /// The armour `id`; null if there is none.
    #[func]
    fn armour(&mut self, id: GString) -> Variant {
        self.item(&format!("armour/{id}"))
    }

    /// The battle's tuning.
    #[func]
    fn tuning(&mut self) -> Variant {
        let fields = self.content.as_ref().map(|c| json_fields(&c.tuning));
        self.made_once("tuning", "BattleTuning", fields)
    }

    /// The one shared terrain library (Decision 32).
    #[func]
    fn terrain_library(&mut self) -> Variant {
        let fields = self.content.as_ref().map(|c| json_fields(&c.terrain));
        self.made_once("terrain", "TerrainLibraryDef", fields)
    }

    /// The map `name` (content/maps/<name>.json); null if there is none. Its layout's
    /// terrain library is CrateContent's, so make that first.
    #[func]
    fn map(&mut self, name: GString) -> Variant {
        let key = format!("maps/{name}");
        if let Some(made) = self.made.get(&key) {
            return made.to_variant();
        }
        let Some(map) = self.content.as_ref().and_then(|c| c.maps.get(&name.to_string())) else {
            return Variant::nil();
        };
        let made = map_object(map);
        self.made.insert(key, made.clone());
        made.to_variant()
    }

    /// A fresh definition of each kind filled with the crate's defaults, for the test that
    /// holds them equal to GDScript's.
    #[func]
    fn defaults() -> VarDictionary {
        let mut out = VarDictionary::new();
        let mut put = |class: &str, fields: serde_json::Map<String, serde_json::Value>| {
            out.set(class, &made_from(class, &fields).to_variant());
        };
        let mut unit = json_fields(&content::Unit::default());
        unit.remove("items");
        put("UnitDef", unit);
        put("WeaponDef", json_fields(&content::Weapon::default()));
        put("ArmourDef", json_fields(&content::Armour::default()));
        put("TerrainLibraryDef", json_fields(&content::TerrainLibrary::default()));
        put("TerrainDef", json_fields(&content::Terrain::default()));
        put("LiquidBodyDef", json_fields(&content::LiquidBody::default()));
        put("StratumDef", json_fields(&content::Stratum::default()));
        put("TerrainFeatureDef", json_fields(&content::TerrainFeature::default()));
        put("MaterialDef", json_fields(&content::Material::default()));
        put("HeatTransitionDef", json_fields(&content::HeatTransition::default()));
        put("NodeDef", json_fields(&content::Node::default()));
        put("GarrisonUnitDef", json_fields(&content::GarrisonUnit::default()));
        put("SubnodeDef", json_fields(&content::Subnode::default()));
        put("StructurePlanDef", json_fields(&content::StructurePlan::default()));
        put("StructureFaceDef", json_fields(&content::StructureFace::default()));
        put("MapEdgeDef", json_fields(&content::MapEdge::default()));
        put("FactionDef", json_fields(&content::Faction::default()));
        put("FactionRelationDef", json_fields(&content::FactionRelation::default()));
        put("LossGroupDef", json_fields(&content::LossGroup::default()));
        put("MapLayoutDef", json_fields(&content::MapLayout::default()));
        put("TileDef", json_fields(&content::Tile::default()));
        put("BridgeDef", json_fields(&content::Bridge::default()));
        put("RoadSegmentDef", json_fields(&content::RoadSegment::default()));
        put("RouteDef", json_fields(&content::Route::default()));
        put("ChannelDef", json_fields(&content::Channel::default()));
        let mut lane = json_fields(&content::Lane::default());
        lane.remove("nodes");
        put("LaneDef", lane);
        out.set("MapDef", &map_object(&content::Map::default()).to_variant());
        out
    }

    /// The designer's `export` (a saved designer map) as a MapDef, read and checked as the
    /// designer's save reads it, its terrain checked against the loaded library: {"map",
    /// "errors", "warnings"}. What the tests that build maps from designer exports use.
    #[func]
    fn map_from_export(&self, export: VarDictionary) -> VarDictionary {
        let library = self.content.as_ref().map(|c| c.terrain.clone()).unwrap_or_default();
        let mut read = designer::map_from_export(&json_of(&export.to_variant()), &library);
        if read.errors.is_empty() {
            read.errors = read.map.validate(&library);
        }
        let mut out = VarDictionary::new();
        out.set("map", &map_object(&read.map).to_variant());
        out.set("errors", &variant_of(&read.errors));
        out.set("warnings", &variant_of(&read.warnings));
        out
    }

    /// A designer structure plan (spec 24) as a StructurePlanDef.
    #[func]
    fn plan_from_export(drawn: VarDictionary) -> Variant {
        let (plan, errors) = designer::plan_from_export(&json_of(&drawn.to_variant()));
        for error in errors {
            godot_error!("ContentStore: {error}");
        }
        made_from("StructurePlanDef", &json_fields(&plan)).to_variant()
    }

    /// The map file at `path` (an OS path), read fresh and checked against the loaded
    /// library: {"map": its MapDef}, or {"error": why it didn't load}.
    #[func]
    fn read_map(&self, path: GString) -> VarDictionary {
        let mut out = VarDictionary::new();
        let path = std::path::Path::new(&path.to_string()).to_path_buf();
        let library = self.content.as_ref().map(|c| c.terrain.clone()).unwrap_or_default();
        match content::load_file::<content::Map>(&path) {
            Ok(map) => match map.check(&library) {
                Ok(()) => out.set("map", &map_object(&map).to_variant()),
                Err(why) => out.set("error", &format!("{}: {why}", path.display()).to_variant()),
            },
            Err(error) => out.set("error", &error.to_string().to_variant()),
        }
        out
    }

    /// Writes `map` (a MapDef) to `path` (an OS path) as a map file, once it fits the schema
    /// and keeps the map's rules against the loaded library: "" when written, else why not.
    #[func]
    fn write_map(&self, map: Gd<Object>, path: GString) -> GString {
        let library = self.content.as_ref().map(|c| c.terrain.clone()).unwrap_or_default();
        let value = map_fields(&map);
        let read: Result<content::Map, _> = serde_path_to_error::deserialize(value);
        let written = read
            .map_err(|error| format!("{}: {}", error.path(), error.inner()))
            .and_then(|map| map.check(&library).map(|()| map))
            .and_then(|map| {
                std::fs::write(path.to_string(), content::file_text(&map))
                    .map_err(|error| format!("{path}: {error}"))
            });
        match written {
            Ok(()) => GString::new(),
            Err(why) => GString::from(&why),
        }
    }
}

impl ContentStore {
    /// The item at `path` (weapons/<id> or armour/<id>), made once; null if there is none.
    fn item(&mut self, path: &str) -> Variant {
        if let Some(made) = self.made.get(path) {
            return made.to_variant();
        }
        let content = self.content.as_ref();
        let (class, fields) = match path.split_once('/') {
            Some(("weapons", id)) => {
                ("WeaponDef", content.and_then(|c| c.weapons.get(id)).map(json_fields))
            }
            Some(("armour", id)) => {
                ("ArmourDef", content.and_then(|c| c.armour.get(id)).map(json_fields))
            }
            _ => ("", None),
        };
        self.made_once(path, class, fields)
    }

    fn made_once(
        &mut self,
        key: &str,
        class: &str,
        fields: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Variant {
        if let Some(made) = self.made.get(key) {
            return made.to_variant();
        }
        let Some(fields) = fields else {
            return Variant::nil();
        };
        let made = made_from(class, &fields);
        self.made.insert(key.to_string(), made.clone());
        made.to_variant()
    }
}

/// `value`'s fields as JSON.
fn json_fields(value: &impl serde::Serialize) -> serde_json::Map<String, serde_json::Value> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::Object(fields)) => fields,
        _ => serde_json::Map::new(),
    }
}

/// A MapDef: every node made once (a node on two lanes is one NodeDef), lanes holding them in
/// order, the off-lane rest, and the other fields by name.
fn map_object(map: &content::Map) -> Gd<Object> {
    let mut fields = json_fields(map);
    fields.remove("nodes");
    fields.remove("lanes");
    let nodes: Vec<(String, Gd<Object>)> = map
        .nodes
        .iter()
        .map(|node| (node.id.clone(), made_from("NodeDef", &json_fields(node))))
        .collect();
    let node = |id: &str| nodes.iter().find(|(known, _)| known == id).map(|(_, made)| made.clone());
    let made = made_from("MapDef", &fields);
    let mut lanes = VarArray::new();
    for lane in &map.lanes {
        let mut lane_fields = json_fields(lane);
        lane_fields.remove("nodes");
        let made_lane = made_from("LaneDef", &lane_fields);
        let on_lane: VarArray =
            lane.nodes.iter().filter_map(|id| node(id)).map(|n| n.to_variant()).collect();
        made_lane.get("nodes").call("assign", &[on_lane.to_variant()]);
        lanes.push(&made_lane.to_variant());
    }
    made.get("lanes").call("assign", &[lanes.to_variant()]);
    let off_lane: VarArray =
        map.off_lane_nodes().filter_map(|n| node(&n.id)).map(|n| n.to_variant()).collect();
    made.get("off_lane_nodes").call("assign", &[off_lane.to_variant()]);
    made
}

/// A new definition of the GDScript class `class` (a `class_name`), filled from `fields`.
fn made_from(class: &str, fields: &serde_json::Map<String, serde_json::Value>) -> Gd<Object> {
    let mut made = new_of(&script_named(class));
    fill(&mut made, fields);
    made
}

/// The script declaring `class_name class`; nil if there is none.
fn script_named(class: &str) -> Variant {
    let classes = godot::classes::ProjectSettings::singleton().get_global_class_list();
    for entry in classes.iter_shared() {
        if entry.get("class").is_some_and(|name| name.to_string() == class) {
            let path = entry.get("path").map(|path| path.to_string()).unwrap_or_default();
            return godot::tools::try_load::<godot::classes::Script>(&path)
                .map(|script| script.to_variant())
                .unwrap_or_default();
        }
    }
    godot_error!("ContentStore: no GDScript class {class}");
    Variant::nil()
}

/// An instance of `script` (a Script variant).
fn new_of(script: &Variant) -> Gd<Object> {
    script.call("new", &[]).to::<Gd<Object>>()
}

/// Each property `made` saves (its script's exported fields), by name.
fn saved_properties(made: &Gd<Object>) -> Vec<VarDictionary> {
    const SCRIPT_VARIABLE: i64 = 4096;
    const STORAGE: i64 = 2;
    made.get_property_list()
        .iter_shared()
        .filter(|property| {
            let usage = property.get("usage").map_or(0, |usage| usage.to::<i64>());
            usage & SCRIPT_VARIABLE != 0 && usage & STORAGE != 0
        })
        .collect()
}

fn property_named(made: &Gd<Object>, key: &str) -> Option<VarDictionary> {
    made.get_property_list()
        .iter_shared()
        .find(|property| property.get("name").is_some_and(|name| name.to_string() == key))
}

/// Sets each of `fields` on `made` by name. An item's shared fields ("item") are its own; a
/// typed list keeps its type ("inf" in a list of reals is an infinity, and a list of
/// definitions is made one by one); a name where an enum is kept is the enum's value; a
/// colour is its "#rrggbb"; a vector its list of numbers; a dictionary keyed by cells its
/// [cell, value] pairs; and a definition held alone is made from its fields (none for null).
fn fill(made: &mut Gd<Object>, fields: &serde_json::Map<String, serde_json::Value>) {
    use serde_json::Value;
    for (key, value) in fields {
        if key == "item" {
            if let Value::Object(shared) = value {
                fill(made, shared);
            }
            continue;
        }
        let current = made.get(key.as_str());
        let set = match (current.get_type(), value) {
            (VariantType::ARRAY, Value::Array(values)) => {
                let typed = current.call("get_typed_builtin", &[]).to::<i64>();
                let script = current.call("get_typed_script", &[]);
                let list: VarArray =
                    values.iter().map(|item| element(item, typed, &script)).collect();
                current.call("assign", &[list.to_variant()]);
                continue;
            }
            (VariantType::DICTIONARY, Value::Array(pairs)) => cell_keyed(pairs),
            (
                VariantType::VECTOR2 | VariantType::VECTOR2I | VariantType::VECTOR3I,
                Value::Array(numbers),
            ) => vector(numbers, current.get_type()),
            (VariantType::COLOR, Value::String(text)) => {
                Color::from_html(text.as_str()).unwrap_or(Color::WHITE).to_variant()
            }
            (VariantType::INT, Value::String(name)) => enum_value(made, key, name).to_variant(),
            (VariantType::NIL, Value::Object(inner)) => {
                let class = property_named(made, key)
                    .and_then(|property| property.get("hint_string"))
                    .map(|hint| hint.to_string())
                    .unwrap_or_default();
                made_from(&class, inner).to_variant()
            }
            _ => json_variant(value),
        };
        made.set(key.as_str(), &set);
    }
}

/// A list element as its list's type (`typed`, a Variant.Type, or a definition of `script`).
fn element(item: &serde_json::Value, typed: i64, script: &Variant) -> Variant {
    use serde_json::Value;
    let typed = VariantType::from_ord(typed as i32);
    match (typed, item) {
        (VariantType::FLOAT, Value::String(text)) => {
            (if text == "-inf" { f64::NEG_INFINITY } else { f64::INFINITY }).to_variant()
        }
        (
            VariantType::VECTOR2 | VariantType::VECTOR2I | VariantType::VECTOR3I,
            Value::Array(numbers),
        ) => vector(numbers, typed),
        (VariantType::OBJECT, Value::Object(fields)) => {
            let mut made = new_of(script);
            fill(&mut made, fields);
            made.to_variant()
        }
        _ => json_variant(item),
    }
}

/// [x, y] or [x, y, level] as a vector of `typed` (a point, a cell, a structure cell).
fn vector(numbers: &[serde_json::Value], typed: VariantType) -> Variant {
    let at = |index: usize| numbers.get(index).and_then(serde_json::Value::as_f64).unwrap_or(0.0);
    match typed {
        VariantType::VECTOR2 => Vector2::new(at(0) as f32, at(1) as f32).to_variant(),
        VariantType::VECTOR3I => {
            Vector3i::new(at(0) as i32, at(1) as i32, at(2) as i32).to_variant()
        }
        _ => Vector2i::new(at(0) as i32, at(1) as i32).to_variant(),
    }
}

/// [[cell, value], ...] as {cell: value}, in order; a cell of three numbers is a structure cell.
fn cell_keyed(pairs: &[serde_json::Value]) -> Variant {
    let mut out = VarDictionary::new();
    for pair in pairs.iter().filter_map(serde_json::Value::as_array) {
        let (Some(serde_json::Value::Array(cell)), Some(value)) = (pair.first(), pair.get(1))
        else {
            continue;
        };
        let typed = if cell.len() == 3 { VariantType::VECTOR3I } else { VariantType::VECTOR2I };
        out.set(&vector(cell, typed), &json_variant(value));
    }
    out.to_variant()
}

/// The value of `made`'s enum `key` named `name` (case aside), from its export's hint:
/// "Origin:0,Resource:1" for a named enum, "north,west,floor" for an export_enum.
fn enum_value(made: &Gd<Object>, key: &str, name: &str) -> i64 {
    let hint = property_named(made, key)
        .and_then(|p| p.get("hint_string"))
        .map(|h| h.to_string())
        .unwrap_or_default();
    let wanted = name.to_lowercase().replace('_', " ");
    for (index, entry) in hint.split(',').enumerate() {
        let (label, at) = entry
            .split_once(':')
            .map_or((entry, index as i64), |(label, at)| (label, at.parse().unwrap_or(0)));
        if label.to_lowercase().replace('_', " ") == wanted {
            return at;
        }
    }
    godot_error!("ContentStore: {name} isn't one of {key}'s values");
    0
}

/// The name of `made`'s enum value `value` in the schema: a named enum's in capitals
/// (Godot writes ORE_VEIN as "Ore Vein"), an export_enum's as written.
fn enum_name(property: &VarDictionary, value: i64) -> String {
    let hint = property.get("hint_string").map(|h| h.to_string()).unwrap_or_default();
    for (index, entry) in hint.split(',').enumerate() {
        match entry.split_once(':') {
            Some((label, at)) if at.parse::<i64>().ok() == Some(value) => {
                return label.to_uppercase().replace(' ', "_");
            }
            None if index as i64 == value => return entry.to_string(),
            _ => {}
        }
    }
    value.to_string()
}

/// A MapDef's fields in the schema: every node once (those on lanes first) under "nodes",
/// and each lane naming its nodes by id.
fn map_fields(map: &Gd<Object>) -> serde_json::Value {
    use serde_json::Value;
    let lanes: Vec<Gd<Object>> = objects(&map.get("lanes"));
    let mut nodes: Vec<Gd<Object>> = Vec::new();
    let on_lanes = lanes.iter().flat_map(|lane| objects(&lane.get("nodes")));
    for node in on_lanes.chain(objects(&map.get("off_lane_nodes"))) {
        if !nodes.iter().any(|known| known.instance_id() == node.instance_id()) {
            nodes.push(node);
        }
    }
    let mut fields = object_fields(map);
    fields.remove("off_lane_nodes");
    fields.insert(
        "nodes".into(),
        Value::Array(nodes.iter().map(|n| Value::Object(object_fields(n))).collect()),
    );
    let lanes = lanes.iter().map(|lane| {
        let ids = objects(&lane.get("nodes"))
            .iter()
            .map(|n| Value::String(n.get("id").to_string()))
            .collect();
        let mut lane_fields = object_fields(lane);
        lane_fields.insert("nodes".into(), Value::Array(ids));
        Value::Object(lane_fields)
    });
    fields.insert("lanes".into(), Value::Array(lanes.collect()));
    Value::Object(fields)
}

fn objects(list: &Variant) -> Vec<Gd<Object>> {
    items(list).iter().filter_map(|item| item.try_to::<Gd<Object>>().ok()).collect()
}

/// The elements of a list, typed (`Array[NodeDef]`) or not: Rust can't hold a typed GDScript
/// array as an untyped one, so they are read one by one.
fn items(list: &Variant) -> Vec<Variant> {
    let size = list.call("size", &[]).try_to::<i64>().unwrap_or(0);
    (0..size).map(|index| list.call("get", &[index.to_variant()])).collect()
}

/// A definition's saved fields by name, in the schema.
fn object_fields(made: &Gd<Object>) -> serde_json::Map<String, serde_json::Value> {
    saved_properties(made)
        .iter()
        .map(|property| {
            let name = property.get("name").map(|name| name.to_string()).unwrap_or_default();
            let value = schema_value(&made.get(name.as_str()), Some(property));
            (name, value)
        })
        .collect()
}

/// A field's value in the schema: a definition its fields, a vector its numbers, a cell-keyed
/// dictionary its [cell, value] pairs, an enum its name.
fn schema_value(value: &Variant, property: Option<&VarDictionary>) -> serde_json::Value {
    use serde_json::Value;
    const HINT_ENUM: i64 = 2;
    match value.get_type() {
        VariantType::OBJECT => match value.try_to::<Gd<Object>>() {
            Ok(made) => Value::Object(object_fields(&made)),
            Err(_) => Value::Null,
        },
        VariantType::ARRAY => {
            Value::Array(items(value).iter().map(|item| schema_value(item, None)).collect())
        }
        VariantType::DICTIONARY => Value::Array(
            value
                .to::<VarDictionary>()
                .iter_shared()
                .map(|(key, item)| {
                    Value::Array(vec![schema_value(&key, None), schema_value(&item, None)])
                })
                .collect(),
        ),
        VariantType::VECTOR2 => {
            let at = value.to::<Vector2>();
            serde_json::json!([at.x as f64, at.y as f64])
        }
        VariantType::VECTOR2I => {
            let at = value.to::<Vector2i>();
            serde_json::json!([at.x, at.y])
        }
        VariantType::VECTOR3I => {
            let at = value.to::<Vector3i>();
            serde_json::json!([at.x, at.y, at.z])
        }
        VariantType::INT => {
            match property.filter(|p| p.get("hint").is_some_and(|h| h.to::<i64>() == HINT_ENUM)) {
                Some(property) => Value::String(enum_name(property, value.to::<i64>())),
                None => Value::from(value.to::<i64>()),
            }
        }
        _ => json_of(value),
    }
}

/// Godot data as JSON: a whole number stays one, a real stays one.
fn json_of(value: &Variant) -> serde_json::Value {
    use serde_json::Value;
    match value.get_type() {
        VariantType::BOOL => Value::Bool(value.to::<bool>()),
        VariantType::INT => Value::from(value.to::<i64>()),
        VariantType::FLOAT => Value::from(value.to::<f64>()),
        VariantType::STRING | VariantType::STRING_NAME => Value::String(value.to_string()),
        VariantType::ARRAY => Value::Array(items(value).iter().map(json_of).collect()),
        VariantType::PACKED_STRING_ARRAY => Value::Array(
            value
                .to::<PackedStringArray>()
                .as_slice()
                .iter()
                .map(|text| Value::String(text.to_string()))
                .collect(),
        ),
        VariantType::DICTIONARY => Value::Object(
            value
                .to::<VarDictionary>()
                .iter_shared()
                .map(|(key, item)| (key.to_string(), json_of(&item)))
                .collect(),
        ),
        _ => Value::Null,
    }
}

/// `value` as Godot data, through its JSON: ints stay ints, reals reals.
/// The ground a battle is on (spec 39 step 3a; `ground`): FormationTerrain is a view over
/// it. A walker is passed as its fields (TerrainWalker: height, swimmer, climber, swims,
/// climbs). It keeps each kind of walker's speed shares for paths (`paths::shares`, step
/// 3c), dropped when it is painted.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct TerrainGround {
    ground: Option<ground::Ground>,
    shares: paths::shares::Cache,
}

#[godot_api]
impl TerrainGround {
    /// Open, level ground of `size` cells, with the tuning's [ground_cliff_quarters,
    /// ground_slope_cost, ground_wading, ground_slow_wading, ground_swim_pace,
    /// ground_climb_pace, ground_climb_demand].
    #[func]
    fn setup(&mut self, size: Vector2i, tuning: PackedFloat64Array) {
        let t = tuning.as_slice();
        let tuning = ground::GroundTuning {
            cliff_quarters: t[0] as i64,
            slope_cost: t[1],
            wading: t[2],
            slow_wading: t[3],
            swim_pace: t[4],
            climb_pace: t[5],
            climb_demand: t[6] as i64,
        };
        self.ground = Some(ground::Ground::new(size.x, size.y, tuning));
    }

    /// Sets the cells of `area`: any of "cost", "height" (quarters), "depth" (cells),
    /// "flows", "climb", "obscurance" and "blocks_sight", as GDScript's packed arrays took
    /// them (a float into an int column truncated).
    #[func]
    fn paint(&mut self, area: Rect2i, props: VarDictionary) {
        let real = |key: &str| props.get(key).map(|value| number(&value));
        let int = |key: &str| real(key).map(|value| value as i64 as i32);
        let paint = ground::Paint {
            cost: real("cost"),
            height: int("height"),
            depth: real("depth"),
            climb: int("climb"),
            flows: int("flows"),
            obscurance: real("obscurance"),
            blocks_sight: props.get("blocks_sight").map(|value| value.booleanize()),
        };
        let (at, size) = (area.position, area.size);
        self.shares.clear();
        self.ground_mut().paint((at.x, at.y, size.x, size.y), &paint);
    }

    #[func]
    fn fastest(&self) -> f64 {
        self.ground().fastest()
    }

    #[func]
    fn height_at(&self, at: Vector2) -> i64 {
        self.ground().height_at(v2(at)) as i64
    }

    #[func]
    fn blocks_sight(&self, at: Vector2) -> bool {
        self.ground().blocks_sight(v2(at))
    }

    #[func]
    fn high_ground(&self, from: Vector2, to: Vector2) -> bool {
        self.ground().high_ground(v2(from), v2(to))
    }

    #[func]
    #[allow(clippy::too_many_arguments)]
    fn crossing(
        &self,
        height: f64,
        swimmer: i64,
        climber: i64,
        swims: bool,
        climbs: bool,
        from: Vector2,
        to: Vector2,
    ) -> f64 {
        let walker = ground::Walker { height, swimmer, climber, swims, climbs };
        self.ground().crossing(&walker, v2(from), v2(to))
    }

    /// TerrainWalker.Mode.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn mode(
        &self,
        height: f64,
        swimmer: i64,
        climber: i64,
        swims: bool,
        climbs: bool,
        from: Vector2,
        to: Vector2,
    ) -> i64 {
        let walker = ground::Walker { height, swimmer, climber, swims, climbs };
        self.ground().mode(&walker, v2(from), v2(to)) as i64
    }

    #[func]
    fn obscurance_at(&self, point: Vector2) -> f64 {
        self.ground().obscurance_at(v2(point))
    }

    #[func]
    fn sight_along(&self, from: Vector2, to: Vector2, limit: f64) -> f64 {
        self.ground().sight_along(v2(from), v2(to), limit)
    }

    #[func]
    fn sight_clear(&self, from: Vector2, to: Vector2, budget: f64) -> bool {
        self.ground().sight_clear(v2(from), v2(to), budget)
    }

    #[func]
    fn fog(&self) -> f64 {
        self.ground().fog()
    }

    #[func]
    fn set_fog(&mut self, fog: f64) {
        self.ground_mut().set_fog(fog);
    }
}

impl TerrainGround {
    fn ground(&self) -> &ground::Ground {
        self.ground.as_ref().expect("TerrainGround.setup first")
    }

    fn ground_mut(&mut self) -> &mut ground::Ground {
        self.ground.as_mut().expect("TerrainGround.setup first")
    }
}

/// Routes, motion, the one mover and steering (spec 39 step 3b; `route`, `motion`,
/// `mover`, `steer`): FormationRoute, RoutePatch, UnitMotion, UnitMover, TerrainStep and
/// UnitSteer are views over it. Stateless: one is shared. A unit's motion is passed as
/// [bearing, turn rate, backward pace], a walker as [height, swimmer, climber, swims,
/// climbs], a point and a bearing as [x, y, bearing], and the ground as its
/// `TerrainGround` (null: open).
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct UnitMoves {}

#[godot_api]
impl UnitMoves {
    #[func]
    fn vector(&self, bearing: f64) -> Vector2 {
        vector2(maths::bearing_vector(bearing))
    }

    #[func]
    fn bearing_to(&self, from: Vector2, to: Vector2, current: f64) -> f64 {
        motion::bearing_to(v2(from), v2(to), current)
    }

    /// [the bearing turned to, 1 if it then faces `wanted`].
    #[func]
    fn turn(&self, unit: PackedFloat64Array, wanted: f64, seconds: f64) -> PackedFloat64Array {
        let (bearing, faces) = motion::turning_to(&mover_of(&unit), wanted, seconds);
        PackedFloat64Array::from(&[bearing, faces as i64 as f64][..])
    }

    #[func]
    fn pace(&self, unit: PackedFloat64Array, heading: Vector2, bearing: f64) -> f64 {
        motion::pace(&mover_of(&unit), v2(heading), bearing)
    }

    #[func]
    fn advance(&self, unit: PackedFloat64Array, at: Vector2, to: Vector2, full: f64) -> Vector2 {
        vector2(motion::advance(&mover_of(&unit), v2(at), v2(to), full))
    }

    /// UnitMotion.walk, looking at `look` if `looks`.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn walk(
        &self,
        unit: PackedFloat64Array,
        at: Vector2,
        to: Vector2,
        full: f64,
        seconds: f64,
        look: Vector2,
        looks: bool,
    ) -> PackedFloat64Array {
        let look = looks.then(|| v2(look));
        let walked = motion::walk(&mover_of(&unit), v2(at), v2(to), full, seconds, look);
        placed(walked)
    }

    /// UnitMover.step, looking at `look` if `looks`.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn step(
        &self,
        ground: Option<Gd<TerrainGround>>,
        walker: PackedFloat64Array,
        unit: PackedFloat64Array,
        at: Vector2,
        to: Vector2,
        full: f64,
        seconds: f64,
        look: Vector2,
        looks: bool,
    ) -> PackedFloat64Array {
        let look = looks.then(|| v2(look));
        let (walker, unit) = (walker_of(&walker), mover_of(&unit));
        on_ground(ground, |g| {
            placed(mover::step(g, &walker, &unit, v2(at), v2(to), full, seconds, look))
        })
    }

    #[func]
    #[allow(clippy::too_many_arguments)]
    fn slide(
        &self,
        ground: Option<Gd<TerrainGround>>,
        walker: PackedFloat64Array,
        unit: PackedFloat64Array,
        at: Vector2,
        to: Vector2,
        full: f64,
    ) -> Vector2 {
        let (walker, unit) = (walker_of(&walker), mover_of(&unit));
        on_ground(ground, |g| vector2(mover::slide(g, &walker, &unit, v2(at), v2(to), full)))
    }

    #[func]
    fn pay(
        &self,
        ground: Option<Gd<TerrainGround>>,
        walker: PackedFloat64Array,
        at: Vector2,
        to: Vector2,
    ) -> Vector2 {
        let walker = walker_of(&walker);
        on_ground(ground, |g| vector2(mover::pay(g, &walker, v2(at), v2(to))))
    }

    #[func]
    fn share(
        &self,
        ground: Option<Gd<TerrainGround>>,
        walker: PackedFloat64Array,
        at: Vector2,
        stepped: Vector2,
    ) -> f64 {
        let walker = walker_of(&walker);
        on_ground(ground, |g| mover::share(g, &walker, v2(at), v2(stepped)))
    }

    /// TerrainStep.way: [x, y, the share of its pace].
    #[func]
    fn way(
        &self,
        ground: Option<Gd<TerrainGround>>,
        walker: PackedFloat64Array,
        at: Vector2,
        goal: Vector2,
        reach: f64,
    ) -> PackedFloat64Array {
        let walker = walker_of(&walker);
        on_ground(ground, |g| placed(mover::way(g, &walker, v2(at), v2(goal), reach)))
    }

    #[func]
    fn route_length(&self, points: PackedVector2Array) -> f64 {
        route_of(&points).length()
    }

    #[func]
    fn point_at(&self, points: PackedVector2Array, distance: f64) -> Vector2 {
        vector2(route_of(&points).point_at(distance))
    }

    #[func]
    fn heading_at(&self, points: PackedVector2Array, distance: f64) -> Vector2 {
        vector2(route_of(&points).heading_at(distance))
    }

    #[func]
    fn distance_of(&self, points: PackedVector2Array, point: Vector2) -> f64 {
        route_of(&points).distance_of(v2(point))
    }

    /// RoutePatch.of_detour: [where the detour leaves the route, where it rejoins] (its
    /// waypoints are the detour's between).
    #[func]
    fn of_detour(
        &self,
        points: PackedVector2Array,
        detour: PackedVector2Array,
    ) -> PackedFloat64Array {
        let patch = route::Patch::of_detour(&route_of(&points), &v2s(&detour));
        PackedFloat64Array::from(&[patch.from_along, patch.to_along][..])
    }

    /// RoutePatch.patched: the route's points with each patch's stretch replaced by its
    /// detour; patch `i` leaves at `froms[i]`, rejoins at `tos[i]` and has the next
    /// `counts[i]` of `waypoints`.
    #[func]
    fn patched(
        &self,
        points: PackedVector2Array,
        froms: PackedFloat64Array,
        tos: PackedFloat64Array,
        counts: PackedInt32Array,
        waypoints: PackedVector2Array,
    ) -> PackedVector2Array {
        let all = v2s(&waypoints);
        let mut first = 0usize;
        let mut patches = Vec::with_capacity(counts.len());
        for (i, &count) in counts.as_slice().iter().enumerate() {
            let next = first + count as usize;
            patches.push(route::Patch {
                from_along: froms[i],
                to_along: tos[i],
                waypoints: all[first..next].to_vec(),
            });
            first = next;
        }
        let out = route::Patch::patched(&route_of(&points), &patches);
        packed(&out.points)
    }

    /// UnitSteer.toward over the bodies near the way: body `i` stands at `points[i]`, of
    /// `radii[i]`, unit `ids[i]` of squad `squads[i]`; the walker is unit `id` of `squad`,
    /// of radius `own`.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn steer(
        &self,
        at: Vector2,
        goal: Vector2,
        own: f64,
        squad: i64,
        id: i64,
        tuning: PackedFloat64Array,
        seed: i64,
        bodies: PackedVector2Array,
        radii: PackedFloat64Array,
        squads: PackedInt64Array,
        ids: PackedInt64Array,
    ) -> Vector2 {
        let (points, radii, squads) = (bodies.as_slice(), radii.as_slice(), squads.as_slice());
        let near: Vec<steer::Body> = (0..points.len())
            .map(|i| steer::Body { at: v2(points[i]), radius: radii[i], squad: squads[i] })
            .collect();
        let ids = ids.as_slice();
        let draw = |i: usize| steer::draw(seed, ids[i]);
        let (look, clear) = (tuning[0], tuning[1]);
        let own_draw = || steer::draw(seed, id);
        let to =
            steer::toward(v2(at), v2(goal), own, squad, &near, look, clear, seed, &own_draw, &draw);
        vector2(to)
    }
}

/// Paths over the ground (spec 39 step 3c; `paths`, `pathing`): TerrainPaths, PathSearch,
/// PathSight, SquadPathfinder, ClimbStamina's standing, ModeStamina's step cost and
/// FormationPathing's look ahead are views over it. Stateless: one is shared. The ground is
/// its `TerrainGround`, whose cache of shares a search uses. A walker for a path is passed
/// as [height, swimmer, climber, swims, climbs, stamina, speed] and the tuning a climb
/// reads after it ([ground_cliff_quarters, stamina_climb, stamina_swim]); a sight as
/// [origin x, y, facing x, y, ahead, side, behind, hides, budget] (empty: it sees
/// everything); what a pathfinder has seen as its PathMemory's bytes (null: it keeps
/// none), handed back as "known"; a group's living units as [the walker's seven, wits, id,
/// x, y, has a definition, detection range, sight_ahead, sight_side, sight_behind] each.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct GroundPaths {}

#[godot_api]
impl GroundPaths {
    /// PathSearch: to the cell `goal`, or (`routed`) onto the route `route` at or ahead of
    /// `progress`: {"cells", "reaches", "expanded", "known"}.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn search(
        &self,
        ground: Gd<TerrainGround>,
        walker: PackedFloat64Array,
        start: Vector2i,
        goal: Vector2i,
        route: PackedVector2Array,
        progress: f64,
        routed: bool,
        sight: PackedFloat64Array,
        known: Variant,
    ) -> VarDictionary {
        let (walker, tuning) = path_walker_of(&walker);
        let (sight, mut memory) = (sight_of(&sight), memory_of(&known));
        let route = route_of(&route);
        let mut out = VarDictionary::new();
        on_paths(ground, tuning, |over| {
            let eyes = paths::Eyes { sight: sight.as_ref(), memory: memory.as_mut() };
            let mut search = paths::search(over, &walker, (start.x, start.y), eyes);
            let cells = if routed {
                search.onto_route(&route, progress)
            } else {
                search.toward_cell((goal.x, goal.y))
            };
            out.set("cells", &cells_of(&cells));
            out.set("reaches", search.reaches);
            out.set("expanded", search.expanded);
        });
        out.set("known", &known_of(memory));
        out
    }

    /// TerrainPaths.plan: {"waypoints", "reaches", "cells", "known"}.
    #[func]
    fn plan(
        &self,
        ground: Gd<TerrainGround>,
        walker: PackedFloat64Array,
        from: Vector2,
        goal: Vector2,
        sight: PackedFloat64Array,
        known: Variant,
    ) -> VarDictionary {
        let (walker, tuning) = path_walker_of(&walker);
        let (sight, mut memory) = (sight_of(&sight), memory_of(&known));
        let plan = on_paths(ground, tuning, |over| {
            let eyes = paths::Eyes { sight: sight.as_ref(), memory: memory.as_mut() };
            paths::plan(over, &walker, v2(from), v2(goal), eyes)
        });
        planned_of(&plan, memory)
    }

    /// TerrainPaths.rejoin: {"waypoints", "reaches", "cells", "along", "known"}.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn rejoin(
        &self,
        ground: Gd<TerrainGround>,
        walker: PackedFloat64Array,
        from: Vector2,
        route: PackedVector2Array,
        progress: f64,
        sight: PackedFloat64Array,
        known: Variant,
    ) -> VarDictionary {
        let (walker, tuning) = path_walker_of(&walker);
        let (sight, mut memory) = (sight_of(&sight), memory_of(&known));
        let route = route_of(&route);
        let plan = on_paths(ground, tuning, |over| {
            let eyes = paths::Eyes { sight: sight.as_ref(), memory: memory.as_mut() };
            paths::rejoin(over, &walker, v2(from), &route, progress, eyes)
        });
        let mut out = planned_of(&plan, memory);
        out.set("along", plan.along);
        out
    }

    /// TerrainPaths.cost, for a walker as [height, swimmer, climber, swims, climbs].
    #[func]
    fn cost(
        &self,
        ground: Gd<TerrainGround>,
        walker: PackedFloat64Array,
        waypoints: PackedVector2Array,
    ) -> f64 {
        paths::cost(ground.bind().ground(), &walker_of(&walker), &v2s(&waypoints))
    }

    #[func]
    fn reach(&self, sight: PackedFloat64Array, way: Vector2) -> f64 {
        sight_of(&sight).map_or(0.0, |sight| sight.reach(v2(way)))
    }

    /// PathSight.sees, over `ground` (null: nothing hides anything).
    #[func]
    fn sees(
        &self,
        ground: Option<Gd<TerrainGround>>,
        sight: PackedFloat64Array,
        point: Vector2,
    ) -> bool {
        let sight = sight_of(&sight).expect("a sight");
        match ground {
            Some(g) => sight.sees(Some(g.bind().ground()), v2(point)),
            None => sight.sees(None, v2(point)),
        }
    }

    /// ClimbStamina.standable, for a walker as [height, swimmer, climber, swims, climbs].
    #[func]
    fn standable(
        &self,
        ground: Gd<TerrainGround>,
        walker: PackedFloat64Array,
        at: Vector2,
        cliff_quarters: i64,
    ) -> bool {
        paths::climb::standable(ground.bind().ground(), &walker_of(&walker), v2(at), cliff_quarters)
    }

    /// ModeStamina.step_cost, with [stamina_climb, stamina_swim].
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn step_cost(
        &self,
        ground: Gd<TerrainGround>,
        walker: PackedFloat64Array,
        from: Vector2,
        to: Vector2,
        speed: f64,
        rates: PackedFloat64Array,
    ) -> f64 {
        let tuning = paths::PathTuning {
            cliff_quarters: 0,
            stamina_climb: rates[0],
            stamina_swim: rates[1],
        };
        let g = ground.bind();
        paths::climb::step_cost(g.ground(), &walker_of(&walker), v2(from), v2(to), speed, &tuning)
    }

    /// FormationPathing.group_walker: the walker's seven.
    #[func]
    fn group_walker(&self, members: PackedFloat64Array) -> PackedFloat64Array {
        let w = pathing::group_walker(&members_of(&members));
        let (g, flag) = (&w.walker, |on: bool| on as i64 as f64);
        let fields = [g.height, g.swimmer as f64, g.climber as f64, flag(g.swims), flag(g.climbs)];
        let mut out = PackedFloat64Array::from(&fields[..]);
        out.extend_array(&PackedFloat64Array::from(&[w.stamina, w.speed][..]));
        out
    }

    /// SquadPathfinder.pick: the index of the group's pathfinder, -1 with none.
    #[func]
    fn pathfinder(&self, members: PackedFloat64Array, seed: i64) -> i64 {
        pathing::pathfinder(&members_of(&members), seed).map_or(-1, |index| index as i64)
    }

    /// FormationPathing's look ahead, with [path_look, path_detour_gain, sight_budget,
    /// ground_cliff_quarters, stamina_climb, stamina_swim]: the way round to keep, empty
    /// for none.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn look_ahead(
        &self,
        ground: Gd<TerrainGround>,
        route: PackedVector2Array,
        front: f64,
        direction: f64,
        members: PackedFloat64Array,
        seed: i64,
        tuning: PackedFloat64Array,
    ) -> PackedVector2Array {
        let t = tuning.as_slice();
        let climb = paths::PathTuning {
            cliff_quarters: t[3] as i64,
            stamina_climb: t[4],
            stamina_swim: t[5],
        };
        let look = pathing::LookTuning { look: t[0], detour_gain: t[1], sight_budget: t[2] };
        let (route, members) = (route_of(&route), members_of(&members));
        let way = on_paths(ground, climb, |over| {
            pathing::look_ahead(over, &route, (front, direction), &members, seed, &look)
        });
        packed(&way.unwrap_or_default())
    }
}

/// `run` over the ground and its cache of shares.
fn on_paths<R>(
    mut ground: Gd<TerrainGround>,
    tuning: paths::PathTuning,
    run: impl FnOnce(&mut paths::Over) -> R,
) -> R {
    let mut bound = ground.bind_mut();
    let TerrainGround { ground, shares } = &mut *bound;
    let ground = ground.as_ref().expect("TerrainGround.setup first");
    run(&mut paths::Over { ground, cache: shares, tuning })
}

/// [the walker's seven, ground_cliff_quarters, stamina_climb, stamina_swim] as a path's
/// walker and its tuning.
fn path_walker_of(fields: &PackedFloat64Array) -> (paths::PathWalker, paths::PathTuning) {
    let f = fields.as_slice();
    let walker = walker_of(fields);
    let tuning =
        paths::PathTuning { cliff_quarters: f[7] as i64, stamina_climb: f[8], stamina_swim: f[9] };
    (paths::PathWalker { walker, stamina: f[5], speed: f[6] }, tuning)
}

fn sight_of(fields: &PackedFloat64Array) -> Option<paths::sight::Sight> {
    let f = fields.as_slice();
    (!f.is_empty()).then(|| paths::sight::Sight {
        origin: V2::new(f[0] as f32, f[1] as f32),
        facing: V2::new(f[2] as f32, f[3] as f32),
        ahead: f[4],
        side: f[5],
        behind: f[6],
        hides: f[7] != 0.0,
        budget: f[8],
    })
}

/// A PathMemory's bytes (null: none kept).
fn memory_of(known: &Variant) -> Option<paths::memory::Memory> {
    (!known.is_nil())
        .then(|| paths::memory::Memory { known: known.to::<PackedByteArray>().to_vec() })
}

fn known_of(memory: Option<paths::memory::Memory>) -> PackedByteArray {
    PackedByteArray::from(memory.map(|memory| memory.known).unwrap_or_default().as_slice())
}

fn cells_of(cells: &[paths::Cell]) -> Array<Vector2i> {
    cells.iter().map(|&(x, y)| Vector2i::new(x, y)).collect()
}

fn planned_of(plan: &paths::Plan, memory: Option<paths::memory::Memory>) -> VarDictionary {
    let mut out = VarDictionary::new();
    out.set("waypoints", &packed(&plan.waypoints));
    out.set("reaches", plan.reaches);
    out.set("cells", &cells_of(&plan.cells));
    out.set("known", &known_of(memory));
    out
}

/// A group's living units, sixteen numbers each.
fn members_of(rows: &PackedFloat64Array) -> Vec<pathing::Member> {
    rows.as_slice()
        .chunks_exact(16)
        .map(|r| pathing::Member {
            walker: paths::PathWalker {
                walker: ground::Walker {
                    height: r[0],
                    swimmer: r[1] as i64,
                    climber: r[2] as i64,
                    swims: r[3] != 0.0,
                    climbs: r[4] != 0.0,
                },
                stamina: r[5],
                speed: r[6],
            },
            wits: r[7] as i64,
            id: r[8] as i64,
            at: V2::new(r[9] as f32, r[10] as f32),
            eyes: (r[11] != 0.0).then_some([r[12], r[13], r[14], r[15]]),
        })
        .collect()
}

/// [bearing, turn rate, backward pace] as a mover.
fn mover_of(unit: &PackedFloat64Array) -> motion::Mover {
    let u = unit.as_slice();
    motion::Mover { bearing: u[0], turn_rate: u[1], backward_pace: u[2] }
}

/// [height, swimmer, climber, swims, climbs] as a walker.
fn walker_of(walker: &PackedFloat64Array) -> ground::Walker {
    let w = walker.as_slice();
    ground::Walker {
        height: w[0],
        swimmer: w[1] as i64,
        climber: w[2] as i64,
        swims: w[3] != 0.0,
        climbs: w[4] != 0.0,
    }
}

fn route_of(points: &PackedVector2Array) -> route::Route {
    route::Route::new(v2s(points), 0.0)
}

/// A point and a number as [x, y, number]: the f32 coordinates are exact in f64.
fn placed((at, value): (V2, f64)) -> PackedFloat64Array {
    PackedFloat64Array::from(&[at.x as f64, at.y as f64, value][..])
}

/// `run` on the ground's own state, or on open ground.
fn on_ground<R>(
    ground: Option<Gd<TerrainGround>>,
    run: impl FnOnce(Option<&ground::Ground>) -> R,
) -> R {
    match ground {
        Some(g) => run(Some(g.bind().ground())),
        None => run(None),
    }
}

fn vector2(at: V2) -> Vector2 {
    Vector2::new(at.x, at.y)
}

fn v2(at: Vector2) -> V2 {
    V2::new(at.x, at.y)
}

/// A GDScript number (an int, a float or a bool) as a float.
fn number(value: &Variant) -> f64 {
    match value.get_type() {
        VariantType::INT => value.to::<i64>() as f64,
        VariantType::BOOL => value.to::<bool>() as i64 as f64,
        _ => value.to::<f64>(),
    }
}

fn variant_of(value: &impl serde::Serialize) -> Variant {
    json_variant(&serde_json::to_value(value).expect("content serialises"))
}

fn json_variant(value: &serde_json::Value) -> Variant {
    use serde_json::Value;
    match value {
        Value::Null => Variant::nil(),
        Value::Bool(flag) => flag.to_variant(),
        Value::Number(number) => match number.as_i64() {
            Some(whole) => whole.to_variant(),
            None => number.as_f64().unwrap_or(f64::NAN).to_variant(),
        },
        Value::String(text) => text.to_variant(),
        Value::Array(items) => {
            let mut array = VarArray::new();
            for item in items {
                array.push(&json_variant(item));
            }
            array.to_variant()
        }
        Value::Object(fields) => {
            let mut dictionary = VarDictionary::new();
            for (key, item) in fields {
                dictionary.set(&key.to_variant(), &json_variant(item));
            }
            dictionary.to_variant()
        }
    }
}
