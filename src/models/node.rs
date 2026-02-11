use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub size_on_disk: u64,
    pub node_type: NodeType,
    pub children: Vec<Node>,
    pub file_count: usize,
    pub dir_count: usize,
    pub modified: Option<SystemTime>,
    #[cfg(unix)]
    pub inode: Option<u64>,
}

impl Node {
    pub fn percentage(&self, total_size: u64) -> f64 {
        if total_size == 0 {
            return 0.0;
        }
        (self.size as f64 / total_size as f64) * 100.0
    }

    pub fn from_file(
        path: PathBuf,
        name: String,
        size: u64,
        modified: Option<SystemTime>,
        #[allow(unused_variables)] inode: Option<u64>,
    ) -> Self {
        Self {
            path,
            name,
            size,
            size_on_disk: size,
            node_type: NodeType::File,
            children: Vec::new(),
            file_count: 1,
            dir_count: 0,
            modified,
            #[cfg(unix)]
            inode,
        }
    }

    pub fn from_directory(path: PathBuf, name: String, children: Vec<Node>) -> Self {
        let size = children.iter().map(|c| c.size).sum();
        let size_on_disk = children.iter().map(|c| c.size_on_disk).sum();
        let file_count = children.iter().map(|c| c.file_count).sum();
        let dir_count: usize = children.iter().map(|c| c.dir_count).sum::<usize>() + 1;

        Self {
            path,
            name,
            size,
            size_on_disk,
            node_type: NodeType::Directory,
            children,
            file_count,
            dir_count,
            modified: None,
            #[cfg(unix)]
            inode: None,
        }
    }

    pub fn total_size(&self) -> u64 {
        self.size
    }

    pub fn human_readable_size(&self) -> String {
        human_readable_size(self.size)
    }
}

pub fn human_readable_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
