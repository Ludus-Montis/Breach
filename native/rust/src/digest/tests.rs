//! Godot's `var_to_bytes`, as Godot 4.7 writes each value (read from `var_to_bytes(v)
//! .hex_encode()`), and DigestState.plain's dictionaries.

use super::*;

fn hex(value: &Plain) -> String {
    value.bytes().iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn nil_bools_and_ints_are_written_as_godot_writes_them() {
    assert_eq!(hex(&Plain::Nil), "00000000");
    assert_eq!(hex(&Plain::Bool(true)), "0100000001000000");
    assert_eq!(hex(&Plain::Bool(false)), "0100000000000000");
    assert_eq!(hex(&Plain::Int(7)), "0200000007000000");
    assert_eq!(hex(&Plain::Int(-1)), "02000000ffffffff");
    assert_eq!(hex(&Plain::Int(3_000_000_000)), "02000100005ed0b200000000", "past i32: 64 bits");
}

#[test]
fn a_float_an_f32_holds_is_written_narrow_and_any_other_wide() {
    assert_eq!(hex(&Plain::Float(2.5)), "0300000000002040");
    assert_eq!(hex(&Plain::Float(f64::INFINITY)), "030000000000807f");
    assert_eq!(hex(&Plain::Float(0.1)), "030001009a9999999999b93f");
}

#[test]
fn strings_points_and_lists_are_written_as_godot_writes_them() {
    assert_eq!(hex(&Plain::str("ab")), "040000000200000061620000", "padded to 4");
    assert_eq!(hex(&Plain::str("abcd")), "040000000400000061626364");
    assert_eq!(hex(&Plain::Point(V2::new(1.0, 2.5))), "050000000000803f00002040");
    assert_eq!(hex(&Plain::List(vec![])), "1c00000000000000");
    let list = Plain::List(vec![Plain::Int(1), Plain::str("x")]);
    assert_eq!(hex(&list), "1c000000020000000200000001000000040000000100000078000000");
}

#[test]
fn a_dictionary_is_its_size_and_its_pairs_in_the_order_of_their_written_keys() {
    let dict = Plain::dict(vec![
        (Plain::str("since"), Plain::Int(4)),
        (Plain::str("foe"), Plain::Int(2)),
        (Plain::Int(12), Plain::Nil),
        (Plain::Int(9), Plain::Nil),
    ]);
    let pair = |key: &str, value: Plain| Plain::List(vec![Plain::str(key), value]);
    let pairs = vec![
        pair("\"foe\"", Plain::Int(2)),
        pair("\"since\"", Plain::Int(4)),
        pair("12", Plain::Nil),
        pair("9", Plain::Nil),
    ];
    assert_eq!(dict, Plain::List(vec![Plain::Int(4), Plain::List(pairs)]));
    assert_eq!(Plain::dict(vec![]), Plain::List(vec![Plain::Int(0), Plain::List(vec![])]));
}

#[test]
fn an_object_is_named_by_its_id() {
    assert_eq!(Plain::object(5), Plain::List(vec![Plain::str("obj"), Plain::Int(5)]));
}
