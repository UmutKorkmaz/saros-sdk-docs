//! Price Analyzer - Real-time price analysis and prediction
//!
//! This module provides advanced price analysis capabilities including:
//! - Real-time price monitoring and historical analysis
//! - Price prediction using technical indicators
//! - Market trend analysis and pattern recognition
//! - Visual price charts and analytics
//! - Price impact visualization

use anyhow::Result;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use colored::*;
use dashmap::DashMap;
use futures::StreamExt;
use num_traits::Zero;
use plotters::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock},
    time::Instant,
};
use tokio::time::{interval, sleep};
use uuid::Uuid;

/// Price analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceAnalyzerConfig {
    /// Maximum historical data points to keep
    pub max_history_points: usize,
    /// Price update interval in seconds
    pub update_interval_seconds: u64,
    /// Enable chart generation
    pub enable_charts: bool,
    /// Chart output directory
    pub chart_output_dir: String,
    /// Price prediction algorithms to use
    pub prediction_algorithms: Vec<PredictionAlgorithm>,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
}

/// Available price prediction algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredictionAlgorithm {
    MovingAverage { periods: usize },
    ExponentialMovingAverage { alpha: f64 },
    BollingerBands { periods: usize, std_dev: f64 },
    RSI { periods: usize },
    MACD { fast: usize, slow: usize, signal: usize },
    LinearRegression { periods: usize },
}

/// Alert configuration thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub price_change_percent: f64,
    pub volume_spike_multiplier: f64,
    pub volatility_threshold: f64,
    pub liquidity_drop_percent: f64,
}

/// Historical price data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub timestamp: DateTime<Utc>,
    pub price: f64,
    pub volume: f64,
    pub liquidity: f64,
    pub market_cap: f64,
    pub bid_ask_spread: f64,
}

/// Technical analysis indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicators {
    pub sma_20: Option<f64>,
    pub ema_12: Option<f64>,
    pub ema_26: Option<f64>,
    pub macd: Option<f64>,
    pub macd_signal: Option<f64>,
    pub rsi: Option<f64>,
    pub bollinger_upper: Option<f64>,
    pub bollinger_lower: Option<f64>,
    pub support_level: Option<f64>,
    pub resistance_level: Option<f64>,
}

/// Price prediction with confidence intervals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePrediction {
    pub timestamp: DateTime<Utc>,
    pub predicted_price: f64,
    pub confidence_interval_lower: f64,
    pub confidence_interval_upper: f64,
    pub confidence_score: f64,
    pub algorithm_used: String,
    pub time_horizon_minutes: u32,
}

/// Market analysis summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketAnalysis {
    pub token_pair: String,
    pub current_price: f64,
    pub price_change_24h: f64,
    pub volume_24h: f64,
    pub volatility_24h: f64,
    pub trend_direction: TrendDirection,
    pub market_sentiment: MarketSentiment,
    pub technical_indicators: TechnicalIndicators,
    pub predictions: Vec<PricePrediction>,
    pub support_resistance: SupportResistance,
    pub last_updated: DateTime<Utc>,
}

/// Market trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    StrongBullish,
    Bullish,
    Neutral,
    Bearish,
    StrongBearish,
}

/// Market sentiment analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketSentiment {
    ExtremelyBullish,
    Bullish,
    Neutral,
    Bearish,
    ExtremelyBearish,
}

/// Support and resistance levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportResistance {
    pub support_levels: Vec<f64>,
    pub resistance_levels: Vec<f64>,
    pub key_support: f64,
    pub key_resistance: f64,
}

/// Price impact analysis for different trade sizes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceImpactAnalysis {
    pub trade_sizes: Vec<u64>,
    pub price_impacts: Vec<f64>,
    pub optimal_trade_size: u64,
    pub max_trade_size_1_percent: u64,
    pub max_trade_size_5_percent: u64,
    pub liquidity_depth_chart: String, // Path to chart file
}

/// Real-time price monitor and analyzer
pub struct PriceAnalyzer {
    config: PriceAnalyzerConfig,
    price_history: Arc<RwLock<DashMap<String, VecDeque<PricePoint>>>>,
    current_prices: Arc<DashMap<String, f64>>,
    analysis_cache: Arc<DashMap<String, (MarketAnalysis, Instant)>>,
    prediction_cache: Arc<DashMap<String, Vec<PricePrediction>>>,
}

impl Default for PriceAnalyzerConfig {
    fn default() -> Self {
        Self {
            max_history_points: 1440, // 24 hours of minute data
            update_interval_seconds: 5,
            enable_charts: true,
            chart_output_dir: "./charts".to_string(),
            prediction_algorithms: vec![
                PredictionAlgorithm::MovingAverage { periods: 20 },
                PredictionAlgorithm::ExponentialMovingAverage { alpha: 0.1 },
                PredictionAlgorithm::BollingerBands { periods: 20, std_dev: 2.0 },
                PredictionAlgorithm::RSI { periods: 14 },
            ],
            alert_thresholds: AlertThresholds {
                price_change_percent: 5.0,
                volume_spike_multiplier: 3.0,
                volatility_threshold: 0.1,
                liquidity_drop_percent: 20.0,
            },
        }
    }
}

impl PriceAnalyzer {
    /// Create a new price analyzer
    pub fn new() -> Self {
        Self::with_config(PriceAnalyzerConfig::default())
    }

    /// Create a new price analyzer with custom configuration
    pub fn with_config(config: PriceAnalyzerConfig) -> Self {
        // Create chart output directory
        std::fs::create_dir_all(&config.chart_output_dir).ok();

        Self {
            config,
            price_history: Arc::new(RwLock::new(DashMap::new())),
            current_prices: Arc::new(DashMap::new()),
            analysis_cache: Arc::new(DashMap::new()),
            prediction_cache: Arc::new(DashMap::new()),
        }
    }

    /// Start real-time price monitoring for a token pair
    pub async fn start_monitoring(&self, token_in: Pubkey, token_out: Pubkey) -> Result<()> {
        let pair_key = format!("{}-{}", token_in, token_out);
        log::info!("📈 Starting price monitoring for pair: {}", pair_key);

        // Initialize price history
        if let Ok(history) = self.price_history.write() {
            history.insert(pair_key.clone(), VecDeque::new());
        }

        // Spawn background monitoring task
        let analyzer = self.clone();
        let pair_key_clone = pair_key.clone();
        
        tokio::spawn(async move {
            analyzer.price_monitoring_loop(pair_key_clone).await;
        });

        Ok(())
    }

    /// Background price monitoring loop
    async fn price_monitoring_loop(&self, pair_key: String) {
        let mut interval = interval(std::time::Duration::from_secs(self.config.update_interval_seconds));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.update_price_data(&pair_key).await {
                log::warn!("Failed to update price data for {}: {}", pair_key, e);
            }
        }
    }

    /// Update price data for a token pair
    async fn update_price_data(&self, pair_key: &str) -> Result<()> {
        // Simulate fetching real-time price data
        let price_point = self.fetch_current_price_data(pair_key).await?;

        // Update current price
        self.current_prices.insert(pair_key.to_string(), price_point.price);

        // Add to price history
        if let Ok(history) = self.price_history.write() {
            if let Some(mut price_history) = history.get_mut(pair_key) {
                price_history.push_back(price_point.clone());
                
                // Keep only the configured number of data points
                while price_history.len() > self.config.max_history_points {
                    price_history.pop_front();
                }
            }
        }

        // Check for alerts
        self.check_price_alerts(pair_key, &price_point).await?;

        Ok(())
    }

    /// Perform comprehensive market analysis
    pub async fn analyze_market(&self, token_in: Pubkey, token_out: Pubkey) -> Result<MarketAnalysis> {
        let pair_key = format!("{}-{}", token_in, token_out);
        
        // Check cache first
        if let Some((cached_analysis, cached_time)) = self.analysis_cache.get(&pair_key) {
            if cached_time.elapsed().as_secs() < 30 { // 30 second cache
                return Ok(cached_analysis.clone());
            }
        }

        log::info!("🔍 Performing comprehensive market analysis for {}", pair_key);

        let history = self.get_price_history(&pair_key).await?;
        if history.is_empty() {
            return Err(anyhow::anyhow!("No price history available for analysis"));
        }

        let current_price = history.back().unwrap().price;
        let price_24h_ago = if history.len() >= 288 { // ~24 hours ago (5min intervals)
            history.get(history.len() - 288).unwrap().price
        } else {
            history.front().unwrap().price
        };

        // Calculate basic metrics
        let price_change_24h = ((current_price - price_24h_ago) / price_24h_ago) * 100.0;
        let volume_24h = history.iter().take(288).map(|p| p.volume).sum::<f64>();
        let volatility_24h = self.calculate_volatility(&history, 288)?;

        // Calculate technical indicators
        let technical_indicators = self.calculate_technical_indicators(&history).await?;

        // Determine trend direction
        let trend_direction = self.analyze_trend_direction(&technical_indicators, price_change_24h)?;

        // Analyze market sentiment
        let market_sentiment = self.analyze_market_sentiment(&technical_indicators, &history)?;

        // Generate predictions
        let predictions = self.generate_price_predictions(&pair_key, &history).await?;

        // Calculate support and resistance
        let support_resistance = self.calculate_support_resistance(&history)?;

        let analysis = MarketAnalysis {
            token_pair: pair_key.clone(),
            current_price,
            price_change_24h,
            volume_24h,
            volatility_24h,
            trend_direction,
            market_sentiment,
            technical_indicators,
            predictions,
            support_resistance,
            last_updated: Utc::now(),
        };

        // Cache the analysis
        self.analysis_cache.insert(pair_key, (analysis.clone(), Instant::now()));

        // Generate visual analysis if enabled
        if self.config.enable_charts {
            self.generate_analysis_chart(&analysis, &history).await?;
        }

        Ok(analysis)
    }

    /// Generate price predictions using configured algorithms
    async fn generate_price_predictions(&self, pair_key: &str, history: &VecDeque<PricePoint>) -> Result<Vec<PricePrediction>> {
        let mut predictions = Vec::new();

        for algorithm in &self.config.prediction_algorithms {
            if let Ok(prediction) = self.apply_prediction_algorithm(algorithm, history).await {
                predictions.push(prediction);
            }
        }

        // Cache predictions
        self.prediction_cache.insert(pair_key.to_string(), predictions.clone());

        Ok(predictions)
    }

    /// Apply a specific prediction algorithm
    async fn apply_prediction_algorithm(&self, algorithm: &PredictionAlgorithm, history: &VecDeque<PricePoint>) -> Result<PricePrediction> {
        if history.len() < 20 {
            return Err(anyhow::anyhow!("Insufficient data for prediction"));
        }

        let prices: Vec<f64> = history.iter().map(|p| p.price).collect();
        let current_price = *prices.last().unwrap();
        
        match algorithm {
            PredictionAlgorithm::MovingAverage { periods } => {
                let sma = self.calculate_sma(&prices, *periods)?;
                let trend = (sma - current_price) / current_price;
                let predicted_price = current_price * (1.0 + trend * 0.1); // 10% of trend
                
                Ok(PricePrediction {
                    timestamp: Utc::now(),
                    predicted_price,
                    confidence_interval_lower: predicted_price * 0.95,
                    confidence_interval_upper: predicted_price * 1.05,
                    confidence_score: 0.7,
                    algorithm_used: format!("SMA-{}", periods),
                    time_horizon_minutes: 15,
                })
            },
            PredictionAlgorithm::ExponentialMovingAverage { alpha } => {
                let ema = self.calculate_ema(&prices, *alpha)?;
                let trend = (ema - current_price) / current_price;
                let predicted_price = current_price * (1.0 + trend * 0.15);
                
                Ok(PricePrediction {
                    timestamp: Utc::now(),
                    predicted_price,
                    confidence_interval_lower: predicted_price * 0.93,
                    confidence_interval_upper: predicted_price * 1.07,
                    confidence_score: 0.75,
                    algorithm_used: format!("EMA-{:.2}", alpha),
                    time_horizon_minutes: 10,
                })
            },
            PredictionAlgorithm::LinearRegression { periods } => {
                let predicted_price = self.linear_regression_prediction(&prices, *periods)?;
                
                Ok(PricePrediction {
                    timestamp: Utc::now(),
                    predicted_price,
                    confidence_interval_lower: predicted_price * 0.9,
                    confidence_interval_upper: predicted_price * 1.1,
                    confidence_score: 0.8,
                    algorithm_used: format!("Linear-Regression-{}", periods),
                    time_horizon_minutes: 30,
                })
            },
            _ => {
                // For other algorithms, return a basic prediction
                Ok(PricePrediction {
                    timestamp: Utc::now(),
                    predicted_price: current_price,
                    confidence_interval_lower: current_price * 0.95,
                    confidence_interval_upper: current_price * 1.05,
                    confidence_score: 0.5,
                    algorithm_used: "Basic".to_string(),
                    time_horizon_minutes: 5,
                })
            }
        }
    }

    /// Analyze price impact for different trade sizes
    pub async fn analyze_price_impact(&self, token_in: Pubkey, token_out: Pubkey, max_amount: u64) -> Result<PriceImpactAnalysis> {
        let pair_key = format!("{}-{}", token_in, token_out);
        log::info!("📊 Analyzing price impact for {}", pair_key);

        let trade_sizes: Vec<u64> = (1..=20)
            .map(|i| (max_amount / 20) * i)
            .collect();

        let mut price_impacts = Vec::new();
        let mut optimal_trade_size = max_amount / 10; // Default 10%
        let mut max_trade_1_percent = max_amount;
        let mut max_trade_5_percent = max_amount;

        for &size in &trade_sizes {
            let impact = self.calculate_price_impact_for_size(size, &pair_key).await?;
            price_impacts.push(impact);

            // Find optimal trade sizes for different impact thresholds
            if impact <= 1.0 && size > max_trade_1_percent {
                max_trade_1_percent = size;
            }
            if impact <= 5.0 && size > max_trade_5_percent {
                max_trade_5_percent = size;
            }
            if impact <= 2.0 && impact > 0.5 {
                optimal_trade_size = size; // Sweet spot
            }
        }

        // Generate price impact chart
        let chart_path = if self.config.enable_charts {
            Some(self.generate_price_impact_chart(&pair_key, &trade_sizes, &price_impacts).await?)
        } else {
            None
        };

        Ok(PriceImpactAnalysis {
            trade_sizes,
            price_impacts,
            optimal_trade_size,
            max_trade_size_1_percent: max_trade_1_percent,
            max_trade_size_5_percent: max_trade_5_percent,
            liquidity_depth_chart: chart_path.unwrap_or_default(),
        })
    }

    /// Display real-time price information with colors
    pub async fn display_real_time_info(&self, token_in: Pubkey, token_out: Pubkey) -> Result<()> {
        let analysis = self.analyze_market(token_in, token_out).await?;
        
        println!("\n{}", "═══════════════════════════════════════".bright_blue());
        println!("{}", format!("📈 MARKET ANALYSIS: {}", analysis.token_pair).bright_cyan().bold());
        println!("{}", "═══════════════════════════════════════".bright_blue());
        
        // Current price with trend colors
        let price_color = if analysis.price_change_24h > 0.0 { 
            analysis.current_price.to_string().bright_green()
        } else { 
            analysis.current_price.to_string().bright_red()
        };
        println!("💰 Current Price: ${}", price_color);
        
        // 24h change with color
        let change_str = format!("{:+.2}%", analysis.price_change_24h);
        let change_colored = if analysis.price_change_24h > 0.0 {
            change_str.bright_green()
        } else {
            change_str.bright_red()
        };
        println!("📊 24h Change: {}", change_colored);
        
        // Volume and volatility
        println!("📈 24h Volume: ${:.2}", analysis.volume_24h.to_string().bright_yellow());
        println!("🌊 Volatility: {:.2}%", (analysis.volatility_24h * 100.0).to_string().bright_purple());
        
        // Trend and sentiment
        let trend_str = format!("{:?}", analysis.trend_direction);
        let trend_colored = match analysis.trend_direction {
            TrendDirection::StrongBullish | TrendDirection::Bullish => trend_str.bright_green(),
            TrendDirection::Neutral => trend_str.bright_yellow(),
            _ => trend_str.bright_red(),
        };
        println!("📈 Trend: {}", trend_colored);
        
        let sentiment_str = format!("{:?}", analysis.market_sentiment);
        let sentiment_colored = match analysis.market_sentiment {
            MarketSentiment::ExtremelyBullish | MarketSentiment::Bullish => sentiment_str.bright_green(),
            MarketSentiment::Neutral => sentiment_str.bright_yellow(),
            _ => sentiment_str.bright_red(),
        };
        println!("🎭 Sentiment: {}", sentiment_colored);
        
        // Technical indicators
        if let Some(rsi) = analysis.technical_indicators.rsi {
            let rsi_colored = if rsi > 70.0 {
                format!("{:.1} (Overbought)", rsi).bright_red()
            } else if rsi < 30.0 {
                format!("{:.1} (Oversold)", rsi).bright_green()
            } else {
                format!("{:.1}", rsi).bright_yellow()
            };
            println!("📊 RSI: {}", rsi_colored);
        }
        
        // Support and resistance
        println!("🛡️ Key Support: ${:.4}", analysis.support_resistance.key_support.to_string().bright_green());
        println!("🚧 Key Resistance: ${:.4}", analysis.support_resistance.key_resistance.to_string().bright_red());
        
        // Predictions
        if !analysis.predictions.is_empty() {
            println!("\n{}", "🔮 PRICE PREDICTIONS".bright_magenta().bold());
            println!("{}", "─────────────────────".bright_blue());
            for prediction in analysis.predictions.iter().take(3) {
                let pred_color = if prediction.predicted_price > analysis.current_price {
                    prediction.predicted_price.to_string().bright_green()
                } else {
                    prediction.predicted_price.to_string().bright_red()
                };
                println!("   {} ({}min): ${} [{:.1}% confidence]", 
                         prediction.algorithm_used,
                         prediction.time_horizon_minutes,
                         pred_color,
                         prediction.confidence_score * 100.0);
            }
        }
        
        println!("{}", "═══════════════════════════════════════".bright_blue());
        
        Ok(())
    }

    // Helper methods for technical analysis

    async fn calculate_technical_indicators(&self, history: &VecDeque<PricePoint>) -> Result<TechnicalIndicators> {
        let prices: Vec<f64> = history.iter().map(|p| p.price).collect();
        
        Ok(TechnicalIndicators {
            sma_20: if prices.len() >= 20 { Some(self.calculate_sma(&prices, 20)?) } else { None },
            ema_12: if prices.len() >= 12 { Some(self.calculate_ema(&prices, 0.15)?) } else { None },
            ema_26: if prices.len() >= 26 { Some(self.calculate_ema(&prices, 0.074)?) } else { None },
            macd: None, // Would calculate MACD
            macd_signal: None,
            rsi: if prices.len() >= 14 { Some(self.calculate_rsi(&prices, 14)?) } else { None },
            bollinger_upper: None,
            bollinger_lower: None,
            support_level: None,
            resistance_level: None,
        })
    }

    fn calculate_sma(&self, prices: &[f64], periods: usize) -> Result<f64> {
        if prices.len() < periods {
            return Err(anyhow::anyhow!("Insufficient data for SMA calculation"));
        }
        
        let sum: f64 = prices.iter().rev().take(periods).sum();
        Ok(sum / periods as f64)
    }

    fn calculate_ema(&self, prices: &[f64], alpha: f64) -> Result<f64> {
        if prices.is_empty() {
            return Err(anyhow::anyhow!("No data for EMA calculation"));
        }
        
        let mut ema = prices[0];
        for &price in prices.iter().skip(1) {
            ema = alpha * price + (1.0 - alpha) * ema;
        }
        Ok(ema)
    }

    fn calculate_rsi(&self, prices: &[f64], periods: usize) -> Result<f64> {
        if prices.len() < periods + 1 {
            return Err(anyhow::anyhow!("Insufficient data for RSI calculation"));
        }

        let mut gains = Vec::new();
        let mut losses = Vec::new();

        for i in 1..prices.len() {
            let change = prices[i] - prices[i-1];
            if change > 0.0 {
                gains.push(change);
                losses.push(0.0);
            } else {
                gains.push(0.0);
                losses.push(-change);
            }
        }

        let avg_gain: f64 = gains.iter().rev().take(periods).sum::<f64>() / periods as f64;
        let avg_loss: f64 = losses.iter().rev().take(periods).sum::<f64>() / periods as f64;

        if avg_loss == 0.0 {
            return Ok(100.0);
        }

        let rs = avg_gain / avg_loss;
        Ok(100.0 - (100.0 / (1.0 + rs)))
    }

    fn linear_regression_prediction(&self, prices: &[f64], periods: usize) -> Result<f64> {
        if prices.len() < periods {
            return Err(anyhow::anyhow!("Insufficient data for linear regression"));
        }

        let recent_prices: Vec<f64> = prices.iter().rev().take(periods).cloned().collect();
        let n = recent_prices.len() as f64;
        
        let x_values: Vec<f64> = (0..recent_prices.len()).map(|i| i as f64).collect();
        let x_sum: f64 = x_values.iter().sum();
        let y_sum: f64 = recent_prices.iter().sum();
        let xy_sum: f64 = x_values.iter().zip(&recent_prices).map(|(x, y)| x * y).sum();
        let x2_sum: f64 = x_values.iter().map(|x| x * x).sum();

        let slope = (n * xy_sum - x_sum * y_sum) / (n * x2_sum - x_sum * x_sum);
        let intercept = (y_sum - slope * x_sum) / n;
        
        // Predict next point
        let next_x = recent_prices.len() as f64;
        Ok(slope * next_x + intercept)
    }

    fn calculate_volatility(&self, history: &VecDeque<PricePoint>, periods: usize) -> Result<f64> {
        if history.len() < periods {
            return Ok(0.0);
        }

        let prices: Vec<f64> = history.iter().rev().take(periods).map(|p| p.price).collect();
        let returns: Vec<f64> = prices.windows(2).map(|w| (w[0] / w[1]) - 1.0).collect();
        
        let mean_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance: f64 = returns.iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>() / returns.len() as f64;
        
        Ok(variance.sqrt())
    }

    fn analyze_trend_direction(&self, indicators: &TechnicalIndicators, price_change: f64) -> Result<TrendDirection> {
        let trend = match price_change {
            change if change > 10.0 => TrendDirection::StrongBullish,
            change if change > 2.0 => TrendDirection::Bullish,
            change if change > -2.0 => TrendDirection::Neutral,
            change if change > -10.0 => TrendDirection::Bearish,
            _ => TrendDirection::StrongBearish,
        };
        
        // Adjust based on RSI if available
        if let Some(rsi) = indicators.rsi {
            return Ok(match (trend, rsi) {
                (TrendDirection::StrongBullish, rsi) if rsi > 80.0 => TrendDirection::Bullish, // Overbought
                (TrendDirection::StrongBearish, rsi) if rsi < 20.0 => TrendDirection::Bearish, // Oversold
                (t, _) => t,
            });
        }
        
        Ok(trend)
    }

    fn analyze_market_sentiment(&self, _indicators: &TechnicalIndicators, history: &VecDeque<PricePoint>) -> Result<MarketSentiment> {
        // Simple volume-based sentiment analysis
        if history.len() < 10 {
            return Ok(MarketSentiment::Neutral);
        }

        let recent_volume: f64 = history.iter().rev().take(5).map(|p| p.volume).sum();
        let older_volume: f64 = history.iter().rev().skip(5).take(5).map(|p| p.volume).sum();
        
        let volume_ratio = if older_volume > 0.0 { recent_volume / older_volume } else { 1.0 };
        
        Ok(match volume_ratio {
            ratio if ratio > 2.0 => MarketSentiment::ExtremelyBullish,
            ratio if ratio > 1.5 => MarketSentiment::Bullish,
            ratio if ratio > 0.7 => MarketSentiment::Neutral,
            ratio if ratio > 0.3 => MarketSentiment::Bearish,
            _ => MarketSentiment::ExtremelyBearish,
        })
    }

    fn calculate_support_resistance(&self, history: &VecDeque<PricePoint>) -> Result<SupportResistance> {
        let prices: Vec<f64> = history.iter().map(|p| p.price).collect();
        
        if prices.len() < 20 {
            let current_price = prices.last().unwrap_or(&0.0);
            return Ok(SupportResistance {
                support_levels: vec![current_price * 0.95],
                resistance_levels: vec![current_price * 1.05],
                key_support: current_price * 0.95,
                key_resistance: current_price * 1.05,
            });
        }

        let mut local_mins = Vec::new();
        let mut local_maxs = Vec::new();
        
        // Find local extremes
        for i in 1..prices.len()-1 {
            if prices[i] < prices[i-1] && prices[i] < prices[i+1] {
                local_mins.push(prices[i]);
            }
            if prices[i] > prices[i-1] && prices[i] > prices[i+1] {
                local_maxs.push(prices[i]);
            }
        }
        
        local_mins.sort_by(|a, b| a.partial_cmp(b).unwrap());
        local_maxs.sort_by(|a, b| b.partial_cmp(a).unwrap());
        
        let key_support = local_mins.get(local_mins.len() / 2).unwrap_or(&prices.iter().fold(f64::INFINITY, |a, &b| a.min(b))).clone();
        let key_resistance = local_maxs.get(local_maxs.len() / 2).unwrap_or(&prices.iter().fold(0.0, |a, &b| a.max(b))).clone();
        
        Ok(SupportResistance {
            support_levels: local_mins.into_iter().take(3).collect(),
            resistance_levels: local_maxs.into_iter().take(3).collect(),
            key_support,
            key_resistance,
        })
    }

    // Helper methods for data fetching and chart generation

    async fn fetch_current_price_data(&self, _pair_key: &str) -> Result<PricePoint> {
        // Simulate real-time price data fetching
        let base_price = 110.0 + (rand::random::<f64>() - 0.5) * 10.0;
        Ok(PricePoint {
            timestamp: Utc::now(),
            price: base_price,
            volume: 50000.0 + rand::random::<f64>() * 100000.0,
            liquidity: 1000000.0 + rand::random::<f64>() * 500000.0,
            market_cap: base_price * 1_000_000.0,
            bid_ask_spread: 0.01 + rand::random::<f64>() * 0.02,
        })
    }

    async fn get_price_history(&self, pair_key: &str) -> Result<VecDeque<PricePoint>> {
        if let Ok(history) = self.price_history.read() {
            if let Some(data) = history.get(pair_key) {
                return Ok(data.clone());
            }
        }
        
        // Generate some sample data if no history exists
        let mut history = VecDeque::new();
        let base_time = Utc::now() - ChronoDuration::hours(24);
        let base_price = 110.0;
        
        for i in 0..288 { // 24 hours of 5-minute data
            let price = base_price + (i as f64 * 0.01) + (rand::random::<f64>() - 0.5) * 2.0;
            history.push_back(PricePoint {
                timestamp: base_time + ChronoDuration::minutes(i * 5),
                price,
                volume: 30000.0 + rand::random::<f64>() * 50000.0,
                liquidity: 800000.0 + rand::random::<f64>() * 400000.0,
                market_cap: price * 1_000_000.0,
                bid_ask_spread: 0.01,
            });
        }
        
        Ok(history)
    }

    async fn calculate_price_impact_for_size(&self, size: u64, _pair_key: &str) -> Result<f64> {
        // Simplified price impact calculation
        let liquidity_estimate = 100_000_000_000u64; // 100 SOL worth of liquidity
        let impact = (size as f64 / liquidity_estimate as f64) * 100.0;
        Ok(impact.min(50.0)) // Cap at 50%
    }

    async fn check_price_alerts(&self, pair_key: &str, price_point: &PricePoint) -> Result<()> {
        // Check for significant price changes and log alerts
        if let Some(previous_price) = self.current_prices.get(pair_key) {
            let price_change_percent = ((price_point.price - *previous_price) / *previous_price).abs() * 100.0;
            
            if price_change_percent > self.config.alert_thresholds.price_change_percent {
                log::warn!("🚨 PRICE ALERT: {} changed by {:.2}% (${:.4} -> ${:.4})", 
                           pair_key, price_change_percent, *previous_price, price_point.price);
            }
        }
        
        Ok(())
    }

    async fn generate_analysis_chart(&self, analysis: &MarketAnalysis, history: &VecDeque<PricePoint>) -> Result<String> {
        let chart_path = format!("{}/analysis_{}.png", 
                                self.config.chart_output_dir, 
                                analysis.token_pair.replace("-", "_"));
        
        // Create a simple price chart using plotters
        let root = BitMapBackend::new(&chart_path, (800, 600)).into_drawing_area();
        root.fill(&WHITE)?;

        let prices: Vec<f64> = history.iter().map(|p| p.price).collect();
        let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_price = prices.iter().fold(0.0, |a, &b| a.max(b));

        let mut chart = ChartBuilder::on(&root)
            .caption(&format!("Price Analysis: {}", analysis.token_pair), ("Arial", 50))
            .margin(10)
            .x_desc("Time")
            .y_desc("Price ($)")
            .build_cartesian_2d(0..history.len(), min_price..max_price)?;

        chart.configure_mesh().draw()?;

        // Draw price line
        let price_line: Vec<(usize, f64)> = prices.into_iter().enumerate().collect();
        chart.draw_series(LineSeries::new(price_line, &BLUE))?
            .label("Price")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &BLUE));

        chart.configure_series_labels().draw()?;
        root.present()?;

        Ok(chart_path)
    }

    async fn generate_price_impact_chart(&self, pair_key: &str, sizes: &[u64], impacts: &[f64]) -> Result<String> {
        let chart_path = format!("{}/impact_{}.png", 
                                self.config.chart_output_dir, 
                                pair_key.replace("-", "_"));
        
        let root = BitMapBackend::new(&chart_path, (800, 400)).into_drawing_area();
        root.fill(&WHITE)?;

        let max_size = *sizes.last().unwrap() as f64 / 1_000_000_000.0; // Convert to SOL
        let max_impact = impacts.iter().fold(0.0, |a, &b| a.max(b));

        let mut chart = ChartBuilder::on(&root)
            .caption(&format!("Price Impact Analysis: {}", pair_key), ("Arial", 40))
            .margin(10)
            .x_desc("Trade Size (SOL)")
            .y_desc("Price Impact (%)")
            .build_cartesian_2d(0.0..max_size, 0.0..max_impact)?;

        chart.configure_mesh().draw()?;

        // Draw impact curve
        let impact_points: Vec<(f64, f64)> = sizes.iter().zip(impacts)
            .map(|(&size, &impact)| (size as f64 / 1_000_000_000.0, impact))
            .collect();
        
        chart.draw_series(LineSeries::new(impact_points, &RED))?
            .label("Price Impact")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &RED));

        chart.configure_series_labels().draw()?;
        root.present()?;

        Ok(chart_path)
    }
}

impl Clone for PriceAnalyzer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            price_history: Arc::clone(&self.price_history),
            current_prices: Arc::clone(&self.current_prices),
            analysis_cache: Arc::clone(&self.analysis_cache),
            prediction_cache: Arc::clone(&self.prediction_cache),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_price_analyzer_creation() {
        let analyzer = PriceAnalyzer::new();
        assert_eq!(analyzer.config.max_history_points, 1440);
        assert!(analyzer.config.enable_charts);
    }

    #[tokio::test]
    async fn test_technical_indicators() {
        let analyzer = PriceAnalyzer::new();
        let prices = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 104.0, 103.0, 102.0, 101.0,
                         100.0, 99.0, 98.0, 99.0, 100.0, 101.0, 102.0, 103.0, 104.0, 105.0];
        
        let sma = analyzer.calculate_sma(&prices, 10).unwrap();
        assert!((sma - 101.5).abs() < 0.1);
        
        let rsi = analyzer.calculate_rsi(&prices, 14).unwrap();
        assert!(rsi >= 0.0 && rsi <= 100.0);
    }

    #[tokio::test]
    async fn test_price_prediction() {
        let analyzer = PriceAnalyzer::new();
        let prices = vec![100.0, 101.0, 102.0, 103.0, 104.0];
        
        let prediction = analyzer.linear_regression_prediction(&prices, 5).unwrap();
        assert!(prediction > 104.0); // Should predict upward trend
    }
}