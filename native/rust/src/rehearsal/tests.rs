//! A march timed by rehearsal: nothing to march is no time, a farther point takes longer, and
//! the ground along the route slows it.

use super::*;
use crate::content::tests::repo_content;
use crate::field_play::{routes, A, B};
use crate::waves::Template;

fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

/// Eight grems abreast, as the field's A wave.
fn grems(content: &Content) -> Vec<Painted> {
    let grem = &content.units["grem"];
    let foot = (grem.footprint_depth as i32, grem.footprint_width as i32);
    Template::line("grem", foot, 8, 8).ordered()
}

fn rehearsed(painted: &[Painted], route: usize, cells: f64, map: Option<&'static str>) -> i64 {
    let content = content();
    let routes = routes();
    let march = March { painted, width: 8, route: &routes[route], cells };
    ticks_to(&march, (0.1, map, false), &content)
}

#[test]
fn nothing_to_march_takes_no_time() {
    assert_eq!(rehearsed(&[], A, 79.0, None), 0);
}

#[test]
fn a_farther_point_takes_longer_and_both_are_reached() {
    let content = content();
    let painted = grems(&content);
    let near = rehearsed(&painted, A, 40.0, None);
    let far = rehearsed(&painted, A, 79.0, None);
    assert!(near > 0, "the front sets out behind the start");
    assert!(far > near, "{far} after {near}");
    assert!(far < LIMIT_TICKS);
}

#[test]
fn the_wood_and_the_stream_on_b_slow_the_march() {
    let content = content();
    let painted = grems(&content);
    let open = rehearsed(&painted, B, 88.0, None);
    let field = rehearsed(&painted, B, 88.0, Some("field"));
    assert!(field > open, "{field} on the field, {open} on open ground");
    assert!(field < LIMIT_TICKS);
}
