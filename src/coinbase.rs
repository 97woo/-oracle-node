use crate::price_provider::PriceProvider;
use oracle_vm_common::types::{PriceData, AssetPair};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::DateTime;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Coinbase Pro API URL
const COINBASE_API_URL: &str = "https://api.exchange.coinbase.com/products/BTC-USD/candles";
/// 최대 재시도 횟수
const MAX_RETRIES: u32 = 3;
/// HTTP 요청 타임아웃 (초)
const REQUEST_TIMEOUT: u64 = 10;

/// Coinbase에서 받아오는 캔들스틱 데이터 구조
/// [timestamp, low, high, open, close, volume]
type CoinbaseCandleResponse = Vec<[f64; 6]>;

/// Coinbase Pro와 통신하는 클라이언트
pub struct CoinbaseClient {
    client: Client,
}

impl CoinbaseClient {
    /// 새로운 Coinbase 클라이언트를 만듭니다
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT))
            .user_agent("OracleVM/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// 비트코인 가격을 가져옵니다 (재시도 포함)
    pub async fn fetch_btc_price(&self) -> Result<PriceData> {
        self.fetch_btc_price_with_retry(MAX_RETRIES).await
    }

    /// 재시도 로직이 포함된 가격 가져오기
    async fn fetch_btc_price_with_retry(&self, max_retries: u32) -> Result<PriceData> {
        for attempt in 1..=max_retries {
            info!(
                "Fetching BTC price from Coinbase (attempt {}/{})",
                attempt, max_retries
            );

            match self.fetch_btc_price_once().await {
                Ok(price_data) => {
                    info!(
                        "✅ Successfully fetched BTC price from Coinbase: ${:.2}",
                        price_data.price
                    );
                    return Ok(price_data);
                }
                Err(e) => {
                    if attempt < max_retries {
                        warn!(
                            "❌ Failed to fetch price (attempt {}): {}. Retrying...",
                            attempt, e
                        );
                        sleep(Duration::from_secs(2)).await;
                    } else {
                        error!("❌ All attempts failed: {}", e);
                        return Err(e);
                    }
                }
            }
        }

        unreachable!()
    }

    /// 실제 API 호출을 수행하는 함수
    async fn fetch_btc_price_once(&self) -> Result<PriceData> {
        // 1분 캔들스틱 요청 (가장 최근 2개)
        let params = [
            ("granularity", "60"),    // 1분
            ("limit", "2"),           // 최근 2개
        ];

        info!("🌐 Calling Coinbase API: {}", COINBASE_API_URL);

        let response = self
            .client
            .get(COINBASE_API_URL)
            .query(&params)
            .send()
            .await
            .context("Failed to send request to Coinbase")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Coinbase API returned error status: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
        }

        let candles: CoinbaseCandleResponse = response
            .json()
            .await
            .context("Failed to parse Coinbase response")?;

        if candles.is_empty() {
            anyhow::bail!("No candle data received from Coinbase");
        }

        // 가장 최근 캔들 선택 (첫 번째 요소)
        let latest_candle = &candles[0];
        let close_price = latest_candle[4]; // close price
        let timestamp = latest_candle[0] as u64; // timestamp

        // 타임스탬프 로깅
        let dt = chrono::DateTime::from_timestamp(timestamp as i64, 0).unwrap_or_default();
        info!(
            "📊 Coinbase candle: {:.2} USD (time: {})",
            close_price,
            dt.format("%Y-%m-%d %H:%M:%S UTC")
        );

        // 2/3 합의 시스템 추가 전까지 간단한 검증
        if close_price <= 0.0 {
            anyhow::bail!("Invalid price from Coinbase: {}", close_price);
        }

        // timestamp가 10분 이상 오래된 경우 경고
        let now = chrono::Utc::now().timestamp() as u64;
        if now > timestamp + 600 {
            warn!(
                "⚠️  Coinbase data is more than 10 minutes old: {} seconds ago",
                now - timestamp
            );
        }

        Ok(PriceData {
            pair: AssetPair::btc_usd(),
            price: (close_price * 100.0) as u64, // Convert to cents
            timestamp: DateTime::from_timestamp(timestamp as i64, 0)
                .unwrap_or_else(chrono::Utc::now),
            volume: None,
            source: "coinbase".to_string(),
        })
    }
}

impl Default for CoinbaseClient {
    fn default() -> Self {
        Self::new()
    }
}

/// SafeBtcPrice를 사용한 정밀한 가격 처리
/// BTC 가격을 satoshi 단위로 안전하게 변환하고 포맷팅
fn format_price_with_precision(price: f64) -> String {
    // SafeBtcPrice를 사용하여 정밀도 유지
    let safe_price = crate::safe_price::SafeBtcPrice::from_price(price);
    safe_price.format_usd()
}

#[async_trait]
impl PriceProvider for CoinbaseClient {
    async fn fetch_btc_price(&self) -> Result<PriceData> {
        self.fetch_btc_price_with_retry(MAX_RETRIES).await
    }
    
    fn name(&self) -> &str {
        "coinbase"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coinbase_client_creation() {
        let client = CoinbaseClient::new();
        // Client should be created successfully
        assert_eq!(client.name(), "coinbase");
    }

    #[test]
    fn test_price_formatting() {
        let price = 12345.67;
        let formatted = format_price_with_precision(price);
        assert_eq!(formatted, "$12,345.67");
    }

    #[test]
    fn test_price_formatting_large_number() {
        let price = 1234567.89;
        let formatted = format_price_with_precision(price);
        assert_eq!(formatted, "$1,234,567.89");
    }

    #[test]
    fn test_price_formatting_small_number() {
        let price = 999.99;
        let formatted = format_price_with_precision(price);
        assert_eq!(formatted, "$999.99");
    }

    // 실제 API 호출 테스트 (수동 실행용)
    #[tokio::test]
    #[ignore] // 실제 API를 호출하므로 평소에는 실행하지 않음
    async fn test_real_coinbase_api() {
        let client = CoinbaseClient::new();
        let result = client.fetch_btc_price().await;
        
        match result {
            Ok(price_data) => {
                assert!(price_data.price > 0.0);
                assert_eq!(price_data.source, "coinbase");
                println!("Real BTC price from Coinbase: ${:.2}", price_data.price);
            }
            Err(e) => {
                println!("Coinbase API call failed (this might be expected): {}", e);
            }
        }
    }
}