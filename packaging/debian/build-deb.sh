#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)"
ARCH="$(dpkg --print-architecture)"
OUT_DIR="${1:-$ROOT_DIR/dist}"
STAGE_DIR="$(mktemp -d)"
trap 'rm -rf "$STAGE_DIR"' EXIT

cargo build --release -p pipelink --locked

make install DESTDIR="$STAGE_DIR" PREFIX=/usr
mkdir -p "$STAGE_DIR/DEBIAN"

cat > "$STAGE_DIR/DEBIAN/control" <<EOF
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

mkdir -p "$OUT_DIR"
dpkg-deb --root-owner-group --build "$STAGE_DIR" "$OUT_DIR/pipelink_${VERSION}_${ARCH}.deb"
