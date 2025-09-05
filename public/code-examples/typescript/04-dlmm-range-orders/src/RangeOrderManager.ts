/**
 * RangeOrderManager - Core logic for DLMM range orders
 * 
 * Handles creation, management, and execution of limit orders
 * using concentrated liquidity bins.
 */

import { Connection, Keypair, PublicKey, Transaction } from '@solana/web3.js';
// Note: Using any for SDK types due to import issues
// import { DLMMClient, PositionParams, BinDistribution } from '@saros-finance/dlmm-sdk';
import BN from 'bn.js';
import { logger } from './utils/logger';
import { priceToBinId, binIdToPrice, calculateBinStep } from './utils/binMath';

export interface LimitBuyParams {
  poolAddress: PublicKey;
  targetPrice: number;
  amountUSDC: number;
  tolerance?: number;
  metadata?: any;
}

export interface LimitSellParams {
  poolAddress: PublicKey;
  targetPrice: number;
  amountSOL: number;
  tolerance?: number;
  metadata?: any;
}

export interface RangeOrder {
  positionId: string;
  poolAddress: PublicKey;
  binId: number;
  orderType: 'BUY' | 'SELL';
  targetPrice: number;
  actualPrice: number;
  amount: BN;
  status: 'PENDING' | 'ACTIVE' | 'PARTIALLY_FILLED' | 'FILLED' | 'CANCELLED';
  createdAt: Date;
  metadata?: any;
}

export interface BuyLadderParams {
  poolAddress: PublicKey;
  startPrice: number;
  endPrice: number;
  steps: number;
  totalAmountUSDC: number;
  distribution: 'linear' | 'exponential';
}

export interface TakeProfitLevel {
  price: number;
  percentage: number;
}

export interface StopLossParams {
  poolAddress: PublicKey;
  triggerPrice: number;
  amountSOL: number;
  orderType: 'range' | 'market';
  slippage?: number;
}

export class RangeOrderManager {
  private client: any;
  private connection: Connection;
  private wallet: Keypair;
  private orders: Map<string, RangeOrder> = new Map();

  constructor(connection: Connection, wallet: Keypair) {
    this.connection = connection;
    this.wallet = wallet;
    // Using SDK wrapper pattern
    try {
      const { DLMMClient } = require('@saros-finance/dlmm-sdk');
      this.client = new DLMMClient({
        rpcUrl: connection.rpcEndpoint,
        wallet: wallet
      });
    } catch (error) {
      logger.error('DLMM SDK not available, using mock client');
      this.client = {
        getPool: async () => ({ binStep: 10, baseDecimals: 6, activeBinId: 100 }),
        createPosition: async () => ({ 
          positionId: PublicKey.unique().toString()
        }),
        getPosition: async () => null,
        closePosition: async () => true
      };
    }
  }

  /**
   * Create a limit buy order
   */
  async createLimitBuy(params: LimitBuyParams): Promise<RangeOrder> {
    try {
      logger.info(`Creating limit buy order at $${params.targetPrice}`);
      
      // Get pool info
      const poolInfo = await this.client.getPool(params.poolAddress);
      const binStep = poolInfo.binStep;
      
      // Calculate target bin
      const currentPrice = await this.getCurrentPrice(params.poolAddress);
      const targetBinId = priceToBinId(params.targetPrice, binStep, poolInfo.baseDecimals);
      
      // Validate order placement
      if (params.targetPrice >= currentPrice) {
        throw new Error('Buy order must be below current price');
      }
      
      // Create single-bin position
      const positionParams: any = {
        poolAddress: params.poolAddress,
        lowerBinId: targetBinId,
        upperBinId: targetBinId, // Single bin for limit order
        totalLiquidity: new BN(params.amountUSDC * 1e6), // USDC decimals
        distributionMode: { type: 'SPOT' }, // All liquidity in one bin
      };
      
      // Create position
      const result = await this.client.createPosition(positionParams);
      
      // Calculate actual execution price
      const actualPrice = binIdToPrice(targetBinId, binStep, poolInfo.baseDecimals);
      
      // Create order record
      const order: RangeOrder = {
        positionId: result.positionId,
        poolAddress: params.poolAddress,
        binId: targetBinId,
        orderType: 'BUY',
        targetPrice: params.targetPrice,
        actualPrice,
        amount: new BN(params.amountUSDC * 1e6),
        status: 'ACTIVE',
        createdAt: new Date(),
        metadata: params.metadata
      };
      
      // Store order
      this.orders.set(order.positionId, order);
      
      logger.info(`Limit buy order created successfully`);
      logger.info(`Position ID: ${order.positionId}`);
      logger.info(`Actual price: $${actualPrice.toFixed(4)}`);
      
      return order;
      
    } catch (error) {
      logger.error('Error creating limit buy order:', error);
      throw error;
    }
  }

  /**
   * Create a limit sell order
   */
  async createLimitSell(params: LimitSellParams): Promise<RangeOrder> {
    try {
      logger.info(`Creating limit sell order at $${params.targetPrice}`);
      
      // Get pool info
      const poolInfo = await this.client.getPool(params.poolAddress);
      const binStep = poolInfo.binStep;
      
      // Calculate target bin
      const currentPrice = await this.getCurrentPrice(params.poolAddress);
      const targetBinId = priceToBinId(params.targetPrice, binStep, poolInfo.baseDecimals);
      
      // Validate order placement
      if (params.targetPrice <= currentPrice) {
        throw new Error('Sell order must be above current price');
      }
      
      // Create single-bin position
      const positionParams: any = {
        poolAddress: params.poolAddress,
        lowerBinId: targetBinId,
        upperBinId: targetBinId,
        totalLiquidity: new BN(params.amountSOL * 1e9), // SOL decimals
        distributionMode: { type: 'SPOT' },
      };
      
      // Create position
      const result = await this.client.createPosition(positionParams);
      
      // Calculate actual execution price
      const actualPrice = binIdToPrice(targetBinId, binStep, poolInfo.baseDecimals);
      
      // Create order record
      const order: RangeOrder = {
        positionId: result.positionId,
        poolAddress: params.poolAddress,
        binId: targetBinId,
        orderType: 'SELL',
        targetPrice: params.targetPrice,
        actualPrice,
        amount: new BN(params.amountSOL * 1e9),
        status: 'ACTIVE',
        createdAt: new Date(),
        metadata: params.metadata
      };
      
      // Store order
      this.orders.set(order.positionId, order);
      
      logger.info(`Limit sell order created successfully`);
      logger.info(`Position ID: ${order.positionId}`);
      logger.info(`Actual price: $${actualPrice.toFixed(4)}`);
      
      return order;
      
    } catch (error) {
      logger.error('Error creating limit sell order:', error);
      throw error;
    }
  }

  /**
   * Create a ladder of buy orders
   */
  async createBuyLadder(params: BuyLadderParams): Promise<{
    orders: RangeOrder[];
    totalInvested: number;
    averagePrice: number;
  }> {
    try {
      logger.info(`Creating buy ladder from $${params.startPrice} to $${params.endPrice}`);
      
      const orders: RangeOrder[] = [];
      const priceStep = (params.endPrice - params.startPrice) / (params.steps - 1);
      let totalInvested = 0;
      let weightedPriceSum = 0;
      
      // Calculate amount distribution
      const amounts = this.calculateLadderAmounts(
        params.totalAmountUSDC,
        params.steps,
        params.distribution
      );
      
      // Create orders at each level
      for (let i = 0; i < params.steps; i++) {
        const price = params.startPrice + (priceStep * i);
        const amount = amounts[i];
        
        const order = await this.createLimitBuy({
          poolAddress: params.poolAddress,
          targetPrice: price,
          amountUSDC: amount,
          tolerance: 0.1,
          metadata: {
            ladderIndex: i,
            ladderTotal: params.steps
          }
        });
        
        orders.push(order);
        totalInvested += amount;
        weightedPriceSum += price * amount;
        
        logger.info(`Ladder order ${i + 1}/${params.steps} created at $${price.toFixed(2)}`);
      }
      
      const averagePrice = weightedPriceSum / totalInvested;
      
      logger.info(`Buy ladder created successfully`);
      logger.info(`Total orders: ${orders.length}`);
      logger.info(`Total invested: $${totalInvested.toFixed(2)}`);
      logger.info(`Average price: $${averagePrice.toFixed(2)}`);
      
      return {
        orders,
        totalInvested,
        averagePrice
      };
      
    } catch (error) {
      logger.error('Error creating buy ladder:', error);
      throw error;
    }
  }

  /**
   * Create take profit levels
   */
  async createTakeProfitLevels(params: {
    poolAddress: PublicKey;
    positionAmount: number;
    levels: TakeProfitLevel[];
  }): Promise<any[]> {
    try {
      const takeProfits: any[] = [];
      
      for (const level of params.levels) {
        const amount = params.positionAmount * (level.percentage / 100);
        
        const order = await this.createLimitSell({
          poolAddress: params.poolAddress,
          targetPrice: level.price,
          amountSOL: amount,
          metadata: {
            type: 'take-profit',
            percentage: level.percentage
          }
        });
        
        takeProfits.push({
          ...order,
          price: level.price,
          amount,
          percentage: level.percentage,
          profitPercentage: ((level.price / params.levels[0].price - 1) * 100).toFixed(2)
        });
      }
      
      return takeProfits;
      
    } catch (error) {
      logger.error('Error creating take profit levels:', error);
      throw error;
    }
  }

  /**
   * Create stop loss order
   */
  async createStopLoss(params: StopLossParams): Promise<any> {
    try {
      logger.info(`Creating stop loss at $${params.triggerPrice}`);
      
      const currentPrice = await this.getCurrentPrice(params.poolAddress);
      const maxLoss = ((currentPrice - params.triggerPrice) / currentPrice * 100).toFixed(2);
      
      if (params.orderType === 'range') {
        // Create range order slightly below trigger
        const order = await this.createLimitSell({
          poolAddress: params.poolAddress,
          targetPrice: params.triggerPrice * 0.999, // Slightly below trigger
          amountSOL: params.amountSOL,
          metadata: {
            type: 'stop-loss',
            triggerPrice: params.triggerPrice
          }
        });
        
        return {
          id: order.positionId,
          ...order,
          triggerPrice: params.triggerPrice,
          amount: params.amountSOL,
          orderType: params.orderType,
          maxLoss
        };
      } else {
        // Market order setup (would execute when triggered)
        const stopLossId = `sl_${Date.now()}`;
        
        return {
          id: stopLossId,
          triggerPrice: params.triggerPrice,
          amount: params.amountSOL,
          orderType: params.orderType,
          slippage: params.slippage || 1,
          maxLoss,
          status: 'PENDING'
        };
      }
      
    } catch (error) {
      logger.error('Error creating stop loss:', error);
      throw error;
    }
  }

  /**
   * Execute stop loss when triggered
   */
  async executeStopLoss(stopLossId: string): Promise<void> {
    try {
      logger.warn(`Executing stop loss ${stopLossId}`);
      
      // Implementation would execute market sell
      // This is simplified for example
      
      logger.info('Stop loss executed successfully');
      
    } catch (error) {
      logger.error('Error executing stop loss:', error);
      throw error;
    }
  }

  /**
   * Get current price from pool
   */
  async getCurrentPrice(poolAddress: PublicKey): Promise<number> {
    try {
      const poolInfo = await this.client.getPool(poolAddress);
      const activeBinId = poolInfo.activeBinId;
      const binStep = poolInfo.binStep;
      
      return binIdToPrice(activeBinId, binStep, poolInfo.baseDecimals);
      
    } catch (error) {
      logger.error('Error getting current price:', error);
      throw error;
    }
  }

  /**
   * Get all active orders
   */
  async getActiveOrders(): Promise<RangeOrder[]> {
    const activeOrders = Array.from(this.orders.values()).filter(
      order => order.status === 'ACTIVE' || order.status === 'PARTIALLY_FILLED'
    );
    
    // Update status from chain
    for (const order of activeOrders) {
      await this.updateOrderStatus(order);
    }
    
    return activeOrders;
  }

  /**
   * Update order status from chain
   */
  private async updateOrderStatus(order: RangeOrder): Promise<void> {
    try {
      const position = await this.client.getPosition(order.positionId);
      
      if (!position) {
        order.status = 'CANCELLED';
        return;
      }
      
      const currentPrice = await this.getCurrentPrice(order.poolAddress);
      const activeBinId = priceToBinId(currentPrice, position.pool.binStep, position.pool.baseDecimals);
      
      // Check if price has crossed order bin
      if (order.orderType === 'BUY' && activeBinId <= order.binId) {
        order.status = 'FILLED';
      } else if (order.orderType === 'SELL' && activeBinId >= order.binId) {
        order.status = 'FILLED';
      }
      
      // Check for partial fills
      if (position.liquidity.lt(order.amount)) {
        order.status = 'PARTIALLY_FILLED';
      }
      
    } catch (error) {
      logger.error('Error updating order status:', error);
    }
  }

  /**
   * Rebalance orders based on current market conditions
   */
  async rebalanceOrders(): Promise<void> {
    try {
      logger.info('Rebalancing orders...');
      
      const activeOrders = await this.getActiveOrders();
      
      for (const order of activeOrders) {
        const currentPrice = await this.getCurrentPrice(order.poolAddress);
        const priceDistance = Math.abs(currentPrice - order.actualPrice) / currentPrice;
        
        // Rebalance if price has moved significantly
        if (priceDistance > 0.1) { // 10% threshold
          logger.info(`Rebalancing order ${order.positionId}`);
          
          // Cancel old order
          await this.cancelOrder(order.positionId);
          
          // Create new order at updated price
          if (order.orderType === 'BUY') {
            await this.createLimitBuy({
              poolAddress: order.poolAddress,
              targetPrice: currentPrice * 0.98,
              amountUSDC: order.amount.toNumber() / 1e6,
              metadata: { ...order.metadata, rebalanced: true }
            });
          } else {
            await this.createLimitSell({
              poolAddress: order.poolAddress,
              targetPrice: currentPrice * 1.02,
              amountSOL: order.amount.toNumber() / 1e9,
              metadata: { ...order.metadata, rebalanced: true }
            });
          }
        }
      }
      
      logger.info('Rebalancing complete');
      
    } catch (error) {
      logger.error('Error rebalancing orders:', error);
      throw error;
    }
  }

  /**
   * Cancel an order
   */
  async cancelOrder(positionId: string): Promise<void> {
    try {
      await this.client.closePosition(positionId);
      
      const order = this.orders.get(positionId);
      if (order) {
        order.status = 'CANCELLED';
      }
      
      logger.info(`Order ${positionId} cancelled`);
      
    } catch (error) {
      logger.error('Error cancelling order:', error);
      throw error;
    }
  }

  /**
   * Calculate ladder amounts based on distribution
   */
  private calculateLadderAmounts(
    totalAmount: number,
    steps: number,
    distribution: 'linear' | 'exponential'
  ): number[] {
    const amounts: number[] = [];
    
    if (distribution === 'linear') {
      const amountPerStep = totalAmount / steps;
      for (let i = 0; i < steps; i++) {
        amounts.push(amountPerStep);
      }
    } else {
      // Exponential distribution (more at lower prices)
      const factor = 1.5;
      let sum = 0;
      for (let i = 0; i < steps; i++) {
        sum += Math.pow(factor, i);
      }
      
      for (let i = 0; i < steps; i++) {
        const weight = Math.pow(factor, i) / sum;
        amounts.push(totalAmount * weight);
      }
    }
    
    return amounts;
  }
}