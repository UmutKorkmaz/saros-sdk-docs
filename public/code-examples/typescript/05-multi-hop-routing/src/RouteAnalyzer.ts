/**
 * RouteAnalyzer - Analysis tools for route comparison
 * 
 * Provides comprehensive analysis of routing options
 * and performance metrics.
 */

import { Connection, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { MultiHopRouter, Route } from './MultiHopRouter';
import { logger } from './utils/logger';

export interface AnalysisParams {
  fromMint: PublicKey;
  toMint: PublicKey;
  amount: BN;
  maxHops?: number;
}

export interface RouteAnalysis {
  totalRoutes: number;
  bestRoute: Route;
  worstRoute: Route;
  averageFees: number;
  averageImpact: number;
  topRoutes: Route[];
  distribution: {
    oneHop: number;
    twoHop: number;
    threeHop: number;
    moreThanThree: number;
  };
}

export interface ComparativeAnalysis {
  route1: Route;
  route2: Route;
  outputDifference: BN;
  outputDifferencePercent: number;
  impactDifference: number;
  feeDifference: number;
  recommendation: string;
}

export class RouteAnalyzer {
  private connection: Connection;
  private router: MultiHopRouter | null = null;
  private analysisCache: Map<string, RouteAnalysis> = new Map();

  constructor(connection: Connection) {
    this.connection = connection;
  }

  /**
   * Initialize the analyzer
   */
  async initialize(): Promise<void> {
    try {
      logger.info('Initializing RouteAnalyzer...');
      
      // Create temporary wallet for analysis (not for execution)
      const dummyWallet = new Uint8Array(64);
      const wallet = { publicKey: new PublicKey(dummyWallet.slice(32)), secretKey: dummyWallet } as any;
      
      this.router = new MultiHopRouter(this.connection, wallet);
      await this.router.initialize();
      
      logger.info('RouteAnalyzer initialized');
      
    } catch (error) {
      logger.error('Error initializing RouteAnalyzer:', error);
      throw error;
    }
  }

  /**
   * Analyze all possible routes
   */
  async analyzeAllRoutes(params: AnalysisParams): Promise<RouteAnalysis> {
    try {
      const cacheKey = this.getCacheKey(params);
      const cached = this.analysisCache.get(cacheKey);
      
      if (cached) {
        logger.debug('Returning cached analysis');
        return cached;
      }
      
      logger.info(`Analyzing routes: ${params.fromMint.toString()} → ${params.toMint.toString()}`);
      
      if (!this.router) {
        throw new Error('Router not initialized');
      }
      
      // Find all routes with different hop counts
      const allRoutes: Route[] = [];
      
      for (let maxHops = 1; maxHops <= (params.maxHops || 3); maxHops++) {
        const routes = await this.router.findMultipleRoutes({
          fromMint: params.fromMint,
          toMint: params.toMint,
          amount: params.amount,
          maxRoutes: 10,
          maxHops
        });
        
        allRoutes.push(...routes);
      }
      
      if (allRoutes.length === 0) {
        throw new Error('No routes found for analysis');
      }
      
      // Sort by output
      const sortedByOutput = [...allRoutes].sort((a, b) => 
        b.expectedOutput.sub(a.expectedOutput).toNumber()
      );
      
      // Calculate statistics
      const totalFees = allRoutes.reduce((sum, r) => sum + r.totalFees, 0);
      const totalImpact = allRoutes.reduce((sum, r) => sum + r.priceImpact, 0);
      
      const analysis: RouteAnalysis = {
        totalRoutes: allRoutes.length,
        bestRoute: sortedByOutput[0],
        worstRoute: sortedByOutput[sortedByOutput.length - 1],
        averageFees: totalFees / allRoutes.length,
        averageImpact: totalImpact / allRoutes.length,
        topRoutes: sortedByOutput.slice(0, 5),
        distribution: {
          oneHop: allRoutes.filter(r => r.hops.length === 1).length,
          twoHop: allRoutes.filter(r => r.hops.length === 2).length,
          threeHop: allRoutes.filter(r => r.hops.length === 3).length,
          moreThanThree: allRoutes.filter(r => r.hops.length > 3).length
        }
      };
      
      // Cache the analysis
      this.analysisCache.set(cacheKey, analysis);
      
      logger.info(`Analysis complete: ${analysis.totalRoutes} routes found`);
      
      return analysis;
      
    } catch (error) {
      logger.error('Error analyzing routes:', error);
      throw error;
    }
  }

  /**
   * Compare two specific routes
   */
  compareRoutes(route1: Route, route2: Route): ComparativeAnalysis {
    const outputDifference = route1.expectedOutput.sub(route2.expectedOutput);
    const outputDifferencePercent = outputDifference
      .mul(new BN(10000))
      .div(route2.expectedOutput)
      .toNumber() / 100;
    
    const impactDifference = route1.priceImpact - route2.priceImpact;
    const feeDifference = route1.totalFees - route2.totalFees;
    
    // Generate recommendation
    let recommendation = '';
    
    if (outputDifferencePercent > 1) {
      recommendation = `Route 1 provides ${outputDifferencePercent.toFixed(2)}% better output`;
    } else if (outputDifferencePercent < -1) {
      recommendation = `Route 2 provides ${Math.abs(outputDifferencePercent).toFixed(2)}% better output`;
    } else if (impactDifference < -0.5) {
      recommendation = 'Route 1 has significantly lower price impact';
    } else if (impactDifference > 0.5) {
      recommendation = 'Route 2 has significantly lower price impact';
    } else if (feeDifference < -0.1) {
      recommendation = 'Route 1 has lower fees';
    } else if (feeDifference > 0.1) {
      recommendation = 'Route 2 has lower fees';
    } else {
      recommendation = 'Both routes are similar, choose based on confidence';
    }
    
    return {
      route1,
      route2,
      outputDifference,
      outputDifferencePercent,
      impactDifference,
      feeDifference,
      recommendation
    };
  }

  /**
   * Analyze price impact across different amounts
   */
  async analyzePriceImpactCurve(
    fromMint: PublicKey,
    toMint: PublicKey,
    amounts: BN[]
  ): Promise<Array<{ amount: BN; impact: number; output: BN }>> {
    try {
      logger.info('Analyzing price impact curve');
      
      if (!this.router) {
        throw new Error('Router not initialized');
      }
      
      const results: Array<{ amount: BN; impact: number; output: BN }> = [];
      
      for (const amount of amounts) {
        const route = await this.router.findBestRoute({
          fromMint,
          toMint,
          amount,
          maxHops: 3
        });
        
        if (route) {
          results.push({
            amount,
            impact: route.priceImpact,
            output: route.expectedOutput
          });
        }
      }
      
      return results;
      
    } catch (error) {
      logger.error('Error analyzing price impact curve:', error);
      return [];
    }
  }

  /**
   * Find optimal trade size
   */
  async findOptimalTradeSize(
    fromMint: PublicKey,
    toMint: PublicKey,
    minAmount: BN,
    maxAmount: BN,
    steps: number = 10
  ): Promise<{ optimalAmount: BN; expectedOutput: BN; priceImpact: number }> {
    try {
      logger.info('Finding optimal trade size');
      
      const stepSize = maxAmount.sub(minAmount).div(new BN(steps));
      let bestEfficiency = 0;
      let optimalAmount = minAmount;
      let bestRoute: Route | null = null;
      
      for (let i = 0; i <= steps; i++) {
        const amount = minAmount.add(stepSize.mul(new BN(i)));
        
        const route = await this.router!.findBestRoute({
          fromMint,
          toMint,
          amount,
          maxHops: 3
        });
        
        if (route) {
          // Efficiency = output / (input * (1 + impact))
          const efficiency = route.expectedOutput.toNumber() / 
            (amount.toNumber() * (1 + route.priceImpact / 100));
          
          if (efficiency > bestEfficiency) {
            bestEfficiency = efficiency;
            optimalAmount = amount;
            bestRoute = route;
          }
        }
      }
      
      if (!bestRoute) {
        throw new Error('No optimal trade size found');
      }
      
      logger.info(`Optimal trade size: ${optimalAmount.toString()}`);
      
      return {
        optimalAmount,
        expectedOutput: bestRoute.expectedOutput,
        priceImpact: bestRoute.priceImpact
      };
      
    } catch (error) {
      logger.error('Error finding optimal trade size:', error);
      throw error;
    }
  }

  /**
   * Analyze route stability over time
   */
  async analyzeRouteStability(
    route: Route,
    intervals: number = 10,
    delayMs: number = 1000
  ): Promise<{
    stableRoute: boolean;
    priceVariation: number;
    liquidityVariation: number;
  }> {
    try {
      logger.info('Analyzing route stability');
      
      const priceSnapshots: number[] = [];
      const liquiditySnapshots: number[] = [];
      
      for (let i = 0; i < intervals; i++) {
        // Simulate route to get current state
        const simulation = await this.router!.simulateRoute(route);
        
        if (simulation.success) {
          const priceRatio = simulation.amountOut.toNumber() / route.expectedOutput.toNumber();
          priceSnapshots.push(priceRatio);
        }
        
        // Wait before next check
        if (i < intervals - 1) {
          await new Promise(resolve => setTimeout(resolve, delayMs));
        }
      }
      
      // Calculate variations
      const avgPrice = priceSnapshots.reduce((a, b) => a + b, 0) / priceSnapshots.length;
      const priceVariation = Math.sqrt(
        priceSnapshots.reduce((sum, p) => sum + Math.pow(p - avgPrice, 2), 0) / priceSnapshots.length
      );
      
      // Route is stable if variation is low
      const stableRoute = priceVariation < 0.02; // 2% threshold
      
      return {
        stableRoute,
        priceVariation: priceVariation * 100,
        liquidityVariation: 0 // Would calculate from actual liquidity data
      };
      
    } catch (error) {
      logger.error('Error analyzing route stability:', error);
      return {
        stableRoute: false,
        priceVariation: 100,
        liquidityVariation: 100
      };
    }
  }

  /**
   * Generate route report
   */
  generateReport(analysis: RouteAnalysis): string {
    const report = [
      '=== Route Analysis Report ===',
      '',
      `Total Routes Found: ${analysis.totalRoutes}`,
      `Average Fees: ${analysis.averageFees.toFixed(4)}%`,
      `Average Price Impact: ${analysis.averageImpact.toFixed(4)}%`,
      '',
      '=== Route Distribution ===',
      `1-Hop Routes: ${analysis.distribution.oneHop}`,
      `2-Hop Routes: ${analysis.distribution.twoHop}`,
      `3-Hop Routes: ${analysis.distribution.threeHop}`,
      `4+ Hop Routes: ${analysis.distribution.moreThanThree}`,
      '',
      '=== Best Route ===',
      `Path: ${analysis.bestRoute.path.map(p => p.symbol).join(' → ')}`,
      `Expected Output: ${analysis.bestRoute.expectedOutput.toString()}`,
      `Price Impact: ${analysis.bestRoute.priceImpact.toFixed(4)}%`,
      `Total Fees: ${analysis.bestRoute.totalFees.toFixed(4)}%`,
      '',
      '=== Top 5 Routes by Output ===',
    ];
    
    analysis.topRoutes.forEach((route, i) => {
      report.push(
        `${i + 1}. ${route.path.map(p => p.symbol).join(' → ')}`,
        `   Output: ${route.expectedOutput.toString()}`,
        `   Impact: ${route.priceImpact.toFixed(4)}%`
      );
    });
    
    return report.join('\n');
  }

  /**
   * Calculate slippage recommendation
   */
  calculateSlippageRecommendation(route: Route): number {
    // Base slippage on price impact
    let slippage = route.priceImpact * 1.5;
    
    // Add buffer for each hop
    slippage += route.hops.length * 0.1;
    
    // Add buffer for low confidence
    if (route.confidence < 80) {
      slippage += (100 - route.confidence) / 100;
    }
    
    // Round to reasonable precision
    slippage = Math.round(slippage * 100) / 100;
    
    // Cap at reasonable maximum
    return Math.min(slippage, 5);
  }

  /**
   * Get cache key for analysis
   */
  private getCacheKey(params: AnalysisParams): string {
    return `${params.fromMint.toString()}-${params.toMint.toString()}-${params.amount.toString()}`;
  }

  /**
   * Clear analysis cache
   */
  clearCache(): void {
    this.analysisCache.clear();
    logger.info('Analysis cache cleared');
  }
}