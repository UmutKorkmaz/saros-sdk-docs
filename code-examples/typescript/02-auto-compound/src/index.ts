/**
 * Saros Auto-Compound Yield Farming
 * Main entry point for automated yield optimization
 */

import * as dotenv from 'dotenv';
import { AutoCompounder } from './AutoCompounder';
import { YieldOptimizer } from './YieldOptimizer';
import { RewardCalculator } from './RewardCalculator';
import { logger } from './utils/logger';
import { PublicKey } from '@solana/web3.js';

// Load environment variables
dotenv.config();

/**
 * Main demonstration of auto-compound functionality
 */
async function main() {
  try {
    logger.info('🚀 Starting Saros Auto-Compound Yield Farming');
    logger.info('==============================================\n');

    // Initialize components
    const autoCompounder = new AutoCompounder({
      rpcUrl: process.env.SOLANA_RPC_URL || 'https://api.devnet.solana.com',
      privateKey: process.env.WALLET_PRIVATE_KEY!,
      network: (process.env.SOLANA_NETWORK as 'devnet' | 'mainnet-beta') || 'devnet'
    });

    const yieldOptimizer = new YieldOptimizer({
      rpcUrl: process.env.SOLANA_RPC_URL!,
      privateKey: process.env.WALLET_PRIVATE_KEY!
    });

    const rewardCalculator = new RewardCalculator();

    // Example pool addresses (replace with actual addresses)
    const LP_POOL = process.env.LP_POOL_ADDRESS || 'POOL_ADDRESS_PLACEHOLDER';
    const STAKING_POOL = process.env.STAKING_POOL_ADDRESS || 'STAKING_ADDRESS_PLACEHOLDER';
    const FARM_ADDRESS = process.env.FARM_ADDRESS || 'FARM_ADDRESS_PLACEHOLDER';

    // Example 1: Basic Auto-Compound for LP Pool
    logger.info('Example 1: Basic LP Auto-Compound');
    logger.info('----------------------------------');
    
    const lpCompoundResult = await autoCompounder.start({
      poolAddress: new PublicKey(LP_POOL),
      strategy: 'LP',
      interval: parseInt(process.env.COMPOUND_INTERVAL || '3600000'),
      minRewardThreshold: parseFloat(process.env.MIN_REWARD_THRESHOLD || '1.0'),
      reinvestPercentage: parseInt(process.env.REINVEST_PERCENTAGE || '100')
    });

    if (lpCompoundResult.success) {
      logger.info('✅ LP Auto-compound started successfully');
      logger.info(`  Pool: ${LP_POOL.slice(0, 8)}...`);
      logger.info(`  Interval: ${lpCompoundResult.interval / 1000}s`);
      logger.info(`  Min threshold: ${lpCompoundResult.minThreshold}`);
      logger.info(`  Next compound: ${lpCompoundResult.nextCompoundTime}`);
    }

    // Example 2: Multi-Strategy Yield Optimization
    logger.info('\nExample 2: Multi-Strategy Yield Optimization');
    logger.info('---------------------------------------------');
    
    // Add LP strategy (50% allocation)
    await yieldOptimizer.addStrategy({
      type: 'LP',
      poolAddress: new PublicKey(LP_POOL),
      weight: 0.5,
      autoCompound: true,
      compoundInterval: 3600000, // 1 hour
      minRewardThreshold: 1.0
    });

    // Add Staking strategy (30% allocation)
    await yieldOptimizer.addStrategy({
      type: 'STAKING',
      poolAddress: new PublicKey(STAKING_POOL),
      weight: 0.3,
      autoCompound: true,
      compoundInterval: 7200000, // 2 hours
      minRewardThreshold: 0.5
    });

    // Add Farming strategy (20% allocation)
    await yieldOptimizer.addStrategy({
      type: 'FARMING',
      poolAddress: new PublicKey(FARM_ADDRESS),
      weight: 0.2,
      autoCompound: true,
      compoundInterval: 86400000, // 24 hours
      minRewardThreshold: 2.0
    });

    // Start optimization
    const optimizationResult = await yieldOptimizer.startOptimization();
    
    logger.info('✅ Multi-strategy optimization started');
    logger.info(`  Active strategies: ${optimizationResult.activeStrategies}`);
    logger.info(`  Total allocation: ${optimizationResult.totalAllocation * 100}%`);
    logger.info(`  Projected APY: ${optimizationResult.projectedAPY}%`);

    // Example 3: Calculate and Display Current Yields
    logger.info('\nExample 3: Current Yield Analysis');
    logger.info('----------------------------------');
    
    const lpPosition = {
      poolAddress: new PublicKey(LP_POOL),
      stakedAmount: 1000,
      rewards: 10,
      timeStaked: Date.now() - 86400000 // 1 day ago
    };

    const yieldAnalysis = await rewardCalculator.analyzePosition(lpPosition);
    
    logger.info('📊 LP Position Analysis:');
    logger.info(`  Staked: $${yieldAnalysis.stakedValue}`);
    logger.info(`  Pending rewards: $${yieldAnalysis.pendingRewards}`);
    logger.info(`  Daily rate: ${yieldAnalysis.dailyRate}%`);
    logger.info(`  Current APY: ${yieldAnalysis.currentAPY}%`);
    logger.info(`  Compound APY: ${yieldAnalysis.compoundAPY}%`);

    // Example 4: Manual Compound Trigger
    logger.info('\nExample 4: Manual Compound Execution');
    logger.info('-------------------------------------');
    
    const manualCompound = await autoCompounder.compoundNow(
      new PublicKey(LP_POOL)
    );

    if (manualCompound.success) {
      logger.info('✅ Manual compound executed');
      logger.info(`  Rewards harvested: ${manualCompound.rewardsHarvested}`);
      logger.info(`  Amount reinvested: ${manualCompound.amountReinvested}`);
      logger.info(`  New position: ${manualCompound.newPositionValue}`);
      logger.info(`  Gas used: ${manualCompound.gasUsed} SOL`);
      logger.info(`  TX: ${manualCompound.signature}`);
    }

    // Example 5: Optimal Compound Frequency
    logger.info('\nExample 5: Optimal Compound Frequency');
    logger.info('--------------------------------------');
    
    const optimalFrequency = rewardCalculator.calculateOptimalFrequency({
      dailyRewards: 10,
      positionSize: 1000,
      gasPrice: 0.001,
      currentAPY: 50
    });

    logger.info('⚡ Optimal compounding settings:');
    logger.info(`  Frequency: Every ${optimalFrequency.hours} hours`);
    logger.info(`  Expected APY gain: +${optimalFrequency.apyIncrease}%`);
    logger.info(`  Daily gas cost: ${optimalFrequency.dailyGasCost} SOL`);
    logger.info(`  Break-even threshold: $${optimalFrequency.breakEvenThreshold}`);

    // Example 6: Performance Statistics
    logger.info('\nExample 6: Performance Statistics');
    logger.info('----------------------------------');
    
    const stats = await autoCompounder.getStatistics();
    
    logger.info('📈 Auto-compound performance:');
    logger.info(`  Total compounds: ${stats.totalCompounds}`);
    logger.info(`  Success rate: ${stats.successRate}%`);
    logger.info(`  Total harvested: $${stats.totalRewardsHarvested}`);
    logger.info(`  Total reinvested: $${stats.totalReinvested}`);
    logger.info(`  Total gas spent: ${stats.totalGasSpent} SOL`);
    logger.info(`  Net profit: $${stats.netProfit}`);
    logger.info(`  Average APY boost: +${stats.averageAPYBoost}%`);

    // Example 7: Emergency Stop
    logger.info('\nExample 7: Emergency Controls');
    logger.info('------------------------------');
    
    // Set up emergency stop
    autoCompounder.setEmergencyStop(async () => {
      const gasPrice = await autoCompounder.getCurrentGasPrice();
      return gasPrice > parseFloat(process.env.MAX_GAS_PRICE || '0.01');
    });

    logger.info('✅ Emergency stop configured');
    logger.info('  Triggers when gas > 0.01 SOL');

    // Display summary
    logger.info('\n🎉 Auto-Compound System Active!');
    logger.info('================================\n');
    logger.info('System Status:');
    logger.info(`  Active pools: ${await autoCompounder.getActivePools().length}`);
    logger.info(`  Next compound check: ${new Date(Date.now() + 60000).toLocaleTimeString()}`);
    logger.info(`  Auto-compound: ${process.env.AUTO_COMPOUND_ENABLED === 'true' ? 'ENABLED' : 'DISABLED'}`);
    
    // Keep the process running for auto-compound
    if (process.env.AUTO_COMPOUND_ENABLED === 'true') {
      logger.info('\n⏰ Auto-compound scheduler is running...');
      logger.info('Press Ctrl+C to stop\n');
      
      // Prevent process from exiting
      setInterval(() => {
        // Heartbeat
      }, 60000);
    }

  } catch (error) {
    logger.error('Fatal error in main:', error);
    process.exit(1);
  }
}

// Handle graceful shutdown
process.on('SIGINT', async () => {
  logger.info('\n👋 Shutting down auto-compounder...');
  
  // Stop all active compounds
  // await autoCompounder.stopAll();
  
  process.exit(0);
});

process.on('unhandledRejection', (error) => {
  logger.error('Unhandled rejection:', error);
  process.exit(1);
});

// Run the demo
if (require.main === module) {
  main().catch((error) => {
    logger.error('Failed to run auto-compound:', error);
    process.exit(1);
  });
}

export { AutoCompounder, YieldOptimizer, RewardCalculator };