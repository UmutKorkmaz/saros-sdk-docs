/**
 * Price calculation utilities
 * 
 * Handles price calculations, swap outputs, and
 * price impact estimations.
 */

import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { PoolData } from './pools';
import { logger } from './logger';

export interface SwapCalculation {
  amountOut: BN;
  priceImpact: number;
  fee: BN;
  price: number;
  slippage?: number;
}

export class PriceCalculator {
  private priceCache: Map<string, { price: number; timestamp: number }> = new Map();
  private cacheTimeout = 5000; // 5 seconds

  constructor() {}

  /**
   * Calculate swap output for given input
   */
  async calculateSwapOutput(params: {
    pool: PoolData;
    amountIn: BN;
    fromMint: PublicKey;
    toMint: PublicKey;
  }): Promise<SwapCalculation | null> {
    try {
      const { pool, amountIn, fromMint, toMint } = params;
      
      // Determine if we're swapping A→B or B→A
      const isForward = pool.tokenA.equals(fromMint) && pool.tokenB.equals(toMint);
      const isReverse = pool.tokenB.equals(fromMint) && pool.tokenA.equals(toMint);
      
      if (!isForward && !isReverse) {
        logger.error('Token pair does not match pool');
        return null;
      }
      
      // Use constant product formula (simplified)
      // In production, would use actual Saros math
      const reserveIn = pool.liquidity.div(new BN(2)); // Simplified reserve
      const reserveOut = pool.liquidity.div(new BN(2));
      
      // Calculate fee
      const feeAmount = amountIn.mul(new BN(pool.fee * 100)).div(new BN(10000));
      const amountInAfterFee = amountIn.sub(feeAmount);
      
      // x * y = k
      // Calculate output using constant product
      const numerator = amountInAfterFee.mul(reserveOut);
      const denominator = reserveIn.add(amountInAfterFee);
      const amountOut = numerator.div(denominator);
      
      // Calculate price impact
      const priceImpact = this.calculatePriceImpact(
        amountIn,
        amountOut,
        reserveIn,
        reserveOut
      );
      
      // Calculate effective price
      const price = amountOut.toNumber() / amountIn.toNumber();
      
      return {
        amountOut,
        priceImpact,
        fee: feeAmount,
        price
      };
      
    } catch (error) {
      logger.error('Error calculating swap output:', error);
      return null;
    }
  }

  /**
   * Calculate price impact
   */
  private calculatePriceImpact(
    amountIn: BN,
    amountOut: BN,
    reserveIn: BN,
    reserveOut: BN
  ): number {
    // Initial price
    const initialPrice = reserveOut.toNumber() / reserveIn.toNumber();
    
    // Price after swap
    const newReserveIn = reserveIn.add(amountIn);
    const newReserveOut = reserveOut.sub(amountOut);
    const newPrice = newReserveOut.toNumber() / newReserveIn.toNumber();
    
    // Price impact percentage
    const impact = Math.abs((newPrice - initialPrice) / initialPrice) * 100;
    
    return impact;
  }

  /**
   * Get current price for token pair
   */
  async getPrice(
    pool: PoolData,
    tokenA: PublicKey,
    tokenB: PublicKey
  ): Promise<number | null> {
    try {
      const cacheKey = `${pool.address.toString()}-${tokenA.toString()}-${tokenB.toString()}`;
      const cached = this.priceCache.get(cacheKey);
      
      // Return cached price if still valid
      if (cached && Date.now() - cached.timestamp < this.cacheTimeout) {
        return cached.price;
      }
      
      // Calculate price from pool reserves
      // Simplified - in production would fetch actual reserves
      const reserveA = pool.liquidity.div(new BN(2));
      const reserveB = pool.liquidity.div(new BN(2));
      
      let price: number;
      
      if (pool.tokenA.equals(tokenA) && pool.tokenB.equals(tokenB)) {
        price = reserveB.toNumber() / reserveA.toNumber();
      } else if (pool.tokenB.equals(tokenA) && pool.tokenA.equals(tokenB)) {
        price = reserveA.toNumber() / reserveB.toNumber();
      } else {
        return null;
      }
      
      // Cache the price
      this.priceCache.set(cacheKey, {
        price,
        timestamp: Date.now()
      });
      
      return price;
      
    } catch (error) {
      logger.error('Error getting price:', error);
      return null;
    }
  }

  /**
   * Calculate minimum amount out with slippage
   */
  calculateMinimumAmountOut(
    expectedAmount: BN,
    slippageBps: number
  ): BN {
    const slippageFactor = 10000 - slippageBps;
    return expectedAmount.mul(new BN(slippageFactor)).div(new BN(10000));
  }

  /**
   * Calculate maximum amount in with slippage
   */
  calculateMaximumAmountIn(
    expectedAmount: BN,
    slippageBps: number
  ): BN {
    const slippageFactor = 10000 + slippageBps;
    return expectedAmount.mul(new BN(slippageFactor)).div(new BN(10000));
  }

  /**
   * Estimate gas cost for swap
   */
  estimateGasCost(hops: number, priorityFee: number = 10000): BN {
    // Base cost + per hop cost
    const baseCost = 5000; // 0.000005 SOL
    const perHopCost = 200000; // 0.0002 SOL
    
    const totalUnits = baseCost + (perHopCost * hops);
    const totalCost = totalUnits + priorityFee;
    
    return new BN(totalCost);
  }

  /**
   * Calculate APY from fees and volume
   */
  calculateAPY(
    totalFees24h: BN,
    totalLiquidity: BN
  ): number {
    if (totalLiquidity.isZero()) {
      return 0;
    }
    
    // Daily return
    const dailyReturn = totalFees24h.mul(new BN(10000)).div(totalLiquidity).toNumber() / 10000;
    
    // Annualized (compound daily)
    const apy = (Math.pow(1 + dailyReturn, 365) - 1) * 100;
    
    return apy;
  }

  /**
   * Calculate impermanent loss
   */
  calculateImpermanentLoss(
    initialPriceRatio: number,
    currentPriceRatio: number
  ): number {
    const priceRatio = currentPriceRatio / initialPriceRatio;
    const il = 2 * Math.sqrt(priceRatio) / (1 + priceRatio) - 1;
    
    return Math.abs(il) * 100; // Return as percentage
  }

  /**
   * Find arbitrage opportunity between pools
   */
  findArbitrageSpread(
    buyPrice: number,
    sellPrice: number,
    buyFee: number,
    sellFee: number
  ): number | null {
    // Account for fees
    const effectiveBuyPrice = buyPrice * (1 + buyFee / 10000);
    const effectiveSellPrice = sellPrice * (1 - sellFee / 10000);
    
    // Calculate spread
    const spread = ((effectiveSellPrice - effectiveBuyPrice) / effectiveBuyPrice) * 10000; // bps
    
    // Only profitable if positive after fees
    return spread > 0 ? spread : null;
  }

  /**
   * Calculate optimal split for large trades
   */
  calculateOptimalSplit(
    amount: BN,
    pools: PoolData[]
  ): Array<{ pool: PoolData; amount: BN; impact: number }> {
    // Simplified split calculation
    // In production would use convex optimization
    
    const splits: Array<{ pool: PoolData; amount: BN; impact: number }> = [];
    const poolCount = pools.length;
    
    // Sort pools by liquidity
    const sortedPools = [...pools].sort((a, b) => 
      b.liquidity.sub(a.liquidity).toNumber()
    );
    
    // Allocate proportionally to liquidity
    const totalLiquidity = pools.reduce((sum, p) => sum.add(p.liquidity), new BN(0));
    
    for (const pool of sortedPools) {
      const proportion = pool.liquidity.mul(new BN(10000)).div(totalLiquidity).toNumber() / 10000;
      const splitAmount = amount.mul(new BN(Math.floor(proportion * 10000))).div(new BN(10000));
      
      // Estimate impact for this amount
      const impact = this.estimatePriceImpact(splitAmount, pool.liquidity);
      
      splits.push({
        pool,
        amount: splitAmount,
        impact
      });
    }
    
    return splits;
  }

  /**
   * Estimate price impact based on trade size
   */
  private estimatePriceImpact(tradeSize: BN, liquidity: BN): number {
    // Simplified impact calculation
    const ratio = tradeSize.mul(new BN(10000)).div(liquidity).toNumber() / 10000;
    
    // Quadratic impact model
    return ratio * ratio * 100;
  }

  /**
   * Get price statistics
   */
  getStatistics(): any {
    return {
      cachedPrices: this.priceCache.size,
      cacheTimeout: this.cacheTimeout
    };
  }
}