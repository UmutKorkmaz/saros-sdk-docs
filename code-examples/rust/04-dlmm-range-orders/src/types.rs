//! Type definitions for DLMM Range Orders Trading System

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::BTreeMap;
use uuid::Uuid;

/// Order status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order is pending execution
    Pending,
    /// Order is partially filled
    PartiallyFilled,
    /// Order is completely filled
    Filled,
    /// Order has been cancelled
    Cancelled,
    /// Order failed due to error
    Failed,
    /// Order is waiting for optimal conditions
    Waiting,
}

/// Order type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    /// Limit buy order at specific bin
    LimitBuy,
    /// Limit sell order at specific bin
    LimitSell,
    /// Take profit order
    TakeProfit,
    /// Stop loss order
    StopLoss,
    /// DCA ladder buy orders
    DcaLadder,
    /// Grid trading orders
    GridTrading,
}

/// Range order definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeOrder {
    /// Unique order ID
    pub id: Uuid,
    /// Pool address
    pub pool_address: Pubkey,
    /// Order type
    pub order_type: OrderType,
    /// Target bin ID for order execution
    pub bin_id: i32,
    /// Amount to trade (in tokens)
    pub amount: Decimal,
    /// Price at target bin
    pub target_price: Decimal,
    /// Current status
    pub status: OrderStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Filled amount
    pub filled_amount: Decimal,
    /// Average fill price
    pub avg_fill_price: Option<Decimal>,
    /// Associated position ID (if any)
    pub position_id: Option<Pubkey>,
    /// Order expiry (optional)
    pub expires_at: Option<DateTime<Utc>>,
    /// Maximum slippage tolerance (in basis points)
    pub max_slippage_bps: u16,
    /// Parent strategy ID (for grouped orders)
    pub strategy_id: Option<Uuid>,
}

/// DCA (Dollar Cost Averaging) ladder configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaLadderConfig {
    /// Total amount to distribute across orders
    pub total_amount: Decimal,
    /// Number of orders in the ladder
    pub order_count: u32,
    /// Starting bin ID (lowest price)
    pub start_bin_id: i32,
    /// Ending bin ID (highest price)
    pub end_bin_id: i32,
    /// Distribution type
    pub distribution: LadderDistribution,
    /// Time interval between order activations (optional)
    pub time_interval: Option<chrono::Duration>,
    /// Maximum orders to execute per period
    pub max_orders_per_period: Option<u32>,
}

/// Distribution types for ladder strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LadderDistribution {
    /// Equal amounts across all orders
    Uniform,
    /// More weight on lower prices (buy side)
    Weighted { bias: f64 },
    /// Fibonacci-based distribution
    Fibonacci,
    /// Custom distribution
    Custom(Vec<Decimal>),
}

/// Grid trading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    /// Center price for the grid
    pub center_price: Decimal,
    /// Grid spacing in basis points
    pub grid_spacing_bps: u16,
    /// Number of buy orders below center
    pub buy_orders_count: u32,
    /// Number of sell orders above center
    pub sell_orders_count: u32,
    /// Order amount for each grid level
    pub order_amount: Decimal,
    /// Take profit percentage for each trade
    pub take_profit_pct: Decimal,
    /// Rebalancing threshold
    pub rebalance_threshold_pct: Decimal,
}

/// Take profit / Stop loss configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpSlConfig {
    /// Take profit price
    pub take_profit_price: Option<Decimal>,
    /// Stop loss price
    pub stop_loss_price: Option<Decimal>,
    /// Trailing stop percentage
    pub trailing_stop_pct: Option<Decimal>,
    /// Position size to close (percentage)
    pub close_percentage: Decimal,
}

/// Order execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderExecution {
    /// Order ID
    pub order_id: Uuid,
    /// Transaction signature
    pub signature: String,
    /// Executed amount
    pub executed_amount: Decimal,
    /// Execution price
    pub execution_price: Decimal,
    /// Execution timestamp
    pub executed_at: DateTime<Utc>,
    /// Gas fees paid
    pub gas_fee: u64,
    /// Slippage experienced (in basis points)
    pub slippage_bps: u16,
}

/// Trading strategy enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradingStrategy {
    /// Single limit order
    LimitOrder {
        order_type: OrderType,
        bin_id: i32,
        amount: Decimal,
        expires_at: Option<DateTime<Utc>>,
    },
    /// DCA ladder strategy
    DcaLadder(DcaLadderConfig),
    /// Grid trading strategy
    GridTrading(GridConfig),
    /// Take profit / Stop loss strategy
    TakeProfit(TpSlConfig),
    /// Combined strategy
    Combined {
        strategies: Vec<TradingStrategy>,
        execution_order: ExecutionOrder,
    },
}

/// Execution order for combined strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionOrder {
    /// Execute strategies in parallel
    Parallel,
    /// Execute strategies sequentially
    Sequential,
    /// Execute based on conditions
    Conditional,
}

/// Bin price information with calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinPriceInfo {
    /// Bin ID
    pub bin_id: i32,
    /// Current price
    pub price: Decimal,
    /// Liquidity in token X
    pub liquidity_x: u64,
    /// Liquidity in token Y
    pub liquidity_y: u64,
    /// Total liquidity
    pub total_liquidity: u128,
    /// Volume in last 24h
    pub volume_24h: Option<Decimal>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Market data snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    /// Pool address
    pub pool_address: Pubkey,
    /// Current active bin
    pub active_bin_id: i32,
    /// Current price
    pub current_price: Decimal,
    /// Bin prices around active bin
    pub bin_prices: BTreeMap<i32, BinPriceInfo>,
    /// 24h price change percentage
    pub price_change_24h_pct: Decimal,
    /// 24h volume
    pub volume_24h: Decimal,
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,
}

/// Order book state
#[derive(Debug, Clone, Default)]
pub struct OrderBook {
    /// Buy orders by bin ID (descending price)
    pub buy_orders: BTreeMap<i32, Vec<RangeOrder>>,
    /// Sell orders by bin ID (ascending price)
    pub sell_orders: BTreeMap<i32, Vec<RangeOrder>>,
    /// All orders by ID for quick lookup
    pub orders_by_id: BTreeMap<Uuid, RangeOrder>,
    /// Orders by strategy ID
    pub orders_by_strategy: BTreeMap<Uuid, Vec<Uuid>>,
}

/// Strategy execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyStatus {
    /// Strategy ID
    pub strategy_id: Uuid,
    /// Strategy type
    pub strategy_type: TradingStrategy,
    /// Current status
    pub status: StrategyExecutionStatus,
    /// Created orders
    pub order_ids: Vec<Uuid>,
    /// Total executed volume
    pub executed_volume: Decimal,
    /// Total profit/loss
    pub pnl: Decimal,
    /// Success rate (percentage of successful orders)
    pub success_rate: Decimal,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Last executed at
    pub last_executed_at: Option<DateTime<Utc>>,
}

/// Strategy execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyExecutionStatus {
    /// Strategy is active and monitoring
    Active,
    /// Strategy is paused
    Paused,
    /// Strategy completed successfully
    Completed,
    /// Strategy failed
    Failed,
    /// Strategy was cancelled
    Cancelled,
}

/// Risk management parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskParameters {
    /// Maximum position size per order
    pub max_position_size: Decimal,
    /// Maximum total exposure
    pub max_total_exposure: Decimal,
    /// Maximum number of active orders
    pub max_active_orders: u32,
    /// Maximum slippage tolerance (basis points)
    pub max_slippage_bps: u16,
    /// Stop loss percentage for all positions
    pub global_stop_loss_pct: Option<Decimal>,
    /// Daily loss limit
    pub daily_loss_limit: Option<Decimal>,
}

/// Configuration for the range order system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeOrderConfig {
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Wallet file path
    pub wallet_path: String,
    /// Default pool address
    pub default_pool: Option<Pubkey>,
    /// Risk management parameters
    pub risk_params: RiskParameters,
    /// Monitoring intervals
    pub monitoring_interval_ms: u64,
    /// Order execution retry attempts
    pub max_retry_attempts: u32,
    /// Enable notifications
    pub enable_notifications: bool,
    /// Webhook URL for notifications
    pub webhook_url: Option<String>,
}

/// Notification types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    /// Order executed
    OrderExecuted {
        order_id: Uuid,
        amount: Decimal,
        price: Decimal,
    },
    /// Order failed
    OrderFailed {
        order_id: Uuid,
        error: String,
    },
    /// Strategy completed
    StrategyCompleted {
        strategy_id: Uuid,
        total_pnl: Decimal,
    },
    /// Risk limit exceeded
    RiskLimitExceeded {
        limit_type: String,
        current_value: Decimal,
        limit_value: Decimal,
    },
    /// Market event
    MarketEvent {
        event_type: String,
        description: String,
    },
}

/// Performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total orders executed
    pub total_orders: u64,
    /// Successful orders
    pub successful_orders: u64,
    /// Total volume traded
    pub total_volume: Decimal,
    /// Total profit/loss
    pub total_pnl: Decimal,
    /// Average execution time (milliseconds)
    pub avg_execution_time_ms: f64,
    /// Success rate percentage
    pub success_rate: Decimal,
    /// Sharpe ratio
    pub sharpe_ratio: Option<Decimal>,
    /// Maximum drawdown
    pub max_drawdown: Decimal,
    /// Current drawdown
    pub current_drawdown: Decimal,
    /// Best performing strategy
    pub best_strategy_id: Option<Uuid>,
    /// Worst performing strategy
    pub worst_strategy_id: Option<Uuid>,
}