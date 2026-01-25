# Build Instructions

RustFrame is a Tauri app with a Rust backend and a Vite/React frontend.

## Prerequisites
- Rust toolchain (stable)
- Node.js (for ui/)
- Platform toolchain:
  - Windows: Visual Studio Build Tools (MSVC)
  - macOS: Xcode Command Line Tools

## Install Frontend Dependencies
From the repo root:

```
cd ui
npm install
```

## Development
```
cargo tauri dev
```

This runs the Vite dev server defined in tauri.conf.json and launches the Tauri app.

## Release Build
```
cargo tauri build
```

This produces platform bundles under target/release/bundle.
