//! tick_phases' `counts=1` in the crate (spec 38 step 2, built in spec 39 step 8c-xxii): for
//! each pass of the tick, how many of the unit store's slots it changed, against the living.
//! The store's columns are copied at each lap (`laps::watched`), and a slot counts as changed
//! by the pass that just ended when any of its columns differs from the copy taken as the pass
//! began. The living are the units the store holds whose state is before DEAD, as the pass
//! began. This is the measurement for Q24 (#279): a pass that visits every unit but changes
//! few is a candidate for a work list fed by events.
//!
//! The copying is only here, under `counts=1`: it reads the field and writes nothing the
//! simulation reads, so a counted battle ends where an uncounted one does, and the bin counts
//! its untimed twin, so the timed battle is timed exactly as without it.

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::rc::Rc;

use super::Settled;
use crate::columns::Columns;
use crate::content::Content;
use crate::field::Field;
use crate::laps;
use crate::squads::DEAD;

/// One pass's counts, summed over the ticks counted: the laps it ended, the slots it changed
/// and the living as each of its laps began.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Count {
    pub phase: &'static str,
    pub laps: usize,
    pub changed: usize,
    pub living: usize,
}

/// A battle's counted ticks: how many, and each pass's counts in the order first lapped.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Counts {
    pub ticks: usize,
    pub passes: Vec<Count>,
}

/// `ticks` ticks of `settled`, each pass's changed slots and living counted.
pub fn counted(settled: &mut Settled, ticks: usize, content: &Content) -> Counts {
    let watch = Rc::new(RefCell::new(Watch::of(&settled.field)));
    let watcher = Rc::clone(&watch);
    let seen: laps::Watcher = Box::new(move |phase, field| watcher.borrow_mut().saw(phase, field));
    laps::watched(seen, || {
        for _ in 0..ticks {
            settled.step(content);
        }
    });
    let passes = std::mem::take(&mut watch.borrow_mut().passes);
    Counts { ticks, passes }
}

/// The copy of the columns as the current pass began, the living then, and the counts so far.
struct Watch {
    last: Columns,
    living: usize,
    passes: Vec<Count>,
}

impl Watch {
    fn of(field: &Field) -> Watch {
        Watch { last: field.columns.clone(), living: living(field), passes: Vec::new() }
    }

    /// `phase` has just ended on `field`: its changed slots and the living as it began are
    /// added to its counts, and the next pass begins from here.
    fn saw(&mut self, phase: &'static str, field: &Field) {
        let changed = changed(&self.last, &field.columns).len();
        let at = match self.passes.iter().position(|count| count.phase == phase) {
            Some(at) => at,
            None => {
                self.passes.push(Count { phase, ..Count::default() });
                self.passes.len() - 1
            }
        };
        let count = &mut self.passes[at];
        count.laps += 1;
        count.changed += changed;
        count.living += self.living;
        self.last = field.columns.clone();
        self.living = living(field);
    }
}

/// The units the store holds whose state is before DEAD (UnitState.living).
pub fn living(field: &Field) -> usize {
    let states = &field.columns.state;
    (0..states.len()).filter(|&s| field.unit_at(s as u32).is_some() && states[s] < DEAD).count()
}

/// The slots whose columns differ between `before` and `after`, in slot order. A slot only
/// one of them has differs.
pub fn changed(before: &Columns, after: &Columns) -> Vec<usize> {
    let slots = before.state.len().max(after.state.len());
    (0..slots).filter(|&slot| differs(before, after, slot)).collect()
}

/// Whether any of `slot`'s columns differs. Every column is named, so a new one won't compile
/// until it is named here too.
fn differs(before: &Columns, after: &Columns, slot: usize) -> bool {
    let Columns {
        position,
        loose_at,
        flee_offset,
        next,
        foe_at,
        toward,
        foothold,
        flags,
        touch,
        keys,
        rank,
        column,
        hp,
        attack_cooldown,
        exerted,
        was_running,
        last_wounding,
        on_face,
        state,
        goal_foe,
        goal_slot,
        caught,
        held_by,
        target_id,
        stuck,
        closest,
        along,
        fanned,
        bearing,
        regeneration_left,
        stamina,
        speed,
        breather,
        run_build,
        condition,
        regeneration_carry,
        regeneration_halt,
        wake_left,
        drain_carry,
        distance,
        climb_left,
        sheet,
        words,
        objects,
    } = before;
    macro_rules! any_differs {
        ($($column:ident),*) => { false $(|| $column.get(slot) != after.$column.get(slot))* };
    }
    any_differs!(
        position,
        loose_at,
        flee_offset,
        next,
        foe_at,
        toward,
        foothold,
        flags,
        touch,
        keys,
        rank,
        column,
        hp,
        attack_cooldown,
        exerted,
        was_running,
        last_wounding,
        on_face,
        state,
        goal_foe,
        goal_slot,
        caught,
        held_by,
        target_id,
        stuck,
        closest,
        along,
        fanned,
        bearing,
        regeneration_left,
        stamina,
        speed,
        breather,
        run_build,
        condition,
        regeneration_carry,
        regeneration_halt,
        wake_left,
        drain_carry,
        distance,
        climb_left,
        sheet,
        words,
        objects
    )
}

/// The lines `counts=1` prints after the times: a "counts:" line, then each pass in the order
/// the tick runs them, its changed slots and its living a lap, and the share changed. Each
/// ends with "%", so `bench_compare.py` never reads one as a phase's time.
pub fn lines(counts: &Counts) -> Vec<String> {
    let mut out = vec![format!(
        "counts: slots changed against the living, a lap, over {} ticks",
        counts.ticks
    )];
    for count in &counts.passes {
        let laps = count.laps.max(1) as f64;
        let (changed, living) = (count.changed as f64 / laps, count.living as f64 / laps);
        let share = if count.living == 0 { 0.0 } else { 100.0 * changed / living };
        out.push(format!(
            "  {:<32} {:>8.1} of {:>8.1} {:>6.1}%",
            count.phase, changed, living, share
        ));
    }
    out
}
