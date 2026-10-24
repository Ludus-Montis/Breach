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
mod contact;
pub mod content;
mod contest;
pub mod designer;
mod det_math;
mod det_pow;
mod engage;
mod entries;
mod face_store;
mod facing;
mod field;
mod field_parting;
mod frame;
mod geometry;
mod ghash;
mod grid;
pub mod ground;
mod groups;
mod locks;
mod lying;
mod manoeuvre;
mod march;
mod maths;
mod motion;
mod mover;
mod near;
mod ordered;
mod parting;
mod pathing;
mod paths;
mod perception;
mod pursuit;
mod ranks;
mod reach;
mod rect;
mod regroup;
mod roster;
mod route;
mod scrum;
mod seeking;
mod snapshot;
mod split;
mod squads;
mod stance;
mod steer;
mod trade;
mod walk_store;
mod walking;
mod withdraw;

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
    /// How many calls have changed the squads' records (spec 39 step 4b): SquadRecords'
    /// views are taken afresh when it moves.
    edits: i64,
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
        let ground = within.is_valid();
        let mut ask = |flat: i32, p: V2| -> bool {
            !ground || within.callv(&varray![flat as i64, Vector2::new(p.x, p.y)]).to::<bool>()
        };
        let bits = [i[3] as i32, i[4] as i32];
        let (gathered, out, active) = seeking::seek(&mut self.field, &say, &params, &mut ask, bits);
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
        let out = face_store::face(
            &mut self.field,
            squads.as_slice(),
            foe_ids.as_slice(),
            members.as_slice(),
            speeds.as_slice(),
            valued as i32,
            &params,
        );
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
        let sizes = lying_sizes.as_slice();
        let lying_radii: Vec<f64> = sizes.chunks(2).map(|c| c[0]).collect();
        let lying_areas: Vec<f64> = sizes.chunks(2).map(|c| c[1]).collect();
        let (anchors, nexts, lying) = (v2s(&squad_anchors), v2s(&nexts), v2s(&lying_ats));
        let say = walk_store::WalkSay {
            squad_floats: squad_floats.as_slice(),
            squad_anchors: &anchors,
            lying_ats: &lying,
            lying_radii: &lying_radii,
            lying_areas: &lying_areas,
        };
        let b = bits.as_slice();
        let out = walk_store::walk(
            &mut self.field,
            squads.as_slice(),
            &say,
            members.as_slice(),
            codes.as_slice(),
            &nexts,
            speeds.as_slice(),
            [b[0], b[1]],
            &params,
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
                self.edits += 1;
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

    /// FormationSight.clear: true if the sight line between two points crosses at most two
    /// cells that block sight (spec 39 step 4c, `perception`).
    #[func]
    fn screens_clear(&self, from: Vector2, to: Vector2) -> bool {
        perception::clear(self.ground(), v2(from), v2(to))
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

/// The scrum's contest and reach (spec 39 step 4a; `contest`, `reach`): ScrumContest and
/// ScrumReach are views over it. Stateless: one is shared.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct ScrumRules {}

#[godot_api]
impl ScrumRules {
    #[func]
    fn draw(&self, seed: i64, id: i64) -> i64 {
        contest::draw(seed, id)
    }

    #[func]
    fn squad_draw(&self, seed: i64, id: i64) -> i64 {
        contest::squad_draw(seed, id)
    }

    #[func]
    fn roll(&self, seed: i64, tick: i64, id: i64, die: i64) -> i64 {
        contest::roll(seed, tick, id, die)
    }

    #[func]
    fn radius(&self, width: i64, depth: i64) -> f64 {
        reach::radius(width, depth)
    }

    #[func]
    fn gap(&self, here: Vector2, radius: f64, there: Vector2, other: f64) -> f64 {
        reach::gap(v2(here), radius, v2(there), other)
    }

    /// `point` in the front of a unit at `from` on `bearing`, its front `arc` degrees.
    #[func]
    fn in_front(&self, bearing: f64, from: Vector2, point: Vector2, arc: f64) -> bool {
        reach::in_front(maths::bearing_vector(bearing), v2(from), v2(point), reach::front(arc))
    }

    #[func]
    fn area(&self, at: Vector2, width: i64, depth: i64, heading: f64) -> Rect2 {
        let (corner, size) = reach::area(v2(at), width, depth, heading);
        Rect2::new(vector2(corner), vector2(size))
    }
}

/// Routes, motion, the one mover and steering (spec 39 step 3b; `route`, `motion`,
/// `mover`): FormationRoute, RoutePatch, UnitMotion, UnitMover and TerrainStep are views
/// over it (steering's phases call `steer` in the crate, spec 39 step 4c). Stateless: one
/// is shared. A unit's motion is passed as [bearing, turn rate, backward pace], a walker
/// as [height, swimmer, climber, swims, climbs], a point and a bearing as [x, y, bearing],
/// and the ground as its `TerrainGround` (null: open).
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

/// The march (`march`, spec 39 step 3d) for FormationMarch, FormationSweep, FormationWalk,
/// FormationPour, FormationShuffle and PlaceOrder, their views. Stateless, so one is shared
/// (`CrateMarch`).
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct GroundMarch {}

#[godot_api]
impl GroundMarch {
    /// FormationMarch.toward, the squad `along` as [front, home, order, direction].
    #[func]
    fn toward(&self, along: PackedFloat64Array, delta: f64, route_end: f64) -> f64 {
        march::toward(&along_of(&along), delta, route_end)
    }

    /// FormationMarch.travel_sign.
    #[func]
    fn travel_sign(&self, along: PackedFloat64Array, route_end: f64) -> i64 {
        march::travel_sign(&along_of(&along), route_end) as i64
    }

    /// FormationMarch.pace's share, its front rank's walkers five numbers each.
    #[func]
    fn pace(
        &self,
        ground: Gd<TerrainGround>,
        walkers: PackedFloat64Array,
        points: PackedVector2Array,
        heading: f64,
    ) -> f64 {
        let (rows, _) = walkers.as_slice().as_chunks::<5>();
        let walkers: Vec<ground::Walker> = rows.iter().map(|r| walker_from(r)).collect();
        march::pace(ground.bind().ground(), &walkers, &v2s(&points), heading)
    }

    /// FormationMarch.face: the bearing of `route` at `cells` along it, travelled in
    /// `direction`.
    #[func]
    fn face(&self, route: PackedVector2Array, cells: f64, direction: i64, heading: f64) -> f64 {
        march::sweep::wanted(&route_of(&route), cells, direction as i32, heading)
    }

    /// FormationSweep.step: the heading turned towards `route`'s at `cells`.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn sweep(
        &self,
        route: PackedVector2Array,
        cells: f64,
        direction: i64,
        heading: f64,
        width: i64,
        cells_per_second: f64,
        seconds: f64,
    ) -> f64 {
        let wanted = march::sweep::wanted(&route_of(&route), cells, direction as i32, heading);
        march::sweep::step(heading, wanted, width as i32, cells_per_second, seconds)
    }

    /// FormationSweep.rate.
    #[func]
    fn sweep_rate(&self, width: i64, cells_per_second: f64) -> f64 {
        march::sweep::rate(width as i32, cells_per_second)
    }

    /// FormationWalk.walk for `units` (eleven numbers each: walker's five, most stamina,
    /// footprint area, turn rate, backward pace, radius, 0) standing as `states` (nine each:
    /// at, foothold, stamina, on face, climb left, bearing, speed) towards `places`, in a
    /// tick of `squad` = [seconds, cells a second at speed 1, heading], over the bodies
    /// `lying` (if `bodies`), with `tuning` (walk_tuning_of): each unit's nine after, then
    /// the hurt a fall did it.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn walk(
        &self,
        ground: Option<Gd<TerrainGround>>,
        bodies: bool,
        lying_ats: PackedVector2Array,
        lying_sizes: PackedFloat64Array,
        squad: PackedFloat64Array,
        units: PackedFloat64Array,
        states: PackedFloat64Array,
        places: PackedVector2Array,
        tuning: PackedFloat64Array,
    ) -> PackedFloat64Array {
        let ats = v2s(&lying_ats);
        let (sizes, _) = lying_sizes.as_slice().as_chunks::<2>();
        let radii: Vec<f64> = sizes.iter().map(|s| s[0]).collect();
        let areas: Vec<f64> = sizes.iter().map(|s| s[1]).collect();
        let lying = lying::Lying::new(&ats, &radii, &areas);
        let s = squad.as_slice();
        let (units, _) = units.as_slice().as_chunks::<11>();
        let (states, _) = states.as_slice().as_chunks::<9>();
        let places = v2s(&places);
        let tuning = walk_tuning_of(&tuning);
        let mut out = Vec::with_capacity(states.len() * 10);
        on_ground(ground, |ground| {
            let on = march::walk::Walking {
                ground,
                lying: bodies.then_some(&lying),
                seconds: s[0],
                pace: s[1],
                heading: s[2],
                tuning,
            };
            for ((unit, state), &place) in units.iter().zip(states).zip(&places) {
                let marcher = marcher_of(unit);
                let mut standing = standing_of(state);
                let hurt = march::walk::walk(&on, &marcher, &mut standing, place);
                let f = &standing.footing;
                out.extend_from_slice(&[
                    f.at.x as f64,
                    f.at.y as f64,
                    f.foothold.x as f64,
                    f.foothold.y as f64,
                    f.stamina,
                    f.on_face as i64 as f64,
                    f.climb_left,
                    standing.bearing,
                    standing.speed,
                    hurt as f64,
                ]);
            }
        });
        PackedFloat64Array::from(out.as_slice())
    }

    /// FormationPour.place: where `unit` (rank, column, footprint width and depth, swap
    /// offset ranks and columns) belongs in `frame` (anchor, heading, width, centre shift).
    #[func]
    fn place(&self, frame: PackedFloat64Array, unit: PackedFloat64Array) -> Vector2 {
        let u = unit.as_slice();
        let pourer = march::pour::Pourer {
            id: 0,
            at: V2::ZERO,
            rank: u[0] as i32,
            column: u[1] as i32,
            footprint_width: u[2] as i32,
            footprint_depth: u[3] as i32,
            offset: V2::new(u[4] as f32, u[5] as f32),
            walker: ground::Walker {
                height: 0.0,
                swimmer: 0,
                climber: 0,
                swims: false,
                climbs: false,
            },
            taking: None,
        };
        vector2(march::pour::place(&frame_of(&frame), &pourer))
    }

    /// FormationPour.targets for `units` (pourer_of's seventeen numbers each), in order.
    #[func]
    fn targets(
        &self,
        ground: Option<Gd<TerrainGround>>,
        frame: PackedFloat64Array,
        units: PackedFloat64Array,
        seed: i64,
        wounds_reach: f64,
    ) -> PackedVector2Array {
        let units = pourers_of(&units);
        let frame = frame_of(&frame);
        let aims = on_ground(ground, |ground| {
            march::pour::targets(ground, &frame, &units, seed, wounds_reach)
        });
        packed(&aims)
    }

    /// FormationWalk.share for the squad's units in its frame (pourer_of's), by its
    /// `rule` (FormationWalk.rule_of's code), slack and walk_lost.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn share(
        &self,
        ground: Option<Gd<TerrainGround>>,
        frame: PackedFloat64Array,
        units: PackedFloat64Array,
        seed: i64,
        wounds_reach: f64,
        rule: i64,
        slack: f64,
        lost: f64,
    ) -> f64 {
        let rule = rule_from(rule);
        if rule == march::walk::Rule::LeftBehind {
            return 1.0;
        }
        let units = pourers_of(&units);
        let frame = frame_of(&frame);
        let aims = on_ground(ground, |ground| {
            march::pour::targets(ground, &frame, &units, seed, wounds_reach)
        });
        let lags: Vec<march::walk::Lag> = units
            .iter()
            .zip(&aims)
            .map(|(unit, &target)| march::walk::Lag {
                target,
                place: march::pour::place(&frame, unit),
                at: unit.at,
            })
            .collect();
        march::walk::share(rule, slack, lost, &lags)
    }

    /// FormationWalk.rule_of over the living units' [leads, left behind, no man left]: 0
    /// none, 1 fall behind left behind, 2 no man left behind.
    #[func]
    fn rule_of(&self, living: PackedInt32Array) -> i64 {
        let (rows, _) = living.as_slice().as_chunks::<3>();
        let traits: Vec<march::walk::Traits> = rows
            .iter()
            .map(|r| march::walk::Traits {
                leads: r[0] != 0,
                left_behind: r[1] != 0,
                no_man_left: r[2] != 0,
            })
            .collect();
        match march::walk::rule_of(&traits) {
            march::walk::Rule::Lagging => 0,
            march::walk::Rule::LeftBehind => 1,
            march::walk::Rule::NoManLeft => 2,
        }
    }

    /// FormationWalk.slack_of.
    #[func]
    fn slack_of(&self, discipline: f64, loose: f64, drilled: f64) -> f64 {
        march::walk::slack_of(discipline, loose, drilled)
    }

    /// PlaceOrder.front_first: the indices of the units at `ranks` and `files`, front first.
    #[func]
    fn front_first(&self, ranks: PackedInt32Array, files: PackedInt32Array) -> PackedInt32Array {
        let places: Vec<(i32, i32)> =
            ranks.as_slice().iter().copied().zip(files.as_slice().iter().copied()).collect();
        march::shuffle::front_first(&places).iter().map(|&index| index as i32).collect()
    }

    /// FormationShuffle._start_moves: the moves `squad` [width, combat width, centre shift,
    /// fighting] can start, its living units seven numbers each (rank, column, footprint
    /// width and depth, band, priority, joined) at `speeds`, the moves under way claiming
    /// `swaps` (rank, column, footprint width and depth each) and their living units
    /// `busy`, at `pace` = [tick seconds, cells a second at speed 1, scrum_crowding]. Each
    /// move: its ticks, distance, its units' count, then each unit's index and place
    /// (rank, column), the mover first; then its passed units' count and indices.
    #[func]
    fn start_moves(
        &self,
        squad: PackedFloat64Array,
        units: PackedInt32Array,
        speeds: PackedFloat64Array,
        swaps: PackedInt32Array,
        busy: PackedInt32Array,
        pace: PackedFloat64Array,
    ) -> PackedInt64Array {
        let s = squad.as_slice();
        let squad = march::shuffle::Squad {
            width: s[0] as i32,
            combat_width: s[1] as i32,
            centre_shift: s[2],
            fighting: s[3] != 0.0,
        };
        let (rows, _) = units.as_slice().as_chunks::<7>();
        let ranked: Vec<march::shuffle::Ranked> = rows
            .iter()
            .zip(speeds.as_slice())
            .map(|(r, &speed)| march::shuffle::Ranked {
                place: (r[0], r[1]),
                footprint_width: r[2],
                footprint_depth: r[3],
                preferred_position: r[4],
                position_priority: r[5],
                speed,
                joined: r[6] != 0,
            })
            .collect();
        let (claims, _) = swaps.as_slice().as_chunks::<4>();
        let claims: Vec<((i32, i32), i32, i32)> =
            claims.iter().map(|c| ((c[0], c[1]), c[2], c[3])).collect();
        let busy: Vec<usize> = busy.as_slice().iter().map(|&index| index as usize).collect();
        let p = pace.as_slice();
        let started =
            march::shuffle::start_moves(&squad, &ranked, &claims, &busy, [p[0], p[1], p[2]]);
        let mut out: Vec<i64> = Vec::new();
        for move_ in started {
            let plan = &move_.plan;
            out.extend_from_slice(&[move_.ticks, plan.distance as i64, plan.to.len() as i64]);
            for &(unit, (rank, column)) in &plan.to {
                out.extend_from_slice(&[unit as i64, rank as i64, column as i64]);
            }
            out.push(plan.passed.len() as i64);
            out.extend(plan.passed.iter().map(|&unit| unit as i64));
        }
        PackedInt64Array::from(out.as_slice())
    }
}

/// [front, home, order, direction] as where a squad stands along its route.
fn along_of(along: &PackedFloat64Array) -> march::Along {
    let a = along.as_slice();
    march::Along { front: a[0], home: a[1], order: a[2] as i32, direction: a[3] as i32 }
}

/// [height, swimmer, climber, swims, climbs] as a walker.
fn walker_from(w: &[f64]) -> ground::Walker {
    ground::Walker {
        height: w[0],
        swimmer: w[1] as i64,
        climber: w[2] as i64,
        swims: w[3] != 0.0,
        climbs: w[4] != 0.0,
    }
}

/// [walk_face_travel, bodies_ground_drag, bodies_ground_block, stamina_climb, stamina_swim,
/// stamina_spent, ground_cliff_quarters, fall_damage_per_cell, fall_mass_power].
fn walk_tuning_of(tuning: &PackedFloat64Array) -> march::walk::WalkTuning {
    let t = tuning.as_slice();
    march::walk::WalkTuning {
        face_travel: t[0],
        drag: t[1],
        block: t[2],
        modes: march::modes::ModeTuning {
            climb_rate: t[3],
            swim_rate: t[4],
            spent: t[5],
            cliff_quarters: t[6] as i64,
            fall_per_cell: t[7],
            fall_mass_power: t[8],
        },
    }
}

fn marcher_of(u: &[f64; 11]) -> march::walk::Marcher {
    march::walk::Marcher {
        climber: march::modes::Climber {
            walker: walker_from(&u[..5]),
            most_stamina: u[5],
            area: u[6],
        },
        turn_rate: u[7],
        backward_pace: u[8],
        radius: u[9],
    }
}

fn standing_of(s: &[f64; 9]) -> march::walk::Standing {
    march::walk::Standing {
        footing: march::modes::Footing {
            at: V2::new(s[0] as f32, s[1] as f32),
            foothold: V2::new(s[2] as f32, s[3] as f32),
            stamina: s[4],
            on_face: s[5] != 0.0,
            climb_left: s[6],
        },
        bearing: s[7],
        speed: s[8],
    }
}

/// [anchor x, y, heading, width, centre shift] as a squad's frame.
fn frame_of(frame: &PackedFloat64Array) -> march::pour::Frame {
    let f = frame.as_slice();
    march::pour::Frame {
        anchor: V2::new(f[0] as f32, f[1] as f32),
        heading: f[2],
        width: f[3] as i32,
        centre_shift: f[4],
    }
}

/// Seventeen numbers each: id, at, rank, column, footprint width and depth, swap offset,
/// the walker's five, and 1 and the body it is out taking, or three 0s.
fn pourers_of(rows: &PackedFloat64Array) -> Vec<march::pour::Pourer> {
    let (rows, _) = rows.as_slice().as_chunks::<17>();
    rows.iter()
        .map(|r| march::pour::Pourer {
            id: r[0] as i64,
            at: V2::new(r[1] as f32, r[2] as f32),
            rank: r[3] as i32,
            column: r[4] as i32,
            footprint_width: r[5] as i32,
            footprint_depth: r[6] as i32,
            offset: V2::new(r[7] as f32, r[8] as f32),
            walker: walker_from(&r[9..14]),
            taking: (r[14] != 0.0).then(|| V2::new(r[15] as f32, r[16] as f32)),
        })
        .collect()
}

fn rule_from(code: i64) -> march::walk::Rule {
    match code {
        1 => march::walk::Rule::LeftBehind,
        2 => march::walk::Rule::NoManLeft,
        _ => march::walk::Rule::Lagging,
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
    let (rows, _) = rows.as_slice().as_chunks::<16>();
    rows.iter()
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

/// The squads' records and the squad-level scrum (spec 39 step 4b): SquadRecords' views of
/// SkirmishSquad's loose, fleeing, stance, flank contacts and chasers, and the phases that
/// read and write them - the entries (SquadEntries, EntryKeys), the locks (FormationLocks),
/// engaging (ScrumEngage), the stance (ScrumStance), the manoeuvre (FormationManoeuvre), the
/// stall (FormationScrum) and closing ranks (SquadRanks); the trade is regroup's (step 4c).
/// Every call that may change a record moves `edits`.
#[godot_api(secondary)]
impl BodyField {
    /// How many calls have changed the records: a view taken at this count still holds.
    #[func]
    fn records_edits(&self) -> i64 {
        self.edits
    }

    /// The squad's loose units' ids, in the order each went loose.
    #[func]
    fn squad_loose(&self, squad: i64) -> PackedInt64Array {
        PackedInt64Array::from(self.squads.loose[squad as usize].as_slice())
    }

    /// The squad's fleeing units' ids, in the order each began to flee.
    #[func]
    fn squad_fleeing(&self, squad: i64) -> PackedInt64Array {
        PackedInt64Array::from(self.squads.fleeing[squad as usize].as_slice())
    }

    /// Sets who of the squad has a loose and a fleeing entry, as they stand in another
    /// store (a squad carried there, SquadRecords.carry): the entries' columns are the units'.
    #[func]
    fn put_squad_entries(
        &mut self,
        squad: i64,
        loose: PackedInt64Array,
        fleeing: PackedInt64Array,
    ) {
        self.squads.loose[squad as usize] = loose.as_slice().to_vec();
        self.squads.fleeing[squad as usize] = fleeing.as_slice().to_vec();
        self.edits += 1;
    }

    /// The squad's stance as [anchor x, y, heading, foe], or empty for none.
    #[func]
    fn squad_stance(&self, squad: i64) -> PackedFloat64Array {
        match self.squads.stance[squad as usize] {
            Some(s) => PackedFloat64Array::from(&[
                s.anchor.x as f64,
                s.anchor.y as f64,
                s.heading,
                s.foe as f64,
            ]),
            None => PackedFloat64Array::new(),
        }
    }

    /// Sets the squad's stance from [anchor x, y, heading, foe], or none from empty.
    #[func]
    fn put_squad_stance(&mut self, squad: i64, stance: PackedFloat64Array) {
        let s = stance.as_slice();
        self.squads.stance[squad as usize] = (s.len() == 4).then(|| squads::Stance {
            anchor: V2::new(s[0] as f32, s[1] as f32),
            heading: s[2],
            foe: s[3] as i64,
        });
        self.edits += 1;
    }

    /// The squad's flank contacts as [edge, foe, since] each, in the order each began.
    #[func]
    fn squad_flanks(&self, squad: i64) -> PackedInt64Array {
        let flanks = &self.squads.flanks[squad as usize];
        let flat: Vec<i64> = flanks.iter().flat_map(|f| [f.edge as i64, f.foe, f.since]).collect();
        PackedInt64Array::from(flat.as_slice())
    }

    /// Sets the foe on one of the squad's edges, and the tick its contact began.
    #[func]
    fn put_squad_flank(&mut self, squad: i64, edge: i64, foe: i64, since: i64) {
        self.squads.flank(squad as usize, edge as i32, foe, since);
        self.edits += 1;
    }

    /// Takes every flank contact of the squad away.
    #[func]
    fn clear_squad_flanks(&mut self, squad: i64) {
        self.squads.flanks[squad as usize].clear();
        self.edits += 1;
    }

    /// The squad's chasers as [unit id, foe, from x, y, leash, 1 if returning else 0] each, in
    /// the order each set out.
    #[func]
    fn squad_chasers(&self, squad: i64) -> PackedFloat64Array {
        let chasers = &self.squads.chasers[squad as usize];
        let flat: Vec<f64> = chasers
            .iter()
            .flat_map(|c| {
                let back = if c.returning { 1.0 } else { 0.0 };
                [c.unit as f64, c.foe as f64, c.from.x as f64, c.from.y as f64, c.leash, back]
            })
            .collect();
        PackedFloat64Array::from(flat.as_slice())
    }

    /// Sends the unit out chasing `foe` (a squad id) from `from`, on `leash` cells.
    #[func]
    fn put_squad_chaser(&mut self, squad: i64, unit_id: i64, foe: i64, from: Vector2, leash: f64) {
        let chaser = squads::Chaser { unit: unit_id, foe, from: v2(from), leash, returning: false };
        self.squads.chase(squad as usize, chaser);
        self.edits += 1;
    }

    /// The unit out chasing gives up and makes its way back.
    #[func]
    fn return_squad_chaser(&mut self, squad: i64, unit_id: i64) {
        let chasers = &mut self.squads.chasers[squad as usize];
        if let Some(chaser) = chasers.iter_mut().find(|c| c.unit == unit_id) {
            chaser.returning = true;
        }
        self.edits += 1;
    }

    /// Calls the unit back from its chase (nothing if it isn't out).
    #[func]
    fn drop_squad_chaser(&mut self, squad: i64, unit_id: i64) {
        self.squads.chasers[squad as usize].retain(|c| c.unit != unit_id);
        self.edits += 1;
    }

    /// SquadEntries.loosen: the unit, in store slot `slot`, goes loose standing at `at`.
    #[func]
    fn loosen(&mut self, squad: i64, unit_id: i64, slot: i64, at: Vector2) {
        let columns = &mut self.field.columns;
        entries::loosen(
            &mut self.squads,
            columns,
            squad as usize,
            (unit_id, slot as usize),
            v2(at),
        );
        self.edits += 1;
    }

    /// SquadEntries.unloose (nothing if the unit has no loose entry).
    #[func]
    fn unloose(&mut self, squad: i64, unit_id: i64) {
        match self.field.slot_of(unit_id) {
            Some(slot) => {
                let columns = &mut self.field.columns;
                entries::unloose(
                    &mut self.squads,
                    columns,
                    squad as usize,
                    (unit_id, slot as usize),
                );
            }
            None => self.squads.loose[squad as usize].retain(|&id| id != unit_id),
        }
        self.edits += 1;
    }

    /// SquadEntries.unloose_all.
    #[func]
    fn unloose_all(&mut self, squad: i64) {
        let field = &mut self.field;
        let slots: Vec<Option<usize>> = self.squads.loose[squad as usize]
            .iter()
            .map(|&id| field.slot_of(id).map(|s| s as usize))
            .collect();
        let mut at = slots.into_iter();
        entries::unloose_all(&mut self.squads, &mut field.columns, squad as usize, |_| {
            at.next().flatten()
        });
        self.edits += 1;
    }

    /// SquadEntries.flee: the unit gets a fleeing entry, `along` its route, `offset` off it.
    #[func]
    fn flee(&mut self, squad: i64, unit_id: i64, slot: i64, along: f64, offset: Vector2) {
        let columns = &mut self.field.columns;
        let unit = (unit_id, slot as usize);
        entries::flee(&mut self.squads, columns, squad as usize, unit, along, v2(offset));
        self.edits += 1;
    }

    /// SquadEntries.unflee (nothing if the unit has no fleeing entry).
    #[func]
    fn unflee(&mut self, squad: i64, unit_id: i64) {
        match self.field.slot_of(unit_id) {
            Some(slot) => {
                let columns = &mut self.field.columns;
                entries::unflee(
                    &mut self.squads,
                    columns,
                    squad as usize,
                    (unit_id, slot as usize),
                );
            }
            None => self.squads.fleeing[squad as usize].retain(|&id| id != unit_id),
        }
        self.edits += 1;
    }

    /// SquadEntries.unflee_all.
    #[func]
    fn unflee_all(&mut self, squad: i64) {
        let field = &mut self.field;
        let slots: Vec<Option<usize>> = self.squads.fleeing[squad as usize]
            .iter()
            .map(|&id| field.slot_of(id).map(|s| s as usize))
            .collect();
        let mut at = slots.into_iter();
        entries::unflee_all(&mut self.squads, &mut field.columns, squad as usize, |_| {
            at.next().flatten()
        });
        self.edits += 1;
    }

    /// EntryKeys.keep: sets the key (its UnitColumns name) of the entry in `slot`; null only
    /// for `toward` and `held_by`.
    #[func]
    fn entry_keep(&mut self, slot: i64, key: StringName, value: Variant) {
        let Some(key) = entries::Key::named(&key.to_string()) else {
            return;
        };
        let value = match value.get_type() {
            VariantType::NIL => entries::Value::None,
            VariantType::INT | VariantType::BOOL => entries::Value::Int(value.to::<i64>()),
            VariantType::FLOAT => entries::Value::Real(value.to::<f64>()),
            _ => entries::Value::Point(v2(value.to::<Vector2>())),
        };
        entries::keep(&mut self.field.columns, slot as usize, key, value);
    }

    /// EntryKeys.forget: takes the keys (their UnitColumns names) out of the entry in `slot`.
    #[func]
    fn entry_forget(&mut self, slot: i64, keys: PackedStringArray) {
        let keys: Vec<entries::Key> =
            keys.as_slice().iter().filter_map(|k| entries::Key::named(&k.to_string())).collect();
        entries::forget(&mut self.field.columns, slot as usize, &keys);
    }

    /// FormationLocks.lock: the squad fights the squad `foe` (its id) from this tick; its
    /// `living` units (store slots) strike at once, and units `joined` on the march spread.
    #[func]
    fn squad_lock(&mut self, squad: i64, foe: i64, living: PackedInt32Array, joined: bool) {
        let locking = locks::Locking { squad: squad as usize, living: living.as_slice(), joined };
        locks::lock(&mut self.squads, &mut self.field.columns, &locking, foe);
        self.edits += 1;
    }

    /// FormationLocks.release: ends the squad's fight and frees every squad of `listed`
    /// (squad slots, in the lists' order) fighting it.
    #[func]
    fn squad_release(&mut self, squad: i64, listed: PackedInt32Array) {
        locks::release(&mut self.squads, squad as usize, &usizes(&listed));
        self.edits += 1;
    }

    /// FormationContact.can_engage.
    #[func]
    fn squad_can_engage(&self, squad: i64) -> bool {
        locks::can_engage(&self.squads, squad as usize)
    }

    /// ScrumSeek.foe_ids: the ids of the squads of `listed` (squad slots) the squad fights;
    /// `factions` each listed squad's faction (as a code), and its own `faction`.
    #[func]
    fn squad_foe_ids(
        &self,
        squad: i64,
        listed: PackedInt32Array,
        factions: PackedInt32Array,
        faction: i64,
    ) -> PackedInt64Array {
        let factions: Vec<u32> = factions.as_slice().iter().map(|&f| f as u32).collect();
        let foes = locks::foe_ids(
            &self.squads,
            squad as usize,
            &usizes(&listed),
            &factions,
            faction as u32,
        );
        PackedInt64Array::from(foes.as_slice())
    }

    /// FormationManoeuvre.step over `cases`, flat: per squad its slot, whether it is
    /// withdrawing and skirmishing, how many living units follow, and each one's id and store
    /// slot (only for a squad that narrowed through a gap).
    #[func]
    fn manoeuvre_step(&mut self, cases: PackedInt64Array) {
        let flat = cases.as_slice();
        let mut out = Vec::new();
        let mut at = 0;
        while at < flat.len() {
            let count = flat[at + 3] as usize;
            let living =
                (0..count).map(|n| (flat[at + 4 + 2 * n], flat[at + 5 + 2 * n] as usize)).collect();
            out.push(manoeuvre::Case {
                squad: flat[at] as usize,
                withdrawing: flat[at + 1] != 0,
                skirmishing: flat[at + 2] != 0,
                living,
            });
            at += 4 + 2 * count;
        }
        manoeuvre::step(&mut self.squads, &mut self.field.columns, &out);
        self.edits += 1;
    }

    /// FormationScrum._stall: counts each fighting squad of `listed` (squad slots, in the
    /// lists' order; `factions` theirs as codes) quiet unless it or a foe is `active` (squad
    /// ids), and releases those stalled `ticks`. Returns their slots.
    #[func]
    fn scrum_stall(
        &mut self,
        listed: PackedInt32Array,
        factions: PackedInt32Array,
        active: PackedInt64Array,
        ticks: i64,
    ) -> PackedInt32Array {
        let listed = usizes(&listed);
        let factions: Vec<u32> = factions.as_slice().iter().map(|&f| f as u32).collect();
        let foes: Vec<Vec<i64>> = (0..listed.len())
            .map(|n| locks::foe_ids(&self.squads, listed[n], &listed, &factions, factions[n]))
            .collect();
        let stall =
            manoeuvre::Stall { listed: &listed, foes: &foes, active: active.as_slice(), ticks };
        let stalled = manoeuvre::stall(&mut self.squads, &stall);
        self.edits += 1;
        PackedInt32Array::from_iter(stalled.into_iter().map(|s| s as i32))
    }

    /// ScrumEngage.step over the listed squads (`slots`, in the lists' order): each one's
    /// faction (a code), whether units joined it on the march, and how many living units
    /// follow in `units` (store slots), `ats` (where each stands) and `feet` (each one's
    /// footprint, width and depth). Returns each lock as [its index in the list, the foe's
    /// id].
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn engage(
        &mut self,
        squads: PackedInt32Array,
        factions: PackedInt32Array,
        joined: PackedByteArray,
        counts: PackedInt32Array,
        units: PackedInt32Array,
        ats: PackedVector2Array,
        feet: PackedInt32Array,
        reach_engage: f64,
        seed: i64,
    ) -> PackedInt64Array {
        let points = v2s(&ats);
        let feet = feet.as_slice();
        let mut sides = Vec::new();
        let mut from = 0;
        for (n, &count) in counts.as_slice().iter().enumerate() {
            let to = from + count as usize;
            sides.push(engage::Side {
                squad: squads.as_slice()[n] as usize,
                faction: factions.as_slice()[n] as u32,
                joined: joined.as_slice()[n] != 0,
                slots: units.as_slice()[from..to].to_vec(),
                at: points[from..to].to_vec(),
                radii: (from..to)
                    .map(|u| reach::radius(feet[2 * u] as i64, feet[2 * u + 1] as i64))
                    .collect(),
            });
            from = to;
        }
        let picks =
            engage::step(&mut self.squads, &mut self.field.columns, &sides, reach_engage, seed);
        self.edits += 1;
        let flat: Vec<i64> = picks.iter().flat_map(|&(n, foe)| [n as i64, foe]).collect();
        PackedInt64Array::from(flat.as_slice())
    }

    /// ScrumStance.anticipate_all over the listed squads (`slots`): each one's faction (a
    /// code), flags (1 it has living units, 2 it meets threats as a line), bounds (`boxes`:
    /// position and size each) and how many of the list it detects follow in `detects` (by
    /// their indices in the list). Lets go of stances in place; returns each new one to take
    /// as [index in the list, heading, foe].
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn anticipate(
        &mut self,
        squads: PackedInt32Array,
        factions: PackedInt32Array,
        flags: PackedByteArray,
        boxes: PackedVector2Array,
        counts: PackedInt32Array,
        detects: PackedInt32Array,
        scrum_anticipate: f64,
        seed: i64,
    ) -> PackedFloat64Array {
        let boxes = v2s(&boxes);
        let detects = usizes(&detects);
        let mut seen = Vec::new();
        let mut from = 0;
        for (n, &count) in counts.as_slice().iter().enumerate() {
            let to = from + count as usize;
            let flag = flags.as_slice()[n];
            seen.push(stance::Seen {
                squad: squads.as_slice()[n] as usize,
                faction: factions.as_slice()[n] as u32,
                living: flag & 1 != 0,
                bounds: rect::Rect::new(boxes[2 * n], boxes[2 * n + 1]),
                meets_threats: flag & 2 != 0,
                detects: detects[from..to].to_vec(),
            });
            from = to;
        }
        let turns = stance::anticipate(&mut self.squads, &seen, (scrum_anticipate, seed));
        self.edits += 1;
        let flat: Vec<f64> =
            turns.iter().flat_map(|t| [t.seen as f64, t.heading, t.foe as f64]).collect();
        PackedFloat64Array::from(flat.as_slice())
    }

    /// SquadRanks.close: the living units (store slots, in the squad's order), each one's
    /// preferred band and footprint (`feet`: width, depth each), laid out again `width` wide.
    #[func]
    fn close_ranks(
        &mut self,
        width: i64,
        slots: PackedInt32Array,
        preferred: PackedInt64Array,
        feet: PackedInt32Array,
    ) {
        let feet = feet.as_slice();
        let units: Vec<ranks::Ranked> = (0..slots.len())
            .map(|n| ranks::Ranked {
                slot: slots.as_slice()[n] as usize,
                preferred: preferred.as_slice()[n],
                foot: (feet[2 * n], feet[2 * n + 1]),
            })
            .collect();
        ranks::close(&mut self.field.columns, &units, width as i32);
    }
}

fn usizes(values: &PackedInt32Array) -> Vec<usize> {
    values.as_slice().iter().map(|&v| v as usize).collect()
}

/// The squads a pass is given (spec 39 step 4c, `roster`): GDScript fills one a tick from
/// its lists (who is living is UnitState's) and hands it to each phase that reads them.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct SquadRoster {
    roster: roster::Roster,
}

#[godot_api]
impl SquadRoster {
    /// Fills it, flat over the squads in the lists' order: per squad its store slot, id,
    /// faction (a code), `flags` (1 units joined it on the march, 2 it merges) and how many
    /// living units follow; per unit its id, `ints` [store slot, footprint width, depth,
    /// preferred band, ranged reach, kind (a code)] and `reals` [turn rate,
    /// backward pace, detection, most stamina, then its walker's five, or none on open
    /// ground].
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn fill(
        &mut self,
        squads: PackedInt32Array,
        ids: PackedInt64Array,
        factions: PackedInt32Array,
        flags: PackedByteArray,
        counts: PackedInt32Array,
        unit_ids: PackedInt64Array,
        ints: PackedInt32Array,
        reals: PackedFloat64Array,
    ) {
        let (ints, reals, unit_ids) = (ints.as_slice(), reals.as_slice(), unit_ids.as_slice());
        let walked = unit_ids.is_empty() || reals.len() == 9 * unit_ids.len();
        let per = if walked { 9 } else { 4 };
        let open =
            ground::Walker { height: 0.0, swimmer: 0, climber: 0, swims: false, climbs: false };
        let mut listed = Vec::with_capacity(squads.len());
        let mut from = 0;
        for (n, &count) in counts.as_slice().iter().enumerate() {
            let to = from + count as usize;
            let living = (from..to)
                .map(|u| {
                    let (i, r) = (&ints[6 * u..6 * u + 6], &reals[per * u..per * u + per]);
                    let walker = if walked {
                        ground::Walker {
                            height: r[4],
                            swimmer: r[5] as i64,
                            climber: r[6] as i64,
                            swims: r[7] != 0.0,
                            climbs: r[8] != 0.0,
                        }
                    } else {
                        open
                    };
                    roster::Member {
                        id: unit_ids[u],
                        slot: i[0] as usize,
                        foot: (i[1] as i64, i[2] as i64),
                        preferred: i[3] as i64,
                        attack_range: i[4] as i64,
                        kind: i[5] as u32,
                        turn_rate: r[0],
                        backward_pace: r[1],
                        detection: r[2],
                        max_stamina: r[3],
                        walker,
                    }
                })
                .collect();
            let flag = flags.as_slice()[n];
            listed.push(roster::Listed {
                squad: squads.as_slice()[n] as usize,
                id: ids.as_slice()[n],
                faction: factions.as_slice()[n] as u32,
                joined: flag & 1 != 0,
                merges: flag & 2 != 0,
                living,
            });
            from = to;
        }
        self.roster = roster::Roster { listed };
    }
}

#[godot_api(secondary)]
impl BodyField {
    /// ScrumRegroup.step over `roster`'s squads out of the fight, flat in the lists' order:
    /// per squad its index in the roster, flags (1 a drilled retreat, 2 withdrawing), its
    /// re-form pace and how many homes follow (`homes`: unit id, and where it walks back to,
    /// in `home_at`). `rules`: [pace, seconds, reach_contact, reach_front_arc_degrees,
    /// bodies_steer_look, bodies_steer_clear, scrum_trade_seconds].
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn regroup(
        &mut self,
        roster: Gd<SquadRoster>,
        listed: PackedInt32Array,
        flags: PackedByteArray,
        reforms: PackedFloat64Array,
        counts: PackedInt32Array,
        homes: PackedInt64Array,
        home_at: PackedVector2Array,
        ground: Option<Gd<TerrainGround>>,
        rules: PackedFloat64Array,
        seed: i64,
    ) {
        let (homes, home_at) = (homes.as_slice(), v2s(&home_at));
        let mut regroupers = Vec::with_capacity(listed.len());
        let mut from = 0;
        for n in 0..listed.len() {
            let to = from + counts.as_slice()[n] as usize;
            let flag = flags.as_slice()[n];
            regroupers.push(regroup::Regrouper {
                listed: listed.as_slice()[n] as usize,
                drilled: flag & 1 != 0,
                reform: reforms.as_slice()[n],
                withdrawing: flag & 2 != 0,
                homes: (from..to).map(|h| (homes[h], home_at[h])).collect(),
            });
            from = to;
        }
        let r = rules.as_slice();
        let rules = regroup::Rules {
            pace: r[0],
            seconds: r[1],
            seed,
            reach: r[2],
            arc: r[3],
            look: r[4],
            clear: r[5],
            trade_seconds: r[6],
        };
        let roster = roster.bind();
        let (squads, columns) = (&mut self.squads, &mut self.field.columns);
        on_ground(ground, |g| {
            regroup::step(squads, columns, &roster.roster, &regroupers, g, &rules)
        });
        self.edits += 1;
    }
}

#[godot_api(secondary)]
impl BodyField {
    /// ScrumPursuit.step over `roster`'s squads with units out chasing, flat in the lists'
    /// order: per squad its index in the roster and how many homes follow (`homes`: unit id,
    /// and where it walks back to, in `home_at`). `rules`: [cells a second at speed 1,
    /// seconds, stamina_give_up, bodies_steer_look, bodies_steer_clear].
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn pursue_chasers(
        &mut self,
        roster: Gd<SquadRoster>,
        listed: PackedInt32Array,
        counts: PackedInt32Array,
        homes: PackedInt64Array,
        home_at: PackedVector2Array,
        ground: Option<Gd<TerrainGround>>,
        rules: PackedFloat64Array,
        seed: i64,
    ) {
        let (homes, home_at) = (homes.as_slice(), v2s(&home_at));
        let mut chasing = Vec::with_capacity(listed.len());
        let mut from = 0;
        for n in 0..listed.len() {
            let to = from + counts.as_slice()[n] as usize;
            chasing.push(pursuit::Chasing {
                listed: listed.as_slice()[n] as usize,
                homes: (from..to).map(|h| (homes[h], home_at[h])).collect(),
            });
            from = to;
        }
        let r = rules.as_slice();
        let rules = pursuit::Rules {
            cells_per_second: r[0],
            seconds: r[1],
            give_up: r[2],
            steering: near::Steering { look: r[3], clear: r[4], seed },
        };
        let roster = roster.bind();
        let (squads, columns) = (&mut self.squads, &mut self.field.columns);
        on_ground(ground, |g| pursuit::chase(squads, columns, &roster.roster, &chasing, g, &rules));
        self.edits += 1;
    }

    /// ScrumPursuit._tempt: the units of `roster`'s squad `enemy` (its index) near its squad
    /// `fleeing` may break ranks to chase it, each by its chance and on its leash (in its
    /// order), if within `within` cells of it.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn pursuit_tempt(
        &mut self,
        roster: Gd<SquadRoster>,
        enemy: i64,
        fleeing: i64,
        chances: PackedFloat64Array,
        leashes: PackedFloat64Array,
        within: f64,
        tick: i64,
        seed: i64,
    ) {
        let pairs: Vec<(f64, f64)> =
            chances.as_slice().iter().copied().zip(leashes.as_slice().iter().copied()).collect();
        let temptation = pursuit::Temptation { chances: &pairs, within, tick, seed };
        let roster = roster.bind();
        let pair = (enemy as usize, fleeing as usize);
        pursuit::tempt(
            &mut self.squads,
            &mut self.field.columns,
            &roster.roster,
            pair,
            &temptation,
        );
        self.edits += 1;
    }

    /// FormationPursuit._lagging for `roster`'s squad `listed` (its index).
    #[func]
    fn pursuit_lagging(&self, roster: Gd<SquadRoster>, listed: i64, lag: f64) -> bool {
        let roster = roster.bind();
        pursuit::lagging(&self.squads, &self.field.columns, &roster.roster, listed as usize, lag)
    }

    /// FormationPursuit._spread for `roster`'s squad `listed`, as [x, y, width, height]; empty
    /// for a squad with none living.
    #[func]
    fn pursuit_spread(&self, roster: Gd<SquadRoster>, listed: i64) -> PackedFloat64Array {
        let roster = roster.bind();
        match pursuit::spread(&self.squads, &self.field.columns, &roster.roster, listed as usize) {
            Some(r) => PackedFloat64Array::from(
                &[r.position.x as f64, r.position.y as f64, r.size.x as f64, r.size.y as f64][..],
            ),
            None => PackedFloat64Array::new(),
        }
    }
}

#[godot_api(secondary)]
impl BodyField {
    /// FormationWithdraw.step's flight over `roster`'s withdrawing squads, flat in the lists'
    /// order: per squad its index in the roster, whether a squad pursues it (`pursued`), its
    /// disorder, its re-form pace, its home along its route (cells), how many of its route's
    /// points follow in `points`, and its living units' own flight angles in `fans`, in its
    /// order. `rules`: [pace, seconds, reach_contact, rout_enemy_near, rout_fan_cells].
    /// Returns per squad 1 if it was safe as the tick began, plus 2 if its units are home.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn withdraw(
        &mut self,
        roster: Gd<SquadRoster>,
        listed: PackedInt32Array,
        pursued: PackedByteArray,
        orders: PackedFloat64Array,
        counts: PackedInt32Array,
        points: PackedVector2Array,
        fans: PackedFloat64Array,
        ground: Option<Gd<TerrainGround>>,
        rules: PackedFloat64Array,
    ) -> PackedByteArray {
        let roster = roster.bind();
        let (points, orders, fans) = (v2s(&points), orders.as_slice(), fans.as_slice());
        let mut withdrawing = Vec::with_capacity(listed.len());
        let (mut from, mut fanned) = (0, 0);
        for n in 0..listed.len() {
            let to = from + counts.as_slice()[n] as usize;
            let index = listed.as_slice()[n] as usize;
            let living = roster.roster.listed[index].living.len();
            withdrawing.push(withdraw::Withdrawing {
                listed: index,
                pursued: pursued.as_slice()[n] != 0,
                disorder: orders[3 * n],
                reform: orders[3 * n + 1],
                home: orders[3 * n + 2],
                route: route::Route::new(points[from..to].to_vec(), 0.0),
                fans: fans[fanned..fanned + living].to_vec(),
            });
            from = to;
            fanned += living;
        }
        let r = rules.as_slice();
        let rules =
            withdraw::Rules { pace: r[0], seconds: r[1], reach: r[2], near: r[3], fan_cells: r[4] };
        let (squads, columns) = (&self.squads, &mut self.field.columns);
        let out = on_ground(ground, |g| {
            withdraw::step(squads, columns, &roster.roster, &withdrawing, g, &rules)
        });
        out.iter().map(|&(safe, home)| safe as u8 | (home as u8) << 1).collect()
    }

    /// FormationWithdraw._home: true if all `roster`'s squad `listed`'s units stand within
    /// half a cell of `home` (cells) along the route through `points`.
    #[func]
    fn withdraw_home(
        &self,
        roster: Gd<SquadRoster>,
        listed: i64,
        points: PackedVector2Array,
        home: f64,
    ) -> bool {
        let roster = roster.bind();
        let route = route::Route::new(v2s(&points), 0.0);
        let (squads, columns) = (&self.squads, &self.field.columns);
        withdraw::home(squads, columns, &roster.roster, listed as usize, &route, home)
    }
}

#[godot_api(secondary)]
impl BodyField {
    /// FormationFronts.engage over `roster`'s squads: returns each pick's lock as [the
    /// attacker's index in the roster, its foe's], in the lists' order.
    #[func]
    fn contact_fronts(&mut self, roster: Gd<SquadRoster>, seed: i64) -> PackedInt32Array {
        let roster = roster.bind();
        let picks =
            contact::fronts(&mut self.squads, &mut self.field.columns, &roster.roster, seed);
        self.edits += 1;
        picks.iter().flat_map(|&(a, f)| [a as i32, f as i32]).collect()
    }

    /// FormationEdges.engage over `roster`'s squads at `tick`: returns each flank lock as
    /// [the victim's index in the roster, the attacker's, the edge], in the order they landed.
    #[func]
    fn contact_edges(&mut self, roster: Gd<SquadRoster>, tick: i64, seed: i64) -> PackedInt32Array {
        let roster = roster.bind();
        let flanked =
            contact::edges(&mut self.squads, &mut self.field.columns, &roster.roster, tick, seed);
        self.edits += 1;
        flanked.iter().flat_map(|f| [f.victim as i32, f.attacker as i32, f.edge]).collect()
    }

    /// FormationContact.limit for `roster`'s squad `mover` (its index), wanting to reach
    /// `next` (tiles), `marching` the ids of the squads advancing this tick.
    #[func]
    fn contact_limit(
        &self,
        roster: Gd<SquadRoster>,
        mover: i64,
        next: f64,
        marching: PackedInt64Array,
    ) -> f64 {
        let roster = roster.bind();
        let field = contact::Field {
            squads: &self.squads,
            columns: &self.field.columns,
            roster: &roster.roster,
        };
        field.limit(mover as usize, next, marching.as_slice())
    }

    /// FormationContact.skirmishing for `roster`'s squad `mover`.
    #[func]
    fn contact_skirmishing(&self, roster: Gd<SquadRoster>, mover: i64) -> bool {
        let roster = roster.bind();
        let field = contact::Field {
            squads: &self.squads,
            columns: &self.field.columns,
            roster: &roster.roster,
        };
        field.skirmishing(mover as usize)
    }

    /// FormationContact.joinable for `roster`'s squad `mover`: the index of the friend it may
    /// join, or -1.
    #[func]
    fn contact_joinable(&self, roster: Gd<SquadRoster>, mover: i64) -> i64 {
        let roster = roster.bind();
        let field = contact::Field {
            squads: &self.squads,
            columns: &self.field.columns,
            roster: &roster.roster,
        };
        field.joinable(mover as usize).map_or(-1, |n| n as i64)
    }

    /// FormationPerception.see over `roster`'s squads: per squad, in the lists' order, how
    /// many it detects and then their indices.
    #[func]
    fn perception_see(
        &self,
        roster: Gd<SquadRoster>,
        ground: Option<Gd<TerrainGround>>,
    ) -> PackedInt32Array {
        let roster = roster.bind();
        let (squads, columns) = (&self.squads, &self.field.columns);
        let seen = on_ground(ground, |g| perception::see(squads, columns, &roster.roster, g));
        let mut flat = Vec::new();
        for one in seen {
            flat.push(one.len() as i32);
            flat.extend(one.iter().map(|&n| n as i32));
        }
        PackedInt32Array::from(flat.as_slice())
    }

    /// FormationSight.detects: true if `roster`'s squad `squad` detects its squad `other`.
    #[func]
    fn sight_detects(
        &self,
        roster: Gd<SquadRoster>,
        squad: i64,
        other: i64,
        ground: Option<Gd<TerrainGround>>,
    ) -> bool {
        let roster = roster.bind();
        let listed = &roster.roster.listed;
        let (a, b) = (&listed[squad as usize], &listed[other as usize]);
        on_ground(ground, |g| perception::detects(&self.field.columns, a, b, g))
    }

    /// SquadEdges.bounds for `roster`'s squad `listed`.
    #[func]
    fn squad_bounds(&self, roster: Gd<SquadRoster>, listed: i64) -> Rect2 {
        let roster = roster.bind();
        let shape = geometry::Shape::of(
            &self.squads,
            &self.field.columns,
            &roster.roster.listed[listed as usize],
        );
        let r = shape.bounds();
        Rect2::new(vector2(r.position), vector2(r.size))
    }

    /// SquadEdges.reach for `roster`'s squad `listed`: (least, most) along `way`.
    #[func]
    fn squad_reach(&self, roster: Gd<SquadRoster>, listed: i64, way: Vector2) -> Vector2 {
        let roster = roster.bind();
        let shape = geometry::Shape::of(
            &self.squads,
            &self.field.columns,
            &roster.roster.listed[listed as usize],
        );
        let (least, most) = shape.reach(v2(way));
        Vector2::new(least, most)
    }
}

#[godot_api(secondary)]
impl BodyField {
    /// GroupSplit.step's decisions over `roster`'s squads. Per candidate (at ease, on a
    /// route), in `candidates`: its index in the roster and 1 if its frame waits for every
    /// unit; its route's points follow in `points` (`counts` of them each), and per living
    /// unit, in its order, 1 in `taking` if it is out taking a downed foe and its swap offset
    /// in `offsets`. `rules`: [walk_lost, group_join, cells per tile]. Returns per split, in
    /// the order they are numbered: [its candidate, its front (tiles), its unit count, then
    /// its units (indices into its squad's living) in the order they line up].
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn group_split(
        &self,
        roster: Gd<SquadRoster>,
        candidates: PackedInt32Array,
        counts: PackedInt32Array,
        points: PackedVector2Array,
        taking: PackedByteArray,
        offsets: PackedVector2Array,
        rules: PackedFloat64Array,
        seed: i64,
    ) -> PackedFloat64Array {
        let roster = roster.bind();
        let (points, offsets, taking) = (v2s(&points), v2s(&offsets), taking.as_slice());
        let mut list = Vec::with_capacity(counts.len());
        let (mut from, mut unit) = (0, 0);
        for n in 0..counts.len() {
            let to = from + counts.as_slice()[n] as usize;
            let listed = candidates.as_slice()[2 * n] as usize;
            let living = roster.roster.listed[listed].living.len();
            list.push(split::Candidate {
                listed,
                waits: candidates.as_slice()[2 * n + 1] != 0,
                route: route::Route::new(points[from..to].to_vec(), 0.0),
                taking: taking[unit..unit + living].iter().map(|&t| t != 0).collect(),
                offsets: offsets[unit..unit + living].to_vec(),
            });
            from = to;
            unit += living;
        }
        let r = rules.as_slice();
        let rules = split::Rules { lost: r[0], join: r[1], seed, cells_per_tile: r[2] };
        let splits = split::step(&self.squads, &self.field.columns, &roster.roster, &list, &rules);
        let mut out = Vec::new();
        for s in splits {
            out.extend([s.candidate as f64, s.front, s.members.len() as f64]);
            out.extend(s.members.iter().map(|&u| u as f64));
        }
        PackedFloat64Array::from(&out[..])
    }

    /// FormationGroups._form_up's decisions over `roster`'s squads (the groups at ease with
    /// living units). `groups`: per group [its command, its route] (codes). `followers`: per
    /// living unit, in each group's order, [1 if it leads + 2 if it joins + 4 if it gathers,
    /// the command it started under (-1: none)]. Returns each meeting's outcome in the order
    /// it lands: [0, taker, taken] for a group taken in, or [1, from, home, count, unit ids...]
    /// for units going back to the command they started under.
    #[func]
    fn group_form_up(
        &self,
        roster: Gd<SquadRoster>,
        groups: PackedInt32Array,
        followers: PackedInt32Array,
        join: f64,
        seed: i64,
    ) -> PackedInt64Array {
        let roster = roster.bind();
        let (g, f) = (groups.as_slice(), followers.as_slice());
        let mut list = Vec::with_capacity(roster.roster.listed.len());
        let mut unit = 0;
        for (n, listed) in roster.roster.listed.iter().enumerate() {
            let followers = (unit..unit + listed.living.len())
                .map(|u| groups::Follower {
                    leads: f[2 * u] & 1 != 0,
                    joins: f[2 * u] & 2 != 0,
                    gathers: f[2 * u] & 4 != 0,
                    origin: (f[2 * u + 1] >= 0).then_some(f[2 * u + 1] as u32),
                })
                .collect();
            unit += listed.living.len();
            list.push(groups::Group {
                command: g[2 * n] as u32,
                route: g[2 * n + 1] as u32,
                followers,
            });
        }
        let meetings =
            groups::form_up(&self.squads, &self.field.columns, &roster.roster, &list, join, seed);
        let mut out = Vec::new();
        for meeting in meetings {
            match meeting {
                groups::Meeting::Taken { taker, taken } => {
                    out.extend([0, taker as i64, taken as i64])
                }
                groups::Meeting::Returned { from, home, units } => {
                    out.extend([1, from as i64, home as i64, units.len() as i64]);
                    out.extend(units);
                }
            }
        }
        PackedInt64Array::from(&out[..])
    }
}
