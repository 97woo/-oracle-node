use anyhow::{Result, Context};
use tonic::transport::Channel;
use tonic::Request;
use tracing::{info, warn, error};
use crate::PriceData;

// gRPC 클라이언트 코드 (tonic-build로 자동 생성됨)
pub mod oracle {
    tonic::include_proto!("oracle");
}

use oracle::{
    oracle_service_client::OracleServiceClient,
    PriceRequest, HealthRequest,
};

/// gRPC를 사용한 Aggregator 클라이언트
pub struct GrpcAggregatorClient {
    client: OracleServiceClient<Channel>,
    node_id: String,
}

impl GrpcAggregatorClient {
    /// 새로운 gRPC Aggregator 클라이언트 생성
    pub async fn new(aggregator_url: &str) -> Result<Self> {
        // Oracle Node 고유 ID 생성
        let node_id = format!("oracle-node-{}", 
                             uuid::Uuid::new_v4().to_string()[..8].to_string());
        
        // gRPC 채널 생성
        let channel = Channel::from_shared(aggregator_url.to_string())
            .context("Invalid aggregator URL")?
            .connect()
            .await
            .context("Failed to connect to Aggregator via gRPC")?;
        
        let client = OracleServiceClient::new(channel);
        
        info!("🔗 Created gRPC Aggregator client with node_id: {}", node_id);
        
        Ok(Self {
            client,
            node_id,
        })
    }
    
    /// 가격 데이터를 gRPC로 Aggregator에 전송
    pub async fn submit_price(&mut self, price_data: &PriceData) -> Result<()> {
        let request = Request::new(PriceRequest {
            price: price_data.price,
            timestamp: price_data.timestamp,
            source: price_data.source.clone(),
            node_id: self.node_id.clone(),
            signature: None, // 나중에 보안용으로 추가
        });
        
        info!("📤 Sending price ${:.2} to Aggregator via gRPC...", price_data.price);
        
        match self.client.submit_price(request).await {
            Ok(response) => {
                let response = response.into_inner();
                if response.success {
                    if let Some(aggregated_price) = response.aggregated_price {
                        info!("✅ gRPC: Price sent successfully! Aggregated price: ${:.2}", 
                              aggregated_price);
                    } else {
                        info!("✅ gRPC: Price sent successfully! {}", response.message);
                    }
                } else {
                    warn!("❌ gRPC: Failed to submit price: {}", response.message);
                    anyhow::bail!("Aggregator rejected price: {}", response.message);
                }
            }
            Err(e) => {
                error!("❌ gRPC: Failed to send price: {}", e);
                anyhow::bail!("gRPC communication error: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// gRPC를 통한 Aggregator 헬스체크
    pub async fn check_health(&mut self) -> Result<bool> {
        let request = Request::new(HealthRequest {
            node_id: self.node_id.clone(),
        });
        
        match self.client.health_check(request).await {
            Ok(response) => {
                let response = response.into_inner();
                if response.healthy {
                    info!("✅ gRPC: Aggregator is healthy (active nodes: {})", 
                          response.active_nodes);
                    Ok(true)
                } else {
                    warn!("❌ gRPC: Aggregator is unhealthy");
                    Ok(false)
                }
            }
            Err(e) => {
                warn!("❌ gRPC: Cannot reach Aggregator: {}", e);
                Ok(false)
            }
        }
    }
    
    /// Node ID 반환
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // 실제 gRPC 서버 필요
    async fn test_grpc_connection() {
        let result = GrpcAggregatorClient::new("http://localhost:50051").await;
        // 연결 테스트는 서버가 실행 중일 때만 가능
        match result {
            Ok(_) => println!("gRPC connection successful"),
            Err(e) => println!("gRPC connection failed (expected): {}", e),
        }
    }
}