# Cuayo widget

Cuayo widget is a Tauri-based desktop character widget for Windows.

- Korean: `README.ko.md`

## Features

- Transparent always-on-top character window
- Right-click settings panel (size, Pumpkin, Exit)
- Pumpkin interactions (spawn, drag, chase, eat)
- Pumpkin index system
  - Starts at `100`
  - Decreases over time (`10` per minute)
  - Pumpkin gives `+10` (max `100`)
  - Expression and voice reactions by pumpkin index range
- Pumpkin index value is persisted in local storage

## Tech Stack

- Frontend: HTML / CSS / Vanilla JavaScript (`web/`)
- Desktop runtime: Tauri v2
- Backend: Rust (`src-tauri/`)

## Prerequisites

- Node.js (LTS recommended)
- Rust + Cargo
- Windows environment (bundle target is `msi`)
- Microsoft Edge WebView2 Runtime
  - This app requires WebView2 to run.

## Development

```bash
npm install
npm run dev
```

## Build

```bash
npm run build
```

Build outputs:

- EXE: `src-tauri/target/release/app.exe`
- MSI: `src-tauri/target/release/bundle/msi/Cuayo widget_1.0.1_x64_en-US.msi`

## Project Structure

```text
cuayo-widget/
|- web/                 # frontend static files (ui, voice, images)
|- src-tauri/           # Tauri/Rust app code and bundle config
|- package.json         # npm scripts (dev/build)
`- README.md
```

## Version

Current version: `1.0.1`

### 1.0.1 Updates

- User-facing metric name changed from `Hunger` to `Pumpkin Index`.
