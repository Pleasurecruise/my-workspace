use serde::{Deserialize, Serialize};
use std::time::Duration;

const ENDPOINT: &str = "https://api.open-meteo.com/v1/forecast";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct Weather {
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
    pub shanghai: Weather,
    pub ningbo: Weather,
    pub nottingham: Weather,
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

async fn request(
    client: &reqwest::Client,
    location: &str,
    latitude: &str,
    longitude: &str,
    timezone: &str,
) -> Result<Weather, String> {
    let response = client
        .get(ENDPOINT)
        .query(&[
            ("latitude", latitude),
            ("longitude", longitude),
            (
                "current",
                "temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,wind_speed_10m,is_day",
            ),
            ("hourly", "temperature_2m,weather_code"),
            ("forecast_days", "2"),
            ("timezone", timezone),
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
        location: location.to_owned(),
        latitude: forecast.latitude,
        longitude: forecast.longitude,
        timezone: forecast.timezone,
        timezone_abbreviation: forecast.timezone_abbreviation,
        utc_offset_seconds: forecast.utc_offset_seconds,
        current: forecast.current,
        forecast: hourly,
    })
}

pub async fn read() -> Result<WeatherReport, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| format!("Could not create weather client: {error}"))?;
    let (shanghai, ningbo, nottingham) = tokio::join!(
        request(&client, "Shanghai", "31.2304", "121.4737", "Asia/Shanghai"),
        request(&client, "Ningbo", "29.8683", "121.5440", "Asia/Shanghai"),
        request(&client, "Nottingham", "52.9548", "-1.1581", "Europe/London"),
    );
    Ok(WeatherReport {
        shanghai: shanghai?,
        ningbo: ningbo?,
        nottingham: nottingham?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_forecast_contract() {
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
}
