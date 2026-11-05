# Builds Breach's native core (Decision 129: Rust through GDExtension, godot-rust) on
# Windows into native\bin\ (git-ignored), as native/build.sh does elsewhere. See
# native/README.md.
#
#   powershell -ExecutionPolicy Bypass -File native\build.ps1 [release|dev]
#
# Needs Rust (https://rustup.rs). Either toolchain works: the default MSVC one (needs the
# Visual Studio Build Tools' C++ workload) or the GNU one (rustup default stable-gnu).
# Env: JOBS (default: all cores).
param([ValidateSet("release", "dev")][string]$Build = "release")
$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$manifest = Join-Path $here "rust\Cargo.toml"
$bin = Join-Path $here "bin"
New-Item -ItemType Directory -Force -Path $bin | Out-Null

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
	throw "build.ps1: cargo not found - install Rust from https://rustup.rs"
}
$jobs = if ($env:JOBS) { $env:JOBS } else { [Environment]::ProcessorCount }
if ($Build -eq "release") {
	cargo build --release -j $jobs --manifest-path $manifest
	$dir = "release"
} else {
	cargo build --profile fast -j $jobs --manifest-path $manifest
	$dir = "fast"
}
if ($LASTEXITCODE -ne 0) { throw "build.ps1: cargo build failed" }

$built = Join-Path $here "rust\target\$dir\breach_native.dll"
Copy-Item $built (Join-Path $bin "breach_rust.windows.x86_64.dll") -Force
# The crate's own binaries (src/bin/), as native/build.sh copies them: native\bin\breach_<name>.exe.
foreach ($name in @("designer", "digest", "trials", "scaling", "tick_phases", "rout_bench", "native_bench")) {
	Copy-Item (Join-Path $here "rust\target\$dir\$name.exe") (Join-Path $bin "breach_$name.exe") -Force
}
Get-ChildItem $bin
