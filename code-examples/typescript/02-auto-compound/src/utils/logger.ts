/**
 * Logger utility for auto-compound operations
 */

import winston from 'winston';
import path from 'path';
import fs from 'fs';

// Ensure logs directory exists
const logsDir = path.join(process.cwd(), 'logs');
if (!fs.existsSync(logsDir)) {
  fs.mkdirSync(logsDir, { recursive: true });
}

// Define log levels
const levels = {
  error: 0,
  warn: 1,
  info: 2,
  debug: 3,
  trace: 4
};

// Define colors for console output
const colors = {
  error: 'red',
  warn: 'yellow',
  info: 'green',
  debug: 'blue',
  trace: 'gray'
};

winston.addColors(colors);

// Create custom format for auto-compound logs
const customFormat = winston.format.combine(
  winston.format.timestamp({ format: 'YYYY-MM-DD HH:mm:ss.SSS' }),
  winston.format.errors({ stack: true }),
  winston.format.splat(),
  winston.format.json()
);

// Console format with colors and structure
const consoleFormat = winston.format.combine(
  winston.format.colorize({ all: true }),
  winston.format.timestamp({ format: 'HH:mm:ss' }),
  winston.format.printf(({ timestamp, level, message, ...meta }) => {
    let output = `${timestamp} [${level}]: ${message}`;
    
    // Add metadata if present
    if (Object.keys(meta).length > 0) {
      // Format specific metadata fields
      if (meta.pool) {
        output += ` | Pool: ${meta.pool}`;
      }
      if (meta.strategy) {
        output += ` | Strategy: ${meta.strategy}`;
      }
      if (meta.rewards) {
        output += ` | Rewards: ${meta.rewards}`;
      }
      if (meta.apy) {
        output += ` | APY: ${meta.apy}%`;
      }
      if (meta.gas) {
        output += ` | Gas: ${meta.gas} SOL`;
      }
    }
    
    return output;
  })
);

// Create logger instance
export const logger = winston.createLogger({
  levels,
  level: process.env.LOG_LEVEL || 'info',
  format: customFormat,
  defaultMeta: { service: 'auto-compound' },
  transports: [
    // Console transport
    new winston.transports.Console({
      format: consoleFormat,
      handleExceptions: true
    }),
    
    // File transport for all logs
    new winston.transports.File({
      filename: path.join(logsDir, 'auto-compound.log'),
      maxsize: 10485760, // 10MB
      maxFiles: 10,
      handleExceptions: true
    }),
    
    // Separate file for compound events
    new winston.transports.File({
      filename: path.join(logsDir, 'compound-events.log'),
      level: 'info',
      maxsize: 5242880, // 5MB
      maxFiles: 5,
      format: winston.format.combine(
        winston.format.timestamp(),
        winston.format.json(),
        winston.format.printf((info) => {
          // Only log compound-related events
          if (info.message.includes('compound') || 
              info.message.includes('harvest') ||
              info.message.includes('reinvest')) {
            return JSON.stringify(info);
          }
          return '';
        })
      )
    }),
    
    // Separate file for errors
    new winston.transports.File({
      filename: path.join(logsDir, 'error.log'),
      level: 'error',
      maxsize: 5242880, // 5MB
      maxFiles: 5
    }),
    
    // Performance metrics log
    new winston.transports.File({
      filename: path.join(logsDir, 'performance.log'),
      format: winston.format.combine(
        winston.format.timestamp(),
        winston.format.json()
      ),
      maxsize: 5242880, // 5MB
      maxFiles: 3
    })
  ],
  
  // Handle uncaught exceptions
  exitOnError: false
});

// Add compound-specific logging functions
export function logCompoundEvent(
  type: 'start' | 'success' | 'failed' | 'skipped',
  pool: string,
  details: any
): void {
  const message = `Compound ${type}: ${pool}`;
  
  switch (type) {
    case 'start':
      logger.info(message, { pool, ...details });
      break;
    case 'success':
      logger.info(`✅ ${message}`, { pool, ...details });
      break;
    case 'failed':
      logger.error(`❌ ${message}`, { pool, ...details });
      break;
    case 'skipped':
      logger.debug(`⏭️ ${message}`, { pool, ...details });
      break;
  }
}

// Log yield performance
export function logYieldPerformance(
  strategy: string,
  apy: number,
  totalValue: number,
  rewards: number
): void {
  logger.info('📊 Yield Performance', {
    strategy,
    apy,
    totalValue,
    rewards,
    timestamp: new Date().toISOString()
  });
}

// Log gas metrics
export function logGasMetrics(
  operation: string,
  gasUsed: number,
  gasPrice: number,
  success: boolean
): void {
  const level = gasUsed > 0.01 ? 'warn' : 'info';
  logger[level](`⛽ Gas: ${operation}`, {
    gasUsed,
    gasPrice,
    success,
    costUSD: gasUsed * 50 // Assuming SOL = $50
  });
}

// Log rebalance event
export function logRebalance(
  before: any,
  after: any,
  gasUsed: number
): void {
  logger.info('🔄 Portfolio Rebalanced', {
    before,
    after,
    gasUsed,
    timestamp: new Date().toISOString()
  });
}

// Create CSV logger for analytics
export class CSVLogger {
  private stream: fs.WriteStream;
  private headers: string[];
  
  constructor(filename: string, headers: string[]) {
    const filepath = path.join(logsDir, filename);
    this.headers = headers;
    
    // Check if file exists
    const exists = fs.existsSync(filepath);
    
    // Create write stream
    this.stream = fs.createWriteStream(filepath, { flags: 'a' });
    
    // Write headers if new file
    if (!exists) {
      this.stream.write(headers.join(',') + '\n');
    }
  }
  
  log(data: any[]): void {
    this.stream.write(data.join(',') + '\n');
  }
  
  close(): void {
    this.stream.end();
  }
}

// Create performance CSV logger
export const performanceCSV = new CSVLogger(
  'performance.csv',
  ['timestamp', 'pool', 'strategy', 'apy', 'totalValue', 'rewards', 'gasUsed']
);

// Log summary statistics
export function logSummary(stats: {
  totalCompounds: number;
  successRate: number;
  totalRewards: number;
  totalGas: number;
  netProfit: number;
}): void {
  const summary = `
╔════════════════════════════════════════════╗
║          AUTO-COMPOUND SUMMARY             ║
╠════════════════════════════════════════════╣
║ Total Compounds: ${stats.totalCompounds.toString().padEnd(26)}║
║ Success Rate: ${(stats.successRate + '%').padEnd(29)}║
║ Total Rewards: $${stats.totalRewards.toFixed(2).padEnd(27)}║
║ Total Gas: ${(stats.totalGas + ' SOL').padEnd(32)}║
║ Net Profit: $${stats.netProfit.toFixed(2).padEnd(30)}║
╚════════════════════════════════════════════╝
  `;
  
  logger.info(summary);
}

// Export log levels for external use
export const LogLevels = {
  ERROR: 'error',
  WARN: 'warn',
  INFO: 'info',
  DEBUG: 'debug',
  TRACE: 'trace'
} as const;

export default logger;