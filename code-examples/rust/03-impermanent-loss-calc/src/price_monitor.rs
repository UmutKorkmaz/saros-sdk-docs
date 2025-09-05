//! Price tracking and volatility monitoring for DLMM pools

use anyhow::Result;
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use indexmap::IndexMap;
use log::{debug, info, warn};
use reqwest::Client;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::{BTreeMap, VecDeque};
use tokio::time::{sleep, Duration};

use saros_dlmm_sdk::DLMMClient as MockSarosClient;
use crate::types::{PriceDataPoint, ILNotificationEvent, ILEventType, NotificationSeverity};

/// External price API response structure
#[derive(Debug, Deserialize)]
struct PriceApiResponse {
    data: Vec<PriceApiData>,
}

#[derive(Debug, Deserialize)]
struct PriceApiData {
    id: String,
    symbol: String,
    name: String,
    current_price: f64,
    price_change_24h: f64,
    price_change_percentage_24h: f64,
    market_cap: Option<f64>,
    total_volume: Option<f64>,
    last_updated: String,
}

/// Historical price data from external API
#[derive(Debug, Deserialize)]
struct HistoricalPriceResponse {
    prices: Vec<[f64; 2]>, // [timestamp, price]
    market_caps: Option<Vec<[f64; 2]>>,
    total_volumes: Option<Vec<[f64; 2]>>,
}

/// Token metadata for price tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenMetadata {
    pub mint: Pubkey,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub coingecko_id: Option<String>,
    pub price_feeds: Vec<String>, // Pyth, Chainlink, etc.
}

/// Price monitor for tracking token prices and volatility
pub struct PriceMonitor {
    client: MockSarosClient,
    http_client: Client,
    price_cache: BTreeMap<Pubkey, (Decimal, DateTime<Utc>)>,
    historical_cache: BTreeMap<(Pubkey, u32), Vec<PriceDataPoint>>, // (token, days) -> data
    token_metadata: BTreeMap<Pubkey, TokenMetadata>,
    price_history: BTreeMap<Pubkey, VecDeque<PriceDataPoint>>,
    volatility_cache: BTreeMap<Pubkey, (Decimal, DateTime<Utc>)>,
    notification_thresholds: BTreeMap<Pubkey, Decimal>, // Price change thresholds
    cache_ttl_secs: u64,
    max_history_points: usize,
}

impl PriceMonitor {
    /// Create a new price monitor
    pub async fn new() -> Result<Self> {
        let rpc_url = std::env::var("RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        
        let client = MockSarosClient::new(&rpc_url)?;
        let http_client = Client::new();
        
        let mut monitor = Self {
            client,
            http_client,
            price_cache: BTreeMap::new(),
            historical_cache: BTreeMap::new(),
            token_metadata: BTreeMap::new(),
            price_history: BTreeMap::new(),
            volatility_cache: BTreeMap::new(),
            notification_thresholds: BTreeMap::new(),
            cache_ttl_secs: 30, // 30 seconds cache
            max_history_points: 10000,
        };
        
        // Initialize with common token metadata
        monitor.initialize_token_metadata().await?;
        
        Ok(monitor)
    }

    /// Get current prices for two tokens
    pub async fn get_current_prices(
        &mut self,
        token_x: Pubkey,
        token_y: Pubkey,
    ) -> Result<(Decimal, Decimal)> {
        let price_x = self.get_token_price(token_x).await?;
        let price_y = self.get_token_price(token_y).await?;
        
        debug!("Current prices - {}: ${:.6}, {}: ${:.6}", 
               token_x, price_x, token_y, price_y);
        
        Ok((price_x, price_y))
    }

    /// Get current price for a single token
    pub async fn get_token_price(&mut self, token_mint: Pubkey) -> Result<Decimal> {
        // Check cache first
        if let Some((cached_price, cached_time)) = self.price_cache.get(&token_mint) {
            if Utc::now().signed_duration_since(*cached_time).num_seconds() < self.cache_ttl_secs as i64 {
                return Ok(*cached_price);
            }
        }

        info!("Fetching current price for token {}", token_mint);
        
        let price = if let Some(metadata) = self.token_metadata.get(&token_mint) {
            // Try to get price from external API
            if let Some(coingecko_id) = &metadata.coingecko_id {
                match self.fetch_coingecko_price(coingecko_id).await {
                    Ok(price) => price,
                    Err(e) => {
                        warn!("Failed to fetch CoinGecko price for {}: {}", coingecko_id, e);
                        self.get_mock_price(&token_mint).await?
                    }
                }
            } else {
                // Try price feeds or use mock data
                self.get_mock_price(&token_mint).await?
            }
        } else {
            // Unknown token, use mock price
            warn!("Unknown token {}, using mock price", token_mint);
            self.get_mock_price(&token_mint).await?
        };

        // Cache the result
        self.price_cache.insert(token_mint, (price, Utc::now()));
        
        // Add to history
        self.add_price_to_history(token_mint, price).await?;
        
        Ok(price)
    }

    /// Get historical price data for specified number of days
    pub async fn get_historical_data(
        &mut self,
        token_x: Pubkey,
        token_y: Pubkey,
        days: u32,
    ) -> Result<Vec<PriceDataPoint>> {
        let cache_key = (token_x, days);
        
        // Check cache first
        if let Some(cached_data) = self.historical_cache.get(&cache_key) {
            if !cached_data.is_empty() {
                let latest = cached_data.last().unwrap();
                if Utc::now().signed_duration_since(latest.timestamp).num_hours() < 1 {
                    info!("Returning cached historical data for {} days", days);
                    return Ok(cached_data.clone());
                }
            }
        }

        info!("Fetching historical data for {} days", days);
        
        let historical_data = if let (Some(metadata_x), Some(metadata_y)) = 
            (self.token_metadata.get(&token_x), self.token_metadata.get(&token_y)) {
            
            // Try to get real historical data
            match self.fetch_historical_prices(metadata_x, metadata_y, days).await {
                Ok(data) => data,
                Err(e) => {
                    warn!("Failed to fetch historical data: {}, using mock data", e);
                    self.generate_mock_historical_data(token_x, token_y, days).await?
                }
            }
        } else {
            // Use mock data for unknown tokens
            self.generate_mock_historical_data(token_x, token_y, days).await?
        };

        // Cache the result
        self.historical_cache.insert(cache_key, historical_data.clone());
        
        info!("Retrieved {} historical data points", historical_data.len());
        Ok(historical_data)
    }

    /// Calculate price volatility over a specified window
    pub async fn calculate_volatility(
        &mut self,
        token_mint: Pubkey,
        window_hours: u32,
    ) -> Result<Decimal> {
        let cache_key = token_mint;
        
        // Check cache
        if let Some((cached_volatility, cached_time)) = self.volatility_cache.get(&cache_key) {
            if Utc::now().signed_duration_since(*cached_time).num_minutes() < 10 {
                return Ok(*cached_volatility);
            }
        }

        // Get price history for the token
        if let Some(history) = self.price_history.get(&token_mint) {
            let cutoff_time = Utc::now() - ChronoDuration::hours(window_hours as i64);
            
            let recent_prices: Vec<f64> = history.iter()
                .filter(|point| point.timestamp > cutoff_time)
                .map(|point| point.price_x.to_f64().unwrap_or(0.0))
                .collect();
            
            if recent_prices.len() < 2 {
                return Ok(Decimal::ZERO);
            }
            
            // Calculate returns
            let returns: Vec<f64> = recent_prices.windows(2)
                .map(|window| (window[1] / window[0] - 1.0).ln())
                .collect();
            
            if returns.is_empty() {
                return Ok(Decimal::ZERO);
            }
            
            // Calculate standard deviation of returns
            let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance = returns.iter()
                .map(|&r| (r - mean_return).powi(2))
                .sum::<f64>() / returns.len() as f64;
            
            let volatility = variance.sqrt() * (365.25 * 24.0 / window_hours as f64).sqrt(); // Annualized
            let volatility_decimal = Decimal::from_f64(volatility).unwrap_or_default();
            
            // Cache result
            self.volatility_cache.insert(cache_key, (volatility_decimal, Utc::now()));
            
            Ok(volatility_decimal)
        } else {
            Ok(Decimal::ZERO)
        }
    }

    /// Set price change notification threshold for a token
    pub fn set_notification_threshold(&mut self, token_mint: Pubkey, threshold_percentage: Decimal) {
        self.notification_thresholds.insert(token_mint, threshold_percentage);
        info!("Set notification threshold for {}: {:.2}%", 
              token_mint, threshold_percentage * Decimal::new(100, 0));
    }

    /// Check for price alerts and generate notifications
    pub async fn check_price_alerts(
        &self,
        token_mint: Pubkey,
        current_price: Decimal,
        previous_price: Decimal,
    ) -> Result<Option<ILNotificationEvent>> {
        if let Some(&threshold) = self.notification_thresholds.get(&token_mint) {
            let price_change = (current_price - previous_price) / previous_price;
            
            if price_change.abs() > threshold {
                let event_type = if price_change.abs() > threshold * Decimal::new(2, 0) {
                    ILEventType::PriceVoLatilitySpike
                } else {
                    ILEventType::HighImpermanentLoss
                };
                
                let severity = if price_change.abs() > threshold * Decimal::new(3, 0) {
                    NotificationSeverity::Emergency
                } else if price_change.abs() > threshold * Decimal::new(15, 1) {
                    NotificationSeverity::Critical
                } else {
                    NotificationSeverity::Warning
                };
                
                return Ok(Some(ILNotificationEvent {
                    event_type,
                    position_id: None,
                    pool_address: Default::default(),
                    il_percentage: price_change,
                    threshold_crossed: Some(threshold),
                    message: format!(
                        "Price alert: {} changed by {:.2}% (threshold: {:.2}%)",
                        token_mint,
                        price_change * Decimal::new(100, 0),
                        threshold * Decimal::new(100, 0)
                    ),
                    timestamp: Utc::now(),
                    severity,
                }));
            }
        }
        
        Ok(None)
    }

    /// Fetch price from CoinGecko API
    async fn fetch_coingecko_price(&self, coingecko_id: &str) -> Result<Decimal> {
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
            coingecko_id
        );
        
        let response = self.http_client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await?;
        
        let data: serde_json::Value = response.json().await?;
        
        if let Some(price) = data[coingecko_id]["usd"].as_f64() {
            Ok(Decimal::from_f64(price).unwrap_or_default())
        } else {
            Err(anyhow::anyhow!("Price not found in response"))
        }
    }

    /// Fetch historical prices from external API
    async fn fetch_historical_prices(
        &self,
        metadata_x: &TokenMetadata,
        metadata_y: &TokenMetadata,
        days: u32,
    ) -> Result<Vec<PriceDataPoint>> {
        if let (Some(id_x), Some(id_y)) = (&metadata_x.coingecko_id, &metadata_y.coingecko_id) {
            let historical_x = self.fetch_coingecko_historical(id_x, days).await?;
            let historical_y = self.fetch_coingecko_historical(id_y, days).await?;
            
            // Combine the data
            let mut combined_data = Vec::new();
            let min_len = historical_x.len().min(historical_y.len());
            
            for i in 0..min_len {
                combined_data.push(PriceDataPoint {
                    timestamp: historical_x[i].timestamp,
                    price_x: historical_x[i].price_x,
                    price_y: historical_y[i].price_x, // Use price_x from both
                    volume_24h: Decimal::new(1000000, 0), // Mock volume
                    liquidity: Decimal::new(5000000, 0), // Mock liquidity
                    active_bin_id: 0, // Mock bin ID
                });
            }
            
            Ok(combined_data)
        } else {
            Err(anyhow::anyhow!("CoinGecko IDs not available for both tokens"))
        }
    }

    /// Fetch historical data from CoinGecko for a single token
    async fn fetch_coingecko_historical(&self, coingecko_id: &str, days: u32) -> Result<Vec<PriceDataPoint>> {
        let url = format!(
            "https://api.coingecko.com/api/v3/coins/{}/market_chart?vs_currency=usd&days={}",
            coingecko_id, days
        );
        
        let response = self.http_client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;
        
        let data: HistoricalPriceResponse = response.json().await?;
        
        let mut price_points = Vec::new();
        for price_data in data.prices {
            let timestamp = DateTime::from_timestamp((price_data[0] / 1000.0) as i64, 0)
                .unwrap_or_else(|| Utc::now());
            let price = Decimal::from_f64(price_data[1]).unwrap_or_default();
            
            price_points.push(PriceDataPoint {
                timestamp,
                price_x: price,
                price_y: Decimal::new(100, 0), // Placeholder
                volume_24h: Decimal::new(1000000, 0),
                liquidity: Decimal::new(5000000, 0),
                active_bin_id: 0,
            });
        }
        
        Ok(price_points)
    }

    /// Generate mock historical data for testing
    async fn generate_mock_historical_data(
        &self,
        _token_x: Pubkey,
        _token_y: Pubkey,
        days: u32,
    ) -> Result<Vec<PriceDataPoint>> {
        let mut data = Vec::new();
        let start_time = Utc::now() - ChronoDuration::days(days as i64);
        
        // Generate realistic price movements
        let mut price_x = Decimal::new(100, 0); // Start at $100
        let mut price_y = Decimal::new(100, 0); // Start at $100
        
        for day in 0..days {
            let timestamp = start_time + ChronoDuration::days(day as i64);
            
            // Simulate daily price changes with some volatility
            let change_x = (fastrand::f64() - 0.5) * 0.1; // ±5% daily change
            let change_y = (fastrand::f64() - 0.5) * 0.08; // ±4% daily change
            
            price_x = price_x * (Decimal::ONE + Decimal::from_f64(change_x).unwrap_or_default());
            price_y = price_y * (Decimal::ONE + Decimal::from_f64(change_y).unwrap_or_default());
            
            // Ensure prices don't go negative
            price_x = price_x.max(Decimal::new(1, 0));
            price_y = price_y.max(Decimal::new(1, 0));
            
            data.push(PriceDataPoint {
                timestamp,
                price_x,
                price_y,
                volume_24h: Decimal::new(fastrand::u32(500000..2000000) as i64, 0),
                liquidity: Decimal::new(fastrand::u32(1000000..10000000) as i64, 0),
                active_bin_id: fastrand::i32(-100..100),
            });
        }
        
        Ok(data)
    }

    /// Get mock price for unknown tokens
    async fn get_mock_price(&self, token_mint: &Pubkey) -> Result<Decimal> {
        // Generate deterministic "random" price based on mint address
        let mint_bytes = token_mint.to_bytes();
        let seed = u64::from_le_bytes([
            mint_bytes[0], mint_bytes[1], mint_bytes[2], mint_bytes[3],
            mint_bytes[4], mint_bytes[5], mint_bytes[6], mint_bytes[7],
        ]);
        
        // Use seed to generate consistent price between $0.01 and $1000
        let base_price = (seed % 100000) as f64 / 100.0; // $0.00 to $999.99
        let price = base_price.max(0.01).min(1000.0); // Clamp to reasonable range
        
        Ok(Decimal::from_f64(price).unwrap_or(Decimal::new(100, 0)))
    }

    /// Add price to historical tracking
    async fn add_price_to_history(&mut self, token_mint: Pubkey, price: Decimal) -> Result<()> {
        let history = self.price_history.entry(token_mint).or_insert_with(VecDeque::new);
        
        // Keep only recent history to manage memory
        if history.len() >= self.max_history_points {
            history.pop_front();
        }
        
        history.push_back(PriceDataPoint {
            timestamp: Utc::now(),
            price_x: price,
            price_y: Decimal::ZERO, // Not used for single token tracking
            volume_24h: Decimal::ZERO,
            liquidity: Decimal::ZERO,
            active_bin_id: 0,
        });
        
        Ok(())
    }

    /// Initialize common token metadata
    async fn initialize_token_metadata(&mut self) -> Result<()> {
        // Common Solana tokens - in a real implementation, this would be loaded from a configuration file
        let tokens = vec![
            TokenMetadata {
                mint: Pubkey::try_from("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(), // USDC
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                coingecko_id: Some("usd-coin".to_string()),
                price_feeds: vec!["pyth".to_string()],
            },
            TokenMetadata {
                mint: Pubkey::try_from("So11111111111111111111111111111111111111112").unwrap(), // SOL
                symbol: "SOL".to_string(),
                name: "Solana".to_string(),
                decimals: 9,
                coingecko_id: Some("solana".to_string()),
                price_feeds: vec!["pyth".to_string(), "chainlink".to_string()],
            },
            TokenMetadata {
                mint: Pubkey::try_from("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So").unwrap(), // mSOL
                symbol: "mSOL".to_string(),
                name: "Marinade Staked SOL".to_string(),
                decimals: 9,
                coingecko_id: Some("marinade-staked-sol".to_string()),
                price_feeds: vec!["pyth".to_string()],
            },
        ];
        
        for token in tokens {
            self.token_metadata.insert(token.mint, token);
        }
        
        info!("Initialized {} token metadata entries", self.token_metadata.len());
        Ok(())
    }

    /// Update token metadata (for dynamic token discovery)
    pub fn update_token_metadata(&mut self, token_metadata: TokenMetadata) {
        info!("Updated metadata for token {}: {}", token_metadata.mint, token_metadata.symbol);
        self.token_metadata.insert(token_metadata.mint, token_metadata);
    }

    /// Get token metadata
    pub fn get_token_metadata(&self, token_mint: &Pubkey) -> Option<&TokenMetadata> {
        self.token_metadata.get(token_mint)
    }

    /// Clear expired caches
    pub fn clear_expired_cache(&mut self) {
        let now = Utc::now();
        
        // Clear price cache
        self.price_cache.retain(|_, (_, timestamp)| {
            now.signed_duration_since(*timestamp).num_seconds() < self.cache_ttl_secs as i64
        });
        
        // Clear volatility cache (10 minute TTL)
        self.volatility_cache.retain(|_, (_, timestamp)| {
            now.signed_duration_since(*timestamp).num_minutes() < 10
        });
        
        // Clear historical cache (1 hour TTL)
        self.historical_cache.retain(|_, data| {
            if let Some(latest) = data.last() {
                now.signed_duration_since(latest.timestamp).num_hours() < 1
            } else {
                false
            }
        });
    }

    /// Get all cached prices for debugging
    pub fn get_cached_prices(&self) -> &BTreeMap<Pubkey, (Decimal, DateTime<Utc>)> {
        &self.price_cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_price_generation() {
        let monitor = PriceMonitor::new().await.unwrap();
        let test_mint = Pubkey::new_unique();
        
        let price1 = monitor.get_mock_price(&test_mint).await.unwrap();
        let price2 = monitor.get_mock_price(&test_mint).await.unwrap();
        
        // Same mint should generate same price
        assert_eq!(price1, price2);
        assert!(price1 > Decimal::ZERO);
        assert!(price1 <= Decimal::new(1000, 0));
    }

    #[tokio::test]
    async fn test_volatility_calculation() {
        let mut monitor = PriceMonitor::new().await.unwrap();
        let test_mint = Pubkey::new_unique();
        
        // Add some price history
        let prices = vec![
            Decimal::new(100, 0),
            Decimal::new(102, 0),
            Decimal::new(98, 0),
            Decimal::new(105, 0),
            Decimal::new(95, 0),
        ];
        
        let history = monitor.price_history.entry(test_mint).or_insert_with(VecDeque::new);
        for (i, price) in prices.iter().enumerate() {
            history.push_back(PriceDataPoint {
                timestamp: Utc::now() - ChronoDuration::hours((prices.len() - i - 1) as i64),
                price_x: *price,
                price_y: Decimal::ZERO,
                volume_24h: Decimal::ZERO,
                liquidity: Decimal::ZERO,
                active_bin_id: 0,
            });
        }
        
        let volatility = monitor.calculate_volatility(test_mint, 24).await.unwrap();
        assert!(volatility > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_historical_data_generation() {
        let monitor = PriceMonitor::new().await.unwrap();
        let token_x = Pubkey::new_unique();
        let token_y = Pubkey::new_unique();
        
        let data = monitor.generate_mock_historical_data(token_x, token_y, 30).await.unwrap();
        
        assert_eq!(data.len(), 30);
        assert!(data.iter().all(|p| p.price_x > Decimal::ZERO));
        assert!(data.iter().all(|p| p.price_y > Decimal::ZERO));
        
        // Check that timestamps are in order
        for window in data.windows(2) {
            assert!(window[1].timestamp > window[0].timestamp);
        }
    }

    #[test]
    fn test_notification_threshold() {
        let mut monitor = PriceMonitor::new().await.unwrap();
        let test_mint = Pubkey::new_unique();
        let threshold = Decimal::new(5, 2); // 5%
        
        monitor.set_notification_threshold(test_mint, threshold);
        
        assert_eq!(monitor.notification_thresholds.get(&test_mint), Some(&threshold));
    }
}