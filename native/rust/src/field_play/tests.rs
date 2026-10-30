//! The field's waves played in the crate: its waves and builders as FormationField makes
//! them, a tick that builds before the battle's, a wave sent whole or closed up and centred,
//! the player's squads hurried and tending by route, and its routers home banked.

use super::*;
use crate::content::tests::repo_content;
use crate::events::KINDS;

pub(super) fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

pub(super) fn fresh(content: &Content) -> (Squads, Field, FieldPlay) {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let play = FieldPlay::new(false, (&mut squads, &mut field), content, (1, true, false));
    (squads, field, play.expect("the field sets up"))
}

/// Plays `ticks` ticks, dropping the battle's events; what the builders and waves did.
pub(super) fn played(
    (squads, field, play): (&mut Squads, &mut Field, &mut FieldPlay),
    content: &Content,
    ticks: usize,
) -> Vec<Produced> {
    let mut produced = Vec::new();
    for _ in 0..ticks {
        let mut log = Log::default();
        produced.extend(play.step(squads, field, content, &mut log).0);
    }
    produced
}

#[test]
fn the_field_has_a_line_of_grems_on_a_and_b_led_by_a_chieftain() {
    let content = content();
    let (squads, _, play) = fresh(&content);
    assert_eq!(play.play.listed.len(), 2, "the kingdom's line and reserve");
    assert_eq!(play.waves[A].template.ordered().len(), 8);
    let b = play.waves[B].template.ordered();
    assert_eq!(b.last().map(|p| (p.id, p.at)), Some(("grem_chieftain", (1, 4))));
    assert_eq!(play.routes[A].1, Some(squads.route_code[0]));
    assert_eq!(play.routes[B].1, None, "coded when first used");
    let kinds: Vec<(&str, usize)> = play.builders.kinds.iter().map(|k| (k.0, k.2.len())).collect();
    assert_eq!(kinds, vec![("grem", BUILDERS), ("grem_chieftain", 1)]);
    assert!(play.builders.round_robin);
}

#[test]
fn the_builders_fill_both_waves_in_turn_and_each_is_announced_full_once() {
    let content = content();
    let (mut squads, mut field, mut play) = fresh(&content);
    let produced = played((&mut squads, &mut field, &mut play), &content, 600);
    let fulls = produced.iter().filter(|p| matches!(p, Produced::WaveFull { .. })).count();
    assert_eq!(fulls, 2);
    assert!(play.waves.iter().all(Wave::full));
    assert_eq!(play.play.tick, 600);
    assert_eq!(play.play.listed.len(), 2, "nothing departs by hand unasked");
}

#[test]
fn a_partial_wave_is_sent_closed_up_and_centred_on_its_route() {
    let content = content();
    let (mut squads, mut field, mut play) = fresh(&content);
    let mut ticks = 0;
    while play.waves[A].built() < 3 {
        played((&mut squads, &mut field, &mut play), &content, 1);
        ticks += 1;
        assert!(ticks < 1000, "the builders fill A");
    }
    let built = play.waves[A].built();
    let s = play.send(A, 0, (&mut squads, &mut field), &content).expect("a squad sent");
    assert_eq!(squads.members[s].len() as i64, built);
    assert_eq!(i64::from(squads.width[s]), built, "as wide as its front band");
    assert_eq!(squads.centre_shift[s], 0.0, "abreast from column 0: centred");
    assert_eq!(play.waves[A].built(), 0, "emptied");
    assert_eq!(play.play.listed.last(), Some(&s));
    assert_eq!(play.routes[A].1, Some(squads.route_code[s]), "on the kingdom's route A");
    assert!(play.send(A, 0, (&mut squads, &mut field), &content).is_none(), "nothing built");
}

#[test]
fn an_automatic_wave_departs_as_it_is_announced_full() {
    let content = content();
    let (mut squads, mut field, mut play) = fresh(&content);
    play.waves[B].auto = true;
    let produced = played((&mut squads, &mut field, &mut play), &content, 600);
    let departed = produced.iter().find(|p| matches!(p, Produced::Departed { .. }));
    assert!(matches!(departed, Some(Produced::Departed { units: 9, .. })));
    assert!(play.play.listed.len() >= 3, "sent, and again each time it fills");
    let s = play.play.listed[2];
    assert_eq!(play.routes[B].1, Some(squads.route_code[s]), "B coded as first used");
    assert_ne!(play.routes[B].1, play.routes[A].1);
}

#[test]
fn the_player_s_squads_hurry_and_tend_as_their_route_s_waves_do() {
    let content = content();
    let (mut squads, mut field, mut play) = fresh(&content);
    played((&mut squads, &mut field, &mut play), &content, 30);
    let s = play.send(A, 0, (&mut squads, &mut field), &content).expect("a squad sent");
    play.hurried = vec![A];
    play.tending = vec![(B, "carry".into()), (A, "recover".into())];
    played((&mut squads, &mut field, &mut play), &content, 1);
    let command = squads.command[s].unwrap();
    assert!(squads.commands.gives(command, HURRY));
    assert_eq!(squads.commands.tends[command], "recover");
    let line = squads.command[0].unwrap();
    assert!(!squads.commands.gives(line, HURRY), "the kingdom's isn't the player's");
    play.hurried.clear();
    played((&mut squads, &mut field, &mut play), &content, 1);
    assert!(!squads.commands.gives(command, HURRY));
}

#[test]
fn the_player_s_routers_home_go_back_to_the_reserve() {
    let content = content();
    let (squads, mut field, mut play) = fresh(&content);
    let player = field.lexicon.code("player");
    let kingdom = field.lexicon.find("the_kingdom").unwrap();
    let mut log = Log::default();
    log.squad(FLED_HOME, 1, &squads, 0, &[7, 0]); // the kingdom's: not banked
    let mut flat = log.take();
    flat.extend([FLED_HOME as i64, 1, 9, i64::from(player), 8, 0]);
    assert_eq!(KINDS[FLED_HOME].1.len(), 2);
    assert_eq!(flat[3], i64::from(kingdom));

    play.bank(&field, &flat);

    assert_eq!(play.builders.reserve, vec!["kingdom_militia"], "definition 0");
}
