use anyhow::Result;
use chrono::Utc;
use log::{error, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer, Signature},
    transaction::Transaction,
};
use std::{sync::Arc, time::Duration};
use backoff::{ExponentialBackoff, backoff::Backoff};

use saros_dlmm_sdk::{DLMMClient, PoolInfo, UserPosition};

use crate::{
    gas_optimizer::GasOptimizer,
    notification_service::NotificationService,
    types::*,
};

/// Handles the execution of compound strategies
pub struct CompoundStrategy {
    config: CompoundStrategyConfig,
    rpc_client: Arc<RpcClient>,
    wallet: Arc<Keypair>,
    gas_optimizer: Arc<GasOptimizer>,
    notification_service: Arc<NotificationService>,
    dlmm_client: DLMMClient,
}

impl CompoundStrategy {
    pub fn new(
        config: CompoundStrategyConfig,
        rpc_client: Arc<RpcClient>,
        wallet: Arc<Keypair>,
        gas_optimizer: Arc<GasOptimizer>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        let dlmm_client = DLMMClient::new_with_rpc(rpc_client.clone());

        Self {
            config,
            rpc_client,
            wallet,
            gas_optimizer,
            notification_service,
            dlmm_client,
        }
    }

    /// Execute the compound operation with retry logic
    pub async fn execute_compound(&self) -> Result<CompoundResult> {
        let start_time = std::time::Instant::now();
        let pool_key = self.config.pool_address.to_string();

        info!("🔄 Starting compound execution for pool {}", pool_key);

        // Execute with exponential backoff on failure
        let mut backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(300)), // 5 minutes max retry
            ..Default::default()
        };

        loop {
            match self.try_execute_compound().await {
                Ok(result) => {
                    let duration = start_time.elapsed();
                    info!("✅ Compound completed in {:.2}s", duration.as_secs_f64());
                    return Ok(result);
                }
                Err(e) => {
                    if let Some(duration) = backoff.next_backoff() {
                        warn!("⏳ Compound failed, retrying in {:.1}s: {}", duration.as_secs_f64(), e);
                        tokio::time::sleep(duration).await;
                    } else {
                        error!("❌ Compound failed after all retries: {}", e);
                        return Ok(CompoundResult {
                            success: false,
                            rewards_harvested: 0.0,
                            amount_reinvested: 0.0,
                            new_position_value: 0.0,
                            gas_used: 0.0,
                            transaction_signature: "".to_string(),
                            timestamp: Utc::now(),
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
        }
    }

    /// Try to execute compound operation once
    async fn try_execute_compound(&self) -> Result<CompoundResult> {
        let pool_key = self.config.pool_address.to_string();

        // Step 1: Gas optimization check
        let gas_check = self.gas_optimizer.should_compound(
            self.config.pool_address,
            self.config.min_reward_threshold,
        ).await?;

        if !gas_check.should_proceed {
            info!("⏸️ Skipping compound: {}", gas_check.reason);
            return Ok(CompoundResult {
                success: false,
                rewards_harvested: 0.0,
                amount_reinvested: 0.0,
                new_position_value: 0.0,
                gas_used: 0.0,
                transaction_signature: "".to_string(),
                timestamp: Utc::now(),
                error: Some(gas_check.reason),
            });
        }

        // Step 2: Get current position and pending rewards
        let position = self.get_current_position().await?;
        let pending_rewards = self.get_pending_rewards().await?;

        info!("📊 Current position: {:.6} LP tokens", position.lp_token_amount);
        info!("🎁 Pending rewards: {:.6} tokens", pending_rewards);

        // Step 3: Check minimum threshold
        if pending_rewards < self.config.min_reward_threshold {
            let reason = format!(
                "Rewards below threshold: {:.6} < {:.6}",
                pending_rewards, self.config.min_reward_threshold
            );
            info!("⏸️ {}", reason);
            return Ok(CompoundResult {
                success: false,
                rewards_harvested: 0.0,
                amount_reinvested: 0.0,
                new_position_value: position.lp_token_amount,
                gas_used: 0.0,
                transaction_signature: "".to_string(),
                timestamp: Utc::now(),
                error: Some(reason),
            });
        }

        // Step 4: Harvest rewards
        let harvest_signature = self.harvest_rewards().await?;
        info!("✅ Rewards harvested: {}", harvest_signature);

        // Step 5: Calculate reinvestment amounts
        let reinvest_amount = (pending_rewards * self.config.reinvest_percentage as f64) / 100.0;
        let keep_amount = pending_rewards - reinvest_amount;

        info!("💰 Reinvesting: {:.6} ({:.0}%)", reinvest_amount, self.config.reinvest_percentage);
        if keep_amount > 0.0 {
            info!("🏦 Keeping: {:.6}", keep_amount);
        }

        // Step 6: Execute reinvestment based on strategy
        let reinvest_signature = match self.config.strategy_type {
            StrategyType::LP => {
                self.reinvest_lp(reinvest_amount, self.config.max_slippage.unwrap_or(1.0)).await?
            }
            StrategyType::Staking => {
                self.reinvest_staking(reinvest_amount).await?
            }
            StrategyType::Farming => {
                self.reinvest_farming(reinvest_amount, self.config.max_slippage.unwrap_or(1.0)).await?
            }
        };

        info!("✅ Reinvestment completed: {}", reinvest_signature);

        // Step 7: Get updated position
        let new_position = self.get_current_position().await?;
        let gas_used = gas_check.estimated_gas_cost;

        // Send success notification
        self.notification_service.send_notification(NotificationEvent {
            event_type: NotificationEventType::CompoundSuccess,
            pool_address: pool_key,
            message: format!("Compound successful: harvested {:.6}, reinvested {:.6}", pending_rewards, reinvest_amount),
            data: serde_json::json!({
                "rewards_harvested": pending_rewards,
                "amount_reinvested": reinvest_amount,
                "new_position": new_position.lp_token_amount,
                "gas_used": gas_used,
                "harvest_signature": harvest_signature,
                "reinvest_signature": reinvest_signature,
            }),
            timestamp: Utc::now(),
        }).await;

        Ok(CompoundResult {
            success: true,
            rewards_harvested: pending_rewards,
            amount_reinvested: reinvest_amount,
            new_position_value: new_position.lp_token_amount,
            gas_used,
            transaction_signature: reinvest_signature,
            timestamp: Utc::now(),
            error: None,
        })
    }

    /// Get current position for the pool
    async fn get_current_position(&self) -> Result<Position> {
        let user_position = self.dlmm_client.get_user_position(
            &self.config.pool_address,
            &self.wallet.pubkey(),
        ).await?;

        Ok(Position {
            pool_address: self.config.pool_address,
            token_a_amount: user_position.token_a_amount,
            token_b_amount: user_position.token_b_amount,
            lp_token_amount: user_position.lp_token_amount,
            pending_rewards: user_position.pending_rewards,
            last_updated: Utc::now(),
        })
    }

    /// Get pending rewards for the position
    async fn get_pending_rewards(&self) -> Result<f64> {
        let user_position = self.dlmm_client.get_user_position(
            &self.config.pool_address,
            &self.wallet.pubkey(),
        ).await?;

        Ok(user_position.pending_rewards)
    }

    /// Harvest rewards from the pool
    async fn harvest_rewards(&self) -> Result<String> {
        info!("🌾 Harvesting rewards from pool {}", self.config.pool_address);

        let transaction = self.dlmm_client.claim_rewards(
            &self.config.pool_address,
            &self.wallet.pubkey(),
        ).await?;

        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        
        Ok(signature.to_string())
    }

    /// Reinvest into LP position
    async fn reinvest_lp(&self, amount: f64, max_slippage: f64) -> Result<String> {
        info!("🔄 Reinvesting {:.6} tokens into LP position", amount);

        let transaction = self.dlmm_client.add_liquidity_tx(
            &self.config.pool_address,
            &self.wallet.pubkey(),
            amount,
            max_slippage,
        ).await?;

        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        
        Ok(signature.to_string())
    }

    /// Reinvest into staking position
    async fn reinvest_staking(&self, amount: f64) -> Result<String> {
        info!("🥩 Reinvesting {:.6} tokens into staking", amount);

        let transaction = self.dlmm_client.stake_tokens(
            &self.config.pool_address,
            &self.wallet.pubkey(),
            amount,
        ).await?;

        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        
        Ok(signature.to_string())
    }

    /// Reinvest into farming position
    async fn reinvest_farming(&self, amount: f64, max_slippage: f64) -> Result<String> {
        info!("🚜 Reinvesting {:.6} tokens into farming", amount);

        let transaction = self.dlmm_client.deposit_farm(
            &self.config.pool_address,
            &self.wallet.pubkey(),
            amount,
            max_slippage,
        ).await?;

        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        
        Ok(signature.to_string())
    }
}