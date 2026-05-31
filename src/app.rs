use std::{boxed::Box, error::Error, time::Duration};

use chrono::Local;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::sleep,
};

use crate::{
    data::weather::{WeatherResponse, icon_to_emoji},
    layout::center,
    weather_service::{WeatherData, dispatch_weather},
    widgets::{
        daily_weather::DailyWeather, day_detail::DayDetail, loader::Loader, search::Search,
    },
};

/// Result delivered from a background fetch task.
pub type FetchResult = Result<WeatherData, String>;

#[derive(PartialEq, Eq, Clone, Copy)]
enum Focus {
    Browse,
    Search,
}

pub struct App {
    search: Search,
    loader: Loader,
    daily: DailyWeather,
    location_name: Option<String>,
    error: Option<String>,
    focus: Focus,
    hourly_offset: usize,
    exit: bool,
    weather: WeatherResponse,
    weather_tx: Sender<FetchResult>,
    loading: bool,
    refresh_handle: Option<tokio::task::JoinHandle<()>>,
}

impl App {
    pub fn new(weather_tx: Sender<FetchResult>) -> Self {
        Self {
            search: Search::default(),
            daily: DailyWeather::default(),
            location_name: None,
            error: None,
            focus: Focus::Browse,
            hourly_offset: 0,
            exit: false,
            weather: WeatherResponse::default(),
            weather_tx,
            loading: false,
            loader: Loader::default(),
            refresh_handle: None,
        }
    }

    /// Trigger the first fetch (empty query → IP-based location).
    pub fn request_initial_fetch(&mut self) {
        self.refresh_handle = Some(self.spawn_fetch(String::new()));
    }

    pub async fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        weather_rx: &mut Receiver<FetchResult>,
    ) -> Result<(), Box<dyn Error>> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events().await?;
            if self.loading {
                self.loader.calc_next();
            }
            if let Ok(Some(result)) =
                tokio::time::timeout(Duration::from_millis(10), weather_rx.recv()).await
            {
                self.update_state(result);
            }
            sleep(Duration::from_millis(10)).await;
        }
        Ok(())
    }

    fn update_state(&mut self, result: FetchResult) {
        self.loading = false;
        self.loader = Loader::default();
        match result {
            Ok(data) => {
                self.error = None;
                self.daily
                    .data(data.weather.daily.clone(), data.weather.timezone_offset);
                self.hourly_offset = 0;
                self.weather = data.weather;
                self.location_name = Some(data.location_name);
            }
            Err(message) => self.error = Some(message),
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let rows = Layout::vertical([
            Constraint::Length(3), // search
            Constraint::Length(2), // location + now
            Constraint::Length(8), // week overview
            Constraint::Fill(1),   // selected-day detail
            Constraint::Length(1), // status line
        ])
        .split(area);

        // ---- search --------------------------------------------------------
        let centered_search = center(rows[0], rows[0].width / 2);
        frame.render_widget(self.search.clone(), centered_search);
        if self.loading {
            let loader_area = Rect {
                x: centered_search.x + centered_search.width.saturating_sub(3),
                y: centered_search.y + centered_search.height.saturating_sub(2),
                width: 1,
                height: 1,
            };
            frame.render_widget(self.loader.clone(), loader_area);
        }

        // ---- location + current summary ------------------------------------
        self.draw_header(frame, rows[1]);

        // ---- week overview -------------------------------------------------
        if !self.weather.daily.is_empty() {
            let centered = center(rows[2], (rows[2].width as f32 * 0.95) as u16);
            frame.render_widget(self.daily.clone(), centered);
        }

        // ---- detail / error ------------------------------------------------
        if let Some(error) = &self.error {
            let msg = Paragraph::new(error.as_str())
                .fg(Color::Red)
                .block(Block::bordered().title("Error"));
            frame.render_widget(msg, center(rows[3], (rows[3].width as f32 * 0.9) as u16));
        } else if !self.weather.daily.is_empty() {
            let centered = center(rows[3], (rows[3].width as f32 * 0.95) as u16);
            frame.render_widget(
                DayDetail::new(&self.weather, self.daily.selected_index(), self.hourly_offset),
                centered,
            );
        }

        // ---- status line ---------------------------------------------------
        self.draw_status(frame, rows[4]);
    }

    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let centered = center(area, (area.width as f32 * 0.9) as u16);
        let mut spans = Vec::new();
        if let Some(name) = &self.location_name {
            spans.push(Span::styled(name.clone(), Style::new().bold()));
        }
        if !self.weather.daily.is_empty() {
            let c = &self.weather.current;
            spans.push(Span::raw("   "));
            spans.push(Span::raw(format!(
                "{} {:.0}°C  ({})  feels {:.0}°C",
                icon_to_emoji(&c.icon),
                c.temp,
                c.description,
                c.feels_like
            )));
        }
        let mut lines = vec![Line::from(spans).centered()];
        if let Some(alert) = self.weather.alerts.first() {
            lines.push(
                Line::from(format!("⚠ {}", alert.event))
                    .centered()
                    .fg(Color::Yellow),
            );
        }
        frame.render_widget(Paragraph::new(lines), centered);
    }

    fn draw_status(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Block::new().bg(Color::DarkGray).fg(Color::White), area);
        let hints = match self.focus {
            Focus::Browse => " h/l day · j/k hours · / search · r refresh · q quit ",
            Focus::Search => " type a city · Enter search · Esc cancel ",
        };
        let inner = center(area, (area.width as f32 * 0.95) as u16);
        let cols = Layout::horizontal([Constraint::Fill(1), Constraint::Length(10)]).split(inner);
        frame.render_widget(Paragraph::new(hints).bold(), cols[0]);
        let time = Local::now().format("%H:%M:%S").to_string();
        frame.render_widget(Paragraph::new(time).right_aligned().bold(), cols[1]);
    }

    async fn handle_events(&mut self) -> Result<(), Box<dyn Error>> {
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key_event) = event::read()? {
                self.handle_key_event(key_event);
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        // Ctrl+C always exits.
        if key_event.modifiers == KeyModifiers::CONTROL && key_event.code == KeyCode::Char('c') {
            self.exit = true;
            return;
        }
        match self.focus {
            Focus::Browse => self.handle_browse_key(key_event),
            Focus::Search => self.handle_search_key(key_event),
        }
    }

    fn handle_browse_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Esc => self.exit = true,
            KeyCode::Char('h') | KeyCode::Left => {
                self.daily.select_previous();
                self.hourly_offset = 0;
            }
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => {
                self.daily.select_next();
                self.hourly_offset = 0;
            }
            KeyCode::BackTab => {
                self.daily.select_previous();
                self.hourly_offset = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => self.hourly_offset += 1,
            KeyCode::Char('k') | KeyCode::Up => {
                self.hourly_offset = self.hourly_offset.saturating_sub(1)
            }
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Char('/') | KeyCode::Char('i') => self.focus = Focus::Search,
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => self.focus = Focus::Browse,
            KeyCode::Enter => {
                self.focus = Focus::Browse;
                self.refresh();
            }
            _ => self.search.handle_key_event(key_event),
        }
    }

    fn refresh(&mut self) {
        if let Some(handle) = self.refresh_handle.take() {
            handle.abort();
        }
        let query = self.search.text();
        self.refresh_handle = Some(self.spawn_fetch(query));
    }

    fn spawn_fetch(&mut self, query: String) -> tokio::task::JoinHandle<()> {
        self.loading = true;
        let tx = self.weather_tx.clone();
        tokio::spawn(async move {
            loop {
                let result = dispatch_weather(query.as_str())
                    .await
                    .map_err(|e| e.to_string());
                let _ = tx.send(result).await;
                // Periodic auto-refresh; keeps usage well under 1000 calls/day.
                tokio::time::sleep(Duration::from_secs(1800)).await;
            }
        })
    }
}
