/**
 * Notification service for auto-compound events
 */

import axios from 'axios';
import { logger } from './logger';

export interface NotificationConfig {
  webhookUrl?: string;
  emailEnabled?: boolean;
  emailAddress?: string;
  discordWebhook?: string;
  telegramBotToken?: string;
  telegramChatId?: string;
  slackWebhook?: string;
}

export interface NotificationPayload {
  type: string;
  pool?: string;
  strategy?: string;
  interval?: number;
  rewards?: number;
  reinvested?: number;
  newPosition?: number;
  error?: string;
  statistics?: any;
  timestamp?: Date;
}

export class NotificationService {
  private config: NotificationConfig;
  private notificationQueue: NotificationPayload[] = [];
  private processing: boolean = false;

  constructor(config?: NotificationConfig) {
    this.config = config || this.loadConfigFromEnv();
    
    if (this.isConfigured()) {
      logger.info('NotificationService initialized');
    } else {
      logger.debug('NotificationService not configured');
    }
  }

  /**
   * Send notification
   */
  async send(payload: NotificationPayload): Promise<void> {
    if (!this.isConfigured()) {
      return;
    }

    // Add timestamp if not present
    if (!payload.timestamp) {
      payload.timestamp = new Date();
    }

    // Add to queue
    this.notificationQueue.push(payload);
    
    // Process queue
    await this.processQueue();
  }

  /**
   * Send compound success notification
   */
  async sendCompoundSuccess(
    pool: string,
    rewards: number,
    reinvested: number,
    newPosition: number
  ): Promise<void> {
    const message = this.formatCompoundSuccess(pool, rewards, reinvested, newPosition);
    
    await this.send({
      type: 'COMPOUND_SUCCESS',
      pool,
      rewards,
      reinvested,
      newPosition
    });
  }

  /**
   * Send compound failure notification
   */
  async sendCompoundFailure(
    pool: string,
    error: string
  ): Promise<void> {
    const message = this.formatCompoundFailure(pool, error);
    
    await this.send({
      type: 'COMPOUND_FAILED',
      pool,
      error
    });
  }

  /**
   * Send daily summary
   */
  async sendDailySummary(stats: {
    totalCompounds: number;
    successfulCompounds: number;
    totalRewards: number;
    totalReinvested: number;
    totalGas: number;
    netProfit: number;
  }): Promise<void> {
    const message = this.formatDailySummary(stats);
    
    await this.send({
      type: 'DAILY_SUMMARY',
      statistics: stats
    });
  }

  /**
   * Process notification queue
   */
  private async processQueue(): Promise<void> {
    if (this.processing || this.notificationQueue.length === 0) {
      return;
    }

    this.processing = true;

    while (this.notificationQueue.length > 0) {
      const payload = this.notificationQueue.shift()!;
      
      try {
        // Send to configured channels
        const promises: Promise<void>[] = [];
        
        if (this.config.webhookUrl) {
          promises.push(this.sendWebhook(payload));
        }
        
        if (this.config.discordWebhook) {
          promises.push(this.sendDiscord(payload));
        }
        
        if (this.config.telegramBotToken && this.config.telegramChatId) {
          promises.push(this.sendTelegram(payload));
        }
        
        if (this.config.slackWebhook) {
          promises.push(this.sendSlack(payload));
        }
        
        await Promise.all(promises);
        
      } catch (error) {
        logger.error('Failed to send notification:', error);
      }
    }

    this.processing = false;
  }

  /**
   * Send webhook notification
   */
  private async sendWebhook(payload: NotificationPayload): Promise<void> {
    if (!this.config.webhookUrl) return;

    try {
      await axios.post(this.config.webhookUrl, payload, {
        headers: { 'Content-Type': 'application/json' },
        timeout: 5000
      });
    } catch (error) {
      logger.error('Webhook notification failed:', error);
    }
  }

  /**
   * Send Discord notification
   */
  private async sendDiscord(payload: NotificationPayload): Promise<void> {
    if (!this.config.discordWebhook) return;

    const embed = this.createDiscordEmbed(payload);

    try {
      await axios.post(this.config.discordWebhook, { embeds: [embed] }, {
        headers: { 'Content-Type': 'application/json' },
        timeout: 5000
      });
    } catch (error) {
      logger.error('Discord notification failed:', error);
    }
  }

  /**
   * Send Telegram notification
   */
  private async sendTelegram(payload: NotificationPayload): Promise<void> {
    if (!this.config.telegramBotToken || !this.config.telegramChatId) return;

    const message = this.formatTelegramMessage(payload);
    const url = `https://api.telegram.org/bot${this.config.telegramBotToken}/sendMessage`;

    try {
      await axios.post(url, {
        chat_id: this.config.telegramChatId,
        text: message,
        parse_mode: 'HTML'
      }, {
        timeout: 5000
      });
    } catch (error) {
      logger.error('Telegram notification failed:', error);
    }
  }

  /**
   * Send Slack notification
   */
  private async sendSlack(payload: NotificationPayload): Promise<void> {
    if (!this.config.slackWebhook) return;

    const blocks = this.createSlackBlocks(payload);

    try {
      await axios.post(this.config.slackWebhook, { blocks }, {
        headers: { 'Content-Type': 'application/json' },
        timeout: 5000
      });
    } catch (error) {
      logger.error('Slack notification failed:', error);
    }
  }

  /**
   * Create Discord embed
   */
  private createDiscordEmbed(payload: NotificationPayload): any {
    const color = payload.type.includes('SUCCESS') ? 0x00ff00 :
                  payload.type.includes('FAILED') ? 0xff0000 : 0x0099ff;

    const embed: any = {
      title: this.getTitle(payload.type),
      color,
      timestamp: payload.timestamp?.toISOString(),
      fields: []
    };

    if (payload.pool) {
      embed.fields.push({
        name: 'Pool',
        value: payload.pool.slice(0, 8) + '...',
        inline: true
      });
    }

    if (payload.rewards) {
      embed.fields.push({
        name: 'Rewards',
        value: `$${payload.rewards.toFixed(2)}`,
        inline: true
      });
    }

    if (payload.reinvested) {
      embed.fields.push({
        name: 'Reinvested',
        value: `$${payload.reinvested.toFixed(2)}`,
        inline: true
      });
    }

    if (payload.error) {
      embed.fields.push({
        name: 'Error',
        value: payload.error,
        inline: false
      });
    }

    return embed;
  }

  /**
   * Format Telegram message
   */
  private formatTelegramMessage(payload: NotificationPayload): string {
    let message = `<b>${this.getTitle(payload.type)}</b>\n\n`;

    if (payload.pool) {
      message += `📍 Pool: <code>${payload.pool.slice(0, 8)}...</code>\n`;
    }

    if (payload.rewards) {
      message += `💰 Rewards: $${payload.rewards.toFixed(2)}\n`;
    }

    if (payload.reinvested) {
      message += `♻️ Reinvested: $${payload.reinvested.toFixed(2)}\n`;
    }

    if (payload.newPosition) {
      message += `📊 New Position: $${payload.newPosition.toFixed(2)}\n`;
    }

    if (payload.error) {
      message += `❌ Error: ${payload.error}\n`;
    }

    if (payload.statistics) {
      message += '\n<b>Statistics:</b>\n';
      message += `• Total Compounds: ${payload.statistics.totalCompounds}\n`;
      message += `• Success Rate: ${payload.statistics.successRate}%\n`;
      message += `• Net Profit: $${payload.statistics.netProfit}\n`;
    }

    return message;
  }

  /**
   * Create Slack blocks
   */
  private createSlackBlocks(payload: NotificationPayload): any[] {
    const blocks: any[] = [
      {
        type: 'header',
        text: {
          type: 'plain_text',
          text: this.getTitle(payload.type)
        }
      }
    ];

    const fields: any[] = [];

    if (payload.pool) {
      fields.push({
        type: 'mrkdwn',
        text: `*Pool:* ${payload.pool.slice(0, 8)}...`
      });
    }

    if (payload.rewards) {
      fields.push({
        type: 'mrkdwn',
        text: `*Rewards:* $${payload.rewards.toFixed(2)}`
      });
    }

    if (fields.length > 0) {
      blocks.push({
        type: 'section',
        fields
      });
    }

    if (payload.error) {
      blocks.push({
        type: 'section',
        text: {
          type: 'mrkdwn',
          text: `⚠️ *Error:* ${payload.error}`
        }
      });
    }

    return blocks;
  }

  /**
   * Format compound success message
   */
  private formatCompoundSuccess(
    pool: string,
    rewards: number,
    reinvested: number,
    newPosition: number
  ): string {
    return `✅ Auto-Compound Success!
Pool: ${pool.slice(0, 8)}...
Rewards Harvested: $${rewards.toFixed(2)}
Amount Reinvested: $${reinvested.toFixed(2)}
New Position Value: $${newPosition.toFixed(2)}`;
  }

  /**
   * Format compound failure message
   */
  private formatCompoundFailure(pool: string, error: string): string {
    return `❌ Auto-Compound Failed
Pool: ${pool.slice(0, 8)}...
Error: ${error}`;
  }

  /**
   * Format daily summary
   */
  private formatDailySummary(stats: any): string {
    return `📊 Daily Auto-Compound Summary
Total Compounds: ${stats.totalCompounds}
Successful: ${stats.successfulCompounds}
Total Rewards: $${stats.totalRewards.toFixed(2)}
Total Reinvested: $${stats.totalReinvested.toFixed(2)}
Gas Spent: ${stats.totalGas.toFixed(4)} SOL
Net Profit: $${stats.netProfit.toFixed(2)}`;
  }

  /**
   * Get title for notification type
   */
  private getTitle(type: string): string {
    const titles: { [key: string]: string } = {
      'COMPOUND_SUCCESS': '✅ Auto-Compound Success',
      'COMPOUND_FAILED': '❌ Auto-Compound Failed',
      'COMPOUND_STARTED': '🚀 Auto-Compound Started',
      'COMPOUND_STOPPED': '⏹️ Auto-Compound Stopped',
      'DAILY_SUMMARY': '📊 Daily Summary',
      'REBALANCE_COMPLETE': '🔄 Rebalance Complete',
      'HIGH_GAS_WARNING': '⚠️ High Gas Warning',
      'LOW_BALANCE_WARNING': '⚠️ Low Balance Warning'
    };

    return titles[type] || type;
  }

  /**
   * Load configuration from environment
   */
  private loadConfigFromEnv(): NotificationConfig {
    return {
      webhookUrl: process.env.WEBHOOK_URL,
      emailEnabled: process.env.EMAIL_NOTIFICATIONS === 'true',
      emailAddress: process.env.EMAIL_ADDRESS,
      discordWebhook: process.env.DISCORD_WEBHOOK,
      telegramBotToken: process.env.TELEGRAM_BOT_TOKEN,
      telegramChatId: process.env.TELEGRAM_CHAT_ID,
      slackWebhook: process.env.SLACK_WEBHOOK
    };
  }

  /**
   * Check if notifications are configured
   */
  private isConfigured(): boolean {
    return !!(
      this.config.webhookUrl ||
      this.config.discordWebhook ||
      (this.config.telegramBotToken && this.config.telegramChatId) ||
      this.config.slackWebhook
    );
  }

  /**
   * Test notification configuration
   */
  async testNotification(): Promise<void> {
    logger.info('Testing notification configuration...');
    
    await this.send({
      type: 'TEST',
      pool: 'TEST_POOL_ADDRESS',
      rewards: 100,
      reinvested: 95,
      newPosition: 1095
    });
    
    logger.info('Test notification sent');
  }
}