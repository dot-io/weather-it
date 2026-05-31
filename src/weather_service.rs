use std::error::Error;

use crate::data::location::{geocode, ip_location};
use crate::data::weather::{WeatherResponse, api_key, fetch_weather};

pub struct WeatherData {
    pub weather: WeatherResponse,
    pub location_name: String,
}

/// Fetch weather for a search query, or for the IP-derived location when the
/// query is empty.
pub async fn dispatch_weather(query: &str) -> Result<WeatherData, Box<dyn Error + Send + Sync>> {
    let key = api_key()?;

    let (name, lat, lon) = if query.trim().is_empty() {
        ip_location().await?
    } else {
        let (name, geocode) = geocode(query).await?;
        let (lat, lon) = geocode
            .first()
            .cloned()
            .ok_or("No matching location found")?;
        (name, lat, lon)
    };

    let weather = fetch_weather(lat, lon, &key).await?;

    Ok(WeatherData {
        weather,
        location_name: name,
    })
}
