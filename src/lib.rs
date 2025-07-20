pub mod binance;
pub mod coinbase;
pub mod grpc_client;
pub mod kraken;
pub mod precision_test;
pub mod safe_price;

use anyhow::Result;
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct PriceData {
    pub price: f64,
    pub timestamp: u64,
    pub source: String,
}

/// 가격 제공자 인터페이스 (TDD를 위한 trait)
#[async_trait]
pub trait PriceProvider: Send + Sync {
    async fn fetch_price(&self, symbol: &str) -> Result<PriceData>;
}