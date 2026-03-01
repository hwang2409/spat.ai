# spat.ai

A desktop application that assists Teamfight Tactics players during live gameplay. Captures the game screen (or analyzes a video recording), uses computer vision to recognize the board state (champions, gold, level, stage), and provides real-time guidance.

> Named after the Spatula — the most iconic item in TFT.

## Architecture

```
Frame source ──► Vision pipeline ──► Game state ──► UI
(xcap / video)   (template match     (watch           (React +
                  + Tesseract OCR)    channels)        Tauri)
```

The vision pipeline is **source-agnostic** — it receives `RgbaImage` frames from either live screen capture or video file decode and processes them identically:

- **Dynamic layout detection** finds UI elements (shop cards, gold, level, stage) by analyzing frame content rather than hardcoding pixel coordinates
- **Champion recognition** via normalized cross-correlation against 102 Set 16 champion icon templates
- **OCR** via Tesseract CLI for gold, level, and stage readout

**Tech stack**: Tauri 2.0 (Rust backend, React/TypeScript frontend), `xcap` for screen capture, `ffmpeg-next` for video decode, Tesseract CLI for OCR, Zustand for state management.

## Project Structure

```
src/                              React/TypeScript frontend
  components/companion/           Main dashboard (status, shop, economy, video loader)
  components/overlay/             Transparent always-on-top overlay (WIP)
  hooks/                          Zustand stores + Tauri event listeners

src-tauri/                        Rust backend
  src/pipeline.rs                 Capture → vision → state → frontend orchestration
  src/commands/capture.rs         Tauri IPC commands
  crates/
    tft-capture/                  Screen capture (xcap) + video file decode (ffmpeg)
    tft-vision/                   Template matching, OCR, dynamic layout detection
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

### Live screen capture

The app auto-detects the TFT game window by title. Use the **Window Picker** to manually select a different window if needed.

On macOS, Screen Recording permission must be granted to the app.

### Video file analysis

Click **Load Video** in the companion window to open a TFT gameplay recording (mp4, mkv, mov, webm, avi). The video is decoded via ffmpeg and fed through the same vision pipeline as live capture. The status panel shows `[Video] filename.mp4` while processing.

### Debugging vision output

```bash
# Analyze a single screenshot from the CLI
cargo run -p tft-vision --bin analyze_frame -- screenshot.png
```

Or use the debug button in the app — it saves the current frame and all detected region crops to `/tmp/spat_ai_debug/`.

## Implementation Status

- [x] **Phase 0** — Project scaffolding (Tauri + Cargo workspace + React frontend)
- [x] **Phase 1** — Screen capture (`xcap`, window detection, capture loop)
- [x] **Phase 2** — Shop champion recognition (NCC template matching, 102 champion templates)
- [x] **Phase 3** — Gold/level/stage OCR (Tesseract CLI, graceful fallback)
- [x] **Phase 3.5** — Video file analysis (ffmpeg decode, same pipeline)
- [ ] **Phase 4** — Rule engine + shop/econ advice
- [ ] **Phase 5** — Item recognition + recommendations
- [ ] **Phase 6** — Overlay window
- [ ] **Phase 7** — Claude API integration
- [ ] **Phase 8** — Board recognition
- [ ] **Phase 9** — Windows support + packaging

## License

MIT
