# Saros SDK Implementation Guide

## SDK Feature Matrix

### @saros-finance/sdk (TypeScript) - v2.4.0

#### Core Modules
```typescript
import sarosSdk, {
  // Swap Functions
  getSwapAmountSaros,
  swapSaros,
  
  // Pool Functions
  createPool,
  getPoolInfo,
  depositAllTokenTypes,
  withdrawAllTokenTypes,
  
  // Utility Functions
  convertBalanceToWei,
  getTokenMintInfo,
  getTokenAccountInfo,
  getInfoTokenByMint,
  genConnectionSolana
} from '@saros-finance/sdk';

// Services
const { SarosFarmService, SarosStakeServices } = sarosSdk;
```

#### Key Program Addresses
```typescript
const TOKEN_PROGRAM_ID = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const SAROS_SWAP_PROGRAM_ADDRESS_V1 = new PublicKey('SSwapUtytfBdBn1b9NUGG6foMVPtcWgpRU32HToDUZr');
const SAROS_FARM_ADDRESS = new PublicKey('SFarmWM5wLFNEw1q5ofqL7CrwBMwdcqQgK6oQuoBGZJ');
const FEE_OWNER = 'FDbLZ5DRo61queVRH9LL1mQnsiAoubQEnoCRuPEmH9M8';
```

#### Main Features to Document
1. **Token Swaps**
   - `getSwapAmountSaros()` - Calculate swap amounts with slippage
   - `swapSaros()` - Execute token swaps
   - Slippage management
   - Price impact calculations

2. **Liquidity Pool Operations**
   - `createPool()` - Create new liquidity pools
   - `getPoolInfo()` - Fetch pool metadata
   - `depositAllTokenTypes()` - Add liquidity
   - `withdrawAllTokenTypes()` - Remove liquidity
   - Curve types (0 = normal, 1 = stable)

3. **Farming Operations**
   - `SarosFarmService.getListPool()` - List all farms
   - `SarosFarmService.stakePool()` - Stake LP tokens
   - `SarosFarmService.unStakePool()` - Unstake LP tokens
   - `SarosFarmService.claimReward()` - Harvest rewards

4. **Staking Operations**
   - `SarosStakeServices.getListPool()` - List staking pools
   - Staking rewards management
   - Auto-compounding features

### @saros-finance/dlmm-sdk (TypeScript) - v1.3.2

#### Core Modules
```typescript
import {
  BIN_STEP_CONFIGS,
  LiquidityBookServices,
  MODE,
} from "@saros-finance/dlmm-sdk";

import {
  LiquidityShape,
  PositionInfo,
  RemoveLiquidityType,
} from "@saros-finance/dlmm-sdk/types/services";

import {
  createUniformDistribution,
  findPosition,
  getBinRange,
  getMaxBinArray,
  getMaxPosition,
} from "@saros-finance/dlmm-sdk/utils";
```

#### Initialization
```typescript
const liquidityBookServices = new LiquidityBookServices({
  mode: MODE.MAINNET, // or MODE.DEVNET
});
```

#### Main Features to Document
1. **DLMM Swaps**
   - `getQuote()` - Get swap quotes with price impact
   - `swap()` - Execute DLMM swaps
   - Exact input/output modes
   - Hook support for rewards

2. **Liquidity Positions**
   - `createPairWithConfig()` - Create DLMM pools
   - `getUserPositions()` - Fetch user positions
   - `getPairAccount()` - Get pair information
   - Liquidity shapes (Spot, Curve, Bid-Ask)
   - Bin range management

3. **Advanced Features**
   - Concentrated liquidity strategies
   - Bin step configurations
   - Active bin tracking
   - Jupiter integration support
   - WebSocket support for new pools

4. **Utility Functions**
   - `getDexName()` - Get DEX identifier
   - `getDexProgramId()` - Get program ID
   - `fetchPoolAddresses()` - List all pools
   - `fetchPoolMetadata()` - Get pool details
   - `listenNewPoolAddress()` - Subscribe to new pools

### saros-dlmm-sdk-rs (Rust)

#### Key Features
- Jupiter AMM trait implementation
- High-performance DLMM operations
- On-chain program integration
- Type-safe Rust interfaces

#### Testing
```bash
cargo test -- --nocapture
```

## Documentation Priorities

### Phase 1: Core Functionality (Must Have)
1. **Quick Start Guides**
   - Environment setup (Solana CLI, Node.js, Rust)
   - First swap in < 5 minutes
   - Basic liquidity provision

2. **Essential Tutorials**
   - Token swap implementation
   - Add/Remove liquidity
   - Staking rewards
   - DLMM concentrated liquidity

3. **Working Examples**
   - Simple swap (AMM)
   - DLMM swap with slippage
   - Liquidity provision with farming
   - Position management

### Phase 2: Advanced Features (Nice to Have)
1. **Advanced Tutorials**
   - Liquidity strategies (Spot, Curve, Bid-Ask)
   - Multi-position management
   - Yield optimization
   - MEV protection

2. **Integration Guides**
   - Jupiter aggregation
   - Wallet integration (Phantom, Solflare)
   - Transaction building patterns
   - Error recovery strategies

3. **Performance & Security**
   - Gas optimization
   - Batch transactions
   - Security best practices
   - Rate limiting

## Code Example Templates

### Basic Swap Template
```typescript
// Tutorial should include:
// 1. Connection setup
// 2. Token selection
// 3. Amount calculation
// 4. Slippage configuration
// 5. Transaction execution
// 6. Error handling
// 7. Transaction confirmation
```

### Liquidity Provision Template
```typescript
// Tutorial should include:
// 1. Pool selection/creation
// 2. Token ratio calculation
// 3. LP token minting
// 4. Position tracking
// 5. Impermanent loss explanation
// 6. Fee earnings calculation
```

### DLMM Position Template
```typescript
// Tutorial should include:
// 1. Bin step selection
// 2. Price range setting
// 3. Liquidity distribution
// 4. Position creation
// 5. Rebalancing strategies
// 6. Fee collection
```

## Testing Requirements

### Devnet Testing
- All examples must work on devnet
- Use devnet faucets for SOL
- Test token mints for examples

### Mainnet Validation
- Critical paths tested on mainnet
- Real token pairs (SOL/USDC, etc.)
- Production-ready error handling

## Documentation Quality Checklist

### For Each Tutorial
- [ ] Clear prerequisites listed
- [ ] Step-by-step instructions
- [ ] Complete code examples
- [ ] Common errors and solutions
- [ ] Expected outputs shown
- [ ] Time estimate provided

### For Each Example
- [ ] Fully runnable code
- [ ] Dependencies in package.json
- [ ] Environment variables documented
- [ ] Comments explaining each step
- [ ] Error handling implemented
- [ ] Success/failure indicators

## Special Focus Areas

### DLMM Documentation (Competitive Advantage)
The DLMM SDK is newer and less documented, making it a key differentiator:
- Comprehensive liquidity shapes guide
- Visual bin distribution diagrams
- Strategy comparison charts
- Real-world use cases
- Performance benchmarks

### Integration Patterns
- Modular code architecture
- Reusable utility functions
- TypeScript best practices
- Async/await patterns
- WebSocket subscriptions

### Developer Experience
- Copy-paste friendly snippets
- Progressive complexity
- Troubleshooting guides
- FAQ section
- Community examples

## Resource Links for Implementation

### NPM Packages
- [@saros-finance/sdk](https://www.npmjs.com/package/@saros-finance/sdk)
- [@saros-finance/dlmm-sdk](https://www.npmjs.com/package/@saros-finance/dlmm-sdk)

### GitHub Repositories
- [saros-xyz/saros-sdk](https://github.com/saros-xyz/saros-sdk)
- [saros-xyz/saros-dlmm-sdk-rs](https://github.com/saros-xyz/saros-dlmm-sdk-rs)

### Official Documentation
- [Saros Integration Guide](https://docs.saros.xyz/integration)
- [Saros DLMM Documentation](https://docs.saros.xyz)

---

*This guide serves as the technical foundation for creating comprehensive Saros SDK documentation.*
