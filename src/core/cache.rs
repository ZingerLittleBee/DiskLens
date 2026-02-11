use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::models::scan_result::ScanResult;

#[derive(Serialize, Deserialize)]
struct CacheMeta {
    original_path: PathBuf,
    scan_timestamp: SystemTime,
    total_size: u64,
    file_count: usize,
    dir_count: usize,
    root_mtime: Option<SystemTime>,
    #[cfg(unix)]
    root_inode: Option<u64>,
}

pub struct Cache {
    cache_dir: PathBuf,
}

impl Cache {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    fn hash_path(path: &PathBuf) -> u64 {
        let mut hasher = DefaultHasher::new();
        path.to_string_lossy().hash(&mut hasher);
        hasher.finish()
    }

    fn cache_path(&self, path: &PathBuf) -> PathBuf {
        let hash = Self::hash_path(path);
        self.cache_dir.join(format!("{:x}.cache", hash))
    }

    fn meta_path(&self, path: &PathBuf) -> PathBuf {
        let hash = Self::hash_path(path);
        self.cache_dir.join(format!("{:x}.meta.json", hash))
    }

    pub async fn load(&self, path: &PathBuf) -> Option<ScanResult> {
        let cache_file = self.cache_path(path);
        let meta_file = self.meta_path(path);

        // Check both files exist
        if !cache_file.exists() || !meta_file.exists() {
            return None;
        }

        // Load and validate metadata
        let meta_bytes = tokio::fs::read(&meta_file).await.ok()?;
        let meta: CacheMeta = serde_json::from_slice(&meta_bytes).ok()?;

        // Verify the cached path matches
        if meta.original_path != *path {
            return None;
        }

        // Check for changes via mtime
        if let Ok(fs_meta) = tokio::fs::metadata(path).await {
            if let Ok(current_mtime) = fs_meta.modified() {
                if let Some(cached_mtime) = meta.root_mtime {
                    if current_mtime != cached_mtime {
                        return None;
                    }
                }
            }

            // Check inode on unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if let Some(cached_inode) = meta.root_inode {
                    if fs_meta.ino() != cached_inode {
                        return None;
                    }
                }
            }
        }

        // Load and deserialize the scan result
        let cache_bytes = tokio::fs::read(&cache_file).await.ok()?;
        bincode::serde::decode_from_slice(&cache_bytes, bincode::config::standard())
            .map(|(result, _)| result)
            .ok()
    }

    pub async fn save(&self, result: &ScanResult) -> anyhow::Result<()> {
        // Ensure cache directory exists
        tokio::fs::create_dir_all(&self.cache_dir).await?;

        let path = &result.scan_path;

        // Build metadata
        let root_mtime = result.root.modified;
        #[cfg(unix)]
        let root_inode = result.root.inode;

        let meta = CacheMeta {
            original_path: path.clone(),
            scan_timestamp: result.timestamp,
            total_size: result.total_size,
            file_count: result.total_files,
            dir_count: result.total_dirs,
            root_mtime,
            #[cfg(unix)]
            root_inode,
        };

        // Serialize scan result with bincode
        let cache_bytes = bincode::serde::encode_to_vec(result, bincode::config::standard())?;
        let meta_bytes = serde_json::to_vec_pretty(&meta)?;

        // Atomic write: write to temp file, then rename
        let cache_file = self.cache_path(path);
        let meta_file = self.meta_path(path);

        let tmp_cache = cache_file.with_extension("cache.tmp");
        let tmp_meta = meta_file.with_extension("meta.json.tmp");

        tokio::fs::write(&tmp_cache, &cache_bytes).await?;
        tokio::fs::rename(&tmp_cache, &cache_file).await?;

        tokio::fs::write(&tmp_meta, &meta_bytes).await?;
        tokio::fs::rename(&tmp_meta, &meta_file).await?;

        Ok(())
    }

    pub async fn clear(&self) -> anyhow::Result<()> {
        if !self.cache_dir.exists() {
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.ends_with(".cache") || name.ends_with(".meta.json") || name.ends_with(".tmp") {
                    tokio::fs::remove_file(&path).await?;
                }
            }
        }
        Ok(())
    }
}
