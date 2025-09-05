/**
 * AutoCompounder - Core auto-compound implementation
 */

import { Connection, PublicKey, Keypair, Transaction } from '@solana/web3.js';
import { 
  claimRewardsSaros,
  addLiquiditySaros,
  stakeSaros,
  getPoolInfo,
  getUserPosition
} from './saros-sdk-mock';
import * as cron from 'node-cron';
import BN from 'bn.js';
import { logger } from './utils/logger';
import { GasOptimizer } from './GasOptimizer';
import { NotificationService } from './utils/notifications';

export interface AutoCompoundConfig {
  rpcUrl: string;
  privateKey: string;
  network?: 'devnet' | 'mainnet-beta';
  maxGasPrice?: number;
  emergencyStop?: () => Promise<boolean>;
}

export interface CompoundStrategy {
  poolAddress: PublicKey;
  strategy: 'LP' | 'STAKING' | 'FARMING';
  interval: number;
  minRewardThreshold: number;
  reinvestPercentage: number;
  maxSlippage?: number;
  emergencyWithdraw?: boolean;
}

export interface CompoundResult {
  success: boolean;
  rewardsHarvested: number;
  amountReinvested: number;
  newPositionValue: number;
  gasUsed: number;
  signature: string;
  timestamp: Date;
  error?: string;
}

export interface StartResult {
  success: boolean;
  poolAddress: string;
  strategy: string;
  interval: number;
  minThreshold: number;
  nextCompoundTime: string;
  error?: string;
}

export interface Statistics {
  totalCompounds: number;
  successfulCompounds: number;
  failedCompounds: number;
  successRate: number;
  totalRewardsHarvested: number;
  totalReinvested: number;
  totalGasSpent: number;
  netProfit: number;
  averageAPYBoost: number;
  lastCompoundTime?: Date;
}

interface ActivePool {
  poolAddress: PublicKey;
  strategy: CompoundStrategy;
  cronJob: cron.ScheduledTask;
  statistics: PoolStatistics;
}

interface PoolStatistics {
  compounds: number;
  totalHarvested: number;
  totalReinvested: number;
  totalGas: number;
  lastCompound?: Date;
  averageReward: number;
}

export class AutoCompounder {
  private connection: Connection;
  private wallet: Keypair;
  private activePools: Map<string, ActivePool> = new Map();
  private gasOptimizer: GasOptimizer;
  private notificationService: NotificationService;
  private globalStats: Statistics;
  private emergencyStopCheck?: () => Promise<boolean>;
  private maxGasPrice: number;

  constructor(config: AutoCompoundConfig) {
    this.connection = new Connection(config.rpcUrl, 'confirmed');
    // Handle different private key formats
    let secretKey: Uint8Array;
    if (config.privateKey.startsWith('[') && config.privateKey.endsWith(']')) {
      // JSON array format
      const keyArray = JSON.parse(config.privateKey);
      secretKey = Uint8Array.from(keyArray);
    } else {
      // Assume base64 format
      secretKey = new Uint8Array(Buffer.from(config.privateKey, 'base64'));
    }
    this.wallet = Keypair.fromSecretKey(secretKey);
    this.gasOptimizer = new GasOptimizer(this.connection);
    this.notificationService = new NotificationService();
    this.maxGasPrice = config.maxGasPrice || 0.01;
    this.emergencyStopCheck = config.emergencyStop;
    
    this.globalStats = {
      totalCompounds: 0,
      successfulCompounds: 0,
      failedCompounds: 0,
      successRate: 0,
      totalRewardsHarvested: 0,
      totalReinvested: 0,
      totalGasSpent: 0,
      netProfit: 0,
      averageAPYBoost: 0
    };

    logger.info('AutoCompounder initialized');
    logger.info(`Wallet: ${this.wallet.publicKey.toString()}`);
  }

  /**
   * Start auto-compounding for a pool
   */
  async start(strategy: CompoundStrategy): Promise<StartResult> {
    const poolKey = strategy.poolAddress.toString();
    
    try {
      // Check if already active
      if (this.activePools.has(poolKey)) {
        logger.warn(`Pool ${poolKey} already active`);
        return {
          success: false,
          poolAddress: poolKey,
          strategy: strategy.strategy,
          interval: strategy.interval,
          minThreshold: strategy.minRewardThreshold,
          nextCompoundTime: '',
          error: 'Pool already active'
        };
      }

      // Validate pool exists
      const poolInfo = await this.getPoolInfo(strategy.poolAddress);
      if (!poolInfo) {
        throw new Error('Pool not found');
      }

      // Create cron schedule
      const cronExpression = this.intervalToCron(strategy.interval);
      const cronJob = cron.schedule(cronExpression, async () => {
        await this.executeCompound(strategy);
      });

      // Store active pool
      const activePool: ActivePool = {
        poolAddress: strategy.poolAddress,
        strategy,
        cronJob,
        statistics: {
          compounds: 0,
          totalHarvested: 0,
          totalReinvested: 0,
          totalGas: 0,
          averageReward: 0
        }
      };

      this.activePools.set(poolKey, activePool);
      cronJob.start();

      // Calculate next compound time
      const nextCompound = new Date(Date.now() + strategy.interval);

      logger.info(`✅ Auto-compound started for pool ${poolKey}`);
      logger.info(`  Strategy: ${strategy.strategy}`);
      logger.info(`  Interval: ${strategy.interval}ms`);
      logger.info(`  Next compound: ${nextCompound.toLocaleString()}`);

      // Send notification
      await this.notificationService.send({
        type: 'COMPOUND_STARTED',
        pool: poolKey,
        strategy: strategy.strategy,
        interval: strategy.interval
      });

      return {
        success: true,
        poolAddress: poolKey,
        strategy: strategy.strategy,
        interval: strategy.interval,
        minThreshold: strategy.minRewardThreshold,
        nextCompoundTime: nextCompound.toLocaleString()
      };

    } catch (error: any) {
      logger.error(`Failed to start auto-compound: ${error.message}`);
      
      return {
        success: false,
        poolAddress: poolKey,
        strategy: strategy.strategy,
        interval: strategy.interval,
        minThreshold: strategy.minRewardThreshold,
        nextCompoundTime: '',
        error: error.message
      };
    }
  }

  /**
   * Stop auto-compounding for a pool
   */
  async stop(poolAddress: PublicKey): Promise<boolean> {
    const poolKey = poolAddress.toString();
    const activePool = this.activePools.get(poolKey);
    
    if (!activePool) {
      logger.warn(`Pool ${poolKey} not active`);
      return false;
    }

    // Stop cron job
    activePool.cronJob.stop();
    
    // Remove from active pools
    this.activePools.delete(poolKey);

    logger.info(`Auto-compound stopped for pool ${poolKey}`);
    
    // Send notification
    await this.notificationService.send({
      type: 'COMPOUND_STOPPED',
      pool: poolKey,
      statistics: activePool.statistics
    });

    return true;
  }

  /**
   * Manually trigger compound for a pool
   */
  async compoundNow(poolAddress: PublicKey): Promise<CompoundResult> {
    const poolKey = poolAddress.toString();
    const activePool = this.activePools.get(poolKey);
    
    if (!activePool) {
      // Create temporary strategy for manual compound
      const strategy: CompoundStrategy = {
        poolAddress,
        strategy: 'LP',
        interval: 0,
        minRewardThreshold: 0,
        reinvestPercentage: 100
      };
      
      return this.executeCompound(strategy);
    }

    return this.executeCompound(activePool.strategy);
  }

  /**
   * Execute compound operation
   */
  private async executeCompound(strategy: CompoundStrategy): Promise<CompoundResult> {
    const startTime = Date.now();
    const poolKey = strategy.poolAddress.toString();
    
    try {
      logger.info(`🔄 Executing compound for pool ${poolKey}`);

      // Check emergency stop
      if (this.emergencyStopCheck && await this.emergencyStopCheck()) {
        logger.warn('Emergency stop triggered, skipping compound');
        return this.createFailedResult('Emergency stop triggered');
      }

      // Check gas price
      const gasPrice = await this.getCurrentGasPrice();
      if (gasPrice > this.maxGasPrice) {
        logger.warn(`Gas price too high: ${gasPrice} > ${this.maxGasPrice}`);
        return this.createFailedResult('Gas price too high');
      }

      // Get current position and rewards
      const position = await this.getUserPosition(strategy.poolAddress);
      const pendingRewards = await this.getPendingRewards(strategy.poolAddress);

      logger.info(`  Position: ${position.amount}`);
      logger.info(`  Pending rewards: ${pendingRewards}`);

      // Check minimum threshold
      if (pendingRewards < strategy.minRewardThreshold) {
        logger.info(`Rewards below threshold: ${pendingRewards} < ${strategy.minRewardThreshold}`);
        return this.createFailedResult('Rewards below threshold');
      }

      // Step 1: Harvest rewards
      const harvestResult = await this.harvestRewards(strategy.poolAddress);
      if (!harvestResult.success) {
        throw new Error('Failed to harvest rewards');
      }

      // Step 2: Calculate reinvest amount
      const reinvestAmount = (pendingRewards * strategy.reinvestPercentage) / 100;
      const keepAmount = pendingRewards - reinvestAmount;

      logger.info(`  Reinvesting: ${reinvestAmount} (${strategy.reinvestPercentage}%)`);
      if (keepAmount > 0) {
        logger.info(`  Keeping: ${keepAmount}`);
      }

      // Step 3: Reinvest based on strategy
      let reinvestResult;
      switch (strategy.strategy) {
        case 'LP':
          reinvestResult = await this.reinvestLP(
            strategy.poolAddress,
            reinvestAmount,
            strategy.maxSlippage || 1.0
          );
          break;
          
        case 'STAKING':
          reinvestResult = await this.reinvestStaking(
            strategy.poolAddress,
            reinvestAmount
          );
          break;
          
        case 'FARMING':
          reinvestResult = await this.reinvestFarming(
            strategy.poolAddress,
            reinvestAmount,
            strategy.maxSlippage || 1.0
          );
          break;
          
        default:
          throw new Error(`Unknown strategy: ${strategy.strategy}`);
      }

      if (!reinvestResult.success) {
        throw new Error('Failed to reinvest');
      }

      // Step 4: Get new position value
      const newPosition = await this.getUserPosition(strategy.poolAddress);
      const gasUsed = (Date.now() - startTime) * 0.000001; // Simplified gas calculation

      // Update statistics
      this.updateStatistics(poolKey, {
        success: true,
        rewardsHarvested: pendingRewards,
        amountReinvested: reinvestAmount,
        gasUsed
      });

      // Send notification
      await this.notificationService.send({
        type: 'COMPOUND_SUCCESS',
        pool: poolKey,
        rewards: pendingRewards,
        reinvested: reinvestAmount,
        newPosition: newPosition.amount
      });

      const result: CompoundResult = {
        success: true,
        rewardsHarvested: pendingRewards,
        amountReinvested: reinvestAmount,
        newPositionValue: newPosition.amount,
        gasUsed,
        signature: reinvestResult.signature,
        timestamp: new Date()
      };

      logger.info(`✅ Compound successful for pool ${poolKey}`);
      logger.info(`  Harvested: ${pendingRewards}`);
      logger.info(`  Reinvested: ${reinvestAmount}`);
      logger.info(`  Gas used: ${gasUsed} SOL`);

      return result;

    } catch (error: any) {
      logger.error(`Compound failed for pool ${poolKey}: ${error.message}`);
      
      // Update statistics
      this.updateStatistics(poolKey, {
        success: false,
        rewardsHarvested: 0,
        amountReinvested: 0,
        gasUsed: 0
      });

      // Send notification
      await this.notificationService.send({
        type: 'COMPOUND_FAILED',
        pool: poolKey,
        error: error.message
      });

      return this.createFailedResult(error.message);
    }
  }

  /**
   * Harvest rewards from pool
   */
  private async harvestRewards(poolAddress: PublicKey): Promise<any> {
    // Implement actual harvest logic using Saros SDK
    // This is a placeholder
    return {
      success: true,
      signature: 'harvest_tx_signature'
    };
  }

  /**
   * Reinvest into LP position
   */
  private async reinvestLP(
    poolAddress: PublicKey,
    amount: number,
    maxSlippage: number
  ): Promise<any> {
    // Implement LP reinvestment using Saros SDK
    // This is a placeholder
    return {
      success: true,
      signature: 'reinvest_lp_tx_signature'
    };
  }

  /**
   * Reinvest into staking position
   */
  private async reinvestStaking(
    poolAddress: PublicKey,
    amount: number
  ): Promise<any> {
    // Implement staking reinvestment using Saros SDK
    // This is a placeholder
    return {
      success: true,
      signature: 'reinvest_stake_tx_signature'
    };
  }

  /**
   * Reinvest into farming position
   */
  private async reinvestFarming(
    poolAddress: PublicKey,
    amount: number,
    maxSlippage: number
  ): Promise<any> {
    // Implement farming reinvestment using Saros SDK
    // This is a placeholder
    return {
      success: true,
      signature: 'reinvest_farm_tx_signature'
    };
  }

  /**
   * Get pool information
   */
  private async getPoolInfo(poolAddress: PublicKey): Promise<any> {
    // Implement using Saros SDK
    // This is a placeholder
    return {
      exists: true,
      tvl: 1000000,
      apy: 50
    };
  }

  /**
   * Get user position
   */
  private async getUserPosition(poolAddress: PublicKey): Promise<any> {
    // Implement using Saros SDK
    // This is a placeholder
    return {
      amount: 1000,
      rewards: 10
    };
  }

  /**
   * Get pending rewards
   */
  private async getPendingRewards(poolAddress: PublicKey): Promise<number> {
    // Implement using Saros SDK
    // This is a placeholder
    return Math.random() * 10 + 1; // Random rewards for demo
  }

  /**
   * Get current gas price
   */
  async getCurrentGasPrice(): Promise<number> {
    return this.gasOptimizer.getCurrentGasPrice();
  }

  /**
   * Convert interval to cron expression
   */
  private intervalToCron(intervalMs: number): string {
    const minutes = Math.floor(intervalMs / 60000);
    
    if (minutes < 60) {
      return `*/${minutes} * * * *`; // Every X minutes
    }
    
    const hours = Math.floor(minutes / 60);
    if (hours < 24) {
      return `0 */${hours} * * *`; // Every X hours
    }
    
    const days = Math.floor(hours / 24);
    return `0 0 */${days} * *`; // Every X days
  }

  /**
   * Update statistics
   */
  private updateStatistics(
    poolKey: string,
    result: {
      success: boolean;
      rewardsHarvested: number;
      amountReinvested: number;
      gasUsed: number;
    }
  ): void {
    // Update global stats
    this.globalStats.totalCompounds++;
    
    if (result.success) {
      this.globalStats.successfulCompounds++;
      this.globalStats.totalRewardsHarvested += result.rewardsHarvested;
      this.globalStats.totalReinvested += result.amountReinvested;
      this.globalStats.totalGasSpent += result.gasUsed;
    } else {
      this.globalStats.failedCompounds++;
    }
    
    this.globalStats.successRate = 
      (this.globalStats.successfulCompounds / this.globalStats.totalCompounds) * 100;
    
    this.globalStats.netProfit = 
      this.globalStats.totalRewardsHarvested - this.globalStats.totalGasSpent;
    
    this.globalStats.lastCompoundTime = new Date();

    // Update pool-specific stats
    const activePool = this.activePools.get(poolKey);
    if (activePool) {
      activePool.statistics.compounds++;
      activePool.statistics.totalHarvested += result.rewardsHarvested;
      activePool.statistics.totalReinvested += result.amountReinvested;
      activePool.statistics.totalGas += result.gasUsed;
      activePool.statistics.lastCompound = new Date();
      
      activePool.statistics.averageReward = 
        activePool.statistics.totalHarvested / activePool.statistics.compounds;
    }
  }

  /**
   * Create failed result
   */
  private createFailedResult(error: string): CompoundResult {
    return {
      success: false,
      rewardsHarvested: 0,
      amountReinvested: 0,
      newPositionValue: 0,
      gasUsed: 0,
      signature: '',
      timestamp: new Date(),
      error
    };
  }

  /**
   * Get statistics
   */
  async getStatistics(): Promise<Statistics> {
    return { ...this.globalStats };
  }

  /**
   * Get active pools
   */
  async getActivePools(): Promise<PublicKey[]> {
    return Array.from(this.activePools.keys()).map(key => new PublicKey(key));
  }

  /**
   * Set emergency stop function
   */
  setEmergencyStop(checkFn: () => Promise<boolean>): void {
    this.emergencyStopCheck = checkFn;
    logger.info('Emergency stop function configured');
  }

  /**
   * Stop all active compounds
   */
  async stopAll(): Promise<void> {
    logger.info('Stopping all active compounds...');
    
    for (const [poolKey, activePool] of this.activePools.entries()) {
      activePool.cronJob.stop();
      logger.info(`Stopped compound for pool ${poolKey}`);
    }
    
    this.activePools.clear();
    logger.info('All compounds stopped');
  }
}