use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashSet;
use tokio::sync::Semaphore;

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
    settings: Arc<Settings>,
    errors: Arc<std::sync::Mutex<Vec<ScanError>>>,
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
            settings: Arc::new(settings),
            errors: Arc::new(std::sync::Mutex::new(Vec::new())),
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
            Arc::clone(&self.settings),
            Arc::clone(&self.errors),
            Arc::clone(&self.last_progress_time),
        )
        .await?;

        let elapsed = self.progress.elapsed();
        let errors = self.errors.lock().unwrap().clone();

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

/// Collected directory entry from batch I/O.
struct DirEntryData {
    path: PathBuf,
    name: String,
    metadata: std::fs::Metadata,
}

/// Read all entries and their metadata from a directory in one blocking call.
/// Returns (entries, entry_errors) or an error if the directory itself can't be read.
fn read_dir_batch(
    dir_path: &std::path::Path,
) -> std::io::Result<(Vec<DirEntryData>, Vec<(PathBuf, String)>)> {
    let mut entries = Vec::new();
    let mut errors = Vec::new();

    for entry_result in std::fs::read_dir(dir_path)? {
        match entry_result {
            Ok(entry) => {
                let entry_path = entry.path();
                let entry_name = entry.file_name().to_string_lossy().to_string();
                match std::fs::symlink_metadata(&entry_path) {
                    Ok(meta) => entries.push(DirEntryData {
                        path: entry_path,
                        name: entry_name,
                        metadata: meta,
                    }),
                    Err(e) => errors.push((entry_path, e.to_string())),
                }
            }
            Err(e) => {
                errors.push((dir_path.to_path_buf(), e.to_string()));
            }
        }
    }

    Ok((entries, errors))
}

fn scan_directory(
    path: PathBuf,
    depth: usize,
    semaphore: Arc<Semaphore>,
    event_tx: EventSender,
    visited: Arc<DashSet<PathBuf>>,
    progress: Arc<ProgressTracker>,
    settings: Arc<Settings>,
    errors: Arc<std::sync::Mutex<Vec<ScanError>>>,
    last_progress_time: Arc<AtomicU64>,
) -> Pin<Box<dyn Future<Output = anyhow::Result<Node>> + Send>> {
    Box::pin(async move {
        progress.increment_dirs();

        if let Some(max_depth) = settings.max_depth {
            if depth >= max_depth {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                return Ok(Node::from_directory(path, name, Vec::new()));
            }
        }

        // Batch I/O: read directory and all entry metadata in a single spawn_blocking.
        // Semaphore permit is held only during I/O, then released before processing.
        let io_result = {
            let _permit = semaphore.acquire().await?;
            let path_clone = path.clone();
            tokio::task::spawn_blocking(move || read_dir_batch(&path_clone)).await?
            // _permit drops here — released before processing entries or waiting for children
        };

        let (entries, entry_errors) = match io_result {
            Ok(result) => result,
            Err(e) => {
                let error_type = match e.kind() {
                    std::io::ErrorKind::PermissionDenied => ScanErrorType::PermissionDenied,
                    std::io::ErrorKind::NotFound => ScanErrorType::NotFound,
                    _ => ScanErrorType::IoError,
                };
                errors.lock().unwrap().push(ScanError {
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

        // Record entry-level I/O errors
        for (err_path, err_msg) in entry_errors {
            errors.lock().unwrap().push(ScanError {
                path: err_path.clone(),
                error_type: ScanErrorType::IoError,
                message: err_msg.clone(),
            });
            progress.increment_errors();
            let _ = event_tx.send(Event::ScanError {
                path: err_path,
                error: err_msg,
            });
        }

        let mut handles = Vec::new();
        let mut file_nodes = Vec::new();

        for entry_data in entries {
            let entry_path = entry_data.path;
            let entry_name = entry_data.name;
            let metadata = entry_data.metadata;
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
                            errors.lock().unwrap().push(ScanError {
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
                                    let handle = tokio::spawn(scan_directory(
                                        real_path,
                                        depth + 1,
                                        Arc::clone(&semaphore),
                                        event_tx.clone(),
                                        Arc::clone(&visited),
                                        Arc::clone(&progress),
                                        Arc::clone(&settings),
                                        Arc::clone(&errors),
                                        Arc::clone(&last_progress_time),
                                    ));
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
                                    file_nodes.push(node);
                                }
                            }
                            Err(e) => {
                                errors.lock().unwrap().push(ScanError {
                                    path: entry_path,
                                    error_type: ScanErrorType::IoError,
                                    message: e.to_string(),
                                });
                                progress.increment_errors();
                            }
                        }
                    }
                    Err(e) => {
                        errors.lock().unwrap().push(ScanError {
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

                let handle = tokio::spawn(scan_directory(
                    entry_path,
                    depth + 1,
                    Arc::clone(&semaphore),
                    event_tx.clone(),
                    Arc::clone(&visited),
                    Arc::clone(&progress),
                    Arc::clone(&settings),
                    Arc::clone(&errors),
                    Arc::clone(&last_progress_time),
                ));
                handles.push(handle);
            } else if file_type.is_file() {
                let size = metadata.len();
                let modified = metadata.modified().ok();
                #[cfg(unix)]
                let inode = Some(std::os::unix::fs::MetadataExt::ino(&metadata));
                #[cfg(not(unix))]
                let inode = None;

                let node = Node::from_file(entry_path, entry_name, size, modified, inode);
                progress.increment_files();
                progress.add_size(size);
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

        // Wait for all spawned directory scans (permit already released)
        for handle in handles {
            match handle.await {
                Ok(Ok(node)) => file_nodes.push(node),
                Ok(Err(e)) => {
                    errors.lock().unwrap().push(ScanError {
                        path: path.clone(),
                        error_type: ScanErrorType::IoError,
                        message: e.to_string(),
                    });
                    progress.increment_errors();
                }
                Err(e) => {
                    errors.lock().unwrap().push(ScanError {
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
