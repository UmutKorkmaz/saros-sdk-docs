//! Position performance analysis and tracking

use anyhow::Result;
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use log::{debug, info, warn};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use solana_sdk::pubkey::Pubkey;
use statrs::statistics::{Statistics, OrderStatistics};
use std::collections::{BTreeMap, VecDeque};

use saros_dlmm_sdk::{DLMMClient as MockSarosClient, Position as SdkPosition, DLMMPoolInfo};
use crate::types::{
    PositionAnalysis, PositionInfo, FeeAnalysis, RiskMetrics, PerformanceSummary,
    HistoricalTrends, TrendDirection, RecoveryPeriod, PoolInfo, ImpermanentLossResult, PriceDataPoint,
    PositionSnapshot, ILError,
};

/// Analyzer for position performance and risk metrics
pub struct PositionAnalyzer {
    client: MockSarosClient,
    position_cache: BTreeMap<Pubkey, (SdkPosition, DateTime<Utc>)>,
    pool_cache: BTreeMap<Pubkey, (DLMMPoolInfo, DateTime<Utc>)>,
    historical_snapshots: BTreeMap<Pubkey, VecDeque<PositionSnapshot>>,
    cache_ttl_secs: u64,
}

impl PositionAnalyzer {
    /// Create a new position analyzer
    pub async fn new() -> Result<Self> {
        let rpc_url = std::env::var("RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        
        let client = MockSarosClient::new(&rpc_url)?;
        
        Ok(Self {
            client,
            position_cache: BTreeMap::new(),
            pool_cache: BTreeMap::new(),
            historical_snapshots: BTreeMap::new(),
            cache_ttl_secs: 60, // Cache for 1 minute
        })
    }

    /// Get comprehensive pool information
    pub async fn get_pool_info(&mut self, pool_address: Pubkey) -> Result<PoolInfo> {
        // Check cache first
        if let Some((cached_pool, cached_time)) = self.pool_cache.get(&pool_address) {
            if Utc::now().signed_duration_since(*cached_time).num_seconds() < self.cache_ttl_secs as i64 {
                return Ok(self.convert_pool_info(cached_pool));
            }
        }

        info!("Fetching pool info for {}", pool_address);
        let pool_info = self.client.get_pool(pool_address).await?;
        
        // Cache the result
        self.pool_cache.insert(pool_address, (pool_info.clone(), Utc::now()));
        
        Ok(self.convert_pool_info(&pool_info))
    }

    /// Get detailed position information
    pub async fn get_position_info(&mut self, position_id: Pubkey) -> Result<PositionInfo> {
        // Check cache first
        if let Some((cached_position, cached_time)) = self.position_cache.get(&position_id) {
            if Utc::now().signed_duration_since(*cached_time).num_seconds() < self.cache_ttl_secs as i64 {
                return Ok(self.convert_position_info(cached_position).await?);
            }
        }

        info!("Fetching position info for {}", position_id);
        let position = self.client.get_position(position_id).await?;
        
        // Cache the result
        self.position_cache.insert(position_id, (position.clone(), Utc::now()));
        
        Ok(self.convert_position_info(&position).await?)
    }

    /// Analyze comprehensive position performance
    pub async fn analyze_position_performance(
        &mut self,
        pool_address: Pubkey,
        position_data: Option<PositionInfo>,
        il_result: &ImpermanentLossResult,
    ) -> Result<PositionAnalysis> {
        info!("Analyzing position performance for pool {}", pool_address);

        let pool_info = self.get_pool_info(pool_address).await?;
        
        let position_info = if let Some(pos) = position_data {
            pos
        } else {
            // Create a synthetic position info for pool-level analysis
            PositionInfo {
                position_id: None,
                pool_address,
                owner: Pubkey::default(),
                token_x_symbol: pool_info.token_x_symbol.clone(),
                token_y_symbol: pool_info.token_y_symbol.clone(),
                lower_bin_id: 0,
                upper_bin_id: 0,
                current_liquidity: pool_info.total_liquidity,
                initial_investment_usd: il_result.hold_value_usd,
                current_value_usd: il_result.current_value_usd,
                created_at: Utc::now() - ChronoDuration::days(30), // Assume 30 days old
                last_updated: Utc::now(),
            }
        };

        // Analyze fees
        let fee_analysis = self.analyze_fees(&position_info, &pool_info, il_result).await?;
        
        // Calculate risk metrics
        let risk_metrics = self.calculate_risk_metrics(&position_info, &pool_info, il_result).await?;
        
        // Generate performance summary
        let performance_summary = self.calculate_performance_summary(&position_info, &fee_analysis, il_result)?;
        
        Ok(PositionAnalysis {
            position_info,
            il_result: il_result.clone(),
            fee_analysis,
            risk_metrics,
            performance_summary,
            timestamp: Utc::now(),
        })
    }

    /// Analyze historical trends from IL data
    pub async fn analyze_historical_trends(
        &self,
        il_history: &[ImpermanentLossResult],
        price_history: &[PriceDataPoint],
    ) -> Result<HistoricalTrends> {
        if il_history.is_empty() || price_history.is_empty() {
            return Err(ILError::InsufficientData("No historical data provided".to_string()).into());
        }

        info!("Analyzing historical trends for {} data points", il_history.len());

        let period_days = if let (Some(first), Some(last)) = (price_history.first(), price_history.last()) {
            (last.timestamp - first.timestamp).num_days() as u32
        } else {
            0
        };

        // Extract IL percentages for statistical analysis
        let il_percentages: Vec<f64> = il_history.iter()
            .map(|il| il.il_percentage.to_f64().unwrap_or(0.0))
            .collect();

        let max_il = il_percentages.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_il = il_percentages.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let avg_il = il_percentages.clone().mean();
        let il_volatility = il_percentages.clone().std_dev();

        // Calculate price changes
        let price_changes: Vec<f64> = price_history.windows(2)
            .map(|window| {
                let prev_ratio = window[0].price_x / window[0].price_y;
                let curr_ratio = window[1].price_x / window[1].price_y;
                ((curr_ratio / prev_ratio) - Decimal::ONE).to_f64().unwrap_or(0.0)
            })
            .collect();

        let avg_daily_price_change = if !price_changes.is_empty() {
            Decimal::from_f64(price_changes.clone().mean()).unwrap_or_default()
        } else {
            Decimal::ZERO
        };

        // Determine trend direction
        let trend_direction = self.determine_trend_direction(&il_percentages, &price_changes)?;
        
        // Find recovery periods
        let recovery_periods = self.find_recovery_periods(il_history)?;
        
        // Calculate market correlation (simplified)
        let market_correlation = self.calculate_market_correlation(&il_percentages, &price_changes)?;
        
        // Find maximum drawdown period
        let max_drawdown_days = self.calculate_max_drawdown_period(&il_percentages)?;

        Ok(HistoricalTrends {
            period_days,
            max_il_percentage: Decimal::from_f64(max_il).unwrap_or_default(),
            min_il_percentage: Decimal::from_f64(min_il).unwrap_or_default(),
            avg_il_percentage: Decimal::from_f64(avg_il).unwrap_or_default(),
            il_volatility: Decimal::from_f64(il_volatility).unwrap_or_default(),
            market_correlation: Decimal::from_f64(market_correlation).unwrap_or_default(),
            trend_direction,
            avg_daily_price_change,
            max_drawdown_days,
            recovery_periods,
        })
    }

    /// Analyze fee earnings and efficiency
    async fn analyze_fees(
        &self,
        position: &PositionInfo,
        pool_info: &PoolInfo,
        il_result: &ImpermanentLossResult,
    ) -> Result<FeeAnalysis> {
        // Calculate fees based on pool performance and position duration
        let days_active = (Utc::now() - position.created_at).num_days().max(1) as f64;
        
        // Estimate fees based on pool volume and position size
        let daily_volume_ratio = pool_info.volume_24h / pool_info.tvl;
        let position_share = position.current_liquidity / pool_info.total_liquidity;
        let estimated_daily_fees = pool_info.fees_24h * position_share;
        let total_fees_earned = estimated_daily_fees * Decimal::from_f64(days_active).unwrap_or_default();
        
        // Split fees between tokens (simplified 50/50)
        let fees_token_x = total_fees_earned / Decimal::new(2, 0) / il_result.current_price_x;
        let fees_token_y = total_fees_earned / Decimal::new(2, 0) / il_result.current_price_y;
        
        // Calculate APY based on fees
        let fee_apy = if position.initial_investment_usd > Decimal::ZERO && days_active > 0.0 {
            (total_fees_earned / position.initial_investment_usd) * Decimal::new(365, 0) / Decimal::from_f64(days_active).unwrap_or(Decimal::ONE)
        } else {
            Decimal::ZERO
        };
        
        let daily_fee_rate = if position.initial_investment_usd > Decimal::ZERO {
            estimated_daily_fees / position.initial_investment_usd
        } else {
            Decimal::ZERO
        };
        
        // Fee vs IL ratio
        let fee_vs_il_ratio = if il_result.il_usd_value.abs() > Decimal::ZERO {
            total_fees_earned / il_result.il_usd_value.abs()
        } else {
            Decimal::ZERO
        };
        
        // Break-even analysis
        let break_even_days = if daily_fee_rate > Decimal::ZERO && il_result.il_usd_value < Decimal::ZERO {
            let days_to_recover = il_result.il_usd_value.abs() / estimated_daily_fees;
            Some(days_to_recover.to_u32().unwrap_or(u32::MAX))
        } else {
            None
        };
        
        Ok(FeeAnalysis {
            total_fees_earned,
            fees_token_x,
            fees_token_y,
            fee_apy,
            daily_fee_rate,
            fee_vs_il_ratio,
            break_even_days,
        })
    }

    /// Calculate risk metrics for the position
    async fn calculate_risk_metrics(
        &self,
        position: &PositionInfo,
        _pool_info: &PoolInfo,
        il_result: &ImpermanentLossResult,
    ) -> Result<RiskMetrics> {
        // Mock implementation - in reality would analyze historical data
        let price_volatility = self.estimate_price_volatility(il_result)?;
        
        // Maximum IL observed (use current as proxy)
        let max_il_observed = il_result.il_percentage.abs();
        
        // Value at Risk (95% confidence) - simplified calculation
        let var_95 = position.current_value_usd * Decimal::new(5, 2); // 5% VaR
        
        // Sharpe ratio (simplified)
        let sharpe_ratio = if price_volatility > Decimal::ZERO {
            il_result.il_percentage / price_volatility
        } else {
            Decimal::ZERO
        };
        
        // Concentration risk based on bin range
        let bin_range = position.upper_bin_id - position.lower_bin_id;
        let concentration_risk = if bin_range > 0 {
            Decimal::ONE / Decimal::new(bin_range as i64, 0)
        } else {
            Decimal::ONE
        };
        
        // Bin utilization (mock)
        let bin_utilization = Decimal::new(75, 2); // Assume 75% utilization
        
        Ok(RiskMetrics {
            price_volatility,
            max_il_observed,
            var_95,
            sharpe_ratio,
            concentration_risk,
            bin_utilization,
        })
    }

    /// Calculate performance summary
    fn calculate_performance_summary(
        &self,
        position: &PositionInfo,
        fee_analysis: &FeeAnalysis,
        il_result: &ImpermanentLossResult,
    ) -> Result<PerformanceSummary> {
        let total_return_usd = fee_analysis.total_fees_earned + il_result.il_usd_value;
        let total_return_percentage = if position.initial_investment_usd > Decimal::ZERO {
            total_return_usd / position.initial_investment_usd
        } else {
            Decimal::ZERO
        };
        
        let days_active = (Utc::now() - position.created_at).num_days().max(1) as u32;
        let annualized_return = if days_active > 0 {
            total_return_percentage * Decimal::new(365, 0) / Decimal::new(days_active as i64, 0)
        } else {
            Decimal::ZERO
        };
        
        // Simplified gas costs estimation
        let estimated_gas_costs = Decimal::new(50, 0); // $50 in gas costs
        let net_pnl = total_return_usd - estimated_gas_costs;
        
        // Performance vs holding tokens
        let vs_hold_performance = if il_result.hold_value_usd > position.initial_investment_usd {
            (il_result.current_value_usd - position.initial_investment_usd) - (il_result.hold_value_usd - position.initial_investment_usd)
        } else {
            il_result.il_usd_value
        };
        
        Ok(PerformanceSummary {
            total_return_usd,
            total_return_percentage,
            annualized_return,
            net_pnl,
            days_active,
            vs_hold_performance,
            vs_market_performance: None, // Would need market benchmark data
        })
    }

    /// Estimate price volatility from IL data
    fn estimate_price_volatility(&self, il_result: &ImpermanentLossResult) -> Result<Decimal> {
        // Simplified volatility estimation based on price ratio change
        let price_change = (il_result.price_ratio_change - Decimal::ONE).abs();
        Ok(price_change * Decimal::new(2, 0)) // Rough estimate
    }

    /// Determine overall trend direction from data
    fn determine_trend_direction(&self, il_data: &[f64], price_changes: &[f64]) -> Result<TrendDirection> {
        if il_data.is_empty() || price_changes.is_empty() {
            return Ok(TrendDirection::Sideways);
        }

        let il_trend = il_data.last().unwrap() - il_data.first().unwrap();
        let price_volatility = price_changes.std_dev();
        
        if price_volatility > 0.1 { // High volatility threshold
            Ok(TrendDirection::Volatile)
        } else if il_trend > 0.02 { // IL increasing (bad)
            Ok(TrendDirection::Bearish)
        } else if il_trend < -0.02 { // IL decreasing (good)
            Ok(TrendDirection::Bullish)
        } else {
            Ok(TrendDirection::Sideways)
        }
    }

    /// Find recovery periods after significant IL events
    fn find_recovery_periods(&self, il_history: &[ImpermanentLossResult]) -> Result<Vec<RecoveryPeriod>> {
        let mut recovery_periods = Vec::new();
        let mut in_recovery = false;
        let mut recovery_start: Option<DateTime<Utc>> = None;
        let mut max_il_in_period = Decimal::ZERO;
        
        const IL_THRESHOLD: f64 = -0.05; // -5% IL threshold
        
        for il_result in il_history {
            let il_percentage = il_result.il_percentage.to_f64().unwrap_or(0.0);
            
            if !in_recovery && il_percentage < IL_THRESHOLD {
                // Start of a significant IL period
                in_recovery = true;
                recovery_start = Some(il_result.timestamp);
                max_il_in_period = il_result.il_percentage.abs();
            } else if in_recovery {
                max_il_in_period = max_il_in_period.max(il_result.il_percentage.abs());
                
                if il_percentage > IL_THRESHOLD {
                    // Recovery detected
                    if let Some(start) = recovery_start {
                        let recovery_days = (il_result.timestamp - start).num_days() as u32;
                        
                        recovery_periods.push(RecoveryPeriod {
                            start_date: start,
                            end_date: il_result.timestamp,
                            max_il_in_period,
                            recovery_days,
                            fee_compensation: Decimal::ZERO, // Would calculate from fee data
                        });
                    }
                    
                    in_recovery = false;
                    recovery_start = None;
                    max_il_in_period = Decimal::ZERO;
                }
            }
        }
        
        Ok(recovery_periods)
    }

    /// Calculate correlation between IL and market movements
    fn calculate_market_correlation(&self, il_data: &[f64], price_changes: &[f64]) -> Result<f64> {
        if il_data.len() != price_changes.len() || il_data.len() < 2 {
            return Ok(0.0);
        }

        // Calculate Pearson correlation coefficient
        let il_mean = il_data.mean();
        let price_mean = price_changes.mean();
        
        let mut numerator = 0.0;
        let mut il_sum_sq = 0.0;
        let mut price_sum_sq = 0.0;
        
        for i in 0..il_data.len() {
            let il_diff = il_data[i] - il_mean;
            let price_diff = price_changes[i] - price_mean;
            
            numerator += il_diff * price_diff;
            il_sum_sq += il_diff * il_diff;
            price_sum_sq += price_diff * price_diff;
        }
        
        let denominator = (il_sum_sq * price_sum_sq).sqrt();
        
        if denominator > 0.0 {
            Ok(numerator / denominator)
        } else {
            Ok(0.0)
        }
    }

    /// Calculate maximum drawdown period in days
    fn calculate_max_drawdown_period(&self, il_data: &[f64]) -> Result<u32> {
        let mut max_drawdown_days = 0u32;
        let mut current_drawdown_days = 0u32;
        let mut peak_value = f64::NEG_INFINITY;
        
        for &il_value in il_data {
            if il_value > peak_value {
                peak_value = il_value;
                current_drawdown_days = 0;
            } else {
                current_drawdown_days += 1;
                max_drawdown_days = max_drawdown_days.max(current_drawdown_days);
            }
        }
        
        Ok(max_drawdown_days)
    }

    /// Convert SDK pool info to our PoolInfo type
    fn convert_pool_info(&self, pool: &DLMMPoolInfo) -> PoolInfo {
        PoolInfo {
            address: pool.address,
            token_x: pool.token_x,
            token_y: pool.token_y,
            token_x_symbol: "TOKX".to_string(), // Would get from token metadata
            token_y_symbol: "TOKY".to_string(), // Would get from token metadata
            active_bin_id: pool.active_bin_id,
            bin_step: pool.bin_step,
            total_liquidity: Decimal::new(pool.liquidity as i64, 12),
            volume_24h: Decimal::new(pool.volume_24h as i64, 12),
            fees_24h: Decimal::new(pool.fees_24h as i64, 12),
            tvl: Decimal::from_f64(pool.apr * 100000.0).unwrap_or_default(),
            created_at: Utc::now() - ChronoDuration::days(365), // Mock creation date
        }
    }

    /// Convert SDK position to our PositionInfo type
    async fn convert_position_info(&self, position: &SdkPosition) -> Result<PositionInfo> {
        Ok(PositionInfo {
            position_id: Some(position.id),
            pool_address: position.pool_address,
            owner: position.owner,
            token_x_symbol: "TOKX".to_string(), // Would get from token metadata
            token_y_symbol: "TOKY".to_string(), // Would get from token metadata
            lower_bin_id: position.lower_bin_id,
            upper_bin_id: position.upper_bin_id,
            current_liquidity: Decimal::new(position.liquidity as i64, 6),
            initial_investment_usd: Decimal::from_f64(position.value_usd).unwrap_or_default(),
            current_value_usd: Decimal::from_f64(position.value_usd).unwrap_or_default(),
            created_at: Utc::now() - ChronoDuration::days(30), // Mock creation date
            last_updated: Utc::now(),
        })
    }

    /// Add a historical snapshot for tracking
    pub fn add_position_snapshot(&mut self, position_id: Pubkey, snapshot: PositionSnapshot) {
        let snapshots = self.historical_snapshots.entry(position_id).or_insert_with(VecDeque::new);
        
        // Keep only last 1000 snapshots to manage memory
        if snapshots.len() >= 1000 {
            snapshots.pop_front();
        }
        
        snapshots.push_back(snapshot);
    }

    /// Get historical snapshots for a position
    pub fn get_position_snapshots(&self, position_id: Pubkey) -> Option<&VecDeque<PositionSnapshot>> {
        self.historical_snapshots.get(&position_id)
    }

    /// Clear expired cache entries
    pub fn clear_expired_cache(&mut self) {
        let now = Utc::now();
        
        self.position_cache.retain(|_, (_, timestamp)| {
            now.signed_duration_since(*timestamp).num_seconds() < self.cache_ttl_secs as i64
        });
        
        self.pool_cache.retain(|_, (_, timestamp)| {
            now.signed_duration_since(*timestamp).num_seconds() < self.cache_ttl_secs as i64
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_trend_direction_determination() {
        let analyzer = PositionAnalyzer::new().await.unwrap();
        
        // Test bullish trend (IL decreasing)
        let il_data = vec![-0.1, -0.08, -0.06, -0.04, -0.02];
        let price_changes = vec![0.01, 0.02, 0.01, 0.015, 0.01];
        
        let trend = analyzer.determine_trend_direction(&il_data, &price_changes).unwrap();
        assert!(matches!(trend, TrendDirection::Bullish));
        
        // Test volatile trend
        let volatile_prices = vec![0.15, -0.12, 0.18, -0.14, 0.16];
        let trend = analyzer.determine_trend_direction(&il_data, &volatile_prices).unwrap();
        assert!(matches!(trend, TrendDirection::Volatile));
    }

    #[tokio::test] 
    async fn test_market_correlation() {
        let analyzer = PositionAnalyzer::new().await.unwrap();
        
        // Perfect positive correlation
        let il_data = vec![0.0, 0.1, 0.2, 0.3, 0.4];
        let price_changes = vec![0.0, 0.1, 0.2, 0.3, 0.4];
        
        let correlation = analyzer.calculate_market_correlation(&il_data, &price_changes).unwrap();
        assert!((correlation - 1.0).abs() < 0.001);
        
        // Perfect negative correlation
        let negative_changes = vec![0.0, -0.1, -0.2, -0.3, -0.4];
        let correlation = analyzer.calculate_market_correlation(&il_data, &negative_changes).unwrap();
        assert!((correlation - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_max_drawdown_calculation() {
        let analyzer = PositionAnalyzer::new().await.unwrap();
        
        // Data with a 3-day drawdown
        let il_data = vec![0.0, -0.02, -0.05, -0.08, -0.06, -0.03, 0.01];
        let max_drawdown = analyzer.calculate_max_drawdown_period(&il_data).unwrap();
        
        // Should detect the drawdown from day 2 to day 6 (4 days)
        assert!(max_drawdown >= 3);
    }
}