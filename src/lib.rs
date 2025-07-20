pub mod binance;
pub mod coinbase;
pub mod grpc_client;
pub mod kraken;
pub mod precision_test;
pub mod safe_price;
pub mod price_provider;
pub mod consensus;

use anyhow::Result;
use async_trait::async_trait;

// common 모듈의 PriceData를 사용
pub use oracle_vm_common::types::PriceData;

/// 가격 제공자 인터페이스 (TDD를 위한 trait)
#[async_trait]
pub trait PriceProvider: Send + Sync {
    async fn fetch_price(&self, symbol: &str) -> Result<PriceData>;
}