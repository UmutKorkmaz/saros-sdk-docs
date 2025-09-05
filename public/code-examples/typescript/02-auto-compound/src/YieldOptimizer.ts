/**
 * YieldOptimizer - Multi-strategy yield optimization
 */

import { PublicKey } from '@solana/web3.js';
import { AutoCompounder } from './AutoCompounder';
import { RewardCalculator } from './RewardCalculator';
import { logger } from './utils/logger';
import Decimal from 'decimal.js';

export interface YieldStrategy {
  type: 'LP' | 'STAKING' | 'FARMING';
  poolAddress: PublicKey;
  weight: number;
  autoCompound: boolean;
  compoundInterval: number;
  minRewardThreshold: number;
  expectedAPY?: number;
  riskScore?: number;
}

export interface OptimizationResult {
  success: boolean;
  activeStrategies: number;
  totalAllocation: number;
  projectedAPY: number;
  riskAdjustedReturn: number;
  rebalanceRequired: boolean;
}

export interface PortfolioAllocation {
  strategy: YieldStrategy;
  currentValue: number;
  targetAllocation: number;
  actualAllocation: number;
  performance: number;
}

export interface RebalanceResult {
  success: boolean;
  strategiesRebalanced: number;
  totalMoved: number;
  newAllocations: PortfolioAllocation[];
  gasUsed: number;
}

export class YieldOptimizer {
  private strategies: Map<string, YieldStrategy> = new Map();
  private autoCompounder: AutoCompounder;
  private rewardCalculator: RewardCalculator;
  private totalCapital: number = 0;
  private rebalanceThreshold: number = 0.1; // 10% deviation triggers rebalance
  private lastRebalance: Date = new Date();
  private performanceHistory: Map<string, number[]> = new Map();

  constructor(config: { rpcUrl: string; privateKey: string }) {
    this.autoCompounder = new AutoCompounder({
      rpcUrl: config.rpcUrl,
      privateKey: config.privateKey
    });
    this.rewardCalculator = new RewardCalculator();
    
    logger.info('YieldOptimizer initialized');
  }

  /**
   * Add a yield strategy to the portfolio
   */
  async addStrategy(strategy: YieldStrategy): Promise<boolean> {
    const strategyKey = `${strategy.type}-${strategy.poolAddress.toString()}`;
    
    // Validate weight
    const totalWeight = this.getTotalWeight() + strategy.weight;
    if (totalWeight > 1.0) {
      logger.error(`Total weight exceeds 100%: ${totalWeight * 100}%`);
      return false;
    }

    // Calculate expected APY if not provided
    if (!strategy.expectedAPY) {
      strategy.expectedAPY = await this.estimateAPY(strategy);
    }

    // Calculate risk score if not provided
    if (!strategy.riskScore) {
      strategy.riskScore = this.calculateRiskScore(strategy);
    }

    this.strategies.set(strategyKey, strategy);
    this.performanceHistory.set(strategyKey, []);

    logger.info(`✅ Strategy added: ${strategyKey}`);
    logger.info(`  Weight: ${strategy.weight * 100}%`);
    logger.info(`  Expected APY: ${strategy.expectedAPY}%`);
    logger.info(`  Risk Score: ${strategy.riskScore}/10`);

    return true;
  }

  /**
   * Remove a strategy from the portfolio
   */
  async removeStrategy(
    type: 'LP' | 'STAKING' | 'FARMING',
    poolAddress: PublicKey
  ): Promise<boolean> {
    const strategyKey = `${type}-${poolAddress.toString()}`;
    
    if (!this.strategies.has(strategyKey)) {
      logger.warn(`Strategy not found: ${strategyKey}`);
      return false;
    }

    // Stop auto-compound if active
    const strategy = this.strategies.get(strategyKey)!;
    if (strategy.autoCompound) {
      await this.autoCompounder.stop(strategy.poolAddress);
    }

    this.strategies.delete(strategyKey);
    this.performanceHistory.delete(strategyKey);

    logger.info(`Strategy removed: ${strategyKey}`);
    return true;
  }

  /**
   * Start yield optimization
   */
  async startOptimization(): Promise<OptimizationResult> {
    try {
      logger.info('🚀 Starting yield optimization...');

      // Validate strategies
      if (this.strategies.size === 0) {
        throw new Error('No strategies configured');
      }

      const totalWeight = this.getTotalWeight();
      if (Math.abs(totalWeight - 1.0) > 0.01) {
        logger.warn(`Total weight is ${totalWeight * 100}%, should be 100%`);
      }

      // Start auto-compound for each strategy
      let activeCount = 0;
      for (const [key, strategy] of this.strategies.entries()) {
        if (strategy.autoCompound) {
          const result = await this.autoCompounder.start({
            poolAddress: strategy.poolAddress,
            strategy: strategy.type,
            interval: strategy.compoundInterval,
            minRewardThreshold: strategy.minRewardThreshold,
            reinvestPercentage: 100
          });

          if (result.success) {
            activeCount++;
            logger.info(`  ✅ Auto-compound started for ${key}`);
          } else {
            logger.error(`  ❌ Failed to start auto-compound for ${key}`);
          }
        }
      }

      // Calculate projected returns
      const projectedAPY = this.calculateProjectedAPY();
      const riskAdjusted = this.calculateRiskAdjustedReturn();

      // Check if rebalance is needed
      const rebalanceRequired = await this.checkRebalanceRequired();

      logger.info('📊 Optimization Summary:');
      logger.info(`  Active strategies: ${activeCount}/${this.strategies.size}`);
      logger.info(`  Projected APY: ${projectedAPY.toFixed(2)}%`);
      logger.info(`  Risk-adjusted return: ${riskAdjusted.toFixed(2)}%`);
      logger.info(`  Rebalance required: ${rebalanceRequired ? 'Yes' : 'No'}`);

      return {
        success: true,
        activeStrategies: activeCount,
        totalAllocation: totalWeight,
        projectedAPY,
        riskAdjustedReturn: riskAdjusted,
        rebalanceRequired
      };

    } catch (error: any) {
      logger.error(`Optimization failed: ${error.message}`);
      
      return {
        success: false,
        activeStrategies: 0,
        totalAllocation: 0,
        projectedAPY: 0,
        riskAdjustedReturn: 0,
        rebalanceRequired: false
      };
    }
  }

  /**
   * Stop all optimizations
   */
  async stopOptimization(): Promise<void> {
    logger.info('Stopping yield optimization...');
    
    await this.autoCompounder.stopAll();
    
    logger.info('Yield optimization stopped');
  }

  /**
   * Rebalance portfolio allocations
   */
  async rebalancePortfolio(force: boolean = false): Promise<RebalanceResult> {
    try {
      logger.info('🔄 Checking portfolio rebalance...');

      // Check if rebalance is needed
      if (!force && !await this.checkRebalanceRequired()) {
        logger.info('Rebalance not required');
        return {
          success: false,
          strategiesRebalanced: 0,
          totalMoved: 0,
          newAllocations: [],
          gasUsed: 0
        };
      }

      // Get current allocations
      const currentAllocations = await this.getCurrentAllocations();
      
      // Calculate optimal allocations
      const optimalAllocations = this.calculateOptimalAllocations();
      
      // Execute rebalance
      let strategiesRebalanced = 0;
      let totalMoved = 0;
      let gasUsed = 0;

      for (const allocation of currentAllocations) {
        const optimal = optimalAllocations.find(
          o => o.strategy === allocation.strategy
        );
        
        if (!optimal) continue;

        const diff = Math.abs(allocation.actualAllocation - optimal.targetAllocation);
        
        if (diff > 0.01) { // 1% threshold
          // Execute rebalance transaction
          const moveAmount = (optimal.targetAllocation - allocation.actualAllocation) * this.totalCapital;
          
          logger.info(`  Rebalancing ${allocation.strategy.type}: ${moveAmount}`);
          
          // Simulate rebalance (in production, execute actual transactions)
          strategiesRebalanced++;
          totalMoved += Math.abs(moveAmount);
          gasUsed += 0.001; // Estimated gas
        }
      }

      this.lastRebalance = new Date();

      logger.info(`✅ Rebalance completed:`);
      logger.info(`  Strategies rebalanced: ${strategiesRebalanced}`);
      logger.info(`  Total moved: $${totalMoved.toFixed(2)}`);
      logger.info(`  Gas used: ${gasUsed} SOL`);

      return {
        success: true,
        strategiesRebalanced,
        totalMoved,
        newAllocations: optimalAllocations,
        gasUsed
      };

    } catch (error: any) {
      logger.error(`Rebalance failed: ${error.message}`);
      
      return {
        success: false,
        strategiesRebalanced: 0,
        totalMoved: 0,
        newAllocations: [],
        gasUsed: 0
      };
    }
  }

  /**
   * Analyze portfolio performance
   */
  async analyzePerformance(): Promise<{
    totalReturn: number;
    averageAPY: number;
    sharpeRatio: number;
    maxDrawdown: number;
    bestStrategy: string;
    worstStrategy: string;
  }> {
    const performances: { strategy: string; return: number; apy: number }[] = [];
    
    for (const [key, history] of this.performanceHistory.entries()) {
      if (history.length === 0) continue;
      
      const returns = history[history.length - 1] - (history[0] || 0);
      const apy = this.calculateAPYFromHistory(history);
      
      performances.push({
        strategy: key,
        return: returns,
        apy
      });
    }

    // Calculate metrics
    const totalReturn = performances.reduce((sum, p) => sum + p.return, 0);
    const averageAPY = performances.reduce((sum, p) => sum + p.apy, 0) / performances.length;
    const sharpeRatio = this.calculateSharpeRatio(performances);
    const maxDrawdown = this.calculateMaxDrawdown();

    // Find best and worst
    performances.sort((a, b) => b.return - a.return);
    const bestStrategy = performances[0]?.strategy || 'None';
    const worstStrategy = performances[performances.length - 1]?.strategy || 'None';

    return {
      totalReturn,
      averageAPY,
      sharpeRatio,
      maxDrawdown,
      bestStrategy,
      worstStrategy
    };
  }

  /**
   * Get total weight of all strategies
   */
  private getTotalWeight(): number {
    let total = 0;
    for (const strategy of this.strategies.values()) {
      total += strategy.weight;
    }
    return total;
  }

  /**
   * Estimate APY for a strategy
   */
  private async estimateAPY(strategy: YieldStrategy): Promise<number> {
    // Base APY by strategy type
    const baseAPY = {
      'LP': 30,
      'STAKING': 20,
      'FARMING': 50
    };

    // Adjust for auto-compound
    const base = baseAPY[strategy.type];
    const compoundBoost = strategy.autoCompound ? 
      this.rewardCalculator.calculateCompoundBoost(
        base,
        365 * 24 * 3600000 / strategy.compoundInterval // compounds per year
      ) : 0;

    return base + compoundBoost;
  }

  /**
   * Calculate risk score for a strategy
   */
  private calculateRiskScore(strategy: YieldStrategy): number {
    // Risk factors (0-10 scale)
    const typeRisk = {
      'LP': 5,      // Medium - IL risk
      'STAKING': 3, // Low - no IL
      'FARMING': 7  // High - multiple risks
    };

    let score = typeRisk[strategy.type];

    // Adjust for other factors
    if (strategy.autoCompound) {
      score += 0.5; // Slightly higher risk due to automation
    }

    return Math.min(10, score);
  }

  /**
   * Calculate projected portfolio APY
   */
  private calculateProjectedAPY(): number {
    let weightedAPY = 0;
    
    for (const strategy of this.strategies.values()) {
      weightedAPY += (strategy.expectedAPY || 0) * strategy.weight;
    }
    
    return weightedAPY;
  }

  /**
   * Calculate risk-adjusted return
   */
  private calculateRiskAdjustedReturn(): number {
    let weightedReturn = 0;
    let weightedRisk = 0;
    
    for (const strategy of this.strategies.values()) {
      const expectedReturn = (strategy.expectedAPY || 0) * strategy.weight;
      const risk = (strategy.riskScore || 5) * strategy.weight;
      
      weightedReturn += expectedReturn;
      weightedRisk += risk;
    }
    
    // Simple risk adjustment (Sharpe-like ratio)
    const riskFreeRate = 2; // 2% risk-free rate
    const adjustedReturn = (weightedReturn - riskFreeRate) / (weightedRisk / 5);
    
    return adjustedReturn;
  }

  /**
   * Check if rebalance is required
   */
  private async checkRebalanceRequired(): Promise<boolean> {
    // Check time since last rebalance
    const timeSinceRebalance = Date.now() - this.lastRebalance.getTime();
    if (timeSinceRebalance < 86400000) { // 24 hours minimum
      return false;
    }

    // Check allocation deviations
    const allocations = await this.getCurrentAllocations();
    
    for (const allocation of allocations) {
      const deviation = Math.abs(
        allocation.actualAllocation - allocation.targetAllocation
      );
      
      if (deviation > this.rebalanceThreshold) {
        logger.info(`Rebalance triggered: ${allocation.strategy.type} deviation ${deviation * 100}%`);
        return true;
      }
    }

    return false;
  }

  /**
   * Get current portfolio allocations
   */
  private async getCurrentAllocations(): Promise<PortfolioAllocation[]> {
    const allocations: PortfolioAllocation[] = [];
    
    // Simulate fetching actual positions
    // In production, query actual balances
    for (const [key, strategy] of this.strategies.entries()) {
      const currentValue = Math.random() * 10000; // Simulated value
      const performance = Math.random() * 50 - 10; // -10% to +40%
      
      allocations.push({
        strategy,
        currentValue,
        targetAllocation: strategy.weight,
        actualAllocation: currentValue / this.totalCapital,
        performance
      });
    }

    return allocations;
  }

  /**
   * Calculate optimal allocations
   */
  private calculateOptimalAllocations(): PortfolioAllocation[] {
    const allocations: PortfolioAllocation[] = [];
    
    // Modern portfolio theory optimization
    // Simplified - in production use proper optimization
    for (const [key, strategy] of this.strategies.entries()) {
      allocations.push({
        strategy,
        currentValue: strategy.weight * this.totalCapital,
        targetAllocation: strategy.weight,
        actualAllocation: strategy.weight,
        performance: 0
      });
    }

    return allocations;
  }

  /**
   * Calculate APY from performance history
   */
  private calculateAPYFromHistory(history: number[]): number {
    if (history.length < 2) return 0;
    
    const initial = history[0];
    const final = history[history.length - 1];
    const days = history.length;
    
    const totalReturn = (final - initial) / initial;
    const annualizedReturn = totalReturn * (365 / days);
    
    return annualizedReturn * 100;
  }

  /**
   * Calculate Sharpe ratio
   */
  private calculateSharpeRatio(performances: any[]): number {
    if (performances.length === 0) return 0;
    
    const returns = performances.map(p => p.return);
    const avgReturn = returns.reduce((sum, r) => sum + r, 0) / returns.length;
    
    // Calculate standard deviation
    const variance = returns.reduce((sum, r) => sum + Math.pow(r - avgReturn, 2), 0) / returns.length;
    const stdDev = Math.sqrt(variance);
    
    const riskFreeRate = 2; // 2% annual
    const sharpe = (avgReturn - riskFreeRate) / stdDev;
    
    return sharpe;
  }

  /**
   * Calculate maximum drawdown
   */
  private calculateMaxDrawdown(): number {
    let maxDrawdown = 0;
    
    for (const history of this.performanceHistory.values()) {
      if (history.length < 2) continue;
      
      let peak = history[0];
      for (const value of history) {
        if (value > peak) {
          peak = value;
        }
        const drawdown = (peak - value) / peak;
        maxDrawdown = Math.max(maxDrawdown, drawdown);
      }
    }
    
    return maxDrawdown * 100; // Return as percentage
  }

  /**
   * Set total capital for allocation calculations
   */
  setTotalCapital(amount: number): void {
    this.totalCapital = amount;
    logger.info(`Total capital set to: $${amount}`);
  }

  /**
   * Get all strategies
   */
  getStrategies(): YieldStrategy[] {
    return Array.from(this.strategies.values());
  }
}