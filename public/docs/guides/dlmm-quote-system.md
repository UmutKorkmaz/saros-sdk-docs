# DLMM SDK: Quote System

Master the DLMM quote system to get accurate swap prices, calculate price impact, and optimize trade execution across concentrated liquidity bins.

## Overview

The DLMM quote system provides advanced price discovery across bin-based liquidity with:
- ✅ Real-time price calculations across active bins
- ✅ Accurate price impact assessment
- ✅ Slippage protection mechanisms
- ✅ Multi-bin routing optimization
- ✅ Variable fee calculations

## Understanding DLMM Quotes

### Quote Structure

```typescript
interface DLMMQuote {
  // Core amounts
  amount: BN;                    // Input amount
  otherAmountOffset: BN;         // Output amount with slippage
  
  // Price metrics
  priceImpact: number;           // Percentage price impact
  exchangeRate: number;          // Current exchange rate
  
  // Fee breakdown
  fee: BN;                       // Total fees in input token
  protocolFee: BN;              // Protocol portion of fees
  compositionFee: BN;           // Variable fee component
  
  // Routing information
  binArraysPubkey: PublicKey[];  // Bin arrays involved
  activeBinId: number;           // Current active bin
  binsTraversed: number;         // Number of bins crossed
  
  // Execution parameters
  minAmountOut: BN;             // Minimum output after slippage
  maxAmountIn: BN;              // Maximum input with slippage
}
```

### Quote Calculation Process

```typescript
import { LiquidityBookServices, MODE } from '@saros-finance/dlmm-sdk';
import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';

class DLMMQuoteEngine {
  private dlmmService: LiquidityBookServices;

  constructor(mode: MODE = MODE.MAINNET) {
    this.dlmmService = new LiquidityBookServices({ mode });
  }

  async getCompleteQuote(
    pairAddress: PublicKey,
    tokenBase: PublicKey,
    tokenQuote: PublicKey,
    amount: BN,
    isExactInput: boolean,
    slippagePercent: number = 0.5
  ): Promise<DLMMQuote> {
    console.log('🔍 Calculating DLMM quote...');
    
    // Step 1: Get pair information
    const pairInfo = await this.dlmmService.getPairAccount(pairAddress);
    console.log(`Active bin: ${pairInfo.activeBinId}`);
    console.log(`Bin step: ${pairInfo.binStep} basis points`);

    // Step 2: Calculate quote
    const quote = await this.dlmmService.getQuote({
      amount,
      isExactInput,
      swapForY: true, // Swap base for quote
      pair: pairAddress,
      tokenBase,
      tokenQuote,
      tokenBaseDecimal: 9,  // SOL decimals
      tokenQuoteDecimal: 6, // USDC decimals
      slippage: slippagePercent
    });

    // Step 3: Calculate additional metrics
    const exchangeRate = this.calculateExchangeRate(
      quote.amount,
      quote.otherAmountOffset,
      isExactInput
    );

    const binsTraversed = this.calculateBinsTraversed(
      pairInfo.activeBinId,
      quote.amount,
      pairInfo.binStep
    );

    console.log(`📊 Quote calculated:`);
    console.log(`  Exchange rate: ${exchangeRate.toFixed(6)}`);
    console.log(`  Price impact: ${quote.priceImpact}%`);
    console.log(`  Bins traversed: ${binsTraversed}`);
    console.log(`  Total fee: ${Number(quote.fee) / 1e6} tokens`);

    return {
      ...quote,
      exchangeRate,
      binsTraversed,
      activeBinId: pairInfo.activeBinId
    };
  }

  private calculateExchangeRate(
    amountIn: BN,
    amountOut: BN,
    isExactInput: boolean
  ): number {
    if (isExactInput) {
      return Number(amountOut) / Number(amountIn);
    } else {
      return Number(amountIn) / Number(amountOut);
    }
  }

  private calculateBinsTraversed(
    activeBinId: number,
    amount: BN,
    binStep: number
  ): number {
    // Simplified calculation - in production, track actual bins
    const priceImpact = 0.01; // 1% example
    const binsPerPercent = 100 / (binStep / 100);
    return Math.ceil(priceImpact * binsPerPercent);
  }
}
```

## Advanced Quote Features

### Multi-Path Quote Comparison

```typescript
interface QuotePath {
  route: PublicKey[];
  quotes: DLMMQuote[];
  totalAmountOut: BN;
  totalFee: BN;
  totalPriceImpact: number;
  isOptimal: boolean;
}

class MultiPathQuoteEngine {
  private quoteEngine: DLMMQuoteEngine;

  constructor() {
    this.quoteEngine = new DLMMQuoteEngine();
  }

  async findBestQuote(
    tokenIn: PublicKey,
    tokenOut: PublicKey,
    amount: BN,
    maxPaths: number = 3
  ): Promise<QuotePath> {
    console.log('🔍 Finding optimal quote path...');

    // Step 1: Find all available paths
    const paths = await this.findAvailablePaths(tokenIn, tokenOut, maxPaths);
    console.log(`Found ${paths.length} possible paths`);

    // Step 2: Get quotes for each path
    const quotePaths: QuotePath[] = [];

    for (const path of paths) {
      try {
        const quotePath = await this.getQuoteForPath(path, amount);
        quotePaths.push(quotePath);
      } catch (error) {
        console.log(`Path ${path.join(' → ')} failed:`, error);
      }
    }

    // Step 3: Select optimal path
    const optimalPath = this.selectOptimalPath(quotePaths);
    console.log(`✅ Optimal path selected with output: ${Number(optimalPath.totalAmountOut) / 1e6}`);

    return optimalPath;
  }

  private async findAvailablePaths(
    tokenIn: PublicKey,
    tokenOut: PublicKey,
    maxPaths: number
  ): Promise<PublicKey[][]> {
    const paths: PublicKey[][] = [];

    // Direct path
    paths.push([tokenIn, tokenOut]);

    // Common intermediate tokens for multi-hop
    const intermediates = [
      new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'), // USDC
      new PublicKey('So11111111111111111111111111111111111111112'),  // SOL
      new PublicKey('7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs'), // ETH
    ];

    for (const intermediate of intermediates) {
      if (!intermediate.equals(tokenIn) && !intermediate.equals(tokenOut)) {
        paths.push([tokenIn, intermediate, tokenOut]);
        
        if (paths.length >= maxPaths) break;
      }
    }

    return paths;
  }

  private async getQuoteForPath(
    path: PublicKey[],
    initialAmount: BN
  ): Promise<QuotePath> {
    const quotes: DLMMQuote[] = [];
    let currentAmount = initialAmount;
    let totalFee = new BN(0);
    let totalPriceImpact = 0;

    for (let i = 0; i < path.length - 1; i++) {
      const quote = await this.quoteEngine.getCompleteQuote(
        await this.findPair(path[i], path[i + 1]),
        path[i],
        path[i + 1],
        currentAmount,
        true, // Exact input
        0.5   // Slippage
      );

      quotes.push(quote);
      currentAmount = quote.otherAmountOffset;
      totalFee = totalFee.add(quote.fee);
      totalPriceImpact += quote.priceImpact;
    }

    return {
      route: path,
      quotes,
      totalAmountOut: currentAmount,
      totalFee,
      totalPriceImpact,
      isOptimal: false
    };
  }

  private selectOptimalPath(paths: QuotePath[]): QuotePath {
    let optimal = paths[0];
    
    for (const path of paths) {
      // Select path with best output amount
      if (path.totalAmountOut.gt(optimal.totalAmountOut)) {
        optimal = path;
      }
    }

    optimal.isOptimal = true;
    return optimal;
  }

  private async findPair(tokenA: PublicKey, tokenB: PublicKey): Promise<PublicKey> {
    // In production, query actual pair addresses
    // This is a placeholder
    return new PublicKey('DLMM_PAIR_ADDRESS_PLACEHOLDER');
  }
}
```

### Dynamic Slippage Calculation

```typescript
class DynamicSlippageCalculator {
  private readonly MIN_SLIPPAGE = 0.1;  // 0.1%
  private readonly MAX_SLIPPAGE = 5.0;  // 5%

  calculateOptimalSlippage(
    quote: DLMMQuote,
    volatility: number,
    tradeSize: 'small' | 'medium' | 'large'
  ): number {
    // Base slippage on price impact
    let slippage = quote.priceImpact * 2;

    // Adjust for volatility
    const volatilityMultiplier = 1 + (volatility / 100);
    slippage *= volatilityMultiplier;

    // Adjust for trade size
    const sizeMultipliers = {
      small: 1.0,
      medium: 1.5,
      large: 2.0
    };
    slippage *= sizeMultipliers[tradeSize];

    // Apply bounds
    slippage = Math.max(this.MIN_SLIPPAGE, Math.min(this.MAX_SLIPPAGE, slippage));

    console.log(`🎯 Recommended slippage: ${slippage.toFixed(2)}%`);
    return slippage;
  }

  adjustSlippageForRetry(
    previousSlippage: number,
    attempt: number
  ): number {
    // Exponential increase with each retry
    const newSlippage = Math.min(
      previousSlippage * Math.pow(1.5, attempt),
      this.MAX_SLIPPAGE
    );

    console.log(`🔄 Retry ${attempt}: Adjusting slippage to ${newSlippage.toFixed(2)}%`);
    return newSlippage;
  }
}
```

## Quote Validation and Simulation

### Pre-Trade Validation

```typescript
interface QuoteValidation {
  isValid: boolean;
  errors: string[];
  warnings: string[];
  recommendations: string[];
}

class QuoteValidator {
  validateQuote(
    quote: DLMMQuote,
    userBalance: BN,
    settings: {
      maxPriceImpact: number;
      maxSlippage: number;
      minOutput: BN;
    }
  ): QuoteValidation {
    const errors: string[] = [];
    const warnings: string[] = [];
    const recommendations: string[] = [];

    // Check balance
    if (quote.amount.gt(userBalance)) {
      errors.push(`Insufficient balance: ${Number(userBalance) / 1e6} < ${Number(quote.amount) / 1e6}`);
    }

    // Check price impact
    if (quote.priceImpact > settings.maxPriceImpact) {
      warnings.push(`High price impact: ${quote.priceImpact.toFixed(2)}%`);
      recommendations.push('Consider splitting the trade into smaller amounts');
    }

    // Check minimum output
    if (quote.otherAmountOffset.lt(settings.minOutput)) {
      errors.push(`Output below minimum: ${Number(quote.otherAmountOffset) / 1e6}`);
    }

    // Check bins traversed
    if (quote.binsTraversed > 10) {
      warnings.push(`Trade crosses ${quote.binsTraversed} bins`);
      recommendations.push('High bin traversal may indicate low liquidity');
    }

    // Check fees
    const feePercent = Number(quote.fee) / Number(quote.amount) * 100;
    if (feePercent > 1) {
      warnings.push(`High fees: ${feePercent.toFixed(2)}%`);
    }

    return {
      isValid: errors.length === 0,
      errors,
      warnings,
      recommendations
    };
  }

  async simulateTrade(
    quote: DLMMQuote,
    pairAddress: PublicKey
  ): Promise<{
    expectedOutput: BN;
    worstCaseOutput: BN;
    averagePrice: number;
    executionRisk: 'low' | 'medium' | 'high';
  }> {
    // Simulate across different market conditions
    const scenarios = [
      { name: 'normal', priceShift: 0 },
      { name: 'adverse', priceShift: -0.005 }, // 0.5% adverse movement
      { name: 'favorable', priceShift: 0.002 }  // 0.2% favorable movement
    ];

    let totalOutput = new BN(0);
    let worstCase = quote.otherAmountOffset;

    for (const scenario of scenarios) {
      const adjustedOutput = this.adjustOutputForScenario(
        quote.otherAmountOffset,
        scenario.priceShift
      );

      totalOutput = totalOutput.add(adjustedOutput);
      
      if (adjustedOutput.lt(worstCase)) {
        worstCase = adjustedOutput;
      }
    }

    const expectedOutput = totalOutput.divn(scenarios.length);
    const averagePrice = Number(quote.amount) / Number(expectedOutput);

    // Assess execution risk
    const risk = this.assessExecutionRisk(quote);

    return {
      expectedOutput,
      worstCaseOutput: worstCase,
      averagePrice,
      executionRisk: risk
    };
  }

  private adjustOutputForScenario(
    baseOutput: BN,
    priceShift: number
  ): BN {
    const adjustment = 1 + priceShift;
    return new BN(Number(baseOutput) * adjustment);
  }

  private assessExecutionRisk(quote: DLMMQuote): 'low' | 'medium' | 'high' {
    if (quote.priceImpact < 0.5 && quote.binsTraversed < 5) {
      return 'low';
    } else if (quote.priceImpact < 2 && quote.binsTraversed < 10) {
      return 'medium';
    } else {
      return 'high';
    }
  }
}
```

## Real-Time Quote Monitoring

### Quote Stream with WebSocket

```typescript
import { EventEmitter } from 'events';

interface QuoteUpdate {
  timestamp: Date;
  quote: DLMMQuote;
  change: {
    priceChange: number;
    liquidityChange: number;
  };
}

class QuoteMonitor extends EventEmitter {
  private dlmmService: LiquidityBookServices;
  private monitoring: Map<string, NodeJS.Timer> = new Map();

  constructor() {
    super();
    this.dlmmService = new LiquidityBookServices({ mode: MODE.MAINNET });
  }

  async startMonitoring(
    pairAddress: PublicKey,
    tokenBase: PublicKey,
    tokenQuote: PublicKey,
    amount: BN,
    intervalMs: number = 5000
  ): string {
    const monitorId = `${pairAddress.toString()}-${Date.now()}`;
    let previousQuote: DLMMQuote | null = null;

    const timer = setInterval(async () => {
      try {
        const quote = await this.dlmmService.getQuote({
          amount,
          isExactInput: true,
          swapForY: true,
          pair: pairAddress,
          tokenBase,
          tokenQuote,
          tokenBaseDecimal: 9,
          tokenQuoteDecimal: 6,
          slippage: 0.5
        });

        const update: QuoteUpdate = {
          timestamp: new Date(),
          quote: quote as DLMMQuote,
          change: {
            priceChange: previousQuote 
              ? (Number(quote.otherAmountOffset) - Number(previousQuote.otherAmountOffset)) / Number(previousQuote.otherAmountOffset)
              : 0,
            liquidityChange: 0 // Would need to track liquidity separately
          }
        };

        this.emit('quoteUpdate', update);

        // Check for significant changes
        if (previousQuote && Math.abs(update.change.priceChange) > 0.01) {
          this.emit('significantChange', update);
        }

        previousQuote = quote as DLMMQuote;

      } catch (error) {
        this.emit('error', { monitorId, error });
      }
    }, intervalMs);

    this.monitoring.set(monitorId, timer);
    console.log(`📊 Started monitoring ${monitorId}`);

    return monitorId;
  }

  stopMonitoring(monitorId: string): void {
    const timer = this.monitoring.get(monitorId);
    if (timer) {
      clearInterval(timer);
      this.monitoring.delete(monitorId);
      console.log(`🛑 Stopped monitoring ${monitorId}`);
    }
  }

  stopAllMonitoring(): void {
    for (const [monitorId, timer] of this.monitoring) {
      clearInterval(timer);
    }
    this.monitoring.clear();
    console.log('🛑 Stopped all monitoring');
  }
}

// Usage example
async function demonstrateQuoteMonitoring() {
  const monitor = new QuoteMonitor();

  monitor.on('quoteUpdate', (update: QuoteUpdate) => {
    console.log(`Quote update at ${update.timestamp.toISOString()}`);
    console.log(`  Output: ${Number(update.quote.otherAmountOffset) / 1e6}`);
    console.log(`  Change: ${(update.change.priceChange * 100).toFixed(2)}%`);
  });

  monitor.on('significantChange', (update: QuoteUpdate) => {
    console.log('⚠️ Significant price change detected!');
    // Could trigger rebalancing or alerts
  });

  const monitorId = await monitor.startMonitoring(
    new PublicKey('PAIR_ADDRESS'),
    new PublicKey('TOKEN_BASE'),
    new PublicKey('TOKEN_QUOTE'),
    new BN(1000000), // 1 token
    3000 // Check every 3 seconds
  );

  // Stop after 1 minute
  setTimeout(() => {
    monitor.stopMonitoring(monitorId);
  }, 60000);
}
```

## Quote Optimization Strategies

### Split Trade Optimization

```typescript
class TradeOptimizer {
  async optimizeLargeTrade(
    pairAddress: PublicKey,
    tokenBase: PublicKey,
    tokenQuote: PublicKey,
    totalAmount: BN,
    maxPriceImpact: number = 1.0 // 1%
  ): Promise<{
    splits: Array<{ amount: BN; quote: DLMMQuote }>;
    totalOutput: BN;
    averagePriceImpact: number;
  }> {
    console.log('🔄 Optimizing large trade with splits...');

    const splits: Array<{ amount: BN; quote: DLMMQuote }> = [];
    let remainingAmount = totalAmount;
    let totalOutput = new BN(0);
    let totalPriceImpact = 0;

    // Start with 10% chunks
    const chunkSize = totalAmount.divn(10);

    while (remainingAmount.gt(new BN(0))) {
      const tradeAmount = BN.min(chunkSize, remainingAmount);

      const quote = await this.dlmmService.getQuote({
        amount: tradeAmount,
        isExactInput: true,
        swapForY: true,
        pair: pairAddress,
        tokenBase,
        tokenQuote,
        tokenBaseDecimal: 9,
        tokenQuoteDecimal: 6,
        slippage: 0.5
      }) as DLMMQuote;

      if (quote.priceImpact <= maxPriceImpact) {
        splits.push({ amount: tradeAmount, quote });
        totalOutput = totalOutput.add(quote.otherAmountOffset);
        totalPriceImpact += quote.priceImpact;
        remainingAmount = remainingAmount.sub(tradeAmount);
      } else {
        // Reduce chunk size if price impact too high
        const reducedChunk = tradeAmount.divn(2);
        if (reducedChunk.gt(new BN(0))) {
          remainingAmount = remainingAmount.add(tradeAmount.sub(reducedChunk));
        } else {
          break; // Cannot split further
        }
      }
    }

    const averagePriceImpact = totalPriceImpact / splits.length;

    console.log(`✅ Trade split into ${splits.length} transactions`);
    console.log(`  Total output: ${Number(totalOutput) / 1e6}`);
    console.log(`  Average price impact: ${averagePriceImpact.toFixed(2)}%`);

    return {
      splits,
      totalOutput,
      averagePriceImpact
    };
  }

  async findOptimalTradeSize(
    pairAddress: PublicKey,
    tokenBase: PublicKey,
    tokenQuote: PublicKey,
    targetPriceImpact: number = 0.5 // 0.5%
  ): Promise<BN> {
    console.log(`🎯 Finding optimal trade size for ${targetPriceImpact}% price impact...`);

    let low = new BN(1000000);  // 1 token minimum
    let high = new BN(1000000000000); // 1M tokens maximum
    let optimal = low;

    // Binary search for optimal size
    while (low.lt(high)) {
      const mid = low.add(high).divn(2);

      const quote = await this.dlmmService.getQuote({
        amount: mid,
        isExactInput: true,
        swapForY: true,
        pair: pairAddress,
        tokenBase,
        tokenQuote,
        tokenBaseDecimal: 9,
        tokenQuoteDecimal: 6,
        slippage: 0.5
      }) as DLMMQuote;

      if (quote.priceImpact <= targetPriceImpact) {
        optimal = mid;
        low = mid.addn(1);
      } else {
        high = mid.subn(1);
      }
    }

    console.log(`✅ Optimal trade size: ${Number(optimal) / 1e9} tokens`);
    return optimal;
  }
}
```

## Usage Examples

### Complete Quote Implementation

```typescript
async function demonstrateQuoteSystem() {
  console.log('🚀 DLMM Quote System Demo');
  console.log('=========================\n');

  // Initialize services
  const quoteEngine = new DLMMQuoteEngine();
  const multiPath = new MultiPathQuoteEngine();
  const validator = new QuoteValidator();
  const optimizer = new TradeOptimizer();

  const pairAddress = new PublicKey('YOUR_PAIR_ADDRESS');
  const tokenBase = new PublicKey('So11111111111111111111111111111111111111112'); // SOL
  const tokenQuote = new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'); // USDC

  try {
    // 1. Get basic quote
    console.log('1️⃣ Getting basic quote...');
    const quote = await quoteEngine.getCompleteQuote(
      pairAddress,
      tokenBase,
      tokenQuote,
      new BN(1000000000), // 1 SOL
      true,
      0.5
    );

    console.log(`Quote: 1 SOL → ${Number(quote.otherAmountOffset) / 1e6} USDC`);

    // 2. Validate quote
    console.log('\n2️⃣ Validating quote...');
    const validation = validator.validateQuote(
      quote,
      new BN(10000000000), // User has 10 SOL
      {
        maxPriceImpact: 2,
        maxSlippage: 1,
        minOutput: new BN(30000000) // Minimum 30 USDC
      }
    );

    if (validation.isValid) {
      console.log('✅ Quote is valid');
    } else {
      console.log('❌ Validation errors:', validation.errors);
    }

    if (validation.warnings.length > 0) {
      console.log('⚠️ Warnings:', validation.warnings);
    }

    // 3. Find optimal path
    console.log('\n3️⃣ Finding optimal path...');
    const bestPath = await multiPath.findBestQuote(
      tokenBase,
      tokenQuote,
      new BN(1000000000),
      3
    );

    console.log(`Best route: ${bestPath.route.map(t => t.toString().slice(0, 8)).join(' → ')}`);
    console.log(`Output: ${Number(bestPath.totalAmountOut) / 1e6} USDC`);

    // 4. Optimize large trade
    console.log('\n4️⃣ Optimizing large trade...');
    const optimization = await optimizer.optimizeLargeTrade(
      pairAddress,
      tokenBase,
      tokenQuote,
      new BN(100000000000), // 100 SOL
      1.0 // Max 1% price impact per trade
    );

    console.log(`Split into ${optimization.splits.length} trades`);
    console.log(`Total output: ${Number(optimization.totalOutput) / 1e6} USDC`);
    console.log(`Average impact: ${optimization.averagePriceImpact.toFixed(2)}%`);

    // 5. Monitor quotes
    console.log('\n5️⃣ Starting quote monitoring...');
    const monitor = new QuoteMonitor();
    
    monitor.on('quoteUpdate', (update) => {
      console.log(`📊 ${update.timestamp.toISOString()}: ${Number(update.quote.otherAmountOffset) / 1e6} USDC`);
    });

    await monitor.startMonitoring(
      pairAddress,
      tokenBase,
      tokenQuote,
      new BN(1000000000),
      5000
    );

  } catch (error) {
    console.error('Demo failed:', error);
  }
}

// Run the demo
demonstrateQuoteSystem();
```

## Best Practices

### Quote Accuracy
- ✅ Always use fresh quotes before execution
- ✅ Account for network latency in calculations
- ✅ Validate quotes against expected ranges
- ✅ Monitor for stale pricing data

### Performance Optimization
- ✅ Cache quote results for repeated queries
- ✅ Batch quote requests when possible
- ✅ Use WebSocket for real-time updates
- ✅ Implement exponential backoff for retries

### Risk Management
- ✅ Set maximum acceptable price impact
- ✅ Implement minimum output thresholds
- ✅ Use dynamic slippage based on volatility
- ✅ Split large trades to minimize impact

## Troubleshooting

### Common Issues

**"Insufficient liquidity in bins"**
```typescript
// Check available liquidity before quoting
const pairInfo = await dlmmService.getPairAccount(pairAddress);
console.log('Active bin liquidity:', pairInfo.liquidityInActiveBin);
```

**"Price impact too high"**
```typescript
// Split trade or wait for better liquidity
const optimizer = new TradeOptimizer();
const splits = await optimizer.optimizeLargeTrade(/* params */);
```

**"Quote expired"**
```typescript
// Refresh quote before execution
const freshQuote = await quoteEngine.getCompleteQuote(/* params */);
```

## Next Steps

- [Position Management](./position-mgmt.md) - Manage DLMM positions
- [Liquidity Shapes](./liquidity-shapes.md) - Distribution strategies
- [Advanced Trading](./advanced-trading.md) - Complex execution patterns
- [Quote Examples](../../code-examples/typescript/04-dlmm-range-orders/)

## Resources

- [DLMM Whitepaper](https://docs.saros.finance/dlmm)
- [Price Impact Calculator](https://app.saros.finance/tools/price-impact)
- [Liquidity Analytics](https://analytics.saros.finance)
- [Community Support](https://discord.gg/saros)