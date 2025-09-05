//! Saros DLMM Impermanent Loss Calculator
//! 
//! This application provides comprehensive analytics for DLMM positions including:
//! - Impermanent loss calculations with mathematical precision
//! - Real-time position monitoring and tracking
//! - Fee vs IL analysis and profitability metrics
//! - Historical price data analysis and volatility tracking
//! - Multi-format report generation (JSON, CSV, HTML)

mod il_calculator;
mod position_analyzer;
mod price_monitor;
mod report_generator;
mod types;

use anyhow::Result;
use clap::{Arg, Command};
use dotenv::dotenv;
use log::info;
use rust_decimal::Decimal;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tokio::time::{sleep, Duration};

use crate::il_calculator::ILCalculator;
use crate::position_analyzer::PositionAnalyzer;
use crate::price_monitor::PriceMonitor;
use crate::report_generator::ReportGenerator;
use crate::types::ReportFormat;
use crate::types::{AnalysisConfig, MonitoringMode, ReportConfig, ImpermanentLossResult};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let matches = Command::new("il_calc")
        .version("0.1.0")
        .about("DLMM Impermanent Loss Calculator with Real-time Analytics")
        .arg(
            Arg::new("pool")
                .short('p')
                .long("pool")
                .value_name("POOL_ADDRESS")
                .help("DLMM pool address to analyze")
                .required(true),
        )
        .arg(
            Arg::new("position")
                .short('P')
                .long("position")
                .value_name("POSITION_ID")
                .help("Specific position ID to analyze (optional)"),
        )
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .value_name("MODE")
                .help("Analysis mode: snapshot, monitor, historical")
                .default_value("snapshot"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("OUTPUT_DIR")
                .help("Output directory for reports")
                .default_value("./reports"),
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("FORMAT")
                .help("Report format: json, csv, html, all")
                .default_value("json"),
        )
        .arg(
            Arg::new("interval")
                .short('i')
                .long("interval")
                .value_name("SECONDS")
                .help("Monitoring interval in seconds (for monitor mode)")
                .default_value("60"),
        )
        .arg(
            Arg::new("initial-price-x")
                .long("initial-price-x")
                .value_name("PRICE")
                .help("Initial price of token X (for manual IL calculation)"),
        )
        .arg(
            Arg::new("initial-price-y")
                .long("initial-price-y")
                .value_name("PRICE")
                .help("Initial price of token Y (for manual IL calculation)"),
        )
        .arg(
            Arg::new("initial-amount-x")
                .long("initial-amount-x")
                .value_name("AMOUNT")
                .help("Initial amount of token X deposited"),
        )
        .arg(
            Arg::new("initial-amount-y")
                .long("initial-amount-y")
                .value_name("AMOUNT")
                .help("Initial amount of token Y deposited"),
        )
        .get_matches();

    // Parse command line arguments
    let pool_address = Pubkey::from_str(matches.get_one::<String>("pool").unwrap())?;
    let position_id = matches
        .get_one::<String>("position")
        .map(|s| Pubkey::from_str(s))
        .transpose()?;
    
    let mode = match matches.get_one::<String>("mode").unwrap().as_str() {
        "snapshot" => MonitoringMode::Snapshot,
        "monitor" => MonitoringMode::RealTime,
        "historical" => MonitoringMode::Historical,
        _ => MonitoringMode::Snapshot,
    };

    let output_dir = matches.get_one::<String>("output").unwrap().to_string();
    let format_str = matches.get_one::<String>("format").unwrap();
    let formats = parse_report_formats(format_str)?;
    
    let interval_secs: u64 = matches
        .get_one::<String>("interval")
        .unwrap()
        .parse()?;

    // Parse manual IL calculation parameters
    let manual_params = if let (Some(px), Some(py), Some(ax), Some(ay)) = (
        matches.get_one::<String>("initial-price-x"),
        matches.get_one::<String>("initial-price-y"), 
        matches.get_one::<String>("initial-amount-x"),
        matches.get_one::<String>("initial-amount-y"),
    ) {
        Some((
            Decimal::from_str(px)?,
            Decimal::from_str(py)?,
            Decimal::from_str(ax)?,
            Decimal::from_str(ay)?,
        ))
    } else {
        None
    };

    info!("Starting DLMM Impermanent Loss Calculator");
    info!("Pool: {}", pool_address);
    info!("Mode: {:?}", mode);
    info!("Output: {}", output_dir);

    // Initialize components
    let mut il_calculator = ILCalculator::new().await?;
    let mut position_analyzer = PositionAnalyzer::new().await?;
    let mut price_monitor = PriceMonitor::new().await?;
    let mut report_generator = ReportGenerator::new(output_dir.clone())?;

    // Create analysis configuration
    let config = AnalysisConfig {
        pool_address,
        position_id,
        mode,
        interval_secs,
        output_directory: output_dir,
        enable_notifications: true,
        max_price_deviation: Decimal::new(500, 2), // 5%
        min_fee_threshold: Decimal::new(100, 2), // $1.00
        historical_days: 30,
        volatility_window: 24, // hours
    };

    match mode {
        MonitoringMode::Snapshot => {
            run_snapshot_analysis(
                &mut il_calculator,
                &mut position_analyzer,
                &mut price_monitor,
                &mut report_generator,
                &config,
                &formats,
                manual_params,
            ).await?;
        }
        MonitoringMode::RealTime => {
            run_realtime_monitoring(
                &mut il_calculator,
                &mut position_analyzer,
                &mut price_monitor,
                &mut report_generator,
                &config,
                &formats,
            ).await?;
        }
        MonitoringMode::Historical => {
            run_historical_analysis(
                &mut il_calculator,
                &mut position_analyzer,
                &mut price_monitor,
                &mut report_generator,
                &config,
                &formats,
            ).await?;
        }
    }

    info!("Analysis completed successfully");
    Ok(())
}

async fn run_snapshot_analysis(
    il_calculator: &mut ILCalculator,
    position_analyzer: &mut PositionAnalyzer,
    price_monitor: &mut PriceMonitor,
    report_generator: &mut ReportGenerator,
    config: &AnalysisConfig,
    formats: &[ReportFormat],
    manual_params: Option<(Decimal, Decimal, Decimal, Decimal)>,
) -> Result<()> {
    info!("Running snapshot analysis...");

    // Get current pool and position data
    let pool_info = position_analyzer.get_pool_info(config.pool_address).await?;
    info!("Pool TVL: ${:.2}, 24h Volume: ${:.2}", pool_info.tvl, pool_info.volume_24h);

    let position_data = if let Some(position_id) = config.position_id {
        Some(position_analyzer.get_position_info(position_id).await?)
    } else {
        None
    };

    // Get current prices
    let current_prices = price_monitor.get_current_prices(
        pool_info.token_x,
        pool_info.token_y,
    ).await?;

    info!("Current prices - Token X: ${:.6}, Token Y: ${:.6}", 
          current_prices.0, current_prices.1);

    // Calculate impermanent loss
    let il_result = if let Some((initial_px, initial_py, amount_x, amount_y)) = manual_params {
        il_calculator.calculate_il_manual(
            initial_px,
            initial_py,
            current_prices.0,
            current_prices.1,
            amount_x,
            amount_y,
        ).await?
    } else if let Some(position) = &position_data {
        il_calculator.calculate_il_from_position(
            config.pool_address,
            position.position_id.unwrap(),
        ).await?
    } else {
        return Err(anyhow::anyhow!("Need either position ID or manual parameters for IL calculation"));
    };

    info!("Impermanent Loss: {:.4}% (${:.2})", 
          il_result.il_percentage * Decimal::new(100, 0), il_result.il_usd_value);

    // Generate comprehensive analysis
    let analysis = position_analyzer.analyze_position_performance(
        config.pool_address,
        position_data,
        &il_result,
    ).await?;

    info!("Total fees earned: ${:.2}", analysis.fee_analysis.total_fees_earned);
    info!("Net PnL: ${:.2}", analysis.performance_summary.net_pnl);

    // Generate reports
    let report_config = ReportConfig {
        title: format!("DLMM IL Analysis - {}", config.pool_address),
        include_charts: true,
        include_raw_data: true,
        timestamp: chrono::Utc::now(),
    };

    for format in formats {
        report_generator.generate_report(
            &analysis,
            &il_result,
            &report_config,
            format.clone(),
        ).await?;
        info!("Generated {} report", format);
    }

    Ok(())
}

async fn run_realtime_monitoring(
    il_calculator: &mut ILCalculator,
    position_analyzer: &mut PositionAnalyzer,
    price_monitor: &mut PriceMonitor,
    report_generator: &mut ReportGenerator,
    config: &AnalysisConfig,
    formats: &[ReportFormat],
) -> Result<()> {
    info!("Starting real-time monitoring (interval: {}s)...", config.interval_secs);

    let mut iteration = 0u64;
    
    loop {
        iteration += 1;
        info!("Monitoring iteration #{}", iteration);

        // Get current data
        let pool_info = position_analyzer.get_pool_info(config.pool_address).await?;
        let position_data = if let Some(position_id) = config.position_id {
            Some(position_analyzer.get_position_info(position_id).await?)
        } else {
            None
        };

        // Calculate IL if we have position data
        if let Some(position) = &position_data {
            match il_calculator.calculate_il_from_position(
                config.pool_address,
                position.position_id.unwrap(),
            ).await {
                Ok(il_result) => {
                    info!("Current IL: {:.4}% (${:.2})", 
                          il_result.il_percentage * Decimal::new(100, 0), 
                          il_result.il_usd_value);

                    // Check for significant changes
                    if il_result.il_percentage.abs() > config.max_price_deviation / Decimal::new(100, 0) {
                        info!("⚠️  High impermanent loss detected!");
                    }

                    // Generate periodic reports (every 10th iteration)
                    if iteration % 10 == 0 {
                        let analysis = position_analyzer.analyze_position_performance(
                            config.pool_address,
                            position_data.clone(),
                            &il_result,
                        ).await?;

                        let report_config = ReportConfig {
                            title: format!("DLMM Real-time Report #{} - {}", iteration, config.pool_address),
                            include_charts: true,
                            include_raw_data: false,
                            timestamp: chrono::Utc::now(),
                        };

                        // Generate JSON report for real-time monitoring
                        report_generator.generate_report(
                            &analysis,
                            &il_result,
                            &report_config,
                            ReportFormat::Json,
                        ).await?;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to calculate IL: {}", e);
                }
            }
        }

        sleep(Duration::from_secs(config.interval_secs)).await;
    }
}

async fn run_historical_analysis(
    il_calculator: &mut ILCalculator,
    position_analyzer: &mut PositionAnalyzer,
    price_monitor: &mut PriceMonitor,
    report_generator: &mut ReportGenerator,
    config: &AnalysisConfig,
    formats: &[ReportFormat],
) -> Result<()> {
    info!("Running historical analysis ({} days)...", config.historical_days);

    // Get historical data
    let pool_info = position_analyzer.get_pool_info(config.pool_address).await?;
    let historical_data = price_monitor.get_historical_data(
        pool_info.token_x,
        pool_info.token_y,
        config.historical_days,
    ).await?;

    info!("Retrieved {} historical data points", historical_data.len());

    // Calculate historical IL progression
    let il_history = il_calculator.calculate_historical_il(
        config.pool_address,
        &historical_data,
    ).await?;

    info!("Calculated IL for {} historical points", il_history.len());

    // Analyze trends and patterns
    let trend_analysis = position_analyzer.analyze_historical_trends(
        &il_history,
        &historical_data,
    ).await?;

    info!("Max historical IL: {:.4}%", trend_analysis.max_il_percentage * Decimal::new(100, 0));
    info!("Average IL: {:.4}%", trend_analysis.avg_il_percentage * Decimal::new(100, 0));

    // Generate comprehensive historical report
    let report_config = ReportConfig {
        title: format!("DLMM Historical Analysis - {} days - {}", config.historical_days, config.pool_address),
        include_charts: true,
        include_raw_data: true,
        timestamp: chrono::Utc::now(),
    };

    // Use the latest IL result for the main report
    let latest_il = il_history.last().unwrap().clone();
    
    for format in formats {
        report_generator.generate_historical_report(
            &trend_analysis,
            &il_history,
            &historical_data,
            &report_config,
            format.clone(),
        ).await?;
        info!("Generated historical {} report", format);
    }

    Ok(())
}

fn parse_report_formats(format_str: &str) -> Result<Vec<ReportFormat>> {
    match format_str.to_lowercase().as_str() {
        "all" => Ok(vec![
            ReportFormat::Json,
            ReportFormat::Csv,
            ReportFormat::Html,
        ]),
        "json" => Ok(vec![ReportFormat::Json]),
        "csv" => Ok(vec![ReportFormat::Csv]),
        "html" => Ok(vec![ReportFormat::Html]),
        _ => Err(anyhow::anyhow!("Invalid format: {}. Use: json, csv, html, all", format_str)),
    }
}