#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)"
ARCH="$(dpkg --print-architecture)"
OUT_DIR="${1:-$ROOT_DIR/dist}"
STAGE_HEADLESS="$(mktemp -d)"
STAGE_GUI="$(mktemp -d)"
trap 'rm -rf "$STAGE_HEADLESS" "$STAGE_GUI"' EXIT

mkdir -p "$OUT_DIR"

cargo build --release -p pipelink --locked
make install DESTDIR="$STAGE_HEADLESS" PREFIX=/usr
mkdir -p "$STAGE_HEADLESS/DEBIAN"
cat > "$STAGE_HEADLESS/DEBIAN/control" <<EOF
Package: pipelink
Version: ${VERSION}
Section: sound
Priority: optional
Architecture: ${ARCH}
Maintainer: Kuroki-Kael <kuroki@kuroki-kael.com>
Homepage: https://codeberg.org/kuroki-kael/pipelink
Depends: libc6, libpipewire-0.3-0 | libpipewire-0.3-0t64, pipewire, wireplumber, pipewire-jack | libjack-jackd2-0
Description: Virtual PipeWire/JACK audio routing daemon and CLI
 PipeLink provides virtual stereo PipeWire audio channels, multi-mix routing
 with per-route gain and mute controls, automatic application discovery, and
 output sink failover.
EOF
dpkg-deb --root-owner-group --build "$STAGE_HEADLESS" "$OUT_DIR/pipelink_${VERSION}_${ARCH}.deb"

cargo build --release -p pipelink --features gui --locked
make install-gui DESTDIR="$STAGE_GUI" PREFIX=/usr
mkdir -p "$STAGE_GUI/DEBIAN"
cat > "$STAGE_GUI/DEBIAN/control" <<EOF
Package: pipelink-gui
Version: ${VERSION}
Section: sound
Priority: optional
Architecture: ${ARCH}
Maintainer: Kuroki-Kael <kuroki@kuroki-kael.com>
Homepage: https://codeberg.org/kuroki-kael/pipelink
Provides: pipelink
Conflicts: pipelink
Replaces: pipelink
Depends: libc6, libpipewire-0.3-0 | libpipewire-0.3-0t64, pipewire, wireplumber, pipewire-jack | libjack-jackd2-0, hicolor-icon-theme
Description: Virtual PipeWire/JACK audio routing daemon, CLI, and graphical mixer
 PipeLink provides virtual stereo PipeWire audio channels, multi-mix routing
 with per-route gain and mute controls, automatic application discovery, and
 output sink failover, including the WaveLink-inspired iced desktop mixer.
EOF
dpkg-deb --root-owner-group --build "$STAGE_GUI" "$OUT_DIR/pipelink-gui_${VERSION}_${ARCH}.deb"
