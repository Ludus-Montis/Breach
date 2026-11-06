use super::*;

#[test]
fn a_pool_takes_a_pass_only_from_the_threshold() {
    assert!(!Threads::Pool.on(FROM - 1), "a small battle stays on one thread");
    assert!(Threads::Pool.on(FROM), "from the threshold, the pool");
    assert!(!Threads::One.on(FROM * 10), "one thread is one thread at any size");
    assert!(Threads::Always.on(1), "always is the pool at any size");
}

#[test]
fn the_tools_argument_names_each_way() {
    assert_eq!(Threads::parse(None), Some(Threads::Pool), "threads by default");
    assert_eq!(Threads::parse(Some("true")), Some(Threads::Pool));
    assert_eq!(Threads::parse(Some("false")), Some(Threads::One));
    assert_eq!(Threads::parse(Some("always")), Some(Threads::Always));
    assert_eq!(Threads::parse(Some("sometimes")), None);
}

#[test]
fn the_ticks_arguments_carry_each_way_and_back() {
    for threads in [Threads::One, Threads::Pool, Threads::Always] {
        assert_eq!(Threads::of_code(threads.code()), threads);
    }
    assert_eq!(Threads::of_code(0), Threads::One, "0, as GDScript's 'not threaded'");
    assert_eq!(Threads::of_code(1), Threads::Pool, "1, as GDScript's 'threaded'");
}

#[test]
fn the_game_threads_from_the_threshold_unless_told_otherwise() {
    assert_eq!(Threads::of_env(None), Threads::Pool);
    assert_eq!(Threads::of_env(Some("1")), Threads::Pool);
    assert_eq!(Threads::of_env(Some("0")), Threads::One);
    assert_eq!(Threads::of_env(Some("always")), Threads::Always);
    assert_eq!(Threads::of_env(Some("anything")), Threads::Pool);
}
