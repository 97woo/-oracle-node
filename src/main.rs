use anyhow::Result;
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, error};

mod binance;
mod aggregator_client;
mod grpc_client;

use binance::BinanceClient;
use aggregator_client::AggregatorClient;
use grpc_client::GrpcAggregatorClient;

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
    
    // Create gRPC Aggregator client (기본값)
    let mut grpc_client = GrpcAggregatorClient::new("http://localhost:50051").await?;
    
    // Check if gRPC Aggregator is healthy
    match grpc_client.check_health().await {
        Ok(true) => info!("✅ Connected to gRPC Aggregator successfully"),
        Ok(false) => info!("⚠️ gRPC Aggregator is unhealthy, but continuing..."),
        Err(e) => {
            error!("❌ Cannot connect to gRPC Aggregator: {}", e);
            info!("💡 Make sure to run: cargo run -p aggregator");
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
                
                // Send to gRPC aggregator
                match grpc_client.submit_price(&price_data).await {
                    Ok(_) => info!("✅ Successfully sent price to gRPC aggregator"),
                    Err(e) => error!("❌ Failed to send price to gRPC aggregator: {}", e),
                }
            }
            Err(e) => {
                error!("Failed to fetch price: {}", e);
            }
        }
    }
}