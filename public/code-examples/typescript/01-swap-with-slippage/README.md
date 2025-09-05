# Saros Swap with Slippage Protection

Production-ready implementation of token swaps with dynamic slippage protection, retry mechanisms, and comprehensive error handling.

## Features

- ✅ Dynamic slippage calculation based on market conditions
- ✅ Automatic retry with exponential backoff
- ✅ Price impact monitoring and alerts
- ✅ Transaction simulation before execution
- ✅ Comprehensive error handling and recovery
- ✅ Real-time price monitoring
- ✅ MEV protection strategies

## Prerequisites

- Node.js v16+ 
- Solana CLI tools
- A funded Solana wallet (devnet or mainnet)
- Basic understanding of token swaps

## Installation

```bash
# Clone this example
cd code-examples/typescript/01-swap-with-slippage

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

# Swap Configuration
DEFAULT_SLIPPAGE=0.5
MAX_SLIPPAGE=5.0
PRICE_IMPACT_WARNING=2.0

# Retry Configuration
MAX_RETRIES=3
RETRY_DELAY_MS=1000

# Logging
LOG_LEVEL=info
```

## Usage

### Basic Swap

```typescript
import { SwapManager } from './src/SwapManager';

const swapManager = new SwapManager({
  rpcUrl: process.env.SOLANA_RPC_URL,
  privateKey: process.env.WALLET_PRIVATE_KEY
});

// Execute a simple swap
const result = await swapManager.swap({
  fromMint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', // USDC
  toMint: 'So11111111111111111111111111111111111111112',     // SOL
  amount: 100, // 100 USDC
  slippageTolerance: 0.5 // 0.5%
});

console.log('Swap successful:', result.signature);
```

### Advanced Usage with Dynamic Slippage

```typescript
import { DynamicSlippageSwap } from './src/DynamicSlippageSwap';

const dynamicSwap = new DynamicSlippageSwap({
  rpcUrl: process.env.SOLANA_RPC_URL,
  privateKey: process.env.WALLET_PRIVATE_KEY
});

// Swap with dynamic slippage based on volatility
const result = await dynamicSwap.executeWithOptimalSlippage({
  fromMint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
  toMint: 'So11111111111111111111111111111111111111112',
  amount: 1000,
  maxPriceImpact: 3.0,
  urgency: 'normal' // 'low', 'normal', 'high'
});
```

### Price Monitoring

```typescript
import { PriceMonitor } from './src/PriceMonitor';

const monitor = new PriceMonitor();

// Monitor price and execute when favorable
await monitor.swapAtTargetPrice({
  fromMint: 'USDC',
  toMint: 'SOL',
  amount: 100,
  targetPrice: 50.0, // Execute when 1 SOL = 50 USDC
  tolerance: 1.0 // ±1%
});
```

## Running Examples

### 1. Basic Swap Test
```bash
npm run dev
```

### 2. Run Test Suite
```bash
npm test
```

### 3. Production Build
```bash
npm run build
npm start
```

## Project Structure

```
01-swap-with-slippage/
├── src/
│   ├── index.ts              # Main entry point
│   ├── SwapManager.ts        # Core swap implementation
│   ├── DynamicSlippageSwap.ts # Dynamic slippage calculator
│   ├── PriceMonitor.ts       # Real-time price monitoring
│   ├── ErrorHandler.ts       # Error handling and recovery
│   ├── config.ts             # Configuration management
│   └── utils/
│       ├── connection.ts     # Solana connection utilities
│       ├── tokens.ts         # Token utilities
│       └── logger.ts         # Logging utilities
├── test/
│   └── swap.test.ts          # Test cases
├── .env.example              # Environment variables template
├── package.json              # Dependencies
├── tsconfig.json            # TypeScript configuration
└── README.md                # This file
```

## Error Handling

The implementation handles various error scenarios:

- **Insufficient Balance**: Checks balance before attempting swap
- **Slippage Exceeded**: Automatically increases slippage and retries
- **Network Errors**: Implements exponential backoff retry
- **Pool Not Found**: Suggests alternative routes
- **Transaction Timeout**: Monitors and retries with higher priority fee

## Performance Optimization

- Connection pooling for better RPC performance
- Transaction simulation before execution
- Batch balance checks
- Caching of pool information
- Priority fee optimization

## Security Considerations

- ✅ Input validation on all parameters
- ✅ Minimum output amount enforcement
- ✅ MEV protection through priority fees
- ✅ Private key never logged or exposed
- ✅ Transaction simulation before execution

## Testing

Run the test suite:

```bash
npm test
```

Tests cover:
- Basic swap execution
- Slippage handling
- Error recovery
- Price monitoring
- Edge cases

## Monitoring & Logs

Logs are written to `logs/swap.log` with the following levels:
- **ERROR**: Failed transactions, critical errors
- **WARN**: High slippage, price impact warnings
- **INFO**: Successful swaps, price updates
- **DEBUG**: Detailed execution flow

## Common Issues

### "Insufficient SOL for fees"
Ensure your wallet has at least 0.01 SOL for transaction fees.

### "Slippage tolerance exceeded"
The market moved too quickly. Increase slippage or wait for lower volatility.

### "Pool not found"
The token pair might not have a direct pool. The system will try multi-hop routing.

## Production Deployment

For production use:
1. Use a dedicated RPC endpoint (Helius, QuickNode, etc.)
2. Implement rate limiting
3. Set up monitoring and alerts
4. Use environment-specific configurations
5. Enable transaction logging for audit

## Support

- [Saros Documentation](https://docs.saros.finance)
- [Discord Community](https://discord.gg/saros)
- [GitHub Issues](https://github.com/saros-finance/sdk-examples)

## License

MIT