use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct ProgressTracker {
    pub files_scanned: AtomicUsize,
    pub dirs_scanned: AtomicUsize,
    pub total_size: AtomicU64,
    pub errors_count: AtomicUsize,
    pub current_path: Arc<RwLock<PathBuf>>,
    pub start_time: Instant,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            files_scanned: AtomicUsize::new(0),
            dirs_scanned: AtomicUsize::new(0),
            total_size: AtomicU64::new(0),
            errors_count: AtomicUsize::new(0),
            current_path: Arc::new(RwLock::new(PathBuf::new())),
            start_time: Instant::now(),
        }
    }

    pub fn increment_files(&self) {
        self.files_scanned.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_dirs(&self) {
        self.dirs_scanned.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_size(&self, size: u64) {
        self.total_size.fetch_add(size, Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.errors_count.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn set_current_path(&self, path: PathBuf) {
        let mut current = self.current_path.write().await;
        *current = path;
    }

    pub fn files_per_second(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed < f64::EPSILON {
            return 0.0;
        }
        self.files_scanned.load(Ordering::Relaxed) as f64 / elapsed
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            files_scanned: self.files_scanned.load(Ordering::Relaxed),
            dirs_scanned: self.dirs_scanned.load(Ordering::Relaxed),
            total_size: self.total_size.load(Ordering::Relaxed),
            errors_count: self.errors_count.load(Ordering::Relaxed),
            elapsed: self.elapsed(),
            files_per_second: self.files_per_second(),
        }
    }
}

pub struct ProgressSnapshot {
    pub files_scanned: usize,
    pub dirs_scanned: usize,
    pub total_size: u64,
    pub errors_count: usize,
    pub elapsed: Duration,
    pub files_per_second: f64,
}
