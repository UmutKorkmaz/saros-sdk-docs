use anyhow::Result;
use dotenv::dotenv;
use log::info;
use solana_sdk::pubkey::Pubkey;
use std::{env, str::FromStr, time::Duration};
use tokio::signal;

mod auto_compounder;
mod compound_strategy;
mod gas_optimizer;
mod notification_service;
mod position_monitor;
mod reward_harvester;
mod scheduler;
mod statistics;
mod types;

use auto_compounder::AutoCompounder;
use compound_strategy::CompoundStrategy;
use types::{AutoCompoundConfig, CompoundStrategyConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("🚀 Starting Saros Auto-Compound Yield Farming Bot");

    // Load configuration
    let config = load_config()?;
    info!("📝 Configuration loaded successfully");

    // Initialize auto-compounder
    let mut compounder = AutoCompounder::new(config).await?;
    info!("⚡ Auto-compounder initialized");

    // Load strategies from environment
    let strategies = load_strategies_from_env()?;
    info!("📊 Loaded {} compound strategies", strategies.len());

    // Start auto-compounding for each strategy
    for strategy in strategies {
        match compounder.start_strategy(strategy).await {
            Ok(result) => {
                info!("✅ Strategy started: {}", result.pool_address);
                info!("   Strategy type: {}", result.strategy_type);
                info!("   Interval: {}ms", result.interval_ms);
                info!("   Next compound: {}", result.next_compound_time);
            }
            Err(e) => {
                log::error!("❌ Failed to start strategy: {}", e);
            }
        }
    }

    // Setup graceful shutdown
    info!("🎯 Auto-compound bot is running. Press Ctrl+C to stop.");
    
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("📴 Shutdown signal received, stopping all strategies...");
            compounder.stop_all().await?;
            info!("✅ All strategies stopped gracefully");
        }
    }

    Ok(())
}

fn load_config() -> Result<AutoCompoundConfig> {
    let rpc_url = env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());

    let private_key = env::var("WALLET_PRIVATE_KEY")
        .map_err(|_| anyhow::anyhow!("WALLET_PRIVATE_KEY environment variable is required"))?;

    let network = env::var("SOLANA_NETWORK")
        .unwrap_or_else(|_| "devnet".to_string());

    let max_gas_price = env::var("MAX_GAS_PRICE")
        .unwrap_or_else(|_| "0.01".to_string())
        .parse::<f64>()?;

    let enable_notifications = env::var("ENABLE_NOTIFICATIONS")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()?;

    let webhook_url = env::var("WEBHOOK_URL").ok();

    Ok(AutoCompoundConfig {
        rpc_url,
        private_key,
        network,
        max_gas_price,
        enable_notifications,
        webhook_url,
    })
}

fn load_strategies_from_env() -> Result<Vec<CompoundStrategyConfig>> {
    let mut strategies = Vec::new();

    // Load primary strategy
    if let Ok(pool_address_str) = env::var("POOL_ADDRESS") {
        let pool_address = Pubkey::from_str(&pool_address_str)?;
        
        let strategy_type = env::var("STRATEGY_TYPE")
            .unwrap_or_else(|_| "LP".to_string());

        let interval_ms = env::var("COMPOUND_INTERVAL")
            .unwrap_or_else(|_| "3600000".to_string()) // 1 hour default
            .parse::<u64>()?;

        let min_reward_threshold = env::var("MIN_REWARD_THRESHOLD")
            .unwrap_or_else(|_| "1.0".to_string())
            .parse::<f64>()?;

        let reinvest_percentage = env::var("REINVEST_PERCENTAGE")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<u8>()?;

        let max_slippage = env::var("MAX_SLIPPAGE")
            .unwrap_or_else(|_| "1.0".to_string())
            .parse::<f64>()?;

        strategies.push(CompoundStrategyConfig {
            pool_address,
            strategy_type: strategy_type.parse()?,
            interval_ms,
            min_reward_threshold,
            reinvest_percentage,
            max_slippage: Some(max_slippage),
            emergency_withdraw: false,
        });
    }

    // Load additional strategies (POOL_ADDRESS_1, POOL_ADDRESS_2, etc.)
    for i in 1..=10 {
        let pool_key = format!("POOL_ADDRESS_{}", i);
        if let Ok(pool_address_str) = env::var(&pool_key) {
            let pool_address = Pubkey::from_str(&pool_address_str)?;
            
            let strategy_type = env::var(&format!("STRATEGY_TYPE_{}", i))
                .unwrap_or_else(|_| "LP".to_string());

            let interval_ms = env::var(&format!("COMPOUND_INTERVAL_{}", i))
                .unwrap_or_else(|_| "3600000".to_string())
                .parse::<u64>()?;

            let min_reward_threshold = env::var(&format!("MIN_REWARD_THRESHOLD_{}", i))
                .unwrap_or_else(|_| "1.0".to_string())
                .parse::<f64>()?;

            let reinvest_percentage = env::var(&format!("REINVEST_PERCENTAGE_{}", i))
                .unwrap_or_else(|_| "100".to_string())
                .parse::<u8>()?;

            strategies.push(CompoundStrategyConfig {
                pool_address,
                strategy_type: strategy_type.parse()?,
                interval_ms,
                min_reward_threshold,
                reinvest_percentage,
                max_slippage: Some(1.0),
                emergency_withdraw: false,
            });
        }
    }

    if strategies.is_empty() {
        return Err(anyhow::anyhow!("No strategies configured. Set POOL_ADDRESS environment variable."));
    }

    Ok(strategies)
}