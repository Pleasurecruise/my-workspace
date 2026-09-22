use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;
use std::time::Duration;
use time::Date;
use time::macros::format_description;

const ENDPOINT: &str = "https://api.open-meteo.com/v1/forecast";
const CONCURRENCY: usize = 4;
const SYNODIC_MONTH_DAYS: f64 = 29.530_588_853;
const KNOWN_NEW_MOON_JULIAN_DAY: f64 = 2_451_550.1;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AstronomyReport {
    pub locations: Vec<Astronomy>,
    pub failures: Vec<AstronomyFailure>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AstronomyFailure {
    pub query: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Astronomy {
    pub query: String,
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
    pub timezone_abbreviation: String,
    pub utc_offset_seconds: i64,
    pub date: String,
    pub sunrise: String,
    pub sunset: String,
    pub daylight_seconds: f64,
    pub moon: Moon,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Moon {
    pub phase: MoonPhase,
    pub age_days: f64,
    pub illumination_percent: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MoonPhase {
    NewMoon,
    WaxingCrescent,
    FirstQuarter,
    WaxingGibbous,
    FullMoon,
    WaningGibbous,
    LastQuarter,
    WaningCrescent,
}

#[derive(Deserialize)]
struct Forecast {
    latitude: f64,
    longitude: f64,
    timezone: String,
    timezone_abbreviation: String,
    utc_offset_seconds: i64,
    daily: Daily,
}

#[derive(Deserialize)]
struct Daily {
    time: Vec<String>,
    sunrise: Vec<String>,
    sunset: Vec<String>,
    daylight_duration: Vec<f64>,
}

fn moon(date: &str) -> Result<Moon, String> {
    let date = Date::parse(date, format_description!("[year]-[month]-[day]"))
        .map_err(|_| format!("Astronomy response contained invalid date `{date}`"))?;
    let age_days = (f64::from(date.to_julian_day()) - KNOWN_NEW_MOON_JULIAN_DAY)
        .rem_euclid(SYNODIC_MONTH_DAYS);
    let fraction = age_days / SYNODIC_MONTH_DAYS;
    let phase = match ((fraction * 8.0 + 0.5).floor() as u8) % 8 {
        0 => MoonPhase::NewMoon,
        1 => MoonPhase::WaxingCrescent,
        2 => MoonPhase::FirstQuarter,
        3 => MoonPhase::WaxingGibbous,
        4 => MoonPhase::FullMoon,
        5 => MoonPhase::WaningGibbous,
        6 => MoonPhase::LastQuarter,
        _ => MoonPhase::WaningCrescent,
    };
    let illumination_percent = (1.0 - (TAU * fraction).cos()) / 2.0 * 100.0;
    Ok(Moon {
        phase,
        age_days,
        illumination_percent,
    })
}

fn project(query: &str, location: String, forecast: Forecast) -> Result<Astronomy, String> {
    let date = forecast
        .daily
        .time
        .first()
        .ok_or_else(|| format!("{location} astronomy returned no date"))?;
    let sunrise = forecast
        .daily
        .sunrise
        .first()
        .ok_or_else(|| format!("{location} astronomy returned no sunrise"))?;
    let sunset = forecast
        .daily
        .sunset
        .first()
        .ok_or_else(|| format!("{location} astronomy returned no sunset"))?;
    let daylight_seconds = *forecast
        .daily
        .daylight_duration
        .first()
        .filter(|value| value.is_finite() && **value >= 0.0)
        .ok_or_else(|| format!("{location} astronomy returned invalid daylight duration"))?;
    Ok(Astronomy {
        query: query.to_owned(),
        location,
        latitude: forecast.latitude,
        longitude: forecast.longitude,
        timezone: forecast.timezone,
        timezone_abbreviation: forecast.timezone_abbreviation,
        utc_offset_seconds: forecast.utc_offset_seconds,
        date: date.clone(),
        sunrise: sunrise.clone(),
        sunset: sunset.clone(),
        daylight_seconds,
        moon: moon(date)?,
    })
}

async fn request(client: &reqwest::Client, query: &str) -> Result<Astronomy, String> {
    let resolved = crate::location::resolve(client, query).await?;
    let latitude = resolved.latitude.to_string();
    let longitude = resolved.longitude.to_string();
    let response = client
        .get(ENDPOINT)
        .query(&[
            ("latitude", latitude.as_str()),
            ("longitude", longitude.as_str()),
            ("daily", "sunrise,sunset,daylight_duration"),
            ("forecast_days", "1"),
            ("timezone", resolved.timezone.as_str()),
        ])
        .send()
        .await
        .map_err(|error| {
            format!(
                "Could not query {} astronomy: {error}",
                resolved.display_name
            )
        })?;
    if !response.status().is_success() {
        return Err(format!(
            "{} astronomy request failed: HTTP {}",
            resolved.display_name,
            response.status()
        ));
    }
    let forecast = response.json().await.map_err(|error| {
        format!(
            "{} astronomy returned an unsupported payload: {error}",
            resolved.display_name
        )
    })?;
    project(query, resolved.display_name, forecast)
}

pub async fn read(queries: Vec<String>) -> Result<AstronomyReport, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Vesper/0.1 astronomy")
        .build()
        .map_err(|error| format!("Could not create astronomy client: {error}"))?;
    let results = stream::iter(queries.into_iter().map(|query| async {
        request(&client, &query)
            .await
            .map_err(|message| AstronomyFailure { query, message })
    }))
    .buffered(CONCURRENCY)
    .collect::<Vec<_>>()
    .await;
    let mut locations = Vec::new();
    let mut failures = Vec::new();
    for result in results {
        match result {
            Ok(astronomy) => locations.push(astronomy),
            Err(failure) => failures.push(failure),
        }
    }
    Ok(AstronomyReport {
        locations,
        failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forecast() -> Forecast {
        serde_json::from_value(serde_json::json!({
            "latitude": 31.25,
            "longitude": 121.5,
            "timezone": "Asia/Shanghai",
            "timezone_abbreviation": "GMT+8",
            "utc_offset_seconds": 28800,
            "daily": {
                "time": ["2026-09-01"],
                "sunrise": ["2026-09-01T05:30"],
                "sunset": ["2026-09-01T18:15"],
                "daylight_duration": [45900.0]
            }
        }))
        .expect("valid astronomy response")
    }

    #[test]
    fn projects_sun_and_moon() {
        let astronomy = project("Shanghai", "上海, 中国".to_owned(), forecast())
            .expect("valid astronomy projection");

        assert_eq!(astronomy.sunrise, "2026-09-01T05:30");
        assert_eq!(astronomy.sunset, "2026-09-01T18:15");
        assert_eq!(astronomy.daylight_seconds, 45900.0);
        assert!((0.0..SYNODIC_MONTH_DAYS).contains(&astronomy.moon.age_days));
        assert!((0.0..=100.0).contains(&astronomy.moon.illumination_percent));
    }

    #[test]
    fn classifies_known_new_moon() {
        let moon = moon("2000-01-07").expect("valid date");

        assert_eq!(moon.phase, MoonPhase::NewMoon);
        assert!(moon.illumination_percent < 1.0);
    }

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn reads_live_astronomy() {
        let report = read(vec!["Shanghai".to_owned()])
            .await
            .expect("astronomy should be readable");

        assert!(report.failures.is_empty(), "{:?}", report.failures);
        assert_eq!(report.locations.len(), 1);
    }
}
