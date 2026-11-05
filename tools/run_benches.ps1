# Runs Breach's simulation benchmarks on this machine and writes one report to compare with
# others (native/README.md, "Running the benchmarks locally"). Windows PowerShell 5.1 or
# PowerShell 7 on any platform:
#
#   powershell -ExecutionPolicy Bypass -File tools\run_benches.ps1 [-Godot <Godot exe>]
#       [-Side 1024] [-SkipDigests]
#
# Build the native core first (native\build.ps1, or native/build.sh elsewhere): every step
# but the first runs the crate's own binaries (native/bin/breach_<name>), with nothing from
# Godot (spec 39 step 9d-iii retired the GDScript tools). Steps:
#   0. platform maths (tools/platform_probe.gd, only with -Godot): a hash per maths
#      primitive, to diff between machines; on Windows use Godot 4.7.2's console build
#      (Godot_v4.7.2-stable_win64_console.exe) so its output reaches the report
#   1. identity: the per-tick digests' hashes (breach_digest), to diff between machines -
#      the same battles must replay bit for bit on every machine
#   2. a whole tick, phase by phase (breach_tick_phases), mid-fight and marching
#   3. the native core's passes (breach_native_bench)
# The report lands in reports\bench\ (git-ignored).
param(
	[string]$Godot = "",
	[int]$Side = 1024,
	[switch]$SkipDigests
)
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$out = Join-Path $root "reports\bench"
New-Item -ItemType Directory -Force -Path $out | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmm"
$machine = [Environment]::MachineName
$report = Join-Path $out "bench_${machine}_$stamp.txt"
$onWindows = [Environment]::OSVersion.Platform -eq "Win32NT"
$exe = if ($onWindows) { ".exe" } else { "" }

function Write-Report([string]$text) {
	Write-Host $text
	Add-Content -Path $report -Value $text
}

# A crate binary, native/bin/breach_<name>; stderr read as plain text (Windows PowerShell 5.1
# would turn each line into an error record, which "Stop" makes fatal).
function Invoke-Crate([string]$name, [string[]]$arguments) {
	$binary = Join-Path $root "native/bin/breach_$name$exe"
	if (-not (Test-Path -LiteralPath $binary)) {
		throw "run_benches.ps1: $binary missing - build the native core first (native\build.ps1)"
	}
	$ErrorActionPreference = "Continue"
	& $binary @arguments "repo=$root" 2>&1 | ForEach-Object { "$_" }
}

Write-Report "Breach benchmarks - $machine - $(Get-Date -Format s)"
if ($onWindows) {
	$cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1)
	$memory = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1)
	$cores = "$($cpu.NumberOfCores) cores, $($cpu.NumberOfLogicalProcessors) threads"
	Write-Report "CPU: $($cpu.Name.Trim()) - $cores, $($cpu.MaxClockSpeed) MHz"
	Write-Report "Memory: $memory GB"
} else {
	$name = (Select-String -Path /proc/cpuinfo -Pattern "model name" | Select-Object -First 1)
	Write-Report "CPU: $(($name.Line -split ':', 2)[1].Trim()) - $([Environment]::ProcessorCount) threads"
	Write-Report "Memory: $((Select-String -Path /proc/meminfo -Pattern 'MemTotal').Line)"
}
Write-Report "Commit: $(git -C $root rev-parse --short HEAD) ($(git -C $root rev-parse --abbrev-ref HEAD))"

if ($Godot) {
	Write-Report "`n== Platform maths (tools/platform_probe.gd): diff these lines between machines"
	$ErrorActionPreference = "Continue"
	& $Godot --headless --path $root --import 2>&1 | Out-Null
	& $Godot --headless --path $root --script "res://tools/platform_probe.gd" 2>&1 | ForEach-Object { "$_" } |
		Where-Object { $_ -match "^\s+\S|platform_probe" } | ForEach-Object { Write-Report $_ }
	$ErrorActionPreference = "Stop"
}

if (-not $SkipDigests) {
	Write-Report "`n== Identity: per-tick digests (diff these lines between machines)"
	foreach ($set in @("standard", "clash160", "wide")) {
		$file = Join-Path $out "digest_$set.txt"
		Invoke-Crate "digest" @("set=$set", "out=$file") | Out-Null
		$hash = (Get-FileHash -Algorithm SHA256 $file).Hash.ToLower()
		Write-Report ("{0,-9} {1}" -f $set, $hash)
	}
}

Write-Report "`n== A tick, phase by phase ($Side a side; ms a tick)"
foreach ($stage in @("fight", "march")) {
	Write-Report "-- $stage"
	Invoke-Crate "tick_phases" @("side=$Side", "stage=$stage", "fight=5") |
		Where-Object { $_ -match "machine:|ms a tick|^\s+\S|WARNING" } | ForEach-Object { Write-Report $_ }
}

Write-Report "`n== The native core's passes (breach_native_bench)"
Invoke-Crate "native_bench" @("runs=3") |
	Where-Object { $_ -notmatch "^$" } | ForEach-Object { Write-Report $_ }

Write-Report "`nReport: $report"
