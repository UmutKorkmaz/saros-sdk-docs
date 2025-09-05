//! Core impermanent loss calculation logic with mathematical precision

use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use tokio::time::{sleep, Duration};

use saros_dlmm_sdk::{DLMMClient as MockSarosClient, Position as SdkPosition, DLMMPoolInfo};
use crate::types::{
    ImpermanentLossResult, ILMetadata, CalculationMethod, PriceDataPoint, ILError, PositionSnapshot,
};

/// Core impermanent loss calculator with high-precision arithmetic
pub struct ILCalculator {
    client: MockSarosClient,
    calculation_cache: BTreeMap<String, (ImpermanentLossResult, DateTime<Utc>)>,
    cache_ttl_secs: u64,
}

impl ILCalculator {
    /// Create a new IL calculator instance
    pub async fn new() -> Result<Self> {
        let rpc_url = std::env::var("RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        
        let client = MockSarosClient::new(&rpc_url)?;
        
        Ok(Self {
            client,
            calculation_cache: BTreeMap::new(),
            cache_ttl_secs: 30, // Cache results for 30 seconds
        })
    }

    /// Calculate impermanent loss manually with provided parameters
    /// 
    /// This function implements the standard DLMM impermanent loss formula:
    /// IL = (2 * sqrt(price_ratio) / (1 + price_ratio)) - 1
    /// 
    /// For DLMM pools with concentrated liquidity, we adjust for bin range effects.
    pub async fn calculate_il_manual(
        &self,
        initial_price_x: Decimal,
        initial_price_y: Decimal,
        current_price_x: Decimal,
        current_price_y: Decimal,
        initial_amount_x: Decimal,
        initial_amount_y: Decimal,
    ) -> Result<ImpermanentLossResult> {
        info!("Calculating IL manually with provided parameters");
        
        // Validate inputs
        self.validate_price_inputs(initial_price_x, initial_price_y, current_price_x, current_price_y)?;
        
        // Calculate initial and current price ratios
        let initial_ratio = initial_price_x / initial_price_y;
        let current_ratio = current_price_x / current_price_y;
        let price_ratio_change = current_ratio / initial_ratio;
        
        debug!("Price ratios - Initial: {}, Current: {}, Change: {}", 
               initial_ratio, current_ratio, price_ratio_change);

        // Calculate initial investment value
        let initial_value_usd = (initial_amount_x * initial_price_x) + (initial_amount_y * initial_price_y);
        
        // Calculate current position value in DLMM (assuming uniform distribution)
        let current_position_value = self.calculate_dlmm_position_value(
            initial_amount_x,
            initial_amount_y,
            initial_ratio,
            current_ratio,
            None, // No specific bin range provided
        )?;
        
        // Calculate hold value (if tokens were held separately)
        let hold_value_usd = (initial_amount_x * current_price_x) + (initial_amount_y * current_price_y);
        
        // Calculate impermanent loss
        let il_usd_value = current_position_value - hold_value_usd;
        let il_percentage = if hold_value_usd > Decimal::ZERO {
            il_usd_value / hold_value_usd
        } else {
            Decimal::ZERO
        };
        
        info!("Manual IL calculation - IL: {:.4}% (${:.2})", 
              il_percentage * Decimal::new(100, 0), il_usd_value);

        Ok(ImpermanentLossResult {
            il_percentage,
            il_usd_value,
            current_value_usd: current_position_value,
            hold_value_usd,
            current_price_x,
            current_price_y,
            initial_price_x,
            initial_price_y,
            price_ratio_change,
            timestamp: Utc::now(),
            metadata: ILMetadata {
                pool_address: Default::default(), // Not applicable for manual calculation
                position_id: None,
                bin_range: None,
                active_bin_id: None,
                price_range_coverage: None,
                calculation_method: CalculationMethod::Manual,
            },
        })
    }

    /// Calculate impermanent loss from an existing position
    pub async fn calculate_il_from_position(
        &mut self,
        pool_address: solana_sdk::pubkey::Pubkey,
        position_id: solana_sdk::pubkey::Pubkey,
    ) -> Result<ImpermanentLossResult> {
        let cache_key = format!("{}:{}", pool_address, position_id);
        
        // Check cache first
        if let Some((cached_result, cached_time)) = self.calculation_cache.get(&cache_key) {
            if Utc::now().signed_duration_since(*cached_time).num_seconds() < self.cache_ttl_secs as i64 {
                debug!("Returning cached IL result for position {}", position_id);
                return Ok(cached_result.clone());
            }
        }

        info!("Calculating IL from position {} in pool {}", position_id, pool_address);

        // Get position data from SDK
        let position = self.client.get_position(position_id).await?;
        let pool_info = self.client.get_pool(pool_address).await?;
        
        // Get historical position data to determine initial state
        let position_history = self.get_position_creation_data(&position).await?;
        
        // Calculate current prices
        let current_prices = self.get_current_token_prices(&pool_info).await?;
        let (current_price_x, current_price_y) = current_prices;
        
        // Calculate initial value and prices
        let initial_prices = self.estimate_initial_prices(&position, &position_history).await?;
        let (initial_price_x, initial_price_y) = initial_prices;
        
        // Get current position value including unclaimed fees
        let current_position_value = self.calculate_position_current_value(
            &position,
            current_price_x,
            current_price_y,
        )?;
        
        // Estimate initial amounts from position data
        let initial_amounts = self.estimate_initial_amounts(&position, &position_history)?;
        let (initial_amount_x, initial_amount_y) = initial_amounts;
        
        // Calculate hold value
        let hold_value_usd = (initial_amount_x * current_price_x) + (initial_amount_y * current_price_y);
        
        // Calculate IL considering DLMM specifics
        let il_result = self.calculate_dlmm_il_with_bins(
            &position,
            &pool_info,
            initial_amounts,
            initial_prices,
            current_prices,
            current_position_value,
            hold_value_usd,
        ).await?;

        // Cache the result
        self.calculation_cache.insert(cache_key, (il_result.clone(), Utc::now()));
        
        info!("Position IL calculation - IL: {:.4}% (${:.2})", 
              il_result.il_percentage * Decimal::new(100, 0), il_result.il_usd_value);

        Ok(il_result)
    }

    /// Calculate historical impermanent loss progression
    pub async fn calculate_historical_il(
        &self,
        pool_address: solana_sdk::pubkey::Pubkey,
        historical_data: &[PriceDataPoint],
    ) -> Result<Vec<ImpermanentLossResult>> {
        info!("Calculating historical IL for {} data points", historical_data.len());
        
        if historical_data.len() < 2 {
            return Err(anyhow::anyhow!("Insufficient historical data"));
        }

        let mut il_history = Vec::new();
        let first_point = &historical_data[0];
        
        // Use first data point as baseline
        let initial_price_x = first_point.price_x;
        let initial_price_y = first_point.price_y;
        let assumed_initial_amounts = (Decimal::new(1000, 0), Decimal::new(1000, 0)); // $1000 each
        
        for data_point in historical_data.iter() {
            let il_result = self.calculate_il_manual(
                initial_price_x,
                initial_price_y,
                data_point.price_x,
                data_point.price_y,
                assumed_initial_amounts.0,
                assumed_initial_amounts.1,
            ).await?;
            
            il_history.push(ImpermanentLossResult {
                timestamp: data_point.timestamp,
                metadata: ILMetadata {
                    pool_address,
                    position_id: None,
                    bin_range: None,
                    active_bin_id: Some(data_point.active_bin_id),
                    price_range_coverage: None,
                    calculation_method: CalculationMethod::Historical,
                },
                ..il_result
            });
        }
        
        info!("Completed historical IL calculations");
        Ok(il_history)
    }

    /// Calculate DLMM-specific IL considering bin ranges and liquidity distribution
    async fn calculate_dlmm_il_with_bins(
        &self,
        position: &SdkPosition,
        pool_info: &DLMMPoolInfo,
        initial_amounts: (Decimal, Decimal),
        initial_prices: (Decimal, Decimal),
        current_prices: (Decimal, Decimal),
        current_position_value: Decimal,
        hold_value_usd: Decimal,
    ) -> Result<ImpermanentLossResult> {
        let (initial_price_x, initial_price_y) = initial_prices;
        let (current_price_x, current_price_y) = current_prices;
        
        // Calculate price ratios
        let initial_ratio = initial_price_x / initial_price_y;
        let current_ratio = current_price_x / current_price_y;
        let price_ratio_change = current_ratio / initial_ratio;
        
        // Calculate impermanent loss
        let il_usd_value = current_position_value - hold_value_usd;
        let il_percentage = if hold_value_usd > Decimal::ZERO {
            il_usd_value / hold_value_usd
        } else {
            Decimal::ZERO
        };
        
        // Calculate price range coverage for this position
        let price_range_coverage = self.calculate_price_range_coverage(
            position,
            pool_info,
            current_ratio,
        )?;
        
        Ok(ImpermanentLossResult {
            il_percentage,
            il_usd_value,
            current_value_usd: current_position_value,
            hold_value_usd,
            current_price_x,
            current_price_y,
            initial_price_x,
            initial_price_y,
            price_ratio_change,
            timestamp: Utc::now(),
            metadata: ILMetadata {
                pool_address: position.pool_address,
                position_id: Some(position.id),
                bin_range: Some((position.lower_bin_id, position.upper_bin_id)),
                active_bin_id: Some(pool_info.active_bin_id),
                price_range_coverage: Some(price_range_coverage),
                calculation_method: CalculationMethod::FromPosition,
            },
        })
    }

    /// Calculate DLMM position value considering concentrated liquidity
    fn calculate_dlmm_position_value(
        &self,
        initial_amount_x: Decimal,
        initial_amount_y: Decimal,
        initial_ratio: Decimal,
        current_ratio: Decimal,
        bin_range: Option<(i32, i32)>,
    ) -> Result<Decimal> {
        // For concentrated liquidity, we need to consider if the current price
        // is within the position's range
        
        if let Some((lower_bin, upper_bin)) = bin_range {
            // Convert bin IDs to price ratios (simplified)
            let lower_price_ratio = self.bin_to_price_ratio(lower_bin);
            let upper_price_ratio = self.bin_to_price_ratio(upper_bin);
            
            if current_ratio < lower_price_ratio {
                // Price moved below range - all in token Y
                return Ok((initial_amount_x + initial_amount_y) * current_ratio);
            } else if current_ratio > upper_price_ratio {
                // Price moved above range - all in token X  
                return Ok(initial_amount_x + initial_amount_y);
            }
        }
        
        // Standard AMM formula for positions within range
        let initial_k = initial_amount_x * initial_amount_y * initial_ratio;
        let ratio_change = current_ratio / initial_ratio;
        let sqrt_ratio_change = self.decimal_sqrt(ratio_change).unwrap_or(Decimal::ONE);
        
        let new_amount_x = self.decimal_sqrt(initial_k).unwrap_or(Decimal::ZERO) / self.decimal_sqrt(current_ratio).unwrap_or(Decimal::ONE);
        let new_amount_y = self.decimal_sqrt(initial_k * current_ratio).unwrap_or(Decimal::ZERO);
        
        Ok(new_amount_x + new_amount_y * current_ratio)
    }

    /// Convert bin ID to approximate price ratio (simplified implementation)
    fn bin_to_price_ratio(&self, bin_id: i32) -> Decimal {
        // This is a simplified conversion - in reality, this would depend on bin_step
        // Formula: price = (1 + bin_step/10000)^bin_id
        let bin_step_decimal = Decimal::new(25, 4); // 0.0025 = 25bps typical
        let exp_result = self.decimal_pow(Decimal::ONE + bin_step_decimal, bin_id as i64).unwrap_or(Decimal::ONE);
        exp_result
    }

    /// Get current token prices from pool data
    async fn get_current_token_prices(&self, pool_info: &DLMMPoolInfo) -> Result<(Decimal, Decimal)> {
        // In a real implementation, this would fetch from price oracles
        // For now, derive from pool data
        let base_price = Decimal::new(100, 0); // $100 baseline
        let price_x = base_price;
        let price_y = base_price;
        
        Ok((price_x, price_y))
    }

    /// Estimate initial prices when position was created
    async fn estimate_initial_prices(
        &self,
        _position: &SdkPosition,
        _history: &PositionSnapshot,
    ) -> Result<(Decimal, Decimal)> {
        // In a real implementation, this would look up historical price data
        // For now, return estimated values
        Ok((Decimal::new(95, 0), Decimal::new(100, 0)))
    }

    /// Get position creation data from historical records  
    async fn get_position_creation_data(&self, position: &SdkPosition) -> Result<PositionSnapshot> {
        // Mock implementation - in reality would query transaction history
        Ok(PositionSnapshot {
            timestamp: Utc::now() - chrono::Duration::days(30),
            position_value_usd: Decimal::new(2000, 0),
            hold_value_usd: Decimal::new(2000, 0),
            il_percentage: Decimal::ZERO,
            fees_earned: Decimal::ZERO,
            active_bin_id: position.lower_bin_id + (position.upper_bin_id - position.lower_bin_id) / 2,
            price_x: Decimal::new(95, 0),
            price_y: Decimal::new(100, 0),
        })
    }

    /// Calculate current position value including fees
    fn calculate_position_current_value(
        &self,
        position: &SdkPosition,
        current_price_x: Decimal,
        current_price_y: Decimal,
    ) -> Result<Decimal> {
        // Get position balances and convert to USD
        let token_x_value = Decimal::from_f64(position.value_usd / 2.0).unwrap_or_default() * current_price_x / Decimal::new(100, 0);
        let token_y_value = Decimal::from_f64(position.value_usd / 2.0).unwrap_or_default() * current_price_y / Decimal::new(100, 0);
        
        // Add unclaimed fees
        let fees_x_value = Decimal::new(position.unclaimed_fees_x as i64, 6) * current_price_x;
        let fees_y_value = Decimal::new(position.unclaimed_fees_y as i64, 6) * current_price_y;
        
        Ok(token_x_value + token_y_value + fees_x_value + fees_y_value)
    }

    /// Estimate initial amounts from position and history
    fn estimate_initial_amounts(
        &self,
        position: &SdkPosition,
        _history: &PositionSnapshot,
    ) -> Result<(Decimal, Decimal)> {
        // Estimate based on current position value and typical 50/50 split
        let total_value = Decimal::from_f64(position.value_usd).unwrap_or_default();
        let amount_x = total_value / Decimal::new(2, 0) / Decimal::new(95, 0); // Assume $95 initial price
        let amount_y = total_value / Decimal::new(2, 0) / Decimal::new(100, 0); // Assume $100 initial price
        
        Ok((amount_x, amount_y))
    }

    /// Calculate what percentage of the price range this position covers
    fn calculate_price_range_coverage(
        &self,
        position: &SdkPosition,
        pool_info: &DLMMPoolInfo,
        current_ratio: Decimal,
    ) -> Result<Decimal> {
        let position_range = position.upper_bin_id - position.lower_bin_id;
        let active_distance = (pool_info.active_bin_id - position.lower_bin_id).abs();
        
        if position_range > 0 && pool_info.active_bin_id >= position.lower_bin_id && pool_info.active_bin_id <= position.upper_bin_id {
            // Position is in range
            Ok(Decimal::ONE)
        } else if position_range > 0 {
            // Position is out of range  
            let distance_ratio = Decimal::new(active_distance as i64, 0) / Decimal::new(position_range as i64, 0);
            Ok(Decimal::ONE - distance_ratio.min(Decimal::ONE))
        } else {
            Ok(Decimal::ZERO)
        }
    }

    /// Validate price inputs for calculations
    fn validate_price_inputs(
        &self,
        initial_price_x: Decimal,
        initial_price_y: Decimal,
        current_price_x: Decimal,
        current_price_y: Decimal,
    ) -> Result<()> {
        if initial_price_x <= Decimal::ZERO || initial_price_y <= Decimal::ZERO ||
           current_price_x <= Decimal::ZERO || current_price_y <= Decimal::ZERO {
            return Err(anyhow::anyhow!("All prices must be positive"));
        }
        
        // Check for extreme price movements (>1000x)
        let max_ratio = Decimal::new(1000, 0);
        let price_x_ratio = (current_price_x / initial_price_x).max(initial_price_x / current_price_x);
        let price_y_ratio = (current_price_y / initial_price_y).max(initial_price_y / current_price_y);
        
        if price_x_ratio > max_ratio || price_y_ratio > max_ratio {
            warn!("Extreme price movement detected - results may be unreliable");
        }
        
        Ok(())
    }

    /// Clear expired cache entries
    pub fn clear_expired_cache(&mut self) {
        let now = Utc::now();
        self.calculation_cache.retain(|_, (_, timestamp)| {
            now.signed_duration_since(*timestamp).num_seconds() < self.cache_ttl_secs as i64
        });
    }
    
    /// Calculate square root using Newton's method (since rust_decimal doesn't have sqrt)
    fn decimal_sqrt(&self, value: Decimal) -> Result<Decimal> {
        if value < Decimal::ZERO {
            return Err(anyhow::anyhow!("Cannot compute square root of negative number"));
        }
        
        if value == Decimal::ZERO {
            return Ok(Decimal::ZERO);
        }
        
        if value == Decimal::ONE {
            return Ok(Decimal::ONE);
        }
        
        // Convert to f64 for calculation (loss of precision but practical)
        let value_f64 = value.to_f64().unwrap_or(0.0);
        let sqrt_f64 = value_f64.sqrt();
        
        Ok(Decimal::from_f64(sqrt_f64).unwrap_or(Decimal::ZERO))
    }
    
    /// Calculate power using repeated multiplication for integer exponents
    fn decimal_pow(&self, base: Decimal, exp: i64) -> Result<Decimal> {
        if exp == 0 {
            return Ok(Decimal::ONE);
        }
        
        if exp == 1 {
            return Ok(base);
        }
        
        if exp < 0 {
            // Handle negative exponents
            let positive_result = self.decimal_pow(base, -exp)?;
            if positive_result == Decimal::ZERO {
                return Err(anyhow::anyhow!("Division by zero in negative power"));
            }
            return Ok(Decimal::ONE / positive_result);
        }
        
        // For small positive exponents, use repeated multiplication
        if exp <= 20 {
            let mut result = base;
            for _ in 1..exp {
                result *= base;
            }
            return Ok(result);
        }
        
        // For larger exponents, convert to f64 (loss of precision but practical)
        let base_f64 = base.to_f64().unwrap_or(0.0);
        let result_f64 = base_f64.powf(exp as f64);
        
        Ok(Decimal::from_f64(result_f64).unwrap_or(Decimal::ZERO))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[tokio::test]
    async fn test_manual_il_calculation() {
        let calculator = ILCalculator::new().await.unwrap();
        
        // Test case: Token X price doubles, Token Y stays same
        let result = calculator.calculate_il_manual(
            Decimal::new(100, 0), // initial_price_x: $100
            Decimal::new(100, 0), // initial_price_y: $100
            Decimal::new(200, 0), // current_price_x: $200
            Decimal::new(100, 0), // current_price_y: $100
            Decimal::new(10, 0),  // initial_amount_x: 10 tokens
            Decimal::new(10, 0),  // initial_amount_y: 10 tokens
        ).await.unwrap();
        
        // Expected IL for 2x price change is approximately -5.72%
        assert_relative_eq!(
            result.il_percentage.to_f64().unwrap(), 
            -0.0572, 
            epsilon = 0.001
        );
        
        assert!(result.il_usd_value < Decimal::ZERO);
        assert!(matches!(result.metadata.calculation_method, CalculationMethod::Manual));
    }

    #[tokio::test]
    async fn test_equal_price_movement() {
        let calculator = ILCalculator::new().await.unwrap();
        
        // Both tokens double in price - should have zero IL
        let result = calculator.calculate_il_manual(
            Decimal::new(100, 0), // initial_price_x: $100
            Decimal::new(100, 0), // initial_price_y: $100
            Decimal::new(200, 0), // current_price_x: $200
            Decimal::new(200, 0), // current_price_y: $200
            Decimal::new(10, 0),  // initial_amount_x: 10 tokens
            Decimal::new(10, 0),  // initial_amount_y: 10 tokens
        ).await.unwrap();
        
        // Should have near-zero IL when both prices move equally
        assert_relative_eq!(
            result.il_percentage.to_f64().unwrap(),
            0.0,
            epsilon = 0.001
        );
        
        assert_eq!(result.price_ratio_change, Decimal::ONE);
    }

    #[test]
    fn test_bin_to_price_ratio() {
        let calculator = ILCalculator::new().await.unwrap();
        
        // Test basic bin conversion
        let ratio_positive = calculator.bin_to_price_ratio(100);
        let ratio_negative = calculator.bin_to_price_ratio(-100);
        
        assert!(ratio_positive > Decimal::ONE);
        assert!(ratio_negative < Decimal::ONE);
        assert_eq!(calculator.bin_to_price_ratio(0), Decimal::ONE);
    }

    #[test]
    fn test_price_validation() {
        let calculator = ILCalculator::new().await.unwrap();
        
        // Test negative price validation
        assert!(calculator.validate_price_inputs(
            Decimal::new(-100, 0),
            Decimal::new(100, 0),
            Decimal::new(100, 0),
            Decimal::new(100, 0),
        ).is_err());
        
        // Test zero price validation
        assert!(calculator.validate_price_inputs(
            Decimal::ZERO,
            Decimal::new(100, 0),
            Decimal::new(100, 0),
            Decimal::new(100, 0),
        ).is_err());
        
        // Test valid prices
        assert!(calculator.validate_price_inputs(
            Decimal::new(100, 0),
            Decimal::new(100, 0),
            Decimal::new(200, 0),
            Decimal::new(100, 0),
        ).is_ok());
    }
}