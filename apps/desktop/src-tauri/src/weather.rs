use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const ENDPOINT: &str = "https://api.open-meteo.com/v1/forecast";
const GEOCODING_ENDPOINT: &str = "https://geocoding-api.open-meteo.com/v1/search";
const CONCURRENCY: usize = 4;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct Weather {
    pub query: String,
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
    pub timezone_abbreviation: String,
    pub utc_offset_seconds: i64,
    pub current: Current,
    pub forecast: Vec<HourlyForecast>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherReport {
    pub locations: Vec<Weather>,
    pub failures: Vec<WeatherFailure>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherFailure {
    pub query: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HourlyForecast {
    pub time: String,
    pub temperature_2m: f64,
    pub weather_code: u16,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct Current {
    pub time: String,
    pub temperature_2m: f64,
    pub apparent_temperature: f64,
    pub relative_humidity_2m: f64,
    pub weather_code: u16,
    pub wind_speed_10m: f64,
    pub is_day: u8,
}

#[derive(Deserialize)]
struct Forecast {
    latitude: f64,
    longitude: f64,
    timezone: String,
    timezone_abbreviation: String,
    utc_offset_seconds: i64,
    current: Current,
    hourly: Hourly,
}

#[derive(Deserialize)]
struct Hourly {
    time: Vec<String>,
    temperature_2m: Vec<f64>,
    weather_code: Vec<u16>,
}

#[derive(Deserialize)]
struct GeocodingResponse {
    #[serde(default)]
    results: Vec<GeocodedLocation>,
}

#[derive(Deserialize)]
struct GeocodedLocation {
    name: String,
    latitude: f64,
    longitude: f64,
    timezone: String,
    country: Option<String>,
    admin1: Option<String>,
}

async fn request(client: &reqwest::Client, query: &str) -> Result<Weather, String> {
    let geocoding = client
        .get(GEOCODING_ENDPOINT)
        .query(&[
            ("name", query),
            ("count", "1"),
            ("language", "zh"),
            ("format", "json"),
        ])
        .send()
        .await
        .map_err(|error| format!("Could not resolve {query}: {error}"))?;
    if !geocoding.status().is_success() {
        return Err(format!(
            "Could not resolve {query}: HTTP {}",
            geocoding.status()
        ));
    }
    let mut results = geocoding
        .json::<GeocodingResponse>()
        .await
        .map_err(|error| {
            format!("Location search for {query} returned an unsupported payload: {error}")
        })?
        .results;
    if results.is_empty() {
        return Err(format!("No weather location matched {query}"));
    }
    let resolved = results.remove(0);
    let mut location_parts = vec![resolved.name.as_str()];
    for part in [resolved.admin1.as_deref(), resolved.country.as_deref()]
        .into_iter()
        .flatten()
    {
        if !part.is_empty()
            && !location_parts
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(part))
        {
            location_parts.push(part);
        }
    }
    let location = location_parts.join(", ");
    let latitude = resolved.latitude.to_string();
    let longitude = resolved.longitude.to_string();
    let response = client
        .get(ENDPOINT)
        .query(&[
            ("latitude", latitude.as_str()),
            ("longitude", longitude.as_str()),
            (
                "current",
                "temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,wind_speed_10m,is_day",
            ),
            ("hourly", "temperature_2m,weather_code"),
            ("forecast_days", "2"),
            ("timezone", resolved.timezone.as_str()),
        ])
        .send()
        .await
        .map_err(|error| format!("Could not query {location} weather: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "{location} weather request failed: HTTP {}",
            response.status()
        ));
    }
    let forecast: Forecast = response
        .json()
        .await
        .map_err(|error| format!("{location} weather returned an unsupported payload: {error}"))?;
    let first_hour = forecast
        .hourly
        .time
        .iter()
        .position(|time| time > &forecast.current.time)
        .unwrap_or(0);
    let hourly = forecast
        .hourly
        .time
        .into_iter()
        .zip(forecast.hourly.temperature_2m)
        .zip(forecast.hourly.weather_code)
        .skip(first_hour)
        .take(6)
        .map(|((time, temperature_2m), weather_code)| HourlyForecast {
            time,
            temperature_2m,
            weather_code,
        })
        .collect();
    Ok(Weather {
        query: query.to_owned(),
        location,
        latitude: forecast.latitude,
        longitude: forecast.longitude,
        timezone: forecast.timezone,
        timezone_abbreviation: forecast.timezone_abbreviation,
        utc_offset_seconds: forecast.utc_offset_seconds,
        current: forecast.current,
        forecast: hourly,
    })
}

pub async fn read(queries: Vec<String>) -> Result<WeatherReport, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| format!("Could not create weather client: {error}"))?;
    let results = stream::iter(queries.into_iter().map(|query| async {
        request(&client, &query)
            .await
            .map_err(|message| WeatherFailure { query, message })
    }))
    .buffered(CONCURRENCY)
    .collect::<Vec<_>>()
    .await;
    let mut locations = Vec::new();
    let mut failures = Vec::new();
    for result in results {
        match result {
            Ok(weather) => locations.push(weather),
            Err(failure) => failures.push(failure),
        }
    }
    Ok(WeatherReport {
        locations,
        failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_forecast() {
        let forecast: Forecast = serde_json::from_value(serde_json::json!({
            "latitude": 31.25,
            "longitude": 121.5,
            "timezone": "Asia/Shanghai",
            "timezone_abbreviation": "GMT+8",
            "utc_offset_seconds": 28800,
            "current": {
                "time": "2026-08-23T15:00",
                "temperature_2m": 31.2,
                "apparent_temperature": 36.1,
                "relative_humidity_2m": 68,
                "weather_code": 2,
                "wind_speed_10m": 11.5,
                "is_day": 1
            },
            "hourly": {
                "time": ["2026-08-23T15:00", "2026-08-23T16:00"],
                "temperature_2m": [31.2, 30.8],
                "weather_code": [2, 3]
            }
        }))
        .expect("valid Open-Meteo response");

        assert_eq!(forecast.timezone, "Asia/Shanghai");
        assert_eq!(forecast.current.weather_code, 2);
        assert_eq!(forecast.hourly.temperature_2m.len(), 2);
    }

    #[test]
    fn parses_geocoding() {
        let response: GeocodingResponse = serde_json::from_value(serde_json::json!({
            "results": [{
                "name": "杭州",
                "latitude": 30.29365,
                "longitude": 120.16142,
                "timezone": "Asia/Shanghai",
                "country": "中国",
                "admin1": "浙江"
            }]
        }))
        .expect("valid geocoding response");

        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].timezone, "Asia/Shanghai");
    }

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn reads_live_forecast() {
        let report = read(vec!["Shanghai".to_owned()])
            .await
            .expect("weather forecast should be readable");

        assert!(report.failures.is_empty(), "{:?}", report.failures);
        assert_eq!(report.locations.len(), 1);
    }
}
