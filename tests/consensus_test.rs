use anyhow::Result;
use oracle_node::PriceData;

/// 컨센서스 매니저 (TDD를 위한 struct)
pub struct ConsensusManager {
    threshold: f64, // 가격 차이 허용 임계값 (%)
}

impl ConsensusManager {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    /// 2/3 컨센서스 달성 여부 확인
    pub fn check_consensus(&self, prices: &[PriceData]) -> Result<Option<f64>> {
        if prices.len() < 3 {
            return Ok(None); // 최소 3개 필요
        }

        // 가격만 추출하여 정렬
        let mut sorted_prices: Vec<f64> = prices.iter().map(|p| p.price).collect();
        sorted_prices.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // 중간값 계산
        let median = if sorted_prices.len() % 2 == 0 {
            let mid = sorted_prices.len() / 2;
            (sorted_prices[mid - 1] + sorted_prices[mid]) / 2.0
        } else {
            sorted_prices[sorted_prices.len() / 2]
        };

        // 중간값 기준으로 임계값 내에 있는 가격들 필터링
        let valid_prices: Vec<f64> = sorted_prices
            .iter()
            .filter(|&&price| {
                let diff_percent = ((price - median).abs() / median) * 100.0;
                diff_percent <= self.threshold
            })
            .cloned()
            .collect();

        // 2/3 이상이 동의하는지 확인
        let required_count = (prices.len() * 2 + 2) / 3; // ceil(2/3)
        if valid_prices.len() >= required_count {
            // 유효한 가격들의 평균 반환
            let consensus_price = valid_prices.iter().sum::<f64>() / valid_prices.len() as f64;
            Ok(Some(consensus_price))
        } else {
            Ok(None)
        }
    }

    /// 아웃라이어 감지
    pub fn detect_outliers(&self, prices: &[PriceData]) -> Vec<String> {
        if prices.len() < 3 {
            return vec![];
        }

        let mut sorted_prices: Vec<(String, f64)> = prices
            .iter()
            .map(|p| (p.source.clone(), p.price))
            .collect();
        sorted_prices.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let median = if sorted_prices.len() % 2 == 0 {
            let mid = sorted_prices.len() / 2;
            (sorted_prices[mid - 1].1 + sorted_prices[mid].1) / 2.0
        } else {
            sorted_prices[sorted_prices.len() / 2].1
        };

        sorted_prices
            .iter()
            .filter(|(_, price)| {
                let diff_percent = ((price - median).abs() / median) * 100.0;
                diff_percent > self.threshold
            })
            .map(|(source, _)| source.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_price_data(source: &str, price: f64) -> PriceData {
        PriceData {
            price,
            timestamp: 1700000000,
            source: source.to_string(),
        }
    }

    #[test]
    fn test_consensus_with_three_agreeing_prices() {
        // Given - 3개의 비슷한 가격 (1% 이내)
        let prices = vec![
            create_price_data("binance", 70000.0),
            create_price_data("coinbase", 70350.0),
            create_price_data("kraken", 70200.0),
        ];

        let consensus_mgr = ConsensusManager::new(1.0); // 1% 임계값

        // When
        let result = consensus_mgr.check_consensus(&prices).unwrap();

        // Then - 컨센서스 달성
        assert!(result.is_some());
        let consensus_price = result.unwrap();
        assert!(consensus_price > 70000.0 && consensus_price < 70350.0);
    }

    #[test]
    fn test_consensus_fails_with_outlier() {
        // Given - 1개의 아웃라이어 포함 (5% 차이)
        let prices = vec![
            create_price_data("binance", 70000.0),
            create_price_data("coinbase", 70200.0),
            create_price_data("kraken", 73500.0), // 아웃라이어
        ];

        let consensus_mgr = ConsensusManager::new(1.0); // 1% 임계값

        // When
        let result = consensus_mgr.check_consensus(&prices).unwrap();

        // Then - 2/3만 동의하므로 컨센서스 달성
        assert!(result.is_some());
        let consensus_price = result.unwrap();
        assert!(consensus_price > 69900.0 && consensus_price < 70300.0);
    }

    #[test]
    fn test_consensus_fails_with_two_outliers() {
        // Given - 2개의 아웃라이어 (모두 다른 방향)
        let prices = vec![
            create_price_data("binance", 70000.0),
            create_price_data("coinbase", 73500.0), // 아웃라이어 (높음)
            create_price_data("kraken", 66500.0),   // 아웃라이어 (낮음)
        ];

        let consensus_mgr = ConsensusManager::new(1.0); // 1% 임계값

        // When
        let result = consensus_mgr.check_consensus(&prices).unwrap();

        // Then - 1/3만 동의하므로 컨센서스 실패
        assert!(result.is_none());
    }

    #[test]
    fn test_consensus_with_five_nodes() {
        // Given - 5개 노드 중 4개가 동의
        let prices = vec![
            create_price_data("binance", 70000.0),
            create_price_data("coinbase", 70100.0),
            create_price_data("kraken", 70200.0),
            create_price_data("okx", 70150.0),
            create_price_data("huobi", 75000.0), // 아웃라이어
        ];

        let consensus_mgr = ConsensusManager::new(1.0); // 1% 임계값

        // When
        let result = consensus_mgr.check_consensus(&prices).unwrap();

        // Then - 4/5 > 2/3이므로 컨센서스 달성
        assert!(result.is_some());
        let consensus_price = result.unwrap();
        assert!(consensus_price > 70000.0 && consensus_price < 70250.0);
    }

    #[test]
    fn test_consensus_requires_minimum_nodes() {
        // Given - 2개 노드만 존재
        let prices = vec![
            create_price_data("binance", 70000.0),
            create_price_data("coinbase", 70100.0),
        ];

        let consensus_mgr = ConsensusManager::new(1.0);

        // When
        let result = consensus_mgr.check_consensus(&prices).unwrap();

        // Then - 최소 3개 필요하므로 None
        assert!(result.is_none());
    }

    #[test]
    fn test_outlier_detection() {
        // Given - 다양한 가격들
        let prices = vec![
            create_price_data("binance", 70000.0),
            create_price_data("coinbase", 70200.0),
            create_price_data("kraken", 73500.0),   // 아웃라이어
            create_price_data("okx", 66500.0),      // 아웃라이어
        ];

        let consensus_mgr = ConsensusManager::new(1.0); // 1% 임계값

        // When
        let outliers = consensus_mgr.detect_outliers(&prices);

        // Then
        assert_eq!(outliers.len(), 2);
        assert!(outliers.contains(&"kraken".to_string()));
        assert!(outliers.contains(&"okx".to_string()));
    }

    #[test]
    fn test_consensus_with_exact_threshold() {
        // Given - 정확히 임계값 경계의 가격들
        let prices = vec![
            create_price_data("binance", 70000.0),
            create_price_data("coinbase", 70700.0), // 정확히 1% 차이
            create_price_data("kraken", 69300.0),   // 정확히 1% 차이
        ];

        let consensus_mgr = ConsensusManager::new(1.0); // 1% 임계값

        // When
        let result = consensus_mgr.check_consensus(&prices).unwrap();

        // Then - 경계값도 포함하므로 컨센서스 달성
        assert!(result.is_some());
    }

    #[test]
    fn test_consensus_price_calculation() {
        // Given - 정확한 가격들
        let prices = vec![
            create_price_data("binance", 70000.0),
            create_price_data("coinbase", 70500.0),
            create_price_data("kraken", 70250.0),
        ];

        let consensus_mgr = ConsensusManager::new(2.0); // 2% 임계값

        // When
        let result = consensus_mgr.check_consensus(&prices).unwrap();

        // Then - 평균값 확인
        assert!(result.is_some());
        let consensus_price = result.unwrap();
        let expected = (70000.0 + 70500.0 + 70250.0) / 3.0;
        assert!((consensus_price - expected).abs() < 0.01);
    }
}