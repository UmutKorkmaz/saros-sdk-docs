use anyhow::Result;
use chrono::Utc;
use dashmap::DashMap;
use log::{error, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{
    compound_strategy::CompoundStrategy,
    gas_optimizer::GasOptimizer,
    notification_service::NotificationService,
    position_monitor::PositionMonitor,
    statistics::StatisticsManager,
    types::*,
};

pub struct ActiveStrategy {
    pub config: CompoundStrategyConfig,
    pub strategy: Arc<CompoundStrategy>,
    pub job_id: Option<uuid::Uuid>,
    pub statistics: PoolStatistics,
}

/// Main auto-compounder that manages multiple compound strategies
pub struct AutoCompounder {
    rpc_client: Arc<RpcClient>,
    wallet: Arc<Keypair>,
    config: AutoCompoundConfig,
    active_strategies: Arc<DashMap<String, ActiveStrategy>>,
    scheduler: Arc<JobScheduler>,
    gas_optimizer: Arc<GasOptimizer>,
    notification_service: Arc<NotificationService>,
    position_monitor: Arc<PositionMonitor>,
    statistics_manager: Arc<RwLock<StatisticsManager>>,
    start_time: chrono::DateTime<Utc>,
}

impl AutoCompounder {
    /// Create a new AutoCompounder instance
    pub async fn new(config: AutoCompoundConfig) -> Result<Self> {
        // Initialize RPC client
        let rpc_client = Arc::new(RpcClient::new(&config.rpc_url));

        // Initialize wallet from private key
        let wallet = Arc::new(Self::parse_private_key(&config.private_key)?);

        // Initialize scheduler
        let scheduler = Arc::new(JobScheduler::new().await?);
        scheduler.start().await?;

        // Initialize components
        let gas_optimizer = Arc::new(GasOptimizer::new(rpc_client.clone()));
        let notification_service = Arc::new(NotificationService::new(&config));
        let position_monitor = Arc::new(PositionMonitor::new(rpc_client.clone()));
        let statistics_manager = Arc::new(RwLock::new(StatisticsManager::new()));

        info!("🔑 Wallet address: {}", wallet.pubkey());
        info!("🌐 Network: {}", config.network);
        info!("📡 RPC URL: {}", config.rpc_url);

        Ok(Self {
            rpc_client,
            wallet,
            config,
            active_strategies: Arc::new(DashMap::new()),
            scheduler,
            gas_optimizer,
            notification_service,
            position_monitor,
            statistics_manager,
            start_time: Utc::now(),
        })
    }

    /// Start a compound strategy
    pub async fn start_strategy(&mut self, config: CompoundStrategyConfig) -> Result<StartResult> {
        let pool_key = config.pool_address.to_string();

        // Check if strategy is already active
        if self.active_strategies.contains_key(&pool_key) {
            warn!("Strategy for pool {} is already active", pool_key);
            return Ok(StartResult {
                success: false,
                pool_address: pool_key,
                strategy_type: config.strategy_type.to_string(),
                interval_ms: config.interval_ms,
                min_threshold: config.min_reward_threshold,
                next_compound_time: "".to_string(),
                error: Some("Strategy already active".to_string()),
            });
        }

        // Validate pool exists and get initial position
        let position = match self.position_monitor.get_position(config.pool_address).await {
            Ok(pos) => pos,
            Err(e) => {
                error!("Failed to get position for pool {}: {}", pool_key, e);
                return Ok(StartResult {
                    success: false,
                    pool_address: pool_key,
                    strategy_type: config.strategy_type.to_string(),
                    interval_ms: config.interval_ms,
                    min_threshold: config.min_reward_threshold,
                    next_compound_time: "".to_string(),
                    error: Some(format!("Failed to validate pool: {}", e)),
                });
            }
        };

        info!("📊 Initial position for {}: {:.6} LP tokens", pool_key, position.lp_token_amount);

        // Create compound strategy
        let strategy = Arc::new(CompoundStrategy::new(
            config.clone(),
            self.rpc_client.clone(),
            self.wallet.clone(),
            self.gas_optimizer.clone(),
            self.notification_service.clone(),
        ));

        // Create cron schedule based on interval
        let cron_expression = Self::interval_to_cron(config.interval_ms);
        info!("⏰ Scheduling with cron expression: {}", cron_expression);

        // Clone necessary values for the job
        let strategy_clone = strategy.clone();
        let statistics_manager = self.statistics_manager.clone();
        let pool_address = config.pool_address;

        // Create scheduled job
        let job = Job::new_async(cron_expression.as_str(), move |_uuid, _l| {
            let strategy = strategy_clone.clone();
            let stats_manager = statistics_manager.clone();
            
            Box::pin(async move {
                info!("🔄 Executing scheduled compound for pool: {}", pool_address);
                
                match strategy.execute_compound().await {
                    Ok(result) => {
                        info!("✅ Compound successful for pool {}", pool_address);
                        
                        // Update statistics
                        let mut stats = stats_manager.write().await;
                        stats.record_compound_result(&pool_address.to_string(), &result).await;
                    }
                    Err(e) => {
                        error!("❌ Compound failed for pool {}: {}", pool_address, e);
                        
                        // Record failed compound
                        let mut stats = stats_manager.write().await;
                        let failed_result = CompoundResult {
                            success: false,
                            rewards_harvested: 0.0,
                            amount_reinvested: 0.0,
                            new_position_value: 0.0,
                            gas_used: 0.0,
                            transaction_signature: "".to_string(),
                            timestamp: Utc::now(),
                            error: Some(e.to_string()),
                        };
                        stats.record_compound_result(&pool_address.to_string(), &failed_result).await;
                    }
                }
            })
        })?;

        // Add job to scheduler
        let job_id = self.scheduler.add(job).await?;

        // Calculate next compound time
        let next_compound = Utc::now() + chrono::Duration::milliseconds(config.interval_ms as i64);

        // Store active strategy
        let active_strategy = ActiveStrategy {
            config: config.clone(),
            strategy,
            job_id: Some(job_id),
            statistics: PoolStatistics {
                pool_address: config.pool_address,
                ..Default::default()
            },
        };

        self.active_strategies.insert(pool_key.clone(), active_strategy);

        // Send notification
        self.notification_service.send_notification(NotificationEvent {
            event_type: NotificationEventType::CompoundStarted,
            pool_address: pool_key.clone(),
            message: format!("Auto-compound started for {} strategy", config.strategy_type),
            data: serde_json::json!({
                "strategy_type": config.strategy_type.to_string(),
                "interval_ms": config.interval_ms,
                "min_threshold": config.min_reward_threshold,
            }),
            timestamp: Utc::now(),
        }).await;

        info!("✅ Auto-compound started for pool {}", pool_key);
        info!("   Strategy: {}", config.strategy_type);
        info!("   Interval: {}ms", config.interval_ms);
        info!("   Next compound: {}", next_compound.format("%Y-%m-%d %H:%M:%S UTC"));

        Ok(StartResult {
            success: true,
            pool_address: pool_key,
            strategy_type: config.strategy_type.to_string(),
            interval_ms: config.interval_ms,
            min_threshold: config.min_reward_threshold,
            next_compound_time: next_compound.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            error: None,
        })
    }

    /// Stop a compound strategy
    pub async fn stop_strategy(&mut self, pool_address: Pubkey) -> Result<bool> {
        let pool_key = pool_address.to_string();

        match self.active_strategies.remove(&pool_key) {
            Some((_, active_strategy)) => {
                // Remove job from scheduler
                if let Some(job_id) = active_strategy.job_id {
                    self.scheduler.remove(&job_id).await?;
                }

                // Send notification
                self.notification_service.send_notification(NotificationEvent {
                    event_type: NotificationEventType::CompoundStopped,
                    pool_address: pool_key.clone(),
                    message: "Auto-compound stopped".to_string(),
                    data: serde_json::json!({
                        "statistics": active_strategy.statistics
                    }),
                    timestamp: Utc::now(),
                }).await;

                info!("🛑 Auto-compound stopped for pool {}", pool_key);
                Ok(true)
            }
            None => {
                warn!("No active strategy found for pool {}", pool_key);
                Ok(false)
            }
        }
    }

    /// Manually trigger compound for a specific pool
    pub async fn compound_now(&self, pool_address: Pubkey) -> Result<CompoundResult> {
        let pool_key = pool_address.to_string();

        match self.active_strategies.get(&pool_key) {
            Some(active_strategy) => {
                info!("🔄 Manual compound triggered for pool {}", pool_key);
                active_strategy.strategy.execute_compound().await
            }
            None => {
                // Create temporary strategy for one-time compound
                let temp_config = CompoundStrategyConfig {
                    pool_address,
                    strategy_type: StrategyType::LP,
                    interval_ms: 0,
                    min_reward_threshold: 0.0,
                    reinvest_percentage: 100,
                    max_slippage: Some(1.0),
                    emergency_withdraw: false,
                };

                let temp_strategy = CompoundStrategy::new(
                    temp_config,
                    self.rpc_client.clone(),
                    self.wallet.clone(),
                    self.gas_optimizer.clone(),
                    self.notification_service.clone(),
                );

                temp_strategy.execute_compound().await
            }
        }
    }

    /// Get global statistics
    pub async fn get_global_statistics(&self) -> Result<GlobalStatistics> {
        let stats_manager = self.statistics_manager.read().await;
        let mut stats = stats_manager.get_global_statistics().await;
        
        // Calculate uptime
        let uptime = Utc::now() - self.start_time;
        stats.uptime_hours = uptime.num_minutes() as f64 / 60.0;
        
        Ok(stats)
    }

    /// Get statistics for a specific pool
    pub async fn get_pool_statistics(&self, pool_address: Pubkey) -> Result<Option<PoolStatistics>> {
        let stats_manager = self.statistics_manager.read().await;
        Ok(stats_manager.get_pool_statistics(&pool_address.to_string()).await)
    }

    /// Get list of active pools
    pub async fn get_active_pools(&self) -> Vec<Pubkey> {
        self.active_strategies
            .iter()
            .map(|entry| entry.key().parse::<Pubkey>().unwrap())
            .collect()
    }

    /// Stop all active strategies
    pub async fn stop_all(&mut self) -> Result<()> {
        info!("🛑 Stopping all active strategies...");
        
        let pool_keys: Vec<String> = self.active_strategies
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for pool_key in pool_keys {
            if let Ok(pool_address) = pool_key.parse::<Pubkey>() {
                if let Err(e) = self.stop_strategy(pool_address).await {
                    error!("Failed to stop strategy for pool {}: {}", pool_key, e);
                }
            }
        }

        // Stop scheduler - we'll just let it drop naturally since we can't get mutable access

        info!("✅ All strategies stopped");
        Ok(())
    }

    /// Parse private key from various formats
    fn parse_private_key(private_key: &str) -> Result<Keypair> {
        // Try base58 format first
        if let Ok(decoded) = bs58::decode(private_key).into_vec() {
            if decoded.len() == 64 {
                return Ok(Keypair::from_bytes(&decoded)?);
            }
        }

        // Try JSON array format
        if private_key.starts_with('[') && private_key.ends_with(']') {
            let key_array: Vec<u8> = serde_json::from_str(private_key)?;
            if key_array.len() == 64 {
                return Ok(Keypair::from_bytes(&key_array)?);
            }
        }

        Err(anyhow::anyhow!("Invalid private key format"))
    }

    /// Convert interval in milliseconds to cron expression
    fn interval_to_cron(interval_ms: u64) -> String {
        let minutes = interval_ms / 60000;

        if minutes < 60 {
            format!("0/{} * * * *", minutes) // Every X minutes
        } else {
            let hours = minutes / 60;
            if hours < 24 {
                format!("0 0/{} * * *", hours) // Every X hours
            } else {
                let days = hours / 24;
                format!("0 0 0/{} * *", days) // Every X days
            }
        }
    }
}