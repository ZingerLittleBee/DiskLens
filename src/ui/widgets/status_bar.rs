use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};

pub struct StatusBar {
    pub error_count: usize,
    pub files_scanned: usize,
    pub speed: f64,
    pub message: Option<String>,
}

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 || area.width < 10 {
            return;
        }

        // If there is a temporary message, show it
        if let Some(msg) = &self.message {
            let line = Line::from(Span::styled(
                format!(" {}", msg),
                Style::default().fg(Color::Green),
            ));
            buf.set_line(area.x, area.y, &line, area.width);
            return;
        }

        let mut spans = Vec::new();

        // Left: error count
        if self.error_count > 0 {
            spans.push(Span::styled(
                format!(" ! {} errors (press 'e' to view) ", self.error_count),
                Style::default().fg(Color::Red),
            ));
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        }

        // Middle: file count
        spans.push(Span::styled(
            format!(" Scanned: {} files", format_number(self.files_scanned)),
            Style::default().fg(Color::White),
        ));

        // Right: speed
        if self.speed > 0.0 {
            // Calculate padding
            let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
            let speed_str = format!("Speed: {:.0}/s ", self.speed);
            let padding = (area.width as usize).saturating_sub(left_len + speed_str.len());
            spans.push(Span::styled(
                format!("{:pad$}", "", pad = padding),
                Style::default(),
            ));
            spans.push(Span::styled(
                speed_str,
                Style::default().fg(Color::DarkGray),
            ));
        }

        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }
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
