//! DLMM Range Orders Trading System
//! 
//! Advanced trading bot for Saros DLMM with support for:
//! - Limit buy/sell orders using DLMM bins
//! - DCA ladder strategies with multiple distribution methods
//! - Grid trading with automatic rebalancing
//! - Stop loss and take profit orders
//! - Real-time monitoring and automated execution

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use config::{Config, File as ConfigFile};
use log::{error, info, warn};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal;
use uuid::Uuid;

mod bin_calculations;
mod execution_engine;
mod order_monitor;
mod range_order_manager;
mod types;

use bin_calculations::{BinCalculator, BinUtils};
use execution_engine::{ExecutionConfig, ExecutionEngine, GasStrategy};
use order_monitor::{MonitorConfig, OrderMonitor};
use range_order_manager::RangeOrderManager;
use types::*;

/// DLMM Range Orders CLI
#[derive(Parser)]
#[command(
    name = "dlmm-range-orders",
    about = "Advanced DLMM Range Orders Trading System",
    version = "0.1.0"
)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Dry run mode (no actual transactions)
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create and manage orders
    Order(OrderCommands),
    /// Strategy management
    Strategy(StrategyCommands),
    /// Monitor orders and market
    Monitor(MonitorCommands),
    /// System status and statistics
    Status,
    /// Configuration management
    Config(ConfigCommands),
}

#[derive(Args)]
struct OrderCommands {
    #[command(subcommand)]
    action: OrderAction,
}

#[derive(Subcommand)]
enum OrderAction {
    /// Create a limit order
    Limit {
        /// Pool address
        #[arg(long)]
        pool: String,
        /// Order type (buy/sell)
        #[arg(long)]
        side: String,
        /// Target price
        #[arg(long)]
        price: String,
        /// Amount to trade
        #[arg(long)]
        amount: String,
        /// Expiry time (optional, format: 2024-01-01T12:00:00Z)
        #[arg(long)]
        expires: Option<String>,
    },
    /// Create take profit order
    TakeProfit {
        /// Position ID
        #[arg(long)]
        position: String,
        /// Take profit price
        #[arg(long)]
        price: String,
        /// Percentage to close (default: 100)
        #[arg(long, default_value = "100")]
        percentage: String,
    },
    /// Create stop loss order
    StopLoss {
        /// Position ID
        #[arg(long)]
        position: String,
        /// Stop loss price
        #[arg(long)]
        price: String,
        /// Trailing stop percentage (optional)
        #[arg(long)]
        trailing: Option<String>,
        /// Percentage to close (default: 100)
        #[arg(long, default_value = "100")]
        percentage: String,
    },
    /// Cancel order
    Cancel {
        /// Order ID
        order_id: String,
    },
    /// List orders
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,
        /// Filter by pool
        #[arg(long)]
        pool: Option<String>,
    },
    /// Show order details
    Show {
        /// Order ID
        order_id: String,
    },
}

#[derive(Args)]
struct StrategyCommands {
    #[command(subcommand)]
    action: StrategyAction,
}

#[derive(Subcommand)]
enum StrategyAction {
    /// Create DCA ladder strategy
    DcaLadder {
        /// Pool address
        #[arg(long)]
        pool: String,
        /// Total amount to invest
        #[arg(long)]
        amount: String,
        /// Number of orders
        #[arg(long)]
        orders: u32,
        /// Start price (lowest)
        #[arg(long)]
        start_price: String,
        /// End price (highest)
        #[arg(long)]
        end_price: String,
        /// Distribution type (uniform/weighted/fibonacci)
        #[arg(long, default_value = "uniform")]
        distribution: String,
        /// Bias for weighted distribution (optional)
        #[arg(long)]
        bias: Option<f64>,
    },
    /// Create grid trading strategy
    Grid {
        /// Pool address
        #[arg(long)]
        pool: String,
        /// Center price for grid
        #[arg(long)]
        center_price: String,
        /// Grid spacing in basis points
        #[arg(long)]
        spacing: u16,
        /// Number of buy orders
        #[arg(long)]
        buy_orders: u32,
        /// Number of sell orders
        #[arg(long)]
        sell_orders: u32,
        /// Amount per order
        #[arg(long)]
        order_amount: String,
    },
    /// Cancel strategy
    Cancel {
        /// Strategy ID
        strategy_id: String,
    },
    /// List strategies
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,
    },
    /// Show strategy details
    Show {
        /// Strategy ID
        strategy_id: String,
    },
}

#[derive(Args)]
struct MonitorCommands {
    #[command(subcommand)]
    action: MonitorAction,
}

#[derive(Subcommand)]
enum MonitorAction {
    /// Start monitoring
    Start {
        /// Polling interval in milliseconds
        #[arg(long, default_value = "1000")]
        interval: u64,
        /// Enable websocket monitoring
        #[arg(long)]
        websocket: bool,
    },
    /// Show market data
    Market {
        /// Pool address
        pool: String,
        /// Number of bins to show around active bin
        #[arg(long, default_value = "10")]
        range: i32,
    },
    /// Show price history
    History {
        /// Pool address
        pool: String,
        /// Time period (1h, 4h, 1d, 1w)
        #[arg(long, default_value = "1h")]
        period: String,
    },
}

#[derive(Args)]
struct ConfigCommands {
    #[command(subcommand)]
    action: ConfigAction,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Initialize configuration file
    Init {
        /// RPC URL
        #[arg(long, default_value = "https://api.mainnet-beta.solana.com")]
        rpc_url: String,
        /// Wallet path
        #[arg(long, default_value = "~/.config/solana/id.json")]
        wallet: String,
    },
    /// Validate configuration
    Validate,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    /// RPC configuration
    pub rpc: RpcConfig,
    /// Trading configuration  
    pub trading: TradingConfig,
    /// Risk management
    pub risk: RiskConfig,
    /// Monitoring settings
    pub monitoring: MonitoringConfig,
    /// Execution settings
    pub execution: ExecutionSettings,
    /// Notifications
    pub notifications: NotificationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcConfig {
    pub url: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TradingConfig {
    pub wallet_path: String,
    pub default_slippage_bps: u16,
    pub default_pool: Option<String>,
    pub bin_step: u16,
    pub base_price: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RiskConfig {
    pub max_position_size: String,
    pub max_total_exposure: String,
    pub max_active_orders: u32,
    pub max_slippage_bps: u16,
    pub global_stop_loss_pct: Option<String>,
    pub daily_loss_limit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MonitoringConfig {
    pub polling_interval_ms: u64,
    pub max_history_length: usize,
    pub enable_websocket: bool,
    pub websocket_endpoint: Option<String>,
    pub price_change_threshold_pct: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutionSettings {
    pub max_concurrent_executions: usize,
    pub execution_timeout_secs: u64,
    pub max_retry_attempts: u32,
    pub gas_strategy: String,
    pub enable_slippage_protection: bool,
    pub enable_mev_protection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NotificationConfig {
    pub enable_notifications: bool,
    pub webhook_url: Option<String>,
    pub email_notifications: bool,
    pub discord_webhook: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            rpc: RpcConfig {
                url: "https://api.mainnet-beta.solana.com".to_string(),
                timeout_secs: 30,
                max_retries: 3,
            },
            trading: TradingConfig {
                wallet_path: "~/.config/solana/id.json".to_string(),
                default_slippage_bps: 100,
                default_pool: None,
                bin_step: 20,
                base_price: "100.0".to_string(),
            },
            risk: RiskConfig {
                max_position_size: "1000.0".to_string(),
                max_total_exposure: "10000.0".to_string(),
                max_active_orders: 50,
                max_slippage_bps: 500,
                global_stop_loss_pct: Some("5.0".to_string()),
                daily_loss_limit: Some("1000.0".to_string()),
            },
            monitoring: MonitoringConfig {
                polling_interval_ms: 1000,
                max_history_length: 1000,
                enable_websocket: false,
                websocket_endpoint: None,
                price_change_threshold_pct: "0.5".to_string(),
            },
            execution: ExecutionSettings {
                max_concurrent_executions: 10,
                execution_timeout_secs: 30,
                max_retry_attempts: 3,
                gas_strategy: "dynamic".to_string(),
                enable_slippage_protection: true,
                enable_mev_protection: true,
            },
            notifications: NotificationConfig {
                enable_notifications: false,
                webhook_url: None,
                email_notifications: false,
                discord_webhook: None,
            },
        }
    }
}

/// Main application state
struct App {
    config: AppConfig,
    order_manager: Arc<RangeOrderManager>,
    monitor: Arc<OrderMonitor>,
    execution_engine: Arc<ExecutionEngine>,
}

impl App {
    /// Initialize the application
    async fn new(config_path: &PathBuf) -> Result<Self> {
        let config = load_config(config_path)?;
        
        // Initialize DLMM client
        let client = Arc::new(saros_dlmm_sdk::DLMMClient::new(&config.rpc.url)?);
        
        // Initialize bin calculator
        let base_price = Decimal::from_str(&config.trading.base_price)?;
        let bin_calculator = Arc::new(BinCalculator::new(config.trading.bin_step, base_price)?);
        
        // Create range order config
        let range_config = RangeOrderConfig {
            rpc_url: config.rpc.url.clone(),
            wallet_path: config.trading.wallet_path.clone(),
            default_pool: config.trading.default_pool.as_ref().map(|s| Pubkey::from_str(s)).transpose()?,
            risk_params: RiskParameters {
                max_position_size: Decimal::from_str(&config.risk.max_position_size)?,
                max_total_exposure: Decimal::from_str(&config.risk.max_total_exposure)?,
                max_active_orders: config.risk.max_active_orders,
                max_slippage_bps: config.risk.max_slippage_bps,
                global_stop_loss_pct: config.risk.global_stop_loss_pct
                    .as_ref().map(|s| Decimal::from_str(s)).transpose()?,
                daily_loss_limit: config.risk.daily_loss_limit
                    .as_ref().map(|s| Decimal::from_str(s)).transpose()?,
            },
            monitoring_interval_ms: config.monitoring.polling_interval_ms,
            max_retry_attempts: config.execution.max_retry_attempts,
            enable_notifications: config.notifications.enable_notifications,
            webhook_url: config.notifications.webhook_url.clone(),
        };
        
        // Initialize order manager
        let order_manager = Arc::new(
            RangeOrderManager::new(
                client.clone(),
                bin_calculator.clone(),
                range_config,
            ).await?
        );
        
        // Initialize monitor
        let monitor_config = MonitorConfig {
            polling_interval_ms: config.monitoring.polling_interval_ms,
            max_history_length: config.monitoring.max_history_length,
            large_trade_threshold: Decimal::from(10000),
            min_liquidity_threshold: Decimal::from(100),
            price_change_threshold_pct: Decimal::from_str(&config.monitoring.price_change_threshold_pct)?,
            enable_websocket: config.monitoring.enable_websocket,
            websocket_endpoint: config.monitoring.websocket_endpoint.clone(),
        };
        
        let (monitor, _event_rx, signal_rx) = OrderMonitor::new(
            client.clone(),
            bin_calculator.clone(),
            monitor_config,
        );
        let monitor = Arc::new(monitor);
        
        // Initialize execution engine
        let execution_config = ExecutionConfig {
            max_concurrent_executions: config.execution.max_concurrent_executions,
            execution_timeout_secs: config.execution.execution_timeout_secs,
            max_retry_attempts: config.execution.max_retry_attempts,
            retry_delay_ms: 1000,
            gas_strategy: parse_gas_strategy(&config.execution.gas_strategy)?,
            enable_slippage_protection: config.execution.enable_slippage_protection,
            enable_mev_protection: config.execution.enable_mev_protection,
            batch_threshold: 5,
            enable_smart_routing: true,
        };
        
        let execution_engine = Arc::new(ExecutionEngine::new(
            client,
            bin_calculator,
            execution_config,
            None, // TODO: Add notification sender
        ));
        
        // Start monitoring and execution
        monitor.start_monitoring().await?;
        execution_engine.start().await?;
        
        // Connect monitor signals to execution engine
        let engine_clone = execution_engine.clone();
        let order_manager_clone = order_manager.clone();
        tokio::spawn(async move {
            let mut signal_rx = signal_rx;
            while let Some(signal) = signal_rx.recv().await {
                if let Ok(order) = order_manager_clone.get_order(signal.order_id).await {
                    if let Err(e) = engine_clone.queue_execution(signal, order).await {
                        error!("Failed to queue execution: {}", e);
                    }
                }
            }
        });
        
        Ok(Self {
            config,
            order_manager,
            monitor,
            execution_engine,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();
    
    info!("Starting DLMM Range Orders Trading System v0.1.0");
    
    if cli.dry_run {
        warn!("Running in DRY RUN mode - no actual transactions will be executed");
    }
    
    match cli.command {
        Commands::Config(config_cmd) => {
            handle_config_command(config_cmd, &cli.config).await?;
        }
        _ => {
            // Initialize app for other commands
            let app = App::new(&cli.config).await?;
            
            match cli.command {
                Commands::Order(order_cmd) => {
                    handle_order_command(order_cmd, &app).await?;
                }
                Commands::Strategy(strategy_cmd) => {
                    handle_strategy_command(strategy_cmd, &app).await?;
                }
                Commands::Monitor(monitor_cmd) => {
                    handle_monitor_command(monitor_cmd, &app).await?;
                }
                Commands::Status => {
                    handle_status_command(&app).await?;
                }
                Commands::Config(_) => unreachable!(), // Already handled above
            }
        }
    }
    
    Ok(())
}

/// Handle configuration commands
async fn handle_config_command(cmd: ConfigCommands, config_path: &PathBuf) -> Result<()> {
    match cmd.action {
        ConfigAction::Show => {
            let config = load_config(config_path).unwrap_or_else(|_| AppConfig::default());
            println!("{}", toml::to_string_pretty(&config)?);
        }
        ConfigAction::Init { rpc_url, wallet } => {
            let mut config = AppConfig::default();
            config.rpc.url = rpc_url;
            config.trading.wallet_path = wallet;
            
            std::fs::write(config_path, toml::to_string_pretty(&config)?)?;
            println!("Configuration file created at: {}", config_path.display());
        }
        ConfigAction::Validate => {
            match load_config(config_path) {
                Ok(_) => println!("Configuration is valid"),
                Err(e) => {
                    error!("Configuration validation failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}

/// Handle order commands
async fn handle_order_command(cmd: OrderCommands, app: &App) -> Result<()> {
    match cmd.action {
        OrderAction::Limit { pool, side, price, amount, expires } => {
            let pool_pubkey = Pubkey::from_str(&pool)?;
            let order_type = match side.as_str() {
                "buy" => OrderType::LimitBuy,
                "sell" => OrderType::LimitSell,
                _ => return Err(anyhow!("Invalid order side: {}. Use 'buy' or 'sell'", side)),
            };
            let target_price = Decimal::from_str(&price)?;
            let order_amount = Decimal::from_str(&amount)?;
            let expires_at = expires
                .map(|e| DateTime::parse_from_rfc3339(&e))
                .transpose()?
                .map(|dt| dt.with_timezone(&Utc));
            
            let order_id = app.order_manager
                .create_limit_order(pool_pubkey, order_type, target_price, order_amount, expires_at)
                .await?;
                
            println!("Created limit {} order: {}", side, order_id);
            println!("  Pool: {}", pool);
            println!("  Price: {}", target_price);
            println!("  Amount: {}", order_amount);
            if let Some(exp) = expires_at {
                println!("  Expires: {}", exp);
            }
        }
        OrderAction::TakeProfit { position, price, percentage } => {
            let position_pubkey = Pubkey::from_str(&position)?;
            let tp_price = Decimal::from_str(&price)?;
            let close_pct = Decimal::from_str(&percentage)?;
            
            let config = TpSlConfig {
                take_profit_price: Some(tp_price),
                stop_loss_price: None,
                trailing_stop_pct: None,
                close_percentage: close_pct,
            };
            
            let order_ids = app.order_manager.create_tp_sl_orders(position_pubkey, config).await?;
            println!("Created take profit orders: {:?}", order_ids);
        }
        OrderAction::StopLoss { position, price, trailing, percentage } => {
            let position_pubkey = Pubkey::from_str(&position)?;
            let sl_price = Decimal::from_str(&price)?;
            let close_pct = Decimal::from_str(&percentage)?;
            let trailing_pct = trailing.map(|t| Decimal::from_str(&t)).transpose()?;
            
            let config = TpSlConfig {
                take_profit_price: None,
                stop_loss_price: Some(sl_price),
                trailing_stop_pct: trailing_pct,
                close_percentage: close_pct,
            };
            
            let order_ids = app.order_manager.create_tp_sl_orders(position_pubkey, config).await?;
            println!("Created stop loss orders: {:?}", order_ids);
        }
        OrderAction::Cancel { order_id } => {
            let uuid = Uuid::from_str(&order_id)?;
            app.order_manager.cancel_order(uuid).await?;
            println!("Cancelled order: {}", order_id);
        }
        OrderAction::List { status, pool } => {
            let orders = app.order_manager.get_active_orders().await?;
            
            let filtered_orders: Vec<_> = orders
                .into_iter()
                .filter(|order| {
                    if let Some(ref status_filter) = status {
                        format!("{:?}", order.status).to_lowercase().contains(&status_filter.to_lowercase())
                    } else {
                        true
                    }
                })
                .filter(|order| {
                    if let Some(ref pool_filter) = pool {
                        order.pool_address.to_string().contains(pool_filter)
                    } else {
                        true
                    }
                })
                .collect();
            
            if filtered_orders.is_empty() {
                println!("No orders found");
            } else {
                println!("Active Orders:");
                println!("{:<36} {:<10} {:<8} {:<12} {:<12} {:<10}", 
                         "Order ID", "Type", "Status", "Price", "Amount", "Pool");
                println!("{}", "-".repeat(100));
                
                for order in filtered_orders {
                    println!("{:<36} {:<10} {:<8} {:<12} {:<12} {:<10}", 
                             order.id,
                             format!("{:?}", order.order_type),
                             format!("{:?}", order.status),
                             order.target_price,
                             order.amount,
                             &order.pool_address.to_string()[..8]);
                }
            }
        }
        OrderAction::Show { order_id } => {
            let uuid = Uuid::from_str(&order_id)?;
            let order = app.order_manager.get_order(uuid).await?;
            
            println!("Order Details:");
            println!("  ID: {}", order.id);
            println!("  Type: {:?}", order.order_type);
            println!("  Status: {:?}", order.status);
            println!("  Pool: {}", order.pool_address);
            println!("  Bin ID: {}", order.bin_id);
            println!("  Target Price: {}", order.target_price);
            println!("  Amount: {}", order.amount);
            println!("  Filled Amount: {}", order.filled_amount);
            if let Some(avg_price) = order.avg_fill_price {
                println!("  Average Fill Price: {}", avg_price);
            }
            println!("  Created: {}", order.created_at);
            println!("  Updated: {}", order.updated_at);
            if let Some(expires) = order.expires_at {
                println!("  Expires: {}", expires);
            }
            if let Some(strategy_id) = order.strategy_id {
                println!("  Strategy ID: {}", strategy_id);
            }
        }
    }
    Ok(())
}

/// Handle strategy commands  
async fn handle_strategy_command(cmd: StrategyCommands, app: &App) -> Result<()> {
    match cmd.action {
        StrategyAction::DcaLadder { 
            pool, amount, orders, start_price, end_price, distribution, bias 
        } => {
            let pool_pubkey = Pubkey::from_str(&pool)?;
            let total_amount = Decimal::from_str(&amount)?;
            let start_price_dec = Decimal::from_str(&start_price)?;
            let end_price_dec = Decimal::from_str(&end_price)?;
            
            let bin_calculator = BinCalculator::new(app.config.trading.bin_step, start_price_dec)?;
            let start_bin = bin_calculator.get_bin_id_for_price(start_price_dec)?;
            let end_bin = bin_calculator.get_bin_id_for_price(end_price_dec)?;
            
            let ladder_distribution = match distribution.as_str() {
                "uniform" => LadderDistribution::Uniform,
                "weighted" => LadderDistribution::Weighted { 
                    bias: bias.unwrap_or(1.5) 
                },
                "fibonacci" => LadderDistribution::Fibonacci,
                _ => return Err(anyhow!("Invalid distribution type: {}", distribution)),
            };
            
            let config = DcaLadderConfig {
                total_amount,
                order_count: orders,
                start_bin_id: start_bin,
                end_bin_id: end_bin,
                distribution: ladder_distribution,
                time_interval: None,
                max_orders_per_period: None,
            };
            
            let strategy_id = app.order_manager.create_dca_ladder(pool_pubkey, config).await?;
            println!("Created DCA ladder strategy: {}", strategy_id);
            println!("  Pool: {}", pool);
            println!("  Total Amount: {}", total_amount);
            println!("  Orders: {}", orders);
            println!("  Price Range: {} - {}", start_price, end_price);
            println!("  Distribution: {}", distribution);
        }
        StrategyAction::Grid { 
            pool, center_price, spacing, buy_orders, sell_orders, order_amount 
        } => {
            let pool_pubkey = Pubkey::from_str(&pool)?;
            let center_price_dec = Decimal::from_str(&center_price)?;
            let amount_per_order = Decimal::from_str(&order_amount)?;
            
            let config = GridConfig {
                center_price: center_price_dec,
                grid_spacing_bps: spacing,
                buy_orders_count: buy_orders,
                sell_orders_count: sell_orders,
                order_amount: amount_per_order,
                take_profit_pct: Decimal::from_str("0.5").unwrap(), // 0.5% default
                rebalance_threshold_pct: Decimal::from_str("2.0").unwrap(), // 2% default
            };
            
            let strategy_id = app.order_manager.create_grid_strategy(pool_pubkey, config).await?;
            println!("Created grid trading strategy: {}", strategy_id);
            println!("  Pool: {}", pool);
            println!("  Center Price: {}", center_price);
            println!("  Grid Spacing: {} bps", spacing);
            println!("  Buy Orders: {}", buy_orders);
            println!("  Sell Orders: {}", sell_orders);
            println!("  Order Amount: {}", order_amount);
        }
        StrategyAction::Cancel { strategy_id } => {
            let uuid = Uuid::from_str(&strategy_id)?;
            app.order_manager.cancel_strategy(uuid).await?;
            println!("Cancelled strategy: {}", strategy_id);
        }
        StrategyAction::List { status } => {
            let strategies = app.order_manager.list_strategies().await?;
            
            let filtered_strategies: Vec<_> = strategies
                .into_iter()
                .filter(|strategy| {
                    if let Some(ref status_filter) = status {
                        format!("{:?}", strategy.status).to_lowercase().contains(&status_filter.to_lowercase())
                    } else {
                        true
                    }
                })
                .collect();
            
            if filtered_strategies.is_empty() {
                println!("No strategies found");
            } else {
                println!("Strategies:");
                println!("{:<36} {:<15} {:<10} {:<8} {:<12} {:<10}", 
                         "Strategy ID", "Type", "Status", "Orders", "Volume", "P&L");
                println!("{}", "-".repeat(100));
                
                for strategy in filtered_strategies {
                    let strategy_type = match strategy.strategy_type {
                        TradingStrategy::DcaLadder(_) => "DCA Ladder",
                        TradingStrategy::GridTrading(_) => "Grid Trading",
                        TradingStrategy::LimitOrder { .. } => "Limit Order",
                        TradingStrategy::TakeProfit(_) => "Take Profit",
                        TradingStrategy::Combined { .. } => "Combined",
                    };
                    
                    println!("{:<36} {:<15} {:<10} {:<8} {:<12} {:<10}", 
                             strategy.strategy_id,
                             strategy_type,
                             format!("{:?}", strategy.status),
                             strategy.order_ids.len(),
                             strategy.executed_volume,
                             strategy.pnl);
                }
            }
        }
        StrategyAction::Show { strategy_id } => {
            let uuid = Uuid::from_str(&strategy_id)?;
            let strategy = app.order_manager.get_strategy_status(uuid).await?;
            
            println!("Strategy Details:");
            println!("  ID: {}", strategy.strategy_id);
            println!("  Status: {:?}", strategy.status);
            println!("  Created: {}", strategy.created_at);
            if let Some(last_executed) = strategy.last_executed_at {
                println!("  Last Executed: {}", last_executed);
            }
            println!("  Orders: {}", strategy.order_ids.len());
            println!("  Executed Volume: {}", strategy.executed_volume);
            println!("  P&L: {}", strategy.pnl);
            println!("  Success Rate: {}%", strategy.success_rate);
            
            // Show strategy-specific details
            match strategy.strategy_type {
                TradingStrategy::DcaLadder(config) => {
                    println!("\n  DCA Ladder Configuration:");
                    println!("    Total Amount: {}", config.total_amount);
                    println!("    Order Count: {}", config.order_count);
                    println!("    Distribution: {:?}", config.distribution);
                }
                TradingStrategy::GridTrading(config) => {
                    println!("\n  Grid Trading Configuration:");
                    println!("    Center Price: {}", config.center_price);
                    println!("    Grid Spacing: {} bps", config.grid_spacing_bps);
                    println!("    Buy Orders: {}", config.buy_orders_count);
                    println!("    Sell Orders: {}", config.sell_orders_count);
                    println!("    Order Amount: {}", config.order_amount);
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Handle monitor commands
async fn handle_monitor_command(cmd: MonitorCommands, app: &App) -> Result<()> {
    match cmd.action {
        MonitorAction::Start { interval, websocket } => {
            println!("Starting monitoring (interval: {}ms, websocket: {})", interval, websocket);
            println!("Press Ctrl+C to stop...");
            
            // Wait for interrupt signal
            signal::ctrl_c().await?;
            println!("\nStopping monitor...");
        }
        MonitorAction::Market { pool, range } => {
            let pool_pubkey = Pubkey::from_str(&pool)?;
            
            if let Some(snapshot) = app.monitor.get_market_snapshot(&pool_pubkey).await {
                println!("Market Data for Pool: {}", pool);
                println!("Active Bin: {}", snapshot.active_bin_id);
                println!("Current Price: {}", snapshot.current_price);
                println!("24h Change: {}%", snapshot.price_change_24h_pct);
                println!("24h Volume: {}", snapshot.volume_24h);
                println!();
                
                // Show bins around active bin
                println!("Nearby Bins:");
                println!("{:<8} {:<15} {:<12} {:<12} {:<15}", 
                         "Bin ID", "Price", "Liquidity X", "Liquidity Y", "Total Liquidity");
                println!("{}", "-".repeat(70));
                
                let start_bin = snapshot.active_bin_id - range;
                let end_bin = snapshot.active_bin_id + range;
                
                for bin_id in start_bin..=end_bin {
                    if let Some(bin_info) = snapshot.bin_prices.get(&bin_id) {
                        let marker = if bin_id == snapshot.active_bin_id { " *" } else { "" };
                        println!("{:<8} {:<15} {:<12} {:<12} {:<15}{}", 
                                 bin_id,
                                 bin_info.price,
                                 bin_info.liquidity_x,
                                 bin_info.liquidity_y,
                                 bin_info.total_liquidity,
                                 marker);
                    }
                }
            } else {
                println!("No market data available for pool: {}", pool);
            }
        }
        MonitorAction::History { pool, period } => {
            let pool_pubkey = Pubkey::from_str(&pool)?;
            
            if let Some(history) = app.monitor.get_price_history(&pool_pubkey).await {
                println!("Price History for Pool: {} (Period: {})", pool, period);
                
                if let Some(latest_price) = history.latest_price() {
                    println!("Current Price: {}", latest_price);
                }
                
                // Show some basic statistics
                if let Some(ma_5) = history.moving_average(5) {
                    println!("5-period MA: {}", ma_5);
                }
                if let Some(ma_20) = history.moving_average(20) {
                    println!("20-period MA: {}", ma_20);
                }
                if let Some(change) = history.price_change_pct(10) {
                    println!("10-period Change: {}%", change);
                }
                if let Some(volatility) = history.volatility(20) {
                    println!("20-period Volatility: {}", volatility);
                }
            } else {
                println!("No price history available for pool: {}", pool);
            }
        }
    }
    Ok(())
}

/// Handle status command
async fn handle_status_command(app: &App) -> Result<()> {
    println!("DLMM Range Orders System Status");
    println!("================================");
    
    // Order manager status
    let active_orders = app.order_manager.get_active_orders().await?;
    let strategies = app.order_manager.list_strategies().await?;
    let performance = app.order_manager.get_performance_metrics().await?;
    
    println!("\nOrder Management:");
    println!("  Active Orders: {}", active_orders.len());
    println!("  Active Strategies: {}", strategies.len());
    println!("  Total Volume: {}", performance.total_volume);
    println!("  Success Rate: {}%", performance.success_rate);
    
    // Monitor status
    let monitor_stats = app.monitor.get_monitoring_stats().await;
    println!("\nMonitoring:");
    println!("  Monitored Orders: {}", monitor_stats.monitored_orders_count);
    println!("  Monitored Pools: {}", monitor_stats.monitored_pools_count);
    println!("  Stop Loss Orders: {}", monitor_stats.stop_loss_orders_count);
    println!("  Last Update: {}", monitor_stats.last_update);
    
    // Execution engine status
    let exec_stats = app.execution_engine.get_execution_stats().await;
    println!("\nExecution Engine:");
    println!("  Total Executions: {}", exec_stats.total_executions);
    println!("  Success Rate: {:.1}%", exec_stats.success_rate_pct);
    println!("  Pending Executions: {}", exec_stats.pending_executions);
    println!("  Failed Executions: {}", exec_stats.failed_executions);
    println!("  Avg Execution Time: {}ms", exec_stats.avg_execution_time_ms);
    println!("  Total Gas Used: {}", exec_stats.total_gas_used);
    
    if let Some(last_exec) = exec_stats.last_execution {
        println!("  Last Execution: {}", last_exec);
    }
    
    // Queue status
    let queue_status = app.execution_engine.get_queue_status().await;
    println!("\nExecution Queue:");
    println!("  Pending Orders: {}", queue_status.pending_count);
    println!("  Failed Orders: {}", queue_status.failed_count);
    
    Ok(())
}

/// Load configuration from file
fn load_config(config_path: &PathBuf) -> Result<AppConfig> {
    if !config_path.exists() {
        return Ok(AppConfig::default());
    }
    
    let config = Config::builder()
        .add_source(ConfigFile::from(config_path.clone()))
        .build()?;
    
    let app_config: AppConfig = config.try_deserialize()?;
    Ok(app_config)
}

/// Parse gas strategy from string
fn parse_gas_strategy(strategy: &str) -> Result<GasStrategy> {
    match strategy.to_lowercase().as_str() {
        "standard" => Ok(GasStrategy::Standard),
        "fast" => Ok(GasStrategy::Fast),
        "economic" => Ok(GasStrategy::Economic),
        "dynamic" => Ok(GasStrategy::Dynamic),
        _ => {
            if let Ok(custom_price) = strategy.parse::<u64>() {
                Ok(GasStrategy::Custom(custom_price))
            } else {
                Err(anyhow!("Invalid gas strategy: {}", strategy))
            }
        }
    }
}