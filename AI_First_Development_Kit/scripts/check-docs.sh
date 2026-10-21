#!/usr/bin/env bash
# Documentation gate for the AI-First Development Kit.
# Usage: check-docs.sh [readme|changelog|docs|all] [base-ref]
#   base-ref: git ref to diff against for the changelog gate (default: origin/main, falls back to HEAD~1)
# Env:
#   DOCS_SKIP_CHANGELOG=1  skip the "code changed => changelog changed" gate (record the reason in the PR)
set -u

MODE="${1:-all}"
BASE="${2:-origin/main}"
README="${README_PATH:-README.md}"
CHANGELOG="${CHANGELOG_PATH:-CHANGELOG.md}"
DOCS_DIR="${DOCS_DIR:-docs}"
FAILED=0

fail() { echo "FAIL: $*"; FAILED=1; }
pass() { echo "ok:   $*"; }

check_readme() {
  [ -f "$README" ] || { fail "$README not found"; return; }

  # Required sections (Standard Readme), in required relative order.
  local last=0 name line
  for name in "Install" "Usage" "Contributing" "License"; do
    line=$(grep -n -i -E "^#{2,3} +${name}\b" "$README" | head -1 | cut -d: -f1)
    if [ -z "$line" ]; then
      fail "README missing required section: $name"
    elif [ "$line" -lt "$last" ]; then
      fail "README section out of order: $name (Standard Readme: Install, Usage, Contributing, License)"
    else
      last="$line"
    fi
  done

  # Short description: first non-empty, non-heading, non-badge line, 120 chars max.
  local short
  short=$(grep -v -E '^\s*$|^#|^\[!\[|^!\[|^<' "$README" | head -1)
  if [ -z "$short" ]; then
    fail "README has no short description"
  elif [ "${#short}" -gt 120 ]; then
    fail "README short description is ${#short} chars (max 120)"
  fi

  # Table of Contents required over 100 lines.
  if [ "$(wc -l < "$README")" -gt 100 ] && ! grep -q -i -E '^#{2,3} +Table of Contents' "$README"; then
    fail "README exceeds 100 lines and has no Table of Contents"
  fi

  [ "$FAILED" -eq 0 ] && pass "README structure"
}

check_changelog() {
  [ -f "$CHANGELOG" ] || { fail "$CHANGELOG not found"; return; }
  grep -q -E '^## \[Unreleased\]' "$CHANGELOG" || fail "$CHANGELOG missing '## [Unreleased]' section"

  # Only allow Keep a Changelog categories at ### level.
  local bad
  bad=$(grep -E '^### ' "$CHANGELOG" | grep -v -E '^### (Added|Changed|Deprecated|Removed|Fixed|Security)\s*$')
  [ -n "$bad" ] && fail "Non-standard changelog categories: $(echo "$bad" | tr '\n' ';')"

  # Gate: if non-doc files changed vs base, the changelog must have changed too.
  if git rev-parse --git-dir >/dev/null 2>&1 && [ "${DOCS_SKIP_CHANGELOG:-0}" != "1" ]; then
    git rev-parse --verify -q "$BASE" >/dev/null || BASE="HEAD~1"
    if git rev-parse --verify -q "$BASE" >/dev/null; then
      local changed code
      changed=$(git diff --name-only "$BASE"...HEAD 2>/dev/null || git diff --name-only "$BASE")
      code=$(echo "$changed" | grep -v -E '(^|/)(docs/|README|CHANGELOG|CONTRIBUTING|LICENSE)|\.md$|^\.github/|(^|/)tests?/' | grep -v '^$')
      if [ -n "$code" ] && ! echo "$changed" | grep -q -x "$CHANGELOG"; then
        fail "Source files changed but $CHANGELOG did not (set DOCS_SKIP_CHANGELOG=1 and justify in PR if not user-facing)"
      fi
    fi
  fi

  [ "$FAILED" -eq 0 ] && pass "CHANGELOG"
}

check_docs() {
  local index="$DOCS_DIR/index.md"
  [ -f "$index" ] || { fail "$index (docs index) not found"; return; }

  # Every relative link in the index must resolve.
  local link target
  while IFS= read -r link; do
    target="${link%%#*}"; target="${target//%20/ }"
    case "$target" in ''|http*|mailto:*) continue;; esac
    [ -e "$DOCS_DIR/$target" ] || fail "docs index has broken link: $link"
  done < <(grep -o -E '\]\([^)]+\)' "$index" | sed -E 's/^\]\((.*)\)$/\1/')

  # Every page under docs/ must be linked from the index (no orphans).
  local page rel
  while IFS= read -r page; do
    rel="${page#"$DOCS_DIR"/}"
    [ "$rel" = "index.md" ] && continue
    grep -q -F "($rel" "$index" || fail "orphan doc page not linked from index: $rel"
  done < <(find "$DOCS_DIR" -type f -name '*.md' | sort)

  [ "$FAILED" -eq 0 ] && pass "docs index"
}

case "$MODE" in
  readme) check_readme ;;
  changelog) check_changelog ;;
  docs) check_docs ;;
  all) check_readme; check_changelog; check_docs ;;
  *) echo "usage: $0 [readme|changelog|docs|all] [base-ref]"; exit 2 ;;
esac

exit "$FAILED"
