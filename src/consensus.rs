use oracle_vm_common::types::PriceData;
use anyhow::Result;
use tracing::{info, warn};

/// 2/3 합의를 위한 ConsensusManager
pub struct ConsensusManager {
    /// 최소 합의 비율 (예: 0.67 = 2/3)
    min_consensus_ratio: f64,
    /// 가격 편차 허용 범위 (예: 0.02 = 2%)
    max_price_deviation: f64,
}

impl ConsensusManager {
    pub fn new() -> Self {
        Self {
            min_consensus_ratio: 0.66, // 2/3 (실제로는 0.666...)
            max_price_deviation: 0.02,  // 2%
        }
    }
    
    /// 여러 거래소의 가격 데이터를 받아서 합의된 가격을 반환
    pub fn get_consensus_price(&self, prices: Vec<PriceData>) -> Result<f64> {
        if prices.is_empty() {
            anyhow::bail!("No price data available");
        }
        
        // 가격만 추출 (cents를 다시 달러로 변환)
        let mut price_values: Vec<f64> = prices.iter().map(|p| p.price as f64 / 100.0).collect();
        price_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        // 중간값 계산
        let median = if price_values.len() % 2 == 0 {
            let mid = price_values.len() / 2;
            (price_values[mid - 1] + price_values[mid]) / 2.0
        } else {
            price_values[price_values.len() / 2]
        };
        
        // 중간값에서 허용 범위 내의 가격들만 필터링
        let valid_prices: Vec<f64> = price_values
            .into_iter()
            .filter(|&price| {
                let deviation = ((price - median) / median).abs();
                deviation <= self.max_price_deviation
            })
            .collect();
        
        // 2/3 이상이 유효한지 확인
        let consensus_count = valid_prices.len();
        let total_count = prices.len();
        let consensus_ratio = consensus_count as f64 / total_count as f64;
        
        if consensus_ratio < self.min_consensus_ratio {
            warn!(
                "Consensus not reached: {}/{} ({:.1}% < {:.1}% required)",
                consensus_count,
                total_count,
                consensus_ratio * 100.0,
                self.min_consensus_ratio * 100.0
            );
            anyhow::bail!("Consensus not reached");
        }
        
        // 유효한 가격들의 평균 반환
        let consensus_price = valid_prices.iter().sum::<f64>() / valid_prices.len() as f64;
        
        info!(
            "✅ Consensus reached: {}/{} exchanges agree on price ${:.2} (±{:.1}%)",
            consensus_count,
            total_count,
            consensus_price,
            self.max_price_deviation * 100.0
        );
        
        Ok(consensus_price)
    }
    
    /// 아웃라이어 감지
    pub fn detect_outliers(&self, prices: &[PriceData]) -> Vec<String> {
        if prices.len() < 3 {
            return vec![];
        }
        
        let mut price_values: Vec<f64> = prices.iter().map(|p| p.price as f64 / 100.0).collect();
        price_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let median = if price_values.len() % 2 == 0 {
            let mid = price_values.len() / 2;
            (price_values[mid - 1] + price_values[mid]) / 2.0
        } else {
            price_values[price_values.len() / 2]
        };
        
        prices
            .iter()
            .filter(|p| {
                let price_usd = p.price as f64 / 100.0;
                let deviation = ((price_usd - median) / median).abs();
                deviation > self.max_price_deviation
            })
            .map(|p| p.source.clone())
            .collect()
    }
}

impl Default for ConsensusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use oracle_vm_common::types::AssetPair;
    
    #[test]
    fn test_consensus_with_all_valid_prices() {
        let manager = ConsensusManager::new();
        
        let prices = vec![
            PriceData {
                pair: AssetPair::btc_usd(),
                price: 7000000, // $70,000 in cents
                timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
                volume: None,
                source: "binance".to_string(),
            },
            PriceData {
                pair: AssetPair::btc_usd(),
                price: 7010000, // $70,100 in cents
                timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
                volume: None,
                source: "coinbase".to_string(),
            },
            PriceData {
                pair: AssetPair::btc_usd(),
                price: 7005000, // $70,050 in cents
                timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
                volume: None,
                source: "kraken".to_string(),
            },
        ];
        
        let result = manager.get_consensus_price(prices);
        assert!(result.is_ok());
        
        let consensus_price = result.unwrap();
        assert!((consensus_price - 70050.0).abs() < 100.0);
    }
    
    #[test]
    fn test_consensus_with_outlier() {
        let manager = ConsensusManager::new();
        
        let prices = vec![
            PriceData {
                pair: AssetPair::btc_usd(),
                price: 7000000, // $70,000 in cents
                timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
                volume: None,
                source: "binance".to_string(),
            },
            PriceData {
                pair: AssetPair::btc_usd(),
                price: 7010000, // $70,100 in cents
                timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
                volume: None,
                source: "coinbase".to_string(),
            },
            PriceData {
                pair: AssetPair::btc_usd(),
                price: 7500000, // $75,000 in cents - Outlier (>7% deviation)
                timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
                volume: None,
                source: "kraken".to_string(),
            },
        ];
        
        // 중간값은 70100, 75000은 7.14% 편차로 2% 제한을 초과
        // 70000과 70100만 유효 (2/3 = 66.7%)
        let result = manager.get_consensus_price(prices.clone());
        
        // 디버깅을 위해 출력
        if result.is_err() {
            println!("Consensus failed: {:?}", result);
            let outliers = manager.detect_outliers(&prices);
            println!("Outliers detected: {:?}", outliers);
        }
        
        assert!(result.is_ok());
        
        let consensus_price = result.unwrap();
        assert!((consensus_price - 70050.0).abs() < 100.0);
    }
    
    #[test]
    fn test_consensus_failure() {
        let manager = ConsensusManager::new();
        
        let prices = vec![
            PriceData {
                pair: AssetPair::btc_usd(),
                price: 7000000, // $70,000 in cents
                timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
                volume: None,
                source: "binance".to_string(),
            },
            PriceData {
                pair: AssetPair::btc_usd(),
                price: 7500000, // $75,000 in cents - Too different
                timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
                volume: None,
                source: "coinbase".to_string(),
            },
            PriceData {
                pair: AssetPair::btc_usd(),
                price: 8000000, // $80,000 in cents - Too different
                timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
                volume: None,
                source: "kraken".to_string(),
            },
        ];
        
        let result = manager.get_consensus_price(prices);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_detect_outliers() {
        let manager = ConsensusManager::new();
        
        let prices = vec![
            PriceData {
                pair: AssetPair::btc_usd(),
                price: 7000000, // $70,000 in cents
                timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
                volume: None,
                source: "binance".to_string(),
            },
            PriceData {
                pair: AssetPair::btc_usd(),
                price: 7010000, // $70,100 in cents
                timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
                volume: None,
                source: "coinbase".to_string(),
            },
            PriceData {
                pair: AssetPair::btc_usd(),
                price: 7500000, // $75,000 in cents - Outlier
                timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
                volume: None,
                source: "kraken".to_string(),
            },
        ];
        
        let outliers = manager.detect_outliers(&prices);
        assert_eq!(outliers.len(), 1);
        assert_eq!(outliers[0], "kraken");
    }
}