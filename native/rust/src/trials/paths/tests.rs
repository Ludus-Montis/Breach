//! PathTrials' and SightTrials' ways, drawings and lines as formation_trials.gd's `paths`
//! plans, draws and prints them, the timings left out.

use super::*;
use crate::content::tests::repo_content;

fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

/// Godot's lines for `scenario=paths repeats=1`, each search's and plan's time set to 0.
const GODOT: &str = include_str!("godot_paths.txt");

#[test]
fn the_ways_and_the_walk_equal_godot_s() {
    let content = content();
    let mut rows = run(1, &content).expect("the field and a grem");
    rows.iter_mut().for_each(|row| row.usec = 0);
    let mut walk = wall::feel_along(&content);
    walk.usec = 0;
    let mut lines = report(&rows, &ground(&content).expect("the field"));
    lines.extend(wall::report(&walk, &content));
    let printed = lines.join("\n");
    let expected: Vec<&str> = GODOT.lines().collect();
    assert_eq!(printed.lines().collect::<Vec<_>>(), expected);
}

#[test]
fn the_ground_is_marked_as_a_bare_walker_crosses_it() {
    let content = content();
    let ground = ground(&content).expect("the field");
    // grass, wood, deep water, the ford's wading water, the hill and the crag
    let marks: String = [(0, 40), (16, 0), (29, 0), (29, 20), (100, 30)]
        .iter()
        .map(|&cell| ground_mark(&ground, cell))
        .collect();
    assert_eq!(marks, ".T~=^");
    assert_eq!(drawing(&[(71, 30)], (70, 30, 4, 1), 1, &ground, &[CRAG]), vec!["#*##"]);
}

#[test]
fn the_loaded_militiaman_neither_swims_nor_climbs_and_the_climber_climbs() {
    let content = packed(&content());
    let walker = |name| walker_of(&content, &unit(&content, name).expect("a definition"));
    let loaded = walker("loaded militiaman");
    assert!(!loaded.swims && !loaded.climbs);
    let cart = walker("cart (sinks)");
    assert!(!cart.swims && !cart.climbs);
    assert_eq!(walker("climber").climber, 2);
    assert!(walker("grem").swims);
}
