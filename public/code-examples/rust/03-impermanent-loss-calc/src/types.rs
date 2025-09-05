//! Type definitions for the DLMM Impermanent Loss Calculator

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::BTreeMap;

/// Analysis configuration for IL calculations
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    pub pool_address: Pubkey,
    pub position_id: Option<Pubkey>,
    pub mode: MonitoringMode,
    pub interval_secs: u64,
    pub output_directory: String,
    pub enable_notifications: bool,
    pub max_price_deviation: Decimal,
    pub min_fee_threshold: Decimal,
    pub historical_days: u32,
    pub volatility_window: u32, // hours
}

/// Different monitoring modes for the IL calculator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitoringMode {
    /// Single snapshot analysis
    Snapshot,
    /// Continuous real-time monitoring
    RealTime,
    /// Historical data analysis
    Historical,
}

/// Comprehensive impermanent loss calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpermanentLossResult {
    /// Impermanent loss as a percentage (e.g., -0.05 for -5%)
    pub il_percentage: Decimal,
    /// Impermanent loss in USD value
    pub il_usd_value: Decimal,
    /// Current position value in USD
    pub current_value_usd: Decimal,
    /// Value if held tokens separately (no LP)
    pub hold_value_usd: Decimal,
    /// Current price of token X
    pub current_price_x: Decimal,
    /// Current price of token Y  
    pub current_price_y: Decimal,
    /// Initial price of token X
    pub initial_price_x: Decimal,
    /// Initial price of token Y
    pub initial_price_y: Decimal,
    /// Price ratio change (current_px/current_py) / (initial_px/initial_py)
    pub price_ratio_change: Decimal,
    /// Timestamp of calculation
    pub timestamp: DateTime<Utc>,
    /// Additional metadata
    pub metadata: ILMetadata,
}

/// Additional metadata for IL calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILMetadata {
    /// Pool address
    pub pool_address: Pubkey,
    /// Position ID if applicable
    pub position_id: Option<Pubkey>,
    /// Bin range for DLMM position
    pub bin_range: Option<(i32, i32)>, // (lower_bin_id, upper_bin_id)
    /// Active bin ID at calculation time
    pub active_bin_id: Option<i32>,
    /// Price range coverage
    pub price_range_coverage: Option<Decimal>,
    /// Calculation method used
    pub calculation_method: CalculationMethod,
}

/// Method used for IL calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CalculationMethod {
    /// From existing position data
    FromPosition,
    /// Manual calculation with provided parameters
    Manual,
    /// Historical reconstruction
    Historical,
}

/// Historical price data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceDataPoint {
    pub timestamp: DateTime<Utc>,
    pub price_x: Decimal,
    pub price_y: Decimal,
    pub volume_24h: Decimal,
    pub liquidity: Decimal,
    pub active_bin_id: i32,
}

/// Position performance analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionAnalysis {
    /// Basic position information
    pub position_info: PositionInfo,
    /// Current IL result
    pub il_result: ImpermanentLossResult,
    /// Fee analysis
    pub fee_analysis: FeeAnalysis,
    /// Risk metrics
    pub risk_metrics: RiskMetrics,
    /// Performance summary
    pub performance_summary: PerformanceSummary,
    /// Timestamp of analysis
    pub timestamp: DateTime<Utc>,
}

/// Basic position information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionInfo {
    pub position_id: Option<Pubkey>,
    pub pool_address: Pubkey,
    pub owner: Pubkey,
    pub token_x_symbol: String,
    pub token_y_symbol: String,
    pub lower_bin_id: i32,
    pub upper_bin_id: i32,
    pub current_liquidity: Decimal,
    pub initial_investment_usd: Decimal,
    pub current_value_usd: Decimal,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Fee analysis for the position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeAnalysis {
    /// Total fees earned in USD
    pub total_fees_earned: Decimal,
    /// Fees earned in token X
    pub fees_token_x: Decimal,
    /// Fees earned in token Y  
    pub fees_token_y: Decimal,
    /// Fee APY based on current position
    pub fee_apy: Decimal,
    /// Daily fee rate average
    pub daily_fee_rate: Decimal,
    /// Fee vs IL comparison
    pub fee_vs_il_ratio: Decimal,
    /// Break-even analysis
    pub break_even_days: Option<u32>,
}

/// Risk metrics for the position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    /// Price volatility (standard deviation)
    pub price_volatility: Decimal,
    /// Maximum observed IL in the period
    pub max_il_observed: Decimal,
    /// Value at Risk (95% confidence)
    pub var_95: Decimal,
    /// Sharpe ratio for the position
    pub sharpe_ratio: Decimal,
    /// Liquidity concentration risk
    pub concentration_risk: Decimal,
    /// Bin utilization percentage
    pub bin_utilization: Decimal,
}

/// Performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    /// Total return including fees and IL
    pub total_return_usd: Decimal,
    /// Total return percentage
    pub total_return_percentage: Decimal,
    /// Annualized return
    pub annualized_return: Decimal,
    /// Net PnL (fees - IL - gas costs)
    pub net_pnl: Decimal,
    /// Days position has been active
    pub days_active: u32,
    /// Performance vs holding tokens
    pub vs_hold_performance: Decimal,
    /// Performance vs market benchmark
    pub vs_market_performance: Option<Decimal>,
}

/// Historical trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalTrends {
    /// Time period analyzed
    pub period_days: u32,
    /// Maximum IL percentage observed
    pub max_il_percentage: Decimal,
    /// Minimum IL percentage observed  
    pub min_il_percentage: Decimal,
    /// Average IL percentage
    pub avg_il_percentage: Decimal,
    /// Standard deviation of IL
    pub il_volatility: Decimal,
    /// Correlation with market movements
    pub market_correlation: Decimal,
    /// Trend direction (positive/negative/sideways)
    pub trend_direction: TrendDirection,
    /// Average daily price change
    pub avg_daily_price_change: Decimal,
    /// Maximum drawdown period
    pub max_drawdown_days: u32,
    /// Recovery periods after major IL events
    pub recovery_periods: Vec<RecoveryPeriod>,
}

/// Trend direction classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Bullish,
    Bearish,
    Sideways,
    Volatile,
}

/// Recovery period after IL event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPeriod {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub max_il_in_period: Decimal,
    pub recovery_days: u32,
    pub fee_compensation: Decimal,
}

/// Configuration for report generation
#[derive(Debug, Clone)]
pub struct ReportConfig {
    pub title: String,
    pub include_charts: bool,
    pub include_raw_data: bool,
    pub timestamp: DateTime<Utc>,
}

/// Report format options
#[derive(Debug, Clone)]
pub enum ReportFormat {
    Json,
    Csv,
    Html,
}

impl std::fmt::Display for ReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportFormat::Json => write!(f, "JSON"),
            ReportFormat::Csv => write!(f, "CSV"),
            ReportFormat::Html => write!(f, "HTML"),
        }
    }
}

/// Pool information for IL calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: Pubkey,
    pub token_x: Pubkey,
    pub token_y: Pubkey,
    pub token_x_symbol: String,
    pub token_y_symbol: String,
    pub active_bin_id: i32,
    pub bin_step: u16,
    pub total_liquidity: Decimal,
    pub volume_24h: Decimal,
    pub fees_24h: Decimal,
    pub tvl: Decimal,
    pub created_at: DateTime<Utc>,
}

/// Bin information for DLMM pools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinData {
    pub bin_id: i32,
    pub price: Decimal,
    pub liquidity_x: Decimal,
    pub liquidity_y: Decimal,
    pub total_liquidity: Decimal,
    pub fee_rate: Decimal,
    pub is_active: bool,
}

/// Position snapshot for historical tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSnapshot {
    pub timestamp: DateTime<Utc>,
    pub position_value_usd: Decimal,
    pub hold_value_usd: Decimal,
    pub il_percentage: Decimal,
    pub fees_earned: Decimal,
    pub active_bin_id: i32,
    pub price_x: Decimal,
    pub price_y: Decimal,
}

/// Notification event for IL monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILNotificationEvent {
    pub event_type: ILEventType,
    pub position_id: Option<Pubkey>,
    pub pool_address: Pubkey,
    pub il_percentage: Decimal,
    pub threshold_crossed: Option<Decimal>,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub severity: NotificationSeverity,
}

/// Types of IL notification events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ILEventType {
    HighImpermanentLoss,
    ILThresholdCrossed,
    PositionOutOfRange,
    LowFeeCompensation,
    PriceVoLatilitySpike,
    MarketCrash,
    RecoveryDetected,
}

/// Notification severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Error types specific to IL calculations
#[derive(Debug, thiserror::Error)]
pub enum ILError {
    #[error("Invalid position data: {0}")]
    InvalidPosition(String),
    
    #[error("Price data unavailable: {0}")]
    PriceDataUnavailable(String),
    
    #[error("Calculation error: {0}")]
    CalculationError(String),
    
    #[error("Historical data insufficient: {0}")]
    InsufficientData(String),
    
    #[error("Pool not found: {0}")]
    PoolNotFound(Pubkey),
    
    #[error("Position not found: {0}")]
    PositionNotFound(Pubkey),
    
    #[error("Report generation failed: {0}")]
    ReportError(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Result type for IL operations
pub type ILResult<T> = Result<T, ILError>;

/// Statistics for multiple positions or pools
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AggregateStatistics {
    pub total_positions: usize,
    pub total_tvl: Decimal,
    pub average_il: Decimal,
    pub total_fees_earned: Decimal,
    pub best_performing_pool: Option<Pubkey>,
    pub worst_performing_pool: Option<Pubkey>,
    pub correlation_matrix: BTreeMap<String, BTreeMap<String, Decimal>>,
    pub risk_adjusted_returns: BTreeMap<Pubkey, Decimal>,
}