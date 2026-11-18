# doom-term

[![CI](https://github.com/Dafenxz0/doom-term/actions/workflows/ci.yml/badge.svg)](https://github.com/Dafenxz0/doom-term/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A Wolfenstein / DOOM-style textured 3D raycaster engine running directly in your terminal in pure Rust — featuring procedural `64x64` textures, perspective floor/ceiling casting, distance fog with a dynamic flashlight cone, animated sliding doors, Z-buffered patrolling billboard sprites, a live corner minimap with FOV cone, `rayon` parallel rendering, and a 24-bit TrueColor Unicode half-block (`▀`) ANSI encoder with run-length color deduplication.

## Features

- **DDA Raycaster + Perspective Floor & Ceiling:** Sub-pixel wall texture sampling with side-wall depth shading and seamless perspective-correct floor/ceiling casting.
- **Procedural `64x64` Textures:** Zero external image assets required — generates `Brick`, `TechMetal`, `Hazard`, `Stone`, `Door`, `Floor`, `Ceiling`, plus transparent `EnergyOrb`, `Pillar`, and `CyberDrone` sprite atlases at startup.
- **Dynamic Flashlight & Distance Fog:** Smooth radial spotlight cone (`F` to toggle) combined with exponential depth fog.
- **Animated Sliding Doors:** Press `E` or `Space` to slide blast doors open/closed (`Closed → Opening → Open → Closing`) with partial ray passthrough and automatic safety re-open if standing in the doorway.
- **1D Z-Buffer Billboard Sprites:** Back-to-front sorted sprites occluded per-column against wall depth, including pulsing bobbing `EnergyOrb`s, cylindrical `Pillar`s, and wall-reversing patrolling `CyberDrone`s.
- **24-Bit TrueColor Half-Block (`▀`) Encoder:** Doubles vertical terminal resolution by packing two RGB pixels per character cell (`fg` top, `bg` bottom), using per-row run-length color deduplication and parallel `rayon` row encoding.
- **Headless Benchmark Mode:** `--headless --frames N --width W --height H --benchmark` exercises the full simulation, parallel raycaster, sprite pass, minimap, and ANSI encoder without requiring a TTY.

## Controls

| Key | Action |
| :--- | :--- |
| `W` / `S` or `↑` / `↓` | Move forward / backward |
| `A` / `D` or `←` / `→` | Turn left / right |
| `Q` / `C` | Strafe left / right |
| `E` / `Space` | Open / close sliding door ahead |
| `F` | Toggle dynamic flashlight cone |
| `M` | Toggle corner minimap & FOV cone |
| `1` / `2` | Switch between Sector 01 & Sector 02 |
| `Esc` / `X` / `Ctrl+C` | Quit |

## Engine Architecture

```text
src/
├── main.rs       # CLI entry point (--headless / --benchmark / interactive TTY)
├── lib.rs        # Library module exports
├── math.rs       # Rgb, Vec2, Camera, exponential fog & flashlight cone shading
├── textures.rs   # Procedural 64x64 wall, floor, ceiling & sprite texture atlas
├── map.rs        # ASCII map parser, sliding door state machine, circle collision
├── raycaster.rs  # Parallel DDA wall + floor/ceiling casting & column-major Framebuffer
├── sprite.rs     # Drone patrol AI, bobbing animation & Z-buffered billboard renderer
├── hud.rs        # Rasterized corner minimap with FOV cone & retro ANSI status bar
├── ansi.rs       # Parallel half-block (▀) 24-bit TrueColor encoder + RLE color dedup
└── engine.rs     # GameState, CLI parser, headless benchmark & crossterm interactive loop
```

### Rendering Pipeline (Per Frame)

1. **Simulation (`GameState::step`):** Advance sliding door animations (`open_amount ∈ [0, 1]`) and patrol `CyberDrone`s with wall bounce checks.
2. **Parallel World Pass (`rayon`):** `Framebuffer::pixels` is chunked by column (`par_chunks_mut`), executing DDA raycasts, recording `z_buffer[x]`, and shading ceiling, wall, and floor pixels concurrently.
3. **Z-Buffered Sprite Pass:** Sprites are sorted back-to-front, projected into camera space, and drawn column-by-column only where `transform_y < z_buffer[x]`.
4. **Minimap Overlay:** Draws walls, door states, sprites, player position, and a 24-ray FOV cone into the top-right corner of the framebuffer.
5. **Parallel Half-Block ANSI Encode (`rayon`):** Encodes each terminal row pair `(2y, 2y+1)` in parallel, skipping redundant `\x1b[38;2;...` / `\x1b[48;2;...` escapes and emitting spaces when top and bottom pixels match.

## Building & Running

Requires a Rust toolchain (`rustc` / `cargo`) and a terminal with 24-bit TrueColor + UTF-8 support:

```bash
# Run interactively (Sector 01)
cargo run --release

# Start directly in Sector 02
cargo run --release -- --map 2

# Run automated headless benchmark (no TTY required)
cargo run --release -- --headless --frames 10 --width 160 --height 48 --benchmark

# Run full test suite and checks
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --verbose
```

## License

MIT — see [LICENSE](LICENSE).
