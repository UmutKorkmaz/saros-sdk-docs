/**
 * ErrorHandler for Multi-Hop Routing System
 * Handles errors specific to pathfinding, route optimization, and execution
 */

import { logger } from './utils/logger';

export enum RoutingErrorType {
  NO_ROUTE_FOUND = 'NO_ROUTE_FOUND',
  INSUFFICIENT_LIQUIDITY = 'INSUFFICIENT_LIQUIDITY',
  EXCESSIVE_PRICE_IMPACT = 'EXCESSIVE_PRICE_IMPACT',
  PATHFINDING_ERROR = 'PATHFINDING_ERROR',
  POOL_DATA_ERROR = 'POOL_DATA_ERROR',
  OPTIMIZATION_ERROR = 'OPTIMIZATION_ERROR',
  EXECUTION_ERROR = 'EXECUTION_ERROR',
  SIMULATION_FAILED = 'SIMULATION_FAILED',
  NETWORK_ERROR = 'NETWORK_ERROR',
  TIMEOUT_ERROR = 'TIMEOUT_ERROR',
  INVALID_ROUTE = 'INVALID_ROUTE',
  SLIPPAGE_EXCEEDED = 'SLIPPAGE_EXCEEDED',
  GRAPH_BUILD_ERROR = 'GRAPH_BUILD_ERROR',
  ARBITRAGE_ERROR = 'ARBITRAGE_ERROR',
  SPLIT_ROUTE_ERROR = 'SPLIT_ROUTE_ERROR',
  UNKNOWN = 'UNKNOWN'
}

export interface RouteErrorContext {
  fromMint: string;
  toMint: string;
  amount: string;
  maxHops?: number;
  strategy?: string;
  routeIndex?: number;
  hopIndex?: number;
  poolAddress?: string;
}

export class RoutingError extends Error {
  public readonly type: RoutingErrorType;
  public readonly context: RouteErrorContext;
  public readonly details: any;
  public readonly retryable: boolean;
  public readonly timestamp: Date;
  public readonly severity: 'low' | 'medium' | 'high' | 'critical';

  constructor(
    type: RoutingErrorType,
    message: string,
    context: RouteErrorContext,
    details?: any,
    retryable: boolean = false,
    severity: 'low' | 'medium' | 'high' | 'critical' = 'medium'
  ) {
    super(message);
    this.type = type;
    this.context = context;
    this.details = details;
    this.retryable = retryable;
    this.severity = severity;
    this.timestamp = new Date();
    this.name = 'RoutingError';
  }
}

export class RoutingErrorHandler {
  private errorHistory: RoutingError[] = [];
  private readonly MAX_HISTORY = 200;
  private routeFailureCount: Map<string, number> = new Map();
  private poolFailureCount: Map<string, number> = new Map();

  /**
   * Handle and categorize routing errors
   */
  handleError(error: any, context: RouteErrorContext): RoutingError {
    let routingError: RoutingError;

    if (error instanceof RoutingError) {
      routingError = error;
    } else if (this.isPathfindingError(error)) {
      routingError = this.handlePathfindingError(error, context);
    } else if (this.isLiquidityError(error)) {
      routingError = this.handleLiquidityError(error, context);
    } else if (this.isPriceImpactError(error)) {
      routingError = this.handlePriceImpactError(error, context);
    } else if (this.isNetworkError(error)) {
      routingError = this.handleNetworkError(error, context);
    } else if (this.isExecutionError(error)) {
      routingError = this.handleExecutionError(error, context);
    } else if (this.isSimulationError(error)) {
      routingError = this.handleSimulationError(error, context);
    } else {
      routingError = new RoutingError(
        RoutingErrorType.UNKNOWN,
        error.message || 'Unknown routing error',
        context,
        error,
        false,
        'medium'
      );
    }

    // Update failure counters
    this.updateFailureCounters(routingError);

    // Log and store error
    this.logError(routingError);
    this.addToHistory(routingError);

    return routingError;
  }

  /**
   * Check if error is pathfinding-related
   */
  private isPathfindingError(error: any): boolean {
    const message = error.message?.toLowerCase() || '';
    return message.includes('path') ||
           message.includes('route') ||
           message.includes('graph') ||
           message.includes('node') ||
           message.includes('edge');
  }

  /**
   * Handle pathfinding errors
   */
  private handlePathfindingError(error: any, context: RouteErrorContext): RoutingError {
    const message = error.message?.toLowerCase() || '';
    
    if (message.includes('no route') || message.includes('no path')) {
      return new RoutingError(
        RoutingErrorType.NO_ROUTE_FOUND,
        'No route found between tokens',
        context,
        error,
        true,
        'high'
      );
    }
    
    if (message.includes('graph')) {
      return new RoutingError(
        RoutingErrorType.GRAPH_BUILD_ERROR,
        'Error building routing graph',
        context,
        error,
        true,
        'high'
      );
    }
    
    return new RoutingError(
      RoutingErrorType.PATHFINDING_ERROR,
      'Pathfinding algorithm error',
      context,
      error,
      true,
      'medium'
    );
  }

  /**
   * Check if error is liquidity-related
   */
  private isLiquidityError(error: any): boolean {
    const message = error.message?.toLowerCase() || '';
    return message.includes('liquidity') ||
           message.includes('reserves') ||
           message.includes('insufficient');
  }

  /**
   * Handle liquidity errors
   */
  private handleLiquidityError(error: any, context: RouteErrorContext): RoutingError {
    return new RoutingError(
      RoutingErrorType.INSUFFICIENT_LIQUIDITY,
      'Insufficient liquidity for swap amount',
      context,
      error,
      true,
      'high'
    );
  }

  /**
   * Check if error is price impact related
   */
  private isPriceImpactError(error: any): boolean {
    const message = error.message?.toLowerCase() || '';
    return message.includes('price impact') ||
           message.includes('slippage') ||
           message.includes('excessive');
  }

  /**
   * Handle price impact errors
   */
  private handlePriceImpactError(error: any, context: RouteErrorContext): RoutingError {
    const message = error.message?.toLowerCase() || '';
    
    if (message.includes('slippage')) {
      return new RoutingError(
        RoutingErrorType.SLIPPAGE_EXCEEDED,
        'Slippage tolerance exceeded during execution',
        context,
        error,
        true,
        'medium'
      );
    }
    
    return new RoutingError(
      RoutingErrorType.EXCESSIVE_PRICE_IMPACT,
      'Price impact too high for safe execution',
      context,
      error,
      true,
      'high'
    );
  }

  /**
   * Check if error is network-related
   */
  private isNetworkError(error: any): boolean {
    const message = error.message?.toLowerCase() || '';
    return error.code === 'NETWORK_ERROR' ||
           message.includes('network') ||
           message.includes('connection') ||
           message.includes('timeout') ||
           message.includes('fetch');
  }

  /**
   * Handle network errors
   */
  private handleNetworkError(error: any, context: RouteErrorContext): RoutingError {
    const message = error.message?.toLowerCase() || '';
    
    if (message.includes('timeout')) {
      return new RoutingError(
        RoutingErrorType.TIMEOUT_ERROR,
        'Network timeout during route execution',
        context,
        error,
        true,
        'medium'
      );
    }
    
    return new RoutingError(
      RoutingErrorType.NETWORK_ERROR,
      'Network error during routing operation',
      context,
      error,
      true,
      'medium'
    );
  }

  /**
   * Check if error is execution-related
   */
  private isExecutionError(error: any): boolean {
    const message = error.message?.toLowerCase() || '';
    return message.includes('execution') ||
           message.includes('transaction') ||
           message.includes('swap') ||
           message.includes('failed');
  }

  /**
   * Handle execution errors
   */
  private handleExecutionError(error: any, context: RouteErrorContext): RoutingError {
    return new RoutingError(
      RoutingErrorType.EXECUTION_ERROR,
      'Route execution failed',
      context,
      error,
      true,
      'high'
    );
  }

  /**
   * Check if error is simulation-related
   */
  private isSimulationError(error: any): boolean {
    const message = error.message?.toLowerCase() || '';
    return message.includes('simulation') ||
           message.includes('estimate') ||
           message.includes('dry run');
  }

  /**
   * Handle simulation errors
   */
  private handleSimulationError(error: any, context: RouteErrorContext): RoutingError {
    return new RoutingError(
      RoutingErrorType.SIMULATION_FAILED,
      'Route simulation failed',
      context,
      error,
      true,
      'medium'
    );
  }

  /**
   * Update failure counters for analytics
   */
  private updateFailureCounters(error: RoutingError): void {
    const routeKey = `${error.context.fromMint}-${error.context.toMint}`;
    this.routeFailureCount.set(routeKey, (this.routeFailureCount.get(routeKey) || 0) + 1);
    
    if (error.context.poolAddress) {
      this.poolFailureCount.set(
        error.context.poolAddress,
        (this.poolFailureCount.get(error.context.poolAddress) || 0) + 1
      );
    }
  }

  /**
   * Log error with detailed context
   */
  private logError(error: RoutingError): void {
    const logMessage = `
    ╔══════════════════════════════════════════════╗
    ║ MULTI-HOP ROUTING ERROR                      ║
    ╠══════════════════════════════════════════════╣
    ║ Type: ${error.type.padEnd(39)}║
    ║ Severity: ${error.severity.padEnd(35)}║
    ║ From: ${error.context.fromMint.substring(0, 8).padEnd(39)}║
    ║ To: ${error.context.toMint.substring(0, 8).padEnd(41)}║
    ║ Amount: ${(error.context.amount || 'N/A').padEnd(37)}║
    ║ Retryable: ${error.retryable.toString().padEnd(34)}║
    ║ Timestamp: ${error.timestamp.toISOString().padEnd(34)}║
    ╚══════════════════════════════════════════════╝
    `;

    const logLevel = error.severity === 'critical' ? 'error' : 
                     error.severity === 'high' ? 'error' :
                     error.severity === 'medium' ? 'warn' : 'info';

    logger[logLevel](logMessage);

    if (error.details) {
      logger.debug('Error details:', error.details);
    }
  }

  /**
   * Add error to history
   */
  private addToHistory(error: RoutingError): void {
    this.errorHistory.push(error);
    
    if (this.errorHistory.length > this.MAX_HISTORY) {
      this.errorHistory.shift();
    }
  }

  /**
   * Get routing error statistics
   */
  getErrorStatistics(): {
    total: number;
    byType: { [key in RoutingErrorType]?: number };
    bySeverity: { [key: string]: number };
    retryable: number;
    recent: RoutingError[];
    failedRoutes: Array<{ route: string; failures: number }>;
    problemPools: Array<{ pool: string; failures: number }>;
  } {
    const byType: { [key in RoutingErrorType]?: number } = {};
    const bySeverity: { [key: string]: number } = {};
    let retryable = 0;

    for (const error of this.errorHistory) {
      byType[error.type] = (byType[error.type] || 0) + 1;
      bySeverity[error.severity] = (bySeverity[error.severity] || 0) + 1;
      if (error.retryable) retryable++;
    }

    const failedRoutes = Array.from(this.routeFailureCount.entries())
      .map(([route, failures]) => ({ route, failures }))
      .sort((a, b) => b.failures - a.failures)
      .slice(0, 10);

    const problemPools = Array.from(this.poolFailureCount.entries())
      .map(([pool, failures]) => ({ pool, failures }))
      .sort((a, b) => b.failures - a.failures)
      .slice(0, 10);

    return {
      total: this.errorHistory.length,
      byType,
      bySeverity,
      retryable,
      recent: this.errorHistory.slice(-10),
      failedRoutes,
      problemPools
    };
  }

  /**
   * Suggest recovery strategies
   */
  suggestRecovery(error: RoutingError): string[] {
    const suggestions: string[] = [];

    switch (error.type) {
      case RoutingErrorType.NO_ROUTE_FOUND:
        suggestions.push('Try increasing max hops parameter');
        suggestions.push('Check if token pair has any liquidity');
        suggestions.push('Consider multi-step routing via popular tokens');
        suggestions.push('Reduce swap amount to find available liquidity');
        break;

      case RoutingErrorType.INSUFFICIENT_LIQUIDITY:
        suggestions.push('Reduce swap amount');
        suggestions.push('Split swap into smaller chunks');
        suggestions.push('Try routing through more liquid pairs');
        suggestions.push('Wait for better liquidity conditions');
        break;

      case RoutingErrorType.EXCESSIVE_PRICE_IMPACT:
        suggestions.push('Reduce swap amount');
        suggestions.push('Use multiple smaller swaps over time');
        suggestions.push('Increase max price impact tolerance');
        suggestions.push('Try different routing strategy');
        break;

      case RoutingErrorType.SLIPPAGE_EXCEEDED:
        suggestions.push('Increase slippage tolerance');
        suggestions.push('Retry with current market conditions');
        suggestions.push('Use more stable routing paths');
        suggestions.push('Consider time-weighted execution');
        break;

      case RoutingErrorType.PATHFINDING_ERROR:
        suggestions.push('Rebuild routing graph');
        suggestions.push('Check pool data freshness');
        suggestions.push('Try simpler routing strategies');
        suggestions.push('Update pathfinding parameters');
        break;

      case RoutingErrorType.NETWORK_ERROR:
        suggestions.push('Check network connectivity');
        suggestions.push('Try different RPC endpoint');
        suggestions.push('Increase timeout settings');
        suggestions.push('Retry with exponential backoff');
        break;

      case RoutingErrorType.EXECUTION_ERROR:
        suggestions.push('Verify wallet has sufficient balance');
        suggestions.push('Check for sufficient SOL for fees');
        suggestions.push('Simulate transaction before execution');
        suggestions.push('Verify all token accounts exist');
        break;

      case RoutingErrorType.SIMULATION_FAILED:
        suggestions.push('Check if pools are still active');
        suggestions.push('Verify token account status');
        suggestions.push('Update pool data before retry');
        suggestions.push('Try with smaller amounts');
        break;

      default:
        suggestions.push('Review routing parameters');
        suggestions.push('Check system logs for details');
        suggestions.push('Try alternative routing strategies');
        suggestions.push('Contact support if issue persists');
    }

    return suggestions;
  }

  /**
   * Check if route should be avoided based on failure history
   */
  shouldAvoidRoute(fromMint: string, toMint: string): boolean {
    const routeKey = `${fromMint}-${toMint}`;
    const failures = this.routeFailureCount.get(routeKey) || 0;
    return failures > 5; // Avoid routes with more than 5 recent failures
  }

  /**
   * Check if pool should be avoided based on failure history
   */
  shouldAvoidPool(poolAddress: string): boolean {
    const failures = this.poolFailureCount.get(poolAddress) || 0;
    return failures > 10; // Avoid pools with more than 10 recent failures
  }

  /**
   * Clear error history and counters
   */
  clearHistory(): void {
    this.errorHistory = [];
    this.routeFailureCount.clear();
    this.poolFailureCount.clear();
    logger.info('Routing error history cleared');
  }

  /**
   * Generate error report
   */
  generateErrorReport(): string {
    const stats = this.getErrorStatistics();
    
    const topErrors = Object.entries(stats.byType)
      .sort(([,a], [,b]) => (b as number) - (a as number))
      .slice(0, 3)
      .map(([type, count]) => `  • ${type}: ${count}`)
      .join('\n');
    
    const failedRoutes = stats.failedRoutes
      .slice(0, 3)
      .map(({ route, failures }) => `  • ${route.substring(0, 20)}...: ${failures}`)
      .join('\n');
    
    return [
      '==================================================',
      '             ROUTING ERROR REPORT               ',
      '==================================================',
      `Total Errors: ${stats.total}`,
      `Retryable: ${stats.retryable}`,
      `Success Rate: ${((100 - (stats.total / (stats.total + 100)) * 100)).toFixed(1)}%`,
      '',
      'Top Error Types:',
      topErrors,
      '',
      'Most Failed Routes:',
      failedRoutes,
      '=================================================='
    ].join('\n');
  }
}

export default RoutingErrorHandler;