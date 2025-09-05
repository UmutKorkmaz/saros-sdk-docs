/**
 * RouteOptimizer - Optimization algorithms for route selection
 * 
 * Provides optimization strategies for finding the best routes
 * and calculating optimal split distributions.
 */

import BN from 'bn.js';
import { Route } from './MultiHopRouter';
import { logger } from './utils/logger';

export interface OptimizationParams {
  maxPriceImpact?: number;
  minOutput?: BN;
  maxFees?: number;
  preferredPools?: string[];
}

export interface SplitResult {
  percentage: number;
  amount: BN;
  expectedOutput: BN;
}

export class RouteOptimizer {
  private optimizationHistory: Map<string, any> = new Map();

  constructor() {}

  /**
   * Initialize the optimizer
   */
  async initialize(): Promise<void> {
    logger.info('RouteOptimizer initialized');
  }

  /**
   * Optimize route selection based on multiple criteria
   */
  optimizeRoutes(
    routes: Route[],
    params: OptimizationParams = {}
  ): Route[] {
    try {
      let optimized = [...routes];
      
      // Filter by constraints
      if (params.maxPriceImpact) {
        optimized = optimized.filter(r => r.priceImpact <= params.maxPriceImpact!);
      }
      
      if (params.minOutput) {
        optimized = optimized.filter(r => r.expectedOutput.gte(params.minOutput!));
      }
      
      if (params.maxFees) {
        optimized = optimized.filter(r => r.totalFees <= params.maxFees!);
      }
      
      // Prefer certain pools if specified
      if (params.preferredPools && params.preferredPools.length > 0) {
        optimized = this.rankByPreferredPools(optimized, params.preferredPools);
      }
      
      // Sort by composite score
      optimized.sort((a, b) => {
        const scoreA = this.calculateCompositeScore(a);
        const scoreB = this.calculateCompositeScore(b);
        return scoreB - scoreA;
      });
      
      return optimized;
      
    } catch (error) {
      logger.error('Error optimizing routes:', error);
      return routes;
    }
  }

  /**
   * Calculate optimal split across multiple routes
   */
  calculateOptimalSplit(routes: Route[], totalAmount: BN): SplitResult[] {
    try {
      if (routes.length === 0) {
        return [];
      }
      
      if (routes.length === 1) {
        return [{
          percentage: 1,
          amount: totalAmount,
          expectedOutput: routes[0].expectedOutput
        }];
      }
      
      // Use convex optimization for split calculation
      const splits = this.convexOptimization(routes, totalAmount);
      
      // Ensure splits sum to 100%
      const totalPercentage = splits.reduce((sum, s) => sum + s.percentage, 0);
      if (Math.abs(totalPercentage - 1) > 0.001) {
        // Normalize
        splits.forEach(s => s.percentage = s.percentage / totalPercentage);
      }
      
      return splits;
      
    } catch (error) {
      logger.error('Error calculating optimal split:', error);
      // Fallback to equal split
      return this.equalSplit(routes, totalAmount);
    }
  }

  /**
   * Convex optimization for split calculation
   */
  private convexOptimization(routes: Route[], totalAmount: BN): SplitResult[] {
    // Simplified convex optimization
    // In production, would use proper optimization library
    
    const results: SplitResult[] = [];
    const routeScores = routes.map(r => this.calculateCompositeScore(r));
    const totalScore = routeScores.reduce((sum, s) => sum + s, 0);
    
    if (totalScore === 0) {
      return this.equalSplit(routes, totalAmount);
    }
    
    // Allocate based on score with impact adjustment
    let remainingAmount = totalAmount;
    
    for (let i = 0; i < routes.length; i++) {
      const route = routes[i];
      const score = routeScores[i];
      
      // Base allocation proportional to score
      let percentage = score / totalScore;
      
      // Adjust for price impact
      const impactFactor = 1 / (1 + route.priceImpact / 100);
      percentage *= impactFactor;
      
      // Adjust for liquidity constraints
      const liquidityFactor = this.calculateLiquidityFactor(route);
      percentage *= liquidityFactor;
      
      // Calculate amount
      const amount = i === routes.length - 1 
        ? remainingAmount // Last route gets remainder
        : totalAmount.mul(new BN(Math.floor(percentage * 10000))).div(new BN(10000));
      
      remainingAmount = remainingAmount.sub(amount);
      
      results.push({
        percentage,
        amount,
        expectedOutput: this.estimateOutputForAmount(route, amount, totalAmount)
      });
    }
    
    return results;
  }

  /**
   * Equal split fallback
   */
  private equalSplit(routes: Route[], totalAmount: BN): SplitResult[] {
    const percentage = 1 / routes.length;
    const amountPerRoute = totalAmount.div(new BN(routes.length));
    
    return routes.map((route, i) => ({
      percentage,
      amount: i === routes.length - 1 
        ? totalAmount.sub(amountPerRoute.mul(new BN(routes.length - 1)))
        : amountPerRoute,
      expectedOutput: this.estimateOutputForAmount(route, amountPerRoute, totalAmount)
    }));
  }

  /**
   * Calculate composite score for route ranking
   */
  private calculateCompositeScore(route: Route): number {
    // Multi-factor scoring
    const outputScore = Math.log(route.expectedOutput.toNumber() + 1);
    const impactPenalty = route.priceImpact * route.priceImpact; // Quadratic penalty
    const feePenalty = route.totalFees * 10;
    const hopPenalty = (route.hops.length - 1) * 5;
    const confidenceBonus = route.confidence / 10;
    
    const score = outputScore - impactPenalty - feePenalty - hopPenalty + confidenceBonus;
    
    return Math.max(0, score);
  }

  /**
   * Calculate liquidity factor for route
   */
  private calculateLiquidityFactor(route: Route): number {
    // Find minimum liquidity in route
    const minLiquidity = Math.min(...route.hops.map(h => h.liquidity.toNumber()));
    
    // Sigmoid function for smooth factor
    const x = minLiquidity / 1000000; // Normalize to millions
    return 2 / (1 + Math.exp(-x)) - 0.5; // Range: 0.5 to 1.5
  }

  /**
   * Estimate output for partial amount
   */
  private estimateOutputForAmount(route: Route, amount: BN, totalAmount: BN): BN {
    // Linear approximation (simplified)
    const ratio = amount.mul(new BN(10000)).div(totalAmount).toNumber() / 10000;
    return route.expectedOutput.mul(new BN(Math.floor(ratio * 10000))).div(new BN(10000));
  }

  /**
   * Rank routes by preferred pools
   */
  private rankByPreferredPools(routes: Route[], preferredPools: string[]): Route[] {
    return routes.sort((a, b) => {
      const aPreferred = a.hops.filter(h => 
        preferredPools.includes(h.poolAddress.toString())
      ).length;
      const bPreferred = b.hops.filter(h => 
        preferredPools.includes(h.poolAddress.toString())
      ).length;
      
      return bPreferred - aPreferred;
    });
  }

  /**
   * Optimize for MEV resistance
   */
  optimizeForMEVResistance(routes: Route[]): Route[] {
    // Prefer routes with:
    // 1. Fewer hops (less surface area)
    // 2. Higher liquidity pools (harder to manipulate)
    // 3. Lower price impact (less profitable to sandwich)
    
    return routes.sort((a, b) => {
      const aMEVScore = this.calculateMEVResistanceScore(a);
      const bMEVScore = this.calculateMEVResistanceScore(b);
      return bMEVScore - aMEVScore;
    });
  }

  /**
   * Calculate MEV resistance score
   */
  private calculateMEVResistanceScore(route: Route): number {
    let score = 100;
    
    // Penalize more hops
    score -= route.hops.length * 10;
    
    // Reward high liquidity
    const avgLiquidity = route.hops.reduce((sum, h) => 
      sum + h.liquidity.toNumber(), 0
    ) / route.hops.length;
    score += Math.log10(avgLiquidity);
    
    // Penalize high price impact
    score -= route.priceImpact * 5;
    
    return score;
  }

  /**
   * Dynamic route adjustment based on market conditions
   */
  dynamicAdjustment(
    routes: Route[],
    marketVolatility: number,
    gasPrice: number
  ): Route[] {
    return routes.map(route => {
      const adjusted = { ...route };
      
      // Increase confidence penalty during high volatility
      if (marketVolatility > 0.05) {
        adjusted.confidence *= (1 - marketVolatility);
      }
      
      // Adjust for gas costs
      const estimatedGas = route.hops.length * 200000 * gasPrice;
      const gasImpact = estimatedGas / route.expectedOutput.toNumber();
      adjusted.totalFees += gasImpact;
      
      return adjusted;
    });
  }

  /**
   * Backtest optimization strategy
   */
  async backtestStrategy(
    historicalData: any[],
    strategy: 'BALANCED' | 'MIN_IMPACT' | 'MAX_OUTPUT'
  ): Promise<any> {
    // Would implement backtesting logic
    logger.info(`Backtesting ${strategy} strategy`);
    
    return {
      strategy,
      avgReturn: 0,
      sharpeRatio: 0,
      maxDrawdown: 0,
      winRate: 0
    };
  }

  /**
   * Get optimization statistics
   */
  getStatistics(): any {
    return {
      historySize: this.optimizationHistory.size,
      // Additional metrics would be tracked
    };
  }
}