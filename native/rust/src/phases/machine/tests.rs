use super::*;

/// The Windows answer the owner's PC gives (spec 48, The machine): name, cores, RAM in bytes.
const OWNER_PC: &str = "12th Gen Intel(R) Core(TM) i7-12700K\r\n12\r\n34126680064\r\n";

#[test]
fn a_windows_answer_names_the_cpu_its_cores_and_ram() {
    let found = windows(OWNER_PC);
    assert_eq!(found.cpu.as_deref(), Some("12th Gen Intel(R) Core(TM) i7-12700K"));
    assert_eq!(found.cores, Some(12));
    assert_eq!(found.ram_gb, Some(32));
    assert_eq!(
        line(&found, 20, "Windows"),
        "machine: 12th Gen Intel(R) Core(TM) i7-12700K, 12 cores, 20 threads, 32 GB, Windows"
    );
}

#[test]
fn no_answer_says_unknown_and_the_line_goes_on() {
    for answer in ["", "\r\n", "  \n\n\n"] {
        let found = windows(answer);
        assert_eq!(found, Found::default());
        assert_eq!(line(&found, 20, "Windows"), "machine: unknown, 20 threads, Windows");
    }
}

#[test]
fn a_partial_answer_keeps_what_it_has() {
    let found = windows("Some CPU\nnot a number\n");
    assert_eq!(found.cpu.as_deref(), Some("Some CPU"));
    assert_eq!(found.cores, None);
    assert_eq!(found.ram_gb, None);
    assert_eq!(line(&found, 4, "Windows"), "machine: Some CPU, 4 threads, Windows");
}

#[test]
fn linux_reads_cpuinfo_and_meminfo() {
    let cpuinfo = "processor\t: 0\nmodel name\t: AMD EPYC 7763 64-Core Processor\nphysical id\t: 0\n\
                   core id\t: 0\nprocessor\t: 1\nmodel name\t: AMD EPYC 7763 64-Core Processor\n\
                   physical id\t: 0\ncore id\t: 0\nprocessor\t: 2\nphysical id\t: 0\ncore id\t: 1\n";
    let meminfo = "MemTotal:       16365240 kB\nMemFree:  100 kB\n";
    let found = linux(cpuinfo, meminfo);
    assert_eq!(found.cpu.as_deref(), Some("AMD EPYC 7763 64-Core Processor"));
    assert_eq!(found.cores, Some(2));
    assert_eq!(found.ram_gb, Some(16));
    assert_eq!(
        line(&found, 4, "Linux"),
        "machine: AMD EPYC 7763 64-Core Processor, 2 cores, 4 threads, 16 GB, Linux"
    );
}
