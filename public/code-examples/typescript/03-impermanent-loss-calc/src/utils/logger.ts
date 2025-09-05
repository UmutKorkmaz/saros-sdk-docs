/**
 * Logger utility for IL calculator
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

// Create custom format for IL calculations
const customFormat = winston.format.combine(
  winston.format.timestamp({ format: 'YYYY-MM-DD HH:mm:ss.SSS' }),
  winston.format.errors({ stack: true }),
  winston.format.splat(),
  winston.format.json()
);

// Console format with colors and IL-specific structure
const consoleFormat = winston.format.combine(
  winston.format.colorize({ all: true }),
  winston.format.timestamp({ format: 'HH:mm:ss' }),
  winston.format.printf(({ timestamp, level, message, ...meta }) => {
    let output = `${timestamp} [${level}]: ${message}`;
    
    // Add IL-specific metadata
    if (Object.keys(meta).length > 0) {
      if (meta.positionId) {
        output += ` | Position: ${meta.positionId}`;
      }
      if (meta.ilPercent !== undefined) {
        output += ` | IL: ${meta.ilPercent}%`;
      }
      if (meta.feeReturn !== undefined) {
        output += ` | Fee Return: ${meta.feeReturn}%`;
      }
      if (meta.netReturn !== undefined) {
        output += ` | Net: ${meta.netReturn}%`;
      }
      if (meta.poolType) {
        output += ` | Type: ${meta.poolType}`;
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
  defaultMeta: { service: 'il-calculator' },
  transports: [
    // Console transport
    new winston.transports.Console({
      format: consoleFormat,
      handleExceptions: true
    }),
    
    // File transport for all logs
    new winston.transports.File({
      filename: path.join(logsDir, 'il-calculator.log'),
      maxsize: 10485760, // 10MB
      maxFiles: 10,
      handleExceptions: true
    }),
    
    // Separate file for calculation results
    new winston.transports.File({
      filename: path.join(logsDir, 'il-calculations.log'),
      level: 'info',
      maxsize: 5242880, // 5MB
      maxFiles: 5,
      format: winston.format.combine(
        winston.format.timestamp(),
        winston.format.json(),
        winston.format.printf((info) => {
          // Only log calculation-related events
          const message = String(info.message || '');
          if (message.includes('IL') || 
              message.includes('calculation') ||
              message.includes('analysis')) {
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

// Add IL-specific logging functions
export function logILCalculation(
  positionId: string,
  ilPercent: number,
  feeReturn: number,
  netReturn: number,
  poolType: 'AMM' | 'DLMM',
  details: any
): void {
  const message = `📊 IL Analysis Complete: ${positionId}`;
  
  logger.info(message, {
    positionId,
    ilPercent: parseFloat(ilPercent.toFixed(4)),
    feeReturn: parseFloat(feeReturn.toFixed(4)),
    netReturn: parseFloat(netReturn.toFixed(4)),
    poolType,
    ...details,
    timestamp: new Date().toISOString()
  });
}

// Log position monitoring
export function logPositionMonitoring(
  positionId: string,
  currentValue: number,
  ilLoss: number,
  status: 'active' | 'out_of_range' | 'closed'
): void {
  logger.info('🔍 Position Monitoring', {
    positionId,
    currentValue,
    ilLoss,
    status,
    timestamp: new Date().toISOString()
  });
}

// Log fee analysis
export function logFeeAnalysis(
  positionId: string,
  feesEarned: number,
  feeAPR: number,
  period: string
): void {
  logger.info('💰 Fee Analysis', {
    positionId,
    feesEarned,
    feeAPR,
    period,
    timestamp: new Date().toISOString()
  });
}

// Log DLMM-specific analysis
export function logDLMMAnalysis(
  positionId: string,
  binRange: { lower: number; upper: number },
  inRange: boolean,
  liquidityDistribution: any[],
  concentrationRatio: number
): void {
  logger.info('🎯 DLMM Analysis', {
    positionId,
    binRange,
    inRange,
    liquidityDistribution: liquidityDistribution.length,
    concentrationRatio,
    timestamp: new Date().toISOString()
  });
}

// Log comparison analysis
export function logComparisonAnalysis(
  ammResult: any,
  dlmmResult: any,
  improvement: number
): void {
  logger.info('⚖️ AMM vs DLMM Comparison', {
    ammIL: ammResult.impermanentLoss,
    dlmmIL: dlmmResult.impermanentLoss,
    ammFees: ammResult.fees,
    dlmmFees: dlmmResult.fees,
    improvement: parseFloat(improvement.toFixed(2)),
    timestamp: new Date().toISOString()
  });
}

// Create CSV logger for IL analytics
export class ILCSVLogger {
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

// Create IL analytics CSV logger
export const ilAnalyticsCSV = new ILCSVLogger(
  'il-analytics.csv',
  ['timestamp', 'positionId', 'poolType', 'ilPercent', 'feeReturn', 'netReturn', 'duration']
);

// Log performance metrics
export function logPerformance(operation: string, startTime: number): void {
  const duration = Date.now() - startTime;
  logger.info(`⏱️ ${operation} completed in ${duration}ms`);
}

// Log error with context
export function logError(
  error: Error,
  context?: { [key: string]: any }
): void {
  logger.error(`❌ ${error.message}`, {
    stack: error.stack,
    ...context,
    timestamp: new Date().toISOString()
  });
}

// Log report generation
export function logReportGeneration(
  reportType: string,
  filename: string,
  positionsAnalyzed: number
): void {
  logger.info(`📄 Report Generated: ${reportType}`, {
    filename,
    positionsAnalyzed,
    timestamp: new Date().toISOString()
  });
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