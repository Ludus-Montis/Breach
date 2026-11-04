//! The tick's end as one call: on a squad built here (squad 1, one column wide, marching
//! under its own command at front 5.0, heading 90, anchored at (10, 4), re-forming; unit
//! 100 preferring the back in its front rank and unit 101 behind it, made by hand with no
//! fresh speed on its sheet, both lying on the origin), it leaves the squads, the columns
//! and the log as stamina, the parting, the shuffle and the sync called one after another
//! do.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::events::SWAPPED;
use crate::locks::MOVING;
use crate::maths::V2;

const TICK: i64 = 5;
const SEED: i64 = 3;
/// UnitDef.Position.BACK.
const BACK: i64 = 2;

pub(crate) fn rules() -> Rules {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    let t = &content.tuning;
    Rules {
        tick: TICK,
        seconds: 0.1,
        seed: SEED,
        stamina: StaminaTuning::of(t),
        parting: PartingTuning {
            passes: t.bodies_passes,
            resist: t.bodies_resist,
            brush: t.bodies_brush,
        },
        threaded: false,
        shuffle: [0.1, 0.125, t.scrum_crowding],
        timing: [0.1, 0.125],
        walk: crate::march::tests::walk_tuning(),
        wounds_reach: t.wounds_reach,
    }
}

struct Built {
    squads: Squads,
    field: Field,
}

fn built() -> Built {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    squads.open(1);
    squads.state[0] = MOVING;
    squads.command[0] = Some(squads.commands.open(0.0));
    squads.width[0] = 1;
    squads.combat_width[0] = 1;
    squads.direction[0] = 1;
    squads.front_distance[0] = 5.0;
    squads.heading[0] = 90.0;
    squads.position[0] = V2::new(10.0, 4.0);
    squads.reforming[0] = 1;
    for (rank, (id, band)) in [(100, BACK), (101, 0)].into_iter().enumerate() {
        let slot = field.hold(id) as usize;
        let columns = &mut field.columns;
        columns.rank[slot] = rank as i32;
        columns.speed[slot] = 1.0;
        (columns.stamina[slot], columns.condition[slot]) = (100.0, 1.0);
        let sheet = &mut columns.sheet[slot];
        (sheet.footprint_width, sheet.footprint_depth) = (1, 1);
        sheet.preferred_position = band;
        sheet.fresh_speed = if id == 101 { 0.0 } else { 1.0 };
        squads.members[0].push(slot as i32);
    }
    Built { squads, field }
}

impl Built {
    /// The four passes one call each, as FormationSimulation made them.
    fn phased(&mut self, rules: &Rules, log: &mut Log) -> Ended {
        let (squads, field) = (&mut self.squads, &mut self.field);
        let made = stamina::pass::step(squads, field, &[0], rules.seconds, &rules.stamina);
        let parts = (&rules.parting, rules.threaded);
        let parted = parting::pass::step(squads, field, &[0], rules.seed, parts);
        swaps::pass::step(squads, field, &[0], (rules.shuffle, rules.tick), log);
        let given = Given {
            ground: None,
            timing: Some(rules.timing),
            seed: rules.seed,
            wounds_reach: rules.wounds_reach,
            tuning: rules.walk,
        };
        sync::step(squads, field, &[0], &given);
        Ended { made, parting_usec: 0, passes: parted.passes, pairs: parted.pairs }
    }

    fn whole(&mut self, rules: &Rules, log: &mut Log) -> Ended {
        let ended = step(&mut self.squads, &mut self.field, &[0], None, rules, log);
        Ended { parting_usec: 0, ..ended }
    }

    /// Each unit's place, foothold, bearing, distance, rank, stamina and sheet's fresh speed.
    fn units(&self) -> Vec<(V2, V2, f64, f64, i32, f64, f64)> {
        let c = &self.field.columns;
        self.squads.members[0]
            .iter()
            .map(|&s| s as usize)
            .map(|s| {
                let fresh = c.sheet[s].fresh_speed;
                (
                    c.position[s],
                    c.foothold[s],
                    c.bearing[s],
                    c.distance[s],
                    c.rank[s],
                    c.stamina[s],
                    fresh,
                )
            })
            .collect()
    }
}

#[test]
fn the_one_call_leaves_the_squads_columns_and_log_as_the_passes_one_by_one() {
    let rules = rules();
    let (mut phased, mut whole) = (built(), built());
    let (mut once, mut all) = (Log::default(), Log::default());

    let mut ends = vec![];
    for _ in 0..30 {
        ends.push((phased.phased(&rules, &mut once), whole.whole(&rules, &mut all)));
    }

    for (by_pass, by_call) in &ends {
        assert_eq!(by_call, by_pass);
    }
    let logged = all.take();
    assert_eq!(logged, once.take());
    assert_eq!(whole.squads, phased.squads);
    assert_eq!(whole.units(), phased.units());
    assert_eq!(ends[0].1.made, vec![101], "the unit made by hand took its speed as fresh");
    assert!(ends[0].1.pairs > 0, "the two lying on each other were parted");
    assert_eq!(logged.first().copied(), Some(SWAPPED as i64), "the swap ended and was told");
    assert_ne!(whole.units()[0].0, V2::ZERO, "the sync walked them toward their places");
}
