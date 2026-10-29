//! Breach's native core (Decision 129: Rust for the simulation's hot core, through
//! GDExtension with godot-rust). One class, `BodyField`: the bodies' state, owned here and
//! kept across ticks, and the hot passes that run on it - body parting (UnitBodies), the
//! scrum's slot search (ScrumSeek, SlotSearch, ScrumSlots, ScrumNear) and its facing
//! (FormationScrum._faces, UnitShuffle, UnitMotion). GDScript keeps the rules and stays the
//! reference; the switch is `BREACH_NATIVE` (NativeKernels).

mod bands;
mod blows;
#[cfg(test)]
mod build;
mod carry;
mod columns;
mod commands;
mod compact;
mod contact;
pub mod content;
mod contest;
mod deaths;
pub mod designer;
mod det_math;
mod det_pow;
mod detours;
mod discipline;
mod engage;
mod entries;
mod events;
mod face_store;
mod facing;
mod field;
mod field_parting;
mod fight;
mod frame;
mod geometry;
mod ghash;
mod grid;
pub mod ground;
mod groups;
mod hurts;
mod joins;
mod landing;
mod locks;
mod lying;
mod manoeuvre;
mod march;
mod maths;
mod morale;
mod motion;
mod mover;
mod moving;
mod near;
mod objects;
mod ordered;
mod orders;
mod parting;
mod pathing;
mod paths;
mod perception;
mod pursuit;
mod ranks;
mod reach;
mod recovery;
mod rect;
mod regeneration;
mod regroup;
mod rendezvous;
mod roster;
mod rout;
mod rout_blows;
mod route;
mod scrum;
mod seeking;
mod sheet;
mod shocks;
mod shots;
mod snapshot;
mod split;
mod squads;
mod staging;
mod stamina;
mod stance;
mod steer;
mod strays;
mod swaps;
mod taking;
mod trade;
mod turn;
mod walk_store;
mod walking;
mod withdraw;
mod words;
mod wounds;

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
    /// The events the passes have built and GDScript hasn't yet taken (spec 39 step 8b5-v).
    events: events::Log,
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

    /// A unit's numbers' names (SkirmishUnit's properties), by index (spec 39 step 8b5-i).
    #[func]
    fn sheet_names(&self) -> PackedStringArray {
        sheet::NAMES.iter().map(|name| GString::from(*name)).collect()
    }

    /// A slot's numbers, by index.
    #[func]
    fn sheet(&self, slot: i64) -> PackedFloat64Array {
        let sheet = &self.field.columns.sheet[slot as usize];
        (0..sheet::NAMES.len()).map(|index| sheet.get(index)).collect()
    }

    /// Sets a slot's numbers, by index (UnitStore.admit: all of a unit's at once).
    #[func]
    fn put_sheet(&mut self, slot: i64, values: PackedFloat64Array) {
        let sheet = &mut self.field.columns.sheet[slot as usize];
        for (index, value) in values.as_slice().iter().enumerate() {
            sheet.set(index, *value);
        }
    }

    /// Sets one of a slot's numbers (SkirmishUnit's setters).
    #[func]
    fn put_sheet_value(&mut self, slot: i64, index: i64, value: f64) {
        self.field.columns.sheet[slot as usize].set(index as usize, value);
    }

    /// A unit's words' and lists' names (SkirmishUnit's properties), by shape: the words,
    /// the lists, the levels, then the parts (spec 39 step 8b5-ii).
    #[func]
    fn word_names(&self) -> PackedStringArray {
        let shapes = [words::WORDS, words::LISTS, words::LEVELS, words::PARTS];
        shapes.iter().flat_map(|names| names.iter()).map(|name| GString::from(*name)).collect()
    }

    /// Sets a slot's word `name` (false: no such word).
    #[func]
    fn put_word(&mut self, slot: i64, name: GString, word: GString) -> bool {
        let code = self.field.lexicon.code(&word.to_string());
        let held = self.field.columns.words[slot as usize].word(&name.to_string());
        held.map(|at| *at = code).is_some()
    }

    /// Sets a slot's list of words `name`, in order (false: no such list).
    #[func]
    fn put_word_list(&mut self, slot: i64, name: GString, list: PackedStringArray) -> bool {
        let lexicon = &mut self.field.lexicon;
        let codes: Vec<u32> =
            list.as_slice().iter().map(|w| lexicon.code(&w.to_string())).collect();
        let held = self.field.columns.words[slot as usize].list(&name.to_string());
        held.map(|at| *at = codes).is_some()
    }

    /// Sets a slot's words with levels `name`, {word: level} in its order (false: none such).
    #[func]
    fn put_word_levels(&mut self, slot: i64, name: GString, levels: VarDictionary) -> bool {
        let lexicon = &mut self.field.lexicon;
        let coded: Vec<(u32, f64)> = levels
            .iter_shared()
            .map(|(word, level)| (lexicon.code(&word.stringify().to_string()), number(&level)))
            .collect();
        let held = self.field.columns.words[slot as usize].levels(&name.to_string());
        held.map(|at| *at = coded).is_some()
    }

    /// Sets a slot's weapons' parts `name`, [[damage, type, magical], ...] (false: none such).
    #[func]
    fn put_word_parts(&mut self, slot: i64, name: GString, parts: VarArray) -> bool {
        let lexicon = &mut self.field.lexicon;
        let coded: Vec<words::Part> = parts
            .iter_shared()
            .map(|part| {
                let part = part.to::<VarArray>();
                words::Part {
                    damage: number(&part.at(0)),
                    kind: lexicon.code(&part.at(1).stringify().to_string()),
                    magical: number(&part.at(2)) != 0.0,
                }
            })
            .collect();
        let held = self.field.columns.words[slot as usize].parts(&name.to_string());
        held.map(|at| *at = coded).is_some()
    }

    /// A slot's words and lists, by name, read back as SkirmishUnit holds them.
    #[func]
    fn words(&self, slot: i64) -> VarDictionary {
        let lexicon = &self.field.lexicon;
        let mut held = self.field.columns.words[slot as usize].clone();
        let mut out = VarDictionary::new();
        for name in words::WORDS {
            let code = *held.word(name).unwrap();
            out.set(*name, &GString::from(lexicon.word(code)).to_variant());
        }
        for name in words::LISTS {
            let list: PackedStringArray =
                held.list(name).unwrap().iter().map(|&c| GString::from(lexicon.word(c))).collect();
            out.set(*name, &list.to_variant());
        }
        for name in words::LEVELS {
            let mut levels = VarDictionary::new();
            for &(code, level) in held.levels(name).unwrap().iter() {
                levels.set(lexicon.word(code), level);
            }
            out.set(*name, &levels.to_variant());
        }
        for name in words::PARTS {
            let mut parts = VarArray::new();
            for part in held.parts(name).unwrap().iter() {
                let kind = GString::from(lexicon.word(part.kind));
                parts.push(&varray![part.damage, &kind, part.magical].to_variant());
            }
            out.set(*name, &parts.to_variant());
        }
        out
    }

    /// A unit's objects' names (SkirmishUnit's properties): its walker, then its codes
    /// (spec 39 step 8b5-iii).
    #[func]
    fn object_names(&self) -> PackedStringArray {
        let names = std::iter::once("walker").chain(objects::CODES.iter().copied());
        names.map(GString::from).collect()
    }

    /// Sets a slot's walker from its fields (TerrainWalker.fields; empty for none).
    #[func]
    fn put_walker(&mut self, slot: i64, fields: PackedFloat64Array) {
        self.field.columns.objects[slot as usize].put_walker(fields.as_slice());
    }

    /// Sets a slot's code `name`: a command's slot or a definition's code, -1 for none
    /// (false: no such code).
    #[func]
    fn put_object(&mut self, slot: i64, name: GString, code: i64) -> bool {
        let held = self.field.columns.objects[slot as usize].code(&name.to_string());
        held.map(|at| *at = code).is_some()
    }

    /// A slot's objects, by name: its walker's fields (empty for none) and its codes.
    #[func]
    fn objects(&self, slot: i64) -> VarDictionary {
        let mut held = self.field.columns.objects[slot as usize].clone();
        let walker: PackedFloat64Array = held.walker.iter().flatten().copied().collect();
        let mut out = VarDictionary::new();
        out.set("walker", &walker.to_variant());
        for name in objects::CODES {
            out.set(*name, *held.code(name).unwrap());
        }
        out
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
/// ScrumReach are views over it, and FormationMorale over its ceiling, bands and shocks
/// (`bands`, step 8a2-iv). Stateless: one is shared.
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

    /// FormationStamina.stage_at: 0 fresh, 1 tired, 2 spent, below the shares `tired` and
    /// `spent` of its `most` stamina.
    #[func]
    fn stamina_stage(&self, stamina: f64, most: f64, tired: f64, spent: f64) -> i64 {
        stamina::stage_of(stamina, most, tired, spent) as i64
    }

    /// FormationMorale.leadership, over the living's leaderships.
    #[func]
    fn morale_leadership(&self, leaderships: PackedInt64Array) -> i64 {
        bands::leadership(leaderships.as_slice())
    }

    /// FormationMorale.ceiling, over the living's courage and leaderships; `tuning` [per
    /// leadership, steady at, shaken at, loss, leader loss] (`bands::BandTuning`).
    #[func]
    fn morale_ceiling(
        &self,
        courage: PackedInt64Array,
        leaderships: PackedInt64Array,
        tuning: PackedInt64Array,
    ) -> i64 {
        let t = bands::BandTuning::of_slice(tuning.as_slice());
        bands::ceiling(courage.as_slice(), leaderships.as_slice(), &t)
    }

    /// FormationMorale.band at `morale`: 0 steady, 1 shaken, 2 wavering, 3 routing.
    #[func]
    fn morale_band(&self, morale: i64, tuning: PackedInt64Array) -> i64 {
        bands::band(morale, &bands::BandTuning::of_slice(tuning.as_slice()))
    }

    /// FormationMorale.interval.
    #[func]
    fn morale_interval(&self, band: i64, usual: i64) -> i64 {
        bands::interval(band, usual)
    }

    /// The morale `value` comes to, held to 0..100.
    #[func]
    fn morale_held(&self, value: i64) -> i64 {
        bands::held(value)
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
        let patches = patches_of(&froms, &tos, &counts, &waypoints);
        let out = route::Patch::patched(&route_of(&points), &patches);
        packed(&out.points)
    }
}

/// A squad as march::shuffle takes it: [width, combat width, centre shift, fighting].
fn shuffle_squad(shape: &PackedFloat64Array) -> march::shuffle::Squad {
    let s = shape.as_slice();
    march::shuffle::Squad {
        width: s[0] as i32,
        combat_width: s[1] as i32,
        centre_shift: s[2],
        fighting: s[3] != 0.0,
    }
}

/// Living units as march::shuffle takes them: seven numbers each in `units` (rank, column,
/// footprint width and depth, band, priority, joined), at `speeds`.
fn ranked_of(units: &PackedInt32Array, speeds: &PackedFloat64Array) -> Vec<march::shuffle::Ranked> {
    let (rows, _) = units.as_slice().as_chunks::<7>();
    rows.iter()
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
        .collect()
}

/// [tick seconds, cells a second at speed 1, scrum_crowding].
fn pace_of(pace: &PackedFloat64Array) -> [f64; 3] {
    let p = pace.as_slice();
    [p[0], p[1], p[2]]
}

/// A number GDScript handed in, an int or a float.
fn variant_number(value: &Variant) -> f64 {
    match value.get_type() {
        VariantType::INT => value.to::<i64>() as f64,
        _ => value.try_to::<f64>().unwrap_or(0.0),
    }
}

/// Route patches handed in flat: patch `i` leaves at `froms[i]`, rejoins at `tos[i]` and
/// has the next `counts[i]` of `waypoints`.
fn patches_of(
    froms: &PackedFloat64Array,
    tos: &PackedFloat64Array,
    counts: &PackedInt32Array,
    waypoints: &PackedVector2Array,
) -> Vec<route::Patch> {
    let all = v2s(waypoints);
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
    patches
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
}

/// The march (`march`, spec 39 step 3d) for FormationMarch, FormationSweep, FormationWalk,
/// FormationPour, FormationShuffle and PlaceOrder, their views. Stateless, so one is shared
/// (`CrateMarch`).
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct GroundMarch {}

#[godot_api]
impl GroundMarch {
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

    /// FormationRendezvous.ticks_to: the ticks for a march `march` = [cells along the route,
    /// width, cells a second, seconds a tick, cells along it it sets out from, slack] to come
    /// along `route`, over `ground` (none: open ground).
    #[func]
    fn rendezvous_ticks(
        &self,
        route: PackedVector2Array,
        march: PackedFloat64Array,
        ground: Option<Gd<TerrainGround>>,
    ) -> i64 {
        let route = route::Route::new(v2s(&route), 0.0);
        let m = march.as_slice();
        let march = rendezvous::March {
            cells: m[0],
            width: m[1] as i32,
            cells_per_second: m[2],
            tick_seconds: m[3],
            from: m[4],
            slack: m[5],
        };
        on_ground(ground, |g| rendezvous::ticks_to(&route, &march, g))
    }

    /// FormationRendezvous.waits: the ticks each wave waits, from each one's predicted ticks.
    #[func]
    fn rendezvous_waits(&self, predicted: PackedInt64Array) -> PackedInt64Array {
        PackedInt64Array::from(rendezvous::waits(predicted.as_slice()).as_slice())
    }

    /// PlaceOrder.front_first: the indices of the units at `ranks` and `files`, front first.
    #[func]
    fn front_first(&self, ranks: PackedInt32Array, files: PackedInt32Array) -> PackedInt32Array {
        let places: Vec<(i32, i32)> =
            ranks.as_slice().iter().copied().zip(files.as_slice().iter().copied()).collect();
        march::shuffle::front_first(&places).iter().map(|&index| index as i32).collect()
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

/// The share's tuning: the fight's `seed`, [discipline_per_leadership, walk_slack_loose,
/// walk_slack_drilled] and [wounds_reach, walk_lost].
fn share_tuning(seed: i64, slack: &[f64], reach: &[f64]) -> moving::share::ShareTuning {
    moving::share::ShareTuning {
        seed,
        wounds_reach: reach[0],
        lost: reach[1],
        slack: [slack[0], slack[1], slack[2]],
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

/// Members as `members_of` reads them: sixteen numbers each.
fn rows_of(members: &[pathing::Member]) -> PackedFloat64Array {
    let mut out = Vec::with_capacity(members.len() * 16);
    for m in members {
        let (w, flag) = (&m.walker.walker, |on: bool| on as i64 as f64);
        out.extend([w.height, w.swimmer as f64, w.climber as f64, flag(w.swims), flag(w.climbs)]);
        out.extend([m.walker.stamina, m.walker.speed, m.wits as f64, m.id as f64]);
        out.extend([f64::from(m.at.x), f64::from(m.at.y)]);
        match m.eyes {
            Some([range, ahead, side, behind]) => out.extend([1.0, range, ahead, side, behind]),
            None => out.extend([0.0; 5]),
        }
    }
    PackedFloat64Array::from(out.as_slice())
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

    /// The squad's pursuit as a whole as [foe, post, post x, y, home, direction, order, 1 if
    /// returning else 0], then the route it held its post on, if any, as [its corridor's
    /// half width, then its points' x and y]; or empty for none.
    #[func]
    fn squad_pursuit(&self, squad: i64) -> PackedFloat64Array {
        let s = squad as usize;
        let Some(p) = self.squads.pursuit[s] else { return PackedFloat64Array::new() };
        let mut held = vec![
            p.foe as f64,
            p.post,
            p.post_at.x as f64,
            p.post_at.y as f64,
            p.home,
            p.direction as f64,
            p.order as f64,
            if p.returning { 1.0 } else { 0.0 },
        ];
        if let Some(route) = &self.squads.post_route[s] {
            held.push(route.half_width);
            held.extend(route.points.iter().flat_map(|p| [p.x as f64, p.y as f64]));
        }
        PackedFloat64Array::from(held.as_slice())
    }

    /// Sets the squad's pursuit, and the route it held its post on, from what
    /// `squad_pursuit` reads, or none from empty.
    #[func]
    fn put_squad_pursuit(&mut self, squad: i64, pursuit: PackedFloat64Array) {
        let p = pursuit.as_slice();
        self.squads.post_route[squad as usize] = (p.len() > 8).then(|| {
            let points = p[9..].chunks(2).map(|xy| V2::new(xy[0] as f32, xy[1] as f32));
            route::Route::new(points.collect(), p[8])
        });
        self.squads.pursuit[squad as usize] = (p.len() >= 8).then(|| squads::Pursuit {
            foe: p[0] as i64,
            post: p[1],
            post_at: V2::new(p[2] as f32, p[3] as f32),
            home: p[4],
            direction: p[5] as i32,
            order: p[6] as i32,
            returning: p[7] != 0.0,
        });
        self.edits += 1;
    }

    /// The ticks the squad's withdrawal has felt safe, or -1 for none under way.
    #[func]
    fn squad_withdraw(&self, squad: i64) -> i64 {
        self.squads.withdraw[squad as usize].unwrap_or(-1)
    }

    /// Sets the ticks the squad's withdrawal has felt safe (below 0: none under way).
    #[func]
    fn put_squad_withdraw(&mut self, squad: i64, safe_ticks: i64) {
        self.squads.withdraw[squad as usize] = (safe_ticks >= 0).then_some(safe_ticks);
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

    /// The route the squad in `squad` follows, as GDScript's SkirmishSquad.route sets it:
    /// its `points` and corridor's `half_width`, or none if `routed` is false.
    #[func]
    fn squad_put_route(
        &mut self,
        squad: i64,
        routed: bool,
        points: PackedVector2Array,
        half_width: f64,
    ) {
        self.squads.route[squad as usize] =
            routed.then(|| route::Route::new(v2s(&points), half_width));
    }

    /// The points of the route the squad in `squad` follows (none: a straight lane).
    #[func]
    fn squad_route(&self, squad: i64) -> PackedVector2Array {
        self.squads.route[squad as usize]
            .as_ref()
            .map_or_else(PackedVector2Array::new, |r| packed(&r.points))
    }

    /// FormationMarch.face for the squad in `squad`, along the route it holds
    /// (`march::sweep::face_held`).
    #[func]
    fn march_face(&mut self, squad: i64) {
        march::sweep::face_held(&mut self.squads, squad as usize);
    }

    /// FormationCommand.new: a command calling `home` its home (tiles along its route), on
    /// no route, pursuing and taking. Returns its slot.
    #[func]
    fn command_open(&mut self, home: f64) -> i64 {
        self.squads.commands.open(home) as i64
    }

    /// The command in `command` takes `points` as its guide route (corridor `half_width`), or
    /// none if `routed` is false.
    #[func]
    fn command_put_route(
        &mut self,
        command: i64,
        routed: bool,
        points: PackedVector2Array,
        half_width: f64,
    ) {
        self.squads.commands.route[command as usize] =
            routed.then(|| route::Route::new(v2s(&points), half_width));
    }

    /// The tiles along its route the command in `command`'s home end lies at.
    #[func]
    fn command_home(&self, command: i64) -> f64 {
        self.squads.commands.home[command as usize]
    }

    #[func]
    fn put_command_home(&mut self, command: i64, home: f64) {
        self.squads.commands.home[command as usize] = home;
    }

    /// Whether the command in `command` gives `flag` (`commands::HURRY`, `MERGES`,
    /// `PURSUES` or `TAKES`).
    #[func]
    fn command_gives(&self, command: i64, flag: i64) -> bool {
        self.squads.commands.gives(command as usize, flag as u8)
    }

    #[func]
    fn command_give(&mut self, command: i64, flag: i64, on: bool) {
        self.squads.commands.give(command as usize, flag as u8, on);
    }

    /// What the command in `command` does with its own downed ("", "recover" or "carry").
    #[func]
    fn command_tends(&self, command: i64) -> GString {
        GString::from(self.squads.commands.tends[command as usize].as_str())
    }

    #[func]
    fn put_command_tends(&mut self, command: i64, tends: GString) {
        self.squads.commands.tends[command as usize] = tends.to_string();
    }

    /// Keeps the way round `detour` on the command in `command` (`Commands::learn`).
    #[func]
    fn command_learn(&mut self, command: i64, detour: PackedVector2Array) {
        self.squads.commands.learn(command as usize, &v2s(&detour));
    }

    /// How many ways round the command in `command` has learned.
    #[func]
    fn command_patch_count(&self, command: i64) -> i64 {
        self.squads.commands.patches[command as usize].len() as i64
    }

    /// The squad in `squad` follows the command in `command`.
    #[func]
    fn squad_put_command(&mut self, squad: i64, command: i64) {
        self.squads.command[squad as usize] = Some(command as usize);
    }

    /// FormationSimulation.order: `order` for the squad `squad_id` takes effect on the next
    /// tick (`orders::queue`).
    #[func]
    fn order_give(&mut self, squad_id: i64, order: i64) {
        orders::queue(&mut self.squads, squad_id, order as i32);
    }

    /// The orders queued for the next tick, two numbers each: [squad id, order].
    #[func]
    fn orders_pending(&self) -> PackedInt64Array {
        let flat: Vec<i64> =
            self.squads.pending.iter().flat_map(|&(id, order)| [id, i64::from(order)]).collect();
        PackedInt64Array::from(&flat[..])
    }

    /// The orders queued are dropped (a tick took them, or nobody was left to take them).
    #[func]
    fn orders_clear(&mut self) {
        self.squads.pending.clear();
    }

    /// The next squad id the battle issues (`Squads::issue_squad`).
    #[func]
    fn issue_squad(&mut self) -> i64 {
        self.squads.issue_squad()
    }

    /// The first of `count` unit ids the battle issues together (`Squads::issue_units`).
    #[func]
    fn issue_units(&mut self, count: i64) -> i64 {
        self.squads.issue_units(count)
    }

    /// How many squad ids the battle has issued; the strays and the groups issue theirs in
    /// GDScript and hand the count back (`put_squads_issued`).
    #[func]
    fn squads_issued(&self) -> i64 {
        self.squads.squads_issued
    }

    #[func]
    fn put_squads_issued(&mut self, issued: i64) {
        self.squads.squads_issued = issued;
    }

    /// FormationContact.can_engage.
    #[func]
    fn squad_can_engage(&self, squad: i64) -> bool {
        locks::can_engage(&self.squads, squad as usize)
    }

    /// FormationScrum.regrouping for the squad in `squad` (`moving::regrouping`).
    #[func]
    fn squad_regrouping(&self, squad: i64) -> bool {
        moving::regrouping(&self.squads, squad as usize)
    }

    /// FormationManoeuvre.step over `roster`'s squads: each one's manoeuvre this tick
    /// (`scrum::pass::manoeuvre`).
    #[func]
    fn scrum_manoeuvre(&mut self, roster: Gd<SquadRoster>) {
        let roster = roster.bind();
        let (squads, columns) = (&mut self.squads, &mut self.field.columns);
        scrum::pass::manoeuvre(squads, columns, &roster.roster);
        self.edits += 1;
    }

    /// FormationScrum._stall over `roster`'s squads on `tick`: counts each fighting squad
    /// quiet unless it or a foe is `active` (squad ids), and releases those stalled `ticks`;
    /// each release's "disengaged" event goes to the log (`scrum::pass::stall`, `events_take`).
    #[func]
    fn scrum_stall(
        &mut self,
        roster: Gd<SquadRoster>,
        active: PackedInt64Array,
        ticks: i64,
        tick: i64,
    ) {
        let roster = roster.bind();
        let rules = (active.as_slice(), ticks);
        scrum::pass::stall(&mut self.squads, &roster.roster, rules, tick, &mut self.events);
        self.edits += 1;
    }

    /// ScrumEngage.step over `roster`'s squads on `tick`: free squads lock onto the nearest
    /// hostile their units have come within `reach_engage` of; each pick's "engaged" event
    /// goes to the log (`scrum::pass::engage`, `events_take`).
    #[func]
    fn scrum_engage(&mut self, roster: Gd<SquadRoster>, tick: i64, seed: i64, reach_engage: f64) {
        let roster = roster.bind();
        let (squads, columns) = (&mut self.squads, &mut self.field.columns);
        let rules = (tick, seed, reach_engage);
        scrum::pass::engage(squads, columns, &roster.roster, rules, &mut self.events);
        self.edits += 1;
    }

    /// FormationScrum._prepare over `roster`'s squads on `tick`: each readied for the scrum
    /// (`scrum::prepare::prepare`).
    #[func]
    fn scrum_prepare(&mut self, roster: Gd<SquadRoster>, tick: i64) {
        let roster = roster.bind();
        scrum::prepare::prepare(&mut self.squads, &mut self.field, &roster.roster, tick);
        self.edits += 1;
    }

    /// ScrumStance.anticipate_all over `roster`'s squads on `ground`, on `tick`: each meets
    /// the threats it sees; stances let go are cleared and new ones taken, each its "faced"
    /// event in the log (`scrum::prepare::anticipate`, `events_take`). `rules`:
    /// [scrum_anticipate, discipline_per_leadership, discipline_meets_threats].
    #[func]
    fn scrum_stance(
        &mut self,
        roster: Gd<SquadRoster>,
        ground: Option<Gd<TerrainGround>>,
        rules: PackedFloat64Array,
        tick: i64,
        seed: i64,
    ) {
        let r = rules.as_slice();
        let discipline = discipline::DisciplineTuning {
            per_leadership: r[1] as i64,
            meets_threats: r[2] as i64,
            ..discipline::DisciplineTuning::default()
        };
        let lines = scrum::prepare::Lines { anticipate: r[0], seed, discipline: &discipline };
        let roster = roster.bind();
        let (squads, field, log) = (&mut self.squads, &self.field, &mut self.events);
        on_ground(ground, |g| {
            scrum::prepare::anticipate(squads, field, &roster.roster, g, (&lines, tick), log)
        });
        self.edits += 1;
    }

    /// FormationScrum._seek over `roster`'s squads on `ground`: the field brought to the
    /// scrum's snapshot, every fighting squad's units planned, walked and turned
    /// (`scrum::seek::step`). `ints` = [tick, seed, combat_contest_die]; `floats` =
    /// [reach_contact, scrum_leash, tick seconds, pace (cells a tick at speed 1),
    /// scrum_crowding, bodies_steer_look, bodies_steer_clear, bodies_ground_drag,
    /// bodies_ground_block, the front's least dot (ScrumReach.in_front)]. Returns the ids of
    /// the fighting squads active this tick (the stall's).
    #[func]
    fn scrum_seek(
        &mut self,
        roster: Gd<SquadRoster>,
        ground: Option<Gd<TerrainGround>>,
        ints: PackedInt64Array,
        floats: PackedFloat64Array,
    ) -> PackedInt64Array {
        let began = Instant::now();
        let (i, f) = (ints.as_slice(), floats.as_slice());
        let rules = scrum::seek::Rules {
            tick: i[0],
            seed: i[1],
            die: i[2],
            reach: f[0],
            leash: f[1],
            seconds: f[2],
            pace: f[3],
            crowding: f[4],
            look: f[5],
            clear: f[6],
            drag: f[7],
            block: f[8],
            front: f[9],
        };
        let roster = roster.bind();
        let (squads, field) = (&self.squads, &mut self.field);
        let active =
            on_ground(ground, |g| scrum::seek::step(squads, field, &roster.roster, g, &rules));
        self.usec = began.elapsed().as_micros() as i64;
        self.edits += 1;
        PackedInt64Array::from(active.as_slice())
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
    /// ScrumRegroup.step and ScrumPursuit.step over `roster`'s squads on `ground`: those out
    /// of the fight walk their units back, then the chasers chase (`scrum::regroup::step`),
    /// each squad's say read where the crate holds it. `rules`: [pace (cells a tick at speed
    /// 1), seconds, reach_contact, reach_front_arc_degrees, bodies_steer_look,
    /// bodies_steer_clear, scrum_trade_seconds, cells a second at speed 1, stamina_give_up,
    /// discipline_per_leadership, discipline_meets_threats, discipline_march_pace_at,
    /// discipline_slowest, discipline_fastest, wounds_reach].
    #[func]
    fn scrum_regroup(
        &mut self,
        roster: Gd<SquadRoster>,
        ground: Option<Gd<TerrainGround>>,
        rules: PackedFloat64Array,
        seed: i64,
    ) {
        let r = rules.as_slice();
        let steering = near::Steering { look: r[4], clear: r[5], seed };
        let rules = scrum::regroup::Rules {
            regroup: regroup::Rules {
                pace: r[0],
                seconds: r[1],
                seed,
                reach: r[2],
                arc: r[3],
                look: r[4],
                clear: r[5],
                trade_seconds: r[6],
            },
            chase: pursuit::Rules {
                cells_per_second: r[7],
                seconds: r[1],
                give_up: r[8],
                steering,
            },
            discipline: discipline::DisciplineTuning {
                per_leadership: r[9] as i64,
                meets_threats: r[10] as i64,
                ..discipline::DisciplineTuning::default()
            },
            reform: [r[11], r[12], r[13]],
            seed,
            wounds_reach: r[14],
        };
        let roster = roster.bind();
        let (squads, field) = (&mut self.squads, &mut self.field);
        on_ground(ground, |g| scrum::regroup::step(squads, field, &roster.roster, g, &rules));
        self.edits += 1;
    }
}

#[godot_api(secondary)]
impl BodyField {
    /// FormationOrders.apply over `roster`'s squads: the orders queued (`orders::queue`), in
    /// the order given, which it takes (`orders::pass`). Per listed squad a code naming the
    /// route it holds (-1: none, the straight lane; empty when no order retreats). `rules`:
    /// [pursuit_tempted_within, pursuit_ragged_shock, discipline_per_leadership,
    /// discipline_meets_threats, discipline_expected_max, how many leash shares follow, the
    /// shares, the leash steps]; `bands`: morale's (`bands::BandTuning`). Its events go to the
    /// log (`events_take`). Returns four numbers an enemy that pursues a retreat, in order: the
    /// quarry's index in the roster, the enemy's, 1 if its pursuit began now, 1 if it took its
    /// quarry's route.
    #[func]
    fn apply_orders(
        &mut self,
        roster: Gd<SquadRoster>,
        routes: PackedInt64Array,
        rules: PackedFloat64Array,
        bands: PackedInt64Array,
        tick: i64,
        seed: i64,
    ) -> PackedInt64Array {
        let r = rules.as_slice();
        let shares = r[5] as usize;
        let discipline = discipline::DisciplineTuning {
            per_leadership: r[2] as i64,
            meets_threats: r[3] as i64,
            expected_max: r[4],
            leash_shares: r[6..6 + shares].to_vec(),
            leash_steps: r[6 + shares..].to_vec(),
        };
        let bands = bands::BandTuning::of_slice(bands.as_slice());
        let roster = roster.bind();
        let given = orders::pass::Given {
            routes: coded_routes(&self.squads, &roster.roster, &routes),
            within: r[0],
            ragged_shock: r[1] as i64,
            discipline: &discipline,
            bands: &bands,
            tick,
            seed,
        };
        let (squads, columns) = (&mut self.squads, &mut self.field.columns);
        let applied = orders::pass::pass(squads, columns, &roster.roster, given, &mut self.events);
        self.edits += 1;
        let mut flat: Vec<i64> = Vec::new();
        for a in applied {
            for p in a.retreat.unwrap_or_default() {
                flat.extend([a.listed as i64, p.listed as i64, p.began as i64, p.took as i64]);
            }
        }
        PackedInt64Array::from(flat.as_slice())
    }

    /// The events the passes have built since last taken, flat (`events::Log`); the log is
    /// left empty.
    #[func]
    fn events_take(&mut self) -> PackedInt64Array {
        PackedInt64Array::from(self.events.take().as_slice())
    }

    /// Each event kind, by its index: [its type, its fields' names, whether it is told of a
    /// unit] (`events::KINDS`, `events::UNIT_LED`).
    #[func]
    fn event_kinds(&self) -> Array<Variant> {
        events::KINDS
            .iter()
            .enumerate()
            .map(|(n, (kind, fields))| {
                let names: PackedStringArray = fields.iter().map(|&f| GString::from(f)).collect();
                let unit = events::UNIT_LED.contains(&n);
                varray![&GString::from(*kind), &names, unit].to_variant()
            })
            .collect()
    }

    /// The field's words, by their codes (`words::Lexicon`).
    #[func]
    fn lexicon_words(&self) -> PackedStringArray {
        self.field.lexicon.words().iter().map(|w| GString::from(w.as_str())).collect()
    }

    /// FormationPursuit.step over `roster`'s squads on `tick` (`scrum::pursue::step`): each
    /// pursuer's routes and leash read in the crate, then it advances after its enemy, gives
    /// up ("pursuit_ended") or, back at its post, takes up the route it held it on and turns
    /// to face the way it held it ("turning", "turned"), in the log (`events_take`). `rules`:
    /// [cells a second at speed 1, seconds, stamina_give_up, reach_pursuit_lag,
    /// discipline_per_leadership, discipline_expected_max, how many leash shares, the
    /// shares, then the leash steps]. Returns the listed indices of those back at their
    /// posts, in order.
    #[func]
    fn scrum_pursue(
        &mut self,
        roster: Gd<SquadRoster>,
        rules: PackedFloat64Array,
        tick: i64,
    ) -> PackedInt32Array {
        let r = rules.as_slice();
        let shares = r[6] as usize;
        let rules = scrum::pursue::Rules {
            whole: pursuit::whole::Rules {
                cells_per_second: r[0],
                seconds: r[1],
                give_up: r[2],
                lag: r[3],
            },
            discipline: discipline::DisciplineTuning {
                per_leadership: r[4] as i64,
                expected_max: r[5],
                leash_shares: r[7..7 + shares].to_vec(),
                leash_steps: r[7 + shares..].to_vec(),
                ..discipline::DisciplineTuning::default()
            },
            tick,
        };
        let roster = roster.bind();
        let (squads, field, log) = (&mut self.squads, &mut self.field, &mut self.events);
        let back = scrum::pursue::step(squads, field, &roster.roster, &rules, log);
        self.edits += 1;
        back.into_iter().map(|n| n as i32).collect()
    }
}

/// Each listed squad's route as [its code, the route it holds]: `codes` one a squad in
/// `roster` (-1 for none).
fn coded_routes(
    squads: &Squads,
    roster: &roster::Roster,
    codes: &PackedInt64Array,
) -> Vec<orders::Coded> {
    let held = |n: usize| squads.route[roster.listed[n].squad].clone();
    codes
        .as_slice()
        .iter()
        .enumerate()
        .map(|(n, &code)| (code, if code >= 0 { held(n) } else { None }))
        .collect()
}

#[godot_api(secondary)]
impl BodyField {
    /// FormationWithdraw.step over `roster`'s squads on `tick` (`scrum::withdraw::step`):
    /// those routing or destroyed withdraw no more; the rest flee, each one's disorder,
    /// re-form pace, home and its units' flight angles (seeded by `seed`) read in the crate,
    /// and those safe long enough, or home, re-form, their turns, "regrouping" and
    /// "returned" in the log (`events_take`). `rules`: [pace, seconds, reach_contact,
    /// rout_enemy_near, rout_fan_cells, rout_rally_seconds, rout_fan_degrees,
    /// discipline_per_leadership, discipline_meets_threats, discipline_march_pace_at,
    /// discipline_slowest, discipline_fastest].
    #[func]
    fn scrum_withdraw(
        &mut self,
        roster: Gd<SquadRoster>,
        ground: Option<Gd<TerrainGround>>,
        rules: PackedFloat64Array,
        tick: i64,
        seed: i64,
    ) {
        let r = rules.as_slice();
        let rules = scrum::withdraw::Rules {
            flight: withdraw::Rules {
                pace: r[0],
                seconds: r[1],
                reach: r[2],
                near: r[3],
                fan_cells: r[4],
            },
            rally_seconds: r[5],
            fan_degrees: r[6],
            discipline: discipline::DisciplineTuning {
                per_leadership: r[7] as i64,
                meets_threats: r[8] as i64,
                ..discipline::DisciplineTuning::default()
            },
            reform: [r[9], r[10], r[11]],
            seed,
            tick,
        };
        let roster = roster.bind();
        let (squads, field, log) = (&mut self.squads, &mut self.field, &mut self.events);
        on_ground(ground, |g| scrum::withdraw::step(squads, field, &roster.roster, g, &rules, log));
        self.edits += 1;
    }

    /// FormationMarch.check_ends, the squad in `squad`, `along` as [front, home, order,
    /// direction], on a route `route_end` tiles long (the one it holds). Whether all its units
    /// are home (FormationWithdraw's test, half a cell of its home end) reads `roster`'s squad
    /// `listed`, handed in for a retreat. Returns [0 marching on, 1 arrived or 2 returned,
    /// the front's tiles along its route].
    #[func]
    fn march_end(
        &self,
        along: PackedFloat64Array,
        route_end: f64,
        squad: i64,
        roster: Option<Gd<SquadRoster>>,
        listed: i64,
    ) -> PackedFloat64Array {
        let along = along_of(&along);
        let held = self.squads.route[squad as usize].as_ref();
        let all_home = || {
            roster.zip(held).is_some_and(|(roster, route)| {
                let roster = roster.bind();
                let home = along.home * content::CELLS_PER_TILE as f64;
                let (squads, columns) = (&self.squads, &self.field.columns);
                withdraw::home(squads, columns, &roster.roster, listed as usize, route, home)
            })
        };
        let ended = match march::end(&along, route_end, held.is_some(), all_home) {
            march::End::Under => [0.0, along.front],
            march::End::Arrived(front) => [1.0, front],
            march::End::Returned(front) => [2.0, front],
        };
        PackedFloat64Array::from(ended.as_slice())
    }

    /// FormationSimulation._move over `roster`'s squads on `ground` (`moving::pass`): per
    /// listed squad its route's code (`routes`, FormationStaging's); `tuning`: [TRAVEL_SCALE,
    /// seconds a tick, the lane's length (tiles), discipline_per_leadership, walk_slack_loose,
    /// walk_slack_drilled, bodies_ground_drag, bodies_ground_block, wounds_reach, walk_lost].
    /// Each squad's share of its pace is worked out here (`moving::share`, by the fight's
    /// `seed`). The squads that advanced then join any friend whose back rank they reached
    /// (`joins::step`). Its events go to the log (`events_take`). Returns the joins as pairs
    /// of listed indices, the leader's then the joining squad's, in the order joined.
    #[func]
    fn move_step(
        &mut self,
        roster: Gd<SquadRoster>,
        routes: PackedInt32Array,
        ground: Option<Gd<TerrainGround>>,
        tick: i64,
        seed: i64,
        tuning: PackedFloat64Array,
    ) -> PackedInt32Array {
        let roster = roster.bind();
        let t = tuning.as_slice();
        let routes: Vec<u32> = routes.as_slice().iter().map(|&code| code as u32).collect();
        let given = moving::pass::Given {
            routes: &routes,
            tick,
            travel_scale: t[0],
            seconds: t[1],
            route_length: t[2],
            share: share_tuning(seed, &t[3..6], &t[8..10]),
            drag: [t[6], t[7]],
        };
        let (squads, field, events) = (&mut self.squads, &mut self.field, &mut self.events);
        let joiners = on_ground(ground, |g| {
            moving::pass::step(squads, field, &roster.roster, g, &given, events)
        });
        let joins = joins::step(squads, field, &roster.roster, &joiners, seed, tick, events);
        self.edits += 1;
        joins.into_iter().flat_map(|(leader, joining)| [leader as i32, joining as i32]).collect()
    }

    /// FormationWalk.share for `roster`'s only squad on `ground` (`moving::share`), by the
    /// fight's `seed`; `tuning`: [discipline_per_leadership, walk_slack_loose,
    /// walk_slack_drilled, wounds_reach, walk_lost].
    #[func]
    fn walk_share(
        &self,
        roster: Gd<SquadRoster>,
        ground: Option<Gd<TerrainGround>>,
        seed: i64,
        tuning: PackedFloat64Array,
    ) -> f64 {
        let roster = roster.bind();
        let t = tuning.as_slice();
        let tuning = share_tuning(seed, &t[0..3], &t[3..5]);
        let listed = &roster.roster.listed[0];
        on_ground(ground, |g| moving::share::of(&self.squads, &self.field, listed, g, &tuning))
    }

    /// FormationPathing.step over the squads in `squads` (their slots, in the lists' order)
    /// on `ground` at `tick`, with [path_relook, path_look, path_detour_gain, sight_budget,
    /// ground_cliff_quarters, stamina_climb, stamina_swim]: each "detoured" event goes to the
    /// log (`pathing::pass`, `events_take`). Returns the slots that re-routed, whose routes
    /// GDScript takes afresh.
    #[func]
    fn path_step(
        &mut self,
        ground: Gd<TerrainGround>,
        squads: PackedInt32Array,
        tick: i64,
        seed: i64,
        tuning: PackedFloat64Array,
    ) -> PackedInt32Array {
        let t = tuning.as_slice();
        let climb = paths::PathTuning {
            cliff_quarters: t[4] as i64,
            stamina_climb: t[5],
            stamina_swim: t[6],
        };
        let look = pathing::LookTuning { look: t[1], detour_gain: t[2], sight_budget: t[3] };
        let looks = pathing::pass::Looks { relook: t[0], look };
        let listed: Vec<usize> = squads.as_slice().iter().map(|&s| s as usize).collect();
        let (field, events) = (&self.field, &mut self.events);
        let followed = on_paths(ground, climb, |over| {
            let given = (tick, seed);
            pathing::pass::step(&mut self.squads, field, over, &listed, given, &looks, events)
        });
        self.edits += 1;
        followed.iter().map(|&s| s as i32).collect()
    }

    /// The squad in `squad`'s living units as a look reads them, sixteen numbers each
    /// (`pathing::pass::members`; GroundPaths.group_walker and pathfinder take them).
    #[func]
    fn path_members(&self, squad: i64) -> PackedFloat64Array {
        rows_of(&pathing::pass::members(&self.field, &self.squads, squad as usize))
    }

    /// Keeps the sight of the definition coded `code` (UnitStore.definition_code):
    /// [detection_range, sight_ahead, sight_side, sight_behind].
    #[func]
    fn put_sight(&mut self, code: i64, sight: PackedFloat64Array) {
        let s = sight.as_slice();
        self.field.put_sight(code as usize, [s[0], s[1], s[2], s[3]]);
    }
}

#[godot_api(secondary)]
impl BodyField {
    /// FormationFronts.engage over `roster`'s squads on `tick`: each pick's "engaged" event
    /// goes to the log (`contact::pass`, `events_take`).
    #[func]
    fn contact_fronts(&mut self, roster: Gd<SquadRoster>, tick: i64, seed: i64) {
        let roster = roster.bind();
        let (squads, columns) = (&mut self.squads, &mut self.field.columns);
        contact::pass::engage_fronts(
            squads,
            columns,
            &roster.roster,
            (tick, seed),
            &mut self.events,
        );
        self.edits += 1;
    }

    /// FormationEdges.engage over `roster`'s squads on `tick`: each flank lock's "flanked"
    /// event and its victim's shock go to the log (`contact::pass`, `events_take`).
    /// `impacts`: [morale_side_impact, morale_rear_impact]; `bands`: morale's
    /// (`bands::BandTuning`).
    #[func]
    fn contact_edges(
        &mut self,
        roster: Gd<SquadRoster>,
        tick: i64,
        seed: i64,
        impacts: PackedInt64Array,
        bands: PackedInt64Array,
    ) {
        let roster = roster.bind();
        let impacts = contact::pass::Impacts { side: impacts[0], rear: impacts[1] };
        let bands = bands::BandTuning::of_slice(bands.as_slice());
        let (squads, columns) = (&mut self.squads, &mut self.field.columns);
        let shocks = (&impacts, &bands);
        let given = (tick, seed);
        contact::pass::engage_edges(
            squads,
            columns,
            &roster.roster,
            given,
            shocks,
            &mut self.events,
        );
        self.edits += 1;
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

    /// ScrumTurn.begin for `roster`'s only squad, about to move `travel` (+1 or -1) along
    /// the route it holds (none: a straight lane). Returns turn::KEPT, TURNED or ABOUT_FACED.
    #[func]
    fn squad_turn(&mut self, roster: Gd<SquadRoster>, travel: i64) -> i64 {
        let roster = roster.bind();
        let route = self.squads.route[roster.roster.listed[0].squad].clone();
        turn::begin(
            &mut self.squads,
            &mut self.field.columns,
            &roster.roster.listed[0],
            travel as i32,
            route.as_ref(),
        )
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
    /// unit, on the route it holds; and per living unit, in its order, 1 in `taking` if it is out taking a downed foe and its swap offset
    /// in `offsets`. `rules`: [walk_lost, group_join, cells per tile]. Returns per split, in
    /// the order they are numbered: [its candidate, its front (tiles), its unit count, then
    /// its units (indices into its squad's living) in the order they line up].
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn group_split(
        &self,
        roster: Gd<SquadRoster>,
        candidates: PackedInt32Array,
        taking: PackedByteArray,
        offsets: PackedVector2Array,
        rules: PackedFloat64Array,
        seed: i64,
    ) -> PackedFloat64Array {
        let roster = roster.bind();
        let (offsets, taking) = (v2s(&offsets), taking.as_slice());
        let (rows, _) = candidates.as_slice().as_chunks::<2>();
        let mut list = Vec::with_capacity(rows.len());
        let mut unit = 0;
        for row in rows {
            let listed = row[0] as usize;
            let living = roster.roster.listed[listed].living.len();
            let held = self.squads.route[roster.roster.listed[listed].squad].clone();
            list.push(split::Candidate {
                listed,
                waits: row[1] != 0,
                route: held.expect("a group splitting is on a route"),
                taking: taking[unit..unit + living].iter().map(|&t| t != 0).collect(),
                offsets: offsets[unit..unit + living].to_vec(),
            });
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

/// Closing up, taking in, strays and staging (spec 39 step 5b).
#[godot_api(secondary)]
impl BodyField {
    /// SquadCompact.compact for the squad in squad slot `squad`: its living units five
    /// numbers each, in its order ([id, store slot, footprint width and depth, band]), round
    /// the moves it holds under way. Returns the indices into `living` of those that stepped
    /// up, in the order each first did.
    #[func]
    fn squad_compact(&mut self, squad: i64, living: PackedInt64Array) -> PackedInt32Array {
        let (living, movers) = (closers(&living), swaps::moving(&self.squads, squad as usize));
        let moved = compact::compact(
            &mut self.squads,
            &mut self.field.columns,
            squad as usize,
            &living,
            &movers,
        );
        PackedInt32Array::from_iter(moved.into_iter().map(|n| n as i32))
    }

    /// FormationShuffle.step's swaps under way for the squad in squad slot `squad`, as it
    /// holds them (`swaps::end_held`): `living` its living units as squad_compact takes them,
    /// `units` the store slots of all its units. Returns, for each swap that ended in the
    /// order it did: its mover's id, how many units it passed and their ids, then how many
    /// stepped up after it and each one's index into `living` and rank.
    #[func]
    fn squad_swaps_end(
        &mut self,
        squad: i64,
        living: PackedInt64Array,
        units: PackedInt32Array,
    ) -> PackedInt64Array {
        let units: Vec<usize> = units.as_slice().iter().map(|&slot| slot as usize).collect();
        let (squads, columns) = (&mut self.squads, &mut self.field.columns);
        let ended = swaps::end_held(squads, columns, squad as usize, &closers(&living), &units);
        let mut out: Vec<i64> = Vec::new();
        for one in ended {
            out.extend_from_slice(&[one.mover, one.passed.len() as i64]);
            out.extend_from_slice(&one.passed);
            out.push(one.stepped.len() as i64);
            for (n, rank) in one.stepped {
                out.extend_from_slice(&[n as i64, rank as i64]);
            }
        }
        PackedInt64Array::from(&out[..])
    }

    /// FormationShuffle._start_moves for the squad in squad slot `squad`: the moves `shape`
    /// [width, combat width, centre shift, fighting] can start round the moves it holds, its
    /// living units given by [id, store slot] in `living` and seven numbers each in `units`
    /// (rank, column, footprint width and depth, band, priority, joined) at `speeds`, at
    /// `pace` = [tick seconds, cells a second at speed 1, scrum_crowding]. It holds each move
    /// it starts (`swaps::begin`). True if any started.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn squad_start_moves(
        &mut self,
        squad: i64,
        living: PackedInt64Array,
        shape: PackedFloat64Array,
        units: PackedInt32Array,
        speeds: PackedFloat64Array,
        pace: PackedFloat64Array,
    ) -> bool {
        let (pairs, _) = living.as_slice().as_chunks::<2>();
        let living: Vec<(i64, usize)> = pairs.iter().map(|p| (p[0], p[1] as usize)).collect();
        let slots: Vec<usize> = living.iter().map(|&(_, slot)| slot).collect();
        let (claimed, busy) = swaps::claims(&self.squads, squad as usize, &slots);
        let ranked = ranked_of(&units, &speeds);
        let started = march::shuffle::start_moves(
            &shuffle_squad(&shape),
            &ranked,
            &claimed,
            &busy,
            pace_of(&pace),
        );
        let places: Vec<(i32, i32)> = ranked.iter().map(|r| r.place).collect();
        let feet: Vec<(i32, i32)> =
            ranked.iter().map(|r| (r.footprint_width, r.footprint_depth)).collect();
        swaps::begin(&mut self.squads, squad as usize, &started, &living, &places, &feet);
        !started.is_empty()
    }

    /// How far each unit in the squad in `squad`'s held swaps is through its move, three
    /// numbers a unit: its id, ranks forward, columns across (`swaps::offsets`).
    #[func]
    fn squad_swap_offsets(&self, squad: i64) -> PackedFloat64Array {
        let flat: Vec<f64> = swaps::offsets(&self.squads, squad as usize)
            .into_iter()
            .flat_map(|(id, ranks, across)| [id as f64, f64::from(ranks), f64::from(across)])
            .collect();
        PackedFloat64Array::from(&flat[..])
    }

    /// The squad in `squad`'s units out taking the downed, as FormationTaking reads them:
    /// {taker's id: its body's store slot}, in the order they set out.
    #[func]
    fn squad_taking(&self, squad: i64) -> VarDictionary {
        let mut out = VarDictionary::new();
        for &(taker, body) in &self.squads.taking[squad as usize] {
            out.set(taker, body as i64);
        }
        out
    }

    /// The squad in `squad`'s units out taking, as `squad_taking` reads them, replacing any.
    #[func]
    fn put_squad_taking(&mut self, squad: i64, taking: VarDictionary) {
        self.squads.taking[squad as usize] =
            taking.iter_shared().map(|(k, v)| (k.to::<i64>(), v.to::<i64>() as usize)).collect();
    }

    /// The squad in `squad`'s faction (SkirmishSquad.faction_id), coded in the lexicon a
    /// unit's is, so the two compare.
    #[func]
    fn squad_put_faction(&mut self, squad: i64, faction: GString) {
        self.squads.faction[squad as usize] = self.field.lexicon.code(&faction.to_string());
    }

    /// The squad in `squad`'s faction, as `squad_put_faction` set it.
    #[func]
    fn squad_faction(&self, squad: i64) -> GString {
        GString::from(self.field.lexicon.word(self.squads.faction[squad as usize]))
    }

    /// The units that joined the squad in `squad` as reinforcements: their ids, in the order
    /// they joined, replacing any.
    #[func]
    fn put_squad_joined(&mut self, squad: i64, ids: PackedInt64Array) {
        self.squads.joined[squad as usize] = ids.as_slice().to_vec();
    }

    /// The units that joined the squad in `squad`, as `put_squad_joined` set them.
    #[func]
    fn squad_joined(&self, squad: i64) -> PackedInt64Array {
        PackedInt64Array::from(self.squads.joined[squad as usize].as_slice())
    }

    /// The squad in `squad`'s standing units, by unit-store slot in its members' order,
    /// read from its members and the STATE column (UnitState.living).
    #[func]
    fn squad_living(&self, squad: i64) -> PackedInt32Array {
        let states = &self.field.columns.state;
        PackedInt32Array::from(self.squads.living(squad as usize, states).as_slice())
    }

    /// The squad in `squad`'s downed units, DOWNED or CARRIED, as `squad_living` reads them.
    #[func]
    fn squad_downed(&self, squad: i64) -> PackedInt32Array {
        let states = &self.field.columns.state;
        PackedInt32Array::from(self.squads.downed(squad as usize, states).as_slice())
    }

    /// Whether the squad in `squad` holds any swap under way.
    #[func]
    fn squad_swapping(&self, squad: i64) -> bool {
        !self.squads.swaps[squad as usize].is_empty()
    }

    /// The squad in `squad`'s swaps under way are cancelled.
    #[func]
    fn squad_clear_swaps(&mut self, squad: i64) {
        self.squads.swaps[squad as usize].clear();
    }

    /// The squad in `squad`'s swaps under way as GDScript reads them: each {"ticks", "total",
    /// "mover" (its id; 0: none), "to" and "from" (unit id -> Vector2i(rank, column)), "foot"
    /// (unit id -> Vector2i(width, depth)), "passed" (ids)}.
    #[func]
    fn squad_swaps(&self, squad: i64) -> VarArray {
        let mut out = VarArray::new();
        for held in &self.squads.swaps[squad as usize] {
            let mut swap = VarDictionary::new();
            let (mut to, mut from, mut foot) =
                (VarDictionary::new(), VarDictionary::new(), VarDictionary::new());
            for u in &held.units {
                to.set(u.id, Vector2i::new(u.to.0, u.to.1));
                from.set(u.id, Vector2i::new(u.from.0, u.from.1));
                foot.set(u.id, Vector2i::new(u.foot.0, u.foot.1));
            }
            swap.set("ticks", held.ticks);
            swap.set("total", held.total);
            swap.set("mover", held.units.first().map_or(0, |u| u.id));
            swap.set("to", &to.to_variant());
            swap.set("from", &from.to_variant());
            swap.set("foot", &foot.to_variant());
            swap.set("passed", &PackedInt64Array::from(&held.passed[..]).to_variant());
            out.push(&swap.to_variant());
        }
        out
    }

    /// The squad in `squad` holds `swaps` under way, as `squad_swaps` reads them (a unit
    /// with no "foot" one cell), replacing any it held.
    #[func]
    fn put_squad_swaps(&mut self, squad: i64, swaps: VarArray) {
        let cell = |map: &VarDictionary, id: i64, or: Vector2i| {
            map.get(id).map_or(or, |v| v.to::<Vector2i>())
        };
        let mut held = Vec::new();
        for swap in swaps.iter_shared() {
            let swap = swap.to::<VarDictionary>();
            let map = |key: &str| swap.get(key).map_or_else(VarDictionary::new, |v| v.to());
            let (to, from, foot) = (map("to"), map("from"), map("foot"));
            let mut units = Vec::new();
            for key in to.keys_array().iter_shared() {
                let id = key.to::<i64>();
                let (t, f) = (cell(&to, id, Vector2i::ZERO), cell(&from, id, Vector2i::ZERO));
                let size = cell(&foot, id, Vector2i::new(1, 1));
                units.push(swaps::HeldUnit {
                    id,
                    slot: self.field.slot_of(id).expect("a unit in the store") as usize,
                    to: (t.x, t.y),
                    from: (f.x, f.y),
                    foot: (size.x, size.y),
                });
            }
            let number = |key: &str| swap.get(key).map_or(0, |v| v.to::<i64>());
            let passed = swap
                .get("passed")
                .map_or_else(Vec::new, |v| v.to::<PackedInt64Array>().as_slice().to_vec());
            held.push(swaps::Held {
                ticks: number("ticks"),
                total: number("total"),
                units,
                passed,
            });
        }
        self.squads.swaps[squad as usize] = held;
    }

    /// FormationContact.reinforce: the squad in squad slot `joining` taken in behind the
    /// squad in `leader` (`joins::take_in`), each read where it lives; its living units are
    /// enlisted in the leader. Returns the indices into the leader's living units then the
    /// joining's (each in its order) of those that stepped up.
    #[func]
    fn squad_take_in(&mut self, leader: i64, joining: i64) -> PackedInt32Array {
        let took =
            joins::take_in(&mut self.squads, &mut self.field, leader as usize, joining as usize);
        self.edits += 1;
        PackedInt32Array::from_iter(took.stepped.into_iter().map(|n| n as i32))
    }

    /// FormationStrays.adopt as one pass (`strays::adopt`): each stray (two numbers in
    /// `strays`: its squad's slot, its unit's store slot) taken out of its squad and set off
    /// home alone as a routing squad the crate opens, numbered from the ids the battle issues
    /// in their draws' order under `seed`. Returns two numbers a stray, in the order
    /// numbered: [its index in `strays`, the squad slot opened for it].
    #[func]
    fn strays_adopt(&mut self, strays: PackedInt32Array, seed: i64) -> PackedInt32Array {
        let pairs: Vec<(usize, usize)> =
            strays.as_slice().chunks_exact(2).map(|p| (p[0] as usize, p[1] as usize)).collect();
        let opened = strays::adopt(&mut self.squads, &mut self.field, &pairs, seed);
        self.edits += 1;
        PackedInt32Array::from_iter(opened.into_iter().flat_map(|(n, s)| [n as i32, s as i32]))
    }

    /// The hold-until order the squad in `squad` holds, as FormationStaging reads it: {"at",
    /// "trigger", "partner", "fallback", "then", and "meet" and "meet_cells" if it has a
    /// meeting point}; empty if none.
    #[func]
    fn squad_staging(&self, squad: i64) -> VarDictionary {
        let mut out = VarDictionary::new();
        if let Some(order) = &self.squads.staging[squad as usize] {
            out.set("at", order.at);
            out.set("trigger", if order.sees_fight { "sees_fight" } else { "sees_partner" });
            out.set("partner", order.partner);
            out.set("fallback", order.fallback);
            if let Some(then) = &order.then {
                out.set("then", then.as_str());
            }
            if let Some((at, cells)) = order.meet {
                out.set("meet", Vector2::new(at.x, at.y));
                out.set("meet_cells", cells);
            }
        }
        out
    }

    /// The squad in `squad` holds the hold-until `order` (as `squad_staging` reads it: any
    /// trigger but "sees_partner" waits to see a fight; no partner is 0); empty: none.
    #[func]
    fn put_squad_staging(&mut self, squad: i64, order: VarDictionary) {
        let number = |key: &str| order.get(key).map_or(0.0, |v| variant_number(&v));
        self.squads.staging[squad as usize] = (!order.is_empty()).then(|| staging::Stage {
            at: number("at"),
            sees_fight: order.get("trigger").is_none_or(|t| t.to_string() != "sees_partner"),
            partner: number("partner") as i64,
            fallback: number("fallback") as i64,
            then: order.get("then").map(|t| t.to_string()),
            meet: order.get("meet").map(|m| {
                let at = m.to::<Vector2>();
                (V2::new(at.x, at.y), number("meet_cells"))
            }),
        });
    }

    /// GroundBodies.drag for `roster`'s squad `listed` (index) over the bodies lying at
    /// `lying_ats` ([radius, footprint area] each in `lying_sizes`), `weights` =
    /// [bodies_ground_drag, bodies_ground_block].
    #[func]
    fn ground_drag(
        &self,
        roster: Gd<SquadRoster>,
        listed: i64,
        lying_ats: PackedVector2Array,
        lying_sizes: PackedFloat64Array,
        weights: PackedFloat64Array,
    ) -> f64 {
        let roster = roster.bind();
        let ats = v2s(&lying_ats);
        let (sizes, _) = lying_sizes.as_slice().as_chunks::<2>();
        let radii: Vec<f64> = sizes.iter().map(|s| s[0]).collect();
        let areas: Vec<f64> = sizes.iter().map(|s| s[1]).collect();
        let lying = lying::Lying::new(&ats, &radii, &areas);
        let w = weights.as_slice();
        let squad = &roster.roster.listed[listed as usize];
        lying.drag(&self.squads, &self.field.columns, squad, w[0], w[1])
    }
}

/// A squad's living units as squad_compact is given them, five numbers each.
fn closers(living: &PackedInt64Array) -> Vec<compact::Closer> {
    let (rows, _) = living.as_slice().as_chunks::<5>();
    rows.iter()
        .map(|r| compact::Closer {
            id: r[0],
            slot: r[1] as usize,
            foot: (r[2] as i32, r[3] as i32),
            preferred: r[4],
        })
        .collect()
}

/// Stamina (spec 39 step 6a).
#[godot_api(secondary)]
impl BodyField {
    /// FormationStamina.step: one tick of `seconds`. `squads`: two numbers a squad, its squad
    /// slot and its flags (1 pursuing, 2 on its way back, 4 hurried); `counts`: its living
    /// units' count; `ints`: four numbers a unit, its id, store slot, load stage and flags (1
    /// leads, 2 "pursues_at_march", 4 "retreats_at_march"); `reals`: six a unit, its fresh
    /// speed, most stamina, tiring, run pace, wits and constitution. `tuning`: stamina_run,
    /// stamina_delay, stamina_recovery, run_build_seconds, run_reaction,
    /// condition_cap, stamina_tired, stamina_spent, the three stamina_pace, then
    /// load_march_tiring.
    #[func]
    fn stamina_step(
        &mut self,
        squads: PackedInt64Array,
        counts: PackedInt32Array,
        ints: PackedInt64Array,
        reals: PackedFloat64Array,
        seconds: f64,
        tuning: PackedFloat64Array,
    ) {
        let listed = paced(&squads, &counts, &ints, &reals);
        let t = tuning.as_slice();
        let tuning = stamina::StaminaTuning {
            run: t[0],
            delay: t[1],
            recovery: t[2],
            run_build_seconds: t[3],
            run_reaction: t[4],
            condition_cap: t[5],
            tired: t[6],
            spent: t[7],
            pace: t[8..11].to_vec(),
            march_tiring: t[11..].to_vec(),
        };
        stamina::step(&self.squads, &mut self.field.columns, &listed, seconds, &tuning);
    }

    /// FormationStamina.squad_gives_up: its living units' store `slots` and most stamina.
    #[func]
    fn stamina_gives_up(
        &self,
        slots: PackedInt32Array,
        most: PackedFloat64Array,
        give_up: f64,
    ) -> bool {
        let living: Vec<(usize, f64)> = slots
            .as_slice()
            .iter()
            .zip(most.as_slice())
            .map(|(&slot, &most)| (slot as usize, most))
            .collect();
        stamina::squad_gives_up(&self.field.columns, &living, give_up)
    }
}

/// The squads as stamina_step is given them.
fn paced(
    squads: &PackedInt64Array,
    counts: &PackedInt32Array,
    ints: &PackedInt64Array,
    reals: &PackedFloat64Array,
) -> Vec<stamina::Paced> {
    let (squad_rows, _) = squads.as_slice().as_chunks::<2>();
    let (int_rows, _) = ints.as_slice().as_chunks::<4>();
    let (real_rows, _) = reals.as_slice().as_chunks::<6>();
    let mut units = int_rows.iter().zip(real_rows).map(|(i, r)| stamina::Gaited {
        id: i[0],
        slot: i[1] as usize,
        load_stage: i[2],
        leads: i[3] & 1 != 0,
        pursues_at_march: i[3] & 2 != 0,
        retreats_at_march: i[3] & 4 != 0,
        fresh_speed: r[0],
        max_stamina: r[1],
        tiring: r[2],
        run_pace: r[3],
        wits: r[4],
        constitution: r[5],
    });
    squad_rows
        .iter()
        .zip(counts.as_slice())
        .map(|(s, &count)| stamina::Paced {
            squad: s[0] as usize,
            pursuing: s[1] & 1 != 0,
            returning: s[1] & 2 != 0,
            hurry: s[1] & 4 != 0,
            living: units.by_ref().take(count as usize).collect(),
        })
        .collect()
}

/// The downed: the fallen, the takers, surrender and recovery (spec 39 step 6b).
#[godot_api(secondary)]
impl BodyField {
    /// FormationDeaths.bury over the squads in squad slots `listed`, in the lists' order
    /// (`deaths::bury::bury`): the fell queue taken, each fallen unit still standing at 0 hp
    /// or below dying or downed with the squad it is in now, its state and wounds written,
    /// "died" or "downed", "leader_fell", "morale_band", "stepped_up" and "destroyed" in the
    /// log, the losses' shock, the ranks closed up and the squads left with no one destroyed
    /// and released. `bands` as BandTuning.of_slice reads it. Returns three numbers a unit
    /// buried, in the order buried: [its squad's index in `listed`, its id, the wounds it
    /// bears now if downed, else -1].
    #[func]
    fn deaths_bury(
        &mut self,
        listed: PackedInt32Array,
        bands: PackedInt64Array,
        tick: i64,
    ) -> PackedInt64Array {
        let listed: Vec<usize> = listed.as_slice().iter().map(|&s| s as usize).collect();
        let t = bands::BandTuning::of_slice(bands.as_slice());
        let (squads, field, log) = (&mut self.squads, &mut self.field, &mut self.events);
        let buried = deaths::bury::bury(squads, field, &listed, (&t, tick), log);
        self.edits += 1;
        let flat = buried.iter().flat_map(|b| [b.listed as i64, b.id, b.wounded.unwrap_or(-1)]);
        PackedInt64Array::from_iter(flat)
    }

    /// FormationWounds.tend: the wounds' tending as one pass (`wounds::pass::step`): the
    /// listed squads' living units and bodies lying read in the crate, the takings, the
    /// bodies taken captured, sent home (the leader's messenger spent, the body's side
    /// shocked) or finished, their states written and "captured", "sent_home", "finished" and
    /// "morale_band" in the log. `listed`: the squads' slots, in the lists' order; `at`: [tick,
    /// ticks a second, seed, wounds_messenger_shock]; `tuning`: as `wounds_tuning`; `bands`:
    /// as BandTuning.of_slice reads it. Returns five numbers a body taken, in the order
    /// landed: [its squad's index in `listed`, its id, its state now, then for one sent home
    /// its leader's squad's index and id, else -1, -1].
    #[func]
    fn wounds_step(
        &mut self,
        listed: PackedInt32Array,
        at: PackedInt64Array,
        tuning: PackedFloat64Array,
        bands: PackedInt64Array,
    ) -> PackedInt64Array {
        let listed: Vec<usize> = listed.as_slice().iter().map(|&s| s as usize).collect();
        let &[tick, interval, seed, shock, ..] = at.as_slice() else {
            return PackedInt64Array::new();
        };
        let wounds = wounds::WoundsTuning::of_slice(tuning.as_slice());
        let bands = bands::BandTuning::of_slice(bands.as_slice());
        let t = wounds::pass::Tuning { wounds: &wounds, shock, bands: &bands };
        let (squads, field, log) = (&mut self.squads, &mut self.field, &mut self.events);
        let landed = wounds::pass::step(squads, field, &listed, (tick, interval, seed), &t, log);
        self.edits += 1;
        let flat = landed.iter().flat_map(|l| {
            let (n, id) = l.leader.map_or((-1, -1), |(n, id)| (n as i64, id));
            [l.listed as i64, l.id, i64::from(l.state), n, id]
        });
        PackedInt64Array::from_iter(flat)
    }

    /// FormationRecovery.step: what the wounded come to as one pass (`recovery::pass::step`):
    /// the listed squads' living and downed read in the crate, recovery run over one tick of
    /// `seconds`, and what it decided landed - those dead of their wounds, those playing dead,
    /// those coming to joining a formation or setting off alone - with "died_of_wounds",
    /// "playing_dead" and "came_to" in the log. `listed`: the squads' slots, in the lists'
    /// order; `at`: [tick, seed]; `tuning`: as `wounds_tuning`. Returns four numbers a unit
    /// something befell, in the order landed: [its squad's index in `listed`, its id, what (0
    /// died of its wounds, 1 began to play dead, 2 came to), the index of the squad it joined,
    /// or -1].
    #[func]
    fn recovery_step(
        &mut self,
        listed: PackedInt32Array,
        at: PackedInt64Array,
        seconds: f64,
        tuning: PackedFloat64Array,
    ) -> PackedInt64Array {
        let listed: Vec<usize> = listed.as_slice().iter().map(|&s| s as usize).collect();
        let &[tick, seed, ..] = at.as_slice() else {
            return PackedInt64Array::new();
        };
        let tuning = wounds::WoundsTuning::of_slice(tuning.as_slice());
        let (squads, field, log) = (&mut self.squads, &mut self.field, &mut self.events);
        let landed =
            recovery::pass::step(squads, field, &listed, (tick, seconds, seed), &tuning, log);
        self.edits += 1;
        let flat = landed.iter().flat_map(|l| {
            let (what, joins) = match l.came {
                recovery::pass::Came::DiedOfWounds => (0, -1),
                recovery::pass::Came::PlayingDead => (1, -1),
                recovery::pass::Came::Alone => (2, -1),
                recovery::pass::Came::Joined(n) => (2, n as i64),
            };
            [l.listed as i64, l.id, what, joins]
        });
        PackedInt64Array::from_iter(flat)
    }

    /// FormationCarry's pick-up: who picks up which downed friend. `squads`: four numbers a
    /// squad in the lists' order, its faction (a code), whether it tends its downed now (1),
    /// how many living units follow and then how many downed bodies lying; `ints`: six a unit,
    /// its id, store slot, faction, then for one living whether it bears a body (1), its
    /// strength and its "hauler", and for a body the cells it covers, 0, 0; `gear`: one a
    /// unit, its gear's weight (0 for a body). `tuning`: as `carry_tuning`. Returns four
    /// numbers a body picked up, in order: [its squad, its index in that squad's downed, the
    /// bearer's squad, its index in that squad's living].
    #[func]
    fn carry_pick_up(
        &self,
        squads: PackedInt32Array,
        ints: PackedInt64Array,
        gear: PackedFloat64Array,
        seed: i64,
        tuning: PackedFloat64Array,
    ) -> PackedInt64Array {
        let squads = carrying(&squads, &ints, &gear);
        let tuning = carry::CarryTuning::of_slice(tuning.as_slice());
        let picked = carry::pick_up(&self.field.columns, &squads, seed, &tuning);
        let out: Vec<i64> = picked
            .iter()
            .flat_map(|&(body, bearer)| [body.0, body.1, bearer.0, bearer.1].map(|n| n as i64))
            .collect();
        PackedInt64Array::from(out.as_slice())
    }

    /// FormationTaking.step: taking the downed as one pass (`taking::pass::step`): the
    /// takers whose work is done called back, the hold run out, then free units sent out to
    /// the downed foes, "sent_to_take" in the log. `listed`: the squads' slots, in the lists'
    /// order; `threatened`: 1 a listed squad that sees a standing foe (FormationPerception),
    /// else 0; `tuning`: [take_hold, take_reach].
    #[func]
    fn taking_step(
        &mut self,
        listed: PackedInt32Array,
        threatened: PackedInt32Array,
        tick: i64,
        seconds: f64,
        seed: i64,
        tuning: PackedFloat64Array,
    ) {
        let listed: Vec<usize> = listed.as_slice().iter().map(|&s| s as usize).collect();
        let threatened: Vec<bool> = threatened.as_slice().iter().map(|&t| t != 0).collect();
        let &[hold, reach, ..] = tuning.as_slice() else { return };
        let t = taking::pass::TakingTuning { hold, reach };
        let (squads, field, log) = (&mut self.squads, &self.field, &mut self.events);
        let lists = (&listed[..], &threatened[..]);
        taking::pass::step(squads, field, lists, (tick, seconds, seed), &t, log);
        self.edits += 1;
    }
}

/// The squads as carry_pick_up is given them.
fn carrying(
    squads: &PackedInt32Array,
    ints: &PackedInt64Array,
    gear: &PackedFloat64Array,
) -> Vec<carry::Carrying> {
    let (rows, _) = ints.as_slice().as_chunks::<6>();
    let mut units = rows.iter().zip(gear.as_slice());
    let (squad_rows, _) = squads.as_slice().as_chunks::<4>();
    squad_rows
        .iter()
        .map(|s| carry::Carrying {
            faction: s[0] as u32,
            tending: s[1] != 0,
            living: units
                .by_ref()
                .take(s[2] as usize)
                .map(|(i, &gear_weight)| carry::Bearer {
                    id: i[0],
                    slot: i[1] as usize,
                    faction: i[2] as u32,
                    carrying: i[3] != 0,
                    gear_weight,
                    strength: i[4],
                    hauler: i[5],
                })
                .collect(),
            downed: units
                .by_ref()
                .take(s[3] as usize)
                .map(|(i, _)| carry::Body {
                    id: i[0],
                    slot: i[1] as usize,
                    faction: i[2] as u32,
                    cells: i[3],
                })
                .collect(),
        })
        .collect()
}

/// Morale and the blows on routers (spec 39 step 7a).
#[godot_api(secondary)]
impl BodyField {
    /// FormationMorale.step's second as one pass (`morale::pass`, spec 39 step 8b5-xxvii) for
    /// `roster`'s squads on `tick`: each one's morale, leadership and ceiling read here, its
    /// morale set and a change of band logged. `tuning`: [morale_recovery,
    /// morale_support_cells, condition_cap, the morale_pressure...]; `bands`: BandTuning's.
    #[func]
    fn morale_step(
        &mut self,
        roster: Gd<SquadRoster>,
        tuning: PackedFloat64Array,
        bands: PackedInt64Array,
        tick: i64,
    ) {
        let roster = roster.bind();
        let tuning = morale::MoraleTuning::of_slice(tuning.as_slice());
        let bands = bands::BandTuning::of_slice(bands.as_slice());
        let (squads, columns, log) = (&mut self.squads, &self.field.columns, &mut self.events);
        morale::pass::step((squads, columns), &roster.roster, (&tuning, &bands), tick, log);
        self.edits += 1;
    }
}

/// Routing formations as RoutBlows and FormationRout hand them in: each one's index in
/// `roster`, on the route it holds.
fn routings(
    squads: &Squads,
    roster: &roster::Roster,
    routing: &PackedInt32Array,
) -> Vec<rout_blows::Routing> {
    routing
        .as_slice()
        .iter()
        .map(|&listed| rout_blows::Routing {
            listed: listed as usize,
            route: squads.route[roster.listed[listed as usize].squad].clone(),
        })
        .collect()
}

/// The rout (spec 39 step 7b). `routing` as `routings` reads it; `leaders`
/// every listed squad's living units' leadership, in the roster's order; `tuning` as
/// RoutTuning.of_slice reads it.
#[godot_api(secondary)]
impl BodyField {
    /// FormationRout's breaking and flight. Returns how many of the roster break, their
    /// indices in it, then six numbers a thing the flight did, in order: its kind (0 a
    /// crush, 1 home, 2 scattered), the formation's index in `routing`, the router's in its
    /// living, and for a crush the friend's index in the roster, its unit's in its living
    /// and 1 if it panics that friend (else 0, 0, 0).
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn rout_flee(
        &mut self,
        roster: Gd<SquadRoster>,
        routing: PackedInt32Array,
        leaders: PackedInt32Array,
        tuning: PackedFloat64Array,
        seed: i64,
        ground: Option<Gd<TerrainGround>>,
    ) -> PackedInt64Array {
        let roster = roster.bind();
        let routing = routings(&self.squads, &roster.roster, &routing);
        let leaders: Vec<i64> = leaders.as_slice().iter().map(|&l| l as i64).collect();
        let tuning = rout::RoutTuning::of_slice(tuning.as_slice());
        let breaking = rout::breaking(&self.squads, &roster.roster);
        let (squads, columns) = (&mut self.squads, &mut self.field.columns);
        let fled = on_ground(ground, |g| {
            rout::flee(squads, columns, &roster.roster, &routing, &leaders, &tuning, seed, g)
        });
        self.edits += 1;
        let mut flat: Vec<i64> = vec![breaking.len() as i64];
        flat.extend(breaking.iter().map(|&n| n as i64));
        for f in fled {
            flat.extend(match f {
                rout::Fled::Crushed { routing, member, hit, panic } => {
                    [0, routing as i64, member as i64, hit.0 as i64, hit.1 as i64, panic as i64]
                }
                rout::Fled::Home { routing, member } => [1, routing as i64, member as i64, 0, 0, 0],
                rout::Fled::Scattered { routing } => [2, routing as i64, 0, 0, 0, 0],
            });
        }
        PackedInt64Array::from(flat.as_slice())
    }

    /// FormationRout's rallies. `morale`: two numbers a listed squad, its morale (below 0: not
    /// read yet) and then its ceiling (else 0). Returns how many squads' morale was first
    /// read, their indices in the roster, then three numbers a join, in the order they join:
    /// the formation's index in `routing`, the router's in its living, and the friend's index
    /// in the roster.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn rout_rallies(
        &mut self,
        roster: Gd<SquadRoster>,
        routing: PackedInt32Array,
        leaders: PackedInt32Array,
        morale: PackedInt64Array,
        tuning: PackedFloat64Array,
        seed: i64,
    ) -> PackedInt64Array {
        let roster = roster.bind();
        let routing = routings(&self.squads, &roster.roster, &routing);
        let leaders: Vec<i64> = leaders.as_slice().iter().map(|&l| l as i64).collect();
        let (rows, _) = morale.as_slice().as_chunks::<2>();
        let morale: Vec<(i64, i64)> = rows.iter().map(|m| (m[0], m[1])).collect();
        let tuning = rout::RoutTuning::of_slice(tuning.as_slice());
        let columns = &mut self.field.columns;
        let rallied = rout::rallies(
            &self.squads,
            columns,
            &roster.roster,
            &routing,
            &leaders,
            &morale,
            &tuning,
            seed,
        );
        let mut flat: Vec<i64> = vec![rallied.first_read.len() as i64];
        flat.extend(rallied.first_read.iter().map(|&n| n as i64));
        flat.extend(rallied.joins.iter().flat_map(|&(r, u, f)| [r, u, f].map(|n| n as i64)));
        PackedInt64Array::from(flat.as_slice())
    }

    /// FormationRout's re-forming. Returns the indices in `routing` of the formations that
    /// re-formed, in order.
    #[func]
    fn rout_regroup(
        &mut self,
        roster: Gd<SquadRoster>,
        routing: PackedInt32Array,
        leaders: PackedInt32Array,
        tuning: PackedFloat64Array,
        seed: i64,
    ) -> PackedInt64Array {
        let roster = roster.bind();
        let routing = routings(&self.squads, &roster.roster, &routing);
        let leaders: Vec<i64> = leaders.as_slice().iter().map(|&l| l as i64).collect();
        let tuning = rout::RoutTuning::of_slice(tuning.as_slice());
        let mut slots = std::collections::BTreeMap::new();
        for formation in &routing {
            for &id in &self.squads.fleeing[roster.roster.listed[formation.listed].squad] {
                slots.insert(id, self.field.slot_of(id).map(|s| s as usize));
            }
        }
        let (squads, columns) = (&mut self.squads, &mut self.field.columns);
        let slot_of = |id: i64| slots.get(&id).copied().flatten();
        let out = rout::regroup(
            squads,
            columns,
            &roster.roster,
            &routing,
            &leaders,
            &tuning,
            seed,
            slot_of,
        );
        self.edits += 1;
        let flat: Vec<i64> = out.iter().map(|&r| r as i64).collect();
        PackedInt64Array::from(flat.as_slice())
    }

    /// FormationRout's breaks, of the formations in `breaking` (as `routings` reads them).
    /// Returns, for each, how many friends saw it break and their indices in the roster.
    #[func]
    fn rout_break(
        &mut self,
        roster: Gd<SquadRoster>,
        breaking: PackedInt32Array,
        ground: Option<Gd<TerrainGround>>,
    ) -> PackedInt64Array {
        let roster = roster.bind();
        let breaking = routings(&self.squads, &roster.roster, &breaking);
        let (squads, columns) = (&mut self.squads, &mut self.field.columns);
        let seen =
            on_ground(ground, |g| rout::breaks(squads, columns, &roster.roster, &breaking, g));
        self.edits += 1;
        let mut flat: Vec<i64> = Vec::new();
        for friends in seen {
            flat.push(friends.len() as i64);
            flat.extend(friends.iter().map(|&n| n as i64));
        }
        PackedInt64Array::from(flat.as_slice())
    }
}

/// The fight as one pass (spec 39 steps 8a1 and 8b5-xix).
#[godot_api(secondary)]
impl BodyField {
    /// FormationSimulation._fight over `roster`'s squads (`fight::step::step`): every living
    /// unit's target cleared, then the blows and shots chosen (`fight::choose`: the field
    /// synced to the scrum if some squad may strike), each striker's target and cooldown set;
    /// each landed (`fight::land`: each unit's arms from its sheet and words, with rolls its
    /// band and the foes pressing each target), the hurts (`hurts::land`, the fallen on the
    /// fell queue), "spat" and "hit" in the log, each striker's stamina spent, then the
    /// surrender (`fight::surrender`:
    /// "surrendered" and "destroyed" in the log). `ints` = [the ticks a second's strike
    /// takes, seed, tick, 1 if blows roll]; `floats` = [reach_contact, the front's least dot,
    /// rout_strike_reach, stamina_blow, wounds_surrender]; `bands` as BandTuning.of_slice
    /// reads it, `tuning` as LandingTuning.of_slice. Returns two numbers a unit the surrender
    /// took, in the order they gave up: [its squad's index in the roster, its index in that
    /// one's living].
    #[func]
    fn fight_step(
        &mut self,
        roster: Gd<SquadRoster>,
        ints: PackedInt64Array,
        floats: PackedFloat64Array,
        bands: PackedInt64Array,
        tuning: PackedFloat64Array,
        ground: Option<Gd<TerrainGround>>,
    ) -> PackedInt64Array {
        let (i, f) = (ints.as_slice(), floats.as_slice());
        let bands = bands::BandTuning::of_slice(bands.as_slice());
        let rules = fight::step::Rules {
            choose: fight::Rules {
                interval: i[0],
                seed: i[1],
                reach: f[0],
                front: f[1],
                rout_reach: f[2],
                bands,
            },
            land: fight::land::Rules {
                tuning: landing::LandingTuning::of_slice(tuning.as_slice()),
                rolls: (i[3] != 0).then_some((i[1], i[2])),
                reach: f[0],
                bands,
            },
            tick: i[2],
            blow: f[3],
            surrender: f[4],
        };
        let roster = roster.bind();
        let (squads, field, log) = (&mut self.squads, &mut self.field, &mut self.events);
        let fought =
            on_ground(ground, |g| fight::step::step(squads, field, &roster.roster, &rules, g, log));
        self.edits += 1;
        let out: Vec<i64> = fought.taken.iter().flat_map(|&(n, m)| [n as i64, m as i64]).collect();
        PackedInt64Array::from(out.as_slice())
    }
}
