/**
 * Multi-Hop Routing Example
 * 
 * Demonstrates advanced routing algorithms for finding optimal
 * swap paths across multiple Saros pools.
 */

import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { SarosClient } from '@saros-finance/sdk';
import BN from 'bn.js';
import dotenv from 'dotenv';
import { MultiHopRouter } from './MultiHopRouter';
import { RouteAnalyzer } from './RouteAnalyzer';
import { ArbitrageDetector } from './ArbitrageDetector';
import { getConnection, getWallet } from './utils/connection';
import { logger } from './utils/logger';

dotenv.config();

// Token mints (mainnet)
const SOL_MINT = new PublicKey('So11111111111111111111111111111111111111112');
const USDC_MINT = new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v');
const USDT_MINT = new PublicKey('Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB');
const ETH_MINT = new PublicKey('7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs');
const BONK_MINT = new PublicKey('DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263');
const SAMO_MINT = new PublicKey('7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU');

async function main() {
  try {
    const connection = getConnection();
    const wallet = getWallet();
    
    logger.info('Starting Multi-Hop Routing Example');
    logger.info(`Wallet: ${wallet.publicKey.toString()}`);

    // Parse command line arguments
    const args = process.argv.slice(2);
    const command = args[0] || 'demo';

    switch (command) {
      case 'demo':
        await runDemoScenarios(connection, wallet);
        break;
      case 'route':
        await findAndExecuteRoute(connection, wallet, args);
        break;
      case 'analyze':
        await analyzeRoutes(connection, wallet, args);
        break;
      case 'arbitrage':
        await findArbitrage(connection, wallet);
        break;
      case 'benchmark':
        await benchmarkRouting(connection, wallet);
        break;
      default:
        console.log('Usage: npm run dev [demo|route|analyze|arbitrage|benchmark]');
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
  logger.info('Running Multi-Hop Routing Demonstrations');
  
  const router = new MultiHopRouter(connection, wallet);
  await router.initialize();
  
  // Scenario 1: Simple 2-Hop Route
  logger.info('\n=== Scenario 1: Simple 2-Hop Route ===');
  await demonstrate2HopRoute(router);
  
  // Scenario 2: Complex 3-Hop Route
  logger.info('\n=== Scenario 2: Complex 3-Hop Route ===');
  await demonstrate3HopRoute(router);
  
  // Scenario 3: Split Route Execution
  logger.info('\n=== Scenario 3: Split Route Execution ===');
  await demonstrateSplitRoute(router);
  
  // Scenario 4: Route Optimization
  logger.info('\n=== Scenario 4: Route Optimization ===');
  await demonstrateRouteOptimization(router);
  
  // Scenario 5: Arbitrage Detection
  logger.info('\n=== Scenario 5: Arbitrage Detection ===');
  await demonstrateArbitrageDetection(connection);
}

/**
 * Demonstrate 2-hop routing
 */
async function demonstrate2HopRoute(router: MultiHopRouter) {
  logger.info('Finding optimal 2-hop route: SOL → USDC → BONK');
  
  const route = await router.findBestRoute({
    fromMint: SOL_MINT,
    toMint: BONK_MINT,
    amount: new BN('1000000000'), // 1 SOL
    maxHops: 2,
    minLiquidity: 10000
  });
  
  if (route) {
    logger.info(`Best route found:`);
    logger.info(`- Path: ${route.path.map(p => p.symbol).join(' → ')}`);
    logger.info(`- Expected output: ${route.expectedOutput.toString()}`);
    logger.info(`- Price impact: ${route.priceImpact.toFixed(4)}%`);
    logger.info(`- Total fees: ${route.totalFees.toFixed(4)}%`);
    logger.info(`- Execution time estimate: ${route.executionTime}ms`);
    
    // Simulate execution (don't actually execute in demo)
    const simulation = await router.simulateRoute(route);
    logger.info(`Simulation result: ${simulation.success ? 'SUCCESS' : 'FAILED'}`);
    
    if (simulation.success) {
      logger.info(`- Simulated output: ${simulation.amountOut.toString()}`);
      logger.info(`- Gas estimate: ${simulation.gasEstimate}`);
    }
  } else {
    logger.warn('No route found for SOL → BONK');
  }
}

/**
 * Demonstrate 3-hop routing
 */
async function demonstrate3HopRoute(router: MultiHopRouter) {
  logger.info('Finding optimal 3-hop route: SAMO → SOL → USDC → USDT');
  
  const route = await router.findBestRoute({
    fromMint: SAMO_MINT,
    toMint: USDT_MINT,
    amount: new BN('1000000000'), // 1000 SAMO (assuming 6 decimals)
    maxHops: 3,
    minLiquidity: 5000
  });
  
  if (route) {
    logger.info(`3-hop route found:`);
    logger.info(`- Path: ${route.path.map(p => p.symbol).join(' → ')}`);
    logger.info(`- Hops: ${route.hops.length}`);
    
    // Show details for each hop
    route.hops.forEach((hop, i) => {
      logger.info(`  Hop ${i + 1}: ${hop.fromToken} → ${hop.toToken}`);
      logger.info(`    Pool: ${hop.poolAddress.toString()}`);
      logger.info(`    Fee: ${hop.fee}%`);
      logger.info(`    Impact: ${hop.priceImpact.toFixed(4)}%`);
    });
    
    logger.info(`- Total price impact: ${route.priceImpact.toFixed(4)}%`);
    logger.info(`- Expected output: ${route.expectedOutput.toString()}`);
  } else {
    logger.warn('No 3-hop route found for SAMO → USDT');
  }
}

/**
 * Demonstrate split route execution
 */
async function demonstrateSplitRoute(router: MultiHopRouter) {
  logger.info('Finding multiple routes for split execution: SOL → USDC');
  
  const routes = await router.findMultipleRoutes({
    fromMint: SOL_MINT,
    toMint: USDC_MINT,
    amount: new BN('10000000000'), // 10 SOL
    maxRoutes: 3,
    maxHops: 2
  });
  
  if (routes.length > 0) {
    logger.info(`Found ${routes.length} routes for split execution:`);
    
    routes.forEach((route, i) => {
      logger.info(`  Route ${i + 1}: ${route.path.map(p => p.symbol).join(' → ')}`);
      logger.info(`    Impact: ${route.priceImpact.toFixed(4)}%`);
      logger.info(`    Output: ${route.expectedOutput.toString()}`);
    });
    
    // Calculate optimal split
    const optimalSplit = router.calculateOptimalSplit(routes, new BN('10000000000'));
    
    logger.info(`Optimal split distribution:`);
    optimalSplit.forEach((split, i) => {
      logger.info(`  Route ${i + 1}: ${(split.percentage * 100).toFixed(2)}% (${split.amount.toString()})`);
    });
    
    logger.info(`Total expected output: ${optimalSplit.reduce((sum, split) => 
      sum.add(split.expectedOutput), new BN(0)).toString()}`);
  } else {
    logger.warn('No routes found for split execution');
  }
}

/**
 * Demonstrate route optimization
 */
async function demonstrateRouteOptimization(router: MultiHopRouter) {
  logger.info('Comparing route optimization strategies');
  
  const amount = new BN('5000000000'); // 5 SOL
  
  // Strategy 1: Minimize price impact
  const minImpactRoute = await router.findBestRoute({
    fromMint: SOL_MINT,
    toMint: USDC_MINT,
    amount,
    strategy: 'MIN_IMPACT',
    maxHops: 3
  });
  
  // Strategy 2: Minimize fees
  const minFeesRoute = await router.findBestRoute({
    fromMint: SOL_MINT,
    toMint: USDC_MINT,
    amount,
    strategy: 'MIN_FEES',
    maxHops: 3
  });
  
  // Strategy 3: Maximize output
  const maxOutputRoute = await router.findBestRoute({
    fromMint: SOL_MINT,
    toMint: USDC_MINT,
    amount,
    strategy: 'MAX_OUTPUT',
    maxHops: 3
  });
  
  logger.info('Route optimization comparison:');
  
  if (minImpactRoute) {
    logger.info(`Min Impact Strategy:`);
    logger.info(`  Output: ${minImpactRoute.expectedOutput.toString()}`);
    logger.info(`  Impact: ${minImpactRoute.priceImpact.toFixed(4)}%`);
    logger.info(`  Fees: ${minImpactRoute.totalFees.toFixed(4)}%`);
  }
  
  if (minFeesRoute) {
    logger.info(`Min Fees Strategy:`);
    logger.info(`  Output: ${minFeesRoute.expectedOutput.toString()}`);
    logger.info(`  Impact: ${minFeesRoute.priceImpact.toFixed(4)}%`);
    logger.info(`  Fees: ${minFeesRoute.totalFees.toFixed(4)}%`);
  }
  
  if (maxOutputRoute) {
    logger.info(`Max Output Strategy:`);
    logger.info(`  Output: ${maxOutputRoute.expectedOutput.toString()}`);
    logger.info(`  Impact: ${maxOutputRoute.priceImpact.toFixed(4)}%`);
    logger.info(`  Fees: ${maxOutputRoute.totalFees.toFixed(4)}%`);
  }
}

/**
 * Demonstrate arbitrage detection
 */
async function demonstrateArbitrageDetection(connection: Connection) {
  logger.info('Scanning for arbitrage opportunities...');
  
  const detector = new ArbitrageDetector(connection);
  await detector.initialize();
  
  // Scan for triangular arbitrage
  const arbitrageOps = await detector.findTriangularArbitrage({
    startToken: SOL_MINT,
    minProfitBps: 10, // 0.1% minimum profit
    maxHops: 3
  });
  
  if (arbitrageOps.length > 0) {
    logger.info(`Found ${arbitrageOps.length} arbitrage opportunities:`);
    
    arbitrageOps.forEach((op, i) => {
      logger.info(`  Opportunity ${i + 1}:`);
      logger.info(`    Path: ${op.path.map(p => p.symbol).join(' → ')}`);
      logger.info(`    Profit: ${op.profitBps / 100}% (${op.profitAmount.toString()})`);
      logger.info(`    Capital required: ${op.capitalRequired.toString()}`);
      logger.info(`    Confidence: ${op.confidence}%`);
    });
  } else {
    logger.info('No arbitrage opportunities found at current prices');
  }
  
  // Check for cross-pool arbitrage
  const crossPoolArb = await detector.findCrossPoolArbitrage({
    tokenA: SOL_MINT,
    tokenB: USDC_MINT,
    minProfitBps: 5
  });
  
  if (crossPoolArb.length > 0) {
    logger.info(`Cross-pool arbitrage opportunities:`);
    crossPoolArb.forEach((arb, i) => {
      logger.info(`  ${i + 1}. Buy on ${arb.buyPool}, sell on ${arb.sellPool}`);
      logger.info(`     Spread: ${arb.spread}%`);
      logger.info(`     Profit: ${arb.profit.toString()}`);
    });
  }
}

/**
 * Find and execute specific route from CLI
 */
async function findAndExecuteRoute(connection: Connection, wallet: Keypair, args: string[]) {
  const fromSymbol = args[1] || 'SOL';
  const toSymbol = args[2] || 'USDC';
  const amount = parseFloat(args[3] || '1');
  
  // Map symbols to mints (simplified)
  const symbolToMint = {
    'SOL': SOL_MINT,
    'USDC': USDC_MINT,
    'USDT': USDT_MINT,
    'ETH': ETH_MINT,
    'BONK': BONK_MINT,
    'SAMO': SAMO_MINT
  };
  
  const fromMint = symbolToMint[fromSymbol as keyof typeof symbolToMint];
  const toMint = symbolToMint[toSymbol as keyof typeof symbolToMint];
  
  if (!fromMint || !toMint) {
    logger.error('Invalid token symbols. Supported: SOL, USDC, USDT, ETH, BONK, SAMO');
    return;
  }
  
  logger.info(`Finding route: ${fromSymbol} → ${toSymbol}`);
  logger.info(`Amount: ${amount} ${fromSymbol}`);
  
  const router = new MultiHopRouter(connection, wallet);
  await router.initialize();
  
  const route = await router.findBestRoute({
    fromMint,
    toMint,
    amount: new BN(amount * Math.pow(10, 9)), // Assume 9 decimals for simplicity
    maxHops: 3
  });
  
  if (!route) {
    logger.error('No route found');
    return;
  }
  
  logger.info(`Route found: ${route.path.map(p => p.symbol).join(' → ')}`);
  logger.info(`Expected output: ${route.expectedOutput.toString()}`);
  logger.info(`Price impact: ${route.priceImpact.toFixed(4)}%`);
  
  // Ask for confirmation (in real implementation)
  logger.info('Execute this route? (This is a demo - not actually executing)');
  
  // Simulate execution
  const result = await router.simulateRoute(route);
  logger.info(`Simulation: ${result.success ? 'Success' : 'Failed'}`);
}

/**
 * Analyze routes from CLI
 */
async function analyzeRoutes(connection: Connection, wallet: Keypair, args: string[]) {
  const fromSymbol = args[1] || 'SOL';
  const toSymbol = args[2] || 'USDC';
  
  const analyzer = new RouteAnalyzer(connection);
  await analyzer.initialize();
  
  logger.info(`Analyzing all routes: ${fromSymbol} → ${toSymbol}`);
  
  const analysis = await analyzer.analyzeAllRoutes({
    fromMint: SOL_MINT, // Simplified
    toMint: USDC_MINT,
    amount: new BN('1000000000')
  });
  
  logger.info(`Analysis complete:`);
  logger.info(`- Total routes found: ${analysis.totalRoutes}`);
  logger.info(`- Best route hops: ${analysis.bestRoute.hops.length}`);
  logger.info(`- Worst route impact: ${analysis.worstRoute.priceImpact.toFixed(4)}%`);
  logger.info(`- Average fees: ${analysis.averageFees.toFixed(4)}%`);
  
  // Show top 5 routes
  logger.info('Top 5 routes by output:');
  analysis.topRoutes.slice(0, 5).forEach((route, i) => {
    logger.info(`  ${i + 1}. ${route.path.map(p => p.symbol).join(' → ')}`);
    logger.info(`     Output: ${route.expectedOutput.toString()}`);
    logger.info(`     Impact: ${route.priceImpact.toFixed(4)}%`);
  });
}

/**
 * Find arbitrage opportunities
 */
async function findArbitrage(connection: Connection, wallet: Keypair) {
  logger.info('Scanning for arbitrage opportunities...');
  
  const detector = new ArbitrageDetector(connection);
  await detector.initialize();
  
  // Scan major tokens
  const tokens = [SOL_MINT, USDC_MINT, USDT_MINT, ETH_MINT];
  
  for (const token of tokens) {
    const opportunities = await detector.findTriangularArbitrage({
      startToken: token,
      minProfitBps: 5,
      maxHops: 3
    });
    
    if (opportunities.length > 0) {
      logger.info(`Arbitrage opportunities for ${token.toString().slice(0, 8)}...:`);
      opportunities.forEach(op => {
        logger.info(`  Profit: ${op.profitBps / 100}% via ${op.path.map(p => p.symbol).join(' → ')}`);
      });
    }
  }
}

/**
 * Benchmark routing performance
 */
async function benchmarkRouting(connection: Connection, wallet: Keypair) {
  logger.info('Running routing benchmark...');
  
  const router = new MultiHopRouter(connection, wallet);
  await router.initialize();
  
  const testCases = [
    { from: SOL_MINT, to: USDC_MINT, amount: new BN('1000000000') },
    { from: SOL_MINT, to: BONK_MINT, amount: new BN('1000000000') },
    { from: USDC_MINT, to: ETH_MINT, amount: new BN('1000000000') },
    { from: SAMO_MINT, to: USDT_MINT, amount: new BN('1000000000') }
  ];
  
  const results = [];
  
  for (const testCase of testCases) {
    const startTime = Date.now();
    
    const route = await router.findBestRoute({
      fromMint: testCase.from,
      toMint: testCase.to,
      amount: testCase.amount,
      maxHops: 3
    });
    
    const endTime = Date.now();
    const duration = endTime - startTime;
    
    results.push({
      pair: `${testCase.from.toString().slice(0, 8)}... → ${testCase.to.toString().slice(0, 8)}...`,
      found: !!route,
      duration,
      hops: route?.hops.length || 0,
      priceImpact: route?.priceImpact || 0
    });
  }
  
  logger.info('Benchmark results:');
  results.forEach((result, i) => {
    logger.info(`  ${i + 1}. ${result.pair}`);
    logger.info(`     Found: ${result.found ? 'Yes' : 'No'}`);
    logger.info(`     Time: ${result.duration}ms`);
    logger.info(`     Hops: ${result.hops}`);
    logger.info(`     Impact: ${result.priceImpact.toFixed(4)}%`);
  });
  
  const avgTime = results.reduce((sum, r) => sum + r.duration, 0) / results.length;
  logger.info(`Average discovery time: ${avgTime.toFixed(2)}ms`);
}

// Run main function
if (require.main === module) {
  main().catch(console.error);
}

export { MultiHopRouter, RouteAnalyzer, ArbitrageDetector };