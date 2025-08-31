//! Common types for Oracle VM

use bitcoin::PublicKey;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Option type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionType {
    Call,
    Put,
}

/// Asset pair identifier (e.g., "BTC/USD")
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetPair(pub String);

impl AssetPair {
    pub fn btc_usd() -> Self {
        Self("BTC/USD".to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Price data from an oracle source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub pair: AssetPair,
    pub price: u64, // Price in cents (for USD pairs)
    pub timestamp: DateTime<Utc>,
    pub volume: Option<u64>, // 24h volume
    pub source: String,      // Exchange name
}

/// Signed price data with oracle signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPriceData {
    pub data: PriceData,
    pub signature: Vec<u8>,
    pub oracle_pubkey: PublicKey,
}

/// Aggregated price from multiple sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedPrice {
    pub pair: AssetPair,
    pub median_price: u64,
    pub mean_price: u64,
    pub timestamp: DateTime<Utc>,
    pub sources: Vec<String>,
    pub confidence: f64, // 0.0 to 1.0
}

/// Oracle node identifier
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Network peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub node_id: NodeId,
    pub address: String,
    pub pubkey: PublicKey,
    pub last_seen: DateTime<Utc>,
}

/// Merkle tree root for price anchoring
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleRoot(pub [u8; 32]);

impl MerkleRoot {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Bitcoin transaction identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxId(pub [u8; 32]);

/// UTXO reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtxoRef {
    pub txid: TxId,
    pub vout: u32,
    pub amount: u64,
    pub address: String, // Address as string for serde compatibility
}
