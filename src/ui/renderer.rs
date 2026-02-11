use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::ui::app_state::{AppState, FocusPanel, ViewMode};
use crate::ui::widgets::file_list::{FileList, FileListItem, FileListState, format_size};
use crate::ui::widgets::progress_bar::ScanProgressBar;
use crate::ui::widgets::ring_chart::{RingChart, RingChartItem};
use crate::ui::widgets::status_bar::StatusBar;

pub fn render(frame: &mut Frame, state: &AppState) {
    match state.view_mode {
        ViewMode::Scanning => render_scanning(frame, state),
        ViewMode::Normal => render_normal(frame, state),
        ViewMode::Help => {
            render_normal(frame, state);
            render_help_overlay(frame);
        }
        ViewMode::ErrorList => {
            render_normal(frame, state);
            render_error_overlay(frame, state);
        }
        ViewMode::Export => render_normal(frame, state),
    }
}

fn render_scanning(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // title
            Constraint::Min(5),    // progress
            Constraint::Length(1), // hint
        ])
        .split(area);

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" DiskLens ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!(" - Scanning: {} ", state.current_path.display()),
            Style::default().fg(Color::White),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(title, chunks[0]);

    // Progress area - center the progress bar
    let progress_area = centered_rect(80, 4, chunks[1]);
    let progress = ScanProgressBar {
        files_scanned: state.files_scanned,
        total_size: state.total_size_scanned,
        speed: state.scan_speed,
        current_path: state.current_scanning_path.clone(),
        elapsed_secs: 0,
    };
    frame.render_widget(progress, progress_area);

    // Bottom hint
    let hint = Paragraph::new(Line::from(vec![
        Span::styled(" q", Style::default().fg(Color::Yellow)),
        Span::styled(": Quit  ", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(hint, chunks[2]);
}

fn render_normal(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // title + breadcrumb
            Constraint::Min(10),   // main content
            Constraint::Length(1), // status bar
            Constraint::Length(1), // key hints
        ])
        .split(area);

    // Title + breadcrumb
    render_breadcrumb(frame, chunks[0], state);

    // Main content: ring chart (left) | file list (right)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // ring chart
            Constraint::Percentage(60), // file list
        ])
        .split(chunks[1]);

    // Ring chart
    let ring_border_style = if state.focus == FocusPanel::RingChart {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let ring_block = Block::default()
        .title(" Ring Chart ")
        .borders(Borders::ALL)
        .border_style(ring_border_style);
    let ring_inner = ring_block.inner(main_chunks[0]);
    frame.render_widget(ring_block, main_chunks[0]);

    let total_size = state
        .current_node()
        .map(|n| n.size)
        .unwrap_or(0);

    let children = state.sorted_children();

    let ring_items: Vec<RingChartItem> = children
        .iter()
        .map(|node| {
            let percentage = if total_size > 0 {
                (node.size as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };
            RingChartItem {
                label: node.name.clone(),
                size: node.size,
                percentage,
            }
        })
        .collect();

    let ring_chart = RingChart::new(ring_items, total_size).selected(state.selected_index);
    frame.render_widget(ring_chart, ring_inner);

    // File list
    let file_border_style = if state.focus == FocusPanel::FileList {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<FileListItem> = children
        .iter()
        .map(|node| FileListItem {
            name: node.name.clone(),
            size: node.size,
            node_type: node.node_type,
            is_merged: false,
            merged_count: 0,
        })
        .collect();

    let threshold_pct = format!("{:.1}%", state.merge_threshold * 100.0);

    let file_list = FileList::new(items, total_size)
        .sort_mode(state.sort_mode, state.sort_order)
        .block(
            Block::default()
                .title(format!(" Files (threshold: {}) ", threshold_pct))
                .borders(Borders::ALL)
                .border_style(file_border_style),
        );

    let mut list_state = FileListState {
        selected: state.selected_index,
        offset: state.list_offset,
    };
    frame.render_stateful_widget(file_list, main_chunks[1], &mut list_state);

    // Status bar
    let status = StatusBar {
        error_count: state.error_count,
        files_scanned: state.files_scanned,
        speed: state.scan_speed,
        message: None,
    };
    frame.render_widget(status, chunks[2]);

    // Key hints
    let hints = Paragraph::new(Line::from(vec![
        Span::styled(" j/k", Style::default().fg(Color::Yellow)),
        Span::styled(": Navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::styled(": Open  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Backspace", Style::default().fg(Color::Yellow)),
        Span::styled(": Back  ", Style::default().fg(Color::DarkGray)),
        Span::styled("s", Style::default().fg(Color::Yellow)),
        Span::styled(": Sort  ", Style::default().fg(Color::DarkGray)),
        Span::styled("t", Style::default().fg(Color::Yellow)),
        Span::styled(": Threshold  ", Style::default().fg(Color::DarkGray)),
        Span::styled("?", Style::default().fg(Color::Yellow)),
        Span::styled(": Help  ", Style::default().fg(Color::DarkGray)),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::styled(": Quit", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(hints, chunks[3]);
}

fn render_help_overlay(frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, area);

    let help_text = vec![
        Line::from(Span::styled(
            " DiskLens - Keyboard Shortcuts ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Navigation", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("    j / Down    ", Style::default().fg(Color::Green)),
            Span::raw("Move down"),
        ]),
        Line::from(vec![
            Span::styled("    k / Up      ", Style::default().fg(Color::Green)),
            Span::raw("Move up"),
        ]),
        Line::from(vec![
            Span::styled("    Enter / l   ", Style::default().fg(Color::Green)),
            Span::raw("Enter directory"),
        ]),
        Line::from(vec![
            Span::styled("    Backspace/h ", Style::default().fg(Color::Green)),
            Span::raw("Go back"),
        ]),
        Line::from(vec![
            Span::styled("    gg          ", Style::default().fg(Color::Green)),
            Span::raw("Go to first item"),
        ]),
        Line::from(vec![
            Span::styled("    G           ", Style::default().fg(Color::Green)),
            Span::raw("Go to last item"),
        ]),
        Line::from(vec![
            Span::styled("    Tab / Arrow ", Style::default().fg(Color::Green)),
            Span::raw("Switch focus panel"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Actions", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("    s           ", Style::default().fg(Color::Green)),
            Span::raw("Cycle sort mode"),
        ]),
        Line::from(vec![
            Span::styled("    t           ", Style::default().fg(Color::Green)),
            Span::raw("Cycle merge threshold"),
        ]),
        Line::from(vec![
            Span::styled("    r           ", Style::default().fg(Color::Green)),
            Span::raw("Refresh scan"),
        ]),
        Line::from(vec![
            Span::styled("    x           ", Style::default().fg(Color::Green)),
            Span::raw("Export results"),
        ]),
        Line::from(vec![
            Span::styled("    y           ", Style::default().fg(Color::Green)),
            Span::raw("Copy current path"),
        ]),
        Line::from(vec![
            Span::styled("    o           ", Style::default().fg(Color::Green)),
            Span::raw("Open in file manager"),
        ]),
        Line::from(vec![
            Span::styled("    e           ", Style::default().fg(Color::Green)),
            Span::raw("Show error list"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ?           ", Style::default().fg(Color::Green)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("    q / Ctrl+C  ", Style::default().fg(Color::Green)),
            Span::raw("Quit"),
        ]),
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
    frame.render_widget(help, area);
}

fn render_error_overlay(frame: &mut Frame, state: &AppState) {
    let area = centered_rect(70, 60, frame.area());
    frame.render_widget(Clear, area);

    let errors = state
        .scan_result
        .as_ref()
        .map(|r| &r.errors)
        .cloned()
        .unwrap_or_default();

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" {} errors found ", errors.len()),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, err) in errors.iter().enumerate() {
        let type_str = format!("{:?}", err.error_type);
        lines.push(Line::from(vec![
            Span::styled(format!("  {}. ", i + 1), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("[{}] ", type_str), Style::default().fg(Color::Yellow)),
            Span::styled(
                err.path.display().to_string(),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("     ", Style::default()),
            Span::styled(&err.message, Style::default().fg(Color::DarkGray)),
        ]));
    }

    if errors.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No errors.",
            Style::default().fg(Color::Green),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press e or Esc to close",
        Style::default().fg(Color::DarkGray),
    )));

    let error_panel = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Errors ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .style(Style::default().bg(Color::Black))
        .wrap(Wrap { trim: false });
    frame.render_widget(error_panel, area);
}

fn render_breadcrumb(frame: &mut Frame, area: Rect, state: &AppState) {
    let path = &state.current_path;
    let mut spans = vec![
        Span::styled(" DiskLens ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
    ];

    let components: Vec<&std::ffi::OsStr> = path.components()
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
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        spans.push(Span::styled(
            component.to_string_lossy().to_string(),
            style,
        ));
    }

    // Show total size if scan result is available
    if let Some(node) = state.current_node() {
        spans.push(Span::styled(
            format!("  ({})", format_size(node.size)),
            Style::default().fg(Color::DarkGray),
        ));
    }

    let breadcrumb = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(breadcrumb, area);
}

/// Helper to create a centered rectangle within a given area
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
