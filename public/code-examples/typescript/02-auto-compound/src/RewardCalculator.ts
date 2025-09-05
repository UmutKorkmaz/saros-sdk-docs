/**
 * RewardCalculator - APY and reward calculations
 */

import { PublicKey } from '@solana/web3.js';
import Decimal from 'decimal.js';
import { logger } from './utils/logger';

export interface Position {
  poolAddress: PublicKey;
  stakedAmount: number;
  rewards: number;
  timeStaked: number;
  lastHarvest?: number;
}

export interface YieldAnalysis {
  stakedValue: number;
  pendingRewards: number;
  dailyRate: number;
  weeklyRate: number;
  monthlyRate: number;
  currentAPY: number;
  compoundAPY: number;
  effectiveAPY: number;
  impermanentLoss?: number;
}

export interface OptimalFrequency {
  hours: number;
  compoundsPerYear: number;
  apyIncrease: number;
  dailyGasCost: number;
  breakEvenThreshold: number;
  netAPY: number;
}

export interface CompoundComparison {
  noCompound: {
    finalValue: number;
    totalRewards: number;
    apy: number;
  };
  dailyCompound: {
    finalValue: number;
    totalRewards: number;
    apy: number;
    gasCost: number;
  };
  weeklyCompound: {
    finalValue: number;
    totalRewards: number;
    apy: number;
    gasCost: number;
  };
  optimalCompound: {
    finalValue: number;
    totalRewards: number;
    apy: number;
    gasCost: number;
    frequency: string;
  };
}

export class RewardCalculator {
  private readonly SECONDS_PER_YEAR = 31536000;
  private readonly DAYS_PER_YEAR = 365;
  private tokenPrices: Map<string, number> = new Map();

  constructor() {
    // Initialize with mock prices
    this.initializePrices();
    logger.info('RewardCalculator initialized');
  }

  /**
   * Analyze position yields
   */
  async analyzePosition(position: Position): Promise<YieldAnalysis> {
    const timeStakedDays = (Date.now() - position.timeStaked) / 86400000;
    const timeStakedHours = timeStakedDays * 24;
    
    // Calculate rates
    const totalReturn = position.rewards / position.stakedAmount;
    const dailyRate = timeStakedDays > 0 ? (totalReturn / timeStakedDays) * 100 : 0;
    const weeklyRate = dailyRate * 7;
    const monthlyRate = dailyRate * 30;
    
    // Calculate APY
    const currentAPY = dailyRate * 365;
    
    // Calculate compound APY with daily compounding
    const dailyRateDecimal = dailyRate / 100;
    const compoundAPY = (Math.pow(1 + dailyRateDecimal, 365) - 1) * 100;
    
    // Calculate effective APY (accounting for gas costs)
    const gasPerCompound = 0.001; // SOL
    const solPrice = 50; // USD
    const compoundsPerYear = 365;
    const yearlyGasCost = gasPerCompound * solPrice * compoundsPerYear;
    const effectiveReturns = (position.stakedAmount * compoundAPY / 100) - yearlyGasCost;
    const effectiveAPY = (effectiveReturns / position.stakedAmount) * 100;
    
    // Estimate impermanent loss for LP positions
    const impermanentLoss = await this.estimateImpermanentLoss(position);

    return {
      stakedValue: position.stakedAmount,
      pendingRewards: position.rewards,
      dailyRate,
      weeklyRate,
      monthlyRate,
      currentAPY,
      compoundAPY,
      effectiveAPY,
      impermanentLoss
    };
  }

  /**
   * Calculate optimal compound frequency
   */
  calculateOptimalFrequency(params: {
    dailyRewards: number;
    positionSize: number;
    gasPrice: number;
    currentAPY: number;
  }): OptimalFrequency {
    const { dailyRewards, positionSize, gasPrice, currentAPY } = params;
    
    // Test different frequencies
    const frequencies = [1, 2, 4, 6, 8, 12, 24, 48, 72, 168]; // hours
    let optimal = { hours: 24, netAPY: 0, apyIncrease: 0 };
    
    for (const hours of frequencies) {
      const compoundsPerYear = (365 * 24) / hours;
      const yearlyGasCost = gasPrice * compoundsPerYear;
      
      // Calculate APY with this frequency
      const periodsPerYear = 365 * 24 / hours;
      const periodRate = (currentAPY / 100) / periodsPerYear;
      const compoundAPY = (Math.pow(1 + periodRate, periodsPerYear) - 1) * 100;
      
      // Calculate net APY after gas
      const yearlyReturns = positionSize * (compoundAPY / 100);
      const netReturns = yearlyReturns - yearlyGasCost;
      const netAPY = (netReturns / positionSize) * 100;
      
      if (netAPY > optimal.netAPY) {
        optimal = {
          hours,
          netAPY,
          apyIncrease: compoundAPY - currentAPY
        };
      }
    }
    
    // Calculate break-even threshold
    const minRewardsPerCompound = gasPrice;
    const breakEvenThreshold = minRewardsPerCompound * (optimal.hours / 24) * (100 / dailyRewards);
    
    return {
      hours: optimal.hours,
      compoundsPerYear: (365 * 24) / optimal.hours,
      apyIncrease: optimal.apyIncrease,
      dailyGasCost: gasPrice * (24 / optimal.hours),
      breakEvenThreshold,
      netAPY: optimal.netAPY
    };
  }

  /**
   * Compare different compound strategies
   */
  compareCompoundStrategies(
    principal: number,
    apy: number,
    gasPrice: number,
    timeYears: number = 1
  ): CompoundComparison {
    const dailyRate = apy / 365 / 100;
    const yearlyGasUSD = gasPrice * 50; // Assume SOL = $50
    
    // No compound
    const noCompound = {
      finalValue: principal * (1 + apy / 100 * timeYears),
      totalRewards: principal * apy / 100 * timeYears,
      apy: apy
    };
    
    // Daily compound
    const dailyCompounds = 365 * timeYears;
    const dailyCompound = {
      finalValue: principal * Math.pow(1 + dailyRate, dailyCompounds),
      totalRewards: 0,
      apy: 0,
      gasCost: yearlyGasUSD * dailyCompounds / 365 * timeYears
    };
    dailyCompound.totalRewards = dailyCompound.finalValue - principal;
    dailyCompound.apy = ((dailyCompound.finalValue - dailyCompound.gasCost - principal) / principal) * 100 / timeYears;
    
    // Weekly compound
    const weeklyCompounds = 52 * timeYears;
    const weeklyRate = apy / 52 / 100;
    const weeklyCompound = {
      finalValue: principal * Math.pow(1 + weeklyRate, weeklyCompounds),
      totalRewards: 0,
      apy: 0,
      gasCost: yearlyGasUSD * weeklyCompounds / 52 * timeYears
    };
    weeklyCompound.totalRewards = weeklyCompound.finalValue - principal;
    weeklyCompound.apy = ((weeklyCompound.finalValue - weeklyCompound.gasCost - principal) / principal) * 100 / timeYears;
    
    // Optimal compound
    const optimalFreq = this.calculateOptimalFrequency({
      dailyRewards: principal * dailyRate,
      positionSize: principal,
      gasPrice: gasPrice,
      currentAPY: apy
    });
    
    const optimalCompounds = optimalFreq.compoundsPerYear * timeYears;
    const optimalRate = apy / optimalFreq.compoundsPerYear / 100;
    const optimalCompound = {
      finalValue: principal * Math.pow(1 + optimalRate, optimalCompounds),
      totalRewards: 0,
      apy: 0,
      gasCost: yearlyGasUSD * optimalCompounds / optimalFreq.compoundsPerYear * timeYears,
      frequency: `Every ${optimalFreq.hours} hours`
    };
    optimalCompound.totalRewards = optimalCompound.finalValue - principal;
    optimalCompound.apy = ((optimalCompound.finalValue - optimalCompound.gasCost - principal) / principal) * 100 / timeYears;
    
    return {
      noCompound,
      dailyCompound,
      weeklyCompound,
      optimalCompound
    };
  }

  /**
   * Calculate compound interest
   */
  calculateCompoundInterest(
    principal: number,
    rate: number,
    compoundsPerYear: number,
    years: number
  ): number {
    const r = rate / 100;
    const n = compoundsPerYear;
    const t = years;
    
    return principal * Math.pow(1 + r/n, n*t);
  }

  /**
   * Calculate APY from APR
   */
  aprToApy(apr: number, compoundsPerYear: number): number {
    const r = apr / 100;
    const n = compoundsPerYear;
    
    return (Math.pow(1 + r/n, n) - 1) * 100;
  }

  /**
   * Calculate APR from APY
   */
  apyToApr(apy: number, compoundsPerYear: number): number {
    const y = apy / 100;
    const n = compoundsPerYear;
    
    return n * (Math.pow(1 + y, 1/n) - 1) * 100;
  }

  /**
   * Calculate compound boost
   */
  calculateCompoundBoost(baseAPY: number, compoundsPerYear: number): number {
    const simpleAPY = baseAPY;
    const compoundAPY = this.aprToApy(baseAPY, compoundsPerYear);
    
    return compoundAPY - simpleAPY;
  }

  /**
   * Estimate time to double investment
   */
  calculateDoublingTime(apy: number, compounding: boolean = true): number {
    if (compounding) {
      // Rule of 72 for compound interest
      return 72 / apy;
    } else {
      // Simple interest
      return 100 / apy;
    }
  }

  /**
   * Calculate required APY for target
   */
  calculateRequiredAPY(
    currentValue: number,
    targetValue: number,
    timeYears: number
  ): number {
    const totalReturn = targetValue / currentValue;
    const annualReturn = Math.pow(totalReturn, 1 / timeYears);
    
    return (annualReturn - 1) * 100;
  }

  /**
   * Estimate impermanent loss
   */
  private async estimateImpermanentLoss(position: Position): Promise<number> {
    // Simplified IL calculation
    // In production, fetch actual price changes
    
    const priceRatio = 1.5; // Assume one token increased 50% relative to other
    
    // IL formula: 2 * sqrt(priceRatio) / (1 + priceRatio) - 1
    const il = 2 * Math.sqrt(priceRatio) / (1 + priceRatio) - 1;
    
    return Math.abs(il) * 100; // Return as percentage
  }

  /**
   * Calculate real yield (inflation-adjusted)
   */
  calculateRealYield(nominalAPY: number, inflationRate: number = 2): number {
    return ((1 + nominalAPY/100) / (1 + inflationRate/100) - 1) * 100;
  }

  /**
   * Calculate yield on cost
   */
  calculateYieldOnCost(
    initialInvestment: number,
    currentRewards: number,
    timeYears: number
  ): number {
    const annualRewards = currentRewards / timeYears;
    return (annualRewards / initialInvestment) * 100;
  }

  /**
   * Project future value
   */
  projectFutureValue(
    currentValue: number,
    apy: number,
    years: number,
    monthlyAddition: number = 0
  ): {
    futureValue: number;
    totalContributed: number;
    totalEarnings: number;
  } {
    const monthlyRate = apy / 12 / 100;
    const months = years * 12;
    
    // Future value with compound interest and regular additions
    let futureValue = currentValue;
    for (let i = 0; i < months; i++) {
      futureValue = futureValue * (1 + monthlyRate) + monthlyAddition;
    }
    
    const totalContributed = currentValue + (monthlyAddition * months);
    const totalEarnings = futureValue - totalContributed;
    
    return {
      futureValue,
      totalContributed,
      totalEarnings
    };
  }

  /**
   * Calculate tax-adjusted returns
   */
  calculateAfterTaxAPY(
    apy: number,
    taxRate: number,
    taxablePercentage: number = 100
  ): number {
    const taxableAPY = apy * (taxablePercentage / 100);
    const nonTaxableAPY = apy * ((100 - taxablePercentage) / 100);
    const afterTaxAPY = taxableAPY * (1 - taxRate / 100) + nonTaxableAPY;
    
    return afterTaxAPY;
  }

  /**
   * Initialize token prices
   */
  private initializePrices(): void {
    this.tokenPrices.set('USDC', 1.0);
    this.tokenPrices.set('SOL', 50.0);
    this.tokenPrices.set('SAROS', 0.1);
    this.tokenPrices.set('RAY', 2.0);
    this.tokenPrices.set('SRM', 1.5);
  }

  /**
   * Get token price
   */
  getTokenPrice(symbol: string): number {
    return this.tokenPrices.get(symbol) || 0;
  }

  /**
   * Update token price
   */
  updateTokenPrice(symbol: string, price: number): void {
    this.tokenPrices.set(symbol, price);
  }
}