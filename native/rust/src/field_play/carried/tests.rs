//! A field's own state carried beside the crate's: a field log played some way in, its state
//! and FieldPlay's written and read back, steps on as the field played on does.

use super::*;
use crate::digest::runs::FIELD_LOGS;
use crate::events::Log;
use crate::field_play::tests::content;
use crate::record;
use crate::snapshot;
use crate::waves::Produced;

/// `record` played to `tick` on one thread, as `actions::replay_each` plays it.
fn played_to(record: &Record, tick: i64, content: &Content) -> (Squads, Field, FieldPlay) {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let at = (record.seed, true, false);
    let mut play = FieldPlay::new(record.captained, (&mut squads, &mut field), content, at)
        .expect("the field sets up");
    while play.play.tick < tick {
        stepped(record, (&mut squads, &mut field, &mut play), content);
    }
    (squads, field, play)
}

/// One tick: the record's commands for it given, then the field stepped; what the builders
/// and waves did and the tick's events.
fn stepped(
    record: &Record,
    (squads, field, play): (&mut Squads, &mut Field, &mut FieldPlay),
    content: &Content,
) -> (Vec<Produced>, Vec<i64>) {
    let tick = play.play.tick;
    for given in record.commands.iter().filter(|c| c.tick == tick) {
        play.apply(given, (squads, field), content);
    }
    let mut log = Log::default();
    let (made, _) = play.step(squads, field, content, &mut log);
    (made, log.take())
}

#[test]
fn a_field_carried_and_read_back_steps_on_as_the_field_played_on() {
    let content = content();
    // field4: B tends "recover", sent at 120 and retreated at 318; cut with B's wave filling.
    let record = record::parse(FIELD_LOGS[4]).expect("a field log");
    for tick in [60, 200] {
        let (mut squads, mut field, mut play) = played_to(&record, tick, &content);
        let bytes = snapshot::write(play.play.tick, play.play.seed, &field, &squads);
        let carried = Carried::read(&play.carried().write()).expect("it reads back");
        let state = snapshot::read(&bytes).expect("the snapshot reads back");
        let mut back =
            FieldPlay::carrying(&record, &carried, (state.tick, true), &content).expect("it fits");
        let (mut kept_squads, mut kept_field) = (state.squads, state.field);
        for _ in 0..140 {
            let on = stepped(&record, (&mut squads, &mut field, &mut play), &content);
            let resumed =
                stepped(&record, (&mut kept_squads, &mut kept_field, &mut back), &content);
            assert_eq!(resumed, on, "from {tick}, tick {}", play.play.tick);
        }
        let ended = snapshot::write(0, 0, &field, &squads);
        assert!(ended == snapshot::write(0, 0, &kept_field, &kept_squads), "from {tick}");
        assert_eq!(back.carried(), play.carried(), "from {tick}");
    }
}

#[test]
fn a_carried_state_that_doesnt_fit_the_field_is_refused() {
    let content = content();
    let record = record::parse(FIELD_LOGS[0]).expect("a field log");
    let (_, _, play) = played_to(&record, 0, &content);
    let mut carried = play.carried();
    carried.reserve.push("dragon".into());
    assert!(FieldPlay::carrying(&record, &carried, (0, true), &content).is_none(), "unknown");
    let mut carried = play.carried();
    carried.waves.pop();
    assert!(FieldPlay::carrying(&record, &carried, (0, true), &content).is_none(), "one short");
}
