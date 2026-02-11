use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashSet;
use tokio::sync::{Mutex, Semaphore};

use crate::config::settings::Settings;
use crate::models::node::{Node, NodeType};
use crate::models::scan_result::{ScanError, ScanErrorType, ScanResult};

use super::events::{Event, EventSender};
use super::progress::ProgressTracker;

pub struct Scanner {
    semaphore: Arc<Semaphore>,
    event_tx: EventSender,
    visited: Arc<DashSet<PathBuf>>,
    progress: Arc<ProgressTracker>,
    settings: Settings,
    errors: Arc<Mutex<Vec<ScanError>>>,
    last_progress_time: Arc<AtomicU64>,
}

impl Scanner {
    pub fn new(settings: Settings, event_tx: EventSender) -> Self {
        let max_io = settings.max_concurrent_io;
        Self {
            semaphore: Arc::new(Semaphore::new(max_io)),
            event_tx,
            visited: Arc::new(DashSet::new()),
            progress: Arc::new(ProgressTracker::new()),
            settings,
            errors: Arc::new(Mutex::new(Vec::new())),
            last_progress_time: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn progress(&self) -> &Arc<ProgressTracker> {
        &self.progress
    }

    pub async fn scan(&self, root: PathBuf) -> anyhow::Result<ScanResult> {
        let _ = self.event_tx.send(Event::ScanStarted { path: root.clone() });

        let root_node = scan_directory(
            root.clone(),
            0,
            Arc::clone(&self.semaphore),
            self.event_tx.clone(),
            Arc::clone(&self.visited),
            Arc::clone(&self.progress),
            self.settings.clone(),
            Arc::clone(&self.errors),
            Arc::clone(&self.last_progress_time),
        )
        .await?;

        let elapsed = self.progress.elapsed();
        let errors = self.errors.lock().await.clone();

        let result = ScanResult {
            total_size: root_node.size,
            total_files: root_node.file_count,
            total_dirs: root_node.dir_count,
            scan_duration: elapsed,
            errors,
            timestamp: SystemTime::now(),
            scan_path: root,
            root: root_node,
        };

        let _ = self.event_tx.send(Event::ScanCompleted {
            total_files: result.total_files,
            total_size: result.total_size,
            duration_ms: result.scan_duration.as_millis() as u64,
        });

        Ok(result)
    }
}

fn scan_directory(
    path: PathBuf,
    depth: usize,
    semaphore: Arc<Semaphore>,
    event_tx: EventSender,
    visited: Arc<DashSet<PathBuf>>,
    progress: Arc<ProgressTracker>,
    settings: Settings,
    errors: Arc<Mutex<Vec<ScanError>>>,
    last_progress_time: Arc<AtomicU64>,
) -> Pin<Box<dyn Future<Output = anyhow::Result<Node>> + Send>> {
    Box::pin(async move {
        let _permit = semaphore.acquire().await?;

        progress.increment_dirs();
        progress.set_current_path(path.clone()).await;
        let _ = event_tx.send(Event::DirEntered { path: path.clone() });

        if let Some(max_depth) = settings.max_depth {
            if depth >= max_depth {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                return Ok(Node::from_directory(path, name, Vec::new()));
            }
        }

        let mut read_dir = match tokio::fs::read_dir(&path).await {
            Ok(rd) => rd,
            Err(e) => {
                let error_type = match e.kind() {
                    std::io::ErrorKind::PermissionDenied => ScanErrorType::PermissionDenied,
                    std::io::ErrorKind::NotFound => ScanErrorType::NotFound,
                    _ => ScanErrorType::IoError,
                };
                errors.lock().await.push(ScanError {
                    path: path.clone(),
                    error_type,
                    message: e.to_string(),
                });
                progress.increment_errors();
                let _ = event_tx.send(Event::ScanError {
                    path: path.clone(),
                    error: e.to_string(),
                });
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                return Ok(Node::from_directory(path, name, Vec::new()));
            }
        };

        let mut handles = Vec::new();
        let mut file_nodes = Vec::new();

        loop {
            let entry = match read_dir.next_entry().await {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(e) => {
                    errors.lock().await.push(ScanError {
                        path: path.clone(),
                        error_type: ScanErrorType::IoError,
                        message: e.to_string(),
                    });
                    progress.increment_errors();
                    continue;
                }
            };

            let entry_path = entry.path();
            let entry_name = entry.file_name().to_string_lossy().to_string();

            let metadata = match tokio::fs::symlink_metadata(&entry_path).await {
                Ok(m) => m,
                Err(e) => {
                    errors.lock().await.push(ScanError {
                        path: entry_path.clone(),
                        error_type: ScanErrorType::IoError,
                        message: e.to_string(),
                    });
                    progress.increment_errors();
                    let _ = event_tx.send(Event::ScanError {
                        path: entry_path,
                        error: e.to_string(),
                    });
                    continue;
                }
            };

            let file_type = metadata.file_type();

            if file_type.is_symlink() {
                if !settings.follow_symlinks {
                    let size = metadata.len();
                    let modified = metadata.modified().ok();
                    #[cfg(unix)]
                    let inode = Some(std::os::unix::fs::MetadataExt::ino(&metadata));
                    #[cfg(not(unix))]
                    let inode = None;
                    let node = Node {
                        path: entry_path,
                        name: entry_name,
                        size,
                        size_on_disk: size,
                        node_type: NodeType::Symlink,
                        children: Vec::new(),
                        file_count: 0,
                        dir_count: 0,
                        modified,
                        #[cfg(unix)]
                        inode,
                    };
                    file_nodes.push(node);
                    continue;
                }
                // Follow symlink - resolve and check for cycles
                match tokio::fs::canonicalize(&entry_path).await {
                    Ok(real_path) => {
                        if !visited.insert(real_path.clone()) {
                            errors.lock().await.push(ScanError {
                                path: entry_path.clone(),
                                error_type: ScanErrorType::SymlinkCycle,
                                message: format!("Symlink cycle detected: {:?}", entry_path),
                            });
                            progress.increment_errors();
                            continue;
                        }
                        match tokio::fs::metadata(&real_path).await {
                            Ok(resolved_meta) => {
                                if resolved_meta.is_dir() {
                                    let sem = Arc::clone(&semaphore);
                                    let tx = event_tx.clone();
                                    let vis = Arc::clone(&visited);
                                    let prog = Arc::clone(&progress);
                                    let sett = settings.clone();
                                    let errs = Arc::clone(&errors);
                                    let next_depth = depth + 1;
                                    let lpt = Arc::clone(&last_progress_time);
                                    let handle = tokio::spawn(
                                        scan_directory(real_path, next_depth, sem, tx, vis, prog, sett, errs, lpt),
                                    );
                                    handles.push(handle);
                                } else {
                                    let size = resolved_meta.len();
                                    let modified = resolved_meta.modified().ok();
                                    #[cfg(unix)]
                                    let inode =
                                        Some(std::os::unix::fs::MetadataExt::ino(&resolved_meta));
                                    #[cfg(not(unix))]
                                    let inode = None;
                                    let node =
                                        Node::from_file(entry_path, entry_name, size, modified, inode);
                                    progress.increment_files();
                                    progress.add_size(size);
                                    let _ = event_tx.send(Event::FileScanned {
                                        path: node.path.clone(),
                                        size,
                                    });
                                    file_nodes.push(node);
                                }
                            }
                            Err(e) => {
                                errors.lock().await.push(ScanError {
                                    path: entry_path,
                                    error_type: ScanErrorType::IoError,
                                    message: e.to_string(),
                                });
                                progress.increment_errors();
                            }
                        }
                    }
                    Err(e) => {
                        errors.lock().await.push(ScanError {
                            path: entry_path,
                            error_type: ScanErrorType::IoError,
                            message: e.to_string(),
                        });
                        progress.increment_errors();
                    }
                }
                continue;
            }

            if file_type.is_dir() {
                if !visited.insert(entry_path.clone()) {
                    continue;
                }

                let sem = Arc::clone(&semaphore);
                let tx = event_tx.clone();
                let vis = Arc::clone(&visited);
                let prog = Arc::clone(&progress);
                let sett = settings.clone();
                let errs = Arc::clone(&errors);
                let next_depth = depth + 1;
                let lpt = Arc::clone(&last_progress_time);
                let handle = tokio::spawn(
                    scan_directory(entry_path, next_depth, sem, tx, vis, prog, sett, errs, lpt),
                );
                handles.push(handle);
            } else if file_type.is_file() {
                let size = metadata.len();
                let modified = metadata.modified().ok();
                #[cfg(unix)]
                let inode = Some(std::os::unix::fs::MetadataExt::ino(&metadata));
                #[cfg(not(unix))]
                let inode = None;

                let node = Node::from_file(entry_path.clone(), entry_name, size, modified, inode);
                progress.increment_files();
                progress.add_size(size);
                let _ = event_tx.send(Event::FileScanned {
                    path: entry_path,
                    size,
                });
                file_nodes.push(node);
            } else {
                let node = Node {
                    path: entry_path,
                    name: entry_name,
                    size: 0,
                    size_on_disk: 0,
                    node_type: NodeType::Other,
                    children: Vec::new(),
                    file_count: 0,
                    dir_count: 0,
                    modified: metadata.modified().ok(),
                    #[cfg(unix)]
                    inode: Some(std::os::unix::fs::MetadataExt::ino(&metadata)),
                };
                file_nodes.push(node);
            }
        }

        // Wait for all spawned directory scans
        for handle in handles {
            match handle.await {
                Ok(Ok(node)) => file_nodes.push(node),
                Ok(Err(e)) => {
                    errors.lock().await.push(ScanError {
                        path: path.clone(),
                        error_type: ScanErrorType::IoError,
                        message: e.to_string(),
                    });
                    progress.increment_errors();
                }
                Err(e) => {
                    errors.lock().await.push(ScanError {
                        path: path.clone(),
                        error_type: ScanErrorType::Other,
                        message: format!("Task join error: {}", e),
                    });
                    progress.increment_errors();
                }
            }
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let dir_node = Node::from_directory(path.clone(), name, file_nodes);

        // Throttle progress events: only send if 100ms+ since last send
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let last = last_progress_time.load(Ordering::Relaxed);
        if now_ms.saturating_sub(last) >= 100 {
            last_progress_time.store(now_ms, Ordering::Relaxed);
            let snapshot = progress.snapshot();
            let _ = event_tx.send(Event::Progress {
                scanned: snapshot.files_scanned,
                total_size: snapshot.total_size,
                current_path: path,
            });
        }

        Ok(dir_node)
    })
}
