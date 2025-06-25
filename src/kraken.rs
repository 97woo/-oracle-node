use anyhow::{Result, Context};
use serde::Deserialize;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};
use chrono::Timelike;
use crate::PriceData;

/// Kraken API URL
const KRAKEN_API_URL: &str = "https://api.kraken.com/0/public/OHLC";
/// 최대 재시도 횟수
const MAX_RETRIES: u32 = 3;
/// HTTP 요청 타임아웃 (초)
const REQUEST_TIMEOUT: u64 = 10;

/// Kraken에서 받아오는 OHLC 데이터 구조
#[derive(Debug, Deserialize)]
struct KrakenOHLCResponse {
    error: Vec<String>,
    result: Option<KrakenResult>,
}

#[derive(Debug, Deserialize)]
struct KrakenResult {
    #[serde(rename = "XXBTZUSD")]
    btc_usd: Vec<KrakenOHLC>,
    last: u64,
}

#[derive(Debug, Deserialize)]
struct KrakenOHLC(u64, String, String, String, String, String, String, u32); // [timestamp, open, high, low, close, vwap, volume, count]

/// Kraken과 통신하는 클라이언트
pub struct KrakenClient {
    client: Client,
}

impl KrakenClient {
    /// 새로운 Kraken 클라이언트를 만듭니다
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
            info!("Fetching BTC price from Kraken (attempt {}/{})", attempt, max_retries);
            
            match self.fetch_btc_price_once().await {
                Ok(price_data) => {
                    info!("Successfully fetched BTC price from Kraken: ${:.2}", price_data.price);
                    return Ok(price_data);
                }
                Err(e) if attempt < max_retries => {
                    let wait_time = 2_u64.pow(attempt - 1);
                    warn!("Failed to fetch price from Kraken (attempt {}): {}. Retrying in {}s...", 
                          attempt, e, wait_time);
                    sleep(Duration::from_secs(wait_time)).await;
                }
                Err(e) => {
                    error!("Failed to fetch price from Kraken after {} attempts: {}", max_retries, e);
                    return Err(e);
                }
            }
        }
        
        unreachable!("This should never be reached")
    }

    /// 한 번만 가격을 가져오기 (재시도 없음)
    async fn fetch_btc_price_once(&self) -> Result<PriceData> {
        // 현재 시간에서 이전 완성된 분봉 시점 계산
        let now = chrono::Utc::now();
        // 현재 분의 00초로 맞추기 (예: 14:37:XX -> 14:37:00)
        let current_minute_start = now.with_second(0).unwrap().with_nanosecond(0).unwrap();
        // 이전 분봉 가져오기 (예: 14:36:00부터)
        let target_minute_start = current_minute_start - chrono::Duration::minutes(1);
        
        let since_timestamp = target_minute_start.timestamp();
        
        info!("🎯 Kraken: Requesting OHLC since {} UTC", 
              target_minute_start.format("%H:%M:%S"));
        
        // 1분 OHLC 데이터 요청 (특정 시점부터)
        let url = format!("{}?pair=XBTUSD&interval=1&since={}", 
                         KRAKEN_API_URL, since_timestamp);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to Kraken")?;
        
        if !response.status().is_success() {
            return self.handle_http_error(response.status().as_u16());
        }
        
        let kraken_response: KrakenOHLCResponse = response
            .json()
            .await
            .context("Failed to parse Kraken JSON response")?;
        
        // API 에러 확인
        if !kraken_response.error.is_empty() {
            anyhow::bail!("Kraken API error: {:?}", kraken_response.error);
        }
        
        let result = kraken_response.result
            .ok_or_else(|| anyhow::anyhow!("No result data from Kraken"))?;
        
        if result.btc_usd.is_empty() {
            anyhow::bail!("No OHLC data received from Kraken");
        }
        
        // 가장 최근 OHLC의 종가 사용
        let latest_ohlc = &result.btc_usd[result.btc_usd.len() - 1];
        let timestamp = latest_ohlc.0; // timestamp
        let close_price = latest_ohlc.4.parse::<f64>()
            .context("Failed to parse close price from Kraken")?;
        
        // OHLC 시간 정보 로깅
        let ohlc_time = chrono::DateTime::from_timestamp(timestamp as i64, 0)
            .unwrap_or_default();
        
        info!("📊 Kraken OHLC: {:.2} USD (time: {})", 
              close_price, 
              ohlc_time.format("%H:%M:%S"));
        
        // 가격 검증
        self.validate_price(close_price)?;
        
        let timestamp = chrono::Utc::now().timestamp() as u64;
        
        Ok(PriceData {
            price: close_price,
            timestamp,
            source: "kraken".to_string(),
        })
    }

    /// HTTP 에러를 처리합니다
    fn handle_http_error(&self, status_code: u16) -> Result<PriceData> {
        match status_code {
            400 => anyhow::bail!("Bad request - Check API parameters"),
            401 => anyhow::bail!("Unauthorized - API key issue"),
            403 => anyhow::bail!("Forbidden - Access denied"),
            404 => anyhow::bail!("Not found - Check pair (XBTUSD)"),
            429 => anyhow::bail!("Rate limit exceeded - Too many requests"),
            500..=599 => anyhow::bail!("Kraken server error - Try again later"),
            _ => anyhow::bail!("HTTP error: {}", status_code),
        }
    }

    /// 가격이 합리적인지 검증합니다
    fn validate_price(&self, price: f64) -> Result<()> {
        if price <= 0.0 {
            anyhow::bail!("Invalid price: must be positive, got {}", price);
        }
        
        if price < 1000.0 {
            warn!("Unusually low BTC price from Kraken: ${:.2}", price);
        }
        
        if price > 1_000_000.0 {
            warn!("Unusually high BTC price from Kraken: ${:.2}", price);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = KrakenClient::new();
        // 클라이언트가 성공적으로 생성되는지 확인
    }

    #[test]
    fn test_price_validation() {
        let client = KrakenClient::new();
        
        // 정상적인 가격
        assert!(client.validate_price(50000.0).is_ok());
        
        // 비정상적인 가격들
        assert!(client.validate_price(0.0).is_err());
        assert!(client.validate_price(-100.0).is_err());
    }

    #[tokio::test]
    #[ignore] // cargo test --ignored 로만 실행
    async fn test_real_api_call() {
        let client = KrakenClient::new();
        let result = client.fetch_btc_price().await;
        
        match result {
            Ok(price_data) => {
                assert!(price_data.price > 0.0);
                assert_eq!(price_data.source, "kraken");
                println!("Real BTC price from Kraken: ${:.2}", price_data.price);
            }
            Err(e) => {
                println!("Kraken API call failed (this might be expected): {}", e);
            }
        }
    }
}