// Weather backend — fetches current conditions from wttr.in.
// Zero GTK4 imports. Call `fetch_weather()` from a background thread.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WeatherError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("failed to parse weather response: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, Default)]
pub struct WeatherSnapshot {
    pub temperature_c: f32,
    pub condition: String,
    pub location: String,
}

/// Fetch weather for `location` (city name, "auto" = IP-based).
/// Blocks the calling thread — run this from a spawn, not the GTK main thread.
pub fn fetch_weather(location: &str) -> Result<WeatherSnapshot, WeatherError> {
    // wttr.in JSON format 1: ?format=j1
    let loc = if location.is_empty() { "auto" } else { location };
    let url = format!("https://wttr.in/{loc}?format=j1");

    let body = ureq::get(&url)
        .call()
        .map_err(|e| WeatherError::Http(e.to_string()))?
        .body_mut()
        .read_to_string()
        .map_err(|e| WeatherError::Parse(e.to_string()))?;

    parse_wttr_json(&body)
}

fn parse_wttr_json(json: &str) -> Result<WeatherSnapshot, WeatherError> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| WeatherError::Parse(e.to_string()))?;

    let temp_c: f32 = v["current_condition"][0]["temp_C"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| WeatherError::Parse("missing temp_C".into()))?;

    let condition = v["current_condition"][0]["weatherDesc"][0]["value"]
        .as_str()
        .unwrap_or("Unknown")
        .to_owned();

    let location = v["nearest_area"][0]["areaName"][0]["value"]
        .as_str()
        .unwrap_or("")
        .to_owned();

    Ok(WeatherSnapshot { temperature_c: temp_c, condition, location })
}
