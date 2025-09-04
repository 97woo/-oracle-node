use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::pin::Pin;
use tokio::sync::RwLock;
use tokio_stream::Stream;
use tonic::{transport::Server, Request, Response, Status};
use tracing::info;

// gRPC 서버 코드 (tonic-build로 자동 생성됨)
pub mod oracle {
    tonic::include_proto!("oracle");
}

use oracle::{
    oracle_service_server::{OracleService, OracleServiceServer},
    AggregatedPriceUpdate, ConfigRequest, ConfigResponse, GetPriceRequest, GetPriceResponse,
    HealthRequest, HealthResponse, PriceDataPoint, PriceRequest, PriceResponse,
};

// 가격 데이터 저장용 구조체
#[derive(Clone, Debug)]
struct PriceEntry {
    price: f64,
    timestamp: u64,
    source: String,
    node_id: String,
}

// Aggregator 서버 상태
struct AggregatorState {
    prices: Vec<PriceEntry>,
    active_nodes: HashMap<String, u64>, // node_id -> last_seen_timestamp
}

// Aggregator 서비스 구현
pub struct AggregatorServiceImpl {
    state: Arc<RwLock<AggregatorState>>,
}

impl AggregatorServiceImpl {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(AggregatorState {
                prices: Vec::new(),
                active_nodes: HashMap::new(),
            })),
        }
    }

    // 중간값(median) 계산
    async fn calculate_median_price(&self) -> Option<f64> {
        let state = self.state.read().await;
        let current_time = Utc::now().timestamp() as u64;
        
        // 최근 60초 이내의 가격 데이터만 사용
        let recent_prices: Vec<f64> = state
            .prices
            .iter()
            .filter(|p| current_time - p.timestamp < 60)
            .map(|p| p.price)
            .collect();

        if recent_prices.is_empty() {
            return None;
        }

        let mut sorted_prices = recent_prices.clone();
        sorted_prices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let len = sorted_prices.len();
        if len % 2 == 0 {
            Some((sorted_prices[len / 2 - 1] + sorted_prices[len / 2]) / 2.0)
        } else {
            Some(sorted_prices[len / 2])
        }
    }

    // 활성 노드 정리
    async fn cleanup_inactive_nodes(&self) {
        let mut state = self.state.write().await;
        let current_time = Utc::now().timestamp() as u64;
        
        // 120초 이상 응답 없는 노드 제거
        state.active_nodes.retain(|_, last_seen| {
            current_time - *last_seen < 120
        });
    }
}

#[tonic::async_trait]
impl OracleService for AggregatorServiceImpl {
    type StreamPricesStream = Pin<Box<dyn Stream<Item = Result<AggregatedPriceUpdate, Status>> + Send + 'static>>;
    async fn submit_price(
        &self,
        request: Request<PriceRequest>,
    ) -> Result<Response<PriceResponse>, Status> {
        let price_data = request.into_inner();
        
        info!(
            "📊 Received price: ${:.2} from {} ({})",
            price_data.price, price_data.node_id, price_data.source
        );

        let current_time = Utc::now().timestamp() as u64;
        
        // 가격 데이터 저장
        {
            let mut state = self.state.write().await;
            
            // 가격 추가
            state.prices.push(PriceEntry {
                price: price_data.price,
                timestamp: price_data.timestamp,
                source: price_data.source,
                node_id: price_data.node_id.clone(),
            });
            
            // 오래된 데이터 제거 (최대 100개 유지)
            if state.prices.len() > 100 {
                let drain_count = state.prices.len() - 100;
                state.prices.drain(0..drain_count);
            }
            
            // 활성 노드 업데이트
            state.active_nodes.insert(price_data.node_id, current_time);
        }

        // 비활성 노드 정리
        self.cleanup_inactive_nodes().await;

        // 중간값 계산
        let median_price = self.calculate_median_price().await;
        
        let response = PriceResponse {
            success: true,
            message: format!("Price received successfully"),
            aggregated_price: median_price,
            timestamp: current_time,
        };

        if let Some(price) = median_price {
            info!("💰 Current median price: ${:.2}", price);
        }

        Ok(Response::new(response))
    }

    async fn stream_prices(
        &self,
        _request: Request<tonic::Streaming<PriceRequest>>,
    ) -> Result<Response<Self::StreamPricesStream>, Status> {
        Err(Status::unimplemented("Stream prices not implemented"))
    }

    async fn health_check(
        &self,
        request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let req = request.into_inner();
        let state = self.state.read().await;
        
        info!("🏥 Health check from: {}", req.node_id);
        
        let response = HealthResponse {
            healthy: true,
            timestamp: Utc::now().timestamp() as u64,
            active_nodes: state.active_nodes.len() as u32,
            version: "1.0.0".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn update_config(
        &self,
        _request: Request<ConfigRequest>,
    ) -> Result<Response<ConfigResponse>, Status> {
        let response = ConfigResponse {
            success: true,
            message: "Config update not implemented".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn get_aggregated_price(
        &self,
        _request: Request<GetPriceRequest>,
    ) -> Result<Response<GetPriceResponse>, Status> {
        let state = self.state.read().await;
        let current_time = Utc::now().timestamp() as u64;
        
        // 최근 10개 가격 데이터
        let recent_prices: Vec<PriceDataPoint> = state
            .prices
            .iter()
            .rev()
            .take(10)
            .map(|p| PriceDataPoint {
                price: p.price,
                timestamp: p.timestamp,
                source: p.source.clone(),
                node_id: p.node_id.clone(),
            })
            .collect();

        let median_price = self.calculate_median_price().await.unwrap_or(0.0);
        
        let response = GetPriceResponse {
            success: true,
            aggregated_price: median_price,
            data_points: recent_prices.len() as u32,
            last_update: current_time,
            recent_prices,
        };

        Ok(Response::new(response))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 로깅 초기화
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting BTCFi Aggregator Server on port 50051");

    let addr = "127.0.0.1:50051".parse()?;
    let aggregator = AggregatorServiceImpl::new();

    info!("📡 Listening for Oracle Nodes at {}", addr);

    Server::builder()
        .add_service(OracleServiceServer::new(aggregator))
        .serve(addr)
        .await?;

    Ok(())
}