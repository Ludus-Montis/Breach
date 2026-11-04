//! The snapshot tests (spec 39 step 8d): each committed cut is still what the digest's run
//! reaches, and each phase of the tick stepped from it, read back, leaves the state it
//! recorded. Regenerate them with `native/bin/breach_digest cuts=native/rust/cuts` after an
//! intended outcome change: only the cuts whose battles changed are rewritten.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use super::*;
use crate::content::tests::repo_content;

fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

/// The committed cuts' folder.
fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cuts")
}

/// The committed cut of `label` at `tick`, with the prints it recorded (a field log's own
/// state from its `.field` file).
fn committed(label: &str, tick: i64) -> (Cut, Vec<(String, String)>) {
    let name = file_name(label, tick);
    let text = fs::read_to_string(folder().join(format!("{name}.txt")))
        .unwrap_or_else(|e| panic!("{name}.txt: {e}"));
    let bytes = fs::read(folder().join(format!("{name}.state")))
        .unwrap_or_else(|e| panic!("{name}.state: {e}"));
    let carried = fs::read(folder().join(format!("{name}.field"))).unwrap_or_default();
    read(&text, (bytes, carried)).unwrap_or_else(|| panic!("{name}.txt isn't a cut"))
}

fn named(prints: &Prints) -> Vec<(String, String)> {
    prints.iter().map(|(phase, print)| (phase.to_string(), print.clone())).collect()
}

#[test]
fn a_tick_stepped_from_a_cut_read_back_prints_as_the_tick_played_on() {
    let content = content();
    let field = [("field2 rtrue", 229), ("field3 rtrue", 470)];
    for (label, tick) in
        [("mirror_headon s1 rtrue", 80), ("clash40 s1 rfalse", 141)].into_iter().chain(field)
    {
        let run = run_of(label).expect("a digest battle");
        let (cut, played) = cut(&run, tick, &content).expect("the battle reaches the tick");
        assert!(played.iter().any(|(phase, _)| *phase == "fight (blows)"), "{label}: a whole tick");
        assert_eq!(resumed(&cut, &content), Some(played), "{label} at {tick}");
    }
}

#[test]
fn a_cut_written_reads_back() {
    let (label, listed) = ("clash40 s1 rfalse".to_string(), vec![0, 3]);
    let cut = Cut { label, tick: 7, listed, bytes: vec![1], carried: vec![2] };
    let prints: Prints =
        vec![("rules", "ab".into()), ("scrum: seek, walk and facing", "cd".into())];
    let back = read(&written(&cut, &prints), (vec![1], vec![2]));
    assert_eq!(back, Some((cut, named(&prints))));
    assert_eq!(file_name("clash40 s1 rfalse", 7), "clash40-s1-rfalse-t7");
}

#[test]
fn every_committed_cut_is_the_digests_battle_and_steps_as_recorded() {
    let content = content();
    for (label, tick) in CUTS {
        let (kept, recorded) = committed(label, tick);
        let run = run_of(label).expect("a digest battle");
        let (now, _) = cut(&run, tick, &content).expect("the battle reaches the tick");
        assert!(
            now == kept,
            "{label} at {tick}: the battle no longer reaches the committed state; if the \
             outcome change is intended, regenerate with breach_digest cuts=native/rust/cuts"
        );
        let stepped = named(&resumed(&kept, &content).expect("the cut reads back"));
        let first = stepped.iter().zip(&recorded).position(|(a, b)| a != b);
        assert_eq!(first, None, "{label} at {tick}: the first lap that differs");
        assert_eq!(stepped.len(), recorded.len(), "{label} at {tick}: the laps");
    }
}

#[test]
fn the_cuts_change_the_state_in_every_phase_the_runs_act_in() {
    let (mut lapped, mut changed) = (BTreeSet::new(), BTreeSet::new());
    for (label, tick) in CUTS {
        let (kept, recorded) = committed(label, tick);
        let state = snapshot::read(&kept.bytes).expect("the cut reads back");
        // A field's commands, builders and waves act before the tick's first lap, so a field
        // cut's first lap is measured from itself, not from the cut.
        let mut last = match kept.carried.is_empty() {
            true => print(&state.field, &state.squads),
            false => recorded.first().map(|(_, print)| print.clone()).unwrap_or_default(),
        };
        for (phase, print) in recorded {
            lapped.insert(phase.clone());
            if print != last {
                changed.insert(phase);
            }
            last = print;
        }
    }
    let unchanged: BTreeSet<String> = UNCHANGED.iter().map(|p| p.to_string()).collect();
    let acting: BTreeSet<String> = lapped.difference(&unchanged).cloned().collect();
    assert_eq!(changed, acting, "every phase but those UNCHANGED names changes some cut");
}
