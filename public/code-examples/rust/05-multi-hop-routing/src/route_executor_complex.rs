use anyhow::Result;
use rust_decimal::Decimal;
use solana_sdk::{
    pubkey::Pubkey,
    transaction::Transaction,
    signature::{Keypair, Signature, Signer},
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn, error};

use crate::pool_graph::PoolGraph;
use crate::types::*;
use saros_dlmm_sdk::{SarosClient, TransactionBuilder};

/// Multi-hop route execution with advanced transaction management
pub struct RouteExecutor {
    /// Saros client for transaction submission
    client: Arc<SarosClient>,
    
    /// Pool graph for route validation
    pool_graph: Arc<PoolGraph>,
    
    /// Transaction builder for multi-hop swaps
    transaction_builder: Arc<TransactionBuilder>,
    
    /// Gas estimation cache
    gas_cache: Arc<tokio::sync::RwLock<HashMap<String, GasEstimation>>>,
    
    /// Execution metrics
    metrics: Arc<tokio::sync::RwLock<ExecutionMetrics>>,
    
    /// MEV protection settings
    mev_protection: MevProtectionConfig,
}

#[derive(Debug, Default)]
struct ExecutionMetrics {
    total_executions: u64,
    successful_executions: u64,
    failed_executions: u64,
    avg_execution_time_ms: u64,
    total_gas_used: u64,
    total_volume_usd: Decimal,
}

#[derive(Debug, Clone)]
struct MevProtectionConfig {
    use_private_mempool: bool,
    max_priority_fee: Decimal,
    slippage_buffer: Decimal,
    bundle_transactions: bool,
}

impl Default for MevProtectionConfig {
    fn default() -> Self {
        Self {
            use_private_mempool: false,
            max_priority_fee: rust_decimal_macros::dec!(0.001),
            slippage_buffer: rust_decimal_macros::dec!(0.005), // 0.5% extra buffer
            bundle_transactions: true,
        }
    }
}

impl RouteExecutor {
    pub async fn new() -> Result<Self> {
        let client = Arc::new(SarosClient::new_mock()?);
        let pool_graph = PoolGraph::new().await?;
        let transaction_builder = Arc::new(TransactionBuilder::new());
        
        Ok(Self {
            client,
            pool_graph,
            transaction_builder,
            gas_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            metrics: Arc::new(tokio::sync::RwLock::new(ExecutionMetrics::default())),
            mev_protection: MevProtectionConfig::default(),
        })
    }
    
    /// Simulate route execution without submitting transactions
    pub async fn simulate_route_execution(
        &self,
        route_id: &str,
        amount: Decimal,
    ) -> Result<RouteExecutionSimulation> {
        info!("Simulating route execution: {} with amount {}", route_id, amount);
        
        // In a real implementation, you would:
        // 1. Retrieve the route from storage/cache
        // 2. Build transaction instructions
        // 3. Simulate each step
        
        // Mock simulation results
        let simulation = RouteExecutionSimulation {
            route_id: route_id.to_string(),
            expected_output: amount * rust_decimal_macros::dec!(0.995), // 0.5% slippage
            total_price_impact: rust_decimal_macros::dec!(0.015), // 1.5%
            estimated_gas: rust_decimal_macros::dec!(0.01), // 0.01 SOL
            success_probability: rust_decimal_macros::dec!(0.95), // 95%
            warnings: self.generate_simulation_warnings(amount).await?,
            execution_steps: self.generate_execution_steps(route_id, amount).await?,
        };
        
        Ok(simulation)
    }
    
    /// Execute a multi-hop route with full transaction management
    pub async fn execute_route(
        &self,
        route_id: &str,
        amount: Decimal,
        user_keypair: &Keypair,
        options: ExecutionOptions,
    ) -> Result<ExecutionResult> {
        let start_time = std::time::Instant::now();
        
        info!("Executing route: {} with amount {}", route_id, amount);
        
        // 1. Pre-execution validation
        self.validate_execution_request(route_id, amount, &options).await?;
        
        // 2. Build transaction instructions
        let instructions = self.build_route_instructions(route_id, amount, &options).await?;
        
        // 3. Estimate and optimize gas
        let gas_config = self.optimize_gas_configuration(&instructions).await?;
        
        // 4. Apply MEV protection if enabled
        let protected_instructions = if self.mev_protection.use_private_mempool {
            self.apply_mev_protection(instructions, &gas_config).await?
        } else {
            instructions
        };
        
        // 5. Build and simulate transaction
        let transaction = self.transaction_builder.build_transaction(
            protected_instructions,
            user_keypair.pubkey(),
            gas_config.clone(),
        ).await?;
        
        // 6. Final simulation check
        let simulation_result = self.client.simulate_transaction(&transaction).await?;
        
        if !simulation_result.success {
            return Err(RoutingError::SimulationFailed {
                reason: simulation_result.error.unwrap_or("Unknown simulation error".to_string())
            }.into());
        }
        
        // 7. Submit transaction
        let execution_result = if options.dry_run {
            ExecutionResult {
                route_id: route_id.to_string(),
                transaction_signature: None,
                actual_output: simulation_result.expected_output,
                gas_used: simulation_result.gas_used,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                success: true,
                error_message: None,
                steps_completed: simulation_result.steps.len() as u8,
                mev_protection_used: self.mev_protection.use_private_mempool,
            }
        } else {
            self.submit_transaction_with_retry(
                transaction,
                user_keypair,
                &options,
                start_time.elapsed().as_millis() as u64,
            ).await?
        };
        
        // 8. Update metrics
        self.update_execution_metrics(&execution_result).await;
        
        info!("Route execution completed: success={}, time={}ms", 
            execution_result.success, execution_result.execution_time_ms);
        
        Ok(execution_result)
    }
    
    /// Execute arbitrage opportunity with atomic transaction bundling
    pub async fn execute_arbitrage(
        &self,
        opportunity: &ArbitrageOpportunity,
        keypair: &Keypair,
        execution_amount: Decimal,
    ) -> Result<ArbitrageExecutionResult> {
        info!("Executing arbitrage opportunity: {} with amount {}", 
            opportunity.id, execution_amount);
        
        let start_time = std::time::Instant::now();
        
        // 1. Validate arbitrage opportunity is still profitable
        let current_profitability = self.validate_arbitrage_profitability(
            opportunity,
            execution_amount,
        ).await?;
        
        if current_profitability.profit_usd < rust_decimal_macros::dec!(10) {
            return Err(RoutingError::CalculationError {
                message: "Arbitrage no longer profitable".to_string()
            }.into());
        }
        
        // 2. Build atomic transaction bundle for the arbitrage cycle
        let bundle_instructions = self.build_arbitrage_bundle(
            &opportunity.cycle,
            execution_amount,
        ).await?;
        
        // 3. Calculate optimal gas and priority fees for fast execution
        let gas_config = self.calculate_arbitrage_gas_config(
            &opportunity.cycle,
            current_profitability.expected_profit,
        ).await?;
        
        // 4. Build and submit transaction with high priority
        let transaction = self.transaction_builder.build_priority_transaction(
            bundle_instructions,
            keypair.pubkey(),
            gas_config,
        ).await?;
        
        // 5. Submit with aggressive retry logic
        let signature = self.submit_arbitrage_transaction(
            transaction,
            keypair,
            3, // max retries
        ).await?;
        
        // 6. Monitor transaction confirmation
        let confirmation_result = self.monitor_transaction_confirmation(
            signature,
            30, // 30 second timeout
        ).await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        let result = ArbitrageExecutionResult {
            opportunity_id: opportunity.id.clone(),
            transaction_signature: confirmation_result.signature,
            actual_profit_usd: confirmation_result.actual_output,
            execution_cost_usd: confirmation_result.execution_cost,
            net_profit_usd: confirmation_result.actual_output - confirmation_result.execution_cost,
            execution_time_ms: execution_time,
            success: confirmation_result.success,
            slippage_experienced: confirmation_result.slippage,
        };
        
        info!("Arbitrage execution result: profit=${:.2}, cost=${:.2}, net=${:.2}",
            result.actual_profit_usd,
            result.execution_cost_usd,
            result.net_profit_usd
        );
        
        Ok(result)
    }
    
    /// Batch execute multiple routes for portfolio rebalancing
    pub async fn batch_execute_routes(
        &self,
        routes: Vec<BatchRouteRequest>,
        keypair: &Keypair,
        options: BatchExecutionOptions,
    ) -> Result<BatchExecutionResult> {
        info!("Batch executing {} routes", routes.len());
        
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();
        let mut total_success = 0;
        
        // Execute routes in parallel or sequentially based on options
        if options.parallel_execution {
            // Parallel execution with controlled concurrency
            let semaphore = Arc::new(tokio::sync::Semaphore::new(options.max_concurrent));
            let mut handles = Vec::new();
            
            for route_request in routes {
                let permit = semaphore.clone().acquire_owned().await?;
                let executor = self.clone();
                let keypair_clone = keypair.try_clone()?;
                
                let handle = tokio::spawn(async move {
                    let _permit = permit;
                    executor.execute_single_batch_route(route_request, &keypair_clone).await
                });
                
                handles.push(handle);
            }
            
            // Wait for all routes to complete
            for handle in handles {
                match handle.await? {
                    Ok(result) => {
                        if result.success {
                            total_success += 1;
                        }
                        results.push(result);
                    }
                    Err(e) => {
                        warn!("Batch route execution failed: {}", e);
                        results.push(ExecutionResult {
                            route_id: "unknown".to_string(),
                            transaction_signature: None,
                            actual_output: Decimal::ZERO,
                            gas_used: 0,
                            execution_time_ms: 0,
                            success: false,
                            error_message: Some(e.to_string()),
                            steps_completed: 0,
                            mev_protection_used: false,
                        });
                    }
                }
            }
        } else {
            // Sequential execution
            for route_request in routes {
                match self.execute_single_batch_route(route_request, keypair).await {
                    Ok(result) => {
                        if result.success {
                            total_success += 1;
                        }
                        results.push(result);
                    }
                    Err(e) => {
                        warn!("Sequential route execution failed: {}", e);
                        results.push(ExecutionResult {
                            route_id: "unknown".to_string(),
                            transaction_signature: None,
                            actual_output: Decimal::ZERO,
                            gas_used: 0,
                            execution_time_ms: 0,
                            success: false,
                            error_message: Some(e.to_string()),
                            steps_completed: 0,
                            mev_protection_used: false,
                        });
                        
                        // Stop on first failure if fail-fast is enabled
                        if options.fail_fast {
                            break;
                        }
                    }
                }
            }
        }
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(BatchExecutionResult {
            total_routes: results.len(),
            successful_routes: total_success,
            failed_routes: results.len() - total_success,
            results,
            total_execution_time_ms: execution_time,
            success_rate: Decimal::from(total_success) / Decimal::from(results.len()),
        })
    }
    
    // Private helper methods
    
    async fn validate_execution_request(
        &self,
        route_id: &str,
        amount: Decimal,
        options: &ExecutionOptions,
    ) -> Result<()> {
        if amount.is_zero() {
            return Err(RoutingError::CalculationError {
                message: "Execution amount must be greater than zero".to_string()
            }.into());
        }
        
        // Validate slippage tolerance
        if options.max_slippage > rust_decimal_macros::dec!(0.5) {
            return Err(RoutingError::CalculationError {
                message: "Slippage tolerance too high (>50%)".to_string()
            }.into());
        }
        
        debug!("Execution request validated for route: {}", route_id);
        Ok(())
    }
    
    async fn build_route_instructions(
        &self,
        route_id: &str,
        amount: Decimal,
        options: &ExecutionOptions,
    ) -> Result<Vec<RouteInstruction>> {
        // Mock instruction building - in real implementation:
        // 1. Retrieve route from cache/storage
        // 2. Build swap instructions for each hop
        // 3. Add slippage protection
        // 4. Add deadline constraints
        
        debug!("Building route instructions for: {}", route_id);
        
        // Mock instructions for a 2-hop route
        Ok(vec![
            RouteInstruction {
                instruction_type: InstructionType::Swap,
                pool_address: Pubkey::new_unique(),
                amount_in: amount,
                minimum_amount_out: amount * (Decimal::ONE - options.max_slippage),
                deadline: chrono::Utc::now().timestamp() as u64 + 300, // 5 minutes
            },
            RouteInstruction {
                instruction_type: InstructionType::Swap,
                pool_address: Pubkey::new_unique(),
                amount_in: amount * rust_decimal_macros::dec!(0.997), // After first swap
                minimum_amount_out: amount * rust_decimal_macros::dec!(0.99),
                deadline: chrono::Utc::now().timestamp() as u64 + 300,
            },
        ])
    }
    
    async fn optimize_gas_configuration(
        &self,
        instructions: &[RouteInstruction],
    ) -> Result<GasEstimation> {
        let base_gas = 20000;
        let per_instruction_gas = 100000;
        let total_compute_units = base_gas + (instructions.len() as u64 * per_instruction_gas);
        
        // Dynamic priority fee based on network congestion
        let priority_fee = self.calculate_dynamic_priority_fee().await?;
        
        Ok(GasEstimation {
            base_gas,
            per_hop_gas: per_instruction_gas,
            compute_units: total_compute_units,
            priority_fee,
        })
    }
    
    async fn calculate_dynamic_priority_fee(&self) -> Result<Decimal> {
        // Mock dynamic fee calculation - in real implementation:
        // 1. Query current network fees
        // 2. Analyze recent transaction success rates
        // 3. Calculate optimal fee for fast inclusion
        
        let base_fee = rust_decimal_macros::dec!(0.000005);
        let congestion_multiplier = rust_decimal_macros::dec!(1.5); // Mock congestion
        
        Ok(base_fee * congestion_multiplier)
    }
    
    async fn apply_mev_protection(
        &self,
        instructions: Vec<RouteInstruction>,
        gas_config: &GasEstimation,
    ) -> Result<Vec<RouteInstruction>> {
        debug!("Applying MEV protection to {} instructions", instructions.len());
        
        let mut protected_instructions = instructions;
        
        // Add slippage buffer for MEV protection
        for instruction in &mut protected_instructions {
            let buffer = self.mev_protection.slippage_buffer;
            instruction.minimum_amount_out = instruction.minimum_amount_out * (Decimal::ONE - buffer);
        }
        
        // Could add other MEV protection strategies:
        // - Private mempool submission
        // - Transaction bundling
        // - Dynamic deadline adjustment
        
        Ok(protected_instructions)
    }
    
    async fn submit_transaction_with_retry(
        &self,
        transaction: Transaction,
        keypair: &Keypair,
        options: &ExecutionOptions,
        simulation_time_ms: u64,
    ) -> Result<ExecutionResult> {
        let mut attempts = 0;
        let max_retries = options.max_retries.unwrap_or(3);
        
        while attempts < max_retries {
            attempts += 1;
            
            match self.client.send_transaction(&transaction).await {
                Ok(signature) => {
                    // Wait for confirmation
                    match self.wait_for_confirmation(signature, 30).await {
                        Ok(confirmation) => {
                            return Ok(ExecutionResult {
                                route_id: "executed".to_string(),
                                transaction_signature: Some(signature),
                                actual_output: confirmation.actual_output,
                                gas_used: confirmation.gas_used,
                                execution_time_ms: simulation_time_ms + confirmation.confirmation_time_ms,
                                success: true,
                                error_message: None,
                                steps_completed: confirmation.steps_completed,
                                mev_protection_used: self.mev_protection.use_private_mempool,
                            });
                        }
                        Err(e) => {
                            warn!("Transaction confirmation failed (attempt {}): {}", attempts, e);
                            if attempts >= max_retries {
                                return Ok(ExecutionResult {
                                    route_id: "failed".to_string(),
                                    transaction_signature: Some(signature),
                                    actual_output: Decimal::ZERO,
                                    gas_used: 0,
                                    execution_time_ms: simulation_time_ms,
                                    success: false,
                                    error_message: Some(e.to_string()),
                                    steps_completed: 0,
                                    mev_protection_used: self.mev_protection.use_private_mempool,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Transaction submission failed (attempt {}): {}", attempts, e);
                    if attempts >= max_retries {
                        return Ok(ExecutionResult {
                            route_id: "failed".to_string(),
                            transaction_signature: None,
                            actual_output: Decimal::ZERO,
                            gas_used: 0,
                            execution_time_ms: simulation_time_ms,
                            success: false,
                            error_message: Some(e.to_string()),
                            steps_completed: 0,
                            mev_protection_used: self.mev_protection.use_private_mempool,
                        });
                    }
                }
            }
            
            // Wait before retry
            tokio::time::sleep(tokio::time::Duration::from_millis(1000 * attempts)).await;
        }
        
        unreachable!()
    }
    
    async fn wait_for_confirmation(
        &self,
        signature: Signature,
        timeout_seconds: u64,
    ) -> Result<TransactionConfirmation> {
        let start = std::time::Instant::now();
        let timeout_duration = tokio::time::Duration::from_secs(timeout_seconds);
        
        while start.elapsed() < timeout_duration {
            match self.client.get_transaction_status(signature).await {
                Ok(status) if status.confirmed => {
                    return Ok(TransactionConfirmation {
                        signature,
                        actual_output: status.output_amount.unwrap_or_default(),
                        gas_used: status.gas_used.unwrap_or(0),
                        confirmation_time_ms: start.elapsed().as_millis() as u64,
                        steps_completed: status.steps_completed.unwrap_or(0),
                        slippage: status.slippage.unwrap_or_default(),
                        success: true,
                        execution_cost: status.execution_cost.unwrap_or_default(),
                    });
                }
                Ok(status) if status.failed => {
                    return Err(RoutingError::SimulationFailed {
                        reason: status.error.unwrap_or("Transaction failed".to_string())
                    }.into());
                }
                Ok(_) => {
                    // Still pending, wait and check again
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                }
                Err(e) => {
                    warn!("Error checking transaction status: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                }
            }
        }
        
        Err(RoutingError::SimulationFailed {
            reason: "Transaction confirmation timeout".to_string()
        }.into())
    }
    
    async fn generate_simulation_warnings(&self, amount: Decimal) -> Result<Vec<String>> {
        let mut warnings = Vec::new();
        
        if amount > rust_decimal_macros::dec!(100000) {
            warnings.push("Large trade size may cause significant price impact".to_string());
        }
        
        // Add more contextual warnings based on market conditions
        warnings.push("Network congestion may cause delays".to_string());
        
        Ok(warnings)
    }
    
    async fn generate_execution_steps(
        &self,
        route_id: &str,
        amount: Decimal,
    ) -> Result<Vec<ExecutionStep>> {
        // Mock execution steps
        Ok(vec![
            ExecutionStep {
                step_number: 1,
                pool_address: Pubkey::new_unique(),
                from_token: Pubkey::new_unique(),
                to_token: Pubkey::new_unique(),
                amount_in: amount,
                expected_amount_out: amount * rust_decimal_macros::dec!(0.997),
                gas_cost: rust_decimal_macros::dec!(0.005),
            },
            ExecutionStep {
                step_number: 2,
                pool_address: Pubkey::new_unique(),
                from_token: Pubkey::new_unique(),
                to_token: Pubkey::new_unique(),
                amount_in: amount * rust_decimal_macros::dec!(0.997),
                expected_amount_out: amount * rust_decimal_macros::dec!(0.99),
                gas_cost: rust_decimal_macros::dec!(0.005),
            },
        ])
    }
    
    async fn validate_arbitrage_profitability(
        &self,
        opportunity: &ArbitrageOpportunity,
        amount: Decimal,
    ) -> Result<ProfitabilityCheck> {
        // Re-calculate profitability with current market conditions
        // This is a simplified version
        
        let estimated_profit = opportunity.expected_profit_usd * (amount / opportunity.required_capital_usd);
        let gas_cost = rust_decimal_macros::dec!(0.02); // Estimated gas cost
        
        Ok(ProfitabilityCheck {
            profit_usd: estimated_profit - gas_cost,
            expected_profit: estimated_profit,
            gas_cost,
            still_profitable: estimated_profit > gas_cost * rust_decimal_macros::dec!(2.0),
        })
    }
    
    async fn build_arbitrage_bundle(
        &self,
        cycle: &[ArbitrageCycleHop],
        amount: Decimal,
    ) -> Result<Vec<RouteInstruction>> {
        let mut instructions = Vec::new();
        let mut current_amount = amount;
        
        for (i, hop) in cycle.iter().enumerate() {
            instructions.push(RouteInstruction {
                instruction_type: InstructionType::Swap,
                pool_address: hop.pool_address,
                amount_in: current_amount,
                minimum_amount_out: hop.expected_amount_out * rust_decimal_macros::dec!(0.99),
                deadline: chrono::Utc::now().timestamp() as u64 + 60, // 1 minute deadline
            });
            
            current_amount = hop.expected_amount_out;
        }
        
        Ok(instructions)
    }
    
    async fn calculate_arbitrage_gas_config(
        &self,
        cycle: &[ArbitrageCycleHop],
        expected_profit: Decimal,
    ) -> Result<GasEstimation> {
        let base_gas = 30000; // Higher base for arbitrage
        let per_hop_gas = 150000; // More gas per hop for complex arbitrage
        let compute_units = base_gas + (cycle.len() as u64 * per_hop_gas);
        
        // Aggressive priority fee for arbitrage (up to 10% of expected profit)
        let max_fee_from_profit = expected_profit * rust_decimal_macros::dec!(0.1);
        let network_fee = self.calculate_dynamic_priority_fee().await?;
        let priority_fee = network_fee.max(max_fee_from_profit);
        
        Ok(GasEstimation {
            base_gas,
            per_hop_gas,
            compute_units,
            priority_fee,
        })
    }
    
    async fn submit_arbitrage_transaction(
        &self,
        transaction: Transaction,
        keypair: &Keypair,
        max_retries: u32,
    ) -> Result<Signature> {
        for attempt in 1..=max_retries {
            match self.client.send_transaction(&transaction).await {
                Ok(signature) => return Ok(signature),
                Err(e) => {
                    error!("Arbitrage transaction submission failed (attempt {}): {}", attempt, e);
                    if attempt >= max_retries {
                        return Err(e.into());
                    }
                    // Very short retry delay for arbitrage
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
        
        unreachable!()
    }
    
    async fn monitor_transaction_confirmation(
        &self,
        signature: Signature,
        timeout_seconds: u64,
    ) -> Result<TransactionConfirmation> {
        self.wait_for_confirmation(signature, timeout_seconds).await
    }
    
    async fn execute_single_batch_route(
        &self,
        route_request: BatchRouteRequest,
        keypair: &Keypair,
    ) -> Result<ExecutionResult> {
        let options = ExecutionOptions {
            max_slippage: route_request.max_slippage,
            deadline: route_request.deadline,
            max_retries: Some(2),
            dry_run: false,
        };
        
        self.execute_route(&route_request.route_id, route_request.amount, keypair, options).await
    }
    
    async fn update_execution_metrics(&self, result: &ExecutionResult) {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_executions += 1;
        
        if result.success {
            metrics.successful_executions += 1;
        } else {
            metrics.failed_executions += 1;
        }
        
        metrics.avg_execution_time_ms = 
            (metrics.avg_execution_time_ms * 9 + result.execution_time_ms) / 10;
        
        metrics.total_gas_used += result.gas_used;
    }
}

impl Clone for RouteExecutor {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            pool_graph: self.pool_graph.clone(),
            transaction_builder: self.transaction_builder.clone(),
            gas_cache: self.gas_cache.clone(),
            metrics: self.metrics.clone(),
            mev_protection: self.mev_protection.clone(),
        }
    }
}

// Additional types for execution
#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    pub max_slippage: Decimal,
    pub deadline: Option<u64>,
    pub max_retries: Option<u32>,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub route_id: String,
    pub transaction_signature: Option<Signature>,
    pub actual_output: Decimal,
    pub gas_used: u64,
    pub execution_time_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
    pub steps_completed: u8,
    pub mev_protection_used: bool,
}

#[derive(Debug)]
struct RouteInstruction {
    instruction_type: InstructionType,
    pool_address: Pubkey,
    amount_in: Decimal,
    minimum_amount_out: Decimal,
    deadline: u64,
}

#[derive(Debug)]
enum InstructionType {
    Swap,
    AddLiquidity,
    RemoveLiquidity,
}

#[derive(Debug)]
struct TransactionConfirmation {
    signature: Signature,
    actual_output: Decimal,
    gas_used: u64,
    confirmation_time_ms: u64,
    steps_completed: u8,
    slippage: Decimal,
    success: bool,
    execution_cost: Decimal,
}

#[derive(Debug)]
struct ArbitrageExecutionResult {
    opportunity_id: String,
    transaction_signature: Signature,
    actual_profit_usd: Decimal,
    execution_cost_usd: Decimal,
    net_profit_usd: Decimal,
    execution_time_ms: u64,
    success: bool,
    slippage_experienced: Decimal,
}

#[derive(Debug)]
struct ProfitabilityCheck {
    profit_usd: Decimal,
    expected_profit: Decimal,
    gas_cost: Decimal,
    still_profitable: bool,
}

#[derive(Debug, Clone)]
pub struct BatchRouteRequest {
    pub route_id: String,
    pub amount: Decimal,
    pub max_slippage: Decimal,
    pub deadline: Option<u64>,
}

#[derive(Debug)]
pub struct BatchExecutionOptions {
    pub parallel_execution: bool,
    pub max_concurrent: usize,
    pub fail_fast: bool,
}

#[derive(Debug)]
pub struct BatchExecutionResult {
    pub total_routes: usize,
    pub successful_routes: usize,
    pub failed_routes: usize,
    pub results: Vec<ExecutionResult>,
    pub total_execution_time_ms: u64,
    pub success_rate: Decimal,
}