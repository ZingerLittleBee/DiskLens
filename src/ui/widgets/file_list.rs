use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, StatefulWidget, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::models::node::NodeType;
use crate::ui::app_state::{SortMode, SortOrder};

pub struct FileListState {
    pub selected: usize,
    pub offset: usize,
}

pub struct FileList<'a> {
    items: Vec<FileListItem>,
    sort_mode: SortMode,
    sort_order: SortOrder,
    total_size: u64,
    block: Option<Block<'a>>,
}

pub struct FileListItem {
    pub name: String,
    pub size: u64,
    pub node_type: NodeType,
    pub is_merged: bool,
    pub merged_count: usize,
}

impl<'a> FileList<'a> {
    pub fn new(items: Vec<FileListItem>, total_size: u64) -> Self {
        Self {
            items,
            sort_mode: SortMode::Size,
            sort_order: SortOrder::Descending,
            total_size,
            block: None,
        }
    }

    pub fn sort_mode(mut self, mode: SortMode, order: SortOrder) -> Self {
        self.sort_mode = mode;
        self.sort_order = order;
        self
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }
}

impl StatefulWidget for FileList<'_> {
    type State = FileListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Render block border and get inner area
        let inner = if let Some(block) = &self.block {
            let inner = block.inner(area);
            block.clone().render(area, buf);
            inner
        } else {
            area
        };

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        // Header line: sort indicator
        let sort_indicator = match self.sort_mode {
            SortMode::Size => {
                let arrow = if self.sort_order == SortOrder::Descending { "v" } else { "^" };
                format!(" Size {} ", arrow)
            }
            SortMode::Name => {
                let arrow = if self.sort_order == SortOrder::Ascending { "^" } else { "v" };
                format!(" Name {} ", arrow)
            }
            SortMode::Modified => {
                let arrow = if self.sort_order == SortOrder::Descending { "v" } else { "^" };
                format!(" Modified {} ", arrow)
            }
        };

        let header = Line::from(vec![
            Span::styled("  Name", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:>width$}", sort_indicator, width = (inner.width as usize).saturating_sub(8)),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        buf.set_line(inner.x, inner.y, &header, inner.width);

        // Available rows for items (reserve 1 for header, 1 for footer)
        let list_height = (inner.height as usize).saturating_sub(2);
        if list_height == 0 {
            return;
        }

        // Adjust offset to ensure selected item is visible
        if state.selected < state.offset {
            state.offset = state.selected;
        }
        if state.selected >= state.offset + list_height {
            state.offset = state.selected - list_height + 1;
        }

        // Render items
        let end = (state.offset + list_height).min(self.items.len());
        for (i, item) in self.items[state.offset..end].iter().enumerate() {
            let row_y = inner.y + 1 + i as u16;
            let idx = state.offset + i;
            let is_selected = idx == state.selected;

            let icon = node_icon(&item.node_type);
            let percentage = if self.total_size > 0 {
                (item.size as f64 / self.total_size as f64) * 100.0
            } else {
                0.0
            };

            let display_name = if item.is_merged {
                format!("Others ({} items)", item.merged_count)
            } else {
                item.name.clone()
            };

            let size_str = format_size(item.size);
            let pct_str = format!("{:5.1}%", percentage);

            // Calculate available width for name
            // Layout: "  icon name     size  pct%"
            let right_part = format!("  {}  {}", size_str, pct_str);
            let right_width = right_part.len();
            let name_max = (inner.width as usize).saturating_sub(right_width + 4); // 2 for leading space + icon + space
            let display_width = display_name.width();
            let truncated_name = if display_width > name_max {
                let target = name_max.saturating_sub(3);
                let mut w = 0;
                let boundary = display_name.char_indices()
                    .find(|&(_, c)| {
                        w += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                        w > target
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(display_name.len());
                format!("{}...", &display_name[..boundary])
            } else {
                display_name
            };

            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                let fg = match item.node_type {
                    NodeType::Directory => Color::Blue,
                    NodeType::Symlink => Color::Cyan,
                    _ => Color::White,
                };
                Style::default().fg(fg)
            };

            let name_part = format!(" {} {}", icon, truncated_name);
            let padding = (inner.width as usize).saturating_sub(name_part.width() + right_part.len());
            let line_text = format!("{}{:pad$}{}", name_part, "", right_part, pad = padding);

            let line = Line::from(Span::styled(line_text, style));
            buf.set_line(inner.x, row_y, &line, inner.width);
        }

        // Footer: Total info
        let footer_y = inner.y + inner.height - 1;
        let total_str = format!(
            " Total: {} / {} items",
            format_size(self.total_size),
            self.items.len()
        );
        let footer = Line::from(Span::styled(total_str, Style::default().fg(Color::DarkGray)));
        buf.set_line(inner.x, footer_y, &footer, inner.width);
    }
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn node_icon(node_type: &NodeType) -> &str {
    match node_type {
        NodeType::Directory => "\u{1F4C1}",
        NodeType::File => "\u{1F4C4}",
        NodeType::Symlink => "\u{1F517}",
        NodeType::Other => " ",
    }
}
