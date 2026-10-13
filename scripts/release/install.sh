#!/usr/bin/env bash
# Installs the declarify/notes/notes-server binaries from this directory
# (bundled alongside this script in the release .tar.gz/.dmg) onto PATH.
#
# Override the install location with DECLARIFY_INSTALL_DIR; otherwise this
# uses /usr/local/bin if writable (or running as root), falling back to
# ~/.local/bin.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEST="${DECLARIFY_INSTALL_DIR:-/usr/local/bin}"

if { ! mkdir -p "$DEST" 2>/dev/null || [ ! -w "$DEST" ]; } && [ "$(id -u)" -ne 0 ]; then
  echo "Cannot write to $DEST; installing to \$HOME/.local/bin instead." >&2
  DEST="$HOME/.local/bin"
  mkdir -p "$DEST"
fi

for bin in declarify notes notes-server; do
  install -m 0755 "$SCRIPT_DIR/$bin" "$DEST/$bin"
  echo "installed $DEST/$bin"
done

case ":$PATH:" in
  *":$DEST:"*) ;;
  *)
    echo
    echo "Note: $DEST is not on your PATH. Add it, e.g.:"
    echo "  export PATH=\"$DEST:\$PATH\""
    ;;
esac

echo
echo "Done. Try: declarify --help"
