use anyhow::Result;
use log::{info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

use saros_dlmm_sdk::DLMMClient;

use crate::types::GasOptimizationResult;

/// Gas optimization component that determines optimal timing for compound operations
pub struct GasOptimizer {
    rpc_client: Arc<RpcClient>,
    dlmm_client: DLMMClient,
}

impl GasOptimizer {
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        let dlmm_client = DLMMClient::new_with_rpc(rpc_client.clone());

        Self {
            rpc_client,
            dlmm_client,
        }
    }

    /// Determine if compound operation should proceed based on gas costs and rewards
    pub async fn should_compound(
        &self,
        pool_address: Pubkey,
        min_reward_threshold: f64,
    ) -> Result<GasOptimizationResult> {
        // Get current gas price
        let gas_price = self.get_current_gas_price().await?;
        
        // Estimate gas cost for compound operation
        let estimated_gas_cost = self.estimate_compound_gas_cost(pool_address).await?;
        
        // Get pending rewards
        let pending_rewards = self.get_pending_rewards(pool_address).await?;

        info!("⛽ Current gas price: {:.6} SOL", gas_price);
        info!("📊 Estimated gas cost: {:.6} SOL", estimated_gas_cost);
        info!("🎁 Pending rewards: {:.6} tokens", pending_rewards);

        // Check if rewards justify gas costs (assuming 1:1 token to SOL ratio for simplicity)
        let profit_threshold = estimated_gas_cost * 2.0; // Require 2x gas cost in rewards
        let gas_efficient = pending_rewards >= profit_threshold;

        // Check minimum reward threshold
        let meets_min_threshold = pending_rewards >= min_reward_threshold;

        // Check if gas price is reasonable (under 0.1 SOL)
        let reasonable_gas_price = gas_price < 0.1;

        let should_proceed = gas_efficient && meets_min_threshold && reasonable_gas_price;

        let reason = if !meets_min_threshold {
            format!("Rewards below minimum threshold: {:.6} < {:.6}", pending_rewards, min_reward_threshold)
        } else if !gas_efficient {
            format!("Rewards don't justify gas costs: {:.6} < {:.6} (2x gas cost)", pending_rewards, profit_threshold)
        } else if !reasonable_gas_price {
            format!("Gas price too high: {:.6} SOL", gas_price)
        } else {
            "Optimal conditions for compounding".to_string()
        };

        if should_proceed {
            info!("✅ Gas optimization: {}", reason);
        } else {
            warn!("⚠️ Gas optimization: {}", reason);
        }

        Ok(GasOptimizationResult {
            should_proceed,
            recommended_gas_price: gas_price,
            estimated_gas_cost,
            reason,
        })
    }

    /// Get current gas price from the network
    pub async fn get_current_gas_price(&self) -> Result<f64> {
        // Get recent blockhash and fee calculator
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        
        // In Solana, fees are relatively stable, but we can get fee rate
        match self.rpc_client.get_fee_for_message(&solana_sdk::message::Message::new_with_blockhash(
            &[],
            None,
            &recent_blockhash,
        )) {
            Ok(fee) => Ok(fee as f64 / 1_000_000_000.0), // Convert lamports to SOL
            Err(_) => {
                // Fallback to average fee
                warn!("Could not get current fee, using default");
                Ok(0.000005) // 5000 lamports = 0.000005 SOL
            }
        }
    }

    /// Estimate gas cost for compound operation
    async fn estimate_compound_gas_cost(&self, pool_address: Pubkey) -> Result<f64> {
        // Simulate compound operation to estimate gas
        // This is a simplified estimation - in production you'd simulate the actual transactions
        
        let base_fee = 0.000005; // Base transaction fee in SOL
        let harvest_fee = 0.000010; // Fee for harvest transaction
        let reinvest_fee = 0.000015; // Fee for reinvest transaction
        
        let total_estimated_fee = base_fee + harvest_fee + reinvest_fee;
        
        info!("📊 Estimated compound gas cost: {:.6} SOL", total_estimated_fee);
        
        Ok(total_estimated_fee)
    }

    /// Get pending rewards for gas calculation
    async fn get_pending_rewards(&self, pool_address: Pubkey) -> Result<f64> {
        // This would need the actual user's pubkey - for now use a placeholder
        // In practice, this would be passed as a parameter
        let user_pubkey = solana_sdk::pubkey::Pubkey::new_unique();
        
        match self.dlmm_client.get_user_position(&pool_address, &user_pubkey).await {
            Ok(position) => Ok(position.pending_rewards),
            Err(_) => {
                // Return mock pending rewards for estimation
                Ok(5.0) // Mock value
            }
        }
    }

    /// Calculate optimal compound frequency based on APY and gas costs
    pub async fn calculate_optimal_frequency(
        &self,
        pool_address: Pubkey,
        current_apy: f64,
        position_size: f64,
    ) -> Result<u64> {
        let gas_cost = self.estimate_compound_gas_cost(pool_address).await?;
        
        // Calculate daily rewards
        let daily_rewards = (position_size * current_apy / 365.0) / 100.0;
        
        // Find frequency where gas cost is ~5% of daily rewards
        let target_gas_ratio = 0.05;
        let target_reward = gas_cost / target_gas_ratio;
        
        // Calculate hours needed to accumulate target reward
        let hours_needed = (target_reward / daily_rewards * 24.0).max(1.0);
        
        // Convert to milliseconds
        let optimal_interval_ms = (hours_needed * 3600.0 * 1000.0) as u64;
        
        info!("🔧 Calculated optimal compound frequency: {} hours", hours_needed);
        
        Ok(optimal_interval_ms)
    }

    /// Check if network is congested
    pub async fn is_network_congested(&self) -> Result<bool> {
        // Get recent performance samples
        match self.rpc_client.get_recent_performance_samples(Some(5)) {
            Ok(samples) => {
                let avg_tx_count: f64 = samples.iter()
                    .map(|s| s.num_transactions as f64)
                    .sum::<f64>() / samples.len() as f64;
                
                // Consider network congested if average TPS is high
                // This is a simplified heuristic
                let is_congested = avg_tx_count > 2000.0;
                
                if is_congested {
                    warn!("🚨 Network appears congested (avg TPS: {:.0})", avg_tx_count);
                } else {
                    info!("🟢 Network conditions normal (avg TPS: {:.0})", avg_tx_count);
                }
                
                Ok(is_congested)
            }
            Err(_) => {
                warn!("Could not check network congestion");
                Ok(false) // Assume not congested if we can't check
            }
        }
    }

    /// Get priority fee recommendations
    pub async fn get_priority_fee_recommendation(&self) -> Result<u64> {
        // In production, this would analyze recent transactions to recommend priority fees
        // For now, return a conservative priority fee
        
        let is_congested = self.is_network_congested().await?;
        
        let priority_fee = if is_congested {
            10000 // Higher priority fee during congestion (0.00001 SOL)
        } else {
            1000  // Lower priority fee during normal conditions (0.000001 SOL)
        };
        
        info!("💰 Recommended priority fee: {} lamports", priority_fee);
        
        Ok(priority_fee)
    }
}