# dota-clock

A Wayland overlay that shows upcoming Dota 2 game events using Game State Integration (GSI).

![screenshot](screenshot.png)

## Features

- Transparent click-through overlay via gtk4-layer-shell
- Countdown timers for runes, lotus pools, wisdom shrines, outposts, day/night, tormentor, neutral item tiers, siege creeps
- Pull and stack timing indicators
- Sub-second accurate clock synced to GSI
- Auto-hides when not in a game
- Patch-based timing system — all event and recurring timings are defined per-patch in `src/patches/`
- Configurable position, icon size, and max visible icons

## Setup

### 1. Build

```sh
nix build
# or within the dev shell:
nix develop
cargo build --release
```

### 2. Install GSI config

Copy `gamestate_integration_dotaclock.cfg` to your Dota 2 GSI config directory:

```sh
cp gamestate_integration_dotaclock.cfg ~/.steam/steam/steamapps/common/dota\ 2\ beta/game/dota/cfg/gamestate_integration/
```

### 3. Run

```sh
./result/bin/dota-clock
```

The overlay appears when a game starts and hides otherwise.

## Configuration

Config lives at `~/.config/dota-clock/config.toml` (created with defaults on first run):

```toml
anchor = "bottom-right"   # bottom-right, bottom-left, top-right, top-left
margin_bottom = 10
margin_right = 470
margin_top = 0
margin_left = 0
icon_size = 40
max_icons = 10
```

## Adding a new patch

Create a new file in `src/patches/` (e.g. `v7_42.rs`) implementing the `Patch` trait, then update `latest()` in `src/patches/mod.rs` to point to it.