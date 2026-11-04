use super::*;
use crate::content::tests::repo_content;

fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

/// `battle` set up on a fresh crate.
fn set(battle: &Battle) -> (Squads, Field, Option<Set>) {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let set = set_up(battle, &mut squads, &mut field, &content());
    (squads, field, set)
}

/// Each squad's faction and its units' definitions by code, in slot order.
fn sides(squads: &Squads, field: &Field) -> Vec<(String, Vec<i64>)> {
    (0..squads.count())
        .map(|s| {
            let codes = squads.members[s].iter().map(|&m| {
                let objects = &mut field.columns.objects[m as usize].clone();
                *objects.code("definition").unwrap()
            });
            (field.lexicon.word(squads.faction[s]).to_string(), codes.collect())
        })
        .collect()
}

#[test]
fn the_head_on_mirror_is_eight_grems_a_side_from_either_end_of_the_lane() {
    let (squads, field, set) = set(&Battle::Mirror { flank: false, swap: false });
    let grems = vec![0; 8];
    let want = vec![("player".to_string(), grems.clone()), ("the_kingdom".to_string(), grems)];
    assert_eq!(sides(&squads, &field), want);
    assert_eq!((squads.home_distance[0], squads.home_distance[1]), (0.0, 2.0));
    assert_eq!(squads.route_code[0], squads.route_code[1]);
    assert!(squads.pending.is_empty());
    let open = Set { route_tiles: 2.0, tick_seconds: 0.1, combat_width: 0, map: None };
    assert_eq!(set, Some(open));
}

#[test]
fn the_flank_mirror_s_line_holds_on_its_route_and_the_grems_come_down_onto_it() {
    let (squads, field, _) = set(&Battle::Mirror { flank: true, swap: false });
    assert_eq!(sides(&squads, &field)[0].0, "the_kingdom");
    assert_eq!(squads.pending, vec![(squads.id[0], HOLD)]);
    assert_ne!(squads.route_code[0], squads.route_code[1]);
    assert_eq!(squads.position[1], V2::new(64.5, 0.0));
    let (swapped, field, _) = self::set(&Battle::Mirror { flank: true, swap: true });
    assert_eq!(sides(&swapped, &field)[0].0, "player");
    assert_eq!(swapped.pending, vec![(swapped.id[1], HOLD)]);
}

#[test]
fn a_clash_s_back_ranks_are_spitters_coded_as_they_are_first_met() {
    let (squads, field, _) = set(&Battle::Clash { per_side: 6, width: 4, archers: 2 });
    let want = vec![
        ("player".to_string(), vec![0, 0, 0, 0, 1, 1]),
        ("the_kingdom".to_string(), vec![2; 6]),
    ];
    assert_eq!(sides(&squads, &field), want);
    let last = squads.members[0][5] as usize;
    assert_eq!((field.columns.rank[last], field.columns.column[last]), (1, 1));
    assert_eq!(squads.width[0], 4);
}

#[test]
fn the_field_s_line_and_reserve_hold_where_they_stand_on_route_a() {
    let (squads, field, set) = set(&Battle::Field { captained: true });
    let mut line = vec![0; 12];
    line[9] = 1; // the captain, in the second rank's middle
    let want = vec![("the_kingdom".to_string(), line), ("the_kingdom".to_string(), vec![0; 6])];
    assert_eq!(sides(&squads, &field), want);
    assert_eq!((squads.front_distance[0], squads.front_distance[1]), (1.25, 1.5));
    assert_eq!(
        (squads.position[0], squads.position[1]),
        (V2::new(80.0, 32.0), V2::new(96.0, 32.0))
    );
    assert_eq!(squads.pending, vec![(squads.id[0], HOLD), (squads.id[1], HOLD)]);
    assert_eq!(squads.route_code[0], squads.route_code[1]);
    let set = set.unwrap();
    assert_eq!((set.combat_width, set.map, squads.combat_width[1]), (8, Some("field"), 8));
}

#[test]
fn nothing_is_set_up_for_a_definition_that_isn_t_there() {
    let mut content = content();
    content.units.remove("grem_spitter");
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let battle = Battle::Clash { per_side: 4, width: 4, archers: 1 };
    assert_eq!(set_up(&battle, &mut squads, &mut field, &content), None);
    assert_eq!((squads.count(), squads.units_issued), (0, 0));
}

#[test]
fn battles_are_named_as_the_digest_names_them() {
    let clash = named("clash", &[40, 10]);
    assert!(matches!(clash, Some(Battle::Clash { per_side: 40, width: 10, archers: 0 })));
    assert!(matches!(
        named("mirror_flank", &[1]),
        Some(Battle::Mirror { flank: true, swap: true })
    ));
    assert!(matches!(named("field", &[]), Some(Battle::Field { captained: false })));
    assert!(named("lanes", &[]).is_none());
}
