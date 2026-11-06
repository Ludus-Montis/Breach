use std::cell::RefCell;
use std::rc::Rc;

use super::*;

#[test]
fn a_split_is_timed_as_a_lap_but_shows_the_watcher_nothing() {
    let (field, squads) = (Field::default(), Squads::default());
    let seen: Rc<RefCell<Vec<&'static str>>> = Rc::default();
    let kept = Rc::clone(&seen);
    let watcher: Watcher = Box::new(move |phase, _, _| kept.borrow_mut().push(phase));
    let ((), spent) = watched(watcher, || {
        timed(|| {
            split("a: part");
            lap("a: rest", &field, &squads);
            split("a: part");
        })
    });
    let names: Vec<&str> = spent.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["a: part", "a: rest"]);
    assert_eq!(*seen.borrow(), ["a: rest"]);
}

#[test]
fn a_split_outside_timed_counts_nothing() {
    split("a: part");
    let ((), spent) = timed(|| ());
    assert!(spent.is_empty());
}
