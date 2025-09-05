use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRequest {
    pub from_token: Pubkey,
    pub to_token: Pubkey,
    pub amount: Decimal,
    pub max_hops: u8,
    pub max_slippage: Decimal,
    pub split_routes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResponse {
    pub route_id: String,
    pub path: Vec<RouteHop>,
    pub expected_output: Decimal,
    pub price_impact: Decimal,
    pub gas_estimate: Decimal,
    pub confidence_score: f64,
    pub split_routes: Vec<SplitRoute>,
    pub execution_time_estimate: u64, // milliseconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHop {
    pub from_token: Pubkey,
    pub to_token: Pubkey,
    pub pool_address: Pubkey,
    pub expected_amount_in: Decimal,
    pub expected_amount_out: Decimal,
    pub fee_tier: Decimal,
    pub price_impact: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitRoute {
    pub route_id: String,
    pub percentage: Decimal,
    pub amount: Decimal,
    pub expected_output: Decimal,
    pub path: Vec<RouteHop>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolNode {
    pub address: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub liquidity_usd: Decimal,
    pub fee_tier: Decimal,
    pub volume_24h: Decimal,
    pub active_bins: u32,
    pub bin_step: u16,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenNode {
    pub address: Pubkey,
    pub symbol: String,
    pub decimals: u8,
    pub price_usd: Decimal,
    pub market_cap: Decimal,
    pub pools: Vec<Pubkey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub pool: Pubkey,
    pub weight: f64, // Routing weight (inverse of liquidity + fees)
    pub gas_cost: Decimal,
    pub price_impact_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub id: String,
    pub cycle: Vec<ArbitrageCycleHop>,
    pub expected_profit_usd: Decimal,
    pub roi_percentage: Decimal,
    pub risk_score: f64,
    pub confidence: Decimal,
    pub required_capital_usd: Decimal,
    pub execution_complexity: u8,
    pub time_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageCycleHop {
    pub token: Pubkey,
    pub pool_address: Pubkey,
    pub expected_amount_in: Decimal,
    pub expected_amount_out: Decimal,
    pub price_impact: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteExecutionSimulation {
    pub route_id: String,
    pub expected_output: Decimal,
    pub total_price_impact: Decimal,
    pub estimated_gas: Decimal,
    pub success_probability: Decimal,
    pub warnings: Vec<String>,
    pub execution_steps: Vec<ExecutionStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_number: u8,
    pub pool_address: Pubkey,
    pub from_token: Pubkey,
    pub to_token: Pubkey,
    pub amount_in: Decimal,
    pub expected_amount_out: Decimal,
    pub gas_cost: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStatistics {
    pub total_pools: usize,
    pub total_tokens: usize,
    pub avg_liquidity_usd: Decimal,
    pub graph_density: f64,
    pub largest_component_size: usize,
    pub clustering_coefficient: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConnectivityAnalysis {
    pub token: Pubkey,
    pub direct_pairs: usize,
    pub two_hop_tokens: usize,
    pub three_hop_tokens: usize,
    pub centrality_score: f64,
    pub liquidity_centrality: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceImpactModel {
    pub linear_factor: f64,
    pub quadratic_factor: f64,
    pub liquidity_depth: Decimal,
    pub bin_spread_impact: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteOptimizationParams {
    pub weight_price_impact: f64,
    pub weight_gas_cost: f64,
    pub weight_liquidity_depth: f64,
    pub weight_execution_certainty: f64,
    pub max_split_routes: u8,
    pub min_split_amount_usd: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub timestamp: u64,
    pub ttl: u64,
    pub access_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimation {
    pub base_gas: u64,
    pub per_hop_gas: u64,
    pub compute_units: u64,
    pub priority_fee: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityDistribution {
    pub bins: Vec<LiquidityBin>,
    pub active_bin_id: i32,
    pub total_liquidity: Decimal,
    pub price_range: (Decimal, Decimal),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityBin {
    pub bin_id: i32,
    pub price: Decimal,
    pub liquidity_x: Decimal,
    pub liquidity_y: Decimal,
    pub total_liquidity_usd: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConditions {
    pub volatility_index: f64,
    pub liquidity_index: f64,
    pub gas_price_gwei: Decimal,
    pub network_congestion: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMetrics {
    pub total_routes_found: u64,
    pub avg_route_length: f64,
    pub avg_price_impact: Decimal,
    pub success_rate: Decimal,
    pub cache_hit_rate: f64,
    pub avg_computation_time_ms: u64,
}

// Error types
#[derive(thiserror::Error, Debug)]
pub enum RoutingError {
    #[error("No route found between tokens")]
    NoRouteFound,
    
    #[error("Insufficient liquidity: required {required}, available {available}")]
    InsufficientLiquidity { required: Decimal, available: Decimal },
    
    #[error("Price impact too high: {impact}% > {max_allowed}%")]
    PriceImpactTooHigh { impact: Decimal, max_allowed: Decimal },
    
    #[error("Graph construction failed: {reason}")]
    GraphConstructionFailed { reason: String },
    
    #[error("Cache operation failed: {operation}")]
    CacheOperationFailed { operation: String },
    
    #[error("Route execution simulation failed: {reason}")]
    SimulationFailed { reason: String },
    
    #[error("Invalid token pair: from={from}, to={to}")]
    InvalidTokenPair { from: Pubkey, to: Pubkey },
    
    #[error("Network error: {source}")]
    NetworkError {
        #[from]
        source: reqwest::Error,
    },
    
    #[error("Solana RPC error: {source}")]
    SolanaRpcError {
        #[from]
        source: solana_client::client_error::ClientError,
    },
    
    #[error("Calculation error: {message}")]
    CalculationError { message: String },
}

// Constants
pub const MAX_ROUTE_HOPS: u8 = 5;
pub const MAX_SPLIT_ROUTES: u8 = 4;
pub const DEFAULT_CACHE_TTL: u64 = 30; // seconds
pub const MIN_LIQUIDITY_USD: Decimal = rust_decimal_macros::dec!(1000);
pub const MAX_PRICE_IMPACT: Decimal = rust_decimal_macros::dec!(0.15); // 15%
pub const GAS_ESTIMATION_BUFFER: f64 = 1.2; // 20% buffer

// Type aliases for graph structures
pub type TokenGraph = petgraph::Graph<TokenNode, GraphEdge, petgraph::Undirected>;
pub type NodeIndex = petgraph::graph::NodeIndex;
pub type EdgeIndex = petgraph::graph::EdgeIndex;
pub type RouteCache = moka::future::Cache<String, RouteResponse>;
pub type ArbitrageCache = moka::future::Cache<String, Vec<ArbitrageOpportunity>>;

// Helper functions
impl RouteResponse {
    pub fn total_gas_cost(&self) -> Decimal {
        let split_gas: u64 = self.split_routes.iter()
            .map(|sr| sr.path.len() as u64 * 5000) // Estimated gas per hop
            .sum();
        
        self.gas_estimate + Decimal::from(split_gas)
    }
    
    pub fn effective_price(&self) -> Decimal {
        if self.expected_output.is_zero() {
            return Decimal::ZERO;
        }
        
        let total_input = self.path.first()
            .map(|hop| hop.expected_amount_in)
            .unwrap_or_default();
            
        total_input / self.expected_output
    }
}

impl ArbitrageOpportunity {
    pub fn is_profitable(&self, gas_cost_usd: Decimal) -> bool {
        self.expected_profit_usd > gas_cost_usd * rust_decimal_macros::dec!(2.0)
    }
    
    pub fn execution_priority(&self) -> u8 {
        let profit_score = (self.expected_profit_usd.to_f64().unwrap_or(0.0) / 100.0).min(10.0) as u8;
        let confidence_score = (self.confidence.to_f64().unwrap_or(0.0) * 10.0) as u8;
        let risk_penalty = (self.risk_score * 2.0) as u8;
        
        (profit_score + confidence_score).saturating_sub(risk_penalty)
    }
}