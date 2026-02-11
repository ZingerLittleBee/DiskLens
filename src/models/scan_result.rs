use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use super::node::Node;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub root: Node,
    pub total_size: u64,
    pub total_files: usize,
    pub total_dirs: usize,
    pub scan_duration: Duration,
    pub errors: Vec<ScanError>,
    pub timestamp: SystemTime,
    pub scan_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanError {
    pub path: PathBuf,
    pub error_type: ScanErrorType,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanErrorType {
    PermissionDenied,
    NotFound,
    SymlinkCycle,
    IoError,
    Other,
}
