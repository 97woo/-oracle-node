use anyhow::Result;
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, error};

mod binance;
mod aggregator_client;

use binance::BinanceClient;
use aggregator_client::AggregatorClient;

#[derive(Clone)]
pub struct PriceData {
    pub price: f64,
    pub timestamp: u64,
    pub source: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Oracle Node (PoC)...");

    // Create Binance client
    let binance_client = BinanceClient::new();
    
    // Create Aggregator client
    let aggregator_client = AggregatorClient::new("http://localhost:8081")?;
    
    // Check if Aggregator is healthy
    match aggregator_client.check_health().await {
        Ok(true) => info!("✅ Connected to Aggregator successfully"),
        Ok(false) => info!("⚠️ Aggregator is unhealthy, but continuing..."),
        Err(e) => {
            error!("❌ Cannot connect to Aggregator: {}", e);
            info!("💡 Make sure to run: python scripts/mock_aggregator.py");
            return Err(e);
        }
    }

    // Start price fetcher task
    let mut interval = interval(Duration::from_secs(10)); // 10초마다 (테스트용)
    
    // Fetch price immediately on startup
    info!("Fetching initial price...");
    
    loop {
        // Fetch price first, then wait
        interval.tick().await;
        
        match binance_client.fetch_btc_price().await {
            Ok(price_data) => {
                info!("Fetched BTC price: ${:.2} at timestamp: {}", 
                    price_data.price, 
                    price_data.timestamp
                );
                
                // Send to aggregator
                match aggregator_client.submit_price(&price_data).await {
                    Ok(_) => info!("✅ Successfully sent price to aggregator"),
                    Err(e) => error!("❌ Failed to send price to aggregator: {}", e),
                }
            }
            Err(e) => {
                error!("Failed to fetch price: {}", e);
            }
        }
    }
}