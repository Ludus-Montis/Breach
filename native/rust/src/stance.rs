//! A disciplined squad meets a threat as a line (ScrumStance, Decisions 88 and 92, spec 27
//! rounds 5 and 6; spec 39 step 4b): marching or holding, with its front free, when it sees
//! an enemy closing in on another face within `scrum_anticipate` cells and is disciplined
//! enough, it re-lays its places facing the threat at that face (its stance: its frame
//! turned a quarter, half or three quarters about) before contact, and commits to it: it
//! faces that enemy while it is still a threat, not swinging to another. A less disciplined
//! squad meets it unit by unit. Two threats equally near: the squads' seeded draw picks, not
//! the list (Decision 97).

use crate::contest;
use crate::locks::{FIGHTING, MOVING, ROUTING};
use crate::maths::{bearing_vector, is_equal_approx, snapped, V2};
use crate::motion::{bearing_to, fposmod};
use crate::rect::{clamp, Rect};
use crate::squads::Squads;

/// SkirmishSquad.State's holding, SkirmishUnit.Order's hold, and SquadEdges' front.
const HOLDING: i32 = 1;
const HOLD: i32 = 1;
const FRONT: i32 = 0;
const HAIR: f64 = 0.000001;

/// A listed squad as the stance reads it: its store slot, faction, whether it has living
/// units, the cells they cover (SquadEdges.bounds), whether it is disciplined enough to meet
/// threats as a line (FormationDiscipline), and the listed squads it detects
/// (FormationSight), by their places in the list - only as many as it may look at.
pub struct Seen {
    pub squad: usize,
    pub faction: u32,
    pub living: bool,
    pub bounds: Rect,
    pub meets_threats: bool,
    pub detects: Vec<usize>,
}

/// A squad's new stance to take: its place in the list, the heading it turns to and the
/// squad it faces. Its anchor is the face of its footprints towards that heading, which the
/// caller works out (SquadEdges.reach).
#[derive(PartialEq, Debug)]
pub struct Facing {
    pub seen: usize,
    pub heading: f64,
    pub foe: i64,
}

/// Every listed squad, in turn, meets the threats it sees: one letting its stance go has it
/// cleared here; one turning to a new threat is returned, in the lists' order (a stance moves
/// no squad's frame or units, so the turns land after). `rules`: scrum_anticipate and the
/// battle's seed.
pub fn anticipate(squads: &mut Squads, seen: &[Seen], rules: (f64, i64)) -> Vec<Facing> {
    let mut out = Vec::new();
    for n in 0..seen.len() {
        if let Some(facing) = meet(squads, seen, n, rules) {
            out.push(facing);
        }
    }
    out
}

fn meet(squads: &mut Squads, seen: &[Seen], n: usize, rules: (f64, i64)) -> Option<Facing> {
    let squad = seen[n].squad;
    let state = squads.state[squad];
    let able = matches!(state, HOLDING | MOVING | FIGHTING);
    if !able || squads.engaged_with[squad] != 0 {
        return None;
    }
    if !seen[n].meets_threats {
        if state != FIGHTING {
            squads.stance[squad] = None; // it can't hold a re-formed line
        }
        return None;
    }
    if committed(squads, seen, n, rules.0) {
        return None; // it faces that enemy while it is still a threat
    }
    let (threat, foe) = threat(squads, seen, n, rules, true);
    if threat < 0 || threat == FRONT {
        let gone = threat_edge(squads, seen, n, rules) < 0;
        let holds = squads.order[squad] == HOLD && !gone;
        if state != FIGHTING && (!holds || threat == FRONT) {
            squads.stance[squad] = None; // a holding line keeps it while that enemy is near
        }
        return None;
    }
    let heading = fposmod(squads.heading[squad] + threat as f64 * 90.0, 360.0);
    if let Some(stance) = squads.stance[squad] {
        if is_equal_approx(stance.heading, heading) {
            return None;
        }
    }
    Some(Facing { seen: n, heading, foe })
}

fn threat_edge(squads: &Squads, seen: &[Seen], n: usize, rules: (f64, i64)) -> i32 {
    threat(squads, seen, n, rules, false).0
}

/// True if the squad holds a re-formed line towards an enemy still a threat - alive, not
/// routing, within `anticipate` cells, and closing in (or near a line ordered to hold).
fn committed(squads: &Squads, seen: &[Seen], n: usize, anticipate: f64) -> bool {
    let squad = seen[n].squad;
    let Some(stance) = squads.stance[squad] else {
        return false;
    };
    let Some(m) = seen.iter().position(|o| squads.id[o.squad] == stance.foe) else {
        return false;
    };
    if !seen[m].living || squads.state[seen[m].squad] == ROUTING {
        return false;
    }
    let pressing = squads.order[squad] == HOLD || closing(squads, seen, m, n);
    pressing && gap(&seen[n], &seen[m]) <= anticipate
}

/// (edge, squad id): the edge of the squad's frame facing the nearest seen hostile within
/// `anticipate` cells - closing in, if `closing` - or (-1, 0).
fn threat(
    squads: &Squads,
    seen: &[Seen],
    n: usize,
    rules: (f64, i64),
    closing_only: bool,
) -> (i32, i64) {
    let (anticipate, seed) = rules;
    let me = &seen[n];
    let heading = squads.heading[me.squad];
    let centre = me.bounds.center();
    let mut best = (anticipate + HAIR, 0);
    let (mut edge, mut foe) = (-1, 0);
    for &m in &me.detects {
        let other = &seen[m];
        if other.faction == me.faction || !other.living || squads.state[other.squad] == ROUTING {
            continue;
        }
        if closing_only && !closing(squads, seen, m, n) {
            continue;
        }
        let id = squads.id[other.squad];
        let key = (snapped(gap(me, other), 0.000001), contest::squad_draw(seed, id));
        if key < best {
            best = key;
            let point = clamp(centre, other.bounds.position, other.bounds.end());
            let bearing = bearing_to(centre, point, heading);
            edge = ((fposmod(bearing - heading, 360.0) / 90.0).round() as i32).rem_euclid(4);
            foe = id;
        }
    }
    (edge, foe)
}

/// Cells between the two squads' boxes.
fn gap(me: &Seen, other: &Seen) -> f64 {
    let nearest = clamp(other.bounds.center(), me.bounds.position, me.bounds.end());
    nearest.distance_to(clamp(nearest, other.bounds.position, other.bounds.end())) as f64
}

/// True if `seen[m]` is closing in on `seen[n]`: marching with its front towards it, or
/// fighting it. A line holding its ground, or fighting someone else, is no reason to re-form.
fn closing(squads: &Squads, seen: &[Seen], m: usize, n: usize) -> bool {
    let (other, squad) = (seen[m].squad, seen[n].squad);
    let id = squads.id[squad];
    if squads.engaged_with[other] == id || squads.flanked_by(other, id) {
        return true;
    }
    if squads.state[other] != MOVING {
        return false;
    }
    let towards: V2 = seen[n].bounds.center().sub(seen[m].bounds.center());
    towards.dot(bearing_vector(squads.heading[other])) > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::squads::Stance;

    const ANTICIPATE: f64 = 6.0;

    /// Squad `slot` (id slot + 1) of `faction` covering a 4x2 box at `at`, seeing every
    /// other listed squad.
    fn seen(slot: usize, faction: u32, at: (f32, f32), count: usize) -> Seen {
        Seen {
            squad: slot,
            faction,
            living: true,
            bounds: Rect::new(V2::new(at.0, at.1), V2::new(4.0, 2.0)),
            meets_threats: true,
            detects: (0..count).filter(|&m| m != slot).collect(),
        }
    }

    fn store(count: usize) -> Squads {
        let mut squads = Squads::default();
        for id in 1..=count as i64 {
            squads.open(id);
        }
        squads
    }

    #[test]
    fn a_disciplined_squad_turns_to_meet_an_enemy_closing_on_its_flank() {
        let mut squads = store(2);
        squads.heading[1] = 0.0; // marching north, up at squad 1 from the south
        squads.heading[0] = 90.0; // facing east: the foe comes at its right
        let list = [seen(0, 0, (0.0, 0.0), 2), seen(1, 1, (0.0, 5.0), 2)];
        let turns = anticipate(&mut squads, &list, (ANTICIPATE, 3));
        assert_eq!(turns, vec![Facing { seen: 0, heading: 180.0, foe: 2 }]);
    }

    #[test]
    fn an_enemy_ahead_or_holding_its_ground_or_unseen_is_no_reason_to_turn() {
        let mut squads = store(2);
        squads.state[1] = HOLDING;
        let mut list = [seen(0, 0, (0.0, 0.0), 2), seen(1, 1, (0.0, 5.0), 2)];
        assert!(anticipate(&mut squads, &list, (ANTICIPATE, 3)).is_empty());
        squads.state[1] = MOVING;
        list[0].detects.clear();
        assert!(anticipate(&mut squads, &list, (ANTICIPATE, 3)).is_empty());
        list[0].detects = vec![1];
        list[1].bounds.position = V2::new(6.0, 0.0); // east: its front
        squads.heading[1] = 270.0;
        assert!(anticipate(&mut squads, &list, (ANTICIPATE, 3)).is_empty());
    }

    #[test]
    fn a_squad_lets_its_stance_go_once_the_threat_has_gone_unless_ordered_to_hold() {
        let mut squads = store(2);
        let stance = Some(Stance { anchor: V2::ZERO, heading: 180.0, foe: 2 });
        squads.stance[0] = stance;
        squads.state[1] = ROUTING;
        let list = [seen(0, 0, (0.0, 0.0), 2), seen(1, 1, (0.0, 5.0), 2)];
        anticipate(&mut squads, &list, (ANTICIPATE, 3));
        assert_eq!(squads.stance[0], None);
        squads.stance[0] = stance;
        squads.state[1] = HOLDING; // near, not closing
        squads.order[0] = HOLD;
        anticipate(&mut squads, &list, (ANTICIPATE, 3));
        assert_eq!(squads.stance[0], stance); // committed: a holding line keeps it
    }

    #[test]
    fn two_threats_equally_near_go_by_draw_not_the_list() {
        let mut squads = store(3);
        for squad in [1, 2] {
            squads.heading[squad] = if squad == 1 { 0.0 } else { 180.0 };
        }
        let list =
            [seen(0, 0, (0.0, 0.0), 3), seen(1, 1, (0.0, 5.0), 3), seen(2, 1, (0.0, -5.0), 3)];
        let lower = if contest::squad_draw(3, 2) < contest::squad_draw(3, 3) { 2 } else { 3 };
        let turn = &anticipate(&mut squads, &list, (ANTICIPATE, 3))[0];
        assert_eq!(turn.foe, lower);
        let mut squads = store(3);
        squads.heading[1] = 180.0;
        squads.heading[2] = 0.0;
        let swapped =
            [seen(0, 0, (0.0, 0.0), 3), seen(1, 1, (0.0, -5.0), 3), seen(2, 1, (0.0, 5.0), 3)];
        assert_eq!(anticipate(&mut squads, &swapped, (ANTICIPATE, 3))[0].foe, lower);
    }
}
