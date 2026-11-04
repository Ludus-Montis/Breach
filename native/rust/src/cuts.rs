//! The snapshot tests' cuts (spec 39 step 8d): a digest battle's whole state after a tick,
//! written by the digest (`breach_digest cuts=<dir>`) and committed under `native/rust/cuts/`,
//! then read back and stepped one tick, the state fingerprinted at the end of each phase.
//!
//! A cut is the snapshot's bytes (`snapshot::write`: the field and the squads, the tick and
//! the seed) and what the run keeps beside them: the run it was cut from (by its digest
//! label, whose battle or field log and rolls give the rest), the squads listed and, for a
//! field log, FieldPlay's own state (`field_play::carried`, step 8d-ii). Its prints are
//! the SHA-256 of the state written as each lap of the next tick ends (`laps::watched`), so
//! every phase is checked on the state the phase before it left, and the first print that
//! differs names the phase that changed. A phase's input is the print before it: the cut's
//! own state for the first, and so each phase is stepped from a state the digest wrote.
//!
//! The cuts are chosen (`CUTS`) so that every phase the digest's runs act in changes the
//! state in at least one of them.

mod played;
#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::rc::Rc;

use crate::content::Content;
use crate::digest::runs::{hex, runs, Chain, Run};
use crate::events::Log;
use crate::field::Field;
use crate::laps;
use crate::snapshot;
use crate::squads::Squads;
use played::Played;

/// The cuts committed: each a run of the standard digest, by its label, and the tick it is
/// cut after (the tick stepped from it is the next). Chosen by which phases change the state
/// in the tick after, so that together they cover every phase the runs act in: the battles',
/// then the field logs' (the pathing, the withdrawal, the carrying and its strays).
pub const CUTS: [(&str, i64); 8 + FIELD_CUTS] = [
    ("mirror_flank s1 rtrue", 0),
    ("mirror_flank s1 rtrue", 34),
    ("mirror_headon s1 rtrue", 80),
    ("mirror_flank s2 rtrue", 108),
    ("mirror_headon s3 rtrue", 129),
    ("clash40 s1 rfalse", 141),
    ("clash40 s1 rfalse", 202),
    ("clash40 s2 rfalse", 277),
    ("field1 rtrue", 120),
    ("field2 rtrue", 229),
    ("field3 rtrue", 470),
];

/// How many of `CUTS` are the field logs' (the last).
pub const FIELD_CUTS: usize = 3;

/// The phases a tick laps that the digest's runs never change the state in, in any tick:
/// the rules and the roster (which only read), and the joins and the groups (no run joins or
/// splits).
pub const UNCHANGED: [&str; 4] = ["rules", "roster", "joins", "groups (form up, split)"];

/// A run's state after `tick`: the run's label, the squads listed, the snapshot and, for a
/// field log, FieldPlay's own state written (empty for a battle).
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub label: String,
    pub tick: i64,
    pub listed: Vec<usize>,
    pub bytes: Vec<u8>,
    pub carried: Vec<u8>,
}

/// Each lap of a tick in order: its phase, and the state's fingerprint as it ended.
pub type Prints = Vec<(&'static str, String)>;

/// The standard digest's run (a battle or a field log) labelled `label`.
pub fn run_of(label: &str) -> Option<Run> {
    runs("standard")?.into_iter().find(|run| run.label == label)
}

/// `run` played on one thread to `tick` and cut there, with the prints of the tick played on
/// after it; None if it can't be set up or stops before then.
pub fn cut(run: &Run, tick: i64, content: &Content) -> Option<(Cut, Prints)> {
    let (mut squads, mut field, mut log) = (Squads::default(), Field::default(), Log::default());
    let mut played = Played::opened(run, (&mut squads, &mut field), content)?;
    while played.play().tick < tick {
        played.step((&mut squads, &mut field), content, &mut log)?;
        log.take();
    }
    let (play, label) = (played.play(), run.label.clone());
    let bytes = snapshot::write(play.tick, play.seed, &field, &squads);
    let (listed, carried) = (play.listed.clone(), played.carried());
    let cut = Cut { label, tick, listed, bytes, carried };
    let prints = printed(&mut played, (&mut squads, &mut field), content);
    Some((cut, prints))
}

/// The tick after `cut` stepped from its state read back, and its prints; None if the bytes
/// don't read, its run isn't one of the digest's or a field's own state doesn't fit it.
pub fn resumed(cut: &Cut, content: &Content) -> Option<Prints> {
    let state = snapshot::read(&cut.bytes).ok()?;
    let run = run_of(&cut.label)?;
    let beside = (cut.listed.as_slice(), cut.carried.as_slice());
    let mut played = Played::resumed(&run, (state.tick, state.seed), beside, content)?;
    let (mut squads, mut field) = (state.squads, state.field);
    Some(printed(&mut played, (&mut squads, &mut field), content))
}

/// The next tick of `played`, the state fingerprinted at each lap.
fn printed(
    played: &mut Played,
    (squads, field): (&mut Squads, &mut Field),
    content: &Content,
) -> Prints {
    let prints: Rc<RefCell<Prints>> = Rc::default();
    let kept = Rc::clone(&prints);
    let watcher: laps::Watcher = Box::new(move |phase, field, squads| {
        kept.borrow_mut().push((phase, print(field, squads)));
    });
    let mut log = Log::default();
    laps::watched(watcher, || played.step((squads, field), content, &mut log));
    prints.take()
}

/// The state's fingerprint: the SHA-256 of the field and the squads as a snapshot writes them.
pub fn print(field: &Field, squads: &Squads) -> String {
    hex(&Chain::of(&snapshot::write(0, 0, field, squads)))
}

/// A cut's file name: its label with spaces as dashes, and its tick.
pub fn file_name(label: &str, tick: i64) -> String {
    format!("{}-t{tick}", label.replace(' ', "-"))
}

/// The cut's text file: its run, its tick, the squads listed, then each lap's print.
pub fn written(cut: &Cut, prints: &Prints) -> String {
    let listed: Vec<String> = cut.listed.iter().map(usize::to_string).collect();
    let mut out = format!("run {}\ntick {}\nlisted {}\n", cut.label, cut.tick, listed.join(" "));
    for (phase, print) in prints {
        out.push_str(&format!("lap {print} {phase}\n"));
    }
    out
}

/// A cut's text file read back with its snapshot's `bytes` and its field's `carried` state
/// (none for a battle): the cut and the prints recorded (each phase as written); None if it
/// isn't one.
pub fn read(
    text: &str,
    (bytes, carried): (Vec<u8>, Vec<u8>),
) -> Option<(Cut, Vec<(String, String)>)> {
    let mut lines = text.lines();
    let label = lines.next()?.strip_prefix("run ")?.to_string();
    let tick = lines.next()?.strip_prefix("tick ")?.parse().ok()?;
    let listed = lines.next()?.strip_prefix("listed")?;
    let listed = listed.split_whitespace().map(str::parse).collect::<Result<_, _>>().ok()?;
    let mut prints = Vec::new();
    for line in lines {
        let (print, phase) = line.strip_prefix("lap ")?.split_once(' ')?;
        prints.push((phase.to_string(), print.to_string()));
    }
    Some((Cut { label, tick, listed, bytes, carried }, prints))
}
