# Liquidity Provision Guide

## Overview

This guide covers everything you need to know about providing liquidity on Saros Finance, including strategies, risk management, and optimization techniques for both AMM and DLMM pools.

## Table of Contents
- [Understanding Liquidity Provision](#understanding-liquidity-provision)
- [AMM vs DLMM Liquidity](#amm-vs-dlmm-liquidity)
- [Position Strategies](#position-strategies)
- [Risk Management](#risk-management)
- [Yield Optimization](#yield-optimization)
- [Advanced Techniques](#advanced-techniques)

## Understanding Liquidity Provision

### What is Liquidity Provision?

Liquidity providers (LPs) deposit token pairs into pools to facilitate trading. In return, they earn:
- Trading fees from swaps
- Yield farming rewards
- Potential token incentives

### Key Concepts

#### 1. Liquidity Depth
- **Definition**: Total value locked (TVL) in the pool
- **Impact**: Higher liquidity = lower slippage for traders
- **Rewards**: Fees proportional to your share of the pool

#### 2. Price Ranges
- **AMM**: Liquidity spread across entire price curve (0 to ∞)
- **DLMM**: Concentrated in specific price bins
- **Efficiency**: DLMM can be 4000x more capital efficient

#### 3. Impermanent Loss (IL)
- **Cause**: Price divergence from entry point
- **Mitigation**: Tight ranges, stable pairs, active management
- **Compensation**: Trading fees should exceed IL for profitability

## AMM vs DLMM Liquidity

### Traditional AMM Pools

```typescript
// AMM: Simple 50/50 liquidity provision
const amm = new SarosAMM(connection, wallet);

// Add liquidity to SOL/USDC pool
const result = await amm.addLiquidity({
  poolAddress: SOL_USDC_POOL,
  amountA: 10 * LAMPORTS_PER_SOL, // 10 SOL
  amountB: 500 * 1e6, // 500 USDC
  slippage: 0.5
});

console.log(`LP tokens received: ${result.lpTokens}`);
```

**Characteristics:**
- Simple to manage
- Passive strategy
- Lower capital efficiency
- Suitable for volatile pairs

### DLMM Concentrated Liquidity

```typescript
// DLMM: Concentrated liquidity in specific price range
const dlmm = new DLMMClient(connection, wallet);

// Add liquidity concentrated around current price
const currentPrice = 50; // SOL = $50
const result = await dlmm.createPosition({
  poolAddress: SOL_USDC_DLMM_POOL,
  lowerPrice: 45, // -10%
  upperPrice: 55, // +10%
  liquidityAmount: 1000 * 1e6, // $1000
  distribution: 'NORMAL' // Concentrate around mid-price
});

console.log(`Position NFT: ${result.positionId}`);
```

**Characteristics:**
- Higher capital efficiency
- Active management required
- Higher fee earnings in range
- Risk of being out of range

## Position Strategies

### 1. Wide Range (Conservative)

**Best for:** Beginners, volatile assets, passive management

```typescript
async function createWideRangePosition() {
  const currentPrice = await dlmm.getCurrentPrice(pool);
  
  return await dlmm.createPosition({
    lowerPrice: currentPrice * 0.5,  // -50%
    upperPrice: currentPrice * 2.0,  // +100%
    liquidityAmount: liquidity,
    distribution: 'UNIFORM'
  });
}
```

**Pros:**
- Lower management needs
- Always earning fees
- Reduced IL risk

**Cons:**
- Lower capital efficiency
- Lower fee earnings per dollar

### 2. Narrow Range (Aggressive)

**Best for:** Stable pairs, active managers, high volume pools

```typescript
async function createNarrowRangePosition() {
  const currentPrice = await dlmm.getCurrentPrice(pool);
  const volatility = await calculateVolatility(pool, 24); // 24h volatility
  
  // Range based on volatility
  const range = volatility * 2; // 2x daily volatility
  
  return await dlmm.createPosition({
    lowerPrice: currentPrice * (1 - range),
    upperPrice: currentPrice * (1 + range),
    liquidityAmount: liquidity,
    distribution: 'NORMAL'
  });
}
```

**Pros:**
- Maximum capital efficiency
- Highest fee earnings when in range
- Ideal for stable pairs

**Cons:**
- Requires active management
- Risk of going out of range
- Higher gas costs from rebalancing

### 3. Ladder Strategy

**Best for:** DCA approach, uncertain market direction

```typescript
async function createLadderPositions() {
  const currentPrice = await dlmm.getCurrentPrice(pool);
  const positions = [];
  
  // Create 5 positions at different ranges
  const ranges = [
    { lower: 0.8, upper: 0.9 },   // Below current
    { lower: 0.9, upper: 1.0 },   // Just below
    { lower: 0.95, upper: 1.05 }, // At current
    { lower: 1.0, upper: 1.1 },   // Just above
    { lower: 1.1, upper: 1.2 }    // Above current
  ];
  
  for (const range of ranges) {
    const position = await dlmm.createPosition({
      lowerPrice: currentPrice * range.lower,
      upperPrice: currentPrice * range.upper,
      liquidityAmount: liquidity / ranges.length,
      distribution: 'UNIFORM'
    });
    positions.push(position);
  }
  
  return positions;
}
```

### 4. Dynamic Range (Adaptive)

**Best for:** Algorithmic traders, advanced users

```typescript
class DynamicRangeManager {
  async adjustRange(position: Position) {
    const metrics = await this.analyzeMarket();
    
    if (metrics.volatility > threshold) {
      // Widen range in high volatility
      await this.widenRange(position, metrics.volatility);
    } else if (metrics.priceOutOfRange) {
      // Rebalance to new range
      await this.rebalancePosition(position, metrics.currentPrice);
    }
  }
  
  async rebalancePosition(position: Position, newPrice: number) {
    // Remove liquidity from old position
    await dlmm.removeLiquidity(position.id);
    
    // Create new position around new price
    return await dlmm.createPosition({
      lowerPrice: newPrice * 0.95,
      upperPrice: newPrice * 1.05,
      liquidityAmount: position.liquidity,
      distribution: 'NORMAL'
    });
  }
}
```

## Risk Management

### 1. Impermanent Loss Protection

```typescript
class ILProtection {
  async calculateIL(position: Position): Promise<number> {
    const entryPrices = position.entryPrices;
    const currentPrices = await this.getCurrentPrices();
    
    // Calculate IL percentage
    const priceRatio = currentPrices.tokenB / currentPrices.tokenA;
    const entryRatio = entryPrices.tokenB / entryPrices.tokenA;
    const k = priceRatio / entryRatio;
    
    const il = 2 * Math.sqrt(k) / (1 + k) - 1;
    return Math.abs(il) * 100;
  }
  
  async hedgePosition(position: Position) {
    const il = await this.calculateIL(position);
    
    if (il > 5) { // 5% IL threshold
      // Open hedge position
      await this.openHedge(position, il);
    }
  }
}
```

### 2. Range Management

```typescript
class RangeManager {
  async monitorPosition(position: Position) {
    const currentPrice = await dlmm.getCurrentPrice(position.pool);
    const { lowerPrice, upperPrice } = position;
    
    // Calculate position metrics
    const rangeUtilization = this.calculateUtilization(
      currentPrice, 
      lowerPrice, 
      upperPrice
    );
    
    if (rangeUtilization < 0.2) { // 20% utilization
      console.log('⚠️ Position nearly out of range');
      await this.sendAlert('Position needs rebalancing');
    }
  }
  
  calculateUtilization(current: number, lower: number, upper: number): number {
    if (current <= lower || current >= upper) return 0;
    
    const rangeSize = upper - lower;
    const distanceToEdge = Math.min(current - lower, upper - current);
    return distanceToEdge / (rangeSize / 2);
  }
}
```

### 3. Portfolio Diversification

```typescript
class PortfolioManager {
  async diversifyLiquidity(totalCapital: number) {
    const allocation = {
      stablePairs: 0.4,    // 40% in stable pairs
      majorPairs: 0.35,    // 35% in major pairs
      volatilePairs: 0.15, // 15% in volatile pairs
      cash: 0.1           // 10% cash reserve
    };
    
    // Stable pairs (USDC/USDT)
    await this.provideToStable(totalCapital * allocation.stablePairs);
    
    // Major pairs (SOL/USDC, ETH/USDC)
    await this.provideToMajor(totalCapital * allocation.majorPairs);
    
    // Volatile pairs (BONK/SOL)
    await this.provideToVolatile(totalCapital * allocation.volatilePairs);
  }
}
```

## Yield Optimization

### 1. Fee Tier Optimization

```typescript
async function selectOptimalPool(tokenA: Token, tokenB: Token, amount: number) {
  const pools = await saros.getPoolsForPair(tokenA, tokenB);
  
  let bestPool = null;
  let bestAPR = 0;
  
  for (const pool of pools) {
    const stats = await pool.getStats();
    
    // Calculate expected APR
    const feeAPR = (stats.fees24h * 365) / stats.tvl;
    const volumeToTVL = stats.volume24h / stats.tvl;
    const impermanentLoss = estimateIL(pool.volatility);
    
    const netAPR = feeAPR - impermanentLoss;
    
    if (netAPR > bestAPR) {
      bestAPR = netAPR;
      bestPool = pool;
    }
  }
  
  return bestPool;
}
```

### 2. Auto-Compound Strategy

```typescript
class AutoCompounder {
  async compound(position: Position) {
    const fees = await dlmm.getUnclaimedFees(position.id);
    
    if (fees.total > minCompoundAmount) {
      // Claim fees
      const claimed = await dlmm.claimFees(position.id);
      
      // Reinvest fees
      await dlmm.addLiquidity({
        positionId: position.id,
        amountA: claimed.tokenA,
        amountB: claimed.tokenB
      });
      
      console.log(`Compounded ${claimed.total} in fees`);
    }
  }
  
  // Run every 24 hours
  scheduleCompounding(position: Position) {
    setInterval(() => this.compound(position), 24 * 60 * 60 * 1000);
  }
}
```

### 3. Yield Farming Integration

```typescript
class YieldFarmer {
  async stakeLPTokens(position: Position) {
    const farms = await saros.getFarmsForPool(position.pool);
    
    // Select best farm
    const bestFarm = farms.reduce((best, farm) => 
      farm.apr > best.apr ? farm : best
    );
    
    // Stake LP position
    await bestFarm.stake(position.id);
    
    // Set up auto-harvest
    this.scheduleHarvest(bestFarm, position);
  }
  
  async scheduleHarvest(farm: Farm, position: Position) {
    setInterval(async () => {
      const rewards = await farm.getPendingRewards(position.id);
      
      if (rewards > minHarvestAmount) {
        await farm.harvest(position.id);
        
        // Optionally convert rewards and reinvest
        await this.reinvestRewards(rewards, position);
      }
    }, 6 * 60 * 60 * 1000); // Every 6 hours
  }
}
```

## Advanced Techniques

### 1. Delta Neutral Strategies

```typescript
class DeltaNeutralLP {
  async createDeltaNeutralPosition(pool: Pool, amount: number) {
    // Provide liquidity
    const lpPosition = await dlmm.createPosition({
      pool: pool.address,
      amount: amount,
      range: { lower: 0.95, upper: 1.05 }
    });
    
    // Calculate delta exposure
    const delta = await this.calculateDelta(lpPosition);
    
    // Open hedge position
    const hedgeSize = Math.abs(delta) * amount;
    if (delta > 0) {
      // Short the asset
      await this.shortAsset(pool.tokenA, hedgeSize);
    } else {
      // Long the asset
      await this.longAsset(pool.tokenA, hedgeSize);
    }
    
    return { lpPosition, hedgePosition };
  }
}
```

### 2. MEV Protection for LPs

```typescript
class MEVProtectedLP {
  async addLiquidityProtected(params: LPParams) {
    // Use Jito for MEV protection
    const tx = await this.buildLPTransaction(params);
    
    // Add tip for priority
    tx.add(SystemProgram.transfer({
      fromPubkey: wallet.publicKey,
      toPubkey: JITO_TIP_ACCOUNT,
      lamports: 0.001 * LAMPORTS_PER_SOL
    }));
    
    // Send through Jito bundle
    return await jitoClient.sendBundle([tx]);
  }
}
```

### 3. Cross-Protocol Arbitrage

```typescript
class CrossProtocolArbitrage {
  async findArbitrageOpportunity() {
    const sarosPrice = await saros.getPrice(TOKEN_PAIR);
    const otherDexPrice = await otherDex.getPrice(TOKEN_PAIR);
    
    const spread = Math.abs(sarosPrice - otherDexPrice) / sarosPrice;
    
    if (spread > 0.005) { // 0.5% spread
      return {
        buy: sarosPrice < otherDexPrice ? 'saros' : 'other',
        sell: sarosPrice < otherDexPrice ? 'other' : 'saros',
        profit: spread - 0.003 // Minus fees
      };
    }
    
    return null;
  }
}
```

## Performance Metrics

### Key Metrics to Track

1. **APR/APY**: Annual returns including compounding
2. **Fee Earnings**: Daily/weekly fee accumulation
3. **Impermanent Loss**: Unrealized loss from price divergence
4. **Range Efficiency**: Time spent in range vs out of range
5. **Gas Costs**: Transaction costs for management

### Monitoring Dashboard

```typescript
class LPDashboard {
  async getMetrics(position: Position) {
    return {
      tvl: await this.getTVL(position),
      fees24h: await this.getFees24h(position),
      apr: await this.calculateAPR(position),
      il: await this.calculateIL(position),
      rangeStatus: await this.getRangeStatus(position),
      pnl: await this.calculatePNL(position)
    };
  }
  
  async generateReport(positions: Position[]) {
    const metrics = await Promise.all(
      positions.map(p => this.getMetrics(p))
    );
    
    return {
      totalTVL: metrics.reduce((sum, m) => sum + m.tvl, 0),
      totalFees24h: metrics.reduce((sum, m) => sum + m.fees24h, 0),
      averageAPR: metrics.reduce((sum, m) => sum + m.apr, 0) / metrics.length,
      positions: metrics
    };
  }
}
```

## Best Practices

1. **Start Small**: Test strategies with small amounts first
2. **Monitor Actively**: Set up alerts for range and IL thresholds
3. **Diversify**: Don't put all capital in one pool or strategy
4. **Consider Costs**: Factor in gas costs for rebalancing
5. **Understand Risks**: IL can exceed fee earnings in volatile markets
6. **Use Tools**: Leverage analytics and automation tools
7. **Stay Informed**: Follow protocol updates and market conditions

## Resources

- [Impermanent Loss Calculator](/tools/il-calculator)
- [APY Calculator](/tools/apy-calculator)
- [Position Manager](/tools/position-manager)
- [Video Tutorials](https://youtube.com/saros-lp-guide)
- [Discord Community](https://discord.gg/saros)