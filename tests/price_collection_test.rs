use anyhow::Result;
use async_trait::async_trait;
use mockall::{automock, predicate::*};
use oracle_node::PriceData;

// MockPriceProvider를 위한 trait
#[automock]
#[async_trait]
trait MockablePriceProvider: Send + Sync {
    async fn fetch_price(&self, symbol: &str) -> Result<PriceData>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_price_provider_returns_valid_price() {
        // Given - 가격 제공자 mock 설정
        let mut mock_provider = MockMockablePriceProvider::new();
        
        let expected_price = PriceData {
            price: 70000.0,
            timestamp: 1700000000,
            source: "mock".to_string(),
        };
        
        mock_provider
            .expect_fetch_price()
            .with(eq("BTC"))
            .times(1)
            .returning(move |_| Ok(expected_price.clone()));

        // When - 가격 조회
        let result = mock_provider.fetch_price("BTC").await;

        // Then - 올바른 가격 데이터 반환 확인
        assert!(result.is_ok());
        let price_data = result.unwrap();
        assert_eq!(price_data.price, 70000.0);
        assert_eq!(price_data.timestamp, 1700000000);
        assert_eq!(price_data.source, "mock");
    }

    #[tokio::test]
    async fn test_price_provider_handles_network_error() {
        // Given - 네트워크 오류를 반환하는 mock
        let mut mock_provider = MockMockablePriceProvider::new();
        
        mock_provider
            .expect_fetch_price()
            .with(eq("BTC"))
            .times(1)
            .returning(|_| Err(anyhow::anyhow!("Network timeout")));

        // When - 가격 조회
        let result = mock_provider.fetch_price("BTC").await;

        // Then - 오류 처리 확인
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Network timeout");
    }

    #[tokio::test]
    async fn test_price_validation_rejects_negative_price() {
        // Given - 음수 가격
        let price_data = PriceData {
            price: -100.0,
            timestamp: 1700000000,
            source: "test".to_string(),
        };

        // When & Then - 가격 검증
        assert!(!is_valid_price(&price_data));
    }

    #[tokio::test]
    async fn test_price_validation_rejects_zero_price() {
        // Given - 0 가격
        let price_data = PriceData {
            price: 0.0,
            timestamp: 1700000000,
            source: "test".to_string(),
        };

        // When & Then - 가격 검증
        assert!(!is_valid_price(&price_data));
    }

    #[tokio::test]
    async fn test_price_validation_rejects_excessive_price() {
        // Given - 비현실적으로 높은 가격 (1억 달러)
        let price_data = PriceData {
            price: 100_000_000.0,
            timestamp: 1700000000,
            source: "test".to_string(),
        };

        // When & Then - 가격 검증
        assert!(!is_valid_price(&price_data));
    }

    #[tokio::test]
    async fn test_price_validation_accepts_valid_price() {
        // Given - 유효한 가격 범위
        let test_cases = vec![1000.0, 50000.0, 100000.0, 500000.0];

        for price in test_cases {
            let price_data = PriceData {
                price,
                timestamp: 1700000000,
                source: "test".to_string(),
            };

            // When & Then - 가격 검증
            assert!(is_valid_price(&price_data), "Price {} should be valid", price);
        }
    }

    #[tokio::test]
    async fn test_timestamp_validation() {
        // Given - 다양한 타임스탬프
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let test_cases = vec![
            (0, false, "zero timestamp"),
            (1000000000, false, "too old timestamp (2001)"),
            (now - 3600, true, "1 hour ago"),
            (now, true, "current time"),
            (now + 60, true, "1 minute in future (clock drift)"),
            (now + 3600, false, "1 hour in future"),
        ];

        for (timestamp, expected, desc) in test_cases {
            let price_data = PriceData {
                price: 50000.0,
                timestamp,
                source: "test".to_string(),
            };

            // When & Then
            assert_eq!(
                is_valid_timestamp(&price_data),
                expected,
                "Timestamp validation failed for: {}",
                desc
            );
        }
    }
}

/// 가격 데이터 유효성 검증
fn is_valid_price(data: &PriceData) -> bool {
    data.price > 0.0 && data.price < 10_000_000.0 // 0 < price < $10M
}

/// 타임스탬프 유효성 검증
fn is_valid_timestamp(data: &PriceData) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let one_hour = 3600;
    let min_timestamp = 1600000000; // 2020-09-13 (reasonable minimum)
    
    data.timestamp >= min_timestamp && 
    data.timestamp <= now + 60 && // Allow 1 minute clock drift
    data.timestamp >= now - one_hour // Not older than 1 hour
}