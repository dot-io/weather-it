# Weather

<p align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/oneirosoft/weather-it/release.yml" height="32" />
  <img src="https://img.shields.io/github/v/release/oneirosoft/weather-it" height="32" />
  <img src="https://img.shields.io/github/license/oneirosoft/weather-it" height="32" />
</p>

![Screenshot](./imgs/screenshot.png)

Weather is a terminal-based weather dashboard built with Rust and [ratatui](https://github.com/ratatui-org/ratatui). It provides real-time weather forecasts, including a scrollable weekly overview and per-day details, using the [OpenWeatherMap One Call API 4.0](https://openweathermap.org/api/one-call-4). On launch it auto-locates you by IP; a search bar lets you look up any other city.

> This is a fork of [oneirosoft/weather-it](https://github.com/oneirosoft/weather-it), migrated from Open-Meteo to OpenWeatherMap and extended with a full per-day detail view (morning/day/evening/night temps, feels-like, humidity, dew point, pressure, wind & gusts, cloud cover, UV index, sunrise/sunset, moon phase, and weather alerts).

## How to Use

- Launch the app in your terminal. It auto-locates by IP and loads the forecast.
- The week overview is on top; the selected day's full details and hourly table are below.
- Move between days with `h`/`l`, scroll the hourly table with `j`/`k`.
- Press `/` to focus the search bar, type a city, and press `Enter`. `Esc` cancels.

### Keyboard Shortcuts

| Shortcut             | Action                                   |
| -------------------- | ---------------------------------------- |
| `h` / `←`            | Previous day                             |
| `l` / `→` / `Tab`    | Next day                                 |
| `j` / `↓`            | Scroll hourly table down                 |
| `k` / `↑`            | Scroll hourly table up                   |
| `/` or `i`           | Focus the search bar                     |
| `Enter`              | Search for the typed location            |
| `Esc`                | Cancel search / quit (in browse mode)    |
| `r`                  | Refresh weather data                     |
| `q` / `Ctrl+C`       | Exit the app                             |

## How to Configure and Run

1. **Install Rust**  
   Make sure you have [Rust](https://rustup.rs/) installed.

2. **Get an OpenWeatherMap API key with One Call 4.0**
   - Create a key at [openweathermap.org](https://openweathermap.org/api).
   - Subscribe to the **One Call by Call** plan (the free tier allows **1,000 calls/day**; a card is required, but you can cap calls in **Billing plans** to avoid charges). One Call 4.0 will return `401` until this subscription is active.

3. **Provide the key** via either:
   - the `OWM_API_KEY` environment variable, or
   - a single-line file at `~/.config/waybar/scripts/.owm_api_key`.

4. **Build and Run**

   ```sh
   cargo run
   ```

The app makes 3 API calls per refresh (current + daily + hourly) and auto-refreshes every 30 minutes, staying well under the free 1,000 calls/day.
