#!/usr/bin/env bash
# get_godot: the pinned headless Godot for a session that needs to run tests (Linux x86_64).
# Installs GODOT_VERSION from .github/workflows/ci.yml (as .github/actions/godot/action.yml
# does in CI) into /opt/godot, or GODOT_DIR if set, unless it is there already; checks the
# zip against Godot's published SHA-512 sums; imports the project once so gdUnit4's class
# names resolve. Prints the GODOT_BIN to export. Fetched on demand, not in every session:
# design and docs sessions never need it.
#
# Tries the GitHub release first, then downloads.godotengine.org. A cloud session needs one
# of them allowed by its environment's network policy (downloads.godotengine.org is the
# narrow one).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
version="$(sed -n 's/^ *GODOT_VERSION: *"\([^"]*\)".*/\1/p' "$root/.github/workflows/ci.yml" | head -1)"
if [[ -z "$version" ]]; then
  echo "get_godot: no GODOT_VERSION in .github/workflows/ci.yml" >&2
  exit 1
fi
dir="${GODOT_DIR:-/opt/godot}"
bin="$dir/godot"
name="Godot_v${version}_linux.x86_64"

if [[ -x "$bin" ]] && "$bin" --version 2>/dev/null | grep -q "^${version%-stable}"; then
  echo "get_godot: Godot $version already at $bin"
else
  work="$(mktemp -d)"
  trap 'rm -rf "$work"' EXIT
  release="https://github.com/godotengine/godot/releases/download/$version"
  zips=(
    "$release/$name.zip"
    "https://downloads.godotengine.org/?version=${version%-*}&flavor=${version#*-}&slug=linux.x86_64.zip&platform=linux.64"
  )
  got=""
  for zip in "${zips[@]}"; do
    if curl -fsSL "$zip" -o "$work/godot.zip" && unzip -tq "$work/godot.zip" >/dev/null 2>&1; then
      got="$zip"
      break
    fi
    echo "get_godot: couldn't fetch $zip" >&2
  done
  if [[ -z "$got" ]]; then
    echo "get_godot: no source reachable. A cloud session's network policy must allow" >&2
    echo "  downloads.godotengine.org (or github.com's release downloads)." >&2
    exit 1
  fi
  # the sums are published with the GitHub release
  if curl -fsSL "$release/SHA512-SUMS.txt" -o "$work/sums.txt" 2>/dev/null; then
    want="$(awk -v f="$name.zip" '$2 == f {print $1}' "$work/sums.txt")"
    have="$(sha512sum "$work/godot.zip" | awk '{print $1}')"
    if [[ -z "$want" || "$want" != "$have" ]]; then
      echo "get_godot: $name.zip doesn't match Godot's published SHA-512 sum" >&2
      exit 1
    fi
    echo "get_godot: checksum matches Godot's published SHA-512 sum"
  else
    echo "get_godot: warning: couldn't fetch $release/SHA512-SUMS.txt, so the zip is unchecked" >&2
  fi
  mkdir -p "$dir"
  unzip -qo "$work/godot.zip" -d "$work"
  mv "$work/$name" "$bin"
  chmod +x "$bin"
  echo "get_godot: installed Godot $version at $bin"
fi

"$bin" --headless --path "$root" --import >/dev/null 2>&1 || true
echo "get_godot: project imported. Export it for the tests:"
echo "  export GODOT_BIN=$bin"
