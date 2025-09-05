/**
 * ErrorHandler - Comprehensive error handling for swap operations
 */

import { logger } from './utils/logger';

export enum SwapErrorType {
  INSUFFICIENT_BALANCE = 'INSUFFICIENT_BALANCE',
  SLIPPAGE_EXCEEDED = 'SLIPPAGE_EXCEEDED',
  POOL_NOT_FOUND = 'POOL_NOT_FOUND',
  TRANSACTION_FAILED = 'TRANSACTION_FAILED',
  NETWORK_ERROR = 'NETWORK_ERROR',
  INVALID_AMOUNT = 'INVALID_AMOUNT',
  TOKEN_ACCOUNT_ERROR = 'TOKEN_ACCOUNT_ERROR',
  PROGRAM_ERROR = 'PROGRAM_ERROR',
  SIMULATION_FAILED = 'SIMULATION_FAILED',
  TIMEOUT = 'TIMEOUT',
  UNKNOWN = 'UNKNOWN'
}

export class SwapError extends Error {
  public readonly type: SwapErrorType;
  public readonly details: any;
  public readonly retryable: boolean;
  public readonly timestamp: Date;

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
    this.timestamp = new Date();
    this.name = 'SwapError';
  }
}

export class ErrorHandler {
  private errorHistory: SwapError[] = [];
  private readonly MAX_HISTORY = 100;

  /**
   * Handle and categorize errors
   */
  handleError(error: any): SwapError {
    let swapError: SwapError;

    // Parse Solana program errors
    if (error.code !== undefined) {
      swapError = this.handleProgramError(error);
    } 
    // Parse transaction errors
    else if (error.message?.includes('Transaction')) {
      swapError = this.handleTransactionError(error);
    }
    // Parse network errors
    else if (error.code === 'NETWORK_ERROR' || error.message?.includes('fetch')) {
      swapError = new SwapError(
        SwapErrorType.NETWORK_ERROR,
        'Network connection error',
        error,
        true // Network errors are retryable
      );
    }
    // Parse balance errors
    else if (error.message?.toLowerCase().includes('insufficient')) {
      swapError = new SwapError(
        SwapErrorType.INSUFFICIENT_BALANCE,
        this.parseBalanceError(error.message),
        error,
        false
      );
    }
    // Parse slippage errors
    else if (error.message?.toLowerCase().includes('slippage')) {
      swapError = new SwapError(
        SwapErrorType.SLIPPAGE_EXCEEDED,
        'Slippage tolerance exceeded',
        error,
        true // Can retry with higher slippage
      );
    }
    // Parse pool errors
    else if (error.message?.toLowerCase().includes('pool')) {
      swapError = new SwapError(
        SwapErrorType.POOL_NOT_FOUND,
        'Trading pool not found or inactive',
        error,
        false
      );
    }
    // Default to unknown error
    else {
      swapError = new SwapError(
        SwapErrorType.UNKNOWN,
        error.message || 'Unknown error occurred',
        error,
        false
      );
    }

    // Log and store error
    this.logError(swapError);
    this.addToHistory(swapError);

    return swapError;
  }

  /**
   * Handle Solana program errors
   */
  private handleProgramError(error: any): SwapError {
    const errorMap: { [key: number]: { type: SwapErrorType; message: string; retryable: boolean } } = {
      6000: {
        type: SwapErrorType.SLIPPAGE_EXCEEDED,
        message: 'Slippage tolerance exceeded',
        retryable: true
      },
      6001: {
        type: SwapErrorType.INSUFFICIENT_BALANCE,
        message: 'Insufficient token balance',
        retryable: false
      },
      6002: {
        type: SwapErrorType.POOL_NOT_FOUND,
        message: 'Pool not found',
        retryable: false
      },
      6003: {
        type: SwapErrorType.INVALID_AMOUNT,
        message: 'Invalid swap amount',
        retryable: false
      },
      6004: {
        type: SwapErrorType.TOKEN_ACCOUNT_ERROR,
        message: 'Token account error',
        retryable: false
      }
    };

    const errorInfo = errorMap[error.code] || {
      type: SwapErrorType.PROGRAM_ERROR,
      message: `Program error: ${error.code}`,
      retryable: false
    };

    return new SwapError(
      errorInfo.type,
      errorInfo.message,
      error,
      errorInfo.retryable
    );
  }

  /**
   * Handle transaction errors
   */
  private handleTransactionError(error: any): SwapError {
    if (error.message?.includes('simulation failed')) {
      return new SwapError(
        SwapErrorType.SIMULATION_FAILED,
        'Transaction simulation failed',
        error,
        true // Can retry with different parameters
      );
    }

    if (error.message?.includes('blockhash not found')) {
      return new SwapError(
        SwapErrorType.NETWORK_ERROR,
        'Blockhash expired',
        error,
        true // Can retry with new blockhash
      );
    }

    if (error.message?.includes('timeout')) {
      return new SwapError(
        SwapErrorType.TIMEOUT,
        'Transaction confirmation timeout',
        error,
        true // Can retry
      );
    }

    return new SwapError(
      SwapErrorType.TRANSACTION_FAILED,
      'Transaction failed',
      error,
      false
    );
  }

  /**
   * Parse balance error messages
   */
  private parseBalanceError(message: string): string {
    const match = message.match(/(\d+\.?\d*)\s*<\s*(\d+\.?\d*)/);
    if (match) {
      return `Insufficient balance: ${match[1]} < ${match[2]}`;
    }
    return 'Insufficient balance for swap';
  }

  /**
   * Log error details
   */
  private logError(error: SwapError): void {
    const logMessage = `
    ╔══════════════════════════════════════════════╗
    ║ SWAP ERROR                                   ║
    ╠══════════════════════════════════════════════╣
    ║ Type: ${error.type.padEnd(39)}║
    ║ Message: ${error.message.substring(0, 36).padEnd(36)}║
    ║ Retryable: ${error.retryable.toString().padEnd(34)}║
    ║ Timestamp: ${error.timestamp.toISOString().padEnd(34)}║
    ╚══════════════════════════════════════════════╝
    `;

    logger.error(logMessage);

    if (error.details) {
      logger.debug('Error details:', error.details);
    }
  }

  /**
   * Add error to history
   */
  private addToHistory(error: SwapError): void {
    this.errorHistory.push(error);
    
    // Keep only recent errors
    if (this.errorHistory.length > this.MAX_HISTORY) {
      this.errorHistory.shift();
    }
  }

  /**
   * Get error history
   */
  getErrorHistory(): SwapError[] {
    return [...this.errorHistory];
  }

  /**
   * Get error statistics
   */
  getErrorStatistics(): {
    total: number;
    byType: { [key in SwapErrorType]?: number };
    retryable: number;
    recent: SwapError[];
  } {
    const byType: { [key in SwapErrorType]?: number } = {};
    let retryable = 0;

    for (const error of this.errorHistory) {
      byType[error.type] = (byType[error.type] || 0) + 1;
      if (error.retryable) retryable++;
    }

    return {
      total: this.errorHistory.length,
      byType,
      retryable,
      recent: this.errorHistory.slice(-10)
    };
  }

  /**
   * Suggest recovery action based on error type
   */
  suggestRecovery(error: SwapError): string[] {
    const suggestions: string[] = [];

    switch (error.type) {
      case SwapErrorType.INSUFFICIENT_BALANCE:
        suggestions.push('Check your token balance');
        suggestions.push('Ensure you have enough tokens for the swap');
        suggestions.push('Consider swapping a smaller amount');
        break;

      case SwapErrorType.SLIPPAGE_EXCEEDED:
        suggestions.push('Increase slippage tolerance');
        suggestions.push('Try swapping during lower volatility');
        suggestions.push('Consider splitting into smaller swaps');
        break;

      case SwapErrorType.POOL_NOT_FOUND:
        suggestions.push('Check if the token pair has liquidity');
        suggestions.push('Try using a different route');
        suggestions.push('Consider multi-hop swapping');
        break;

      case SwapErrorType.NETWORK_ERROR:
        suggestions.push('Check your internet connection');
        suggestions.push('Try using a different RPC endpoint');
        suggestions.push('Wait and retry in a few moments');
        break;

      case SwapErrorType.TRANSACTION_FAILED:
        suggestions.push('Check transaction details on explorer');
        suggestions.push('Verify all parameters are correct');
        suggestions.push('Ensure sufficient SOL for fees');
        break;

      case SwapErrorType.TIMEOUT:
        suggestions.push('Increase transaction timeout');
        suggestions.push('Check network congestion');
        suggestions.push('Try with higher priority fee');
        break;

      default:
        suggestions.push('Check all parameters');
        suggestions.push('Review transaction details');
        suggestions.push('Contact support if issue persists');
    }

    return suggestions;
  }

  /**
   * Clear error history
   */
  clearHistory(): void {
    this.errorHistory = [];
    logger.info('Error history cleared');
  }
}