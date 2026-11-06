//! The digest's runs, played and hashed in the crate as tools/formation_digest.gd plays and
//! hashes them (spec 39 step 8c-xiii), for the cargo binary `breach_digest` (src/bin/digest.rs):
//! each set's runs in the digest's order and by its labels, each run's records (`digest::battle`
//! or `digest::field`) chained with SHA-256, and its line and each tick's line written as the
//! digest prints them and writes them to `out=`.

#[cfg(test)]
mod tests;

use sha2::{Digest as _, Sha256};

use super::{battle, field, Told};
use crate::battles;
use crate::content::Content;
use crate::events::Log;
use crate::field::Field;
use crate::play::{Play, Watch};
use crate::record;
use crate::squads::Squads;
use crate::threads::Threads;

/// The feel-test field logs the digest replays (formation_digest.gd's FIELD_LOGS).
pub const FIELD_LOGS: [&str; 5] = [
    "seed 492625 captain off\n96 via_c on\n158 send A+B",
    "seed 515557 captain off\n0 pursues off\n120 send B\n318 retreat B",
    "seed 109563 captain off\n0 via_c on\n87 send A\n229 retreat A",
    "record 1 seed 492625 captain off\n0 player tend A carry\n0 player tend B recover\n96 player \
     route A C\n158 player send A+B",
    "record 1 seed 515557 captain off\n0 kingdom pursue all off\n0 player tend B recover\n120 \
     player send B\n318 player retreat B",
];
/// The ticks a battle plays at most (LIMIT), and a field past its last command (FIELD_EXTRA).
pub const LIMIT: i64 = 1500;
pub const FIELD_EXTRA: i64 = 700;

/// What a run plays: a battle by its `battles::named` name and numbers, seeded, stopped so
/// many ticks past first contact or not (-1); or a field log.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Battle { name: &'static str, numbers: Vec<i64>, seed: i64, after: i64 },
    Field(&'static str),
}

/// A run as the digest labels it, its blows rolled or not, and its limit: a battle's most
/// ticks, or a field's ticks past its last command.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub label: String,
    pub kind: Kind,
    pub rolls: bool,
    pub limit: i64,
}

/// The set's runs in the digest's order (`set=`: standard, clash160 or wide); None for a set
/// the digest doesn't know.
pub fn runs(set: &str) -> Option<Vec<Run>> {
    Some(match set {
        "standard" => standard(),
        "clash160" => vec![clash("clash160", (160, 10), 1, true, 120)],
        "wide" => vec![
            clash("wide96", (96, 32), 1, true, LIMIT),
            clash("wide96", (96, 32), 2, false, LIMIT),
        ],
        _ => return None,
    })
}

/// formation_digest.gd's `_standard`: for blows rolled, then not, the mirrors head-on and on a
/// flank, seeds 1 to 3; the five field logs; a 40-a-side clash, seeds 1 and 2.
fn standard() -> Vec<Run> {
    let mut runs = Vec::new();
    for rolls in [true, false] {
        for name in ["mirror_headon", "mirror_flank"] {
            for seed in 1..=3 {
                let kind = Kind::Battle { name, numbers: vec![0], seed, after: -1 };
                runs.push(Run {
                    label: format!("{name} s{seed} r{rolls}"),
                    kind,
                    rolls,
                    limit: LIMIT,
                });
            }
        }
        for (index, log) in FIELD_LOGS.iter().enumerate() {
            let label = format!("field{index} r{rolls}");
            runs.push(Run { label, kind: Kind::Field(log), rolls, limit: FIELD_EXTRA });
        }
        for seed in 1..=2 {
            runs.push(clash("clash40", (40, 10), seed, rolls, LIMIT));
        }
    }
    runs
}

/// `_clash`: grems against militia, so many a side and so wide, `after` ticks past first
/// contact.
fn clash(label: &str, (per_side, width): (i64, i64), seed: i64, rolls: bool, after: i64) -> Run {
    let kind = Kind::Battle { name: "clash", numbers: vec![per_side, width], seed, after };
    Run { label: format!("{label} s{seed} r{rolls}"), kind, rolls, limit: LIMIT }
}

impl Run {
    /// The run played on a fresh crate, `threaded` or not, each tick told to `tick` with its
    /// record's own hash and the record itself (for the trace): the ticks played and the chain of every record, or (0, None) if
    /// it can't be set up (an unknown battle or definition, a log that isn't a record).
    pub fn digest(
        &self,
        content: &Content,
        threaded: Threads,
        tick: &mut impl FnMut(i64, &[u8; 32], &Told<'_>),
    ) -> (i64, Option<[u8; 32]>) {
        let (mut squads, mut field_, mut log) =
            (Squads::default(), Field::default(), Log::default());
        let mut chain = Chain::default();
        let mut ticks = 0;
        let mut out = |told: Told<'_>| {
            let record = told.record();
            ticks += 1;
            tick(ticks, &Chain::of(&record), &told);
            chain.update(&record);
        };
        let crate_ = (&mut squads, &mut field_);
        let played = match &self.kind {
            Kind::Battle { name, numbers, seed, after } => {
                let Some(set) = battles::named(name, numbers) else { return (0, None) };
                let at = (*seed, self.rolls, threaded);
                let Some(mut play) = Play::new(&set, crate_, content, at) else { return (0, None) };
                let mut watch = Watch::new(self.limit, (*after >= 0).then_some(*after));
                let crate_ = (&mut squads, &mut field_);
                Some(battle(&mut play, crate_, content, &mut log, &mut watch, &mut out))
            }
            Kind::Field(text) => {
                let Some(record) = record::parse(text) else { return (0, None) };
                let how = (self.rolls, threaded);
                field(&record, self.limit, crate_, content, how, &mut log, &mut out)
            }
        };
        match played {
            Some(played) => (played, Some(chain.finish())),
            None => (0, None),
        }
    }
}

/// SHA-256 over every record in turn, as the digest's HashingContext chains them.
#[derive(Default)]
pub struct Chain(Sha256);

impl Chain {
    pub fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    pub fn finish(self) -> [u8; 32] {
        self.0.finalize().into()
    }

    /// One record's own hash.
    pub fn of(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }
}

/// Lowercase hex, as Godot's `hex_encode`.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The run's printed line: `"%-32s ticks %4d  %s"`.
pub fn line(label: &str, ticks: i64, hash: &[u8; 32]) -> String {
    format!("{label:<32} ticks {ticks:>4}  {}", hex(hash))
}

/// A tick's line in `out=`: `"%s %d %s"`, the record's hash cut to 16 hex digits.
pub fn tick_line(label: &str, tick: i64, hash: &[u8; 32]) -> String {
    format!("{label} {tick} {}", &hex(hash)[..16])
}
