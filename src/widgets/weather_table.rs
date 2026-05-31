use chrono::{DateTime, NaiveDateTime, Timelike};
use ratatui::{
    layout::Constraint,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, Widget},
};

use crate::data::weather::{HourlyEntry, get_cardinal_direction, icon_to_emoji, ms_to_kmh};

/// Hourly forecast table for the selected day. Timestamps are rendered in the
/// forecast location's wall-clock time; the current hour is highlighted.
pub struct WeatherTable {
    data: Vec<HourlyEntry>,
    tz_offset: i64,
    offset: usize,
    now_ts: i64,
}

impl Widget for WeatherTable {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let block = Block::new().borders(Borders::LEFT).title("Hourly");
        if self.data.is_empty() {
            Block::render(
                block.title("Hourly (no hourly data for this day)"),
                area,
                buf,
            );
            return;
        }
        let header = Row::new(vec!["Time", "Weather", "Temp", "Feels", "Wind", "Precip"])
            .style(Style::new().bold());

        let now_local = local_dt(self.now_ts, self.tz_offset);
        let rows = self.data.iter().skip(self.offset).map(|h| {
            let t = local_dt(h.dt, self.tz_offset);
            let style = if t == now_local {
                Style::new().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::new()
            };
            Row::new(vec![
                Cell::from(format!("{:>5}", format_hour(t.hour()))),
                Cell::from(format!("{} {}", icon_to_emoji(&h.icon), h.description)),
                Cell::from(format!("{:.0}°C", h.temp)),
                Cell::from(format!("{:.0}°C", h.feels_like)),
                Cell::from(format!(
                    "{:.0} {}",
                    ms_to_kmh(h.wind_speed),
                    get_cardinal_direction(h.wind_deg)
                )),
                render_precip_bar(h.pop),
            ])
            .style(style)
        });

        let widths = [
            Constraint::Length(6),
            Constraint::Percentage(30),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Min(14),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(1)
            .block(block);
        Widget::render(table, area, buf);
    }
}

impl WeatherTable {
    pub fn new(data: Vec<HourlyEntry>, tz_offset: i64, offset: usize, now_ts: i64) -> Self {
        Self {
            data,
            tz_offset,
            offset,
            now_ts,
        }
    }
}

fn local_dt(ts: i64, tz_offset: i64) -> NaiveDateTime {
    DateTime::from_timestamp(ts + tz_offset, 0)
        .map(|dt| dt.naive_utc())
        .unwrap_or_default()
}

fn format_hour(hour: u32) -> String {
    let suffix = if hour < 12 { "AM" } else { "PM" };
    let hour_12 = match hour {
        0 => 12,
        1..=12 => hour,
        _ => hour - 12,
    };
    format!("{}{}", hour_12, suffix)
}

fn render_precip_bar(pct: u16) -> Cell<'static> {
    let width = 8usize;
    let filled = (pct as usize * width) / 100;
    let empty_len = width - filled;
    let filled = Span::styled("█".repeat(filled), Style::new().fg(Color::Blue));
    let empty = Span::raw(" ".repeat(empty_len));
    Cell::from(Line::from(vec![
        Span::raw("["),
        filled,
        empty,
        Span::raw("]"),
        Span::raw(format!(" {}%", pct)),
    ]))
}
