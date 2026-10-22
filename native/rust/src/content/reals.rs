//! A list of reals that may hold an infinity, which JSON numbers can't: written `"inf"` (or
//! `"-inf"`), every other value a JSON number. The tuning's leash steps end in one (no
//! leash past the last step).

use std::borrow::Cow;

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// One real, or an infinity named.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Real {
    Number(f64),
    Named(String),
}

pub fn serialize<S: Serializer>(values: &[f64], out: S) -> Result<S::Ok, S::Error> {
    let reals: Vec<Real> = values
        .iter()
        .map(|&value| match value {
            f64::INFINITY => Real::Named("inf".into()),
            f64::NEG_INFINITY => Real::Named("-inf".into()),
            _ => Real::Number(value),
        })
        .collect();
    reals.serialize(out)
}

pub fn deserialize<'de, D: Deserializer<'de>>(input: D) -> Result<Vec<f64>, D::Error> {
    Vec::<Real>::deserialize(input)?
        .into_iter()
        .map(|real| match real {
            Real::Number(value) => Ok(value),
            Real::Named(name) if name == "inf" => Ok(f64::INFINITY),
            Real::Named(name) if name == "-inf" => Ok(f64::NEG_INFINITY),
            Real::Named(name) => {
                Err(D::Error::custom(format!("expected a number, got \"{name}\"")))
            }
        })
        .collect()
}

/// The schema of such a list: numbers, or "inf" and "-inf".
pub struct Reals;

impl JsonSchema for Reals {
    fn schema_name() -> Cow<'static, str> {
        "Reals".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "array",
            "items": {"anyOf": [{"type": "number"}, {"enum": ["inf", "-inf"]}]}
        })
    }
}
