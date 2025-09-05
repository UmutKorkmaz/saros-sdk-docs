//! Type definitions for the Saros DLMM SDK

use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use rust_decimal::Decimal;

/// Pool information (original DLMM pool info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLMMPoolInfo {
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

/// Swap parameters  
#[derive(Debug, Clone)]
pub struct SwapParams {
    pub pool_address: Pubkey,
    pub amount_in: Decimal,
    pub minimum_amount_out: Decimal,
    pub gas_price: Option<u64>,
    pub slippage_bps: Option<u16>,
}

/// Transaction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    pub signature: Signature,
    pub gas_used: u64,
    pub success: bool,
}

/// Pool account (aliased from DLMMPoolInfo for compatibility)
pub type PoolAccount = DLMMPoolInfo;

/// Token amount wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAmount {
    pub amount: Decimal,
    pub decimals: u8,
}

/// Swap result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResult {
    pub signature: String,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee: u64,
    pub price_impact: f64,
}

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

/// Position information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: Pubkey,
    pub owner: Pubkey,
    pub pool_address: Pubkey,
    pub lower_bin_id: i32,
    pub upper_bin_id: i32,
    pub liquidity: u128,
    pub amount: Decimal,
    pub unclaimed_fees_x: u64,
    pub unclaimed_fees_y: u64,
    pub value_usd: f64,
}

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

/// Quote result
#[derive(Debug, Clone)]
pub struct Quote {
    pub amount_out: u64,
    pub price_impact: f64,
    pub fee: u64,
    pub route: Vec<i32>, // bin IDs
}

/// Multi-hop swap parameters
#[derive(Debug, Clone)]
pub struct MultiHopSwapParams {
    pub route: Vec<Pubkey>, // Pool addresses
    pub amount_in: u64,
    pub minimum_amount_out: u64,
    pub slippage_bps: u16,
}

/// Position result after creation
#[derive(Debug, Clone)]
pub struct PositionResult {
    pub position_id: Pubkey,
    pub actual_amount_x: u64,
    pub actual_amount_y: u64,
    pub signature: String,
}

/// Add liquidity result
#[derive(Debug, Clone)]
pub struct AddLiquidityResult {
    pub amount_x_added: u64,
    pub amount_y_added: u64,
    pub signature: String,
}

/// Remove liquidity result
#[derive(Debug, Clone)]
pub struct RemoveLiquidityResult {
    pub amount_x_removed: u64,
    pub amount_y_removed: u64,
    pub fees_x_claimed: u64,
    pub fees_y_claimed: u64,
    pub signature: String,
}

/// Simulation result
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub amount_out: u64,
    pub price_impact: f64,
    pub fee: u64,
    pub success: bool,
}

/// Pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub tvl: f64,
    pub volume_24h: f64,
    pub fees_24h: f64,
    pub transactions_24h: u64,
    pub unique_traders_24h: u64,
    pub price_range_24h: (f64, f64),
}

/// Create pool parameters
#[derive(Debug, Clone)]
pub struct CreatePoolParams {
    pub token_x: Pubkey,
    pub token_y: Pubkey,
    pub bin_step: u16,
    pub base_factor: u16,
    pub initial_price: f64,
    pub activation_type: ActivationType,
}

/// Pool activation type
#[derive(Debug, Clone, Copy)]
pub enum ActivationType {
    Immediate,
    Delayed { slots: u64 },
    Manual,
}

/// Create pool result
#[derive(Debug, Clone)]
pub struct CreatePoolResult {
    pub pool_address: Pubkey,
    pub signature: String,
}

/// Claim fees result
#[derive(Debug, Clone)]
pub struct ClaimResult {
    pub fees_x_claimed: u64,
    pub fees_y_claimed: u64,
    pub signature: String,
}

/// Close position result
#[derive(Debug, Clone)]
pub struct CloseResult {
    pub final_amount_x: u64,
    pub final_amount_y: u64,
    pub total_fees_claimed: f64,
    pub signature: String,
}

/// User position for auto-compound system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPosition {
    pub pool_address: Pubkey,
    pub user_pubkey: Pubkey,
    pub token_a_amount: f64,
    pub token_b_amount: f64,
    pub lp_token_amount: f64,
    pub pending_rewards: f64,
}

/// Pool info for auto-compound system (different from regular PoolInfo)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_symbol: String,
    pub token_b_symbol: String,
    pub tvl: f64,
    pub apy: f64,
    pub fee_rate: f64,
    pub is_active: bool,
}

// Additional types for multi-hop routing

/// Mock pool data for multi-hop routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockPool {
    pub address: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub liquidity_usd: Decimal,
    pub fee_rate: Decimal,
    pub volume_24h: Option<Decimal>,
    pub active_bins: Option<u32>,
    pub bin_step: Option<u16>,
}

/// Mock token data for multi-hop routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockToken {
    pub mint: Pubkey,
    pub symbol: String,
    pub decimals: u8,
    pub price_usd: Option<Decimal>,
}