/**
 * Saros Swap with Slippage Protection
 * Main entry point for the swap example
 */

import * as dotenv from 'dotenv';
import { SwapManager } from './SwapManager';
import { DynamicSlippageSwap } from './DynamicSlippageSwap';
import { PriceMonitor } from './PriceMonitor';
import { logger } from './utils/logger';
import { PublicKey } from '@solana/web3.js';

// Load environment variables
dotenv.config();

/**
 * Main demonstration of swap functionality
 */
async function main() {
  try {
    logger.info('🚀 Starting Saros Swap with Slippage Protection Demo');
    logger.info('================================================\n');

    // Initialize swap manager
    const swapManager = new SwapManager({
      rpcUrl: process.env.SOLANA_RPC_URL || 'https://api.devnet.solana.com',
      privateKey: process.env.WALLET_PRIVATE_KEY!,
      network: (process.env.SOLANA_NETWORK as 'devnet' | 'mainnet-beta') || 'devnet'
    });

    // Token addresses (devnet)
    const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
    const SOL_MINT = 'So11111111111111111111111111111111111111112';
    const C98_MINT = 'C98A4nkJXhpVZNAZdHUA95RpTF3T4whtQubL3YobiUX9';

    // Example 1: Basic swap with fixed slippage
    logger.info('Example 1: Basic Swap with Fixed Slippage');
    logger.info('------------------------------------------');
    
    const basicSwapResult = await swapManager.swap({
      fromMint: new PublicKey(USDC_MINT),
      toMint: new PublicKey(SOL_MINT),
      amount: 10, // 10 USDC
      slippageTolerance: 0.5, // 0.5%
      simulateFirst: true
    });

    if (basicSwapResult.success) {
      logger.info(`✅ Basic swap completed successfully!`);
      logger.info(`  Transaction: ${basicSwapResult.signature}`);
      logger.info(`  Amount in: ${basicSwapResult.amountIn} USDC`);
      logger.info(`  Amount out: ${basicSwapResult.amountOut} SOL`);
      logger.info(`  Price impact: ${basicSwapResult.priceImpact}%`);
    }

    // Example 2: Dynamic slippage based on market conditions
    logger.info('\nExample 2: Dynamic Slippage Swap');
    logger.info('----------------------------------');
    
    const dynamicSwap = new DynamicSlippageSwap({
      rpcUrl: process.env.SOLANA_RPC_URL!,
      privateKey: process.env.WALLET_PRIVATE_KEY!
    });

    const dynamicResult = await dynamicSwap.executeWithOptimalSlippage({
      fromMint: new PublicKey(USDC_MINT),
      toMint: new PublicKey(C98_MINT),
      amount: 50, // 50 USDC
      maxPriceImpact: 3.0,
      urgency: 'normal',
      useMultiHop: true
    });

    if (dynamicResult.success) {
      logger.info(`✅ Dynamic slippage swap completed!`);
      logger.info(`  Optimal slippage used: ${dynamicResult.slippageUsed}%`);
      logger.info(`  Route: ${dynamicResult.route?.join(' → ')}`);
    }

    // Example 3: Price monitoring and execution
    logger.info('\nExample 3: Price-Triggered Swap');
    logger.info('---------------------------------');
    
    const priceMonitor = new PriceMonitor(swapManager);
    
    // Set up price alert and execute when target is reached
    const monitoringResult = await priceMonitor.swapAtTargetPrice({
      fromMint: new PublicKey(USDC_MINT),
      toMint: new PublicKey(SOL_MINT),
      amount: 100,
      targetPrice: 50.0, // Execute when 1 SOL = 50 USDC
      tolerance: 2.0, // ±2%
      maxWaitTime: 60000, // Wait max 1 minute (for demo)
      checkInterval: 5000 // Check every 5 seconds
    });

    if (monitoringResult.executed) {
      logger.info(`✅ Price-triggered swap executed at target price!`);
      logger.info(`  Target price: ${monitoringResult.targetPrice}`);
      logger.info(`  Actual price: ${monitoringResult.executionPrice}`);
    }

    // Example 4: Batch swaps with optimization
    logger.info('\nExample 4: Optimized Batch Swaps');
    logger.info('----------------------------------');
    
    const batchSwaps = [
      { from: USDC_MINT, to: SOL_MINT, amount: 10 },
      { from: SOL_MINT, to: C98_MINT, amount: 0.5 },
      { from: C98_MINT, to: USDC_MINT, amount: 100 }
    ];

    const batchResults = await swapManager.executeBatchSwaps(batchSwaps);
    
    logger.info(`✅ Batch swaps completed: ${batchResults.successful}/${batchResults.total}`);
    logger.info(`  Total gas saved: ~${batchResults.gasSaved} SOL`);

    // Example 5: MEV protection
    logger.info('\nExample 5: Swap with MEV Protection');
    logger.info('-------------------------------------');
    
    const mevProtectedSwap = await swapManager.swapWithMEVProtection({
      fromMint: new PublicKey(USDC_MINT),
      toMint: new PublicKey(SOL_MINT),
      amount: 1000, // Large swap
      maxSlippage: 1.0,
      priorityFee: 0.001, // Priority fee in SOL
      useJitoBundle: false // Set to true if using Jito
    });

    if (mevProtectedSwap.success) {
      logger.info(`✅ MEV-protected swap completed!`);
      logger.info(`  Protected from sandwich attacks`);
      logger.info(`  Priority fee used: ${mevProtectedSwap.priorityFee} SOL`);
    }

    logger.info('\n🎉 All examples completed successfully!');
    logger.info('========================================\n');

    // Display summary statistics
    const stats = swapManager.getStatistics();
    logger.info('📊 Session Statistics:');
    logger.info(`  Total swaps: ${stats.totalSwaps}`);
    logger.info(`  Successful: ${stats.successfulSwaps}`);
    logger.info(`  Failed: ${stats.failedSwaps}`);
    logger.info(`  Average slippage: ${stats.averageSlippage}%`);
    logger.info(`  Total volume: $${stats.totalVolume}`);

  } catch (error) {
    logger.error('Fatal error in main:', error);
    process.exit(1);
  }
}

// Handle graceful shutdown
process.on('SIGINT', async () => {
  logger.info('\n👋 Shutting down gracefully...');
  process.exit(0);
});

process.on('unhandledRejection', (error) => {
  logger.error('Unhandled rejection:', error);
  process.exit(1);
});

// Run the demo
if (require.main === module) {
  main().catch((error) => {
    logger.error('Failed to run demo:', error);
    process.exit(1);
  });
}

export { SwapManager, DynamicSlippageSwap, PriceMonitor };