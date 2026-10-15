#!/usr/bin/env bash
# tests-red-then-green (the "green" half) / contract-tests-pass, per
# AI_First_Development_Kit/rubrics/run-baseline.rubrics.md.
#
# Thin wrapper around gdUnit4's own CI runner so both pre-commit and CI call one
# command. Requires GODOT_BIN to point at a Godot 4.7+ executable - see
# ci/godot/README.md's "Running the checks locally" section.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

if [ -z "${GODOT_BIN:-}" ]; then
	echo "run_tests: GODOT_BIN is not set. Export it to a Godot 4.7+ executable, e.g.:"
	echo "  export GODOT_BIN=/path/to/Godot_v4.7.2-stable_win64.exe"
	exit 1
fi

if [ ! -d tests ]; then
	echo "run_tests: no tests/ directory yet, nothing to run."
	exit 0
fi

# Not using addons/gdUnit4/runtest.sh: it runs in `-d --remote-debug` mode, which
# needs a real display server (X11/Wayland) - fine on a dev machine with a GPU, but it
# hard-failed on GitHub Actions' ubuntu-latest runner (no display at all), caught by
# this PR's own first real CI run. `--headless --ignoreHeadlessMode` runs the same
# GdUnitCmdTool.gd CLI tool without needing a display - verified locally and in CI.
# ignoreHeadlessMode is safe here: this project has no UI-interaction tests yet (see
# specs/*.md's test-first orders), which is the one thing gdUnit4's headless warning
# is about.
#
# With paths (the pre-commit hook passes the staged files), only the staged suites run,
# as staged_tests.py picks them; the full suite is CI's (CLAUDE.md). With none, the full
# suite runs, for a run by hand.
suites=(-a res://tests)
if [ "$#" -gt 0 ]; then
	suites=()
	while IFS= read -r suite; do
		suite="${suite%$'\r'}" # Windows' Python ends lines with \r\n
		if [ -n "$suite" ]; then suites+=(-a "$suite"); fi
	done < <(python3 ci/godot/scripts/staged_tests.py "$@")
	if [ "${#suites[@]}" -eq 0 ]; then
		echo "run_tests: no test suite staged; CI runs the suite."
		exit 0
	fi
fi
"$GODOT_BIN" --headless --path . -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd "${suites[@]}" -c --ignoreHeadlessMode
