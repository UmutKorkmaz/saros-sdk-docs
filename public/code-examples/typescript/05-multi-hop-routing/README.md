# Multi-Hop Routing Example

## Overview

This example demonstrates advanced multi-hop routing for finding optimal swap paths across multiple Saros pools. It includes path finding algorithms, route optimization, and execution strategies.

## Features

- 🛣️ **Dijkstra's Algorithm**: Shortest path finding for optimal routes
- 🔄 **Multi-Path Execution**: Split trades across multiple routes
- 💰 **Price Impact Optimization**: Minimize slippage through route selection
- ⚡ **Parallel Route Discovery**: Fast path finding using graph algorithms
- 📊 **Route Analysis**: Compare different paths for best execution
- 🎯 **Arbitrage Detection**: Identify profitable circular routes

## What is Multi-Hop Routing?

Multi-hop routing finds the best path to swap tokens that may not have direct pools:
- **Direct Route**: Token A → Token B (if pool exists)
- **2-Hop Route**: Token A → USDC → Token B
- **3-Hop Route**: Token A → SOL → USDC → Token B
- **Split Routes**: 50% through Route 1, 50% through Route 2

## Installation

```bash
# Install dependencies
npm install

# Configure environment
cp .env.example .env
# Edit .env with your configuration
```

## Usage

### Basic Multi-Hop Swap

```bash
# Find and execute best route
npm run dev -- --from SOL --to BONK --amount 10
```

### Route Analysis

```bash
# Analyze all possible routes
npm run analyze -- --from SOL --to BONK
```

### Benchmark Routes

```bash
# Compare route performance
npm run benchmark
```

## Code Examples

### Find Optimal Route

```typescript
import { MultiHopRouter } from './MultiHopRouter';

const router = new MultiHopRouter(connection);

// Find best route
const route = await router.findBestRoute({
  fromMint: SOL_MINT,
  toMint: BONK_MINT,
  amount: 1000000000, // 1 SOL
  maxHops: 3,
  minLiquidity: 10000
});

console.log(`Best route: ${route.path.join(' → ')}`);
console.log(`Expected output: ${route.expectedOutput}`);
console.log(`Price impact: ${route.priceImpact}%`);
```

### Execute Multi-Hop Swap

```typescript
const result = await router.executeRoute({
  route,
  wallet,
  slippage: 0.5
});

console.log(`Swap completed: ${result.signature}`);
console.log(`Actual output: ${result.amountOut}`);
```

### Split Route Execution

```typescript
// Find multiple routes
const routes = await router.findMultipleRoutes({
  fromMint: SOL_MINT,
  toMint: USDC_MINT,
  amount: 10000000000, // 10 SOL
  maxRoutes: 3
});

// Execute with split
const result = await router.executeSplitRoute({
  routes,
  splits: [0.5, 0.3, 0.2], // 50%, 30%, 20%
  wallet,
  slippage: 0.5
});
```

## File Structure

```
05-multi-hop-routing/
├── src/
│   ├── index.ts                # Main entry with examples
│   ├── MultiHopRouter.ts       # Core routing logic
│   ├── RouteOptimizer.ts       # Route optimization algorithms
│   ├── PathFinder.ts           # Graph-based path finding
│   ├── RouteExecutor.ts        # Route execution engine
│   ├── RouteAnalyzer.ts        # Route comparison tools
│   ├── ArbitrageDetector.ts    # Circular route detection
│   └── utils/
│       ├── graph.ts            # Graph utilities
│       ├── pools.ts            # Pool data management
│       └── pricing.ts          # Price calculations
├── tests/
│   └── routing.test.ts         # Test suite
├── package.json
├── tsconfig.json
└── README.md
```

## Routing Algorithms

### 1. Dijkstra's Algorithm
Finds shortest path based on price impact:
```
Graph edges weighted by: -log(1 - fee - slippage)
Result: Path with minimum total cost
```

### 2. Bellman-Ford Algorithm
Handles negative weights for arbitrage detection:
```
Can detect negative cycles (profitable loops)
Useful for: Finding arbitrage opportunities
```

### 3. A* Search
Heuristic-based for faster path finding:
```
Heuristic: Direct price ratio to target
Result: Faster than Dijkstra for known targets
```

## Route Examples

### Simple 2-Hop
```
SOL → USDC → BONK
- Pool 1: SOL/USDC (0.3% fee)
- Pool 2: USDC/BONK (0.3% fee)
- Total fees: ~0.6%
```

### Complex 3-Hop
```
SAMO → SOL → USDC → USDT
- Pool 1: SAMO/SOL (1% fee)
- Pool 2: SOL/USDC (0.3% fee)
- Pool 3: USDC/USDT (0.04% fee)
- Total fees: ~1.34%
```

### Split Route
```
Route 1 (60%): SOL → USDC
Route 2 (40%): SOL → USDT → USDC
- Better price execution for large trades
- Reduced overall slippage
```

## Performance Optimization

### Route Caching
```typescript
// Cache discovered routes
const cache = new RouteCache({
  ttl: 60000, // 1 minute
  maxSize: 1000
});

const route = cache.get(fromMint, toMint) || 
  await router.findBestRoute(...);
```

### Parallel Discovery
```typescript
// Find routes in parallel
const routes = await Promise.all([
  router.findDirectRoute(from, to),
  router.find2HopRoute(from, to),
  router.find3HopRoute(from, to)
]);
```

### Liquidity Filtering
```typescript
// Only consider liquid pools
const pools = await getPoolsWithMinLiquidity(100000); // $100k min
```

## Advanced Features

### Arbitrage Detection
```typescript
const arbitrage = await detector.findArbitrage({
  startToken: SOL_MINT,
  minProfit: 0.1, // 0.1%
  maxHops: 4
});

if (arbitrage) {
  console.log(`Profit opportunity: ${arbitrage.profit}%`);
  console.log(`Path: ${arbitrage.path.join(' → ')}`);
}
```

### MEV Protection
```typescript
// Use private mempool for sensitive routes
const result = await router.executeRoute({
  route,
  wallet,
  useJito: true, // Use Jito for MEV protection
  tip: 0.001 // SOL tip for block inclusion
});
```

### Dynamic Route Updates
```typescript
// Monitor and update routes in real-time
router.on('poolUpdate', (pool) => {
  // Recalculate affected routes
  router.invalidateRoutesWithPool(pool);
});
```

## Configuration

### Environment Variables
```env
# RPC Configuration
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com

# Routing Parameters
MAX_HOPS=3
MIN_LIQUIDITY=10000
MAX_PRICE_IMPACT=5
ROUTE_CACHE_TTL=60000

# Execution Settings
USE_JITO=false
PRIORITY_FEE=10000
MAX_RETRIES=3
```

## Troubleshooting

### No Route Found
- Check if pools exist for token pair
- Increase `maxHops` parameter
- Lower `minLiquidity` requirement

### High Price Impact
- Split trade into smaller amounts
- Use multiple routes
- Wait for better liquidity

### Transaction Failures
- Increase slippage tolerance
- Check for sufficient SOL for fees
- Verify token accounts exist

## Performance Metrics

| Metric | Value |
|--------|-------|
| Route Discovery | < 100ms |
| 2-Hop Execution | ~2 seconds |
| 3-Hop Execution | ~3 seconds |
| Max Hops Supported | 4 |
| Route Cache Hit Rate | > 80% |

## Best Practices

1. **Cache Routes**: Reuse discovered routes for similar trades
2. **Monitor Liquidity**: Track pool depths for accurate routing
3. **Split Large Trades**: Use multiple routes for better execution
4. **Set Reasonable Limits**: Don't exceed 3-4 hops
5. **Handle Failures**: Implement retry logic with fallback routes

## Security Considerations

- Validate all route calculations
- Implement maximum slippage checks
- Use simulation before execution
- Monitor for sandwich attacks
- Consider private mempools for large trades

## Resources

- [Routing Algorithm Details](https://docs.saros.finance/routing)
- [Pool Documentation](https://docs.saros.finance/pools)
- [Discord Support](https://discord.gg/saros)

## License

MIT