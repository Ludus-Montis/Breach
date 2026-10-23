//! Content as the files hold it: JSON indented by two spaces, one field a line, with a list of
//! plain values (a cell, a list of ids, a route's cells) kept on one line so a map stays
//! readable. Reading it back gives the same value: reals keep every bit (`float_roundtrip`).

use serde::Serialize;
use serde_json::Value;

/// `value` as a content file's text, ending in a newline.
pub fn file_text(value: &impl Serialize) -> String {
    let mut out = String::new();
    write(&serde_json::to_value(value).expect("content serialises"), 0, &mut out);
    out.push('\n');
    out
}

fn write(value: &Value, depth: usize, out: &mut String) {
    let indent = |depth: usize| "  ".repeat(depth);
    match value {
        Value::Object(fields) if !fields.is_empty() => {
            out.push_str("{\n");
            for (at, (key, field)) in fields.iter().enumerate() {
                out.push_str(&indent(depth + 1));
                out.push_str(&Value::String(key.clone()).to_string());
                out.push_str(": ");
                write(field, depth + 1, out);
                out.push_str(if at + 1 < fields.len() { ",\n" } else { "\n" });
            }
            out.push_str(&indent(depth));
            out.push('}');
        }
        Value::Array(items) if !items.iter().all(is_plain) => {
            out.push_str("[\n");
            for (at, item) in items.iter().enumerate() {
                out.push_str(&indent(depth + 1));
                write(item, depth + 1, out);
                out.push_str(if at + 1 < items.len() { ",\n" } else { "\n" });
            }
            out.push_str(&indent(depth));
            out.push(']');
        }
        Value::Array(items) => {
            out.push('[');
            for (at, item) in items.iter().enumerate() {
                if at > 0 {
                    out.push_str(", ");
                }
                write(item, depth, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// A value kept on its list's line: a scalar, or a list of scalars (a cell).
fn is_plain(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().all(|item| !item.is_array() && !item.is_object()),
        Value::Object(fields) => fields.is_empty(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_list_of_cells_stays_on_one_line_and_reads_back_the_same() {
        let value = json!({"cells": [[1, 2], [3, 4]], "cost": 1.5, "nodes": [{"id": "p"}]});
        let text = file_text(&value);
        assert!(text.contains("\"cells\": [[1, 2], [3, 4]],"), "{text}");
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), value);
    }
}
