//! The crate's log and the field's production as the digest's events: each the dictionary
//! CrateEvents (or DomainProduction, FormationProduction) gives, as DigestState.plain gives it.

use super::*;
use crate::events::{
    Log, CAME_TO, FLED_HOME, GAVE_UP, HIT, MORALE_BAND, ORDER_APPLIED, SWAPPED, TURNING,
};
use crate::squads::Squads;

/// A squad of id 7, of the kingdom, and its lexicon.
fn kingdom() -> (Squads, Lexicon) {
    let mut lexicon = Lexicon::default();
    let mut squads = Squads::default();
    squads.open(7);
    squads.faction[0] = lexicon.code("kingdom");
    (squads, lexicon)
}

/// The dictionary of an event of `kind` told of squad 7 on tick 4, with `fields`.
fn told(kind: &str, fields: Vec<(&str, Plain)>) -> Plain {
    let mut entries = vec![
        (Plain::str("type"), Plain::str(kind)),
        (Plain::str("tick"), Plain::Int(4)),
        (Plain::str("squad"), Plain::Int(7)),
        (Plain::str("faction"), Plain::str("kingdom")),
    ];
    entries.extend(fields.into_iter().map(|(name, value)| (Plain::str(name), value)));
    Plain::dict(entries)
}

#[test]
fn an_order_a_band_and_a_heading_are_decoded_as_crate_events_decodes_them() {
    let (squads, lexicon) = kingdom();
    let mut log = Log::default();
    log.squad(ORDER_APPLIED, 4, &squads, 0, &[1]);
    log.squad(MORALE_BAND, 4, &squads, 0, &[2, 35]);
    log.squad(TURNING, 4, &squads, 0, &[-1, 0.3_f64.to_bits() as i64]);
    let expected = vec![
        told("order_applied", vec![("order", Plain::str("HOLD"))]),
        told("morale_band", vec![("band", Plain::str("WAVERING")), ("morale", Plain::Int(35))]),
        told("turning", vec![("facing", Plain::Int(-1)), ("heading", Plain::Float(0.3))]),
    ];
    assert_eq!(logged(&log.take(), &lexicon), expected);
}

#[test]
fn a_unit_led_event_names_its_unit_and_its_blow_and_flank() {
    let (_, mut lexicon) = kingdom();
    let player = lexicon.code("player");
    let mut log = Log::default();
    log.unit(HIT, 4, 101, player, &[200, 7, 1, 3, 13]);
    let expected = Plain::dict(vec![
        (Plain::str("type"), Plain::str("hit")),
        (Plain::str("tick"), Plain::Int(4)),
        (Plain::str("unit"), Plain::Int(101)),
        (Plain::str("faction"), Plain::str("player")),
        (Plain::str("target"), Plain::Int(200)),
        (Plain::str("dmg"), Plain::Int(7)),
        (Plain::str("flank"), Plain::Bool(true)),
        (Plain::str("blow"), Plain::str("hit")),
        (Plain::str("target_hp"), Plain::Int(13)),
    ]);
    assert_eq!(logged(&log.take(), &lexicon), vec![expected]);
}

#[test]
fn words_lists_an_unset_joined_and_a_definition_are_decoded_as_crate_events_decodes_them() {
    let (squads, mut lexicon) = kingdom();
    let go = i64::from(lexicon.code("go"));
    let mut log = Log::default();
    log.squad(GAVE_UP, 4, &squads, 0, &[go]);
    log.squad_listed(SWAPPED, (4, &squads, 0), &[101], &[102, 103]);
    log.squad(CAME_TO, 4, &squads, 0, &[101, 1, -1]);
    log.squad(CAME_TO, 4, &squads, 0, &[101, 0, 9]);
    log.squad(FLED_HOME, 4, &squads, 0, &[101, 2]);
    let expected = vec![
        told("gave_up", vec![("then", Plain::str("go"))]),
        told(
            "swapped",
            vec![
                ("unit", Plain::Int(101)),
                ("passed", Plain::List(vec![Plain::Int(102), Plain::Int(103)])),
            ],
        ),
        told("came_to", vec![("unit", Plain::Int(101)), ("alone", Plain::Bool(true))]),
        told(
            "came_to",
            vec![
                ("unit", Plain::Int(101)),
                ("alone", Plain::Bool(false)),
                ("joined", Plain::Int(9)),
            ],
        ),
        told(
            "fled_home",
            vec![
                ("unit", Plain::Int(101)),
                ("definition", Plain::List(vec![Plain::str("obj"), Plain::str("Resource")])),
            ],
        ),
    ];
    assert_eq!(logged(&log.take(), &lexicon), expected);
}

#[test]
fn the_fields_production_is_told_as_the_builders_and_the_waves_tell_it() {
    let made = [
        Produced::FromReserve { tick: 3, lane: 1 },
        Produced::Built { tick: 3, lane: 0 },
        Produced::ToReserve { tick: 3 },
        Produced::WaveFull { tick: 3, built: 8 },
        Produced::Departed { tick: 3, units: 8 },
    ];
    let event = |kind: &str, field: &str, value: Plain| {
        Plain::dict(vec![
            (Plain::str("type"), Plain::str(kind)),
            (Plain::str("tick"), Plain::Int(3)),
            (Plain::str("faction"), Plain::str("player")),
            (Plain::str(field), value),
        ])
    };
    let expected = vec![
        event("from_reserve", "lane", Plain::str("B")),
        event("built", "lane", Plain::str("A")),
        event("to_reserve", "lane", Plain::str("")),
        event("wave_full", "built", Plain::Int(8)),
        event("departed", "units", Plain::Int(8)),
    ];
    assert_eq!(produced(&made), expected);
}
