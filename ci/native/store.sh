#!/usr/bin/env bash
# The store of native binaries (spec 49 step 3, Decision 179): assets on the long-lived
# pre-release `native-store`, one `<key>.zip` per key (`store_key.sh`).
#
#   store.sh fetch <commit> <profile> <target> <dir>
#
# Downloads the key's zip and unzips it into <dir>. If the key is missing, builds it from
# <commit>'s own tree (a worktree, so the working tree is never built) with the head's
# pinned toolchain, puts it in <dir>, and uploads it when the run may write.
#   release:   native/bin/ whole (native/build.sh release)
#   profiling: breach_tick_phases alone (cargo build --profile profiling --bin tick_phases)
# <target> is the triple the binaries are for; a build must be for the host's.
#
# May write: a push to main, a same-repo PR, a release, workflow_dispatch; never a fork's
# PR, and never outside GitHub Actions. An upload's label says where the key was first
# built, for the prune (prune.py). gh reads the repo from GH_REPO, or the checkout.
# Every build flag lives here or under native/ (step 2's check), so the key covers it.
set -euo pipefail

store=native-store

usage() {
	echo "usage: store.sh fetch <commit> <profile> <target> <dir>" >&2
	exit 2
}
[ "$#" -eq 5 ] && [ "$1" = fetch ] || usage
commit="$2" profile="$3" target="$4" dir="$5"
case "$profile" in release | profiling) ;; *) echo "store.sh: unknown profile $profile" >&2; exit 2 ;; esac

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
key="$(bash "$here/store_key.sh" "$commit" "$profile" "$target")"
zip_name="$key.zip"

python=
for candidate in python3 python; do
	if command -v "$candidate" > /dev/null 2>&1 && "$candidate" -c 'import zipfile' > /dev/null 2>&1; then
		python="$candidate"
		break
	fi
done
[ -n "$python" ] || { echo "store.sh: needs Python for zip files" >&2; exit 1; }

work="$(mktemp -d)"
tree=
cleanup() {
	[ -z "$tree" ] || git -C "$root" worktree remove --force "$tree" > /dev/null 2>&1 || true
	rm -rf "$work"
}
trap cleanup EXIT

may_write() {
	[ -n "${GITHUB_ACTIONS:-}${GITHUB_EVENT_NAME:-}" ] || return 1
	case "${GITHUB_EVENT_NAME:-}" in
		push) [ "${GITHUB_REF:-}" = refs/heads/main ] ;;
		release | workflow_dispatch) return 0 ;;
		pull_request)
			local head_repo
			head_repo="$("$python" -c 'import json, sys
print(json.load(open(sys.argv[1]))["pull_request"]["head"]["repo"]["full_name"])' \
				"${GITHUB_EVENT_PATH:-/nonexistent}" 2> /dev/null)" || return 1
			[ -n "$head_repo" ] && [ "$head_repo" = "${GITHUB_REPOSITORY:-}" ]
			;;
		*) return 1 ;;
	esac
}

# Where a key was first built, for the prune (prune.py): `main <sha>`, `pr <n> <sha>`,
# `release <sha>` or `dispatch <sha>`.
origin_label() {
	local sha
	sha="$(git -C "$root" rev-parse "$commit^{commit}")"
	case "${GITHUB_EVENT_NAME:-}" in
		push) echo "main $sha" ;;
		pull_request)
			echo "pr $("$python" -c 'import json, sys
print(json.load(open(sys.argv[1]))["pull_request"]["number"])' "$GITHUB_EVENT_PATH") $sha"
			;;
		release) echo "release $sha" ;;
		*) echo "dispatch $sha" ;;
	esac
}

unzip_into() { # <zip> <dir>
	mkdir -p "$2"
	"$python" -c 'import sys, zipfile; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])' "$1" "$2"
}

zip_dir() { # <dir> <zip>: the files directly in <dir>, by name alone
	"$python" -c 'import os, sys, zipfile
with zipfile.ZipFile(sys.argv[2], "w", zipfile.ZIP_DEFLATED) as z:
    for name in sorted(os.listdir(sys.argv[1])):
        z.write(os.path.join(sys.argv[1], name), name)' "$1" "$2"
}

report() { # <how>
	local size line
	size="$(wc -c < "$work/$zip_name" | tr -d ' ')"
	line="store.sh: $zip_name, $size bytes ($1)"
	echo "$line"
	[ -z "${GITHUB_STEP_SUMMARY:-}" ] || echo "- \`$zip_name\`: $size bytes ($1)" >> "$GITHUB_STEP_SUMMARY"
}

# 1. The store has it.
if gh release download "$store" -p "$zip_name" -D "$work" > /dev/null 2>&1 && [ -f "$work/$zip_name" ]; then
	unzip_into "$work/$zip_name" "$dir"
	report fetched
	exit 0
fi

# 2. Build it, from the commit's own tree, with the head's pin.
toolchain="$(sed -n 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' \
	"$root/native/rust/rust-toolchain.toml" | head -n 1)"
if command -v rustup > /dev/null 2>&1; then
	rustup toolchain install "$toolchain" --profile minimal > /dev/null 2>&1 || true
fi
host="$(RUSTUP_TOOLCHAIN="$toolchain" rustc -vV | sed -n 's/^host: //p')"
if [ "$host" != "$target" ]; then
	echo "store.sh: $target isn't this host's target ($host); a key is built on its own OS" >&2
	exit 1
fi

tree="$work/tree"
git -C "$root" worktree add -q --detach "$tree" "$commit^{commit}"
staged="$work/staged"
mkdir -p "$staged"
exe=
case "$target" in *windows*) exe=.exe ;; esac
export RUSTUP_TOOLCHAIN="$toolchain"
case "$profile" in
	release)
		JOBS="${JOBS:-4}" bash "$tree/native/build.sh" release > "$work/build.log" 2>&1 \
			|| { tail -n 30 "$work/build.log" >&2; exit 1; }
		cp -R "$tree/native/bin/." "$staged/"
		;;
	profiling)
		(cd "$tree/native/rust" && cargo build --profile profiling --bin tick_phases) > "$work/build.log" 2>&1 \
			|| { tail -n 30 "$work/build.log" >&2; exit 1; }
		cp "$tree/native/rust/target/profiling/tick_phases$exe" "$staged/breach_tick_phases$exe"
		;;
esac
zip_dir "$staged" "$work/$zip_name"
unzip_into "$work/$zip_name" "$dir"

# 3. Upload it, when the run may write.
if ! may_write; then
	report "built, not uploaded: this run may not write"
	exit 0
fi
assets() { gh release view "$store" --json assets --jq '.assets[].name' 2> /dev/null; }
if ! assets > /dev/null; then
	gh release create "$store" --prerelease --title "Native store" \
		--notes "Native binaries keyed by native/'s tree, toolchain, profile and target (spec 49, Decision 179). Written by CI only." \
		> /dev/null 2>&1 || assets > /dev/null # another run made it first
fi
if assets | grep -qxF "$zip_name"; then
	report "built, upload skipped: already stored"
elif gh release upload "$store" "$work/$zip_name#$(origin_label)" > /dev/null 2>&1; then
	report "built and uploaded"
elif assets | grep -qxF "$zip_name"; then
	report "built, upload skipped: stored meanwhile"
else
	echo "store.sh: uploading $zip_name failed" >&2
	exit 1
fi
