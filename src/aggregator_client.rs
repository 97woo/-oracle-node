use anyhow::{Result, Context};
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use tracing::{info, warn};
use crate::PriceData;

/// Aggregator에 전송할 가격 데이터 구조
#[derive(Serialize)]
struct PriceSubmission {
    price: f64,
    timestamp: u64,
    source: String,
    node_id: String,
}

/// Aggregator 응답 구조
#[derive(serde::Deserialize)]
struct AggregatorResponse {
    status: String,
    message: String,
    aggregated_price: Option<f64>,
}

/// Aggregator와 통신하는 클라이언트
pub struct AggregatorClient {
    client: Client,
    aggregator_url: String,
    node_id: String,
}

impl AggregatorClient {
    /// 새로운 Aggregator 클라이언트 생성
    pub fn new(aggregator_url: &str) -> Result<Self> {
        // Oracle Node 고유 ID 생성 (UUID 사용)
        let node_id = format!("oracle-node-{}", 
                             uuid::Uuid::new_v4().to_string()[..8].to_string());
        
        let client = Client::builder()
            .timeout(Duration::from_secs(5))  // 5초 타임아웃
            .user_agent("OracleNode/1.0")
            .build()
            .context("Failed to create HTTP client for Aggregator")?;
        
        info!("Created Aggregator client with node_id: {}", node_id);
        
        Ok(Self {
            client,
            aggregator_url: aggregator_url.to_string(),
            node_id,
        })
    }

    /// 가격 데이터를 Aggregator에 전송
    pub async fn submit_price(&self, price_data: &PriceData) -> Result<()> {
        let submission = PriceSubmission {
            price: price_data.price,
            timestamp: price_data.timestamp,
            source: price_data.source.clone(),
            node_id: self.node_id.clone(),
        };

        let url = format!("{}/submit-price", self.aggregator_url);
        
        info!("Sending price ${:.2} to Aggregator...", submission.price);
        
        let response = self.client
            .post(&url)
            .json(&submission)
            .send()
            .await
            .context("Failed to send price to Aggregator")?;
        
        if response.status().is_success() {
            // 성공 응답 파싱
            match response.json::<AggregatorResponse>().await {
                Ok(agg_response) => {
                    if let Some(aggregated_price) = agg_response.aggregated_price {
                        info!("✅ Price sent successfully! Aggregated price: ${:.2}", 
                              aggregated_price);
                    } else {
                        info!("✅ Price sent successfully! {}", agg_response.message);
                    }
                }
                Err(_) => {
                    info!("✅ Price sent successfully! (Could not parse response)");
                }
            }
        } else {
            // 에러 응답 처리
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            
            match status.as_u16() {
                400 => {
                    warn!("❌ Bad request to Aggregator: {}", error_text);
                    anyhow::bail!("Invalid price data sent to Aggregator");
                }
                429 => {
                    warn!("❌ Rate limited by Aggregator");
                    anyhow::bail!("Too many requests to Aggregator");
                }
                500..=599 => {
                    warn!("❌ Aggregator server error: {}", error_text);
                    anyhow::bail!("Aggregator server error");
                }
                _ => {
                    warn!("❌ Unexpected error from Aggregator: {} - {}", status, error_text);
                    anyhow::bail!("Aggregator communication error: {}", status);
                }
            }
        }
        
        Ok(())
    }

    /// Aggregator 상태 확인
    pub async fn check_health(&self) -> Result<bool> {
        let url = format!("{}/health", self.aggregator_url);
        
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                info!("✅ Aggregator is healthy");
                Ok(true)
            }
            Ok(response) => {
                warn!("❌ Aggregator health check failed: {}", response.status());
                Ok(false)
            }
            Err(e) => {
                warn!("❌ Cannot reach Aggregator: {}", e);
                Ok(false)
            }
        }
    }

    /// Node ID 반환
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}