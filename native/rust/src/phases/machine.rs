//! The `machine:` line `breach_tick_phases` prints (spec 48 step 3): the processor, its cores
//! and threads, the RAM and the system, from the OS. On Linux it reads `/proc/cpuinfo` and
//! `/proc/meminfo`; on Windows it asks the CIM (`Win32_Processor`, `Win32_ComputerSystem`)
//! through PowerShell. A part the OS doesn't give is left out, and a CPU it can't name reads
//! `unknown`, so the line is always printed and the run goes on.

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::process::Command;

/// What the OS told: each part `None` when it didn't.
#[derive(Debug, Default, PartialEq)]
pub struct Found {
    pub cpu: Option<String>,
    pub cores: Option<usize>,
    pub ram_gb: Option<u64>,
}

/// The PowerShell asked on Windows: three lines, the first processor's name, the cores of
/// every processor, and the RAM in bytes.
const WINDOWS_QUERY: &str = "$p=@(Get-CimInstance Win32_Processor); \
     $p[0].Name; ($p | Measure-Object NumberOfCores -Sum).Sum; \
     (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory";

/// The line for this machine.
pub fn machine() -> String {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let (found, system) = match std::env::consts::OS {
        "linux" => (linux(&read("/proc/cpuinfo"), &read("/proc/meminfo")), "Linux"),
        "windows" => (windows(&ask_windows()), "Windows"),
        "macos" => (Found::default(), "macOS"),
        other => (Found::default(), other),
    };
    line(&found, threads, system)
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn ask_windows() -> String {
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", WINDOWS_QUERY])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default()
}

/// "machine: <cpu>, [<cores> cores, ]<threads> threads, [<ram> GB, ]<system>".
pub fn line(found: &Found, threads: usize, system: &str) -> String {
    let mut parts = vec![found.cpu.clone().unwrap_or_else(|| "unknown".to_string())];
    parts.extend(found.cores.map(|cores| format!("{cores} cores")));
    parts.push(format!("{threads} threads"));
    parts.extend(found.ram_gb.map(|gb| format!("{gb} GB")));
    parts.push(system.to_string());
    format!("machine: {}", parts.join(", "))
}

/// The PowerShell answer: name, cores and RAM in bytes, a line each.
pub fn windows(answer: &str) -> Found {
    let mut lines = answer.lines().map(str::trim);
    let cpu = lines.next().filter(|name| !name.is_empty()).map(str::to_string);
    let cores = lines.next().and_then(|n| n.parse().ok()).filter(|&n: &usize| n > 0);
    let bytes = lines.next().and_then(|n| n.parse::<u64>().ok());
    Found { cpu, cores, ram_gb: bytes.and_then(|b| gigabytes(b, 1 << 30)) }
}

/// `/proc/cpuinfo` and `/proc/meminfo`: the first model name, the distinct physical cores
/// (physical id and core id), and MemTotal.
pub fn linux(cpuinfo: &str, meminfo: &str) -> Found {
    let mut cpu = None;
    let mut cores = BTreeSet::new();
    let mut socket = "";
    for (key, value) in cpuinfo.lines().filter_map(field) {
        match key {
            "model name" if cpu.is_none() => cpu = Some(value.to_string()),
            "physical id" => socket = value,
            "core id" => {
                cores.insert((socket, value));
            }
            _ => {}
        }
    }
    let kb = meminfo
        .lines()
        .filter_map(field)
        .find(|(key, _)| *key == "MemTotal")
        .and_then(|(_, value)| value.trim_end_matches("kB").trim().parse::<u64>().ok());
    Found {
        cpu,
        cores: Some(cores.len()).filter(|&n| n > 0),
        ram_gb: kb.and_then(|kb| gigabytes(kb, 1 << 20)),
    }
}

/// "key : value", trimmed.
fn field(line: &str) -> Option<(&str, &str)> {
    line.split_once(':').map(|(key, value)| (key.trim(), value.trim()))
}

/// `amount` in whole GB, rounded, given the units in a GB; `None` for none.
fn gigabytes(amount: u64, per_gb: u64) -> Option<u64> {
    Some((amount + per_gb / 2) / per_gb).filter(|&gb| gb > 0)
}
