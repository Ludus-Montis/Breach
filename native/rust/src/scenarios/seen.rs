//! What a scenario shows, read from its replay tick by tick (spec 44 step 2): the first tick
//! with damage (first contact) and the first tick each behaviour is seen.
//! - **engaging:** an `engaged` event;
//! - **flanking:** a `flanked` event;
//! - **breaking off:** a `disengaged` or `withdrawing` event;
//! - **routing:** a squad's morale falling to its routing band, or a `routed` event;
//! - **fording:** a living unit standing in water it can cross (a cell deeper than 0; the only
//!   water a squad stands in is a ford, the streams being too deep);
//! - **cornering:** a squad fighting two foes at once: two different squads it has engaged or
//!   been flanked by, since it last disengaged or withdrew. Read from the events, so it is a
//!   squad caught by a second foe, whatever the edges.

use crate::bands::ROUTING;
use crate::events::{DISENGAGED, ENGAGED, FLANKED, HIT, KINDS, MORALE_BAND, ROUTED, WITHDRAWING};
use crate::field::Field;
use crate::ground::Ground;
use crate::play;
use crate::squads::Squads;

/// The behaviours a scenario can be named for, as the list names them.
pub const BEHAVIOURS: [&str; 6] =
    ["fording", "flanking", "engaging", "breaking_off", "cornering", "routing"];
const FORDING: usize = 0;
const FLANKING: usize = 1;
const ENGAGING: usize = 2;
const BREAKING_OFF: usize = 3;
const CORNERING: usize = 4;
const ROUTING_: usize = 5;

/// What has been seen so far: first contact, each behaviour's first tick (in BEHAVIOURS'
/// order), the behaviours the scenario must show, and each squad's foes since it last broke
/// off (squad id, foe ids).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Seen {
    pub contact: Option<i64>,
    pub first: [Option<i64>; 6],
    pub shows: Vec<String>,
    foes: Vec<(i64, Vec<i64>)>,
}

impl Seen {
    /// Nothing seen yet, by a scenario that must show `shows`.
    pub fn new(shows: &[String]) -> Seen {
        Seen { shows: shows.to_vec(), ..Seen::default() }
    }

    /// The first tick `behaviour` was seen, if it has been (None for a name not in
    /// BEHAVIOURS).
    pub fn at(&self, behaviour: &str) -> Option<i64> {
        BEHAVIOURS.iter().position(|&b| b == behaviour).and_then(|i| self.first[i])
    }

    /// The behaviours the scenario must show and hasn't, in its order.
    pub fn missing(&self) -> Vec<&str> {
        self.shows.iter().map(String::as_str).filter(|b| self.at(b).is_none()).collect()
    }

    /// Tick `tick`'s events (flat) and where the units stand after it, on `ground`.
    pub fn read(
        &mut self,
        tick: i64,
        flat: &[i64],
        (squads, field): (&Squads, &Field),
        ground: &Ground,
    ) {
        for at in play::starts(flat) {
            self.event(tick, &flat[at..]);
        }
        if self.first[FORDING].is_none() && fording(squads, field, ground) {
            self.first[FORDING] = Some(tick);
        }
    }

    /// One event, `event` starting at its kind.
    pub(super) fn event(&mut self, tick: i64, event: &[i64]) {
        let (kind, squad) = (event[0] as usize, event[2]);
        let field = |name: &str| {
            let names = KINDS[kind].1;
            names.iter().position(|&n| n == name).map(|i| event[4 + i])
        };
        match kind {
            HIT if self.contact.is_none() && field("dmg").is_some_and(|dmg| dmg > 0) => {
                self.contact = Some(tick);
            }
            ENGAGED => {
                self.mark(ENGAGING, tick);
                self.faces(tick, squad, field("with"));
            }
            FLANKED => {
                self.mark(FLANKING, tick);
                self.faces(tick, squad, field("by"));
            }
            DISENGAGED | WITHDRAWING => {
                self.mark(BREAKING_OFF, tick);
                self.foes.retain(|(s, _)| *s != squad);
            }
            MORALE_BAND if field("band") == Some(ROUTING) => self.mark(ROUTING_, tick),
            ROUTED => self.mark(ROUTING_, tick),
            _ => {}
        }
    }

    /// `squad` fighting `foe`: cornered once it fights two.
    fn faces(&mut self, tick: i64, squad: i64, foe: Option<i64>) {
        let Some(foe) = foe else { return };
        let index = match self.foes.iter().position(|(s, _)| *s == squad) {
            Some(index) => index,
            None => {
                self.foes.push((squad, Vec::new()));
                self.foes.len() - 1
            }
        };
        let foes = &mut self.foes[index].1;
        if !foes.contains(&foe) {
            foes.push(foe);
        }
        if foes.len() >= 2 {
            self.mark(CORNERING, tick);
        }
    }

    fn mark(&mut self, behaviour: usize, tick: i64) {
        self.first[behaviour].get_or_insert(tick);
    }
}

/// Whether some living unit of a listed squad stands in water.
fn fording(squads: &Squads, field: &Field, ground: &Ground) -> bool {
    let states = &field.columns.state;
    (0..squads.id.len()).any(|s| {
        squads
            .living(s, states)
            .iter()
            .any(|&unit| ground.depth_at(field.columns.position[unit as usize]) > 0.0)
    })
}
