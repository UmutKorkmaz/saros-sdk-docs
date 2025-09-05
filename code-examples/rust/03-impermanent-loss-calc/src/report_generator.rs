//! Multi-format report generation for IL analysis

use anyhow::Result;
use chrono::{DateTime, Utc};
use log::info;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::types::{
    PositionAnalysis, ReportConfig, ReportFormat, ImpermanentLossResult, HistoricalTrends, 
    PriceDataPoint, PositionInfo, FeeAnalysis, RiskMetrics, PerformanceSummary,
    TrendDirection,
};

/// Multi-format report generator for IL analysis
pub struct ReportGenerator {
    output_directory: PathBuf,
    report_counter: u64,
}

impl ReportGenerator {
    /// Create a new report generator
    pub fn new<P: AsRef<Path>>(output_directory: P) -> Result<Self> {
        let output_dir = output_directory.as_ref().to_path_buf();
        create_dir_all(&output_dir)?;
        
        info!("Report generator initialized with output directory: {:?}", output_dir);
        
        Ok(Self {
            output_directory: output_dir,
            report_counter: 0,
        })
    }

    /// Generate a comprehensive analysis report
    pub async fn generate_report(
        &mut self,
        analysis: &PositionAnalysis,
        il_result: &ImpermanentLossResult,
        config: &ReportConfig,
        format: ReportFormat,
    ) -> Result<PathBuf> {
        self.report_counter += 1;
        
        let timestamp_str = config.timestamp.format("%Y%m%d_%H%M%S").to_string();
        let filename = match format {
            ReportFormat::Json => format!("il_analysis_{}_{}.json", timestamp_str, self.report_counter),
            ReportFormat::Csv => format!("il_analysis_{}_{}.csv", timestamp_str, self.report_counter),
            ReportFormat::Html => format!("il_analysis_{}_{}.html", timestamp_str, self.report_counter),
        };
        
        let file_path = self.output_directory.join(filename);
        
        match format {
            ReportFormat::Json => self.generate_json_report(analysis, il_result, config, &file_path).await?,
            ReportFormat::Csv => self.generate_csv_report(analysis, il_result, config, &file_path).await?,
            ReportFormat::Html => self.generate_html_report(analysis, il_result, config, &file_path).await?,
        }
        
        info!("Generated {} report: {:?}", format, file_path);
        Ok(file_path)
    }

    /// Generate a historical trends report
    pub async fn generate_historical_report(
        &mut self,
        trends: &HistoricalTrends,
        il_history: &[ImpermanentLossResult],
        price_history: &[PriceDataPoint],
        config: &ReportConfig,
        format: ReportFormat,
    ) -> Result<PathBuf> {
        self.report_counter += 1;
        
        let timestamp_str = config.timestamp.format("%Y%m%d_%H%M%S").to_string();
        let filename = match format {
            ReportFormat::Json => format!("historical_analysis_{}_{}.json", timestamp_str, self.report_counter),
            ReportFormat::Csv => format!("historical_analysis_{}_{}.csv", timestamp_str, self.report_counter),
            ReportFormat::Html => format!("historical_analysis_{}_{}.html", timestamp_str, self.report_counter),
        };
        
        let file_path = self.output_directory.join(filename);
        
        match format {
            ReportFormat::Json => self.generate_historical_json(trends, il_history, price_history, config, &file_path).await?,
            ReportFormat::Csv => self.generate_historical_csv(trends, il_history, price_history, config, &file_path).await?,
            ReportFormat::Html => self.generate_historical_html(trends, il_history, price_history, config, &file_path).await?,
        }
        
        info!("Generated historical {} report: {:?}", format, file_path);
        Ok(file_path)
    }

    /// Generate JSON report
    async fn generate_json_report(
        &self,
        analysis: &PositionAnalysis,
        il_result: &ImpermanentLossResult,
        config: &ReportConfig,
        file_path: &Path,
    ) -> Result<()> {
        let report_data = json!({
            "report_info": {
                "title": config.title,
                "generated_at": config.timestamp,
                "format": "JSON",
                "version": "1.0"
            },
            "summary": {
                "impermanent_loss_percentage": il_result.il_percentage,
                "impermanent_loss_usd": il_result.il_usd_value,
                "current_position_value": il_result.current_value_usd,
                "hold_strategy_value": il_result.hold_value_usd,
                "net_pnl": analysis.performance_summary.net_pnl,
                "total_fees_earned": analysis.fee_analysis.total_fees_earned,
                "performance_vs_hold": analysis.performance_summary.vs_hold_performance
            },
            "position_details": {
                "pool_address": analysis.position_info.pool_address,
                "position_id": analysis.position_info.position_id,
                "token_pair": format!("{}/{}", analysis.position_info.token_x_symbol, analysis.position_info.token_y_symbol),
                "bin_range": {
                    "lower": analysis.position_info.lower_bin_id,
                    "upper": analysis.position_info.upper_bin_id,
                    "active": il_result.metadata.active_bin_id
                },
                "liquidity": analysis.position_info.current_liquidity,
                "created_at": analysis.position_info.created_at,
                "days_active": analysis.performance_summary.days_active
            },
            "price_analysis": {
                "initial_prices": {
                    "token_x": il_result.initial_price_x,
                    "token_y": il_result.initial_price_y
                },
                "current_prices": {
                    "token_x": il_result.current_price_x,
                    "token_y": il_result.current_price_y
                },
                "price_ratio_change": il_result.price_ratio_change,
                "price_volatility": analysis.risk_metrics.price_volatility
            },
            "fee_analysis": self.serialize_fee_analysis(&analysis.fee_analysis),
            "risk_metrics": self.serialize_risk_metrics(&analysis.risk_metrics),
            "performance_metrics": self.serialize_performance_summary(&analysis.performance_summary),
            "calculation_metadata": {
                "method": format!("{:?}", il_result.metadata.calculation_method),
                "price_range_coverage": il_result.metadata.price_range_coverage,
                "calculation_timestamp": il_result.timestamp
            }
        });

        let mut file = File::create(file_path)?;
        file.write_all(serde_json::to_string_pretty(&report_data)?.as_bytes())?;
        
        Ok(())
    }

    /// Generate CSV report
    async fn generate_csv_report(
        &self,
        analysis: &PositionAnalysis,
        il_result: &ImpermanentLossResult,
        config: &ReportConfig,
        file_path: &Path,
    ) -> Result<()> {
        let mut csv_content = String::new();
        
        // Header
        csv_content.push_str("Metric,Value,Unit\n");
        
        // Basic metrics
        csv_content.push_str(&format!("Report Title,\"{}\",\n", config.title));
        csv_content.push_str(&format!("Generated At,\"{}\",\n", config.timestamp));
        csv_content.push_str(&format!("Pool Address,\"{}\",\n", analysis.position_info.pool_address));
        csv_content.push_str(&format!("Position ID,\"{:?}\",\n", analysis.position_info.position_id));
        csv_content.push_str(&format!("Token Pair,\"{}/{}\",\n", 
            analysis.position_info.token_x_symbol, analysis.position_info.token_y_symbol));
        
        // IL metrics
        csv_content.push_str(&format!("Impermanent Loss,{:.6},%\n", il_result.il_percentage * Decimal::new(100, 0)));
        csv_content.push_str(&format!("IL USD Value,{:.2},USD\n", il_result.il_usd_value));
        csv_content.push_str(&format!("Current Position Value,{:.2},USD\n", il_result.current_value_usd));
        csv_content.push_str(&format!("Hold Strategy Value,{:.2},USD\n", il_result.hold_value_usd));
        
        // Price data
        csv_content.push_str(&format!("Initial Price X,{:.6},USD\n", il_result.initial_price_x));
        csv_content.push_str(&format!("Initial Price Y,{:.6},USD\n", il_result.initial_price_y));
        csv_content.push_str(&format!("Current Price X,{:.6},USD\n", il_result.current_price_x));
        csv_content.push_str(&format!("Current Price Y,{:.6},USD\n", il_result.current_price_y));
        csv_content.push_str(&format!("Price Ratio Change,{:.6},ratio\n", il_result.price_ratio_change));
        
        // Fee analysis
        csv_content.push_str(&format!("Total Fees Earned,{:.2},USD\n", analysis.fee_analysis.total_fees_earned));
        csv_content.push_str(&format!("Fee APY,{:.2},%\n", analysis.fee_analysis.fee_apy * Decimal::new(100, 0)));
        csv_content.push_str(&format!("Daily Fee Rate,{:.4},%\n", analysis.fee_analysis.daily_fee_rate * Decimal::new(100, 0)));
        csv_content.push_str(&format!("Fee vs IL Ratio,{:.2},ratio\n", analysis.fee_analysis.fee_vs_il_ratio));
        
        // Performance metrics
        csv_content.push_str(&format!("Net PnL,{:.2},USD\n", analysis.performance_summary.net_pnl));
        csv_content.push_str(&format!("Total Return,{:.2},%\n", analysis.performance_summary.total_return_percentage * Decimal::new(100, 0)));
        csv_content.push_str(&format!("Annualized Return,{:.2},%\n", analysis.performance_summary.annualized_return * Decimal::new(100, 0)));
        csv_content.push_str(&format!("Performance vs Hold,{:.2},USD\n", analysis.performance_summary.vs_hold_performance));
        csv_content.push_str(&format!("Days Active,{},days\n", analysis.performance_summary.days_active));
        
        // Risk metrics
        csv_content.push_str(&format!("Price Volatility,{:.4},\n", analysis.risk_metrics.price_volatility));
        csv_content.push_str(&format!("Max IL Observed,{:.4},%\n", analysis.risk_metrics.max_il_observed * Decimal::new(100, 0)));
        csv_content.push_str(&format!("VaR 95%,{:.2},USD\n", analysis.risk_metrics.var_95));
        csv_content.push_str(&format!("Sharpe Ratio,{:.3},\n", analysis.risk_metrics.sharpe_ratio));
        csv_content.push_str(&format!("Concentration Risk,{:.4},\n", analysis.risk_metrics.concentration_risk));
        csv_content.push_str(&format!("Bin Utilization,{:.2},%\n", analysis.risk_metrics.bin_utilization * Decimal::new(100, 0)));

        let mut file = File::create(file_path)?;
        file.write_all(csv_content.as_bytes())?;
        
        Ok(())
    }

    /// Generate HTML report
    async fn generate_html_report(
        &self,
        analysis: &PositionAnalysis,
        il_result: &ImpermanentLossResult,
        config: &ReportConfig,
        file_path: &Path,
    ) -> Result<()> {
        let html_content = format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; background-color: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 10px; box-shadow: 0 0 20px rgba(0,0,0,0.1); }}
        h1 {{ color: #2c3e50; border-bottom: 3px solid #3498db; padding-bottom: 10px; }}
        h2 {{ color: #34495e; margin-top: 30px; }}
        .metrics-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 20px; margin: 20px 0; }}
        .metric-card {{ background: #ecf0f1; padding: 15px; border-radius: 8px; border-left: 4px solid #3498db; }}
        .metric-value {{ font-size: 1.5em; font-weight: bold; color: #2c3e50; }}
        .metric-label {{ font-size: 0.9em; color: #7f8c8d; margin-top: 5px; }}
        .positive {{ color: #27ae60; }}
        .negative {{ color: #e74c3c; }}
        .warning {{ color: #f39c12; }}
        .info-table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
        .info-table th, .info-table td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        .info-table th {{ background-color: #34495e; color: white; }}
        .summary {{ background: #3498db; color: white; padding: 20px; border-radius: 8px; margin: 20px 0; }}
        .alert {{ background: #e74c3c; color: white; padding: 15px; border-radius: 8px; margin: 10px 0; }}
        .success {{ background: #27ae60; color: white; padding: 15px; border-radius: 8px; margin: 10px 0; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>{}</h1>
        <p><strong>Generated:</strong> {}</p>
        
        <div class="summary">
            <h2>Executive Summary</h2>
            <p><strong>Pool:</strong> {}</p>
            <p><strong>Position:</strong> {} (Active for {} days)</p>
            <p><strong>Token Pair:</strong> {}/{}</p>
        </div>

        {}

        <h2>💰 Performance Overview</h2>
        <div class="metrics-grid">
            <div class="metric-card">
                <div class="metric-value {}">{}%</div>
                <div class="metric-label">Impermanent Loss</div>
            </div>
            <div class="metric-card">
                <div class="metric-value {}">${}  </div>
                <div class="metric-label">IL USD Value</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">${}</div>
                <div class="metric-label">Current Position Value</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">${}</div>
                <div class="metric-label">Hold Strategy Value</div>
            </div>
        </div>

        <h2>📊 Fee Analysis</h2>
        <div class="metrics-grid">
            <div class="metric-card">
                <div class="metric-value">${}</div>
                <div class="metric-label">Total Fees Earned</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}%</div>
                <div class="metric-label">Fee APY</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Fee vs IL Ratio</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Break-even Days</div>
            </div>
        </div>

        <h2>⚡ Risk Metrics</h2>
        <div class="metrics-grid">
            <div class="metric-card">
                <div class="metric-value">{}%</div>
                <div class="metric-label">Price Volatility</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">${}</div>
                <div class="metric-label">Value at Risk (95%)</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Sharpe Ratio</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}%</div>
                <div class="metric-label">Bin Utilization</div>
            </div>
        </div>

        <h2>📈 Detailed Analysis</h2>
        <table class="info-table">
            <tr><th>Metric</th><th>Value</th></tr>
            <tr><td>Price Ratio Change</td><td>{:.4}x</td></tr>
            <tr><td>Net PnL</td><td class="{}">${}</td></tr>
            <tr><td>Total Return</td><td class="{}">{:.2}%</td></tr>
            <tr><td>Annualized Return</td><td class="{}">{:.2}%</td></tr>
            <tr><td>Performance vs Hold</td><td class="{}">${}</td></tr>
            <tr><td>Initial Investment</td><td>${}</td></tr>
            <tr><td>Concentration Risk</td><td>{:.4}</td></tr>
        </table>

        <h2>🏷️ Position Details</h2>
        <table class="info-table">
            <tr><th>Property</th><th>Value</th></tr>
            <tr><td>Pool Address</td><td><code>{}</code></td></tr>
            <tr><td>Position ID</td><td><code>{:?}</code></td></tr>
            <tr><td>Lower Bin ID</td><td>{}</td></tr>
            <tr><td>Upper Bin ID</td><td>{}</td></tr>
            <tr><td>Active Bin ID</td><td>{:?}</td></tr>
            <tr><td>Current Liquidity</td><td>{}</td></tr>
            <tr><td>Created At</td><td>{}</td></tr>
            <tr><td>Price Range Coverage</td><td>{:?}</td></tr>
        </table>

        <p style="margin-top: 40px; text-align: center; color: #7f8c8d; font-size: 0.9em;">
            Generated by Saros DLMM Impermanent Loss Calculator | {}<br>
            This report is for informational purposes only and should not be considered financial advice.
        </p>
    </div>
</body>
</html>
"#,
            config.title,
            config.title,
            config.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            analysis.position_info.pool_address,
            analysis.position_info.position_id.map_or("Pool-level".to_string(), |id| format!("{}", id)),
            analysis.performance_summary.days_active,
            analysis.position_info.token_x_symbol,
            analysis.position_info.token_y_symbol,
            
            // IL Alert box
            if il_result.il_percentage.abs() > Decimal::new(5, 2) {
                format!(r#"<div class="alert">⚠️ <strong>High Impermanent Loss Detected:</strong> {:.2}% - Consider your risk tolerance and fee compensation.</div>"#,
                       il_result.il_percentage * Decimal::new(100, 0))
            } else if analysis.fee_analysis.fee_vs_il_ratio > Decimal::ONE {
                format!(r#"<div class="success">✅ <strong>Fees Compensating IL:</strong> Fee-to-IL ratio of {:.2}x indicates positive performance.</div>"#,
                       analysis.fee_analysis.fee_vs_il_ratio)
            } else {
                "".to_string()
            },
            
            // IL percentage styling and value
            if il_result.il_percentage < Decimal::ZERO { "negative" } else { "positive" },
            (il_result.il_percentage * Decimal::new(100, 0)),
            
            // IL USD value styling
            if il_result.il_usd_value < Decimal::ZERO { "negative" } else { "positive" },
            il_result.il_usd_value,
            
            il_result.current_value_usd,
            il_result.hold_value_usd,
            
            // Fee analysis
            analysis.fee_analysis.total_fees_earned,
            (analysis.fee_analysis.fee_apy * Decimal::new(100, 0)),
            analysis.fee_analysis.fee_vs_il_ratio,
            analysis.fee_analysis.break_even_days.map_or("N/A".to_string(), |d| d.to_string()),
            
            // Risk metrics
            (analysis.risk_metrics.price_volatility * Decimal::new(100, 0)),
            analysis.risk_metrics.var_95,
            analysis.risk_metrics.sharpe_ratio,
            (analysis.risk_metrics.bin_utilization * Decimal::new(100, 0)),
            
            // Detailed analysis
            il_result.price_ratio_change,
            if analysis.performance_summary.net_pnl >= Decimal::ZERO { "positive" } else { "negative" },
            analysis.performance_summary.net_pnl,
            if analysis.performance_summary.total_return_percentage >= Decimal::ZERO { "positive" } else { "negative" },
            analysis.performance_summary.total_return_percentage * Decimal::new(100, 0),
            if analysis.performance_summary.annualized_return >= Decimal::ZERO { "positive" } else { "negative" },
            analysis.performance_summary.annualized_return * Decimal::new(100, 0),
            if analysis.performance_summary.vs_hold_performance >= Decimal::ZERO { "positive" } else { "negative" },
            analysis.performance_summary.vs_hold_performance,
            analysis.position_info.initial_investment_usd,
            analysis.risk_metrics.concentration_risk,
            
            // Position details
            analysis.position_info.pool_address,
            analysis.position_info.position_id,
            analysis.position_info.lower_bin_id,
            analysis.position_info.upper_bin_id,
            il_result.metadata.active_bin_id,
            analysis.position_info.current_liquidity,
            analysis.position_info.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
            il_result.metadata.price_range_coverage,
            config.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );

        let mut file = File::create(file_path)?;
        file.write_all(html_content.as_bytes())?;
        
        Ok(())
    }

    /// Generate historical JSON report
    async fn generate_historical_json(
        &self,
        trends: &HistoricalTrends,
        il_history: &[ImpermanentLossResult],
        price_history: &[PriceDataPoint],
        config: &ReportConfig,
        file_path: &Path,
    ) -> Result<()> {
        let report_data = json!({
            "report_info": {
                "title": config.title,
                "generated_at": config.timestamp,
                "format": "JSON",
                "type": "historical_analysis",
                "version": "1.0"
            },
            "analysis_period": {
                "days": trends.period_days,
                "data_points": il_history.len(),
                "price_points": price_history.len()
            },
            "trend_summary": {
                "direction": format!("{:?}", trends.trend_direction),
                "max_il_percentage": trends.max_il_percentage,
                "min_il_percentage": trends.min_il_percentage,
                "average_il_percentage": trends.avg_il_percentage,
                "il_volatility": trends.il_volatility,
                "market_correlation": trends.market_correlation,
                "avg_daily_price_change": trends.avg_daily_price_change,
                "max_drawdown_days": trends.max_drawdown_days
            },
            "recovery_analysis": {
                "recovery_periods": trends.recovery_periods.len(),
                "recovery_events": trends.recovery_periods.iter().map(|r| json!({
                    "start_date": r.start_date,
                    "end_date": r.end_date,
                    "max_il": r.max_il_in_period,
                    "recovery_days": r.recovery_days,
                    "fee_compensation": r.fee_compensation
                })).collect::<Vec<_>>()
            },
            "raw_data": if config.include_raw_data {
                Some(json!({
                    "il_history": il_history,
                    "price_history": price_history
                }))
            } else {
                None
            }
        });

        let mut file = File::create(file_path)?;
        file.write_all(serde_json::to_string_pretty(&report_data)?.as_bytes())?;
        
        Ok(())
    }

    /// Generate historical CSV report
    async fn generate_historical_csv(
        &self,
        trends: &HistoricalTrends,
        il_history: &[ImpermanentLossResult],
        price_history: &[PriceDataPoint],
        config: &ReportConfig,
        file_path: &Path,
    ) -> Result<()> {
        let mut csv_content = String::new();
        
        // Write header
        csv_content.push_str("Timestamp,IL_Percentage,IL_USD_Value,Price_X,Price_Y,Price_Ratio_Change,Current_Value,Hold_Value\n");
        
        // Write IL history data
        for il_result in il_history {
            csv_content.push_str(&format!(
                "{},{:.6},{:.2},{:.6},{:.6},{:.6},{:.2},{:.2}\n",
                il_result.timestamp.format("%Y-%m-%d %H:%M:%S"),
                il_result.il_percentage,
                il_result.il_usd_value,
                il_result.current_price_x,
                il_result.current_price_y,
                il_result.price_ratio_change,
                il_result.current_value_usd,
                il_result.hold_value_usd
            ));
        }
        
        let mut file = File::create(file_path)?;
        file.write_all(csv_content.as_bytes())?;
        
        Ok(())
    }

    /// Generate historical HTML report
    async fn generate_historical_html(
        &self,
        trends: &HistoricalTrends,
        il_history: &[ImpermanentLossResult],
        price_history: &[PriceDataPoint],
        config: &ReportConfig,
        file_path: &Path,
    ) -> Result<()> {
        let html_content = format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; background-color: #f5f5f5; }}
        .container {{ max-width: 1400px; margin: 0 auto; background: white; padding: 30px; border-radius: 10px; box-shadow: 0 0 20px rgba(0,0,0,0.1); }}
        h1 {{ color: #2c3e50; border-bottom: 3px solid #3498db; padding-bottom: 10px; }}
        h2 {{ color: #34495e; margin-top: 30px; }}
        .metrics-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; margin: 20px 0; }}
        .metric-card {{ background: #ecf0f1; padding: 15px; border-radius: 8px; border-left: 4px solid #3498db; }}
        .metric-value {{ font-size: 1.3em; font-weight: bold; color: #2c3e50; }}
        .metric-label {{ font-size: 0.9em; color: #7f8c8d; margin-top: 5px; }}
        .chart-container {{ width: 100%; height: 400px; margin: 20px 0; }}
        .trend-{}{{ border-left-color: {}; }}
        .summary {{ background: #34495e; color: white; padding: 20px; border-radius: 8px; margin: 20px 0; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>{}</h1>
        <p><strong>Generated:</strong> {}</p>
        
        <div class="summary">
            <h2>Historical Analysis Summary</h2>
            <p><strong>Analysis Period:</strong> {} days ({} data points)</p>
            <p><strong>Trend Direction:</strong> {:?}</p>
            <p><strong>Max IL Observed:</strong> {:.2}%</p>
            <p><strong>Average IL:</strong> {:.2}%</p>
        </div>

        <h2>📊 Historical Metrics</h2>
        <div class="metrics-grid">
            <div class="metric-card trend-{}">
                <div class="metric-value">{:.2}%</div>
                <div class="metric-label">Maximum IL</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.2}%</div>
                <div class="metric-label">Minimum IL</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.2}%</div>
                <div class="metric-label">Average IL</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.4}</div>
                <div class="metric-label">IL Volatility</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.3}</div>
                <div class="metric-label">Market Correlation</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Max Drawdown Days</div>
            </div>
        </div>

        {}

        <p style="margin-top: 40px; text-align: center; color: #7f8c8d; font-size: 0.9em;">
            Generated by Saros DLMM Historical IL Analyzer | {}
        </p>
    </div>
</body>
</html>
"#,
            config.title,
            
            // Trend styling
            match trends.trend_direction {
                TrendDirection::Bullish => ("bullish", "#27ae60"),
                TrendDirection::Bearish => ("bearish", "#e74c3c"),
                TrendDirection::Volatile => ("volatile", "#f39c12"),
                TrendDirection::Sideways => ("sideways", "#95a5a6"),
            }.0,
            match trends.trend_direction {
                TrendDirection::Bullish => ("bullish", "#27ae60"),
                TrendDirection::Bearish => ("bearish", "#e74c3c"),
                TrendDirection::Volatile => ("volatile", "#f39c12"),
                TrendDirection::Sideways => ("sideways", "#95a5a6"),
            }.1,
            
            config.title,
            config.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            trends.period_days,
            il_history.len(),
            trends.trend_direction,
            trends.max_il_percentage * Decimal::new(100, 0),
            trends.avg_il_percentage * Decimal::new(100, 0),
            
            // Metrics with trend styling
            match trends.trend_direction {
                TrendDirection::Bullish => "bullish",
                TrendDirection::Bearish => "bearish", 
                TrendDirection::Volatile => "volatile",
                TrendDirection::Sideways => "sideways",
            },
            trends.max_il_percentage * Decimal::new(100, 0),
            trends.min_il_percentage * Decimal::new(100, 0),
            trends.avg_il_percentage * Decimal::new(100, 0),
            trends.il_volatility,
            trends.market_correlation,
            trends.max_drawdown_days,
            
            // Recovery periods section
            if !trends.recovery_periods.is_empty() {
                format!(r#"
                <h2>🔄 Recovery Analysis</h2>
                <p>Detected {} recovery periods after significant IL events:</p>
                <ul>
                {}</ul>
                "#,
                trends.recovery_periods.len(),
                trends.recovery_periods.iter().map(|r| format!(
                    "<li>{} to {} ({} days, Max IL: {:.2}%)</li>",
                    r.start_date.format("%Y-%m-%d"),
                    r.end_date.format("%Y-%m-%d"),
                    r.recovery_days,
                    r.max_il_in_period * Decimal::new(100, 0)
                )).collect::<Vec<_>>().join("\n")
                )
            } else {
                "<h2>🔄 Recovery Analysis</h2><p>No significant recovery periods detected in the analyzed timeframe.</p>".to_string()
            },
            
            config.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );

        let mut file = File::create(file_path)?;
        file.write_all(html_content.as_bytes())?;
        
        Ok(())
    }

    /// Serialize fee analysis to JSON Value
    fn serialize_fee_analysis(&self, fee_analysis: &FeeAnalysis) -> Value {
        json!({
            "total_fees_earned": fee_analysis.total_fees_earned,
            "fees_token_x": fee_analysis.fees_token_x,
            "fees_token_y": fee_analysis.fees_token_y,
            "fee_apy": fee_analysis.fee_apy,
            "daily_fee_rate": fee_analysis.daily_fee_rate,
            "fee_vs_il_ratio": fee_analysis.fee_vs_il_ratio,
            "break_even_days": fee_analysis.break_even_days
        })
    }

    /// Serialize risk metrics to JSON Value
    fn serialize_risk_metrics(&self, risk_metrics: &RiskMetrics) -> Value {
        json!({
            "price_volatility": risk_metrics.price_volatility,
            "max_il_observed": risk_metrics.max_il_observed,
            "var_95": risk_metrics.var_95,
            "sharpe_ratio": risk_metrics.sharpe_ratio,
            "concentration_risk": risk_metrics.concentration_risk,
            "bin_utilization": risk_metrics.bin_utilization
        })
    }

    /// Serialize performance summary to JSON Value
    fn serialize_performance_summary(&self, performance: &PerformanceSummary) -> Value {
        json!({
            "total_return_usd": performance.total_return_usd,
            "total_return_percentage": performance.total_return_percentage,
            "annualized_return": performance.annualized_return,
            "net_pnl": performance.net_pnl,
            "days_active": performance.days_active,
            "vs_hold_performance": performance.vs_hold_performance,
            "vs_market_performance": performance.vs_market_performance
        })
    }

    /// Get the current report counter
    pub fn get_report_counter(&self) -> u64 {
        self.report_counter
    }

    /// Set output directory
    pub fn set_output_directory<P: AsRef<Path>>(&mut self, directory: P) -> Result<()> {
        self.output_directory = directory.as_ref().to_path_buf();
        create_dir_all(&self.output_directory)?;
        info!("Changed output directory to: {:?}", self.output_directory);
        Ok(())
    }

    /// List generated reports
    pub fn list_reports(&self) -> Result<Vec<PathBuf>> {
        let mut reports = Vec::new();
        
        if self.output_directory.exists() {
            for entry in std::fs::read_dir(&self.output_directory)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_file() {
                    if let Some(extension) = path.extension() {
                        if matches!(extension.to_str(), Some("json") | Some("csv") | Some("html")) {
                            reports.push(path);
                        }
                    }
                }
            }
        }
        
        reports.sort();
        Ok(reports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::types::*;

    #[tokio::test]
    async fn test_json_report_generation() {
        let temp_dir = tempdir().unwrap();
        let mut generator = ReportGenerator::new(temp_dir.path()).unwrap();
        
        let analysis = create_mock_analysis();
        let il_result = create_mock_il_result();
        let config = ReportConfig {
            title: "Test Report".to_string(),
            include_charts: true,
            include_raw_data: true,
            timestamp: Utc::now(),
        };
        
        let report_path = generator.generate_report(
            &analysis, 
            &il_result, 
            &config, 
            ReportFormat::Json
        ).await.unwrap();
        
        assert!(report_path.exists());
        assert!(report_path.to_string_lossy().ends_with(".json"));
        
        // Verify JSON is valid
        let content = std::fs::read_to_string(&report_path).unwrap();
        let _: Value = serde_json::from_str(&content).unwrap();
    }

    #[tokio::test]
    async fn test_multiple_format_generation() {
        let temp_dir = tempdir().unwrap();
        let mut generator = ReportGenerator::new(temp_dir.path()).unwrap();
        
        let analysis = create_mock_analysis();
        let il_result = create_mock_il_result();
        let config = ReportConfig {
            title: "Multi-format Test".to_string(),
            include_charts: false,
            include_raw_data: false,
            timestamp: Utc::now(),
        };
        
        let formats = vec![ReportFormat::Json, ReportFormat::Csv, ReportFormat::Html];
        let mut paths = Vec::new();
        
        for format in formats {
            let path = generator.generate_report(&analysis, &il_result, &config, format).await.unwrap();
            paths.push(path);
        }
        
        assert_eq!(paths.len(), 3);
        for path in paths {
            assert!(path.exists());
        }
        
        let reports = generator.list_reports().unwrap();
        assert!(reports.len() >= 3);
    }

    fn create_mock_analysis() -> PositionAnalysis {
        PositionAnalysis {
            position_info: PositionInfo {
                position_id: Some(Pubkey::new_unique()),
                pool_address: Pubkey::new_unique(),
                owner: Pubkey::new_unique(),
                token_x_symbol: "TOKEN_X".to_string(),
                token_y_symbol: "TOKEN_Y".to_string(),
                lower_bin_id: -50,
                upper_bin_id: 50,
                current_liquidity: Decimal::new(100000, 0),
                initial_investment_usd: Decimal::new(2000, 0),
                current_value_usd: Decimal::new(2100, 0),
                created_at: Utc::now() - chrono::Duration::days(30),
                last_updated: Utc::now(),
            },
            il_result: create_mock_il_result(),
            fee_analysis: FeeAnalysis {
                total_fees_earned: Decimal::new(150, 0),
                fees_token_x: Decimal::new(75, 6),
                fees_token_y: Decimal::new(75, 6),
                fee_apy: Decimal::new(15, 2),
                daily_fee_rate: Decimal::new(4, 4),
                fee_vs_il_ratio: Decimal::new(3, 0),
                break_even_days: Some(20),
            },
            risk_metrics: RiskMetrics {
                price_volatility: Decimal::new(25, 2),
                max_il_observed: Decimal::new(3, 2),
                var_95: Decimal::new(100, 0),
                sharpe_ratio: Decimal::new(12, 1),
                concentration_risk: Decimal::new(2, 1),
                bin_utilization: Decimal::new(75, 2),
            },
            performance_summary: PerformanceSummary {
                total_return_usd: Decimal::new(100, 0),
                total_return_percentage: Decimal::new(5, 2),
                annualized_return: Decimal::new(60, 2),
                net_pnl: Decimal::new(50, 0),
                days_active: 30,
                vs_hold_performance: Decimal::new(25, 0),
                vs_market_performance: Some(Decimal::new(10, 0)),
            },
            timestamp: Utc::now(),
        }
    }

    fn create_mock_il_result() -> ImpermanentLossResult {
        ImpermanentLossResult {
            il_percentage: Decimal::new(-25, 3), // -2.5%
            il_usd_value: Decimal::new(-50, 0),
            current_value_usd: Decimal::new(2100, 0),
            hold_value_usd: Decimal::new(2150, 0),
            current_price_x: Decimal::new(105, 0),
            current_price_y: Decimal::new(100, 0),
            initial_price_x: Decimal::new(100, 0),
            initial_price_y: Decimal::new(100, 0),
            price_ratio_change: Decimal::new(105, 2),
            timestamp: Utc::now(),
            metadata: ILMetadata {
                pool_address: Pubkey::new_unique(),
                position_id: Some(Pubkey::new_unique()),
                bin_range: Some((-50, 50)),
                active_bin_id: Some(0),
                price_range_coverage: Some(Decimal::ONE),
                calculation_method: CalculationMethod::FromPosition,
            },
        }
    }
}