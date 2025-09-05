# DLMM Range Orders Example

## Overview

This example demonstrates how to implement limit order functionality using DLMM's concentrated liquidity bins. Range orders allow you to:

- **Create limit buy/sell orders** using single-bin liquidity provision
- **Automated execution** when price enters your range
- **Zero slippage** within your specified bin
- **Earn fees** while waiting for execution

## What are Range Orders?

Range orders are DLMM's innovative approach to limit orders:
- Place liquidity in a single bin above/below current price
- When price moves into your bin, your liquidity gets converted
- Acts as a limit order with added fee-earning potential

## Features

- 🎯 **Single-Bin Range Orders**: True limit order functionality
- 🤖 **Automated Monitoring**: WebSocket-based price tracking
- 💰 **Fee Optimization**: Earn while waiting for execution
- 📊 **Position Management**: Track and manage multiple orders
- ⚡ **Instant Execution**: Automatic conversion when price hits
- 🔄 **Rebalancing Support**: Adjust ranges based on market conditions

## Installation

```bash
# Install dependencies
npm install

# Configure environment
cp .env.example .env
# Edit .env with your configuration
```

## Configuration

Create `.env` file:
```env
# Required
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
WALLET_PRIVATE_KEY=your_private_key_base58

# Optional
NETWORK=mainnet-beta
AUTO_EXECUTE=true
MONITOR_INTERVAL=5000
PRIORITY_FEE=10000
```

## Usage

### 1. Create a Limit Buy Order

```bash
npm run dev -- --type buy --price 50 --amount 1000
```

### 2. Create a Limit Sell Order

```bash
npm run dev -- --type sell --price 55 --amount 1000
```

### 3. Monitor Range Orders

```bash
npm run monitor
```

### 4. Run Automated Executor

```bash
npm run automate
```

## Code Examples

### Basic Range Order

```typescript
import { RangeOrderManager } from './RangeOrderManager';

const manager = new RangeOrderManager(connection, wallet);

// Create limit buy at $50
const buyOrder = await manager.createLimitBuy({
  poolAddress: POOL_ADDRESS,
  targetPrice: 50,
  amount: 1000, // USDC
  tolerance: 0.1 // 0.1% price tolerance
});

// Create limit sell at $55
const sellOrder = await manager.createLimitSell({
  poolAddress: POOL_ADDRESS,
  targetPrice: 55,
  amount: 20, // SOL
  tolerance: 0.1
});
```

### Monitor Execution

```typescript
const monitor = new RangeOrderMonitor(connection);

monitor.on('orderFilled', (order) => {
  console.log(`Order filled at price ${order.executionPrice}`);
});

monitor.on('partialFill', (order, percentage) => {
  console.log(`Order ${percentage}% filled`);
});

await monitor.start();
```

## File Structure

```
04-dlmm-range-orders/
├── src/
│   ├── index.ts                 # Main entry point with examples
│   ├── RangeOrderManager.ts     # Core range order logic
│   ├── RangeOrderMonitor.ts     # WebSocket monitoring
│   ├── AutomatedExecutor.ts     # Auto-execution engine
│   ├── OrderBook.ts            # Order book management
│   ├── PriceCalculator.ts      # Bin-to-price conversion
│   └── utils/
│       ├── binMath.ts          # Bin calculations
│       ├── connection.ts       # RPC setup
│       └── logger.ts           # Logging utilities
├── tests/
│   └── rangeOrders.test.ts     # Test suite
├── package.json
├── tsconfig.json
└── README.md
```

## Understanding Range Orders

### Limit Buy Order
```
Current Price: $52
Target Buy: $50

1. Place USDC liquidity in bin at $50
2. When price drops to $50, USDC converts to SOL
3. Result: Bought SOL at exactly $50
```

### Limit Sell Order
```
Current Price: $52
Target Sell: $55

1. Place SOL liquidity in bin at $55
2. When price rises to $55, SOL converts to USDC
3. Result: Sold SOL at exactly $55
```

### Benefits over Traditional Limit Orders
- **Earn fees** while waiting (if price oscillates through your bin)
- **Zero slippage** execution
- **Gas efficient** (single transaction)
- **Composable** with other DeFi protocols

## Advanced Features

### Multi-Order Strategy
```typescript
// Create ladder of buy orders
const ladder = await manager.createBuyLadder({
  startPrice: 48,
  endPrice: 50,
  steps: 5,
  totalAmount: 5000
});
```

### Take-Profit Orders
```typescript
// Set take-profit after position entry
const takeProfit = await manager.createTakeProfit({
  position: myPosition,
  targetPrice: 60,
  percentage: 50 // Sell 50% at target
});
```

### Stop-Loss Orders
```typescript
// Protective stop-loss
const stopLoss = await manager.createStopLoss({
  position: myPosition,
  triggerPrice: 45,
  marketOrder: false // Use range order
});
```

## Performance Metrics

| Metric | Value |
|--------|-------|
| Execution Precision | ±0.01% |
| Gas Cost | ~0.002 SOL |
| Min Order Size | $10 |
| Max Orders/Pool | Unlimited |
| Fill Time | 1-2 blocks |

## Troubleshooting

### Order Not Filling
- Check if price actually reached your target bin
- Verify sufficient liquidity in pool
- Ensure bin width is appropriate

### High Gas Costs
- Batch multiple orders in single transaction
- Use optimal bin selection
- Consider wider bins for less precision

### WebSocket Disconnection
- Automatic reconnection is built-in
- Check RPC endpoint stability
- Use fallback RPC endpoints

## Best Practices

1. **Bin Selection**: Choose bins based on volatility
2. **Position Sizing**: Don't place entire order in single bin
3. **Monitoring**: Use WebSocket for real-time updates
4. **Gas Optimization**: Batch operations when possible
5. **Risk Management**: Set appropriate tolerances

## Security Considerations

- Never commit private keys
- Use hardware wallets in production
- Implement slippage protection
- Monitor for MEV attacks
- Use priority fees for time-sensitive orders

## Resources

- [DLMM Documentation](https://docs.saros.finance/dlmm)
- [Range Order Guide](https://docs.saros.finance/guides/range-orders)
- [Discord Support](https://discord.gg/saros)

## License

MIT