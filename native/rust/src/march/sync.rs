//! FormationMarch.sync_units as one pass (spec 39 step 8b5-xxxiii): each listed squad's
//! living units follow its placement, including any swap under way. Each one's DISTANCE is
//! its rank's along the route; a router stands on its point on the route (`where_on`), facing
//! the squad's heading unless loose; a loose one stands where its own moves took it. Placing
//! (no timing) sets each unit in its frame on its place, facing the heading; otherwise each
//! walks a tick to where it makes for (`pour::targets`) over the bodies lying, and a fall
//! from a climb face hurts it after, in its squad's members' order.

#[cfg(test)]
mod tests;

use super::modes::{Climber, Footing};
use super::pour::{self, Frame};
use super::walk::{self, Marcher, Standing, WalkTuning, Walking};
use crate::field::Field;
use crate::geometry::RANK_DEPTH;
use crate::ground::Ground;
use crate::lying::Lying;
use crate::maths::V2;
use crate::moving::pass::Bodies;
use crate::moving::share;
use crate::reach;
use crate::rout_blows;
use crate::squads::{Squads, DEAD};
use crate::swaps;
use crate::wounds;

/// What the pass is given: the ground (`None`: open), [seconds, cells a second at speed 1]
/// for a walk (`None`: placing), the fight's seed (the pour's draws), wounds_reach and the
/// walk's tuning.
pub struct Given<'a> {
    pub ground: Option<&'a Ground>,
    pub timing: Option<[f64; 2]>,
    pub seed: i64,
    pub wounds_reach: f64,
    pub tuning: WalkTuning,
}

/// One sync of the squads in `listed` (their slots, in the lists' order).
pub fn step(squads: &Squads, field: &mut Field, listed: &[usize], given: &Given) {
    let bodies = Bodies::listed(squads, field, listed); // lying before any unit walks
    let lying = Lying::new(&bodies.ats, &bodies.radii, &bodies.areas);
    for &s in listed {
        let walkers = place(squads, field, s, given);
        if let Some([seconds, pace]) = given.timing {
            let on = Walking {
                ground: given.ground,
                lying: Some(&lying),
                seconds,
                pace,
                heading: squads.heading[s],
                tuning: given.tuning,
            };
            walk_all(field, &on, &walkers);
        }
    }
}

/// The squad in `s`'s living units set on their places, routes and loose points; those
/// that walk returned, [store slot, id, where each makes for], in its members' order.
fn place(squads: &Squads, field: &mut Field, s: usize, given: &Given) -> Vec<(usize, i64, V2)> {
    let living: Vec<(usize, i64)> = squads.members[s]
        .iter()
        .map(|&u| u as usize)
        .filter(|&u| field.columns.state[u] < DEAD) // the fallen lie where they fell
        .map(|u| (u, field.unit_at(u as u32).unwrap_or(0)))
        .collect();
    let (fleeing, loose) = (&squads.fleeing[s], &squads.loose[s]);
    let framed: Vec<(i32, i64)> = living
        .iter()
        .filter(|(_, id)| !fleeing.contains(id) && !loose.contains(id))
        .map(|&(u, id)| (u as i32, id))
        .collect();
    let frame = Frame {
        anchor: squads.position[s],
        heading: squads.heading[s],
        width: squads.width[s],
        centre_shift: squads.centre_shift[s],
    };
    let pourers = share::pourers(squads, field, s, &framed, true);
    let aims = match given.timing {
        Some(_) => pour::targets(given.ground, &frame, &pourers, given.seed, given.wounds_reach),
        None => Vec::new(),
    };
    let moving = swaps::offsets(squads, s);
    let (front, direction, heading) =
        (squads.front_distance[s], squads.direction[s], squads.heading[s]);
    let c = &mut field.columns;
    let mut walkers = Vec::new();
    let mut in_frame = 0;
    for (u, id) in living {
        let ranks = moving.iter().find(|&&(mover, _, _)| mover == id).map_or(0.0, |m| m.1);
        c.distance[u] = (front - f64::from(direction * c.rank[u]) * RANK_DEPTH)
            + f64::from(direction) * f64::from(ranks) * RANK_DEPTH;
        if fleeing.contains(&id) {
            c.position[u] =
                rout_blows::where_on(squads.route[s].as_ref(), c.along[u], c.flee_offset[u]);
            if !loose.contains(&id) {
                c.bearing[u] = heading;
            }
        } else if loose.contains(&id) {
            c.position[u] = c.loose_at[u];
        } else {
            if given.timing.is_none() {
                let at = pour::place(&frame, &pourers[in_frame]);
                c.foothold[u] = at;
                c.position[u] = at;
                c.bearing[u] = heading;
            } else {
                walkers.push((u, id, aims[in_frame]));
            }
            in_frame += 1;
        }
    }
    walkers
}

/// FormationWalk.walk_all: each of `walkers` walks a tick towards where it makes for, each
/// reading only its own columns; a fall from a climb face hurts it after, in their order.
fn walk_all(field: &mut Field, on: &Walking, walkers: &[(usize, i64, V2)]) {
    let mut falls = Vec::new();
    for &(u, id, aim) in walkers {
        let c = &mut field.columns;
        let sheet = &c.sheet[u];
        let (width, depth) = (sheet.footprint_width, sheet.footprint_depth);
        let marcher = Marcher {
            climber: Climber {
                walker: crate::pathing::pass::walker(c.objects[u].walker, sheet.height),
                most_stamina: sheet.max_stamina,
                area: (width * depth) as f64,
            },
            turn_rate: sheet.turn_rate,
            backward_pace: sheet.backward_pace,
            radius: reach::radius(width, depth),
        };
        let mut standing = Standing {
            footing: Footing {
                at: c.position[u],
                foothold: c.foothold[u],
                stamina: c.stamina[u],
                on_face: c.on_face[u] != 0,
                climb_left: c.climb_left[u],
            },
            bearing: c.bearing[u],
            speed: c.speed[u],
        };
        let hurt = walk::walk(on, &marcher, &mut standing, aim);
        let f = &standing.footing;
        c.position[u] = f.at;
        c.foothold[u] = f.foothold;
        c.stamina[u] = f.stamina;
        c.on_face[u] = i32::from(f.on_face);
        c.climb_left[u] = f.climb_left;
        c.bearing[u] = standing.bearing;
        if hurt != 0 {
            falls.push((u, id, hurt));
        }
    }
    let mut fell = Vec::new();
    for (u, id, hurt) in falls {
        wounds::hurt(&mut field.columns, u, id, hurt, &mut fell);
    }
    for id in fell {
        field.fell(id);
    }
}
