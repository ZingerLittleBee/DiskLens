# DiskLens 设计文档

**版本：** 2.0（修订版）
**日期：** 2026-02-11
**作者：** DiskLens Team
**修订说明：** 根据架构评审团队的反馈进行重大修订

---

## 变更日志

### v2.0 (2026-02-11)
- 🔴 **架构调整**：修正术语（MVC → 多层架构）
- 🔴 **并发模型重构**：改用纯 tokio 异步模型，移除 rayon 线程池
- 🔴 **数据模型优化**：移除 Node.percentage 冗余字段
- 🔴 **缓存增强**：改进变更检测机制（mtime + inode + 文件数量）
- 🟡 **依赖更新**：添加 thiserror, tracing, smallvec 等
- 🟡 **快捷键调整**：解决 u 键冲突
- 🟡 **UI 增强**：添加焦点指示器和错误计数器

---

## 1. 项目概述

### 1.1 项目定位

DiskLens 是一个高性能的磁盘空间分析工具，使用 Rust + ratatui 构建 TUI 界面，专注于：
- 快速异步扫描（支持从 GB 到 TB 级别）
- 直观的圆环图可视化
- 智能缓存和增量更新
- 详细的实时反馈

### 1.2 核心功能

**扫描功能：**
- 快速解析存储空间结构，分析文件和文件夹占用详情
- 支持输入路径参数来分析，不输入则默认分析整个磁盘
- 实时显示当前正在分析的文件信息
- 支持从小规模（GB 级）到大规模（TB 级）的磁盘分析

**可视化功能：**
- 左侧圆环图：直观显示目录占用比例
- 右侧列表视图：详细显示文件/文件夹信息
- 钻取式导航：可以深入查看文件夹内部情况
- 自适应合并：小文件/文件夹自动合并显示（默认 1% 阈值）

**增强功能：**
- 搜索/过滤：在结果中搜索特定文件或文件夹
- 排序：按大小、名称、修改时间排序
- 书签：标记关注的目录，快速跳转
- 多格式导出：JSON、Markdown、HTML

**性能特性：**
- **纯异步并发模型**：tokio 异步运行时 + Semaphore 限流控制
- **智能缓存**：增量扫描，提升重复分析速度
- **不阻塞系统**：自适应并发数，避免 I/O 过载
- **友好的用户提示**：详细的进度反馈和错误信息

---

## 2. 技术栈

### 2.1 核心依赖

**UI 层：**
- `ratatui` (0.28+)：TUI 框架
- `crossterm` (0.28+)：跨平台终端控制

**并发引擎：**
- `tokio` (1.42+)：异步运行时（所有 I/O 操作）
  - `tokio::fs`：异步文件系统操作
  - `tokio::sync::Semaphore`：并发控制
- `dashmap` (6.1+)：无锁并发 HashMap
- ~~`rayon`~~：移除（不适合 I/O 密集任务）

**存储和序列化：**
- `serde` (1.0+)：序列化/反序列化
- `bincode` (2.0+)：二进制缓存格式（快速）
- `serde_json` (1.0+)：JSON 导出

**文件系统：**
- ~~`walkdir`~~：移除，改用 `tokio::fs` 异步 API

**错误处理和日志：**
- `thiserror` (2.0+)：结构化错误定义
- `tracing` (0.1+)：结构化日志（替代传统日志）
- `tracing-subscriber` (0.3+)：日志订阅器

**内存优化：**
- `smallvec` (1.13+)：优化小向量分配
- `compact_str` (0.8+)：优化字符串存储

**数据可视化：**
- 自定义实现圆环图（基于 ratatui 的 Canvas widget）
- `unicode-width` (0.2+)：准确计算文本宽度

**其他：**
- `clap` (4.5+)：命令行参数解析
- `anyhow` (1.0+)：应用级错误处理
- `chrono` (0.4+)：时间处理

---

## 3. 核心架构

### 3.1 架构模式

**多层架构（Layered Architecture）**

DiskLens 采用多层架构，而非 MVC 模式。各层职责清晰，单向依赖：

```
┌─────────────────────────────────────┐
│  表示层 (Presentation Layer)        │  ← UI / TUI 界面
├─────────────────────────────────────┤
│  应用层 (Application Layer)         │  ← 应用逻辑、事件总线
├─────────────────────────────────────┤
│  领域层 (Domain Layer)              │  ← 核心业务逻辑
├─────────────────────────────────────┤
│  基础设施层 (Infrastructure Layer)  │  ← 文件系统、缓存、导出
└─────────────────────────────────────┘
```

### 3.2 目录结构

```
disklens/
├── src/
│   ├── main.rs              # 程序入口
│   ├── app.rs               # 全局应用状态（配置、事件总线）
│   ├── config/              # 配置管理 [新增]
│   │   ├── mod.rs
│   │   └── settings.rs      # 用户配置
│   ├── core/                # 领域层
│   │   ├── mod.rs
│   │   ├── scanner.rs       # 异步扫描引擎
│   │   ├── analyzer.rs      # 数据分析和聚合
│   │   ├── cache.rs         # 缓存管理器
│   │   ├── progress.rs      # 进度追踪
│   │   └── events.rs        # 事件总线 [新增]
│   ├── models/              # 数据模型
│   │   ├── mod.rs
│   │   ├── node.rs          # 文件/目录节点
│   │   ├── scan_result.rs   # 扫描结果
│   │   └── index.rs         # 索引结构 [新增]
│   ├── ui/                  # 表示层
│   │   ├── mod.rs
│   │   ├── app_state.rs     # UI 状态机
│   │   ├── renderer.rs      # 主渲染器
│   │   ├── input.rs         # 输入处理 [重命名自 events.rs]
│   │   └── widgets/
│   │       ├── mod.rs
│   │       ├── ring_chart.rs    # 圆环图
│   │       ├── file_list.rs     # 文件列表
│   │       ├── breadcrumb.rs    # 面包屑
│   │       ├── progress_bar.rs  # 进度条
│   │       ├── status_bar.rs    # 状态栏 [新增]
│   │       └── help_panel.rs    # 帮助面板
│   └── export/              # 基础设施层
│       ├── mod.rs
│       ├── json.rs          # JSON 导出
│       ├── markdown.rs      # Markdown 导出
│       └── html.rs          # HTML 导出
├── tests/                   # 集成测试
├── benches/                 # 性能基准测试
└── docs/                    # 文档
```

### 3.3 数据流

```
1. 扫描阶段：
   Scanner → Events (Progress/Error) → UI (实时更新)
   Scanner → Analyzer → Cache (持久化)

2. 分析阶段：
   Cache → Analyzer → App State → UI

3. 交互阶段：
   UI Input → App State → Events → Renderer

4. 导出阶段：
   App State → Exporter → 文件系统
```

### 3.4 模块职责

**app.rs**：
- 全局应用状态管理（配置、运行时状态）
- 初始化各个子系统
- 协调各层之间的交互

**config/**：
- 用户配置管理（阈值、并发数、缓存策略等）
- 配置文件读写（`~/.disklens/config.toml`）

**core/scanner**：
- 异步扫描文件系统
- 使用 tokio 和 Semaphore 控制并发
- 发送进度和错误事件

**core/analyzer**：
- 数据分析：计算大小、排序、合并小项
- 构建索引（路径索引、大小索引）
- 百分比计算（动态，不存储）

**core/cache**：
- 管理扫描结果缓存
- 增量变更检测（mtime + inode + 文件数量）
- 原子写入（temp file + rename）

**core/events**：
- 事件总线系统
- 统一管理进度、错误、UI 事件
- 解耦各模块之间的通信

**models**：
- 核心数据结构定义
- 不包含业务逻辑

**ui/app_state**：
- UI 状态机（当前目录、选中项、视图模式等）
- 响应用户输入
- 触发 UI 更新

**ui/input**：
- 键盘事件处理
- 快捷键映射

**export**：
- 多格式报告导出
- 独立于核心业务逻辑

---

## 4. 并发扫描引擎

### 4.1 扫描器架构（重构版）

**核心思路：** 纯 tokio 异步模型 + Semaphore 限流控制

```rust
use tokio::sync::{mpsc, Semaphore};
use dashmap::{DashMap, DashSet};
use std::sync::Arc;

// 核心结构
struct Scanner {
    semaphore: Arc<Semaphore>,                        // 并发限流
    progress_tx: mpsc::UnboundedSender<Event>,        // 事件通道
    result_map: Arc<DashMap<PathBuf, Node>>,          // 结果集
    visited: Arc<DashSet<PathBuf>>,                   // 防循环
    config: ScanConfig,                               // 扫描配置
}

struct ScanConfig {
    max_depth: Option<usize>,             // 最大深度限制
    max_concurrent_io: usize,             // 最大并发 I/O 数
    follow_symlinks: bool,                // 是否跟随符号链接
    merge_threshold: f64,                 // 合并阈值（默认 1%）
    ignore_patterns: Vec<String>,         // 忽略模式
    storage_type: StorageType,            // 存储类型 [新增]
}

#[derive(Debug, Clone, Copy)]
enum StorageType {
    SSD,      // 固态硬盘，支持高并发（256）
    HDD,      // 机械硬盘，需限制并发（32）
    Unknown,  // 未知类型，保守值（64）
}

impl ScanConfig {
    fn default() -> Self {
        let storage_type = detect_storage_type();
        let max_concurrent_io = match storage_type {
            StorageType::SSD => 256,
            StorageType::HDD => 32,
            StorageType::Unknown => 64,
        };

        Self {
            max_depth: None,
            max_concurrent_io,
            follow_symlinks: false,
            merge_threshold: 0.01,  // 1%
            ignore_patterns: vec![],
            storage_type,
        }
    }
}
```

### 4.2 扫描流程（异步版本）

```rust
impl Scanner {
    async fn scan(&self, root: PathBuf) -> Result<ScanResult> {
        // 1. 初始化
        self.visited.insert(root.clone());

        // 2. 递归异步扫描
        let root_node = self.scan_directory(root.clone()).await?;

        // 3. 后处理（使用 rayon 并行计算）
        let analyzed_node = rayon::spawn(|| {
            analyzer::analyze(root_node)
        }).await;

        Ok(ScanResult {
            root: analyzed_node,
            // ... 其他字段
        })
    }

    async fn scan_directory(&self, path: PathBuf) -> Result<Node> {
        // 获取信号量许可（限流）
        let _permit = self.semaphore.acquire().await?;

        // 异步读取目录
        let mut entries = tokio::fs::read_dir(&path).await?;
        let mut children = Vec::new();
        let mut tasks = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();

            // 检查是否已访问（防循环）
            if !self.visited.insert(entry_path.clone()) {
                self.report_error(entry_path, ErrorType::SymlinkCycle);
                continue;
            }

            let metadata = match entry.metadata().await {
                Ok(m) => m,
                Err(e) => {
                    self.report_error(entry_path, ErrorType::from(e));
                    continue;
                }
            };

            if metadata.is_dir() {
                // 异步递归扫描子目录
                let scanner = self.clone();
                let task = tokio::spawn(async move {
                    scanner.scan_directory(entry_path).await
                });
                tasks.push(task);
            } else {
                // 文件节点
                children.push(Node::from_file(entry_path, metadata));
            }

            // 发送进度事件
            self.report_progress(entry_path);
        }

        // 等待所有子任务完成
        for task in tasks {
            if let Ok(Ok(child_node)) = task.await {
                children.push(child_node);
            }
        }

        Ok(Node::from_directory(path, children))
    }
}
```

**关键改进：**
1. ✅ 纯异步 I/O：所有文件系统操作使用 `tokio::fs`
2. ✅ Semaphore 限流：精确控制并发数，避免 I/O 过载
3. ✅ 防止符号链接循环：使用 `DashSet` 追踪已访问路径
4. ✅ 异步递归：使用 `tokio::spawn` 并发扫描子目录
5. ✅ CPU 密集操作后置：扫描完成后使用 rayon 并行计算

### 4.3 存储类型检测

```rust
#[cfg(target_os = "linux")]
fn detect_storage_type() -> StorageType {
    // 读取 /sys/block/*/queue/rotational
    // 0 = SSD, 1 = HDD
    // 实现略
}

#[cfg(target_os = "macos")]
fn detect_storage_type() -> StorageType {
    // 使用 system_profiler SPStorageDataType
    // 实现略
}

#[cfg(target_os = "windows")]
fn detect_storage_type() -> StorageType {
    // 使用 IOCTL_STORAGE_QUERY_PROPERTY
    // 实现略
}
```

### 4.4 错误处理

```rust
#[derive(Debug, thiserror::Error)]
enum ScanError {
    #[error("权限拒绝: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("文件未找到: {path}")]
    NotFound { path: PathBuf },

    #[error("符号链接循环: {path}")]
    SymlinkCycle { path: PathBuf },

    #[error("I/O 错误: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
}
```

- 所有错误不中断扫描
- 通过事件总线发送错误通知
- 记录到 `ScanResult.errors`

---

## 5. 数据模型

### 5.1 核心数据结构（优化版）

```rust
use std::sync::Arc;
use std::path::Path;
use smallvec::SmallVec;

// 文件/目录节点
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Node {
    path: Arc<Path>,                      // 共享路径，避免克隆
    name: Arc<str>,                       // 共享文件名
    size: u64,                            // 字节
    size_on_disk: u64,                    // 实际磁盘占用
    node_type: NodeType,
    children: SmallVec<[Arc<Node>; 8]>,   // 小目录优化
    file_count: usize,                    // 文件数量（递归）
    dir_count: usize,                     // 目录数量（递归）
    modified: Option<SystemTime>,         // 最后修改时间
    inode: Option<u64>,                   // inode 编号（用于缓存检测）
    // ❌ 移除: percentage 字段（冗余，动态计算）
}

impl Node {
    // 动态计算百分比
    fn percentage(&self, total_size: u64) -> f64 {
        if total_size == 0 {
            0.0
        } else {
            (self.size as f64 / total_size as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum NodeType {
    File,
    Directory,
    Symlink,
    Other,
}

// 扫描结果
struct ScanResult {
    root: Arc<Node>,
    total_size: u64,
    total_files: usize,
    total_dirs: usize,
    scan_duration: Duration,
    errors: Vec<ScanError>,
    timestamp: SystemTime,
    path_index: PathIndex,                // 路径索引 [新增]
    size_index: SizeIndex,                // 大小索引 [新增]
}

// 路径索引（用于快速搜索）
struct PathIndex {
    map: HashMap<Arc<Path>, Arc<Node>>,
}

impl PathIndex {
    fn search(&self, pattern: &str) -> Vec<Arc<Node>> {
        self.map
            .iter()
            .filter(|(path, _)| path.to_string_lossy().contains(pattern))
            .map(|(_, node)| node.clone())
            .collect()
    }
}

// 大小索引（用于快速排序）
struct SizeIndex {
    sorted: Vec<Arc<Node>>,  // 按大小降序排列
}
```

### 5.2 数据优化

**内存优化：**
- `Arc<Path>` 和 `Arc<str>`：共享路径和文件名，避免重复存储
- `SmallVec<[Arc<Node>; 8]>`：小目录（≤8 个子项）无堆分配
- `Arc<Node>`：节点引用计数共享，避免深度克隆

**性能优化：**
- 移除 `percentage` 字段：动态计算，节省存储空间
- 添加索引结构：支持 O(1) 路径查找和 O(log n) 大小查询

**缓存友好：**
- 添加 `inode` 字段：用于可靠的变更检测

---

## 6. UI 组件设计

### 6.1 界面布局（增强版）

```
┌─────────────────────────────────────────────────────────────────┐
│ DiskLens v0.2.0 | / > Users > zingerbee > Documents   [列表]   │ ← 顶部：标题 + 面包屑 + 焦点指示
├────────────────────────────┬────────────────────────────────────┤
│                            │ ╭─ Name ──────────Size────%─────╮  │
│         ╱───╲              │ │ 📁 Projects   15.2 GB   45.3% │  │ ← 列表视图边框
│       ╱       ╲            │ │ 📁 Photos      8.7 GB   25.9% │  │
│      │         │           │ │ 📁 Downloads   5.1 GB   15.2% │  │
│      │  圆环图  │           │ │ 📁 Videos      3.2 GB    9.5% │  │
│      │         │           │ │ 📄 Others (合并 12 项) 1.4 GB 4.1% │ ← "Others" 显示合并数量
│       ╲       ╱            │ ╰───────────────────────────────╯  │
│         ╲───╱              │  Total: 33.6 GB / 256 items       │
│                            │  排序: 大小 ↓                      │ ← 排序指示器
├────────────────────────────┴────────────────────────────────────┤
│ ⚠️  23 个错误（按 'e' 查看） | 已扫描: 1,234 文件 | 速度: 500/s │ ← 错误计数器 + 进度信息
├──────────────────────────────────────────────────────────────────┤
│ ↑↓:导航 Enter:进入 Bksp:返回 /:搜索 s:排序 b:书签 x:导出 ?:帮助 q:退出 │
└──────────────────────────────────────────────────────────────────┘
```

**改进点：**
1. ✅ **焦点指示器**：右上角显示 `[列表]` 或 `[圆环图]`
2. ✅ **错误计数器**：状态栏左侧显示错误数量
3. ✅ **排序指示器**：列表下方显示当前排序方式
4. ✅ **列表视图边框**：明确视觉边界
5. ✅ **"Others" 详情**：显示合并的项目数量

### 6.2 Widget 职责

**RingChart（圆环图）:**
- 绘制彩色圆环，每个扇区代表一个子项
- 高亮与列表选中项对应的扇区
- 仅作为"视觉指示器"，不支持独立导航
- 对于 <5% 的扇区，不显示标签（避免重叠）

**FileList（文件列表）:**
- 显示当前目录的子项（名称、大小、百分比）
- 支持上下滚动、高亮选中项
- 图标区分文件/目录/符号链接
- 显示人类可读的大小格式（KB/MB/GB/TB）
- 排序指示器：`Name ↓`, `Size ↑`, `Modified ↓`

**Breadcrumb（面包屑）:**
- 显示当前路径层级
- 使用 ` > ` 分隔符

**StatusBar（状态栏）:**
- 左侧：错误计数器（`⚠️  23 个错误`）
- 中间：进度信息（`已扫描: 1,234 文件`）
- 右侧：扫描速度（`速度: 500/s`）

**ProgressBar（进度条）:**
- 显示扫描进度百分比
- 显示当前扫描的文件路径（截断长路径）
- 显示预估剩余时间

**HelpPanel（帮助面板）:**
- 按 `?` 或 `F1` 显示完整的快捷键列表
- 分类显示（导航、操作、视图等）

### 6.3 快捷键设计（修订版）

**导航：**
- `↑↓` 或 `jk`：上下移动
- `←→` 或 `hl`：左右切换焦点（圆环图 ↔ 列表）
- `Enter`：进入选中的目录
- `Backspace` 或 `h`：返回上级目录（移除 `u` 冲突）
- `gg`：跳到第一项
- `G`：跳到最后一项

**操作：**
- `/`：搜索/过滤
- `s`：排序切换（大小 → 名称 → 时间）
- `b`：添加/移除书签
- `e`：查看错误日志（任何时候都可按）
- `r`：刷新/重新扫描
- `x`：导出报告
- `c`：清除缓存（需二次确认）
- `y`：复制当前路径到剪贴板
- `o`：用系统默认应用打开

**视图：**
- `t`：切换阈值（0.5% / 1% / 2% / 5%）
- `?` 或 `F1`：显示/隐藏帮助面板
- `Esc`：关闭当前面板（帮助、错误列表等）
- `q`：退出应用（仅在主界面）

**改进说明：**
- ❌ 移除 `u` 作为返回键（与 Vim undo 冲突）
- ✅ 使用 `h` 或 `Backspace` 返回（更符合 Vim 习惯）
- ✅ 新增 `y` 复制路径、`o` 打开文件
- ✅ 明确 `Esc` 用于关闭子面板，`q` 用于退出应用

---

## 7. 缓存和增量扫描

### 7.1 缓存策略

**缓存位置：** `~/.disklens/cache/`

**缓存文件命名：**
```
{path_hash}.cache        # 扫描结果（bincode 2.0 格式）
{path_hash}.meta.json    # 元数据（JSON 格式）
```

**元数据结构（增强版）：**
```rust
#[derive(Serialize, Deserialize)]
struct CacheMeta {
    original_path: PathBuf,
    scan_timestamp: SystemTime,
    total_size: u64,
    file_count: usize,
    dir_count: usize,

    // 增强的变更检测 [新增]
    root_mtime: SystemTime,     // 根目录修改时间
    root_inode: Option<u64>,    // 根目录 inode
    content_hash: u64,          // 目录内容哈希（文件名 + 大小）
}
```

### 7.2 增量扫描流程（增强版）

```rust
impl Cache {
    async fn load_or_scan(&self, path: PathBuf) -> Result<ScanResult> {
        // 1. 加载缓存元数据
        let meta = self.load_meta(&path)?;

        // 2. 增强的变更检测
        let current_metadata = tokio::fs::metadata(&path).await?;
        let is_changed = self.detect_changes(&meta, &current_metadata).await?;

        if !is_changed {
            // 缓存有效，直接加载
            return self.load_cache(&path);
        }

        // 3. 增量扫描
        let result = self.incremental_scan(path, meta).await?;

        // 4. 原子写入缓存 [新增]
        self.atomic_write_cache(&result).await?;

        Ok(result)
    }

    async fn detect_changes(
        &self,
        meta: &CacheMeta,
        current: &Metadata
    ) -> Result<bool> {
        // 多重检测机制

        // 检查 1: mtime
        if current.modified()? != meta.root_mtime {
            return Ok(true);
        }

        // 检查 2: inode（仅 Unix）
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if Some(current.ino()) != meta.root_inode {
                return Ok(true);
            }
        }

        // 检查 3: 文件数量快速检测
        let quick_count = self.quick_count_files(&path).await?;
        if quick_count != meta.file_count {
            return Ok(true);
        }

        // 检查 4: 内容哈希（可选，耗时）
        // let content_hash = self.compute_content_hash(&path).await?;
        // if content_hash != meta.content_hash {
        //     return Ok(true);
        // }

        Ok(false)
    }

    async fn atomic_write_cache(&self, result: &ScanResult) -> Result<()> {
        // 原子写入：temp file + rename
        let cache_path = self.cache_path(&result.root.path);
        let temp_path = cache_path.with_extension("tmp");

        // 1. 写入临时文件
        let encoded = bincode::serialize(result)?;
        tokio::fs::write(&temp_path, encoded).await?;

        // 2. 原子重命名
        tokio::fs::rename(&temp_path, &cache_path).await?;

        // 3. 写入元数据
        let meta = CacheMeta::from_result(result);
        let meta_json = serde_json::to_string_pretty(&meta)?;
        tokio::fs::write(
            cache_path.with_extension("meta.json"),
            meta_json
        ).await?;

        Ok(())
    }
}
```

**改进点：**
1. ✅ **多重变更检测**：mtime + inode + 文件数量
2. ✅ **原子写入**：temp file + rename，避免中断导致不一致
3. ✅ **可靠性增强**：即使扫描中断，也不会破坏旧缓存

### 7.3 缓存管理

- 用户可选择清除缓存（按 `c` 键，需二次确认）
- 自动清理超过 30 天的缓存
- 显示缓存占用空间（在设置或状态栏）
- 缓存大小限制：默认 500MB，可配置

---

## 8. 错误处理策略

### 8.1 分级错误处理

**Level 1 - 实时错误计数器：**
- 右上角显示：`⚠️  23 个错误`
- 实时更新，不会被覆盖
- 点击或按 `e` 查看详情

**Level 2 - 状态栏滚动提示：**
- 在底部状态栏滚动显示最新错误
- 格式：`⚠️ 跳过: /System/Library - 权限拒绝`
- 3 秒后自动消失或被新错误覆盖
- 不中断扫描流程

**Level 3 - 错误摘要（扫描完成后）:**
```
┌────────────────────────────┐
│   扫描完成！               │
│                            │
│   ✓ 成功: 12,345 个项目    │
│   ⚠️  跳过: 23 个项目       │
│                            │
│   详细错误类型：           │
│   • 权限拒绝: 18           │
│   • 符号链接循环: 3        │
│   • 其他错误: 2            │
│                            │
│   [e] 查看详情 [Enter] 继续│
└────────────────────────────┘
```

**Level 4 - 详细错误列表（按 'e' 键）:**
```
┌─────────────────────────────────────────┐
│         错误详情 (23 项)  [f] 过滤      │
├─────────────────────────────────────────┤
│ ⚠️  /System/Library/CoreServices        │
│     权限拒绝                            │
│                                         │
│ ⚠️  /private/var/db                     │
│     权限拒绝                            │
│                                         │
│ 🔁 /Users/link -> /Users/target         │
│     符号链接循环                        │
├─────────────────────────────────────────┤
│ ↑↓:滚动 f:过滤 y:复制路径 Esc:关闭     │
└─────────────────────────────────────────┘
```

**Level 5 - 导出报告中的错误日志:**
- JSON: `errors` 字段包含完整错误列表
- Markdown: "## 扫描错误" 章节
- HTML: 可折叠的错误列表

### 8.2 错误恢复

- 所有错误都不中断扫描
- 记录错误但继续处理其他路径
- 对于严重错误（如磁盘满），提示用户并优雅退出

---

## 9. 导出功能设计

### 9.1 导出流程（增强版）

**触发方式：** 按 `x` 键打开导出菜单

**导出菜单：**
```
┌─────────────────────────────────────────┐
│         导出报告                        │
├─────────────────────────────────────────┤
│  [1] JSON - 结构化数据                  │
│  [2] Markdown - 人类可读                │
│  [3] HTML - 交互式报告（纯 CSS）        │ ← 明确技术选型
│  [4] 全部导出                           │
│  [5] 仅导出当前目录                     │ ← 新增选项
│                                         │
│  保存位置:                              │
│  ~/disklens_report_{timestamp}          │
│  [Tab] 编辑路径                         │ ← 允许自定义
│                                         │
│  [Esc] 取消                             │
└─────────────────────────────────────────┘
```

**改进点：**
1. ✅ HTML 使用纯 CSS/SVG（无外部依赖）
2. ✅ 支持"仅导出当前目录"
3. ✅ 允许自定义保存路径
4. ✅ 显示导出进度条

### 9.2 各格式详细设计

（与 v1.0 相同，此处省略）

---

## 10. 性能优化策略

### 10.1 扫描性能优化

**1. 并发控制（自适应）：**
- 检测存储类型（SSD/HDD）
- SSD：256 并发 I/O
- HDD：32 并发 I/O
- Unknown：64 并发 I/O（保守值）

**2. 内存优化：**
- 使用 `Arc<Path>` 和 `SmallVec` 减少分配
- 流式处理：不一次性加载所有数据
- 及时释放已处理完的节点数据

**3. I/O 优化：**
- 使用 `tokio::fs` 异步 I/O
- 批量读取元数据
- 预读优化：提示操作系统顺序读取

**4. 缓存命中优化：**
- 多重变更检测（mtime + inode + 文件数量）
- 增量扫描：只重新扫描变化的子树

### 10.2 UI 渲染优化

**1. 增量渲染：**
- 只重绘变化的区域
- 使用 ratatui 的 diff 算法

**2. 节流和防抖：**
- 进度更新节流：最多每 100ms 更新一次
- 批量发送进度事件（减少 channel 开销）
- 键盘事件防抖：避免快速按键导致的卡顿

**3. 圆环图优化：**
- 预计算扇区坐标，缓存绘制路径
- 使用 Unicode 块字符（▀▄█）提升绘制效果

**4. 虚拟滚动：**
- 列表视图只渲染可见区域的行
- 对于超长列表（>1000 项），使用虚拟滚动

### 10.3 基准测试目标（修订版）

**SSD 环境：**
- 小规模（1GB / 1 万文件）：< 3 秒
- 中规模（100GB / 10 万文件）：< 20 秒
- 大规模（1TB / 100 万文件）：< 2 分钟

**HDD 环境：**
- 小规模（1GB / 1 万文件）：< 10 秒
- 中规模（100GB / 10 万文件）：< 2 分钟
- 大规模（1TB / 100 万文件）：< 15 分钟

**改进说明：**
- ✅ 区分 SSD 和 HDD 的不同目标
- ✅ HDD 目标更现实（< 15 分钟 vs 原来的 < 5 分钟）

---

## 11. 开发路线图

### Phase 1: 核心功能（MVP）

**目标：** 基本可用的磁盘分析工具

- [ ] 基础项目结构和依赖配置
- [ ] 数据模型实现（Node, ScanResult, 索引）
- [ ] 简单的异步扫描器（tokio + Semaphore）
- [ ] 基本的 TUI 界面（列表视图）
- [ ] 显示扫描进度
- [ ] JSON 格式导出

**预计时间：** 1-2 周

### Phase 2: 并发和性能

**目标：** 高性能异步扫描

- [ ] 完善异步扫描引擎（符号链接检测、错误处理）
- [ ] 存储类型检测和自适应并发
- [ ] 事件总线系统
- [ ] 性能基准测试和优化
- [ ] 进度追踪优化（详细的实时反馈）
- [ ] 错误计数器和状态栏

**预计时间：** 1-2 周

### Phase 3: 可视化增强

**目标：** 圆环图和交互体验

- [ ] 实现圆环图 Widget
- [ ] 钻取式导航（Enter/Backspace）
- [ ] 面包屑导航和焦点指示器
- [ ] 左右分栏布局
- [ ] 自适应阈值合并小文件
- [ ] 颜色主题和视觉优化

**预计时间：** 1 周

### Phase 4: 高级功能

**目标：** 增强实用性

- [ ] 智能缓存系统（增强的变更检测 + 原子写入）
- [ ] 增量扫描
- [ ] 搜索/过滤功能（基于索引）
- [ ] 排序功能（大小/名称/时间）
- [ ] 书签功能
- [ ] Markdown 和 HTML 导出
- [ ] 完整的键盘快捷键支持

**预计时间：** 1-2 周

### Phase 5: 完善和发布

**目标：** 生产就绪

- [ ] 完整的单元测试和集成测试
- [ ] 文档编写（README, 使用指南）
- [ ] 错误处理完善
- [ ] 跨平台测试（macOS, Linux, Windows）
- [ ] 性能调优
- [ ] 发布到 crates.io

**预计时间：** 1 周

---

## 12. 总结

DiskLens 采用**多层架构（Layered Architecture）**，核心特性包括：

1. **高性能异步扫描**：纯 tokio 异步模型 + Semaphore 限流，支持从 GB 到 TB 级别的磁盘
2. **智能并发控制**：自动检测存储类型（SSD/HDD），自适应调整并发数
3. **直观的可视化**：圆环图 + 列表视图，钻取式导航，焦点指示器
4. **智能缓存**：增强的变更检测（mtime + inode + 文件数量），原子写入
5. **完善的错误处理**：分级错误提示（计数器 + 滚动提示 + 详细列表），不中断扫描流程
6. **多格式导出**：JSON、Markdown、HTML（纯 CSS/SVG）
7. **丰富的交互**：优化的 Vim 风格快捷键（解决冲突），搜索、排序、书签等功能
8. **性能优化**：内存优化（Arc + SmallVec），虚拟滚动，增量渲染

**主要改进：**
- ✅ 架构术语修正（MVC → 多层架构）
- ✅ 并发模型重构（rayon → tokio + Semaphore）
- ✅ 数据模型优化（移除冗余字段，添加索引）
- ✅ 缓存增强（多重检测 + 原子写入）
- ✅ UI 增强（焦点指示器、错误计数器、排序指示器）
- ✅ 快捷键优化（解决 u 键冲突）
- ✅ 基准测试目标更现实（区分 SSD/HDD）

整个项目预计 6-8 周完成，分为 5 个开发阶段。
