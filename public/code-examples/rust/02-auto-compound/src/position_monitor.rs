use anyhow::Result;
use chrono::Utc;
use log::{error, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use saros_dlmm_sdk::{DLMMClient, UserPosition};

use crate::types::{Position, PoolInfo};

/// Monitors positions and detects changes
pub struct PositionMonitor {
    rpc_client: Arc<RpcClient>,
    dlmm_client: DLMMClient,
    position_cache: Arc<RwLock<HashMap<String, Position>>>,
}

impl PositionMonitor {
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        let dlmm_client = DLMMClient::new_with_rpc(rpc_client.clone());

        Self {
            rpc_client,
            dlmm_client,
            position_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get current position for a pool
    pub async fn get_position(&self, pool_address: Pubkey) -> Result<Position> {
        info!("📊 Getting position for pool: {}", pool_address);

        // For demo purposes, use a placeholder user pubkey
        // In production, this would be the actual user's wallet
        let user_pubkey = Pubkey::new_unique();

        let user_position = self.dlmm_client.get_user_position(&pool_address, &user_pubkey).await?;

        let position = Position {
            pool_address,
            token_a_amount: user_position.token_a_amount,
            token_b_amount: user_position.token_b_amount,
            lp_token_amount: user_position.lp_token_amount,
            pending_rewards: user_position.pending_rewards,
            last_updated: Utc::now(),
        };

        // Update cache
        let mut cache = self.position_cache.write().await;
        cache.insert(pool_address.to_string(), position.clone());

        info!("✅ Position retrieved: {:.6} LP tokens", position.lp_token_amount);

        Ok(position)
    }

    /// Monitor positions for changes and return difference
    pub async fn check_position_changes(&self, pool_address: Pubkey) -> Result<Option<PositionChange>> {
        let current_position = self.get_position(pool_address).await?;
        
        let cache = self.position_cache.read().await;
        if let Some(previous_position) = cache.get(&pool_address.to_string()) {
            let change = self.calculate_position_change(previous_position, &current_position);
            if change.has_significant_change() {
                info!("📈 Position change detected for pool {}", pool_address);
                return Ok(Some(change));
            }
        }

        Ok(None)
    }

    /// Calculate position change between two positions
    fn calculate_position_change(&self, previous: &Position, current: &Position) -> PositionChange {
        PositionChange {
            pool_address: current.pool_address,
            token_a_change: current.token_a_amount - previous.token_a_amount,
            token_b_change: current.token_b_amount - previous.token_b_amount,
            lp_token_change: current.lp_token_amount - previous.lp_token_amount,
            rewards_change: current.pending_rewards - previous.pending_rewards,
            time_elapsed: (current.last_updated - previous.last_updated).num_seconds() as f64,
        }
    }

    /// Get pool information
    pub async fn get_pool_info(&self, pool_address: Pubkey) -> Result<PoolInfo> {
        info!("🏊 Getting pool info for: {}", pool_address);

        let pool_info = self.dlmm_client.get_pool_info(&pool_address).await?;

        let info = PoolInfo {
            address: pool_address,
            token_a_mint: pool_info.token_a_mint,
            token_b_mint: pool_info.token_b_mint,
            token_a_symbol: pool_info.token_a_symbol,
            token_b_symbol: pool_info.token_b_symbol,
            tvl: pool_info.tvl,
            apy: pool_info.apy,
            fee_rate: pool_info.fee_rate,
            is_active: pool_info.is_active,
        };

        info!("✅ Pool info: {} / {} (TVL: ${:.0}, APY: {:.2}%)", 
              info.token_a_symbol, info.token_b_symbol, info.tvl, info.apy);

        Ok(info)
    }

    /// Monitor multiple positions concurrently
    pub async fn monitor_positions(&self, pool_addresses: Vec<Pubkey>) -> Result<Vec<Position>> {
        info!("👀 Monitoring {} positions", pool_addresses.len());

        let futures: Vec<_> = pool_addresses
            .iter()
            .map(|&pool_address| self.get_position(pool_address))
            .collect();

        let results = futures::future::join_all(futures).await;
        
        let mut positions = Vec::new();
        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(position) => positions.push(position),
                Err(e) => {
                    error!("Failed to get position for pool {}: {}", pool_addresses[i], e);
                }
            }
        }

        info!("✅ Retrieved {} positions successfully", positions.len());
        Ok(positions)
    }

    /// Calculate position performance metrics
    pub async fn calculate_performance_metrics(&self, pool_address: Pubkey, duration_hours: f64) -> Result<PerformanceMetrics> {
        let current_position = self.get_position(pool_address).await?;
        let pool_info = self.get_pool_info(pool_address).await?;

        // Get historical position (from cache or estimate)
        let cache = self.position_cache.read().await;
        let historical_position = cache.get(&pool_address.to_string()).cloned()
            .unwrap_or_else(|| {
                // If no historical data, create an estimated historical position
                Position {
                    pool_address,
                    token_a_amount: current_position.token_a_amount * 0.95, // Assume 5% growth
                    token_b_amount: current_position.token_b_amount * 0.95,
                    lp_token_amount: current_position.lp_token_amount * 0.95,
                    pending_rewards: 0.0,
                    last_updated: Utc::now() - chrono::Duration::hours(duration_hours as i64),
                }
            });

        let position_growth = if historical_position.lp_token_amount > 0.0 {
            ((current_position.lp_token_amount - historical_position.lp_token_amount) / historical_position.lp_token_amount) * 100.0
        } else {
            0.0
        };

        let rewards_earned = current_position.pending_rewards;
        let effective_apy = if duration_hours > 0.0 {
            (position_growth * 365.0 * 24.0) / duration_hours
        } else {
            pool_info.apy
        };

        Ok(PerformanceMetrics {
            pool_address,
            position_growth_percent: position_growth,
            rewards_earned,
            effective_apy,
            duration_hours,
            current_position_value: current_position.lp_token_amount,
            pool_apy: pool_info.apy,
        })
    }

    /// Clear position cache
    pub async fn clear_cache(&self) {
        let mut cache = self.position_cache.write().await;
        cache.clear();
        info!("🧹 Position cache cleared");
    }

    /// Get cached position if available
    pub async fn get_cached_position(&self, pool_address: Pubkey) -> Option<Position> {
        let cache = self.position_cache.read().await;
        cache.get(&pool_address.to_string()).cloned()
    }

    /// Update position cache manually
    pub async fn update_cache(&self, position: Position) {
        let mut cache = self.position_cache.write().await;
        cache.insert(position.pool_address.to_string(), position);
    }
}

/// Represents a change in position between two time points
#[derive(Debug, Clone)]
pub struct PositionChange {
    pub pool_address: Pubkey,
    pub token_a_change: f64,
    pub token_b_change: f64,
    pub lp_token_change: f64,
    pub rewards_change: f64,
    pub time_elapsed: f64, // seconds
}

impl PositionChange {
    /// Check if this represents a significant change worth reporting
    pub fn has_significant_change(&self) -> bool {
        let threshold = 0.000001; // Minimum change threshold
        
        self.token_a_change.abs() > threshold
            || self.token_b_change.abs() > threshold
            || self.lp_token_change.abs() > threshold
            || self.rewards_change.abs() > threshold
    }

    /// Calculate the rate of change per hour
    pub fn hourly_rate(&self) -> f64 {
        if self.time_elapsed > 0.0 {
            self.lp_token_change * 3600.0 / self.time_elapsed
        } else {
            0.0
        }
    }
}

/// Performance metrics for a position over time
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub pool_address: Pubkey,
    pub position_growth_percent: f64,
    pub rewards_earned: f64,
    pub effective_apy: f64,
    pub duration_hours: f64,
    pub current_position_value: f64,
    pub pool_apy: f64,
}

impl PerformanceMetrics {
    /// Compare effective APY vs pool APY to measure compound effectiveness
    pub fn compound_effectiveness(&self) -> f64 {
        if self.pool_apy > 0.0 {
            (self.effective_apy - self.pool_apy) / self.pool_apy * 100.0
        } else {
            0.0
        }
    }
}