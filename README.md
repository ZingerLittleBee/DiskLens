# DiskLens

Language: 🇺🇸 English | [🇨🇳 简体中文](./README.zh-CN.md)

A high-performance disk space analyzer built with Rust, featuring a TUI terminal interface.

```
┌─────────────────────────────────────────────────────────────────────────┐
│ DiskLens  | / > Users > zingerbee > Documents  (33.6 GB)               │
├──────────────────────────────┬──────────────────────────────────────────┤
│                              │  Name                       Size v      │
│         ╱───╲               │ 📁 Projects      15.2 GB     45.3%     │
│       ╱       ╲             │ 📁 Photos         8.7 GB     25.9%     │
│      │  33.6   │             │ 📁 Downloads      5.1 GB     15.2%     │
│      │   GB    │             │ 📁 Videos         3.2 GB      9.5%     │
│       ╲       ╱             │ 📄 Others          1.4 GB      4.1%     │
│         ╲───╱               │                                         │
│                              │ Total: 33.6 GB / 256 items             │
├──────────────────────────────┴──────────────────────────────────────────┤
│ ⚠ 23 errors | Scanned: 1,234 files | Speed: 500/s                     │
├─────────────────────────────────────────────────────────────────────────┤
│ j/k: Navigate  Enter: Open  Backspace: Back  s: Sort  ?: Help  q: Quit│
└─────────────────────────────────────────────────────────────────────────┘
```

## Features

- **Fast Async Scanning** — Powered by tokio async runtime, auto-detects storage type (SSD/HDD) and adjusts concurrency
- **Ring Chart Visualization** — Colorful ring chart drawn with Unicode half-block characters (▀▄█) for intuitive disk usage display
- **Drill-down Navigation** — Vim-style keybindings with directory drill-down, parent navigation, and jump-to-first/last
- **Multiple Sort Modes** — Sort by size, name, or modification time with ascending/descending toggle
- **Smart Merging** — Small files/folders auto-merged into "Others" with adjustable threshold (0.5%/1%/2%/5%)
- **Multi-format Export** — JSON, Markdown, HTML (pure CSS, dark theme, collapsible directory tree)
- **Cache System** — bincode binary cache with mtime + inode change detection and atomic writes
- **Error Tolerant** — Permission denied, symlink cycles, and other errors won't interrupt scanning; press `e` to view the full error list

## Installation

```bash
# Install from crates.io
cargo install disklens

# Or build from source
git clone https://github.com/ZingerLittleBee/DiskLens.git
cd DiskLens
cargo install --path .
```

Requires Rust 2024 edition (nightly or 1.85+).

## Usage

```bash
# Analyze current directory
disklens

# Analyze a specific path
disklens /home/user/Documents

# Limit scan depth
disklens -d 5 /path

# Custom concurrency
disklens -c 128 /path

# Follow symbolic links
disklens --follow-symlinks /path

# Non-interactive mode: export JSON directly
disklens --export-json report.json /path
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` / `l` | Enter directory |
| `Backspace` / `h` | Go to parent |
| `gg` | Jump to first item |
| `G` | Jump to last item |
| `Tab` / `←` `→` | Switch focus panel (ring chart ↔ file list) |

### Actions

| Key | Action |
|-----|--------|
| `s` | Cycle sort mode (size → name → modified time) |
| `t` | Cycle merge threshold (0.5% → 1% → 2% → 5%) |
| `x` | Export JSON report |
| `e` | View error list |
| `?` | Show help panel |
| `q` / `Ctrl+C` | Quit |

## Technical Details

### Concurrency Model

The scanner uses `tokio::spawn` for async recursive scanning of each subdirectory, with a `Semaphore` controlling max concurrent I/O. Concurrency is auto-tuned by storage type:

| Storage Type | Concurrency |
|-------------|-------------|
| SSD / NVMe | 256 |
| HDD | 32 |
| Unknown | 64 |

A `DashSet<PathBuf>` tracks visited paths to prevent symlink cycles. Progress updates use atomic counters (`AtomicU64`/`AtomicUsize`) to avoid lock contention.

### Cache

Cache is stored at `~/Library/Caches/disklens` (macOS) or `~/.cache/disklens` (Linux), serialized with bincode. Change detection: mtime → inode (Unix) → rescan on mismatch. Writes use temp file + rename for atomic operation, ensuring crash safety.

## License

MIT
