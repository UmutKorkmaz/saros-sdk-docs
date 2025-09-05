/**
 * PriceMonitor - Real-time price monitoring and triggered swap execution
 */

import { PublicKey } from '@solana/web3.js';
import { SwapManager, SwapResult } from './SwapManager';
import { logger } from './utils/logger';

export interface PriceAlert {
  fromMint: PublicKey;
  toMint: PublicKey;
  targetPrice: number;
  tolerance: number;
  callback?: (price: number) => void;
}

export interface PriceMonitorConfig {
  checkInterval?: number;
  maxAlerts?: number;
  enableWebSocket?: boolean;
}

export interface PriceData {
  pair: string;
  price: number;
  volume24h: number;
  change24h: number;
  timestamp: Date;
  source: string;
}

export interface SwapAtTargetParams {
  fromMint: PublicKey;
  toMint: PublicKey;
  amount: number;
  targetPrice: number;
  tolerance: number;
  maxWaitTime?: number;
  checkInterval?: number;
  executeImmediatelyIfBetter?: boolean;
}

export interface SwapAtTargetResult {
  executed: boolean;
  targetPrice: number;
  executionPrice?: number;
  swapResult?: SwapResult;
  waitTime?: number;
  reason?: string;
}

export class PriceMonitor {
  private swapManager: SwapManager;
  private alerts: Map<string, PriceAlert[]> = new Map();
  private priceHistory: Map<string, PriceData[]> = new Map();
  private monitoring: boolean = false;
  private checkInterval: number;
  private maxAlerts: number;
  private websocket?: any; // WebSocket connection for price feeds
  private priceCache: Map<string, PriceData> = new Map();

  constructor(
    swapManager: SwapManager,
    config?: PriceMonitorConfig
  ) {
    this.swapManager = swapManager;
    this.checkInterval = config?.checkInterval || 5000; // 5 seconds default
    this.maxAlerts = config?.maxAlerts || 100;

    if (config?.enableWebSocket) {
      this.initializeWebSocket();
    }

    logger.info('PriceMonitor initialized');
    logger.info(`  Check interval: ${this.checkInterval}ms`);
    logger.info(`  Max alerts: ${this.maxAlerts}`);
  }

  /**
   * Monitor price and execute swap when target is reached
   */
  async swapAtTargetPrice(params: SwapAtTargetParams): Promise<SwapAtTargetResult> {
    const pairKey = this.getPairKey(params.fromMint, params.toMint);
    const startTime = Date.now();
    const maxWaitTime = params.maxWaitTime || 300000; // 5 minutes default
    const checkInterval = params.checkInterval || this.checkInterval;

    logger.info(`📊 Monitoring price for ${pairKey}`);
    logger.info(`  Target: ${params.targetPrice} (±${params.tolerance}%)`);
    logger.info(`  Max wait: ${maxWaitTime / 1000}s`);

    // Check current price first
    const currentPrice = await this.getCurrentPrice(params.fromMint, params.toMint);

    if (this.isPriceInRange(currentPrice, params.targetPrice, params.tolerance)) {
      logger.info(`✅ Current price ${currentPrice} is already in target range!`);

      // Execute immediately
      const swapResult = await this.executeSwap(params);

      return {
        executed: true,
        targetPrice: params.targetPrice,
        executionPrice: currentPrice,
        swapResult,
        waitTime: 0
      };
    }

    if (params.executeImmediatelyIfBetter && currentPrice < params.targetPrice) {
      logger.info(`🎯 Current price ${currentPrice} is better than target!`);

      // Execute immediately at better price
      const swapResult = await this.executeSwap(params);

      return {
        executed: true,
        targetPrice: params.targetPrice,
        executionPrice: currentPrice,
        swapResult,
        waitTime: 0,
        reason: 'Better price available'
      };
    }

    // Start monitoring
    return new Promise((resolve) => {
      const intervalId = setInterval(async () => {
        const elapsed = Date.now() - startTime;

        // Check timeout
        if (elapsed > maxWaitTime) {
          clearInterval(intervalId);
          logger.warn(`⏱️ Timeout reached after ${elapsed / 1000}s`);

          resolve({
            executed: false,
            targetPrice: params.targetPrice,
            reason: 'Timeout'
          });
          return;
        }

        // Check current price
        const currentPrice = await this.getCurrentPrice(params.fromMint, params.toMint);
        logger.debug(`Current price: ${currentPrice} (target: ${params.targetPrice})`);

        // Check if price is in range
        if (this.isPriceInRange(currentPrice, params.targetPrice, params.tolerance)) {
          clearInterval(intervalId);

          logger.info(`🎯 Target price reached! Current: ${currentPrice}`);

          // Execute swap
          const swapResult = await this.executeSwap(params);

          resolve({
            executed: true,
            targetPrice: params.targetPrice,
            executionPrice: currentPrice,
            swapResult,
            waitTime: elapsed
          });
        }
      }, checkInterval);
    });
  }

  /**
   * Add price alert
   */
  addPriceAlert(alert: PriceAlert): string {
    const pairKey = this.getPairKey(alert.fromMint, alert.toMint);
    const alertId = `${pairKey}-${Date.now()}`;

    if (!this.alerts.has(pairKey)) {
      this.alerts.set(pairKey, []);
    }

    const alerts = this.alerts.get(pairKey)!;

    if (alerts.length >= this.maxAlerts) {
      logger.warn(`Max alerts reached for ${pairKey}`);
      return '';
    }

    alerts.push(alert);
    logger.info(`📢 Price alert added for ${pairKey} at ${alert.targetPrice}`);

    // Start monitoring if not already
    if (!this.monitoring) {
      this.startMonitoring();
    }

    return alertId;
  }

  /**
   * Remove price alert
   */
  removePriceAlert(alertId: string): boolean {
    for (const [pairKey, alerts] of this.alerts.entries()) {
      const index = alerts.findIndex(a =>
        `${pairKey}-${a.targetPrice}` === alertId
      );

      if (index !== -1) {
        alerts.splice(index, 1);
        logger.info(`Price alert removed: ${alertId}`);

        // Clean up empty pairs
        if (alerts.length === 0) {
          this.alerts.delete(pairKey);
        }

        return true;
      }
    }

    return false;
  }

  /**
   * Get current price for a pair
   */
  async getCurrentPrice(fromMint: PublicKey, toMint: PublicKey): Promise<number> {
    const pairKey = this.getPairKey(fromMint, toMint);

    // Check cache first
    const cached = this.priceCache.get(pairKey);
    if (cached && Date.now() - cached.timestamp.getTime() < 5000) {
      return cached.price;
    }

    // In production, fetch from price oracle or DEX
    // For this example, simulate price
    const price = await this.fetchPrice(fromMint, toMint);

    // Update cache
    const priceData: PriceData = {
      pair: pairKey,
      price,
      volume24h: Math.random() * 1000000,
      change24h: (Math.random() - 0.5) * 10,
      timestamp: new Date(),
      source: 'simulated'
    };

    this.priceCache.set(pairKey, priceData);
    this.addToPriceHistory(pairKey, priceData);

    return price;
  }

  /**
   * Get price history for a pair
   */
  getPriceHistory(
    fromMint: PublicKey,
    toMint: PublicKey,
    limit: number = 100
  ): PriceData[] {
    const pairKey = this.getPairKey(fromMint, toMint);
    const history = this.priceHistory.get(pairKey) || [];
    return history.slice(-limit);
  }

  /**
   * Calculate price statistics
   */
  getPriceStatistics(
    fromMint: PublicKey,
    toMint: PublicKey,
    period: number = 3600000 // 1 hour default
  ): {
    mean: number;
    median: number;
    std: number;
    min: number;
    max: number;
    volatility: number;
  } {
    const history = this.getPriceHistory(fromMint, toMint);
    const cutoff = Date.now() - period;
    const recentPrices = history
      .filter(h => h.timestamp.getTime() > cutoff)
      .map(h => h.price);

    if (recentPrices.length === 0) {
      return { mean: 0, median: 0, std: 0, min: 0, max: 0, volatility: 0 };
    }

    // Calculate statistics
    const mean = recentPrices.reduce((sum, p) => sum + p, 0) / recentPrices.length;
    const sorted = [...recentPrices].sort((a, b) => a - b);
    const median = sorted[Math.floor(sorted.length / 2)];
    const min = Math.min(...recentPrices);
    const max = Math.max(...recentPrices);

    // Standard deviation
    const variance = recentPrices.reduce((sum, p) => sum + Math.pow(p - mean, 2), 0) / recentPrices.length;
    const std = Math.sqrt(variance);

    // Volatility as percentage
    const volatility = (std / mean) * 100;

    return { mean, median, std, min, max, volatility };
  }

  /**
   * Start monitoring prices
   */
  private startMonitoring(): void {
    if (this.monitoring) return;

    this.monitoring = true;
    logger.info('🔍 Price monitoring started');

    const monitoringLoop = setInterval(async () => {
      if (this.alerts.size === 0) {
        clearInterval(monitoringLoop);
        this.monitoring = false;
        logger.info('Price monitoring stopped (no alerts)');
        return;
      }

      // Check all alerts
      for (const [pairKey, alerts] of this.alerts.entries()) {
        const [fromMint, toMint] = pairKey.split('-').map(s => new PublicKey(s));
        const currentPrice = await this.getCurrentPrice(fromMint, toMint);

        // Check each alert
        for (const alert of alerts) {
          if (this.isPriceInRange(currentPrice, alert.targetPrice, alert.tolerance)) {
            logger.info(`🔔 Price alert triggered for ${pairKey} at ${currentPrice}`);

            // Execute callback if provided
            if (alert.callback) {
              alert.callback(currentPrice);
            }
          }
        }
      }
    }, this.checkInterval);
  }

  /**
   * Initialize WebSocket for real-time prices
   */
  private initializeWebSocket(): void {
    // In production, connect to real price feed
    // This is a placeholder for WebSocket implementation
    logger.info('WebSocket price feed initialized (simulated)');
  }

  /**
   * Fetch price from oracle/DEX
   */
  private async fetchPrice(fromMint: PublicKey, toMint: PublicKey): Promise<number> {
    // In production, fetch from:
    // 1. Pyth oracle
    // 2. Jupiter aggregator
    // 3. Saros pools
    // 4. Other DEXs

    // Simulate price for demo
    const basePrices: { [key: string]: number } = {
      'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v': 1,    // USDC = $1
      'So11111111111111111111111111111111111111112': 50,    // SOL = $50
      'C98A4nkJXhpVZNAZdHUA95RpTF3T4whtQubL3YobiUX9': 0.5, // C98 = $0.5
    };

    const fromPrice = basePrices[fromMint.toString()] || 1;
    const toPrice = basePrices[toMint.toString()] || 1;

    // Add some random variation
    const variation = 1 + (Math.random() - 0.5) * 0.1; // ±5%

    return (fromPrice / toPrice) * variation;
  }

  /**
   * Check if price is in target range
   */
  private isPriceInRange(
    currentPrice: number,
    targetPrice: number,
    tolerance: number
  ): boolean {
    const lowerBound = targetPrice * (1 - tolerance / 100);
    const upperBound = targetPrice * (1 + tolerance / 100);
    return currentPrice >= lowerBound && currentPrice <= upperBound;
  }

  /**
   * Execute swap when conditions are met
   */
  private async executeSwap(params: SwapAtTargetParams): Promise<SwapResult> {
    logger.info('💱 Executing swap at target price...');

    return this.swapManager.swap({
      fromMint: params.fromMint,
      toMint: params.toMint,
      amount: params.amount,
      slippageTolerance: 1.0, // Use 1% for target price swaps
      simulateFirst: true
    });
  }

  /**
   * Get pair key for caching
   */
  private getPairKey(fromMint: PublicKey, toMint: PublicKey): string {
    return `${fromMint.toString()}-${toMint.toString()}`;
  }

  /**
   * Add price data to history
   */
  private addToPriceHistory(pairKey: string, priceData: PriceData): void {
    if (!this.priceHistory.has(pairKey)) {
      this.priceHistory.set(pairKey, []);
    }

    const history = this.priceHistory.get(pairKey)!;
    history.push(priceData);

    // Keep only last 1000 entries
    if (history.length > 1000) {
      history.shift();
    }
  }

  /**
   * Clear all alerts and stop monitoring
   */
  stopMonitoring(): void {
    this.alerts.clear();
    this.monitoring = false;

    if (this.websocket) {
      this.websocket.close();
    }

    logger.info('Price monitoring stopped');
  }
}