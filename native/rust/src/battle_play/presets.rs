//! WavePresets (Decision 43) in the crate: a lane's shape saved by its units' kinds, and a
//! saved shape fitted to a lane's width and the slots it can reach, front-first, with what no
//! longer fits dropped and kinds it doesn't know skipped.

use super::{foot, KINDS};
use crate::content::Content;
use crate::waves::Template;

/// The kinds a preset names, by unit (the feel test's grem, brute and spitter).
const NAMES: [&str; 3] = ["grem", "brute", "spitter"];
/// WavePresets.LEGACY_KINDS: the names presets saved before kinds were named used.
const LEGACY: [(&str, &str); 2] = [("light", "grem"), ("heavy", "brute")];

/// A unit of a preset: its kind's name, rank and column.
#[derive(Clone, Debug, PartialEq)]
pub struct Saved {
    pub kind: String,
    pub rank: i32,
    pub column: i32,
}

/// WavePresets.to_dict's units: the template's units front-first, by kind; units of no kind
/// a preset names are left out.
pub fn saved(template: &Template) -> Vec<Saved> {
    let kind = |id: &str| KINDS.iter().position(|&k| k == id).map(|k| NAMES[k]);
    let units = template.ordered().into_iter();
    let named = units.filter_map(|p| kind(p.id).map(|name| (name, p.at)));
    named.map(|(name, at)| Saved { kind: name.to_string(), rank: at.0, column: at.1 }).collect()
}

/// WavePresets.from_dict: `units` painted in order on a template `width` wide with
/// `allowance` slots.
pub fn fitted(units: &[Saved], width: i32, allowance: i32, content: &Content) -> Template {
    let mut template = Template::new(width, allowance);
    for unit in units {
        let Some(id) = unit_of(&unit.kind) else { continue };
        if let Some(foot) = foot(content, id) {
            template.paint(id, foot, (unit.rank, unit.column));
        }
    }
    template
}

/// The unit a kind's name stands for, legacy names read as their kinds.
fn unit_of(kind: &str) -> Option<&'static str> {
    let kind = LEGACY.iter().find(|(old, _)| *old == kind).map_or(kind, |(_, new)| new);
    NAMES.iter().position(|&n| n == kind).map(|k| KINDS[k])
}
