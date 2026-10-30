//! The record of play read back (FormationRecord, Decision 115, spec 39 step 8c-ix): a header
//! naming the format's version, the battle's seed and set-up and the build that played it,
//! then one command a line (its tick, the side giving it, the order, its target and a value
//! where the order takes one). A version 0 log (the feel test's words, `from_words`) and a
//! version 1 record (no build) still read.

#[cfg(test)]
mod tests;

pub const PLAYER: &str = "player";
pub const KINGDOM: &str = "kingdom";

/// A command: its tick, who gives it, the order, its target and its value ("" where it takes
/// none).
#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub tick: i64,
    pub who: String,
    pub order: String,
    pub target: String,
    pub value: String,
}

/// A record: its version, seed, whether the line is captained, the build that played it (""
/// if not stamped) and its commands in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub version: i64,
    pub seed: i64,
    pub captained: bool,
    pub build: String,
    pub commands: Vec<Command>,
}

/// FormationRecord.command.
pub fn command(tick: i64, who: &str, order: &str, target: &str, value: &str) -> Command {
    let owned = |s: &str| s.to_string();
    Command {
        tick,
        who: owned(who),
        order: owned(order),
        target: owned(target),
        value: owned(value),
    }
}

/// FormationRecord.from_words: the feel test's words ("send A", "via_c on", "pursues off",
/// ...) as a command at `tick`; None if they aren't the feel test's.
pub fn from_words(tick: i64, words: &str) -> Option<Command> {
    let parts: Vec<&str> = words.split(' ').filter(|w| !w.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    let on = parts[parts.len() - 1];
    match parts[0] {
        "send" | "retreat" => Some(command(tick, PLAYER, parts[0], parts[1], "")),
        "auto" | "hurry" | "tend" if parts.len() == 3 => {
            Some(command(tick, PLAYER, parts[0], parts[1], on))
        }
        "wait" => Some(command(tick, PLAYER, "wait", "waves", on)),
        "via_c" => Some(command(tick, PLAYER, "route", "A", if on == "on" { "C" } else { "A" })),
        "pursues" => Some(command(tick, KINGDOM, "pursue", "all", on)),
        _ => None,
    }
}

/// FormationRecord.parse: the record `text` holds; None if it isn't one. Lines that aren't
/// commands are skipped.
pub fn parse(text: &str) -> Option<Record> {
    let lines: Vec<&str> = text.trim().split('\n').filter(|l| !l.is_empty()).collect();
    let mut head: Vec<&str> = lines.first().map_or(Vec::new(), |l| words(l));
    let mut version = 0;
    if head.len() >= 6 && head[0] == "record" && is_int(head[1]) {
        version = head[1].parse().ok()?;
        head.drain(..2);
    }
    if head.len() < 2 || head[0] != "seed" || !is_int(head[1]) {
        return None;
    }
    let seed = head[1].parse().ok()?;
    let captained = head.len() > 3 && head[3] == "on";
    let build = if head.len() > 5 && head[4] == "build" { head[5] } else { "" };
    let commands = lines.iter().skip(1).filter_map(|l| read(l, version)).collect();
    Some(Record { version, seed, captained, build: build.to_string(), commands })
}

/// FormationRecord._read: one line as a command, read as `version` wrote it.
fn read(line: &str, version: i64) -> Option<Command> {
    let line = line.trim();
    let (tick, rest) = line.split_once(' ')?;
    let rest = rest.trim_start_matches(' ');
    if rest.is_empty() || !is_int(tick) {
        return None;
    }
    let tick = tick.parse().ok()?;
    if version == 0 {
        return from_words(tick, rest);
    }
    let w = words(rest);
    if w.len() < 3 || ![PLAYER, KINGDOM].contains(&w[0]) {
        return None;
    }
    Some(command(tick, w[0], w[1], w[2], w.get(3).copied().unwrap_or("")))
}

/// The words of `line`, empty ones dropped (`split(" ", false)`).
fn words(line: &str) -> Vec<&str> {
    line.trim().split(' ').filter(|w| !w.is_empty()).collect()
}

/// String.is_valid_int: an optional sign, then digits.
fn is_int(word: &str) -> bool {
    let digits = word.strip_prefix(['+', '-']).unwrap_or(word);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}
