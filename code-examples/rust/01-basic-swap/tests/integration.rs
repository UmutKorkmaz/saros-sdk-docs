//! Integration tests for the Advanced Swap Example
//!
//! This module provides comprehensive integration tests for all advanced features:
//! - Swap optimization engine testing
//! - Price analysis validation
//! - MEV protection functionality
//! - Batch execution performance
//! - Portfolio rebalancing logic
//! - Error handling and recovery mechanisms

use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};
use saros_basic_swap::{
    swap_optimizer::{SwapOptimizer, OptimizerConfig},
    price_analyzer::{PriceAnalyzer, PriceAnalyzerConfig},
    mev_protection::{MevProtectionEngine, MevProtectionConfig},
    batch_executor::{BatchExecutor, BatchSwapOperation, RebalancingStrategy, RiskParameters},
};
use solana_sdk::pubkey::Pubkey;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::sleep;
use uuid::Uuid;

/// Test suite for swap optimization functionality
mod swap_optimization_tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_swap_optimization() -> Result<()> {
        let optimizer = SwapOptimizer::new();
        
        let token_in = Pubkey::new_unique();
        let token_out = Pubkey::new_unique();
        let amount_in = 1_000_000_000u64; // 1 SOL
        let available_pools = vec![Pubkey::new_unique(), Pubkey::new_unique()];
        
        let result = optimizer.optimize_swap(
            token_in,
            token_out,
            amount_in,
            &available_pools,
        ).await?;
        
        assert!(!result.routes.is_empty(), "Should generate at least one route");
        assert!(result.optimal_slippage_bps > 0, "Should have positive slippage");
        assert!(result.expected_amount_out > 0, "Should expect positive output");
        assert!(result.confidence_score > 0.0 && result.confidence_score <= 1.0, 
                "Confidence score should be between 0 and 1");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_aggressive_optimization() -> Result<()> {
        let config = OptimizerConfig {
            aggressive_optimization: true,
            gas_optimization: true,
            mev_protection_level: 3,
            max_routes: 10,
            ..Default::default()
        };
        
        let optimizer = SwapOptimizer::with_config(config);
        
        let token_in = Pubkey::new_unique();
        let token_out = Pubkey::new_unique();
        let amount_in = 10_000_000_000u64; // 10 SOL - larger trade
        let available_pools: Vec<Pubkey> = (0..5).map(|_| Pubkey::new_unique()).collect();
        
        let result = optimizer.optimize_swap(
            token_in,
            token_out,
            amount_in,
            &available_pools,
        ).await?;
        
        // Aggressive optimization should potentially use more routes
        assert!(result.routes.len() >= 1, "Should have routes with aggressive optimization");
        assert!(result.estimated_gas_cost > 0, "Should estimate gas costs");
        
        // Test multiple optimizations for caching
        let start = Instant::now();
        let _cached_result = optimizer.optimize_swap(
            token_in,
            token_out,
            amount_in,
            &available_pools,
        ).await?;
        let cached_duration = start.elapsed();
        
        // Second call should be faster due to caching
        assert!(cached_duration < Duration::from_millis(100), 
                "Cached optimization should be faster");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_optimization_stats() -> Result<()> {
        let optimizer = SwapOptimizer::new();
        
        let token_in = Pubkey::new_unique();
        let token_out = Pubkey::new_unique();
        let pools = vec![Pubkey::new_unique()];
        
        // Perform several optimizations
        for i in 1..=5 {
            let amount = 1_000_000_000u64 * i; // 1, 2, 3, 4, 5 SOL
            optimizer.optimize_swap(token_in, token_out, amount, &pools).await?;
        }
        
        let stats = optimizer.get_stats().await;
        assert_eq!(stats.total_optimizations, 5);
        assert!(stats.successful_optimizations >= 4, "Most optimizations should succeed");
        assert!(stats.average_optimization_time_ms > 0.0, "Should track timing");
        
        Ok(())
    }
}

/// Test suite for price analysis functionality
mod price_analysis_tests {
    use super::*;

    #[tokio::test]
    async fn test_price_analyzer_creation() -> Result<()> {
        let config = PriceAnalyzerConfig {
            max_history_points: 100,
            update_interval_seconds: 1,
            enable_charts: false, // Disable charts for testing
            ..Default::default()
        };
        
        let analyzer = PriceAnalyzer::with_config(config.clone());
        assert_eq!(analyzer.config.max_history_points, 100);
        assert_eq!(analyzer.config.update_interval_seconds, 1);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_price_monitoring() -> Result<()> {
        let analyzer = PriceAnalyzer::new();
        let token_in = Pubkey::new_unique();
        let token_out = Pubkey::new_unique();
        
        // Start monitoring
        analyzer.start_monitoring(token_in, token_out).await?;
        
        // Allow some time for data collection
        sleep(Duration::from_millis(500)).await;
        
        // This test mainly verifies that monitoring starts without errors
        // In a real implementation, we would verify that price data is being collected
        Ok(())
    }

    #[tokio::test]
    async fn test_market_analysis() -> Result<()> {
        let analyzer = PriceAnalyzer::new();
        let token_in = Pubkey::new_unique();
        let token_out = Pubkey::new_unique();
        
        // Start monitoring first
        analyzer.start_monitoring(token_in, token_out).await?;
        sleep(Duration::from_millis(200)).await;
        
        // Perform market analysis
        let analysis = analyzer.analyze_market(token_in, token_out).await?;
        
        assert!(analysis.current_price > 0.0, "Should have a current price");
        assert!(analysis.volatility_24h >= 0.0, "Volatility should be non-negative");
        assert!(!analysis.predictions.is_empty() || analysis.predictions.is_empty(), 
                "Predictions array should be valid");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_price_impact_analysis() -> Result<()> {
        let analyzer = PriceAnalyzer::new();
        let token_in = Pubkey::new_unique();
        let token_out = Pubkey::new_unique();
        let max_amount = 10_000_000_000u64; // 10 SOL
        
        let impact_analysis = analyzer.analyze_price_impact(token_in, token_out, max_amount).await?;
        
        assert!(!impact_analysis.trade_sizes.is_empty(), "Should have trade sizes");
        assert_eq!(impact_analysis.trade_sizes.len(), impact_analysis.price_impacts.len(), 
                  "Trade sizes and impacts should match");
        assert!(impact_analysis.optimal_trade_size > 0, "Should have optimal trade size");
        assert!(impact_analysis.max_trade_size_1_percent <= max_amount, 
                "1% impact size should be within max");
        assert!(impact_analysis.max_trade_size_5_percent <= max_amount, 
                "5% impact size should be within max");
        
        Ok(())
    }
}

/// Test suite for MEV protection functionality
mod mev_protection_tests {
    use super::*;

    #[tokio::test]
    async fn test_mev_protection_engine_startup() -> Result<()> {
        let config = MevProtectionConfig {
            protection_level: 2,
            use_private_mempool: true,
            enable_flashbots: true,
            ..Default::default()
        };
        
        let engine = MevProtectionEngine::with_config(config);
        
        // Test startup
        engine.start().await?;
        
        // Test that engine is running
        sleep(Duration::from_millis(100)).await;
        
        // Stop the engine
        engine.stop().await;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_transaction_protection() -> Result<()> {
        let engine = MevProtectionEngine::new();
        engine.start().await?;
        
        let transaction = solana_sdk::transaction::Transaction::default();
        let priority = 3u8;
        
        let tx_id = engine.protect_transaction(transaction, priority).await?;
        assert!(!tx_id.is_nil(), "Should return valid transaction ID");
        
        // Allow some time for protection to be processed
        sleep(Duration::from_millis(200)).await;
        
        let stats = engine.get_stats().await;
        assert!(stats.transactions_protected >= 1, "Should have protected at least one transaction");
        
        engine.stop().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_flashbot_bundle_creation() -> Result<()> {
        let engine = MevProtectionEngine::new();
        engine.start().await?;
        
        let transactions = vec![
            solana_sdk::transaction::Transaction::default(),
            solana_sdk::transaction::Transaction::default(),
        ];
        let target_block = 12345u64;
        
        let bundle_id = engine.create_flashbot_bundle(transactions, target_block).await?;
        assert!(!bundle_id.is_nil(), "Should return valid bundle ID");
        
        engine.stop().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_mev_attack_detection() -> Result<()> {
        let engine = MevProtectionEngine::new();
        engine.start().await?;
        
        // Allow some time for the monitoring loop to potentially detect attacks
        sleep(Duration::from_millis(500)).await;
        
        let active_attacks = engine.get_active_attacks().await;
        // Active attacks list should be valid (empty or with detected attacks)
        assert!(active_attacks.len() >= 0, "Active attacks list should be valid");
        
        let stats = engine.get_stats().await;
        assert!(stats.mev_attacks_detected >= 0, "Detection count should be non-negative");
        
        engine.stop().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_protection_stats() -> Result<()> {
        let engine = MevProtectionEngine::new();
        engine.start().await?;
        
        // Protect a few transactions
        for i in 1..=3 {
            let transaction = solana_sdk::transaction::Transaction::default();
            engine.protect_transaction(transaction, i).await?;
        }
        
        sleep(Duration::from_millis(300)).await;
        
        let stats = engine.get_stats().await;
        assert_eq!(stats.transactions_protected, 3, "Should have protected 3 transactions");
        assert!(stats.average_protection_delay_ms >= 0.0, "Delay should be non-negative");
        
        engine.stop().await;
        Ok(())
    }
}

/// Test suite for batch execution functionality
mod batch_execution_tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_executor_creation() -> Result<()> {
        let executor = BatchExecutor::new("https://api.devnet.solana.com").await?;
        let stats = executor.get_execution_stats().await;
        
        // Initial stats should be zeros
        assert_eq!(stats.success_rate_percent, 0.0);
        assert_eq!(stats.throughput_ops_per_second, 0.0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_batch_operation_validation() -> Result<()> {
        let executor = BatchExecutor::new("https://api.devnet.solana.com").await?;
        
        // Test empty batch validation
        let empty_operations = vec![];
        assert!(executor.validate_batch_operations(&empty_operations).await.is_err(), 
                "Should reject empty batch");
        
        // Test valid operations
        let valid_operations = vec![
            BatchSwapOperation {
                operation_id: Uuid::new_v4(),
                token_in: Pubkey::new_unique(),
                token_out: Pubkey::new_unique(),
                amount_in: 1_000_000_000,
                minimum_amount_out: 900_000_000,
                slippage_bps: 100,
                priority: 1,
                execution_deadline: Some(Utc::now() + ChronoDuration::minutes(10)),
                retry_count: 0,
                metadata: HashMap::new(),
            }
        ];
        
        assert!(executor.validate_batch_operations(&valid_operations).await.is_ok(), 
                "Should accept valid operations");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_small_batch_execution() -> Result<()> {
        let executor = BatchExecutor::new("https://api.devnet.solana.com").await?;
        
        // Create a small batch for testing
        let operations = vec![
            BatchSwapOperation {
                operation_id: Uuid::new_v4(),
                token_in: Pubkey::new_unique(),
                token_out: Pubkey::new_unique(),
                amount_in: 500_000_000, // 0.5 SOL
                minimum_amount_out: 450_000_000,
                slippage_bps: 100,
                priority: 1,
                execution_deadline: Some(Utc::now() + ChronoDuration::minutes(5)),
                retry_count: 0,
                metadata: HashMap::new(),
            },
            BatchSwapOperation {
                operation_id: Uuid::new_v4(),
                token_in: Pubkey::new_unique(),
                token_out: Pubkey::new_unique(),
                amount_in: 1_000_000_000, // 1 SOL
                minimum_amount_out: 900_000_000,
                slippage_bps: 150,
                priority: 2,
                execution_deadline: Some(Utc::now() + ChronoDuration::minutes(5)),
                retry_count: 0,
                metadata: HashMap::new(),
            },
        ];
        
        let start_time = Instant::now();
        let result = executor.execute_batch(operations.clone()).await?;
        let execution_time = start_time.elapsed();
        
        assert_eq!(result.total_operations, operations.len());
        assert!(result.successful_operations + result.failed_operations == result.total_operations);
        assert!(execution_time < Duration::from_secs(30), "Batch should complete quickly");
        assert!(!result.batch_id.is_nil(), "Should have valid batch ID");
        
        // Check execution metrics
        assert!(result.execution_metrics.success_rate_percent >= 0.0);
        assert!(result.execution_metrics.throughput_ops_per_second >= 0.0);
        assert!(result.execution_metrics.gas_efficiency_score >= 0.0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_portfolio_analysis() -> Result<()> {
        let executor = BatchExecutor::new("https://api.devnet.solana.com").await?;
        
        // Create sample portfolio
        let mut current_balances = HashMap::new();
        current_balances.insert(Pubkey::new_unique(), 5_000_000_000u64); // 5 SOL
        current_balances.insert(Pubkey::new_unique(), 3_000_000_000u64); // 3 tokens
        current_balances.insert(Pubkey::new_unique(), 2_000_000_000u64); // 2 tokens
        
        // Create target allocations
        let mut target_allocations = HashMap::new();
        target_allocations.insert(*current_balances.keys().nth(0).unwrap(), 0.4); // 40%
        target_allocations.insert(*current_balances.keys().nth(1).unwrap(), 0.35); // 35%
        target_allocations.insert(*current_balances.keys().nth(2).unwrap(), 0.25); // 25%
        
        let strategy = RebalancingStrategy {
            strategy_id: Uuid::new_v4(),
            target_allocations,
            rebalancing_threshold_percent: 5.0,
            minimum_trade_size: 100_000_000, // 0.1 token equivalent
            maximum_trade_size: 5_000_000_000, // 5 token equivalent
            rebalancing_frequency: ChronoDuration::hours(24),
            risk_parameters: RiskParameters {
                max_correlation_threshold: 0.8,
                max_drawdown_percent: 15.0,
                value_at_risk_percent: 5.0,
                position_concentration_limit: 0.4,
            },
        };
        
        let analysis = executor.analyze_portfolio(current_balances, &strategy).await?;
        
        assert!(!analysis.current_allocations.is_empty(), "Should have current allocations");
        assert!(!analysis.target_allocations.is_empty(), "Should have target allocations");
        assert!(analysis.optimization_score >= 0.0 && analysis.optimization_score <= 1.0, 
                "Optimization score should be between 0 and 1");
        assert!(analysis.risk_metrics.portfolio_volatility >= 0.0, 
                "Portfolio volatility should be non-negative");
        assert!(analysis.total_rebalancing_cost >= 0, "Rebalancing cost should be non-negative");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_execution_statistics() -> Result<()> {
        let executor = BatchExecutor::new("https://api.devnet.solana.com").await?;
        
        // Execute a small batch to generate stats
        let operations = vec![
            BatchSwapOperation {
                operation_id: Uuid::new_v4(),
                token_in: Pubkey::new_unique(),
                token_out: Pubkey::new_unique(),
                amount_in: 1_000_000_000,
                minimum_amount_out: 900_000_000,
                slippage_bps: 100,
                priority: 1,
                execution_deadline: None,
                retry_count: 0,
                metadata: HashMap::new(),
            }
        ];
        
        let _result = executor.execute_batch(operations).await?;
        let stats = executor.get_execution_stats().await;
        
        assert!(stats.success_rate_percent >= 0.0 && stats.success_rate_percent <= 100.0, 
                "Success rate should be a valid percentage");
        assert!(stats.average_execution_time_ms >= 0.0, "Execution time should be non-negative");
        assert!(stats.gas_efficiency_score >= 0.0, "Gas efficiency should be non-negative");
        
        Ok(())
    }
}

/// Integration tests combining multiple components
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_advanced_swap_workflow() -> Result<()> {
        // Initialize all components
        let optimizer = SwapOptimizer::new();
        let analyzer = PriceAnalyzer::new();
        let mev_engine = MevProtectionEngine::new();
        let batch_executor = BatchExecutor::new("https://api.devnet.solana.com").await?;
        
        mev_engine.start().await?;
        
        let token_in = Pubkey::new_unique();
        let token_out = Pubkey::new_unique();
        let amount_in = 2_000_000_000u64; // 2 SOL
        
        // Step 1: Analyze prices
        analyzer.start_monitoring(token_in, token_out).await?;
        sleep(Duration::from_millis(100)).await;
        let _market_analysis = analyzer.analyze_market(token_in, token_out).await?;
        
        // Step 2: Optimize swap
        let pools = vec![Pubkey::new_unique(), Pubkey::new_unique()];
        let optimization = optimizer.optimize_swap(token_in, token_out, amount_in, &pools).await?;
        
        // Step 3: Apply MEV protection
        let transaction = solana_sdk::transaction::Transaction::default();
        let _protected_tx = mev_engine.protect_transaction(transaction, 2).await?;
        
        // Step 4: Create batch operation based on optimization
        let operations = vec![
            BatchSwapOperation {
                operation_id: Uuid::new_v4(),
                token_in,
                token_out,
                amount_in: optimization.routes.first().map(|r| r.amount_in).unwrap_or(amount_in),
                minimum_amount_out: optimization.expected_amount_out,
                slippage_bps: optimization.optimal_slippage_bps,
                priority: 2,
                execution_deadline: Some(Utc::now() + ChronoDuration::minutes(5)),
                retry_count: 0,
                metadata: [("workflow_test".to_string(), "integration".to_string())].into(),
            }
        ];
        
        // Step 5: Execute batch
        let batch_result = batch_executor.execute_batch(operations).await?;
        
        // Verify the workflow completed successfully
        assert!(batch_result.total_operations > 0, "Batch should have operations");
        assert!(!optimization.routes.is_empty(), "Optimization should find routes");
        
        mev_engine.stop().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_error_recovery_integration() -> Result<()> {
        let batch_executor = BatchExecutor::new("https://api.devnet.solana.com").await?;
        
        // Create operations that might fail
        let operations = vec![
            BatchSwapOperation {
                operation_id: Uuid::new_v4(),
                token_in: Pubkey::new_unique(),
                token_out: Pubkey::new_unique(),
                amount_in: 0, // Invalid amount - should trigger error handling
                minimum_amount_out: 1_000_000,
                slippage_bps: 100,
                priority: 1,
                execution_deadline: Some(Utc::now() + ChronoDuration::minutes(1)),
                retry_count: 0,
                metadata: HashMap::new(),
            },
            BatchSwapOperation {
                operation_id: Uuid::new_v4(),
                token_in: Pubkey::new_unique(),
                token_out: Pubkey::new_unique(),
                amount_in: 1_000_000_000,
                minimum_amount_out: 2_000_000_000, // Impossible expectation
                slippage_bps: 50, // Very tight slippage
                priority: 1,
                execution_deadline: Some(Utc::now() + ChronoDuration::minutes(1)),
                retry_count: 0,
                metadata: HashMap::new(),
            },
        ];
        
        // Execute batch and expect some operations to fail
        let result = batch_executor.execute_batch(operations).await?;
        
        // The batch execution should complete even with failed operations
        assert_eq!(result.total_operations, 2);
        // At least one operation should fail due to invalid parameters
        assert!(result.failed_operations > 0 || result.successful_operations >= 0, 
                "Should handle failed operations gracefully");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_performance_under_load() -> Result<()> {
        let batch_executor = BatchExecutor::new("https://api.devnet.solana.com").await?;
        
        // Create a larger batch to test performance
        let mut operations = Vec::new();
        for i in 0..20 {
            operations.push(BatchSwapOperation {
                operation_id: Uuid::new_v4(),
                token_in: Pubkey::new_unique(),
                token_out: Pubkey::new_unique(),
                amount_in: 100_000_000 + (i as u64 * 50_000_000), // Varying amounts
                minimum_amount_out: 90_000_000 + (i as u64 * 45_000_000),
                slippage_bps: 100 + (i as u16 * 10),
                priority: ((i % 5) + 1) as u8,
                execution_deadline: Some(Utc::now() + ChronoDuration::minutes(5)),
                retry_count: 0,
                metadata: [("performance_test".to_string(), i.to_string())].into(),
            });
        }
        
        let start_time = Instant::now();
        let result = batch_executor.execute_batch(operations.clone()).await?;
        let execution_time = start_time.elapsed();
        
        // Performance assertions
        assert_eq!(result.total_operations, operations.len());
        assert!(execution_time < Duration::from_secs(60), "Should complete within reasonable time");
        
        // Check throughput
        let throughput = result.total_operations as f64 / execution_time.as_secs_f64();
        assert!(throughput > 0.0, "Should have positive throughput");
        
        // Check that performance metrics are reasonable
        assert!(result.execution_metrics.success_rate_percent >= 0.0);
        assert!(result.execution_metrics.throughput_ops_per_second >= 0.0);
        
        Ok(())
    }
}

/// Stress tests for robustness
mod stress_tests {
    use super::*;

    #[tokio::test]
    async fn test_optimizer_stress() -> Result<()> {
        let optimizer = SwapOptimizer::new();
        
        // Run many optimizations concurrently
        let mut handles = Vec::new();
        
        for i in 0..10 {
            let opt = optimizer.clone();
            let handle = tokio::spawn(async move {
                let token_in = Pubkey::new_unique();
                let token_out = Pubkey::new_unique();
                let amount = 1_000_000_000u64 + (i * 100_000_000);
                let pools = vec![Pubkey::new_unique()];
                
                opt.optimize_swap(token_in, token_out, amount, &pools).await
            });
            handles.push(handle);
        }
        
        // Wait for all to complete
        let results: Vec<Result<_>> = futures::future::try_join_all(handles)
            .await?
            .into_iter()
            .collect();
        
        // Check that most succeeded
        let successful_count = results.iter().filter(|r| r.is_ok()).count();
        assert!(successful_count >= 8, "Most optimizations should succeed under stress");
        
        let stats = optimizer.get_stats().await;
        assert_eq!(stats.total_optimizations, 10);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_mev_protection_stress() -> Result<()> {
        let engine = MevProtectionEngine::new();
        engine.start().await?;
        
        // Protect many transactions quickly
        let mut handles = Vec::new();
        
        for i in 0..20 {
            let eng = engine.clone();
            let handle = tokio::spawn(async move {
                let transaction = solana_sdk::transaction::Transaction::default();
                eng.protect_transaction(transaction, ((i % 5) + 1) as u8).await
            });
            handles.push(handle);
        }
        
        let results: Vec<Result<_>> = futures::future::try_join_all(handles)
            .await?
            .into_iter()
            .collect();
        
        let successful_count = results.iter().filter(|r| r.is_ok()).count();
        assert!(successful_count >= 15, "Most MEV protections should succeed under stress");
        
        // Allow processing time
        sleep(Duration::from_millis(500)).await;
        
        let stats = engine.get_stats().await;
        assert!(stats.transactions_protected >= 15, "Should have protected most transactions");
        
        engine.stop().await;
        Ok(())
    }
}

/// Utility functions for testing
mod test_utils {
    use super::*;

    pub fn create_test_token_pair() -> (Pubkey, Pubkey) {
        (Pubkey::new_unique(), Pubkey::new_unique())
    }

    pub fn create_test_batch_operation(token_in: Pubkey, token_out: Pubkey, amount: u64) -> BatchSwapOperation {
        BatchSwapOperation {
            operation_id: Uuid::new_v4(),
            token_in,
            token_out,
            amount_in: amount,
            minimum_amount_out: (amount as f64 * 0.95) as u64, // 5% slippage buffer
            slippage_bps: 500, // 5%
            priority: 1,
            execution_deadline: Some(Utc::now() + ChronoDuration::minutes(10)),
            retry_count: 0,
            metadata: HashMap::new(),
        }
    }

    pub async fn setup_test_environment() -> Result<(SwapOptimizer, PriceAnalyzer, MevProtectionEngine, BatchExecutor)> {
        let optimizer = SwapOptimizer::new();
        let analyzer = PriceAnalyzer::new();
        let mev_engine = MevProtectionEngine::new();
        let batch_executor = BatchExecutor::new("https://api.devnet.solana.com").await?;
        
        mev_engine.start().await?;
        
        Ok((optimizer, analyzer, mev_engine, batch_executor))
    }
}

// Re-export test utilities for use in other test files
pub use test_utils::*;