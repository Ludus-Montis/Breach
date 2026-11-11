#!/usr/bin/env bash
# Stores a PR's showcase record and rebuilds manifest.json (spec 44 step 3): deletes the
# assets the retention rule let go ("<release> <name>" lines in <removed.txt>), uploads
# pr-<N>.json to the pre-release showcase-index, rebuilds manifest.json from every PR's
# record and dispatches the site build. Each PR writes only its own record, so two PRs'
# runs never overwrite each other's; manifest.json is rebuilt whole by every run.
#
# Usage: ci/showcase/index.sh <record.json> <removed.txt>   (needs GH_TOKEN)
set -euo pipefail

record=$1 removed=$2
here=$(cd "$(dirname "$0")" && pwd)
INDEX=showcase-index

while read -r release name; do
  [ -n "$name" ] && { gh release delete-asset "$release" "$name" -y || echo "no $name in $release"; }
done < "$removed"

gh release upload "$INDEX" "$record" --clobber
all=$(mktemp -d)
gh release download "$INDEX" -p 'pr-*.json' -D "$all"
python3 "$here/manifest.py" merge "$all/manifest.json" "$all"/pr-*.json
gh release upload "$INDEX" "$all/manifest.json" --clobber
gh workflow run site-pages.yml --ref main || echo "::warning::the site build wasn't dispatched"
