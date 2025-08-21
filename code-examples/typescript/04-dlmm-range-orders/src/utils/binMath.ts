/**
 * Bin Math Utilities for DLMM
 * 
 * Functions for converting between prices and bin IDs,
 * calculating bin steps, and other DLMM-specific math.
 */

import BN from 'bn.js';

const BASIS_POINT_MAX = 10000;

/**
 * Convert price to bin ID
 */
export function priceToBinId(
  price: number,
  binStep: number,
  baseDecimals: number
): number {
  // Simplified calculation - actual implementation would be more complex
  const basePrice = 1; // Reference price
  const priceRatio = price / basePrice;
  const binStepBps = binStep / BASIS_POINT_MAX;
  
  return Math.floor(Math.log(priceRatio) / Math.log(1 + binStepBps));
}

/**
 * Convert bin ID to price
 */
export function binIdToPrice(
  binId: number,
  binStep: number,
  baseDecimals: number
): number {
  const basePrice = 1;
  const binStepBps = binStep / BASIS_POINT_MAX;
  
  return basePrice * Math.pow(1 + binStepBps, binId);
}

/**
 * Calculate optimal bin step based on volatility
 */
export function calculateBinStep(volatility: number): number {
  if (volatility < 0.01) return 1;     // 0.01% for stable pairs
  if (volatility < 0.05) return 10;    // 0.10% for low volatility
  if (volatility < 0.10) return 20;    // 0.20% for medium volatility
  if (volatility < 0.20) return 50;    // 0.50% for high volatility
  return 100;                          // 1.00% for very high volatility
}

/**
 * Calculate number of bins for a price range
 */
export function calculateBinRange(
  lowerPrice: number,
  upperPrice: number,
  binStep: number
): number {
  const lowerBin = priceToBinId(lowerPrice, binStep, 9);
  const upperBin = priceToBinId(upperPrice, binStep, 9);
  return upperBin - lowerBin + 1;
}

/**
 * Get price at specific bin offset
 */
export function getPriceAtBinOffset(
  currentPrice: number,
  binOffset: number,
  binStep: number
): number {
  const binStepBps = binStep / BASIS_POINT_MAX;
  return currentPrice * Math.pow(1 + binStepBps, binOffset);
}

/**
 * Calculate liquidity distribution for range
 */
export function calculateLiquidityDistribution(
  totalLiquidity: BN,
  numBins: number,
  distributionType: 'uniform' | 'normal' | 'exponential'
): BN[] {
  const distribution: BN[] = [];
  
  if (distributionType === 'uniform') {
    const liquidityPerBin = totalLiquidity.div(new BN(numBins));
    for (let i = 0; i < numBins; i++) {
      distribution.push(liquidityPerBin);
    }
  } else if (distributionType === 'normal') {
    // Normal distribution centered at middle bin
    const center = Math.floor(numBins / 2);
    const sigma = numBins / 6; // 99.7% within range
    
    let weights: number[] = [];
    let totalWeight = 0;
    
    for (let i = 0; i < numBins; i++) {
      const weight = Math.exp(-Math.pow(i - center, 2) / (2 * sigma * sigma));
      weights.push(weight);
      totalWeight += weight;
    }
    
    for (let i = 0; i < numBins; i++) {
      const normalizedWeight = weights[i] / totalWeight;
      const binLiquidity = totalLiquidity.muln(normalizedWeight);
      distribution.push(binLiquidity);
    }
  } else if (distributionType === 'exponential') {
    // Exponential decay from first bin
    const lambda = 3 / numBins; // Decay rate
    
    let weights: number[] = [];
    let totalWeight = 0;
    
    for (let i = 0; i < numBins; i++) {
      const weight = Math.exp(-lambda * i);
      weights.push(weight);
      totalWeight += weight;
    }
    
    for (let i = 0; i < numBins; i++) {
      const normalizedWeight = weights[i] / totalWeight;
      const binLiquidity = totalLiquidity.muln(normalizedWeight);
      distribution.push(binLiquidity);
    }
  }
  
  return distribution;
}

/**
 * Calculate impermanent loss for concentrated position
 */
export function calculateConcentratedIL(
  lowerPrice: number,
  upperPrice: number,
  currentPrice: number,
  finalPrice: number
): number {
  // Simplified IL calculation for concentrated liquidity
  if (finalPrice < lowerPrice || finalPrice > upperPrice) {
    // Position is out of range
    return 1; // 100% IL (simplified)
  }
  
  const priceRatio = finalPrice / currentPrice;
  const il = 2 * Math.sqrt(priceRatio) / (1 + priceRatio) - 1;
  
  return Math.abs(il);
}

/**
 * Calculate fees earned in a bin
 */
export function calculateBinFees(
  volume: number,
  feeRate: number,
  liquidityShare: number
): number {
  return volume * feeRate * liquidityShare;
}

/**
 * Find active bin for current price
 */
export function findActiveBin(
  price: number,
  binStep: number,
  baseDecimals: number
): number {
  return priceToBinId(price, binStep, baseDecimals);
}

/**
 * Calculate token composition at bin
 */
export function calculateBinComposition(
  binId: number,
  activeBinId: number
): { tokenX: number; tokenY: number } {
  if (binId < activeBinId) {
    // Below active bin - 100% token Y (quote)
    return { tokenX: 0, tokenY: 1 };
  } else if (binId > activeBinId) {
    // Above active bin - 100% token X (base)
    return { tokenX: 1, tokenY: 0 };
  } else {
    // Active bin - 50/50 (simplified)
    return { tokenX: 0.5, tokenY: 0.5 };
  }
}

/**
 * Validate bin ID is within valid range
 */
export function isValidBinId(binId: number): boolean {
  const MAX_BIN_ID = 443636;
  const MIN_BIN_ID = -443636;
  return binId >= MIN_BIN_ID && binId <= MAX_BIN_ID;
}

/**
 * Calculate optimal range for limit order
 */
export function calculateOptimalRange(
  targetPrice: number,
  currentPrice: number,
  orderType: 'BUY' | 'SELL',
  tolerance: number = 0.001
): { lowerPrice: number; upperPrice: number } {
  if (orderType === 'BUY') {
    return {
      lowerPrice: targetPrice * (1 - tolerance),
      upperPrice: targetPrice * (1 + tolerance)
    };
  } else {
    return {
      lowerPrice: targetPrice * (1 - tolerance),
      upperPrice: targetPrice * (1 + tolerance)
    };
  }
}