/**
 * DynamicSlippageSwap - Intelligent slippage calculation based on market conditions
 */

import { PublicKey } from '@solana/web3.js';
import { SwapManager, SwapParams, SwapResult } from './SwapManager';
import { logger } from './utils/logger';

export interface DynamicSwapParams {
  fromMint: PublicKey;
  toMint: PublicKey;
  amount: number;
  maxPriceImpact: number;
  urgency: 'low' | 'normal' | 'high';
  useMultiHop?: boolean;
}

export interface MarketConditions {
  volatility: number;
  liquidity: number;
  volume24h: number;
  priceChange24h: number;
  spread: number;
}

export interface DynamicSwapResult extends SwapResult {
  slippageUsed: number;
  route?: string[];
  marketConditions?: MarketConditions;
}

export class DynamicSlippageSwap {
  private swapManager: SwapManager;
  private readonly MIN_SLIPPAGE = 0.1;
  private readonly MAX_SLIPPAGE = 5.0;
  private readonly volatilityCache: Map<string, number> = new Map();

  constructor(config: { rpcUrl: string; privateKey: string }) {
    this.swapManager = new SwapManager({
      rpcUrl: config.rpcUrl,
      privateKey: config.privateKey
    });
  }

  /**
   * Execute swap with dynamically calculated optimal slippage
   */
  async executeWithOptimalSlippage(params: DynamicSwapParams): Promise<DynamicSwapResult> {
    logger.info('🎯 Calculating optimal slippage for swap...');

    try {
      // Step 1: Analyze market conditions
      const marketConditions = await this.analyzeMarketConditions(
        params.fromMint,
        params.toMint
      );

      logger.info('📊 Market Analysis:');
      logger.info(`  Volatility: ${marketConditions.volatility.toFixed(2)}%`);
      logger.info(`  Liquidity score: ${marketConditions.liquidity}/100`);
      logger.info(`  24h volume: $${marketConditions.volume24h.toLocaleString()}`);
      logger.info(`  Spread: ${marketConditions.spread.toFixed(3)}%`);

      // Step 2: Calculate optimal slippage
      const optimalSlippage = this.calculateOptimalSlippage(
        marketConditions,
        params.amount,
        params.maxPriceImpact,
        params.urgency
      );

      logger.info(`✨ Optimal slippage calculated: ${optimalSlippage.toFixed(2)}%`);

      // Step 3: Determine if multi-hop is beneficial
      let route: string[] | undefined;
      if (params.useMultiHop) {
        route = await this.findOptimalRoute(
          params.fromMint,
          params.toMint,
          params.amount,
          marketConditions
        );
        
        if (route && route.length > 2) {
          logger.info(`🔀 Using multi-hop route: ${route.join(' → ')}`);
        }
      }

      // Step 4: Execute swap with calculated parameters
      const swapResult = await this.swapManager.swap({
        fromMint: params.fromMint,
        toMint: params.toMint,
        amount: params.amount,
        slippageTolerance: optimalSlippage,
        simulateFirst: true,
        maxRetries: this.getRetryCount(params.urgency)
      });

      // Step 5: Return enhanced result
      return {
        ...swapResult,
        slippageUsed: optimalSlippage,
        route,
        marketConditions
      };

    } catch (error: any) {
      logger.error('Dynamic slippage swap failed:', error);
      
      return {
        success: false,
        signature: '',
        amountIn: params.amount,
        amountOut: 0,
        priceImpact: 0,
        slippageUsed: 0,
        gasUsed: 0,
        retries: 0,
        error: error.message,
        explorerUrl: ''
      };
    }
  }

  /**
   * Analyze current market conditions for the token pair
   */
  private async analyzeMarketConditions(
    fromMint: PublicKey,
    toMint: PublicKey
  ): Promise<MarketConditions> {
    const pairKey = `${fromMint.toString()}-${toMint.toString()}`;

    // In production, these would be fetched from real data sources
    // For this example, we'll simulate market conditions

    // Check cache for recent volatility
    let volatility = this.volatilityCache.get(pairKey);
    if (!volatility) {
      volatility = await this.calculateVolatility(fromMint, toMint);
      this.volatilityCache.set(pairKey, volatility);
      
      // Clear cache after 5 minutes
      setTimeout(() => this.volatilityCache.delete(pairKey), 300000);
    }

    // Simulate other market metrics
    const liquidity = await this.assessLiquidity(fromMint, toMint);
    const volume24h = await this.get24hVolume(fromMint, toMint);
    const priceChange24h = await this.get24hPriceChange(fromMint, toMint);
    const spread = await this.calculateSpread(fromMint, toMint);

    return {
      volatility,
      liquidity,
      volume24h,
      priceChange24h,
      spread
    };
  }

  /**
   * Calculate optimal slippage based on market conditions and trade parameters
   */
  private calculateOptimalSlippage(
    conditions: MarketConditions,
    amount: number,
    maxPriceImpact: number,
    urgency: 'low' | 'normal' | 'high'
  ): number {
    let baseSlippage = 0.5; // Start with 0.5%

    // Adjust for volatility (higher volatility = higher slippage)
    const volatilityMultiplier = 1 + (conditions.volatility / 100);
    baseSlippage *= volatilityMultiplier;

    // Adjust for liquidity (lower liquidity = higher slippage)
    const liquidityMultiplier = 2 - (conditions.liquidity / 100);
    baseSlippage *= liquidityMultiplier;

    // Adjust for spread (higher spread = higher slippage)
    baseSlippage += conditions.spread;

    // Adjust for trade size (larger trades = higher slippage)
    const sizeImpact = this.calculateSizeImpact(amount, conditions.volume24h);
    baseSlippage *= (1 + sizeImpact);

    // Adjust for urgency
    const urgencyMultipliers = {
      low: 0.8,    // Can wait, use tighter slippage
      normal: 1.0, // Standard slippage
      high: 1.5    // Need fast execution, use higher slippage
    };
    baseSlippage *= urgencyMultipliers[urgency];

    // Apply bounds
    baseSlippage = Math.max(this.MIN_SLIPPAGE, Math.min(this.MAX_SLIPPAGE, baseSlippage));

    // Ensure slippage covers expected price impact
    if (maxPriceImpact > 0) {
      baseSlippage = Math.max(baseSlippage, maxPriceImpact * 1.2);
    }

    return Math.round(baseSlippage * 100) / 100; // Round to 2 decimal places
  }

  /**
   * Calculate volatility for the token pair
   */
  private async calculateVolatility(
    fromMint: PublicKey,
    toMint: PublicKey
  ): Promise<number> {
    // In production, calculate from historical price data
    // For this example, return simulated volatility

    // Stable pairs have low volatility
    if (this.isStablePair(fromMint, toMint)) {
      return Math.random() * 2 + 0.5; // 0.5-2.5%
    }

    // Regular pairs have moderate volatility
    return Math.random() * 10 + 5; // 5-15%
  }

  /**
   * Assess liquidity depth for the pair
   */
  private async assessLiquidity(
    fromMint: PublicKey,
    toMint: PublicKey
  ): Promise<number> {
    // In production, fetch from pool data
    // Return score 0-100 (100 = excellent liquidity)

    // Major pairs have good liquidity
    if (this.isMajorPair(fromMint, toMint)) {
      return 80 + Math.random() * 20; // 80-100
    }

    // Regular pairs have moderate liquidity
    return 40 + Math.random() * 40; // 40-80
  }

  /**
   * Get 24h trading volume
   */
  private async get24hVolume(
    fromMint: PublicKey,
    toMint: PublicKey
  ): Promise<number> {
    // In production, fetch from analytics API
    // Return simulated volume

    if (this.isMajorPair(fromMint, toMint)) {
      return 1000000 + Math.random() * 5000000; // $1M - $6M
    }

    return 50000 + Math.random() * 450000; // $50k - $500k
  }

  /**
   * Get 24h price change
   */
  private async get24hPriceChange(
    fromMint: PublicKey,
    toMint: PublicKey
  ): Promise<number> {
    // In production, fetch from price API
    // Return simulated price change percentage

    if (this.isStablePair(fromMint, toMint)) {
      return (Math.random() - 0.5) * 2; // -1% to +1%
    }

    return (Math.random() - 0.5) * 20; // -10% to +10%
  }

  /**
   * Calculate bid-ask spread
   */
  private async calculateSpread(
    fromMint: PublicKey,
    toMint: PublicKey
  ): Promise<number> {
    // In production, fetch from order book
    // Return simulated spread percentage

    if (this.isMajorPair(fromMint, toMint)) {
      return 0.05 + Math.random() * 0.1; // 0.05% - 0.15%
    }

    return 0.1 + Math.random() * 0.4; // 0.1% - 0.5%
  }

  /**
   * Calculate size impact based on trade size vs daily volume
   */
  private calculateSizeImpact(amount: number, volume24h: number): number {
    const tradeValue = amount * 50; // Assume $50 per token for example
    const volumeRatio = tradeValue / volume24h;

    if (volumeRatio < 0.001) return 0;      // < 0.1% of daily volume
    if (volumeRatio < 0.01) return 0.1;     // < 1% of daily volume
    if (volumeRatio < 0.05) return 0.3;     // < 5% of daily volume
    if (volumeRatio < 0.1) return 0.5;      // < 10% of daily volume
    return 1.0;                             // >= 10% of daily volume
  }

  /**
   * Find optimal route for multi-hop swaps
   */
  private async findOptimalRoute(
    fromMint: PublicKey,
    toMint: PublicKey,
    amount: number,
    conditions: MarketConditions
  ): Promise<string[] | undefined> {
    // In production, implement proper routing algorithm
    // For this example, return simple routes

    const routes: string[][] = [
      [fromMint.toString(), toMint.toString()], // Direct route
    ];

    // Add multi-hop routes through common intermediates
    const USDC = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
    const SOL = 'So11111111111111111111111111111111111111112';

    if (!fromMint.toString().includes(USDC) && !toMint.toString().includes(USDC)) {
      routes.push([
        fromMint.toString(),
        USDC,
        toMint.toString()
      ]);
    }

    if (!fromMint.toString().includes(SOL) && !toMint.toString().includes(SOL)) {
      routes.push([
        fromMint.toString(),
        SOL,
        toMint.toString()
      ]);
    }

    // Select best route based on conditions
    // In production, calculate actual costs for each route
    if (conditions.liquidity < 50 && routes.length > 1) {
      // Use multi-hop for low liquidity pairs
      return routes[1].map(addr => addr.slice(0, 8) + '...');
    }

    // Use direct route by default
    return routes[0].map(addr => addr.slice(0, 8) + '...');
  }

  /**
   * Determine retry count based on urgency
   */
  private getRetryCount(urgency: 'low' | 'normal' | 'high'): number {
    switch (urgency) {
      case 'low':
        return 5; // More retries, can wait
      case 'normal':
        return 3; // Standard retries
      case 'high':
        return 1; // Quick execution, minimal retries
    }
  }

  /**
   * Check if token pair is stable (e.g., USDC-USDT)
   */
  private isStablePair(fromMint: PublicKey, toMint: PublicKey): boolean {
    const stableTokens = [
      'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', // USDC
      'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', // USDT
      'USDH1SM1ojwWUga67PGrgFWUHibbjqMvuMaDkRJTgkX', // USDH
    ];

    return stableTokens.includes(fromMint.toString()) && 
           stableTokens.includes(toMint.toString());
  }

  /**
   * Check if token pair is major (high volume)
   */
  private isMajorPair(fromMint: PublicKey, toMint: PublicKey): boolean {
    const majorTokens = [
      'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', // USDC
      'So11111111111111111111111111111111111111112',  // SOL
      '7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs', // ETH
    ];

    return majorTokens.includes(fromMint.toString()) || 
           majorTokens.includes(toMint.toString());
  }
}