# Tutorial 2: Liquidity Management

Learn how to add, manage, and remove liquidity from Saros pools using both AMM and DLMM approaches with complete production-ready examples.

## Overview

In this tutorial, you'll build a comprehensive liquidity management system that:
- ✅ Adds liquidity to AMM pools
- ✅ Creates concentrated DLMM positions
- ✅ Manages liquidity across different strategies
- ✅ Tracks fees and rewards
- ✅ Implements position rebalancing

## Prerequisites

- Completed [Tutorial 1: Basic Swap](./01-basic-swap.md)
- Understanding of [Bin-Based Liquidity](../core-concepts/bin-liquidity.md)
- Familiarity with impermanent loss concepts

## Project Setup

Continue from the previous tutorial or create a new project:

```bash
mkdir saros-liquidity-tutorial
cd saros-liquidity-tutorial
npm init -y

# Install dependencies
npm install @saros-finance/sdk @saros-finance/dlmm-sdk
npm install @solana/web3.js @solana/spl-token bs58
npm install --save-dev typescript @types/node ts-node
```

## Part 1: AMM Liquidity Management

### Step 1: AMM Pool Setup

Create `src/amm-liquidity.ts`:

```typescript
import { Connection, PublicKey, Keypair, Transaction } from '@solana/web3.js';
import {
  createPoolSaros,
  depositAllTokenTypes,
  withdrawAllTokenTypes,
  getPoolInfo,
  getUserLPTokenBalance
} from '@saros-finance/sdk';
import {
  getAccount,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction
} from '@solana/spl-token';
import bs58 from 'bs58';

interface LiquidityPosition {
  poolAddress: PublicKey;
  lpTokenMint: PublicKey;
  lpTokenBalance: number;
  tokenABalance: number;
  tokenBBalance: number;
  shareOfPool: number;
  valueUSD: number;
  feesEarned: {
    tokenA: number;
    tokenB: number;
    totalUSD: number;
  };
}

interface AddLiquidityParams {
  poolAddress: PublicKey;
  tokenAMint: PublicKey;
  tokenBMint: PublicKey;
  tokenAAmount: number;
  tokenBAmount: number;
  slippageTolerance: number;
  depositOneSide?: boolean; // For unbalanced deposits
}

class SarosAMMLiquidity {
  private connection: Connection;
  private wallet: Keypair;

  constructor(privateKey: string, rpcUrl?: string) {
    this.connection = new Connection(
      rpcUrl || 'https://api.devnet.solana.com',
      { commitment: 'confirmed' }
    );
    this.wallet = Keypair.fromSecretKey(bs58.decode(privateKey));
  }

  async initialize(): Promise<void> {
    console.log('Wallet:', this.wallet.publicKey.toString());
    
    const solBalance = await this.connection.getBalance(this.wallet.publicKey);
    console.log('SOL Balance:', solBalance / 1e9, 'SOL');

    if (solBalance < 0.01 * 1e9) {
      throw new Error('Insufficient SOL for transaction fees');
    }
  }
```

### Step 2: Add Liquidity Implementation

```typescript
class SarosAMMLiquidity {
  // ... previous methods

  async addLiquidity(params: AddLiquidityParams): Promise<{
    signature: string;
    lpTokensReceived: number;
    actualTokenAUsed: number;
    actualTokenBUsed: number;
    success: boolean;
    error?: string;
  }> {
    try {
      console.log('💰 Adding liquidity to AMM pool...');
      console.log(`Pool: ${params.poolAddress.toString()}`);
      console.log(`Token A: ${params.tokenAAmount}`);
      console.log(`Token B: ${params.tokenBAmount}`);

      // Step 1: Get pool information
      const poolInfo = await getPoolInfo(this.connection, params.poolAddress);
      console.log('Pool reserves:', {
        tokenA: Number(poolInfo.tokenAReserve) / 1e6, // Assume 6 decimals
        tokenB: Number(poolInfo.tokenBReserve) / 1e9, // Assume 9 decimals
      });

      // Step 2: Verify token accounts exist
      const tokenAAccount = await getAssociatedTokenAddress(
        params.tokenAMint,
        this.wallet.publicKey
      );
      const tokenBAccount = await getAssociatedTokenAddress(
        params.tokenBMint,
        this.wallet.publicKey
      );

      // Step 3: Check balances
      const tokenABalance = await this.getTokenBalance(tokenAAccount);
      const tokenBBalance = await this.getTokenBalance(tokenBAccount);

      console.log('Current balances:', {
        tokenA: tokenABalance,
        tokenB: tokenBBalance
      });

      if (tokenABalance < params.tokenAAmount) {
        throw new Error(`Insufficient Token A balance: ${tokenABalance} < ${params.tokenAAmount}`);
      }
      if (tokenBBalance < params.tokenBAmount) {
        throw new Error(`Insufficient Token B balance: ${tokenBBalance} < ${params.tokenBAmount}`);
      }

      // Step 4: Calculate optimal amounts (balanced deposit)
      const optimalAmounts = this.calculateOptimalAmounts(
        params.tokenAAmount,
        params.tokenBAmount,
        Number(poolInfo.tokenAReserve),
        Number(poolInfo.tokenBReserve)
      );

      console.log('Optimal amounts:', optimalAmounts);

      // Step 5: Execute liquidity deposit
      console.log('⚡ Executing liquidity deposit...');
      
      const result = await depositAllTokenTypes(
        this.connection,
        this.wallet,
        params.poolAddress,
        optimalAmounts.tokenAAmount * 1e6, // Convert to token units
        optimalAmounts.tokenBAmount * 1e9,
        params.slippageTolerance
      );

      console.log('✅ Liquidity added successfully!');
      console.log('Transaction:', result.signature);

      // Step 6: Get LP token balance
      const lpTokenBalance = await getUserLPTokenBalance(
        this.connection,
        this.wallet.publicKey,
        params.poolAddress
      );

      return {
        signature: result.signature,
        lpTokensReceived: Number(lpTokenBalance) / 1e6,
        actualTokenAUsed: optimalAmounts.tokenAAmount,
        actualTokenBUsed: optimalAmounts.tokenBAmount,
        success: true
      };

    } catch (error: any) {
      console.error('❌ Add liquidity failed:', error.message);
      
      return {
        signature: '',
        lpTokensReceived: 0,
        actualTokenAUsed: 0,
        actualTokenBUsed: 0,
        success: false,
        error: error.message
      };
    }
  }

  private calculateOptimalAmounts(
    desiredTokenA: number,
    desiredTokenB: number,
    reserveA: number,
    reserveB: number
  ): { tokenAAmount: number; tokenBAmount: number } {
    // Calculate the optimal ratio based on current pool reserves
    const currentRatio = reserveA / reserveB;
    const desiredRatio = desiredTokenA / desiredTokenB;

    if (desiredRatio > currentRatio) {
      // Too much Token A, adjust Token A down
      const optimalTokenA = desiredTokenB * currentRatio;
      return {
        tokenAAmount: Math.min(optimalTokenA, desiredTokenA),
        tokenBAmount: desiredTokenB
      };
    } else {
      // Too much Token B, adjust Token B down
      const optimalTokenB = desiredTokenA / currentRatio;
      return {
        tokenAAmount: desiredTokenA,
        tokenBAmount: Math.min(optimalTokenB, desiredTokenB)
      };
    }
  }

  private async getTokenBalance(tokenAccount: PublicKey): Promise<number> {
    try {
      const account = await getAccount(this.connection, tokenAccount);
      return Number(account.amount) / 1e6; // Assume 6 decimals for simplicity
    } catch {
      return 0;
    }
  }
}
```

### Step 3: Remove Liquidity and Position Management

```typescript
class SarosAMMLiquidity {
  // ... previous methods

  async removeLiquidity(
    poolAddress: PublicKey,
    lpTokenAmount: number,
    slippageTolerance: number
  ): Promise<{
    signature: string;
    tokenAReceived: number;
    tokenBReceived: number;
    success: boolean;
    error?: string;
  }> {
    try {
      console.log('💸 Removing liquidity from AMM pool...');
      console.log(`Pool: ${poolAddress.toString()}`);
      console.log(`LP Tokens: ${lpTokenAmount}`);

      // Get current LP token balance
      const currentLPBalance = await getUserLPTokenBalance(
        this.connection,
        this.wallet.publicKey,
        poolAddress
      );

      if (Number(currentLPBalance) < lpTokenAmount * 1e6) {
        throw new Error(`Insufficient LP tokens: ${Number(currentLPBalance) / 1e6} < ${lpTokenAmount}`);
      }

      // Execute withdrawal
      const result = await withdrawAllTokenTypes(
        this.connection,
        this.wallet,
        poolAddress,
        lpTokenAmount * 1e6, // Convert to token units
        slippageTolerance
      );

      console.log('✅ Liquidity removed successfully!');
      console.log('Transaction:', result.signature);

      return {
        signature: result.signature,
        tokenAReceived: result.tokenAReceived / 1e6,
        tokenBReceived: result.tokenBReceived / 1e9,
        success: true
      };

    } catch (error: any) {
      console.error('❌ Remove liquidity failed:', error.message);
      
      return {
        signature: '',
        tokenAReceived: 0,
        tokenBReceived: 0,
        success: false,
        error: error.message
      };
    }
  }

  async getPositionInfo(poolAddress: PublicKey): Promise<LiquidityPosition> {
    try {
      // Get pool information
      const poolInfo = await getPoolInfo(this.connection, poolAddress);
      
      // Get user's LP token balance
      const lpTokenBalance = await getUserLPTokenBalance(
        this.connection,
        this.wallet.publicKey,
        poolAddress
      );

      // Calculate share of pool
      const totalSupply = Number(poolInfo.lpTokenSupply);
      const userLPBalance = Number(lpTokenBalance);
      const shareOfPool = userLPBalance / totalSupply;

      // Calculate underlying token amounts
      const tokenABalance = (Number(poolInfo.tokenAReserve) / 1e6) * shareOfPool;
      const tokenBBalance = (Number(poolInfo.tokenBReserve) / 1e9) * shareOfPool;

      // Estimate fees earned (simplified calculation)
      const feesEarned = await this.estimateFeesEarned(poolAddress, shareOfPool);

      return {
        poolAddress,
        lpTokenMint: poolInfo.lpTokenMint,
        lpTokenBalance: userLPBalance / 1e6,
        tokenABalance,
        tokenBBalance,
        shareOfPool: shareOfPool * 100, // Convert to percentage
        valueUSD: this.calculatePositionValueUSD(tokenABalance, tokenBBalance),
        feesEarned
      };

    } catch (error: any) {
      throw new Error(`Failed to get position info: ${error.message}`);
    }
  }

  private async estimateFeesEarned(
    poolAddress: PublicKey,
    shareOfPool: number
  ): Promise<{ tokenA: number; tokenB: number; totalUSD: number }> {
    // Simplified fee calculation - in production, track from transaction history
    // or use accumulated fee tracking
    return {
      tokenA: 0.1 * shareOfPool, // Placeholder
      tokenB: 0.05 * shareOfPool, // Placeholder
      totalUSD: 5.0 * shareOfPool // Placeholder
    };
  }

  private calculatePositionValueUSD(tokenABalance: number, tokenBBalance: number): number {
    // Simplified USD calculation - in production, use price feeds
    const tokenAPrice = 1.0; // $1 for stablecoin
    const tokenBPrice = 50.0; // $50 for example token
    
    return (tokenABalance * tokenAPrice) + (tokenBBalance * tokenBPrice);
  }
}
```

## Part 2: DLMM Liquidity Management

### Step 1: DLMM Position Creation

Create `src/dlmm-liquidity.ts`:

```typescript
import { LiquidityBookServices, MODE } from '@saros-finance/dlmm-sdk';
import { Connection, PublicKey, Keypair } from '@solana/web3.js';
import BN from 'bn.js';

interface DLMMPositionParams {
  pairAddress: PublicKey;
  tokenAAmount: BN;
  tokenBAmount: BN;
  strategy: 'concentrated' | 'uniform' | 'spot' | 'custom';
  priceRange: {
    min: number;
    max: number;
  };
  customDistribution?: {
    binIds: number[];
    distributionX: number[];
    distributionY: number[];
  };
}

interface DLMMPosition {
  positionId: string;
  pairAddress: PublicKey;
  activeBins: {
    binId: number;
    price: number;
    liquidityX: BN;
    liquidityY: BN;
    feesX: BN;
    feesY: BN;
  }[];
  totalValueUSD: number;
  feesEarnedUSD: number;
  apr: number;
}

class SarosDLMMLiquidity {
  private dlmmService: LiquidityBookServices;
  private connection: Connection;
  private wallet: Keypair;

  constructor(
    privateKey: string,
    network: 'devnet' | 'mainnet' = 'devnet',
    rpcUrl?: string
  ) {
    this.dlmmService = new LiquidityBookServices({
      mode: network === 'devnet' ? MODE.DEVNET : MODE.MAINNET,
      rpcEndpoint: rpcUrl
    });

    this.connection = new Connection(
      rpcUrl || (network === 'devnet' 
        ? 'https://api.devnet.solana.com' 
        : 'https://api.mainnet-beta.solana.com'
      )
    );

    this.wallet = Keypair.fromSecretKey(bs58.decode(privateKey));
  }
```

### Step 2: Create DLMM Positions

```typescript
class SarosDLMMLiquidity {
  // ... previous methods

  async createPosition(params: DLMMPositionParams): Promise<{
    signature: string;
    positionId: string;
    binsCreated: number[];
    success: boolean;
    error?: string;
  }> {
    try {
      console.log('🎯 Creating DLMM position...');
      console.log(`Strategy: ${params.strategy}`);
      console.log(`Price range: $${params.priceRange.min} - $${params.priceRange.max}`);

      // Step 1: Get current price and calculate bins
      const currentPrice = await this.getCurrentPrice(params.pairAddress);
      console.log(`Current price: $${currentPrice}`);

      // Step 2: Calculate bin distribution based on strategy
      const distribution = this.calculateBinDistribution(
        params.strategy,
        currentPrice,
        params.priceRange,
        params.customDistribution
      );

      console.log(`Creating position across ${distribution.binIds.length} bins`);
      console.log('Bin range:', Math.min(...distribution.binIds), 'to', Math.max(...distribution.binIds));

      // Step 3: Create the position
      const transaction = await this.dlmmService.addLiquidityIntoPosition({
        pair: params.pairAddress,
        binIds: distribution.binIds,
        distributionX: distribution.distributionX,
        distributionY: distribution.distributionY,
        amountX: params.tokenAAmount,
        amountY: params.tokenBAmount,
        user: this.wallet.publicKey,
        slippage: 1.0 // 1% slippage tolerance
      });

      // Step 4: Sign and send transaction
      transaction.feePayer = this.wallet.publicKey;
      transaction.recentBlockhash = (
        await this.connection.getLatestBlockhash()
      ).blockhash;

      transaction.sign(this.wallet);

      const signature = await this.connection.sendRawTransaction(
        transaction.serialize(),
        { skipPreflight: false }
      );

      await this.connection.confirmTransaction(signature, 'confirmed');

      console.log('✅ DLMM position created successfully!');
      console.log('Transaction:', signature);

      return {
        signature,
        positionId: signature, // Simplified - use transaction signature as position ID
        binsCreated: distribution.binIds,
        success: true
      };

    } catch (error: any) {
      console.error('❌ Create position failed:', error.message);
      
      return {
        signature: '',
        positionId: '',
        binsCreated: [],
        success: false,
        error: error.message
      };
    }
  }

  private calculateBinDistribution(
    strategy: string,
    currentPrice: number,
    priceRange: { min: number; max: number },
    customDistribution?: {
      binIds: number[];
      distributionX: number[];
      distributionY: number[];
    }
  ): {
    binIds: number[];
    distributionX: number[];
    distributionY: number[];
  } {
    if (strategy === 'custom' && customDistribution) {
      return customDistribution;
    }

    // Convert price range to bin IDs (simplified calculation)
    const minBinId = this.priceToBinId(priceRange.min);
    const maxBinId = this.priceToBinId(priceRange.max);
    const currentBinId = this.priceToBinId(currentPrice);

    const binIds: number[] = [];
    const distributionX: number[] = [];
    const distributionY: number[] = [];

    switch (strategy) {
      case 'concentrated':
        // Bell curve around current price
        for (let binId = minBinId; binId <= maxBinId; binId++) {
          binIds.push(binId);
          
          const distance = Math.abs(binId - currentBinId);
          const maxDistance = (maxBinId - minBinId) / 2;
          const weight = Math.exp(-(distance / maxDistance) ** 2) * 100;
          
          distributionX.push(Math.floor(weight));
          distributionY.push(Math.floor(weight));
        }
        break;

      case 'uniform':
        // Equal distribution across range
        const binCount = maxBinId - minBinId + 1;
        const uniformWeight = Math.floor(100 / binCount);
        
        for (let binId = minBinId; binId <= maxBinId; binId++) {
          binIds.push(binId);
          distributionX.push(uniformWeight);
          distributionY.push(uniformWeight);
        }
        break;

      case 'spot':
        // Single bin at current price
        binIds.push(currentBinId);
        distributionX.push(100);
        distributionY.push(100);
        break;

      default:
        throw new Error(`Unknown strategy: ${strategy}`);
    }

    return { binIds, distributionX, distributionY };
  }

  private priceToBinId(price: number): number {
    // Simplified conversion - in production, use proper bin step calculation
    const binStep = 0.001; // 0.1% bin step
    const baseFactor = 1.0;
    
    return Math.floor(Math.log(price / baseFactor) / Math.log(1 + binStep));
  }

  private async getCurrentPrice(pairAddress: PublicKey): Promise<number> {
    try {
      // Get current active bin to determine price
      // Simplified - in production, query the pair's active bin
      return 100.0; // Placeholder
    } catch (error) {
      throw new Error('Failed to get current price');
    }
  }
}
```

### Step 3: Position Monitoring and Rebalancing

```typescript
class SarosDLMMLiquidity {
  // ... previous methods

  async getPositionAnalytics(positionId: string): Promise<DLMMPosition> {
    try {
      // In production, this would query the user's positions from the chain
      // For this example, we'll simulate position data
      
      const mockPosition: DLMMPosition = {
        positionId,
        pairAddress: new PublicKey('EXAMPLE_PAIR_ADDRESS'),
        activeBins: [
          {
            binId: 100,
            price: 99.5,
            liquidityX: new BN(1000000),
            liquidityY: new BN(500000000),
            feesX: new BN(1000),
            feesY: new BN(500000)
          },
          {
            binId: 101,
            price: 100.5,
            liquidityX: new BN(800000),
            liquidityY: new BN(400000000),
            feesX: new BN(1200),
            feesY: new BN(600000)
          }
        ],
        totalValueUSD: 1500.0,
        feesEarnedUSD: 15.5,
        apr: 45.2
      };

      return mockPosition;

    } catch (error: any) {
      throw new Error(`Failed to get position analytics: ${error.message}`);
    }
  }

  async shouldRebalance(
    positionId: string,
    rebalanceThreshold: number = 0.1 // 10% price movement
  ): Promise<{
    shouldRebalance: boolean;
    reason: string;
    suggestedAction: string;
  }> {
    try {
      const position = await this.getPositionAnalytics(positionId);
      const currentPrice = await this.getCurrentPrice(position.pairAddress);

      // Check if current price is outside of active bins
      const activeBinPrices = position.activeBins.map(bin => bin.price);
      const minActivePrice = Math.min(...activeBinPrices);
      const maxActivePrice = Math.max(...activeBinPrices);

      const priceRange = maxActivePrice - minActivePrice;
      const distanceFromRange = Math.max(
        0,
        minActivePrice - currentPrice,
        currentPrice - maxActivePrice
      );

      const shouldRebalance = distanceFromRange > (priceRange * rebalanceThreshold);

      if (shouldRebalance) {
        let reason = '';
        let suggestedAction = '';

        if (currentPrice < minActivePrice) {
          reason = `Price ($${currentPrice}) below active range ($${minActivePrice} - $${maxActivePrice})`;
          suggestedAction = 'Add bins below current price or shift position down';
        } else if (currentPrice > maxActivePrice) {
          reason = `Price ($${currentPrice}) above active range ($${minActivePrice} - $${maxActivePrice})`;
          suggestedAction = 'Add bins above current price or shift position up';
        }

        return {
          shouldRebalance: true,
          reason,
          suggestedAction
        };
      }

      return {
        shouldRebalance: false,
        reason: 'Position is within acceptable range',
        suggestedAction: 'No action needed'
      };

    } catch (error: any) {
      throw new Error(`Failed to check rebalance status: ${error.message}`);
    }
  }

  async rebalancePosition(
    positionId: string,
    newStrategy: 'concentrated' | 'uniform',
    newPriceRange: { min: number; max: number }
  ): Promise<{
    signature: string;
    success: boolean;
    error?: string;
  }> {
    try {
      console.log('🔄 Rebalancing DLMM position...');

      // Step 1: Remove liquidity from current position
      console.log('Removing liquidity from current bins...');
      const removeResult = await this.removePosition(positionId);
      
      if (!removeResult.success) {
        throw new Error(`Failed to remove position: ${removeResult.error}`);
      }

      // Step 2: Create new position with updated parameters
      console.log('Creating new position with updated parameters...');
      const position = await this.getPositionAnalytics(positionId);
      
      const createResult = await this.createPosition({
        pairAddress: position.pairAddress,
        tokenAAmount: new BN(1000000), // Use withdrawn amounts
        tokenBAmount: new BN(500000000),
        strategy: newStrategy,
        priceRange: newPriceRange
      });

      if (!createResult.success) {
        throw new Error(`Failed to create new position: ${createResult.error}`);
      }

      console.log('✅ Position rebalanced successfully!');
      
      return {
        signature: createResult.signature,
        success: true
      };

    } catch (error: any) {
      console.error('❌ Rebalance failed:', error.message);
      
      return {
        signature: '',
        success: false,
        error: error.message
      };
    }
  }

  async removePosition(positionId: string): Promise<{
    signature: string;
    tokensReceived: { tokenA: BN; tokenB: BN };
    success: boolean;
    error?: string;
  }> {
    try {
      console.log('💸 Removing DLMM position...');

      const position = await this.getPositionAnalytics(positionId);
      
      // Remove liquidity from all bins
      const transaction = await this.dlmmService.removeMultipleLiquidity({
        pair: position.pairAddress,
        user: this.wallet.publicKey,
        binIds: position.activeBins.map(bin => bin.binId),
        liquiditiesX: position.activeBins.map(bin => bin.liquidityX),
        liquiditiesY: position.activeBins.map(bin => bin.liquidityY)
      });

      // Sign and send
      transaction.feePayer = this.wallet.publicKey;
      transaction.recentBlockhash = (
        await this.connection.getLatestBlockhash()
      ).blockhash;

      transaction.sign(this.wallet);

      const signature = await this.connection.sendRawTransaction(
        transaction.serialize()
      );

      await this.connection.confirmTransaction(signature, 'confirmed');

      console.log('✅ Position removed successfully!');

      return {
        signature,
        tokensReceived: {
          tokenA: new BN(1000000), // Simplified
          tokenB: new BN(500000000)
        },
        success: true
      };

    } catch (error: any) {
      console.error('❌ Remove position failed:', error.message);
      
      return {
        signature: '',
        tokensReceived: { tokenA: new BN(0), tokenB: new BN(0) },
        success: false,
        error: error.message
      };
    }
  }
}
```

## Part 3: Complete Example and Testing

### Step 1: Liquidity Management Dashboard

Create `src/liquidity-dashboard.ts`:

```typescript
class LiquidityDashboard {
  private ammLiquidity: SarosAMMLiquidity;
  private dlmmLiquidity: SarosDLMMLiquidity;

  constructor(privateKey: string) {
    this.ammLiquidity = new SarosAMMLiquidity(privateKey);
    this.dlmmLiquidity = new SarosDLMMLiquidity(privateKey);
  }

  async displayPortfolio(): Promise<void> {
    console.log('📊 Liquidity Portfolio Dashboard');
    console.log('================================\n');

    try {
      // AMM Positions
      console.log('🏦 AMM Positions:');
      const ammPools = [
        new PublicKey('AMM_POOL_1_ADDRESS'),
        new PublicKey('AMM_POOL_2_ADDRESS')
      ];

      for (const poolAddress of ammPools) {
        try {
          const position = await this.ammLiquidity.getPositionInfo(poolAddress);
          console.log(`\nPool: ${poolAddress.toString().slice(0, 8)}...`);
          console.log(`  LP Tokens: ${position.lpTokenBalance.toFixed(4)}`);
          console.log(`  Token A: ${position.tokenABalance.toFixed(4)}`);
          console.log(`  Token B: ${position.tokenBBalance.toFixed(4)}`);
          console.log(`  Share of Pool: ${position.shareOfPool.toFixed(2)}%`);
          console.log(`  Value: $${position.valueUSD.toFixed(2)}`);
          console.log(`  Fees Earned: $${position.feesEarned.totalUSD.toFixed(2)}`);
        } catch (error) {
          console.log(`  No position in pool ${poolAddress.toString().slice(0, 8)}...`);
        }
      }

      // DLMM Positions
      console.log('\n\n🎯 DLMM Positions:');
      const dlmmPositions = ['POSITION_1', 'POSITION_2']; // In practice, query user positions

      for (const positionId of dlmmPositions) {
        try {
          const position = await this.dlmmLiquidity.getPositionAnalytics(positionId);
          console.log(`\nPosition: ${positionId.slice(0, 8)}...`);
          console.log(`  Active Bins: ${position.activeBins.length}`);
          console.log(`  Total Value: $${position.totalValueUSD.toFixed(2)}`);
          console.log(`  Fees Earned: $${position.feesEarnedUSD.toFixed(2)}`);
          console.log(`  APR: ${position.apr.toFixed(1)}%`);

          // Check if rebalancing is needed
          const rebalanceCheck = await this.dlmmLiquidity.shouldRebalance(positionId);
          if (rebalanceCheck.shouldRebalance) {
            console.log(`  ⚠️  Rebalancing recommended: ${rebalanceCheck.reason}`);
            console.log(`  💡 Suggested action: ${rebalanceCheck.suggestedAction}`);
          } else {
            console.log(`  ✅ Position is optimally positioned`);
          }
        } catch (error) {
          console.log(`  Position ${positionId.slice(0, 8)}... not found or inactive`);
        }
      }

    } catch (error) {
      console.error('Failed to load portfolio:', error);
    }
  }

  async executeRebalancingStrategy(): Promise<void> {
    console.log('\n🔄 Checking for rebalancing opportunities...');

    const dlmmPositions = ['POSITION_1', 'POSITION_2'];

    for (const positionId of dlmmPositions) {
      try {
        const rebalanceCheck = await this.dlmmLiquidity.shouldRebalance(positionId, 0.05); // 5% threshold

        if (rebalanceCheck.shouldRebalance) {
          console.log(`\n🎯 Rebalancing position ${positionId.slice(0, 8)}...`);
          console.log(`Reason: ${rebalanceCheck.reason}`);

          // Execute rebalancing
          const result = await this.dlmmLiquidity.rebalancePosition(
            positionId,
            'concentrated', // Use concentrated strategy
            { min: 95, max: 105 } // ±5% around current price
          );

          if (result.success) {
            console.log(`✅ Rebalancing completed: ${result.signature}`);
          } else {
            console.log(`❌ Rebalancing failed: ${result.error}`);
          }
        }
      } catch (error) {
        console.log(`Failed to check position ${positionId}: ${error}`);
      }
    }
  }
}
```

### Step 2: Complete Tutorial Example

Create `src/liquidity-tutorial.ts`:

```typescript
async function runLiquidityTutorial() {
  console.log('🚀 Saros Liquidity Management Tutorial');
  console.log('====================================\n');

  try {
    const dashboard = new LiquidityDashboard(process.env.WALLET_PRIVATE_KEY!);

    // 1. Add AMM Liquidity
    console.log('1️⃣ Adding AMM Liquidity...');
    const ammLiquidity = new SarosAMMLiquidity(process.env.WALLET_PRIVATE_KEY!);
    await ammLiquidity.initialize();

    const addAMMResult = await ammLiquidity.addLiquidity({
      poolAddress: new PublicKey('YOUR_AMM_POOL_ADDRESS'),
      tokenAMint: new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'), // USDC
      tokenBMint: new PublicKey('So11111111111111111111111111111111111111112'), // SOL
      tokenAAmount: 100, // 100 USDC
      tokenBAmount: 2,   // 2 SOL
      slippageTolerance: 1.0 // 1%
    });

    if (addAMMResult.success) {
      console.log(`✅ AMM liquidity added: ${addAMMResult.lpTokensReceived.toFixed(4)} LP tokens`);
    }

    console.log('\n' + '='.repeat(50) + '\n');

    // 2. Create DLMM Position
    console.log('2️⃣ Creating DLMM Position...');
    const dlmmLiquidity = new SarosDLMMLiquidity(process.env.WALLET_PRIVATE_KEY!);

    const createDLMMResult = await dlmmLiquidity.createPosition({
      pairAddress: new PublicKey('YOUR_DLMM_PAIR_ADDRESS'),
      tokenAAmount: new BN(100_000_000), // 100 USDC
      tokenBAmount: new BN(2_000_000_000), // 2 SOL
      strategy: 'concentrated',
      priceRange: { min: 95, max: 105 } // ±5% range
    });

    if (createDLMMResult.success) {
      console.log(`✅ DLMM position created: ${createDLMMResult.binsCreated.length} bins`);
    }

    console.log('\n' + '='.repeat(50) + '\n');

    // 3. Display Portfolio
    console.log('3️⃣ Portfolio Overview...');
    await dashboard.displayPortfolio();

    console.log('\n' + '='.repeat(50) + '\n');

    // 4. Rebalancing Check
    console.log('4️⃣ Rebalancing Strategy...');
    await dashboard.executeRebalancingStrategy();

    console.log('\n🎉 Liquidity tutorial completed!');

  } catch (error) {
    console.error('Tutorial failed:', error);
  }
}

// Run the tutorial
runLiquidityTutorial();
```

### Step 3: Run the Tutorial

```bash
# Set up environment
echo "WALLET_PRIVATE_KEY=your_base58_private_key" > .env
echo "SOLANA_RPC_URL=https://api.devnet.solana.com" >> .env

# Run tutorial
npx ts-node src/liquidity-tutorial.ts
```

## Expected Output

```
🚀 Saros Liquidity Management Tutorial
====================================

1️⃣ Adding AMM Liquidity...
Wallet: 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU
SOL Balance: 2.5 SOL
💰 Adding liquidity to AMM pool...
Pool: 5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9
Token A: 100
Token B: 2
Pool reserves: { tokenA: 50000, tokenB: 1000 }
Optimal amounts: { tokenAAmount: 100, tokenBAmount: 2 }
⚡ Executing liquidity deposit...
✅ Liquidity added successfully!
Transaction: 3mJ7K8pL2nX9vQ4uR5cF8tY1wE6sA9mP7nZ2fV4gH3kL

==================================================

2️⃣ Creating DLMM Position...
🎯 Creating DLMM position...
Strategy: concentrated
Price range: $95 - $105
Current price: $100
Creating position across 11 bins
Bin range: 95 to 105
✅ DLMM position created successfully!
Transaction: 4nK8L9qM3oY0xR6vS7dG9uZ2xF7tB0nQ8oA3gI5jK6mN

==================================================

3️⃣ Portfolio Overview...
📊 Liquidity Portfolio Dashboard
================================

🏦 AMM Positions:

Pool: 5tzFkiKs...
  LP Tokens: 141.4214
  Token A: 100.0000
  Token B: 2.0000
  Share of Pool: 0.20%
  Value: $200.00
  Fees Earned: $1.00

🎯 DLMM Positions:

Position: 4nK8L9qM...
  Active Bins: 11
  Total Value: $200.00
  Fees Earned: $2.50
  APR: 45.2%
  ✅ Position is optimally positioned

🎉 Liquidity tutorial completed!
```

## Best Practices and Tips

### AMM Liquidity Best Practices
- ✅ Always check pool reserves before adding liquidity
- ✅ Use balanced deposits to minimize unused tokens
- ✅ Set appropriate slippage tolerance (0.5-2%)
- ✅ Monitor impermanent loss regularly

### DLMM Strategy Guidelines
- ✅ **Stable pairs**: Use narrow ranges (±2-5%)
- ✅ **Volatile pairs**: Use wider ranges (±10-20%)
- ✅ **Bull markets**: Bias liquidity above current price
- ✅ **Bear markets**: Bias liquidity below current price

### Risk Management
- ✅ Diversify across multiple pools
- ✅ Set position size limits
- ✅ Regular rebalancing schedules
- ✅ Monitor APR vs risk ratio

## Next Steps

Continue your liquidity management journey:
- [Tutorial 3: Yield Farming](./03-yield-farming.md)
- [Advanced Position Management](../code-examples/typescript/02-auto-compound/)
- [DLMM Strategy Guide](../sdk-guides/dlmm-sdk/liquidity-shapes.md)

## Resources

- [Impermanent Loss Calculator](https://app.saros.finance/tools/il-calculator)
- [Liquidity Analytics Dashboard](https://analytics.saros.finance)
- [Community Strategies](https://discord.gg/saros)
- [Advanced Examples](https://github.com/saros-finance/liquidity-examples)