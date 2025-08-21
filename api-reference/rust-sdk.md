# Rust SDK API Reference

## Overview

The Saros Rust SDK (`saros-dlmm-sdk-rs`) provides native Rust bindings for interacting with Saros Finance's DLMM protocol. Built for high-performance applications, on-chain programs, and systems requiring low-level control.

## Installation

```toml
[dependencies]
saros-dlmm-sdk = "0.2.0"
solana-sdk = "1.17"
anchor-lang = "0.29"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use saros_dlmm_sdk::{DLMMClient, SwapParams, PositionParams};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = DLMMClient::new("https://api.mainnet-beta.solana.com").await?;
    
    // Perform swap
    let swap_params = SwapParams {
        pool_address: Pubkey::from_str("...")?,
        amount_in: 1_000_000_000, // 1 SOL
        minimum_amount_out: 48_000_000, // 48 USDC
        slippage_bps: 50, // 0.5%
    };
    
    let result = client.swap(swap_params).await?;
    println!("Swap executed: {:?}", result);
    
    Ok(())
}
```

## Core Modules

### 1. DLMMClient

Main entry point for SDK operations.

```rust
pub struct DLMMClient {
    rpc_client: RpcClient,
    program_id: Pubkey,
    wallet: Option<Keypair>,
}

impl DLMMClient {
    /// Create new client instance
    pub async fn new(rpc_url: &str) -> Result<Self, DLMMError>;
    
    /// Create client with wallet
    pub async fn with_wallet(
        rpc_url: &str, 
        wallet: Keypair
    ) -> Result<Self, DLMMError>;
    
    /// Set wallet after initialization
    pub fn set_wallet(&mut self, wallet: Keypair);
    
    /// Get current network
    pub fn network(&self) -> Network;
}
```

### 2. Pool Operations

#### Get Pool Information

```rust
/// Pool information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: Pubkey,
    pub token_x: Pubkey,
    pub token_y: Pubkey,
    pub active_bin_id: i32,
    pub bin_step: u16,
    pub liquidity: u128,
    pub volume_24h: u128,
    pub fees_24h: u128,
    pub apr: f64,
}

impl DLMMClient {
    /// Get pool information
    pub async fn get_pool(&self, pool_address: Pubkey) -> Result<PoolInfo, DLMMError>;
    
    /// Get all pools
    pub async fn get_all_pools(&self) -> Result<Vec<PoolInfo>, DLMMError>;
    
    /// Find pool by token pair
    pub async fn find_pool(
        &self,
        token_a: Pubkey,
        token_b: Pubkey
    ) -> Result<Option<PoolInfo>, DLMMError>;
    
    /// Get pool statistics
    pub async fn get_pool_stats(
        &self,
        pool_address: Pubkey
    ) -> Result<PoolStats, DLMMError>;
}
```

#### Create Pool

```rust
/// Pool creation parameters
#[derive(Debug, Clone)]
pub struct CreatePoolParams {
    pub token_x: Pubkey,
    pub token_y: Pubkey,
    pub bin_step: u16,
    pub base_factor: u16,
    pub initial_price: f64,
    pub activation_type: ActivationType,
}

/// Activation type for new pools
#[derive(Debug, Clone, Copy)]
pub enum ActivationType {
    Immediate,
    Delayed { slots: u64 },
    Manual,
}

impl DLMMClient {
    /// Create new DLMM pool
    pub async fn create_pool(
        &self,
        params: CreatePoolParams
    ) -> Result<CreatePoolResult, DLMMError>;
}
```

### 3. Swap Operations

#### Basic Swap

```rust
/// Swap parameters
#[derive(Debug, Clone)]
pub struct SwapParams {
    pub pool_address: Pubkey,
    pub amount_in: u64,
    pub minimum_amount_out: u64,
    pub slippage_bps: u16,
}

/// Swap result
#[derive(Debug, Clone)]
pub struct SwapResult {
    pub signature: String,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee: u64,
    pub price_impact: f64,
}

impl DLMMClient {
    /// Execute swap
    pub async fn swap(&self, params: SwapParams) -> Result<SwapResult, DLMMError>;
    
    /// Simulate swap without execution
    pub async fn simulate_swap(
        &self, 
        params: SwapParams
    ) -> Result<SimulationResult, DLMMError>;
    
    /// Get swap quote
    pub async fn get_quote(
        &self,
        pool_address: Pubkey,
        amount_in: u64,
        is_x_to_y: bool
    ) -> Result<Quote, DLMMError>;
}
```

#### Advanced Swap Features

```rust
/// Multi-hop swap parameters
#[derive(Debug, Clone)]
pub struct MultiHopSwapParams {
    pub route: Vec<Pubkey>, // Pool addresses
    pub amount_in: u64,
    pub minimum_amount_out: u64,
    pub slippage_bps: u16,
}

/// Split swap parameters
#[derive(Debug, Clone)]
pub struct SplitSwapParams {
    pub routes: Vec<SwapRoute>,
    pub splits: Vec<u8>, // Percentage for each route
    pub total_amount_in: u64,
    pub minimum_total_out: u64,
}

impl DLMMClient {
    /// Execute multi-hop swap
    pub async fn multi_hop_swap(
        &self,
        params: MultiHopSwapParams
    ) -> Result<SwapResult, DLMMError>;
    
    /// Execute split swap
    pub async fn split_swap(
        &self,
        params: SplitSwapParams
    ) -> Result<SwapResult, DLMMError>;
}
```

### 4. Position Management

#### Create Position

```rust
/// Position parameters
#[derive(Debug, Clone)]
pub struct PositionParams {
    pub pool_address: Pubkey,
    pub lower_bin_id: i32,
    pub upper_bin_id: i32,
    pub liquidity_distribution: LiquidityDistribution,
    pub total_amount_x: u64,
    pub total_amount_y: u64,
}

/// Liquidity distribution strategies
#[derive(Debug, Clone)]
pub enum LiquidityDistribution {
    Uniform,
    Normal { mean: i32, std_dev: f64 },
    Exponential { lambda: f64 },
    Custom(Vec<(i32, u128)>), // (bin_id, liquidity)
}

impl DLMMClient {
    /// Create new position
    pub async fn create_position(
        &self,
        params: PositionParams
    ) -> Result<PositionResult, DLMMError>;
    
    /// Add liquidity to existing position
    pub async fn add_liquidity(
        &self,
        position_id: Pubkey,
        amount_x: u64,
        amount_y: u64
    ) -> Result<AddLiquidityResult, DLMMError>;
    
    /// Remove liquidity from position
    pub async fn remove_liquidity(
        &self,
        position_id: Pubkey,
        liquidity_amount: u128,
        min_amount_x: u64,
        min_amount_y: u64
    ) -> Result<RemoveLiquidityResult, DLMMError>;
}
```

#### Position Queries

```rust
/// Position information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: Pubkey,
    pub owner: Pubkey,
    pub pool_address: Pubkey,
    pub lower_bin_id: i32,
    pub upper_bin_id: i32,
    pub liquidity: u128,
    pub unclaimed_fees_x: u64,
    pub unclaimed_fees_y: u64,
    pub value_usd: f64,
}

impl DLMMClient {
    /// Get position by ID
    pub async fn get_position(
        &self,
        position_id: Pubkey
    ) -> Result<Position, DLMMError>;
    
    /// Get all positions for wallet
    pub async fn get_user_positions(
        &self,
        owner: Pubkey
    ) -> Result<Vec<Position>, DLMMError>;
    
    /// Claim fees from position
    pub async fn claim_fees(
        &self,
        position_id: Pubkey
    ) -> Result<ClaimResult, DLMMError>;
    
    /// Close position
    pub async fn close_position(
        &self,
        position_id: Pubkey
    ) -> Result<CloseResult, DLMMError>;
}
```

### 5. Bin Operations

#### Bin Management

```rust
/// Bin information
#[derive(Debug, Clone)]
pub struct BinInfo {
    pub id: i32,
    pub price: f64,
    pub liquidity_x: u64,
    pub liquidity_y: u64,
    pub total_liquidity: u128,
    pub fee_rate: u16,
}

impl DLMMClient {
    /// Get bin information
    pub async fn get_bin(
        &self,
        pool_address: Pubkey,
        bin_id: i32
    ) -> Result<BinInfo, DLMMError>;
    
    /// Get active bin
    pub async fn get_active_bin(
        &self,
        pool_address: Pubkey
    ) -> Result<BinInfo, DLMMError>;
    
    /// Get bins in range
    pub async fn get_bins_in_range(
        &self,
        pool_address: Pubkey,
        lower_bin_id: i32,
        upper_bin_id: i32
    ) -> Result<Vec<BinInfo>, DLMMError>;
}
```

#### Bin Calculations

```rust
/// Bin math utilities
pub mod bin_math {
    /// Convert price to bin ID
    pub fn price_to_bin_id(
        price: f64,
        bin_step: u16,
        decimals_diff: i8
    ) -> i32;
    
    /// Convert bin ID to price
    pub fn bin_id_to_price(
        bin_id: i32,
        bin_step: u16,
        decimals_diff: i8
    ) -> f64;
    
    /// Calculate price impact
    pub fn calculate_price_impact(
        amount_in: u64,
        bin_liquidity: u128,
        bin_step: u16
    ) -> f64;
    
    /// Get composition at bin
    pub fn get_bin_composition(
        bin_id: i32,
        active_bin_id: i32
    ) -> (f64, f64); // (x_percentage, y_percentage)
}
```

### 6. Oracle Integration

```rust
/// Oracle price feed
#[derive(Debug, Clone)]
pub struct OraclePrice {
    pub price: f64,
    pub confidence: f64,
    pub timestamp: i64,
    pub source: OracleSource,
}

/// Oracle sources
#[derive(Debug, Clone)]
pub enum OracleSource {
    Pyth,
    Switchboard,
    Internal,
}

impl DLMMClient {
    /// Get oracle price
    pub async fn get_oracle_price(
        &self,
        token: Pubkey
    ) -> Result<OraclePrice, DLMMError>;
    
    /// Subscribe to price updates
    pub async fn subscribe_price_updates<F>(
        &self,
        token: Pubkey,
        callback: F
    ) -> Result<SubscriptionId, DLMMError>
    where
        F: Fn(OraclePrice) + Send + 'static;
}
```

### 7. Farming & Staking

```rust
/// Farm information
#[derive(Debug, Clone)]
pub struct FarmInfo {
    pub id: Pubkey,
    pub pool_address: Pubkey,
    pub reward_token: Pubkey,
    pub reward_per_second: u64,
    pub total_staked: u128,
    pub apr: f64,
}

/// Staking parameters
#[derive(Debug, Clone)]
pub struct StakeParams {
    pub farm_id: Pubkey,
    pub position_id: Pubkey,
    pub amount: u128,
}

impl DLMMClient {
    /// Get farm information
    pub async fn get_farm(
        &self,
        farm_id: Pubkey
    ) -> Result<FarmInfo, DLMMError>;
    
    /// Stake position in farm
    pub async fn stake(
        &self,
        params: StakeParams
    ) -> Result<StakeResult, DLMMError>;
    
    /// Unstake position
    pub async fn unstake(
        &self,
        farm_id: Pubkey,
        position_id: Pubkey
    ) -> Result<UnstakeResult, DLMMError>;
    
    /// Harvest rewards
    pub async fn harvest(
        &self,
        farm_id: Pubkey,
        position_id: Pubkey
    ) -> Result<HarvestResult, DLMMError>;
}
```

### 8. Event Streaming

```rust
/// Event types
#[derive(Debug, Clone)]
pub enum DLMMEvent {
    Swap(SwapEvent),
    AddLiquidity(LiquidityEvent),
    RemoveLiquidity(LiquidityEvent),
    PositionUpdate(PositionEvent),
    PriceUpdate(PriceEvent),
}

/// Event subscription
impl DLMMClient {
    /// Subscribe to pool events
    pub async fn subscribe_pool_events<F>(
        &self,
        pool_address: Pubkey,
        callback: F
    ) -> Result<SubscriptionId, DLMMError>
    where
        F: Fn(DLMMEvent) + Send + 'static;
    
    /// Subscribe to all events
    pub async fn subscribe_all_events<F>(
        &self,
        callback: F
    ) -> Result<SubscriptionId, DLMMError>
    where
        F: Fn(DLMMEvent) + Send + 'static;
    
    /// Unsubscribe from events
    pub async fn unsubscribe(
        &self,
        subscription_id: SubscriptionId
    ) -> Result<(), DLMMError>;
}
```

### 9. Transaction Building

```rust
use solana_sdk::transaction::Transaction;
use solana_sdk::instruction::Instruction;

/// Transaction builder
pub struct TransactionBuilder {
    instructions: Vec<Instruction>,
    signers: Vec<Keypair>,
}

impl TransactionBuilder {
    /// Create new builder
    pub fn new() -> Self;
    
    /// Add instruction
    pub fn add_instruction(mut self, instruction: Instruction) -> Self;
    
    /// Add signer
    pub fn add_signer(mut self, signer: Keypair) -> Self;
    
    /// Set compute units
    pub fn with_compute_units(mut self, units: u32) -> Self;
    
    /// Set priority fee
    pub fn with_priority_fee(mut self, lamports: u64) -> Self;
    
    /// Build transaction
    pub async fn build(
        self,
        client: &DLMMClient
    ) -> Result<Transaction, DLMMError>;
    
    /// Build and send transaction
    pub async fn build_and_send(
        self,
        client: &DLMMClient
    ) -> Result<String, DLMMError>;
}
```

### 10. Error Handling

```rust
/// SDK errors
#[derive(Debug, thiserror::Error)]
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
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, DLMMError>;
```

## Advanced Features

### 1. Batch Operations

```rust
/// Batch swap parameters
#[derive(Debug, Clone)]
pub struct BatchSwapParams {
    pub swaps: Vec<SwapParams>,
    pub atomic: bool, // All or nothing
}

impl DLMMClient {
    /// Execute batch swaps
    pub async fn batch_swap(
        &self,
        params: BatchSwapParams
    ) -> Result<Vec<SwapResult>, DLMMError>;
    
    /// Batch position operations
    pub async fn batch_position_ops(
        &self,
        operations: Vec<PositionOperation>
    ) -> Result<Vec<OperationResult>, DLMMError>;
}
```

### 2. MEV Protection

```rust
/// MEV protection options
#[derive(Debug, Clone)]
pub struct MevProtection {
    pub use_jito: bool,
    pub tip_amount: u64,
    pub max_retries: u8,
    pub backrun_protection: bool,
}

impl DLMMClient {
    /// Execute swap with MEV protection
    pub async fn protected_swap(
        &self,
        params: SwapParams,
        protection: MevProtection
    ) -> Result<SwapResult, DLMMError>;
}
```

### 3. Simulation & Testing

```rust
/// Simulation context
pub struct SimulationContext {
    pub block_height: u64,
    pub timestamp: i64,
    pub mock_prices: HashMap<Pubkey, f64>,
}

impl DLMMClient {
    /// Create simulation client
    pub fn simulation_mode() -> Self;
    
    /// Run simulation
    pub async fn simulate<F, T>(
        &self,
        context: SimulationContext,
        operation: F
    ) -> Result<T, DLMMError>
    where
        F: FnOnce(&DLMMClient) -> Result<T, DLMMError>;
}
```

### 4. Analytics

```rust
/// Pool analytics
#[derive(Debug, Clone, Serialize)]
pub struct PoolAnalytics {
    pub tvl: f64,
    pub volume_24h: f64,
    pub fees_24h: f64,
    pub apr: f64,
    pub price_range_24h: (f64, f64),
    pub transactions_24h: u64,
    pub unique_traders_24h: u64,
}

/// Position analytics
#[derive(Debug, Clone, Serialize)]
pub struct PositionAnalytics {
    pub pnl: f64,
    pub pnl_percentage: f64,
    pub fees_earned: f64,
    pub impermanent_loss: f64,
    pub days_active: u64,
    pub apr: f64,
}

impl DLMMClient {
    /// Get pool analytics
    pub async fn get_pool_analytics(
        &self,
        pool_address: Pubkey
    ) -> Result<PoolAnalytics, DLMMError>;
    
    /// Get position analytics
    pub async fn get_position_analytics(
        &self,
        position_id: Pubkey
    ) -> Result<PositionAnalytics, DLMMError>;
}
```

## Utilities

### Constants

```rust
pub mod constants {
    use solana_sdk::pubkey::Pubkey;
    
    /// DLMM program ID
    pub const DLMM_PROGRAM_ID: Pubkey = pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");
    
    /// Maximum bin ID
    pub const MAX_BIN_ID: i32 = 443636;
    
    /// Minimum bin ID
    pub const MIN_BIN_ID: i32 = -443636;
    
    /// Basis points
    pub const BPS: u64 = 10000;
    
    /// Maximum fee rate (in BPS)
    pub const MAX_FEE_RATE: u16 = 10000; // 100%
}
```

### Type Conversions

```rust
pub mod conversions {
    /// Convert lamports to SOL
    pub fn lamports_to_sol(lamports: u64) -> f64 {
        lamports as f64 / 1e9
    }
    
    /// Convert SOL to lamports
    pub fn sol_to_lamports(sol: f64) -> u64 {
        (sol * 1e9) as u64
    }
    
    /// Convert basis points to percentage
    pub fn bps_to_percentage(bps: u16) -> f64 {
        bps as f64 / 100.0
    }
    
    /// Convert percentage to basis points
    pub fn percentage_to_bps(percentage: f64) -> u16 {
        (percentage * 100.0) as u16
    }
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_swap() {
        let client = DLMMClient::simulation_mode();
        
        let params = SwapParams {
            pool_address: Pubkey::new_unique(),
            amount_in: 1_000_000_000,
            minimum_amount_out: 48_000_000,
            slippage_bps: 50,
        };
        
        let result = client.swap(params).await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_bin_math() {
        let price = 50.0;
        let bin_step = 20; // 0.2%
        let bin_id = bin_math::price_to_bin_id(price, bin_step, 0);
        let recovered_price = bin_math::bin_id_to_price(bin_id, bin_step, 0);
        
        assert!((price - recovered_price).abs() < 0.01);
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // Run with --ignored flag
    async fn test_mainnet_swap() {
        let client = DLMMClient::new("https://api.mainnet-beta.solana.com")
            .await
            .unwrap();
        
        // Test with real mainnet pool
        let pool = client.get_pool(
            Pubkey::from_str("...").unwrap()
        ).await.unwrap();
        
        assert!(pool.liquidity > 0);
    }
}
```

## Migration Guide

### From TypeScript to Rust

```rust
// TypeScript: const client = new DLMMClient(...)
// Rust:
let client = DLMMClient::new("...").await?;

// TypeScript: await client.swap({...})
// Rust:
let result = client.swap(SwapParams { ... }).await?;

// TypeScript: client.on('swap', callback)
// Rust:
client.subscribe_pool_events(pool, |event| {
    match event {
        DLMMEvent::Swap(e) => println!("Swap: {:?}", e),
        _ => {}
    }
}).await?;
```

## Performance Optimization

### Connection Pooling

```rust
use std::sync::Arc;

/// Connection pool for better performance
pub struct ConnectionPool {
    connections: Vec<Arc<RpcClient>>,
    current: AtomicUsize,
}

impl ConnectionPool {
    pub fn new(urls: Vec<String>, size: usize) -> Self {
        // Implementation
    }
    
    pub fn get(&self) -> Arc<RpcClient> {
        // Round-robin selection
    }
}
```

### Caching

```rust
use lru::LruCache;
use std::sync::Mutex;

/// Cache for pool data
pub struct PoolCache {
    cache: Arc<Mutex<LruCache<Pubkey, PoolInfo>>>,
    ttl: Duration,
}

impl PoolCache {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        // Implementation
    }
    
    pub async fn get_or_fetch<F>(
        &self,
        key: Pubkey,
        fetch: F
    ) -> Result<PoolInfo, DLMMError>
    where
        F: Future<Output = Result<PoolInfo, DLMMError>>;
}
```

## Best Practices

1. **Error Handling**: Always use proper error handling with `?` operator
2. **Resource Management**: Use `Arc` for shared resources
3. **Async/Await**: Leverage Tokio for concurrent operations
4. **Testing**: Write comprehensive unit and integration tests
5. **Documentation**: Use rustdoc comments for all public APIs

## Resources

- [Crates.io Package](https://crates.io/crates/saros-dlmm-sdk)
- [GitHub Repository](https://github.com/saros-finance/saros-dlmm-sdk-rs)
- [API Documentation](https://docs.rs/saros-dlmm-sdk)
- [Examples](https://github.com/saros-finance/saros-dlmm-sdk-rs/tree/main/examples)