# DLMM SDK API Reference

## @saros-finance/dlmm-sdk v1.3.2

Complete API documentation for Saros Finance's Dynamic Liquidity Market Maker (DLMM) SDK.

---

## Table of Contents

- [Introduction](#introduction)
- [Installation](#installation)
- [Core Concepts](#core-concepts)
- [DLMM Client](#dlmm-client)
- [Position Management](#position-management)
- [Bin Operations](#bin-operations)
- [Liquidity Functions](#liquidity-functions)
- [Quote System](#quote-system)
- [Oracle Integration](#oracle-integration)
- [Types and Interfaces](#types-and-interfaces)
- [Constants](#constants)
- [Advanced Features](#advanced-features)
- [Error Handling](#error-handling)

---

## Introduction

The DLMM SDK provides access to Saros Finance's concentrated liquidity protocol with bin-based liquidity distribution. DLMM offers:

- **Concentrated Liquidity**: Up to 4000x capital efficiency
- **Bin-based System**: Discrete price levels for precise control
- **Zero Slippage Within Bins**: Perfect execution at bin prices
- **Dynamic Fees**: Volatility-responsive fee structure
- **Range Orders**: Limit order functionality

---

## Installation

```bash
npm install @saros-finance/dlmm-sdk @solana/web3.js decimal.js
```

## Core Concepts

### Bins
Discrete price levels where liquidity is deposited. Each bin has a unique ID and price range.

### Active Bin
The current trading bin where swaps occur.

### Bin Step
The price increment between adjacent bins (in basis points).

### Liquidity Shape
Distribution pattern of liquidity across bins (Uniform, Normal, Curve, Bid-Ask).

---

## DLMM Client

### DLMMClient

Main client for interacting with DLMM pools.

```typescript
class DLMMClient {
  constructor(config: DLMMConfig)
  
  // Properties
  connection: Connection
  programId: PublicKey
  
  // Core Methods
  async initialize(): Promise<void>
  async getPools(filters?: PoolFilter): Promise<DLMMPool[]>
  async getPool(poolAddress: PublicKey): Promise<DLMMPool>
  async createPosition(params: CreatePositionParams): Promise<Position>
  async closePosition(positionId: PublicKey): Promise<TransactionSignature>
}
```

#### DLMMConfig

```typescript
interface DLMMConfig {
  rpcUrl: string
  cluster?: Cluster
  programId?: PublicKey
  wallet?: Wallet
}
```

---

## Position Management

### createPosition

Create a new liquidity position with custom range and distribution.

```typescript
async function createPosition(
  client: DLMMClient,
  params: CreatePositionParams
): Promise<PositionResult>
```

#### CreatePositionParams

```typescript
interface CreatePositionParams {
  poolAddress: PublicKey
  lowerBinId: number
  upperBinId: number
  totalLiquidity: BN
  distributionMode: LiquidityDistribution
  slippage?: number
  lockDuration?: number  // Optional lock for boosted rewards
}
```

#### LiquidityDistribution

```typescript
type LiquidityDistribution = 
  | { type: 'SPOT' }                    // Concentrate at active bin
  | { type: 'CURVE'; alpha: number }     // Curve distribution
  | { type: 'UNIFORM' }                  // Even distribution
  | { type: 'NORMAL'; sigma: number }    // Normal distribution
  | { type: 'BID_ASK' }                  // Split at edges
```

#### PositionResult

```typescript
interface PositionResult {
  positionId: PublicKey
  signature: string
  lowerPrice: number
  upperPrice: number
  activeBins: number[]
  tokenXDeposited: BN
  tokenYDeposited: BN
  shareOfPool: number
}
```

### updatePosition

Modify an existing position (add/remove liquidity, adjust range).

```typescript
async function updatePosition(
  client: DLMMClient,
  positionId: PublicKey,
  params: UpdatePositionParams
): Promise<UpdateResult>
```

#### UpdatePositionParams

```typescript
interface UpdatePositionParams {
  action: 'ADD' | 'REMOVE' | 'REBALANCE'
  amount?: BN
  newLowerBinId?: number
  newUpperBinId?: number
  newDistribution?: LiquidityDistribution
}
```

### getPosition

Get detailed position information.

```typescript
async function getPosition(
  client: DLMMClient,
  positionId: PublicKey
): Promise<PositionInfo>
```

#### PositionInfo

```typescript
interface PositionInfo {
  id: PublicKey
  owner: PublicKey
  pool: PublicKey
  lowerBinId: number
  upperBinId: number
  liquidity: BN
  tokenXAmount: BN
  tokenYAmount: BN
  feesEarned: {
    tokenX: BN
    tokenY: BN
  }
  rewards: RewardInfo[]
  inRange: boolean
  createdAt: number
  lastUpdated: number
}
```

### getAllPositions

Get all positions for a wallet.

```typescript
async function getAllPositions(
  client: DLMMClient,
  wallet: PublicKey
): Promise<PositionInfo[]>
```

---

## Bin Operations

### getBin

Get information about a specific bin.

```typescript
async function getBin(
  client: DLMMClient,
  poolAddress: PublicKey,
  binId: number
): Promise<BinInfo>
```

#### BinInfo

```typescript
interface BinInfo {
  id: number
  price: number
  pricePerToken: number
  liquidityX: BN
  liquidityY: BN
  totalLiquidity: BN
  feeRate: number
  isActive: boolean
}
```

### getActiveBin

Get the current active trading bin.

```typescript
async function getActiveBin(
  client: DLMMClient,
  poolAddress: PublicKey
): Promise<ActiveBinInfo>
```

#### ActiveBinInfo

```typescript
interface ActiveBinInfo extends BinInfo {
  volume24h: number
  priceChange24h: number
  lastUpdate: number
}
```

### getBinRange

Get information about a range of bins.

```typescript
async function getBinRange(
  client: DLMMClient,
  poolAddress: PublicKey,
  startBinId: number,
  endBinId: number
): Promise<BinInfo[]>
```

### calculateBinPrice

Calculate the price for a given bin ID.

```typescript
function calculateBinPrice(
  binId: number,
  binStep: number,
  basePrice: number
): number
```

---

## Liquidity Functions

### addLiquidity

Add liquidity to specific bins.

```typescript
async function addLiquidity(
  client: DLMMClient,
  params: AddLiquidityParams
): Promise<AddLiquidityResult>
```

#### AddLiquidityParams

```typescript
interface AddLiquidityParams {
  positionId: PublicKey
  amountX?: BN
  amountY?: BN
  bins?: BinLiquidity[]  // Specific bin allocations
  autoBalance?: boolean   // Auto-balance tokens
  slippage?: number
}

interface BinLiquidity {
  binId: number
  liquidityShare: number  // Percentage of total
}
```

### removeLiquidity

Remove liquidity from position.

```typescript
async function removeLiquidity(
  client: DLMMClient,
  params: RemoveLiquidityParams
): Promise<RemoveLiquidityResult>
```

#### RemoveLiquidityParams

```typescript
interface RemoveLiquidityParams {
  positionId: PublicKey
  liquidityAmount: BN     // Amount to remove
  withdrawMode: 'BALANCED' | 'TOKEN_X' | 'TOKEN_Y'
  bins?: number[]         // Specific bins to remove from
}
```

### migratePosition

Migrate position to new range.

```typescript
async function migratePosition(
  client: DLMMClient,
  params: MigrateParams
): Promise<MigrateResult>
```

#### MigrateParams

```typescript
interface MigrateParams {
  oldPositionId: PublicKey
  newLowerBinId: number
  newUpperBinId: number
  newDistribution: LiquidityDistribution
  closeOld?: boolean
}
```

---

## Quote System

### getQuote

Get swap quote with detailed execution path.

```typescript
async function getQuote(
  client: DLMMClient,
  params: QuoteParams
): Promise<Quote>
```

#### QuoteParams

```typescript
interface QuoteParams {
  poolAddress: PublicKey
  tokenIn: PublicKey
  tokenOut: PublicKey
  amountIn: BN
  slippage?: number
  includeRoute?: boolean
}
```

#### Quote

```typescript
interface Quote {
  amountIn: BN
  amountOut: BN
  minAmountOut: BN
  priceImpact: number
  fee: BN
  feePercentage: number
  route: BinRoute[]
  executionPrice: number
  priceAfterSwap: number
}

interface BinRoute {
  binId: number
  price: number
  amountIn: BN
  amountOut: BN
  liquidity: BN
}
```

### simulateSwap

Simulate swap execution without sending transaction.

```typescript
async function simulateSwap(
  client: DLMMClient,
  params: SwapParams
): Promise<SimulationResult>
```

#### SimulationResult

```typescript
interface SimulationResult {
  success: boolean
  amountOut: BN
  priceImpact: number
  binsCrossed: number
  finalBinId: number
  gasEstimate: number
  warnings: string[]
}
```

---

## Oracle Integration

### getPriceFromOracle

Get oracle price for accurate quoting.

```typescript
async function getPriceFromOracle(
  client: DLMMClient,
  poolAddress: PublicKey
): Promise<OraclePrice>
```

#### OraclePrice

```typescript
interface OraclePrice {
  price: number
  confidence: number
  lastUpdate: number
  source: 'PYTH' | 'SWITCHBOARD' | 'CHAINLINK'
}
```

### updateOraclePrice

Update pool's oracle price feed.

```typescript
async function updateOraclePrice(
  client: DLMMClient,
  poolAddress: PublicKey,
  oracleAddress: PublicKey
): Promise<TransactionSignature>
```

---

## Types and Interfaces

### DLMMPool

```typescript
interface DLMMPool {
  address: PublicKey
  tokenX: TokenInfo
  tokenY: TokenInfo
  binStep: number
  activeBinId: number
  feeRate: number
  protocolFee: number
  maxBinId: number
  minBinId: number
  totalLiquidity: BN
  reserves: {
    tokenX: BN
    tokenY: BN
  }
  oracle?: PublicKey
  rewards: RewardInfo[]
  statistics: PoolStatistics
}
```

### PoolStatistics

```typescript
interface PoolStatistics {
  volume24h: number
  volume7d: number
  fees24h: number
  tvl: number
  apr: number
  numberOfPositions: number
  priceChange24h: number
  volatility: number
}
```

### RewardInfo

```typescript
interface RewardInfo {
  token: PublicKey
  amount: BN
  duration: number
  rate: number
  remaining: BN
}
```

### BinArray

```typescript
interface BinArray {
  index: number
  version: number
  bins: Bin[]
}

interface Bin {
  id: number
  xAmount: BN
  yAmount: BN
  price: number
  liquiditySupply: BN
  reserveX: BN
  reserveY: BN
  feeX: BN
  feeY: BN
}
```

### FeeParameters

```typescript
interface FeeParameters {
  baseFee: number         // Base fee in bps
  maxFee: number         // Maximum fee in bps
  protocolShare: number  // Protocol's share of fees
  volatilityFee: number  // Additional fee based on volatility
}
```

---

## Constants

### Program Constants

```typescript
const DLMM_PROGRAM_ID = new PublicKey('DLMM6eXv5FypxmDtLjXzN7cVvh4eFjnZPY4f5gxiPJq')
const MAX_BIN_ID = 443636
const MIN_BIN_ID = -443636
const MAX_BINS_PER_POSITION = 70
const BASIS_POINT_MAX = 10000
```

### Bin Steps

```typescript
const BIN_STEPS = {
  STABLE: 1,      // 0.01% - for stable pairs
  LOW: 10,        // 0.10% - low volatility
  MEDIUM: 20,     // 0.20% - medium volatility
  HIGH: 50,       // 0.50% - high volatility
  EXTREME: 100    // 1.00% - extreme volatility
}
```

### Fee Tiers

```typescript
const FEE_TIERS = {
  STABLE: 1,      // 0.01%
  LOW: 5,         // 0.05%
  MEDIUM: 30,     // 0.30%
  HIGH: 100       // 1.00%
}
```

---

## Advanced Features

### Range Orders

Place limit orders using concentrated liquidity.

```typescript
async function placeRangeOrder(
  client: DLMMClient,
  params: RangeOrderParams
): Promise<RangeOrderResult>
```

#### RangeOrderParams

```typescript
interface RangeOrderParams {
  poolAddress: PublicKey
  orderType: 'BUY' | 'SELL'
  price: number
  amount: BN
  tolerance: number  // Price range tolerance
}
```

### Auto-Rebalancing

Set up automatic position rebalancing.

```typescript
async function enableAutoRebalance(
  client: DLMMClient,
  positionId: PublicKey,
  strategy: RebalanceStrategy
): Promise<void>
```

#### RebalanceStrategy

```typescript
interface RebalanceStrategy {
  trigger: 'OUT_OF_RANGE' | 'IMPERMANENT_LOSS' | 'TIME'
  threshold?: number
  targetDistribution: LiquidityDistribution
  maxGasPrice?: number
}
```

### Yield Optimization

Optimize position for maximum yield.

```typescript
async function optimizePosition(
  client: DLMMClient,
  params: OptimizeParams
): Promise<OptimizedPosition>
```

#### OptimizeParams

```typescript
interface OptimizeParams {
  poolAddress: PublicKey
  capital: BN
  targetAPR?: number
  riskTolerance: 'LOW' | 'MEDIUM' | 'HIGH'
  timeHorizon: number  // days
}
```

---

## Error Handling

### DLMMError

```typescript
class DLMMError extends Error {
  code: DLMMErrorCode
  details?: any
  
  constructor(message: string, code: DLMMErrorCode, details?: any)
}
```

### Error Codes

```typescript
enum DLMMErrorCode {
  BIN_NOT_FOUND = 'BIN_NOT_FOUND',
  POSITION_NOT_FOUND = 'POSITION_NOT_FOUND',
  OUT_OF_RANGE = 'OUT_OF_RANGE',
  INSUFFICIENT_LIQUIDITY = 'INSUFFICIENT_LIQUIDITY',
  INVALID_BIN_ID = 'INVALID_BIN_ID',
  MAX_BINS_EXCEEDED = 'MAX_BINS_EXCEEDED',
  SLIPPAGE_EXCEEDED = 'SLIPPAGE_EXCEEDED',
  INVALID_DISTRIBUTION = 'INVALID_DISTRIBUTION',
  ORACLE_STALE = 'ORACLE_STALE'
}
```

### Error Recovery

```typescript
try {
  const position = await createPosition(client, params);
} catch (error) {
  if (error instanceof DLMMError) {
    switch (error.code) {
      case DLMMErrorCode.OUT_OF_RANGE:
        // Suggest new range based on current price
        const newRange = await suggestRange(client, pool);
        break;
      case DLMMErrorCode.INSUFFICIENT_LIQUIDITY:
        // Find alternative pools
        const alternatives = await findAlternativePools(client);
        break;
    }
  }
}
```

---

## Best Practices

1. **Choose appropriate bin steps** based on pair volatility
2. **Monitor active bin** to keep positions in range
3. **Use oracle prices** for accurate quotes
4. **Implement rebalancing strategies** for concentrated positions
5. **Set reasonable slippage** based on liquidity depth
6. **Batch operations** to save on gas
7. **Cache bin arrays** to reduce RPC calls

---

## Migration from AMM

### Key Differences

| Feature | AMM | DLMM |
|---------|-----|------|
| Capital Efficiency | 1x | Up to 4000x |
| Price Range | 0 to ∞ | Custom range |
| Slippage | Always present | Zero within bins |
| Fees | Fixed | Dynamic |
| IL Risk | Standard | Amplified in range |

### Migration Example

```typescript
// AMM Position
const ammPosition = {
  pool: 'AMM_POOL',
  liquidity: 10000,
  tokens: { A: 5000, B: 5000 }
};

// Equivalent DLMM Position
const dlmmParams = {
  poolAddress: DLMM_POOL,
  lowerBinId: activeBin - 50,  // ±50 bins from current
  upperBinId: activeBin + 50,
  totalLiquidity: new BN(10000),
  distributionMode: { type: 'NORMAL', sigma: 1.5 }
};

const dlmmPosition = await createPosition(client, dlmmParams);
// Achieves similar exposure with potentially higher returns
```

---

## Support

- [GitHub](https://github.com/saros-finance/dlmm-sdk)
- [Discord](https://discord.gg/saros)
- [Docs](https://docs.saros.finance/dlmm)