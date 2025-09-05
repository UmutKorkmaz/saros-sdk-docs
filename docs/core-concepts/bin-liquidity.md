# Bin-Based Liquidity System

Deep dive into Saros DLMM's innovative bin-based liquidity architecture that enables up to 20x capital efficiency compared to traditional AMMs.

## What are Bins?

Bins are discrete price ranges where liquidity providers can deposit capital. Instead of spreading liquidity across the entire price curve (like traditional AMMs), DLMM allows concentrated liquidity placement in specific bins.

### Bin Structure

```typescript
interface Bin {
  binId: number;           // Unique identifier
  price: number;           // Center price of the bin
  liquidity: BN;           // Total liquidity in this bin
  reserveX: BN;            // Amount of token X
  reserveY: BN;            // Amount of token Y
  feeX: BN;                // Accumulated fees for token X
  feeY: BN;                // Accumulated fees for token Y
  compositionFee: BN;      // Variable fee component
  protocolFee: BN;         // Protocol fee accumulation
}
```

### Visual Representation

```
Traditional AMM:
Price: $90  $95  $100 $105 $110
Liq:   ████  ████  ████  ████  ████
       Uniform distribution

DLMM Bins:
Price: $90  $95  $100 $105 $110
Liq:    █   ███  ████  ███   █
       Concentrated around current price
```

## Bin Mathematics

### Bin Step Calculation

Each bin represents a specific price range determined by the `binStep`:

```typescript
// Bin step is the percentage width of each bin
const binStep = 0.0001; // 0.01% (1 basis point)

// Price of bin i
const binPrice = (1 + binStep) ** binId * baseFactor;

// Example for 1% bin step:
// Bin 0: $100.00
// Bin 1: $101.00  
// Bin 2: $102.01
// Bin -1: $99.00
```

### Price Range Per Bin

```typescript
function getBinPriceRange(binId: number, binStep: number): {min: number, max: number} {
  const center = (1 + binStep) ** binId;
  const halfStep = binStep / 2;
  
  return {
    min: center * (1 - halfStep),
    max: center * (1 + halfStep)
  };
}

// Example: Bin 100 with 0.01 binStep
// Range: $99.995 to $100.005
```

## Liquidity Distribution Strategies

### 1. Spot Liquidity (Single Bin)

Concentrated liquidity in one specific bin - acts like a limit order.

```typescript
// Single bin position at $100
await addLiquidityIntoPosition({
  binIds: [100],
  distributionX: [100], // 100% of token X
  distributionY: [0],   // 0% of token Y
  tokenX: usdcAmount,
  tokenY: solAmount,
});

// Result: Sell USDC for SOL when price hits $100
```

**Use Cases:**
- Limit orders
- Exit strategies  
- Arbitrage positions

### 2. Uniform Distribution

Spread liquidity evenly across a range - similar to AMM behavior.

```typescript
// Uniform distribution across 10 bins
await addLiquidityIntoPosition({
  binIds: [95, 96, 97, 98, 99, 100, 101, 102, 103, 104],
  distributionX: [10, 10, 10, 10, 10, 10, 10, 10, 10, 10],
  distributionY: [10, 10, 10, 10, 10, 10, 10, 10, 10, 10],
  tokenX: usdcAmount,
  tokenY: solAmount,
});
```

**Benefits:**
- Lower impermanent loss
- Consistent fee generation
- Less maintenance required

### 3. Normal Distribution (Bell Curve)

Concentrated around current price with gradual decrease.

```typescript
// Bell curve distribution
const normalDistribution = [
  { binId: 95, weightX: 5, weightY: 5 },
  { binId: 96, weightX: 10, weightY: 10 },
  { binId: 97, weightX: 15, weightY: 15 },
  { binId: 98, weightX: 20, weightY: 20 },
  { binId: 99, weightX: 25, weightY: 25 },
  { binId: 100, weightX: 25, weightY: 25 }, // Current price
  { binId: 101, weightX: 25, weightY: 25 },
  { binId: 102, weightX: 20, weightY: 20 },
  { binId: 103, weightX: 15, weightY: 15 },
  { binId: 104, weightX: 10, weightY: 10 },
  { binId: 105, weightX: 5, weightY: 5 },
];
```

**Benefits:**
- High capital efficiency
- Reduced impermanent loss risk
- Good for stable pairs

### 4. Bid-Ask Spread

Asymmetric distribution favoring one side.

```typescript
// More liquidity below current price (buying pressure)
const bidHeavyDistribution = [
  { binId: 95, weightX: 20, weightY: 5 },
  { binId: 96, weightX: 25, weightY: 8 },
  { binId: 97, weightX: 30, weightY: 12 },
  { binId: 98, weightX: 25, weightY: 15 },
  { binId: 99, weightX: 20, weightY: 20 },
  { binId: 100, weightX: 15, weightY: 25 }, // Current price
  { binId: 101, weightX: 8, weightY: 30 },
  { binId: 102, weightX: 5, weightY: 25 },
];
```

**Use Cases:**
- Market making strategies
- Directional bias positions
- Inventory management

## Advanced Bin Operations

### Dynamic Rebalancing

```typescript
class DynamicBinManager {
  async rebalancePosition(
    currentPrice: number,
    targetRange: { min: number, max: number }
  ) {
    // 1. Get current position
    const position = await this.getCurrentPosition();
    
    // 2. Check if rebalancing is needed
    const needsRebalancing = this.shouldRebalance(currentPrice, position);
    
    if (needsRebalancing) {
      // 3. Remove liquidity from out-of-range bins
      const outOfRangeBins = this.getOutOfRangeBins(position, targetRange);
      await this.removeLiquidityFromBins(outOfRangeBins);
      
      // 4. Add liquidity to new optimal bins
      const optimalBins = this.calculateOptimalBins(currentPrice, targetRange);
      await this.addLiquidityToBins(optimalBins);
    }
  }
  
  shouldRebalance(currentPrice: number, position: Position): boolean {
    const activeBins = position.bins.filter(bin => bin.liquidity > 0);
    const priceRange = this.getPriceRange(activeBins);
    
    // Rebalance if price moved outside 80% of range
    return currentPrice < priceRange.min * 1.2 || 
           currentPrice > priceRange.max * 0.8;
  }
}
```

### Fee Accumulation Tracking

```typescript
interface FeeTracker {
  binId: number;
  feeX: BN;
  feeY: BN;
  feeRate: number;
  volume24h: BN;
  apr: number;
}

async function trackBinFees(binIds: number[]): Promise<FeeTracker[]> {
  const feeData: FeeTracker[] = [];
  
  for (const binId of binIds) {
    const bin = await getBinData(binId);
    const volume = await get24hVolume(binId);
    
    // Calculate APR based on fees and liquidity
    const totalFees = bin.feeX.add(bin.feeY);
    const totalLiquidity = bin.reserveX.add(bin.reserveY);
    const apr = totalFees.div(totalLiquidity).mul(365).toNumber();
    
    feeData.push({
      binId,
      feeX: bin.feeX,
      feeY: bin.feeY,
      feeRate: bin.compositionFee.toNumber() / 10000, // basis points to percentage
      volume24h: volume,
      apr
    });
  }
  
  return feeData;
}
```

## Bin Trading Mechanics

### How Swaps Work Through Bins

```typescript
async function swapThroughBins(
  amountIn: BN,
  swapForY: boolean, // true = X to Y, false = Y to X
  pairAddress: PublicKey
) {
  // 1. Get current active bin
  const activeBinId = await getActiveBinId(pairAddress);
  
  // 2. Start swap from active bin
  let remainingAmount = amountIn;
  let totalAmountOut = new BN(0);
  let currentBinId = activeBinId;
  
  // 3. Iterate through bins until amount is fully swapped
  while (remainingAmount.gt(new BN(0))) {
    const bin = await getBinData(currentBinId);
    
    // Check if bin has liquidity for this direction
    const availableLiquidity = swapForY ? bin.reserveX : bin.reserveY;
    
    if (availableLiquidity.gt(new BN(0))) {
      // Calculate swap amount for this bin
      const swapAmount = BN.min(remainingAmount, availableLiquidity);
      const amountOut = calculateSwapAmountInBin(swapAmount, bin, swapForY);
      
      totalAmountOut = totalAmountOut.add(amountOut);
      remainingAmount = remainingAmount.sub(swapAmount);
    }
    
    // Move to next bin
    currentBinId = swapForY ? currentBinId + 1 : currentBinId - 1;
  }
  
  return totalAmountOut;
}
```

### Price Impact Calculation

```typescript
function calculatePriceImpact(
  amountIn: BN,
  bins: Bin[],
  swapForY: boolean
): number {
  // Initial price (from active bin)
  const initialPrice = bins[0].price;
  
  // Simulate swap to get final price
  const finalBin = simulateSwap(amountIn, bins, swapForY);
  const finalPrice = finalBin.price;
  
  // Calculate price impact
  const priceImpact = Math.abs(finalPrice - initialPrice) / initialPrice;
  
  return priceImpact * 100; // Return as percentage
}
```

## Variable Fee Model

DLMM implements dynamic fees based on market volatility:

```typescript
interface VariableFeeParameters {
  baseFee: number;        // Base fee in basis points
  maxVolatilityFee: number; // Maximum additional fee
  volatilityReference: number; // Reference volatility
  decayPeriod: number;    // Fee decay time in seconds
}

function calculateVariableFee(
  params: VariableFeeParameters,
  currentVolatility: number
): number {
  // Base fee always applies
  let totalFee = params.baseFee;
  
  // Add volatility component
  if (currentVolatility > params.volatilityReference) {
    const excessVolatility = currentVolatility - params.volatilityReference;
    const volatilityFee = Math.min(
      excessVolatility * params.maxVolatilityFee,
      params.maxVolatilityFee
    );
    totalFee += volatilityFee;
  }
  
  return totalFee;
}

// Example: 
// Base fee: 10 basis points (0.1%)
// High volatility adds up to 100 basis points (1.0%)
// Total range: 0.1% to 1.1%
```

## Liquidity Provider Rewards

### Fee Distribution

```typescript
interface LPRewards {
  tradingFees: BN;        // Fees from swaps
  volatilityRewards: BN;  // Bonus for providing liquidity during volatility
  protocolRewards: BN;    // Additional protocol incentives
}

function calculateLPRewards(
  position: Position,
  binActivity: BinActivity[]
): LPRewards {
  let totalFees = new BN(0);
  let volatilityRewards = new BN(0);
  
  for (const bin of position.bins) {
    const activity = binActivity.find(a => a.binId === bin.binId);
    if (!activity) continue;
    
    // Trading fees proportional to liquidity share
    const liquidityShare = bin.liquidity.div(activity.totalLiquidity);
    const binFees = activity.totalFees.mul(liquidityShare);
    totalFees = totalFees.add(binFees);
    
    // Volatility rewards for active bins during high volatility
    if (activity.volatility > VOLATILITY_THRESHOLD) {
      const reward = binFees.mul(new BN(2)); // 2x multiplier
      volatilityRewards = volatilityRewards.add(reward);
    }
  }
  
  return {
    tradingFees: totalFees,
    volatilityRewards,
    protocolRewards: new BN(0), // Calculated separately
  };
}
```

### Impermanent Loss in Bins

```typescript
function calculateBinImpermanentLoss(
  initialPrice: number,
  currentPrice: number,
  initialTokenX: BN,
  initialTokenY: BN,
  currentTokenX: BN,
  currentTokenY: BN
): number {
  // Value if held (no LP)
  const holdValue = initialTokenX.toNumber() + 
    initialTokenY.toNumber() * initialPrice;
  
  // Current LP position value
  const lpValue = currentTokenX.toNumber() + 
    currentTokenY.toNumber() * currentPrice;
  
  // Calculate impermanent loss
  const impermanentLoss = (lpValue - holdValue) / holdValue;
  
  return impermanentLoss * 100; // Return as percentage
}

// Note: IL is typically lower in DLMM due to concentrated ranges
```

## Bin Management Strategies

### 1. Conservative Strategy (Wide Range)

```typescript
const conservativeStrategy = {
  range: { min: currentPrice * 0.8, max: currentPrice * 1.2 }, // ±20%
  distribution: 'normal',
  rebalanceThreshold: 0.15, // 15% price movement
  targetUtilization: 0.7,   // 70% of liquidity active
};
```

### 2. Aggressive Strategy (Narrow Range)

```typescript
const aggressiveStrategy = {
  range: { min: currentPrice * 0.95, max: currentPrice * 1.05 }, // ±5%
  distribution: 'concentrated',
  rebalanceThreshold: 0.03, // 3% price movement
  targetUtilization: 0.9,   // 90% of liquidity active
};
```

### 3. Range Order Strategy

```typescript
const rangeOrderStrategy = {
  // Sell order: only provide token X above current price
  sellRange: { min: currentPrice * 1.02, max: currentPrice * 1.1 },
  // Buy order: only provide token Y below current price  
  buyRange: { min: currentPrice * 0.9, max: currentPrice * 0.98 },
};
```

## Implementation Examples

### Creating a Concentrated Position

```typescript
import { LiquidityBookServices } from '@saros-finance/dlmm-sdk';

async function createConcentratedPosition(
  dlmmService: LiquidityBookServices,
  pairAddress: PublicKey,
  centerPrice: number,
  rangePercent: number,
  amountX: BN,
  amountY: BN
) {
  // Calculate bin range
  const minPrice = centerPrice * (1 - rangePercent / 100);
  const maxPrice = centerPrice * (1 + rangePercent / 100);
  
  // Get bin IDs for the range
  const minBinId = await dlmmService.getBinIdFromPrice(pairAddress, minPrice);
  const maxBinId = await dlmmService.getBinIdFromPrice(pairAddress, maxPrice);
  
  // Create normal distribution
  const binIds = [];
  const distributionX = [];
  const distributionY = [];
  
  for (let binId = minBinId; binId <= maxBinId; binId++) {
    binIds.push(binId);
    
    // Bell curve weighting
    const distance = Math.abs(binId - (minBinId + maxBinId) / 2);
    const maxDistance = (maxBinId - minBinId) / 2;
    const weight = Math.exp(-(distance / maxDistance) ** 2);
    
    distributionX.push(Math.floor(weight * 100));
    distributionY.push(Math.floor(weight * 100));
  }
  
  // Add liquidity to position
  const transaction = await dlmmService.addLiquidityIntoPosition({
    pair: pairAddress,
    binIds,
    distributionX,
    distributionY,
    amountX,
    amountY,
    user: wallet.publicKey,
    slippage: 1.0, // 1% slippage tolerance
  });
  
  return transaction;
}
```

### Monitoring Bin Performance

```typescript
class BinMonitor {
  async getBinAnalytics(binId: number): Promise<BinAnalytics> {
    const bin = await getBinData(binId);
    const history = await getBinHistory(binId, 24 * 60 * 60); // 24 hours
    
    return {
      binId,
      currentPrice: bin.price,
      liquidity: bin.liquidity,
      volume24h: this.calculateVolume(history),
      fees24h: this.calculateFees(history),
      utilization: this.calculateUtilization(bin),
      apr: this.calculateAPR(bin, history),
      priceRange: this.getPriceRange(binId),
    };
  }
  
  async getOptimalBins(
    currentPrice: number,
    volatility: number,
    strategy: 'conservative' | 'aggressive' | 'balanced'
  ): Promise<number[]> {
    const strategies = {
      conservative: { range: 0.2, binCount: 20 },
      balanced: { range: 0.1, binCount: 15 },
      aggressive: { range: 0.05, binCount: 10 },
    };
    
    const config = strategies[strategy];
    const minPrice = currentPrice * (1 - config.range);
    const maxPrice = currentPrice * (1 + config.range);
    
    return this.getBinIdsInRange(minPrice, maxPrice, config.binCount);
  }
}
```

## Best Practices

### 1. Bin Selection
- **Stable pairs**: Use narrow ranges (±2-5%)
- **Volatile pairs**: Use wider ranges (±10-20%)
- **Trending markets**: Bias distribution toward trend direction

### 2. Rebalancing
- Set clear rebalancing triggers (price movement %)
- Consider gas costs vs potential gains
- Use automated rebalancing for active strategies

### 3. Fee Optimization
- Monitor fee accumulation across bins
- Concentrate liquidity in high-activity bins
- Consider variable fee impacts on profitability

### 4. Risk Management
- Set position size limits
- Diversify across multiple bin ranges
- Monitor impermanent loss regularly

## Next Steps

- [⚡ Jupiter Integration](./jupiter-integration.md) - Learn about routing
- [💼 Fee Structures](./fee-structures.md) - Understand fee mechanics
- [📖 DLMM SDK Guide](../sdk-guides/dlmm-sdk/position-mgmt.md) - Implementation details
- [📝 DLMM Tutorial](../tutorials/04-dlmm-positions.md) - Hands-on practice

## Resources

- [DLMM Whitepaper](https://docs.saros.finance/dlmm/whitepaper)
- [Bin Calculator Tool](https://app.saros.finance/tools/bin-calculator)
- [Community Strategies](https://discord.gg/saros)
- [Advanced Examples](https://github.com/saros-finance/dlmm-examples)