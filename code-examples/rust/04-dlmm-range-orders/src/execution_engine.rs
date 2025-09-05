//! Automated order execution engine
//! 
//! This module handles the automated execution of range orders based on 
//! market conditions and execution signals from the order monitor.

use crate::bin_calculations::BinCalculator;
use crate::order_monitor::{ExecutionSignal, SignalType, SignalUrgency};
use crate::types::*;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use log::{debug, error, info, warn};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use saros_dlmm_sdk::{DLMMClient, DLMMError, DLMMResult, SwapParams, TransactionResult};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::time::{sleep, timeout, Duration as TokioDuration, Instant};
use uuid::Uuid;

/// Execution engine for automated order processing
pub struct ExecutionEngine {
    /// DLMM client for blockchain interactions
    client: Arc<DLMMClient>,
    /// Bin calculator for price calculations
    bin_calculator: Arc<BinCalculator>,
    /// Execution configuration
    config: ExecutionConfig,
    /// Pending executions queue
    execution_queue: Arc<RwLock<VecDeque<PendingExecution>>>,
    /// Execution history
    execution_history: Arc<RwLock<VecDeque<CompletedExecution>>>,
    /// Failed executions for retry
    failed_executions: Arc<RwLock<HashMap<Uuid, FailedExecution>>>,
    /// Execution semaphore to limit concurrent executions
    execution_semaphore: Arc<Semaphore>,
    /// Notification sender
    notification_sender: Option<mpsc::UnboundedSender<NotificationType>>,
    /// Gas price optimizer
    gas_optimizer: GasOptimizer,
}

/// Execution configuration
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// Maximum concurrent executions
    pub max_concurrent_executions: usize,
    /// Execution timeout in seconds
    pub execution_timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retry_attempts: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Gas price strategy
    pub gas_strategy: GasStrategy,
    /// Slippage protection
    pub enable_slippage_protection: bool,
    /// MEV protection
    pub enable_mev_protection: bool,
    /// Batch execution threshold
    pub batch_threshold: u32,
    /// Enable smart routing
    pub enable_smart_routing: bool,
}

/// Gas price strategy
#[derive(Debug, Clone, PartialEq)]
pub enum GasStrategy {
    /// Use standard gas price
    Standard,
    /// Use fast gas price (higher cost)
    Fast,
    /// Use economic gas price (slower but cheaper)
    Economic,
    /// Dynamic gas price based on network conditions
    Dynamic,
    /// Custom gas price
    Custom(u64),
}

/// Pending execution in queue
#[derive(Debug, Clone)]
struct PendingExecution {
    pub signal: ExecutionSignal,
    pub order: RangeOrder,
    pub retry_count: u32,
    pub created_at: DateTime<Utc>,
    pub scheduled_for: DateTime<Utc>,
    pub gas_price: Option<u64>,
}

/// Completed execution record
#[derive(Debug, Clone)]
struct CompletedExecution {
    pub order_id: Uuid,
    pub execution: OrderExecution,
    pub profit_loss: Option<Decimal>,
    pub execution_time_ms: u64,
    pub gas_used: u64,
    pub success: bool,
}

/// Failed execution for retry
#[derive(Debug, Clone)]
struct FailedExecution {
    pub order_id: Uuid,
    pub error: String,
    pub retry_count: u32,
    pub next_retry_at: DateTime<Utc>,
    pub original_signal: ExecutionSignal,
}

/// Gas price optimizer
#[derive(Debug, Clone)]
struct GasOptimizer {
    /// Recent gas prices
    recent_gas_prices: Arc<RwLock<VecDeque<GasPrice>>>,
    /// Network congestion indicator
    network_congestion: Arc<RwLock<NetworkCongestion>>,
}

#[derive(Debug, Clone)]
struct GasPrice {
    pub timestamp: DateTime<Utc>,
    pub standard: u64,
    pub fast: u64,
    pub economic: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum NetworkCongestion {
    Low,
    Medium,
    High,
    Critical,
}

/// Execution result with detailed metrics
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub order_id: Uuid,
    pub success: bool,
    pub transaction_signature: Option<Signature>,
    pub executed_amount: Decimal,
    pub execution_price: Decimal,
    pub slippage_bps: u16,
    pub gas_fee: u64,
    pub execution_time_ms: u64,
    pub error: Option<String>,
    pub mev_protection_triggered: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_executions: 10,
            execution_timeout_secs: 30,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
            gas_strategy: GasStrategy::Dynamic,
            enable_slippage_protection: true,
            enable_mev_protection: true,
            batch_threshold: 5,
            enable_smart_routing: true,
        }
    }
}

impl GasOptimizer {
    fn new() -> Self {
        Self {
            recent_gas_prices: Arc::new(RwLock::new(VecDeque::new())),
            network_congestion: Arc::new(RwLock::new(NetworkCongestion::Medium)),
        }
    }

    /// Update gas price data
    async fn update_gas_prices(&self) -> Result<()> {
        // In a real implementation, this would fetch from Solana RPC or gas tracking service
        let mock_gas_price = GasPrice {
            timestamp: Utc::now(),
            standard: 5000,
            fast: 10000,
            economic: 2000,
        };

        let mut prices = self.recent_gas_prices.write().await;
        prices.push_back(mock_gas_price);
        
        // Keep only recent prices (last hour)
        let cutoff = Utc::now() - Duration::hours(1);
        while let Some(front) = prices.front() {
            if front.timestamp < cutoff {
                prices.pop_front();
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Get optimal gas price based on strategy
    async fn get_gas_price(&self, strategy: &GasStrategy, urgency: SignalUrgency) -> Result<u64> {
        let prices = self.recent_gas_prices.read().await;
        let latest = prices.back().ok_or_else(|| anyhow!("No gas price data available"))?;

        let base_price = match strategy {
            GasStrategy::Standard => latest.standard,
            GasStrategy::Fast => latest.fast,
            GasStrategy::Economic => latest.economic,
            GasStrategy::Custom(price) => *price,
            GasStrategy::Dynamic => {
                // Dynamic pricing based on urgency and network conditions
                let congestion = *self.network_congestion.read().await;
                let base = match urgency {
                    SignalUrgency::Critical => latest.fast,
                    SignalUrgency::High => latest.standard,
                    SignalUrgency::Medium => latest.standard,
                    SignalUrgency::Low => latest.economic,
                };

                // Adjust for network congestion
                let multiplier = match congestion {
                    NetworkCongestion::Critical => 2.0,
                    NetworkCongestion::High => 1.5,
                    NetworkCongestion::Medium => 1.0,
                    NetworkCongestion::Low => 0.8,
                };

                (base as f64 * multiplier) as u64
            }
        };

        Ok(base_price)
    }

    /// Update network congestion based on recent execution times
    async fn update_network_congestion(&self, recent_execution_times: &[u64]) {
        if recent_execution_times.is_empty() {
            return;
        }

        let avg_time = recent_execution_times.iter().sum::<u64>() / recent_execution_times.len() as u64;
        
        let congestion = if avg_time > 30000 {  // > 30 seconds
            NetworkCongestion::Critical
        } else if avg_time > 15000 {  // > 15 seconds
            NetworkCongestion::High
        } else if avg_time > 5000 {   // > 5 seconds
            NetworkCongestion::Medium
        } else {
            NetworkCongestion::Low
        };

        let mut current_congestion = self.network_congestion.write().await;
        *current_congestion = congestion;
    }
}

impl ExecutionEngine {
    /// Create new execution engine
    pub fn new(
        client: Arc<DLMMClient>,
        bin_calculator: Arc<BinCalculator>,
        config: ExecutionConfig,
        notification_sender: Option<mpsc::UnboundedSender<NotificationType>>,
    ) -> Self {
        Self {
            client,
            bin_calculator,
            config: config.clone(),
            execution_queue: Arc::new(RwLock::new(VecDeque::new())),
            execution_history: Arc::new(RwLock::new(VecDeque::new())),
            failed_executions: Arc::new(RwLock::new(HashMap::new())),
            execution_semaphore: Arc::new(Semaphore::new(config.max_concurrent_executions)),
            notification_sender,
            gas_optimizer: GasOptimizer::new(),
        }
    }

    /// Start the execution engine
    pub async fn start(&self) -> Result<()> {
        info!("Starting execution engine with {} max concurrent executions", 
              self.config.max_concurrent_executions);

        // Start main execution loop
        let engine_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = engine_clone.execution_loop().await {
                error!("Execution loop error: {}", e);
            }
        });

        // Start retry handler
        let engine_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = engine_clone.retry_loop().await {
                error!("Retry loop error: {}", e);
            }
        });

        // Start gas price updater
        let engine_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = engine_clone.gas_price_update_loop().await {
                error!("Gas price update loop error: {}", e);
            }
        });

        Ok(())
    }

    /// Add execution signal to queue
    pub async fn queue_execution(&self, signal: ExecutionSignal, order: RangeOrder) -> Result<()> {
        // Capture values before moving
        let order_id = order.id.clone();
        let signal_urgency = signal.urgency.clone();
        
        let gas_price = self.gas_optimizer
            .get_gas_price(&self.config.gas_strategy, signal.urgency.clone())
            .await
            .ok();

        let pending = PendingExecution {
            signal,
            order,
            retry_count: 0,
            created_at: Utc::now(),
            scheduled_for: Utc::now(),
            gas_price,
        };

        let mut queue = self.execution_queue.write().await;
        
        // Insert based on urgency (higher urgency = front of queue)
        let insert_position = queue
            .iter()
            .position(|p| p.signal.urgency < pending.signal.urgency)
            .unwrap_or(queue.len());
            
        queue.insert(insert_position, pending);

        debug!("Queued execution for order {} with urgency {:?}", 
               order_id, signal_urgency);

        Ok(())
    }

    /// Execute order immediately (bypass queue)
    pub async fn execute_immediately(
        &self,
        signal: ExecutionSignal,
        order: RangeOrder,
    ) -> Result<ExecutionResult> {
        let _permit = self.execution_semaphore.acquire().await.unwrap();
        self.execute_order_internal(signal, order, 0).await
    }

    /// Get execution statistics
    pub async fn get_execution_stats(&self) -> ExecutionStats {
        let history = self.execution_history.read().await;
        let queue = self.execution_queue.read().await;
        let failed = self.failed_executions.read().await;

        let total_executions = history.len();
        let successful_executions = history.iter().filter(|e| e.success).count();
        let failed_executions = failed.len();
        let pending_executions = queue.len();

        let avg_execution_time = if !history.is_empty() {
            history.iter().map(|e| e.execution_time_ms).sum::<u64>() / history.len() as u64
        } else {
            0
        };

        let total_gas_used = history.iter().map(|e| e.gas_used).sum::<u64>();

        ExecutionStats {
            total_executions: total_executions as u64,
            successful_executions: successful_executions as u64,
            failed_executions: failed_executions as u64,
            pending_executions: pending_executions as u64,
            success_rate_pct: if total_executions > 0 {
                (successful_executions as f64 / total_executions as f64) * 100.0
            } else {
                0.0
            },
            avg_execution_time_ms: avg_execution_time,
            total_gas_used,
            last_execution: history.back().map(|e| e.execution.executed_at),
        }
    }


    /// Main execution loop
    async fn execution_loop(&self) -> Result<()> {
        let mut interval = tokio::time::interval(TokioDuration::from_millis(100));

        loop {
            interval.tick().await;

            // Process execution queue
            if let Err(e) = self.process_execution_queue().await {
                error!("Failed to process execution queue: {}", e);
            }

            // Update network congestion based on recent executions
            self.update_congestion_metrics().await;
        }
    }

    /// Process execution queue
    async fn process_execution_queue(&self) -> Result<()> {
        let now = Utc::now();
        let mut executions_to_process = Vec::new();

        // Get executions that are ready
        {
            let mut queue = self.execution_queue.write().await;
            while let Some(pending) = queue.front() {
                if pending.scheduled_for <= now {
                    if let Some(pending) = queue.pop_front() {
                        executions_to_process.push(pending);
                    }
                } else {
                    break;
                }
            }
        }

        // Process executions concurrently
        let mut handles = Vec::new();
        for pending in executions_to_process {
            let engine_clone = self.clone();
            let handle = tokio::spawn(async move {
                let _permit = engine_clone.execution_semaphore.acquire().await.unwrap();
                engine_clone.execute_order_internal(
                    pending.signal,
                    pending.order,
                    pending.retry_count,
                ).await
            });
            handles.push(handle);
        }

        // Wait for all executions to complete
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Execution task error: {}", e);
            }
        }

        Ok(())
    }

    /// Execute a single order
    async fn execute_order_internal(
        &self,
        signal: ExecutionSignal,
        order: RangeOrder,
        retry_count: u32,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        
        info!("Executing order {} (attempt {})", order.id, retry_count + 1);

        // Pre-execution validation
        if let Err(e) = self.validate_execution(&signal, &order).await {
            return Ok(ExecutionResult {
                order_id: order.id,
                success: false,
                transaction_signature: None,
                executed_amount: Decimal::ZERO,
                execution_price: Decimal::ZERO,
                slippage_bps: 0,
                gas_fee: 0,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                error: Some(e.to_string()),
                mev_protection_triggered: false,
            });
        }

        // Get optimal gas price
        let gas_price = self.gas_optimizer
            .get_gas_price(&self.config.gas_strategy, signal.urgency.clone())
            .await?;

        // Prepare swap parameters
        let swap_params = self.prepare_swap_params(&order, gas_price).await?;

        // Execute with timeout
        let execution_result = timeout(
            TokioDuration::from_secs(self.config.execution_timeout_secs),
            self.execute_swap(swap_params, &order, &signal),
        )
        .await;

        let execution_time = start_time.elapsed().as_millis() as u64;

        match execution_result {
            Ok(Ok(tx_result)) => {
                // Successful execution
                let result = ExecutionResult {
                    order_id: order.id,
                    success: true,
                    transaction_signature: Some(tx_result.signature),
                    executed_amount: order.amount, // In real implementation, get from transaction
                    execution_price: order.target_price, // In real implementation, calculate actual price
                    slippage_bps: 0, // Calculate actual slippage
                    gas_fee: tx_result.gas_used,
                    execution_time_ms: execution_time,
                    error: None,
                    mev_protection_triggered: false,
                };

                // Record successful execution
                let execution = OrderExecution {
                    order_id: order.id,
                    signature: tx_result.signature.to_string(),
                    executed_amount: result.executed_amount,
                    execution_price: result.execution_price,
                    executed_at: Utc::now(),
                    gas_fee: result.gas_fee,
                    slippage_bps: result.slippage_bps,
                };

                let completed = CompletedExecution {
                    order_id: order.id,
                    execution: execution.clone(),
                    profit_loss: None, // Calculate if needed
                    execution_time_ms: execution_time,
                    gas_used: result.gas_fee,
                    success: true,
                };

                {
                    let mut history = self.execution_history.write().await;
                    history.push_back(completed);
                    
                    // Keep history bounded
                    if history.len() > 1000 {
                        history.pop_front();
                    }
                }

                // Send notification
                if let Some(sender) = &self.notification_sender {
                    let notification = NotificationType::OrderExecuted {
                        order_id: order.id,
                        amount: result.executed_amount,
                        price: result.execution_price,
                    };
                    let _ = sender.send(notification);
                }

                info!("Successfully executed order {} in {}ms", 
                      order.id, execution_time);

                Ok(result)
            }
            Ok(Err(e)) => {
                // Failed execution
                let error_msg = e.to_string();
                warn!("Failed to execute order {}: {}", order.id, error_msg);

                // Handle retry
                if retry_count < self.config.max_retry_attempts {
                    self.schedule_retry(signal, order.clone(), retry_count + 1, &error_msg).await?;
                    
                    // Return intermediate result for retry case
                    Ok(ExecutionResult {
                        order_id: order.id,
                        success: false,
                        transaction_signature: None,
                        executed_amount: Decimal::ZERO,
                        execution_price: Decimal::ZERO,
                        slippage_bps: 0,
                        gas_fee: 0,
                        execution_time_ms: execution_time,
                        error: Some(format!("Retry scheduled: {}", error_msg)),
                        mev_protection_triggered: false,
                    })
                } else {
                    // Max retries reached, record as permanently failed
                    let result = ExecutionResult {
                        order_id: order.id,
                        success: false,
                        transaction_signature: None,
                        executed_amount: Decimal::ZERO,
                        execution_price: Decimal::ZERO,
                        slippage_bps: 0,
                        gas_fee: 0,
                        execution_time_ms: execution_time,
                        error: Some(error_msg.clone()),
                        mev_protection_triggered: false,
                    };

                    let failed = FailedExecution {
                        order_id: order.id,
                        error: error_msg.clone(),
                        retry_count,
                        next_retry_at: Utc::now(),
                        original_signal: signal.clone(),
                    };

                    {
                        let mut failed_executions = self.failed_executions.write().await;
                        failed_executions.insert(order.id, failed);
                    }

                    // Send failure notification
                    if let Some(sender) = &self.notification_sender {
                        let notification = NotificationType::OrderFailed {
                            order_id: order.id,
                            error: error_msg.clone(),
                        };
                        let _ = sender.send(notification);
                    }

                    Ok(result)
                }
            }
            Err(timeout_err) => {
                // Execution timeout
                let error_msg = "Execution timeout".to_string();

                warn!("Failed to execute order {}: {}", order.id, error_msg);

                // Handle retry
                if retry_count < self.config.max_retry_attempts {
                    self.schedule_retry(signal, order.clone(), retry_count + 1, &error_msg).await?;
                    
                    // Return intermediate result for retry case
                    Ok(ExecutionResult {
                        order_id: order.id,
                        success: false,
                        transaction_signature: None,
                        executed_amount: Decimal::ZERO,
                        execution_price: Decimal::ZERO,
                        slippage_bps: 0,
                        gas_fee: 0,
                        execution_time_ms: execution_time,
                        error: Some(format!("Timeout - retry scheduled: {}", error_msg)),
                        mev_protection_triggered: false,
                    })
                } else {
                    // Max retries reached, record as permanently failed
                    let result = ExecutionResult {
                        order_id: order.id,
                        success: false,
                        transaction_signature: None,
                        executed_amount: Decimal::ZERO,
                        execution_price: Decimal::ZERO,
                        slippage_bps: 0,
                        gas_fee: 0,
                        execution_time_ms: execution_time,
                        error: Some(error_msg.clone()),
                        mev_protection_triggered: false,
                    };

                    // Send failure notification
                    if let Some(sender) = &self.notification_sender {
                        let notification = NotificationType::OrderFailed {
                            order_id: order.id,
                            error: error_msg,
                        };
                        let _ = sender.send(notification);
                    }

                    Ok(result)
                }
            }
        }
    }

    /// Validate execution conditions
    async fn validate_execution(&self, signal: &ExecutionSignal, order: &RangeOrder) -> Result<()> {
        // Check order status
        if !matches!(order.status, OrderStatus::Pending | OrderStatus::PartiallyFilled) {
            return Err(anyhow!("Order is not in executable state: {:?}", order.status));
        }

        // Check expiry
        if let Some(expires_at) = order.expires_at {
            if Utc::now() > expires_at {
                return Err(anyhow!("Order has expired"));
            }
        }

        // Check slippage protection
        if self.config.enable_slippage_protection && signal.expected_slippage > Decimal::from_str("0.05").unwrap() {
            return Err(anyhow!("Expected slippage {} exceeds protection threshold", signal.expected_slippage));
        }

        // Check minimum liquidity
        if signal.available_liquidity < Decimal::from(1000) {
            return Err(anyhow!("Insufficient liquidity for execution: {}", signal.available_liquidity));
        }

        Ok(())
    }

    /// Prepare swap parameters for execution
    async fn prepare_swap_params(&self, order: &RangeOrder, gas_price: u64) -> Result<SwapParams> {
        // This would prepare the actual swap parameters for the DLMM SDK
        // For now, return a mock structure
        Ok(SwapParams {
            pool_address: order.pool_address,
            amount_in: order.amount,
            minimum_amount_out: order.amount * (Decimal::ONE - Decimal::from_str("0.005").unwrap()), // 0.5% slippage
            gas_price: Some(gas_price),
            slippage_bps: Some(50), // 0.5% in basis points
        })
    }

    /// Execute the actual swap
    async fn execute_swap(
        &self,
        params: SwapParams,
        order: &RangeOrder,
        signal: &ExecutionSignal,
    ) -> Result<TransactionResult> {
        // In a real implementation, this would call the DLMM SDK
        // For now, return a mock successful transaction
        
        debug!("Executing swap for order {} at bin {}", order.id, order.bin_id);
        
        // Simulate execution time based on network conditions
        let execution_delay = match signal.urgency {
            SignalUrgency::Critical => 100,
            SignalUrgency::High => 200,
            SignalUrgency::Medium => 500,
            SignalUrgency::Low => 1000,
        };
        
        sleep(TokioDuration::from_millis(execution_delay)).await;
        
        // Mock successful result
        Ok(TransactionResult {
            signature: solana_sdk::signature::Signature::new_unique(),
            gas_used: params.gas_price.unwrap_or(5000),
            success: true,
        })
    }

    /// Schedule retry for failed execution
    async fn schedule_retry(
        &self,
        signal: ExecutionSignal,
        order: RangeOrder,
        retry_count: u32,
        error: &str,
    ) -> Result<()> {
        let retry_delay = Duration::milliseconds(self.config.retry_delay_ms as i64 * (1 << retry_count) as i64);
        let scheduled_for = Utc::now() + retry_delay;
        let order_id = order.id; // Store before moving

        let failed_execution = FailedExecution {
            order_id,
            error: error.to_string(),
            retry_count,
            next_retry_at: scheduled_for,
            original_signal: signal.clone(),
        };

        {
            let mut failed = self.failed_executions.write().await;
            failed.insert(order_id, failed_execution);
        }

        // Add back to execution queue with delay
        let pending = PendingExecution {
            signal,
            order,
            retry_count,
            created_at: Utc::now(),
            scheduled_for,
            gas_price: None, // Will be recalculated
        };

        let mut queue = self.execution_queue.write().await;
        queue.push_back(pending);

        info!("Scheduled retry {} for order {} at {}", retry_count, order_id, scheduled_for);
        Ok(())
    }

    /// Retry loop for handling failed executions
    async fn retry_loop(&self) -> Result<()> {
        let mut interval = tokio::time::interval(TokioDuration::from_secs(10));

        loop {
            interval.tick().await;
            
            // Clean up old failed executions
            self.cleanup_failed_executions().await?;
        }
    }

    /// Clean up old failed executions
    async fn cleanup_failed_executions(&self) -> Result<()> {
        let cutoff = Utc::now() - Duration::hours(24);
        let mut failed = self.failed_executions.write().await;
        failed.retain(|_, execution| execution.next_retry_at > cutoff);
        Ok(())
    }

    /// Gas price update loop
    async fn gas_price_update_loop(&self) -> Result<()> {
        let mut interval = tokio::time::interval(TokioDuration::from_secs(30));

        loop {
            interval.tick().await;
            
            if let Err(e) = self.gas_optimizer.update_gas_prices().await {
                warn!("Failed to update gas prices: {}", e);
            }
        }
    }

    /// Update network congestion metrics
    async fn update_congestion_metrics(&self) {
        let history = self.execution_history.read().await;
        let recent_times: Vec<u64> = history
            .iter()
            .rev()
            .take(50)
            .map(|e| e.execution_time_ms)
            .collect();

        if !recent_times.is_empty() {
            self.gas_optimizer.update_network_congestion(&recent_times).await;
        }
    }

    /// Cancel pending execution
    pub async fn cancel_execution(&self, order_id: Uuid) -> Result<bool> {
        let mut queue = self.execution_queue.write().await;
        let original_len = queue.len();
        queue.retain(|pending| pending.order.id != order_id);
        
        let cancelled = queue.len() < original_len;
        if cancelled {
            info!("Cancelled pending execution for order {}", order_id);
        }
        
        Ok(cancelled)
    }

    /// Get queue status
    pub async fn get_queue_status(&self) -> QueueStatus {
        let queue = self.execution_queue.read().await;
        let failed = self.failed_executions.read().await;
        
        QueueStatus {
            pending_count: queue.len(),
            failed_count: failed.len(),
            oldest_pending: queue.front().map(|p| p.created_at),
            newest_pending: queue.back().map(|p| p.created_at),
        }
    }
}

/// Clone implementation for ExecutionEngine
impl Clone for ExecutionEngine {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            bin_calculator: Arc::clone(&self.bin_calculator),
            config: self.config.clone(),
            execution_queue: Arc::clone(&self.execution_queue),
            execution_history: Arc::clone(&self.execution_history),
            failed_executions: Arc::clone(&self.failed_executions),
            execution_semaphore: Arc::clone(&self.execution_semaphore),
            notification_sender: self.notification_sender.clone(),
            gas_optimizer: self.gas_optimizer.clone(),
        }
    }
}

/// Execution statistics
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub pending_executions: u64,
    pub success_rate_pct: f64,
    pub avg_execution_time_ms: u64,
    pub total_gas_used: u64,
    pub last_execution: Option<DateTime<Utc>>,
}

/// Queue status information
#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub pending_count: usize,
    pub failed_count: usize,
    pub oldest_pending: Option<DateTime<Utc>>,
    pub newest_pending: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bin_calculations::BinCalculator;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_execution_engine_creation() {
        let client = Arc::new(saros_dlmm_sdk::DLMMClient::new("mock://test").unwrap());
        let bin_calculator = Arc::new(BinCalculator::new(20, dec!(100)).unwrap());
        let config = ExecutionConfig::default();
        
        let engine = ExecutionEngine::new(client, bin_calculator, config, None);
        
        let stats = engine.get_execution_stats().await;
        assert_eq!(stats.total_executions, 0);
        assert_eq!(stats.pending_executions, 0);
    }

    #[tokio::test]
    async fn test_gas_optimizer() {
        let optimizer = GasOptimizer::new();
        
        // Update with mock data
        optimizer.update_gas_prices().await.unwrap();
        
        // Test gas price retrieval
        let standard_price = optimizer.get_gas_price(&GasStrategy::Standard, SignalUrgency::Medium).await.unwrap();
        let fast_price = optimizer.get_gas_price(&GasStrategy::Fast, SignalUrgency::Critical).await.unwrap();
        
        assert!(fast_price >= standard_price);
    }

    #[tokio::test]
    async fn test_execution_queue() {
        let client = Arc::new(saros_dlmm_sdk::DLMMClient::new("mock://test").unwrap());
        let bin_calculator = Arc::new(BinCalculator::new(20, dec!(100)).unwrap());
        let config = ExecutionConfig::default();
        let engine = ExecutionEngine::new(client, bin_calculator, config, None);

        let test_order = RangeOrder {
            id: Uuid::new_v4(),
            pool_address: Pubkey::new_unique(),
            order_type: OrderType::LimitBuy,
            bin_id: 95,
            amount: dec!(100),
            target_price: dec!(95),
            status: OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            filled_amount: Decimal::ZERO,
            avg_fill_price: None,
            position_id: None,
            expires_at: None,
            max_slippage_bps: 100,
            strategy_id: None,
        };

        let signal = ExecutionSignal {
            order_id: test_order.id,
            signal_type: SignalType::PriceTarget,
            urgency: SignalUrgency::High,
            expected_slippage: dec!(0.01),
            available_liquidity: dec!(10000),
            timestamp: Utc::now(),
        };

        engine.queue_execution(signal, test_order).await.unwrap();
        
        let queue_status = engine.get_queue_status().await;
        assert_eq!(queue_status.pending_count, 1);
    }
}