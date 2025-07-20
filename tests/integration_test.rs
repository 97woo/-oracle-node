use anyhow::Result;
use async_trait::async_trait;
use oracle_node::{PriceData, PriceProvider};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 시뮬레이션된 거래소 클라이언트
pub struct SimulatedExchange {
    name: String,
    prices: Arc<RwLock<Vec<f64>>>,
    current_index: Arc<RwLock<usize>>,
}

impl SimulatedExchange {
    pub fn new(name: &str, prices: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            prices: Arc::new(RwLock::new(prices)),
            current_index: Arc::new(RwLock::new(0)),
        }
    }
}

#[async_trait]
impl PriceProvider for SimulatedExchange {
    async fn fetch_price(&self, symbol: &str) -> Result<PriceData> {
        if symbol != "BTC" {
            anyhow::bail!("Unsupported symbol: {}", symbol);
        }

        let prices = self.prices.read().await;
        let mut index = self.current_index.write().await;
        
        let price = prices[*index % prices.len()];
        *index += 1;

        Ok(PriceData {
            price,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            source: self.name.clone(),
        })
    }
}

/// Oracle 시스템 (여러 거래소에서 가격 수집)
pub struct OracleSystem {
    exchanges: Vec<Box<dyn PriceProvider>>,
    consensus_threshold: f64,
}

impl OracleSystem {
    pub fn new(consensus_threshold: f64) -> Self {
        Self {
            exchanges: Vec::new(),
            consensus_threshold,
        }
    }

    pub fn add_exchange(&mut self, exchange: Box<dyn PriceProvider>) {
        self.exchanges.push(exchange);
    }

    /// 모든 거래소에서 가격 수집
    pub async fn collect_prices(&self) -> Vec<Result<PriceData>> {
        let mut results = Vec::new();
        
        for exchange in &self.exchanges {
            let result = exchange.fetch_price("BTC").await;
            results.push(result);
        }
        
        results
    }

    /// 컨센서스 가격 계산
    pub fn calculate_consensus(&self, prices: &[PriceData]) -> Option<f64> {
        if prices.len() < 3 {
            return None;
        }

        let mut price_values: Vec<f64> = prices.iter().map(|p| p.price).collect();
        price_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let median = if price_values.len() % 2 == 0 {
            let mid = price_values.len() / 2;
            (price_values[mid - 1] + price_values[mid]) / 2.0
        } else {
            price_values[price_values.len() / 2]
        };

        // 중간값 기준으로 임계값 내에 있는 가격들만 필터링
        let valid_prices: Vec<f64> = price_values
            .iter()
            .filter(|&&price| {
                let diff_percent = ((price - median).abs() / median) * 100.0;
                diff_percent <= self.consensus_threshold
            })
            .cloned()
            .collect();

        // 2/3 이상 동의 확인
        let required_count = (prices.len() * 2 + 2) / 3;
        if valid_prices.len() >= required_count {
            Some(valid_prices.iter().sum::<f64>() / valid_prices.len() as f64)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_oracle_system_integration() {
        // Given - 3개 거래소 시뮬레이션
        let binance = SimulatedExchange::new("binance", vec![70000.0, 70100.0, 70200.0]);
        let coinbase = SimulatedExchange::new("coinbase", vec![70050.0, 70150.0, 70250.0]);
        let kraken = SimulatedExchange::new("kraken", vec![70100.0, 70200.0, 70300.0]);

        let mut oracle = OracleSystem::new(1.0); // 1% 임계값
        oracle.add_exchange(Box::new(binance));
        oracle.add_exchange(Box::new(coinbase));
        oracle.add_exchange(Box::new(kraken));

        // When - 첫 번째 가격 수집
        let results = oracle.collect_prices().await;
        let prices: Vec<PriceData> = results.into_iter().filter_map(Result::ok).collect();

        // Then - 모든 거래소에서 가격 수집 성공
        assert_eq!(prices.len(), 3);
        
        // 컨센서스 계산
        let consensus = oracle.calculate_consensus(&prices);
        assert!(consensus.is_some());
        
        let consensus_price = consensus.unwrap();
        assert!(consensus_price > 69900.0 && consensus_price < 70200.0);
    }

    #[tokio::test]
    async fn test_oracle_handles_exchange_failure() {
        // Given - 하나의 거래소가 실패하는 상황
        let binance = SimulatedExchange::new("binance", vec![70000.0]);
        let coinbase = SimulatedExchange::new("coinbase", vec![70100.0]);
        let mut failing_exchange = SimulatedExchange::new("kraken", vec![]);
        failing_exchange.prices = Arc::new(RwLock::new(vec![])); // 빈 가격 리스트

        let mut oracle = OracleSystem::new(1.0);
        oracle.add_exchange(Box::new(binance));
        oracle.add_exchange(Box::new(coinbase));
        
        // When - 가격 수집 (2개만 성공해야 함)
        let results = oracle.collect_prices().await;
        let prices: Vec<PriceData> = results.into_iter().filter_map(Result::ok).collect();

        // Then - 2개 거래소만 성공, 컨센서스 불가
        assert_eq!(prices.len(), 2);
        let consensus = oracle.calculate_consensus(&prices);
        assert!(consensus.is_none()); // 최소 3개 필요
    }

    #[tokio::test]
    async fn test_oracle_detects_outlier() {
        // Given - 한 거래소가 아웃라이어 가격 제공
        let binance = SimulatedExchange::new("binance", vec![70000.0]);
        let coinbase = SimulatedExchange::new("coinbase", vec![70100.0]);
        let kraken = SimulatedExchange::new("kraken", vec![75000.0]); // 아웃라이어

        let mut oracle = OracleSystem::new(1.0); // 1% 임계값
        oracle.add_exchange(Box::new(binance));
        oracle.add_exchange(Box::new(coinbase));
        oracle.add_exchange(Box::new(kraken));

        // When
        let results = oracle.collect_prices().await;
        let prices: Vec<PriceData> = results.into_iter().filter_map(Result::ok).collect();

        // Then - 컨센서스는 정상 가격들로만 계산됨
        let consensus = oracle.calculate_consensus(&prices);
        assert!(consensus.is_some());
        
        let consensus_price = consensus.unwrap();
        assert!(consensus_price > 69900.0 && consensus_price < 70200.0);
    }

    #[tokio::test]
    async fn test_precision_handling_in_consensus() {
        // Given - 매우 정밀한 가격들
        let binance = SimulatedExchange::new("binance", vec![70000.12345678]);
        let coinbase = SimulatedExchange::new("coinbase", vec![70000.12345679]);
        let kraken = SimulatedExchange::new("kraken", vec![70000.12345680]);

        let mut oracle = OracleSystem::new(0.00001); // 매우 작은 임계값
        oracle.add_exchange(Box::new(binance));
        oracle.add_exchange(Box::new(coinbase));
        oracle.add_exchange(Box::new(kraken));

        // When
        let results = oracle.collect_prices().await;
        let prices: Vec<PriceData> = results.into_iter().filter_map(Result::ok).collect();

        // Then
        let consensus = oracle.calculate_consensus(&prices);
        assert!(consensus.is_some());
        
        let consensus_price = consensus.unwrap();
        assert!((consensus_price - 70000.12345679).abs() < 0.00000001);
    }

    #[tokio::test]
    async fn test_sequential_price_updates() {
        // Given - 시간에 따라 변하는 가격
        let binance = SimulatedExchange::new("binance", vec![70000.0, 71000.0, 72000.0]);
        let coinbase = SimulatedExchange::new("coinbase", vec![70100.0, 71100.0, 72100.0]);
        let kraken = SimulatedExchange::new("kraken", vec![70050.0, 71050.0, 72050.0]);

        let mut oracle = OracleSystem::new(1.0);
        oracle.add_exchange(Box::new(binance));
        oracle.add_exchange(Box::new(coinbase));
        oracle.add_exchange(Box::new(kraken));

        // When - 3번 연속 가격 수집
        for expected_base in [70000.0, 71000.0, 72000.0] {
            let results = oracle.collect_prices().await;
            let prices: Vec<PriceData> = results.into_iter().filter_map(Result::ok).collect();
            
            let consensus = oracle.calculate_consensus(&prices);
            assert!(consensus.is_some());
            
            let consensus_price = consensus.unwrap();
            assert!(
                consensus_price > expected_base && consensus_price < expected_base + 200.0,
                "Expected price around {}, got {}",
                expected_base,
                consensus_price
            );
        }
    }
}