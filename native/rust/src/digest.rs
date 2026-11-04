//! The formation digest's record in the crate (tools/formation_digest.gd, spec 39 step 8c-xi):
//! each tick's state as the plain values DigestState gives (`state`), and those values as the
//! bytes Godot's `var_to_bytes` gives them, which the digest hashes. A plain value is nil, a
//! bool, an int, a float, a string, a point or a list of them: DigestState.plain has already
//! turned every object into ["obj", its id] and every dictionary into a list (`Plain::dict`).
//!
//! Godot's encoding: a 4-byte header naming the type, its bit 16 set where an int or a float
//! needs 64 bits (an int outside i32, a float an f32 doesn't hold exactly), then the value,
//! little-endian: a bool or a narrow int in 4 bytes, a narrow float as an f32, a point as two
//! f32s, a string as its UTF-8 length and bytes padded to 4, a list as its length and each
//! value in turn.

pub mod state;

#[cfg(test)]
mod tests;

use crate::maths::V2;

/// Godot's Variant types the digest's values take.
const NIL: u32 = 0;
const BOOL: u32 = 1;
const INT: u32 = 2;
const FLOAT: u32 = 3;
const STRING: u32 = 4;
const VECTOR2: u32 = 5;
const ARRAY: u32 = 28;
/// The header's flag: the int or float is written in 64 bits.
const WIDE: u32 = 1 << 16;

/// A plain value, as DigestState gives it.
#[derive(Clone, Debug, PartialEq)]
pub enum Plain {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Point(V2),
    List(Vec<Plain>),
}

impl Plain {
    /// A string.
    pub fn str(s: &str) -> Plain {
        Plain::Str(s.to_string())
    }

    /// An object as DigestState.plain gives it: ["obj", its id].
    pub fn object(id: i64) -> Plain {
        Plain::List(vec![Plain::str("obj"), Plain::Int(id)])
    }

    /// A dictionary as DigestState.plain gives it: [how many keys, [[key, value]...]], each
    /// key as `var_to_str` writes it and the pairs in the order of those strings. Its keys are
    /// ints and strings.
    pub fn dict(entries: Vec<(Plain, Plain)>) -> Plain {
        let mut pairs: Vec<(String, Plain)> =
            entries.into_iter().map(|(key, value)| (key.written(), value)).collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let count = pairs.len() as i64;
        let pairs = pairs.into_iter().map(|(key, value)| Plain::List(vec![Plain::Str(key), value]));
        Plain::List(vec![Plain::Int(count), Plain::List(pairs.collect())])
    }

    /// A dictionary key as Godot's `var_to_str` writes it: an int in decimal, a string quoted
    /// (the digest's keys are names, with nothing to escape).
    fn written(&self) -> String {
        match self {
            Plain::Int(i) => i.to_string(),
            Plain::Str(s) => format!("\"{s}\""),
            other => panic!("digest: a dictionary key is an int or a string, not {other:?}"),
        }
    }

    /// The bytes Godot's `var_to_bytes` gives the value.
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.put(&mut out);
        out
    }

    fn put(&self, out: &mut Vec<u8>) {
        match self {
            Plain::Nil => word(NIL, out),
            Plain::Bool(b) => {
                word(BOOL, out);
                word(*b as u32, out);
            }
            Plain::Int(i) => {
                if *i >= i32::MIN as i64 && *i <= i32::MAX as i64 {
                    word(INT, out);
                    out.extend_from_slice(&(*i as i32).to_le_bytes());
                } else {
                    word(INT | WIDE, out);
                    out.extend_from_slice(&i.to_le_bytes());
                }
            }
            Plain::Float(f) => {
                let narrow = *f as f32;
                if narrow as f64 == *f {
                    word(FLOAT, out);
                    out.extend_from_slice(&narrow.to_le_bytes());
                } else {
                    word(FLOAT | WIDE, out);
                    out.extend_from_slice(&f.to_le_bytes());
                }
            }
            Plain::Str(s) => {
                word(STRING, out);
                word(s.len() as u32, out);
                out.extend_from_slice(s.as_bytes());
                out.resize(out.len() + (4 - s.len() % 4) % 4, 0);
            }
            Plain::Point(p) => {
                word(VECTOR2, out);
                out.extend_from_slice(&p.x.to_le_bytes());
                out.extend_from_slice(&p.y.to_le_bytes());
            }
            Plain::List(values) => {
                word(ARRAY, out);
                word(values.len() as u32, out);
                for value in values {
                    value.put(out);
                }
            }
        }
    }
}

/// Writes a 4-byte word.
fn word(w: u32, out: &mut Vec<u8>) {
    out.extend_from_slice(&w.to_le_bytes());
}
