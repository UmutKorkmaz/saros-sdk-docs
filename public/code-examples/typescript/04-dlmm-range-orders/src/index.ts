/**
 * DLMM Range Orders Example
 * 
 * Demonstrates limit order functionality using concentrated liquidity bins.
 * Range orders allow precise price execution with zero slippage.
 */

import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import dotenv from 'dotenv';
import { RangeOrderManager } from './RangeOrderManager';
import { RangeOrderMonitor } from './RangeOrderMonitor';
import { AutomatedExecutor } from './AutomatedExecutor';
import { getConnection, getWallet } from './utils/connection';
import { logger } from './utils/logger';

dotenv.config();

// Example pool addresses (mainnet)
const SOL_USDC_POOL = new PublicKey('7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm');
const ETH_USDC_POOL = new PublicKey('2QdhepnKRTLjjSqPL1PtKNwqrUkoLee5Gqs8bvZhRdMv');

async function main() {
  try {
    const connection = getConnection();
    const wallet = getWallet();
    
    logger.info('Starting DLMM Range Orders Example');
    logger.info(`Wallet: ${wallet.publicKey.toString()}`);

    // Parse command line arguments
    const args = process.argv.slice(2);
    const command = args[0] || 'demo';

    switch (command) {
      case 'demo':
        await runDemoScenarios(connection, wallet);
        break;
      case 'buy':
        await createLimitBuyOrder(connection, wallet, args);
        break;
      case 'sell':
        await createLimitSellOrder(connection, wallet, args);
        break;
      case 'ladder':
        await createOrderLadder(connection, wallet, args);
        break;
      case 'monitor':
        await monitorOrders(connection, wallet);
        break;
      case 'automate':
        await runAutomation(connection, wallet);
        break;
      default:
        console.log('Usage: npm run dev [demo|buy|sell|ladder|monitor|automate]');
    }
  } catch (error) {
    logger.error('Error in main:', error);
    process.exit(1);
  }
}

/**
 * Run demonstration scenarios
 */
async function runDemoScenarios(connection: Connection, wallet: Keypair) {
  logger.info('Running DLMM Range Order Demonstrations');
  
  const manager = new RangeOrderManager(connection, wallet);
  
  // Scenario 1: Simple Limit Buy Order
  logger.info('\n=== Scenario 1: Limit Buy Order ===');
  await demonstrateLimitBuy(manager, connection);
  
  // Scenario 2: Simple Limit Sell Order
  logger.info('\n=== Scenario 2: Limit Sell Order ===');
  await demonstrateLimitSell(manager);
  
  // Scenario 3: Buy Ladder Strategy
  logger.info('\n=== Scenario 3: Buy Ladder Strategy ===');
  await demonstrateBuyLadder(manager);
  
  // Scenario 4: Take Profit Orders
  logger.info('\n=== Scenario 4: Take Profit Strategy ===');
  await demonstrateTakeProfit(manager);
  
  // Scenario 5: Stop Loss Protection
  logger.info('\n=== Scenario 5: Stop Loss Protection ===');
  await demonstrateStopLoss(manager, connection);
}

/**
 * Demonstrate limit buy order
 */
async function demonstrateLimitBuy(manager: RangeOrderManager, connection: Connection) {
  const currentPrice = await manager.getCurrentPrice(SOL_USDC_POOL);
  logger.info(`Current SOL price: $${currentPrice.toFixed(2)}`);
  
  const targetBuyPrice = currentPrice * 0.98; // Buy 2% below current
  logger.info(`Creating limit buy at $${targetBuyPrice.toFixed(2)}`);
  
  const order = await manager.createLimitBuy({
    poolAddress: SOL_USDC_POOL,
    targetPrice: targetBuyPrice,
    amountUSDC: 1000,
    tolerance: 0.1, // 0.1% tolerance
    metadata: {
      strategy: 'dip-buy',
      createdAt: Date.now()
    }
  });
  
  logger.info(`Limit buy order created:`);
  logger.info(`- Position ID: ${order.positionId}`);
  logger.info(`- Bin ID: ${order.binId}`);
  logger.info(`- Actual Price: $${order.actualPrice.toFixed(4)}`);
  logger.info(`- Status: ${order.status}`);
  
  // Monitor for execution
  const monitor = new RangeOrderMonitor(connection);
  monitor.watchPosition(order.positionId, (update) => {
    logger.info(`Order update: ${update.status} at price $${update.currentPrice}`);
  });
}

/**
 * Demonstrate limit sell order
 */
async function demonstrateLimitSell(manager: RangeOrderManager) {
  const currentPrice = await manager.getCurrentPrice(SOL_USDC_POOL);
  logger.info(`Current SOL price: $${currentPrice.toFixed(2)}`);
  
  const targetSellPrice = currentPrice * 1.02; // Sell 2% above current
  logger.info(`Creating limit sell at $${targetSellPrice.toFixed(2)}`);
  
  const order = await manager.createLimitSell({
    poolAddress: SOL_USDC_POOL,
    targetPrice: targetSellPrice,
    amountSOL: 10,
    tolerance: 0.1,
    metadata: {
      strategy: 'take-profit',
      createdAt: Date.now()
    }
  });
  
  logger.info(`Limit sell order created:`);
  logger.info(`- Position ID: ${order.positionId}`);
  logger.info(`- Bin ID: ${order.binId}`);
  logger.info(`- Actual Price: $${order.actualPrice.toFixed(4)}`);
  logger.info(`- Status: ${order.status}`);
}

/**
 * Demonstrate buy ladder strategy
 */
async function demonstrateBuyLadder(manager: RangeOrderManager) {
  const currentPrice = await manager.getCurrentPrice(SOL_USDC_POOL);
  logger.info(`Current SOL price: $${currentPrice.toFixed(2)}`);
  
  // Create ladder of buy orders from -2% to -5%
  const ladder = await manager.createBuyLadder({
    poolAddress: SOL_USDC_POOL,
    startPrice: currentPrice * 0.98,
    endPrice: currentPrice * 0.95,
    steps: 5,
    totalAmountUSDC: 5000,
    distribution: 'linear' // or 'exponential'
  });
  
  logger.info(`Buy ladder created with ${ladder.orders.length} orders:`);
  ladder.orders.forEach((order: any, i) => {
    logger.info(`  ${i + 1}. $${order.price?.toFixed(2) || 'N/A'} - ${order.amount || 'N/A'} USDC`);
  });
  
  logger.info(`Total invested: $${ladder.totalInvested}`);
  logger.info(`Average entry: $${ladder.averagePrice.toFixed(2)}`);
}

/**
 * Demonstrate take profit orders
 */
async function demonstrateTakeProfit(manager: RangeOrderManager) {
  const currentPrice = await manager.getCurrentPrice(SOL_USDC_POOL);
  logger.info(`Current SOL price: $${currentPrice.toFixed(2)}`);
  
  // Assume we have a position to take profit on
  const position = {
    amount: 100, // 100 SOL
    entryPrice: currentPrice * 0.8  // Entry at 80% of current price for better demo
  };
  
  logger.info(`Position: ${position.amount} SOL @ $${position.entryPrice.toFixed(2)}`);
  
  // Create multiple take profit levels
  const takeProfits = await manager.createTakeProfitLevels({
    poolAddress: SOL_USDC_POOL,
    positionAmount: position.amount,
    levels: [
      { price: currentPrice * 1.05, percentage: 25 }, // 5% above current, sell 25%
      { price: currentPrice * 1.10, percentage: 25 }, // 10% above current, sell 25%
      { price: currentPrice * 1.15, percentage: 25 }, // 15% above current, sell 25%
      { price: currentPrice * 1.20, percentage: 25 }, // 20% above current, sell 25%
    ]
  });
  
  logger.info(`Take profit orders created:`);
  takeProfits.forEach((tp, i) => {
    logger.info(`  Level ${i + 1}: Sell ${tp.amount} SOL at $${tp.price.toFixed(2)} (+${tp.profitPercentage}%)`);
  });
}

/**
 * Demonstrate stop loss protection
 */
async function demonstrateStopLoss(manager: RangeOrderManager, connection: Connection) {
  const currentPrice = await manager.getCurrentPrice(SOL_USDC_POOL);
  logger.info(`Current SOL price: $${currentPrice.toFixed(2)}`);
  
  logger.info(`⚠️  DLMM Range Orders Limitation:`);
  logger.info(`   DLMM range orders cannot create sell orders below current price.`);
  logger.info(`   For stop losses in DLMM, you would typically:`);
  logger.info(`   1. Use a separate monitoring system`);
  logger.info(`   2. Trigger market sells when price hits target`);
  logger.info(`   3. Or use AMM pools for stop loss functionality`);
  
  // Demonstrate the concept with market monitoring instead
  const stopLossPrice = currentPrice * 0.95;
  logger.info(`\n🔍 Simulating stop loss monitoring:`);
  logger.info(`   Target price: $${stopLossPrice.toFixed(2)} (5% below current)`);
  logger.info(`   Monitor position: 50 SOL`);
  logger.info(`   Action: Market sell if price <= $${stopLossPrice.toFixed(2)}`);
  
  // Simulate monitoring (in production, this would be a real price feed)
  logger.info(`\n📊 Price monitoring active (simulation):`);
  for (let i = 0; i < 3; i++) {
    const mockPrice = currentPrice - (Math.random() * 0.05);
    await new Promise(resolve => setTimeout(resolve, 500));
    
    if (mockPrice <= stopLossPrice) {
      logger.info(`🚨 Stop loss triggered at $${mockPrice.toFixed(2)}`);
      logger.info(`   Executing market sell for 50 SOL...`);
      logger.info(`   ✅ Stop loss executed successfully`);
      return;
    } else {
      logger.info(`   Price: $${mockPrice.toFixed(2)} (above trigger)`);
    }
  }
  
  logger.info(`   Stop loss monitoring continues...`);
}

/**
 * Create limit buy order from CLI
 */
async function createLimitBuyOrder(connection: Connection, wallet: Keypair, args: string[]) {
  const price = parseFloat(args[1] || '50');
  const amount = parseFloat(args[2] || '1000');
  
  const manager = new RangeOrderManager(connection, wallet);
  
  logger.info(`Creating limit buy order:`);
  logger.info(`- Price: $${price}`);
  logger.info(`- Amount: ${amount} USDC`);
  
  const order = await manager.createLimitBuy({
    poolAddress: SOL_USDC_POOL,
    targetPrice: price,
    amountUSDC: amount,
    tolerance: 0.1
  });
  
  logger.info(`Order created successfully!`);
  logger.info(`Position ID: ${order.positionId}`);
  logger.info(`Monitor with: npm run monitor -- ${order.positionId}`);
}

/**
 * Create limit sell order from CLI
 */
async function createLimitSellOrder(connection: Connection, wallet: Keypair, args: string[]) {
  const price = parseFloat(args[1] || '55');
  const amount = parseFloat(args[2] || '10');
  
  const manager = new RangeOrderManager(connection, wallet);
  
  logger.info(`Creating limit sell order:`);
  logger.info(`- Price: $${price}`);
  logger.info(`- Amount: ${amount} SOL`);
  
  const order = await manager.createLimitSell({
    poolAddress: SOL_USDC_POOL,
    targetPrice: price,
    amountSOL: amount,
    tolerance: 0.1
  });
  
  logger.info(`Order created successfully!`);
  logger.info(`Position ID: ${order.positionId}`);
}

/**
 * Create order ladder from CLI
 */
async function createOrderLadder(connection: Connection, wallet: Keypair, args: string[]) {
  const startPrice = parseFloat(args[1] || '48');
  const endPrice = parseFloat(args[2] || '50');
  const totalAmount = parseFloat(args[3] || '5000');
  const steps = parseInt(args[4] || '5');
  
  const manager = new RangeOrderManager(connection, wallet);
  
  logger.info(`Creating buy ladder:`);
  logger.info(`- Range: $${startPrice} - $${endPrice}`);
  logger.info(`- Total: ${totalAmount} USDC`);
  logger.info(`- Steps: ${steps}`);
  
  const ladder = await manager.createBuyLadder({
    poolAddress: SOL_USDC_POOL,
    startPrice,
    endPrice,
    steps,
    totalAmountUSDC: totalAmount,
    distribution: 'linear'
  });
  
  logger.info(`Ladder created with ${ladder.orders.length} orders`);
  logger.info(`Average entry: $${ladder.averagePrice.toFixed(2)}`);
}

/**
 * Monitor existing orders
 */
async function monitorOrders(connection: Connection, wallet: Keypair) {
  const monitor = new RangeOrderMonitor(connection);
  const manager = new RangeOrderManager(connection, wallet);
  
  // Get all active orders
  const orders = await manager.getActiveOrders();
  
  logger.info(`Monitoring ${orders.length} active orders...`);
  
  // Set up monitoring for each order
  orders.forEach(order => {
    monitor.watchPosition(order.positionId, (update) => {
      logger.info(`[${order.positionId}] ${update.status} - Price: $${update.currentPrice}`);
      
      if (update.status === 'FILLED') {
        logger.info(`Order filled! Execution price: $${update.executionPrice}`);
      } else if (update.status === 'PARTIALLY_FILLED') {
        logger.info(`Order ${update.fillPercentage}% filled`);
      }
    });
  });
  
  // Keep monitoring
  await new Promise(() => {});
}

/**
 * Run automated execution
 */
async function runAutomation(connection: Connection, wallet: Keypair) {
  const executor = new AutomatedExecutor(connection, wallet);
  
  logger.info('Starting automated executor...');
  
  // Configure strategies
  executor.addStrategy({
    name: 'DCA Buy',
    interval: '0 */4 * * *', // Every 4 hours
    action: async () => {
      const manager = new RangeOrderManager(connection, wallet);
      const currentPrice = await manager.getCurrentPrice(SOL_USDC_POOL);
      
      await manager.createLimitBuy({
        poolAddress: SOL_USDC_POOL,
        targetPrice: currentPrice * 0.99,
        amountUSDC: 100,
        tolerance: 0.2
      });
    }
  });
  
  executor.addStrategy({
    name: 'Rebalance',
    interval: '0 0 * * *', // Daily
    action: async () => {
      const manager = new RangeOrderManager(connection, wallet);
      await manager.rebalanceOrders();
    }
  });
  
  // Start automation
  await executor.start();
  
  logger.info('Automation running. Press Ctrl+C to stop.');
  
  // Keep running
  process.on('SIGINT', async () => {
    logger.info('Stopping automation...');
    await executor.stop();
    process.exit(0);
  });
}

// Run main function
if (require.main === module) {
  main().catch(console.error);
}

export { RangeOrderManager, RangeOrderMonitor, AutomatedExecutor };