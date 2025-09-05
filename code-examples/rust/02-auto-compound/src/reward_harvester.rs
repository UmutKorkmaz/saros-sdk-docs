use anyhow::Result;
use chrono::Utc;
use log::{error, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use std::{sync::Arc, time::Duration};
use backoff::{ExponentialBackoff, backoff::Backoff};

use saros_dlmm_sdk::DLMMClient;

use crate::types::{CompoundResult, StrategyType};

/// Specialized component for harvesting rewards efficiently
pub struct RewardHarvester {
    rpc_client: Arc<RpcClient>,
    wallet: Arc<Keypair>,
    dlmm_client: DLMMClient,
}

impl RewardHarvester {
    pub fn new(rpc_client: Arc<RpcClient>, wallet: Arc<Keypair>) -> Self {
        let dlmm_client = DLMMClient::new_with_rpc(rpc_client.clone());

        Self {
            rpc_client,
            wallet,
            dlmm_client,
        }
    }

    /// Harvest rewards with retry logic and optimization
    pub async fn harvest_rewards(
        &self,
        pool_address: Pubkey,
        strategy_type: StrategyType,
    ) -> Result<HarvestResult> {
        info!("🌾 Starting reward harvest for pool: {}", pool_address);

        let mut backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(60)),
            ..Default::default()
        };

        loop {
            match self.try_harvest_rewards(pool_address, strategy_type).await {
                Ok(result) => {
                    info!("✅ Rewards harvested successfully: {}", result.signature);
                    return Ok(result);
                }
                Err(e) => {
                    if let Some(duration) = backoff.next_backoff() {
                        warn!("⏳ Harvest failed, retrying in {:.1}s: {}", duration.as_secs_f64(), e);
                        tokio::time::sleep(duration).await;
                    } else {
                        error!("❌ Harvest failed after all retries: {}", e);
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Single attempt to harvest rewards
    async fn try_harvest_rewards(
        &self,
        pool_address: Pubkey,
        strategy_type: StrategyType,
    ) -> Result<HarvestResult> {
        // Get current pending rewards
        let pending_rewards = self.get_pending_rewards(pool_address).await?;
        
        if pending_rewards <= 0.0 {
            return Err(anyhow::anyhow!("No rewards to harvest"));
        }

        info!("🎁 Pending rewards: {:.6} tokens", pending_rewards);

        // Create harvest transaction based on strategy type
        let transaction = match strategy_type {
            StrategyType::LP => {
                self.create_lp_harvest_transaction(pool_address).await?
            }
            StrategyType::Staking => {
                self.create_staking_harvest_transaction(pool_address).await?
            }
            StrategyType::Farming => {
                self.create_farming_harvest_transaction(pool_address).await?
            }
        };

        // Send and confirm transaction
        let signature = self.send_transaction_with_retry(transaction).await?;

        // Verify harvest was successful
        let new_pending_rewards = self.get_pending_rewards(pool_address).await?;
        let actual_harvested = pending_rewards - new_pending_rewards;

        if actual_harvested <= 0.0 {
            return Err(anyhow::anyhow!("Harvest transaction did not reduce pending rewards"));
        }

        Ok(HarvestResult {
            signature: signature.to_string(),
            rewards_harvested: actual_harvested,
            gas_used: self.estimate_gas_used(&signature).await?,
            timestamp: Utc::now(),
        })
    }

    /// Get pending rewards for the pool
    async fn get_pending_rewards(&self, pool_address: Pubkey) -> Result<f64> {
        let user_position = self.dlmm_client.get_user_position(
            &pool_address,
            &self.wallet.pubkey(),
        ).await?;

        Ok(user_position.pending_rewards)
    }

    /// Create harvest transaction for LP rewards
    async fn create_lp_harvest_transaction(&self, pool_address: Pubkey) -> Result<Transaction> {
        info!("🔄 Creating LP harvest transaction");
        
        self.dlmm_client.claim_rewards(
            &pool_address,
            &self.wallet.pubkey(),
        ).await.map_err(|e| anyhow::anyhow!("Claim rewards failed: {}", e))
    }

    /// Create harvest transaction for staking rewards
    async fn create_staking_harvest_transaction(&self, pool_address: Pubkey) -> Result<Transaction> {
        info!("🥩 Creating staking harvest transaction");
        
        self.dlmm_client.claim_staking_rewards(
            &pool_address,
            &self.wallet.pubkey(),
        ).await.map_err(|e| anyhow::anyhow!("Claim staking rewards failed: {}", e))
    }

    /// Create harvest transaction for farming rewards
    async fn create_farming_harvest_transaction(&self, pool_address: Pubkey) -> Result<Transaction> {
        info!("🚜 Creating farming harvest transaction");
        
        self.dlmm_client.claim_farming_rewards(
            &pool_address,
            &self.wallet.pubkey(),
        ).await.map_err(|e| anyhow::anyhow!("Claim farming rewards failed: {}", e))
    }

    /// Send transaction with retry logic
    async fn send_transaction_with_retry(&self, transaction: Transaction) -> Result<Signature> {
        let mut backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(30)),
            ..Default::default()
        };

        loop {
            match self.rpc_client.send_and_confirm_transaction(&transaction) {
                Ok(signature) => return Ok(signature),
                Err(e) => {
                    if let Some(duration) = backoff.next_backoff() {
                        warn!("Transaction failed, retrying in {:.1}s: {}", duration.as_secs_f64(), e);
                        tokio::time::sleep(duration).await;
                    } else {
                        return Err(anyhow::anyhow!("Transaction failed after retries: {}", e));
                    }
                }
            }
        }
    }

    /// Estimate gas used for a completed transaction
    async fn estimate_gas_used(&self, signature: &Signature) -> Result<f64> {
        // Get transaction details to calculate actual gas used
        match self.rpc_client.get_transaction_with_config(signature, solana_client::rpc_config::RpcTransactionConfig {
            encoding: Some(solana_transaction_status::UiTransactionEncoding::Json),
            commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        }) {
            Ok(transaction) => {
                if let Some(meta) = transaction.transaction.meta {
                    // Convert lamports to SOL
                    Ok(meta.fee as f64 / 1_000_000_000.0)
                } else {
                    // Fallback to estimated fee
                    Ok(0.000005) // 5000 lamports
                }
            }
            Err(_) => {
                // Fallback to estimated fee
                Ok(0.000005)
            }
        }
    }

    /// Batch harvest from multiple pools (gas optimization)
    pub async fn batch_harvest(
        &self,
        pool_addresses: Vec<Pubkey>,
        strategy_types: Vec<StrategyType>,
    ) -> Result<Vec<HarvestResult>> {
        if pool_addresses.len() != strategy_types.len() {
            return Err(anyhow::anyhow!("Pool addresses and strategy types length mismatch"));
        }

        info!("🔄 Batch harvesting from {} pools", pool_addresses.len());

        let mut results = Vec::new();

        // For now, harvest sequentially
        // In production, you could optimize by batching transactions
        for (pool_address, strategy_type) in pool_addresses.iter().zip(strategy_types.iter()) {
            match self.harvest_rewards(*pool_address, *strategy_type).await {
                Ok(result) => {
                    results.push(result);
                    info!("✅ Harvested from pool: {}", pool_address);
                }
                Err(e) => {
                    error!("❌ Failed to harvest from pool {}: {}", pool_address, e);
                    // Continue with other pools even if one fails
                }
            }
        }

        info!("🌾 Batch harvest completed: {}/{} successful", results.len(), pool_addresses.len());

        Ok(results)
    }

    /// Check if rewards are harvestable (above gas cost threshold)
    pub async fn is_harvest_profitable(
        &self,
        pool_address: Pubkey,
        min_profit_threshold: f64,
    ) -> Result<bool> {
        let pending_rewards = self.get_pending_rewards(pool_address).await?;
        let estimated_gas_cost = 0.000005; // Estimated harvest gas cost in SOL

        // Assuming 1:1 token to SOL ratio for simplicity
        // In production, you'd use actual token prices
        let is_profitable = pending_rewards >= (estimated_gas_cost + min_profit_threshold);

        info!(
            "💰 Harvest profitability check: {:.6} rewards, {:.6} gas cost, profitable: {}",
            pending_rewards, estimated_gas_cost, is_profitable
        );

        Ok(is_profitable)
    }

    /// Get harvest history for analytics
    pub async fn get_harvest_history(&self, pool_address: Pubkey, limit: usize) -> Result<Vec<HarvestRecord>> {
        // In production, this would query transaction history
        // For now, return mock data
        info!("📊 Getting harvest history for pool: {} (limit: {})", pool_address, limit);

        // Mock harvest records for demonstration
        let mut records = Vec::new();
        let base_time = Utc::now() - chrono::Duration::hours(24);

        for i in 0..std::cmp::min(limit, 10) {
            records.push(HarvestRecord {
                signature: format!("harvest_signature_{}", i),
                pool_address,
                rewards_harvested: 5.0 + (i as f64 * 0.5),
                gas_used: 0.000005 + (i as f64 * 0.000001),
                timestamp: base_time + chrono::Duration::hours(i as i64 * 2),
            });
        }

        Ok(records)
    }
}

/// Result of a harvest operation
#[derive(Debug, Clone)]
pub struct HarvestResult {
    pub signature: String,
    pub rewards_harvested: f64,
    pub gas_used: f64,
    pub timestamp: chrono::DateTime<Utc>,
}

/// Historical harvest record
#[derive(Debug, Clone)]
pub struct HarvestRecord {
    pub signature: String,
    pub pool_address: Pubkey,
    pub rewards_harvested: f64,
    pub gas_used: f64,
    pub timestamp: chrono::DateTime<Utc>,
}

impl HarvestRecord {
    /// Calculate harvest efficiency (rewards per gas unit)
    pub fn harvest_efficiency(&self) -> f64 {
        if self.gas_used > 0.0 {
            self.rewards_harvested / self.gas_used
        } else {
            0.0
        }
    }
}