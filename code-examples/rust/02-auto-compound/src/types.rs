use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::{fmt, str::FromStr};

/// Auto-compound configuration
#[derive(Debug, Clone)]
pub struct AutoCompoundConfig {
    pub rpc_url: String,
    pub private_key: String,
    pub network: String,
    pub max_gas_price: f64,
    pub enable_notifications: bool,
    pub webhook_url: Option<String>,
}

/// Compound strategy configuration
#[derive(Debug, Clone)]
pub struct CompoundStrategyConfig {
    pub pool_address: Pubkey,
    pub strategy_type: StrategyType,
    pub interval_ms: u64,
    pub min_reward_threshold: f64,
    pub reinvest_percentage: u8,
    pub max_slippage: Option<f64>,
    pub emergency_withdraw: bool,
}

/// Strategy types for different compound operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyType {
    /// Liquidity Provider token compounding
    LP,
    /// Staking rewards compounding
    Staking,
    /// Farm rewards compounding
    Farming,
}

impl fmt::Display for StrategyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StrategyType::LP => write!(f, "LP"),
            StrategyType::Staking => write!(f, "STAKING"),
            StrategyType::Farming => write!(f, "FARMING"),
        }
    }
}

impl FromStr for StrategyType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "LP" => Ok(StrategyType::LP),
            "STAKING" => Ok(StrategyType::Staking),
            "FARMING" => Ok(StrategyType::Farming),
            _ => Err(anyhow::anyhow!("Invalid strategy type: {}", s)),
        }
    }
}

/// Result of a compound operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundResult {
    pub success: bool,
    pub rewards_harvested: f64,
    pub amount_reinvested: f64,
    pub new_position_value: f64,
    pub gas_used: f64,
    pub transaction_signature: String,
    pub timestamp: DateTime<Utc>,
    pub error: Option<String>,
}

/// Result of starting a strategy
#[derive(Debug, Clone)]
pub struct StartResult {
    pub success: bool,
    pub pool_address: String,
    pub strategy_type: String,
    pub interval_ms: u64,
    pub min_threshold: f64,
    pub next_compound_time: String,
    pub error: Option<String>,
}

/// Position information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub pool_address: Pubkey,
    pub token_a_amount: f64,
    pub token_b_amount: f64,
    pub lp_token_amount: f64,
    pub pending_rewards: f64,
    pub last_updated: DateTime<Utc>,
}

/// Pool information
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

/// Global statistics for all compound operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalStatistics {
    pub total_compounds: u64,
    pub successful_compounds: u64,
    pub failed_compounds: u64,
    pub success_rate: f64,
    pub total_rewards_harvested: f64,
    pub total_reinvested: f64,
    pub total_gas_spent: f64,
    pub net_profit: f64,
    pub average_apy_boost: f64,
    pub last_compound_time: Option<DateTime<Utc>>,
    pub uptime_hours: f64,
}

/// Statistics for a specific pool
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PoolStatistics {
    pub pool_address: Pubkey,
    pub compounds: u64,
    pub total_harvested: f64,
    pub total_reinvested: f64,
    pub total_gas: f64,
    pub last_compound: Option<DateTime<Utc>>,
    pub average_reward: f64,
    pub apy_before_compound: f64,
    pub apy_after_compound: f64,
    pub position_growth: f64,
}

/// Gas optimization result
#[derive(Debug, Clone)]
pub struct GasOptimizationResult {
    pub should_proceed: bool,
    pub recommended_gas_price: f64,
    pub estimated_gas_cost: f64,
    pub reason: String,
}

/// Notification event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEvent {
    pub event_type: NotificationEventType,
    pub pool_address: String,
    pub message: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

/// Types of notification events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationEventType {
    CompoundStarted,
    CompoundSuccess,
    CompoundFailed,
    CompoundStopped,
    HighGasPrice,
    LowRewards,
    EmergencyStop,
    PositionChanged,
    APYUpdate,
}

impl fmt::Display for NotificationEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotificationEventType::CompoundStarted => write!(f, "COMPOUND_STARTED"),
            NotificationEventType::CompoundSuccess => write!(f, "COMPOUND_SUCCESS"),
            NotificationEventType::CompoundFailed => write!(f, "COMPOUND_FAILED"),
            NotificationEventType::CompoundStopped => write!(f, "COMPOUND_STOPPED"),
            NotificationEventType::HighGasPrice => write!(f, "HIGH_GAS_PRICE"),
            NotificationEventType::LowRewards => write!(f, "LOW_REWARDS"),
            NotificationEventType::EmergencyStop => write!(f, "EMERGENCY_STOP"),
            NotificationEventType::PositionChanged => write!(f, "POSITION_CHANGED"),
            NotificationEventType::APYUpdate => write!(f, "APY_UPDATE"),
        }
    }
}