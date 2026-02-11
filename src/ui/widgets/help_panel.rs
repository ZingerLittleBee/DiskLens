use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

pub struct HelpPanel;

impl Widget for HelpPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let help_text = vec![
            Line::from(Span::styled(
                " DiskLens - Keyboard Shortcuts ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Navigation",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            help_line("    j / Down    ", "Move down"),
            help_line("    k / Up      ", "Move up"),
            help_line("    Enter / l   ", "Enter directory"),
            help_line("    Backspace/h ", "Go back"),
            help_line("    gg          ", "Go to first item"),
            help_line("    G           ", "Go to last item"),
            help_line("    Tab / Arrow ", "Switch focus panel"),
            Line::from(""),
            Line::from(Span::styled(
                "  Actions",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            help_line("    s           ", "Cycle sort mode"),
            help_line("    t           ", "Cycle merge threshold"),
            help_line("    r           ", "Refresh scan"),
            help_line("    x           ", "Export results"),
            help_line("    y           ", "Copy current path"),
            help_line("    o           ", "Open in file manager"),
            help_line("    e           ", "Show error list"),
            Line::from(""),
            help_line("    ?           ", "Toggle this help"),
            help_line("    q / Ctrl+C  ", "Quit"),
            Line::from(""),
            Line::from(Span::styled(
                "  Press ? or Esc to close",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let help = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title(" Help ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .style(Style::default().bg(Color::Black));
        help.render(area, buf);
    }
}

fn help_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(key, Style::default().fg(Color::Green)),
        Span::raw(desc),
    ])
}
