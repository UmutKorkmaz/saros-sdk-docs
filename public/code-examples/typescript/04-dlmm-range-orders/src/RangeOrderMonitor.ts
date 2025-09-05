/**
 * RangeOrderMonitor - WebSocket monitoring for range orders
 * 
 * Provides real-time monitoring of range order positions and
 * automatic notifications when orders are filled.
 */

import { Connection, PublicKey } from '@solana/web3.js';
import WebSocket from 'ws';
import { EventEmitter } from 'events';
import { logger } from './utils/logger';

export interface OrderUpdate {
  positionId: string;
  status: 'PENDING' | 'ACTIVE' | 'PARTIALLY_FILLED' | 'FILLED' | 'OUT_OF_RANGE';
  currentPrice: number;
  targetPrice: number;
  fillPercentage?: number;
  executionPrice?: number;
  timestamp: Date;
}

export interface PriceUpdate {
  poolAddress: string;
  price: number;
  activeBinId: number;
  volume24h: number;
  timestamp: Date;
}

export class RangeOrderMonitor extends EventEmitter {
  private connection: Connection;
  private ws: WebSocket | null = null;
  private subscriptions: Map<string, any> = new Map();
  private positions: Map<string, any> = new Map();
  private priceFeeds: Map<string, number> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 5000;

  constructor(connection: Connection) {
    super();
    this.connection = connection;
  }

  /**
   * Start monitoring with WebSocket connection
   */
  async start(): Promise<void> {
    try {
      logger.info('Starting range order monitor...');
      
      // Connect to WebSocket endpoint
      await this.connectWebSocket();
      
      // Set up price monitoring
      await this.setupPriceMonitoring();
      
      logger.info('Range order monitor started successfully');
      
    } catch (error) {
      logger.error('Error starting monitor:', error);
      throw error;
    }
  }

  /**
   * Watch a specific position
   */
  async watchPosition(
    positionId: string,
    callback: (update: OrderUpdate) => void
  ): Promise<void> {
    try {
      logger.info(`Watching position ${positionId}`);
      
      // Validate position ID format
      try {
        new PublicKey(positionId);
      } catch (error) {
        logger.warn(`Invalid position ID format: ${positionId}, skipping monitoring`);
        return;
      }
      
      // Subscribe to position updates
      const subscription = this.connection.onAccountChange(
        new PublicKey(positionId),
        (accountInfo) => {
          this.handlePositionUpdate(positionId, accountInfo, callback);
        },
        'confirmed'
      );
      
      this.subscriptions.set(positionId, subscription);
      
      // Get initial position state
      await this.fetchPositionState(positionId);
      
    } catch (error) {
      logger.error(`Error watching position ${positionId}:`, error);
      throw error;
    }
  }

  /**
   * Watch for stop loss triggers
   */
  async watchStopLoss(
    stopLossId: string,
    callback: (triggered: boolean) => void
  ): Promise<void> {
    try {
      logger.info(`Watching stop loss ${stopLossId}`);
      
      // Monitor price for trigger condition
      this.on('priceUpdate', (update: PriceUpdate) => {
        const stopLoss = this.positions.get(stopLossId);
        if (stopLoss && update.price <= stopLoss.triggerPrice) {
          logger.warn(`Stop loss triggered at $${update.price}`);
          callback(true);
        }
      });
      
    } catch (error) {
      logger.error(`Error watching stop loss ${stopLossId}:`, error);
      throw error;
    }
  }

  /**
   * Watch pool price updates
   */
  async watchPool(
    poolAddress: PublicKey,
    callback: (price: number) => void
  ): Promise<void> {
    try {
      logger.info(`Watching pool ${poolAddress.toString()}`);
      
      // Subscribe to pool updates
      const subscription = this.connection.onAccountChange(
        poolAddress,
        (accountInfo) => {
          const price = this.extractPriceFromPoolData(accountInfo.data);
          callback(price);
          this.priceFeeds.set(poolAddress.toString(), price);
          
          this.emit('priceUpdate', {
            poolAddress: poolAddress.toString(),
            price,
            timestamp: new Date()
          });
        },
        'confirmed'
      );
      
      this.subscriptions.set(`pool_${poolAddress.toString()}`, subscription);
      
    } catch (error) {
      logger.error(`Error watching pool ${poolAddress.toString()}:`, error);
      throw error;
    }
  }

  /**
   * Connect to WebSocket for real-time updates
   */
  private async connectWebSocket(): Promise<void> {
    return new Promise((resolve, reject) => {
      const wsUrl = this.connection.rpcEndpoint.replace('https', 'wss').replace('http', 'ws');
      
      this.ws = new WebSocket(wsUrl);
      
      this.ws.on('open', () => {
        logger.info('WebSocket connected');
        this.reconnectAttempts = 0;
        resolve();
      });
      
      this.ws.on('message', (data) => {
        this.handleWebSocketMessage(data);
      });
      
      this.ws.on('error', (error) => {
        logger.error('WebSocket error:', error);
      });
      
      this.ws.on('close', () => {
        logger.warn('WebSocket disconnected');
        this.attemptReconnect();
      });
      
      // Timeout connection attempt
      setTimeout(() => {
        if (this.ws?.readyState !== WebSocket.OPEN) {
          reject(new Error('WebSocket connection timeout'));
        }
      }, 10000);
    });
  }

  /**
   * Handle WebSocket messages
   */
  private handleWebSocketMessage(data: any): void {
    try {
      const message = JSON.parse(data.toString());
      
      if (message.method === 'accountNotification') {
        this.handleAccountNotification(message.params);
      } else if (message.method === 'priceUpdate') {
        this.handlePriceUpdate(message.params);
      }
      
    } catch (error) {
      logger.error('Error handling WebSocket message:', error);
    }
  }

  /**
   * Handle account notification from WebSocket
   */
  private handleAccountNotification(params: any): void {
    const { subscription, result } = params;
    
    // Find corresponding position
    for (const [positionId, sub] of this.subscriptions) {
      if (sub === subscription) {
        this.processPositionData(positionId, result.value.data);
        break;
      }
    }
  }

  /**
   * Handle price update
   */
  private handlePriceUpdate(params: any): void {
    const { pool, price, activeBinId } = params;
    
    this.emit('priceUpdate', {
      poolAddress: pool,
      price,
      activeBinId,
      timestamp: new Date()
    });
    
    // Check positions for fills
    this.checkPositionsForFills(pool, price, activeBinId);
  }

  /**
   * Handle position update
   */
  private handlePositionUpdate(
    positionId: string,
    accountInfo: any,
    callback: (update: OrderUpdate) => void
  ): void {
    try {
      const position = this.positions.get(positionId);
      if (!position) return;
      
      const currentPrice = this.priceFeeds.get(position.poolAddress) || 0;
      const status = this.determineOrderStatus(position, currentPrice);
      
      const update: OrderUpdate = {
        positionId,
        status,
        currentPrice,
        targetPrice: position.targetPrice,
        timestamp: new Date()
      };
      
      // Calculate fill percentage for partial fills
      if (status === 'PARTIALLY_FILLED') {
        update.fillPercentage = this.calculateFillPercentage(position, accountInfo);
      }
      
      // Set execution price for filled orders
      if (status === 'FILLED') {
        update.executionPrice = position.targetPrice;
      }
      
      callback(update);
      this.emit('orderUpdate', update);
      
    } catch (error) {
      logger.error('Error handling position update:', error);
    }
  }

  /**
   * Setup price monitoring for all pools
   */
  private async setupPriceMonitoring(): Promise<void> {
    // This would monitor all relevant pools
    // Implementation depends on your pool discovery mechanism
    logger.info('Price monitoring setup complete');
  }

  /**
   * Fetch initial position state
   */
  private async fetchPositionState(positionId: string): Promise<void> {
    try {
      const accountInfo = await this.connection.getAccountInfo(new PublicKey(positionId));
      if (accountInfo) {
        this.processPositionData(positionId, accountInfo.data);
      }
    } catch (error) {
      logger.error(`Error fetching position state for ${positionId}:`, error);
    }
  }

  /**
   * Process position data from chain
   */
  private processPositionData(positionId: string, data: Buffer): void {
    // Parse position data (simplified for example)
    // In real implementation, you'd decode the actual position account
    const position = {
      id: positionId,
      poolAddress: '', // Extract from data
      binId: 0, // Extract from data
      targetPrice: 0, // Extract from data
      liquidity: 0, // Extract from data
      // ... other fields
    };
    
    this.positions.set(positionId, position);
  }

  /**
   * Extract price from pool data
   */
  private extractPriceFromPoolData(data: Buffer): number {
    // Parse pool data to extract current price
    // This is simplified - actual implementation would decode pool account
    return 50; // Placeholder
  }

  /**
   * Determine order status based on current state
   */
  private determineOrderStatus(position: any, currentPrice: number): OrderUpdate['status'] {
    const priceDiff = Math.abs(currentPrice - position.targetPrice);
    const tolerance = position.targetPrice * 0.001; // 0.1% tolerance
    
    if (position.liquidity === 0) {
      return 'FILLED';
    }
    
    if (priceDiff < tolerance) {
      return 'ACTIVE';
    }
    
    if (position.liquidity < position.initialLiquidity) {
      return 'PARTIALLY_FILLED';
    }
    
    return 'PENDING';
  }

  /**
   * Calculate fill percentage
   */
  private calculateFillPercentage(position: any, accountInfo: any): number {
    const currentLiquidity = position.liquidity;
    const initialLiquidity = position.initialLiquidity;
    
    if (initialLiquidity === 0) return 100;
    
    return ((initialLiquidity - currentLiquidity) / initialLiquidity) * 100;
  }

  /**
   * Check all positions for potential fills
   */
  private checkPositionsForFills(poolAddress: string, price: number, activeBinId: number): void {
    for (const position of this.positions.values()) {
      if (position.poolAddress === poolAddress) {
        // Check if price has crossed position's bin
        if (position.orderType === 'BUY' && activeBinId <= position.binId) {
          this.emit('orderFilled', {
            positionId: position.id,
            executionPrice: price,
            timestamp: new Date()
          });
        } else if (position.orderType === 'SELL' && activeBinId >= position.binId) {
          this.emit('orderFilled', {
            positionId: position.id,
            executionPrice: price,
            timestamp: new Date()
          });
        }
      }
    }
  }

  /**
   * Attempt to reconnect WebSocket
   */
  private async attemptReconnect(): Promise<void> {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      logger.error('Max reconnection attempts reached');
      this.emit('error', new Error('WebSocket reconnection failed'));
      return;
    }
    
    this.reconnectAttempts++;
    logger.info(`Attempting reconnection ${this.reconnectAttempts}/${this.maxReconnectAttempts}...`);
    
    await new Promise(resolve => setTimeout(resolve, this.reconnectDelay));
    
    try {
      await this.connectWebSocket();
      logger.info('Reconnection successful');
      
      // Resubscribe to all positions
      for (const [positionId] of this.subscriptions) {
        await this.fetchPositionState(positionId);
      }
    } catch (error) {
      logger.error('Reconnection failed:', error);
      this.attemptReconnect();
    }
  }

  /**
   * Stop monitoring
   */
  async stop(): Promise<void> {
    logger.info('Stopping range order monitor...');
    
    // Remove all subscriptions
    for (const subscription of this.subscriptions.values()) {
      await this.connection.removeAccountChangeListener(subscription);
    }
    this.subscriptions.clear();
    
    // Close WebSocket
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    
    // Clear data
    this.positions.clear();
    this.priceFeeds.clear();
    
    logger.info('Range order monitor stopped');
  }
}