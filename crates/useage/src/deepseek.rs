use serde::{Deserialize, Serialize};
use std::time::Duration;

const BALANCE_URL: &str = "https://api.deepseek.com/user/balance";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct DeepSeekBalance {
    pub is_available: bool,
    pub balance_infos: Vec<BalanceInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct BalanceInfo {
    pub currency: String,
    pub total_balance: String,
    pub granted_balance: String,
    pub topped_up_balance: String,
}

pub async fn read() -> Result<DeepSeekBalance, String> {
    let api_key = crate::auth::api_key("deepseek").await?;
    let response = reqwest::Client::new()
        .get(BALANCE_URL)
        .bearer_auth(api_key)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|error| format!("Could not query DeepSeek balance: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read DeepSeek response: {error}"))?;
    if !status.is_success() {
        return Err(format!("DeepSeek balance request failed: HTTP {status}"));
    }
    serde_json::from_str(&body)
        .map_err(|error| format!("DeepSeek returned an unsupported balance payload: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_balance() {
        let balance: DeepSeekBalance = serde_json::from_value(serde_json::json!({
            "is_available": true,
            "balance_infos": [{
                "currency": "CNY",
                "total_balance": "110.00",
                "granted_balance": "10.00",
                "topped_up_balance": "100.00"
            }]
        }))
        .expect("valid balance response");

        assert!(balance.is_available);
        assert_eq!(balance.balance_infos[0].total_balance, "110.00");
    }

    #[tokio::test]
    #[ignore = "requires a locally configured DeepSeek API key"]
    async fn reads_live_balance() {
        let balance = read().await.expect("DeepSeek balance should be readable");
        assert!(!balance.balance_infos.is_empty());
    }
}
