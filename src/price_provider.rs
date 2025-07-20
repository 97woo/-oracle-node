use anyhow::Result;
use async_trait::async_trait;
use oracle_vm_common::types::PriceData;

/// Price provider trait for different exchanges
#[async_trait]
pub trait PriceProvider: Send + Sync {
    /// Fetch the current BTC price
    async fn fetch_btc_price(&self) -> Result<PriceData>;
    
    /// Get the name of the exchange
    fn name(&self) -> &str;
}

/// Multi-exchange price provider that can aggregate prices
pub struct MultiExchangePriceProvider {
    providers: Vec<Box<dyn PriceProvider>>,
}

impl MultiExchangePriceProvider {
    pub fn new(providers: Vec<Box<dyn PriceProvider>>) -> Self {
        Self { providers }
    }
    
    /// Fetch prices from all providers
    pub async fn fetch_all_prices(&self) -> Vec<(String, Result<PriceData>)> {
        let mut results = Vec::new();
        
        for provider in &self.providers {
            let name = provider.name().to_string();
            let result = provider.fetch_btc_price().await;
            results.push((name, result));
        }
        
        results
    }
    
    /// Fetch prices and return only successful ones
    pub async fn fetch_valid_prices(&self) -> Vec<PriceData> {
        let results = self.fetch_all_prices().await;
        
        results
            .into_iter()
            .filter_map(|(_, result)| result.ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::{mock, predicate::*};
    
    mock! {
        Provider {}
        
        #[async_trait]
        impl PriceProvider for Provider {
            async fn fetch_btc_price(&self) -> Result<PriceData>;
            fn name(&self) -> &str;
        }
    }
    
    #[tokio::test]
    async fn test_multi_exchange_fetches_all_prices() {
        // Given
        let mut mock1 = MockProvider::new();
        let mut mock2 = MockProvider::new();
        
        mock1.expect_name().return_const("Exchange1".to_string());
        mock1.expect_fetch_btc_price()
            .times(1)
            .returning(|| Ok(PriceData {
                price: 70000.0,
                timestamp: 1700000000,
                source: "Exchange1".to_string(),
            }));
            
        mock2.expect_name().return_const("Exchange2".to_string());
        mock2.expect_fetch_btc_price()
            .times(1)
            .returning(|| Ok(PriceData {
                price: 70100.0,
                timestamp: 1700000001,
                source: "Exchange2".to_string(),
            }));
        
        let provider = MultiExchangePriceProvider::new(vec![
            Box::new(mock1),
            Box::new(mock2),
        ]);
        
        // When
        let prices = provider.fetch_valid_prices().await;
        
        // Then
        assert_eq!(prices.len(), 2);
        assert_eq!(prices[0].price, 70000.0);
        assert_eq!(prices[1].price, 70100.0);
    }
    
    #[tokio::test]
    async fn test_multi_exchange_handles_failures() {
        // Given
        let mut mock1 = MockProvider::new();
        let mut mock2 = MockProvider::new();
        
        mock1.expect_name().return_const("Exchange1".to_string());
        mock1.expect_fetch_btc_price()
            .times(1)
            .returning(|| Err(anyhow::anyhow!("Network error")));
            
        mock2.expect_name().return_const("Exchange2".to_string());
        mock2.expect_fetch_btc_price()
            .times(1)
            .returning(|| Ok(PriceData {
                price: 70100.0,
                timestamp: 1700000001,
                source: "Exchange2".to_string(),
            }));
        
        let provider = MultiExchangePriceProvider::new(vec![
            Box::new(mock1),
            Box::new(mock2),
        ]);
        
        // When
        let prices = provider.fetch_valid_prices().await;
        
        // Then - Only successful price is returned
        assert_eq!(prices.len(), 1);
        assert_eq!(prices[0].price, 70100.0);
    }
}