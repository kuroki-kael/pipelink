# PipeLink

A virtual PipeWire/JACK audio routing daemon and CLI for Linux.

> **Source & Mirror:** The primary repository (origin, issues, pull requests, CI, and package registries) is hosted on [Codeberg](https://codeberg.org/kuroki-kael/pipelink). [GitHub](https://github.com/kuroki-kael/pipelink) is an automated mirror with mirrored releases.

PipeLink creates persistent virtual stereo PipeWire channels (`System`, `Music`, `Games`, `Voice`, etc.), mixes them into one or more named output mixes (`Personal`, `Stream`, ...), routes applications automatically into channels, manages per-route gain and mute state, handles physical output failover, and can set WirePlumber's default audio sink.

## Features

- **Virtual Stereo Channels & Mixes**: Independent virtual PipeWire sinks combined via a lock-free JACK routing graph (`pipelink-jack`).
- **Per-Route Mixer Controls**: Adjust channel-to-mix gain (in percent) and mute state independently per mix.
- **Automatic App Routing**: Discovers PipeWire client streams and automatically links configured applications to their target channels.
- **Output Failover & Hotplug**: Switches hardware/software output sinks cleanly and restores routes across PipeWire/device restarts.
- **WirePlumber Default Sink Helper**: Point WirePlumber's `default.configured.audio.sink` at a PipeLink channel or hardware device with `pipelink default-sink`.
- **XDG Config & State Persistence**: Separates declarative topology (`config.json`) from volatile mixer state (`state.json`) and generates a starter setup on first run.

## Requirements

Runtime:
- PipeWire (`pipewire`)
- PipeWire JACK compatibility (`pipewire-jack` on Arch, `pipewire-jack` or `libjack-jackd2-0` on Debian/Ubuntu)
- WirePlumber (`wireplumber`)

Build dependencies (from source):
- Rust toolchain (`1.99.0` / edition `2024`)
- `pkg-config` / `pkgconf`, `clang` / `libclang-dev`
- PipeWire and JACK development headers (`libpipewire-0.3-dev`, `libjack-jackd2-dev` on Debian/Ubuntu; `pipewire`, `pipewire-jack` on Arch)

## Installation

### Arch Linux (Codeberg `pacman` registry)

1. Import the Codeberg Arch repository key:
   ```bash
   curl -fsSL -o /tmp/kuroki-kael-arch.gpg https://codeberg.org/api/packages/kuroki-kael/arch/repository.key
   sudo pacman-key --add /tmp/kuroki-kael-arch.gpg
   sudo pacman-key --lsign-key "$(gpg --show-keys --with-colons /tmp/kuroki-kael-arch.gpg | awk -F: '/^fpr:/ {print $10; exit}')"
   ```
2. Add the repository to `/etc/pacman.conf`:
   ```ini
   [kuroki-kael.main]
   SigLevel = Required
   Server = https://codeberg.org/api/packages/kuroki-kael/arch/main/$arch
   ```
3. Install `pipelink`:
   ```bash
   sudo pacman -Syu pipelink
   ```

Alternatively, build locally from the included [PKGBUILD](packaging/arch/PKGBUILD):
```bash
cd packaging/arch
makepkg -si
```

### Debian / Ubuntu (Codeberg `apt` registry)

1. Add the Codeberg Debian repository key and source:
   ```bash
   sudo mkdir -p /etc/apt/keyrings
   sudo curl -fsSL -o /etc/apt/keyrings/kuroki-kael.asc https://codeberg.org/api/packages/kuroki-kael/debian/repository.key
   echo "deb [signed-by=/etc/apt/keyrings/kuroki-kael.asc] https://codeberg.org/api/packages/kuroki-kael/debian stable main" \
     | sudo tee /etc/apt/sources.list.d/pipelink.list
   ```
2. Install `pipelink`:
   ```bash
   sudo apt update
   sudo apt install pipelink
   ```

Alternatively, download the `.deb` from the [Codeberg Releases](https://codeberg.org/kuroki-kael/pipelink/releases) page or build one locally:
```bash
./packaging/debian/build-deb.sh dist
sudo apt install ./dist/pipelink_*.deb
```

### From Source (`Makefile`)

```bash
make build
sudo make install PREFIX=/usr
```

This installs `/usr/bin/pipelink`, the systemd user unit `/usr/lib/systemd/user/pipelink.service`, [config.example.json](packaging/config.example.json), and `LICENSE`.

You can also install the binary directly via Cargo (`crates.io` or git):
```bash
cargo install pipelink
# or directly from Codeberg git:
cargo install --git https://codeberg.org/kuroki-kael/pipelink pipelink
```

## Daemon Setup (`systemd --user`)

Enable and start the background routing daemon:
```bash
systemctl --user enable --now pipelink
```

Or run it in the foreground for testing:
```bash
pipelink daemon
```

## CLI Usage

```bash
# Show daemon status, active mix, output, channels, and app routes
pipelink --status

# Channels
pipelink channel list
pipelink channel add Gaming
pipelink channel remove Gaming

# Mixes & active output mix
pipelink mix list
pipelink mix add Stream
pipelink mix active Personal

# Channel-to-mix routes, gain (%), and mute
pipelink route list
pipelink route set Music Stream
pipelink route gain Music Personal 75
pipelink route mute Voice Stream
pipelink route unmute Voice Stream
pipelink route remove Music Stream

# Applications
pipelink app list
pipelink app route "Firefox" Music
pipelink app unroute "Firefox"

# Output sinks & WirePlumber default sink
pipelink output list
pipelink output set "Headphones"
pipelink default-sink System        # alias: wp-default
```

## Configuration & State

Files follow XDG Base Directory paths by default and can be overridden via environment variables:

| Purpose | Default Path | Override Env Var |
|---|---|---|
| Topology (`ConfigGraph`) | `$XDG_CONFIG_HOME/pipelink/config.json` (`~/.config/pipelink/config.json`) | `PIPELINK_CONFIG` |
| Mixer state (`RuntimeState`) | `$XDG_STATE_HOME/pipelink/state.json` (`~/.local/state/pipelink/state.json`) | `PIPELINK_STATE` |

On the very first start without an existing `config.json`, `pipelink daemon` creates a starter topology matching [packaging/config.example.json](packaging/config.example.json):
- **Channels:** `System`, `Music`, `Games`, `Voice`
- **Mixes:** `Personal` (linked to all four channels) and `Stream` (empty)

## License

Licensed under the [GNU General Public License v3.0 or later (GPL-3.0-or-later)](LICENSE).
