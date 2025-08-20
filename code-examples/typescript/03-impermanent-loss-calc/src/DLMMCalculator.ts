/**
 * DLMMCalculator - Specialized IL calculations for DLMM (Dynamic Liquidity Market Maker) pools
 */

import Decimal from 'decimal.js';
import { logger } from './utils/logger';

export interface DLMMPosition {
  lowerPrice: number;
  upperPrice: number;
  currentPrice: number;
  initialPrice: number;
  liquidity: number;
  binStep: number;        // DLMM bin width in basis points
  activeBin: number;       // Current active bin ID
}

export interface ConcentratedILResult {
  impermanentLoss: number;
  fullRangeIL: number;           // IL if position was full range
  concentrationFactor: number;    // How concentrated the position is
  inRange: boolean;               // Whether position is in range
  tokenXAmount: number;           // Current token X amount
  tokenYAmount: number;           // Current token Y amount
  effectiveIL: number;            // IL accounting for concentration
  rangeUtilization: number;       // Percentage of range being utilized
}

export interface OptimalRangeParams {
  token0: string;
  token1: string;
  currentPrice: number;
  volatility: number;     // Annual volatility percentage
  targetAPR: number;      // Target annual percentage return
  maxIL: number;         // Maximum acceptable IL
  capital: number;       // Capital to deploy
  timeHorizon: number;   // Investment horizon in days
}

export interface OptimalRangeResult {
  lowerPrice: number;
  upperPrice: number;
  rangeWidth: number;            // Percentage width
  expectedAPR: number;
  expectedIL: number;
  capitalEfficiency: number;     // Multiplier vs full range
  probabilityInRange: number;    // Probability of staying in range
  expectedFees: number;
  breakEvenDays: number;
}

export interface BinAnalysis {
  binId: number;
  priceRange: [number, number];
  liquidity: number;
  composition: { tokenX: number; tokenY: number };
  il: number;
  fees: number;
}

export class DLMMCalculator {
  private decimal: typeof Decimal;
  private readonly BASIS_POINT_MAX = 10000;

  constructor() {
    this.decimal = Decimal;
    this.decimal.set({ precision: 20, rounding: 4 });
    logger.info('DLMMCalculator initialized');
  }

  /**
   * Calculate IL for concentrated liquidity position in DLMM
   */
  async calculateConcentratedIL(position: DLMMPosition): Promise<ConcentratedILResult> {
    const {
      lowerPrice,
      upperPrice,
      currentPrice,
      initialPrice,
      liquidity,
      binStep
    } = position;

    // Check if position is in range
    const inRange = currentPrice >= lowerPrice && currentPrice <= upperPrice;

    // Calculate concentration factor
    const fullRange = Number.MAX_SAFE_INTEGER; // Theoretical full range
    const positionRange = upperPrice - lowerPrice;
    const concentrationFactor = Math.sqrt(fullRange / positionRange);

    // Calculate price ratio
    const priceRatio = currentPrice / initialPrice;

    // Calculate IL for full range position
    const fullRangeIL = this.calculateFullRangeIL(priceRatio);

    // Calculate concentrated IL
    let impermanentLoss: number;
    let tokenXAmount: number;
    let tokenYAmount: number;

    if (!inRange) {
      // Position is out of range
      if (currentPrice < lowerPrice) {
        // All liquidity is in token Y
        impermanentLoss = ((lowerPrice / initialPrice) - 1) * 100;
        tokenXAmount = 0;
        tokenYAmount = liquidity;
      } else {
        // All liquidity is in token X
        impermanentLoss = ((currentPrice / initialPrice) - 1) * 100;
        tokenXAmount = liquidity / currentPrice;
        tokenYAmount = 0;
      }
    } else {
      // Position is in range - calculate using DLMM formula
      const sqrtPriceCurrent = Math.sqrt(currentPrice);
      const sqrtPriceInitial = Math.sqrt(initialPrice);
      const sqrtPriceLower = Math.sqrt(lowerPrice);
      const sqrtPriceUpper = Math.sqrt(upperPrice);

      // Calculate token amounts using DLMM liquidity math
      tokenXAmount = liquidity * (sqrtPriceUpper - sqrtPriceCurrent) / 
                     (sqrtPriceCurrent * sqrtPriceUpper);
      tokenYAmount = liquidity * (sqrtPriceCurrent - sqrtPriceLower);

      // Calculate IL with concentration adjustment
      const baseIL = fullRangeIL;
      const rangeAdjustment = this.calculateRangeAdjustment(
        lowerPrice,
        upperPrice,
        initialPrice,
        currentPrice
      );

      impermanentLoss = baseIL * concentrationFactor * rangeAdjustment;
    }

    // Calculate effective IL (considering capital efficiency)
    const effectiveIL = impermanentLoss / concentrationFactor;

    // Calculate range utilization
    const priceMovement = Math.abs(currentPrice - initialPrice);
    const maxMovement = Math.max(
      Math.abs(upperPrice - initialPrice),
      Math.abs(initialPrice - lowerPrice)
    );
    const rangeUtilization = (priceMovement / maxMovement) * 100;

    return {
      impermanentLoss: Math.abs(impermanentLoss),
      fullRangeIL: Math.abs(fullRangeIL),
      concentrationFactor,
      inRange,
      tokenXAmount,
      tokenYAmount,
      effectiveIL: Math.abs(effectiveIL),
      rangeUtilization
    };
  }

  /**
   * Find optimal price range for DLMM position
   */
  async findOptimalRange(params: OptimalRangeParams): Promise<OptimalRangeResult> {
    const {
      currentPrice,
      volatility,
      targetAPR,
      maxIL,
      capital,
      timeHorizon
    } = params;

    // Calculate optimal range based on volatility and time horizon
    const timeAdjustedVolatility = volatility * Math.sqrt(timeHorizon / 365);
    
    // Use 2-sigma range for ~95% probability of staying in range
    const rangeMultiplier = Math.exp(2 * timeAdjustedVolatility / 100);
    
    let lowerPrice = currentPrice / rangeMultiplier;
    let upperPrice = currentPrice * rangeMultiplier;

    // Adjust range based on IL tolerance
    const estimatedIL = this.estimateILForRange(
      lowerPrice,
      upperPrice,
      currentPrice,
      volatility
    );

    if (estimatedIL > maxIL) {
      // Widen range to reduce IL
      const adjustmentFactor = Math.sqrt(estimatedIL / maxIL);
      lowerPrice = currentPrice / (rangeMultiplier * adjustmentFactor);
      upperPrice = currentPrice * (rangeMultiplier * adjustmentFactor);
    }

    // Calculate expected returns
    const rangeWidth = ((upperPrice - lowerPrice) / currentPrice) * 100;
    const capitalEfficiency = this.calculateCapitalEfficiency(
      lowerPrice,
      upperPrice,
      currentPrice
    );

    // Estimate fees based on capital efficiency
    const baseFeeAPR = 20; // Base APR for full range
    const expectedAPR = baseFeeAPR * capitalEfficiency;
    const expectedFees = (capital * expectedAPR / 100) * (timeHorizon / 365);

    // Calculate probability of staying in range
    const probabilityInRange = this.calculateRangeProbability(
      lowerPrice,
      upperPrice,
      currentPrice,
      volatility,
      timeHorizon
    );

    // Calculate break-even days
    const dailyFees = expectedFees / timeHorizon;
    const ilInUSD = capital * (estimatedIL / 100);
    const breakEvenDays = ilInUSD / dailyFees;

    return {
      lowerPrice,
      upperPrice,
      rangeWidth,
      expectedAPR: Math.min(expectedAPR, targetAPR * 1.5), // Cap at 150% of target
      expectedIL: estimatedIL,
      capitalEfficiency,
      probabilityInRange,
      expectedFees,
      breakEvenDays
    };
  }

  /**
   * Analyze IL across DLMM bins
   */
  analyzeBins(params: {
    poolAddress: string;
    binStep: number;
    activeBin: number;
    numBins: number;
    currentPrice: number;
    liquidity: number[];  // Liquidity per bin
  }): BinAnalysis[] {
    const { binStep, activeBin, numBins, currentPrice, liquidity } = params;
    const analysis: BinAnalysis[] = [];

    // Calculate price per bin
    const binWidth = binStep / this.BASIS_POINT_MAX;
    
    for (let i = 0; i < numBins; i++) {
      const binId = activeBin - Math.floor(numBins / 2) + i;
      const binPrice = currentPrice * Math.pow(1 + binWidth, binId - activeBin);
      
      // Calculate price range for this bin
      const lowerPrice = binPrice * (1 - binWidth / 2);
      const upperPrice = binPrice * (1 + binWidth / 2);
      
      // Calculate token composition in bin
      let tokenX = 0;
      let tokenY = 0;
      
      if (binId < activeBin) {
        // Below current price - all Y
        tokenY = liquidity[i] || 0;
      } else if (binId > activeBin) {
        // Above current price - all X
        tokenX = (liquidity[i] || 0) / binPrice;
      } else {
        // Active bin - mixed composition
        tokenX = (liquidity[i] || 0) * 0.5 / binPrice;
        tokenY = (liquidity[i] || 0) * 0.5;
      }
      
      // Calculate IL for this bin
      const priceRatio = binPrice / currentPrice;
      const binIL = this.calculateFullRangeIL(priceRatio);
      
      // Estimate fees for this bin (simplified)
      const binFees = (liquidity[i] || 0) * 0.003 * 30; // 0.3% fee * 30 days
      
      analysis.push({
        binId,
        priceRange: [lowerPrice, upperPrice],
        liquidity: liquidity[i] || 0,
        composition: { tokenX, tokenY },
        il: Math.abs(binIL),
        fees: binFees
      });
    }

    return analysis;
  }

  /**
   * Calculate IL impact of bin liquidity shape
   */
  calculateShapeImpact(params: {
    shape: 'uniform' | 'normal' | 'bidirectional' | 'spot';
    currentPrice: number;
    rangeWidth: number;
    priceChange: number;
  }): {
    shapeIL: number;
    uniformIL: number;
    ilReduction: number;
    optimalShape: string;
  } {
    const { shape, currentPrice, rangeWidth, priceChange } = params;
    
    // Calculate IL for uniform distribution
    const uniformIL = this.calculateFullRangeIL(priceChange);
    
    // Calculate IL based on liquidity shape
    let shapeIL: number;
    
    switch (shape) {
      case 'uniform':
        // Even distribution across range
        shapeIL = uniformIL;
        break;
        
      case 'normal':
        // Concentrated around current price
        shapeIL = uniformIL * 1.5; // Higher IL due to concentration
        break;
        
      case 'bidirectional':
        // Concentrated at range edges
        shapeIL = uniformIL * 0.7; // Lower IL, less efficient
        break;
        
      case 'spot':
        // Highly concentrated at current price
        shapeIL = uniformIL * 2.5; // Highest IL
        break;
        
      default:
        shapeIL = uniformIL;
    }
    
    // Determine optimal shape based on expected volatility
    let optimalShape = 'uniform';
    if (priceChange < 1.1) {
      optimalShape = 'normal'; // Low volatility - concentrate around price
    } else if (priceChange > 1.5) {
      optimalShape = 'bidirectional'; // High volatility - protect edges
    }
    
    const ilReduction = ((uniformIL - shapeIL) / uniformIL) * 100;
    
    return {
      shapeIL: Math.abs(shapeIL),
      uniformIL: Math.abs(uniformIL),
      ilReduction,
      optimalShape
    };
  }

  /**
   * Calculate rebalancing strategy for DLMM position
   */
  calculateRebalanceStrategy(params: {
    position: DLMMPosition;
    targetRange: [number, number];
    gasEstimate: number;
    currentFees: number;
  }): {
    shouldRebalance: boolean;
    estimatedCost: number;
    estimatedBenefit: number;
    newIL: number;
    breakEvenTime: number;
    recommendation: string;
  } {
    const { position, targetRange, gasEstimate, currentFees } = params;
    const [newLower, newUpper] = targetRange;
    
    // Calculate current IL
    const currentILResult = this.calculateConcentratedIL(position);
    
    // Calculate new IL with rebalanced position
    const newPosition: DLMMPosition = {
      ...position,
      lowerPrice: newLower,
      upperPrice: newUpper,
      initialPrice: position.currentPrice // Reset to current price
    };
    
    const newILResult = this.calculateConcentratedIL(newPosition);
    
    // Calculate costs and benefits
    const rebalanceCost = gasEstimate + (position.liquidity * 0.003); // Gas + 0.3% swap fee
    const ilSaved = (currentILResult.impermanentLoss - newILResult.impermanentLoss) * 
                    position.liquidity / 100;
    
    // Estimate increased fee generation from better capital efficiency
    const oldEfficiency = currentILResult.inRange ? currentILResult.concentrationFactor : 0;
    const newEfficiency = newILResult.concentrationFactor;
    const feeIncrease = currentFees * (newEfficiency / oldEfficiency - 1);
    
    const totalBenefit = ilSaved + feeIncrease * 30; // 30 day benefit
    const breakEvenTime = rebalanceCost / (feeIncrease || 1);
    
    // Determine recommendation
    let recommendation = '';
    const shouldRebalance = totalBenefit > rebalanceCost;
    
    if (shouldRebalance && breakEvenTime < 7) {
      recommendation = 'Strongly recommended - quick payback';
    } else if (shouldRebalance && breakEvenTime < 30) {
      recommendation = 'Recommended - reasonable payback period';
    } else if (!shouldRebalance) {
      recommendation = 'Not recommended - costs exceed benefits';
    } else {
      recommendation = 'Consider waiting for better conditions';
    }
    
    return {
      shouldRebalance,
      estimatedCost: rebalanceCost,
      estimatedBenefit: totalBenefit,
      newIL: newILResult.impermanentLoss,
      breakEvenTime,
      recommendation
    };
  }

  /**
   * Calculate full range IL (standard AMM formula)
   */
  private calculateFullRangeIL(priceRatio: number): number {
    const sqrtRatio = Math.sqrt(priceRatio);
    return (2 * sqrtRatio / (1 + priceRatio) - 1) * 100;
  }

  /**
   * Calculate range adjustment factor for concentrated IL
   */
  private calculateRangeAdjustment(
    lowerPrice: number,
    upperPrice: number,
    initialPrice: number,
    currentPrice: number
  ): number {
    // Calculate how much of the price movement is within range
    const totalMovement = Math.abs(currentPrice - initialPrice);
    const rangeSize = upperPrice - lowerPrice;
    
    // Adjustment factor based on position within range
    const positionInRange = (currentPrice - lowerPrice) / rangeSize;
    const centeredness = Math.abs(positionInRange - 0.5) * 2;
    
    // More IL when position moves away from center
    return 1 + centeredness * 0.5;
  }

  /**
   * Estimate IL for a given range
   */
  private estimateILForRange(
    lowerPrice: number,
    upperPrice: number,
    currentPrice: number,
    volatility: number
  ): number {
    // Estimate maximum price movement within range
    const rangeWidth = (upperPrice - lowerPrice) / currentPrice;
    const expectedMovement = volatility / 100;
    
    // IL increases with tighter ranges
    const concentrationFactor = 1 / rangeWidth;
    const baseIL = 2; // Base IL for moderate movement
    
    return baseIL * concentrationFactor * expectedMovement;
  }

  /**
   * Calculate capital efficiency for concentrated position
   */
  private calculateCapitalEfficiency(
    lowerPrice: number,
    upperPrice: number,
    currentPrice: number
  ): number {
    // Simplified calculation based on range width
    const fullRange = 1000; // Arbitrary full range
    const positionRange = (upperPrice - lowerPrice) / currentPrice;
    
    return Math.min(fullRange / positionRange, 100); // Cap at 100x
  }

  /**
   * Calculate probability of price staying within range
   */
  private calculateRangeProbability(
    lowerPrice: number,
    upperPrice: number,
    currentPrice: number,
    volatility: number,
    timeHorizon: number
  ): number {
    // Use log-normal distribution for price movements
    const timeAdjustedVol = volatility * Math.sqrt(timeHorizon / 365) / 100;
    
    const lowerBound = Math.log(lowerPrice / currentPrice);
    const upperBound = Math.log(upperPrice / currentPrice);
    
    // Calculate cumulative probabilities
    const lowerProb = this.normalCDF(lowerBound / timeAdjustedVol);
    const upperProb = this.normalCDF(upperBound / timeAdjustedVol);
    
    return (upperProb - lowerProb) * 100;
  }

  /**
   * Normal cumulative distribution function
   */
  private normalCDF(x: number): number {
    const a1 = 0.254829592;
    const a2 = -0.284496736;
    const a3 = 1.421413741;
    const a4 = -1.453152027;
    const a5 = 1.061405429;
    const p = 0.3275911;
    
    const sign = x < 0 ? -1 : 1;
    x = Math.abs(x) / Math.sqrt(2);
    
    const t = 1 / (1 + p * x);
    const t2 = t * t;
    const t3 = t2 * t;
    const t4 = t3 * t;
    const t5 = t4 * t;
    
    const y = 1 - ((((a5 * t5 + a4 * t4) + a3 * t3) + a2 * t2) + a1 * t) * 
              Math.exp(-x * x);
    
    return 0.5 * (1 + sign * y);
  }
}