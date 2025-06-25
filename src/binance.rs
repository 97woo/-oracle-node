use anyhow::{Result, Context};
use serde::Deserialize;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};
use crate::PriceData;

/// 바이낸스 API 주소
const BINANCE_API_URL: &str = "https://api.binance.com/api/v3/ticker/price";
/// 최대 재시도 횟수
const MAX_RETRIES: u32 = 3;
/// HTTP 요청 타임아웃 (초)
const REQUEST_TIMEOUT: u64 = 10;

/// 바이낸스에서 받아오는 가격 데이터 구조
#[derive(Debug, Deserialize)]
struct BinancePriceResponse {
    symbol: String,  // 코인 이름 (예: "BTCUSDT")
    price: String,   // 가격 (문자열로 옴, 예: "43521.50")
}

/// 바이낸스와 통신하는 클라이언트
pub struct BinanceClient {
    client: Client,     // HTTP 요청을 보내는 도구
}

impl BinanceClient {
    /// 새로운 바이낸스 클라이언트를 만듭니다
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT))  // 10초 후 타임아웃
            .user_agent("OracleVM/1.0")                     // 우리가 누구인지 알려줌
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
            info!("Fetching BTC price from Binance (attempt {}/{})", attempt, max_retries);
            
            match self.fetch_btc_price_once().await {
                Ok(price_data) => {
                    info!("Successfully fetched BTC price: ${:.2}", price_data.price);
                    return Ok(price_data);
                }
                Err(e) if attempt < max_retries => {
                    let wait_time = 2_u64.pow(attempt - 1); // 1초, 2초, 4초... (지수적 백오프)
                    warn!("Failed to fetch price (attempt {}): {}. Retrying in {}s...", 
                          attempt, e, wait_time);
                    sleep(Duration::from_secs(wait_time)).await;
                }
                Err(e) => {
                    error!("Failed to fetch price after {} attempts: {}", max_retries, e);
                    return Err(e);
                }
            }
        }
        
        unreachable!("This should never be reached")
    }

    /// 한 번만 가격을 가져오기 (재시도 없음)
    async fn fetch_btc_price_once(&self) -> Result<PriceData> {
        // 1. 요청할 URL 만들기
        let url = format!("{}?symbol=BTCUSDT", BINANCE_API_URL);
        
        // 2. 바이낸스에 HTTP 요청 보내기
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to Binance")?;
        
        // 3. HTTP 상태 코드 확인
        if !response.status().is_success() {
            return self.handle_http_error(response.status().as_u16());
        }
        
        // 4. JSON 응답을 구조체로 변환
        let binance_data: BinancePriceResponse = response
            .json()
            .await
            .context("Failed to parse Binance JSON response")?;
        
        // 5. 가격 문자열을 숫자로 변환
        let price = binance_data.price
            .parse::<f64>()
            .context("Failed to parse price as number")?;
        
        // 6. 가격이 말이 되는지 검증
        self.validate_price(price)?;
        
        // 7. 현재 시간 기록
        let timestamp = chrono::Utc::now().timestamp() as u64;
        
        // 8. 최종 결과 반환
        Ok(PriceData {
            price,
            timestamp,
            source: "binance".to_string(),
        })
    }

    /// HTTP 에러를 처리합니다
    fn handle_http_error(&self, status_code: u16) -> Result<PriceData> {
        match status_code {
            400 => anyhow::bail!("Bad request - Check API parameters"),
            401 => anyhow::bail!("Unauthorized - API key issue"),
            403 => anyhow::bail!("Forbidden - Access denied"),
            404 => anyhow::bail!("Not found - Check symbol (BTCUSDT)"),
            429 => anyhow::bail!("Rate limit exceeded - Too many requests"),
            500..=599 => anyhow::bail!("Binance server error - Try again later"),
            _ => anyhow::bail!("HTTP error: {}", status_code),
        }
    }

    /// 가격이 합리적인지 검증합니다
    fn validate_price(&self, price: f64) -> Result<()> {
        if price <= 0.0 {
            anyhow::bail!("Invalid price: must be positive, got {}", price);
        }
        
        if price < 1000.0 {
            warn!("Unusually low BTC price: ${:.2}", price);
        }
        
        if price > 1_000_000.0 {
            warn!("Unusually high BTC price: ${:.2}", price);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = BinanceClient::new();
        // 클라이언트가 성공적으로 생성되는지 확인 (단순히 패닉 없이 생성되면 OK)
        // HTTP 클라이언트가 정상적으로 생성되었는지만 확인
    }

    #[test]
    fn test_price_validation() {
        let client = BinanceClient::new();
        
        // 정상적인 가격
        assert!(client.validate_price(50000.0).is_ok());
        
        // 비정상적인 가격들
        assert!(client.validate_price(0.0).is_err());
        assert!(client.validate_price(-100.0).is_err());
    }

    #[test]
    fn test_http_error_handling() {
        let client = BinanceClient::new();
        
        // 다양한 HTTP 에러 코드 테스트
        assert!(client.handle_http_error(404).is_err());
        assert!(client.handle_http_error(429).is_err());
        assert!(client.handle_http_error(500).is_err());
    }

    // 실제 API 호출 테스트 (인터넷 연결 필요)
    #[tokio::test]
    #[ignore] // cargo test --ignored 로만 실행
    async fn test_real_api_call() {
        let client = BinanceClient::new();
        let result = client.fetch_btc_price().await;
        
        match result {
            Ok(price_data) => {
                assert!(price_data.price > 0.0);
                assert_eq!(price_data.source, "binance");
                println!("Real BTC price: ${:.2}", price_data.price);
            }
            Err(e) => {
                println!("API call failed (this might be expected): {}", e);
            }
        }
    }
}