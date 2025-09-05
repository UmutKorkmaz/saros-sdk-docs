# AMM vs DLMM: Understanding Liquidity Models

Saros offers two powerful liquidity engines: **AMM (Automated Market Maker)** and **DLMM (Dynamic Liquidity Market Maker)**. Understanding the differences helps you choose the right approach for your application.

## Overview Comparison

| Feature | AMM | DLMM |
|---------|-----|------|
| **Liquidity Distribution** | Uniform across all prices | Concentrated in specific ranges |
| **Capital Efficiency** | Lower | Up to 20x higher |
| **Complexity** | Simple | Advanced |
| **Gas Costs** | Lower | Higher |
| **Impermanent Loss** | Standard | Reduced in range |
| **Best For** | General trading | Professional LPs |

### Visual Comparison: Liquidity Distribution

```mermaid
graph TB
    subgraph "AMM Model"
        AMM_Price[Price Range: $0 → $∞]
        AMM_Liq["Liquidity: ████████████████"]
        AMM_Desc[Uniform distribution]
        AMM_Eff["Capital Efficiency: ~0.1%"]
        
        AMM_Price --> AMM_Liq
        AMM_Liq --> AMM_Desc
        AMM_Desc --> AMM_Eff
    end
    
    subgraph "DLMM Model"
        DLMM_Price["Price Bins: $95 $96 $97 $98 $99 $100 $101 $102 $103"]
        DLMM_Liq["Liquidity: █ ██ ███ ████ ████ ███ ██ █"]
        DLMM_Desc[Concentrated around current price]
        DLMM_Eff["Capital Efficiency: up to 20x"]
        
        DLMM_Price --> DLMM_Liq
        DLMM_Liq --> DLMM_Desc
        DLMM_Desc --> DLMM_Eff
    end
    
    style AMM_Eff fill:#ffeb3b
    style DLMM_Eff fill:#4caf50
```

### Decision Matrix

```mermaid
flowchart TD
    Start[Choose Liquidity Model] --> Question1{Need Maximum Capital Efficiency?}
    
    Question1 -->|Yes| Question2{Can Actively Manage Position?}
    Question1 -->|No| Question3{Prefer Simplicity?}
    
    Question2 -->|Yes| DLMM[Choose DLMM]
    Question2 -->|No| AMM1[Choose AMM]
    
    Question3 -->|Yes| AMM2[Choose AMM]
    Question3 -->|No| Question4{Working with Stable Pairs?}
    
    Question4 -->|Yes| DLMM2[Consider DLMM]
    Question4 -->|No| AMM3[Choose AMM]
    
    DLMM --> Benefits1["✅ 20x Capital Efficiency<br/>✅ Higher Fee Earnings<br/>✅ Range Orders<br/>❌ Complex Management"]
    AMM1 --> Benefits2["✅ Simple & Passive<br/>✅ Always Earning<br/>✅ Low Gas Costs<br/>❌ Lower Efficiency"]
    AMM2 --> Benefits2
    DLMM2 --> Benefits1
    AMM3 --> Benefits2
    
    style DLMM fill:#4caf50,color:#fff
    style DLMM2 fill:#4caf50,color:#fff
    style AMM1 fill:#2196f3,color:#fff
    style AMM2 fill:#2196f3,color:#fff
    style AMM3 fill:#2196f3,color:#fff
```

## Automated Market Maker (AMM) 

### How AMM Works

Traditional AMM model where liquidity is distributed uniformly across the entire price curve.

```
Price Range: $0 → $∞
Liquidity: ████████████████████████████████
           Uniform distribution
```

### Key Characteristics

**Continuous Liquidity**
- Always provides liquidity at every price point
- Liquidity never runs out (theoretically)
- Familiar x * y = k formula

**Predictable Behavior**  
- Simple to understand and implement
- Lower gas costs for operations
- Straightforward impermanent loss calculations

**Lower Capital Efficiency**
- Only ~0.1% of capital is active near current price
- Most liquidity sits unused outside trading range
- Higher slippage for large trades

### When to Use AMM

✅ **Choose AMM when:**
- Building simple swap interfaces
- Need predictable behavior
- Working with volatile asset pairs
- Prioritizing low gas costs
- Users are retail/casual traders

❌ **Avoid AMM when:**
- Need maximum capital efficiency
- Working with stable/correlated pairs
- Users are sophisticated LPs
- High-volume trading requirements

### AMM Implementation Example

```typescript
import { getSwapAmountSaros, swapSaros } from '@saros-finance/sdk';

// Simple AMM swap
const swapResult = await swapSaros(
  connection,
  fromTokenAccount,
  toTokenAccount,
  amount,
  minimumReceived,
  null, // No referrer
  poolAddress,
  SAROS_SWAP_PROGRAM,
  wallet.publicKey,
  fromMint,
  toMint
);
```

## Dynamic Liquidity Market Maker (DLMM)

### How DLMM Works

Advanced model using discrete "bins" where liquidity providers can concentrate capital in specific price ranges.

```
Price Bins:  $95  $96  $97  $98  $99  $100 $101 $102 $103
Liquidity:    █    ██   ███  ████ ████  ████ ███  ██   █
             Concentrated around current price
```

### Key Characteristics

**Bin-Based Liquidity**
- Liquidity divided into discrete price bins
- Each bin represents a small price range (e.g., 1% width)
- LPs choose exactly which bins to provide liquidity to

**Capital Efficiency**
- Up to 20x more capital efficient than traditional AMM
- Most capital concentrated near current market price
- Lower slippage for trades within active range

**Advanced Strategies**
- Range orders (limit orders using LP positions)
- Custom liquidity shapes
- Fee tier optimization
- Automated rebalancing

### Bin Structure

```typescript
// Each bin has specific properties
interface Bin {
  binId: number;           // Unique bin identifier  
  price: number;           // Center price of bin
  liquidity: BN;           // Amount of liquidity
  feeX: BN;                // Accumulated fees for token X
  feeY: BN;                // Accumulated fees for token Y
  reserveX: BN;            // Reserve of token X
  reserveY: BN;            // Reserve of token Y
}
```

### Liquidity Distribution Patterns

**Uniform Distribution** (AMM-like)
```
████████████████████████████████
Spread across wide range
```

**Concentrated** (Capital efficient)
```
    ████████████████████████
   Focused around current price
```

**Spot** (Range orders)
```
              ████
         Single bin positions
```

**Curve** (Custom shapes)
```
   ██████████████████████████
     Custom distribution curve
```

### When to Use DLMM

✅ **Choose DLMM when:**
- Need maximum capital efficiency
- Working with stable/correlated pairs
- Users want advanced LP strategies
- Building professional trading tools
- Need range order functionality

❌ **Avoid DLMM when:**
- Building simple applications
- Gas costs are critical concern
- Users are inexperienced with DeFi
- Working with highly volatile pairs

### DLMM Implementation Example

```typescript
import { LiquidityBookServices, MODE } from '@saros-finance/dlmm-sdk';

const dlmmService = new LiquidityBookServices({ mode: MODE.MAINNET });

// Get optimized quote with price impact
const quote = await dlmmService.getQuote({
  amount: BigInt(1_000_000), // 1 USDC
  isExactInput: true,
  swapForY: true,
  pair: pairAddress,
  tokenBase: baseToken,
  tokenQuote: quoteToken,
  tokenBaseDecimal: 9,
  tokenQuoteDecimal: 6,
  slippage: 0.5
});

// Execute swap through optimal bins
const transaction = await dlmmService.swap({
  amount: quote.amount,
  otherAmountOffset: quote.otherAmountOffset,
  isExactInput: true,
  swapForY: true,
  pair: pairAddress,
  payer: wallet.publicKey
});
```

## Technical Deep Dive

### AMM Formula

Traditional constant product formula:
```
x * y = k

Where:
- x = reserves of token X
- y = reserves of token Y  
- k = constant product
```

Price calculation:
```
price = y / x
```

Swap calculation:
```
amountOut = (y * amountIn) / (x + amountIn)
```

### DLMM Bin Mathematics

Bin price calculation:
```typescript
// Price of bin i
price(i) = (1 + binStep)^i * basePrice

// Where binStep is the percentage width (e.g., 0.01 for 1%)
```

Liquidity concentration:
```typescript
// Active bin receives most trading volume
activeBin = getCurrentBinId(currentPrice);

// Liquidity in nearby bins
nearbyLiquidity = getBinsInRange(activeBin - 5, activeBin + 5);
```

### Fee Comparison

**AMM Fees**
```
Fixed fee: 0.25%
Applied to: Full trade amount
Distribution: Uniform to all LPs
```

**DLMM Fees**  
```
Variable fees: 0.1% - 1.0%
Applied to: Per bin traded through
Distribution: Only to active bins
Bonus: Volatility rewards
```

## Liquidity Provider Comparison

### AMM LP Experience

**Simple but Inefficient**
```typescript
// Provide liquidity to entire curve
await addLiquidity({
  tokenA: tokenAAmount,
  tokenB: tokenBAmount,
  // Liquidity spread across $0 to $∞
});

// Earn fees proportionally
// Most capital sits unused
```

### DLMM LP Experience

**Complex but Efficient**
```typescript
// Choose specific price ranges
await addLiquidityIntoPosition({
  tokenA: tokenAAmount,
  tokenB: tokenBAmount,
  binIds: [45, 46, 47, 48, 49], // Specific bins
  distributionX: [20, 25, 30, 15, 10], // Custom distribution
  distributionY: [10, 15, 30, 25, 20],
});

// Earn concentrated fees
// Higher capital efficiency
```

## Trading Experience Comparison

### AMM Trading
```typescript
// Simple swap interface
const amountOut = calculateAMMSwap(amountIn);
// Predictable slippage curve
// Always executable (with slippage)
```

### DLMM Trading  
```typescript
// Optimized routing through bins
const quote = await getOptimalDLMMRoute(amountIn);
// Dynamic pricing based on active bins  
// Better prices for trades within liquidity range
```

## Gas Cost Analysis

### Transaction Costs

| Operation | AMM | DLMM | Difference |
|-----------|-----|------|------------|
| **Swap** | ~0.005 SOL | ~0.008 SOL | +60% |
| **Add Liquidity** | ~0.006 SOL | ~0.012 SOL | +100% |
| **Remove Liquidity** | ~0.006 SOL | ~0.010 SOL | +67% |
| **Claim Fees** | ~0.003 SOL | ~0.005 SOL | +67% |

### Why DLMM Costs More

- More complex calculations
- Multiple bin interactions
- Additional state updates
- Bin rebalancing operations

## Use Case Decision Matrix

### Choose AMM For:

**DeFi Aggregators**
```typescript
// Simple integration, predictable behavior
const aggregatorSwap = await routeThroughAMM(tokenA, tokenB, amount);
```

**Mobile Apps**  
```typescript  
// Lower gas costs, simpler UX
const mobileSwap = await simpleAMMSwap(amount);
```

**Volatile Pairs**
```typescript
// ETH/MEME tokens - wide price ranges
const volatileSwap = await ammSwap('ETH', 'BONK', amount);
```

### Choose DLMM For:

**Stable Pairs**
```typescript
// USDC/USDT - tight ranges, high efficiency  
const stableSwap = await dlmmConcentratedSwap('USDC', 'USDT', amount);
```

**Professional Trading**
```typescript
// Advanced strategies, range orders
const proTrade = await dlmmRangeOrder(price, amount);
```

**Yield Optimization**
```typescript
// Maximize LP returns
const optimizedLP = await dlmmOptimalPosition(currentPrice);
```

## Migration Considerations

### From AMM to DLMM

```typescript
// 1. Remove AMM position
await removeAMMPosition(positionId);

// 2. Analyze current price and volatility
const analysis = await analyzePriceRange(tokenPair);

// 3. Create concentrated DLMM position
await addDLMMPosition({
  bins: analysis.optimalBins,
  distribution: analysis.optimalDistribution
});
```

### Hybrid Approach

Some applications use both:

```typescript
// AMM for volatile pairs
if (pair.volatility > 50) {
  return useAMM(pair);
}

// DLMM for stable pairs  
if (pair.volatility < 10) {
  return useDLMM(pair);
}

// User choice for medium volatility
return showBothOptions(pair);
```

## Performance Benchmarks

### Capital Efficiency Example

**$10,000 USDC/USDT Position**

AMM Results:
- Active capital: ~$100 (1%)
- Daily volume: $50,000
- Daily fees: $12.50
- APY: ~45%

DLMM Results (±2% range):
- Active capital: ~$2,000 (20%)  
- Daily volume: $50,000
- Daily fees: $25.00
- APY: ~90%

*Note: Results vary based on market conditions*

## Next Steps

Now that you understand the difference between AMM and DLMM:

1. [🧠 Learn Bin-Based Liquidity](./bin-liquidity.md) - DLMM deep dive
2. [⚡ Jupiter Integration](./jupiter-integration.md) - Router patterns
3. [📖 SDK Guides](../sdk-guides/typescript-sdk/swap-operations.md) - Implementation details
4. [📝 Tutorials](../tutorials/01-basic-swap.md) - Hands-on practice

## Resources

- [Saros App](https://app.saros.finance) - Try both AMM and DLMM
- [DLMM Research](https://docs.saros.finance/dlmm) - Technical specifications  
- [Community Discord](https://discord.gg/saros) - Ask questions
- [GitHub Examples](https://github.com/saros-finance/sdk-examples) - Code samples