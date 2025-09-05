/**
 * Pool data management utilities
 * 
 * Provides pool discovery, caching, and management
 * for the routing system.
 */

import { Connection, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { logger } from './logger';

export interface PoolData {
  address: PublicKey;
  tokenA: PublicKey;
  tokenB: PublicKey;
  fee: number;
  liquidity: BN;
  volume24h: BN;
  apy?: number;
  lastUpdate: Date;
}

export class PoolDataProvider {
  private connection: Connection;
  private pools: Map<string, PoolData> = new Map();
  private pairIndex: Map<string, PoolData[]> = new Map();
  private lastFetch: Date | null = null;
  private cacheTimeout = 60000; // 1 minute

  constructor(connection: Connection) {
    this.connection = connection;
  }

  /**
   * Load all pools from chain
   */
  async loadPools(): Promise<void> {
    try {
      logger.info('Loading pool data...');
      
      // In production, this would fetch from Saros program
      // For now, using mock data
      const mockPools = this.getMockPools();
      
      for (const pool of mockPools) {
        this.addPool(pool);
      }
      
      this.lastFetch = new Date();
      
      logger.info(`Loaded ${this.pools.size} pools`);
      
    } catch (error) {
      logger.error('Error loading pools:', error);
      throw error;
    }
  }

  /**
   * Add pool to cache
   */
  private addPool(pool: PoolData): void {
    // Store by address
    this.pools.set(pool.address.toString(), pool);
    
    // Index by pair (both directions)
    const pairKey1 = this.getPairKey(pool.tokenA, pool.tokenB);
    const pairKey2 = this.getPairKey(pool.tokenB, pool.tokenA);
    
    if (!this.pairIndex.has(pairKey1)) {
      this.pairIndex.set(pairKey1, []);
    }
    if (!this.pairIndex.has(pairKey2)) {
      this.pairIndex.set(pairKey2, []);
    }
    
    this.pairIndex.get(pairKey1)!.push(pool);
    this.pairIndex.get(pairKey2)!.push(pool);
  }

  /**
   * Get pool by address
   */
  getPoolByAddress(address: PublicKey): PoolData | null {
    return this.pools.get(address.toString()) || null;
  }

  /**
   * Get pool for token pair
   */
  getPool(tokenA: PublicKey, tokenB: PublicKey): PoolData | null {
    const pools = this.getPoolsForPair(tokenA, tokenB);
    
    if (pools.length === 0) {
      return null;
    }
    
    // Return pool with highest liquidity
    return pools.reduce((best, pool) => 
      pool.liquidity.gt(best.liquidity) ? pool : best
    );
  }

  /**
   * Get all pools for token pair
   */
  getPoolsForPair(tokenA: PublicKey, tokenB: PublicKey): PoolData[] {
    const pairKey = this.getPairKey(tokenA, tokenB);
    return this.pairIndex.get(pairKey) || [];
  }

  /**
   * Get all pools
   */
  getAllPools(): PoolData[] {
    return Array.from(this.pools.values());
  }

  /**
   * Get pool count
   */
  getPoolCount(): number {
    return this.pools.size;
  }

  /**
   * Refresh pool data if cache expired
   */
  async refreshIfNeeded(): Promise<void> {
    if (!this.lastFetch || 
        Date.now() - this.lastFetch.getTime() > this.cacheTimeout) {
      await this.loadPools();
    }
  }

  /**
   * Get pair key for indexing
   */
  private getPairKey(tokenA: PublicKey, tokenB: PublicKey): string {
    // Sort tokens for consistent key
    const [first, second] = tokenA.toString() < tokenB.toString() 
      ? [tokenA, tokenB] 
      : [tokenB, tokenA];
    
    return `${first.toString()}-${second.toString()}`;
  }

  /**
   * Get mock pool data for testing
   */
  private getMockPools(): PoolData[] {
    const SOL = new PublicKey('So11111111111111111111111111111111111111112');
    const USDC = new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v');
    const USDT = new PublicKey('Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB');
    const ETH = new PublicKey('7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs');
    const BONK = new PublicKey('DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263');
    const SAMO = new PublicKey('7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU');
    
    return [
      // Major pools
      {
        address: new PublicKey('7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm'),
        tokenA: SOL,
        tokenB: USDC,
        fee: 0.3,
        liquidity: new BN('10000000000000'), // $10M
        volume24h: new BN('1000000000000'), // $1M
        apy: 12.5,
        lastUpdate: new Date()
      },
      {
        address: new PublicKey('2QdhepnKRTLjjSqPL1PtKNwqrUkoLee5Gqs8bvZhRdMv'),
        tokenA: ETH,
        tokenB: USDC,
        fee: 0.3,
        liquidity: new BN('5000000000000'), // $5M
        volume24h: new BN('500000000000'), // $500k
        apy: 8.2,
        lastUpdate: new Date()
      },
      {
        address: new PublicKey('EGZ7tiLeH62TPV1gL8WwbXGzEPa9zmcpVnnkPKKnrE2U'),
        tokenA: USDC,
        tokenB: USDT,
        fee: 0.04,
        liquidity: new BN('20000000000000'), // $20M
        volume24h: new BN('2000000000000'), // $2M
        apy: 3.5,
        lastUpdate: new Date()
      },
      {
        address: new PublicKey('HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ'),
        tokenA: SOL,
        tokenB: ETH,
        fee: 0.3,
        liquidity: new BN('3000000000000'), // $3M
        volume24h: new BN('300000000000'), // $300k
        apy: 15.0,
        lastUpdate: new Date()
      },
      {
        address: new PublicKey('A8YFbxQYFVqKZaoYJLLUVcQiWP7G2MeEgW5wsAQgMvFw'),
        tokenA: SOL,
        tokenB: BONK,
        fee: 1.0,
        liquidity: new BN('1000000000000'), // $1M
        volume24h: new BN('200000000000'), // $200k
        apy: 45.0,
        lastUpdate: new Date()
      },
      {
        address: new PublicKey('9wFFyRfZBsuAha4YcuxcXLKwMxJR43S7fPfQLusDBzvT'),
        tokenA: USDC,
        tokenB: BONK,
        fee: 1.0,
        liquidity: new BN('500000000000'), // $500k
        volume24h: new BN('100000000000'), // $100k
        apy: 35.0,
        lastUpdate: new Date()
      },
      {
        address: new PublicKey('F8Vyqk3unwxkXukZFQeYyGmFfTG3CAX4v24iyrjEYBJV'),
        tokenA: SAMO,
        tokenB: SOL,
        fee: 1.0,
        liquidity: new BN('200000000000'), // $200k
        volume24h: new BN('50000000000'), // $50k
        apy: 60.0,
        lastUpdate: new Date()
      },
      {
        address: new PublicKey('ChVzxWRmrTeSgwd3Ui3UumcN8KX7VK3WaD4KGeSKpypj'),
        tokenA: SAMO,
        tokenB: USDC,
        fee: 1.0,
        liquidity: new BN('100000000000'), // $100k
        volume24h: new BN('20000000000'), // $20k
        apy: 55.0,
        lastUpdate: new Date()
      },
      // Additional pools for better routing
      {
        address: new PublicKey('BqWWnhCxEgkGsqPC4bPtbFJu5jHkVGk2XfMqLEZQDDZE'),
        tokenA: SOL,
        tokenB: USDT,
        fee: 0.3,
        liquidity: new BN('2000000000000'), // $2M
        volume24h: new BN('400000000000'), // $400k
        apy: 10.0,
        lastUpdate: new Date()
      },
      {
        address: new PublicKey('FpCMFDFGYotvufJ7HrFHsWEiiQCGbkLCtwHiDnh7o28Q'),
        tokenA: ETH,
        tokenB: USDT,
        fee: 0.3,
        liquidity: new BN('1500000000000'), // $1.5M
        volume24h: new BN('300000000000'), // $300k
        apy: 9.0,
        lastUpdate: new Date()
      }
    ];
  }

  /**
   * Get top pools by liquidity
   */
  getTopPoolsByLiquidity(limit: number = 10): PoolData[] {
    const sorted = Array.from(this.pools.values()).sort((a, b) => 
      b.liquidity.sub(a.liquidity).toNumber()
    );
    
    return sorted.slice(0, limit);
  }

  /**
   * Get top pools by volume
   */
  getTopPoolsByVolume(limit: number = 10): PoolData[] {
    const sorted = Array.from(this.pools.values()).sort((a, b) => 
      b.volume24h.sub(a.volume24h).toNumber()
    );
    
    return sorted.slice(0, limit);
  }

  /**
   * Find pools with specific token
   */
  getPoolsWithToken(token: PublicKey): PoolData[] {
    return Array.from(this.pools.values()).filter(pool => 
      pool.tokenA.equals(token) || pool.tokenB.equals(token)
    );
  }

  /**
   * Update pool data
   */
  async updatePool(address: PublicKey): Promise<void> {
    try {
      // Fetch latest pool data from chain
      // This would use Saros SDK in production
      
      const pool = this.pools.get(address.toString());
      if (pool) {
        pool.lastUpdate = new Date();
        logger.debug(`Updated pool ${address.toString()}`);
      }
      
    } catch (error) {
      logger.error(`Error updating pool ${address.toString()}:`, error);
    }
  }

  /**
   * Get pool statistics
   */
  getStatistics(): any {
    const pools = Array.from(this.pools.values());
    
    return {
      totalPools: pools.length,
      totalLiquidity: pools.reduce((sum, p) => sum.add(p.liquidity), new BN(0)).toString(),
      totalVolume24h: pools.reduce((sum, p) => sum.add(p.volume24h), new BN(0)).toString(),
      averageAPY: pools.reduce((sum, p) => sum + (p.apy || 0), 0) / pools.length,
      uniqueTokens: new Set([
        ...pools.map(p => p.tokenA.toString()),
        ...pools.map(p => p.tokenB.toString())
      ]).size
    };
  }
}