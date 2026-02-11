# DiskLens

高性能磁盘空间分析工具，基于 Rust 构建，提供 TUI 终端交互界面。

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

## 功能特性

- **高速异步扫描** — 基于 tokio 异步运行时，自动检测存储类型（SSD/HDD）并调整并发度
- **圆环图可视化** — 使用 Unicode 半块字符（▀▄█）绘制的彩色圆环图，直观展示磁盘占用比例
- **钻取式导航** — Vim 风格快捷键，支持进入子目录、返回上级、跳转首尾项
- **多排序模式** — 按大小、名称、修改时间排序，支持升序/降序切换
- **智能合并** — 小文件/文件夹自动合并为 "Others"，可调节阈值（0.5%/1%/2%/5%）
- **多格式导出** — JSON、Markdown、HTML（纯 CSS，暗色主题，可折叠目录树）
- **缓存系统** — bincode 二进制缓存，基于 mtime + inode 的变更检测，原子写入
- **错误容忍** — 权限拒绝、符号链接循环等错误不中断扫描，可按 `e` 查看完整错误列表

## 安装

```bash
# 从源码构建
git clone https://github.com/your-username/disklens.git
cd disklens
cargo install --path .
```

需要 Rust 2024 edition (nightly 或 1.85+)。

## 使用

```bash
# 分析当前目录
disklens

# 分析指定路径
disklens /home/user/Documents

# 限制扫描深度
disklens -d 5 /path

# 自定义并发数
disklens -c 128 /path

# 跟随符号链接
disklens --follow-symlinks /path

# 非交互模式：直接导出 JSON
disklens --export-json report.json /path
```

## 快捷键

### 导航

| 按键 | 功能 |
|------|------|
| `j` / `↓` | 向下移动 |
| `k` / `↑` | 向上移动 |
| `Enter` / `l` | 进入目录 |
| `Backspace` / `h` | 返回上级 |
| `gg` | 跳到首项 |
| `G` | 跳到末项 |
| `Tab` / `←` `→` | 切换焦点面板（圆环图 ↔ 文件列表）|

### 操作

| 按键 | 功能 |
|------|------|
| `s` | 切换排序模式（大小 → 名称 → 修改时间）|
| `t` | 切换合并阈值（0.5% → 1% → 2% → 5%）|
| `x` | 导出 JSON 报告 |
| `e` | 查看错误列表 |
| `?` | 显示帮助面板 |
| `q` / `Ctrl+C` | 退出 |

## 技术细节

### 并发模型

扫描器使用 `tokio::spawn` 对每个子目录进行异步递归扫描，通过 `Semaphore` 控制最大并发 I/O 数。并发度根据存储类型自动调整：

| 存储类型 | 并发数 |
|----------|--------|
| SSD / NVMe | 256 |
| HDD | 32 |
| 未知 | 64 |

使用 `DashSet<PathBuf>` 追踪已访问路径，防止符号链接循环。进度更新通过原子计数器（`AtomicU64`/`AtomicUsize`）实现，避免锁竞争。

### 缓存

缓存位于 `~/Library/Caches/disklens`（macOS）或 `~/.cache/disklens`（Linux），使用 bincode 序列化。变更检测机制：mtime → inode（Unix）→ 不一致则重新扫描。写入采用 temp file + rename 的原子操作，确保中断安全。

## License

MIT
