# Saros SDK Documentation Hub 🚀

> Comprehensive developer documentation for Saros Finance SDKs - Your complete guide to building DeFi applications on Solana with Saros protocol.

## 📚 Quick Navigation

### 🚀 Getting Started
- [Prerequisites](./getting-started/prerequisites.md) - Environment setup and requirements
- [Installation Guide](./getting-started/installation.md) - Install all Saros SDKs
- [Configuration](./getting-started/configuration.md) - RPC endpoints and network setup
- [First Transaction](./getting-started/first-transaction.md) - Your hello world example

### 🧠 Core Concepts
- [AMM vs DLMM](./core-concepts/amm-vs-dlmm.md) - Understanding liquidity models
- [Bin-Based Liquidity](./core-concepts/bin-liquidity.md) - DLMM system deep dive
- [Jupiter Integration](./core-concepts/jupiter-integration.md) - Router aggregation patterns
- [Fee Structures](./core-concepts/fee-structures.md) - Trading and LP fee mechanics

### 📖 SDK Guides
- **TypeScript SDK**
  - [Swap Operations](./sdk-guides/typescript-sdk/swap-operations.md) - swapSaros() deep dive
  - [Pool Management](./sdk-guides/typescript-sdk/pool-management.md) - createPool() and LP ops
  - [Staking Guide](./sdk-guides/typescript-sdk/staking.md) - SarosStakeServices
  - [Farming Guide](./sdk-guides/typescript-sdk/farming.md) - SarosFarmService
- **DLMM SDK**
  - [Quote System](./sdk-guides/dlmm-sdk/quote-system.md) - getQuote() mechanics
  - [Position Management](./sdk-guides/dlmm-sdk/position-mgmt.md) - Bin positioning
  - [Liquidity Shapes](./sdk-guides/dlmm-sdk/liquidity-shapes.md) - Distribution patterns
  - [Advanced Trading](./sdk-guides/dlmm-sdk/advanced-trading.md) - Complex routes
- **Rust SDK**
  - [Jupiter AMM](./sdk-guides/rust-sdk/jupiter-amm.md) - AMM trait implementation
  - [On-Chain Operations](./sdk-guides/rust-sdk/on-chain-ops.md) - Direct program calls

### 📝 Tutorials
- [Basic Swap](./tutorials/01-basic-swap.md) - Simple token swap implementation
- [Add Liquidity](./tutorials/02-add-liquidity.md) - Provide liquidity to pools
- [Yield Farming](./tutorials/03-yield-farming.md) - Stake and earn rewards
- [DLMM Positions](./tutorials/04-dlmm-positions.md) - Concentrated liquidity
- [Arbitrage Bot](./tutorials/05-arbitrage-bot.md) - Advanced MEV strategies
- [Portfolio Tracker](./tutorials/06-portfolio-tracker.md) - Track positions & rewards

### 💻 Code Examples
- **TypeScript Examples**
  - [Swap with Slippage](./code-examples/typescript/01-swap-with-slippage/) - Production-ready swaps
  - [Auto-Compound](./code-examples/typescript/02-auto-compound/) - Automated yield optimization
  - [IL Calculator](./code-examples/typescript/03-impermanent-loss-calc/) - Risk analysis tools
  - [DLMM Range Orders](./code-examples/typescript/04-dlmm-range-orders/) - Limit order patterns
  - [Multi-Hop Routing](./code-examples/typescript/05-multi-hop-routing/) - Complex swap paths
- **Rust Examples**
  - [Jupiter Integration](./code-examples/rust/01-jupiter-integration/) - AMM trait usage
  - [Direct Program Calls](./code-examples/rust/02-direct-program-calls/) - On-chain interactions

## 🎯 What is Saros?

Saros Finance is Solana's liquidity hub featuring three powerful liquidity engines:

### 🔄 Saros AMM (Automated Market Maker)
- **Continuous Liquidity**: Traditional AMM with liquidity across all price ranges
- **Low Slippage**: Optimized for high-volume trading
- **Simple Integration**: Easy-to-use TypeScript SDK

### ⚡ Saros DLMM (Dynamic Liquidity Market Maker)  
- **Bin-Based Liquidity**: Precise liquidity positioning across price ranges
- **Capital Efficiency**: Up to 20x more efficient than traditional AMMs
- **Advanced Strategies**: Range orders, concentrated liquidity, custom distributions

### 🌱 Saros CLMM (Coming Soon)
- **Next-Generation**: Concentrated Liquidity Market Maker
- **Enhanced Performance**: Optimized for institutional trading

## 🛠 Available SDKs

### @saros-finance/sdk (TypeScript)
The main TypeScript SDK for interacting with Saros protocol.

**Features:**
- Token swaps with optimal routing
- Liquidity pool operations
- Staking functionality
- Yield farming integration

**Installation:**
```bash
npm install @saros-finance/sdk
# or
yarn add @saros-finance/sdk
```

### @saros-finance/dlmm-sdk (TypeScript)
Specialized SDK for Dynamic Liquidity Market Maker features.

**Features:**
- Concentrated liquidity positions
- Range orders
- Advanced LP strategies
- Fee tier optimization

**Installation:**
```bash
npm install @saros-finance/dlmm-sdk
# or
yarn add @saros-finance/dlmm-sdk
```

### saros-dlmm-sdk-rs (Rust)
High-performance Rust SDK for DLMM integration.

**Features:**
- Native Rust performance
- On-chain program integration
- Advanced DLMM operations
- Type-safe interfaces

**Installation:**
```toml
[dependencies]
saros-dlmm-sdk = "0.1.0"
```

## 🚀 Quick Start Examples

### AMM Swap with @saros-finance/sdk
```typescript
import { getSwapAmountSaros, swapSaros, genConnectionSolana } from '@saros-finance/sdk';
import { PublicKey } from '@solana/web3.js';

const connection = genConnectionSolana();
const SAROS_SWAP_PROGRAM = new PublicKey('SSwapUtytfBdBn1b9NUGG6foMVPtcWgpRU32HToDUZr');

// Calculate swap amount with slippage
const estSwap = await getSwapAmountSaros(
  connection,
  'C98A4nkJXhpVZNAZdHUA95RpTF3T4whtQubL3YobiUX9', // From: C98
  'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', // To: USDC
  1, // Amount
  0.5, // Slippage %
  poolParams
);

// Execute the swap
const result = await swapSaros(
  connection,
  fromTokenAccount,
  toTokenAccount,
  fromAmount,
  estSwap.amountOutWithSlippage,
  null,
  poolAddress,
  SAROS_SWAP_PROGRAM,
  walletAddress,
  fromMint,
  toMint
);

console.log('Swap TX:', result.hash);
```

### DLMM Swap with @saros-finance/dlmm-sdk
```typescript
import { LiquidityBookServices, MODE } from '@saros-finance/dlmm-sdk';
import { PublicKey } from '@solana/web3.js';

const liquidityBookServices = new LiquidityBookServices({
  mode: MODE.MAINNET,
});

// Get quote with price impact
const quoteData = await liquidityBookServices.getQuote({
  amount: BigInt(1e6), // 1 USDC
  isExactInput: true,
  swapForY: true,
  pair: new PublicKey(poolAddress),
  tokenBase: new PublicKey(baseTokenMint),
  tokenQuote: new PublicKey(quoteTokenMint),
  tokenBaseDecimal: 6,
  tokenQuoteDecimal: 6,
  slippage: 0.5
});

// Execute DLMM swap
const transaction = await liquidityBookServices.swap({
  amount: quoteData.amount,
  otherAmountOffset: quoteData.otherAmountOffset,
  isExactInput: true,
  swapForY: true,
  pair: new PublicKey(poolAddress),
  payer: wallet.publicKey
});
```

### 📚 Complete API References
- [TypeScript SDK API](./api-reference/typescript-sdk/) - Complete method documentation
- [DLMM SDK API](./api-reference/dlmm-sdk/) - DLMM-specific methods and interfaces
- [Rust SDK API](./api-reference/rust-sdk/) - Rust trait and struct documentation

### 🔧 Integration Patterns
- [Error Handling](./integration-patterns/error-handling.md) - Common errors & solutions
- [Transaction Retry](./integration-patterns/transaction-retry.md) - Handling failed transactions
- [WebSocket Events](./integration-patterns/websocket-events.md) - Real-time updates
- [Batch Operations](./integration-patterns/batch-operations.md) - Optimizing multiple operations
- [Testing Strategies](./integration-patterns/testing-strategies.md) - Devnet vs Mainnet

### 🏆 Best Practices
- [Security](./best-practices/security.md) - Key management and validation
- [Performance](./best-practices/performance.md) - RPC optimization and caching
- [UX Patterns](./best-practices/ux-patterns.md) - Loading states and confirmations
- [Monitoring](./best-practices/monitoring.md) - Tracking pool metrics and health

### 📋 Resources
- [Glossary](./resources/glossary.md) - DeFi and Solana terminology
- [Troubleshooting](./resources/troubleshooting.md) - Common issues & fixes
- [Migration Guide](./resources/migration-guide.md) - Upgrading between versions
- [Ecosystem](./resources/ecosystem.md) - Related projects and tools
- [Tools](./resources/tools.md) - Developer tools and utilities

## 📖 Documentation Structure

```
📦 saros-sdk-docs
├── 🚀 getting-started/     - Environment setup & first steps
├── 🧠 core-concepts/       - AMM vs DLMM, liquidity models
├── 📖 sdk-guides/          - Detailed SDK usage guides
├── 📝 tutorials/           - Step-by-step integration tutorials
├── 💻 code-examples/       - Production-ready sample code
├── 📚 api-reference/       - Complete method documentation
├── 🔧 integration-patterns/ - Common integration patterns
├── 🏆 best-practices/      - Security, performance, UX
└── 📋 resources/           - Glossary, troubleshooting, tools
```

## 🏗 Prerequisites

Before you begin, ensure you have:

- **Node.js** v16+ (for TypeScript SDKs)
- **Rust** 1.70+ (for Rust SDK)
- **Solana CLI** tools installed
- A **Solana wallet** (Phantom, Solflare, etc.)
- Basic knowledge of **Solana development**

## 💡 Key Features Covered

### For Traders
- ✅ Best price discovery across pools
- ✅ MEV protection
- ✅ Slippage management
- ✅ Transaction optimization

### For Liquidity Providers
- ✅ Concentrated liquidity positions
- ✅ Impermanent loss calculations
- ✅ Fee earnings optimization
- ✅ Position management tools

### For Developers
- ✅ Type-safe interfaces
- ✅ Comprehensive error handling
- ✅ WebSocket subscriptions
- ✅ Batch operations support

## 🧪 Testing Environment

All examples are tested on:
- **Devnet**: For development and testing
- **Mainnet**: For production deployment

### Devnet Configuration
```typescript
const connection = new Connection('https://api.devnet.solana.com');
```

### Mainnet Configuration
```typescript
const connection = new Connection('https://api.mainnet-beta.solana.com');
```

## 📊 Performance Benchmarks

| Operation | Average Time | Gas Cost |
|-----------|-------------|----------|
| Simple Swap | ~1.2s | ~0.005 SOL |
| Add Liquidity | ~1.5s | ~0.008 SOL |
| Stake Tokens | ~1.0s | ~0.003 SOL |
| DLMM Position | ~2.0s | ~0.012 SOL |

## 🤝 Community & Support

- **Discord**: [Saros Dev Station](https://discord.gg/saros)
- **Documentation**: [docs.saros.finance](https://docs.saros.finance)
- **GitHub**: [github.com/saros-finance](https://github.com/saros-finance)
- **Twitter**: [@SarosFinance](https://twitter.com/SarosFinance)

## 🛡 Security Considerations

- Always validate input parameters
- Use appropriate slippage settings
- Implement proper error handling
- Test thoroughly on devnet first
- Review [Security Best Practices](./guides/best-practices.md)

## 📝 License

This documentation is provided under the MIT License. See [LICENSE](./LICENSE) for details.

## 🙏 Contributing

We welcome contributions! Please see our [Contributing Guide](./CONTRIBUTING.md) for details.

## 🎖 Acknowledgments

Built for the Saros SDK Documentation Challenge on Superteam Earn.

---

<div align="center">

**Ready to build?** Start with our [Quick Start Guide](./quick-start/typescript-sdk.md) →

*Making DeFi accessible, one integration at a time.*

</div>
