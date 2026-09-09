#!/usr/bin/env bash
# Verifies every relative link in every Markdown file in this repository
# resolves to an existing file. Absolute URLs (http:, https:, mailto:, ...)
# and pure in-page anchors (#section) are skipped — this checks that
# cross-file references are not broken, per README.md ("Every relative link
# in this list must resolve in CI") and CLAUDE.md's "Required checks before
# completion" ("the repository's ... Markdown link check").
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

failures=0
checked=0

while IFS= read -r -d '' file; do
  while IFS= read -r raw_link; do
    # Drop an optional ` "title"` suffix, then an in-page fragment.
    link="${raw_link%% *}"
    target="${link%%#*}"
    if [[ -z "$target" ]]; then
      continue # pure in-page anchor, e.g. (#section)
    fi
    if [[ "$target" =~ ^[a-zA-Z][a-zA-Z0-9+.-]*: ]]; then
      continue # absolute URL: http:, https:, mailto:, etc.
    fi
    checked=$((checked + 1))
    resolved="$(cd "$(dirname "$file")" && realpath -m "$target" 2>/dev/null || echo "$(dirname "$file")/$target")"
    if [[ ! -e "$resolved" ]]; then
      echo "BROKEN LINK: $file -> $raw_link (resolved: $resolved)" >&2
      failures=$((failures + 1))
    fi
  done < <(grep -oE '\]\([^)]+\)' "$file" | sed -E 's/^\]\((.*)\)$/\1/')
done < <(find . -name "*.md" -not -path "./target/*" -not -path "./.git/*" -print0)

echo "checked ${checked} relative link(s) across Markdown files"
if [[ "$failures" -gt 0 ]]; then
  echo "${failures} broken link(s) found" >&2
  exit 1
fi
echo "OK: all relative Markdown links resolve"
