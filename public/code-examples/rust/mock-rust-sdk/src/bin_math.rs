//! Bin math utilities for DLMM calculations

/// Convert price to bin ID
pub fn price_to_bin_id(price: f64, bin_step: u16, decimals_diff: i8) -> i32 {
    let price_adjusted = price * 10_f64.powi(decimals_diff as i32);
    let step_ratio = 1.0 + (bin_step as f64 / 10000.0);
    (price_adjusted.ln() / step_ratio.ln()).round() as i32
}

/// Convert bin ID to price
pub fn bin_id_to_price(bin_id: i32, bin_step: u16, decimals_diff: i8) -> f64 {
    let step_ratio = 1.0 + (bin_step as f64 / 10000.0);
    let price = step_ratio.powi(bin_id);
    price / 10_f64.powi(decimals_diff as i32)
}

/// Calculate price impact
pub fn calculate_price_impact(amount_in: u64, bin_liquidity: u128, _bin_step: u16) -> f64 {
    if bin_liquidity == 0 {
        return 100.0; // Maximum impact if no liquidity
    }
    
    let ratio = amount_in as f64 / bin_liquidity as f64;
    (ratio * 100.0).min(100.0) // Cap at 100%
}

/// Get composition at bin (percentage of X and Y tokens)
pub fn get_bin_composition(bin_id: i32, active_bin_id: i32) -> (f64, f64) {
    if bin_id < active_bin_id {
        // Below active bin: mostly token Y
        (0.0, 1.0)
    } else if bin_id > active_bin_id {
        // Above active bin: mostly token X
        (1.0, 0.0)
    } else {
        // At active bin: 50/50 split
        (0.5, 0.5)
    }
}

/// Calculate liquidity for uniform distribution
pub fn uniform_liquidity_distribution(
    total_liquidity: u128,
    lower_bin: i32,
    upper_bin: i32,
) -> Vec<(i32, u128)> {
    let num_bins = (upper_bin - lower_bin + 1) as u128;
    let liquidity_per_bin = total_liquidity / num_bins;
    
    (lower_bin..=upper_bin)
        .map(|bin_id| (bin_id, liquidity_per_bin))
        .collect()
}

/// Calculate liquidity for normal distribution
pub fn normal_liquidity_distribution(
    total_liquidity: u128,
    lower_bin: i32,
    upper_bin: i32,
    mean: i32,
    std_dev: f64,
) -> Vec<(i32, u128)> {
    let mut distributions = Vec::new();
    let mut total_weight = 0.0;
    
    // Calculate weights based on normal distribution
    for bin_id in lower_bin..=upper_bin {
        let x = (bin_id - mean) as f64 / std_dev;
        let weight = (-0.5 * x * x).exp();
        distributions.push((bin_id, weight));
        total_weight += weight;
    }
    
    // Normalize weights to distribute total liquidity
    distributions
        .into_iter()
        .map(|(bin_id, weight)| {
            let liquidity = ((weight / total_weight) * total_liquidity as f64) as u128;
            (bin_id, liquidity)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_bin_conversion() {
        let price = 110.5;
        let bin_step = 20; // 0.2%
        let decimals_diff = 0;
        
        let bin_id = price_to_bin_id(price, bin_step, decimals_diff);
        let recovered_price = bin_id_to_price(bin_id, bin_step, decimals_diff);
        
        // Should be approximately equal (within 1%)
        assert!((price - recovered_price).abs() / price < 0.01);
    }
    
    #[test]
    fn test_bin_composition() {
        let active_bin = 100;
        
        let (x_below, y_below) = get_bin_composition(95, active_bin);
        assert_eq!(x_below, 0.0);
        assert_eq!(y_below, 1.0);
        
        let (x_above, y_above) = get_bin_composition(105, active_bin);
        assert_eq!(x_above, 1.0);
        assert_eq!(y_above, 0.0);
        
        let (x_active, y_active) = get_bin_composition(active_bin, active_bin);
        assert_eq!(x_active, 0.5);
        assert_eq!(y_active, 0.5);
    }
    
    #[test]
    fn test_uniform_distribution() {
        let total_liquidity = 1000;
        let lower_bin = 95;
        let upper_bin = 105;
        
        let distribution = uniform_liquidity_distribution(total_liquidity, lower_bin, upper_bin);
        
        assert_eq!(distribution.len(), 11); // 105 - 95 + 1
        
        let total_distributed: u128 = distribution.iter().map(|(_, liq)| liq).sum();
        assert_eq!(total_distributed, total_liquidity);
        
        // Each bin should have equal liquidity
        let expected_per_bin = total_liquidity / 11;
        for (_, liquidity) in distribution {
            assert_eq!(liquidity, expected_per_bin);
        }
    }
}