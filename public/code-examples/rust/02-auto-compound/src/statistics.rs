use anyhow::Result;
use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{CompoundResult, GlobalStatistics, PoolStatistics};

/// Manages statistics for all compound operations
pub struct StatisticsManager {
    global_stats: GlobalStatistics,
    pool_stats: HashMap<String, PoolStatistics>,
    compound_history: Vec<CompoundHistoryEntry>,
    start_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundHistoryEntry {
    pub pool_address: String,
    pub result: CompoundResult,
    pub apy_before: f64,
    pub apy_after: f64,
    pub position_growth: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub global_stats: GlobalStatistics,
    pub pool_performance: Vec<PoolPerformance>,
    pub top_performers: Vec<String>,
    pub recommendations: Vec<String>,
    pub report_generated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolPerformance {
    pub pool_address: String,
    pub stats: PoolStatistics,
    pub efficiency_score: f64,
    pub compound_frequency_optimal: bool,
    pub recent_trend: PerformanceTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceTrend {
    Improving,
    Stable,
    Declining,
}

impl StatisticsManager {
    pub fn new() -> Self {
        Self {
            global_stats: GlobalStatistics::default(),
            pool_stats: HashMap::new(),
            compound_history: Vec::new(),
            start_time: Utc::now(),
        }
    }

    /// Record a compound operation result
    pub async fn record_compound_result(&mut self, pool_address: &str, result: &CompoundResult) {
        info!("📊 Recording compound result for pool: {}", pool_address);

        // Update global statistics
        self.global_stats.total_compounds += 1;
        
        if result.success {
            self.global_stats.successful_compounds += 1;
            self.global_stats.total_rewards_harvested += result.rewards_harvested;
            self.global_stats.total_reinvested += result.amount_reinvested;
        } else {
            self.global_stats.failed_compounds += 1;
        }

        self.global_stats.total_gas_spent += result.gas_used;
        self.global_stats.success_rate = if self.global_stats.total_compounds > 0 {
            (self.global_stats.successful_compounds as f64 / self.global_stats.total_compounds as f64) * 100.0
        } else {
            0.0
        };
        
        self.global_stats.net_profit = self.global_stats.total_rewards_harvested - self.global_stats.total_gas_spent;
        self.global_stats.last_compound_time = Some(result.timestamp);

        // Update pool-specific statistics
        let pool_stats = self.pool_stats.entry(pool_address.to_string()).or_insert_with(|| {
            PoolStatistics {
                pool_address: pool_address.parse().unwrap_or_default(),
                ..Default::default()
            }
        });

        pool_stats.compounds += 1;
        
        if result.success {
            pool_stats.total_harvested += result.rewards_harvested;
            pool_stats.total_reinvested += result.amount_reinvested;
        }
        
        pool_stats.total_gas += result.gas_used;
        pool_stats.last_compound = Some(result.timestamp);
        
        pool_stats.average_reward = if pool_stats.compounds > 0 {
            pool_stats.total_harvested / pool_stats.compounds as f64
        } else {
            0.0
        };

        // Add to history
        let history_entry = CompoundHistoryEntry {
            pool_address: pool_address.to_string(),
            result: result.clone(),
            apy_before: 0.0, // Would be calculated from position data
            apy_after: 0.0,  // Would be calculated after compound
            position_growth: 0.0,
        };
        
        self.compound_history.push(history_entry);

        // Limit history to last 1000 entries
        if self.compound_history.len() > 1000 {
            self.compound_history.remove(0);
        }

        info!("✅ Statistics updated - Global compounds: {}, Success rate: {:.1}%", 
              self.global_stats.total_compounds, self.global_stats.success_rate);
    }

    /// Get global statistics
    pub async fn get_global_statistics(&self) -> GlobalStatistics {
        let mut stats = self.global_stats.clone();
        
        // Calculate uptime
        let uptime = Utc::now() - self.start_time;
        stats.uptime_hours = uptime.num_minutes() as f64 / 60.0;
        
        // Calculate average APY boost
        stats.average_apy_boost = self.calculate_average_apy_boost().await;
        
        stats
    }

    /// Get statistics for a specific pool
    pub async fn get_pool_statistics(&self, pool_address: &str) -> Option<PoolStatistics> {
        self.pool_stats.get(pool_address).cloned()
    }

    /// Get all pool statistics
    pub async fn get_all_pool_statistics(&self) -> Vec<PoolStatistics> {
        self.pool_stats.values().cloned().collect()
    }

    /// Calculate average APY boost from compounding
    async fn calculate_average_apy_boost(&self) -> f64 {
        if self.compound_history.is_empty() {
            return 0.0;
        }

        let total_boost: f64 = self.compound_history
            .iter()
            .filter(|entry| entry.apy_after > entry.apy_before)
            .map(|entry| entry.apy_after - entry.apy_before)
            .sum();

        total_boost / self.compound_history.len() as f64
    }

    /// Generate comprehensive performance report
    pub async fn generate_performance_report(&self) -> PerformanceReport {
        info!("📈 Generating performance report");

        let mut pool_performances = Vec::new();
        let mut top_performers = Vec::new();
        let mut recommendations = Vec::new();

        // Analyze each pool's performance
        for (pool_address, stats) in &self.pool_stats {
            let efficiency_score = self.calculate_efficiency_score(stats);
            let frequency_optimal = self.is_compound_frequency_optimal(pool_address).await;
            let trend = self.analyze_performance_trend(pool_address).await;

            let performance = PoolPerformance {
                pool_address: pool_address.clone(),
                stats: stats.clone(),
                efficiency_score,
                compound_frequency_optimal: frequency_optimal,
                recent_trend: trend,
            };

            pool_performances.push(performance);

            // Track top performers
            if efficiency_score > 80.0 {
                top_performers.push(pool_address.clone());
            }

            // Generate recommendations
            if efficiency_score < 50.0 {
                recommendations.push(format!("Consider adjusting compound frequency for pool {}", pool_address));
            }

            if stats.total_gas > stats.total_harvested * 0.1 {
                recommendations.push(format!("High gas costs detected for pool {} - consider increasing minimum threshold", pool_address));
            }
        }

        // Global recommendations
        if self.global_stats.success_rate < 80.0 {
            recommendations.push("Overall success rate is below 80% - review gas optimization settings".to_string());
        }

        if self.global_stats.net_profit < 0.0 {
            recommendations.push("Net profit is negative - review reward thresholds and gas costs".to_string());
        }

        // Sort top performers by efficiency score
        pool_performances.sort_by(|a, b| b.efficiency_score.partial_cmp(&a.efficiency_score).unwrap_or(std::cmp::Ordering::Equal));
        top_performers = pool_performances.iter()
            .take(5)
            .map(|p| p.pool_address.clone())
            .collect();

        PerformanceReport {
            global_stats: self.get_global_statistics().await,
            pool_performance: pool_performances,
            top_performers,
            recommendations,
            report_generated: Utc::now(),
        }
    }

    /// Calculate efficiency score for a pool (0-100)
    fn calculate_efficiency_score(&self, stats: &PoolStatistics) -> f64 {
        if stats.compounds == 0 {
            return 0.0;
        }

        let success_rate = if stats.total_harvested > 0.0 { 100.0 } else { 0.0 }; // Simplified
        let gas_efficiency = if stats.total_gas > 0.0 {
            (stats.total_harvested / stats.total_gas * 10.0).min(100.0)
        } else {
            100.0
        };
        let consistency = if stats.compounds > 5 { 100.0 } else { stats.compounds as f64 * 20.0 };

        (success_rate * 0.4 + gas_efficiency * 0.4 + consistency * 0.2).min(100.0)
    }

    /// Check if compound frequency is optimal for a pool
    async fn is_compound_frequency_optimal(&self, pool_address: &str) -> bool {
        // Get recent compound history for this pool
        let recent_compounds: Vec<_> = self.compound_history
            .iter()
            .filter(|entry| entry.pool_address == pool_address)
            .rev()
            .take(10)
            .collect();

        if recent_compounds.len() < 3 {
            return true; // Not enough data
        }

        // Check if gas costs are reasonable compared to rewards
        let avg_gas_ratio: f64 = recent_compounds
            .iter()
            .filter(|entry| entry.result.rewards_harvested > 0.0)
            .map(|entry| entry.result.gas_used / entry.result.rewards_harvested)
            .sum::<f64>() / recent_compounds.len() as f64;

        // Consider optimal if gas is less than 5% of rewards
        avg_gas_ratio < 0.05
    }

    /// Analyze performance trend for a pool
    async fn analyze_performance_trend(&self, pool_address: &str) -> PerformanceTrend {
        let recent_compounds: Vec<_> = self.compound_history
            .iter()
            .filter(|entry| entry.pool_address == pool_address)
            .rev()
            .take(10)
            .collect();

        if recent_compounds.len() < 5 {
            return PerformanceTrend::Stable;
        }

        // Calculate trend based on position growth over time
        let recent_avg = recent_compounds[..5].iter()
            .map(|entry| entry.position_growth)
            .sum::<f64>() / 5.0;

        let older_avg = recent_compounds[5..].iter()
            .map(|entry| entry.position_growth)
            .sum::<f64>() / (recent_compounds.len() - 5) as f64;

        if recent_avg > older_avg * 1.1 {
            PerformanceTrend::Improving
        } else if recent_avg < older_avg * 0.9 {
            PerformanceTrend::Declining
        } else {
            PerformanceTrend::Stable
        }
    }

    /// Get compound history for a specific pool
    pub async fn get_compound_history(&self, pool_address: &str, limit: usize) -> Vec<CompoundHistoryEntry> {
        self.compound_history
            .iter()
            .filter(|entry| entry.pool_address == pool_address)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Calculate ROI for a pool
    pub async fn calculate_roi(&self, pool_address: &str) -> f64 {
        if let Some(stats) = self.pool_stats.get(pool_address) {
            if stats.total_gas > 0.0 {
                ((stats.total_harvested - stats.total_gas) / stats.total_gas) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Get performance metrics for dashboard
    pub async fn get_dashboard_metrics(&self) -> DashboardMetrics {
        let global_stats = self.get_global_statistics().await;
        let total_pools = self.pool_stats.len();
        let active_pools = self.pool_stats
            .values()
            .filter(|stats| stats.last_compound.map(|last| {
                (Utc::now() - last).num_hours() < 24
            }).unwrap_or(false))
            .count();

        let avg_efficiency = if !self.pool_stats.is_empty() {
            self.pool_stats
                .values()
                .map(|stats| self.calculate_efficiency_score(stats))
                .sum::<f64>() / self.pool_stats.len() as f64
        } else {
            0.0
        };

        DashboardMetrics {
            total_pools,
            active_pools,
            global_success_rate: global_stats.success_rate,
            total_profit: global_stats.net_profit,
            average_efficiency: avg_efficiency,
            uptime_hours: global_stats.uptime_hours,
        }
    }

    /// Reset statistics (for testing or maintenance)
    pub async fn reset_statistics(&mut self) {
        info!("🔄 Resetting all statistics");
        
        self.global_stats = GlobalStatistics::default();
        self.pool_stats.clear();
        self.compound_history.clear();
        self.start_time = Utc::now();
    }

    /// Export statistics to JSON
    pub async fn export_statistics(&self) -> Result<String> {
        let export_data = StatisticsExport {
            global_stats: self.global_stats.clone(),
            pool_stats: self.pool_stats.clone(),
            compound_history: self.compound_history.clone(),
            exported_at: Utc::now(),
        };

        Ok(serde_json::to_string_pretty(&export_data)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub total_pools: usize,
    pub active_pools: usize,
    pub global_success_rate: f64,
    pub total_profit: f64,
    pub average_efficiency: f64,
    pub uptime_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatisticsExport {
    global_stats: GlobalStatistics,
    pool_stats: HashMap<String, PoolStatistics>,
    compound_history: Vec<CompoundHistoryEntry>,
    exported_at: DateTime<Utc>,
}