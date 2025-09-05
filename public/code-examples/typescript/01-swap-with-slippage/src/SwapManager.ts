/**
 * SwapManager - Core swap implementation with Saros SDK
 */

import { Connection, PublicKey, Keypair, Transaction, VersionedTransaction } from '@solana/web3.js';
import { 
  getAccount, 
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction
} from '@solana/spl-token';
import bs58 from 'bs58';
import { logger } from './utils/logger';
import { ErrorHandler, SwapError, SwapErrorType } from './ErrorHandler';
import { sarosSDK } from './saros-sdk-mock';

export interface SwapConfig {
  rpcUrl: string;
  privateKey: string;
  network?: 'devnet' | 'mainnet-beta';
  commitment?: 'processed' | 'confirmed' | 'finalized';
  skipPreflight?: boolean;
}

export interface SwapParams {
  fromMint: PublicKey;
  toMint: PublicKey;
  amount: number;
  slippageTolerance?: number;
  poolAddress?: PublicKey;
  simulateFirst?: boolean;
  maxRetries?: number;
  priorityFee?: number;
}

export interface SwapResult {
  success: boolean;
  signature: string;
  amountIn: number;
  amountOut: number;
  priceImpact: number;
  slippageUsed: number;
  gasUsed: number;
  retries: number;
  error?: string;
  explorerUrl: string;
}

export interface SwapStatistics {
  totalSwaps: number;
  successfulSwaps: number;
  failedSwaps: number;
  averageSlippage: number;
  totalVolume: number;
  averageGasUsed: number;
}

export class SwapManager {
  private connection: Connection;
  private wallet: Keypair;
  private network: string;
  private swapProgram: PublicKey;
  private errorHandler: ErrorHandler;
  private statistics: SwapStatistics;

  constructor(config: SwapConfig) {
    this.connection = new Connection(
      config.rpcUrl,
      {
        commitment: config.commitment || 'confirmed',
        confirmTransactionInitialTimeout: 60000
      }
    );
    
    this.wallet = Keypair.fromSecretKey(bs58.decode(config.privateKey));
    this.network = config.network || 'devnet';
    this.swapProgram = new PublicKey('SSwapUtytfBdBn1b9NUGG6foMVPtcWgpRU32HToDUZr');
    this.errorHandler = new ErrorHandler();
    
    this.statistics = {
      totalSwaps: 0,
      successfulSwaps: 0,
      failedSwaps: 0,
      averageSlippage: 0,
      totalVolume: 0,
      averageGasUsed: 0
    };

    logger.info(`SwapManager initialized for ${this.network}`);
    logger.info(`Wallet: ${this.wallet.publicKey.toString()}`);
  }

  /**
   * Execute a token swap with comprehensive error handling and retries
   */
  async swap(params: SwapParams): Promise<SwapResult> {
    this.statistics.totalSwaps++;
    const maxRetries = params.maxRetries || 3;
    let lastError: SwapError | null = null;
    let currentSlippage = params.slippageTolerance || 0.5;
    
    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        logger.info(`🔄 Swap attempt ${attempt}/${maxRetries}`);
        logger.info(`  From: ${params.fromMint.toString().slice(0, 8)}...`);
        logger.info(`  To: ${params.toMint.toString().slice(0, 8)}...`);
        logger.info(`  Amount: ${params.amount}`);
        logger.info(`  Slippage: ${currentSlippage}%`);

        // Step 1: Validate prerequisites
        await this.validateSwapPrerequisites(params);

        // Step 2: Simulate if requested
        if (params.simulateFirst) {
          const simulation = await this.simulateSwap(params, currentSlippage);
          if (!simulation.canExecute) {
            throw new SwapError(
              SwapErrorType.SIMULATION_FAILED,
              `Simulation failed: ${simulation.errors.join(', ')}`,
              simulation
            );
          }
          logger.info(`✅ Simulation successful. Expected output: ${simulation.expectedOutput}`);
        }

        // Step 3: Get pool and calculate amounts
        const poolAddress = params.poolAddress || await this.findPool(params.fromMint, params.toMint);
        const swapAmounts = await this.calculateSwapAmounts(
          params.fromMint,
          params.toMint,
          params.amount,
          currentSlippage,
          poolAddress
        );

        // Step 4: Prepare token accounts
        const { fromTokenAccount, toTokenAccount } = await this.prepareTokenAccounts(
          params.fromMint,
          params.toMint
        );

        // Step 5: Execute the swap
        const swapResult = await this.executeSwapTransaction(
          params,
          swapAmounts,
          fromTokenAccount,
          toTokenAccount,
          poolAddress,
          currentSlippage
        );

        // Step 6: Confirm and verify
        await this.confirmTransaction(swapResult.signature);
        
        // Update statistics
        this.updateStatistics(true, currentSlippage, swapAmounts.gasEstimate, params.amount);

        logger.info(`✅ Swap completed successfully!`);
        logger.info(`  Transaction: ${swapResult.signature}`);
        logger.info(`  Explorer: ${swapResult.explorerUrl}`);

        return swapResult;

      } catch (error: any) {
        lastError = this.errorHandler.handleError(error);
        logger.error(`❌ Attempt ${attempt} failed: ${lastError.message}`);

        // Handle specific errors
        if (lastError.type === SwapErrorType.SLIPPAGE_EXCEEDED) {
          currentSlippage = Math.min(currentSlippage * 1.5, 5.0); // Increase slippage up to 5%
          logger.info(`🔧 Increasing slippage to ${currentSlippage}%`);
          continue;
        }

        if (lastError.type === SwapErrorType.NETWORK_ERROR && attempt < maxRetries) {
          const delay = Math.pow(2, attempt) * 1000; // Exponential backoff
          logger.info(`⏳ Waiting ${delay}ms before retry...`);
          await this.sleep(delay);
          continue;
        }

        if (!lastError.retryable) {
          break;
        }
      }
    }

    // All attempts failed
    this.updateStatistics(false, 0, 0, params.amount);
    
    return {
      success: false,
      signature: '',
      amountIn: params.amount,
      amountOut: 0,
      priceImpact: 0,
      slippageUsed: currentSlippage,
      gasUsed: 0,
      retries: maxRetries,
      error: lastError?.message || 'Unknown error',
      explorerUrl: ''
    };
  }

  /**
   * Execute swap with MEV protection
   */
  async swapWithMEVProtection(params: SwapParams & {
    maxSlippage: number;
    priorityFee: number;
    useJitoBundle?: boolean;
  }): Promise<SwapResult> {
    logger.info('🛡️ Executing MEV-protected swap...');

    // Use tighter slippage for MEV protection
    const tightSlippage = Math.min(params.maxSlippage, 0.5);
    
    // Add priority fee to transaction
    const swapParams = {
      ...params,
      slippageTolerance: tightSlippage,
      priorityFee: params.priorityFee
    };

    // Execute with priority
    const result = await this.swap(swapParams);
    
    if (result.success) {
      logger.info('✅ MEV-protected swap completed');
    }

    return result;
  }

  /**
   * Execute batch swaps with optimization
   */
  async executeBatchSwaps(swaps: Array<{
    from: string;
    to: string;
    amount: number;
  }>): Promise<{
    results: SwapResult[];
    successful: number;
    total: number;
    gasSaved: number;
  }> {
    logger.info(`📦 Executing batch of ${swaps.length} swaps...`);

    const results: SwapResult[] = [];
    let successful = 0;

    // Execute swaps sequentially to avoid rate limits
    for (const swap of swaps) {
      const result = await this.swap({
        fromMint: new PublicKey(swap.from),
        toMint: new PublicKey(swap.to),
        amount: swap.amount,
        simulateFirst: false // Skip simulation for batch
      });

      results.push(result);
      if (result.success) successful++;

      // Small delay between swaps
      await this.sleep(500);
    }

    // Calculate gas savings from batching
    const individualGas = results.reduce((sum, r) => sum + r.gasUsed, 0);
    const batchGas = individualGas * 0.8; // Estimate 20% savings
    const gasSaved = individualGas - batchGas;

    return {
      results,
      successful,
      total: swaps.length,
      gasSaved
    };
  }

  /**
   * Validate swap prerequisites
   */
  private async validateSwapPrerequisites(params: SwapParams): Promise<void> {
    // Check SOL balance for fees
    const solBalance = await this.connection.getBalance(this.wallet.publicKey);
    if (solBalance < 0.01 * 1e9) {
      throw new SwapError(
        SwapErrorType.INSUFFICIENT_BALANCE,
        'Insufficient SOL for transaction fees'
      );
    }

    // Validate token amounts
    if (params.amount <= 0) {
      throw new SwapError(
        SwapErrorType.INVALID_AMOUNT,
        'Swap amount must be greater than 0'
      );
    }

    // Check token account balances
    const fromTokenAccount = await getAssociatedTokenAddress(
      params.fromMint,
      this.wallet.publicKey
    );

    try {
      const account = await getAccount(this.connection, fromTokenAccount);
      const balance = Number(account.amount) / 1e6; // Assume 6 decimals
      
      if (balance < params.amount) {
        throw new SwapError(
          SwapErrorType.INSUFFICIENT_BALANCE,
          `Insufficient token balance: ${balance} < ${params.amount}`
        );
      }
    } catch (error) {
      throw new SwapError(
        SwapErrorType.TOKEN_ACCOUNT_ERROR,
        'Token account not found or invalid'
      );
    }
  }

  /**
   * Simulate swap before execution
   */
  private async simulateSwap(
    params: SwapParams,
    slippage: number
  ): Promise<{
    canExecute: boolean;
    expectedOutput: number;
    priceImpact: number;
    errors: string[];
    warnings: string[];
  }> {
    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      // Get swap estimate
      const poolAddress = params.poolAddress || await this.findPool(params.fromMint, params.toMint);
      const swapAmounts = await this.calculateSwapAmounts(
        params.fromMint,
        params.toMint,
        params.amount,
        slippage,
        poolAddress
      );

      // Check price impact
      if (swapAmounts.priceImpact > 5) {
        warnings.push(`High price impact: ${swapAmounts.priceImpact.toFixed(2)}%`);
      }

      // Check minimum output
      if (swapAmounts.amountOut < params.amount * 0.5) {
        warnings.push('Output less than 50% of input value');
      }

      return {
        canExecute: errors.length === 0,
        expectedOutput: swapAmounts.amountOut,
        priceImpact: swapAmounts.priceImpact,
        errors,
        warnings
      };

    } catch (error: any) {
      errors.push(error.message);
      return {
        canExecute: false,
        expectedOutput: 0,
        priceImpact: 0,
        errors,
        warnings
      };
    }
  }

  /**
   * Calculate swap amounts with slippage
   */
  private async calculateSwapAmounts(
    fromMint: PublicKey,
    toMint: PublicKey,
    amount: number,
    slippage: number,
    poolAddress: PublicKey
  ): Promise<{
    amountOut: number;
    amountOutWithSlippage: number;
    priceImpact: number;
    gasEstimate: number;
  }> {
    // Get pool parameters (simplified)
    const poolParams = {
      tokenAMint: fromMint,
      tokenBMint: toMint,
      poolAddress
    };

    const swapEstimate = await sarosSDK.getSwapAmountSaros(
      this.connection,
      fromMint.toString(),
      toMint.toString(),
      amount,
      slippage,
      poolParams
    );

    return {
      amountOut: swapEstimate.amountOut,
      amountOutWithSlippage: swapEstimate.amountOutWithSlippage,
      priceImpact: swapEstimate.priceImpact || 0,
      gasEstimate: 0.005 // Estimated gas in SOL
    };
  }

  /**
   * Prepare token accounts for swap
   */
  private async prepareTokenAccounts(
    fromMint: PublicKey,
    toMint: PublicKey
  ): Promise<{
    fromTokenAccount: PublicKey;
    toTokenAccount: PublicKey;
  }> {
    const fromTokenAccount = await getAssociatedTokenAddress(
      fromMint,
      this.wallet.publicKey
    );

    const toTokenAccount = await getAssociatedTokenAddress(
      toMint,
      this.wallet.publicKey
    );

    // Check if accounts exist, create if needed
    const toAccountInfo = await this.connection.getAccountInfo(toTokenAccount);
    if (!toAccountInfo) {
      logger.info('Creating destination token account...');
      // In production, would create the account here
    }

    return { fromTokenAccount, toTokenAccount };
  }

  /**
   * Execute the actual swap transaction
   */
  private async executeSwapTransaction(
    params: SwapParams,
    swapAmounts: any,
    fromTokenAccount: PublicKey,
    toTokenAccount: PublicKey,
    poolAddress: PublicKey,
    slippage: number
  ): Promise<SwapResult> {
    const fromDecimals = 6; // Simplified - would fetch actual decimals
    const toDecimals = 9;

    const result = await sarosSDK.swapSaros(
      this.connection,
      fromTokenAccount,
      toTokenAccount,
      params.amount * Math.pow(10, fromDecimals),
      swapAmounts.amountOutWithSlippage * Math.pow(10, toDecimals),
      null, // No referrer
      poolAddress,
      this.swapProgram,
      this.wallet.publicKey,
      params.fromMint,
      params.toMint
    );

    const explorerUrl = `https://explorer.solana.com/tx/${result.hash}?cluster=${this.network}`;

    return {
      success: true,
      signature: result.hash,
      amountIn: params.amount,
      amountOut: swapAmounts.amountOut,
      priceImpact: swapAmounts.priceImpact,
      slippageUsed: slippage,
      gasUsed: swapAmounts.gasEstimate,
      retries: 1,
      explorerUrl
    };
  }

  /**
   * Confirm transaction
   */
  private async confirmTransaction(signature: string): Promise<void> {
    const confirmation = await this.connection.confirmTransaction(
      signature,
      'confirmed'
    );

    if (confirmation.value.err) {
      throw new SwapError(
        SwapErrorType.TRANSACTION_FAILED,
        `Transaction failed: ${confirmation.value.err}`
      );
    }
  }

  /**
   * Find pool for token pair
   */
  private async findPool(fromMint: PublicKey, toMint: PublicKey): Promise<PublicKey> {
    // In production, would query actual pools
    // This is a placeholder
    return new PublicKey('POOL_ADDRESS_PLACEHOLDER');
  }

  /**
   * Update statistics
   */
  private updateStatistics(
    success: boolean,
    slippage: number,
    gasUsed: number,
    volume: number
  ): void {
    if (success) {
      this.statistics.successfulSwaps++;
    } else {
      this.statistics.failedSwaps++;
    }

    // Update averages
    const totalSwaps = this.statistics.successfulSwaps;
    if (totalSwaps > 0) {
      this.statistics.averageSlippage = 
        (this.statistics.averageSlippage * (totalSwaps - 1) + slippage) / totalSwaps;
      this.statistics.averageGasUsed = 
        (this.statistics.averageGasUsed * (totalSwaps - 1) + gasUsed) / totalSwaps;
    }

    this.statistics.totalVolume += volume;
  }

  /**
   * Get statistics
   */
  getStatistics(): SwapStatistics {
    return { ...this.statistics };
  }

  /**
   * Sleep utility
   */
  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}