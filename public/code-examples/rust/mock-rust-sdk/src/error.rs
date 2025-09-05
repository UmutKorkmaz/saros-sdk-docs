//! Error types for the Saros DLMM SDK

use thiserror::Error;

/// SDK errors
#[derive(Debug, Error)]
pub enum DLMMError {
    #[error("RPC error: {0}")]
    RpcError(String),
    
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    
    #[error("Pool not found")]
    PoolNotFound,
    
    #[error("Insufficient liquidity")]
    InsufficientLiquidity,
    
    #[error("Slippage exceeded")]
    SlippageExceeded,
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Wallet not configured")]
    WalletNotConfigured,
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    
    #[error("Position not found")]
    PositionNotFound,
    
    #[error("Invalid bin range")]
    InvalidBinRange,
    
    #[error("Insufficient balance")]
    InsufficientBalance,
    
    #[error("Price oracle unavailable")]
    OracleUnavailable,
    
    #[error("Farm not found")]
    FarmNotFound,
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Convert from solana_client errors
impl From<solana_client::client_error::ClientError> for DLMMError {
    fn from(err: solana_client::client_error::ClientError) -> Self {
        DLMMError::RpcError(err.to_string())
    }
}

/// Convert from serde_json errors
impl From<serde_json::Error> for DLMMError {
    fn from(err: serde_json::Error) -> Self {
        DLMMError::DeserializationError(err.to_string())
    }
}