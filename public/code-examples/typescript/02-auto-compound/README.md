# Saros Auto-Compound Yield Farming

Production-ready implementation of automated yield farming with compound interest strategies using Saros Finance.

## Features

- 🔄 Automated reward harvesting and reinvestment
- 📊 APY optimization through frequent compounding
- ⏰ Scheduled compound intervals (hourly, daily, custom)
- 🛡️ Gas-efficient batched operations
- 📈 Real-time yield tracking and analytics
- 🔔 Notifications for compound events
- 💰 Multiple strategy support (LP, Staking, Farming)
- 🎯 Minimum threshold enforcement

## Prerequisites

- Node.js v16+
- Solana CLI tools
- Funded wallet with SOL for gas fees
- LP tokens or staked positions

## Installation

```bash
# Clone this example
cd code-examples/typescript/02-auto-compound

# Install dependencies
npm install

# Build the project
npm run build
```

## Configuration

Create a `.env` file:

```env
# Network Configuration
SOLANA_NETWORK=devnet
SOLANA_RPC_URL=https://api.devnet.solana.com

# Wallet Configuration
WALLET_PRIVATE_KEY=your_base58_private_key_here

# Auto-Compound Settings
COMPOUND_INTERVAL=3600000  # 1 hour in milliseconds
MIN_REWARD_THRESHOLD=1.0   # Minimum rewards before compounding
AUTO_COMPOUND_ENABLED=true
MAX_GAS_PRICE=0.01         # Maximum SOL to spend on gas

# Strategy Configuration
STRATEGY_TYPE=LP           # Options: LP, STAKING, FARMING
POOL_ADDRESS=pool_address_here
REINVEST_PERCENTAGE=100    # Percentage of rewards to reinvest

# Notifications (Optional)
ENABLE_NOTIFICATIONS=false
WEBHOOK_URL=your_webhook_url
EMAIL_NOTIFICATIONS=false
```

## Usage

### Basic Auto-Compound Setup

```typescript
import { AutoCompounder } from './src/AutoCompounder';

const compounder = new AutoCompounder({
  rpcUrl: process.env.SOLANA_RPC_URL,
  privateKey: process.env.WALLET_PRIVATE_KEY,
  strategy: 'LP'
});

// Start auto-compounding
await compounder.start({
  poolAddress: 'pool_address_here',
  interval: 3600000, // 1 hour
  minRewardThreshold: 1.0
});
```

### Advanced Strategy Configuration

```typescript
import { YieldOptimizer } from './src/YieldOptimizer';

const optimizer = new YieldOptimizer({
  rpcUrl: process.env.SOLANA_RPC_URL,
  privateKey: process.env.WALLET_PRIVATE_KEY
});

// Configure multi-strategy yield optimization
await optimizer.addStrategy({
  type: 'LP',
  poolAddress: 'lp_pool_address',
  weight: 0.5,
  autoCompound: true,
  compoundInterval: 3600000
});

await optimizer.addStrategy({
  type: 'STAKING',
  poolAddress: 'staking_pool_address',
  weight: 0.3,
  autoCompound: true,
  compoundInterval: 7200000
});

await optimizer.addStrategy({
  type: 'FARMING',
  poolAddress: 'farm_address',
  weight: 0.2,
  autoCompound: true,
  compoundInterval: 86400000
});

// Start optimized yield farming
await optimizer.startOptimization();
```

### Manual Compound Trigger

```typescript
// Manually trigger compound
const result = await compounder.compoundNow();
console.log('Compound result:', {
  rewardsHarvested: result.rewardsHarvested,
  amountReinvested: result.amountReinvested,
  newPosition: result.newPosition,
  gasUsed: result.gasUsed,
  apy: result.currentAPY
});
```

## Running Examples

### 1. Start Auto-Compounder
```bash
npm run dev
```

### 2. Run with Custom Strategy
```bash
STRATEGY_TYPE=LP npm run dev
```

### 3. Test Compound Logic
```bash
npm test
```

### 4. Production Mode
```bash
npm run build
npm start
```

## Project Structure

```
02-auto-compound/
├── src/
│   ├── index.ts              # Main entry point
│   ├── AutoCompounder.ts     # Core auto-compound logic
│   ├── YieldOptimizer.ts     # Yield optimization strategies
│   ├── RewardCalculator.ts   # Reward and APY calculations
│   ├── GasOptimizer.ts       # Gas optimization utilities
│   ├── strategies/
│   │   ├── LPStrategy.ts     # LP token compounding
│   │   ├── StakingStrategy.ts # Staking rewards compound
│   │   └── FarmingStrategy.ts # Farm rewards compound
│   └── utils/
│       ├── scheduler.ts      # Cron job scheduler
│       ├── notifications.ts  # Alert system
│       └── analytics.ts      # Performance tracking
├── test/
│   └── compound.test.ts      # Test suite
├── .env.example              # Environment template
├── package.json              # Dependencies
├── tsconfig.json            # TypeScript config
└── README.md                # This file
```

## Strategy Types

### 1. LP Token Compounding
- Harvests trading fees and rewards
- Reinvests into LP position
- Rebalances when necessary

### 2. Staking Compound
- Claims staking rewards
- Re-stakes automatically
- Maximizes staking APY

### 3. Farm Compounding
- Harvests farm rewards
- Swaps to LP tokens
- Adds to farm position

## APY Calculation

The system calculates both simple and compound APY:

```typescript
// Simple APY
const simpleAPY = (dailyRewards * 365) / totalStaked * 100;

// Compound APY (with daily compounding)
const compoundAPY = (Math.pow(1 + dailyRate, 365) - 1) * 100;

// Optimal compound frequency
const optimalFrequency = optimizer.calculateOptimalFrequency({
  gasPrice: currentGasPrice,
  rewardRate: dailyRewards,
  position: totalStaked
});
```

## Gas Optimization

The system includes several gas optimization features:

- **Batch Operations**: Combines multiple operations in single transaction
- **Threshold Enforcement**: Only compounds when rewards exceed gas costs
- **Dynamic Scheduling**: Adjusts compound frequency based on gas prices
- **Priority Fees**: Uses optimal priority fees for faster execution

## Monitoring & Analytics

Track your auto-compound performance:

```typescript
const stats = await compounder.getStatistics();
console.log({
  totalCompounds: stats.totalCompounds,
  totalRewardsHarvested: stats.totalRewardsHarvested,
  totalGasSpent: stats.totalGasSpent,
  currentAPY: stats.currentAPY,
  projectedYearlyReturn: stats.projectedYearlyReturn
});
```

## Error Handling

The system handles various scenarios:

- **Insufficient Rewards**: Waits for minimum threshold
- **High Gas Prices**: Delays compounding during congestion
- **Failed Transactions**: Automatic retry with exponential backoff
- **Position Changes**: Adapts to manual interventions

## Security Considerations

- ✅ Private keys never logged or exposed
- ✅ Slippage protection on all swaps
- ✅ Maximum gas price limits
- ✅ Emergency stop functionality
- ✅ Position size validation

## Common Issues

### "Rewards below threshold"
The system is waiting for more rewards to accumulate before compounding.

### "Gas price too high"
Current network gas prices exceed your maximum limit. Adjust `MAX_GAS_PRICE`.

### "Insufficient SOL for fees"
Ensure your wallet has enough SOL for transaction fees.

## Performance Tips

1. **Optimal Intervals**: Use 4-8 hour intervals for most pools
2. **Gas Monitoring**: Compound during low congestion periods
3. **Batch Positions**: Manage multiple positions together
4. **Reward Thresholds**: Set based on current gas prices

## Advanced Features

### Custom Compound Strategies
```typescript
class CustomStrategy extends BaseStrategy {
  async shouldCompound(): Promise<boolean> {
    // Custom logic for when to compound
    return true;
  }
  
  async compound(): Promise<CompoundResult> {
    // Custom compound implementation
  }
}
```

### Multi-Pool Management
```typescript
const multiPool = new MultiPoolManager();
await multiPool.addPool('pool1', { strategy: 'LP' });
await multiPool.addPool('pool2', { strategy: 'STAKING' });
await multiPool.optimizeAll();
```

## Production Deployment

1. Use dedicated RPC endpoint
2. Set up monitoring and alerts
3. Configure backup wallets
4. Implement rate limiting
5. Enable comprehensive logging

## Support

- [Saros Documentation](https://docs.saros.finance)
- [Discord Community](https://discord.gg/saros)
- [GitHub Issues](https://github.com/saros-finance/sdk-examples)

## License

MIT