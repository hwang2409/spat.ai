# spat.ai

A desktop application that assists Teamfight Tactics players during live gameplay. Captures the game screen, uses computer vision to recognize the board state (champions, items, gold, level), and provides real-time guidance on compositions, items, and economy decisions.

Uses a hybrid AI approach: a rule-based engine for instant advice + Claude API for contextual strategic guidance.

> Named after the Spatula — the most iconic item in TFT.

## Architecture

```
Screen Capture (xcap) → CV Pipeline (template matching + OCR) → Game State → Advisor (rules + LLM) → UI (overlay + companion)
```

**Tech stack**: Tauri 2.0 (Rust backend, React/TypeScript frontend), `xcap` for cross-platform capture, `imageproc` for template matching, `leptess` for OCR, Claude API for contextual advice.

## Project Structure

```
tft/
├── src/                              # React/TypeScript frontend
│   ├── App.tsx                       # Root component, window routing
│   ├── types/                        # Game state + advice types
│   ├── hooks/                        # Zustand stores + event listeners
│   └── components/
│       ├── overlay/                  # Transparent always-on-top panel
│       └── companion/               # Full dashboard window
├── src-tauri/                        # Rust backend
│   ├── src/
│   │   ├── lib.rs                    # Tauri setup + command registration
│   │   ├── pipeline.rs              # Capture → CV → state → advice loop
│   │   └── commands/                # Tauri IPC commands
│   └── crates/
│       ├── tft-capture/             # Screen capture (xcap)
│       ├── tft-vision/              # CV: template matching + OCR
│       ├── tft-state/               # Game state model
│       ├── tft-advisor/             # Rule engine + LLM advisor
│       └── tft-data/               # Static game data (Data Dragon)
├── data/
│   ├── templates/{champions,items}/ # Icon templates for CV matching
│   └── meta/comps.json             # Meta composition definitions
└── scripts/
    ├── fetch-templates.py           # Download icons from Data Dragon
    └── update-meta.py               # Update meta comp data
```

## Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [Node.js](https://nodejs.org/) (18+)
- [Tauri CLI](https://tauri.app/): `cargo install tauri-cli --version "^2"`
- macOS: Xcode Command Line Tools (`xcode-select --install`)
- Windows: [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

## Getting Started

```bash
# Install frontend dependencies
npm install

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

## How It Works

1. **Screen Capture**: Finds the TFT game window by title and captures frames at ~2 FPS using `xcap`
2. **Computer Vision**: Crops known screen regions (shop, board, gold/level) and matches against champion/item icon templates
3. **Game State**: Assembles recognized data into a structured game state, tracking changes over time
4. **Advisor**: A rule engine scores your board against meta compositions and recommends shop actions, item builds, and economy decisions. Optionally calls Claude API for deeper strategic insight.
5. **UI**: A companion window shows the full dashboard; a transparent overlay sits alongside the game with compact advice

## Screen Regions

Screen positions are defined as normalized coordinates (0.0–1.0) relative to the game window, so they scale to any resolution:

| Region | Purpose |
|--------|---------|
| Shop (5 slots) | y≈0.94, x from 0.25 to 0.77 |
| Gold | (0.45, 0.82) |
| Level | (0.22, 0.95) |
| Stage | (0.47, 0.0) |

## Implementation Status

- [x] **Phase 0** — Project scaffolding (Tauri + Cargo workspace + React frontend)
- [x] **Phase 1** — Screen capture (`xcap`, window detection, capture loop)
- [ ] **Phase 2** — Shop champion recognition (template matching)
- [ ] **Phase 3** — Gold/level/stage OCR
- [ ] **Phase 4** — Rule engine + shop/econ advice
- [ ] **Phase 5** — Item recognition + recommendations
- [ ] **Phase 6** — Overlay window
- [ ] **Phase 7** — Claude API integration
- [ ] **Phase 8** — Board recognition
- [ ] **Phase 9** — Windows support + packaging

## License

MIT
