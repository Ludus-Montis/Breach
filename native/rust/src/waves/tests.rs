//! The waves and the builders: a template painted and laid out, a wave filled front-first,
//! and builders sharing in turn, claiming, and building for the reserve.

use super::*;

const ONE: (i32, i32) = (1, 1);

fn line(count: i32) -> Template {
    Template::line("grem", ONE, count, 8)
}

#[test]
fn a_line_is_as_wide_as_allowed_then_deeper() {
    let template = Template::line("grem", ONE, 5, 3);
    let at: Vec<(i32, i32)> = template.ordered().iter().map(|p| p.at).collect();
    assert_eq!(at, vec![(0, 0), (0, 1), (0, 2), (1, 0), (1, 1)]);
}

#[test]
fn painting_replaces_what_it_overlaps_is_pulled_inside_and_keeps_to_the_slots() {
    let mut template = line(8);
    assert!(!template.paint("chief", ONE, (1, 4)), "no slot left");
    template.slot_limit += 1;
    assert!(template.paint("chief", ONE, (1, 4)));
    assert!(!template.paint("ogre", (2, 2), (0, 7)), "7 cells kept and its 4 are more than 9");
    template.slot_limit = 16;
    assert!(template.paint("ogre", (2, 2), (0, 7)), "pulled back to column 6");
    let ordered = template.ordered();
    assert_eq!(ordered.len(), 8, "the ogre replaced the grems at columns 6 and 7");
    assert_eq!(ordered.last().map(|p| (p.id, p.at)), Some(("chief", (1, 4))));
    assert!(ordered.iter().any(|p| p.id == "ogre" && p.at == (0, 6)));
    assert!(!template.paint("grem", ONE, (4, 0)), "off the grid");
}

#[test]
fn a_layout_starts_its_columns_at_0_and_is_as_wide_as_it_spans() {
    let mut template = Template::new(8, 8);
    template.paint("grem", ONE, (0, 3));
    template.paint("grem", (1, 2), (1, 4));
    let (width, placed) = template.layout();
    assert_eq!(width, 3);
    assert_eq!(placed.iter().map(|p| p.at).collect::<Vec<_>>(), vec![(0, 0), (1, 1)]);
    assert_eq!(Template::new(8, 8).layout(), (1, Vec::new()));
}

#[test]
fn a_wave_fills_its_front_most_place_of_a_type_till_it_is_full() {
    let mut template = line(2);
    template.slot_limit += 1;
    template.paint("chief", ONE, (1, 0));
    let mut wave = Wave::new(template, 0);
    assert_eq!((wave.wanting("grem"), wave.wanting("chief")), (2, 1));
    assert!(wave.fill("chief"));
    assert!(!wave.fill("chief"));
    assert!(wave.fill("grem"));
    assert_eq!(wave.filled, vec![true, false, true]);
    assert!(!wave.full());
    wave.fill("grem");
    assert!(wave.full());
    assert_eq!(wave.built(), 3);
}

/// Builders of grems taking `ticks` ticks a build (at 0.1 s a tick).
fn builders(count: usize, ticks: f64) -> Builders {
    let mut builders = Builders::default();
    builders.set("grem", ticks / 10.0, count);
    builders.prefer(None);
    builders
}

#[test]
fn builders_share_the_lanes_in_turn_and_never_claim_a_place_twice() {
    let mut lanes = vec![Wave::new(line(2), 0), Wave::new(line(2), 1)];
    let mut builders = builders(3, 2.0);
    let mut out = Vec::new();
    builders.step(&mut lanes, 0.1, 0, &mut out);
    let claims: Vec<Claim> = builders.kinds[0].2.iter().map(|b| b.claim).collect();
    assert_eq!(claims, vec![Claim::Lane(0), Claim::Lane(1), Claim::Lane(0)]);
    assert!(out.is_empty());
    builders.step(&mut lanes, 0.1, 1, &mut out);
    assert_eq!(
        out,
        vec![
            Produced::Built { tick: 1, lane: 0 },
            Produced::Built { tick: 1, lane: 1 },
            Produced::Built { tick: 1, lane: 0 }
        ]
    );
    assert!(lanes[0].full());
}

#[test]
fn with_no_lane_wanting_them_builders_build_for_the_reserve_up_to_its_cap() {
    let mut lanes = vec![Wave::new(line(1), 0)];
    let mut builders = builders(3, 1.0);
    builders.reserve_cap = 1;
    let mut out = Vec::new();
    builders.step(&mut lanes, 0.1, 0, &mut out);
    let claims: Vec<Claim> = builders.kinds[0].2.iter().map(|b| b.claim).collect();
    assert_eq!(claims, vec![Claim::Idle; 3], "each finished at once");
    assert_eq!(
        out,
        vec![Produced::Built { tick: 0, lane: 0 }, Produced::ToReserve { tick: 0 }]
    );
    assert_eq!(builders.reserve, vec!["grem"]);
}

#[test]
fn the_reserve_fills_the_lanes_first() {
    let mut lanes = vec![Wave::new(line(1), 0)];
    let mut builders = builders(0, 1.0);
    builders.bank("grem");
    builders.bank("grem");
    let mut out = Vec::new();
    builders.step(&mut lanes, 0.1, 3, &mut out);
    assert_eq!(out, vec![Produced::FromReserve { tick: 3, lane: 0 }]);
    assert_eq!(builders.reserve.len(), 1);
}

#[test]
fn fewer_builders_keep_those_furthest_along() {
    let mut builders = builders(3, 9.0);
    builders.kinds[0].2[1].ticks = 5;
    builders.set("grem", 0.9, 1);
    assert_eq!(builders.kinds[0].2.iter().map(|b| b.ticks).collect::<Vec<_>>(), vec![5]);
}
