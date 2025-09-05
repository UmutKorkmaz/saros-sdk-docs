/**
 * GasOptimizer - Gas optimization utilities for auto-compound operations
 */

import { Connection, Transaction, PublicKey } from '@solana/web3.js';
import { logger } from './utils/logger';

export interface GasEstimate {
  baseFee: number;
  priorityFee: number;
  totalFee: number;
  computeUnits: number;
  recommendedFee: number;
}

export interface GasMetrics {
  currentPrice: number;
  averagePrice24h: number;
  highPrice24h: number;
  lowPrice24h: number;
  congestionLevel: 'low' | 'medium' | 'high';
  recommendedAction: string;
}

export class GasOptimizer {
  private connection: Connection;
  private priceHistory: number[] = [];
  private readonly MAX_HISTORY = 100;
  private readonly PRIORITY_LEVELS = {
    low: 1000,     // 0.000001 SOL
    medium: 10000, // 0.00001 SOL
    high: 100000,  // 0.0001 SOL
    urgent: 1000000 // 0.001 SOL
  };

  constructor(connection: Connection) {
    this.connection = connection;
    logger.info('GasOptimizer initialized');
  }

  /**
   * Get current gas price
   */
  async getCurrentGasPrice(): Promise<number> {
    try {
      const recentFees = await this.connection.getRecentPrioritizationFees();
      
      if (recentFees.length === 0) {
        return 0.000005; // Default 5000 lamports
      }

      // Calculate average of recent fees
      const avgFee = recentFees.reduce((sum, fee) => 
        sum + fee.prioritizationFee, 0
      ) / recentFees.length;

      const gasPriceSOL = avgFee / 1e9; // Convert lamports to SOL
      
      // Add to history
      this.addToPriceHistory(gasPriceSOL);
      
      return gasPriceSOL;
    } catch (error) {
      logger.error('Failed to get gas price:', error);
      return 0.000005; // Default fallback
    }
  }

  /**
   * Estimate gas for transaction
   */
  async estimateGas(transaction: Transaction): Promise<GasEstimate> {
    try {
      // Simulate transaction to get compute units
      const simulation = await this.connection.simulateTransaction(transaction);
      
      const computeUnits = simulation.value.unitsConsumed || 200000;
      const baseFee = 0.000005; // 5000 lamports base fee
      
      // Get current priority fee
      const currentGasPrice = await this.getCurrentGasPrice();
      const priorityFee = currentGasPrice - baseFee;
      
      // Calculate recommended fee based on congestion
      const congestion = await this.getCongestionLevel();
      let recommendedMultiplier = 1;
      
      switch (congestion) {
        case 'high':
          recommendedMultiplier = 2;
          break;
        case 'medium':
          recommendedMultiplier = 1.5;
          break;
        default:
          recommendedMultiplier = 1;
      }
      
      const recommendedFee = currentGasPrice * recommendedMultiplier;
      
      return {
        baseFee,
        priorityFee,
        totalFee: baseFee + priorityFee,
        computeUnits,
        recommendedFee
      };
    } catch (error) {
      logger.error('Failed to estimate gas:', error);
      
      return {
        baseFee: 0.000005,
        priorityFee: 0,
        totalFee: 0.000005,
        computeUnits: 200000,
        recommendedFee: 0.000005
      };
    }
  }

  /**
   * Get gas metrics
   */
  async getGasMetrics(): Promise<GasMetrics> {
    const currentPrice = await this.getCurrentGasPrice();
    const congestionLevel = await this.getCongestionLevel();
    
    // Calculate 24h metrics from history
    const prices24h = this.priceHistory.slice(-96); // Assuming 15min intervals
    const averagePrice24h = prices24h.length > 0 ?
      prices24h.reduce((sum, p) => sum + p, 0) / prices24h.length :
      currentPrice;
    
    const highPrice24h = prices24h.length > 0 ?
      Math.max(...prices24h) :
      currentPrice;
    
    const lowPrice24h = prices24h.length > 0 ?
      Math.min(...prices24h) :
      currentPrice;
    
    // Determine recommended action
    let recommendedAction = 'Proceed with transaction';
    
    if (congestionLevel === 'high') {
      recommendedAction = 'Consider waiting for lower congestion';
    } else if (currentPrice > averagePrice24h * 1.5) {
      recommendedAction = 'Gas prices elevated, consider waiting';
    } else if (currentPrice < averagePrice24h * 0.7) {
      recommendedAction = 'Good time to execute transactions';
    }
    
    return {
      currentPrice,
      averagePrice24h,
      highPrice24h,
      lowPrice24h,
      congestionLevel,
      recommendedAction
    };
  }

  /**
   * Check if gas price is favorable
   */
  async isGasFavorable(maxGasPrice: number): Promise<boolean> {
    const currentPrice = await this.getCurrentGasPrice();
    return currentPrice <= maxGasPrice;
  }

  /**
   * Wait for favorable gas price
   */
  async waitForFavorableGas(
    maxGasPrice: number,
    maxWaitTime: number = 300000, // 5 minutes
    checkInterval: number = 10000 // 10 seconds
  ): Promise<boolean> {
    const startTime = Date.now();
    
    logger.info(`Waiting for gas price <= ${maxGasPrice} SOL...`);
    
    while (Date.now() - startTime < maxWaitTime) {
      if (await this.isGasFavorable(maxGasPrice)) {
        logger.info('Favorable gas price reached');
        return true;
      }
      
      await new Promise(resolve => setTimeout(resolve, checkInterval));
    }
    
    logger.warn('Timeout waiting for favorable gas price');
    return false;
  }

  /**
   * Optimize transaction for gas
   */
  async optimizeTransaction(
    transaction: Transaction,
    urgency: 'low' | 'medium' | 'high' | 'urgent' = 'medium'
  ): Promise<Transaction> {
    // Set compute budget
    const computeBudgetIx = this.createComputeBudgetInstruction(
      this.PRIORITY_LEVELS[urgency]
    );
    
    // Add as first instruction
    transaction.instructions.unshift(computeBudgetIx);
    
    // Set priority fee
    const priorityFeeIx = this.createPriorityFeeInstruction(
      this.PRIORITY_LEVELS[urgency]
    );
    
    transaction.instructions.unshift(priorityFeeIx);
    
    return transaction;
  }

  /**
   * Batch transactions for gas efficiency
   */
  async batchTransactions(
    transactions: Transaction[]
  ): Promise<Transaction[]> {
    const MAX_TX_SIZE = 1232; // Max transaction size in bytes
    const batched: Transaction[] = [];
    let current = new Transaction();
    let currentSize = 0;
    
    for (const tx of transactions) {
      const txSize = tx.serialize().length;
      
      if (currentSize + txSize > MAX_TX_SIZE) {
        // Start new batch
        if (current.instructions.length > 0) {
          batched.push(current);
        }
        current = new Transaction();
        currentSize = 0;
      }
      
      // Add instructions to current batch
      current.instructions.push(...tx.instructions);
      currentSize += txSize;
    }
    
    // Add remaining batch
    if (current.instructions.length > 0) {
      batched.push(current);
    }
    
    logger.info(`Batched ${transactions.length} transactions into ${batched.length}`);
    
    return batched;
  }

  /**
   * Calculate optimal priority fee
   */
  async calculateOptimalPriorityFee(
    targetConfirmationTime: number = 5000 // 5 seconds
  ): Promise<number> {
    const recentFees = await this.connection.getRecentPrioritizationFees();
    
    if (recentFees.length === 0) {
      return this.PRIORITY_LEVELS.medium;
    }
    
    // Sort by slot (most recent first)
    recentFees.sort((a, b) => b.slot - a.slot);
    
    // Take top 25% for fast confirmation
    const topQuartile = Math.ceil(recentFees.length * 0.25);
    const fastFees = recentFees.slice(0, topQuartile);
    
    const avgFastFee = fastFees.reduce((sum, fee) => 
      sum + fee.prioritizationFee, 0
    ) / fastFees.length;
    
    // Add 10% buffer
    return Math.ceil(avgFastFee * 1.1);
  }

  /**
   * Get congestion level
   */
  private async getCongestionLevel(): Promise<'low' | 'medium' | 'high'> {
    try {
      const perfSamples = await this.connection.getRecentPerformanceSamples(1);
      
      if (perfSamples.length === 0) {
        return 'medium';
      }
      
      const tps = perfSamples[0].numTransactions / perfSamples[0].samplePeriodSecs;
      
      // Estimate congestion based on TPS
      if (tps > 2500) {
        return 'high';
      } else if (tps > 1500) {
        return 'medium';
      } else {
        return 'low';
      }
    } catch (error) {
      logger.error('Failed to get congestion level:', error);
      return 'medium';
    }
  }

  /**
   * Add price to history
   */
  private addToPriceHistory(price: number): void {
    this.priceHistory.push(price);
    
    if (this.priceHistory.length > this.MAX_HISTORY) {
      this.priceHistory.shift();
    }
  }

  /**
   * Create compute budget instruction
   */
  private createComputeBudgetInstruction(units: number): any {
    // Simplified - in production use actual compute budget program
    return {
      programId: new PublicKey('ComputeBudget111111111111111111111111111111'),
      keys: [],
      data: Buffer.from([units])
    };
  }

  /**
   * Create priority fee instruction
   */
  private createPriorityFeeInstruction(lamports: number): any {
    // Simplified - in production use actual priority fee instruction
    return {
      programId: new PublicKey('ComputeBudget111111111111111111111111111111'),
      keys: [],
      data: Buffer.from([lamports])
    };
  }

  /**
   * Estimate annual gas costs
   */
  estimateAnnualGasCost(
    transactionsPerDay: number,
    averageGasPrice?: number
  ): {
    dailyCost: number;
    monthlyCost: number;
    annualCost: number;
    inSOL: number;
    inUSD: number;
  } {
    const gasPrice = averageGasPrice || 0.000005;
    const solPrice = 50; // USD
    
    const dailyCost = transactionsPerDay * gasPrice;
    const monthlyCost = dailyCost * 30;
    const annualCost = dailyCost * 365;
    
    return {
      dailyCost,
      monthlyCost,
      annualCost,
      inSOL: annualCost,
      inUSD: annualCost * solPrice
    };
  }

  /**
   * Get gas price statistics
   */
  getGasPriceStats(): {
    current: number;
    average: number;
    median: number;
    stdDev: number;
    trend: 'increasing' | 'decreasing' | 'stable';
  } {
    if (this.priceHistory.length === 0) {
      return {
        current: 0,
        average: 0,
        median: 0,
        stdDev: 0,
        trend: 'stable'
      };
    }
    
    const current = this.priceHistory[this.priceHistory.length - 1];
    const average = this.priceHistory.reduce((sum, p) => sum + p, 0) / this.priceHistory.length;
    
    // Calculate median
    const sorted = [...this.priceHistory].sort((a, b) => a - b);
    const median = sorted[Math.floor(sorted.length / 2)];
    
    // Calculate standard deviation
    const variance = this.priceHistory.reduce((sum, p) => 
      sum + Math.pow(p - average, 2), 0
    ) / this.priceHistory.length;
    const stdDev = Math.sqrt(variance);
    
    // Determine trend
    const recentAvg = this.priceHistory.slice(-10).reduce((sum, p) => sum + p, 0) / 10;
    const olderAvg = this.priceHistory.slice(-20, -10).reduce((sum, p) => sum + p, 0) / 10;
    
    let trend: 'increasing' | 'decreasing' | 'stable' = 'stable';
    if (recentAvg > olderAvg * 1.1) {
      trend = 'increasing';
    } else if (recentAvg < olderAvg * 0.9) {
      trend = 'decreasing';
    }
    
    return {
      current,
      average,
      median,
      stdDev,
      trend
    };
  }
}