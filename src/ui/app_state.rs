use std::path::PathBuf;

use crate::models::node::Node;
use crate::models::scan_result::ScanResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Scanning,
    Normal,
    Help,
    ErrorList,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPanel {
    RingChart,
    FileList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Size,
    Name,
    Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

pub struct AppState {
    pub view_mode: ViewMode,
    pub focus: FocusPanel,
    pub current_path: PathBuf,
    pub path_stack: Vec<PathBuf>,
    pub selected_index: usize,
    pub list_offset: usize,
    pub sort_mode: SortMode,
    pub sort_order: SortOrder,
    pub merge_threshold: f64,
    pub scan_result: Option<ScanResult>,
    pub should_quit: bool,
    pub files_scanned: usize,
    pub total_size_scanned: u64,
    pub scan_speed: f64,
    pub current_scanning_path: String,
    pub error_count: usize,
    pub pending_g: bool,
}

impl AppState {
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            view_mode: ViewMode::Scanning,
            focus: FocusPanel::FileList,
            current_path: root_path,
            path_stack: Vec::new(),
            selected_index: 0,
            list_offset: 0,
            sort_mode: SortMode::Size,
            sort_order: SortOrder::Descending,
            merge_threshold: 0.01,
            scan_result: None,
            should_quit: false,
            files_scanned: 0,
            total_size_scanned: 0,
            scan_speed: 0.0,
            current_scanning_path: String::new(),
            error_count: 0,
            pending_g: false,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.list_offset {
                self.list_offset = self.selected_index;
            }
        }
    }

    pub fn move_down(&mut self) {
        let count = self.visible_children_count();
        if count > 0 && self.selected_index < count - 1 {
            self.selected_index += 1;
        }
    }

    pub fn enter_directory(&mut self) {
        let children = self.sorted_children();
        if let Some(child) = children.get(self.selected_index) {
            if child.node_type == crate::models::node::NodeType::Directory {
                let child_path = child.path.clone();
                self.path_stack.push(self.current_path.clone());
                self.current_path = child_path;
                self.selected_index = 0;
                self.list_offset = 0;
            }
        }
    }

    pub fn go_back(&mut self) {
        if let Some(parent) = self.path_stack.pop() {
            self.current_path = parent;
            self.selected_index = 0;
            self.list_offset = 0;
        }
    }

    pub fn go_to_first(&mut self) {
        self.selected_index = 0;
        self.list_offset = 0;
    }

    pub fn go_to_last(&mut self) {
        let count = self.visible_children_count();
        if count > 0 {
            self.selected_index = count - 1;
        }
    }

    pub fn current_node(&self) -> Option<&Node> {
        let result = self.scan_result.as_ref()?;
        find_node(&result.root, &self.current_path)
    }

    pub fn current_children(&self) -> Vec<&Node> {
        match self.current_node() {
            Some(node) => node.children.iter().collect(),
            None => Vec::new(),
        }
    }

    pub fn sorted_children(&self) -> Vec<&Node> {
        let mut children = self.current_children();
        match self.sort_mode {
            SortMode::Size => {
                children.sort_by(|a, b| {
                    if self.sort_order == SortOrder::Descending {
                        b.size.cmp(&a.size)
                    } else {
                        a.size.cmp(&b.size)
                    }
                });
            }
            SortMode::Name => {
                children.sort_by(|a, b| {
                    if self.sort_order == SortOrder::Ascending {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    } else {
                        b.name.to_lowercase().cmp(&a.name.to_lowercase())
                    }
                });
            }
            SortMode::Modified => {
                children.sort_by(|a, b| {
                    let a_time = a.modified.unwrap_or(std::time::UNIX_EPOCH);
                    let b_time = b.modified.unwrap_or(std::time::UNIX_EPOCH);
                    if self.sort_order == SortOrder::Descending {
                        b_time.cmp(&a_time)
                    } else {
                        a_time.cmp(&b_time)
                    }
                });
            }
        }
        children
    }

    pub fn visible_children_count(&self) -> usize {
        self.sorted_children().len()
    }

    pub fn toggle_sort(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::Size => SortMode::Name,
            SortMode::Name => SortMode::Modified,
            SortMode::Modified => SortMode::Size,
        };
        self.sort_order = match self.sort_mode {
            SortMode::Size => SortOrder::Descending,
            SortMode::Name => SortOrder::Ascending,
            SortMode::Modified => SortOrder::Descending,
        };
        self.selected_index = 0;
        self.list_offset = 0;
    }

    pub fn toggle_help(&mut self) {
        self.view_mode = if self.view_mode == ViewMode::Help {
            ViewMode::Normal
        } else {
            ViewMode::Help
        };
    }

    pub fn toggle_error_list(&mut self) {
        self.view_mode = if self.view_mode == ViewMode::ErrorList {
            ViewMode::Normal
        } else {
            ViewMode::ErrorList
        };
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            FocusPanel::RingChart => FocusPanel::FileList,
            FocusPanel::FileList => FocusPanel::RingChart,
        };
    }

    pub fn cycle_threshold(&mut self) {
        self.merge_threshold = match () {
            _ if (self.merge_threshold - 0.005).abs() < 0.001 => 0.01,
            _ if (self.merge_threshold - 0.01).abs() < 0.001 => 0.02,
            _ if (self.merge_threshold - 0.02).abs() < 0.001 => 0.05,
            _ => 0.005,
        };
    }

    pub fn update_progress(&mut self, files: usize, size: u64, speed: f64, path: String) {
        self.files_scanned = files;
        self.total_size_scanned = size;
        self.scan_speed = speed;
        self.current_scanning_path = path;
    }

    pub fn set_scan_result(&mut self, result: ScanResult) {
        self.error_count = result.errors.len();
        self.view_mode = ViewMode::Normal;
        self.current_path = result.scan_path.clone();
        self.scan_result = Some(result);
        self.selected_index = 0;
        self.list_offset = 0;
    }
}

fn find_node<'a>(node: &'a Node, path: &PathBuf) -> Option<&'a Node> {
    if &node.path == path {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_node(child, path) {
            return Some(found);
        }
    }
    None
}
