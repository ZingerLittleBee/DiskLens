# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

DiskLens is a high-performance TUI disk space analyzer built with Rust. It uses tokio for async filesystem scanning and ratatui for terminal UI rendering, featuring a ring chart visualization alongside a navigable file list.

## Build & Development Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build (LTO enabled)
cargo run -- [path]            # Run with optional target path
cargo run -- --export-json out.json [path]  # Non-interactive JSON export
cargo test                     # Run all tests (unit + integration)
cargo test test_scan_basic     # Run a single test
RUST_LOG=debug cargo run       # Enable tracing output (logs to stderr)
```

CLI flags: `-d` (max depth), `-c` (concurrency), `--follow-symlinks`, `--export-json <path>`.

## Architecture

Layered architecture with single-direction dependencies:

```
main.rs          CLI (clap) → builds Settings → launches App
app.rs           Orchestrator: owns AppState, spawns Scanner on tokio, runs event loop
core/scanner.rs  Async recursive scan using tokio::spawn per subdirectory, Semaphore for concurrency control
core/events.rs   mpsc::unbounded_channel carrying Event variants between scanner and UI
core/progress.rs Lock-free counters (AtomicU64/AtomicUsize) for real-time scan progress
ui/app_state.rs  UI state machine: ViewMode (Scanning→Normal→Help/ErrorList), navigation stack, sort/threshold state
ui/renderer.rs   Dispatches rendering by ViewMode; splits layout into breadcrumb, ring chart + file list, status bar, key hints
ui/input.rs      Maps crossterm KeyEvents to InputAction per ViewMode; supports vim-style navigation + gg/G
models/node.rs   Recursive tree: Node::from_directory aggregates size/file_count/dir_count from children
```

**Key data flow:** Scanner emits `Event::Progress`/`ScanError` via channel → `App::event_loop` uses `tokio::select!` to multiplex terminal input, scan events, and a 100ms tick → updates `AppState` → `renderer::render` draws current frame.

**Concurrency model:** Scanner acquires a `Semaphore` permit per directory, spawns `tokio::spawn` tasks for subdirectories. Concurrency is auto-tuned by storage type detection (SSD=256, HDD=32, Unknown=64). Progress is tracked with atomics to avoid lock contention. A `DashSet<PathBuf>` prevents symlink cycles.

**Terminal input:** A dedicated `spawn_blocking` thread polls crossterm events and sends them through an unbounded mpsc channel, avoiding blocking the async runtime.

## Module Map

- `models/` — Pure data: `Node` (recursive tree), `ScanResult`, `ScanError`, `PathIndex`/`SizeIndex` (search and top-N queries)
- `core/` — Scanner, Analyzer (sort/merge utilities), Cache (stub), ProgressTracker, Event bus
- `ui/` — AppState (state machine), renderer, input handler, widgets (ring_chart, file_list, breadcrumb, progress_bar, status_bar, help_panel)
- `export/` — JSON (implemented), Markdown/HTML (stubs)
- `config/settings.rs` — Settings with platform-specific defaults and storage type detection

## Key Patterns

- `edition = "2024"` — Uses Rust 2024 edition
- Node percentages are computed dynamically via `node.percentage(total_size)`, never stored
- `AppState::sorted_children()` returns `Vec<&Node>` sorted per current SortMode; `current_node()` does recursive `find_node` from scan result root by matching `current_path`
- Navigation uses a `path_stack: Vec<PathBuf>` for back-tracking through directories
- Progress events are throttled to 100ms intervals in the scanner to reduce channel pressure
- The `pending_g` flag in AppState implements the vim `gg` two-key sequence

## Testing

Tests are in `tests/integration_test.rs`. They create temp directories under `std::env::temp_dir()` with `disklens_test_` prefix. Tests cover: scanner (basic scan, empty dir), Node (percentage, human_readable_size), Analyzer (sort, merge), PathIndex/SizeIndex, JSON export round-trip, and Settings defaults.
