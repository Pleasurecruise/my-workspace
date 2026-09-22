use serde::Deserialize;

const ENDPOINT: &str = "https://geocoding-api.open-meteo.com/v1/search";

pub(crate) struct ResolvedLocation {
    pub display_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
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

pub(crate) async fn resolve(
    client: &reqwest::Client,
    query: &str,
) -> Result<ResolvedLocation, String> {
    let response = client
        .get(ENDPOINT)
        .query(&[
            ("name", query),
            ("count", "1"),
            ("language", "zh"),
            ("format", "json"),
        ])
        .send()
        .await
        .map_err(|error| format!("Could not resolve {query}: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Could not resolve {query}: HTTP {}",
            response.status()
        ));
    }
    let mut results = response
        .json::<GeocodingResponse>()
        .await
        .map_err(|error| {
            format!("Location search for {query} returned an unsupported payload: {error}")
        })?
        .results;
    if results.is_empty() {
        return Err(format!("No location matched {query}"));
    }
    let resolved = results.remove(0);
    let mut parts = vec![resolved.name.as_str()];
    for part in [resolved.admin1.as_deref(), resolved.country.as_deref()]
        .into_iter()
        .flatten()
    {
        if part.is_empty()
            || parts
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(part))
        {
            continue;
        }
        parts.push(part);
    }
    Ok(ResolvedLocation {
        display_name: parts.join(", "),
        latitude: resolved.latitude,
        longitude: resolved.longitude,
        timezone: resolved.timezone,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
