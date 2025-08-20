/**
 * FeeAnalyzer - Analyze fee compensation vs impermanent loss
 */

import { logger } from './utils/logger';

export interface FeeAnalysisParams {
  pool: {
    address: string;
    token0: string;
    token1: string;
    feeRate: number;  // Percentage
  };
  position: {
    liquidity: number;
    duration: number;  // Days
    averageTVL: number;
    dailyVolume: number;
  };
  currentIL: number;  // Current IL percentage
}

export interface FeeAnalysisResult {
  positionValue: number;
  totalFees: number;
  ilInUSD: number;
  netProfit: number;
  feeAPR: number;
  ilCompensated: boolean;
  daysToBreakeven: number;
  projectedProfit30d: number;
  projectedProfit90d: number;
}

export class FeeAnalyzer {
  constructor() {
    logger.info('FeeAnalyzer initialized');
  }

  /**
   * Analyze if fees compensate for IL
   */
  async analyzeFeesVsIL(params: FeeAnalysisParams): Promise<FeeAnalysisResult> {
    const { pool, position, currentIL } = params;
    
    // Calculate position share of pool
    const poolShare = position.liquidity / position.averageTVL;
    
    // Calculate daily fees earned
    const dailyFees = position.dailyVolume * (pool.feeRate / 100) * poolShare;
    
    // Calculate total fees earned over duration
    const totalFees = dailyFees * position.duration;
    
    // Calculate IL in USD
    const ilInUSD = position.liquidity * (currentIL / 100);
    
    // Calculate net profit/loss
    const netProfit = totalFees - ilInUSD;
    
    // Calculate fee APR
    const annualFees = dailyFees * 365;
    const feeAPR = (annualFees / position.liquidity) * 100;
    
    // Calculate days to breakeven
    const daysToBreakeven = ilInUSD / dailyFees;
    
    // Project future profits
    const projectedProfit30d = (dailyFees * 30) - ilInUSD;
    const projectedProfit90d = (dailyFees * 90) - ilInUSD;
    
    return {
      positionValue: position.liquidity,
      totalFees,
      ilInUSD,
      netProfit,
      feeAPR,
      ilCompensated: totalFees >= ilInUSD,
      daysToBreakeven,
      projectedProfit30d,
      projectedProfit90d
    };
  }

  /**
   * Calculate optimal fee tier based on volatility
   */
  calculateOptimalFeeTier(params: {
    volatility: number;
    expectedVolume: number;
    priceRange: [number, number];
  }): {
    recommendedTier: number;
    expectedFees: { [tier: string]: number };
    ilVsFees: { [tier: string]: { fees: number; il: number; net: number } };
  } {
    const { volatility, expectedVolume } = params;
    
    // Common fee tiers
    const feeTiers = [0.05, 0.3, 1.0]; // 5bps, 30bps, 100bps
    
    // Calculate expected fees for each tier
    const expectedFees: { [tier: string]: number } = {};
    const ilVsFees: { [tier: string]: any } = {};
    
    let recommendedTier = 0.3; // Default
    let maxNet = -Infinity;
    
    for (const tier of feeTiers) {
      const tierKey = `${tier}%`;
      const fees = expectedVolume * (tier / 100);
      expectedFees[tierKey] = fees;
      
      // Higher fee tiers typically have lower volume but higher fees per trade
      const volumeAdjustment = tier === 0.05 ? 2 : tier === 1.0 ? 0.5 : 1;
      const adjustedFees = fees * volumeAdjustment;
      
      // Estimate IL based on volatility
      const estimatedIL = volatility * 0.1; // Simplified
      
      const net = adjustedFees - estimatedIL;
      ilVsFees[tierKey] = {
        fees: adjustedFees,
        il: estimatedIL,
        net
      };
      
      if (net > maxNet) {
        maxNet = net;
        recommendedTier = tier;
      }
    }
    
    return {
      recommendedTier,
      expectedFees,
      ilVsFees
    };
  }

  /**
   * Historical fee analysis
   */
  async analyzeHistoricalFees(params: {
    poolAddress: string;
    period: number;  // Days
  }): Promise<{
    averageDailyFees: number;
    totalFees: number;
    feeVolatility: number;
    bestDay: { date: Date; fees: number };
    worstDay: { date: Date; fees: number };
  }> {
    // In production, fetch actual historical data
    // Simulated data for demo
    
    const dailyFees: number[] = [];
    let totalFees = 0;
    let bestDay = { date: new Date(), fees: 0 };
    let worstDay = { date: new Date(), fees: Infinity };
    
    for (let i = 0; i < params.period; i++) {
      const fees = Math.random() * 50 + 10; // $10-60 daily
      dailyFees.push(fees);
      totalFees += fees;
      
      const date = new Date(Date.now() - i * 24 * 60 * 60 * 1000);
      
      if (fees > bestDay.fees) {
        bestDay = { date, fees };
      }
      if (fees < worstDay.fees) {
        worstDay = { date, fees };
      }
    }
    
    const averageDailyFees = totalFees / params.period;
    
    // Calculate volatility
    const variance = dailyFees.reduce((sum, fee) => 
      sum + Math.pow(fee - averageDailyFees, 2), 0
    ) / params.period;
    const feeVolatility = Math.sqrt(variance);
    
    return {
      averageDailyFees,
      totalFees,
      feeVolatility,
      bestDay,
      worstDay
    };
  }

  /**
   * Compare fee generation across protocols
   */
  compareProtocolFees(params: {
    position: number;
    protocols: Array<{
      name: string;
      tvl: number;
      volume24h: number;
      feeRate: number;
    }>;
  }): Array<{
    protocol: string;
    expectedDailyFees: number;
    expectedAPR: number;
    efficiency: number;
  }> {
    const results = [];
    
    for (const protocol of params.protocols) {
      const poolShare = params.position / protocol.tvl;
      const dailyFees = protocol.volume24h * (protocol.feeRate / 100) * poolShare;
      const annualFees = dailyFees * 365;
      const apr = (annualFees / params.position) * 100;
      const efficiency = protocol.volume24h / protocol.tvl; // Volume/TVL ratio
      
      results.push({
        protocol: protocol.name,
        expectedDailyFees: dailyFees,
        expectedAPR: apr,
        efficiency
      });
    }
    
    // Sort by APR
    results.sort((a, b) => b.expectedAPR - a.expectedAPR);
    
    return results;
  }
}