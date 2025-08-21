# DLMM Deep Dive

## Understanding Dynamic Liquidity Market Maker

DLMM (Dynamic Liquidity Market Maker) is Saros Finance's innovative approach to concentrated liquidity, offering up to 4000x capital efficiency compared to traditional AMMs.

## Table of Contents
- [Core Concepts](#core-concepts)
- [Bin Architecture](#bin-architecture)
- [Price Discovery](#price-discovery)
- [Liquidity Strategies](#liquidity-strategies)
- [Mathematical Foundation](#mathematical-foundation)
- [Implementation Examples](#implementation-examples)

## Core Concepts

### What Makes DLMM Different?

Traditional AMMs spread liquidity across the entire price curve (0 to ∞), while DLMM allows liquidity concentration in specific price "bins".

```
Traditional AMM:
[=========================================] 0 → ∞
     Liquidity spread across entire range

DLMM:
[    ][    ][████][████][████][    ][    ] Discrete bins
              Concentrated liquidity
```

### Key Advantages

1. **Capital Efficiency**: Up to 4000x more efficient than xy=k AMMs
2. **Zero Slippage Within Bins**: Fixed price per bin
3. **Flexible Fee Tiers**: Dynamic fees based on volatility
4. **Limit Order Capability**: Single-bin positions act as limit orders
5. **Reduced Impermanent Loss**: When properly managed

## Bin Architecture

### Understanding Bins

Each bin represents a discrete price level where liquidity can be deposited.

```typescript
interface Bin {
  id: number;           // Unique identifier
  price: number;        // Fixed price for this bin
  liquidityX: bigint;   // Amount of token X
  liquidityY: bigint;   // Amount of token Y
  totalLiquidity: bigint; // Total liquidity units
}
```

### Bin Spacing (Bin Step)

Bin step determines the price difference between adjacent bins:

```typescript
// Calculate price at bin
function getPriceAtBin(binId: number, binStep: number): number {
  const base = 1 + binStep / 10000; // binStep in basis points
  return Math.pow(base, binId);
}

// Example bin steps:
// 1 bps (0.01%) - For stablecoins
// 10 bps (0.10%) - For correlated assets
// 20 bps (0.20%) - For major pairs
// 100 bps (1.00%) - For volatile pairs
```

### Bin Composition

Bins contain different token ratios based on their position relative to the active bin:

```typescript
function getBinComposition(binId: number, activeBinId: number): {
  tokenX: number;
  tokenY: number;
} {
  if (binId < activeBinId) {
    // Below active: 100% token Y (quote token)
    return { tokenX: 0, tokenY: 1 };
  } else if (binId > activeBinId) {
    // Above active: 100% token X (base token)
    return { tokenX: 1, tokenY: 0 };
  } else {
    // Active bin: Mixed composition
    // Actual ratio depends on trades within the bin
    return { tokenX: 0.5, tokenY: 0.5 }; // Simplified
  }
}
```

## Price Discovery

### Active Bin Mechanism

The active bin determines the current market price:

```typescript
class ActiveBinManager {
  private activeBinId: number;
  private bins: Map<number, Bin>;
  
  // Price moves when active bin liquidity depletes
  moveToNextBin(direction: 'up' | 'down') {
    if (direction === 'up') {
      // Buy pressure depleted bin Y liquidity
      this.activeBinId++;
    } else {
      // Sell pressure depleted bin X liquidity
      this.activeBinId--;
    }
    
    // Emit price update event
    this.emitPriceUpdate(this.getPriceAtBin(this.activeBinId));
  }
  
  getCurrentPrice(): number {
    return this.getPriceAtBin(this.activeBinId);
  }
}
```

### Swap Execution Through Bins

```typescript
class BinSwapExecutor {
  async executeSwap(
    amountIn: bigint,
    tokenIn: Token,
    minAmountOut: bigint
  ): Promise<SwapResult> {
    let remainingInput = amountIn;
    let totalOutput = 0n;
    let currentBinId = this.activeBinId;
    
    const swapPath: BinSwap[] = [];
    
    while (remainingInput > 0n) {
      const bin = this.getBin(currentBinId);
      
      // Calculate swap within this bin
      const binSwap = this.swapWithinBin(
        bin,
        remainingInput,
        tokenIn
      );
      
      swapPath.push(binSwap);
      totalOutput += binSwap.amountOut;
      remainingInput -= binSwap.amountIn;
      
      // Move to next bin if current is depleted
      if (binSwap.binDepleted) {
        currentBinId = tokenIn === Token.X ? 
          currentBinId - 1 : currentBinId + 1;
      } else {
        break; // Swap complete
      }
    }
    
    // Check slippage
    if (totalOutput < minAmountOut) {
      throw new Error('Slippage exceeded');
    }
    
    return {
      amountIn,
      amountOut: totalOutput,
      swapPath,
      finalPrice: this.getPriceAtBin(currentBinId)
    };
  }
}
```

## Liquidity Strategies

### 1. Uniform Distribution

Spread liquidity evenly across bins:

```typescript
function createUniformPosition(
  lowerBinId: number,
  upperBinId: number,
  totalLiquidity: bigint
): BinLiquidity[] {
  const numBins = upperBinId - lowerBinId + 1;
  const liquidityPerBin = totalLiquidity / BigInt(numBins);
  
  const distribution: BinLiquidity[] = [];
  
  for (let binId = lowerBinId; binId <= upperBinId; binId++) {
    distribution.push({
      binId,
      liquidity: liquidityPerBin
    });
  }
  
  return distribution;
}
```

### 2. Normal Distribution

Concentrate liquidity around expected price:

```typescript
function createNormalDistribution(
  centerBinId: number,
  stdDev: number,
  totalLiquidity: bigint,
  numBins: number
): BinLiquidity[] {
  const distribution: BinLiquidity[] = [];
  const weights: number[] = [];
  
  // Calculate normal distribution weights
  for (let i = 0; i < numBins; i++) {
    const binId = centerBinId - Math.floor(numBins / 2) + i;
    const z = (binId - centerBinId) / stdDev;
    const weight = Math.exp(-0.5 * z * z);
    weights.push(weight);
  }
  
  // Normalize weights
  const sumWeights = weights.reduce((a, b) => a + b, 0);
  
  // Distribute liquidity
  for (let i = 0; i < numBins; i++) {
    const binId = centerBinId - Math.floor(numBins / 2) + i;
    const liquidity = BigInt(
      Math.floor(Number(totalLiquidity) * weights[i] / sumWeights)
    );
    
    distribution.push({ binId, liquidity });
  }
  
  return distribution;
}
```

### 3. Exponential Distribution

For directional strategies:

```typescript
function createExponentialDistribution(
  startBinId: number,
  numBins: number,
  totalLiquidity: bigint,
  lambda: number = 0.5
): BinLiquidity[] {
  const distribution: BinLiquidity[] = [];
  const weights: number[] = [];
  
  // Calculate exponential weights
  for (let i = 0; i < numBins; i++) {
    weights.push(Math.exp(-lambda * i));
  }
  
  const sumWeights = weights.reduce((a, b) => a + b, 0);
  
  // Distribute liquidity
  for (let i = 0; i < numBins; i++) {
    const liquidity = BigInt(
      Math.floor(Number(totalLiquidity) * weights[i] / sumWeights)
    );
    
    distribution.push({
      binId: startBinId + i,
      liquidity
    });
  }
  
  return distribution;
}
```

### 4. Custom Shape Distribution

For complex strategies:

```typescript
class CustomDistribution {
  createBimodal(
    peak1: number,
    peak2: number,
    totalLiquidity: bigint
  ): BinLiquidity[] {
    // Create two normal distributions
    const dist1 = createNormalDistribution(
      peak1, 2, totalLiquidity / 2n, 10
    );
    const dist2 = createNormalDistribution(
      peak2, 2, totalLiquidity / 2n, 10
    );
    
    // Merge distributions
    return this.mergeDistributions([dist1, dist2]);
  }
  
  createSkewed(
    center: number,
    skew: number,
    totalLiquidity: bigint
  ): BinLiquidity[] {
    // Implement skewed distribution
    // More liquidity on one side
  }
}
```

## Mathematical Foundation

### Price Calculation

The price at each bin follows a geometric progression:

```
P(i) = P(0) × (1 + s)^i

Where:
- P(i) = Price at bin i
- P(0) = Base price (bin 0)
- s = bin step (in decimal)
- i = bin ID
```

### Liquidity Math

```typescript
class LiquidityMath {
  // Calculate liquidity from token amounts
  static getLiquidityFromAmounts(
    amountX: bigint,
    amountY: bigint,
    binId: number,
    activeBinId: number
  ): bigint {
    if (binId < activeBinId) {
      // Only Y tokens
      return amountY;
    } else if (binId > activeBinId) {
      // Only X tokens
      return amountX;
    } else {
      // Active bin: use geometric mean
      return sqrt(amountX * amountY);
    }
  }
  
  // Calculate token amounts from liquidity
  static getAmountsFromLiquidity(
    liquidity: bigint,
    binId: number,
    activeBinId: number,
    binPrice: number
  ): { amountX: bigint; amountY: bigint } {
    if (binId < activeBinId) {
      return { amountX: 0n, amountY: liquidity };
    } else if (binId > activeBinId) {
      return { amountX: liquidity, amountY: 0n };
    } else {
      // Active bin: split based on current composition
      const ratio = this.getActiveBinRatio();
      return {
        amountX: liquidity * BigInt(ratio.x * 1000) / 1000n,
        amountY: liquidity * BigInt(ratio.y * 1000) / 1000n
      };
    }
  }
}
```

### Fee Calculation

```typescript
class FeeCalculator {
  private baseFee: number = 0.003; // 0.3%
  private volatilityMultiplier: number = 1.0;
  
  calculateFee(
    volume: bigint,
    volatility: number
  ): bigint {
    // Dynamic fee based on volatility
    const dynamicFee = this.baseFee * (1 + volatility * this.volatilityMultiplier);
    
    // Cap at maximum fee
    const finalFee = Math.min(dynamicFee, 0.01); // 1% max
    
    return volume * BigInt(Math.floor(finalFee * 10000)) / 10000n;
  }
  
  calculateLPShare(
    positionLiquidity: bigint,
    binTotalLiquidity: bigint,
    binFees: bigint
  ): bigint {
    if (binTotalLiquidity === 0n) return 0n;
    
    return binFees * positionLiquidity / binTotalLiquidity;
  }
}
```

## Implementation Examples

### 1. Creating a DLMM Position

```typescript
async function createDLMMPosition() {
  const dlmm = new DLMMClient(connection, wallet);
  
  // Get current market state
  const pool = await dlmm.getPool(POOL_ADDRESS);
  const currentPrice = pool.getCurrentPrice();
  const activeBinId = pool.activeBinId;
  
  // Define range (±5% from current price)
  const lowerPrice = currentPrice * 0.95;
  const upperPrice = currentPrice * 1.05;
  
  // Convert prices to bin IDs
  const lowerBinId = dlmm.priceToBinId(lowerPrice, pool.binStep);
  const upperBinId = dlmm.priceToBinId(upperPrice, pool.binStep);
  
  // Create position with normal distribution
  const position = await dlmm.createPosition({
    poolAddress: pool.address,
    lowerBinId,
    upperBinId,
    totalLiquidity: parseUnits('1000', 6), // 1000 USDC
    distribution: {
      type: 'NORMAL',
      parameters: {
        mean: activeBinId,
        stdDev: 5 // 5 bins standard deviation
      }
    }
  });
  
  console.log('Position created:', position.id);
  console.log('Bins covered:', upperBinId - lowerBinId + 1);
}
```

### 2. Implementing Range Orders

```typescript
class RangeOrderManager {
  async createLimitBuyOrder(
    pool: Pool,
    targetPrice: number,
    amount: bigint
  ): Promise<Position> {
    const targetBinId = this.priceToBinId(targetPrice, pool.binStep);
    
    // Create single-bin position (acts as limit order)
    return await this.dlmm.createPosition({
      poolAddress: pool.address,
      lowerBinId: targetBinId,
      upperBinId: targetBinId, // Same bin = limit order
      totalLiquidity: amount,
      distribution: {
        type: 'SPOT' // All liquidity in one bin
      }
    });
  }
  
  async monitorOrderExecution(position: Position) {
    const subscription = this.dlmm.subscribeToPosition(
      position.id,
      (update) => {
        if (update.liquidity === 0n) {
          console.log('Order filled!');
          this.handleOrderFilled(position);
        } else if (update.liquidity < position.initialLiquidity) {
          const fillPercent = (1 - Number(update.liquidity) / 
            Number(position.initialLiquidity)) * 100;
          console.log(`Order ${fillPercent.toFixed(2)}% filled`);
        }
      }
    );
    
    return subscription;
  }
}
```

### 3. Dynamic Fee Adjustment

```typescript
class DynamicFeeManager {
  private readonly VOLATILITY_WINDOW = 3600; // 1 hour
  
  async adjustPoolFees(pool: Pool) {
    // Calculate recent volatility
    const volatility = await this.calculateVolatility(
      pool,
      this.VOLATILITY_WINDOW
    );
    
    // Determine optimal fee tier
    let optimalFee: number;
    if (volatility < 0.001) {
      optimalFee = 0.0001; // 0.01% for stable
    } else if (volatility < 0.01) {
      optimalFee = 0.0005; // 0.05% for low volatility
    } else if (volatility < 0.05) {
      optimalFee = 0.003;  // 0.3% for medium
    } else {
      optimalFee = 0.01;   // 1% for high volatility
    }
    
    // Update pool fee if needed
    if (Math.abs(pool.feeRate - optimalFee) > 0.0001) {
      await this.updatePoolFee(pool, optimalFee);
    }
  }
  
  private async calculateVolatility(
    pool: Pool,
    windowSeconds: number
  ): Promise<number> {
    const prices = await this.getPriceHistory(pool, windowSeconds);
    
    // Calculate returns
    const returns = [];
    for (let i = 1; i < prices.length; i++) {
      returns.push(Math.log(prices[i] / prices[i - 1]));
    }
    
    // Calculate standard deviation
    const mean = returns.reduce((a, b) => a + b, 0) / returns.length;
    const variance = returns.reduce(
      (sum, r) => sum + Math.pow(r - mean, 2), 0
    ) / returns.length;
    
    return Math.sqrt(variance);
  }
}
```

### 4. Yield Optimization

```typescript
class YieldOptimizer {
  async optimizePosition(position: Position) {
    const pool = await this.dlmm.getPool(position.poolAddress);
    const metrics = await this.analyzePosition(position);
    
    // Check if rebalancing needed
    if (metrics.outOfRangeTime > 0.3) { // 30% time out of range
      await this.rebalancePosition(position, pool);
    }
    
    // Compound fees if profitable
    if (metrics.unclaimedFees > this.minCompoundAmount) {
      await this.compoundFees(position);
    }
    
    // Adjust range based on volatility
    if (metrics.volatilityChange > 0.5) { // 50% volatility change
      await this.adjustRange(position, metrics.currentVolatility);
    }
  }
  
  private async rebalancePosition(
    position: Position,
    pool: Pool
  ) {
    // Remove liquidity
    const removed = await this.dlmm.removeLiquidity(position.id);
    
    // Calculate new range
    const newRange = this.calculateOptimalRange(
      pool.currentPrice,
      pool.volatility
    );
    
    // Create new position
    return await this.dlmm.createPosition({
      poolAddress: pool.address,
      lowerBinId: newRange.lower,
      upperBinId: newRange.upper,
      totalLiquidity: removed.amountX + removed.amountY,
      distribution: { type: 'NORMAL' }
    });
  }
}
```

## Advanced Topics

### Bin Merging and Splitting

For pools with adaptive bin steps:

```typescript
class AdaptiveBinManager {
  async mergeBins(pool: Pool, binIds: number[]) {
    // Combine multiple bins into one larger bin
    const totalLiquidity = await this.sumBinLiquidity(binIds);
    const avgPrice = await this.calculateWeightedAvgPrice(binIds);
    
    return await this.createMergedBin(avgPrice, totalLiquidity);
  }
  
  async splitBin(pool: Pool, binId: number, parts: number) {
    // Split one bin into multiple smaller bins
    const bin = await this.getBin(binId);
    const liquidityPerPart = bin.liquidity / BigInt(parts);
    
    const newBins = [];
    for (let i = 0; i < parts; i++) {
      newBins.push(await this.createSubBin(
        binId,
        i,
        liquidityPerPart
      ));
    }
    
    return newBins;
  }
}
```

### Cross-Bin Arbitrage

```typescript
class CrossBinArbitrage {
  async findArbitrage(pool: Pool): Promise<ArbitrageOpp | null> {
    const bins = await pool.getAllBins();
    
    for (let i = 0; i < bins.length - 1; i++) {
      const currentBin = bins[i];
      const nextBin = bins[i + 1];
      
      // Check for price discontinuity
      const expectedPrice = this.getExpectedPrice(
        currentBin.id + 1,
        pool.binStep
      );
      
      if (Math.abs(nextBin.price - expectedPrice) > 0.001) {
        return {
          buyBin: currentBin,
          sellBin: nextBin,
          profit: this.calculateProfit(currentBin, nextBin)
        };
      }
    }
    
    return null;
  }
}
```

## Performance Considerations

1. **Bin Selection**: Choose appropriate bin steps for your use case
2. **Gas Optimization**: Batch operations when managing multiple bins
3. **Rebalancing Frequency**: Balance between staying in range and gas costs
4. **Distribution Strategy**: Match distribution to expected price movement
5. **Fee Tier Selection**: Higher fees for volatile pairs, lower for stable

## Conclusion

DLMM represents a significant advancement in AMM design, offering unprecedented capital efficiency and flexibility. By understanding bins, distributions, and optimization strategies, you can maximize returns while minimizing risks.

## Resources

- [DLMM Whitepaper](https://docs.saros.finance/dlmm-whitepaper)
- [Interactive Bin Visualizer](/tools/bin-visualizer)
- [Strategy Backtester](/tools/strategy-backtester)
- [GitHub Examples](https://github.com/saros-finance/dlmm-examples)