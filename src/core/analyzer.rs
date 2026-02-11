use crate::models::node::{Node, NodeType};

pub struct Analyzer;

impl Analyzer {
    pub fn sort_by_size(node: &mut Node) {
        node.children.sort_by(|a, b| b.size.cmp(&a.size));
        for child in &mut node.children {
            if child.node_type == NodeType::Directory {
                Self::sort_by_size(child);
            }
        }
    }

    pub fn merge_small_items(node: &Node, threshold: f64) -> Vec<MergedItem> {
        let total_size = node.size;
        if total_size == 0 {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut merged_size: u64 = 0;
        let mut merged_count: usize = 0;

        for child in &node.children {
            let percentage = child.size as f64 / total_size as f64;
            if percentage >= threshold {
                result.push(MergedItem {
                    name: child.name.clone(),
                    size: child.size,
                    percentage: percentage * 100.0,
                    is_merged: false,
                    merged_count: 0,
                    node_type: child.node_type,
                });
            } else {
                merged_size += child.size;
                merged_count += 1;
            }
        }

        if merged_count > 0 {
            let percentage = merged_size as f64 / total_size as f64;
            result.push(MergedItem {
                name: String::from("Others"),
                size: merged_size,
                percentage: percentage * 100.0,
                is_merged: true,
                merged_count,
                node_type: NodeType::File,
            });
        }

        result
    }

    pub fn compute_stats(node: &Node) -> (usize, usize) {
        (node.file_count, node.dir_count)
    }
}

pub struct MergedItem {
    pub name: String,
    pub size: u64,
    pub percentage: f64,
    pub is_merged: bool,
    pub merged_count: usize,
    pub node_type: NodeType,
}
