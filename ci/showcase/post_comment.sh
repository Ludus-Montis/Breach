#!/usr/bin/env bash
# The PR's one showcase comment (spec 44 step 3): finds it by its marker and edits it, or
# adds it. Usage: ci/showcase/post_comment.sh <pr> <body.md>   (needs GH_TOKEN)
set -euo pipefail

pr=$1 body=$2
marker="<!-- showcase-capture -->"
id=$(gh api "repos/$GITHUB_REPOSITORY/issues/$pr/comments" --paginate \
       --jq ".[] | select(.body | startswith(\"$marker\")) | .id" | head -1)
if [ -n "$id" ]; then
  gh api -X PATCH "repos/$GITHUB_REPOSITORY/issues/comments/$id" -F body=@"$body" > /dev/null
else
  gh api "repos/$GITHUB_REPOSITORY/issues/$pr/comments" -F body=@"$body" > /dev/null
fi
