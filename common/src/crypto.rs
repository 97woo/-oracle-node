//! Cryptographic utilities for Oracle VM

use crate::{OracleVmError, Result};
use bitcoin::secp256k1::{ecdsa::Signature, Message, PublicKey, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};
use rand::RngCore;

/// Sign data with a private key
pub fn sign_data(data: &[u8], secret_key: &SecretKey) -> Result<Signature> {
    let secp = Secp256k1::new();
    let hash = Sha256::digest(data);
    let message = Message::from_digest_slice(&hash)
        .map_err(|e| OracleVmError::Crypto(format!("Invalid message: {}", e)))?;

    Ok(secp.sign_ecdsa(&message, secret_key))
}

/// Verify signature
pub fn verify_signature(
    data: &[u8],
    signature: &Signature,
    public_key: &PublicKey,
) -> Result<bool> {
    let secp = Secp256k1::new();
    let hash = Sha256::digest(data);
    let message = Message::from_digest_slice(&hash)
        .map_err(|e| OracleVmError::Crypto(format!("Invalid message: {}", e)))?;

    match secp.verify_ecdsa(&message, signature, public_key) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Generate key pair
pub fn generate_keypair() -> (SecretKey, PublicKey) {
    let secp = Secp256k1::new();
    let mut rng = rand::thread_rng();
    let mut secret_bytes = [0u8; 32];
    rng.fill_bytes(&mut secret_bytes);
    let secret_key = SecretKey::from_slice(&secret_bytes).expect("32 bytes, within curve order");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    (secret_key, public_key)
}

/// Hash data with SHA256
pub fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

/// Merkle tree implementation
pub struct MerkleTree {
    leaves: Vec<[u8; 32]>,
}

impl MerkleTree {
    pub fn new(leaves: Vec<[u8; 32]>) -> Self {
        Self { leaves }
    }

    pub fn root(&self) -> [u8; 32] {
        if self.leaves.is_empty() {
            return [0u8; 32];
        }

        let mut current_level = self.leaves.clone();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in current_level.chunks(2) {
                let hash = if chunk.len() == 2 {
                    let mut hasher = Sha256::new();
                    hasher.update(chunk[0]);
                    hasher.update(chunk[1]);
                    hasher.finalize().into()
                } else {
                    chunk[0]
                };
                next_level.push(hash);
            }

            current_level = next_level;
        }

        current_level[0]
    }

    pub fn proof(&self, index: usize) -> Option<Vec<[u8; 32]>> {
        if index >= self.leaves.len() {
            return None;
        }

        let mut proof = Vec::new();
        let mut current_level = self.leaves.clone();
        let mut current_index = index;

        while current_level.len() > 1 {
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            if sibling_index < current_level.len() {
                proof.push(current_level[sibling_index]);
            }

            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                let hash = if chunk.len() == 2 {
                    let mut hasher = Sha256::new();
                    hasher.update(chunk[0]);
                    hasher.update(chunk[1]);
                    hasher.finalize().into()
                } else {
                    chunk[0]
                };
                next_level.push(hash);
            }

            current_level = next_level;
            current_index /= 2;
        }

        Some(proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let (secret_key, public_key) = generate_keypair();
        let data = b"test message";

        let signature = sign_data(data, &secret_key).unwrap();
        let is_valid = verify_signature(data, &signature, &public_key).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_merkle_tree() {
        let leaves = vec![
            sha256(b"leaf1"),
            sha256(b"leaf2"),
            sha256(b"leaf3"),
            sha256(b"leaf4"),
        ];

        let tree = MerkleTree::new(leaves.clone());
        let root = tree.root();

        // Test proof generation
        let proof = tree.proof(0).unwrap();
        assert!(!proof.is_empty());
    }
}
