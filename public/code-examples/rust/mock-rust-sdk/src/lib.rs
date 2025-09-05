//! Mock Saros DLMM SDK for Rust Examples
//! 
//! This is a mock implementation of the Saros DLMM SDK for demonstration purposes.
//! In production, you would use the actual `saros-dlmm-sdk` crate.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod client;
pub mod types;
pub mod bin_math;
pub mod error;

pub use client::DLMMClient;
pub use types::*;
pub use error::DLMMError;

// Type aliases for multi-hop routing compatibility
pub type SarosClient = DLMMClient;

// Mock transaction builder
pub struct TransactionBuilder {
    instructions: Vec<solana_sdk::instruction::Instruction>,
    payer: Option<solana_sdk::pubkey::Pubkey>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            payer: None,
        }
    }
    
    pub fn build_transaction(
        &self, 
        _payer: solana_sdk::pubkey::Pubkey,
        _route_data: &[u8]
    ) -> Result<solana_sdk::transaction::Transaction, crate::error::DLMMError> {
        Ok(solana_sdk::transaction::Transaction::default())
    }
    
    pub fn build_priority_transaction(
        &self,
        _payer: solana_sdk::pubkey::Pubkey,
        _route_data: &[u8],
        _priority_fee: rust_decimal::Decimal,
    ) -> Result<solana_sdk::transaction::Transaction, crate::error::DLMMError> {
        Ok(solana_sdk::transaction::Transaction::default())
    }
}

/// Re-export common types for convenience
pub type DLMMResult<T> = std::result::Result<T, DLMMError>;

/// Mock price oracle for testing
static MOCK_PRICES: once_cell::sync::Lazy<Arc<RwLock<HashMap<String, f64>>>> =
    once_cell::sync::Lazy::new(|| {
        let mut prices = HashMap::new();
        prices.insert("SOL".to_string(), 110.50);
        prices.insert("USDC".to_string(), 1.0);
        prices.insert("ETH".to_string(), 3200.0);
        prices.insert("BTC".to_string(), 65000.0);
        prices.insert("RAY".to_string(), 2.45);
        prices.insert("SRM".to_string(), 0.85);
        Arc::new(RwLock::new(prices))
    });

/// Get mock price for a token
pub async fn get_mock_price(symbol: &str) -> f64 {
    let prices = MOCK_PRICES.read().await;
    prices.get(symbol).copied().unwrap_or(1.0)
}

/// Set mock price for testing
pub async fn set_mock_price(symbol: String, price: f64) {
    let mut prices = MOCK_PRICES.write().await;
    prices.insert(symbol, price);
}

/// Initialize the mock SDK (for testing)
pub fn init_mock() {
    // env_logger::init(); // Commented out as env_logger is not in dependencies
    log::info!("Mock Saros DLMM SDK initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_prices() {
        let sol_price = get_mock_price("SOL").await;
        assert_eq!(sol_price, 110.50);
        
        set_mock_price("SOL".to_string(), 120.0).await;
        let updated_price = get_mock_price("SOL").await;
        assert_eq!(updated_price, 120.0);
    }
}