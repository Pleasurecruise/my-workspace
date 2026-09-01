use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

const ENDPOINT: &str = "https://data-api.ecb.europa.eu/service/data/EXR/D.USD+GBP+CNY+JPY+CHF+AUD+CAD+HKD+SGD.EUR.SP00.A";
const CURRENCY_ORDER: [&str; 10] = [
    "EUR", "USD", "CNY", "GBP", "JPY", "CHF", "HKD", "SGD", "CAD", "AUD",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeReport {
    pub reference_currency: String,
    pub prepared_at: String,
    pub rates: Vec<ExchangeRate>,
}

impl ExchangeReport {
    /// Returns the amount of `quote` currency represented by one unit of `base` currency.
    pub fn conversion_rate(&self, base: &str, quote: &str) -> Option<f64> {
        let base = self.rates.iter().find(|rate| rate.code == base)?;
        let quote = self.rates.iter().find(|rate| rate.code == quote)?;
        Some(quote.units_per_euro / base.units_per_euro)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRate {
    pub code: String,
    pub name: String,
    pub date: String,
    pub units_per_euro: f64,
    pub previous_units_per_euro: f64,
    pub change: f64,
    pub change_percent: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope {
    header: Header,
    data_sets: Vec<DataSet>,
    structure: Structure,
}

#[derive(Deserialize)]
struct Header {
    prepared: String,
}

#[derive(Deserialize)]
struct DataSet {
    #[serde(default)]
    series: HashMap<String, Series>,
}

#[derive(Deserialize)]
struct Series {
    #[serde(default)]
    observations: HashMap<String, Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
struct Structure {
    dimensions: Dimensions,
}

#[derive(Deserialize)]
struct Dimensions {
    series: Vec<Dimension>,
    observation: Vec<Dimension>,
}

#[derive(Deserialize)]
struct Dimension {
    id: String,
    values: Vec<DimensionValue>,
}

#[derive(Deserialize)]
struct DimensionValue {
    id: String,
    name: String,
}

fn project(envelope: Envelope) -> Result<ExchangeReport, String> {
    let currency_position = envelope
        .structure
        .dimensions
        .series
        .iter()
        .position(|dimension| dimension.id == "CURRENCY")
        .ok_or_else(|| "ECB exchange response omitted the currency dimension".to_owned())?;
    let currencies = &envelope.structure.dimensions.series[currency_position].values;
    let periods = envelope
        .structure
        .dimensions
        .observation
        .iter()
        .find(|dimension| dimension.id == "TIME_PERIOD")
        .ok_or_else(|| "ECB exchange response omitted the time dimension".to_owned())?;
    let data_set = envelope
        .data_sets
        .into_iter()
        .next()
        .ok_or_else(|| "ECB exchange response contained no data set".to_owned())?;

    let mut rates = Vec::with_capacity(CURRENCY_ORDER.len());
    rates.push(ExchangeRate {
        code: "EUR".to_owned(),
        name: "Euro".to_owned(),
        date: periods
            .values
            .last()
            .map(|value| value.id.clone())
            .unwrap_or_default(),
        units_per_euro: 1.0,
        previous_units_per_euro: 1.0,
        change: 0.0,
        change_percent: 0.0,
    });

    for (series_key, series) in data_set.series {
        let currency_index = series_key
            .split(':')
            .nth(currency_position)
            .and_then(|value| value.parse::<usize>().ok())
            .ok_or_else(|| {
                format!("ECB exchange response contained invalid series key `{series_key}`")
            })?;
        let currency = currencies.get(currency_index).ok_or_else(|| {
            format!("ECB exchange response referenced unknown currency index {currency_index}")
        })?;
        if !CURRENCY_ORDER.contains(&currency.id.as_str()) {
            continue;
        }

        let mut observations = series
            .observations
            .into_iter()
            .map(|(index, values)| {
                let index = index.parse::<usize>().map_err(|_| {
                    format!("ECB exchange response contained invalid observation index `{index}`")
                })?;
                let date = periods.values.get(index).ok_or_else(|| {
                    format!("ECB exchange response referenced unknown time index {index}")
                })?;
                let rate = values
                    .first()
                    .and_then(serde_json::Value::as_f64)
                    .filter(|rate| rate.is_finite() && *rate > 0.0)
                    .ok_or_else(|| {
                        format!(
                            "ECB exchange response contained an invalid {} rate",
                            currency.id
                        )
                    })?;
                Ok((date.id.clone(), rate))
            })
            .collect::<Result<Vec<_>, String>>()?;
        observations.sort_by(|left, right| left.0.cmp(&right.0));
        let [.., previous, latest] = observations.as_slice() else {
            return Err(format!(
                "ECB exchange response contained fewer than two {} rates",
                currency.id
            ));
        };
        let change = latest.1 - previous.1;
        rates.push(ExchangeRate {
            code: currency.id.clone(),
            name: currency.name.clone(),
            date: latest.0.clone(),
            units_per_euro: latest.1,
            previous_units_per_euro: previous.1,
            change,
            change_percent: change / previous.1 * 100.0,
        });
    }

    rates.sort_by_key(|rate| {
        CURRENCY_ORDER
            .iter()
            .position(|code| *code == rate.code)
            .unwrap_or(usize::MAX)
    });
    if rates.len() != CURRENCY_ORDER.len() {
        let missing = CURRENCY_ORDER
            .iter()
            .filter(|code| !rates.iter().any(|rate| rate.code == **code))
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "ECB exchange response omitted configured currencies: {missing}"
        ));
    }

    Ok(ExchangeReport {
        reference_currency: "EUR".to_owned(),
        prepared_at: envelope.header.prepared,
        rates,
    })
}

pub async fn read() -> Result<ExchangeReport, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Vesper/0.1 exchange rates")
        .build()
        .map_err(|error| format!("Could not create exchange-rate client: {error}"))?;
    let response = client
        .get(ENDPOINT)
        .query(&[("lastNObservations", "2"), ("detail", "dataonly")])
        .header(
            reqwest::header::ACCEPT,
            "application/vnd.sdmx.data+json;version=1.0.0-wd",
        )
        .send()
        .await
        .map_err(|error| format!("Could not query ECB exchange rates: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "ECB exchange-rate request failed: HTTP {}",
            response.status()
        ));
    }
    let envelope = response
        .json()
        .await
        .map_err(|error| format!("ECB exchange rates returned an unsupported payload: {error}"))?;
    project(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> Envelope {
        serde_json::from_value(serde_json::json!({
            "header": { "prepared": "2026-08-31T21:27:28.575+02:00" },
            "dataSets": [{
                "series": {
                    "0:0:0:0:0": { "observations": { "0": [1.61], "1": [1.62] } },
                    "0:1:0:0:0": { "observations": { "0": [1.60], "1": [1.61] } },
                    "0:2:0:0:0": { "observations": { "0": [0.93], "1": [0.94] } },
                    "0:3:0:0:0": { "observations": { "0": [7.80], "1": [7.79] } },
                    "0:4:0:0:0": { "observations": { "0": [0.85], "1": [0.86] } },
                    "0:5:0:0:0": { "observations": { "0": [9.10], "1": [9.09] } },
                    "0:6:0:0:0": { "observations": { "0": [185.0], "1": [186.0] } },
                    "0:7:0:0:0": { "observations": { "0": [1.47], "1": [1.48] } },
                    "0:8:0:0:0": { "observations": { "0": [1.16], "1": [1.17] } }
                }
            }],
            "structure": {
                "dimensions": {
                    "series": [
                        { "id": "FREQ", "values": [{ "id": "D", "name": "Daily" }] },
                        { "id": "CURRENCY", "values": [
                            { "id": "AUD", "name": "Australian dollar" },
                            { "id": "CAD", "name": "Canadian dollar" },
                            { "id": "CHF", "name": "Swiss franc" },
                            { "id": "CNY", "name": "Chinese yuan renminbi" },
                            { "id": "GBP", "name": "UK pound sterling" },
                            { "id": "HKD", "name": "Hong Kong dollar" },
                            { "id": "JPY", "name": "Japanese yen" },
                            { "id": "SGD", "name": "Singapore dollar" },
                            { "id": "USD", "name": "US dollar" }
                        ] },
                        { "id": "CURRENCY_DENOM", "values": [{ "id": "EUR", "name": "Euro" }] },
                        { "id": "EXR_TYPE", "values": [{ "id": "SP00", "name": "Spot" }] },
                        { "id": "EXR_SUFFIX", "values": [{ "id": "A", "name": "Average" }] }
                    ],
                    "observation": [{ "id": "TIME_PERIOD", "values": [
                        { "id": "2026-08-28", "name": "2026-08-28" },
                        { "id": "2026-08-31", "name": "2026-08-31" }
                    ] }]
                }
            }
        }))
        .expect("valid ECB response")
    }

    #[test]
    fn projects_major_rates_and_cross_rate() {
        let report = project(payload()).expect("valid exchange report");

        assert_eq!(report.reference_currency, "EUR");
        assert_eq!(report.rates.len(), CURRENCY_ORDER.len());
        assert_eq!(report.rates[0].code, "EUR");
        assert_eq!(report.rates[1].code, "USD");
        assert_eq!(report.rates[2].code, "CNY");
        assert_eq!(report.rates[1].date, "2026-08-31");
        assert_eq!(report.conversion_rate("USD", "CNY"), Some(7.79 / 1.17));
    }

    #[test]
    fn rejects_missing_currency() {
        let mut envelope = payload();
        envelope
            .data_sets
            .first_mut()
            .expect("data set")
            .series
            .remove("0:8:0:0:0");

        let error = project(envelope).expect_err("missing USD should fail");
        assert!(error.contains("USD"));
    }

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn reads_live_rates() {
        let report = read().await.expect("exchange rates should be readable");

        assert_eq!(report.rates.len(), CURRENCY_ORDER.len());
        assert!(report.conversion_rate("USD", "CNY").is_some());
    }
}
