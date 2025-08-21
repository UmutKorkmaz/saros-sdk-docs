/**
 * AutomatedExecutor - Automated range order execution engine
 * 
 * Provides automated strategies for range order management,
 * including DCA, rebalancing, and conditional orders.
 */

import { Connection, Keypair } from '@solana/web3.js';
import * as cron from 'node-cron';
import { RangeOrderManager } from './RangeOrderManager';
import { RangeOrderMonitor } from './RangeOrderMonitor';
import { logger } from './utils/logger';

export interface Strategy {
  name: string;
  interval: string; // Cron expression
  action: () => Promise<void>;
  enabled?: boolean;
  lastRun?: Date;
  nextRun?: Date;
}

export interface ExecutorConfig {
  enableDCA?: boolean;
  enableRebalancing?: boolean;
  enableStopLoss?: boolean;
  maxConcurrentOrders?: number;
  maxDailyTrades?: number;
  minOrderSize?: number;
}

export class AutomatedExecutor {
  private connection: Connection;
  private wallet: Keypair;
  private manager: RangeOrderManager;
  private monitor: RangeOrderMonitor;
  private strategies: Map<string, Strategy> = new Map();
  private tasks: Map<string, any> = new Map();
  private config: ExecutorConfig;
  private dailyTradeCount = 0;
  private isRunning = false;

  constructor(
    connection: Connection,
    wallet: Keypair,
    config: ExecutorConfig = {}
  ) {
    this.connection = connection;
    this.wallet = wallet;
    this.manager = new RangeOrderManager(connection, wallet);
    this.monitor = new RangeOrderMonitor(connection);
    this.config = {
      enableDCA: true,
      enableRebalancing: true,
      enableStopLoss: true,
      maxConcurrentOrders: 10,
      maxDailyTrades: 50,
      minOrderSize: 10,
      ...config
    };
  }

  /**
   * Add a strategy to the executor
   */
  addStrategy(strategy: Strategy): void {
    if (!cron.validate(strategy.interval)) {
      throw new Error(`Invalid cron expression: ${strategy.interval}`);
    }
    
    this.strategies.set(strategy.name, {
      ...strategy,
      enabled: strategy.enabled !== false
    });
    
    logger.info(`Strategy added: ${strategy.name}`);
  }

  /**
   * Start automated execution
   */
  async start(): Promise<void> {
    if (this.isRunning) {
      logger.warn('Automated executor is already running');
      return;
    }
    
    try {
      logger.info('Starting automated executor...');
      
      // Start monitor
      await this.monitor.start();
      
      // Setup default strategies
      await this.setupDefaultStrategies();
      
      // Schedule all strategies
      for (const [name, strategy] of this.strategies) {
        if (strategy.enabled) {
          this.scheduleStrategy(name, strategy);
        }
      }
      
      // Setup order monitoring
      await this.setupOrderMonitoring();
      
      // Reset daily counter at midnight
      cron.schedule('0 0 * * *', () => {
        this.dailyTradeCount = 0;
        logger.info('Daily trade counter reset');
      });
      
      this.isRunning = true;
      logger.info('Automated executor started successfully');
      
    } catch (error) {
      logger.error('Error starting automated executor:', error);
      throw error;
    }
  }

  /**
   * Setup default strategies
   */
  private async setupDefaultStrategies(): Promise<void> {
    // DCA Strategy
    if (this.config.enableDCA) {
      this.addStrategy({
        name: 'DCA_BUY',
        interval: '0 */6 * * *', // Every 6 hours
        action: async () => await this.executeDCA()
      });
    }
    
    // Rebalancing Strategy
    if (this.config.enableRebalancing) {
      this.addStrategy({
        name: 'REBALANCE',
        interval: '0 0 * * *', // Daily at midnight
        action: async () => await this.executeRebalancing()
      });
    }
    
    // Grid Trading Strategy
    this.addStrategy({
      name: 'GRID_TRADING',
      interval: '*/30 * * * *', // Every 30 minutes
      action: async () => await this.executeGridTrading(),
      enabled: false // Disabled by default
    });
    
    // Momentum Strategy
    this.addStrategy({
      name: 'MOMENTUM',
      interval: '*/15 * * * *', // Every 15 minutes
      action: async () => await this.executeMomentumStrategy(),
      enabled: false
    });
  }

  /**
   * Schedule a strategy
   */
  private scheduleStrategy(name: string, strategy: Strategy): void {
    const task = cron.schedule(strategy.interval, async () => {
      try {
        logger.info(`Executing strategy: ${name}`);
        
        // Check daily trade limit
        if (this.dailyTradeCount >= (this.config.maxDailyTrades || 50)) {
          logger.warn('Daily trade limit reached, skipping execution');
          return;
        }
        
        // Execute strategy
        await strategy.action();
        
        // Update metadata
        strategy.lastRun = new Date();
        this.dailyTradeCount++;
        
        logger.info(`Strategy ${name} executed successfully`);
        
      } catch (error) {
        logger.error(`Error executing strategy ${name}:`, error);
      }
    });
    
    this.tasks.set(name, task);
    logger.info(`Strategy scheduled: ${name} (${strategy.interval})`);
  }

  /**
   * Execute DCA strategy
   */
  private async executeDCA(): Promise<void> {
    try {
      logger.info('Executing DCA strategy...');
      
      // Get current market conditions
      const currentPrice = await this.manager.getCurrentPrice(
        // Use your pool address
        new PublicKey('7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm')
      );
      
      // Check if we should buy (simple logic - buy if price dropped)
      const previousPrice = await this.getPreviousPrice();
      const priceChange = (currentPrice - previousPrice) / previousPrice;
      
      if (priceChange < -0.01) { // Price dropped 1%
        logger.info(`Price dropped ${(priceChange * 100).toFixed(2)}%, placing DCA buy order`);
        
        await this.manager.createLimitBuy({
          poolAddress: new PublicKey('7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm'),
          targetPrice: currentPrice * 0.995, // Buy slightly below current
          amountUSDC: 100, // Fixed DCA amount
          metadata: {
            strategy: 'DCA',
            executedAt: Date.now()
          }
        });
      }
      
    } catch (error) {
      logger.error('Error executing DCA strategy:', error);
    }
  }

  /**
   * Execute rebalancing strategy
   */
  private async executeRebalancing(): Promise<void> {
    try {
      logger.info('Executing rebalancing strategy...');
      
      // Get all active orders
      const activeOrders = await this.manager.getActiveOrders();
      
      if (activeOrders.length === 0) {
        logger.info('No active orders to rebalance');
        return;
      }
      
      // Analyze each order for rebalancing need
      for (const order of activeOrders) {
        const currentPrice = await this.manager.getCurrentPrice(order.poolAddress);
        const priceDistance = Math.abs(currentPrice - order.actualPrice) / currentPrice;
        
        // Rebalance if price has moved significantly
        if (priceDistance > 0.05) { // 5% threshold
          logger.info(`Rebalancing order ${order.positionId} (${(priceDistance * 100).toFixed(2)}% away)`);
          
          // Cancel old order
          await this.manager.cancelOrder(order.positionId);
          
          // Create new order at better price
          if (order.orderType === 'BUY') {
            await this.manager.createLimitBuy({
              poolAddress: order.poolAddress,
              targetPrice: currentPrice * 0.98,
              amountUSDC: order.amount.toNumber() / 1e6,
              metadata: {
                ...order.metadata,
                rebalanced: true,
                rebalancedAt: Date.now()
              }
            });
          } else {
            await this.manager.createLimitSell({
              poolAddress: order.poolAddress,
              targetPrice: currentPrice * 1.02,
              amountSOL: order.amount.toNumber() / 1e9,
              metadata: {
                ...order.metadata,
                rebalanced: true,
                rebalancedAt: Date.now()
              }
            });
          }
        }
      }
      
      logger.info('Rebalancing complete');
      
    } catch (error) {
      logger.error('Error executing rebalancing:', error);
    }
  }

  /**
   * Execute grid trading strategy
   */
  private async executeGridTrading(): Promise<void> {
    try {
      logger.info('Executing grid trading strategy...');
      
      const poolAddress = new PublicKey('7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm');
      const currentPrice = await this.manager.getCurrentPrice(poolAddress);
      
      // Define grid parameters
      const gridLevels = 5;
      const gridSpacing = 0.01; // 1% spacing
      const amountPerLevel = 50; // USDC per level
      
      // Check existing grid orders
      const activeOrders = await this.manager.getActiveOrders();
      const gridOrders = activeOrders.filter(o => o.metadata?.strategy === 'GRID');
      
      // Maintain grid by replacing filled orders
      if (gridOrders.length < gridLevels * 2) { // Buy and sell sides
        // Create buy grid below current price
        for (let i = 1; i <= gridLevels; i++) {
          const price = currentPrice * (1 - gridSpacing * i);
          const existingOrder = gridOrders.find(o => 
            Math.abs(o.targetPrice - price) < 0.01
          );
          
          if (!existingOrder) {
            await this.manager.createLimitBuy({
              poolAddress,
              targetPrice: price,
              amountUSDC: amountPerLevel,
              metadata: {
                strategy: 'GRID',
                level: i,
                side: 'BUY'
              }
            });
          }
        }
        
        // Create sell grid above current price
        for (let i = 1; i <= gridLevels; i++) {
          const price = currentPrice * (1 + gridSpacing * i);
          const existingOrder = gridOrders.find(o => 
            Math.abs(o.targetPrice - price) < 0.01
          );
          
          if (!existingOrder) {
            await this.manager.createLimitSell({
              poolAddress,
              targetPrice: price,
              amountSOL: amountPerLevel / currentPrice, // Convert to SOL
              metadata: {
                strategy: 'GRID',
                level: i,
                side: 'SELL'
              }
            });
          }
        }
      }
      
      logger.info('Grid trading maintenance complete');
      
    } catch (error) {
      logger.error('Error executing grid trading:', error);
    }
  }

  /**
   * Execute momentum strategy
   */
  private async executeMomentumStrategy(): Promise<void> {
    try {
      logger.info('Executing momentum strategy...');
      
      const poolAddress = new PublicKey('7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm');
      const currentPrice = await this.manager.getCurrentPrice(poolAddress);
      
      // Calculate momentum (simplified - would use more sophisticated indicators)
      const priceHistory = await this.getPriceHistory(poolAddress, 20);
      const sma20 = priceHistory.reduce((a, b) => a + b, 0) / priceHistory.length;
      const momentum = (currentPrice - sma20) / sma20;
      
      // Strong upward momentum - place sell orders
      if (momentum > 0.02) { // 2% above SMA
        logger.info(`Upward momentum detected (+${(momentum * 100).toFixed(2)}%), placing sell orders`);
        
        await this.manager.createLimitSell({
          poolAddress,
          targetPrice: currentPrice * 1.01,
          amountSOL: 10,
          metadata: {
            strategy: 'MOMENTUM',
            indicator: 'SMA20',
            momentum: momentum
          }
        });
      }
      
      // Strong downward momentum - place buy orders
      if (momentum < -0.02) { // 2% below SMA
        logger.info(`Downward momentum detected (${(momentum * 100).toFixed(2)}%), placing buy orders`);
        
        await this.manager.createLimitBuy({
          poolAddress,
          targetPrice: currentPrice * 0.99,
          amountUSDC: 500,
          metadata: {
            strategy: 'MOMENTUM',
            indicator: 'SMA20',
            momentum: momentum
          }
        });
      }
      
    } catch (error) {
      logger.error('Error executing momentum strategy:', error);
    }
  }

  /**
   * Setup order monitoring
   */
  private async setupOrderMonitoring(): Promise<void> {
    // Monitor for filled orders
    this.monitor.on('orderFilled', async (data) => {
      logger.info(`Order filled: ${data.positionId} at $${data.executionPrice}`);
      
      // Execute post-fill actions
      await this.handleOrderFilled(data);
    });
    
    // Monitor for partial fills
    this.monitor.on('orderUpdate', async (update) => {
      if (update.status === 'PARTIALLY_FILLED') {
        logger.info(`Order partially filled: ${update.positionId} (${update.fillPercentage}%)`);
      }
    });
    
    // Monitor price updates
    this.monitor.on('priceUpdate', async (update) => {
      // Check for stop loss triggers
      if (this.config.enableStopLoss) {
        await this.checkStopLossTriggers(update.price);
      }
    });
  }

  /**
   * Handle filled order
   */
  private async handleOrderFilled(data: any): Promise<void> {
    try {
      // Log trade for analysis
      logger.info(`Trade executed: ${JSON.stringify(data)}`);
      
      // Check if we need to place a counter order (for grid trading)
      const order = await this.getOrderDetails(data.positionId);
      if (order?.metadata?.strategy === 'GRID') {
        // Replace filled grid order
        if (order.orderType === 'BUY') {
          // Place sell order above
          await this.manager.createLimitSell({
            poolAddress: order.poolAddress,
            targetPrice: data.executionPrice * 1.01,
            amountSOL: order.amount.toNumber() / 1e9,
            metadata: {
              strategy: 'GRID',
              level: order.metadata.level,
              side: 'SELL',
              replacedOrder: data.positionId
            }
          });
        } else {
          // Place buy order below
          await this.manager.createLimitBuy({
            poolAddress: order.poolAddress,
            targetPrice: data.executionPrice * 0.99,
            amountUSDC: order.amount.toNumber() / 1e6,
            metadata: {
              strategy: 'GRID',
              level: order.metadata.level,
              side: 'BUY',
              replacedOrder: data.positionId
            }
          });
        }
      }
      
    } catch (error) {
      logger.error('Error handling filled order:', error);
    }
  }

  /**
   * Check stop loss triggers
   */
  private async checkStopLossTriggers(currentPrice: number): Promise<void> {
    // Implementation would check all stop loss orders
    // and trigger them if price conditions are met
  }

  /**
   * Get previous price (for comparison)
   */
  private async getPreviousPrice(): Promise<number> {
    // Implementation would fetch previous price from storage/cache
    return 50; // Placeholder
  }

  /**
   * Get price history
   */
  private async getPriceHistory(poolAddress: PublicKey, periods: number): Promise<number[]> {
    // Implementation would fetch historical prices
    return Array(periods).fill(50); // Placeholder
  }

  /**
   * Get order details
   */
  private async getOrderDetails(positionId: string): Promise<any> {
    // Implementation would fetch order details
    return null; // Placeholder
  }

  /**
   * Enable/disable a strategy
   */
  setStrategyEnabled(name: string, enabled: boolean): void {
    const strategy = this.strategies.get(name);
    if (strategy) {
      strategy.enabled = enabled;
      
      if (enabled && this.isRunning) {
        this.scheduleStrategy(name, strategy);
      } else if (!enabled) {
        const task = this.tasks.get(name);
        if (task) {
          task.stop();
          this.tasks.delete(name);
        }
      }
      
      logger.info(`Strategy ${name} ${enabled ? 'enabled' : 'disabled'}`);
    }
  }

  /**
   * Get executor statistics
   */
  getStatistics(): any {
    return {
      isRunning: this.isRunning,
      strategies: Array.from(this.strategies.values()).map(s => ({
        name: s.name,
        enabled: s.enabled,
        interval: s.interval,
        lastRun: s.lastRun,
        nextRun: s.nextRun
      })),
      dailyTradeCount: this.dailyTradeCount,
      config: this.config
    };
  }

  /**
   * Stop automated execution
   */
  async stop(): Promise<void> {
    logger.info('Stopping automated executor...');
    
    // Stop all scheduled tasks
    for (const task of this.tasks.values()) {
      task.stop();
    }
    this.tasks.clear();
    
    // Stop monitor
    await this.monitor.stop();
    
    this.isRunning = false;
    logger.info('Automated executor stopped');
  }
}