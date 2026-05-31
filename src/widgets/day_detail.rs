use chrono::{DateTime, Timelike};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::{
    data::weather::{
        DailyEntry, WeatherResponse, get_cardinal_direction, hourly_weather_for, icon_to_emoji,
        moon_phase_name, ms_to_kmh,
    },
    widgets::weather_table::WeatherTable,
};

/// Detailed view of the selected day: a stats column on the left and the
/// hourly forecast table on the right.
pub struct DayDetail<'a> {
    weather: &'a WeatherResponse,
    selected: usize,
    hourly_offset: usize,
}

impl<'a> DayDetail<'a> {
    pub fn new(weather: &'a WeatherResponse, selected: usize, hourly_offset: usize) -> Self {
        Self {
            weather,
            selected,
            hourly_offset,
        }
    }
}

impl Widget for DayDetail<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let Some(entry) = self.weather.daily.get(self.selected) else {
            return;
        };

        let cols = Layout::horizontal([Constraint::Length(38), Constraint::Min(20)]).split(area);

        // ---- Left: daily stats ---------------------------------------------
        let mut lines = self.stat_lines(entry);
        if self.selected == 0 {
            lines.extend(self.current_extras());
        }
        Paragraph::new(lines)
            .block(Block::new().borders(Borders::RIGHT).title("Details"))
            .render(cols[0], buf);

        // ---- Right: hourly table -------------------------------------------
        let date = entry.date(self.weather.timezone_offset);
        let hours = hourly_weather_for(self.weather, date);
        WeatherTable::new(
            hours,
            self.weather.timezone_offset,
            self.hourly_offset,
            self.weather.current.dt,
        )
        .render(cols[1], buf);
    }
}

impl DayDetail<'_> {
    fn stat_lines(&self, e: &DailyEntry) -> Vec<Line<'static>> {
        let tz = self.weather.timezone_offset;
        let mut lines = vec![
            kv(
                "Sky",
                format!("{} {}", icon_to_emoji(&e.icon), e.description),
            ),
            kv("Morning", format!("{:.0}°C  (feels {:.0}°)", e.temp_morn, e.feels_morn)),
            kv("Day", format!("{:.0}°C  (feels {:.0}°)", e.temp_day, e.feels_day)),
            kv("Evening", format!("{:.0}°C  (feels {:.0}°)", e.temp_eve, e.feels_eve)),
            kv("Night", format!("{:.0}°C  (feels {:.0}°)", e.temp_night, e.feels_night)),
            kv("Min / Max", format!("{:.0}° / {:.0}°", e.temp_min, e.temp_max)),
            kv("Humidity", format!("{}%", e.humidity)),
            kv("Pressure", format!("{} hPa", e.pressure)),
            kv(
                "Wind",
                format!(
                    "{:.0} km/h {}",
                    ms_to_kmh(e.wind_speed),
                    get_cardinal_direction(e.wind_deg)
                ),
            ),
        ];
        if let Some(gust) = e.wind_gust {
            lines.push(kv("Gusts", format!("{:.0} km/h", ms_to_kmh(gust))));
        }
        lines.push(kv("Cloud", format!("{}%", e.clouds)));
        if let Some(rain) = e.rain {
            lines.push(kv("Rain", format!("{:.1} mm", rain)));
        }
        if let Some(snow) = e.snow {
            lines.push(kv("Snow", format!("{:.1} mm", snow)));
        }
        lines.push(kv("UV index", format!("{:.0}", e.uvi)));
        lines.push(kv("Sunrise", time_hm(e.sunrise, tz)));
        lines.push(kv("Sunset", time_hm(e.sunset, tz)));
        lines.push(kv("Moonrise", time_hm(e.moonrise, tz)));
        lines.push(kv("Moonset", time_hm(e.moonset, tz)));
        lines.push(kv("Moon", moon_phase_name(e.moon_phase).to_string()));
        lines
    }

    fn current_extras(&self) -> Vec<Line<'static>> {
        let c = &self.weather.current;
        let tz = self.weather.timezone_offset;
        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled("Now", Style::new().bold())),
            kv("Temp", format!("{:.0}°C (feels {:.0}°)", c.temp, c.feels_like)),
            kv("Humidity", format!("{}%", c.humidity)),
            kv("Dew point", format!("{:.0}°C", c.dew_point)),
            kv("Pressure", format!("{} hPa", c.pressure)),
            kv(
                "Wind",
                match c.wind_gust {
                    Some(g) => format!(
                        "{:.0} km/h {} (gust {:.0})",
                        ms_to_kmh(c.wind_speed),
                        get_cardinal_direction(c.wind_deg),
                        ms_to_kmh(g)
                    ),
                    None => format!(
                        "{:.0} km/h {}",
                        ms_to_kmh(c.wind_speed),
                        get_cardinal_direction(c.wind_deg)
                    ),
                },
            ),
            kv("Cloud", format!("{}%", c.clouds)),
            kv("Visibility", format!("{:.1} km", c.visibility as f32 / 1000.0)),
            kv("UV index", format!("{:.0}", c.uvi)),
        ];
        for alert in &self.weather.alerts {
            lines.push(Line::from(""));
            lines.push(Line::from(format!("⚠ {}", alert.event)).style(Style::new().bold().yellow()));
            if !alert.sender_name.is_empty() {
                lines.push(kv("Source", alert.sender_name.clone()));
            }
            lines.push(kv(
                "Window",
                format!("{} → {}", time_hm(alert.start, tz), time_hm(alert.end, tz)),
            ));
            // Show the first line of the description to keep the panel compact.
            if let Some(first) = alert.description.lines().find(|l| !l.trim().is_empty()) {
                lines.push(Line::from(first.trim().to_string()));
            }
        }
        lines
    }
}

fn kv(key: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<11}", key), Style::new().dim()),
        Span::raw(value),
    ])
}

fn time_hm(ts: i64, tz_offset: i64) -> String {
    if ts == 0 {
        return "—".to_string();
    }
    DateTime::from_timestamp(ts + tz_offset, 0)
        .map(|dt| {
            let t = dt.naive_utc();
            format!("{:02}:{:02}", t.hour(), t.minute())
        })
        .unwrap_or_else(|| "—".to_string())
}
