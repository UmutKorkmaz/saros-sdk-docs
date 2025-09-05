//! Swap Optimizer - Advanced parameter optimization for best execution
//!
//! This module provides intelligent optimization strategies for swap parameters including:
//! - Dynamic slippage adjustment based on market conditions
//! - Route optimization across multiple pools
//! - Gas price optimization for transaction priority
//! - Price impact minimization strategies

use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use lru::LruCache;
use num_traits::Zero;
use once_cell::sync::Lazy;
use priority_queue::PriorityQueue;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::{
    collections::HashMap,
    num::NonZeroUsize,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::time::sleep;
use uuid::Uuid;

/// Global cache for pool liquidity data
static LIQUIDITY_CACHE: Lazy<Arc<RwLock<LruCache<Pubkey, PoolLiquidity>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(1000).unwrap()))));

/// Global cache for price data
static PRICE_CACHE: Lazy<DashMap<String, PriceData>> = Lazy::new(|| DashMap::new());

/// Optimizer configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerConfig {
    /// Maximum slippage tolerance (basis points)
    pub max_slippage_bps: u16,
    /// Minimum slippage tolerance (basis points)
    pub min_slippage_bps: u16,
    /// Price impact threshold for route splitting (percentage)
    pub price_impact_threshold: f64,
    /// Maximum number of routes to consider
    pub max_routes: usize,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Enable aggressive optimization
    pub aggressive_optimization: bool,
    /// Gas optimization enabled
    pub gas_optimization: bool,
    /// MEV protection level (0-3)
    pub mev_protection_level: u8,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            max_slippage_bps: 300,  // 3%
            min_slippage_bps: 10,   // 0.1%
            price_impact_threshold: 2.0,  // 2%
            max_routes: 5,
            cache_ttl_seconds: 30,
            aggressive_optimization: false,
            gas_optimization: true,
            mev_protection_level: 2,
        }
    }
}

/// Pool liquidity information for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolLiquidity {
    pub pool_address: Pubkey,
    pub token_x_liquidity: u64,
    pub token_y_liquidity: u64,
    pub total_liquidity_usd: f64,
    pub volume_24h: f64,
    pub fee_tier: u16,
    pub last_updated: DateTime<Utc>,
}

/// Price data with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub price: f64,
    pub volume: f64,
    pub price_change_24h: f64,
    pub volatility: f64,
    pub timestamp: DateTime<Utc>,
}

/// Optimized swap parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedSwapParams {
    pub routes: Vec<SwapRoute>,
    pub optimal_slippage_bps: u16,
    pub estimated_price_impact: f64,
    pub estimated_gas_cost: u64,
    pub execution_priority: u8,
    pub mev_protection: MevProtection,
    pub expected_amount_out: u64,
    pub confidence_score: f64,
    pub optimization_id: Uuid,
}

/// Individual swap route with optimization details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRoute {
    pub pool_address: Pubkey,
    pub amount_in: u64,
    pub expected_amount_out: u64,
    pub price_impact: f64,
    pub fee: u64,
    pub liquidity_depth: f64,
    pub route_priority: u8,
}

/// MEV protection strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevProtection {
    pub use_private_mempool: bool,
    pub bundle_transactions: bool,
    pub randomize_timing: bool,
    pub split_large_orders: bool,
    pub protection_level: u8,
}

/// Swap optimization statistics
#[derive(Debug, Clone, Default)]
pub struct OptimizationStats {
    pub total_optimizations: u64,
    pub successful_optimizations: u64,
    pub average_improvement_bps: f64,
    pub cache_hit_rate: f64,
    pub average_optimization_time_ms: f64,
    pub gas_savings_total: u64,
}

/// Main swap optimizer with advanced strategies
pub struct SwapOptimizer {
    config: OptimizerConfig,
    stats: Arc<RwLock<OptimizationStats>>,
    route_cache: DashMap<String, (OptimizedSwapParams, Instant)>,
    gas_tracker: Arc<RwLock<GasPriceTracker>>,
    market_conditions: Arc<RwLock<MarketConditions>>,
}

/// Gas price tracking for optimization
#[derive(Debug, Clone)]
struct GasPriceTracker {
    current_gas_price: u64,
    gas_price_history: Vec<(u64, DateTime<Utc>)>,
    optimal_gas_price: u64,
    congestion_level: f64,
}

/// Market conditions for optimization decisions
#[derive(Debug, Clone)]
struct MarketConditions {
    overall_volatility: f64,
    market_sentiment: f64,
    liquidity_conditions: f64,
    network_congestion: f64,
    last_updated: DateTime<Utc>,
}

impl SwapOptimizer {
    /// Create a new swap optimizer with default configuration
    pub fn new() -> Self {
        Self::with_config(OptimizerConfig::default())
    }

    /// Create a new swap optimizer with custom configuration
    pub fn with_config(config: OptimizerConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(OptimizationStats::default())),
            route_cache: DashMap::new(),
            gas_tracker: Arc::new(RwLock::new(GasPriceTracker::default())),
            market_conditions: Arc::new(RwLock::new(MarketConditions::default())),
        }
    }

    /// Optimize swap parameters for best execution
    pub async fn optimize_swap(
        &self,
        token_in: Pubkey,
        token_out: Pubkey,
        amount_in: u64,
        available_pools: &[Pubkey],
    ) -> Result<OptimizedSwapParams> {
        let start_time = Instant::now();
        let optimization_id = Uuid::new_v4();

        log::info!("🚀 Starting swap optimization {} for {:.4} tokens", 
                   optimization_id, amount_in as f64 / 1e9);

        // Check cache first
        let cache_key = format!("{}-{}-{}", token_in, token_out, amount_in);
        if let Some((cached_params, cached_time)) = self.route_cache.get(&cache_key) {
            if cached_time.elapsed().as_secs() < self.config.cache_ttl_seconds {
                self.update_cache_hit_stats().await;
                log::debug!("📊 Using cached optimization result");
                return Ok(cached_params.clone());
            }
        }

        // Analyze current market conditions
        let market_conditions = self.analyze_market_conditions().await?;
        log::debug!("📈 Market conditions: volatility={:.2}%, liquidity={:.2}%", 
                   market_conditions.overall_volatility * 100.0, 
                   market_conditions.liquidity_conditions * 100.0);

        // Get pool liquidity data
        let pool_liquidities = self.fetch_pool_liquidities(available_pools).await?;
        log::debug!("💧 Analyzed {} pools for liquidity", pool_liquidities.len());

        // Calculate optimal routes
        let routes = self.calculate_optimal_routes(
            token_in,
            token_out,
            amount_in,
            &pool_liquidities,
            &market_conditions,
        ).await?;

        // Optimize slippage based on market conditions and route analysis
        let optimal_slippage = self.optimize_slippage(&routes, &market_conditions).await?;

        // Calculate MEV protection strategy
        let mev_protection = self.calculate_mev_protection(amount_in, &market_conditions).await?;

        // Estimate gas costs and optimize priority
        let (gas_cost, execution_priority) = self.optimize_gas_and_priority(&routes).await?;

        // Calculate expected output and confidence
        let expected_amount_out: u64 = routes.iter().map(|r| r.expected_amount_out).sum();
        let confidence_score = self.calculate_confidence_score(&routes, &market_conditions).await?;

        let optimization_result = OptimizedSwapParams {
            routes,
            optimal_slippage_bps: optimal_slippage,
            estimated_price_impact: self.calculate_total_price_impact(&pool_liquidities).await?,
            estimated_gas_cost: gas_cost,
            execution_priority,
            mev_protection,
            expected_amount_out,
            confidence_score,
            optimization_id,
        };

        // Cache the result
        self.route_cache.insert(
            cache_key,
            (optimization_result.clone(), Instant::now()),
        );

        // Update statistics
        self.update_optimization_stats(start_time.elapsed(), true).await;

        log::info!("✅ Optimization {} complete: {:.4} expected output, {:.2}% confidence", 
                   optimization_id, 
                   expected_amount_out as f64 / 1e6,
                   confidence_score * 100.0);

        Ok(optimization_result)
    }

    /// Analyze current market conditions
    async fn analyze_market_conditions(&self) -> Result<MarketConditions> {
        let mut conditions = self.market_conditions.write().unwrap();
        
        // Simulate market analysis (in real implementation, this would fetch from APIs)
        conditions.overall_volatility = self.calculate_volatility().await?;
        conditions.market_sentiment = self.analyze_sentiment().await?;
        conditions.liquidity_conditions = self.assess_liquidity_conditions().await?;
        conditions.network_congestion = self.check_network_congestion().await?;
        conditions.last_updated = Utc::now();

        Ok(conditions.clone())
    }

    /// Fetch and cache pool liquidity data
    async fn fetch_pool_liquidities(&self, pools: &[Pubkey]) -> Result<Vec<PoolLiquidity>> {
        let mut liquidities = Vec::new();
        
        for &pool in pools {
            if let Ok(mut cache) = LIQUIDITY_CACHE.write() {
                if let Some(cached_liquidity) = cache.get(&pool) {
                    if (Utc::now() - cached_liquidity.last_updated).num_seconds() < 60 {
                        liquidities.push(cached_liquidity.clone());
                        continue;
                    }
                }
            }

            // Fetch fresh liquidity data
            let liquidity = self.fetch_pool_liquidity(pool).await?;
            
            // Cache the result
            if let Ok(mut cache) = LIQUIDITY_CACHE.write() {
                cache.put(pool, liquidity.clone());
            }
            
            liquidities.push(liquidity);
        }

        Ok(liquidities)
    }

    /// Calculate optimal routes with advanced algorithms
    async fn calculate_optimal_routes(
        &self,
        token_in: Pubkey,
        token_out: Pubkey,
        amount_in: u64,
        pool_liquidities: &[PoolLiquidity],
        market_conditions: &MarketConditions,
    ) -> Result<Vec<SwapRoute>> {
        let mut routes = Vec::new();
        let mut route_queue = PriorityQueue::new();

        // Analyze each pool for potential routes
        for liquidity in pool_liquidities {
            let route_score = self.calculate_route_score(liquidity, amount_in, market_conditions).await?;
            route_queue.push(liquidity.pool_address, (route_score * 1000.0) as i64);
        }

        // Select top routes based on configuration
        let mut total_allocated = 0u64;
        let mut route_priority = 1u8;

        while let Some((pool_address, _score)) = route_queue.pop() {
            if routes.len() >= self.config.max_routes {
                break;
            }

            let pool_liquidity = pool_liquidities.iter()
                .find(|l| l.pool_address == pool_address)
                .unwrap();

            // Allocate portion of total amount based on pool quality
            let allocation_ratio = self.calculate_allocation_ratio(
                pool_liquidity, 
                amount_in, 
                total_allocated
            ).await?;
            
            let route_amount = if self.config.aggressive_optimization {
                ((amount_in - total_allocated) as f64 * allocation_ratio) as u64
            } else {
                // More conservative allocation
                ((amount_in - total_allocated) as f64 * allocation_ratio * 0.8) as u64
            };

            if route_amount < 1000 { // Minimum viable route size
                continue;
            }

            let expected_output = self.calculate_expected_output(
                pool_liquidity,
                route_amount,
            ).await?;

            let price_impact = self.calculate_price_impact(
                pool_liquidity,
                route_amount,
            ).await?;

            routes.push(SwapRoute {
                pool_address,
                amount_in: route_amount,
                expected_amount_out: expected_output,
                price_impact,
                fee: self.calculate_route_fee(pool_liquidity, route_amount).await?,
                liquidity_depth: pool_liquidity.total_liquidity_usd,
                route_priority,
            });

            total_allocated += route_amount;
            route_priority += 1;

            if total_allocated >= amount_in {
                break;
            }
        }

        // If we haven't allocated everything, put the remainder in the best route
        if total_allocated < amount_in && !routes.is_empty() {
            routes[0].amount_in += amount_in - total_allocated;
            routes[0].expected_amount_out = self.calculate_expected_output(
                &pool_liquidities.iter().find(|l| l.pool_address == routes[0].pool_address).unwrap(),
                routes[0].amount_in,
            ).await?;
        }

        log::debug!("🛣️ Generated {} optimal routes", routes.len());
        Ok(routes)
    }

    /// Optimize slippage based on market conditions
    async fn optimize_slippage(
        &self,
        routes: &[SwapRoute],
        market_conditions: &MarketConditions,
    ) -> Result<u16> {
        let base_slippage = if market_conditions.overall_volatility > 0.1 {
            // High volatility - increase slippage
            (self.config.min_slippage_bps as f64 * (1.0 + market_conditions.overall_volatility * 2.0)) as u16
        } else {
            self.config.min_slippage_bps
        };

        // Adjust for route complexity
        let route_adjustment = if routes.len() > 1 {
            (base_slippage as f64 * 0.2 * routes.len() as f64) as u16
        } else {
            0
        };

        // Adjust for network congestion
        let congestion_adjustment = (base_slippage as f64 * market_conditions.network_congestion * 0.3) as u16;

        let optimal_slippage = (base_slippage + route_adjustment + congestion_adjustment)
            .min(self.config.max_slippage_bps)
            .max(self.config.min_slippage_bps);

        log::debug!("🎯 Optimal slippage: {} bps (base: {}, route: +{}, congestion: +{})", 
                   optimal_slippage, base_slippage, route_adjustment, congestion_adjustment);

        Ok(optimal_slippage)
    }

    /// Calculate MEV protection strategy
    async fn calculate_mev_protection(
        &self,
        amount_in: u64,
        market_conditions: &MarketConditions,
    ) -> Result<MevProtection> {
        let is_large_trade = amount_in > 10_000_000_000; // > 10 SOL
        let high_volatility = market_conditions.overall_volatility > 0.05;
        let congested_network = market_conditions.network_congestion > 0.7;

        let protection = MevProtection {
            use_private_mempool: self.config.mev_protection_level >= 3 || 
                (is_large_trade && self.config.mev_protection_level >= 2),
            bundle_transactions: self.config.mev_protection_level >= 2 && is_large_trade,
            randomize_timing: self.config.mev_protection_level >= 1,
            split_large_orders: is_large_trade && (high_volatility || congested_network),
            protection_level: self.config.mev_protection_level,
        };

        log::debug!("🛡️ MEV protection: private_mempool={}, bundling={}, timing_randomization={}", 
                   protection.use_private_mempool, protection.bundle_transactions, protection.randomize_timing);

        Ok(protection)
    }

    /// Optimize gas pricing and execution priority
    async fn optimize_gas_and_priority(&self, routes: &[SwapRoute]) -> Result<(u64, u8)> {
        let gas_tracker = self.gas_tracker.read().unwrap();
        
        let base_gas = 100_000u64; // Base gas for simple swap
        let complex_gas_multiplier = 1.2 + (routes.len() as f64 * 0.1);
        
        let estimated_gas = (base_gas as f64 * complex_gas_multiplier) as u64;
        
        // Calculate optimal priority based on network conditions and trade size
        let total_value = routes.iter().map(|r| r.amount_in).sum::<u64>();
        let priority = if total_value > 5_000_000_000 { // > 5 SOL
            3 // High priority
        } else if total_value > 1_000_000_000 { // > 1 SOL
            2 // Medium priority
        } else {
            1 // Normal priority
        };

        let optimized_gas = if self.config.gas_optimization {
            // Use slightly higher than current optimal to ensure execution
            (gas_tracker.optimal_gas_price as f64 * 1.05) as u64
        } else {
            gas_tracker.current_gas_price
        };

        log::debug!("⛽ Gas optimization: estimated={}, priority={}", 
                   optimized_gas, priority);

        Ok((estimated_gas * optimized_gas / 1_000_000, priority))
    }

    /// Calculate confidence score for the optimization
    async fn calculate_confidence_score(
        &self,
        routes: &[SwapRoute],
        market_conditions: &MarketConditions,
    ) -> Result<f64> {
        let mut confidence = 0.8; // Base confidence

        // Adjust for route diversity
        confidence += (routes.len() as f64 * 0.05).min(0.15);

        // Adjust for market stability
        confidence -= market_conditions.overall_volatility * 0.5;

        // Adjust for liquidity depth
        let avg_liquidity = routes.iter()
            .map(|r| r.liquidity_depth)
            .sum::<f64>() / routes.len() as f64;
        
        if avg_liquidity > 1_000_000.0 { // > $1M
            confidence += 0.1;
        } else if avg_liquidity < 100_000.0 { // < $100K
            confidence -= 0.2;
        }

        // Adjust for network congestion
        confidence -= market_conditions.network_congestion * 0.1;

        Ok(confidence.max(0.1).min(0.99))
    }

    /// Get optimization statistics
    pub async fn get_stats(&self) -> OptimizationStats {
        self.stats.read().unwrap().clone()
    }

    /// Update optimization statistics
    async fn update_optimization_stats(&self, duration: Duration, successful: bool) {
        let mut stats = self.stats.write().unwrap();
        stats.total_optimizations += 1;
        if successful {
            stats.successful_optimizations += 1;
        }
        
        let duration_ms = duration.as_millis() as f64;
        stats.average_optimization_time_ms = 
            (stats.average_optimization_time_ms * (stats.total_optimizations - 1) as f64 + duration_ms) 
            / stats.total_optimizations as f64;
    }

    async fn update_cache_hit_stats(&self) {
        let mut stats = self.stats.write().unwrap();
        // Update cache hit rate calculation
        stats.cache_hit_rate = (stats.cache_hit_rate * 0.9) + 0.1;
    }

    // Helper methods for market analysis (simplified implementations)
    
    async fn calculate_volatility(&self) -> Result<f64> {
        // Simulate volatility calculation
        Ok(0.02 + (rand::random::<f64>() * 0.08)) // 2-10% volatility
    }

    async fn analyze_sentiment(&self) -> Result<f64> {
        // Simulate market sentiment analysis
        Ok(0.3 + (rand::random::<f64>() * 0.4)) // Neutral to positive
    }

    async fn assess_liquidity_conditions(&self) -> Result<f64> {
        // Simulate liquidity assessment
        Ok(0.6 + (rand::random::<f64>() * 0.3)) // Good liquidity
    }

    async fn check_network_congestion(&self) -> Result<f64> {
        // Simulate network congestion check
        Ok(rand::random::<f64>() * 0.8) // Variable congestion
    }

    async fn fetch_pool_liquidity(&self, pool: Pubkey) -> Result<PoolLiquidity> {
        // Simulate fetching pool liquidity data
        Ok(PoolLiquidity {
            pool_address: pool,
            token_x_liquidity: 50_000_000_000_000, // 50K tokens
            token_y_liquidity: 5_000_000_000_000,  // 5K tokens  
            total_liquidity_usd: 500_000.0 + (rand::random::<f64>() * 1_000_000.0),
            volume_24h: 100_000.0 + (rand::random::<f64>() * 500_000.0),
            fee_tier: 25, // 0.25%
            last_updated: Utc::now(),
        })
    }

    async fn calculate_route_score(&self, liquidity: &PoolLiquidity, amount_in: u64, conditions: &MarketConditions) -> Result<f64> {
        let liquidity_score = (liquidity.total_liquidity_usd / 1_000_000.0).min(1.0);
        let volume_score = (liquidity.volume_24h / 1_000_000.0).min(1.0);
        let size_compatibility = if amount_in as f64 > liquidity.total_liquidity_usd * 0.1 { 0.5 } else { 1.0 };
        
        Ok(liquidity_score * 0.4 + volume_score * 0.4 + size_compatibility * 0.2)
    }

    async fn calculate_allocation_ratio(&self, _liquidity: &PoolLiquidity, _total_amount: u64, _allocated: u64) -> Result<f64> {
        Ok(0.5 + (rand::random::<f64>() * 0.3)) // 50-80% allocation
    }

    async fn calculate_expected_output(&self, liquidity: &PoolLiquidity, amount_in: u64) -> Result<u64> {
        // Simplified constant product formula simulation
        let price_impact = self.calculate_price_impact(liquidity, amount_in).await?;
        let base_rate = liquidity.token_y_liquidity as f64 / liquidity.token_x_liquidity as f64;
        let effective_rate = base_rate * (1.0 - price_impact / 100.0);
        Ok((amount_in as f64 * effective_rate * 0.9975) as u64) // Include 0.25% fee
    }

    async fn calculate_price_impact(&self, liquidity: &PoolLiquidity, amount_in: u64) -> Result<f64> {
        let impact = (amount_in as f64 / liquidity.token_x_liquidity as f64) * 100.0;
        Ok(impact.min(50.0)) // Cap at 50% impact
    }

    async fn calculate_route_fee(&self, liquidity: &PoolLiquidity, amount_in: u64) -> Result<u64> {
        Ok((amount_in as f64 * liquidity.fee_tier as f64 / 10_000.0) as u64)
    }

    async fn calculate_total_price_impact(&self, liquidities: &[PoolLiquidity]) -> Result<f64> {
        let avg_impact = liquidities.iter()
            .map(|l| (rand::random::<f64>() * 2.0)) // 0-2% impact per pool
            .sum::<f64>() / liquidities.len() as f64;
        Ok(avg_impact)
    }
}

impl Default for GasPriceTracker {
    fn default() -> Self {
        Self {
            current_gas_price: 5000, // 5000 lamports
            gas_price_history: Vec::new(),
            optimal_gas_price: 4800,
            congestion_level: 0.5,
        }
    }
}

impl Default for MarketConditions {
    fn default() -> Self {
        Self {
            overall_volatility: 0.03,
            market_sentiment: 0.6,
            liquidity_conditions: 0.8,
            network_congestion: 0.4,
            last_updated: Utc::now(),
        }
    }
}

/// Performance benchmark for optimizer
pub async fn benchmark_optimizer() -> Result<()> {
    log::info!("🏁 Starting optimizer performance benchmark");
    
    let optimizer = SwapOptimizer::new();
    let token_in = Pubkey::new_unique();
    let token_out = Pubkey::new_unique();
    let pools: Vec<Pubkey> = (0..10).map(|_| Pubkey::new_unique()).collect();
    
    let start = Instant::now();
    let iterations = 100;
    
    for i in 0..iterations {
        let amount = 1_000_000_000 + (i * 100_000_000); // Varying amounts
        let _result = optimizer.optimize_swap(token_in, token_out, amount, &pools).await?;
    }
    
    let duration = start.elapsed();
    let avg_time = duration.as_millis() as f64 / iterations as f64;
    
    let stats = optimizer.get_stats().await;
    
    log::info!("📊 Benchmark Results:");
    log::info!("   Total optimizations: {}", stats.total_optimizations);
    log::info!("   Average time per optimization: {:.2}ms", avg_time);
    log::info!("   Success rate: {:.1}%", 
               stats.successful_optimizations as f64 / stats.total_optimizations as f64 * 100.0);
    log::info!("   Cache hit rate: {:.1}%", stats.cache_hit_rate * 100.0);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_optimizer_creation() {
        let optimizer = SwapOptimizer::new();
        assert_eq!(optimizer.config.max_slippage_bps, 300);
        assert_eq!(optimizer.config.min_slippage_bps, 10);
    }

    #[tokio::test]
    async fn test_pool_liquidity_caching() {
        let pool = Pubkey::new_unique();
        let liquidity = PoolLiquidity {
            pool_address: pool,
            token_x_liquidity: 1000000,
            token_y_liquidity: 1000000,
            total_liquidity_usd: 100000.0,
            volume_24h: 50000.0,
            fee_tier: 25,
            last_updated: Utc::now(),
        };

        // Test caching
        if let Ok(mut cache) = LIQUIDITY_CACHE.write() {
            cache.put(pool, liquidity.clone());
            assert!(cache.get(&pool).is_some());
        }
    }

    #[tokio::test]
    async fn test_optimization_stats() {
        let optimizer = SwapOptimizer::new();
        optimizer.update_optimization_stats(Duration::from_millis(100), true).await;
        
        let stats = optimizer.get_stats().await;
        assert_eq!(stats.total_optimizations, 1);
        assert_eq!(stats.successful_optimizations, 1);
        assert!(stats.average_optimization_time_ms > 0.0);
    }
}