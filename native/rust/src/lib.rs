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
mod content;
mod det_math;
mod face_store;
mod facing;
mod field;
mod field_parting;
mod ghash;
mod grid;
mod maths;
mod motion;
mod ordered;
mod parting;
mod scrum;
mod seeking;
mod snapshot;
mod squads;
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

/// The simulation's content as the crate loaded it (spec 39 step 2, Decision 153), handed
/// to GDScript to fill its definitions (CrateContent) until their last reader ports.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct ContentStore {}

#[godot_api]
impl ContentStore {
    /// The content under `root` (an OS path to the repo's `content/`): {"units", "weapons",
    /// "armour"} each id -> fields, "tuning" and "terrain" (the terrain library) their fields;
    /// or {"error": why it didn't load}.
    #[func]
    fn load(root: GString) -> VarDictionary {
        let mut out = VarDictionary::new();
        match content::Content::load(std::path::Path::new(&root.to_string())) {
            Ok(loaded) => out = variant_of(&loaded).to(),
            Err(error) => out.set("error", &error.to_string().to_variant()),
        }
        out
    }

    /// Each kind's fields when its file names none ({"unit", "weapon", "armour", and the
    /// terrain library's kinds}), for the test that holds them equal to GDScript's.
    #[func]
    fn defaults() -> VarDictionary {
        let mut out = VarDictionary::new();
        out.set("unit", &variant_of(&content::Unit::default()));
        out.set("weapon", &variant_of(&content::Weapon::default()));
        out.set("armour", &variant_of(&content::Armour::default()));
        out.set("terrain_library", &variant_of(&content::TerrainLibrary::default()));
        out.set("terrain", &variant_of(&content::Terrain::default()));
        out.set("liquid_body", &variant_of(&content::LiquidBody::default()));
        out.set("stratum", &variant_of(&content::Stratum::default()));
        out.set("feature", &variant_of(&content::TerrainFeature::default()));
        out.set("material", &variant_of(&content::Material::default()));
        out.set("heat_transition", &variant_of(&content::HeatTransition::default()));
        out
    }
}

/// `value` as Godot data, through its JSON: ints stay ints, reals reals.
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
