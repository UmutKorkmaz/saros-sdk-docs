/**
 * ILMonitor - Real-time IL monitoring and alerts
 */

import { logger } from './utils/logger';

export interface MonitorConfig {
  poolAddress: string;
  positionId: string;
  checkInterval?: number;
  alerts?: {
    warning: number;
    critical: number;
    maxLoss?: number;
  };
  onAlert?: (alert: Alert) => void;
}

export interface Alert {
  level: 'warning' | 'critical';
  currentIL: number;
  priceChange: number;
  timestamp: Date;
  message: string;
}

export class ILMonitor {
  private monitoring: Map<string, NodeJS.Timeout> = new Map();
  
  constructor() {
    logger.info('ILMonitor initialized');
  }

  /**
   * Start monitoring a position
   */
  async startMonitoring(config: MonitorConfig): Promise<void> {
    const { 
      poolAddress, 
      positionId,
      checkInterval = 60000,
      alerts = { warning: 5, critical: 10 },
      onAlert
    } = config;
    
    const monitorKey = `${poolAddress}-${positionId}`;
    
    // Stop existing monitor if any
    if (this.monitoring.has(monitorKey)) {
      this.stopMonitoring(monitorKey);
    }
    
    logger.info(`Starting IL monitoring for position ${positionId}`);
    
    const interval = setInterval(async () => {
      try {
        // Fetch current IL (simulated for demo)
        const currentIL = Math.random() * 15; // 0-15% IL
        const priceChange = Math.random() * 100 - 50; // -50% to +50%
        
        // Check alert thresholds
        if (currentIL >= alerts.critical) {
          const alert: Alert = {
            level: 'critical',
            currentIL,
            priceChange,
            timestamp: new Date(),
            message: `Critical IL level: ${currentIL.toFixed(2)}%`
          };
          
          logger.error(`🚨 ${alert.message}`);
          if (onAlert) onAlert(alert);
          
        } else if (currentIL >= alerts.warning) {
          const alert: Alert = {
            level: 'warning',
            currentIL,
            priceChange,
            timestamp: new Date(),
            message: `Warning IL level: ${currentIL.toFixed(2)}%`
          };
          
          logger.warn(`⚠️ ${alert.message}`);
          if (onAlert) onAlert(alert);
        }
        
        // Check max loss if specified
        if (alerts.maxLoss) {
          const lossInUSD = currentIL * 100; // Simplified calculation
          if (lossInUSD > alerts.maxLoss) {
            const alert: Alert = {
              level: 'critical',
              currentIL,
              priceChange,
              timestamp: new Date(),
              message: `Max loss exceeded: $${lossInUSD.toFixed(2)}`
            };
            
            logger.error(`💸 ${alert.message}`);
            if (onAlert) onAlert(alert);
          }
        }
        
      } catch (error) {
        logger.error('Error in IL monitoring:', error);
      }
    }, checkInterval);
    
    this.monitoring.set(monitorKey, interval);
  }

  /**
   * Stop monitoring a position
   */
  stopMonitoring(monitorKey: string): void {
    const interval = this.monitoring.get(monitorKey);
    if (interval) {
      clearInterval(interval);
      this.monitoring.delete(monitorKey);
      logger.info(`Stopped monitoring for ${monitorKey}`);
    }
  }

  /**
   * Stop all monitoring
   */
  stopAll(): void {
    for (const [key, interval] of this.monitoring.entries()) {
      clearInterval(interval);
    }
    this.monitoring.clear();
    logger.info('All monitoring stopped');
  }
}