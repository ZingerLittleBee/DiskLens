use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use crate::ui::widgets::file_list::format_size;

const COLORS: &[Color] = &[
    Color::Blue,
    Color::Green,
    Color::Yellow,
    Color::Red,
    Color::Magenta,
    Color::Cyan,
    Color::LightBlue,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightRed,
];

const HIGHLIGHT_COLORS: &[Color] = &[
    Color::LightBlue,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightRed,
    Color::LightMagenta,
    Color::LightCyan,
    Color::White,
    Color::White,
    Color::White,
    Color::White,
];

pub struct RingChartItem {
    pub label: String,
    pub size: u64,
    pub percentage: f64,
}

pub struct RingChart {
    pub items: Vec<RingChartItem>,
    pub selected_index: usize,
    pub total_size: u64,
}

impl RingChart {
    pub fn new(items: Vec<RingChartItem>, total_size: u64) -> Self {
        Self {
            items,
            selected_index: 0,
            total_size,
        }
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }
}

struct Sector {
    start_angle: f64,
    end_angle: f64,
    color_index: usize,
    is_selected: bool,
}

impl Widget for RingChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        if self.items.is_empty() {
            let msg = "No data";
            let x = area.x + area.width.saturating_sub(msg.len() as u16) / 2;
            let y = area.y + area.height / 2;
            buf.set_string(x, y, msg, Style::default().fg(Color::DarkGray));
            return;
        }

        // Use bar chart fallback for small areas
        if area.width < 20 || area.height < 10 {
            render_bar_chart(&self, area, buf);
            return;
        }

        // Reserve right side for legend
        let legend_width = 22u16;
        let chart_width = if area.width > legend_width + 12 {
            area.width - legend_width
        } else {
            area.width
        };
        let show_legend = area.width > legend_width + 12;

        let chart_area = Rect::new(area.x, area.y, chart_width, area.height);

        // Calculate center and radii
        // Terminal chars are roughly 1:2 aspect ratio (width:height)
        // We work in "pixel" coords where each cell = 1 wide, 2 tall (half-block)
        let cx = chart_area.width as f64 / 2.0;
        let cy = chart_area.height as f64; // in half-block pixels, total height = area.height * 2

        let max_r_by_width = cx * 0.90;
        let max_r_by_height = cy * 0.85;
        let outer_r = max_r_by_width.min(max_r_by_height);
        let inner_r = outer_r * 0.50;

        // Build sectors
        let total: f64 = self.items.iter().map(|i| i.size as f64).sum();
        if total == 0.0 {
            return;
        }

        let mut sectors = Vec::new();
        let mut angle = -std::f64::consts::FRAC_PI_2; // start from top

        for (i, item) in self.items.iter().enumerate() {
            let fraction = item.size as f64 / total;
            let sweep = fraction * std::f64::consts::TAU;
            let end = angle + sweep;
            sectors.push(Sector {
                start_angle: angle,
                end_angle: end,
                color_index: i % COLORS.len(),
                is_selected: i == self.selected_index,
            });
            angle = end;
        }

        // Render the ring pixel by pixel using half-block characters
        for row in 0..chart_area.height {
            for col in 0..chart_area.width {
                let py_top = row as f64 * 2.0;
                let py_bottom = row as f64 * 2.0 + 1.0;
                let px = col as f64;

                let top_color = pixel_color(px, py_top, cx, cy, inner_r, outer_r, &sectors);
                let bottom_color = pixel_color(px, py_bottom, cx, cy, inner_r, outer_r, &sectors);

                if let Some(cell) = buf.cell_mut((chart_area.x + col, chart_area.y + row)) {
                    match (top_color, bottom_color) {
                        (Some(tc), Some(bc)) if tc == bc => {
                            cell.set_char('\u{2588}'); // █ full block
                            cell.set_fg(tc);
                        }
                        (Some(tc), Some(bc)) => {
                            cell.set_char('\u{2580}'); // ▀ upper half
                            cell.set_fg(tc);
                            cell.set_bg(bc);
                        }
                        (Some(tc), None) => {
                            cell.set_char('\u{2580}'); // ▀ upper half
                            cell.set_fg(tc);
                        }
                        (None, Some(bc)) => {
                            cell.set_char('\u{2584}'); // ▄ lower half
                            cell.set_fg(bc);
                        }
                        (None, None) => {}
                    }
                }
            }
        }

        // Render center text (total size)
        let center_text = format_size(self.total_size);
        let text_len = center_text.len() as u16;
        let text_x = chart_area.x + (chart_area.width.saturating_sub(text_len)) / 2;
        let text_y = chart_area.y + chart_area.height / 2;
        buf.set_string(
            text_x,
            text_y,
            &center_text,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

        // Render legend on right side
        if show_legend {
            let legend_x = chart_area.x + chart_area.width + 1;
            let max_legend_items = (area.height as usize).saturating_sub(1);
            let legend_items = self.items.len().min(max_legend_items);

            for (i, item) in self.items.iter().take(legend_items).enumerate() {
                let y = area.y + i as u16;
                if y >= area.y + area.height {
                    break;
                }

                let color = COLORS[i % COLORS.len()];
                let is_sel = i == self.selected_index;

                let style = if is_sel {
                    Style::default()
                        .fg(HIGHLIGHT_COLORS[i % HIGHLIGHT_COLORS.len()])
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color)
                };

                // Color swatch
                buf.set_string(legend_x, y, "\u{2588}\u{2588}", style);

                // Label: truncated name + percentage
                let pct_str = format!("{:4.1}%", item.percentage);
                let avail = (area.x + area.width).saturating_sub(legend_x + 3) as usize;
                let pct_len = pct_str.len();
                let name_max = avail.saturating_sub(pct_len + 1);

                let truncated = if item.label.len() > name_max {
                    format!("{}~", &item.label[..name_max.saturating_sub(1).max(1)])
                } else {
                    item.label.clone()
                };
                let padding = name_max.saturating_sub(truncated.len());

                let label_style = if is_sel {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };
                let pct_style = if is_sel {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let label_text = format!(" {}{:pad$} ", truncated, "", pad = padding);
                buf.set_string(legend_x + 2, y, &label_text, label_style);
                let pct_x = legend_x + 2 + label_text.len() as u16;
                if pct_x + pct_str.len() as u16 <= area.x + area.width {
                    buf.set_string(pct_x, y, &pct_str, pct_style);
                }
            }
        }
    }
}

fn pixel_color(
    px: f64,
    py: f64,
    cx: f64,
    cy: f64,
    inner_r: f64,
    outer_r: f64,
    sectors: &[Sector],
) -> Option<Color> {
    // Distance from center, compensating for terminal char aspect ratio
    let dx = px - cx;
    let dy = py - cy;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < inner_r || dist > outer_r {
        return None;
    }

    // Calculate angle
    let mut angle = dy.atan2(dx);
    // Normalize to same range as sectors (starting from -PI/2)
    if angle < -std::f64::consts::FRAC_PI_2 {
        angle += std::f64::consts::TAU;
    }

    for sector in sectors {
        let mut start = sector.start_angle;
        let mut end = sector.end_angle;

        // Normalize for comparison
        if start < -std::f64::consts::FRAC_PI_2 {
            start += std::f64::consts::TAU;
        }
        if end < -std::f64::consts::FRAC_PI_2 {
            end += std::f64::consts::TAU;
        }

        let in_sector = if start <= end {
            angle >= start && angle < end
        } else {
            angle >= start || angle < end
        };

        if in_sector {
            return if sector.is_selected {
                Some(HIGHLIGHT_COLORS[sector.color_index])
            } else {
                Some(COLORS[sector.color_index])
            };
        }
    }

    None
}

fn render_bar_chart(chart: &RingChart, area: Rect, buf: &mut Buffer) {
    let total: f64 = chart.items.iter().map(|i| i.size as f64).sum();
    if total == 0.0 {
        return;
    }

    // Title
    let title = format_size(chart.total_size);
    let title_x = area.x + area.width.saturating_sub(title.len() as u16) / 2;
    buf.set_string(
        title_x,
        area.y,
        &title,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let bar_area_y = area.y + 1;
    let bar_area_height = area.height.saturating_sub(1) as usize;
    let bar_width = area.width.saturating_sub(2) as usize;

    for (i, item) in chart.items.iter().take(bar_area_height).enumerate() {
        let y = bar_area_y + i as u16;
        if y >= area.y + area.height {
            break;
        }

        let fraction = item.size as f64 / total;
        let filled = (fraction * bar_width as f64).round() as usize;
        let color_idx = i % COLORS.len();
        let is_sel = i == chart.selected_index;

        let color = if is_sel {
            HIGHLIGHT_COLORS[color_idx]
        } else {
            COLORS[color_idx]
        };

        let style = if is_sel {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };

        // Draw bar
        let bar: String = "\u{2588}".repeat(filled.max(1));
        buf.set_string(area.x + 1, y, &bar, style);

        // Label after bar
        let label = format!(" {:4.1}%", item.percentage);
        let label_x = area.x + 1 + filled.max(1) as u16;
        if label_x + label.len() as u16 <= area.x + area.width {
            buf.set_string(
                label_x,
                y,
                &label,
                Style::default().fg(Color::DarkGray),
            );
        }
    }
}
