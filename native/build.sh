#!/usr/bin/env bash
# Builds Breach's native core (Decision 129: Rust through GDExtension, godot-rust) into
# native/bin/ (git-ignored). See native/README.md.
#
#   bash native/build.sh [release|dev]     # default: release
#
# Env: JOBS (default 2).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
jobs="${JOBS:-2}"
profile="${1:-release}"

case "$(uname -s)" in
	Linux) platform=linux.x86_64; prefix=lib; ext=so ;;
	Darwin) platform=macos.universal; prefix=lib; ext=dylib ;;
	MINGW* | MSYS* | CYGWIN*) platform=windows.x86_64; prefix=; ext=dll ;;
	*) echo "build.sh: unknown platform $(uname -s)"; exit 1 ;;
esac
mkdir -p "$here/bin"

case "$profile" in
	release) cargo build --release -j "$jobs" --manifest-path "$here/rust/Cargo.toml"; dir=release ;;
	dev) cargo build --profile fast -j "$jobs" --manifest-path "$here/rust/Cargo.toml"; dir=fast ;;
	*) echo "build.sh: unknown profile $profile (release or dev)"; exit 1 ;;
esac
cp "$here/rust/target/$dir/${prefix}breach_native.$ext" "$here/bin/${prefix}breach_rust.$platform.$ext"
# The map designer's server (src/bin/designer.rs): native/bin/breach_designer.
exe=; [ "$ext" = dll ] && exe=.exe
cp "$here/rust/target/$dir/designer$exe" "$here/bin/breach_designer$exe"
ls -l "$here/bin"
