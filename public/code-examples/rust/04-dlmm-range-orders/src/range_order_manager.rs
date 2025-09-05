//! Core range order management functionality
//! 
//! This module handles the creation, modification, and tracking of range orders
//! for DLMM trading strategies including limit orders, DCA ladders, and grid trading.

use crate::bin_calculations::BinCalculator;
use crate::types::*;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use log::{debug, error, info, warn};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use saros_dlmm_sdk::{DLMMClient, DLMMResult, PoolAccount, Position, TokenAmount};
use solana_sdk::pubkey::Pubkey;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Core range order management system
pub struct RangeOrderManager {
    /// DLMM client for blockchain interactions
    client: Arc<DLMMClient>,
    /// Bin calculator for price/bin conversions
    bin_calculator: Arc<BinCalculator>,
    /// Order book state
    order_book: Arc<RwLock<OrderBook>>,
    /// Active strategies
    strategies: Arc<RwLock<IndexMap<Uuid, StrategyStatus>>>,
    /// Configuration
    config: RangeOrderConfig,
    /// Performance metrics
    metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Risk management
    risk_manager: RiskManager,
}

/// Risk management for order execution
#[derive(Debug, Clone)]
pub struct RiskManager {
    /// Maximum position size per order
    pub max_position_size: Decimal,
    /// Maximum total exposure
    pub max_total_exposure: Decimal,
    /// Maximum active orders
    pub max_active_orders: u32,
    /// Current total exposure
    pub current_exposure: Arc<RwLock<Decimal>>,
    /// Active order count
    pub active_order_count: Arc<RwLock<u32>>,
}

impl RiskManager {
    pub fn new(risk_params: RiskParameters) -> Self {
        Self {
            max_position_size: risk_params.max_position_size,
            max_total_exposure: risk_params.max_total_exposure,
            max_active_orders: risk_params.max_active_orders,
            current_exposure: Arc::new(RwLock::new(Decimal::ZERO)),
            active_order_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Check if a new order passes risk checks
    pub async fn validate_order(&self, order: &RangeOrder) -> Result<()> {
        // Check position size
        if order.amount > self.max_position_size {
            return Err(anyhow!(
                "Order amount {} exceeds maximum position size {}",
                order.amount,
                self.max_position_size
            ));
        }

        // Check total exposure
        let current_exposure = *self.current_exposure.read().await;
        let notional_value = order.amount * order.target_price;
        
        if current_exposure + notional_value > self.max_total_exposure {
            return Err(anyhow!(
                "Adding order would exceed maximum exposure. Current: {}, Addition: {}, Max: {}",
                current_exposure,
                notional_value,
                self.max_total_exposure
            ));
        }

        // Check order count
        let active_count = *self.active_order_count.read().await;
        if active_count >= self.max_active_orders {
            return Err(anyhow!(
                "Maximum active orders ({}) already reached",
                self.max_active_orders
            ));
        }

        Ok(())
    }

    /// Update exposure tracking when order is created
    pub async fn add_order_exposure(&self, order: &RangeOrder) -> Result<()> {
        let notional_value = order.amount * order.target_price;
        let mut exposure = self.current_exposure.write().await;
        *exposure += notional_value;

        let mut count = self.active_order_count.write().await;
        *count += 1;

        debug!("Added order exposure: {} (total: {})", notional_value, *exposure);
        Ok(())
    }

    /// Remove exposure when order is cancelled or filled
    pub async fn remove_order_exposure(&self, order: &RangeOrder) -> Result<()> {
        let remaining_notional = (order.amount - order.filled_amount) * order.target_price;
        let mut exposure = self.current_exposure.write().await;
        *exposure = (*exposure - remaining_notional).max(Decimal::ZERO);

        let mut count = self.active_order_count.write().await;
        if *count > 0 {
            *count -= 1;
        }

        debug!("Removed order exposure: {} (total: {})", remaining_notional, *exposure);
        Ok(())
    }
}

impl RangeOrderManager {
    /// Create new range order manager
    pub async fn new(
        client: Arc<DLMMClient>,
        bin_calculator: Arc<BinCalculator>,
        config: RangeOrderConfig,
    ) -> Result<Self> {
        let risk_manager = RiskManager::new(config.risk_params.clone());

        Ok(Self {
            client,
            bin_calculator,
            order_book: Arc::new(RwLock::new(OrderBook::default())),
            strategies: Arc::new(RwLock::new(IndexMap::new())),
            config,
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            risk_manager,
        })
    }

    /// Create a single limit order
    pub async fn create_limit_order(
        &self,
        pool_address: Pubkey,
        order_type: OrderType,
        target_price: Decimal,
        amount: Decimal,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<Uuid> {
        let bin_id = self.bin_calculator.get_bin_id_for_price(target_price)?;
        
        // Validate DLMM constraints
        self.validate_dlmm_constraints(&order_type, bin_id, &pool_address).await?;

        let order = RangeOrder {
            id: Uuid::new_v4(),
            pool_address,
            order_type,
            bin_id,
            amount,
            target_price,
            status: OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            filled_amount: Decimal::ZERO,
            avg_fill_price: None,
            position_id: None,
            expires_at,
            max_slippage_bps: self.config.risk_params.max_slippage_bps,
            strategy_id: None,
        };

        // Risk validation
        self.risk_manager.validate_order(&order).await?;

        // Add to order book
        self.add_order_to_book(order.clone()).await?;
        self.risk_manager.add_order_exposure(&order).await?;

        info!("Created limit order: {} at bin {} (price: {})", 
              order.id, bin_id, target_price);

        Ok(order.id)
    }

    /// Create DCA ladder strategy
    pub async fn create_dca_ladder(
        &self,
        pool_address: Pubkey,
        config: DcaLadderConfig,
    ) -> Result<Uuid> {
        let strategy_id = Uuid::new_v4();
        
        // Calculate distribution
        let distribution = self.bin_calculator.calculate_dca_distribution(
            config.total_amount,
            config.start_bin_id,
            config.end_bin_id,
            &config.distribution,
        )?;

        let mut order_ids = Vec::new();

        // Create individual orders
        for (bin_id, amount) in &distribution {
            let target_price = self.bin_calculator.get_price_at_bin(*bin_id)?;
            
            let order = RangeOrder {
                id: Uuid::new_v4(),
                pool_address,
                order_type: OrderType::LimitBuy, // DCA typically uses buy orders
                bin_id: *bin_id,
                amount: *amount,
                target_price,
                status: OrderStatus::Pending,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                filled_amount: Decimal::ZERO,
                avg_fill_price: None,
                position_id: None,
                expires_at: None,
                max_slippage_bps: self.config.risk_params.max_slippage_bps,
                strategy_id: Some(strategy_id),
            };

            // Risk validation for each order
            self.risk_manager.validate_order(&order).await?;
            
            // Add to order book
            self.add_order_to_book(order.clone()).await?;
            self.risk_manager.add_order_exposure(&order).await?;
            
            order_ids.push(order.id);
        }

        // Create strategy status
        let strategy_status = StrategyStatus {
            strategy_id,
            strategy_type: TradingStrategy::DcaLadder(config),
            status: StrategyExecutionStatus::Active,
            order_ids,
            executed_volume: Decimal::ZERO,
            pnl: Decimal::ZERO,
            success_rate: Decimal::ZERO,
            created_at: Utc::now(),
            last_executed_at: None,
        };

        let mut strategies = self.strategies.write().await;
        strategies.insert(strategy_id, strategy_status);

        info!("Created DCA ladder strategy {} with {} orders", 
              strategy_id, distribution.len());

        Ok(strategy_id)
    }

    /// Create grid trading strategy
    pub async fn create_grid_strategy(
        &self,
        pool_address: Pubkey,
        config: GridConfig,
    ) -> Result<Uuid> {
        let strategy_id = Uuid::new_v4();
        
        // Calculate grid levels
        let (buy_bins, sell_bins) = self.bin_calculator.calculate_grid_levels(
            config.center_price,
            config.grid_spacing_bps,
            config.buy_orders_count,
            config.sell_orders_count,
        )?;

        let mut order_ids = Vec::new();

        // Create buy orders
        for bin_id in &buy_bins {
            let target_price = self.bin_calculator.get_price_at_bin(*bin_id)?;
            
            let order = RangeOrder {
                id: Uuid::new_v4(),
                pool_address,
                order_type: OrderType::LimitBuy,
                bin_id: *bin_id,
                amount: config.order_amount,
                target_price,
                status: OrderStatus::Pending,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                filled_amount: Decimal::ZERO,
                avg_fill_price: None,
                position_id: None,
                expires_at: None,
                max_slippage_bps: self.config.risk_params.max_slippage_bps,
                strategy_id: Some(strategy_id),
            };

            self.risk_manager.validate_order(&order).await?;
            self.add_order_to_book(order.clone()).await?;
            self.risk_manager.add_order_exposure(&order).await?;
            order_ids.push(order.id);
        }

        // Create sell orders
        for bin_id in &sell_bins {
            let target_price = self.bin_calculator.get_price_at_bin(*bin_id)?;
            
            let order = RangeOrder {
                id: Uuid::new_v4(),
                pool_address,
                order_type: OrderType::LimitSell,
                bin_id: *bin_id,
                amount: config.order_amount,
                target_price,
                status: OrderStatus::Pending,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                filled_amount: Decimal::ZERO,
                avg_fill_price: None,
                position_id: None,
                expires_at: None,
                max_slippage_bps: self.config.risk_params.max_slippage_bps,
                strategy_id: Some(strategy_id),
            };

            self.risk_manager.validate_order(&order).await?;
            self.add_order_to_book(order.clone()).await?;
            self.risk_manager.add_order_exposure(&order).await?;
            order_ids.push(order.id);
        }

        // Create strategy status
        let strategy_status = StrategyStatus {
            strategy_id,
            strategy_type: TradingStrategy::GridTrading(config),
            status: StrategyExecutionStatus::Active,
            order_ids,
            executed_volume: Decimal::ZERO,
            pnl: Decimal::ZERO,
            success_rate: Decimal::ZERO,
            created_at: Utc::now(),
            last_executed_at: None,
        };

        let mut strategies = self.strategies.write().await;
        strategies.insert(strategy_id, strategy_status);

        info!("Created grid strategy {} with {} buy orders and {} sell orders", 
              strategy_id, buy_bins.len(), sell_bins.len());

        Ok(strategy_id)
    }

    /// Create take profit / stop loss orders for existing position
    pub async fn create_tp_sl_orders(
        &self,
        position_id: Pubkey,
        config: TpSlConfig,
    ) -> Result<Vec<Uuid>> {
        // Get position details from SDK
        let position = self.client.get_position(position_id).await?;
        let pool = self.client.get_pool(position.pool_address).await?;
        
        let mut order_ids = Vec::new();

        // Create take profit order if specified
        if let Some(tp_price) = config.take_profit_price {
            let tp_bin_id = self.bin_calculator.get_bin_id_for_price(tp_price)?;
            let tp_amount = position.amount * config.close_percentage / Decimal::from(100);
            
            let tp_order = RangeOrder {
                id: Uuid::new_v4(),
                pool_address: position.pool_address,
                order_type: OrderType::TakeProfit,
                bin_id: tp_bin_id,
                amount: tp_amount,
                target_price: tp_price,
                status: OrderStatus::Pending,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                filled_amount: Decimal::ZERO,
                avg_fill_price: None,
                position_id: Some(position_id),
                expires_at: None,
                max_slippage_bps: self.config.risk_params.max_slippage_bps,
                strategy_id: None,
            };

            self.add_order_to_book(tp_order.clone()).await?;
            order_ids.push(tp_order.id);
        }

        // Create stop loss order if specified
        if let Some(sl_price) = config.stop_loss_price {
            let sl_bin_id = self.bin_calculator.get_bin_id_for_price(sl_price)?;
            let sl_amount = position.amount * config.close_percentage / Decimal::from(100);
            
            let sl_order = RangeOrder {
                id: Uuid::new_v4(),
                pool_address: position.pool_address,
                order_type: OrderType::StopLoss,
                bin_id: sl_bin_id,
                amount: sl_amount,
                target_price: sl_price,
                status: OrderStatus::Pending,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                filled_amount: Decimal::ZERO,
                avg_fill_price: None,
                position_id: Some(position_id),
                expires_at: None,
                max_slippage_bps: self.config.risk_params.max_slippage_bps,
                strategy_id: None,
            };

            self.add_order_to_book(sl_order.clone()).await?;
            order_ids.push(sl_order.id);
        }

        info!("Created TP/SL orders for position {}: {:?}", position_id, order_ids);
        Ok(order_ids)
    }

    /// Cancel an order
    pub async fn cancel_order(&self, order_id: Uuid) -> Result<()> {
        let mut order_book = self.order_book.write().await;
        
        if let Some(order) = order_book.orders_by_id.get(&order_id).cloned() {
            // Update the order status
            if let Some(order_mut) = order_book.orders_by_id.get_mut(&order_id) {
                order_mut.status = OrderStatus::Cancelled;
                order_mut.updated_at = Utc::now();
            }
            
            // Remove from exposure tracking
            self.risk_manager.remove_order_exposure(&order).await?;
            
            // Remove from bin-organized order lists  
            self.remove_order_from_bins_by_id(&mut order_book, &order_id).await?;
            
            info!("Cancelled order: {}", order_id);
        } else {
            return Err(anyhow!("Order not found: {}", order_id));
        }

        Ok(())
    }

    /// Cancel all orders in a strategy
    pub async fn cancel_strategy(&self, strategy_id: Uuid) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        
        if let Some(strategy) = strategies.get_mut(&strategy_id) {
            strategy.status = StrategyExecutionStatus::Cancelled;
            
            // Cancel all associated orders
            for &order_id in &strategy.order_ids {
                if let Err(e) = self.cancel_order(order_id).await {
                    warn!("Failed to cancel order {} in strategy {}: {}", 
                          order_id, strategy_id, e);
                }
            }
            
            info!("Cancelled strategy: {}", strategy_id);
        } else {
            return Err(anyhow!("Strategy not found: {}", strategy_id));
        }

        Ok(())
    }

    /// Get order by ID
    pub async fn get_order(&self, order_id: Uuid) -> Result<RangeOrder> {
        let order_book = self.order_book.read().await;
        order_book.orders_by_id
            .get(&order_id)
            .cloned()
            .ok_or_else(|| anyhow!("Order not found: {}", order_id))
    }

    /// Get all active orders
    pub async fn get_active_orders(&self) -> Result<Vec<RangeOrder>> {
        let order_book = self.order_book.read().await;
        let active_orders = order_book.orders_by_id
            .values()
            .filter(|order| matches!(order.status, OrderStatus::Pending | OrderStatus::PartiallyFilled))
            .cloned()
            .collect();
        
        Ok(active_orders)
    }

    /// Get orders by strategy
    pub async fn get_strategy_orders(&self, strategy_id: Uuid) -> Result<Vec<RangeOrder>> {
        let order_book = self.order_book.read().await;
        
        if let Some(order_ids) = order_book.orders_by_strategy.get(&strategy_id) {
            let orders = order_ids
                .iter()
                .filter_map(|id| order_book.orders_by_id.get(id))
                .cloned()
                .collect();
            Ok(orders)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get strategy status
    pub async fn get_strategy_status(&self, strategy_id: Uuid) -> Result<StrategyStatus> {
        let strategies = self.strategies.read().await;
        strategies.get(&strategy_id)
            .cloned()
            .ok_or_else(|| anyhow!("Strategy not found: {}", strategy_id))
    }

    /// List all strategies
    pub async fn list_strategies(&self) -> Result<Vec<StrategyStatus>> {
        let strategies = self.strategies.read().await;
        Ok(strategies.values().cloned().collect())
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> Result<PerformanceMetrics> {
        let metrics = self.metrics.read().await;
        Ok(metrics.clone())
    }

    /// Mark order as executed
    pub async fn mark_order_executed(
        &self, 
        order_id: Uuid,
        execution: OrderExecution,
    ) -> Result<()> {
        let mut order_book = self.order_book.write().await;
        
        if let Some(order) = order_book.orders_by_id.get_mut(&order_id) {
            order.filled_amount += execution.executed_amount;
            order.updated_at = Utc::now();
            
            // Update average fill price
            if let Some(avg_price) = order.avg_fill_price {
                let total_filled = order.filled_amount;
                let previous_total = total_filled - execution.executed_amount;
                order.avg_fill_price = Some(
                    (avg_price * previous_total + execution.execution_price * execution.executed_amount) 
                    / total_filled
                );
            } else {
                order.avg_fill_price = Some(execution.execution_price);
            }
            
            // Update order status
            let order_clone = order.clone();
            if order.filled_amount >= order.amount {
                order.status = OrderStatus::Filled;
                self.risk_manager.remove_order_exposure(&order_clone).await?;
                self.remove_order_from_bins_by_id(&mut order_book, &order_id).await?;
            } else {
                order.status = OrderStatus::PartiallyFilled;
            }
            
            // Update metrics
            self.update_metrics(&execution).await?;
            
            info!("Order {} executed: {} at price {}", 
                  order_id, execution.executed_amount, execution.execution_price);
        } else {
            return Err(anyhow!("Order not found: {}", order_id));
        }

        Ok(())
    }

    /// Internal helper methods

    async fn add_order_to_book(&self, order: RangeOrder) -> Result<()> {
        let mut order_book = self.order_book.write().await;
        
        // Add to main order map
        order_book.orders_by_id.insert(order.id, order.clone());
        
        // Add to bin-organized lists
        match order.order_type {
            OrderType::LimitBuy | OrderType::DcaLadder => {
                order_book.buy_orders
                    .entry(order.bin_id)
                    .or_insert_with(Vec::new)
                    .push(order.clone());
            },
            OrderType::LimitSell | OrderType::TakeProfit => {
                order_book.sell_orders
                    .entry(order.bin_id)
                    .or_insert_with(Vec::new)
                    .push(order.clone());
            },
            OrderType::StopLoss | OrderType::GridTrading => {
                // These can be both buy or sell depending on context
                // For now, add to appropriate side based on current market price
                // This would need market data in real implementation
                order_book.buy_orders
                    .entry(order.bin_id)
                    .or_insert_with(Vec::new)
                    .push(order.clone());
            },
        }
        
        // Add to strategy mapping if applicable
        if let Some(strategy_id) = order.strategy_id {
            order_book.orders_by_strategy
                .entry(strategy_id)
                .or_insert_with(Vec::new)
                .push(order.id);
        }

        Ok(())
    }

    async fn remove_order_from_bins_by_id(&self, order_book: &mut OrderBook, order_id: &Uuid) -> Result<()> {
        // Find and clone the order first to avoid borrow checker issues
        if let Some(order) = order_book.orders_by_id.get(order_id).cloned() {
            self.remove_order_from_bins_impl(order_book, &order).await
        } else {
            Ok(())
        }
    }

    async fn remove_order_from_bins(&self, order_book: &mut OrderBook, order: &RangeOrder) -> Result<()> {
        self.remove_order_from_bins_impl(order_book, order).await
    }

    async fn remove_order_from_bins_impl(&self, order_book: &mut OrderBook, order: &RangeOrder) -> Result<()> {
        // Remove from buy orders
        if let Some(orders) = order_book.buy_orders.get_mut(&order.bin_id) {
            orders.retain(|o| o.id != order.id);
            if orders.is_empty() {
                order_book.buy_orders.remove(&order.bin_id);
            }
        }
        
        // Remove from sell orders
        if let Some(orders) = order_book.sell_orders.get_mut(&order.bin_id) {
            orders.retain(|o| o.id != order.id);
            if orders.is_empty() {
                order_book.sell_orders.remove(&order.bin_id);
            }
        }

        Ok(())
    }

    async fn validate_dlmm_constraints(
        &self,
        order_type: &OrderType,
        bin_id: i32,
        pool_address: &Pubkey,
    ) -> Result<()> {
        // Get current active bin from pool
        let pool = self.client.get_pool(*pool_address).await?;
        let current_bin = pool.active_bin_id;

        // DLMM constraint: can't place sell orders below current price
        match order_type {
            OrderType::LimitSell | OrderType::TakeProfit => {
                if bin_id <= current_bin {
                    return Err(anyhow!(
                        "Cannot place sell order at bin {} below or at current active bin {}",
                        bin_id, current_bin
                    ));
                }
            },
            OrderType::LimitBuy => {
                if bin_id >= current_bin {
                    return Err(anyhow!(
                        "Cannot place buy order at bin {} above or at current active bin {}",
                        bin_id, current_bin
                    ));
                }
            },
            _ => {} // Other order types may have different constraints
        }

        Ok(())
    }

    async fn update_metrics(&self, execution: &OrderExecution) -> Result<()> {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_orders += 1;
        metrics.successful_orders += 1;
        metrics.total_volume += execution.executed_amount;
        
        // Update success rate
        metrics.success_rate = if metrics.total_orders > 0 {
            Decimal::from(metrics.successful_orders) / Decimal::from(metrics.total_orders) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        Ok(())
    }
}

/// Configuration validation helpers
impl RangeOrderManager {
    /// Validate risk parameters
    pub fn validate_risk_parameters(params: &RiskParameters) -> Result<()> {
        if params.max_position_size <= Decimal::ZERO {
            return Err(anyhow!("Maximum position size must be positive"));
        }
        
        if params.max_total_exposure <= Decimal::ZERO {
            return Err(anyhow!("Maximum total exposure must be positive"));
        }
        
        if params.max_active_orders == 0 {
            return Err(anyhow!("Maximum active orders must be greater than 0"));
        }
        
        if params.max_slippage_bps > 10000 {
            return Err(anyhow!("Maximum slippage cannot exceed 100%"));
        }

        Ok(())
    }

    /// Validate DCA ladder configuration
    pub fn validate_dca_config(config: &DcaLadderConfig) -> Result<()> {
        if config.total_amount <= Decimal::ZERO {
            return Err(anyhow!("Total amount must be positive"));
        }
        
        if config.order_count == 0 {
            return Err(anyhow!("Order count must be greater than 0"));
        }
        
        if config.start_bin_id >= config.end_bin_id {
            return Err(anyhow!("Start bin ID must be less than end bin ID"));
        }
        
        if let crate::types::LadderDistribution::Custom(amounts) = &config.distribution {
            if amounts.len() != config.order_count as usize {
                return Err(anyhow!("Custom distribution length must match order count"));
            }
        }

        Ok(())
    }

    /// Validate grid trading configuration
    pub fn validate_grid_config(config: &GridConfig) -> Result<()> {
        if config.center_price <= Decimal::ZERO {
            return Err(anyhow!("Center price must be positive"));
        }
        
        if config.grid_spacing_bps == 0 {
            return Err(anyhow!("Grid spacing must be greater than 0"));
        }
        
        if config.buy_orders_count == 0 && config.sell_orders_count == 0 {
            return Err(anyhow!("Must have at least one buy or sell order"));
        }
        
        if config.order_amount <= Decimal::ZERO {
            return Err(anyhow!("Order amount must be positive"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bin_calculations::BinCalculator;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_test_config() -> RangeOrderConfig {
        RangeOrderConfig {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            wallet_path: "/tmp/test-keypair.json".to_string(),
            default_pool: None,
            risk_params: RiskParameters {
                max_position_size: dec!(1000),
                max_total_exposure: dec!(10000),
                max_active_orders: 50,
                max_slippage_bps: 100,
                global_stop_loss_pct: Some(dec!(5)),
                daily_loss_limit: Some(dec!(500)),
            },
            monitoring_interval_ms: 1000,
            max_retry_attempts: 3,
            enable_notifications: false,
            webhook_url: None,
        }
    }

    #[tokio::test]
    async fn test_risk_manager() {
        let config = create_test_config();
        let risk_manager = RiskManager::new(config.risk_params.clone());
        
        let test_order = RangeOrder {
            id: Uuid::new_v4(),
            pool_address: Pubkey::new_unique(),
            order_type: OrderType::LimitBuy,
            bin_id: 100,
            amount: dec!(100),
            target_price: dec!(50),
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

        // Should pass validation
        assert!(risk_manager.validate_order(&test_order).await.is_ok());
        
        // Add exposure
        risk_manager.add_order_exposure(&test_order).await.unwrap();
        
        // Check exposure tracking
        let exposure = *risk_manager.current_exposure.read().await;
        assert_eq!(exposure, dec!(5000)); // 100 * 50
        
        let count = *risk_manager.active_order_count.read().await;
        assert_eq!(count, 1);
    }

    #[test]
    fn test_config_validation() {
        let valid_config = create_test_config();
        assert!(RangeOrderManager::validate_risk_parameters(&valid_config.risk_params).is_ok());
        
        let mut invalid_config = valid_config.risk_params.clone();
        invalid_config.max_position_size = Decimal::ZERO;
        assert!(RangeOrderManager::validate_risk_parameters(&invalid_config).is_err());
    }
}