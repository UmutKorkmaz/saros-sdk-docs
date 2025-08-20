/**
 * Saros Impermanent Loss Calculator
 * Main entry point for IL analysis and monitoring
 */

import * as dotenv from 'dotenv';
import { ImpermanentLossCalculator } from './ImpermanentLossCalculator';
import { DLMMCalculator } from './DLMMCalculator';
import { FeeAnalyzer } from './FeeAnalyzer';
import { ILMonitor } from './ILMonitor';
import { ReportGenerator } from './ReportGenerator';
import { logger } from './utils/logger';

// Load environment variables
dotenv.config();

/**
 * Main demonstration of IL calculator functionality
 */
async function main() {
  try {
    logger.info('🚀 Starting Saros Impermanent Loss Calculator');
    logger.info('=============================================\n');

    // Initialize components
    const ilCalculator = new ImpermanentLossCalculator();
    const dlmmCalculator = new DLMMCalculator();
    const feeAnalyzer = new FeeAnalyzer();
    const monitor = new ILMonitor();
    const reportGenerator = new ReportGenerator();

    // Example 1: Basic IL Calculation
    logger.info('Example 1: Basic AMM Impermanent Loss');
    logger.info('--------------------------------------');
    
    const basicIL = ilCalculator.calculateIL({
      initialPriceRatio: 1,    // 1 SOL = 50 USDC initially
      currentPriceRatio: 2,    // 1 SOL = 100 USDC now (2x)
      poolType: 'AMM'
    });

    logger.info('Price Change: 2x (100% increase)');
    logger.info(`Impermanent Loss: ${basicIL.impermanentLoss.toFixed(2)}%`);
    logger.info(`Value if Held: $${basicIL.valueIfHeld.toFixed(2)}`);
    logger.info(`Value in Pool: $${basicIL.valueInPool.toFixed(2)}`);
    logger.info(`Loss in USD: $${basicIL.lossInUSD.toFixed(2)}\n`);

    // Example 2: IL at Various Price Points
    logger.info('Example 2: IL at Different Price Points');
    logger.info('----------------------------------------');
    
    const pricePoints = [0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4, 5];
    const ilTable = pricePoints.map(ratio => {
      const result = ilCalculator.calculateIL({
        initialPriceRatio: 1,
        currentPriceRatio: ratio,
        poolType: 'AMM'
      });
      return {
        priceChange: `${((ratio - 1) * 100).toFixed(0)}%`,
        il: `${result.impermanentLoss.toFixed(2)}%`
      };
    });

    console.table(ilTable);

    // Example 3: DLMM Concentrated Liquidity IL
    logger.info('\nExample 3: DLMM Concentrated Liquidity IL');
    logger.info('------------------------------------------');
    
    const dlmmResult = await dlmmCalculator.calculateConcentratedIL({
      lowerPrice: 45,
      upperPrice: 55,
      currentPrice: 52,
      initialPrice: 50,
      liquidity: 10000,
      binStep: 10,
      activeBin: 100
    });

    logger.info('DLMM Position:');
    logger.info(`  Range: $45 - $55`);
    logger.info(`  Initial Price: $50`);
    logger.info(`  Current Price: $52`);
    logger.info(`  Concentration Factor: ${dlmmResult.concentrationFactor.toFixed(2)}x`);
    logger.info(`  IL (Concentrated): ${dlmmResult.impermanentLoss.toFixed(2)}%`);
    logger.info(`  IL (Full Range): ${dlmmResult.fullRangeIL.toFixed(2)}%`);
    logger.info(`  Position In Range: ${dlmmResult.inRange ? 'Yes' : 'No'}\n`);

    // Example 4: Fee Compensation Analysis
    logger.info('Example 4: Fee Compensation Analysis');
    logger.info('-------------------------------------');
    
    const feeAnalysis = await feeAnalyzer.analyzeFeesVsIL({
      pool: {
        address: 'SOL-USDC-POOL',
        token0: 'SOL',
        token1: 'USDC',
        feeRate: 0.3  // 0.3% fee tier
      },
      position: {
        liquidity: 10000,
        duration: 30,        // 30 days
        averageTVL: 1000000,
        dailyVolume: 500000
      },
      currentIL: 5.7  // Current IL percentage
    });

    logger.info('Fee Analysis Results:');
    logger.info(`  Position Value: $${feeAnalysis.positionValue.toFixed(2)}`);
    logger.info(`  Total Fees Earned: $${feeAnalysis.totalFees.toFixed(2)}`);
    logger.info(`  IL in USD: $${feeAnalysis.ilInUSD.toFixed(2)}`);
    logger.info(`  Net Profit: $${feeAnalysis.netProfit.toFixed(2)}`);
    logger.info(`  Fee APR: ${feeAnalysis.feeAPR.toFixed(2)}%`);
    logger.info(`  Days to Break Even: ${feeAnalysis.daysToBreakeven.toFixed(1)}`);
    logger.info(`  IL Compensated: ${feeAnalysis.ilCompensated ? '✅ Yes' : '❌ No'}\n`);

    // Example 5: Optimal Range Finding (DLMM)
    logger.info('Example 5: Optimal Range for DLMM');
    logger.info('----------------------------------');
    
    const optimalRange = await dlmmCalculator.findOptimalRange({
      token0: 'SOL',
      token1: 'USDC',
      currentPrice: 50,
      volatility: 30,        // 30% annual volatility
      targetAPR: 50,         // Target 50% APR
      maxIL: 10,            // Max 10% IL tolerance
      capital: 10000,
      timeHorizon: 30       // 30 days
    });

    logger.info('Optimal Range Suggestion:');
    logger.info(`  Lower Bound: $${optimalRange.lowerPrice.toFixed(2)}`);
    logger.info(`  Upper Bound: $${optimalRange.upperPrice.toFixed(2)}`);
    logger.info(`  Range Width: ${optimalRange.rangeWidth.toFixed(2)}%`);
    logger.info(`  Expected APR: ${optimalRange.expectedAPR.toFixed(2)}%`);
    logger.info(`  Expected IL: ${optimalRange.expectedIL.toFixed(2)}%`);
    logger.info(`  Capital Efficiency: ${optimalRange.capitalEfficiency.toFixed(1)}x`);
    logger.info(`  Probability In Range: ${optimalRange.probabilityInRange.toFixed(2)}%\n`);

    // Example 6: Historical IL Analysis
    logger.info('Example 6: Historical IL Analysis');
    logger.info('----------------------------------');
    
    const historicalAnalysis = await ilCalculator.analyzeHistoricalIL({
      pool: 'SOL-USDC',
      startDate: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000), // 30 days ago
      endDate: new Date(),
      resolution: '1d'
    });

    logger.info('30-Day Historical Analysis:');
    logger.info(`  Average IL: ${historicalAnalysis.averageIL.toFixed(2)}%`);
    logger.info(`  Maximum IL: ${historicalAnalysis.maxIL.toFixed(2)}%`);
    logger.info(`  Minimum IL: ${historicalAnalysis.minIL.toFixed(2)}%`);
    logger.info(`  Days with IL > 5%: ${historicalAnalysis.daysAbove5Percent}`);
    logger.info(`  Volatility: ${historicalAnalysis.volatility.toFixed(2)}%\n`);

    // Example 7: Multi-Pool IL Comparison
    logger.info('Example 7: Multi-Pool IL Comparison');
    logger.info('------------------------------------');
    
    const poolComparison = await ilCalculator.comparePools([
      { 
        name: 'SOL-USDC AMM',
        type: 'AMM',
        feeRate: 0.3,
        tvl: 1000000,
        volume24h: 500000
      },
      {
        name: 'SOL-USDC DLMM',
        type: 'DLMM',
        feeRate: 0.3,
        tvl: 500000,
        volume24h: 300000,
        binStep: 10
      },
      {
        name: 'SOL-USDT AMM',
        type: 'AMM',
        feeRate: 0.1,
        tvl: 750000,
        volume24h: 250000
      }
    ], {
      priceChange: 1.5,  // 50% price increase scenario
      timeHorizon: 30    // 30 days
    });

    logger.info('Pool Comparison (50% price increase):');
    poolComparison.forEach(pool => {
      logger.info(`\n  ${pool.name}:`);
      logger.info(`    IL: ${pool.impermanentLoss.toFixed(2)}%`);
      logger.info(`    Expected Fees: $${pool.expectedFees.toFixed(2)}`);
      logger.info(`    Net Return: $${pool.netReturn.toFixed(2)}`);
      logger.info(`    Risk Score: ${pool.riskScore}/10`);
    });

    // Example 8: IL Mitigation Strategies
    logger.info('\n\nExample 8: IL Mitigation Strategies');
    logger.info('------------------------------------');
    
    const mitigation = ilCalculator.suggestMitigation({
      currentIL: 8.5,
      position: {
        value: 10000,
        poolType: 'AMM',
        duration: 15  // 15 days in position
      },
      marketConditions: {
        volatility: 'high',
        trend: 'bullish',
        volume: 'increasing'
      }
    });

    logger.info('Recommended Strategies:');
    mitigation.strategies.forEach((strategy, index) => {
      logger.info(`  ${index + 1}. ${strategy.name}`);
      logger.info(`     Action: ${strategy.action}`);
      logger.info(`     Expected IL Reduction: ${strategy.expectedReduction}%`);
      logger.info(`     Risk Level: ${strategy.riskLevel}`);
    });

    // Example 9: Real-time Monitoring Setup
    logger.info('\n\nExample 9: Setting Up Real-time IL Monitoring');
    logger.info('----------------------------------------------');
    
    const monitoringConfig = {
      poolAddress: 'POOL_ADDRESS_HERE',
      positionId: 'POSITION_ID_HERE',
      checkInterval: 60000,  // Check every minute
      alerts: {
        warning: 5,     // Warn at 5% IL
        critical: 10,   // Critical at 10% IL
        maxLoss: 500    // Alert if loss exceeds $500
      }
    };

    logger.info('Monitoring Configuration:');
    logger.info(`  Pool: ${monitoringConfig.poolAddress}`);
    logger.info(`  Check Interval: ${monitoringConfig.checkInterval / 1000}s`);
    logger.info(`  Warning Threshold: ${monitoringConfig.alerts.warning}%`);
    logger.info(`  Critical Threshold: ${monitoringConfig.alerts.critical}%`);

    // Start monitoring (commented out for demo)
    // await monitor.startMonitoring(monitoringConfig);

    // Example 10: Generate Comprehensive Report
    logger.info('\n\nExample 10: Generating IL Report');
    logger.info('---------------------------------');
    
    const report = await reportGenerator.generateReport({
      positions: [
        {
          pool: 'SOL-USDC',
          type: 'AMM',
          value: 10000,
          il: 5.7,
          fees: 285,
          duration: 30
        },
        {
          pool: 'RAY-USDC',
          type: 'DLMM',
          value: 5000,
          il: 3.2,
          fees: 180,
          duration: 15
        }
      ],
      format: 'json',
      includeCharts: true
    });

    logger.info('Report Generated:');
    logger.info(`  Total Portfolio Value: $${report.summary.totalValue}`);
    logger.info(`  Average IL: ${report.summary.averageIL}%`);
    logger.info(`  Total Fees Earned: $${report.summary.totalFees}`);
    logger.info(`  Net P&L: $${report.summary.netPnL}`);
    logger.info(`  Report saved to: ${report.filepath}\n`);

    // Display Summary
    logger.info('=========================================');
    logger.info('📊 IL Calculator Session Complete');
    logger.info('=========================================');
    logger.info('Key Takeaways:');
    logger.info('• IL increases with price divergence');
    logger.info('• Concentrated liquidity amplifies IL');
    logger.info('• Fees can compensate for IL over time');
    logger.info('• Regular monitoring is essential');
    logger.info('• Consider volatility when selecting ranges\n');

  } catch (error) {
    logger.error('Fatal error:', error);
    process.exit(1);
  }
}

// Interactive CLI mode
async function interactiveMode() {
  const readline = require('readline');
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
  });

  const calculator = new ImpermanentLossCalculator();

  console.log('\n📊 Saros IL Calculator - Interactive Mode');
  console.log('=========================================\n');

  const question = (query: string): Promise<string> => {
    return new Promise(resolve => rl.question(query, resolve));
  };

  while (true) {
    console.log('\nOptions:');
    console.log('1. Calculate IL for price change');
    console.log('2. Compare AMM vs DLMM');
    console.log('3. Analyze fee compensation');
    console.log('4. Find optimal DLMM range');
    console.log('5. Exit\n');

    const choice = await question('Select option (1-5): ');

    switch (choice) {
      case '1':
        const initial = parseFloat(await question('Initial price ratio: '));
        const current = parseFloat(await question('Current price ratio: '));
        const result = calculator.calculateIL({
          initialPriceRatio: initial,
          currentPriceRatio: current,
          poolType: 'AMM'
        });
        console.log(`\nImpermanent Loss: ${result.impermanentLoss.toFixed(2)}%`);
        break;

      case '2':
        console.log('\nComparing AMM vs DLMM for 50% price increase:');
        const amm = calculator.calculateIL({
          initialPriceRatio: 1,
          currentPriceRatio: 1.5,
          poolType: 'AMM'
        });
        console.log(`AMM IL: ${amm.impermanentLoss.toFixed(2)}%`);
        console.log(`DLMM IL (concentrated): ~${(amm.impermanentLoss * 2).toFixed(2)}%`);
        break;

      case '3':
        const il = parseFloat(await question('Current IL (%): '));
        const volume = parseFloat(await question('Daily volume ($): '));
        const tvl = parseFloat(await question('Pool TVL ($): '));
        const feeRate = parseFloat(await question('Fee rate (%): '));
        
        const dailyFees = (volume * feeRate / 100);
        const shareOfFees = dailyFees * (10000 / tvl);  // Assuming $10k position
        const daysToBreakeven = (il * 100) / shareOfFees;
        
        console.log(`\nDaily fees earned: $${shareOfFees.toFixed(2)}`);
        console.log(`Days to break even: ${daysToBreakeven.toFixed(1)}`);
        break;

      case '4':
        const price = parseFloat(await question('Current price: '));
        const volatility = parseFloat(await question('Expected volatility (%): '));
        const range = volatility / 2;
        
        console.log(`\nSuggested range:`);
        console.log(`Lower: $${(price * (1 - range/100)).toFixed(2)}`);
        console.log(`Upper: $${(price * (1 + range/100)).toFixed(2)}`);
        break;

      case '5':
        rl.close();
        process.exit(0);

      default:
        console.log('Invalid option');
    }
  }
}

// Handle command line arguments
const args = process.argv.slice(2);
if (args.includes('--interactive') || args.includes('-i')) {
  interactiveMode().catch(console.error);
} else {
  main().catch(console.error);
}

export { 
  ImpermanentLossCalculator, 
  DLMMCalculator, 
  FeeAnalyzer, 
  ILMonitor,
  ReportGenerator
};