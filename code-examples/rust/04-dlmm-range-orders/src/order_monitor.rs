//! Real-time order monitoring and market data tracking
//! 
//! This module provides real-time monitoring of DLMM pools and order execution
//! conditions, including websocket connections for live updates.

use crate::bin_calculations::BinCalculator;
use crate::types::*;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use saros_dlmm_sdk::{DLMMClient, PoolAccount};
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, sleep, Instant};
use uuid::Uuid;

/// Market data event types
#[derive(Debug, Clone)]
pub enum MarketEvent {
    /// Price update for a bin
    BinPriceUpdate {
        pool_address: Pubkey,
        bin_id: i32,
        price: Decimal,
        liquidity_x: u64,
        liquidity_y: u64,
        timestamp: DateTime<Utc>,
    },
    /// Active bin changed
    ActiveBinChanged {
        pool_address: Pubkey,
        old_bin_id: i32,
        new_bin_id: i32,
        timestamp: DateTime<Utc>,
    },
    /// Large trade detected
    LargeTradeDetected {
        pool_address: Pubkey,
        amount: Decimal,
        price: Decimal,
        direction: TradeDirection,
        timestamp: DateTime<Utc>,
    },
    /// Liquidity change
    LiquidityChanged {
        pool_address: Pubkey,
        bin_id: i32,
        old_liquidity: u128,
        new_liquidity: u128,
        timestamp: DateTime<Utc>,
    },
    /// Order execution opportunity
    ExecutionOpportunity {
        order_id: Uuid,
        expected_price: Decimal,
        available_liquidity: Decimal,
        timestamp: DateTime<Utc>,
    },
}

/// Trade direction enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum TradeDirection {
    Buy,
    Sell,
}

/// Order execution signal
#[derive(Debug, Clone)]
pub struct ExecutionSignal {
    pub order_id: Uuid,
    pub signal_type: SignalType,
    pub urgency: SignalUrgency,
    pub expected_slippage: Decimal,
    pub available_liquidity: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SignalType {
    /// Price reached target
    PriceTarget,
    /// Stop loss triggered
    StopLoss,
    /// Take profit triggered
    TakeProfit,
    /// Optimal execution window
    OptimalWindow,
    /// Liquidity availability
    LiquidityAvailable,
    /// Time-based execution
    TimeTriggered,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum SignalUrgency {
    Low,
    Medium,
    High,
    Critical,
}

/// Price history for technical analysis
#[derive(Debug, Clone)]
pub struct PriceHistory {
    /// Price data points (timestamp, price)
    pub prices: VecDeque<(DateTime<Utc>, Decimal)>,
    /// Maximum history length
    pub max_length: usize,
    /// Volume data
    pub volume: VecDeque<(DateTime<Utc>, Decimal)>,
}

impl PriceHistory {
    pub fn new(max_length: usize) -> Self {
        Self {
            prices: VecDeque::new(),
            max_length,
            volume: VecDeque::new(),
        }
    }

    /// Add new price point
    pub fn add_price(&mut self, timestamp: DateTime<Utc>, price: Decimal) {
        self.prices.push_back((timestamp, price));
        if self.prices.len() > self.max_length {
            self.prices.pop_front();
        }
    }

    /// Add volume data
    pub fn add_volume(&mut self, timestamp: DateTime<Utc>, volume: Decimal) {
        self.volume.push_back((timestamp, volume));
        if self.volume.len() > self.max_length {
            self.volume.pop_front();
        }
    }

    /// Calculate moving average
    pub fn moving_average(&self, periods: usize) -> Option<Decimal> {
        if self.prices.len() < periods {
            return None;
        }

        let sum: Decimal = self.prices
            .iter()
            .rev()
            .take(periods)
            .map(|(_, price)| *price)
            .sum();

        Some(sum / Decimal::from(periods))
    }

    /// Calculate price change percentage
    pub fn price_change_pct(&self, periods: usize) -> Option<Decimal> {
        if self.prices.len() < periods + 1 {
            return None;
        }

        let current = self.prices.back()?.1;
        let previous = self.prices[self.prices.len() - periods - 1].1;

        Some((current - previous) / previous * Decimal::from(100))
    }

    /// Get latest price
    pub fn latest_price(&self) -> Option<Decimal> {
        self.prices.back().map(|(_, price)| *price)
    }

    /// Calculate volatility (standard deviation)
    pub fn volatility(&self, periods: usize) -> Option<Decimal> {
        if let Some(avg) = self.moving_average(periods) {
            let sum_squared_diff: Decimal = self.prices
                .iter()
                .rev()
                .take(periods)
                .map(|(_, price)| (*price - avg) * (*price - avg))
                .sum();

            let variance = sum_squared_diff / Decimal::from(periods);
            Some(decimal_sqrt(variance).unwrap_or(Decimal::ZERO))
        } else {
            None
        }
    }
}

/// Real-time order monitor
pub struct OrderMonitor {
    /// DLMM client
    client: Arc<DLMMClient>,
    /// Bin calculator
    bin_calculator: Arc<BinCalculator>,
    /// Market snapshots by pool
    market_snapshots: Arc<RwLock<HashMap<Pubkey, MarketSnapshot>>>,
    /// Price history by pool
    price_history: Arc<RwLock<HashMap<Pubkey, PriceHistory>>>,
    /// Active orders to monitor
    monitored_orders: Arc<RwLock<HashMap<Uuid, RangeOrder>>>,
    /// Event sender channel
    event_sender: mpsc::UnboundedSender<MarketEvent>,
    /// Execution signal sender
    signal_sender: mpsc::UnboundedSender<ExecutionSignal>,
    /// Configuration
    config: MonitorConfig,
    /// Stop loss tracking
    stop_loss_tracker: Arc<RwLock<HashMap<Uuid, StopLossState>>>,
}

/// Stop loss state tracking
#[derive(Debug, Clone)]
struct StopLossState {
    pub order_id: Uuid,
    pub current_price: Decimal,
    pub stop_price: Decimal,
    pub trailing_distance: Option<Decimal>,
    pub highest_price: Decimal, // For trailing stops
    pub last_updated: DateTime<Utc>,
}

/// Monitor configuration
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Polling interval for price updates (ms)
    pub polling_interval_ms: u64,
    /// Maximum price history length
    pub max_history_length: usize,
    /// Large trade threshold (in USD)
    pub large_trade_threshold: Decimal,
    /// Minimum liquidity threshold for execution
    pub min_liquidity_threshold: Decimal,
    /// Price change threshold for notifications (%)
    pub price_change_threshold_pct: Decimal,
    /// Enable websocket connections
    pub enable_websocket: bool,
    /// Websocket endpoint
    pub websocket_endpoint: Option<String>,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            polling_interval_ms: 1000,
            max_history_length: 1000,
            large_trade_threshold: Decimal::from(10000),
            min_liquidity_threshold: Decimal::from(100),
            price_change_threshold_pct: Decimal::from_str("0.5").unwrap(),
            enable_websocket: false,
            websocket_endpoint: None,
        }
    }
}

impl OrderMonitor {
    /// Create new order monitor
    pub fn new(
        client: Arc<DLMMClient>,
        bin_calculator: Arc<BinCalculator>,
        config: MonitorConfig,
    ) -> (Self, mpsc::UnboundedReceiver<MarketEvent>, mpsc::UnboundedReceiver<ExecutionSignal>) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let (signal_sender, signal_receiver) = mpsc::unbounded_channel();

        let monitor = Self {
            client,
            bin_calculator,
            market_snapshots: Arc::new(RwLock::new(HashMap::new())),
            price_history: Arc::new(RwLock::new(HashMap::new())),
            monitored_orders: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            signal_sender,
            config,
            stop_loss_tracker: Arc::new(RwLock::new(HashMap::new())),
        };

        (monitor, event_receiver, signal_receiver)
    }

    /// Start monitoring
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting order monitor with polling interval: {}ms", self.config.polling_interval_ms);

        // Start main monitoring loop
        let monitor_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = monitor_clone.monitoring_loop().await {
                error!("Monitoring loop error: {}", e);
            }
        });

        // Start websocket connection if enabled
        if self.config.enable_websocket {
            let monitor_clone = self.clone();
            tokio::spawn(async move {
                if let Err(e) = monitor_clone.websocket_loop().await {
                    error!("Websocket loop error: {}", e);
                }
            });
        }

        // Start stop loss monitoring
        let monitor_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = monitor_clone.stop_loss_monitoring_loop().await {
                error!("Stop loss monitoring error: {}", e);
            }
        });

        Ok(())
    }

    /// Add order to monitoring
    pub async fn add_order(&self, order: RangeOrder) {
        let mut monitored_orders = self.monitored_orders.write().await;
        monitored_orders.insert(order.id, order.clone());
        
        // Initialize stop loss tracking if needed
        if matches!(order.order_type, OrderType::StopLoss) {
            let mut stop_tracker = self.stop_loss_tracker.write().await;
            stop_tracker.insert(order.id, StopLossState {
                order_id: order.id,
                current_price: order.target_price,
                stop_price: order.target_price,
                trailing_distance: None,
                highest_price: order.target_price,
                last_updated: Utc::now(),
            });
        }

        debug!("Added order {} to monitoring", order.id);
    }

    /// Remove order from monitoring
    pub async fn remove_order(&self, order_id: Uuid) {
        let mut monitored_orders = self.monitored_orders.write().await;
        monitored_orders.remove(&order_id);
        
        let mut stop_tracker = self.stop_loss_tracker.write().await;
        stop_tracker.remove(&order_id);
        
        debug!("Removed order {} from monitoring", order_id);
    }

    /// Get current market snapshot
    pub async fn get_market_snapshot(&self, pool_address: &Pubkey) -> Option<MarketSnapshot> {
        let snapshots = self.market_snapshots.read().await;
        snapshots.get(pool_address).cloned()
    }

    /// Get price history
    pub async fn get_price_history(&self, pool_address: &Pubkey) -> Option<PriceHistory> {
        let history = self.price_history.read().await;
        history.get(pool_address).cloned()
    }


    /// Main monitoring loop
    async fn monitoring_loop(&self) -> Result<()> {
        let mut interval = interval(std::time::Duration::from_millis(self.config.polling_interval_ms));

        loop {
            interval.tick().await;

            if let Err(e) = self.update_market_data().await {
                error!("Failed to update market data: {}", e);
                continue;
            }

            if let Err(e) = self.check_execution_conditions().await {
                error!("Failed to check execution conditions: {}", e);
            }

            // Small delay to prevent overwhelming the system
            sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    /// Update market data for all monitored pools
    async fn update_market_data(&self) -> Result<()> {
        let monitored_orders = self.monitored_orders.read().await;
        let unique_pools: std::collections::HashSet<_> = monitored_orders
            .values()
            .map(|order| order.pool_address)
            .collect();

        for pool_address in unique_pools {
            if let Err(e) = self.update_pool_data(&pool_address).await {
                warn!("Failed to update data for pool {}: {}", pool_address, e);
            }
        }

        Ok(())
    }

    /// Update data for a specific pool
    async fn update_pool_data(&self, pool_address: &Pubkey) -> Result<()> {
        // Get pool data from client
        let pool = self.client.get_pool(*pool_address).await?;
        let current_price = self.bin_calculator.get_price_at_bin(pool.active_bin_id)?;

        // Create market snapshot
        let mut bin_prices = BTreeMap::new();
        
        // Get prices for bins around the active bin
        let range = 50; // Get ±50 bins around active bin
        for bin_offset in -range..=range {
            let bin_id = pool.active_bin_id + bin_offset;
            
            if let Ok(price) = self.bin_calculator.get_price_at_bin(bin_id) {
                // In a real implementation, you'd get actual liquidity data
                let bin_info = BinPriceInfo {
                    bin_id,
                    price,
                    liquidity_x: 1000000, // Mock data
                    liquidity_y: 1000000, // Mock data
                    total_liquidity: 2000000,
                    volume_24h: Some(Decimal::from(10000)),
                    updated_at: Utc::now(),
                };
                bin_prices.insert(bin_id, bin_info);
            }
        }

        let snapshot = MarketSnapshot {
            pool_address: *pool_address,
            active_bin_id: pool.active_bin_id,
            current_price,
            bin_prices,
            price_change_24h_pct: Decimal::from_str("2.5").unwrap(), // Mock data
            volume_24h: Decimal::from(100000), // Mock data
            timestamp: Utc::now(),
        };

        // Update snapshot
        {
            let mut snapshots = self.market_snapshots.write().await;
            snapshots.insert(*pool_address, snapshot.clone());
        }

        // Update price history
        {
            let mut history = self.price_history.write().await;
            let price_hist = history
                .entry(*pool_address)
                .or_insert_with(|| PriceHistory::new(self.config.max_history_length));
            
            price_hist.add_price(Utc::now(), current_price);
        }

        // Send market events
        if let Err(e) = self.event_sender.send(MarketEvent::BinPriceUpdate {
            pool_address: *pool_address,
            bin_id: pool.active_bin_id,
            price: current_price,
            liquidity_x: 1000000,
            liquidity_y: 1000000,
            timestamp: Utc::now(),
        }) {
            warn!("Failed to send market event: {}", e);
        }

        Ok(())
    }

    /// Check execution conditions for all monitored orders
    async fn check_execution_conditions(&self) -> Result<()> {
        let monitored_orders = self.monitored_orders.read().await;
        let snapshots = self.market_snapshots.read().await;

        for (order_id, order) in monitored_orders.iter() {
            if let Some(snapshot) = snapshots.get(&order.pool_address) {
                self.evaluate_order_execution(order, snapshot).await?;
            }
        }

        Ok(())
    }

    /// Evaluate if an order should be executed
    async fn evaluate_order_execution(&self, order: &RangeOrder, snapshot: &MarketSnapshot) -> Result<()> {
        let current_bin = snapshot.active_bin_id;
        let current_price = snapshot.current_price;

        match order.order_type {
            OrderType::LimitBuy => {
                // Execute buy order when price reaches or goes below target
                if current_bin <= order.bin_id {
                    self.send_execution_signal(order, SignalType::PriceTarget, SignalUrgency::High, snapshot).await?;
                }
            },
            
            OrderType::LimitSell => {
                // Execute sell order when price reaches or goes above target
                if current_bin >= order.bin_id {
                    self.send_execution_signal(order, SignalType::PriceTarget, SignalUrgency::High, snapshot).await?;
                }
            },
            
            OrderType::TakeProfit => {
                // Take profit when price reaches target (similar to limit sell)
                if current_bin >= order.bin_id {
                    self.send_execution_signal(order, SignalType::TakeProfit, SignalUrgency::Critical, snapshot).await?;
                }
            },
            
            OrderType::StopLoss => {
                // Stop loss logic is handled separately in stop_loss_monitoring_loop
            },
            
            _ => {
                // Other order types might have different execution logic
            }
        }

        // Check for optimal execution windows (high liquidity, low volatility)
        if let Some(bin_info) = snapshot.bin_prices.get(&order.bin_id) {
            let available_liquidity = Decimal::from(bin_info.total_liquidity);
            
            if available_liquidity >= self.config.min_liquidity_threshold {
                // Check if we're within acceptable slippage range
                let price_diff = (current_price - order.target_price).abs();
                let slippage_pct = price_diff / order.target_price * Decimal::from(100);
                
                let max_slippage = Decimal::from(order.max_slippage_bps) / Decimal::from(100);
                
                if slippage_pct <= max_slippage {
                    self.send_execution_signal(order, SignalType::OptimalWindow, SignalUrgency::Medium, snapshot).await?;
                }
            }
        }

        Ok(())
    }

    /// Send execution signal
    async fn send_execution_signal(
        &self,
        order: &RangeOrder,
        signal_type: SignalType,
        urgency: SignalUrgency,
        snapshot: &MarketSnapshot,
    ) -> Result<()> {
        let available_liquidity = if let Some(bin_info) = snapshot.bin_prices.get(&order.bin_id) {
            Decimal::from(bin_info.total_liquidity)
        } else {
            Decimal::ZERO
        };

        let expected_slippage = self.calculate_expected_slippage(order, snapshot)?;

        let signal = ExecutionSignal {
            order_id: order.id,
            signal_type: signal_type.clone(),
            urgency,
            expected_slippage,
            available_liquidity,
            timestamp: Utc::now(),
        };

        if let Err(e) = self.signal_sender.send(signal) {
            warn!("Failed to send execution signal for order {}: {}", order.id, e);
        } else {
            debug!("Sent execution signal for order {}: {:?}", order.id, signal_type);
        }

        Ok(())
    }

    /// Calculate expected slippage
    fn calculate_expected_slippage(&self, order: &RangeOrder, snapshot: &MarketSnapshot) -> Result<Decimal> {
        let current_price = snapshot.current_price;
        let target_price = order.target_price;
        
        let slippage = (current_price - target_price).abs() / target_price;
        Ok(slippage)
    }

    /// Stop loss monitoring loop
    async fn stop_loss_monitoring_loop(&self) -> Result<()> {
        let mut interval = interval(std::time::Duration::from_millis(500)); // More frequent for stop losses

        loop {
            interval.tick().await;

            if let Err(e) = self.update_stop_losses().await {
                error!("Failed to update stop losses: {}", e);
            }
        }
    }

    /// Update stop loss states
    async fn update_stop_losses(&self) -> Result<()> {
        let mut stop_tracker = self.stop_loss_tracker.write().await;
        let snapshots = self.market_snapshots.read().await;
        let monitored_orders = self.monitored_orders.read().await;

        let mut orders_to_execute = Vec::new();

        for (order_id, stop_state) in stop_tracker.iter_mut() {
            if let Some(order) = monitored_orders.get(order_id) {
                if let Some(snapshot) = snapshots.get(&order.pool_address) {
                    let current_price = snapshot.current_price;
                    stop_state.current_price = current_price;
                    stop_state.last_updated = Utc::now();

                    // Update highest price for trailing stops
                    if current_price > stop_state.highest_price {
                        stop_state.highest_price = current_price;
                        
                        // Adjust trailing stop
                        if let Some(trailing_distance) = stop_state.trailing_distance {
                            stop_state.stop_price = current_price - trailing_distance;
                        }
                    }

                    // Check if stop loss should trigger
                    if current_price <= stop_state.stop_price {
                        orders_to_execute.push(*order_id);
                    }
                }
            }
        }

        // Send execution signals for triggered stop losses
        for order_id in orders_to_execute {
            if let Some(order) = monitored_orders.get(&order_id) {
                if let Some(snapshot) = snapshots.get(&order.pool_address) {
                    self.send_execution_signal(order, SignalType::StopLoss, SignalUrgency::Critical, snapshot).await?;
                }
            }
        }

        Ok(())
    }

    /// Websocket monitoring (placeholder implementation)
    async fn websocket_loop(&self) -> Result<()> {
        info!("Websocket monitoring not implemented in mock version");
        // In a real implementation, this would connect to Solana websockets
        // for real-time account and transaction updates
        
        let mut interval = interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            debug!("Websocket heartbeat");
        }
    }

    /// Add trailing stop loss
    pub async fn add_trailing_stop(&self, order_id: Uuid, trailing_distance_pct: Decimal) -> Result<()> {
        let mut stop_tracker = self.stop_loss_tracker.write().await;
        
        if let Some(stop_state) = stop_tracker.get_mut(&order_id) {
            let trailing_distance = stop_state.current_price * trailing_distance_pct / Decimal::from(100);
            stop_state.trailing_distance = Some(trailing_distance);
            stop_state.stop_price = stop_state.current_price - trailing_distance;
            
            info!("Added trailing stop for order {}: {}% ({})", 
                  order_id, trailing_distance_pct, trailing_distance);
        } else {
            return Err(anyhow!("Order not found in stop loss tracker: {}", order_id));
        }

        Ok(())
    }

    /// Get monitoring statistics
    pub async fn get_monitoring_stats(&self) -> MonitoringStats {
        let monitored_orders = self.monitored_orders.read().await;
        let snapshots = self.market_snapshots.read().await;
        let stop_tracker = self.stop_loss_tracker.read().await;

        MonitoringStats {
            monitored_orders_count: monitored_orders.len() as u32,
            monitored_pools_count: snapshots.len() as u32,
            stop_loss_orders_count: stop_tracker.len() as u32,
            last_update: Utc::now(),
        }
    }
}

/// Clone implementation for OrderMonitor
impl Clone for OrderMonitor {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            bin_calculator: Arc::clone(&self.bin_calculator),
            market_snapshots: Arc::clone(&self.market_snapshots),
            price_history: Arc::clone(&self.price_history),
            monitored_orders: Arc::clone(&self.monitored_orders),
            event_sender: self.event_sender.clone(),
            signal_sender: self.signal_sender.clone(),
            config: self.config.clone(),
            stop_loss_tracker: Arc::clone(&self.stop_loss_tracker),
        }
    }
}

/// Monitoring statistics
#[derive(Debug, Clone)]
pub struct MonitoringStats {
    pub monitored_orders_count: u32,
    pub monitored_pools_count: u32,
    pub stop_loss_orders_count: u32,
    pub last_update: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bin_calculations::BinCalculator;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_price_history() {
        let mut history = PriceHistory::new(5);
        
        // Add some price points
        history.add_price(Utc::now(), dec!(100));
        history.add_price(Utc::now(), dec!(101));
        history.add_price(Utc::now(), dec!(102));
        history.add_price(Utc::now(), dec!(103));
        history.add_price(Utc::now(), dec!(104));
        
        assert_eq!(history.prices.len(), 5);
        assert_eq!(history.latest_price().unwrap(), dec!(104));
        
        let ma = history.moving_average(3).unwrap();
        assert_eq!(ma, dec!(103)); // (102 + 103 + 104) / 3
        
        let change = history.price_change_pct(4).unwrap();
        assert_eq!(change, dec!(4)); // (104 - 100) / 100 * 100 = 4%
    }

    #[tokio::test]
    async fn test_monitor_creation() {
        let client = Arc::new(saros_dlmm_sdk::DLMMClient::new("mock://test").unwrap());
        let bin_calculator = Arc::new(BinCalculator::new(20, dec!(100)).unwrap());
        let config = MonitorConfig::default();

        let (monitor, _event_rx, _signal_rx) = OrderMonitor::new(client, bin_calculator, config);
        
        // Test basic functionality
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

        monitor.add_order(test_order).await;
        
        let stats = monitor.get_monitoring_stats().await;
        assert_eq!(stats.monitored_orders_count, 1);
    }
}

/// Calculate square root of a Decimal using Newton's method
fn decimal_sqrt(value: Decimal) -> Option<Decimal> {
    if value.is_zero() {
        return Some(Decimal::ZERO);
    }
    
    if value < Decimal::ZERO {
        return None; // Cannot take square root of negative number
    }
    
    // Convert to f64 for calculation, then back to Decimal
    let f_value = value.to_f64()?;
    let sqrt_f = f_value.sqrt();
    Decimal::from_f64(sqrt_f)
}