[![Test](https://github.com/MelsRoughSketch/Rapid/actions/workflows/rust.yml/badge.svg)](https://github.com/MelsRoughSketch/Rapid/actions/workflows/rust.yml)
[![Windows Release](https://github.com/MelsRoughSketch/Rapid/actions/workflows/windows-release.yml/badge.svg)](https://github.com/MelsRoughSketch/Rapid/actions/workflows/windows-release.yml)

# Rapid

Rapid is a native Rust desktop editor for the generated `Rapid.html` document used by this project.

## What It Does

- Load the current `Rapid.html`
- Edit nested sections and items
- Reorder sections and items
- Save the document back to `Rapid.html`

## Tech Stack

- Rust 2024
- `eframe` 0.34
- `egui` 0.34

## Prerequisites

- A recent Rust toolchain with Cargo installed

## Build

```bash
cargo build --release --target x86_64-pc-windows-gnu
```

The Windows GNU build output is placed under `target/x86_64-pc-windows-gnu/release/`.

## Run

```bash
cargo run --bin Rapid
```

## File Behavior

- Rapid loads `Rapid.html` from the current working directory on startup.
- `Load` reloads `Rapid.html` from the current working directory.
- `Save` writes the current document back to `Rapid.html` in the current working directory.

## Development Checks

```bash
cargo fmt --check
cargo test
```
