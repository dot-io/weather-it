use chrono::{DateTime, Datelike};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::data::weather::{DailyEntry, icon_to_emoji};

#[derive(Debug, Default, Clone)]
pub struct DailyWeather {
    data: Vec<DailyEntry>,
    tz_offset: i64,
    selected: usize,
}

impl Widget for DailyWeather {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        if self.data.is_empty() {
            return;
        }
        let horizontal =
            Layout::horizontal((0..self.data.len()).map(|_| Constraint::Fill(1))).spacing(1);
        let rows = Layout::vertical([Constraint::Length(6)]).split(area);
        let cells = rows.iter().flat_map(|&row| horizontal.split(row).to_vec());

        for (i, cell) in cells.enumerate() {
            let entry = &self.data[i];
            let inner = Rect {
                x: cell.x + 1,
                y: cell.y + 1,
                width: cell.width.saturating_sub(2),
                height: cell.height.saturating_sub(2),
            };
            let lines = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

            let block = if i == self.selected {
                Block::default().style(Style::new().fg(Color::LightBlue))
            } else {
                Block::default()
            };
            block
                .borders(Borders::ALL)
                .title(self.day_label(i))
                .render(cell, buf);

            Paragraph::new(format!("{} {}", icon_to_emoji(&entry.icon), entry.description))
                .render(lines[0], buf);
            Paragraph::new(format!("🔺 {:.0}°  🔻 {:.0}°", entry.temp_max, entry.temp_min))
                .render(lines[1], buf);
            Paragraph::new(format!("🌡️ day {:.0}°", entry.temp_day)).render(lines[2], buf);
            let precip = match entry.rain {
                Some(mm) if mm > 0.0 => format!("🌧 {:.1}mm", mm),
                _ => format!("☁ {}%", entry.clouds),
            };
            Paragraph::new(precip).render(lines[3], buf);
        }
    }
}

impl DailyWeather {
    pub fn data(&mut self, data: Vec<DailyEntry>, tz_offset: i64) {
        self.data = data;
        self.tz_offset = tz_offset;
        if self.selected >= self.data.len() {
            self.selected = 0;
        }
    }

    pub fn select_next(&mut self) {
        if self.data.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.data.len();
    }

    pub fn select_previous(&mut self) {
        if self.data.is_empty() {
            return;
        }
        self.selected = (self.selected + self.data.len() - 1) % self.data.len();
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    fn day_label(&self, i: usize) -> String {
        let date = DateTime::from_timestamp(self.data[i].dt + self.tz_offset, 0)
            .map(|dt| dt.naive_utc().date())
            .unwrap_or_default();
        let weekday = date.format("%a");
        let day = date.day();
        let suffix = match day {
            11 | 12 | 13 => "th",
            _ => match day % 10 {
                1 => "st",
                2 => "nd",
                3 => "rd",
                _ => "th",
            },
        };
        if i == 0 {
            format!("Today {}", day)
        } else {
            format!("{} {}{}", weekday, day, suffix)
        }
    }
}
