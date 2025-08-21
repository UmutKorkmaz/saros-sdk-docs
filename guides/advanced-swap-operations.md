# TypeScript SDK: Swap Operations

Complete guide to implementing token swaps using the Saros TypeScript SDK with advanced features, error handling, and optimization techniques.

## Overview

The Saros TypeScript SDK provides powerful swap functionality through the AMM protocol with:
- ✅ Multi-hop routing for best prices
- ✅ Slippage protection and price impact calculation
- ✅ Real-time quote updates
- ✅ Transaction simulation and validation
- ✅ Comprehensive error handling

## Core Swap Functions

### Primary Swap Functions

```typescript
// Core swap functions from @saros-finance/sdk
import {
  getSwapAmountSaros,    // Calculate swap amounts with slippage
  swapSaros,             // Execute token swap
  genConnectionSolana,   // Create optimized connection
  createPoolSaros,       // Create new trading pools
  getPoolInfo           // Retrieve pool information
} from '@saros-finance/sdk';
```

### Supporting Functions

```typescript
import {
  getTokenAccountBalance,  // Check token balances
  createTokenAccount,      // Create associated token accounts
  getOptimalRoute,         // Find best swap route
  calculatePriceImpact,    // Estimate price impact
  estimateGasFees         // Calculate transaction costs
} from '@saros-finance/sdk';
```

## Basic Swap Implementation

### Step 1: Setup and Configuration

```typescript
import { Connection, PublicKey, Keypair } from '@solana/web3.js';
import { getSwapAmountSaros, swapSaros } from '@saros-finance/sdk';

interface SwapConfig {
  connection: Connection;
  wallet: Keypair;
  swapProgram: PublicKey;
  slippageTolerance: number;
  priorityFee?: number;
}

class SarosSwapper {
  private config: SwapConfig;

  constructor(config: SwapConfig) {
    this.config = config;
  }

  async initialize(): Promise<void> {
    // Verify connection and wallet
    const balance = await this.config.connection.getBalance(
      this.config.wallet.publicKey
    );
    
    if (balance < 0.01 * 1e9) {
      throw new Error('Insufficient SOL for transaction fees');
    }
  }
}
```

### Step 2: Quote Calculation

```typescript
interface SwapQuote {
  amountIn: number;
  amountOut: number;
  amountOutWithSlippage: number;
  priceImpact: number;
  route: PublicKey[];
  minimumReceived: number;
  fee: number;
  exchangeRate: number;
}

class SarosSwapper {
  async getQuote(
    fromMint: PublicKey,
    toMint: PublicKey,
    amount: number,
    slippagePercent?: number
  ): Promise<SwapQuote> {
    try {
      const slippage = slippagePercent || this.config.slippageTolerance;
      
      console.log(`🔍 Getting quote: ${amount} ${fromMint.toString().slice(0, 8)}... → ${toMint.toString().slice(0, 8)}...`);

      // Get pool information for routing
      const poolParams = await this.getPoolParameters(fromMint, toMint);
      
      // Calculate swap amounts
      const swapEstimate = await getSwapAmountSaros(
        this.config.connection,
        fromMint.toString(),
        toMint.toString(),
        amount,
        slippage,
        poolParams
      );

      // Calculate additional metrics
      const exchangeRate = swapEstimate.amountOut / amount;
      const minimumReceived = swapEstimate.amountOutWithSlippage;
      const fee = amount * 0.0025; // 0.25% fee estimate

      console.log(`💱 Quote: ${amount} → ${swapEstimate.amountOut.toFixed(6)}`);
      console.log(`📊 Price impact: ${swapEstimate.priceImpact.toFixed(3)}%`);
      console.log(`🛡️ Minimum received: ${minimumReceived.toFixed(6)}`);

      return {
        amountIn: amount,
        amountOut: swapEstimate.amountOut,
        amountOutWithSlippage: swapEstimate.amountOutWithSlippage,
        priceImpact: swapEstimate.priceImpact,
        route: [fromMint, toMint], // Simplified - multi-hop routes would have more entries
        minimumReceived,
        fee,
        exchangeRate
      };

    } catch (error: any) {
      console.error('❌ Quote calculation failed:', error.message);
      throw new Error(`Failed to get swap quote: ${error.message}`);
    }
  }

  private async getPoolParameters(
    fromMint: PublicKey, 
    toMint: PublicKey
  ): Promise<any> {
    // In production, this would query the actual pool
    // For this example, we return mock parameters
    return {
      tokenAMint: fromMint,
      tokenBMint: toMint,
      poolAddress: new PublicKey('POOL_ADDRESS_PLACEHOLDER'),
      fee: 0.0025, // 0.25%
      // Additional pool parameters...
    };
  }
}
```

### Step 3: Execute Swap with Validation

```typescript
interface SwapResult {
  signature: string;
  amountIn: number;
  amountOut: number;
  actualAmountOut?: number;
  priceImpact: number;
  gasUsed: number;
  success: boolean;
  error?: string;
  explorerUrl: string;
}

class SarosSwapper {
  async executeSwap(
    fromMint: PublicKey,
    toMint: PublicKey,
    amount: number,
    minimumAmountOut?: number,
    options?: {
      slippage?: number;
      deadline?: number;
      priorityFee?: number;
    }
  ): Promise<SwapResult> {
    try {
      console.log('⚡ Executing swap...');

      // Step 1: Get fresh quote
      const quote = await this.getQuote(
        fromMint, 
        toMint, 
        amount, 
        options?.slippage
      );

      // Step 2: Validate minimum amount out
      const minAmountOut = minimumAmountOut || quote.amountOutWithSlippage;
      
      if (quote.amountOut < minAmountOut) {
        throw new Error(
          `Insufficient output amount: ${quote.amountOut} < ${minAmountOut}`
        );
      }

      // Step 3: Prepare token accounts
      const { fromTokenAccount, toTokenAccount } = await this.prepareTokenAccounts(
        fromMint,
        toMint
      );

      // Step 4: Validate balances
      await this.validateBalances(fromTokenAccount, amount);

      // Step 5: Calculate transaction amounts in token units
      const fromTokenDecimals = await this.getTokenDecimals(fromMint);
      const toTokenDecimals = await this.getTokenDecimals(toMint);
      
      const amountInTokenUnits = amount * Math.pow(10, fromTokenDecimals);
      const minAmountOutTokenUnits = minAmountOut * Math.pow(10, toTokenDecimals);

      // Step 6: Execute the swap
      console.log('📝 Submitting swap transaction...');
      
      const swapResult = await swapSaros(
        this.config.connection,
        fromTokenAccount,
        toTokenAccount,
        amountInTokenUnits,
        minAmountOutTokenUnits,
        null, // No referrer
        quote.route[0], // Pool address (simplified)
        this.config.swapProgram,
        this.config.wallet.publicKey,
        fromMint,
        toMint
      );

      // Step 7: Confirm transaction
      console.log('⏳ Confirming transaction...');
      await this.config.connection.confirmTransaction(
        swapResult.hash,
        'confirmed'
      );

      // Step 8: Verify actual amounts received
      const actualAmountOut = await this.getActualAmountReceived(
        toTokenAccount,
        toTokenDecimals
      );

      console.log('✅ Swap completed successfully!');
      console.log(`📈 Amount received: ${actualAmountOut?.toFixed(6) || 'N/A'}`);

      const explorerUrl = `https://explorer.solana.com/tx/${swapResult.hash}?cluster=devnet`;

      return {
        signature: swapResult.hash,
        amountIn: amount,
        amountOut: quote.amountOut,
        actualAmountOut,
        priceImpact: quote.priceImpact,
        gasUsed: 0.005, // Estimated
        success: true,
        explorerUrl
      };

    } catch (error: any) {
      console.error('❌ Swap execution failed:', error.message);
      
      return {
        signature: '',
        amountIn: amount,
        amountOut: 0,
        priceImpact: 0,
        gasUsed: 0,
        success: false,
        error: error.message,
        explorerUrl: ''
      };
    }
  }

  private async prepareTokenAccounts(
    fromMint: PublicKey,
    toMint: PublicKey
  ): Promise<{ fromTokenAccount: PublicKey; toTokenAccount: PublicKey }> {
    const { getAssociatedTokenAddress } = await import('@solana/spl-token');

    const fromTokenAccount = await getAssociatedTokenAddress(
      fromMint,
      this.config.wallet.publicKey
    );

    const toTokenAccount = await getAssociatedTokenAddress(
      toMint,
      this.config.wallet.publicKey
    );

    // Verify accounts exist or create them
    await this.ensureTokenAccountExists(fromTokenAccount, fromMint);
    await this.ensureTokenAccountExists(toTokenAccount, toMint);

    return { fromTokenAccount, toTokenAccount };
  }

  private async ensureTokenAccountExists(
    tokenAccount: PublicKey,
    mint: PublicKey
  ): Promise<void> {
    try {
      await this.config.connection.getAccountInfo(tokenAccount);
    } catch (error) {
      console.log(`Creating token account for ${mint.toString().slice(0, 8)}...`);
      // Create associated token account if it doesn't exist
      // Implementation would go here
    }
  }

  private async validateBalances(
    fromTokenAccount: PublicKey,
    requiredAmount: number
  ): Promise<void> {
    const { getAccount } = await import('@solana/spl-token');
    
    try {
      const account = await getAccount(this.config.connection, fromTokenAccount);
      const balance = Number(account.amount) / 1e6; // Assume 6 decimals
      
      if (balance < requiredAmount) {
        throw new Error(
          `Insufficient balance: ${balance} < ${requiredAmount}`
        );
      }
    } catch (error: any) {
      throw new Error(`Balance validation failed: ${error.message}`);
    }
  }

  private async getTokenDecimals(mint: PublicKey): Promise<number> {
    // In production, fetch from mint account
    // For this example, return common decimals
    const commonMints = {
      'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v': 6, // USDC
      'So11111111111111111111111111111111111111112': 9,  // SOL
      'C98A4nkJXhpVZNAZdHUA95RpTF3T4whtQubL3YobiUX9': 9   // C98
    };
    
    return commonMints[mint.toString()] || 9;
  }

  private async getActualAmountReceived(
    toTokenAccount: PublicKey,
    decimals: number
  ): Promise<number | undefined> {
    try {
      const { getAccount } = await import('@solana/spl-token');
      const account = await getAccount(this.config.connection, toTokenAccount);
      return Number(account.amount) / Math.pow(10, decimals);
    } catch {
      return undefined;
    }
  }
}
```

## Advanced Swap Features

### Multi-Hop Routing

```typescript
interface RouteStep {
  poolAddress: PublicKey;
  tokenIn: PublicKey;
  tokenOut: PublicKey;
  amountIn: number;
  amountOut: number;
  fee: number;
}

interface SwapRoute {
  steps: RouteStep[];
  totalAmountIn: number;
  totalAmountOut: number;
  totalFee: number;
  priceImpact: number;
}

class SarosSwapper {
  async findOptimalRoute(
    fromMint: PublicKey,
    toMint: PublicKey,
    amount: number
  ): Promise<SwapRoute> {
    console.log('🔍 Finding optimal route...');

    // Step 1: Get direct route
    const directRoute = await this.getDirectRoute(fromMint, toMint, amount);

    // Step 2: Get multi-hop routes
    const multiHopRoutes = await this.getMultiHopRoutes(fromMint, toMint, amount);

    // Step 3: Compare routes and select best
    const allRoutes = [directRoute, ...multiHopRoutes];
    const bestRoute = allRoutes.reduce((best, current) => 
      current.totalAmountOut > best.totalAmountOut ? current : best
    );

    console.log(`🎯 Best route: ${bestRoute.steps.length} hops`);
    console.log(`💰 Output: ${bestRoute.totalAmountOut.toFixed(6)}`);

    return bestRoute;
  }

  private async getDirectRoute(
    fromMint: PublicKey,
    toMint: PublicKey,
    amount: number
  ): Promise<SwapRoute> {
    const quote = await this.getQuote(fromMint, toMint, amount);
    
    return {
      steps: [{
        poolAddress: new PublicKey('DIRECT_POOL_ADDRESS'),
        tokenIn: fromMint,
        tokenOut: toMint,
        amountIn: amount,
        amountOut: quote.amountOut,
        fee: quote.fee
      }],
      totalAmountIn: amount,
      totalAmountOut: quote.amountOut,
      totalFee: quote.fee,
      priceImpact: quote.priceImpact
    };
  }

  private async getMultiHopRoutes(
    fromMint: PublicKey,
    toMint: PublicKey,
    amount: number
  ): Promise<SwapRoute[]> {
    // Common intermediate tokens for routing
    const intermediateTokens = [
      new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'), // USDC
      new PublicKey('So11111111111111111111111111111111111111112'),  // SOL
    ];

    const routes: SwapRoute[] = [];

    for (const intermediate of intermediateTokens) {
      if (intermediate.equals(fromMint) || intermediate.equals(toMint)) {
        continue;
      }

      try {
        // Step 1: from -> intermediate
        const step1Quote = await this.getQuote(fromMint, intermediate, amount);
        
        // Step 2: intermediate -> to
        const step2Quote = await this.getQuote(
          intermediate, 
          toMint, 
          step1Quote.amountOut
        );

        const route: SwapRoute = {
          steps: [
            {
              poolAddress: new PublicKey('POOL_1_ADDRESS'),
              tokenIn: fromMint,
              tokenOut: intermediate,
              amountIn: amount,
              amountOut: step1Quote.amountOut,
              fee: step1Quote.fee
            },
            {
              poolAddress: new PublicKey('POOL_2_ADDRESS'),
              tokenIn: intermediate,
              tokenOut: toMint,
              amountIn: step1Quote.amountOut,
              amountOut: step2Quote.amountOut,
              fee: step2Quote.fee
            }
          ],
          totalAmountIn: amount,
          totalAmountOut: step2Quote.amountOut,
          totalFee: step1Quote.fee + step2Quote.fee,
          priceImpact: step1Quote.priceImpact + step2Quote.priceImpact
        };

        routes.push(route);

      } catch (error) {
        // Route not available
        console.log(`Route via ${intermediate.toString().slice(0, 8)}... not available`);
      }
    }

    return routes;
  }

  async executeMultiHopSwap(route: SwapRoute): Promise<SwapResult> {
    console.log(`⚡ Executing ${route.steps.length}-hop swap...`);

    try {
      let currentAmount = route.totalAmountIn;
      const signatures: string[] = [];

      for (let i = 0; i < route.steps.length; i++) {
        const step = route.steps[i];
        console.log(`Step ${i + 1}: ${currentAmount.toFixed(6)} ${step.tokenIn.toString().slice(0, 8)}... → ${step.tokenOut.toString().slice(0, 8)}...`);

        const stepResult = await this.executeSwap(
          step.tokenIn,
          step.tokenOut,
          currentAmount,
          step.amountOut * 0.99 // 1% slippage buffer per step
        );

        if (!stepResult.success) {
          throw new Error(`Step ${i + 1} failed: ${stepResult.error}`);
        }

        signatures.push(stepResult.signature);
        currentAmount = stepResult.actualAmountOut || step.amountOut;
      }

      console.log('✅ Multi-hop swap completed!');
      
      return {
        signature: signatures.join(','),
        amountIn: route.totalAmountIn,
        amountOut: route.totalAmountOut,
        actualAmountOut: currentAmount,
        priceImpact: route.priceImpact,
        gasUsed: signatures.length * 0.005,
        success: true,
        explorerUrl: `https://explorer.solana.com/tx/${signatures[0]}?cluster=devnet`
      };

    } catch (error: any) {
      console.error('❌ Multi-hop swap failed:', error.message);
      
      return {
        signature: '',
        amountIn: route.totalAmountIn,
        amountOut: 0,
        priceImpact: 0,
        gasUsed: 0,
        success: false,
        error: error.message,
        explorerUrl: ''
      };
    }
  }
}
```

### Real-Time Price Monitoring

```typescript
interface PriceAlert {
  fromMint: PublicKey;
  toMint: PublicKey;
  targetPrice: number;
  threshold: number; // Percentage
  callback: (currentPrice: number) => void;
}

class PriceMonitor {
  private alerts: Map<string, PriceAlert> = new Map();
  private swapper: SarosSwapper;
  private intervalId?: NodeJS.Timeout;

  constructor(swapper: SarosSwapper) {
    this.swapper = swapper;
  }

  addPriceAlert(alert: PriceAlert): string {
    const alertId = `${alert.fromMint.toString()}-${alert.toMint.toString()}-${Date.now()}`;
    this.alerts.set(alertId, alert);
    
    console.log(`📢 Price alert added: ${alert.targetPrice} ±${alert.threshold}%`);
    
    if (!this.intervalId) {
      this.startMonitoring();
    }
    
    return alertId;
  }

  removePriceAlert(alertId: string): void {
    this.alerts.delete(alertId);
    
    if (this.alerts.size === 0 && this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = undefined;
      console.log('🔇 Price monitoring stopped');
    }
  }

  private startMonitoring(): void {
    console.log('👁️ Starting price monitoring...');
    
    this.intervalId = setInterval(async () => {
      for (const [alertId, alert] of this.alerts) {
        try {
          const quote = await this.swapper.getQuote(
            alert.fromMint,
            alert.toMint,
            1 // Standard unit for price checking
          );

          const currentPrice = quote.exchangeRate;
          const priceDifference = Math.abs(currentPrice - alert.targetPrice) / alert.targetPrice;

          if (priceDifference <= alert.threshold / 100) {
            console.log(`🎯 Price alert triggered! Current: ${currentPrice}, Target: ${alert.targetPrice}`);
            alert.callback(currentPrice);
          }

        } catch (error) {
          console.error(`Price check failed for alert ${alertId}:`, error);
        }
      }
    }, 10000); // Check every 10 seconds
  }

  async getCurrentPrices(pairs: Array<{ fromMint: PublicKey; toMint: PublicKey }>): Promise<
    Array<{ pair: string; price: number; change24h?: number }>
  > {
    const prices = [];

    for (const pair of pairs) {
      try {
        const quote = await this.swapper.getQuote(pair.fromMint, pair.toMint, 1);
        
        prices.push({
          pair: `${pair.fromMint.toString().slice(0, 8)}.../${pair.toMint.toString().slice(0, 8)}...`,
          price: quote.exchangeRate,
          change24h: undefined // Would need historical data
        });

      } catch (error) {
        console.error(`Failed to get price for ${pair.fromMint.toString()}/${pair.toMint.toString()}`);
      }
    }

    return prices;
  }
}
```

## Error Handling and Recovery

### Comprehensive Error Types

```typescript
enum SwapErrorType {
  INSUFFICIENT_BALANCE = 'INSUFFICIENT_BALANCE',
  SLIPPAGE_EXCEEDED = 'SLIPPAGE_EXCEEDED',
  POOL_NOT_FOUND = 'POOL_NOT_FOUND',
  TRANSACTION_FAILED = 'TRANSACTION_FAILED',
  NETWORK_ERROR = 'NETWORK_ERROR',
  INVALID_AMOUNT = 'INVALID_AMOUNT',
  TOKEN_ACCOUNT_ERROR = 'TOKEN_ACCOUNT_ERROR',
  PROGRAM_ERROR = 'PROGRAM_ERROR'
}

class SwapError extends Error {
  public readonly type: SwapErrorType;
  public readonly details: any;
  public readonly retryable: boolean;

  constructor(
    type: SwapErrorType,
    message: string,
    details?: any,
    retryable: boolean = false
  ) {
    super(message);
    this.type = type;
    this.details = details;
    this.retryable = retryable;
    this.name = 'SwapError';
  }
}

class SwapErrorHandler {
  static handleError(error: any): SwapError {
    // Parse Solana transaction errors
    if (error.code === 6000) {
      return new SwapError(
        SwapErrorType.SLIPPAGE_EXCEEDED,
        'Slippage tolerance exceeded',
        error,
        true
      );
    }

    if (error.message?.includes('insufficient')) {
      return new SwapError(
        SwapErrorType.INSUFFICIENT_BALANCE,
        'Insufficient token balance',
        error,
        false
      );
    }

    if (error.message?.includes('pool')) {
      return new SwapError(
        SwapErrorType.POOL_NOT_FOUND,
        'Trading pool not found or inactive',
        error,
        false
      );
    }

    // Network errors are usually retryable
    if (error.code === 'NETWORK_ERROR' || error.message?.includes('timeout')) {
      return new SwapError(
        SwapErrorType.NETWORK_ERROR,
        'Network connection error',
        error,
        true
      );
    }

    // Generic error
    return new SwapError(
      SwapErrorType.PROGRAM_ERROR,
      error.message || 'Unknown swap error',
      error,
      false
    );
  }

  static async retrySwap<T>(
    operation: () => Promise<T>,
    maxRetries: number = 3,
    delayMs: number = 1000
  ): Promise<T> {
    let lastError: SwapError;

    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        return await operation();
      } catch (error) {
        lastError = this.handleError(error);
        
        console.log(`❌ Attempt ${attempt} failed: ${lastError.message}`);

        if (!lastError.retryable || attempt === maxRetries) {
          throw lastError;
        }

        console.log(`⏳ Retrying in ${delayMs}ms...`);
        await new Promise(resolve => setTimeout(resolve, delayMs));
        delayMs *= 2; // Exponential backoff
      }
    }

    throw lastError!;
  }
}
```

### Recovery Strategies

```typescript
class SwapRecoveryService {
  private swapper: SarosSwapper;

  constructor(swapper: SarosSwapper) {
    this.swapper = swapper;
  }

  async executeSwapWithRecovery(
    fromMint: PublicKey,
    toMint: PublicKey,
    amount: number,
    options?: {
      maxSlippage?: number;
      maxRetries?: number;
      enableRecovery?: boolean;
    }
  ): Promise<SwapResult> {
    const maxSlippage = options?.maxSlippage || 5; // 5% max
    const maxRetries = options?.maxRetries || 3;
    const enableRecovery = options?.enableRecovery ?? true;

    let currentSlippage = this.swapper.config.slippageTolerance;

    return SwapErrorHandler.retrySwap(async () => {
      try {
        return await this.swapper.executeSwap(
          fromMint,
          toMint,
          amount,
          undefined,
          { slippage: currentSlippage }
        );

      } catch (error) {
        const swapError = SwapErrorHandler.handleError(error);

        if (enableRecovery && swapError.type === SwapErrorType.SLIPPAGE_EXCEEDED) {
          // Increase slippage tolerance
          currentSlippage = Math.min(currentSlippage + 0.5, maxSlippage);
          console.log(`🔧 Increasing slippage tolerance to ${currentSlippage}%`);
          
          if (currentSlippage < maxSlippage) {
            throw swapError; // Retry with higher slippage
          }
        }

        if (enableRecovery && swapError.type === SwapErrorType.POOL_NOT_FOUND) {
          // Try finding alternative route
          console.log('🔍 Searching for alternative route...');
          const route = await this.swapper.findOptimalRoute(fromMint, toMint, amount);
          return await this.swapper.executeMultiHopSwap(route);
        }

        throw swapError;
      }
    }, maxRetries);
  }

  async simulateSwap(
    fromMint: PublicKey,
    toMint: PublicKey,
    amount: number
  ): Promise<{
    canExecute: boolean;
    estimatedResult?: SwapQuote;
    blockers: string[];
    recommendations: string[];
  }> {
    const blockers: string[] = [];
    const recommendations: string[] = [];

    try {
      // Check balance
      const fromTokenAccount = await getAssociatedTokenAddress(
        fromMint,
        this.swapper.config.wallet.publicKey
      );

      const balance = await this.getTokenBalance(fromTokenAccount);
      if (balance < amount) {
        blockers.push(`Insufficient balance: ${balance} < ${amount}`);
      }

      // Check pool exists
      const quote = await this.swapper.getQuote(fromMint, toMint, amount);

      // Check price impact
      if (quote.priceImpact > 5) {
        recommendations.push(`High price impact: ${quote.priceImpact.toFixed(2)}%. Consider smaller amount.`);
      }

      // Check slippage
      if (quote.priceImpact > 2) {
        recommendations.push('Consider increasing slippage tolerance');
      }

      return {
        canExecute: blockers.length === 0,
        estimatedResult: quote,
        blockers,
        recommendations
      };

    } catch (error) {
      blockers.push(`Simulation failed: ${error.message}`);
      
      return {
        canExecute: false,
        blockers,
        recommendations
      };
    }
  }

  private async getTokenBalance(tokenAccount: PublicKey): Promise<number> {
    try {
      const { getAccount } = await import('@solana/spl-token');
      const account = await getAccount(this.swapper.config.connection, tokenAccount);
      return Number(account.amount) / 1e6; // Assume 6 decimals
    } catch {
      return 0;
    }
  }
}
```

## Usage Examples

### Complete Implementation Example

```typescript
async function demonstrateSwapOperations() {
  console.log('🚀 Saros Swap Operations Demo');
  console.log('=============================\n');

  // Initialize swapper
  const config: SwapConfig = {
    connection: new Connection('https://api.devnet.solana.com'),
    wallet: Keypair.fromSecretKey(bs58.decode(process.env.WALLET_PRIVATE_KEY!)),
    swapProgram: new PublicKey('SSwapUtytfBdBn1b9NUGG6foMVPtcWgpRU32HToDUZr'),
    slippageTolerance: 0.5
  };

  const swapper = new SarosSwapper(config);
  await swapper.initialize();

  // Setup price monitoring
  const priceMonitor = new PriceMonitor(swapper);
  
  // Setup recovery service
  const recoveryService = new SwapRecoveryService(swapper);

  try {
    // 1. Get quote
    console.log('1️⃣ Getting swap quote...');
    const quote = await swapper.getQuote(
      new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'), // USDC
      new PublicKey('So11111111111111111111111111111111111111112'),  // SOL
      100 // 100 USDC
    );

    console.log(`💱 Quote: 100 USDC → ${quote.amountOut.toFixed(6)} SOL`);
    console.log(`📊 Price impact: ${quote.priceImpact.toFixed(3)}%`);

    // 2. Simulate swap
    console.log('\n2️⃣ Simulating swap...');
    const simulation = await recoveryService.simulateSwap(
      new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
      new PublicKey('So11111111111111111111111111111111111111112'),
      100
    );

    if (simulation.canExecute) {
      console.log('✅ Swap can be executed');
    } else {
      console.log('❌ Blockers:', simulation.blockers);
    }

    if (simulation.recommendations.length > 0) {
      console.log('💡 Recommendations:', simulation.recommendations);
    }

    // 3. Execute swap with recovery
    console.log('\n3️⃣ Executing swap with recovery...');
    const result = await recoveryService.executeSwapWithRecovery(
      new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
      new PublicKey('So11111111111111111111111111111111111111112'),
      100,
      {
        maxSlippage: 2.0,
        maxRetries: 3,
        enableRecovery: true
      }
    );

    if (result.success) {
      console.log('✅ Swap completed successfully!');
      console.log(`🔗 Explorer: ${result.explorerUrl}`);
    } else {
      console.log('❌ Swap failed:', result.error);
    }

    // 4. Set up price alert
    console.log('\n4️⃣ Setting up price alert...');
    priceMonitor.addPriceAlert({
      fromMint: new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
      toMint: new PublicKey('So11111111111111111111111111111111111111112'),
      targetPrice: quote.exchangeRate,
      threshold: 1, // 1%
      callback: (currentPrice) => {
        console.log(`🚨 Price alert! Current price: ${currentPrice}`);
        // Execute swap or notification
      }
    });

  } catch (error) {
    console.error('Demo failed:', error);
  }
}

// Run the demo
demonstrateSwapOperations();
```

## Best Practices

### Performance Optimization
- ✅ **Cache pool data**: Avoid repeated pool queries
- ✅ **Batch operations**: Group multiple swaps when possible
- ✅ **Use connection pooling**: Maintain persistent connections
- ✅ **Implement retries**: Handle network hiccups gracefully

### Security Considerations
- ✅ **Validate all inputs**: Check amounts, addresses, slippage
- ✅ **Use minimum amount out**: Protect against MEV attacks
- ✅ **Monitor price impact**: Alert on suspicious movements
- ✅ **Implement timeouts**: Prevent hanging transactions

### User Experience
- ✅ **Real-time quotes**: Update prices frequently
- ✅ **Clear error messages**: Help users understand issues
- ✅ **Progress indicators**: Show transaction status
- ✅ **Simulation mode**: Let users preview swaps

## Next Steps

- [Pool Management Guide](./pool-management.md)
- [Advanced Error Handling](../../best-practices/error-handling.md)
- [Performance Optimization](../../best-practices/performance.md)
- [Complete Swap Examples](../../code-examples/typescript/01-swap-with-slippage/)

## Resources

- [Saros SDK Reference](../../api-reference/typescript-sdk/)
- [Solana Web3.js Documentation](https://solana-labs.github.io/solana-web3.js/)
- [SPL Token Documentation](https://spl.solana.com/token)
- [Community Examples](https://github.com/saros-finance/sdk-examples)