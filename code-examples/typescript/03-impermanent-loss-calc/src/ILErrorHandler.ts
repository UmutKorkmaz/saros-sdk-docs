/**
 * ErrorHandler for Impermanent Loss Calculator
 * Handles errors specific to IL calculations and analysis
 */

import { logger, logError } from './utils/logger';

export enum ILErrorType {
  INVALID_PRICE_DATA = 'INVALID_PRICE_DATA',
  CALCULATION_ERROR = 'CALCULATION_ERROR',
  POOL_DATA_ERROR = 'POOL_DATA_ERROR',
  NETWORK_ERROR = 'NETWORK_ERROR',
  INSUFFICIENT_DATA = 'INSUFFICIENT_DATA',
  POSITION_NOT_FOUND = 'POSITION_NOT_FOUND',
  INVALID_PARAMETERS = 'INVALID_PARAMETERS',
  DATA_FETCH_ERROR = 'DATA_FETCH_ERROR',
  REPORT_GENERATION_ERROR = 'REPORT_GENERATION_ERROR',
  ANALYSIS_ERROR = 'ANALYSIS_ERROR',
  HISTORICAL_DATA_ERROR = 'HISTORICAL_DATA_ERROR',
  UNKNOWN = 'UNKNOWN'
}

export class ILError extends Error {
  public readonly type: ILErrorType;
  public readonly details: any;
  public readonly retryable: boolean;
  public readonly timestamp: Date;
  public readonly context: string;

  constructor(
    type: ILErrorType,
    message: string,
    context: string = 'general',
    details?: any,
    retryable: boolean = false
  ) {
    super(message);
    this.type = type;
    this.context = context;
    this.details = details;
    this.retryable = retryable;
    this.timestamp = new Date();
    this.name = 'ILError';
  }
}

export class ILErrorHandler {
  private errorHistory: ILError[] = [];
  private readonly MAX_HISTORY = 100;

  /**
   * Handle and categorize errors
   */
  handleError(error: any, context: string = 'general'): ILError {
    let ilError: ILError;

    // Parse different types of errors
    if (error instanceof ILError) {
      ilError = error;
    } else if (this.isValidationError(error)) {
      ilError = this.handleValidationError(error, context);
    } else if (this.isNetworkError(error)) {
      ilError = this.handleNetworkError(error, context);
    } else if (this.isCalculationError(error)) {
      ilError = this.handleCalculationError(error, context);
    } else if (this.isDataError(error)) {
      ilError = this.handleDataError(error, context);
    } else {
      ilError = new ILError(
        ILErrorType.UNKNOWN,
        error.message || 'Unknown error occurred',
        context,
        error,
        false
      );
    }

    // Log and store error
    this.logError(ilError);
    this.addToHistory(ilError);

    return ilError;
  }

  /**
   * Check if error is validation-related
   */
  private isValidationError(error: any): boolean {
    const message = error.message?.toLowerCase() || '';
    return message.includes('invalid') ||
           message.includes('validation') ||
           message.includes('parameter') ||
           message.includes('range') ||
           message.includes('negative');
  }

  /**
   * Handle validation errors
   */
  private handleValidationError(error: any, context: string): ILError {
    const message = error.message?.toLowerCase() || '';
    
    if (message.includes('price')) {
      return new ILError(
        ILErrorType.INVALID_PRICE_DATA,
        'Invalid price data provided',
        context,
        error,
        false
      );
    }
    
    if (message.includes('parameter')) {
      return new ILError(
        ILErrorType.INVALID_PARAMETERS,
        'Invalid calculation parameters',
        context,
        error,
        false
      );
    }

    return new ILError(
      ILErrorType.INVALID_PARAMETERS,
      'Parameter validation failed',
      context,
      error,
      false
    );
  }

  /**
   * Check if error is network-related
   */
  private isNetworkError(error: any): boolean {
    const message = error.message?.toLowerCase() || '';
    return error.code === 'NETWORK_ERROR' ||
           message.includes('fetch') ||
           message.includes('network') ||
           message.includes('timeout') ||
           message.includes('connection');
  }

  /**
   * Handle network errors
   */
  private handleNetworkError(error: any, context: string): ILError {
    return new ILError(
      ILErrorType.NETWORK_ERROR,
      'Network error occurred while fetching data',
      context,
      error,
      true // Network errors are retryable
    );
  }

  /**
   * Check if error is calculation-related
   */
  private isCalculationError(error: any): boolean {
    const message = error.message?.toLowerCase() || '';
    return message.includes('calculation') ||
           message.includes('math') ||
           message.includes('division') ||
           message.includes('precision') ||
           message.includes('overflow') ||
           message.includes('underflow');
  }

  /**
   * Handle calculation errors
   */
  private handleCalculationError(error: any, context: string): ILError {
    return new ILError(
      ILErrorType.CALCULATION_ERROR,
      'Mathematical calculation error',
      context,
      error,
      false
    );
  }

  /**
   * Check if error is data-related
   */
  private isDataError(error: any): boolean {
    const message = error.message?.toLowerCase() || '';
    return message.includes('data') ||
           message.includes('missing') ||
           message.includes('not found') ||
           message.includes('empty') ||
           message.includes('insufficient');
  }

  /**
   * Handle data errors
   */
  private handleDataError(error: any, context: string): ILError {
    const message = error.message?.toLowerCase() || '';
    
    if (message.includes('pool')) {
      return new ILError(
        ILErrorType.POOL_DATA_ERROR,
        'Pool data not available or invalid',
        context,
        error,
        true
      );
    }
    
    if (message.includes('position')) {
      return new ILError(
        ILErrorType.POSITION_NOT_FOUND,
        'Position data not found',
        context,
        error,
        false
      );
    }
    
    if (message.includes('historical')) {
      return new ILError(
        ILErrorType.HISTORICAL_DATA_ERROR,
        'Historical data unavailable',
        context,
        error,
        true
      );
    }

    return new ILError(
      ILErrorType.INSUFFICIENT_DATA,
      'Insufficient data for analysis',
      context,
      error,
      true
    );
  }

  /**
   * Log error with formatted output
   */
  private logError(error: ILError): void {
    const logMessage = `
    ╔══════════════════════════════════════════════╗
    ║ IL CALCULATION ERROR                         ║
    ╠══════════════════════════════════════════════╣
    ║ Type: ${error.type.padEnd(39)}║
    ║ Context: ${error.context.padEnd(36)}║
    ║ Message: ${error.message.substring(0, 36).padEnd(36)}║
    ║ Retryable: ${error.retryable.toString().padEnd(34)}║
    ║ Timestamp: ${error.timestamp.toISOString().padEnd(34)}║
    ╚══════════════════════════════════════════════╝
    `;

    logError(error, { context: error.context });

    if (error.details) {
      logger.debug('Error details:', error.details);
    }
  }

  /**
   * Add error to history
   */
  private addToHistory(error: ILError): void {
    this.errorHistory.push(error);
    
    if (this.errorHistory.length > this.MAX_HISTORY) {
      this.errorHistory.shift();
    }
  }

  /**
   * Get error statistics
   */
  getErrorStatistics(): {
    total: number;
    byType: { [key in ILErrorType]?: number };
    byContext: { [key: string]: number };
    retryable: number;
    recent: ILError[];
  } {
    const byType: { [key in ILErrorType]?: number } = {};
    const byContext: { [key: string]: number } = {};
    let retryable = 0;

    for (const error of this.errorHistory) {
      byType[error.type] = (byType[error.type] || 0) + 1;
      byContext[error.context] = (byContext[error.context] || 0) + 1;
      if (error.retryable) retryable++;
    }

    return {
      total: this.errorHistory.length,
      byType,
      byContext,
      retryable,
      recent: this.errorHistory.slice(-10)
    };
  }

  /**
   * Suggest recovery actions
   */
  suggestRecovery(error: ILError): string[] {
    const suggestions: string[] = [];

    switch (error.type) {
      case ILErrorType.INVALID_PRICE_DATA:
        suggestions.push('Verify price data sources');
        suggestions.push('Check for negative or zero prices');
        suggestions.push('Validate price ratio calculations');
        break;

      case ILErrorType.CALCULATION_ERROR:
        suggestions.push('Check input parameters for valid ranges');
        suggestions.push('Verify mathematical operations');
        suggestions.push('Reduce precision if overflow occurs');
        break;

      case ILErrorType.POOL_DATA_ERROR:
        suggestions.push('Refresh pool data from source');
        suggestions.push('Check if pool exists and is active');
        suggestions.push('Verify network connectivity');
        break;

      case ILErrorType.NETWORK_ERROR:
        suggestions.push('Check internet connection');
        suggestions.push('Try different data provider');
        suggestions.push('Increase timeout settings');
        break;

      case ILErrorType.INSUFFICIENT_DATA:
        suggestions.push('Ensure sufficient historical data');
        suggestions.push('Check data time range');
        suggestions.push('Use alternative data sources');
        break;

      case ILErrorType.POSITION_NOT_FOUND:
        suggestions.push('Verify position ID');
        suggestions.push('Check if position is still active');
        suggestions.push('Update position data');
        break;

      case ILErrorType.INVALID_PARAMETERS:
        suggestions.push('Validate all input parameters');
        suggestions.push('Check parameter types and ranges');
        suggestions.push('Review calculation requirements');
        break;

      case ILErrorType.HISTORICAL_DATA_ERROR:
        suggestions.push('Check data availability for time range');
        suggestions.push('Use shorter time periods');
        suggestions.push('Try alternative data providers');
        break;

      case ILErrorType.REPORT_GENERATION_ERROR:
        suggestions.push('Check file system permissions');
        suggestions.push('Verify output directory exists');
        suggestions.push('Reduce report complexity');
        break;

      default:
        suggestions.push('Review error details');
        suggestions.push('Check system logs');
        suggestions.push('Contact support if issue persists');
    }

    return suggestions;
  }

  /**
   * Execute recovery action
   */
  async executeRecovery(error: ILError, action: string): Promise<boolean> {
    logger.info(`Executing recovery action: ${action}`, { 
      errorType: error.type,
      context: error.context 
    });

    try {
      switch (action) {
        case 'retry':
          if (error.retryable) {
            // Wait before retry
            await new Promise(resolve => setTimeout(resolve, 1000));
            return true;
          }
          return false;

        case 'refresh_data':
          // Implement data refresh logic
          logger.info('Refreshing data sources...');
          return true;

        case 'fallback_calculation':
          // Use alternative calculation method
          logger.info('Using fallback calculation method...');
          return true;

        default:
          logger.warn(`Unknown recovery action: ${action}`);
          return false;
      }
    } catch (recoveryError) {
      logger.error('Recovery action failed:', recoveryError);
      return false;
    }
  }

  /**
   * Clear error history
   */
  clearHistory(): void {
    this.errorHistory = [];
    logger.info('IL error history cleared');
  }

  /**
   * Validate calculation parameters
   */
  validateCalculationParams(params: any): void {
    if (!params) {
      throw new ILError(
        ILErrorType.INVALID_PARAMETERS,
        'Parameters are required',
        'validation'
      );
    }

    if (typeof params.initialPriceRatio !== 'number' || params.initialPriceRatio <= 0) {
      throw new ILError(
        ILErrorType.INVALID_PRICE_DATA,
        'Initial price ratio must be a positive number',
        'validation'
      );
    }

    if (typeof params.currentPriceRatio !== 'number' || params.currentPriceRatio <= 0) {
      throw new ILError(
        ILErrorType.INVALID_PRICE_DATA,
        'Current price ratio must be a positive number',
        'validation'
      );
    }

    if (params.weights && (!Array.isArray(params.weights) || params.weights.length !== 2)) {
      throw new ILError(
        ILErrorType.INVALID_PARAMETERS,
        'Weights must be an array of two numbers',
        'validation'
      );
    }
  }
}

export default ILErrorHandler;