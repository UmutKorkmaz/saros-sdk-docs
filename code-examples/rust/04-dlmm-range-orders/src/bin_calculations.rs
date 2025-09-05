//! Bin price calculations and utilities for DLMM trading

use anyhow::{anyhow, Result};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Simple mathematical helper functions for Decimal
trait DecimalMath {
    fn power_approx(&self, exponent: i64) -> Option<Self> where Self: Sized;
    fn ln_approx(&self) -> Option<Self> where Self: Sized;
    fn powi_simple(&self, exp: i32) -> Self where Self: Sized;
}

impl DecimalMath for Decimal {
    fn power_approx(&self, exponent: i64) -> Option<Self> {
        // Simple approximation using repeated multiplication
        if exponent == 0 {
            Some(Decimal::ONE)
        } else if exponent > 0 {
            let mut result = *self;
            for _ in 1..exponent {
                result = result.checked_mul(*self)?;
            }
            Some(result)
        } else {
            // Negative exponent: 1 / (self^|exponent|)
            let positive_power = self.power_approx(-exponent)?;
            Some(Decimal::ONE.checked_div(positive_power)?)
        }
    }
    
    fn ln_approx(&self) -> Option<Self> {
        // Very simple ln approximation: ln(x) ≈ (x - 1) for values close to 1
        // This is only accurate for small changes from 1
        if *self <= Decimal::ZERO {
            None
        } else {
            Some(*self - Decimal::ONE)
        }
    }
    
    fn powi_simple(&self, exp: i32) -> Self {
        let mut result = Decimal::ONE;
        let mut base = *self;
        let mut exponent = exp.abs();
        
        while exponent > 0 {
            if exponent % 2 == 1 {
                result = result * base;
            }
            base = base * base;
            exponent /= 2;
        }
        
        if exp < 0 {
            Decimal::ONE / result
        } else {
            result
        }
    }
}

/// Bin step constants (in basis points)
pub const MIN_BIN_STEP: u16 = 1; // 0.01%
pub const MAX_BIN_STEP: u16 = 1000; // 10%
pub const DEFAULT_BIN_STEP: u16 = 20; // 0.2%

/// Bin calculations utility struct
#[derive(Debug, Clone)]
pub struct BinCalculator {
    /// Bin step in basis points
    pub bin_step: u16,
    /// Base price (usually at bin_id = 0)
    pub base_price: Decimal,
}

impl BinCalculator {
    /// Create new bin calculator with given bin step and base price
    pub fn new(bin_step: u16, base_price: Decimal) -> Result<Self> {
        if bin_step < MIN_BIN_STEP || bin_step > MAX_BIN_STEP {
            return Err(anyhow!("Invalid bin step: {}. Must be between {} and {}", 
                              bin_step, MIN_BIN_STEP, MAX_BIN_STEP));
        }
        
        if base_price <= Decimal::ZERO {
            return Err(anyhow!("Base price must be positive"));
        }
        
        Ok(Self {
            bin_step,
            base_price,
        })
    }
    
    /// Calculate price at specific bin ID
    pub fn get_price_at_bin(&self, bin_id: i32) -> Result<Decimal> {
        // Price = base_price * (1 + bin_step/10000)^bin_id
        let step_ratio = Decimal::from(self.bin_step) / Decimal::from(10000);
        let multiplier = (Decimal::ONE + step_ratio).power_approx(bin_id as i64)
            .ok_or_else(|| anyhow!("Price calculation overflow for bin_id: {}", bin_id))?;
        
        Ok(self.base_price * multiplier)
    }
    
    /// Calculate bin ID for a given price
    pub fn get_bin_id_for_price(&self, price: Decimal) -> Result<i32> {
        if price <= Decimal::ZERO {
            return Err(anyhow!("Price must be positive"));
        }
        
        let step_ratio = Decimal::from(self.bin_step) / Decimal::from(10000);
        let price_ratio = price / self.base_price;
        
        // bin_id = log(price_ratio) / log(1 + step_ratio)
        let ln_price_ratio = price_ratio.ln_approx()
            .ok_or_else(|| anyhow!("Invalid price ratio for ln calculation"))?;
        let ln_step_plus_one = (Decimal::ONE + step_ratio).ln_approx()
            .ok_or_else(|| anyhow!("Invalid step ratio for ln calculation"))?;
        
        let bin_id_f = ln_price_ratio / ln_step_plus_one;
        Ok(bin_id_f.round().to_i32().unwrap_or(0))
    }
    
    /// Get price range for a bin (lower and upper bounds)
    pub fn get_bin_price_range(&self, bin_id: i32) -> Result<(Decimal, Decimal)> {
        let current_price = self.get_price_at_bin(bin_id)?;
        let next_price = self.get_price_at_bin(bin_id + 1)?;
        
        Ok((current_price, next_price))
    }
    
    /// Calculate optimal bin distribution for DCA strategy
    pub fn calculate_dca_distribution(
        &self,
        total_amount: Decimal,
        start_bin_id: i32,
        end_bin_id: i32,
        distribution_type: &crate::types::LadderDistribution,
    ) -> Result<BTreeMap<i32, Decimal>> {
        if start_bin_id >= end_bin_id {
            return Err(anyhow!("Start bin ID must be less than end bin ID"));
        }
        
        let bin_count = (end_bin_id - start_bin_id) as usize;
        let mut distribution = BTreeMap::new();
        
        match distribution_type {
            crate::types::LadderDistribution::Uniform => {
                let amount_per_bin = total_amount / Decimal::from(bin_count);
                for bin_id in start_bin_id..end_bin_id {
                    distribution.insert(bin_id, amount_per_bin);
                }
            },
            
            crate::types::LadderDistribution::Weighted { bias } => {
                let weights = self.calculate_weighted_distribution(start_bin_id, end_bin_id, *bias)?;
                let total_weight: Decimal = weights.values().sum();
                
                for (bin_id, weight) in weights {
                    let amount = total_amount * weight / total_weight;
                    distribution.insert(bin_id, amount);
                }
            },
            
            crate::types::LadderDistribution::Fibonacci => {
                let fib_weights = self.calculate_fibonacci_distribution(start_bin_id, end_bin_id)?;
                let total_weight: Decimal = fib_weights.values().sum();
                
                for (bin_id, weight) in fib_weights {
                    let amount = total_amount * weight / total_weight;
                    distribution.insert(bin_id, amount);
                }
            },
            
            crate::types::LadderDistribution::Custom(amounts) => {
                if amounts.len() != bin_count {
                    return Err(anyhow!("Custom distribution length doesn't match bin count"));
                }
                
                let total_custom: Decimal = amounts.iter().sum();
                for (i, &weight) in amounts.iter().enumerate() {
                    let bin_id = start_bin_id + i as i32;
                    let amount = total_amount * weight / total_custom;
                    distribution.insert(bin_id, amount);
                }
            },
        }
        
        Ok(distribution)
    }
    
    /// Calculate grid trading levels
    pub fn calculate_grid_levels(
        &self,
        center_price: Decimal,
        grid_spacing_bps: u16,
        buy_levels: u32,
        sell_levels: u32,
    ) -> Result<(Vec<i32>, Vec<i32>)> {
        let center_bin_id = self.get_bin_id_for_price(center_price)?;
        
        // Calculate spacing in bin steps
        let spacing_ratio = Decimal::from(grid_spacing_bps) / Decimal::from(self.bin_step);
        let bin_spacing = spacing_ratio.round().to_i32().unwrap_or(1).max(1);
        
        let mut buy_bins = Vec::new();
        let mut sell_bins = Vec::new();
        
        // Create buy levels below center
        for i in 1..=buy_levels {
            let bin_id = center_bin_id - (bin_spacing * i as i32);
            buy_bins.push(bin_id);
        }
        
        // Create sell levels above center
        for i in 1..=sell_levels {
            let bin_id = center_bin_id + (bin_spacing * i as i32);
            sell_bins.push(bin_id);
        }
        
        Ok((buy_bins, sell_bins))
    }
    
    /// Calculate price impact for a trade
    pub fn calculate_price_impact(
        &self,
        amount_in: Decimal,
        liquidity_available: Decimal,
    ) -> Result<Decimal> {
        if liquidity_available <= Decimal::ZERO {
            return Ok(Decimal::from(100)); // 100% price impact if no liquidity
        }
        
        // Simple price impact model: impact = amount / liquidity
        let impact = amount_in / liquidity_available;
        Ok(impact.min(Decimal::ONE)) // Cap at 100%
    }
    
    /// Calculate slippage for bin-based trading
    pub fn calculate_bin_slippage(
        &self,
        target_bin_id: i32,
        actual_bin_id: i32,
    ) -> Result<Decimal> {
        let target_price = self.get_price_at_bin(target_bin_id)?;
        let actual_price = self.get_price_at_bin(actual_bin_id)?;
        
        let slippage = ((actual_price - target_price) / target_price).abs();
        Ok(slippage)
    }
    
    /// Find optimal execution bins for large orders
    pub fn find_execution_bins(
        &self,
        total_amount: Decimal,
        available_liquidity: &BTreeMap<i32, Decimal>,
        max_price_impact: Decimal,
    ) -> Result<Vec<(i32, Decimal)>> {
        let mut execution_plan = Vec::new();
        let mut remaining_amount = total_amount;
        
        // Sort bins by price (ascending for buys, would be descending for sells)
        let mut sorted_bins: Vec<_> = available_liquidity.iter().collect();
        sorted_bins.sort_by_key(|(bin_id, _)| *bin_id);
        
        for (&bin_id, &liquidity) in sorted_bins {
            if remaining_amount <= Decimal::ZERO {
                break;
            }
            
            let max_tradeable = liquidity * max_price_impact;
            let amount_to_trade = remaining_amount.min(max_tradeable);
            
            if amount_to_trade > Decimal::ZERO {
                execution_plan.push((bin_id, amount_to_trade));
                remaining_amount -= amount_to_trade;
            }
        }
        
        if remaining_amount > Decimal::ZERO {
            return Err(anyhow!(
                "Insufficient liquidity to execute trade. Remaining amount: {}", 
                remaining_amount
            ));
        }
        
        Ok(execution_plan)
    }
    
    // Helper methods for distribution calculations
    
    fn calculate_weighted_distribution(
        &self,
        start_bin: i32,
        end_bin: i32,
        bias: f64,
    ) -> Result<BTreeMap<i32, Decimal>> {
        let mut weights = BTreeMap::new();
        let bin_count = (end_bin - start_bin) as f64;
        
        for (i, bin_id) in (start_bin..end_bin).enumerate() {
            // Higher weight for lower bins (better prices) when bias > 1
            let position_factor = (bin_count - i as f64) / bin_count;
            let weight = position_factor.powf(bias);
            weights.insert(bin_id, Decimal::from_f64(weight).unwrap_or(Decimal::ONE));
        }
        
        Ok(weights)
    }
    
    fn calculate_fibonacci_distribution(
        &self,
        start_bin: i32,
        end_bin: i32,
    ) -> Result<BTreeMap<i32, Decimal>> {
        let mut weights = BTreeMap::new();
        let bin_count = (end_bin - start_bin) as usize;
        
        // Generate Fibonacci sequence
        let mut fib_sequence = vec![1u64, 1u64];
        while fib_sequence.len() < bin_count {
            let next = fib_sequence[fib_sequence.len() - 1] + fib_sequence[fib_sequence.len() - 2];
            fib_sequence.push(next);
        }
        
        // Reverse for descending weights (more allocation at better prices)
        fib_sequence.reverse();
        
        for (i, bin_id) in (start_bin..end_bin).enumerate() {
            let weight = if i < fib_sequence.len() {
                Decimal::from(fib_sequence[i])
            } else {
                Decimal::ONE
            };
            weights.insert(bin_id, weight);
        }
        
        Ok(weights)
    }
}

/// Utility functions for bin-related calculations
pub struct BinUtils;

impl BinUtils {
    /// Convert basis points to decimal
    pub fn bps_to_decimal(bps: u16) -> Decimal {
        Decimal::from(bps) / Decimal::from(10000)
    }
    
    /// Convert decimal to basis points
    pub fn decimal_to_bps(decimal: Decimal) -> u16 {
        (decimal * Decimal::from(10000)).to_u16().unwrap_or(0)
    }
    
    /// Calculate the number of bins between two prices
    pub fn bins_between_prices(
        price1: Decimal,
        price2: Decimal,
        bin_step: u16,
    ) -> Result<i32> {
        if price1 <= Decimal::ZERO || price2 <= Decimal::ZERO {
            return Err(anyhow!("Prices must be positive"));
        }
        
        let step_ratio = Decimal::from(bin_step) / Decimal::from(10000);
        let price_ratio = price2 / price1;
        
        let ln_price_ratio = price_ratio.ln_approx()
            .ok_or_else(|| anyhow!("Invalid price ratio for ln calculation"))?;
        let ln_step_plus_one = (Decimal::ONE + step_ratio).ln_approx()
            .ok_or_else(|| anyhow!("Invalid step ratio for ln calculation"))?;
        let bin_diff = ln_price_ratio / ln_step_plus_one;
        Ok(bin_diff.round().to_i32().unwrap_or(0))
    }
    
    /// Validate bin range for strategies
    pub fn validate_bin_range(start_bin: i32, end_bin: i32, max_range: u32) -> Result<()> {
        if start_bin >= end_bin {
            return Err(anyhow!("Start bin must be less than end bin"));
        }
        
        let range = (end_bin - start_bin) as u32;
        if range > max_range {
            return Err(anyhow!(
                "Bin range {} exceeds maximum allowed range {}", 
                range, max_range
            ));
        }
        
        Ok(())
    }
    
    /// Calculate optimal rebalancing threshold
    pub fn calculate_rebalancing_threshold(
        bin_step: u16,
        target_rebalances_per_day: u32,
        volatility_estimate: Decimal,
    ) -> Decimal {
        // Simple model: threshold = bin_step * volatility_factor / rebalance_frequency
        let base_threshold = Self::bps_to_decimal(bin_step);
        let volatility_factor = volatility_estimate.max(Decimal::from_str("0.1").unwrap());
        let frequency_factor = Decimal::from(target_rebalances_per_day).max(Decimal::ONE);
        
        base_threshold * volatility_factor / frequency_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    
    #[test]
    fn test_bin_calculator_creation() {
        let calc = BinCalculator::new(20, dec!(100)).unwrap();
        assert_eq!(calc.bin_step, 20);
        assert_eq!(calc.base_price, dec!(100));
    }
    
    #[test]
    fn test_price_calculation() {
        let calc = BinCalculator::new(20, dec!(100)).unwrap();
        
        // At bin 0, price should be base price
        let price_0 = calc.get_price_at_bin(0).unwrap();
        assert_eq!(price_0, dec!(100));
        
        // At bin 1, price should be 100 * 1.002 = 100.2
        let price_1 = calc.get_price_at_bin(1).unwrap();
        assert!((price_1 - dec!(100.2)).abs() < dec!(0.001));
        
        // At bin -1, price should be 100 / 1.002
        let price_neg1 = calc.get_price_at_bin(-1).unwrap();
        assert!(price_neg1 < dec!(100));
    }
    
    #[test]
    fn test_bin_id_for_price() {
        let calc = BinCalculator::new(20, dec!(100)).unwrap();
        
        // Price 100 should be bin 0
        let bin_0 = calc.get_bin_id_for_price(dec!(100)).unwrap();
        assert_eq!(bin_0, 0);
        
        // Price slightly above 100 should be bin 1
        let bin_1 = calc.get_bin_id_for_price(dec!(100.2)).unwrap();
        assert_eq!(bin_1, 1);
    }
    
    #[test]
    fn test_uniform_dca_distribution() {
        let calc = BinCalculator::new(20, dec!(100)).unwrap();
        let distribution = calc.calculate_dca_distribution(
            dec!(1000),
            90,
            100,
            &crate::types::LadderDistribution::Uniform,
        ).unwrap();
        
        assert_eq!(distribution.len(), 10);
        for amount in distribution.values() {
            assert_eq!(*amount, dec!(100)); // 1000 / 10 = 100
        }
    }
    
    #[test]
    fn test_grid_levels() {
        let calc = BinCalculator::new(20, dec!(100)).unwrap();
        let (buy_bins, sell_bins) = calc.calculate_grid_levels(
            dec!(100),
            100, // 1% spacing
            5,
            5,
        ).unwrap();
        
        assert_eq!(buy_bins.len(), 5);
        assert_eq!(sell_bins.len(), 5);
        
        // Buy bins should be below center (negative relative to center)
        for &bin_id in &buy_bins {
            assert!(bin_id < 0);
        }
        
        // Sell bins should be above center (positive relative to center)
        for &bin_id in &sell_bins {
            assert!(bin_id > 0);
        }
    }
}