# spat.ai

A desktop application that assists Teamfight Tactics players by analyzing gameplay videos. Uses computer vision to recognize the board state (champions, gold, level, stage) and provides guidance.

> Named after the Spatula — the most iconic item in TFT.

## Architecture

```
Video file ──► Game area detection ──► Vision pipeline ──► Game state ──► UI
(ffmpeg)       (HUD bar + card         (template match     (watch           (React +
                pattern matching)       + Tesseract OCR)    channels)        Tauri)
```

The app is **video-only** — it decodes video files via ffmpeg and processes each frame through the vision pipeline:

1. **Game area detection** finds the TFT game window boundary within arbitrary frames (handles windowed recordings, streamer overlays, desktop content around the game)
2. **Dynamic layout detection** finds UI elements (shop cards, gold, level, stage) by analyzing frame content rather than hardcoding pixel coordinates
3. **Champion recognition** via normalized cross-correlation against 102 Set 16 champion icon templates
4. **OCR** via Tesseract CLI for gold, level, and stage readout

**Tech stack**: Tauri 2.0 (Rust backend, React/TypeScript frontend), `ffmpeg-next` for video decode, Tesseract CLI for OCR, Zustand for state management.

## Project Structure

```
src/                              React/TypeScript frontend
  components/companion/           Main dashboard (status, shop, economy, video loader)
  components/overlay/             Transparent always-on-top overlay (WIP)
  hooks/                          Zustand stores + Tauri event listeners

src-tauri/                        Rust backend
  src/pipeline.rs                 Video → vision → state → frontend orchestration
  src/commands/capture.rs         Tauri IPC commands
  crates/
    tft-capture/                  Video file decode (ffmpeg)
    tft-vision/                   Game area detection, template matching, OCR, layout detection
    tft-state/                    Game state data structures
    tft-advisor/                  Advice engine (placeholder)
    tft-data/                     Champion metadata + static game data

data/
  champions.json                  Champion metadata (Set 16)
  templates/champions/            102 champion icon PNGs
  meta/comps.json                 Meta composition data

scripts/
  fetch-templates.py              Download champion data from Riot Data Dragon
  update-meta.py                  Update meta compositions
```

## Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [Node.js](https://nodejs.org/) (18+)
- [Tauri CLI](https://tauri.app/): `cargo install tauri-cli --version "^2"`
- macOS: Xcode Command Line Tools (`xcode-select --install`)

```bash
brew install ffmpeg pkg-config tesseract
```

## Getting Started

```bash
# Install frontend dependencies
npm install

# Download champion data + icons from Riot Data Dragon
python3 scripts/fetch-templates.py

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

## Usage

### Video file analysis

Click **Load Video** in the companion window to open a TFT gameplay recording (mp4, mkv, mov, webm, avi). The video is decoded via ffmpeg and each frame is processed through the vision pipeline. The app automatically detects the TFT game area within each frame, so it works with both fullscreen recordings and windowed gameplay (with desktop content, streamer overlays, etc.).

### Debugging vision output

```bash
# Analyze a single screenshot from the CLI
cargo run -p tft-vision --bin analyze_frame -- screenshot.png
```

Or use the debug button in the app — it saves the current frame, the detected game area crop, and all detected region crops to `/tmp/spat_ai_debug/`.

## Implementation Status

- [x] **Phase 0** — Project scaffolding (Tauri + Cargo workspace + React frontend)
- [x] **Phase 1** — Video capture pipeline (ffmpeg decode, game area detection)
- [x] **Phase 2** — Shop champion recognition (NCC template matching, 102 champion templates)
- [x] **Phase 3** — Gold/level/stage OCR (Tesseract CLI, graceful fallback)
- [ ] **Phase 4** — Rule engine + shop/econ advice
- [ ] **Phase 5** — Item recognition + recommendations
- [ ] **Phase 6** — Overlay window
- [ ] **Phase 7** — Claude API integration
- [ ] **Phase 8** — Board recognition
- [ ] **Phase 9** — Windows support + packaging

## License

MIT
