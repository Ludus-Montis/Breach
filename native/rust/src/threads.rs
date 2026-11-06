//! Whether a pass of the tick runs on Rayon's pool (spec 39 step 10c). A threaded pass gives
//! the same outcome as on one thread, whatever the number of threads, so this only says when
//! the pool is worth its cost: from `FROM` units in the pass, unless told always or never.

/// The least units a pass works on for the pool to take it, with `Threads::Pool`.
pub const FROM: usize = 4096;

/// How the tick's passes use threads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Threads {
    /// Every pass on the calling thread.
    #[default]
    One,
    /// A pass on the pool from `FROM` units.
    Pool,
    /// Every pass that can on the pool, at any size (CI's check that threads change nothing).
    Always,
}

impl Threads {
    /// Whether a pass over `units` runs on the pool.
    pub fn on(self, units: usize) -> bool {
        match self {
            Threads::One => false,
            Threads::Pool => units >= FROM,
            Threads::Always => true,
        }
    }

    /// The tools' `threaded=` argument: `true` (the default) the pool from the threshold,
    /// `false` one thread, `always` the pool at any size. Anything else is `None`.
    pub fn parse(text: Option<&str>) -> Option<Threads> {
        match text {
            None | Some("true") => Some(Threads::Pool),
            Some("false") => Some(Threads::One),
            Some("always") => Some(Threads::Always),
            Some(_) => None,
        }
    }

    /// The game's `BREACH_NATIVE_THREADS`: unset (the default) or anything else the pool from
    /// the threshold, `0` one thread, `always` the pool at any size (spec 39 step 10c-iii).
    pub fn of_env(value: Option<&str>) -> Threads {
        match value {
            Some("0") => Threads::One,
            Some("always") => Threads::Always,
            _ => Threads::Pool,
        }
    }

    /// The tick's argument (FormationEnding.args' third int).
    pub fn code(self) -> i64 {
        match self {
            Threads::One => 0,
            Threads::Pool => 1,
            Threads::Always => 2,
        }
    }

    /// The tick's argument read back: 0 one thread, 2 always, anything else the pool.
    pub fn of_code(code: i64) -> Threads {
        match code {
            0 => Threads::One,
            2 => Threads::Always,
            _ => Threads::Pool,
        }
    }
}

#[cfg(test)]
mod tests;
