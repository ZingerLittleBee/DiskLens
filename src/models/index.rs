use std::collections::HashMap;
use std::path::PathBuf;

use super::node::Node;

pub struct PathIndex {
    map: HashMap<PathBuf, usize>,
}

impl PathIndex {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn build(root: &Node) -> Self {
        let mut index = Self::new();
        let mut counter = 0;
        Self::build_recursive(root, &mut index.map, &mut counter);
        index
    }

    fn build_recursive(
        node: &Node,
        map: &mut HashMap<PathBuf, usize>,
        counter: &mut usize,
    ) {
        map.insert(node.path.clone(), *counter);
        *counter += 1;
        for child in &node.children {
            Self::build_recursive(child, map, counter);
        }
    }

    pub fn search(&self, pattern: &str) -> Vec<PathBuf> {
        let pattern_lower = pattern.to_lowercase();
        let mut results: Vec<PathBuf> = self
            .map
            .keys()
            .filter(|path| {
                path.to_string_lossy()
                    .to_lowercase()
                    .contains(&pattern_lower)
            })
            .cloned()
            .collect();
        results.sort();
        results
    }
}

pub struct SizeIndex {
    sorted: Vec<(PathBuf, u64)>,
}

impl SizeIndex {
    pub fn new() -> Self {
        Self { sorted: Vec::new() }
    }

    pub fn build(root: &Node) -> Self {
        let mut index = Self::new();
        Self::collect_recursive(root, &mut index.sorted);
        index.sorted.sort_by(|a, b| b.1.cmp(&a.1));
        index
    }

    fn collect_recursive(node: &Node, entries: &mut Vec<(PathBuf, u64)>) {
        entries.push((node.path.clone(), node.size));
        for child in &node.children {
            Self::collect_recursive(child, entries);
        }
    }

    pub fn top_n(&self, n: usize) -> &[(PathBuf, u64)] {
        let end = n.min(self.sorted.len());
        &self.sorted[..end]
    }
}
