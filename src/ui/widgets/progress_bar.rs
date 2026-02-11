use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::file_list::format_size;

pub struct ScanProgressBar {
    pub files_scanned: usize,
    pub total_size: u64,
    pub speed: f64,
    pub current_path: String,
    pub elapsed_secs: u64,
}

impl Widget for ScanProgressBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 || area.width < 20 {
            return;
        }

        // Line 1: scan stats
        let size_str = format_size(self.total_size);
        let stats_line = Line::from(vec![
            Span::styled("Scanning... ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!(
                    "Scanned: {} files | Size: {} | Speed: {:.0}/s",
                    format_number(self.files_scanned),
                    size_str,
                    self.speed,
                ),
                Style::default().fg(Color::White),
            ),
        ]);
        buf.set_line(area.x, area.y, &stats_line, area.width);

        // Line 2: current path
        if area.height >= 2 {
            let path_display = truncate_path(&self.current_path, area.width as usize - 10);
            let path_line = Line::from(vec![
                Span::styled("Current: ", Style::default().fg(Color::DarkGray)),
                Span::styled(path_display, Style::default().fg(Color::DarkGray)),
            ]);
            buf.set_line(area.x, area.y + 1, &path_line, area.width);
        }
    }
}

fn truncate_path(path: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthStr;
    if path.width() <= max_width {
        return path.to_string();
    }
    if max_width < 6 {
        return "...".to_string();
    }
    // Show start and end of path
    let keep = max_width - 3; // for "..."
    let tail_len = keep / 2;
    let head_len = keep - tail_len;

    // Find char boundary for head
    let mut w = 0;
    let head_end = path.char_indices()
        .find(|&(_, c)| {
            w += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            w > head_len
        })
        .map(|(i, _)| i)
        .unwrap_or(path.len());

    // Find char boundary for tail
    w = 0;
    let tail_start = path.char_indices()
        .rev()
        .find(|&(_, c)| {
            w += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            w > tail_len
        })
        .map(|(i, _)| i + path[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(0))
        .unwrap_or(0);

    format!("{}...{}", &path[..head_end], &path[tail_start..])
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
