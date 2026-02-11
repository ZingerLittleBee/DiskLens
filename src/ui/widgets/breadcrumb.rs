use std::path::Path;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

pub struct Breadcrumb<'a> {
    path: &'a Path,
    focus_label: &'a str,
}

impl<'a> Breadcrumb<'a> {
    pub fn new(path: &'a Path, focus_label: &'a str) -> Self {
        Self { path, focus_label }
    }
}

impl Widget for Breadcrumb<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut spans = vec![
            Span::styled(
                " DiskLens v0.1.0 ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        ];

        let components: Vec<&std::ffi::OsStr> = self
            .path
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s),
                std::path::Component::RootDir => None,
                _ => None,
            })
            .collect();

        spans.push(Span::styled("/", Style::default().fg(Color::White)));

        for (i, component) in components.iter().enumerate() {
            spans.push(Span::styled(" > ", Style::default().fg(Color::DarkGray)));
            let is_last = i == components.len() - 1;
            let style = if is_last {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            spans.push(Span::styled(
                component.to_string_lossy().to_string(),
                style,
            ));
        }

        // Focus label
        spans.push(Span::styled(
            format!("   {}", self.focus_label),
            Style::default().fg(Color::DarkGray),
        ));

        let breadcrumb = Paragraph::new(Line::from(spans)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        breadcrumb.render(area, buf);
    }
}
