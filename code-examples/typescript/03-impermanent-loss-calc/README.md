# Saros Impermanent Loss Calculator

Advanced impermanent loss calculator for AMM and DLMM pools with real-time monitoring, fee analysis, and comprehensive reporting.

## Features

- 📊 **IL Calculation** for both AMM and DLMM pools
- 📈 **Real-time price tracking** and IL monitoring
- 💰 **Fee compensation analysis** - See if fees offset IL
- 🎯 **Concentrated liquidity IL** for DLMM positions
- 📉 **Historical IL tracking** with charts
- 🔔 **IL alerts** at custom thresholds
- 📋 **Detailed reports** with breakeven analysis
- 🎨 **Visual charts** for IL scenarios

## What is Impermanent Loss?

Impermanent Loss (IL) occurs when providing liquidity to an AMM pool, where the value of deposited tokens changes compared to simply holding them. The loss is "impermanent" because it's only realized when you withdraw liquidity.

### AMM vs DLMM Impermanent Loss

**AMM (Traditional):**
- IL across entire price range (0 to ∞)
- Formula: `IL = 2 × √(price_ratio) / (1 + price_ratio) - 1`

**DLMM (Concentrated):**
- IL only within your selected price range
- Higher capital efficiency but potentially higher IL
- IL depends on bin width and range selection

## Installation

```bash
# Clone this example
cd code-examples/typescript/03-impermanent-loss-calc

# Install dependencies
npm install

# Build the project
npm run build
```

## Configuration

Create a `.env` file:

```env
# Network Configuration
SOLANA_NETWORK=mainnet-beta
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com

# Price Data Source
PRICE_API_URL=https://api.coingecko.com/api/v3
PRICE_UPDATE_INTERVAL=60000  # 1 minute

# IL Monitoring
ENABLE_MONITORING=true
IL_WARNING_THRESHOLD=5       # Warn at 5% IL
IL_CRITICAL_THRESHOLD=10     # Critical at 10% IL

# Reporting
GENERATE_REPORTS=true
REPORT_INTERVAL=86400000     # Daily reports
REPORT_OUTPUT_DIR=./reports
```

## Usage

### Basic IL Calculation

```typescript
import { ImpermanentLossCalculator } from './src/ImpermanentLossCalculator';

const calculator = new ImpermanentLossCalculator();

// Calculate IL for price change
const result = calculator.calculateIL({
  initialPriceRatio: 1,      // Initial price of token A/B
  currentPriceRatio: 1.5,    // Current price (50% increase)
  poolType: 'AMM'
});

console.log(`Impermanent Loss: ${result.impermanentLoss}%`);
console.log(`Value if held: $${result.valueIfHeld}`);
console.log(`Value in pool: $${result.valueInPool}`);
```

### DLMM Concentrated Liquidity IL

```typescript
import { DLMMCalculator } from './src/DLMMCalculator';

const dlmmCalc = new DLMMCalculator();

// Calculate IL for concentrated position
const dlmmResult = await dlmmCalc.calculateConcentratedIL({
  lowerPrice: 45,           // Lower price bound
  upperPrice: 55,           // Upper price bound
  currentPrice: 50,         // Current price
  initialPrice: 48,         // Price when position opened
  liquidity: 10000,         // Liquidity amount
  binStep: 10              // DLMM bin step
});

console.log(`DLMM IL: ${dlmmResult.impermanentLoss}%`);
console.log(`Position in range: ${dlmmResult.inRange}`);
```

### Fee Compensation Analysis

```typescript
import { FeeAnalyzer } from './src/FeeAnalyzer';

const analyzer = new FeeAnalyzer();

// Analyze if fees compensate for IL
const analysis = await analyzer.analyzeFeesVsIL({
  pool: 'SOL-USDC',
  position: {
    liquidity: 10000,
    duration: 30,           // 30 days
    averageTVL: 1000000,
    dailyVolume: 500000
  },
  feeRate: 0.3,           // 0.3% fee tier
  impermanentLoss: 5.2    // Current IL
});

console.log(`Fees earned: $${analysis.totalFees}`);
console.log(`IL in USD: $${analysis.ilInUSD}`);
console.log(`Net profit: $${analysis.netProfit}`);
console.log(`Breakeven in: ${analysis.daysToBreakeven} days`);
```

### Real-time IL Monitoring

```typescript
import { ILMonitor } from './src/ILMonitor';

const monitor = new ILMonitor();

// Start monitoring a position
await monitor.startMonitoring({
  poolAddress: 'POOL_ADDRESS',
  positionId: 'POSITION_ID',
  alertThresholds: {
    warning: 5,    // 5% IL
    critical: 10   // 10% IL
  },
  onAlert: (alert) => {
    console.log(`IL Alert: ${alert.level} - ${alert.currentIL}%`);
  }
});
```

## Running Examples

### 1. Interactive IL Calculator
```bash
npm run dev
```

### 2. Analyze Specific Position
```bash
POOL_ADDRESS=xxx npm run analyze
```

### 3. Generate IL Report
```bash
npm run report
```

### 4. Run Test Suite
```bash
npm test
```

## Project Structure

```
03-impermanent-loss-calc/
├── src/
│   ├── index.ts                    # Main entry point
│   ├── ImpermanentLossCalculator.ts # Core IL calculations
│   ├── DLMMCalculator.ts           # DLMM-specific IL logic
│   ├── FeeAnalyzer.ts              # Fee compensation analysis
│   ├── ILMonitor.ts                # Real-time IL monitoring
│   ├── PriceTracker.ts             # Price data fetching
│   ├── ReportGenerator.ts          # IL report generation
│   ├── strategies/
│   │   ├── ILMitigation.ts         # IL mitigation strategies
│   │   └── RangeOptimizer.ts       # Optimal range selection
│   └── utils/
│       ├── charts.ts               # Chart generation
│       ├── formulas.ts             # IL formulas
│       └── logger.ts               # Logging utilities
├── test/
│   └── calculator.test.ts          # Test suite
├── examples/
│   ├── basic-il.ts                 # Basic IL examples
│   ├── dlmm-analysis.ts            # DLMM analysis
│   └── fee-compensation.ts         # Fee analysis
├── .env.example                    # Environment template
├── package.json                    # Dependencies
├── tsconfig.json                   # TypeScript config
└── README.md                       # This file
```

## IL Formulas

### Traditional AMM IL Formula
```
IL = 2 × √(price_ratio) / (1 + price_ratio) - 1

Where:
- price_ratio = current_price / initial_price
```

### IL at Different Price Points
| Price Change | Impermanent Loss |
|-------------|------------------|
| 1.25x       | 0.6%            |
| 1.50x       | 2.0%            |
| 2.00x       | 5.7%            |
| 3.00x       | 13.4%           |
| 4.00x       | 20.0%           |
| 5.00x       | 25.5%           |

### DLMM Concentrated IL
For concentrated positions, IL is calculated differently:
- IL = 0 when price is outside range (but 100% of position is in one token)
- IL is amplified within range based on concentration factor
- Tighter ranges = higher capital efficiency but higher IL risk

## Understanding the Output

### Basic Calculation Output
```json
{
  "impermanentLoss": 5.72,           // IL percentage
  "valueIfHeld": 2000,               // Value if tokens were held
  "valueInPool": 1885.62,            // Current value in pool
  "tokenAAmount": 7.07,              // Current token A in pool
  "tokenBAmount": 141.42,            // Current token B in pool
  "priceImpact": 100,                // Price change percentage
  "breakEvenFees": 114.38            // Fees needed to break even
}
```

### Fee Analysis Output
```json
{
  "totalFees": 150.25,               // Total fees earned
  "ilInUSD": 114.38,                // IL in USD terms
  "netProfit": 35.87,                // Profit after IL
  "feeAPR": 54.75,                  // Annualized fee return
  "ilCompensated": true,             // Whether fees cover IL
  "daysToBreakeven": 22.8,          // Days until fees cover IL
  "optimalRange": {                  // Suggested range optimization
    "lower": 45,
    "upper": 55,
    "expectedFees": 180.50,
    "expectedIL": 95.20
  }
}
```

## Advanced Features

### 1. Historical IL Analysis
```typescript
const historical = await calculator.analyzeHistoricalIL({
  pool: 'SOL-USDC',
  period: '30d',
  resolution: '1h'
});

// Returns IL over time with statistics
```

### 2. Multi-Pool Comparison
```typescript
const comparison = await calculator.comparePools([
  { pool: 'SOL-USDC', type: 'AMM' },
  { pool: 'SOL-USDC', type: 'DLMM' },
  { pool: 'SOL-USDT', type: 'AMM' }
]);

// Compare IL across different pools
```

### 3. Optimal Range Finder (DLMM)
```typescript
const optimal = await dlmmCalc.findOptimalRange({
  token0: 'SOL',
  token1: 'USDC',
  capital: 10000,
  targetAPR: 50,
  maxIL: 10
});

// Suggests optimal price range for target returns
```

### 4. IL Hedging Strategies
```typescript
const hedging = await calculator.suggestHedging({
  position: yourPosition,
  riskTolerance: 'medium'
});

// Suggests hedging strategies to minimize IL
```

## Mitigation Strategies

The calculator suggests various IL mitigation strategies:

1. **Range Adjustment** - Widen/narrow range based on volatility
2. **Rebalancing** - When to rebalance positions
3. **Fee Tier Selection** - Optimal fee tier for volatility
4. **Partial Hedging** - Hold some tokens outside pool
5. **Time-based Exit** - Optimal holding periods

## Charts and Visualization

Generate visual reports:

```bash
# Generate IL curve chart
npm run chart:il-curve

# Generate fee vs IL comparison
npm run chart:fee-comparison

# Generate position P&L chart
npm run chart:pnl
```

## Common Scenarios

### "Should I provide liquidity?"
Run the full analysis to see if expected fees outweigh IL risk:
```bash
npm run analyze -- --pool SOL-USDC --capital 10000
```

### "What's my current IL?"
Check real-time IL for your position:
```bash
npm run check -- --position YOUR_POSITION_ID
```

### "What range should I use?" (DLMM)
Find optimal range for your risk/return profile:
```bash
npm run optimize -- --pool SOL-USDC --target-apr 50
```

## Troubleshooting

### "IL calculation seems wrong"
- Ensure you're using the correct initial and current prices
- For DLMM, verify your position is within range
- Check if fees are included in the calculation

### "Monitor not updating"
- Check RPC connection
- Verify price feed is working
- Ensure pool address is correct

## Best Practices

1. **Always consider fees** - IL alone doesn't tell the full story
2. **Monitor regularly** - IL can change quickly with price movements
3. **Set alerts** - Don't let IL exceed your risk tolerance
4. **Understand concentration** - Tighter ranges mean higher IL risk
5. **Consider time horizon** - Longer periods allow more fee accumulation

## Support

- [Saros Documentation](https://docs.saros.finance)
- [Discord Community](https://discord.gg/saros)
- [IL Education](https://docs.saros.finance/impermanent-loss)

## License

MIT