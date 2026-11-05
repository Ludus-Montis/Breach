//! The trace's line as tools/digest_trace.gd wrote it: named fields, nested as the state
//! nests them, and values as JSON that reads back bit for bit.

use super::*;
use crate::maths::V2;
use Plain::{Bool, Float, Int, List, Nil, Point};

/// A unit's state: its id and hp, where it is, and the rest.
fn unit(id: i64) -> Plain {
    List(vec![
        List(vec![Int(id), Float(2.5), Point(V2 { x: 0.1, y: -3.0 }), Float(1.0), Int(0)]),
        List(vec![Int(-1), Int(0), Int(1), List(vec![Nil, Bool(false), Nil])]),
    ])
}

/// A squad's state holding `units`.
fn squad(id: i64, units: Vec<Plain>) -> Plain {
    List(vec![
        List(vec![Int(id), Int(2), Int(0), Float(f64::INFINITY), Float(0.0)]),
        List(vec![Point(V2 { x: 4.0, y: 5.0 }), Int(80), Int(-1), Int(3)]),
        List(vec![Int(7), List(vec![Nil, Nil, Nil, Nil]), List(units)]),
    ])
}

#[test]
fn a_tick_names_every_squad_field_and_unit_field_in_order() {
    let state = List(vec![squad(1, vec![unit(10)])]);
    let events = vec![List(vec![Plain::str("hit"), Int(10)])];
    let written = line("mirror_headon s1 rtrue", 4, &events, &state, &["player".to_string()]);
    let unit = r#"{"id":10,"hp":2.5,"position":[0.10000000149011612,-3.0],"bearing":1.0,"#
        .to_string()
        + r#""attack_cooldown":0,"target_id":-1,"rank":0,"column":1,"loose":null,"#
        + r#""fleeing":false,"chaser":null}"#;
    let squad = r#"{"faction":"player","id":1,"state":2,"order":0,"#.to_string()
        + r#""front_distance":Infinity,"heading":0.0,"position":[4.0,5.0],"morale":80,"#
        + r#""engaged_with":-1,"stall_ticks":3,"fight_since":7,"flank_contacts":null,"#
        + r#""stance":null,"pursuit":null,"withdraw":null,"units":["#
        + &unit
        + "]}";
    let want = r#"{"run":"mirror_headon s1 rtrue","tick":4,"events":[["hit",10]],"squads":["#
        .to_string()
        + &squad
        + "]}";
    assert_eq!(written, want);
}

#[test]
fn values_read_back_bit_for_bit_and_strings_are_escaped() {
    let events = vec![
        Float(0.1),
        Float(f64::NEG_INFINITY),
        Float(f64::NAN),
        Float(1e300),
        Plain::str("a \"b\"\n"),
        List(vec![]),
    ];
    let written = line("r", 1, &events, &List(vec![]), &[]);
    let want = r#"{"run":"r","tick":1,"events":[0.1,-Infinity,NaN,1e300,"a \"b\"\n",[]],"#
        .to_string()
        + r#""squads":[]}"#;
    assert_eq!(written, want);
}

#[test]
fn ticks_are_traced_from_from_to_to_or_on_when_to_is_negative() {
    assert!(!traced(4, 5, 9));
    assert!(traced(5, 5, 9));
    assert!(traced(9, 5, 9));
    assert!(!traced(10, 5, 9));
    assert!(traced(10_000, 1, -1));
}
