# Runs Breach's simulation benchmarks on this machine and writes one report to compare with
# others (native/README.md, "Running the benchmarks locally"). Windows PowerShell 5.1 or
# PowerShell 7 on any platform:
#
#   powershell -ExecutionPolicy Bypass -File tools\run_benches.ps1 -Godot <Godot exe>
#       [-Side 1024] [-SkipDigests] [-SkipGdscript]
#
# On Windows use Godot 4.7.2's console build (Godot_v4.7.2-stable_win64_console.exe) so
# its output reaches the report. Build the native core first (native\build.ps1, or
# native/build.sh elsewhere). Steps:
#   0. platform maths: a hash per maths primitive, to diff between machines
#   1. identity: the per-tick digests under each engine, against the reference hashes -
#      the same battles must replay bit for bit on every machine and engine
#   2. a whole tick, phase by phase (tools/tick_phases.gd), mid-fight and marching
#   3. the native core's passes (tools/native_bench.gd)
# The report lands in reports\bench\ (git-ignored).
param(
	[Parameter(Mandatory = $true)][string]$Godot,
	[int]$Side = 1024,
	[switch]$SkipDigests,
	[switch]$SkipGdscript
)
$ErrorActionPreference = "Stop"

# -Godot may name the executable or a folder holding it (Windows' "Extract All" puts the
# zip's contents in a folder named after the zip). Prefer the console build: it passes
# Godot's output through to this script.
function Resolve-Godot([string]$given) {
	if (Test-Path -LiteralPath $given -PathType Leaf) { return (Resolve-Path -LiteralPath $given).Path }
	$folder = if (Test-Path -LiteralPath $given -PathType Container) { $given } else { Split-Path -Parent $given }
	if ($folder -and (Test-Path -LiteralPath $folder -PathType Container)) {
		$found = Get-ChildItem -LiteralPath $folder -Recurse -Depth 2 -File -Filter "Godot*" |
			Where-Object { $_.Name -match "console\.exe$" -or ($_.Name -match "^Godot_v[^/]*_linux|\.x86_64$") } |
			Sort-Object { $_.Name -notmatch "console" } | Select-Object -First 1
		if ($found) {
			Write-Host "Using Godot at $($found.FullName)"
			return $found.FullName
		}
	}
	throw "run_benches.ps1: no Godot at '$given'. Point -Godot at Godot_v4.7.2-stable_win64_console.exe, or the folder it was unzipped to."
}
$Godot = Resolve-Godot $Godot
if ($Godot -match "\.exe$" -and $Godot -notmatch "console") {
	Write-Warning "That is not Godot's console build; its output may not reach the report. Use Godot_v4.7.2-stable_win64_console.exe from the same zip."
}

$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$out = Join-Path $root "reports\bench"
New-Item -ItemType Directory -Force -Path $out | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmm"
$machine = [Environment]::MachineName
$report = Join-Path $out "bench_${machine}_$stamp.txt"

# The per-tick digest files' sha256 on the reference (Linux) build, with DetMath: every
# platform should give these.
$reference = @{
	"standard" = "14878d70c9800c31bc37ab2173343b8900758f933e0f4a0a5c4b590c28d8663a"
	"clash160" = "846075a85ef4f9b1ebd6eb8f75205daab8f6dc03cf29382131e4b283d65b58d0"
	"wide"     = "0cb41776ecb08e35cccef399d15aa6a3285f9615da043aec9a28cd7a90fd27a9"
}

function Write-Report([string]$text) {
	Write-Host $text
	Add-Content -Path $report -Value $text
}

# Godot writes warnings to stderr; Windows PowerShell 5.1 turns each such line into an
# error record, which "Stop" would make fatal. Read them as plain text instead.
function Invoke-Native([string[]]$arguments) {
	$ErrorActionPreference = "Continue"
	& $Godot @arguments 2>&1 | ForEach-Object { "$_" } |
		Where-Object { $_ -notmatch "^\s+at:|ObjectDB instances|resources still in use|PagedAllocator" }
}

function Invoke-Godot([string]$script, [string[]]$arguments, [string]$engine) {
	$env:BREACH_NATIVE = $engine
	Invoke-Native (@("--headless", "--path", $root, "--script", $script, "--") + $arguments)
}

Write-Report "Breach benchmarks - $machine - $(Get-Date -Format s)"
$onWindows = [Environment]::OSVersion.Platform -eq "Win32NT"
if ($onWindows) {
	$cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1)
	$memory = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1)
	$cores = "$($cpu.NumberOfCores) cores, $($cpu.NumberOfLogicalProcessors) threads"
	Write-Report "CPU: $($cpu.Name.Trim()) - $cores, $($cpu.MaxClockSpeed) MHz"
	Write-Report "Memory: $memory GB"
	$dll = Join-Path $root "native\bin\breach_rust.windows.x86_64.dll"
} else {
	$name = (Select-String -Path /proc/cpuinfo -Pattern "model name" | Select-Object -First 1)
	Write-Report "CPU: $(($name.Line -split ':', 2)[1].Trim()) - $([Environment]::ProcessorCount) threads"
	Write-Report "Memory: $((Select-String -Path /proc/meminfo -Pattern 'MemTotal').Line)"
	$dll = Join-Path $root "native/bin/libbreach_rust.linux.x86_64.so"
}
Write-Report "Commit: $(git -C $root rev-parse --short HEAD) ($(git -C $root rev-parse --abbrev-ref HEAD))"
$engines = @("rust")
if (-not (Test-Path $dll)) {
	Write-Report "WARNING: $dll missing - build the native core first; Rust runs skipped"
	$engines = @()
}
if (-not $SkipGdscript) { $engines = @("gdscript") + $engines }
Write-Report "Engines: $($engines -join ', ')"

Write-Report "`n== Importing the project"
Invoke-Native @("--headless", "--path", $root, "--import") | Out-Null

Write-Report "`n== Platform maths (tools/platform_probe.gd): diff these lines between machines"
Invoke-Godot "res://tools/platform_probe.gd" @() "" |
	Where-Object { $_ -match "^\s+\S|platform_probe" } | ForEach-Object { Write-Report $_ }

if (-not $SkipDigests) {
	Write-Report "`n== Identity: per-tick digests against the reference build"
	foreach ($engine in $engines) {
		foreach ($set in $reference.Keys) {
			$file = Join-Path $out "digest_${set}_$engine.txt"
			Invoke-Godot "res://tools/formation_digest.gd" @("set=$set", "out=$file") $engine | Out-Null
			$hash = (Get-FileHash -Algorithm SHA256 $file).Hash.ToLower()
			$verdict = if ($hash -eq $reference[$set]) { "matches" } else { "DIFFERS from $($reference[$set])" }
			Write-Report ("{0,-9} {1,-9} {2} {3}" -f $engine, $set, $hash, $verdict)
		}
	}
}

Write-Report "`n== A tick, phase by phase ($Side a side; ms a tick)"
foreach ($engine in $engines) {
	foreach ($stage in @("fight", "march")) {
		Write-Report "-- $engine, $stage"
		Invoke-Godot "res://tools/tick_phases.gd" @("engine=$engine", "side=$Side", "stage=$stage", "fight=5") $engine |
			Where-Object { $_ -match "machine:|ms a tick|^\s+\S|WARNING" } | ForEach-Object { Write-Report $_ }
	}
}

Write-Report "`n== The native core's passes (tools/native_bench.gd)"
$list = ($engines -join ",")
Invoke-Godot "res://tools/native_bench.gd" @("engines=$list", "runs=3") "" |
	Where-Object { $_ -notmatch "^Godot Engine|^Initialize|^$" } | ForEach-Object { Write-Report $_ }

Write-Report "`nReport: $report"
