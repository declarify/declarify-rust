#!/usr/bin/env bash
# Completes the full Note resource lifecycle using only the compiled `notes`
# CLI against a real, separately-running `notes-server` process backed by
# SQLite. This is the docs/08-implementation-plan.md DCL-0005 exit
# criterion, walked through by hand; tests/cli_smoke.rs is the same
# lifecycle as an automated test.
#
# Requires: cargo, jq, curl.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DB_PATH="$(mktemp -u -t notes-lifecycle-XXXXXX).sqlite3"
PORT="${NOTES_LIFECYCLE_PORT:-18080}"
BASE_URL="http://127.0.0.1:${PORT}"

echo "building notes-server and notes..." >&2
cargo build --quiet --manifest-path "${ROOT_DIR}/Cargo.toml" --bin notes-server --bin notes

echo "starting notes-server on ${BASE_URL} (db: ${DB_PATH})..." >&2
cargo run --quiet --manifest-path "${ROOT_DIR}/Cargo.toml" --bin notes-server -- \
  --db "${DB_PATH}" --listen "127.0.0.1:${PORT}" &
SERVER_PID=$!
trap 'kill "${SERVER_PID}" 2>/dev/null || true; rm -f "${DB_PATH}"' EXIT

notes() {
  cargo run --quiet --manifest-path "${ROOT_DIR}/Cargo.toml" --bin notes -- \
    --base-url "${BASE_URL}" "$@"
}

for _ in $(seq 1 50); do
  if curl -sf "${BASE_URL}/api/v1/notes" >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done

echo "create" >&2
created=$(echo '{"name":"first","spec":{"title":"Title","body":"Body"}}' | notes create)
id=$(echo "${created}" | jq -r '.metadata.id')
revision=$(echo "${created}" | jq -r '.metadata.revision')
echo "  -> ${id} at revision ${revision}" >&2

echo "get" >&2
notes get "${id}" >/dev/null

echo "list" >&2
notes list >/dev/null

echo "update (replace spec)" >&2
updated=$(echo '{"title":"New title","body":"Body"}' | notes update "${id}" --expected-revision "${revision}")
revision=$(echo "${updated}" | jq -r '.metadata.revision')

echo "status (update status)" >&2
status_updated=$(echo '{"indexed":true}' | notes status "${id}" --expected-revision "${revision}")
revision=$(echo "${status_updated}" | jq -r '.metadata.revision')

echo "delete" >&2
notes delete "${id}" --expected-revision "${revision}"

echo "OK: full Note lifecycle completed using only the CLI" >&2
