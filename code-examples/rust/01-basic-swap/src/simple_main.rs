//! Basic Swap Example - Simplified Working Version
//! 
//! This example demonstrates how to perform basic token swaps using the Saros DLMM SDK.
//! Features: Simple swap functionality with mock data

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use log::{info, error};
use rust_decimal::{Decimal, prelude::FromPrimitive};
use saros_dlmm_sdk::{DLMMClient, types::*};
use std::str::FromStr;

/// Command line interface for the basic swap example
#[derive(Parser)]
#[command(name = "basic-swap")]
#[command(about = "Basic token swap operations with Saros DLMM SDK")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a simple token swap
    Swap {
        /// Amount to swap
        #[arg(short, long)]
        amount: f64,
        /// Input token (e.g., SOL, USDC)
        #[arg(long)]
        token_in: String,
        /// Output token (e.g., SOL, USDC)  
        #[arg(long)]
        token_out: String,
        /// Maximum slippage in percentage (default: 0.5%)
        #[arg(long, default_value = "0.5")]
        max_slippage: f64,
    },
    /// Get pool information
    Pool {
        /// Pool address to query
        #[arg(short, long)]
        address: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    info!("Starting Saros Basic Swap Example");

    let cli = Cli::parse();

    // Initialize client with mock RPC endpoint
    let client = DLMMClient::new("https://api.mainnet-beta.solana.com")?;
    
    match cli.command {
        Commands::Swap { amount, token_in, token_out, max_slippage } => {
            execute_swap(&client, amount, &token_in, &token_out, max_slippage).await?;
        },
        Commands::Pool { address } => {
            get_pool_info(&client, &address).await?;
        },
    }

    Ok(())
}

/// Execute a token swap
async fn execute_swap(
    client: &DLMMClient,
    amount: f64,
    token_in: &str,
    token_out: &str,
    max_slippage: f64,
) -> Result<()> {
    info!("Executing swap: {} {} -> {}", amount, token_in, token_out);
    info!("Maximum slippage: {}%", max_slippage);

    // Convert amount to Decimal
    let amount_in = Decimal::from_f64(amount)
        .ok_or_else(|| anyhow!("Invalid amount: {}", amount))?;

    // Calculate minimum amount out based on slippage
    let slippage_decimal = Decimal::from_f64(max_slippage / 100.0)
        .ok_or_else(|| anyhow!("Invalid slippage: {}", max_slippage))?;
    
    let minimum_amount_out = amount_in * (Decimal::ONE - slippage_decimal);

    // Create swap parameters
    let swap_params = SwapParams {
        pool_address: solana_sdk::pubkey::Pubkey::new_unique(), // Mock pool address
        amount_in,
        minimum_amount_out,
        gas_price: Some(5000), // 5000 lamports
        slippage_bps: Some((max_slippage * 100.0) as u16), // Convert to basis points
    };

    // Execute the swap (this is mocked)
    match client.swap(swap_params).await {
        Ok(result) => {
            info!("✅ Swap executed successfully!");
            info!("Transaction signature: {}", result.signature);
            info!("Amount in: {} {}", amount, token_in);
            info!("Amount out: {} {}", result.amount_out, token_out);
            info!("Fee paid: {} lamports", result.fee);
            info!("Price impact: {:.4}%", result.price_impact * 100.0);
        },
        Err(e) => {
            error!("❌ Swap failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Get pool information
async fn get_pool_info(client: &DLMMClient, address: &str) -> Result<()> {
    info!("Fetching pool information for: {}", address);

    // Parse pool address
    let pool_address = solana_sdk::pubkey::Pubkey::from_str(address)
        .map_err(|e| anyhow!("Invalid pool address: {}", e))?;

    // Get pool info (this is mocked)
    match client.get_pool(pool_address).await {
        Ok(pool) => {
            info!("📊 Pool Information:");
            info!("Address: {}", pool.address);
            info!("Token X: {}", pool.token_x);
            info!("Token Y: {}", pool.token_y);
            info!("Active Bin ID: {}", pool.active_bin_id);
            info!("Bin Step: {}", pool.bin_step);
            info!("Total Liquidity: {}", pool.liquidity);
            info!("24h Volume: {}", pool.volume_24h);
            info!("24h Fees: {}", pool.fees_24h);
            info!("APR: {:.2}%", pool.apr);
        },
        Err(e) => {
            error!("❌ Failed to fetch pool info: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = DLMMClient::new("https://api.mainnet-beta.solana.com");
        assert!(client.is_ok());
    }

    #[test]
    fn test_decimal_conversion() {
        let amount = 1.5_f64;
        let decimal = Decimal::from_f64(amount);
        assert!(decimal.is_some());
        assert_eq!(decimal.unwrap().to_string(), "1.5");
    }

    #[test]
    fn test_slippage_calculation() {
        let amount = Decimal::from_str("100").unwrap();
        let slippage = Decimal::from_str("0.005").unwrap(); // 0.5%
        let min_out = amount * (Decimal::ONE - slippage);
        assert_eq!(min_out.to_string(), "99.5");
    }
}