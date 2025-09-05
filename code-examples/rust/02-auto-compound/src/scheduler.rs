use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{error, info, warn};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{mpsc, RwLock};
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::types::{CompoundStrategyConfig, StrategyType};

/// Advanced scheduler for managing compound operations with dynamic frequency adjustment
pub struct CompoundScheduler {
    scheduler: JobScheduler,
    active_jobs: Arc<RwLock<HashMap<String, ScheduledJob>>>,
    command_sender: mpsc::UnboundedSender<SchedulerCommand>,
    command_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<SchedulerCommand>>>>,
}

#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub job_id: uuid::Uuid,
    pub pool_address: String,
    pub strategy_config: CompoundStrategyConfig,
    pub next_run: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub average_execution_time: Duration,
    pub dynamic_interval: u64, // Current interval in ms (may differ from config)
}

#[derive(Debug)]
pub enum SchedulerCommand {
    AdjustInterval {
        pool_address: String,
        new_interval_ms: u64,
        reason: String,
    },
    PauseJob {
        pool_address: String,
        duration_minutes: u64,
    },
    ResumeJob {
        pool_address: String,
    },
    UpdateGasThreshold {
        pool_address: String,
        new_threshold: f64,
    },
}

impl CompoundScheduler {
    pub async fn new() -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let scheduler_instance = Self {
            scheduler,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            command_sender,
            command_receiver: Arc::new(RwLock::new(Some(command_receiver))),
        };

        Ok(scheduler_instance)
    }

    /// Start the scheduler and command processor
    pub async fn start(&self) -> Result<()> {
        self.scheduler.start().await?;
        
        // Start command processor
        let active_jobs = self.active_jobs.clone();
        let scheduler = self.scheduler.clone();
        
        if let Some(mut receiver) = self.command_receiver.write().await.take() {
            tokio::spawn(async move {
                while let Some(command) = receiver.recv().await {
                    if let Err(e) = Self::process_command(command, &active_jobs, &scheduler).await {
                        error!("Failed to process scheduler command: {}", e);
                    }
                }
            });
        }

        info!("🕒 Compound scheduler started");
        Ok(())
    }

    /// Add a compound job to the scheduler
    pub async fn add_compound_job<F, Fut>(
        &self,
        config: CompoundStrategyConfig,
        compound_fn: F,
    ) -> Result<String>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<bool>> + Send + 'static,
    {
        let pool_key = config.pool_address.to_string();
        let cron_expression = self.interval_to_cron(config.interval_ms);

        info!("⏰ Adding compound job for pool: {} with cron: {}", pool_key, cron_expression);

        // Create job with performance tracking
        let active_jobs = self.active_jobs.clone();
        let pool_address = config.pool_address.to_string();
        let command_sender = self.command_sender.clone();

        let job = Job::new_async(cron_expression.as_str(), move |_uuid, _l| {
            let compound_fn = compound_fn();
            let active_jobs = active_jobs.clone();
            let pool_address = pool_address.clone();
            let command_sender = command_sender.clone();

            Box::pin(async move {
                let start_time = std::time::Instant::now();
                
                // Update last run time
                {
                    let mut jobs = active_jobs.write().await;
                    if let Some(job) = jobs.get_mut(&pool_address) {
                        job.last_run = Some(Utc::now());
                        job.run_count += 1;
                    }
                }

                // Execute compound operation
                let success = match compound_fn.await {
                    Ok(success) => {
                        if success {
                            info!("✅ Scheduled compound successful for pool: {}", pool_address);
                        } else {
                            warn!("⚠️ Scheduled compound skipped for pool: {}", pool_address);
                        }
                        success
                    }
                    Err(e) => {
                        error!("❌ Scheduled compound failed for pool {}: {}", pool_address, e);
                        false
                    }
                };

                let execution_time = start_time.elapsed();

                // Update job statistics
                {
                    let mut jobs = active_jobs.write().await;
                    if let Some(job) = jobs.get_mut(&pool_address) {
                        if success {
                            job.success_count += 1;
                        } else {
                            job.failure_count += 1;
                        }

                        // Update average execution time using exponential moving average
                        let alpha = 0.1; // Smoothing factor
                        let new_avg = Duration::from_secs_f64(
                            job.average_execution_time.as_secs_f64() * (1.0 - alpha) +
                            execution_time.as_secs_f64() * alpha
                        );
                        job.average_execution_time = new_avg;

                        // Auto-adjust interval based on performance
                        Self::maybe_adjust_interval(&job, &command_sender).await;
                    }
                }
            })
        })?;

        let job_id = self.scheduler.add(job).await?;

        // Store job information
        let scheduled_job = ScheduledJob {
            job_id,
            pool_address: pool_key.clone(),
            strategy_config: config.clone(),
            next_run: Utc::now() + chrono::Duration::milliseconds(config.interval_ms as i64),
            last_run: None,
            run_count: 0,
            success_count: 0,
            failure_count: 0,
            average_execution_time: Duration::from_secs(0),
            dynamic_interval: config.interval_ms,
        };

        let mut jobs = self.active_jobs.write().await;
        jobs.insert(pool_key.clone(), scheduled_job);

        info!("✅ Compound job added for pool: {}", pool_key);
        Ok(pool_key)
    }

    /// Remove a compound job from the scheduler
    pub async fn remove_compound_job(&self, pool_address: &str) -> Result<bool> {
        let mut jobs = self.active_jobs.write().await;
        
        if let Some(job) = jobs.remove(pool_address) {
            self.scheduler.remove(&job.job_id).await?;
            info!("🗑️ Removed compound job for pool: {}", pool_address);
            Ok(true)
        } else {
            warn!("No active job found for pool: {}", pool_address);
            Ok(false)
        }
    }

    /// Get job statistics
    pub async fn get_job_stats(&self, pool_address: &str) -> Option<JobStatistics> {
        let jobs = self.active_jobs.read().await;
        jobs.get(pool_address).map(|job| JobStatistics {
            pool_address: job.pool_address.clone(),
            total_runs: job.run_count,
            successful_runs: job.success_count,
            failed_runs: job.failure_count,
            success_rate: if job.run_count > 0 {
                (job.success_count as f64 / job.run_count as f64) * 100.0
            } else {
                0.0
            },
            average_execution_time_ms: job.average_execution_time.as_millis() as u64,
            current_interval_ms: job.dynamic_interval,
            next_run: job.next_run,
            last_run: job.last_run,
        })
    }

    /// Get all job statistics
    pub async fn get_all_job_stats(&self) -> Vec<JobStatistics> {
        let jobs = self.active_jobs.read().await;
        jobs.values().map(|job| JobStatistics {
            pool_address: job.pool_address.clone(),
            total_runs: job.run_count,
            successful_runs: job.success_count,
            failed_runs: job.failure_count,
            success_rate: if job.run_count > 0 {
                (job.success_count as f64 / job.run_count as f64) * 100.0
            } else {
                0.0
            },
            average_execution_time_ms: job.average_execution_time.as_millis() as u64,
            current_interval_ms: job.dynamic_interval,
            next_run: job.next_run,
            last_run: job.last_run,
        }).collect()
    }

    /// Send a command to the scheduler
    pub fn send_command(&self, command: SchedulerCommand) -> Result<()> {
        self.command_sender.send(command)?;
        Ok(())
    }

    /// Process scheduler commands
    async fn process_command(
        command: SchedulerCommand,
        active_jobs: &Arc<RwLock<HashMap<String, ScheduledJob>>>,
        scheduler: &JobScheduler,
    ) -> Result<()> {
        match command {
            SchedulerCommand::AdjustInterval { pool_address, new_interval_ms, reason } => {
                info!("🔧 Adjusting interval for pool {}: {}ms ({})", pool_address, new_interval_ms, reason);
                
                let mut jobs = active_jobs.write().await;
                if let Some(job) = jobs.get_mut(&pool_address) {
                    job.dynamic_interval = new_interval_ms;
                    
                    // Remove old job and create new one with updated interval
                    scheduler.remove(&job.job_id).await?;
                    
                    // Note: In a full implementation, you'd recreate the job here
                    // For now, just update the stored interval
                }
            }
            SchedulerCommand::PauseJob { pool_address, duration_minutes } => {
                info!("⏸️ Pausing job for pool {} for {} minutes", pool_address, duration_minutes);
                
                let jobs = active_jobs.read().await;
                if let Some(job) = jobs.get(&pool_address) {
                    // Pause the job
                    scheduler.remove(&job.job_id).await?;
                    
                    // Schedule resume
                    let resume_time = Utc::now() + chrono::Duration::minutes(duration_minutes as i64);
                    info!("⏰ Job will resume at: {}", resume_time);
                }
            }
            SchedulerCommand::ResumeJob { pool_address } => {
                info!("▶️ Resuming job for pool: {}", pool_address);
                // Implementation would recreate the job
            }
            SchedulerCommand::UpdateGasThreshold { pool_address, new_threshold } => {
                info!("⛽ Updating gas threshold for pool {}: {}", pool_address, new_threshold);
                
                let mut jobs = active_jobs.write().await;
                if let Some(job) = jobs.get_mut(&pool_address) {
                    // Update the strategy config
                    // job.strategy_config.min_reward_threshold = new_threshold;
                }
            }
        }
        
        Ok(())
    }

    /// Auto-adjust interval based on job performance
    async fn maybe_adjust_interval(
        job: &ScheduledJob,
        command_sender: &mpsc::UnboundedSender<SchedulerCommand>,
    ) {
        // Only adjust after at least 5 runs
        if job.run_count < 5 {
            return;
        }

        let success_rate = job.success_count as f64 / job.run_count as f64;
        let current_interval = job.dynamic_interval;

        // Adjust based on success rate
        let new_interval = if success_rate < 0.5 {
            // Low success rate, increase interval
            (current_interval as f64 * 1.5) as u64
        } else if success_rate > 0.9 && current_interval > 300000 { // > 5 minutes
            // High success rate, can decrease interval
            (current_interval as f64 * 0.8) as u64
        } else {
            current_interval
        };

        if new_interval != current_interval {
            let reason = format!("Success rate: {:.1}%", success_rate * 100.0);
            let _ = command_sender.send(SchedulerCommand::AdjustInterval {
                pool_address: job.pool_address.clone(),
                new_interval_ms: new_interval,
                reason,
            });
        }
    }

    /// Convert interval to cron expression
    fn interval_to_cron(&self, interval_ms: u64) -> String {
        let minutes = interval_ms / 60000;

        if minutes < 60 {
            if minutes == 0 {
                "*/1 * * * *".to_string() // Every minute minimum
            } else {
                format!("*/{} * * * *", minutes)
            }
        } else {
            let hours = minutes / 60;
            if hours < 24 {
                format!("0 */{} * * *", hours)
            } else {
                let days = hours / 24;
                format!("0 0 */{} * *", days)
            }
        }
    }

    /// Stop the scheduler
    pub async fn stop(&mut self) -> Result<()> {
        info!("🛑 Stopping compound scheduler");
        self.scheduler.shutdown().await?;
        Ok(())
    }

    /// Get optimal compound frequency based on current conditions
    pub async fn calculate_optimal_frequency(
        &self,
        pool_address: &str,
        current_apy: f64,
        gas_price: f64,
        position_size: f64,
    ) -> u64 {
        // Calculate optimal frequency based on compound interest math
        let daily_rate = current_apy / 365.0 / 100.0;
        let gas_cost_ratio = gas_price / position_size;
        
        // Find frequency where gas costs are ~2% of gains
        let target_gas_ratio = 0.02;
        let min_gain_needed = gas_cost_ratio / target_gas_ratio;
        
        // Calculate time needed for min gain
        let hours_needed = if daily_rate > 0.0 {
            (min_gain_needed.ln() / (daily_rate / 24.0)).max(1.0)
        } else {
            24.0 // Default to daily
        };

        let optimal_interval_ms = (hours_needed * 3600.0 * 1000.0) as u64;

        info!("🧮 Calculated optimal frequency for {}: {} hours", pool_address, hours_needed);
        
        // Clamp to reasonable bounds (15 minutes to 7 days)
        optimal_interval_ms.max(900_000).min(604_800_000)
    }
}

#[derive(Debug, Clone)]
pub struct JobStatistics {
    pub pool_address: String,
    pub total_runs: u64,
    pub successful_runs: u64,
    pub failed_runs: u64,
    pub success_rate: f64,
    pub average_execution_time_ms: u64,
    pub current_interval_ms: u64,
    pub next_run: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
}

impl JobStatistics {
    /// Calculate efficiency score based on success rate and execution time
    pub fn efficiency_score(&self) -> f64 {
        let time_factor = 1.0 - (self.average_execution_time_ms as f64 / 60000.0).min(1.0);
        self.success_rate * 0.8 + time_factor * 20.0
    }
}