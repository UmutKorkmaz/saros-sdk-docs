# DLMM SDK: Position Management

Comprehensive guide to creating, managing, and optimizing concentrated liquidity positions using the Saros DLMM SDK.

## Overview

DLMM position management enables sophisticated liquidity provision strategies with:
- ✅ Precise price range positioning
- ✅ Multiple position tracking
- ✅ Automated rebalancing strategies
- ✅ Fee collection and compounding
- ✅ Impermanent loss mitigation

## Position Structure

### Understanding DLMM Positions

```typescript
import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';

interface DLMMPosition {
  // Position identification
  positionId: PublicKey;
  owner: PublicKey;
  pair: PublicKey;
  
  // Bin configuration
  bins: Array<{
    binId: number;
    pricePerToken: number;
    liquidityX: BN;  // Token X liquidity
    liquidityY: BN;  // Token Y liquidity
    feeX: BN;        // Accumulated fees
    feeY: BN;
  }>;
  
  // Position metrics
  totalLiquidityX: BN;
  totalLiquidityY: BN;
  totalFeesX: BN;
  totalFeesY: BN;
  
  // Range information
  minBinId: number;
  maxBinId: number;
  activeBins: number;
  
  // Performance metrics
  createdAt: Date;
  lastUpdated: Date;
  totalValue: BN;
  pnl: BN;
}
```

## Creating Positions

### Basic Position Creation

```typescript
import { LiquidityBookServices, MODE } from '@saros-finance/dlmm-sdk';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';

class DLMMPositionManager {
  private dlmmService: LiquidityBookServices;
  private connection: Connection;
  private wallet: Keypair;

  constructor(
    connection: Connection,
    wallet: Keypair,
    mode: MODE = MODE.MAINNET
  ) {
    this.connection = connection;
    this.wallet = wallet;
    this.dlmmService = new LiquidityBookServices({ mode });
  }

  async createBasicPosition(
    pairAddress: PublicKey,
    amountX: BN,
    amountY: BN,
    minPrice: number,
    maxPrice: number
  ): Promise<{
    positionId: string;
    signature: string;
    binsAllocated: number;
    success: boolean;
  }> {
    try {
      console.log('🎯 Creating DLMM position...');
      console.log(`Range: $${minPrice} - $${maxPrice}`);

      // Step 1: Get pair information
      const pairInfo = await this.dlmmService.getPairAccount(pairAddress);
      const binStep = pairInfo.binStep; // Basis points per bin
      const activeBinId = pairInfo.activeBinId;

      console.log(`Active bin: ${activeBinId}`);
      console.log(`Bin step: ${binStep} basis points`);

      // Step 2: Calculate bin range
      const minBinId = this.priceToBindId(minPrice, binStep);
      const maxBinId = this.priceToBindId(maxPrice, binStep);
      const numberOfBins = maxBinId - minBinId + 1;

      console.log(`Bin range: ${minBinId} to ${maxBinId} (${numberOfBins} bins)`);

      // Step 3: Create distribution
      const { binIds, distributionX, distributionY } = this.createUniformDistribution(
        minBinId,
        maxBinId
      );

      // Step 4: Add liquidity to position
      console.log('⚡ Adding liquidity to position...');
      
      const transaction = await this.dlmmService.addLiquidityIntoPosition({
        pair: pairAddress,
        binIds,
        distributionX,
        distributionY,
        amountX,
        amountY,
        user: this.wallet.publicKey,
        slippage: 1.0 // 1% slippage tolerance
      });

      // Step 5: Sign and send transaction
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

      console.log('✅ Position created successfully!');
      console.log(`Transaction: ${signature}`);

      return {
        positionId: signature, // Use signature as position ID
        signature,
        binsAllocated: numberOfBins,
        success: true
      };

    } catch (error: any) {
      console.error('❌ Position creation failed:', error.message);
      
      return {
        positionId: '',
        signature: '',
        binsAllocated: 0,
        success: false
      };
    }
  }

  private priceToBindId(price: number, binStep: number): number {
    // Convert price to bin ID based on bin step
    const baseFactor = 1.0;
    const binStepDecimal = binStep / 10000; // Convert basis points to decimal
    
    return Math.floor(Math.log(price / baseFactor) / Math.log(1 + binStepDecimal));
  }

  private createUniformDistribution(
    minBinId: number,
    maxBinId: number
  ): {
    binIds: number[];
    distributionX: number[];
    distributionY: number[];
  } {
    const binIds: number[] = [];
    const distributionX: number[] = [];
    const distributionY: number[] = [];
    
    const numberOfBins = maxBinId - minBinId + 1;
    const weightPerBin = Math.floor(100 / numberOfBins);

    for (let binId = minBinId; binId <= maxBinId; binId++) {
      binIds.push(binId);
      distributionX.push(weightPerBin);
      distributionY.push(weightPerBin);
    }

    return { binIds, distributionX, distributionY };
  }
}
```

### Advanced Position Strategies

```typescript
enum LiquidityShape {
  SPOT = 'SPOT',           // Single bin concentration
  CURVE = 'CURVE',         // Bell curve distribution
  BID_ASK = 'BID_ASK',    // Asymmetric distribution
  UNIFORM = 'UNIFORM',     // Even distribution
  CUSTOM = 'CUSTOM'        // User-defined shape
}

interface PositionStrategy {
  shape: LiquidityShape;
  concentration: number;    // 0-1, higher = more concentrated
  skew: number;            // -1 to 1, negative = bid heavy, positive = ask heavy
  rebalanceThreshold: number; // Price movement % to trigger rebalance
}

class AdvancedPositionManager extends DLMMPositionManager {
  async createStrategicPosition(
    pairAddress: PublicKey,
    amountX: BN,
    amountY: BN,
    strategy: PositionStrategy
  ): Promise<DLMMPosition> {
    console.log(`🎯 Creating ${strategy.shape} position...`);

    // Get current price and pair info
    const pairInfo = await this.dlmmService.getPairAccount(pairAddress);
    const currentBinId = pairInfo.activeBinId;
    const binStep = pairInfo.binStep;

    // Calculate distribution based on strategy
    const distribution = this.calculateStrategicDistribution(
      currentBinId,
      binStep,
      strategy
    );

    // Create the position
    const transaction = await this.dlmmService.addLiquidityIntoPosition({
      pair: pairAddress,
      binIds: distribution.binIds,
      distributionX: distribution.distributionX,
      distributionY: distribution.distributionY,
      amountX,
      amountY,
      user: this.wallet.publicKey,
      slippage: 1.0
    });

    // Execute transaction
    const signature = await this.sendTransaction(transaction);

    // Build position object
    const position: DLMMPosition = {
      positionId: new PublicKey(signature),
      owner: this.wallet.publicKey,
      pair: pairAddress,
      bins: distribution.binIds.map((binId, index) => ({
        binId,
        pricePerToken: this.binIdToPrice(binId, binStep),
        liquidityX: amountX.muln(distribution.distributionX[index]).divn(100),
        liquidityY: amountY.muln(distribution.distributionY[index]).divn(100),
        feeX: new BN(0),
        feeY: new BN(0)
      })),
      totalLiquidityX: amountX,
      totalLiquidityY: amountY,
      totalFeesX: new BN(0),
      totalFeesY: new BN(0),
      minBinId: Math.min(...distribution.binIds),
      maxBinId: Math.max(...distribution.binIds),
      activeBins: distribution.binIds.length,
      createdAt: new Date(),
      lastUpdated: new Date(),
      totalValue: this.calculatePositionValue(amountX, amountY),
      pnl: new BN(0)
    };

    console.log('✅ Strategic position created');
    console.log(`Shape: ${strategy.shape}`);
    console.log(`Bins: ${position.activeBins}`);
    console.log(`Range: Bin ${position.minBinId} to ${position.maxBinId}`);

    return position;
  }

  private calculateStrategicDistribution(
    currentBinId: number,
    binStep: number,
    strategy: PositionStrategy
  ): {
    binIds: number[];
    distributionX: number[];
    distributionY: number[];
  } {
    switch (strategy.shape) {
      case LiquidityShape.SPOT:
        return this.createSpotDistribution(currentBinId, strategy);
      
      case LiquidityShape.CURVE:
        return this.createCurveDistribution(currentBinId, strategy);
      
      case LiquidityShape.BID_ASK:
        return this.createBidAskDistribution(currentBinId, strategy);
      
      case LiquidityShape.UNIFORM:
        return this.createUniformDistributionAdvanced(currentBinId, strategy);
      
      default:
        throw new Error(`Unknown liquidity shape: ${strategy.shape}`);
    }
  }

  private createSpotDistribution(
    currentBinId: number,
    strategy: PositionStrategy
  ): any {
    // Single bin concentration
    return {
      binIds: [currentBinId],
      distributionX: [100],
      distributionY: [100]
    };
  }

  private createCurveDistribution(
    currentBinId: number,
    strategy: PositionStrategy
  ): any {
    // Bell curve distribution
    const range = Math.floor(20 * (1 - strategy.concentration));
    const binIds: number[] = [];
    const distributionX: number[] = [];
    const distributionY: number[] = [];

    for (let i = -range; i <= range; i++) {
      const binId = currentBinId + i;
      binIds.push(binId);

      // Gaussian distribution
      const weight = Math.exp(-(i * i) / (2 * range * range / 4)) * 100;
      distributionX.push(Math.floor(weight));
      distributionY.push(Math.floor(weight));
    }

    // Normalize to 100%
    const sumX = distributionX.reduce((a, b) => a + b, 0);
    const sumY = distributionY.reduce((a, b) => a + b, 0);
    
    return {
      binIds,
      distributionX: distributionX.map(w => Math.floor(w * 100 / sumX)),
      distributionY: distributionY.map(w => Math.floor(w * 100 / sumY))
    };
  }

  private createBidAskDistribution(
    currentBinId: number,
    strategy: PositionStrategy
  ): any {
    // Asymmetric distribution for market making
    const range = 10;
    const binIds: number[] = [];
    const distributionX: number[] = [];
    const distributionY: number[] = [];

    for (let i = -range; i <= range; i++) {
      const binId = currentBinId + i;
      binIds.push(binId);

      if (i < 0) {
        // Bid side (below current price) - provide Y token
        distributionX.push(Math.floor(5 * (1 + strategy.skew)));
        distributionY.push(Math.floor(20 * (1 - strategy.skew)));
      } else if (i > 0) {
        // Ask side (above current price) - provide X token
        distributionX.push(Math.floor(20 * (1 + strategy.skew)));
        distributionY.push(Math.floor(5 * (1 - strategy.skew)));
      } else {
        // Current price bin
        distributionX.push(10);
        distributionY.push(10);
      }
    }

    return { binIds, distributionX, distributionY };
  }

  private createUniformDistributionAdvanced(
    currentBinId: number,
    strategy: PositionStrategy
  ): any {
    const range = Math.floor(15 * (1 - strategy.concentration));
    const binIds: number[] = [];
    const distributionX: number[] = [];
    const distributionY: number[] = [];
    
    const weight = Math.floor(100 / (2 * range + 1));

    for (let i = -range; i <= range; i++) {
      binIds.push(currentBinId + i);
      distributionX.push(weight);
      distributionY.push(weight);
    }

    return { binIds, distributionX, distributionY };
  }

  private binIdToPrice(binId: number, binStep: number): number {
    const binStepDecimal = binStep / 10000;
    return Math.pow(1 + binStepDecimal, binId);
  }

  private calculatePositionValue(amountX: BN, amountY: BN): BN {
    // Simplified value calculation
    // In production, use actual token prices
    const priceX = 50; // $50 per token X
    const priceY = 1;  // $1 per token Y
    
    return amountX.muln(priceX).add(amountY.muln(priceY));
  }

  private async sendTransaction(transaction: any): Promise<string> {
    transaction.feePayer = this.wallet.publicKey;
    transaction.recentBlockhash = (
      await this.connection.getLatestBlockhash()
    ).blockhash;

    transaction.sign(this.wallet);

    const signature = await this.connection.sendRawTransaction(
      transaction.serialize()
    );

    await this.connection.confirmTransaction(signature, 'confirmed');

    return signature;
  }
}
```

## Position Monitoring

### Real-Time Position Tracking

```typescript
interface PositionMetrics {
  position: DLMMPosition;
  currentValue: BN;
  unrealizedPnL: BN;
  realizedPnL: BN;
  fees24h: { tokenX: BN; tokenY: BN };
  apr: number;
  healthScore: number; // 0-100
  inRange: boolean;
}

class PositionMonitor {
  private dlmmService: LiquidityBookServices;
  private positions: Map<string, DLMMPosition> = new Map();
  private monitoring: Map<string, NodeJS.Timer> = new Map();

  constructor() {
    this.dlmmService = new LiquidityBookServices({ mode: MODE.MAINNET });
  }

  async trackPosition(
    positionId: string,
    pairAddress: PublicKey,
    updateInterval: number = 30000 // 30 seconds
  ): Promise<void> {
    console.log(`📊 Starting position tracking: ${positionId}`);

    const timer = setInterval(async () => {
      try {
        const metrics = await this.getPositionMetrics(positionId, pairAddress);
        
        console.log(`\n📈 Position Update [${new Date().toISOString()}]`);
        console.log(`Value: $${Number(metrics.currentValue) / 1e6}`);
        console.log(`Unrealized PnL: $${Number(metrics.unrealizedPnL) / 1e6}`);
        console.log(`24h Fees: X=${Number(metrics.fees24h.tokenX) / 1e9}, Y=${Number(metrics.fees24h.tokenY) / 1e6}`);
        console.log(`APR: ${metrics.apr.toFixed(2)}%`);
        console.log(`Health: ${metrics.healthScore}/100`);
        console.log(`In Range: ${metrics.inRange ? '✅' : '❌'}`);

        // Check for alerts
        this.checkAlerts(metrics);

      } catch (error) {
        console.error(`Error tracking position ${positionId}:`, error);
      }
    }, updateInterval);

    this.monitoring.set(positionId, timer);
  }

  async getPositionMetrics(
    positionId: string,
    pairAddress: PublicKey
  ): Promise<PositionMetrics> {
    // Get position data
    const position = await this.fetchPosition(positionId, pairAddress);
    
    // Get current pair state
    const pairInfo = await this.dlmmService.getPairAccount(pairAddress);
    const currentBinId = pairInfo.activeBinId;

    // Calculate metrics
    const currentValue = await this.calculateCurrentValue(position, pairInfo);
    const unrealizedPnL = currentValue.sub(position.totalValue);
    const fees24h = await this.calculate24hFees(position);
    const apr = this.calculateAPR(position, fees24h);
    const healthScore = this.calculateHealthScore(position, currentBinId);
    const inRange = this.isPositionInRange(position, currentBinId);

    return {
      position,
      currentValue,
      unrealizedPnL,
      realizedPnL: position.totalFeesX.add(position.totalFeesY),
      fees24h,
      apr,
      healthScore,
      inRange
    };
  }

  private async fetchPosition(
    positionId: string,
    pairAddress: PublicKey
  ): Promise<DLMMPosition> {
    // In production, fetch from chain
    // This is a mock implementation
    let position = this.positions.get(positionId);
    
    if (!position) {
      position = {
        positionId: new PublicKey(positionId),
        owner: new PublicKey('OWNER_ADDRESS'),
        pair: pairAddress,
        bins: [],
        totalLiquidityX: new BN(1000000000),
        totalLiquidityY: new BN(50000000),
        totalFeesX: new BN(0),
        totalFeesY: new BN(0),
        minBinId: 95,
        maxBinId: 105,
        activeBins: 11,
        createdAt: new Date(),
        lastUpdated: new Date(),
        totalValue: new BN(1500000000),
        pnl: new BN(0)
      };
      
      this.positions.set(positionId, position);
    }

    return position;
  }

  private async calculateCurrentValue(
    position: DLMMPosition,
    pairInfo: any
  ): Promise<BN> {
    // Calculate current position value based on liquidity and prices
    const priceX = 50; // Mock price
    const priceY = 1;
    
    return position.totalLiquidityX.muln(priceX).add(
      position.totalLiquidityY.muln(priceY)
    );
  }

  private async calculate24hFees(position: DLMMPosition): Promise<{
    tokenX: BN;
    tokenY: BN;
  }> {
    // In production, calculate from historical data
    return {
      tokenX: new BN(100000),  // 0.0001 token X
      tokenY: new BN(5000000)  // 5 token Y
    };
  }

  private calculateAPR(
    position: DLMMPosition,
    fees24h: { tokenX: BN; tokenY: BN }
  ): number {
    // Simplified APR calculation
    const dailyFeeValue = Number(fees24h.tokenX) * 50 + Number(fees24h.tokenY);
    const positionValue = Number(position.totalValue);
    
    return (dailyFeeValue * 365 / positionValue) * 100;
  }

  private calculateHealthScore(
    position: DLMMPosition,
    currentBinId: number
  ): number {
    // Score based on various factors
    let score = 100;

    // Reduce score if position is out of range
    if (currentBinId < position.minBinId || currentBinId > position.maxBinId) {
      score -= 30;
    }

    // Reduce score if position is too concentrated
    if (position.activeBins < 5) {
      score -= 20;
    }

    // Reduce score if position hasn't been rebalanced recently
    const daysSinceUpdate = (Date.now() - position.lastUpdated.getTime()) / (1000 * 60 * 60 * 24);
    if (daysSinceUpdate > 7) {
      score -= Math.min(20, daysSinceUpdate * 2);
    }

    return Math.max(0, score);
  }

  private isPositionInRange(
    position: DLMMPosition,
    currentBinId: number
  ): boolean {
    return currentBinId >= position.minBinId && currentBinId <= position.maxBinId;
  }

  private checkAlerts(metrics: PositionMetrics): void {
    // Check for important conditions
    if (!metrics.inRange) {
      console.log('⚠️ ALERT: Position is out of range!');
    }

    if (metrics.healthScore < 50) {
      console.log('⚠️ ALERT: Low position health score!');
    }

    if (metrics.unrealizedPnL.isNeg() && Math.abs(Number(metrics.unrealizedPnL)) > Number(metrics.position.totalValue) * 0.1) {
      console.log('⚠️ ALERT: Significant unrealized loss (>10%)!');
    }
  }

  stopTracking(positionId: string): void {
    const timer = this.monitoring.get(positionId);
    if (timer) {
      clearInterval(timer);
      this.monitoring.delete(positionId);
      console.log(`🛑 Stopped tracking position ${positionId}`);
    }
  }
}
```

## Position Updates

### Adding and Removing Liquidity

```typescript
class PositionUpdater {
  private dlmmService: LiquidityBookServices;
  private connection: Connection;
  private wallet: Keypair;

  constructor(connection: Connection, wallet: Keypair) {
    this.connection = connection;
    this.wallet = wallet;
    this.dlmmService = new LiquidityBookServices({ mode: MODE.MAINNET });
  }

  async addLiquidityToPosition(
    position: DLMMPosition,
    additionalX: BN,
    additionalY: BN
  ): Promise<{
    signature: string;
    newTotalX: BN;
    newTotalY: BN;
    success: boolean;
  }> {
    try {
      console.log('➕ Adding liquidity to existing position...');
      console.log(`Adding X: ${Number(additionalX) / 1e9}`);
      console.log(`Adding Y: ${Number(additionalY) / 1e6}`);

      // Use existing bin distribution
      const binIds = position.bins.map(b => b.binId);
      const totalLiquidityX = position.bins.reduce((sum, b) => sum.add(b.liquidityX), new BN(0));
      const totalLiquidityY = position.bins.reduce((sum, b) => sum.add(b.liquidityY), new BN(0));

      // Calculate proportional distribution
      const distributionX = position.bins.map(b => 
        Number(b.liquidityX.mul(new BN(100)).div(totalLiquidityX))
      );
      const distributionY = position.bins.map(b => 
        Number(b.liquidityY.mul(new BN(100)).div(totalLiquidityY))
      );

      // Add liquidity
      const transaction = await this.dlmmService.addLiquidityIntoPosition({
        pair: position.pair,
        binIds,
        distributionX,
        distributionY,
        amountX: additionalX,
        amountY: additionalY,
        user: this.wallet.publicKey,
        slippage: 1.0
      });

      const signature = await this.sendTransaction(transaction);

      console.log('✅ Liquidity added successfully');
      console.log(`Transaction: ${signature}`);

      return {
        signature,
        newTotalX: position.totalLiquidityX.add(additionalX),
        newTotalY: position.totalLiquidityY.add(additionalY),
        success: true
      };

    } catch (error: any) {
      console.error('❌ Failed to add liquidity:', error.message);
      
      return {
        signature: '',
        newTotalX: position.totalLiquidityX,
        newTotalY: position.totalLiquidityY,
        success: false
      };
    }
  }

  async removeLiquidityFromPosition(
    position: DLMMPosition,
    percentageToRemove: number // 0-100
  ): Promise<{
    signature: string;
    removedX: BN;
    removedY: BN;
    success: boolean;
  }> {
    try {
      console.log(`➖ Removing ${percentageToRemove}% liquidity from position...`);

      // Calculate amounts to remove per bin
      const liquidityBps = Math.floor(percentageToRemove * 100); // Convert to basis points

      const transaction = await this.dlmmService.removeMultipleLiquidity({
        pair: position.pair,
        user: this.wallet.publicKey,
        binIds: position.bins.map(b => b.binId),
        liquidityBps: Array(position.bins.length).fill(liquidityBps)
      });

      const signature = await this.sendTransaction(transaction);

      const removedX = position.totalLiquidityX.muln(percentageToRemove).divn(100);
      const removedY = position.totalLiquidityY.muln(percentageToRemove).divn(100);

      console.log('✅ Liquidity removed successfully');
      console.log(`Removed X: ${Number(removedX) / 1e9}`);
      console.log(`Removed Y: ${Number(removedY) / 1e6}`);

      return {
        signature,
        removedX,
        removedY,
        success: true
      };

    } catch (error: any) {
      console.error('❌ Failed to remove liquidity:', error.message);
      
      return {
        signature: '',
        removedX: new BN(0),
        removedY: new BN(0),
        success: false
      };
    }
  }

  async claimFees(position: DLMMPosition): Promise<{
    signature: string;
    claimedX: BN;
    claimedY: BN;
    success: boolean;
  }> {
    try {
      console.log('💰 Claiming accumulated fees...');

      const transaction = await this.dlmmService.claimReward({
        pair: position.pair,
        user: this.wallet.publicKey,
        binIds: position.bins.map(b => b.binId)
      });

      const signature = await this.sendTransaction(transaction);

      const claimedX = position.totalFeesX;
      const claimedY = position.totalFeesY;

      console.log('✅ Fees claimed successfully');
      console.log(`Claimed X: ${Number(claimedX) / 1e9}`);
      console.log(`Claimed Y: ${Number(claimedY) / 1e6}`);

      return {
        signature,
        claimedX,
        claimedY,
        success: true
      };

    } catch (error: any) {
      console.error('❌ Failed to claim fees:', error.message);
      
      return {
        signature: '',
        claimedX: new BN(0),
        claimedY: new BN(0),
        success: false
      };
    }
  }

  private async sendTransaction(transaction: any): Promise<string> {
    transaction.feePayer = this.wallet.publicKey;
    transaction.recentBlockhash = (
      await this.connection.getLatestBlockhash()
    ).blockhash;

    transaction.sign(this.wallet);

    const signature = await this.connection.sendRawTransaction(
      transaction.serialize()
    );

    await this.connection.confirmTransaction(signature, 'confirmed');

    return signature;
  }
}
```

## Rebalancing Strategies

### Automated Rebalancing

```typescript
interface RebalanceStrategy {
  type: 'RANGE' | 'RATIO' | 'DYNAMIC';
  triggerThreshold: number; // % movement to trigger
  targetRange: { min: number; max: number };
  targetRatio?: { x: number; y: number };
  maxGasCost: number; // Maximum acceptable gas in SOL
}

class PositionRebalancer {
  private positionManager: AdvancedPositionManager;
  private positionUpdater: PositionUpdater;
  private monitor: PositionMonitor;

  constructor(
    connection: Connection,
    wallet: Keypair
  ) {
    this.positionManager = new AdvancedPositionManager(connection, wallet);
    this.positionUpdater = new PositionUpdater(connection, wallet);
    this.monitor = new PositionMonitor();
  }

  async autoRebalance(
    position: DLMMPosition,
    strategy: RebalanceStrategy
  ): Promise<{
    rebalanced: boolean;
    newPosition?: DLMMPosition;
    reason?: string;
  }> {
    console.log('🔄 Checking rebalance conditions...');

    // Get current metrics
    const metrics = await this.monitor.getPositionMetrics(
      position.positionId.toString(),
      position.pair
    );

    // Check if rebalancing is needed
    const shouldRebalance = this.shouldRebalance(metrics, strategy);

    if (!shouldRebalance.needed) {
      console.log('✅ Position is balanced');
      return {
        rebalanced: false,
        reason: shouldRebalance.reason
      };
    }

    console.log(`⚠️ Rebalancing needed: ${shouldRebalance.reason}`);

    // Estimate gas cost
    const estimatedGas = await this.estimateRebalanceGas();
    if (estimatedGas > strategy.maxGasCost) {
      console.log(`❌ Gas cost too high: ${estimatedGas} SOL`);
      return {
        rebalanced: false,
        reason: 'Gas cost exceeds maximum'
      };
    }

    // Execute rebalancing
    return await this.executeRebalance(position, strategy, metrics);
  }

  private shouldRebalance(
    metrics: PositionMetrics,
    strategy: RebalanceStrategy
  ): { needed: boolean; reason: string } {
    // Check if position is out of range
    if (!metrics.inRange) {
      return {
        needed: true,
        reason: 'Position is out of active price range'
      };
    }

    // Check health score
    if (metrics.healthScore < 60) {
      return {
        needed: true,
        reason: `Low health score: ${metrics.healthScore}`
      };
    }

    // Strategy-specific checks
    switch (strategy.type) {
      case 'RANGE':
        // Rebalance if price moved significantly
        const priceMovement = this.calculatePriceMovement(metrics.position);
        if (Math.abs(priceMovement) > strategy.triggerThreshold) {
          return {
            needed: true,
            reason: `Price moved ${priceMovement.toFixed(2)}% from center`
          };
        }
        break;

      case 'RATIO':
        // Rebalance if token ratio is off
        const currentRatio = Number(metrics.position.totalLiquidityX) / Number(metrics.position.totalLiquidityY);
        const targetRatio = strategy.targetRatio!.x / strategy.targetRatio!.y;
        const ratioDiff = Math.abs(currentRatio - targetRatio) / targetRatio;
        
        if (ratioDiff > strategy.triggerThreshold / 100) {
          return {
            needed: true,
            reason: `Token ratio off by ${(ratioDiff * 100).toFixed(2)}%`
          };
        }
        break;

      case 'DYNAMIC':
        // Dynamic rebalancing based on market conditions
        if (metrics.apr < 20) {
          return {
            needed: true,
            reason: `Low APR: ${metrics.apr.toFixed(2)}%`
          };
        }
        break;
    }

    return {
      needed: false,
      reason: 'Position is within acceptable parameters'
    };
  }

  private async executeRebalance(
    position: DLMMPosition,
    strategy: RebalanceStrategy,
    metrics: PositionMetrics
  ): Promise<{
    rebalanced: boolean;
    newPosition?: DLMMPosition;
    reason?: string;
  }> {
    try {
      console.log('⚡ Executing rebalance...');

      // Step 1: Remove all liquidity
      const removeResult = await this.positionUpdater.removeLiquidityFromPosition(
        position,
        100 // Remove 100%
      );

      if (!removeResult.success) {
        throw new Error('Failed to remove liquidity');
      }

      // Step 2: Calculate new position parameters
      const newStrategy: PositionStrategy = {
        shape: LiquidityShape.CURVE,
        concentration: 0.7,
        skew: 0,
        rebalanceThreshold: strategy.triggerThreshold
      };

      // Step 3: Create new position
      const newPosition = await this.positionManager.createStrategicPosition(
        position.pair,
        removeResult.removedX,
        removeResult.removedY,
        newStrategy
      );

      console.log('✅ Rebalancing completed successfully');

      return {
        rebalanced: true,
        newPosition,
        reason: 'Successfully rebalanced position'
      };

    } catch (error: any) {
      console.error('❌ Rebalancing failed:', error.message);
      
      return {
        rebalanced: false,
        reason: `Rebalancing failed: ${error.message}`
      };
    }
  }

  private calculatePriceMovement(position: DLMMPosition): number {
    // Calculate how far current price moved from position center
    const centerBin = (position.minBinId + position.maxBinId) / 2;
    // This would need actual current bin from pair info
    const currentBin = 100; // Mock value
    
    return ((currentBin - centerBin) / centerBin) * 100;
  }

  private async estimateRebalanceGas(): Promise<number> {
    // Estimate gas for remove + add operations
    // In production, use actual gas estimation
    return 0.02; // 0.02 SOL estimated
  }
}
```

## Usage Examples

### Complete Position Management Flow

```typescript
async function demonstratePositionManagement() {
  console.log('🚀 DLMM Position Management Demo');
  console.log('=================================\n');

  const connection = new Connection('https://api.mainnet-beta.solana.com');
  const wallet = Keypair.fromSecretKey(bs58.decode(process.env.WALLET_PRIVATE_KEY!));

  const positionManager = new AdvancedPositionManager(connection, wallet);
  const positionUpdater = new PositionUpdater(connection, wallet);
  const positionMonitor = new PositionMonitor();
  const rebalancer = new PositionRebalancer(connection, wallet);

  const pairAddress = new PublicKey('YOUR_DLMM_PAIR_ADDRESS');

  try {
    // 1. Create strategic position
    console.log('1️⃣ Creating strategic position...');
    const strategy: PositionStrategy = {
      shape: LiquidityShape.CURVE,
      concentration: 0.8, // 80% concentrated
      skew: 0, // Balanced
      rebalanceThreshold: 5 // 5% movement triggers rebalance
    };

    const position = await positionManager.createStrategicPosition(
      pairAddress,
      new BN(1000000000), // 1 token X
      new BN(50000000),   // 50 token Y
      strategy
    );

    console.log(`Position created with ${position.activeBins} bins`);

    // 2. Start monitoring
    console.log('\n2️⃣ Starting position monitoring...');
    await positionMonitor.trackPosition(
      position.positionId.toString(),
      pairAddress,
      30000 // Update every 30 seconds
    );

    // 3. Add more liquidity
    console.log('\n3️⃣ Adding additional liquidity...');
    const addResult = await positionUpdater.addLiquidityToPosition(
      position,
      new BN(500000000), // 0.5 token X
      new BN(25000000)   // 25 token Y
    );

    if (addResult.success) {
      console.log(`New total X: ${Number(addResult.newTotalX) / 1e9}`);
      console.log(`New total Y: ${Number(addResult.newTotalY) / 1e6}`);
    }

    // 4. Check rebalancing
    console.log('\n4️⃣ Checking rebalance conditions...');
    const rebalanceStrategy: RebalanceStrategy = {
      type: 'RANGE',
      triggerThreshold: 5, // 5% price movement
      targetRange: { min: 95, max: 105 },
      maxGasCost: 0.05 // Max 0.05 SOL for gas
    };

    const rebalanceResult = await rebalancer.autoRebalance(
      position,
      rebalanceStrategy
    );

    if (rebalanceResult.rebalanced) {
      console.log('✅ Position rebalanced');
    } else {
      console.log(`ℹ️ No rebalance needed: ${rebalanceResult.reason}`);
    }

    // 5. Claim fees
    console.log('\n5️⃣ Claiming accumulated fees...');
    const feeResult = await positionUpdater.claimFees(position);
    
    if (feeResult.success) {
      console.log(`Claimed X: ${Number(feeResult.claimedX) / 1e9}`);
      console.log(`Claimed Y: ${Number(feeResult.claimedY) / 1e6}`);
    }

    // 6. Remove partial liquidity
    console.log('\n6️⃣ Removing 25% of liquidity...');
    const removeResult = await positionUpdater.removeLiquidityFromPosition(
      position,
      25 // Remove 25%
    );

    if (removeResult.success) {
      console.log(`Removed X: ${Number(removeResult.removedX) / 1e9}`);
      console.log(`Removed Y: ${Number(removeResult.removedY) / 1e6}`);
    }

  } catch (error) {
    console.error('Demo failed:', error);
  } finally {
    // Clean up monitoring
    positionMonitor.stopTracking(position.positionId.toString());
  }
}

// Run the demo
demonstratePositionManagement();
```

## Best Practices

### Position Creation
- ✅ Analyze current liquidity distribution before creating
- ✅ Choose appropriate bin ranges based on volatility
- ✅ Consider gas costs for wide distributions
- ✅ Use concentrated positions for stable pairs

### Position Monitoring
- ✅ Track positions in real-time during volatile periods
- ✅ Set up alerts for out-of-range conditions
- ✅ Monitor fee accumulation regularly
- ✅ Calculate impermanent loss continuously

### Rebalancing
- ✅ Set clear rebalancing triggers
- ✅ Consider gas costs in rebalancing decisions
- ✅ Implement gradual rebalancing for large positions
- ✅ Track rebalancing history for optimization

## Troubleshooting

### Common Issues

**"Insufficient liquidity for distribution"**
```typescript
// Ensure enough tokens for minimum bin requirements
const minLiquidityPerBin = new BN(1000); // Minimum per bin
const requiredLiquidity = minLiquidityPerBin.muln(numberOfBins);
```

**"Position out of range"**
```typescript
// Check current price before creating position
const pairInfo = await dlmmService.getPairAccount(pairAddress);
console.log('Current active bin:', pairInfo.activeBinId);
```

**"Failed to claim fees"**
```typescript
// Ensure fees have accumulated
const position = await getPosition(positionId);
if (position.totalFeesX.eq(new BN(0)) && position.totalFeesY.eq(new BN(0))) {
  console.log('No fees to claim yet');
}
```

## Next Steps

- [Liquidity Shapes](./liquidity-shapes.md) - Advanced distribution patterns
- [Advanced Trading](./advanced-trading.md) - Complex execution strategies
- [Position Examples](../../code-examples/typescript/04-dlmm-range-orders/)
- [API Reference](../../api-reference/dlmm-sdk/)

## Resources

- [DLMM Position Calculator](https://app.saros.finance/tools/position-calculator)
- [Liquidity Analytics](https://analytics.saros.finance/positions)
- [Community Strategies](https://discord.gg/saros)
- [GitHub Examples](https://github.com/saros-finance/dlmm-examples)