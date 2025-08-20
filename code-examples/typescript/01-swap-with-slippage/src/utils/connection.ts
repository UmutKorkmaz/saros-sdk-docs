/**
 * Solana connection utilities
 */

import { 
  Connection, 
  ConnectionConfig,
  Commitment,
  PublicKey,
  ParsedAccountData,
  RpcResponseAndContext,
  GetProgramAccountsFilter
} from '@solana/web3.js';
import { logger } from './logger';

/**
 * RPC endpoint configurations
 */
export const RPC_ENDPOINTS = {
  mainnet: {
    public: 'https://api.mainnet-beta.solana.com',
    helius: process.env.HELIUS_RPC_URL,
    quicknode: process.env.QUICKNODE_RPC_URL,
    alchemy: process.env.ALCHEMY_RPC_URL
  },
  devnet: {
    public: 'https://api.devnet.solana.com',
    helius: process.env.HELIUS_DEVNET_URL
  },
  testnet: {
    public: 'https://api.testnet.solana.com'
  }
};

/**
 * Connection pool for managing multiple RPC connections
 */
export class ConnectionPool {
  private connections: Map<string, Connection> = new Map();
  private currentIndex: number = 0;
  private endpoints: string[];

  constructor(endpoints: string[], config?: ConnectionConfig) {
    this.endpoints = endpoints;
    
    // Initialize connections
    for (const endpoint of endpoints) {
      const connection = new Connection(endpoint, config || {
        commitment: 'confirmed',
        confirmTransactionInitialTimeout: 60000
      });
      
      this.connections.set(endpoint, connection);
    }
    
    logger.info(`Connection pool initialized with ${endpoints.length} endpoints`);
  }

  /**
   * Get next connection in round-robin fashion
   */
  getConnection(): Connection {
    const endpoint = this.endpoints[this.currentIndex];
    this.currentIndex = (this.currentIndex + 1) % this.endpoints.length;
    return this.connections.get(endpoint)!;
  }

  /**
   * Get all connections
   */
  getAllConnections(): Connection[] {
    return Array.from(this.connections.values());
  }

  /**
   * Get fastest responding connection
   */
  async getFastestConnection(): Promise<Connection> {
    const startTime = Date.now();
    const promises = this.getAllConnections().map(async (conn) => {
      try {
        await conn.getSlot();
        return { conn, time: Date.now() - startTime };
      } catch {
        return { conn, time: Infinity };
      }
    });

    const results = await Promise.all(promises);
    const fastest = results.reduce((min, curr) => 
      curr.time < min.time ? curr : min
    );

    logger.debug(`Fastest connection responded in ${fastest.time}ms`);
    return fastest.conn;
  }
}

/**
 * Create optimized connection with retry logic
 */
export function createConnection(
  endpoint?: string,
  commitment: Commitment = 'confirmed'
): Connection {
  const url = endpoint || RPC_ENDPOINTS.devnet.public;
  
  return new Connection(url, {
    commitment,
    confirmTransactionInitialTimeout: 60000,
    disableRetryOnRateLimit: false,
    httpHeaders: {
      'Content-Type': 'application/json',
    }
  });
}

/**
 * Get connection with automatic fallback
 */
export async function getConnectionWithFallback(
  network: 'mainnet' | 'devnet' | 'testnet' = 'devnet'
): Promise<Connection> {
  const endpoints = Object.values(RPC_ENDPOINTS[network]).filter(Boolean) as string[];
  
  for (const endpoint of endpoints) {
    try {
      const connection = createConnection(endpoint);
      
      // Test connection
      await connection.getSlot();
      
      logger.info(`Connected to ${endpoint}`);
      return connection;
      
    } catch (error) {
      logger.warn(`Failed to connect to ${endpoint}:`, error);
      continue;
    }
  }
  
  throw new Error('All RPC endpoints failed');
}

/**
 * Retry utility for RPC calls
 */
export async function retryRpcCall<T>(
  fn: () => Promise<T>,
  maxRetries: number = 3,
  delay: number = 1000
): Promise<T> {
  let lastError: any;
  
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error: any) {
      lastError = error;
      
      // Check if error is retryable
      if (error.message?.includes('429') || // Rate limit
          error.message?.includes('502') || // Bad gateway
          error.message?.includes('503') || // Service unavailable
          error.message?.includes('timeout')) {
        
        logger.debug(`RPC call failed (attempt ${i + 1}/${maxRetries}), retrying...`);
        await new Promise(resolve => setTimeout(resolve, delay * Math.pow(2, i)));
        continue;
      }
      
      // Non-retryable error
      throw error;
    }
  }
  
  throw lastError;
}

/**
 * Batch RPC requests for efficiency
 */
export class BatchRequestManager {
  private connection: Connection;
  private batchSize: number;
  private queue: Array<{
    method: string;
    params: any[];
    resolve: (value: any) => void;
    reject: (error: any) => void;
  }> = [];
  private processing: boolean = false;

  constructor(connection: Connection, batchSize: number = 100) {
    this.connection = connection;
    this.batchSize = batchSize;
  }

  /**
   * Add request to batch queue
   */
  async addRequest<T>(method: string, params: any[]): Promise<T> {
    return new Promise((resolve, reject) => {
      this.queue.push({ method, params, resolve, reject });
      
      if (this.queue.length >= this.batchSize) {
        this.processBatch();
      } else if (!this.processing) {
        // Process after short delay to allow batching
        setTimeout(() => this.processBatch(), 100);
      }
    });
  }

  /**
   * Process batch of requests
   */
  private async processBatch(): Promise<void> {
    if (this.processing || this.queue.length === 0) return;
    
    this.processing = true;
    const batch = this.queue.splice(0, this.batchSize);
    
    try {
      // Group by method for efficient batching
      const grouped = batch.reduce((acc, req) => {
        if (!acc[req.method]) acc[req.method] = [];
        acc[req.method].push(req);
        return acc;
      }, {} as { [method: string]: typeof batch });

      // Process each group
      for (const [method, requests] of Object.entries(grouped)) {
        await this.processMethodBatch(method, requests);
      }
      
    } catch (error) {
      // Reject all requests in batch on error
      batch.forEach(req => req.reject(error));
    } finally {
      this.processing = false;
      
      // Process next batch if queue not empty
      if (this.queue.length > 0) {
        this.processBatch();
      }
    }
  }

  /**
   * Process batch of same method
   */
  private async processMethodBatch(
    method: string,
    requests: Array<any>
  ): Promise<void> {
    // This is simplified - in production, implement actual batch RPC
    const promises = requests.map(req => 
      (this.connection as any)[method](...req.params)
        .then(req.resolve)
        .catch(req.reject)
    );
    
    await Promise.all(promises);
  }
}

/**
 * Monitor connection health
 */
export class ConnectionMonitor {
  private connection: Connection;
  private healthCheckInterval: NodeJS.Timeout | null = null;
  private isHealthy: boolean = true;
  private latency: number = 0;

  constructor(connection: Connection) {
    this.connection = connection;
  }

  /**
   * Start monitoring connection health
   */
  startMonitoring(intervalMs: number = 30000): void {
    if (this.healthCheckInterval) return;
    
    this.healthCheckInterval = setInterval(async () => {
      await this.checkHealth();
    }, intervalMs);
    
    logger.info('Connection health monitoring started');
  }

  /**
   * Stop monitoring
   */
  stopMonitoring(): void {
    if (this.healthCheckInterval) {
      clearInterval(this.healthCheckInterval);
      this.healthCheckInterval = null;
      logger.info('Connection health monitoring stopped');
    }
  }

  /**
   * Check connection health
   */
  private async checkHealth(): Promise<void> {
    const startTime = Date.now();
    
    try {
      await this.connection.getSlot();
      this.latency = Date.now() - startTime;
      this.isHealthy = true;
      
      if (this.latency > 1000) {
        logger.warn(`High RPC latency detected: ${this.latency}ms`);
      }
      
    } catch (error) {
      this.isHealthy = false;
      logger.error('Connection health check failed:', error);
    }
  }

  /**
   * Get health status
   */
  getHealthStatus(): { isHealthy: boolean; latency: number } {
    return {
      isHealthy: this.isHealthy,
      latency: this.latency
    };
  }
}

/**
 * Get multiple accounts efficiently
 */
export async function getMultipleAccounts(
  connection: Connection,
  publicKeys: PublicKey[],
  commitment?: Commitment
): Promise<(any | null)[]> {
  const batchSize = 100; // Max accounts per request
  const results: (any | null)[] = [];
  
  for (let i = 0; i < publicKeys.length; i += batchSize) {
    const batch = publicKeys.slice(i, i + batchSize);
    const accounts = await retryRpcCall(() => 
      connection.getMultipleAccountsInfo(batch, commitment)
    );
    results.push(...accounts);
  }
  
  return results;
}

/**
 * Subscribe to account changes with automatic reconnection
 */
export class AccountSubscription {
  private connection: Connection;
  private publicKey: PublicKey;
  private callback: (data: any) => void;
  private subscriptionId: number | null = null;
  private reconnectAttempts: number = 0;
  private maxReconnectAttempts: number = 5;

  constructor(
    connection: Connection,
    publicKey: PublicKey,
    callback: (data: any) => void
  ) {
    this.connection = connection;
    this.publicKey = publicKey;
    this.callback = callback;
  }

  /**
   * Start subscription
   */
  async subscribe(): Promise<void> {
    try {
      this.subscriptionId = this.connection.onAccountChange(
        this.publicKey,
        (accountInfo) => {
          this.reconnectAttempts = 0; // Reset on successful update
          this.callback(accountInfo);
        },
        'confirmed'
      );
      
      logger.info(`Subscribed to account ${this.publicKey.toString()}`);
      
    } catch (error) {
      logger.error('Failed to subscribe:', error);
      await this.handleReconnect();
    }
  }

  /**
   * Unsubscribe
   */
  async unsubscribe(): Promise<void> {
    if (this.subscriptionId !== null) {
      await this.connection.removeAccountChangeListener(this.subscriptionId);
      this.subscriptionId = null;
      logger.info(`Unsubscribed from account ${this.publicKey.toString()}`);
    }
  }

  /**
   * Handle reconnection
   */
  private async handleReconnect(): Promise<void> {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      logger.error('Max reconnection attempts reached');
      return;
    }
    
    this.reconnectAttempts++;
    const delay = Math.pow(2, this.reconnectAttempts) * 1000;
    
    logger.info(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
    await new Promise(resolve => setTimeout(resolve, delay));
    
    await this.subscribe();
  }
}

export default {
  createConnection,
  getConnectionWithFallback,
  retryRpcCall,
  ConnectionPool,
  BatchRequestManager,
  ConnectionMonitor,
  AccountSubscription,
  getMultipleAccounts,
  RPC_ENDPOINTS
};