use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Event {
    // Scan progress
    Progress { scanned: usize, total_size: u64, current_path: PathBuf },

    // Scan state
    ScanStarted { path: PathBuf },
    ScanCompleted { total_files: usize, total_size: u64, duration_ms: u64 },
    ScanError { path: PathBuf, error: String },

    // UI events
    Tick,
}

pub type EventSender = mpsc::UnboundedSender<Event>;
pub type EventReceiver = mpsc::UnboundedReceiver<Event>;

pub fn create_event_channel() -> (EventSender, EventReceiver) {
    mpsc::unbounded_channel()
}
