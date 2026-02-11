use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub max_depth: Option<usize>,
    pub max_concurrent_io: usize,
    pub follow_symlinks: bool,
    pub merge_threshold: f64,
    pub ignore_patterns: Vec<String>,
    pub cache_dir: PathBuf,
    pub cache_max_size_mb: u64,
    pub cache_max_age_days: u64,
}

impl Default for Settings {
    fn default() -> Self {
        let cache_dir = dirs_cache_dir().unwrap_or_else(|| PathBuf::from(".disklens"));

        let max_concurrent_io = match detect_storage_type() {
            StorageType::SSD => 128,
            StorageType::HDD => 32,
            StorageType::Unknown => 64,
        };

        // Cap concurrency to avoid "too many open files" (EMFILE)
        let max_concurrent_io = cap_by_fd_limit(max_concurrent_io);

        Self {
            max_depth: None,
            max_concurrent_io,
            follow_symlinks: false,
            merge_threshold: 0.01,
            ignore_patterns: vec![],
            cache_dir,
            cache_max_size_mb: 512,
            cache_max_age_days: 7,
        }
    }
}

fn dirs_cache_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Caches/disklens"))
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
            .map(|p| p.join("disklens"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Some(PathBuf::from(".disklens"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageType {
    SSD,
    HDD,
    Unknown,
}

pub fn detect_storage_type() -> StorageType {
    #[cfg(target_os = "macos")]
    {
        detect_storage_type_macos()
    }
    #[cfg(target_os = "linux")]
    {
        detect_storage_type_linux()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        StorageType::Unknown
    }
}

#[cfg(target_os = "macos")]
fn detect_storage_type_macos() -> StorageType {
    use std::process::Command;

    let output = Command::new("system_profiler")
        .arg("SPStorageDataType")
        .output();

    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if text.contains("solid state") || text.contains("ssd") || text.contains("nvme") {
                StorageType::SSD
            } else if text.contains("rotational") || text.contains("hdd") {
                StorageType::HDD
            } else {
                StorageType::Unknown
            }
        }
        Err(_) => StorageType::Unknown,
    }
}

#[cfg(target_os = "linux")]
fn detect_storage_type_linux() -> StorageType {
    use std::fs;

    let entries = match fs::read_dir("/sys/block") {
        Ok(e) => e,
        Err(_) => return StorageType::Unknown,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("sd") && !name_str.starts_with("nvme") {
            continue;
        }

        let rotational_path = format!("/sys/block/{}/queue/rotational", name_str);
        if let Ok(val) = fs::read_to_string(&rotational_path) {
            return match val.trim() {
                "0" => StorageType::SSD,
                "1" => StorageType::HDD,
                _ => StorageType::Unknown,
            };
        }
    }

    StorageType::Unknown
}

/// Cap concurrency based on the system's file descriptor soft limit.
/// Reserves 25% of fds for non-scan use (stdin/stdout, terminal, channels, etc.).
fn cap_by_fd_limit(max_io: usize) -> usize {
    #[cfg(unix)]
    {
        let mut rlim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        let ret = unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) };
        if ret == 0 && rlim.rlim_cur != libc::RLIM_INFINITY {
            let fd_limit = rlim.rlim_cur as usize;
            let usable = fd_limit * 3 / 4; // reserve 25%
            return max_io.min(usable).max(16); // at least 16
        }
    }
    max_io
}
