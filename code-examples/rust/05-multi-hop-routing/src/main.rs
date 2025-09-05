use anyhow::Result;
use clap::{Parser, Subcommand};
use rust_decimal::Decimal;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tracing::{info, warn, error};

mod pool_graph;
mod route_finder;
mod arbitrage_detector;
mod route_executor;
mod types;

use pool_graph::PoolGraph;
use route_finder::RouteFinder;
use arbitrage_detector::ArbitrageDetector;
use route_executor::RouteExecutor;
use types::*;

#[derive(Parser)]
#[command(name = "multi-hop-router")]
#[command(about = "Advanced multi-hop routing and arbitrage detection for Saros DLMM")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Find optimal route between two tokens
    Route {
        #[arg(long)]
        from_token: String,
        #[arg(long)]
        to_token: String,
        #[arg(long)]
        amount: String,
        #[arg(long, default_value = "3")]
        max_hops: u8,
        #[arg(long, default_value = "0.01")]
        slippage: String,
    },
    /// Detect arbitrage opportunities
    Arbitrage {
        #[arg(long, default_value = "1000")]
        min_profit_usd: String,
        #[arg(long, default_value = "4")]
        max_cycle_length: u8,
    },
    /// Execute multi-hop swap
    Execute {
        #[arg(long)]
        route_id: String,
        #[arg(long)]
        amount: String,
        #[arg(long, default_value = "false")]
        simulate: bool,
    },
    /// Analyze pool graph connectivity
    Analyze {
        #[arg(long)]
        token: Option<String>,
        #[arg(long, default_value = "false")]
        export_graph: bool,
    },
    /// Monitor real-time routing opportunities
    Monitor {
        #[arg(long)]
        tokens: Vec<String>,
        #[arg(long, default_value = "5000")]
        interval_ms: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,multi_hop_routing=debug")
        .init();

    dotenv::dotenv().ok();

    let cli = Cli::parse();

    info!("Starting Saros Multi-Hop Router");

    // Initialize components
    let pool_graph = PoolGraph::new().await?;
    let route_finder = RouteFinder::new(pool_graph.clone()).await?;
    let arbitrage_detector = ArbitrageDetector::new(pool_graph.clone()).await?;
    let route_executor = RouteExecutor::new().await?;

    match cli.command {
        Commands::Route {
            from_token,
            to_token,
            amount,
            max_hops,
            slippage,
        } => {
            handle_route_command(
                &route_finder,
                from_token,
                to_token,
                amount,
                max_hops,
                slippage,
            ).await?;
        }
        Commands::Arbitrage {
            min_profit_usd,
            max_cycle_length,
        } => {
            handle_arbitrage_command(&arbitrage_detector, min_profit_usd, max_cycle_length).await?;
        }
        Commands::Execute {
            route_id,
            amount,
            simulate,
        } => {
            handle_execute_command(&route_executor, route_id, amount, simulate).await?;
        }
        Commands::Analyze { token, export_graph } => {
            handle_analyze_command(&pool_graph, token, export_graph).await?;
        }
        Commands::Monitor { tokens, interval_ms } => {
            handle_monitor_command(&route_finder, &arbitrage_detector, tokens, interval_ms).await?;
        }
    }

    Ok(())
}

async fn handle_route_command(
    route_finder: &RouteFinder,
    from_token: String,
    to_token: String,
    amount: String,
    max_hops: u8,
    slippage: String,
) -> Result<()> {
    info!("Finding optimal route from {} to {}", from_token, to_token);

    let from_token_pubkey = Pubkey::from_str(&from_token)?;
    let to_token_pubkey = Pubkey::from_str(&to_token)?;
    let amount_decimal = Decimal::from_str(&amount)?;
    let slippage_decimal = Decimal::from_str(&slippage)?;

    let route_request = RouteRequest {
        from_token: from_token_pubkey,
        to_token: to_token_pubkey,
        amount: amount_decimal,
        max_hops,
        max_slippage: slippage_decimal,
        split_routes: true,
    };

    match route_finder.find_optimal_route(route_request).await {
        Ok(routes) => {
            println!("\n=== OPTIMAL ROUTES FOUND ===");
            for (i, route) in routes.iter().enumerate() {
                println!("\nRoute #{}: {}", i + 1, route.route_id);
                println!("Path: {}", format_route_path(&route.path));
                println!("Expected Output: {} {}", route.expected_output, to_token);
                println!("Price Impact: {:.4}%", route.price_impact * Decimal::from(100));
                println!("Gas Estimate: {} SOL", route.gas_estimate);
                println!("Confidence Score: {:.2}/10", route.confidence_score);
                
                if !route.split_routes.is_empty() {
                    println!("Split Routes:");
                    for (j, split) in route.split_routes.iter().enumerate() {
                        println!("  Split {}: {} -> {} ({}%)", 
                            j + 1, 
                            split.amount, 
                            split.expected_output,
                            split.percentage * Decimal::from(100)
                        );
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to find route: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

async fn handle_arbitrage_command(
    arbitrage_detector: &ArbitrageDetector,
    min_profit_usd: String,
    max_cycle_length: u8,
) -> Result<()> {
    info!("Scanning for arbitrage opportunities");

    let min_profit = Decimal::from_str(&min_profit_usd)?;
    
    let opportunities = arbitrage_detector
        .scan_arbitrage_opportunities(min_profit, max_cycle_length)
        .await?;

    if opportunities.is_empty() {
        println!("No arbitrage opportunities found above ${} profit threshold", min_profit);
    } else {
        println!("\n=== ARBITRAGE OPPORTUNITIES ===");
        for (i, opp) in opportunities.iter().enumerate() {
            println!("\nOpportunity #{}: {}", i + 1, opp.id);
            println!("Cycle: {}", format_arbitrage_cycle(&opp.cycle));
            println!("Expected Profit: ${:.2}", opp.expected_profit_usd);
            println!("ROI: {:.2}%", opp.roi_percentage);
            println!("Risk Score: {:.1}/10", opp.risk_score);
            println!("Confidence: {:.1}%", opp.confidence * Decimal::from(100));
            println!("Required Capital: ${:.2}", opp.required_capital_usd);
        }

        // Show execution recommendation
        if let Some(best_opp) = opportunities.first() {
            println!("\n=== EXECUTION RECOMMENDATION ===");
            println!("Best opportunity: {}", best_opp.id);
            println!("Execute with: cargo run -- execute --route-id {}", best_opp.id);
        }
    }

    Ok(())
}

async fn handle_execute_command(
    route_executor: &RouteExecutor,
    route_id: String,
    amount: String,
    simulate: bool,
) -> Result<()> {
    let amount_decimal = Decimal::from_str(&amount)?;

    if simulate {
        info!("Simulating execution of route: {}", route_id);
        let simulation = route_executor.simulate_route_execution(&route_id, amount_decimal).await?;
        
        println!("\n=== SIMULATION RESULTS ===");
        println!("Route: {}", route_id);
        println!("Input Amount: {}", amount_decimal);
        println!("Expected Output: {}", simulation.expected_output);
        println!("Price Impact: {:.4}%", simulation.total_price_impact * Decimal::from(100));
        println!("Gas Cost: {} SOL", simulation.estimated_gas);
        println!("Success Probability: {:.1}%", simulation.success_probability * Decimal::from(100));
        
        if !simulation.warnings.is_empty() {
            println!("\nWarnings:");
            for warning in &simulation.warnings {
                println!("  - {}", warning);
            }
        }
    } else {
        info!("Executing route: {}", route_id);
        warn!("Real execution not implemented in this example");
        println!("Real execution would happen here...");
    }

    Ok(())
}

async fn handle_analyze_command(
    pool_graph: &PoolGraph,
    token: Option<String>,
    export_graph: bool,
) -> Result<()> {
    info!("Analyzing pool graph connectivity");

    let stats = pool_graph.get_graph_statistics().await?;
    
    println!("\n=== POOL GRAPH ANALYSIS ===");
    println!("Total Pools: {}", stats.total_pools);
    println!("Total Tokens: {}", stats.total_tokens);
    println!("Average Pool Liquidity: ${:.2}", stats.avg_liquidity_usd);
    println!("Graph Density: {:.4}", stats.graph_density);
    println!("Largest Connected Component: {} nodes", stats.largest_component_size);

    if let Some(token_str) = token {
        let token_pubkey = Pubkey::from_str(&token_str)?;
        let token_analysis = pool_graph.analyze_token_connectivity(token_pubkey).await?;
        
        println!("\n=== TOKEN CONNECTIVITY ANALYSIS ===");
        println!("Token: {}", token_str);
        println!("Direct Pairs: {}", token_analysis.direct_pairs);
        println!("2-Hop Reachability: {}", token_analysis.two_hop_tokens);
        println!("3-Hop Reachability: {}", token_analysis.three_hop_tokens);
        println!("Liquidity Centrality: {:.4}", token_analysis.centrality_score);
    }

    if export_graph {
        let graph_file = "pool_graph.dot";
        pool_graph.export_graph_visualization(graph_file).await?;
        println!("\nGraph exported to: {}", graph_file);
        println!("Visualize with: dot -Tpng {} -o graph.png", graph_file);
    }

    Ok(())
}

async fn handle_monitor_command(
    route_finder: &RouteFinder,
    arbitrage_detector: &ArbitrageDetector,
    tokens: Vec<String>,
    interval_ms: u64,
) -> Result<()> {
    info!("Starting real-time monitoring for {} tokens", tokens.len());
    
    let token_pubkeys: Result<Vec<Pubkey>, _> = tokens
        .iter()
        .map(|t| Pubkey::from_str(t))
        .collect();
    let token_pubkeys = token_pubkeys?;

    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(interval_ms));

    loop {
        interval.tick().await;
        
        // Monitor for arbitrage opportunities
        match arbitrage_detector.scan_arbitrage_opportunities(Decimal::from(100), 4).await {
            Ok(opportunities) => {
                if !opportunities.is_empty() {
                    println!("\n[{}] Found {} arbitrage opportunities", 
                        chrono::Utc::now().format("%H:%M:%S"),
                        opportunities.len()
                    );
                    
                    for opp in opportunities.iter().take(3) {
                        println!("  - {} | Profit: ${:.2} | ROI: {:.1}%", 
                            opp.id, opp.expected_profit_usd, opp.roi_percentage);
                    }
                }
            }
            Err(e) => {
                warn!("Arbitrage scan failed: {}", e);
            }
        }

        // Monitor route quality for token pairs
        for (i, from_token) in token_pubkeys.iter().enumerate() {
            for to_token in token_pubkeys.iter().skip(i + 1) {
                let route_request = RouteRequest {
                    from_token: *from_token,
                    to_token: *to_token,
                    amount: Decimal::from(1000), // 1000 units sample
                    max_hops: 3,
                    max_slippage: Decimal::from_str("0.02")?,
                    split_routes: false,
                };

                // Quick route check without full optimization
                if let Ok(routes) = route_finder.find_optimal_route(route_request).await {
                    if let Some(best_route) = routes.first() {
                        if best_route.price_impact > Decimal::from_str("0.05")? {
                            println!("  WARNING: High price impact {:.2}% for {}->{}", 
                                best_route.price_impact * Decimal::from(100),
                                from_token,
                                to_token
                            );
                        }
                    }
                }
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

fn format_route_path(path: &[RouteHop]) -> String {
    if path.is_empty() {
        return "No path".to_string();
    }

    let mut formatted = path[0].from_token.to_string();
    for hop in path {
        formatted.push_str(&format!(" -> {} ({})", 
            hop.to_token, 
            hop.pool_address
        ));
    }
    formatted
}

fn format_arbitrage_cycle(cycle: &[ArbitrageCycleHop]) -> String {
    if cycle.is_empty() {
        return "No cycle".to_string();
    }

    let mut formatted = cycle[0].token.to_string();
    for hop in cycle {
        formatted.push_str(&format!(" -> {} via {}", 
            hop.token, 
            hop.pool_address
        ));
    }
    // Complete the cycle
    if let Some(first) = cycle.first() {
        formatted.push_str(&format!(" -> {}", first.token));
    }
    formatted
}