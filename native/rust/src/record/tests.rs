//! The record read back as FormationRecord.parse reads it: each version's header, its
//! commands in order, the feel test's words, and lines that aren't commands skipped.

use super::*;

#[test]
fn a_record_reads_its_header_and_commands_in_order() {
    let read = parse("record 2 seed 109563 captain on build 3f6800f43e11\n0 kingdom pursue all off\n88 player send B\n304 player retreat B\n");
    let read = read.expect("a record");
    assert_eq!((read.version, read.seed, read.captained), (2, 109_563, true));
    assert_eq!(read.build, "3f6800f43e11");
    assert_eq!(
        read.commands,
        vec![
            command(0, KINGDOM, "pursue", "all", "off"),
            command(88, PLAYER, "send", "B", ""),
            command(304, PLAYER, "retreat", "B", ""),
        ]
    );
}

#[test]
fn a_version_1_record_has_no_build_and_skips_what_isn_t_a_command() {
    let text = "record 1 seed 492625 captain off\n0 player tend A carry\nnoise\nx player send A\n9 nobody send A\n96 player route A C";
    let read = parse(text).expect("a record");
    assert_eq!(
        (read.version, read.seed, read.captained, read.build.as_str()),
        (1, 492_625, false, "")
    );
    assert_eq!(
        read.commands,
        vec![command(0, PLAYER, "tend", "A", "carry"), command(96, PLAYER, "route", "A", "C")]
    );
}

#[test]
fn a_version_0_log_reads_the_feel_test_s_words() {
    let read = parse("seed 515557 captain off\n0 pursues off\n96 via_c on\n120 send B\n130 wait on\n140 hurry A on\n150 auto B\n").expect("a log");
    assert_eq!((read.version, read.seed), (0, 515_557));
    assert_eq!(
        read.commands,
        vec![
            command(0, KINGDOM, "pursue", "all", "off"),
            command(96, PLAYER, "route", "A", "C"),
            command(120, PLAYER, "send", "B", ""),
            command(130, PLAYER, "wait", "waves", "on"),
            command(140, PLAYER, "hurry", "A", "on"),
        ],
        "auto takes a value"
    );
    assert_eq!(from_words(5, "via_c off"), Some(command(5, PLAYER, "route", "A", "A")));
    assert_eq!(from_words(5, "send"), None);
}

#[test]
fn what_isn_t_a_record_reads_as_none() {
    assert_eq!(parse(""), None);
    assert_eq!(parse("record 2 seed x captain off"), None);
    assert_eq!(parse("hello world"), None);
}
