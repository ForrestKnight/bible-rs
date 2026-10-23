#!/bin/sh
# Installs a Linux release of bible-rs: the binary, the desktop entry and the icons.
#
#   ./install.sh                         # into ~/.local, no root needed
#   sudo PREFIX=/usr/local ./install.sh  # for every user
#   ./install.sh --uninstall             # removes exactly what it installed
#
# Notes, highlights and settings live in ~/.local/share/bible-rs and are never touched.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
PREFIX=${PREFIX:-"$HOME/.local"}
BINDIR=${BINDIR:-"$PREFIX/bin"}
APPDIR=${APPDIR:-"$PREFIX/share/applications"}
ICONDIR=${ICONDIR:-"$PREFIX/share/icons/hicolor"}

sizes() {
  for png in "$here"/icons/hicolor/*/apps/bible-rs.png; do
    basename "$(dirname "$(dirname "$png")")"
  done
}

refresh() {
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APPDIR" 2>/dev/null || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1 && [ -f "$ICONDIR/index.theme" ]; then
    gtk-update-icon-cache -q "$ICONDIR" 2>/dev/null || true
  fi
}

if [ "${1:-}" = "--uninstall" ]; then
  rm -f "$BINDIR/bible-rs" "$APPDIR/bible-rs.desktop"
  for size in $(sizes); do
    rm -f "$ICONDIR/$size/apps/bible-rs.png"
  done
  refresh
  echo "removed bible-rs from $PREFIX"
  exit 0
fi

install -d "$BINDIR" "$APPDIR"
install -m755 "$here/bible-rs" "$BINDIR/bible-rs"
install -m644 "$here/bible-rs.desktop" "$APPDIR/bible-rs.desktop"
for size in $(sizes); do
  install -d "$ICONDIR/$size/apps"
  install -m644 "$here/icons/hicolor/$size/apps/bible-rs.png" "$ICONDIR/$size/apps/bible-rs.png"
done
refresh

echo "installed bible-rs $("$BINDIR/bible-rs" --version | sed 's/^bible-rs //'):"
echo "  $BINDIR/bible-rs"
echo "  $APPDIR/bible-rs.desktop"
echo "  $ICONDIR/<size>/apps/bible-rs.png"
case ":$PATH:" in
  *":$BINDIR:"*) ;;
  *) echo; echo "note: $BINDIR is not on PATH in this shell" ;;
esac
