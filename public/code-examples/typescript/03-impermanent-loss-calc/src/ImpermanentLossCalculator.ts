/**
 * ImpermanentLossCalculator - Core IL calculation logic for AMM pools
 */

import Decimal from 'decimal.js';
import { logger } from './utils/logger';

export interface ILCalculationParams {
  initialPriceRatio: number;  // Initial price of token A / token B
  currentPriceRatio: number;  // Current price of token A / token B
  poolType: 'AMM' | 'STABLE' | 'WEIGHTED';
  initialLiquidity?: number;  // Initial liquidity value in USD
  weights?: [number, number]; // For weighted pools [weightA, weightB]
}

export interface ILResult {
  impermanentLoss: number;    // IL as percentage
  valueIfHeld: number;        // Value if tokens were held
  valueInPool: number;        // Current value in pool
  lossInUSD: number;         // Absolute loss in USD
  tokenAAmount: number;       // Current token A amount
  tokenBAmount: number;       // Current token B amount
  priceChange: number;        // Price change percentage
  breakEvenFees: number;      // Fees needed to break even
}

export interface HistoricalILParams {
  pool: string;
  startDate: Date;
  endDate: Date;
  resolution: '1h' | '4h' | '1d' | '1w';
}

export interface HistoricalILResult {
  dataPoints: Array<{
    timestamp: Date;
    price: number;
    il: number;
    cumulativeFees: number;
  }>;
  averageIL: number;
  maxIL: number;
  minIL: number;
  volatility: number;
  daysAbove5Percent: number;
  totalFees: number;
}

export interface PoolComparisonParams {
  priceChange: number;
  timeHorizon: number;  // days
  liquidityAmount?: number;
}

export interface PoolInfo {
  name: string;
  type: 'AMM' | 'DLMM' | 'STABLE';
  feeRate: number;
  tvl: number;
  volume24h: number;
  binStep?: number;  // For DLMM
}

export interface PoolComparisonResult {
  name: string;
  impermanentLoss: number;
  expectedFees: number;
  netReturn: number;
  riskScore: number;
  recommendation: string;
}

export interface MitigationStrategy {
  name: string;
  action: string;
  expectedReduction: number;
  riskLevel: 'low' | 'medium' | 'high';
  implementation: string;
}

export class ImpermanentLossCalculator {
  private decimal: typeof Decimal;

  constructor() {
    this.decimal = Decimal;
    this.decimal.set({ precision: 20, rounding: 4 });
    logger.info('ImpermanentLossCalculator initialized');
  }

  /**
   * Calculate impermanent loss for given price change
   */
  calculateIL(params: ILCalculationParams): ILResult {
    const {
      initialPriceRatio,
      currentPriceRatio,
      poolType,
      initialLiquidity = 1000,
      weights = [0.5, 0.5]
    } = params;

    let il: Decimal;
    let valueIfHeld: Decimal;
    let valueInPool: Decimal;

    const priceRatio = new Decimal(currentPriceRatio).div(initialPriceRatio);

    switch (poolType) {
      case 'AMM':
        il = this.calculateStandardAMMIL(priceRatio);
        break;
      
      case 'STABLE':
        il = this.calculateStablePoolIL(priceRatio);
        break;
      
      case 'WEIGHTED':
        il = this.calculateWeightedPoolIL(priceRatio, weights);
        break;
      
      default:
        throw new Error(`Unknown pool type: ${poolType}`);
    }

    // Calculate values
    const initialValue = new Decimal(initialLiquidity);
    
    // Value if held: 50% in each token initially
    valueIfHeld = initialValue.mul(new Decimal(1).add(priceRatio)).div(2);
    
    // Value in pool: affected by IL
    valueInPool = valueIfHeld.mul(new Decimal(1).add(il));

    // Calculate token amounts in pool (for 50/50 AMM)
    const k = initialValue.div(2).mul(initialValue.div(2).div(initialPriceRatio));
    const tokenBAmount = Decimal.sqrt(k.mul(currentPriceRatio));
    const tokenAAmount = k.div(tokenBAmount);

    const lossInUSD = valueIfHeld.sub(valueInPool);
    const priceChange = priceRatio.sub(1).mul(100);

    // Calculate break-even fees
    const breakEvenFees = lossInUSD.abs();

    return {
      impermanentLoss: il.mul(100).toNumber(),
      valueIfHeld: valueIfHeld.toNumber(),
      valueInPool: valueInPool.toNumber(),
      lossInUSD: lossInUSD.toNumber(),
      tokenAAmount: tokenAAmount.toNumber(),
      tokenBAmount: tokenBAmount.toNumber(),
      priceChange: priceChange.toNumber(),
      breakEvenFees: breakEvenFees.toNumber()
    };
  }

  /**
   * Calculate IL for standard x*y=k AMM
   */
  private calculateStandardAMMIL(priceRatio: Decimal): Decimal {
    // IL = 2 * sqrt(price_ratio) / (1 + price_ratio) - 1
    const sqrtRatio = Decimal.sqrt(priceRatio);
    const numerator = sqrtRatio.mul(2);
    const denominator = priceRatio.add(1);
    
    return numerator.div(denominator).sub(1);
  }

  /**
   * Calculate IL for stable pools (reduced IL for correlated assets)
   */
  private calculateStablePoolIL(priceRatio: Decimal): Decimal {
    // Stable pools have significantly reduced IL
    // Using StableSwap invariant approximation
    const deviation = priceRatio.sub(1).abs();
    
    // IL is approximately (deviation^2) / 8 for small deviations
    if (deviation.lt(0.1)) {
      return deviation.pow(2).div(8).neg();
    }
    
    // For larger deviations, use modified formula
    return this.calculateStandardAMMIL(priceRatio).mul(0.3); // 30% of standard IL
  }

  /**
   * Calculate IL for weighted pools (e.g., 80/20 pools)
   */
  private calculateWeightedPoolIL(
    priceRatio: Decimal, 
    weights: [number, number]
  ): Decimal {
    const [w1, w2] = weights;
    
    // IL = (price_ratio^w1)^(w1/(w1+w2)) * (1)^(w2/(w1+w2)) / 
    //      ((w1 * price_ratio + w2) / (w1 + w2)) - 1
    
    const totalWeight = w1 + w2;
    const adjustedRatio = priceRatio.pow(w1 / totalWeight);
    const weightedAvg = new Decimal(w1).mul(priceRatio).add(w2).div(totalWeight);
    
    return adjustedRatio.div(weightedAvg).sub(1);
  }

  /**
   * Analyze historical IL for a pool
   */
  async analyzeHistoricalIL(params: HistoricalILParams): Promise<HistoricalILResult> {
    // In production, fetch actual historical price data
    // For demo, generate simulated data
    
    const dataPoints = [];
    const daysDiff = Math.ceil(
      (params.endDate.getTime() - params.startDate.getTime()) / (1000 * 60 * 60 * 24)
    );

    let totalIL = 0;
    let maxIL = 0;
    let minIL = Infinity;
    let daysAbove5 = 0;
    let cumulativeFees = 0;
    const prices: number[] = [];

    // Simulate historical data
    for (let i = 0; i <= daysDiff; i++) {
      const timestamp = new Date(params.startDate.getTime() + i * 24 * 60 * 60 * 1000);
      
      // Simulate price with random walk
      const randomWalk = (Math.random() - 0.5) * 10;
      const price = 50 + randomWalk + Math.sin(i / 10) * 5;
      prices.push(price);
      
      // Calculate IL for this price
      const priceRatio = price / 50; // Assuming initial price of 50
      const ilResult = this.calculateIL({
        initialPriceRatio: 1,
        currentPriceRatio: priceRatio,
        poolType: 'AMM'
      });
      
      const il = Math.abs(ilResult.impermanentLoss);
      totalIL += il;
      maxIL = Math.max(maxIL, il);
      minIL = Math.min(minIL, il);
      
      if (il > 5) daysAbove5++;
      
      // Simulate fee accumulation
      const dailyFees = Math.random() * 10 + 5;
      cumulativeFees += dailyFees;
      
      dataPoints.push({
        timestamp,
        price,
        il,
        cumulativeFees
      });
    }

    // Calculate volatility
    const returns = prices.slice(1).map((price, i) => 
      Math.log(price / prices[i])
    );
    const avgReturn = returns.reduce((sum, r) => sum + r, 0) / returns.length;
    const variance = returns.reduce((sum, r) => 
      sum + Math.pow(r - avgReturn, 2), 0
    ) / returns.length;
    const volatility = Math.sqrt(variance * 252) * 100; // Annualized

    return {
      dataPoints,
      averageIL: totalIL / daysDiff,
      maxIL,
      minIL: minIL === Infinity ? 0 : minIL,
      volatility,
      daysAbove5Percent: daysAbove5,
      totalFees: cumulativeFees
    };
  }

  /**
   * Compare IL across multiple pools
   */
  async comparePools(
    pools: PoolInfo[],
    params: PoolComparisonParams
  ): Promise<PoolComparisonResult[]> {
    const results: PoolComparisonResult[] = [];
    const liquidityAmount = params.liquidityAmount || 10000;

    for (const pool of pools) {
      // Calculate IL based on pool type
      let il: number;
      
      if (pool.type === 'STABLE') {
        // Stable pools have reduced IL
        il = this.calculateIL({
          initialPriceRatio: 1,
          currentPriceRatio: params.priceChange,
          poolType: 'STABLE',
          initialLiquidity: liquidityAmount
        }).impermanentLoss;
      } else if (pool.type === 'DLMM') {
        // DLMM has concentrated IL
        il = this.calculateIL({
          initialPriceRatio: 1,
          currentPriceRatio: params.priceChange,
          poolType: 'AMM',
          initialLiquidity: liquidityAmount
        }).impermanentLoss * 2; // Simplified: 2x for concentration
      } else {
        il = this.calculateIL({
          initialPriceRatio: 1,
          currentPriceRatio: params.priceChange,
          poolType: 'AMM',
          initialLiquidity: liquidityAmount
        }).impermanentLoss;
      }

      // Calculate expected fees
      const shareOfPool = liquidityAmount / pool.tvl;
      const dailyFees = pool.volume24h * pool.feeRate / 100 * shareOfPool;
      const expectedFees = dailyFees * params.timeHorizon;

      // Calculate net return
      const ilInUSD = liquidityAmount * Math.abs(il) / 100;
      const netReturn = expectedFees - ilInUSD;

      // Calculate risk score (1-10)
      let riskScore = 5;
      if (pool.type === 'STABLE') riskScore = 3;
      if (pool.type === 'DLMM') riskScore = 7;
      if (Math.abs(il) > 10) riskScore += 2;
      if (pool.tvl < 100000) riskScore += 1;
      riskScore = Math.min(10, Math.max(1, riskScore));

      // Generate recommendation
      let recommendation = '';
      if (netReturn > 0 && riskScore <= 5) {
        recommendation = 'Highly Recommended';
      } else if (netReturn > 0 && riskScore <= 7) {
        recommendation = 'Recommended with caution';
      } else if (netReturn < 0 && riskScore > 7) {
        recommendation = 'Not recommended';
      } else {
        recommendation = 'Neutral';
      }

      results.push({
        name: pool.name,
        impermanentLoss: il,
        expectedFees,
        netReturn,
        riskScore,
        recommendation
      });
    }

    // Sort by net return
    results.sort((a, b) => b.netReturn - a.netReturn);

    return results;
  }

  /**
   * Suggest IL mitigation strategies
   */
  suggestMitigation(params: {
    currentIL: number;
    position: {
      value: number;
      poolType: string;
      duration: number;
    };
    marketConditions: {
      volatility: 'low' | 'medium' | 'high';
      trend: 'bullish' | 'bearish' | 'neutral';
      volume: 'increasing' | 'decreasing' | 'stable';
    };
  }): { strategies: MitigationStrategy[] } {
    const strategies: MitigationStrategy[] = [];
    const { currentIL, position, marketConditions } = params;

    // Strategy 1: Rebalancing
    if (currentIL > 5) {
      strategies.push({
        name: 'Position Rebalancing',
        action: 'Withdraw and re-enter at current price ratio',
        expectedReduction: currentIL * 0.8,
        riskLevel: 'low',
        implementation: 'Withdraw liquidity, swap to 50/50 ratio at current prices, re-deposit'
      });
    }

    // Strategy 2: Range adjustment (for concentrated liquidity)
    if (marketConditions.volatility === 'high') {
      strategies.push({
        name: 'Widen Price Range',
        action: 'Adjust to wider price range to reduce IL sensitivity',
        expectedReduction: 30,
        riskLevel: 'low',
        implementation: 'Withdraw and re-deposit with wider price bounds'
      });
    }

    // Strategy 3: Partial withdrawal
    if (currentIL > 8 && marketConditions.trend === 'bullish') {
      strategies.push({
        name: 'Partial Position Reduction',
        action: 'Withdraw 50% of position to limit further losses',
        expectedReduction: 50,
        riskLevel: 'medium',
        implementation: 'Remove 50% liquidity and hold tokens directly'
      });
    }

    // Strategy 4: Hedging
    if (position.value > 10000) {
      strategies.push({
        name: 'Options Hedging',
        action: 'Purchase put options on volatile token',
        expectedReduction: 60,
        riskLevel: 'medium',
        implementation: 'Buy put options with strike near current price'
      });
    }

    // Strategy 5: Fee tier optimization
    if (marketConditions.volume === 'increasing') {
      strategies.push({
        name: 'Move to Higher Fee Tier',
        action: 'Switch to higher fee pool for better compensation',
        expectedReduction: 20,
        riskLevel: 'low',
        implementation: 'Move position to 1% fee tier pool'
      });
    }

    // Strategy 6: Time-based exit
    if (position.duration > 30 && currentIL > 6) {
      strategies.push({
        name: 'Scheduled Exit',
        action: 'Set exit date based on fee accumulation forecast',
        expectedReduction: 40,
        riskLevel: 'low',
        implementation: `Exit position in ${Math.ceil(currentIL * 2)} days when fees cover IL`
      });
    }

    return { strategies };
  }

  /**
   * Calculate IL for a specific token pair with current prices
   */
  calculateRealTimeIL(params: {
    token0Symbol: string;
    token1Symbol: string;
    initialPrice0: number;
    initialPrice1: number;
    currentPrice0: number;
    currentPrice1: number;
    positionValue: number;
  }): ILResult {
    const initialRatio = params.initialPrice0 / params.initialPrice1;
    const currentRatio = params.currentPrice0 / params.currentPrice1;

    return this.calculateIL({
      initialPriceRatio: initialRatio,
      currentPriceRatio: currentRatio,
      poolType: 'AMM',
      initialLiquidity: params.positionValue
    });
  }

  /**
   * Estimate future IL based on volatility
   */
  estimateFutureIL(params: {
    currentPrice: number;
    volatility: number;  // Annual volatility percentage
    timeHorizon: number; // Days
    confidenceLevel: number; // 0.95 for 95% confidence
  }): {
    expectedIL: number;
    worstCaseIL: number;
    bestCaseIL: number;
    probabilityDistribution: Array<{ priceChange: number; probability: number; il: number }>;
  } {
    const { currentPrice, volatility, timeHorizon, confidenceLevel } = params;
    
    // Convert annual volatility to period volatility
    const periodVolatility = volatility * Math.sqrt(timeHorizon / 365);
    
    // Calculate price ranges based on volatility
    const zScore = this.getZScore(confidenceLevel);
    const worstCasePrice = currentPrice * Math.exp(-zScore * periodVolatility / 100);
    const bestCasePrice = currentPrice * Math.exp(zScore * periodVolatility / 100);
    
    // Calculate IL for different scenarios
    const worstCaseIL = this.calculateIL({
      initialPriceRatio: 1,
      currentPriceRatio: worstCasePrice / currentPrice,
      poolType: 'AMM'
    }).impermanentLoss;
    
    const bestCaseIL = this.calculateIL({
      initialPriceRatio: 1,
      currentPriceRatio: bestCasePrice / currentPrice,
      poolType: 'AMM'
    }).impermanentLoss;
    
    // Expected IL (at current price, no change)
    const expectedIL = 0;
    
    // Generate probability distribution
    const distribution = [];
    for (let i = -3; i <= 3; i += 0.5) {
      const priceChange = Math.exp(i * periodVolatility / 100);
      const probability = this.normalPDF(i);
      const il = this.calculateIL({
        initialPriceRatio: 1,
        currentPriceRatio: priceChange,
        poolType: 'AMM'
      }).impermanentLoss;
      
      distribution.push({
        priceChange: (priceChange - 1) * 100,
        probability,
        il
      });
    }
    
    return {
      expectedIL,
      worstCaseIL: Math.abs(worstCaseIL),
      bestCaseIL: Math.abs(bestCaseIL),
      probabilityDistribution: distribution
    };
  }

  /**
   * Get Z-score for confidence level
   */
  private getZScore(confidenceLevel: number): number {
    const zScores: { [key: number]: number } = {
      0.90: 1.645,
      0.95: 1.96,
      0.99: 2.576
    };
    return zScores[confidenceLevel] || 1.96;
  }

  /**
   * Normal probability density function
   */
  private normalPDF(x: number): number {
    return Math.exp(-0.5 * x * x) / Math.sqrt(2 * Math.PI);
  }
}