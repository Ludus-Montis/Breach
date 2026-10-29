//! The move's pass (FormationSimulation._move and _march; spec 39 step 8b5-viii): each
//! squad decides its move from the tick's start (`choose`); a staged one holds till its
//! trigger (`staging::hold_staged`), a retreat home already ends there, one free to march
//! holds or goes (`hold`), one going back the way it faces turns first (`turn::begin`), and
//! the rest step at their slowest unit's pace within the share their units keep up with,
//! sweeping round bends (`march::sweep`), slowed by the ground (`march::pace`) and the bodies
//! lying on it (`lying`). Every move then lands together (`land`), arrivals and returns
//! after; the waiting count down. Each event goes to the log as FormationEvents made it.
//! The share each squad's units keep up with is the crate's too (`share`, spec 39 step
//! 8b5-ix), worked out from the tick's start. The joins after are GDScript's still, given
//! the squads that advanced.

use super::share::{self, ShareTuning};
use super::{choose, hold, land, Choice, Plan};
use crate::contact;
use crate::events::{self, Log};
use crate::field::Field;
use crate::geometry::CELLS;
use crate::ground::Ground;
use crate::locks::{ARRIVED, HOLDING};
use crate::lying::Lying;
use crate::march::{self, sweep, Along, End, ADVANCE, RETREAT};
use crate::maths::V2;
use crate::reach;
use crate::roster::{Listed, Roster};
use crate::squads::Squads;
use crate::staging::{self, Hold};
use crate::turn;
use crate::withdraw;

/// SkirmishUnit.Order's hold.
const HOLD: i32 = 1;
/// SkirmishUnit.State's DEAD and DOWNED: the bodies lying on the field (GroundBodies.LYING).
const LYING: [i32; 2] = [4, 5];

/// What the pass is given besides the squads: per listed squad its route's code
/// (FormationStaging's), then the tick's numbers and tuning.
pub struct Given<'a> {
    pub routes: &'a [u32],
    pub tick: i64,
    /// FormationSimulation.TRAVEL_SCALE: tiles a second at speed 1.
    pub travel_scale: f64,
    pub seconds: f64,
    /// Tiles: a squad without a route marches the lane's length.
    pub route_length: f64,
    /// The share's (FormationWalk.share), whose slack is a coordinated leader's too.
    pub share: ShareTuning,
    /// bodies_ground_drag and bodies_ground_block.
    pub drag: [f64; 2],
}

/// FormationSimulation._move over `roster`'s squads on `ground` (none: open, level ground):
/// returns the listed squads whose landing advanced, in the order planned, for the joins.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    ground: Option<&Ground>,
    given: &Given,
    log: &mut Log,
) -> Vec<usize> {
    let mut marching: Vec<(usize, f64)> = Vec::new(); // [listed, its route's end] a march
    let mut plans: Vec<Plan> = Vec::new();
    let mut waiting: Vec<usize> = Vec::new();
    let mut lying: Option<Bodies> = None; // found once, as a march first steps over them
    let keeping = keeping(squads, field, roster, ground, &given.share);
    for listed in 0..roster.listed.len() {
        let s = roster.listed[listed].squad;
        match choose(squads, s) {
            Choice::Stands => continue,
            Choice::Waits => {
                waiting.push(s);
                continue;
            }
            Choice::Goes => {}
        }
        if staged(squads, field, roster, ground, listed, given, log) {
            continue;
        }
        let end = squads.route[s].as_ref().map_or(given.route_length, |r| r.length() / CELLS);
        if squads.order[s] == RETREAT {
            ends(squads, field, roster, listed, end, given.tick, log); // home already: it holds
        }
        if hold(squads, &field.columns, roster, listed) {
            continue;
        }
        let travel = march::travel_sign(&along(squads, s), end);
        let columns = &mut field.columns;
        if travel != squads.direction[s]
            && turn::landed(squads, columns, &roster.listed[listed], travel, given.tick, log)
        {
            continue; // an about-face is a re-form: its units walk to their places
        }
        let lying = lying.get_or_insert_with(|| Bodies::of(squads, field, roster));
        let keeping = keeping[listed];
        let step = stepped(squads, field, roster, ground, listed, lying, keeping, given, log);
        if step < 0.0 {
            continue; // it waits for its units
        }
        marching.push((listed, end));
        let next = march::toward(&along(squads, s), f64::from(travel) * step, end);
        plans.push(Plan { listed, next });
    }
    let joiners = landed(squads, field, roster, &plans);
    for &(listed, end) in &marching {
        ends(squads, field, roster, listed, end, given.tick, log);
    }
    for s in waiting {
        squads.wait_ticks[s] -= 1;
    }
    joiners
}

/// The share of its pace each listed squad that may march keeps for its units
/// (FormationWalk.share), as the tick starts; 0 for one that stands, waits or is ordered to
/// hold.
fn keeping(
    squads: &Squads,
    field: &Field,
    roster: &Roster,
    ground: Option<&Ground>,
    tuning: &ShareTuning,
) -> Vec<f64> {
    let goes = |s| choose(squads, s) == Choice::Goes && squads.order[s] != HOLD;
    let share = |listed: &Listed| share::of(squads, field, listed, ground, tuning);
    roster.listed.iter().map(|l| if goes(l.squad) { share(l) } else { 0.0 }).collect()
}

/// FormationStaging.holds: whether the squad holds at its staging point this tick. Its
/// "staged", "signalled" and "gave_up" events go to `log`; giving up turns it back if its
/// order says so.
fn staged(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    ground: Option<&Ground>,
    listed: usize,
    given: &Given,
    log: &mut Log,
) -> bool {
    let s = roster.listed[listed].squad;
    let Some(order) = squads.staging[s].clone() else {
        return false;
    };
    if squads.order[s] != ADVANCE {
        return false;
    }
    let slack = (order.meet.is_some() && coordinated(field, roster, listed))
        .then(|| share::slack(field, &roster.listed[listed], given.share.slack));
    let pace = [given.travel_scale * CELLS, given.seconds];
    let (columns, routes) = (&field.columns, given.routes);
    let held = staging::hold_staged(squads, columns, roster, ground, listed, routes, slack, pace);
    let tick = given.tick;
    let staged_now = matches!(held, Hold::Holds { staged: true } | Hold::GivesUp { staged: true });
    match held {
        Hold::Signalled => {
            squads.staging[s] = None;
            log.squad(events::SIGNALLED, tick, squads, s, &[]);
            return false;
        }
        _ if staged_now => log.squad(events::STAGED, tick, squads, s, &[]),
        _ => {}
    }
    match held {
        Hold::Holds { .. } => true, // its fallback counted down
        Hold::GivesUp { .. } => {
            let then = order.then.unwrap_or_else(|| "go".to_string());
            squads.staging[s] = None;
            let word = i64::from(field.lexicon.code(&then));
            log.squad(events::GAVE_UP, tick, squads, s, &[word]);
            if then == "back" {
                squads.order[s] = RETREAT;
            }
            false
        }
        _ => false, // still on its way
    }
}

/// FormationStaging._coordinated: any living unit of the squad has the "coordinated" tactic.
fn coordinated(field: &Field, roster: &Roster, listed: usize) -> bool {
    let Some(code) = field.lexicon.find("coordinated") else {
        return false;
    };
    let columns = &field.columns;
    roster.listed[listed].living.iter().any(|m| columns.words[m.slot].tactics.contains(&code))
}

/// FormationSimulation._step: how far the squad steps this tick (tiles), its frame swept
/// round any bend as it goes, keeping `keeping` of its pace for its units; -1 if it waits
/// for them, out of its slack.
#[allow(clippy::too_many_arguments)]
fn stepped(
    squads: &mut Squads,
    field: &Field,
    roster: &Roster,
    ground: Option<&Ground>,
    listed: usize,
    lying: &Bodies,
    keeping: f64,
    given: &Given,
    log: &mut Log,
) -> f64 {
    if keeping <= 0.0 {
        return -1.0;
    }
    let s = roster.listed[listed].squad;
    let speed = staging::speed(&field.columns, &roster.listed[listed]);
    let cells_per_second = speed * given.travel_scale * CELLS;
    sweep::sweep_held(squads, s, cells_per_second, given.seconds * keeping);
    let open_step = speed * given.travel_scale * given.seconds;
    let step = match ground {
        None => open_step,
        Some(ground) => open_step * paced(squads, field, roster, ground, listed, given.tick, log),
    };
    step * lying.drag(squads, field, roster, listed, given.drag) * keeping
}

/// FormationMarch.pace's share: the worst crossing its front rank steps into, left to right;
/// where none of it can go it halts, reported once as "blocked".
fn paced(
    squads: &mut Squads,
    field: &Field,
    roster: &Roster,
    ground: &Ground,
    listed: usize,
    tick: i64,
    log: &mut Log,
) -> f64 {
    let columns = &field.columns;
    let mut front: Vec<usize> = roster.listed[listed]
        .living
        .iter()
        .map(|m| m.slot)
        .filter(|&u| columns.rank[u] == 0)
        .collect();
    front.sort_by_key(|&u| columns.column[u]);
    let walkers: Vec<_> = front
        .iter()
        .map(|&u| crate::pathing::pass::walker(columns.objects[u].walker, columns.sheet[u].height))
        .collect();
    let points: Vec<V2> = front.iter().map(|&u| columns.position[u]).collect();
    let s = roster.listed[listed].squad;
    let worst = march::pace(ground, &walkers, &points, squads.heading[s]);
    if worst <= 0.0 && squads.blocked[s] == 0 {
        log.squad(events::BLOCKED, tick, squads, s, &[]);
    }
    squads.blocked[s] = i32::from(worst <= 0.0);
    worst
}

/// FormationSimulation._march's landing: each march's front as it lands; returns the
/// listed squads that advanced.
fn landed(squads: &mut Squads, field: &Field, roster: &Roster, plans: &[Plan]) -> Vec<usize> {
    let judged = land(&contact::Field { squads, columns: &field.columns, roster }, plans);
    let mut joiners = Vec::new();
    for (plan, (front, advances)) in plans.iter().zip(judged) {
        put_front(squads, roster.listed[plan.listed].squad, front);
        if advances {
            joiners.push(plan.listed);
        }
    }
    joiners
}

/// FormationMarch.check_ends: arrival at the enemy's end, or a retreat home (its front
/// there, or every unit of it home), on a route `end` tiles long; "arrived" or "returned"
/// goes to `log`.
fn ends(
    squads: &mut Squads,
    field: &Field,
    roster: &Roster,
    listed: usize,
    end: f64,
    tick: i64,
    log: &mut Log,
) {
    let s = roster.listed[listed].squad;
    let along = along(squads, s);
    let held = squads.route[s].as_ref();
    let all_home = || {
        held.is_some_and(|route| {
            let home = along.home * CELLS;
            withdraw::home(squads, &field.columns, roster, listed, route, home)
        })
    };
    match march::end(&along, end, held.is_some(), all_home) {
        End::Under => {}
        End::Arrived(front) => {
            put_front(squads, s, front);
            squads.state[s] = ARRIVED;
            log.squad(events::ARRIVED, tick, squads, s, &[]);
        }
        End::Returned(front) => {
            put_front(squads, s, front);
            squads.order[s] = HOLD;
            squads.state[s] = HOLDING;
            log.squad(events::RETURNED, tick, squads, s, &[]);
        }
    }
}

/// StoredSquad.front_distance's setter: its front `front` tiles along, its position with it.
fn put_front(squads: &mut Squads, s: usize, front: f64) {
    squads.front_distance[s] = front;
    squads.position[s] = turn::place(squads.route[s].as_ref(), front * CELLS);
}

/// Where the squad in `s` stands along its route (FormationMarch._along).
fn along(squads: &Squads, s: usize) -> Along {
    Along {
        front: squads.front_distance[s],
        home: squads.home_distance[s],
        order: squads.order[s],
        direction: squads.direction[s],
    }
}

/// The bodies lying on the field as the tick's moves find them (GroundBodies.ground): the
/// downed and the dead of every listed squad, in the lists' and its members' order, where
/// each lies, its radius and its footprint's area.
pub struct Bodies {
    pub ats: Vec<V2>,
    pub radii: Vec<f64>,
    pub areas: Vec<f64>,
}

impl Bodies {
    pub fn of(squads: &Squads, field: &Field, roster: &Roster) -> Bodies {
        let columns = &field.columns;
        let mut bodies = Bodies { ats: Vec::new(), radii: Vec::new(), areas: Vec::new() };
        for listed in &roster.listed {
            for &unit in &squads.members[listed.squad] {
                let u = unit as usize;
                if LYING.contains(&columns.state[u]) {
                    let sheet = &columns.sheet[u];
                    bodies.ats.push(columns.position[u]);
                    bodies.radii.push(reach::radius(sheet.footprint_width, sheet.footprint_depth));
                    bodies.areas.push((sheet.footprint_width * sheet.footprint_depth) as f64);
                }
            }
        }
        bodies
    }

    /// GroundBodies.drag: the share of its pace the squad keeps for the bodies its front rank
    /// steps over.
    fn drag(
        &self,
        squads: &Squads,
        field: &Field,
        roster: &Roster,
        listed: usize,
        [drag, block]: [f64; 2],
    ) -> f64 {
        if self.ats.is_empty() {
            return 1.0;
        }
        let lying = Lying::new(&self.ats, &self.radii, &self.areas);
        lying.drag(squads, &field.columns, &roster.listed[listed], drag, block)
    }
}

#[cfg(test)]
mod tests;
