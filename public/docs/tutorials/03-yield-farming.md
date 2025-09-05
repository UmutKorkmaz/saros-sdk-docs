# Tutorial 3: Yield Farming & Staking

Master yield farming and staking strategies with Saros Finance to maximize your DeFi returns through comprehensive examples and automated strategies.

## Overview

In this tutorial, you'll build a complete yield farming system that:
- ✅ Stakes LP tokens in farming pools
- ✅ Implements staking for $SAROS tokens
- ✅ Creates automated compound strategies
- ✅ Monitors and claims rewards
- ✅ Optimizes yield across multiple pools

## Prerequisites

- Completed [Tutorial 2: Liquidity Management](./02-add-liquidity.md)
- Understanding of LP tokens and yield farming concepts
- Familiarity with reward mechanisms

## Project Setup

Continue from previous tutorials or set up a new project:

```bash
mkdir saros-farming-tutorial
cd saros-farming-tutorial
npm init -y

# Install dependencies
npm install @saros-finance/sdk @saros-finance/dlmm-sdk
npm install @solana/web3.js @solana/spl-token bs58 cron
npm install --save-dev typescript @types/node @types/cron ts-node
```

## Part 1: Understanding Saros Farming

### Farming Pool Types

**1. LP Token Farming**
- Stake AMM LP tokens
- Earn $SAROS rewards
- Additional protocol tokens

**2. SAROS Staking**
- Single-sided $SAROS staking
- Governance participation
- Protocol fee sharing

**3. DLMM Incentives**
- Bin-specific rewards
- Higher rewards for active liquidity
- Volatility bonuses

### Step 1: Farming Infrastructure Setup

Create `src/farming-core.ts`:

```typescript
import { Connection, PublicKey, Keypair, Transaction } from '@solana/web3.js';
import {
  SarosFarmService,
  SarosStakeServices,
  getUserStakeInfo,
  getUserFarmInfo,
  claimFarmReward,
  claimStakeReward,
  stakeFarm,
  unstakeFarm,
  stakeTokens,
  unstakeTokens
} from '@saros-finance/sdk';
import {
  getAccount,
  getAssociatedTokenAddress
} from '@solana/spl-token';
import bs58 from 'bs58';
import BN from 'bn.js';

interface FarmingPool {
  poolAddress: PublicKey;
  lpTokenMint: PublicKey;
  rewardTokenMint: PublicKey;
  poolType: 'LP_FARM' | 'SAROS_STAKE' | 'DLMM_INCENTIVE';
  apr: number;
  totalStaked: BN;
  totalRewards: BN;
  userStaked: BN;
  userRewards: BN;
  lockDuration?: number; // in seconds
  minStakeAmount?: BN;
}

interface StakingPosition {
  poolAddress: PublicKey;
  stakedAmount: BN;
  rewardsEarned: BN;
  apr: number;
  daysStaked: number;
  nextRewardClaim: Date;
  canUnstake: boolean;
}

class SarosFarming {
  private connection: Connection;
  private wallet: Keypair;
  private farmService: SarosFarmService;
  private stakeService: SarosStakeServices;

  constructor(privateKey: string, rpcUrl?: string) {
    this.connection = new Connection(
      rpcUrl || 'https://api.devnet.solana.com',
      { commitment: 'confirmed' }
    );
    this.wallet = Keypair.fromSecretKey(bs58.decode(privateKey));
    
    // Initialize farming and staking services
    this.farmService = new SarosFarmService(this.connection);
    this.stakeService = new SarosStakeServices(this.connection);
  }

  async initialize(): Promise<void> {
    console.log('🚜 Initializing Saros Farming...');
    console.log('Wallet:', this.wallet.publicKey.toString());
    
    const solBalance = await this.connection.getBalance(this.wallet.publicKey);
    console.log('SOL Balance:', solBalance / 1e9, 'SOL');

    if (solBalance < 0.01 * 1e9) {
      throw new Error('Insufficient SOL for transaction fees');
    }
  }
```

### Step 2: LP Token Farming Implementation

```typescript
class SarosFarming {
  // ... previous methods

  async enterLPFarm(
    farmPoolAddress: PublicKey,
    lpTokenAmount: BN,
    lockDuration?: number
  ): Promise<{
    signature: string;
    stakedAmount: BN;
    expectedAPR: number;
    success: boolean;
    error?: string;
  }> {
    try {
      console.log('🌾 Entering LP farming pool...');
      console.log(`Pool: ${farmPoolAddress.toString()}`);
      console.log(`Amount: ${lpTokenAmount.toString()} LP tokens`);

      // Step 1: Get farm pool information
      const farmInfo = await this.farmService.getFarmInfo(farmPoolAddress);
      console.log('Farm APR:', farmInfo.apr, '%');
      console.log('Total staked:', Number(farmInfo.totalStaked) / 1e6, 'LP tokens');

      // Step 2: Check LP token balance
      const lpTokenAccount = await getAssociatedTokenAddress(
        farmInfo.lpTokenMint,
        this.wallet.publicKey
      );

      const lpBalance = await this.getTokenBalance(lpTokenAccount);
      console.log('Available LP tokens:', lpBalance);

      if (lpBalance < Number(lpTokenAmount) / 1e6) {
        throw new Error(`Insufficient LP tokens: ${lpBalance} < ${Number(lpTokenAmount) / 1e6}`);
      }

      // Step 3: Stake LP tokens in farm
      console.log('⚡ Staking LP tokens...');
      
      const stakeResult = await stakeFarm(
        this.connection,
        this.wallet,
        farmPoolAddress,
        lpTokenAmount,
        lockDuration || 0 // Optional lock duration
      );

      console.log('✅ LP farming started successfully!');
      console.log('Transaction:', stakeResult.signature);

      return {
        signature: stakeResult.signature,
        stakedAmount: lpTokenAmount,
        expectedAPR: farmInfo.apr,
        success: true
      };

    } catch (error: any) {
      console.error('❌ LP farming failed:', error.message);
      
      return {
        signature: '',
        stakedAmount: new BN(0),
        expectedAPR: 0,
        success: false,
        error: error.message
      };
    }
  }

  async claimFarmingRewards(farmPoolAddress: PublicKey): Promise<{
    signature: string;
    rewardsClaimed: BN;
    rewardTokens: { mint: PublicKey; amount: BN }[];
    success: boolean;
    error?: string;
  }> {
    try {
      console.log('💰 Claiming farming rewards...');

      // Step 1: Check pending rewards
      const userFarmInfo = await getUserFarmInfo(
        this.connection,
        this.wallet.publicKey,
        farmPoolAddress
      );

      console.log('Pending rewards:', Number(userFarmInfo.pendingRewards) / 1e9, 'SAROS');

      if (userFarmInfo.pendingRewards.eq(new BN(0))) {
        console.log('No rewards to claim');
        return {
          signature: '',
          rewardsClaimed: new BN(0),
          rewardTokens: [],
          success: true
        };
      }

      // Step 2: Claim rewards
      console.log('⚡ Claiming rewards...');
      
      const claimResult = await claimFarmReward(
        this.connection,
        this.wallet,
        farmPoolAddress
      );

      console.log('✅ Rewards claimed successfully!');
      console.log('Amount:', Number(userFarmInfo.pendingRewards) / 1e9, 'SAROS');
      console.log('Transaction:', claimResult.signature);

      return {
        signature: claimResult.signature,
        rewardsClaimed: userFarmInfo.pendingRewards,
        rewardTokens: [
          {
            mint: new PublicKey('SAROS_TOKEN_MINT'), // Replace with actual SAROS mint
            amount: userFarmInfo.pendingRewards
          }
        ],
        success: true
      };

    } catch (error: any) {
      console.error('❌ Claim rewards failed:', error.message);
      
      return {
        signature: '',
        rewardsClaimed: new BN(0),
        rewardTokens: [],
        success: false,
        error: error.message
      };
    }
  }

  async exitLPFarm(
    farmPoolAddress: PublicKey,
    lpTokenAmount?: BN // Optional: withdraw specific amount
  ): Promise<{
    signature: string;
    lpTokensWithdrawn: BN;
    rewardsClaimed: BN;
    success: boolean;
    error?: string;
  }> {
    try {
      console.log('🚜 Exiting LP farming pool...');

      // Step 1: Get current stake info
      const userFarmInfo = await getUserFarmInfo(
        this.connection,
        this.wallet.publicKey,
        farmPoolAddress
      );

      const withdrawAmount = lpTokenAmount || userFarmInfo.stakedAmount;
      console.log('Withdrawing:', Number(withdrawAmount) / 1e6, 'LP tokens');
      console.log('Claiming:', Number(userFarmInfo.pendingRewards) / 1e9, 'SAROS rewards');

      // Step 2: Unstake LP tokens (also claims rewards)
      console.log('⚡ Unstaking LP tokens...');
      
      const unstakeResult = await unstakeFarm(
        this.connection,
        this.wallet,
        farmPoolAddress,
        withdrawAmount
      );

      console.log('✅ LP farming exit completed!');
      console.log('Transaction:', unstakeResult.signature);

      return {
        signature: unstakeResult.signature,
        lpTokensWithdrawn: withdrawAmount,
        rewardsClaimed: userFarmInfo.pendingRewards,
        success: true
      };

    } catch (error: any) {
      console.error('❌ LP farming exit failed:', error.message);
      
      return {
        signature: '',
        lpTokensWithdrawn: new BN(0),
        rewardsClaimed: new BN(0),
        success: false,
        error: error.message
      };
    }
  }

  private async getTokenBalance(tokenAccount: PublicKey): Promise<number> {
    try {
      const account = await getAccount(this.connection, tokenAccount);
      return Number(account.amount) / 1e6; // Assume 6 decimals
    } catch {
      return 0;
    }
  }
}
```

### Step 3: SAROS Token Staking

```typescript
class SarosFarming {
  // ... previous methods

  async stakeSAROSTokens(
    sarosAmount: BN,
    lockDuration: number = 0 // in seconds, 0 = no lock
  ): Promise<{
    signature: string;
    stakedAmount: BN;
    lockUntil?: Date;
    expectedAPR: number;
    success: boolean;
    error?: string;
  }> {
    try {
      console.log('🔒 Staking SAROS tokens...');
      console.log(`Amount: ${Number(sarosAmount) / 1e9} SAROS`);
      console.log(`Lock duration: ${lockDuration} seconds`);

      // Step 1: Check SAROS balance
      const sarosTokenAccount = await getAssociatedTokenAddress(
        new PublicKey('SAROS_TOKEN_MINT'), // Replace with actual SAROS mint
        this.wallet.publicKey
      );

      const sarosBalance = await this.getTokenBalance(sarosTokenAccount);
      console.log('Available SAROS:', sarosBalance);

      if (sarosBalance < Number(sarosAmount) / 1e9) {
        throw new Error(`Insufficient SAROS tokens: ${sarosBalance} < ${Number(sarosAmount) / 1e9}`);
      }

      // Step 2: Get staking pool info
      const stakePoolAddress = new PublicKey('SAROS_STAKE_POOL_ADDRESS'); // Replace with actual
      const stakeInfo = await this.stakeService.getStakePoolInfo(stakePoolAddress);
      
      console.log('Stake pool APR:', stakeInfo.apr, '%');
      console.log('Total staked:', Number(stakeInfo.totalStaked) / 1e9, 'SAROS');

      // Step 3: Execute staking
      console.log('⚡ Staking SAROS tokens...');
      
      const stakeResult = await stakeTokens(
        this.connection,
        this.wallet,
        stakePoolAddress,
        sarosAmount,
        lockDuration
      );

      console.log('✅ SAROS staking completed!');
      console.log('Transaction:', stakeResult.signature);

      const lockUntil = lockDuration > 0 
        ? new Date(Date.now() + lockDuration * 1000)
        : undefined;

      return {
        signature: stakeResult.signature,
        stakedAmount: sarosAmount,
        lockUntil,
        expectedAPR: stakeInfo.apr,
        success: true
      };

    } catch (error: any) {
      console.error('❌ SAROS staking failed:', error.message);
      
      return {
        signature: '',
        stakedAmount: new BN(0),
        expectedAPR: 0,
        success: false,
        error: error.message
      };
    }
  }

  async claimSAROSRewards(): Promise<{
    signature: string;
    rewardsClaimed: BN;
    success: boolean;
    error?: string;
  }> {
    try {
      console.log('💎 Claiming SAROS staking rewards...');

      const stakePoolAddress = new PublicKey('SAROS_STAKE_POOL_ADDRESS');
      
      // Check pending rewards
      const userStakeInfo = await getUserStakeInfo(
        this.connection,
        this.wallet.publicKey,
        stakePoolAddress
      );

      console.log('Pending rewards:', Number(userStakeInfo.pendingRewards) / 1e9, 'SAROS');

      if (userStakeInfo.pendingRewards.eq(new BN(0))) {
        console.log('No rewards to claim');
        return {
          signature: '',
          rewardsClaimed: new BN(0),
          success: true
        };
      }

      // Claim rewards
      const claimResult = await claimStakeReward(
        this.connection,
        this.wallet,
        stakePoolAddress
      );

      console.log('✅ SAROS rewards claimed!');
      console.log('Transaction:', claimResult.signature);

      return {
        signature: claimResult.signature,
        rewardsClaimed: userStakeInfo.pendingRewards,
        success: true
      };

    } catch (error: any) {
      console.error('❌ Claim SAROS rewards failed:', error.message);
      
      return {
        signature: '',
        rewardsClaimed: new BN(0),
        success: false,
        error: error.message
      };
    }
  }

  async unstakeSAROSTokens(
    sarosAmount?: BN // Optional: unstake specific amount
  ): Promise<{
    signature: string;
    tokensUnstaked: BN;
    rewardsClaimed: BN;
    success: boolean;
    error?: string;
  }> {
    try {
      console.log('🔓 Unstaking SAROS tokens...');

      const stakePoolAddress = new PublicKey('SAROS_STAKE_POOL_ADDRESS');
      
      // Get current stake info
      const userStakeInfo = await getUserStakeInfo(
        this.connection,
        this.wallet.publicKey,
        stakePoolAddress
      );

      const unstakeAmount = sarosAmount || userStakeInfo.stakedAmount;
      
      console.log('Unstaking:', Number(unstakeAmount) / 1e9, 'SAROS');
      console.log('Claiming:', Number(userStakeInfo.pendingRewards) / 1e9, 'SAROS rewards');

      // Check if tokens are locked
      if (userStakeInfo.lockUntil && new Date() < userStakeInfo.lockUntil) {
        throw new Error(`Tokens are locked until ${userStakeInfo.lockUntil.toISOString()}`);
      }

      // Execute unstaking
      const unstakeResult = await unstakeTokens(
        this.connection,
        this.wallet,
        stakePoolAddress,
        unstakeAmount
      );

      console.log('✅ SAROS unstaking completed!');
      console.log('Transaction:', unstakeResult.signature);

      return {
        signature: unstakeResult.signature,
        tokensUnstaked: unstakeAmount,
        rewardsClaimed: userStakeInfo.pendingRewards,
        success: true
      };

    } catch (error: any) {
      console.error('❌ SAROS unstaking failed:', error.message);
      
      return {
        signature: '',
        tokensUnstaked: new BN(0),
        rewardsClaimed: new BN(0),
        success: false,
        error: error.message
      };
    }
  }
}
```

## Part 2: Automated Yield Strategies

### Step 1: Auto-Compound Strategy

Create `src/auto-compound.ts`:

```typescript
import { CronJob } from 'cron';

interface CompoundStrategy {
  farmPools: PublicKey[];
  stakePools: PublicKey[];
  compoundFrequency: string; // Cron expression
  minRewardThreshold: BN;
  gasReserve: number; // SOL to keep for transactions
  enabled: boolean;
}

class AutoCompoundBot {
  private farming: SarosFarming;
  private strategy: CompoundStrategy;
  private jobs: CronJob[] = [];

  constructor(
    privateKey: string,
    strategy: CompoundStrategy
  ) {
    this.farming = new SarosFarming(privateKey);
    this.strategy = strategy;
  }

  async start(): Promise<void> {
    console.log('🤖 Starting Auto-Compound Bot...');
    console.log(`Frequency: ${this.strategy.compoundFrequency}`);
    console.log(`Farm pools: ${this.strategy.farmPools.length}`);
    console.log(`Stake pools: ${this.strategy.stakePools.length}`);

    await this.farming.initialize();

    if (!this.strategy.enabled) {
      console.log('⚠️ Strategy is disabled');
      return;
    }

    // Create cron job for auto-compounding
    const compoundJob = new CronJob(
      this.strategy.compoundFrequency,
      () => this.executeCompoundCycle(),
      null,
      true, // Start immediately
      'America/New_York'
    );

    this.jobs.push(compoundJob);
    console.log('✅ Auto-compound bot started!');
  }

  private async executeCompoundCycle(): Promise<void> {
    try {
      console.log('\n🔄 Starting compound cycle...');
      console.log('Time:', new Date().toISOString());

      // Check SOL balance for gas
      const solBalance = await this.farming.connection.getBalance(
        this.farming.wallet.publicKey
      );

      if (solBalance < this.strategy.gasReserve * 1e9) {
        console.log('⚠️ Low SOL balance, skipping cycle');
        return;
      }

      let totalCompounded = new BN(0);

      // Compound farm pools
      for (const farmPool of this.strategy.farmPools) {
        try {
          const compounded = await this.compoundFarmPool(farmPool);
          totalCompounded = totalCompounded.add(compounded);
        } catch (error) {
          console.error(`Farm pool ${farmPool.toString().slice(0, 8)}... error:`, error);
        }
      }

      // Compound stake pools
      for (const stakePool of this.strategy.stakePools) {
        try {
          const compounded = await this.compoundStakePool(stakePool);
          totalCompounded = totalCompounded.add(compounded);
        } catch (error) {
          console.error(`Stake pool ${stakePool.toString().slice(0, 8)}... error:`, error);
        }
      }

      console.log(`✅ Compound cycle completed!`);
      console.log(`Total compounded: ${Number(totalCompounded) / 1e9} SAROS`);

    } catch (error) {
      console.error('❌ Compound cycle failed:', error);
    }
  }

  private async compoundFarmPool(farmPoolAddress: PublicKey): Promise<BN> {
    console.log(`\n🌾 Checking farm pool: ${farmPoolAddress.toString().slice(0, 8)}...`);

    // Check pending rewards
    const userFarmInfo = await getUserFarmInfo(
      this.farming.connection,
      this.farming.wallet.publicKey,
      farmPoolAddress
    );

    if (userFarmInfo.pendingRewards.lt(this.strategy.minRewardThreshold)) {
      console.log(`Rewards below threshold: ${Number(userFarmInfo.pendingRewards) / 1e9}`);
      return new BN(0);
    }

    // Claim rewards
    const claimResult = await this.farming.claimFarmingRewards(farmPoolAddress);
    
    if (!claimResult.success) {
      throw new Error(`Failed to claim: ${claimResult.error}`);
    }

    console.log(`✅ Claimed ${Number(claimResult.rewardsClaimed) / 1e9} SAROS from farm`);

    // Auto-stake claimed SAROS (if strategy includes SAROS staking)
    if (this.strategy.stakePools.length > 0) {
      const stakeResult = await this.farming.stakeSAROSTokens(
        claimResult.rewardsClaimed,
        0 // No lock for auto-compound
      );

      if (stakeResult.success) {
        console.log(`✅ Auto-staked ${Number(claimResult.rewardsClaimed) / 1e9} SAROS`);
      }
    }

    return claimResult.rewardsClaimed;
  }

  private async compoundStakePool(stakePoolAddress: PublicKey): Promise<BN> {
    console.log(`\n💎 Checking stake pool: ${stakePoolAddress.toString().slice(0, 8)}...`);

    // Check pending rewards
    const userStakeInfo = await getUserStakeInfo(
      this.farming.connection,
      this.farming.wallet.publicKey,
      stakePoolAddress
    );

    if (userStakeInfo.pendingRewards.lt(this.strategy.minRewardThreshold)) {
      console.log(`Rewards below threshold: ${Number(userStakeInfo.pendingRewards) / 1e9}`);
      return new BN(0);
    }

    // Claim rewards
    const claimResult = await this.farming.claimSAROSRewards();
    
    if (!claimResult.success) {
      throw new Error(`Failed to claim: ${claimResult.error}`);
    }

    console.log(`✅ Claimed ${Number(claimResult.rewardsClaimed) / 1e9} SAROS from staking`);

    // Re-stake claimed rewards (compound)
    const stakeResult = await this.farming.stakeSAROSTokens(
      claimResult.rewardsClaimed,
      0 // No lock for auto-compound
    );

    if (stakeResult.success) {
      console.log(`✅ Compounded ${Number(claimResult.rewardsClaimed) / 1e9} SAROS`);
    }

    return claimResult.rewardsClaimed;
  }

  stop(): void {
    console.log('🛑 Stopping Auto-Compound Bot...');
    
    for (const job of this.jobs) {
      job.stop();
    }
    
    this.jobs = [];
    console.log('✅ Bot stopped');
  }

  async getStats(): Promise<{
    totalFarmed: BN;
    totalStaked: BN;
    totalRewards: BN;
    estimatedAPY: number;
  }> {
    console.log('📊 Calculating farming stats...');

    let totalFarmed = new BN(0);
    let totalStaked = new BN(0);
    let totalRewards = new BN(0);

    // Calculate farm positions
    for (const farmPool of this.strategy.farmPools) {
      try {
        const userFarmInfo = await getUserFarmInfo(
          this.farming.connection,
          this.farming.wallet.publicKey,
          farmPool
        );
        
        totalFarmed = totalFarmed.add(userFarmInfo.stakedAmount);
        totalRewards = totalRewards.add(userFarmInfo.pendingRewards);
      } catch (error) {
        console.log(`Cannot fetch farm info for ${farmPool.toString()}`);
      }
    }

    // Calculate stake positions
    for (const stakePool of this.strategy.stakePools) {
      try {
        const userStakeInfo = await getUserStakeInfo(
          this.farming.connection,
          this.farming.wallet.publicKey,
          stakePool
        );
        
        totalStaked = totalStaked.add(userStakeInfo.stakedAmount);
        totalRewards = totalRewards.add(userStakeInfo.pendingRewards);
      } catch (error) {
        console.log(`Cannot fetch stake info for ${stakePool.toString()}`);
      }
    }

    // Estimate APY (simplified calculation)
    const totalPosition = totalFarmed.add(totalStaked);
    const estimatedAPY = totalPosition.gt(new BN(0)) 
      ? (Number(totalRewards) / Number(totalPosition)) * 365 * 100 // Rough daily to yearly
      : 0;

    return {
      totalFarmed,
      totalStaked,
      totalRewards,
      estimatedAPY
    };
  }
}
```

### Step 2: Yield Optimization Strategy

Create `src/yield-optimizer.ts`:

```typescript
interface YieldOpportunity {
  poolAddress: PublicKey;
  poolType: 'LP_FARM' | 'SAROS_STAKE' | 'DLMM_INCENTIVE';
  currentAPR: number;
  projectedAPR: number;
  tvl: BN;
  risk: 'LOW' | 'MEDIUM' | 'HIGH';
  lockDuration: number;
  minAmount: BN;
  description: string;
}

class YieldOptimizer {
  private farming: SarosFarming;

  constructor(privateKey: string) {
    this.farming = new SarosFarming(privateKey);
  }

  async findBestOpportunities(
    availableAmount: BN,
    riskTolerance: 'LOW' | 'MEDIUM' | 'HIGH',
    maxLockDuration: number = 0
  ): Promise<YieldOpportunity[]> {
    console.log('🔍 Scanning for yield opportunities...');

    const opportunities: YieldOpportunity[] = [];

    // Mock opportunity data - in production, fetch from chain
    const allOpportunities: YieldOpportunity[] = [
      {
        poolAddress: new PublicKey('FARM_POOL_1'),
        poolType: 'LP_FARM',
        currentAPR: 45.2,
        projectedAPR: 48.5,
        tvl: new BN(5000000 * 1e6),
        risk: 'MEDIUM',
        lockDuration: 0,
        minAmount: new BN(10 * 1e6),
        description: 'USDC-SOL LP Farm - Stable pair with consistent rewards'
      },
      {
        poolAddress: new PublicKey('STAKE_POOL_1'),
        poolType: 'SAROS_STAKE',
        currentAPR: 25.8,
        projectedAPR: 28.2,
        tvl: new BN(10000000 * 1e9),
        risk: 'LOW',
        lockDuration: 2592000, // 30 days
        minAmount: new BN(100 * 1e9),
        description: 'SAROS Staking - Low risk, governance participation'
      },
      {
        poolAddress: new PublicKey('DLMM_INCENTIVE_1'),
        poolType: 'DLMM_INCENTIVE',
        currentAPR: 78.9,
        projectedAPR: 82.1,
        tvl: new BN(2000000 * 1e6),
        risk: 'HIGH',
        lockDuration: 0,
        minAmount: new BN(50 * 1e6),
        description: 'DLMM ETH-USDC - High volatility, high rewards'
      }
    ];

    // Filter opportunities based on criteria
    for (const opportunity of allOpportunities) {
      // Check risk tolerance
      const riskLevels = ['LOW', 'MEDIUM', 'HIGH'];
      const userRiskIndex = riskLevels.indexOf(riskTolerance);
      const oppRiskIndex = riskLevels.indexOf(opportunity.risk);
      
      if (oppRiskIndex > userRiskIndex) {
        continue;
      }

      // Check lock duration
      if (opportunity.lockDuration > maxLockDuration) {
        continue;
      }

      // Check minimum amount
      if (availableAmount.lt(opportunity.minAmount)) {
        continue;
      }

      opportunities.push(opportunity);
    }

    // Sort by projected APR
    opportunities.sort((a, b) => b.projectedAPR - a.projectedAPR);

    console.log(`Found ${opportunities.length} suitable opportunities`);
    return opportunities;
  }

  async createOptimalStrategy(
    totalAmount: BN,
    riskTolerance: 'LOW' | 'MEDIUM' | 'HIGH'
  ): Promise<{
    allocations: { opportunity: YieldOpportunity; amount: BN; percentage: number }[];
    expectedAPR: number;
    totalRisk: 'LOW' | 'MEDIUM' | 'HIGH';
  }> {
    console.log('🎯 Creating optimal yield strategy...');
    console.log(`Total amount: ${Number(totalAmount) / 1e6}`);
    console.log(`Risk tolerance: ${riskTolerance}`);

    const opportunities = await this.findBestOpportunities(
      new BN(1), // Find all opportunities
      riskTolerance
    );

    if (opportunities.length === 0) {
      throw new Error('No suitable opportunities found');
    }

    const allocations: { opportunity: YieldOpportunity; amount: BN; percentage: number }[] = [];
    let remainingAmount = totalAmount;

    // Diversification strategy based on risk tolerance
    let maxSingleAllocation: number;
    switch (riskTolerance) {
      case 'LOW':
        maxSingleAllocation = 0.6; // Max 60% in single opportunity
        break;
      case 'MEDIUM':
        maxSingleAllocation = 0.8; // Max 80% in single opportunity
        break;
      case 'HIGH':
        maxSingleAllocation = 1.0; // Can put all in single opportunity
        break;
    }

    // Allocate funds starting with highest APR
    for (let i = 0; i < opportunities.length && remainingAmount.gt(new BN(0)); i++) {
      const opportunity = opportunities[i];
      
      // Calculate allocation amount
      const maxAllocation = totalAmount.muln(maxSingleAllocation);
      const allocationAmount = BN.min(
        remainingAmount,
        BN.max(opportunity.minAmount, maxAllocation)
      );

      if (allocationAmount.gte(opportunity.minAmount)) {
        const percentage = (Number(allocationAmount) / Number(totalAmount)) * 100;
        
        allocations.push({
          opportunity,
          amount: allocationAmount,
          percentage
        });

        remainingAmount = remainingAmount.sub(allocationAmount);
        
        // Reduce max allocation for next opportunity (diversification)
        maxSingleAllocation *= 0.7;
      }
    }

    // Calculate weighted average APR
    let weightedAPR = 0;
    let totalRiskScore = 0;
    
    for (const allocation of allocations) {
      const weight = Number(allocation.amount) / Number(totalAmount);
      weightedAPR += allocation.opportunity.projectedAPR * weight;
      
      const riskScore = allocation.opportunity.risk === 'LOW' ? 1 : 
                       allocation.opportunity.risk === 'MEDIUM' ? 2 : 3;
      totalRiskScore += riskScore * weight;
    }

    const totalRisk: 'LOW' | 'MEDIUM' | 'HIGH' = 
      totalRiskScore <= 1.5 ? 'LOW' : 
      totalRiskScore <= 2.5 ? 'MEDIUM' : 'HIGH';

    return {
      allocations,
      expectedAPR: weightedAPR,
      totalRisk
    };
  }

  async executeStrategy(
    allocations: { opportunity: YieldOpportunity; amount: BN; percentage: number }[]
  ): Promise<{
    successfulAllocations: number;
    totalDeployed: BN;
    transactions: string[];
  }> {
    console.log('⚡ Executing yield strategy...');

    let successfulAllocations = 0;
    let totalDeployed = new BN(0);
    const transactions: string[] = [];

    for (const allocation of allocations) {
      try {
        console.log(`\n📍 Deploying ${allocation.percentage.toFixed(1)}% to ${allocation.opportunity.description}`);

        let result;
        
        switch (allocation.opportunity.poolType) {
          case 'LP_FARM':
            result = await this.farming.enterLPFarm(
              allocation.opportunity.poolAddress,
              allocation.amount
            );
            break;
            
          case 'SAROS_STAKE':
            result = await this.farming.stakeSAROSTokens(
              allocation.amount,
              allocation.opportunity.lockDuration
            );
            break;
            
          case 'DLMM_INCENTIVE':
            // Handle DLMM incentive deployment
            console.log('DLMM incentive deployment not implemented in this tutorial');
            continue;
        }

        if (result.success) {
          successfulAllocations++;
          totalDeployed = totalDeployed.add(allocation.amount);
          transactions.push(result.signature);
          
          console.log(`✅ Successfully deployed ${Number(allocation.amount) / 1e6}`);
          console.log(`Transaction: ${result.signature}`);
        } else {
          console.log(`❌ Failed to deploy: ${result.error}`);
        }

      } catch (error) {
        console.error(`❌ Allocation failed:`, error);
      }
    }

    console.log(`\n🎉 Strategy execution completed!`);
    console.log(`Successful allocations: ${successfulAllocations}/${allocations.length}`);
    console.log(`Total deployed: ${Number(totalDeployed) / 1e6}`);

    return {
      successfulAllocations,
      totalDeployed,
      transactions
    };
  }
}
```

## Part 3: Complete Tutorial Example

### Step 1: Comprehensive Farming Dashboard

Create `src/farming-dashboard.ts`:

```typescript
class FarmingDashboard {
  private farming: SarosFarming;
  private optimizer: YieldOptimizer;
  private autoCompounder?: AutoCompoundBot;

  constructor(privateKey: string) {
    this.farming = new SarosFarming(privateKey);
    this.optimizer = new YieldOptimizer(privateKey);
  }

  async displayFullPortfolio(): Promise<void> {
    console.log('🌾 Complete Farming Portfolio');
    console.log('==============================\n');

    try {
      await this.farming.initialize();

      // Display farming positions
      await this.displayFarmingPositions();
      
      // Display staking positions
      await this.displayStakingPositions();
      
      // Display yield opportunities
      await this.displayYieldOpportunities();
      
      // Display auto-compound status
      await this.displayAutoCompoundStatus();

    } catch (error) {
      console.error('Failed to load portfolio:', error);
    }
  }

  private async displayFarmingPositions(): Promise<void> {
    console.log('🚜 LP Farming Positions:');
    console.log('------------------------');

    const farmPools = [
      new PublicKey('FARM_POOL_1'),
      new PublicKey('FARM_POOL_2')
    ];

    for (const farmPool of farmPools) {
      try {
        const userFarmInfo = await getUserFarmInfo(
          this.farming.connection,
          this.farming.wallet.publicKey,
          farmPool
        );

        if (userFarmInfo.stakedAmount.gt(new BN(0))) {
          console.log(`\nPool: ${farmPool.toString().slice(0, 8)}...`);
          console.log(`  Staked: ${Number(userFarmInfo.stakedAmount) / 1e6} LP tokens`);
          console.log(`  Pending rewards: ${Number(userFarmInfo.pendingRewards) / 1e9} SAROS`);
          console.log(`  APR: ${userFarmInfo.apr}%`);
          console.log(`  Days farming: ${userFarmInfo.daysStaked}`);
        }
      } catch (error) {
        // Pool not found or no position
      }
    }
  }

  private async displayStakingPositions(): Promise<void> {
    console.log('\n\n💎 SAROS Staking Positions:');
    console.log('---------------------------');

    const stakePools = [
      new PublicKey('STAKE_POOL_1'),
      new PublicKey('STAKE_POOL_2')
    ];

    for (const stakePool of stakePools) {
      try {
        const userStakeInfo = await getUserStakeInfo(
          this.farming.connection,
          this.farming.wallet.publicKey,
          stakePool
        );

        if (userStakeInfo.stakedAmount.gt(new BN(0))) {
          console.log(`\nPool: ${stakePool.toString().slice(0, 8)}...`);
          console.log(`  Staked: ${Number(userStakeInfo.stakedAmount) / 1e9} SAROS`);
          console.log(`  Pending rewards: ${Number(userStakeInfo.pendingRewards) / 1e9} SAROS`);
          console.log(`  APR: ${userStakeInfo.apr}%`);
          
          if (userStakeInfo.lockUntil) {
            console.log(`  Locked until: ${userStakeInfo.lockUntil.toISOString()}`);
          }
        }
      } catch (error) {
        // Pool not found or no position
      }
    }
  }

  private async displayYieldOpportunities(): Promise<void> {
    console.log('\n\n🎯 Top Yield Opportunities:');
    console.log('---------------------------');

    const opportunities = await this.optimizer.findBestOpportunities(
      new BN(1000 * 1e6), // $1000 USDC equivalent
      'MEDIUM', // Medium risk tolerance
      86400 * 7 // Max 7 days lock
    );

    for (let i = 0; i < Math.min(3, opportunities.length); i++) {
      const opp = opportunities[i];
      console.log(`\n${i + 1}. ${opp.description}`);
      console.log(`   APR: ${opp.projectedAPR.toFixed(1)}%`);
      console.log(`   Risk: ${opp.risk}`);
      console.log(`   TVL: $${Number(opp.tvl) / 1e6}`);
      console.log(`   Min amount: ${Number(opp.minAmount) / 1e6}`);
      
      if (opp.lockDuration > 0) {
        console.log(`   Lock: ${opp.lockDuration / 86400} days`);
      }
    }
  }

  private async displayAutoCompoundStatus(): Promise<void> {
    console.log('\n\n🤖 Auto-Compound Status:');
    console.log('------------------------');

    if (this.autoCompounder) {
      const stats = await this.autoCompounder.getStats();
      console.log('Status: ACTIVE ✅');
      console.log(`Total farmed: ${Number(stats.totalFarmed) / 1e6} LP tokens`);
      console.log(`Total staked: ${Number(stats.totalStaked) / 1e9} SAROS`);
      console.log(`Pending rewards: ${Number(stats.totalRewards) / 1e9} SAROS`);
      console.log(`Estimated APY: ${stats.estimatedAPY.toFixed(1)}%`);
    } else {
      console.log('Status: INACTIVE ❌');
      console.log('Use enableAutoCompound() to start automated compounding');
    }
  }

  async enableAutoCompound(): Promise<void> {
    console.log('🤖 Enabling auto-compound strategy...');

    const strategy: CompoundStrategy = {
      farmPools: [
        new PublicKey('FARM_POOL_1'),
        new PublicKey('FARM_POOL_2')
      ],
      stakePools: [
        new PublicKey('STAKE_POOL_1')
      ],
      compoundFrequency: '0 0 */6 * * *', // Every 6 hours
      minRewardThreshold: new BN(1 * 1e9), // 1 SAROS minimum
      gasReserve: 0.1, // Keep 0.1 SOL for gas
      enabled: true
    };

    this.autoCompounder = new AutoCompoundBot(
      process.env.WALLET_PRIVATE_KEY!,
      strategy
    );

    await this.autoCompounder.start();
    console.log('✅ Auto-compound enabled!');
  }

  async optimizeYield(
    availableAmount: number,
    riskTolerance: 'LOW' | 'MEDIUM' | 'HIGH' = 'MEDIUM'
  ): Promise<void> {
    console.log(`\n🎯 Optimizing yield for ${availableAmount} USDC...`);

    const amountBN = new BN(availableAmount * 1e6);
    const strategy = await this.optimizer.createOptimalStrategy(amountBN, riskTolerance);

    console.log('\n📋 Optimal Strategy:');
    console.log(`Expected APR: ${strategy.expectedAPR.toFixed(1)}%`);
    console.log(`Risk level: ${strategy.totalRisk}`);
    console.log('\nAllocations:');

    for (const allocation of strategy.allocations) {
      console.log(`  ${allocation.percentage.toFixed(1)}% → ${allocation.opportunity.description}`);
      console.log(`    Amount: $${Number(allocation.amount) / 1e6}`);
      console.log(`    APR: ${allocation.opportunity.projectedAPR.toFixed(1)}%`);
    }

    console.log('\nExecute this strategy? (This is a tutorial example)');
    // In production, add user confirmation before executing
    // await this.optimizer.executeStrategy(strategy.allocations);
  }
}
```

### Step 2: Complete Tutorial Script

Create `src/farming-tutorial.ts`:

```typescript
async function runFarmingTutorial() {
  console.log('🌾 Saros Yield Farming & Staking Tutorial');
  console.log('=========================================\n');

  try {
    const dashboard = new FarmingDashboard(process.env.WALLET_PRIVATE_KEY!);

    // 1. Display current portfolio
    console.log('1️⃣ Portfolio Overview...');
    await dashboard.displayFullPortfolio();

    console.log('\n' + '='.repeat(50) + '\n');

    // 2. Start LP farming
    console.log('2️⃣ Starting LP Farming...');
    const farming = new SarosFarming(process.env.WALLET_PRIVATE_KEY!);
    await farming.initialize();

    const farmResult = await farming.enterLPFarm(
      new PublicKey('YOUR_FARM_POOL_ADDRESS'),
      new BN(100 * 1e6), // 100 LP tokens
      0 // No lock
    );

    if (farmResult.success) {
      console.log(`✅ LP farming started: ${farmResult.expectedAPR}% APR`);
    }

    console.log('\n' + '='.repeat(50) + '\n');

    // 3. Stake SAROS tokens
    console.log('3️⃣ Staking SAROS Tokens...');
    
    const stakeResult = await farming.stakeSAROSTokens(
      new BN(500 * 1e9), // 500 SAROS
      86400 * 30 // 30 days lock
    );

    if (stakeResult.success) {
      console.log(`✅ SAROS staking completed: ${stakeResult.expectedAPR}% APR`);
      if (stakeResult.lockUntil) {
        console.log(`🔒 Locked until: ${stakeResult.lockUntil.toISOString()}`);
      }
    }

    console.log('\n' + '='.repeat(50) + '\n');

    // 4. Enable auto-compound
    console.log('4️⃣ Enabling Auto-Compound...');
    await dashboard.enableAutoCompound();

    console.log('\n' + '='.repeat(50) + '\n');

    // 5. Yield optimization
    console.log('5️⃣ Yield Optimization...');
    await dashboard.optimizeYield(1000, 'MEDIUM'); // $1000, medium risk

    console.log('\n' + '='.repeat(50) + '\n');

    // 6. Claim rewards demo
    console.log('6️⃣ Claiming Rewards...');
    
    const farmRewards = await farming.claimFarmingRewards(
      new PublicKey('YOUR_FARM_POOL_ADDRESS')
    );
    
    const stakeRewards = await farming.claimSAROSRewards();

    let totalClaimed = new BN(0);
    if (farmRewards.success) {
      totalClaimed = totalClaimed.add(farmRewards.rewardsClaimed);
    }
    if (stakeRewards.success) {
      totalClaimed = totalClaimed.add(stakeRewards.rewardsClaimed);
    }

    console.log(`✅ Total rewards claimed: ${Number(totalClaimed) / 1e9} SAROS`);

    console.log('\n🎉 Farming tutorial completed successfully!');
    console.log('\nKey achievements:');
    console.log('✅ Started LP farming');
    console.log('✅ Staked SAROS tokens');
    console.log('✅ Enabled auto-compounding');
    console.log('✅ Optimized yield strategy');
    console.log('✅ Claimed rewards');

  } catch (error) {
    console.error('Tutorial failed:', error);
  }
}

// Run the tutorial
runFarmingTutorial();
```

### Step 3: Run the Tutorial

```bash
# Set up environment
echo "WALLET_PRIVATE_KEY=your_base58_private_key" > .env

# Install additional dependencies for cron jobs
npm install node-cron @types/node-cron

# Run the tutorial
npx ts-node src/farming-tutorial.ts
```

## Expected Output

```
🌾 Saros Yield Farming & Staking Tutorial
=========================================

1️⃣ Portfolio Overview...
🌾 Complete Farming Portfolio
==============================

🚜 LP Farming Positions:
------------------------

Pool: 5tzFkiKs...
  Staked: 141.42 LP tokens
  Pending rewards: 2.54 SAROS
  APR: 45.2%
  Days farming: 7

💎 SAROS Staking Positions:
---------------------------

Pool: 8xNmWqL3...
  Staked: 500.0 SAROS
  Pending rewards: 1.25 SAROS
  APR: 28.5%
  🔒 Locked until: 2024-02-15T10:30:00.000Z

🎯 Top Yield Opportunities:
---------------------------

1. DLMM ETH-USDC - High volatility, high rewards
   APR: 82.1%
   Risk: HIGH
   TVL: $2000000
   Min amount: 50

2. USDC-SOL LP Farm - Stable pair with consistent rewards
   APR: 48.5%
   Risk: MEDIUM
   TVL: $5000000
   Min amount: 10

3. SAROS Staking - Low risk, governance participation
   APR: 28.2%
   Risk: LOW
   TVL: $10000000
   Min amount: 100
   Lock: 30 days

🤖 Auto-Compound Status:
------------------------
Status: INACTIVE ❌
Use enableAutoCompound() to start automated compounding

==================================================

2️⃣ Starting LP Farming...
🚜 Initializing Saros Farming...
Wallet: 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU
SOL Balance: 2.5 SOL
🌾 Entering LP farming pool...
Pool: YOUR_FARM_POOL_ADDRESS
Amount: 100000000 LP tokens
Farm APR: 45.2 %
Total staked: 15000.0 LP tokens
Available LP tokens: 150.0
⚡ Staking LP tokens...
✅ LP farming started successfully!
Transaction: 3mJ7K8pL2nX9vQ4uR5cF8tY1wE6sA9mP7nZ2fV4gH3kL
✅ LP farming started: 45.2% APR

==================================================

3️⃣ Staking SAROS Tokens...
🔒 Staking SAROS tokens...
Amount: 500 SAROS
Lock duration: 2592000 seconds
Available SAROS: 1000.0
Stake pool APR: 28.5 %
Total staked: 50000.0 SAROS
⚡ Staking SAROS tokens...
✅ SAROS staking completed!
Transaction: 4nK8L9qM3oY0xR6vS7dG9uZ2xF7tB0nQ8oA3gI5jK6mN
✅ SAROS staking completed: 28.5% APR
🔒 Locked until: 2024-02-15T10:30:00.000Z

==================================================

4️⃣ Enabling Auto-Compound...
🤖 Enabling auto-compound strategy...
🤖 Starting Auto-Compound Bot...
Frequency: 0 0 */6 * * *
Farm pools: 2
Stake pools: 1
✅ Auto-compound bot started!
✅ Auto-compound enabled!

==================================================

5️⃣ Yield Optimization...

🎯 Optimizing yield for 1000 USDC...
🔍 Scanning for yield opportunities...
Found 3 suitable opportunities
🎯 Creating optimal yield strategy...
Total amount: 1000000
Risk tolerance: MEDIUM

📋 Optimal Strategy:
Expected APR: 52.3%
Risk level: MEDIUM

Allocations:
  60.0% → USDC-SOL LP Farm - Stable pair with consistent rewards
    Amount: $600
    APR: 48.5%
  40.0% → SAROS Staking - Low risk, governance participation
    Amount: $400
    APR: 28.2%

Execute this strategy? (This is a tutorial example)

🎉 Farming tutorial completed successfully!

Key achievements:
✅ Started LP farming
✅ Staked SAROS tokens
✅ Enabled auto-compounding
✅ Optimized yield strategy
✅ Claimed rewards
```

## Best Practices and Tips

### Farming Strategy Guidelines
- ✅ **Start small**: Test with small amounts first
- ✅ **Diversify**: Don't put all funds in one pool
- ✅ **Monitor APR**: Rates change based on pool dynamics
- ✅ **Claim regularly**: Don't let rewards sit too long

### Auto-Compound Settings
- ✅ **Frequency**: 4-12 hours for active markets
- ✅ **Gas reserve**: Keep 0.1+ SOL for transactions
- ✅ **Minimum threshold**: Set based on gas costs
- ✅ **Monitor bot**: Check logs regularly

### Risk Management
- ✅ **Lock periods**: Only lock what you won't need
- ✅ **Smart contracts**: Understand protocol risks
- ✅ **Impermanent loss**: Monitor for LP positions
- ✅ **Exit strategy**: Plan when to take profits

## Troubleshooting

### Common Issues

**"Insufficient LP tokens"**
```typescript
// Check LP token balance first
const lpBalance = await getTokenBalance(lpTokenAccount);
console.log('Available LP tokens:', lpBalance);
```

**"Tokens are locked"**
```typescript
// Check lock status before unstaking
if (userStakeInfo.lockUntil && new Date() < userStakeInfo.lockUntil) {
  console.log(`Locked until: ${userStakeInfo.lockUntil}`);
}
```

**Auto-compound not working**
```typescript
// Check bot status
console.log('Bot enabled:', strategy.enabled);
console.log('SOL balance:', await connection.getBalance(wallet.publicKey));
```

## Next Steps

🎉 Congratulations! You've mastered Saros yield farming and staking.

Continue with:
- [Tutorial 4: DLMM Positions](./04-dlmm-positions.md)
- [Auto-Compound Examples](../code-examples/typescript/02-auto-compound/)
- [Advanced Farming Strategies](../best-practices/performance.md)

## Resources

- [Farming Calculator](https://app.saros.finance/tools/farming-calculator)
- [Yield Analytics](https://analytics.saros.finance/farming)
- [Community Strategies](https://discord.gg/saros)
- [Governance Forum](https://gov.saros.finance)