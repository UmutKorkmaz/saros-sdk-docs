use anyhow::Result;
use log::{error, info, warn};
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;

use crate::types::{AutoCompoundConfig, NotificationEvent};

/// Service for sending notifications about compound events
pub struct NotificationService {
    client: Client,
    webhook_url: Option<String>,
    enabled: bool,
}

impl NotificationService {
    pub fn new(config: &AutoCompoundConfig) -> Self {
        Self {
            client: Client::new(),
            webhook_url: config.webhook_url.clone(),
            enabled: config.enable_notifications,
        }
    }

    /// Send a notification event
    pub async fn send_notification(&self, event: NotificationEvent) {
        if !self.enabled {
            return;
        }

        info!("🔔 Sending notification: {}", event.event_type);

        // Send webhook notification
        if let Some(webhook_url) = &self.webhook_url {
            if let Err(e) = self.send_webhook_notification(webhook_url, &event).await {
                error!("Failed to send webhook notification: {}", e);
            }
        }

        // Log notification for file-based monitoring
        self.log_notification(&event);

        // Could add other notification channels here:
        // - Discord webhook
        // - Slack webhook  
        // - Email notifications
        // - SMS notifications
        // - Push notifications
    }

    /// Send webhook notification
    async fn send_webhook_notification(&self, webhook_url: &str, event: &NotificationEvent) -> Result<()> {
        let payload = json!({
            "event_type": event.event_type.to_string(),
            "pool_address": event.pool_address,
            "message": event.message,
            "data": event.data,
            "timestamp": event.timestamp.to_rfc3339(),
            "source": "saros-auto-compound-bot"
        });

        let response = self.client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "Saros-Auto-Compound-Bot/1.0")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if response.status().is_success() {
            info!("✅ Webhook notification sent successfully");
        } else {
            warn!("⚠️ Webhook notification failed with status: {}", response.status());
        }

        Ok(())
    }

    /// Log notification to console/file
    fn log_notification(&self, event: &NotificationEvent) {
        match event.event_type {
            crate::types::NotificationEventType::CompoundStarted => {
                info!("🚀 {}: {}", event.event_type, event.message);
            }
            crate::types::NotificationEventType::CompoundSuccess => {
                info!("✅ {}: {}", event.event_type, event.message);
            }
            crate::types::NotificationEventType::CompoundFailed => {
                error!("❌ {}: {}", event.event_type, event.message);
            }
            crate::types::NotificationEventType::CompoundStopped => {
                info!("🛑 {}: {}", event.event_type, event.message);
            }
            crate::types::NotificationEventType::HighGasPrice => {
                warn!("⛽ {}: {}", event.event_type, event.message);
            }
            crate::types::NotificationEventType::LowRewards => {
                info!("💰 {}: {}", event.event_type, event.message);
            }
            crate::types::NotificationEventType::EmergencyStop => {
                error!("🚨 {}: {}", event.event_type, event.message);
            }
            crate::types::NotificationEventType::PositionChanged => {
                info!("📊 {}: {}", event.event_type, event.message);
            }
            crate::types::NotificationEventType::APYUpdate => {
                info!("📈 {}: {}", event.event_type, event.message);
            }
        }
    }

    /// Send Discord webhook notification (optional enhancement)
    pub async fn send_discord_notification(&self, webhook_url: &str, event: &NotificationEvent) -> Result<()> {
        let color = match event.event_type {
            crate::types::NotificationEventType::CompoundSuccess => 0x00ff00, // Green
            crate::types::NotificationEventType::CompoundFailed => 0xff0000,  // Red
            crate::types::NotificationEventType::CompoundStarted => 0x0099ff, // Blue
            crate::types::NotificationEventType::EmergencyStop => 0xff6600,   // Orange
            _ => 0x888888, // Gray
        };

        let embed = json!({
            "embeds": [{
                "title": format!("🤖 Saros Auto-Compound Bot"),
                "description": event.message,
                "color": color,
                "fields": [
                    {
                        "name": "Pool Address",
                        "value": format!("`{}`", event.pool_address),
                        "inline": false
                    },
                    {
                        "name": "Event Type",
                        "value": event.event_type.to_string(),
                        "inline": true
                    },
                    {
                        "name": "Timestamp",
                        "value": event.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                        "inline": true
                    }
                ],
                "footer": {
                    "text": "Saros Finance Auto-Compound Bot"
                },
                "timestamp": event.timestamp.to_rfc3339()
            }]
        });

        let response = self.client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&embed)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if response.status().is_success() {
            info!("✅ Discord notification sent successfully");
        } else {
            warn!("⚠️ Discord notification failed with status: {}", response.status());
        }

        Ok(())
    }

    /// Send Slack webhook notification (optional enhancement)
    pub async fn send_slack_notification(&self, webhook_url: &str, event: &NotificationEvent) -> Result<()> {
        let emoji = match event.event_type {
            crate::types::NotificationEventType::CompoundSuccess => ":white_check_mark:",
            crate::types::NotificationEventType::CompoundFailed => ":x:",
            crate::types::NotificationEventType::CompoundStarted => ":rocket:",
            crate::types::NotificationEventType::EmergencyStop => ":warning:",
            _ => ":information_source:",
        };

        let payload = json!({
            "text": format!("{} Saros Auto-Compound Bot", emoji),
            "blocks": [
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": format!("*{}*\n{}", event.event_type, event.message)
                    }
                },
                {
                    "type": "section",
                    "fields": [
                        {
                            "type": "mrkdwn",
                            "text": format!("*Pool Address:*\n`{}`", event.pool_address)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*Timestamp:*\n{}", event.timestamp.format("%Y-%m-%d %H:%M:%S UTC"))
                        }
                    ]
                }
            ]
        });

        let response = self.client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if response.status().is_success() {
            info!("✅ Slack notification sent successfully");
        } else {
            warn!("⚠️ Slack notification failed with status: {}", response.status());
        }

        Ok(())
    }

    /// Create a formatted notification message
    pub fn format_compound_success_message(
        &self,
        pool_address: &str,
        rewards_harvested: f64,
        amount_reinvested: f64,
        new_position_value: f64,
        gas_used: f64,
    ) -> String {
        format!(
            "Compound successful for pool {}!\n\
             💰 Rewards harvested: {:.6} tokens\n\
             🔄 Amount reinvested: {:.6} tokens\n\
             📊 New position value: {:.6} tokens\n\
             ⛽ Gas used: {:.6} SOL",
            pool_address, rewards_harvested, amount_reinvested, new_position_value, gas_used
        )
    }

    /// Create a formatted notification for failed compound
    pub fn format_compound_failed_message(
        &self,
        pool_address: &str,
        error: &str,
    ) -> String {
        format!(
            "Compound failed for pool {}!\n\
             ❌ Error: {}",
            pool_address, error
        )
    }

    /// Create a formatted notification for gas price warning
    pub fn format_high_gas_message(
        &self,
        pool_address: &str,
        current_gas_price: f64,
        max_gas_price: f64,
    ) -> String {
        format!(
            "High gas price detected for pool {}!\n\
             ⛽ Current gas price: {:.6} SOL\n\
             📊 Max allowed: {:.6} SOL\n\
             ⏱️ Waiting for better conditions...",
            pool_address, current_gas_price, max_gas_price
        )
    }
}