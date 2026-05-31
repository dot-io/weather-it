//! OpenWeatherMap One Call API 4.0 client and internal weather model.
//!
//! One Call 4.0 splits data across several endpoints, so a full refresh makes
//! three calls: `/current`, `/timeline/1day` (week overview) and
//! `/timeline/1h` (near-term hourly). Requires a key subscribed to the
//! "One Call by Call" plan (free tier: 1000 calls/day).

use chrono::{DateTime, NaiveDate, NaiveDateTime};
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

const ONE_CALL_BASE: &str = "https://api.openweathermap.org/data/4.0/onecall";

// ---------------------------------------------------------------------------
// Internal model consumed by the widgets
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct WeatherResponse {
    pub current: CurrentWeather,
    pub hourly: Vec<HourlyEntry>,
    pub daily: Vec<DailyEntry>,
    pub alerts: Vec<Alert>,
    /// Seconds to add to a UTC timestamp to get local wall-clock time.
    pub timezone_offset: i64,
}

impl WeatherResponse {
    /// Convert a UTC unix timestamp into local wall-clock time for the
    /// forecast location (not the machine running the app).
    pub fn local(&self, ts: i64) -> NaiveDateTime {
        DateTime::from_timestamp(ts + self.timezone_offset, 0)
            .map(|dt| dt.naive_utc())
            .unwrap_or_default()
    }
}

#[derive(Debug, Default, Clone)]
pub struct CurrentWeather {
    pub dt: i64,
    pub temp: f32,
    pub feels_like: f32,
    pub pressure: u32,
    pub humidity: u32,
    pub dew_point: f32,
    pub uvi: f32,
    pub clouds: u32,
    pub visibility: u32,
    pub wind_speed: f32,
    pub wind_deg: f32,
    pub wind_gust: Option<f32>,
    pub description: String,
    pub icon: String,
}

#[derive(Debug, Default, Clone)]
pub struct HourlyEntry {
    pub dt: i64,
    pub temp: f32,
    pub feels_like: f32,
    /// Probability of precipitation, 0..=100 (percent).
    pub pop: u16,
    pub wind_speed: f32,
    pub wind_deg: f32,
    pub description: String,
    pub icon: String,
}

#[derive(Debug, Default, Clone)]
pub struct DailyEntry {
    pub dt: i64,
    pub sunrise: i64,
    pub sunset: i64,
    pub moonrise: i64,
    pub moonset: i64,
    pub moon_phase: f32,
    pub temp_min: f32,
    pub temp_max: f32,
    pub temp_morn: f32,
    pub temp_day: f32,
    pub temp_eve: f32,
    pub temp_night: f32,
    pub feels_morn: f32,
    pub feels_day: f32,
    pub feels_eve: f32,
    pub feels_night: f32,
    pub pressure: u32,
    pub humidity: u32,
    pub wind_speed: f32,
    pub wind_deg: f32,
    pub wind_gust: Option<f32>,
    pub clouds: u32,
    pub uvi: f32,
    pub rain: Option<f32>,
    pub snow: Option<f32>,
    /// Synthesized from clouds/rain — the daily timeline carries no condition.
    pub description: String,
    pub icon: String,
}

impl DailyEntry {
    pub fn date(&self, tz_offset: i64) -> NaiveDate {
        DateTime::from_timestamp(self.dt + tz_offset, 0)
            .map(|dt| dt.naive_utc().date())
            .unwrap_or_default()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Alert {
    pub event: String,
    pub sender_name: String,
    pub start: i64,
    pub end: i64,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Raw OWM JSON (One Call 4.0 envelopes)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    #[serde(default)]
    timezone_offset: i64,
    #[serde(default = "Vec::new")]
    data: Vec<T>,
    #[serde(default)]
    alerts: Vec<RawAlert>,
}

#[derive(Debug, Deserialize)]
struct RawCondition {
    #[serde(default)]
    description: String,
    #[serde(default)]
    icon: String,
}

#[derive(Debug, Default, Deserialize)]
struct RawCurrent {
    dt: i64,
    temp: f32,
    feels_like: f32,
    pressure: f32,
    humidity: u32,
    dew_point: f32,
    #[serde(default)]
    uvi: f32,
    clouds: u32,
    #[serde(default)]
    visibility: u32,
    wind_speed: f32,
    #[serde(default)]
    wind_deg: f32,
    wind_gust: Option<f32>,
    #[serde(default)]
    weather: Vec<RawCondition>,
}

#[derive(Debug, Deserialize)]
struct RawHourly {
    dt: i64,
    temp: f32,
    feels_like: f32,
    wind_speed: f32,
    #[serde(default)]
    wind_deg: f32,
    #[serde(default)]
    pop: f32,
    #[serde(default)]
    weather: Vec<RawCondition>,
}

#[derive(Debug, Deserialize)]
struct RawTemp {
    min: f32,
    max: f32,
    morn: f32,
    day: f32,
    eve: f32,
    night: f32,
}

#[derive(Debug, Deserialize)]
struct RawFeels {
    morn: f32,
    day: f32,
    eve: f32,
    night: f32,
}

#[derive(Debug, Deserialize)]
struct RawDaily {
    dt: i64,
    #[serde(default)]
    sunrise: i64,
    #[serde(default)]
    sunset: i64,
    #[serde(default)]
    moonrise: i64,
    #[serde(default)]
    moonset: i64,
    #[serde(default)]
    moon_phase: f32,
    temp: RawTemp,
    feels_like: RawFeels,
    pressure: f32,
    humidity: u32,
    wind_speed: f32,
    #[serde(default)]
    wind_deg: f32,
    wind_gust: Option<f32>,
    clouds: u32,
    #[serde(default)]
    uvi: f32,
    #[serde(default)]
    rain: Option<f32>,
    #[serde(default)]
    snow: Option<f32>,
    // Note: the daily timeline sends `weather: null`, so we don't read it here
    // (serde ignores it) and synthesize the condition from clouds/rain instead.
}

#[derive(Debug, Deserialize)]
struct RawAlert {
    #[serde(default)]
    event: String,
    #[serde(default)]
    sender_name: String,
    #[serde(default)]
    start: i64,
    #[serde(default)]
    end: i64,
    #[serde(default)]
    description: String,
}

fn first_condition(conds: &[RawCondition]) -> (String, String) {
    conds
        .first()
        .map(|c| (capitalize(&c.description), c.icon.clone()))
        .unwrap_or_default()
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

// ---------------------------------------------------------------------------
// API key resolution
// ---------------------------------------------------------------------------

fn key_file() -> PathBuf {
    let home = env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".config/waybar/scripts/.owm_api_key")
}

/// Resolve the OpenWeatherMap API key from `OWM_API_KEY` or the key file.
pub fn api_key() -> Result<String, Box<dyn Error + Send + Sync>> {
    if let Ok(key) = env::var("OWM_API_KEY") {
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Ok(key);
        }
    }
    let path = key_file();
    let key = fs::read_to_string(&path)
        .map_err(|_| {
            format!(
                "No OpenWeatherMap API key. Set OWM_API_KEY or write it to {}",
                path.display()
            )
        })?
        .trim()
        .to_string();
    if key.is_empty() {
        return Err(format!("API key file {} is empty", path.display()).into());
    }
    Ok(key)
}

// ---------------------------------------------------------------------------
// Fetching
// ---------------------------------------------------------------------------

async fn get_envelope<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    url: &str,
) -> Result<Envelope<T>, Box<dyn Error + Send + Sync>> {
    let resp = client.get(url).send().await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(format!("OpenWeatherMap {}: {}", status.as_u16(), text.trim()).into());
    }
    Ok(serde_json::from_str(&text)?)
}

pub async fn fetch_weather(
    latitude: f32,
    longitude: f32,
    key: &str,
) -> Result<WeatherResponse, Box<dyn Error + Send + Sync>> {
    let client = reqwest::Client::builder().user_agent("weather-it").build()?;
    let q = format!("lat={latitude}&lon={longitude}&units=metric&appid={key}");

    let current: Envelope<RawCurrent> =
        get_envelope(&client, &format!("{ONE_CALL_BASE}/current?{q}")).await?;
    let daily: Envelope<RawDaily> =
        get_envelope(&client, &format!("{ONE_CALL_BASE}/timeline/1day?{q}")).await?;
    let hourly: Envelope<RawHourly> =
        get_envelope(&client, &format!("{ONE_CALL_BASE}/timeline/1h?{q}")).await?;

    let timezone_offset = current.timezone_offset;

    let cur = current.data.into_iter().next().unwrap_or_default();
    let (cur_desc, cur_icon) = first_condition(&cur.weather);

    let response = WeatherResponse {
        timezone_offset,
        current: CurrentWeather {
            dt: cur.dt,
            temp: cur.temp,
            feels_like: cur.feels_like,
            pressure: cur.pressure.round() as u32,
            humidity: cur.humidity,
            dew_point: cur.dew_point,
            uvi: cur.uvi,
            clouds: cur.clouds,
            visibility: cur.visibility,
            wind_speed: cur.wind_speed,
            wind_deg: cur.wind_deg,
            wind_gust: cur.wind_gust,
            description: cur_desc,
            icon: cur_icon,
        },
        hourly: hourly
            .data
            .into_iter()
            .map(|h| {
                let (description, icon) = first_condition(&h.weather);
                HourlyEntry {
                    dt: h.dt,
                    temp: h.temp,
                    feels_like: h.feels_like,
                    pop: (h.pop * 100.0).round() as u16,
                    wind_speed: h.wind_speed,
                    wind_deg: h.wind_deg,
                    description,
                    icon,
                }
            })
            .collect(),
        daily: daily
            .data
            .into_iter()
            .map(|d| {
                // The daily timeline carries no `weather`, so synthesize a
                // condition from cloud cover and precipitation.
                let (description, icon) =
                    synthesize_condition(d.clouds, d.rain.unwrap_or(0.0), d.snow.unwrap_or(0.0));
                DailyEntry {
                    dt: d.dt,
                    sunrise: d.sunrise,
                    sunset: d.sunset,
                    moonrise: d.moonrise,
                    moonset: d.moonset,
                    moon_phase: d.moon_phase,
                    temp_min: d.temp.min,
                    temp_max: d.temp.max,
                    temp_morn: d.temp.morn,
                    temp_day: d.temp.day,
                    temp_eve: d.temp.eve,
                    temp_night: d.temp.night,
                    feels_morn: d.feels_like.morn,
                    feels_day: d.feels_like.day,
                    feels_eve: d.feels_like.eve,
                    feels_night: d.feels_like.night,
                    pressure: d.pressure.round() as u32,
                    humidity: d.humidity,
                    wind_speed: d.wind_speed,
                    wind_deg: d.wind_deg,
                    wind_gust: d.wind_gust,
                    clouds: d.clouds,
                    uvi: d.uvi,
                    rain: d.rain,
                    snow: d.snow,
                    description,
                    icon,
                }
            })
            .collect(),
        alerts: daily
            .alerts
            .into_iter()
            .chain(hourly.alerts)
            .chain(current.alerts)
            .map(|a| Alert {
                event: a.event,
                sender_name: a.sender_name,
                start: a.start,
                end: a.end,
                description: a.description,
            })
            .collect(),
    };

    Ok(response)
}

/// Hourly entries that fall on the given local date. May be empty for days
/// beyond the near-term hourly horizon (One Call 4.0 returns ~1 day of hours).
pub fn hourly_weather_for(data: &WeatherResponse, date: NaiveDate) -> Vec<HourlyEntry> {
    data.hourly
        .iter()
        .filter(|h| data.local(h.dt).date() == date)
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// Presentation helpers
// ---------------------------------------------------------------------------

/// Derive a `(description, icon)` for a day from cloud cover and precipitation.
/// One Call 4.0's daily timeline omits the `weather` condition, so we infer it.
fn synthesize_condition(clouds: u32, rain_mm: f32, snow_mm: f32) -> (String, String) {
    let (desc, icon) = if snow_mm > 0.0 {
        ("Snow", "13d")
    } else if rain_mm >= 2.0 {
        ("Rain", "10d")
    } else if rain_mm > 0.0 {
        ("Light rain", "10d")
    } else if clouds >= 85 {
        ("Overcast", "04d")
    } else if clouds >= 50 {
        ("Partly cloudy", "03d")
    } else if clouds >= 25 {
        ("Few clouds", "02d")
    } else {
        ("Clear sky", "01d")
    };
    (desc.to_string(), icon.to_string())
}

/// Map an OpenWeatherMap icon code (e.g. `01d`, `10n`) to an emoji.
pub fn icon_to_emoji(icon: &str) -> &'static str {
    match icon {
        "01d" => "☀️",
        "01n" => "🌙",
        "02d" => "🌤️",
        "02n" => "☁️",
        "03d" | "03n" => "⛅",
        "04d" | "04n" => "☁️",
        "09d" | "09n" => "🌧️",
        "10d" => "🌦️",
        "10n" => "🌧️",
        "11d" | "11n" => "⛈️",
        "13d" | "13n" => "❄️",
        "50d" | "50n" => "🌫️",
        _ => "🌡️",
    }
}

pub fn get_cardinal_direction(degrees: f32) -> &'static str {
    let directions = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW", "NW",
        "NNW",
    ];
    let normalized = (degrees % 360.0 + 360.0) % 360.0;
    let index = (normalized / 22.5).round() as usize % 16;
    directions[index]
}

/// Convert wind speed from m/s (OWM metric) to km/h.
pub fn ms_to_kmh(ms: f32) -> f32 {
    ms * 3.6
}

/// Describe the lunar phase from the 0..1 value OWM provides.
pub fn moon_phase_name(phase: f32) -> &'static str {
    match phase {
        p if p <= 0.0 || p >= 1.0 => "🌑 New moon",
        p if p < 0.25 => "🌒 Waxing crescent",
        p if (p - 0.25).abs() < 0.02 => "🌓 First quarter",
        p if p < 0.5 => "🌔 Waxing gibbous",
        p if (p - 0.5).abs() < 0.02 => "🌕 Full moon",
        p if p < 0.75 => "🌖 Waning gibbous",
        p if (p - 0.75).abs() < 0.02 => "🌗 Last quarter",
        _ => "🌘 Waning crescent",
    }
}
