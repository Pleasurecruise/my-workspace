use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const ENDPOINT: &str = "https://query1.finance.yahoo.com/v8/finance/chart";
const QUERY: &str = "range=1mo&interval=1d&includePrePost=false&events=div%2Csplits";
const CONCURRENCY: usize = 2;
#[derive(Clone)]
struct Stock {
    symbol: String,
    name: String,
}

impl Stock {
    fn new(symbol: &str, name: &str) -> Self {
        Self {
            symbol: symbol.to_owned(),
            name: name.to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockReport {
    pub stocks: Vec<StockSeries>,
    pub failures: Vec<StockFailure>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockFailure {
    pub symbol: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockSeries {
    pub symbol: String,
    pub name: String,
    pub currency: String,
    pub exchange: String,
    pub price: f64,
    pub change: f64,
    pub change_percent: f64,
    pub points: Vec<StockPoint>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockPoint {
    pub timestamp: i64,
    pub close: f64,
}

#[derive(Deserialize)]
struct Envelope {
    chart: Chart,
}

#[derive(Deserialize)]
struct Chart {
    result: Option<Vec<ChartResult>>,
    error: Option<ChartError>,
}

#[derive(Deserialize)]
struct ChartError {
    code: String,
    description: String,
}

#[derive(Deserialize)]
struct ChartResult {
    meta: Meta,
    #[serde(default)]
    timestamp: Vec<i64>,
    indicators: Indicators,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    currency: Option<String>,
    exchange_name: Option<String>,
    short_name: Option<String>,
}

#[derive(Deserialize)]
struct Indicators {
    #[serde(default)]
    quote: Vec<Quote>,
}

#[derive(Deserialize)]
struct Quote {
    #[serde(default)]
    close: Vec<Option<f64>>,
}

fn project(stock: Stock, envelope: Envelope) -> Result<StockSeries, String> {
    if let Some(error) = envelope.chart.error {
        return Err(format!(
            "{} quote failed: {} ({})",
            stock.symbol, error.description, error.code
        ));
    }
    let chart = envelope
        .chart
        .result
        .and_then(|result| result.into_iter().next())
        .ok_or_else(|| format!("{} quote returned no chart", stock.symbol))?;
    let closes = chart
        .indicators
        .quote
        .into_iter()
        .next()
        .ok_or_else(|| format!("{} quote returned no prices", stock.symbol))?
        .close;
    let mut points: Vec<_> = chart
        .timestamp
        .into_iter()
        .zip(closes)
        .filter_map(|(timestamp, close)| {
            close
                .filter(|close| close.is_finite())
                .map(|close| StockPoint { timestamp, close })
        })
        .collect();
    points.sort_by_key(|point| point.timestamp);
    points.dedup_by_key(|point| point.timestamp);
    let [.., previous, latest] = points.as_slice() else {
        return Err(format!(
            "{} quote returned fewer than two prices",
            stock.symbol
        ));
    };
    let change = latest.close - previous.close;
    let change_percent = if previous.close == 0.0 {
        0.0
    } else {
        change / previous.close * 100.0
    };
    Ok(StockSeries {
        symbol: stock.symbol,
        name: chart.meta.short_name.unwrap_or(stock.name),
        currency: chart.meta.currency.unwrap_or_else(|| "USD".to_owned()),
        exchange: chart.meta.exchange_name.unwrap_or_default(),
        price: latest.close,
        change,
        change_percent,
        points,
    })
}

async fn request(client: &reqwest::Client, stock: Stock) -> Result<StockSeries, String> {
    let url = format!("{ENDPOINT}/{}?{QUERY}", stock.symbol);
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Could not query {} quote: {error}", stock.symbol))?;
    if !response.status().is_success() {
        return Err(format!(
            "{} quote request failed: HTTP {}",
            stock.symbol,
            response.status()
        ));
    }
    let envelope = response.json().await.map_err(|error| {
        format!(
            "{} quote returned an unsupported payload: {error}",
            stock.symbol
        )
    })?;
    project(stock, envelope)
}

pub async fn read(symbols: Vec<String>) -> Result<StockReport, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Vesper/0.1 stock dashboard")
        .build()
        .map_err(|error| format!("Could not create stock client: {error}"))?;
    let results = stream::iter(symbols.into_iter().map(|symbol| {
        let stock = Stock::new(&symbol, &symbol);
        async {
            request(&client, stock)
                .await
                .map_err(|message| StockFailure { symbol, message })
        }
    }))
    .buffered(CONCURRENCY)
    .collect::<Vec<_>>()
    .await;
    let mut stocks = Vec::with_capacity(results.len());
    let mut failures = Vec::new();
    for result in results {
        match result {
            Ok(stock) => stocks.push(stock),
            Err(failure) => failures.push(failure),
        }
    }
    Ok(StockReport { stocks, failures })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(closes: serde_json::Value) -> Envelope {
        serde_json::from_value(serde_json::json!({
            "chart": {
                "result": [{
                    "meta": { "currency": "USD", "exchangeName": "NMS" },
                    "timestamp": [30, 10, 20, 40],
                    "indicators": { "quote": [{ "close": closes }] }
                }],
                "error": null
            }
        }))
        .expect("valid Yahoo chart response")
    }

    #[test]
    fn projects_chart() {
        let series = project(
            Stock::new("AAPL", "Apple"),
            payload(serde_json::json!([103.0, 100.0, null, 106.0])),
        )
        .expect("valid stock series");

        assert_eq!(series.symbol, "AAPL");
        assert_eq!(series.exchange, "NMS");
        assert_eq!(series.price, 106.0);
        assert_eq!(series.change, 3.0);
        assert_eq!(series.points.len(), 3);
        assert_eq!(series.points[0].timestamp, 10);
    }

    #[test]
    fn rejects_short_chart() {
        let error = project(
            Stock::new("AAPL", "Apple"),
            payload(serde_json::json!([null, 100.0, null, null])),
        )
        .expect_err("one price should be rejected");

        assert!(error.contains("fewer than two prices"));
    }

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn reads_live_stocks() {
        let symbols = vec!["AAPL".to_owned(), "MSFT".to_owned()];
        let report = read(symbols.clone())
            .await
            .expect("stock report should be readable");

        assert_eq!(report.stocks.len(), symbols.len());
        assert!(report.failures.is_empty());
        assert!(report.stocks.iter().all(|stock| stock.points.len() >= 2));
    }
}
