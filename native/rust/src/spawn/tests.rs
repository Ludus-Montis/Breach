use super::*;
use crate::arms::tests::content;
use crate::maths::V2;

/// Ten tiles east along x.
fn road() -> Route {
    Route::new(vec![V2::new(0.0, 0.0), V2::new(640.0, 0.0)], 2.0)
}

/// A squad of `ids`' definitions, two abreast, spawned on `road()` by `faction` (the
/// player's at the player's end); each definition coded by its first place in `ids`.
fn spawned(ids: &[&str], faction: &str, code: Option<i64>) -> (Squads, Field, usize) {
    let content = content();
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let codes = |id: &str| ids.iter().position(|other| *other == id).unwrap() as i64;
    let placed: Vec<Placed> = (0..ids.len())
        .map(|k| Placed {
            unit: &content.units[ids[k]],
            code: codes(ids[k]),
            place: ((k / 2) as i32, (k % 2) as i32),
        })
        .collect();
    let route = road();
    let spawn = Spawn {
        width: 2,
        placed: &placed,
        faction,
        at_player_end: faction == "player",
        wait_ticks: 4,
        route: &route,
        route_code: code,
        combat_width: 6,
    };
    let s = squad(&mut squads, &mut field, &content, &spawn);
    (squads, field, s)
}

#[test]
fn its_units_ids_and_its_squad_s_are_issued_and_its_command_opened_on_its_route() {
    let (squads, field, s) = spawned(&["grem", "grem", "kingdom_militia"], "player", None);
    let slots: Vec<i32> = (1..=3).map(|id| field.slot_of(id).unwrap() as i32).collect();
    assert_eq!((squads.id[s], squads.units_issued, squads.squads_issued), (1, 3, 1));
    assert_eq!(squads.members[s], slots);
    assert_eq!((squads.command[s], squads.commands.route[0].as_ref()), (Some(0), Some(&road())));
    assert_eq!((squads.commands.home[0], squads.width[s], squads.direction[s]), (0.0, 2, 1));
    let militia = slots[2] as usize;
    let objects = &mut field.columns.objects[militia].clone();
    let codes = ["command", "origin", "definition"].map(|name| *objects.code(name).unwrap());
    assert_eq!((codes, field.columns.sheet[militia].squad_id), ([0, 0, 2], 1));
    assert_eq!(field.lexicon.word(squads.faction[s]), "player");
}

#[test]
fn each_unit_stands_at_its_place_at_full_hp_stamina_and_pace() {
    let content = content();
    let (_, field, _) = spawned(&["grem", "grem", "kingdom_militia"], "player", None);
    let militia = field.slot_of(3).unwrap() as usize;
    let columns = &field.columns;
    let unit = &content.units["kingdom_militia"];
    assert_eq!((columns.rank[militia], columns.column[militia]), (1, 0));
    assert_eq!(i64::from(columns.hp[militia]), unit.hp);
    assert_eq!(columns.regeneration_left[militia], unit.regeneration_limit * unit.hp as f64);
    let sheet = &columns.sheet[militia];
    assert_eq!(
        (columns.stamina[militia], columns.speed[militia]),
        (sheet.max_stamina, sheet.fresh_speed)
    );
    assert_eq!(field.speeds[2], unit.speed);
}

#[test]
fn a_kingdom_squad_starts_at_the_far_end_facing_back_along_its_route() {
    let content = content();
    let (squads, field, s) =
        spawned(&["kingdom_militia", "kingdom_militia"], "the_kingdom", Some(5));
    assert_eq!((squads.home_distance[s], squads.front_distance[s]), (10.0, 10.0));
    assert_eq!((squads.position[s], squads.heading[s]), (V2::new(640.0, 0.0), 270.0));
    assert_eq!((squads.route_code[s], squads.routes_issued), (5, 0));
    assert_eq!((squads.wait_ticks[s], squads.combat_width[s]), (4, 6));
    let unit = &content.units["kingdom_militia"];
    let bands = BandTuning::of_slice(&given::bands(&content.tuning));
    let ceiling = bands::ceiling(&[unit.courage; 2], &[unit.leadership; 2], &bands);
    assert_eq!(i64::from(squads.morale[s]), ceiling);
    let front = field.slot_of(1).unwrap() as usize;
    assert_ne!(field.columns.position[front], V2::default(), "stood on its place");
}
