#!/usr/bin/env bash
# The key a native binary is stored under in the `native-store` pre-release (spec 49,
# Decision 179):
#
#   store_key.sh <commit> <profile> <target>
#   -> <tree id of native/ at commit>-<toolchain>-<profile>-<target>
#
# Every build input lives under native/ (step 2's check keeps it so), so its tree id stands
# for the code. The toolchain is the pin of the checkout running this script (the head),
# not of <commit>: a reference that predates the pin is keyed, and built, with the head's
# compiler.
set -euo pipefail

if [ "$#" -ne 3 ] || [ -z "$1" ] || [ -z "$2" ] || [ -z "$3" ]; then
	echo "usage: store_key.sh <commit> <profile> <target>" >&2
	exit 2
fi
commit="$1" profile="$2" target="$3"

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
pin="$root/native/rust/rust-toolchain.toml"
toolchain="$(sed -n 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$pin" | head -n 1)"
if [ -z "$toolchain" ]; then
	echo "store_key.sh: no channel in $pin" >&2
	exit 1
fi

tree="$(git -C "$root" rev-parse --verify "$commit^{commit}:native")"
echo "$tree-$toolchain-$profile-$target"
