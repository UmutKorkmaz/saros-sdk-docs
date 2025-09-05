use anyhow::Result;
use dashmap::DashMap;
use moka::future::Cache;
use petgraph::visit::{Dfs, EdgeRef};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::pool_graph::PoolGraph;
use crate::types::*;
use saros_dlmm_sdk::SarosClient;

/// Advanced arbitrage detection using graph cycle detection algorithms
pub struct ArbitrageDetector {
    /// Pool connectivity graph
    pool_graph: Arc<PoolGraph>,
    
    /// Saros client for price data
    client: Arc<SarosClient>,
    
    /// Cache for arbitrage opportunities
    arbitrage_cache: ArbitrageCache,
    
    /// Known profitable cycles (for quick re-checking)
    known_cycles: Arc<DashMap<String, Vec<ArbitrageCycleHop>>>,
    
    /// Price monitor for detecting arbitrage triggers
    price_monitor: Arc<tokio::sync::RwLock<HashMap<Pubkey, Decimal>>>,
    
    /// Detection metrics
    metrics: Arc<tokio::sync::RwLock<ArbitrageMetrics>>,
}

#[derive(Debug, Default)]
struct ArbitrageMetrics {
    cycles_detected: u64,
    profitable_opportunities: u64,
    avg_profit_usd: Decimal,
    detection_time_ms: u64,
    false_positives: u64,
}

impl ArbitrageDetector {
    pub async fn new(pool_graph: Arc<PoolGraph>) -> Result<Self> {
        let client = Arc::new(SarosClient::new_mock()?);
        
        // Initialize cache with 60-second TTL for arbitrage opportunities
        let arbitrage_cache = Cache::builder()
            .time_to_live(tokio::time::Duration::from_secs(60))
            .max_capacity(100)
            .build();
        
        let detector = Self {
            pool_graph,
            client,
            arbitrage_cache,
            known_cycles: Arc::new(DashMap::new()),
            price_monitor: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            metrics: Arc::new(tokio::sync::RwLock::new(ArbitrageMetrics::default())),
        };
        
        // Start background price monitoring
        let detector_clone = detector.clone();
        tokio::spawn(async move {
            detector_clone.background_price_monitoring().await;
        });
        
        Ok(detector)
    }
    
    /// Scan for arbitrage opportunities using cycle detection
    pub async fn scan_arbitrage_opportunities(
        &self,
        min_profit_usd: Decimal,
        max_cycle_length: u8,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        let start_time = std::time::Instant::now();
        
        info!("Scanning for arbitrage opportunities (min profit: ${}, max cycle: {})",
            min_profit_usd, max_cycle_length);
        
        // Check cache first
        let cache_key = format!("arb-{}-{}", min_profit_usd, max_cycle_length);
        if let Some(cached_opportunities) = self.arbitrage_cache.get(&cache_key).await {
            return Ok(cached_opportunities);
        }
        
        let mut opportunities = Vec::new();
        
        // 1. Detect cycles using modified Floyd-Warshall for negative cycles
        let cycles = self.detect_profitable_cycles(max_cycle_length).await?;
        debug!("Found {} potential cycles", cycles.len());
        
        // 2. Analyze each cycle for profitability
        for cycle in cycles {
            if let Some(opportunity) = self.analyze_cycle_profitability(
                cycle,
                min_profit_usd,
            ).await? {
                opportunities.push(opportunity);
            }
        }
        
        // 3. Re-check known profitable cycles with current prices
        let known_cycle_opportunities = self.check_known_cycles(min_profit_usd).await?;
        opportunities.extend(known_cycle_opportunities);
        
        // 4. Filter and rank opportunities
        opportunities = self.filter_and_rank_opportunities(opportunities, min_profit_usd).await?;
        
        // Cache results
        self.arbitrage_cache.insert(cache_key, opportunities.clone()).await;
        
        // Update metrics
        let detection_time = start_time.elapsed().as_millis() as u64;
        self.update_detection_metrics(&opportunities, detection_time).await;
        
        info!("Found {} arbitrage opportunities in {}ms", 
            opportunities.len(), detection_time);
        
        Ok(opportunities)
    }
    
    /// Detect profitable cycles using enhanced DFS with negative weight detection
    async fn detect_profitable_cycles(
        &self,
        max_length: u8,
    ) -> Result<Vec<Vec<ArbitrageCycleHop>>> {
        debug!("Detecting cycles with max length {}", max_length);
        
        let graph = self.pool_graph.graph.read().await;
        let mut cycles = Vec::new();
        let mut visited_global = HashSet::new();
        
        // Iterate through all nodes to find cycles starting from each
        for start_node in graph.node_indices() {
            if visited_global.contains(&start_node) {
                continue;
            }
            
            let start_token = self.pool_graph.get_token_from_node(start_node)
                .ok_or_else(|| RoutingError::GraphConstructionFailed {
                    reason: "Node to token mapping not found".to_string()
                })?;
            
            // Find cycles starting from this token
            let node_cycles = self.find_cycles_from_node(
                &graph,
                start_node,
                *start_token,
                max_length,
            ).await?;
            
            cycles.extend(node_cycles);
            visited_global.insert(start_node);
            
            // Limit total cycles to prevent excessive computation
            if cycles.len() > 1000 {
                break;
            }
        }
        
        debug!("Detected {} raw cycles", cycles.len());
        
        // Deduplicate cycles
        cycles = self.deduplicate_cycles(cycles).await?;
        
        Ok(cycles)
    }
    
    /// Find cycles starting from a specific node using DFS
    async fn find_cycles_from_node(
        &self,
        graph: &TokenGraph,
        start_node: NodeIndex,
        start_token: Pubkey,
        max_length: u8,
    ) -> Result<Vec<Vec<ArbitrageCycleHop>>> {
        let mut cycles = Vec::new();
        let mut stack = VecDeque::new();
        
        // Initialize DFS with starting node
        stack.push_back((start_node, vec![], HashSet::new(), Decimal::ONE));
        
        while let Some((current_node, path, visited, current_amount)) = stack.pop_back() {
            if path.len() >= max_length as usize {
                continue;
            }
            
            // Check if we've completed a cycle back to start
            if path.len() > 2 && current_node == start_node {
                if self.is_potentially_profitable_cycle(&path, current_amount).await? {
                    cycles.push(path);
                    continue;
                }
            }
            
            // Explore neighbors
            for edge in graph.edges(current_node) {
                let next_node = edge.target();
                let edge_data = edge.weight();
                
                // Skip if already visited (except for returning to start)
                if visited.contains(&next_node) && next_node != start_node {
                    continue;
                }
                
                // Skip if this would be too early to return to start
                if next_node == start_node && path.len() < 2 {
                    continue;
                }
                
                let next_token = self.pool_graph.node_to_token.get(&next_node)
                    .ok_or_else(|| RoutingError::GraphConstructionFailed {
                        reason: "Node to token mapping not found".to_string()
                    })?;
                
                // Calculate expected output for this hop
                let hop_result = self.calculate_arbitrage_hop_output(
                    path.last().map(|h| h.token).unwrap_or(start_token),
                    *next_token,
                    current_amount,
                    edge_data.pool,
                ).await?;
                
                if hop_result.expected_amount_out.is_zero() {
                    continue;
                }
                
                let mut new_path = path.clone();
                new_path.push(ArbitrageCycleHop {
                    token: *next_token,
                    pool_address: edge_data.pool,
                    expected_amount_in: current_amount,
                    expected_amount_out: hop_result.expected_amount_out,
                    price_impact: hop_result.price_impact,
                });
                
                let mut new_visited = visited.clone();
                new_visited.insert(current_node);
                
                stack.push_back((
                    next_node,
                    new_path,
                    new_visited,
                    hop_result.expected_amount_out,
                ));
            }
        }
        
        Ok(cycles)
    }
    
    /// Analyze cycle profitability with detailed calculations
    async fn analyze_cycle_profitability(
        &self,
        cycle: Vec<ArbitrageCycleHop>,
        min_profit_usd: Decimal,
    ) -> Result<Option<ArbitrageOpportunity>> {
        if cycle.is_empty() {
            return Ok(None);
        }
        
        // Calculate total expected profit
        let initial_amount = Decimal::ONE; // Normalize to 1 unit
        let final_amount = cycle.last()
            .map(|hop| hop.expected_amount_out)
            .unwrap_or_default();
        
        let profit_ratio = final_amount - initial_amount;
        
        if profit_ratio <= Decimal::ZERO {
            return Ok(None);
        }
        
        // Estimate required capital and actual profit in USD
        let start_token = cycle.first()
            .map(|hop| hop.token)
            .ok_or_else(|| RoutingError::CalculationError {
                message: "Empty cycle".to_string()
            })?;
        
        let (required_capital_usd, expected_profit_usd) = 
            self.calculate_optimal_arbitrage_size(start_token, profit_ratio).await?;
        
        if expected_profit_usd < min_profit_usd {
            return Ok(None);
        }
        
        // Calculate risk metrics
        let risk_score = self.calculate_arbitrage_risk(&cycle).await?;
        let confidence = self.calculate_arbitrage_confidence(&cycle).await?;
        
        // Calculate ROI
        let roi_percentage = if required_capital_usd > Decimal::ZERO {
            (expected_profit_usd / required_capital_usd) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };
        
        let opportunity = ArbitrageOpportunity {
            id: Uuid::new_v4().to_string(),
            cycle,
            expected_profit_usd,
            roi_percentage,
            risk_score,
            confidence,
            required_capital_usd,
            execution_complexity: self.calculate_execution_complexity(&cycle).await?,
            time_sensitive: true,
        };
        
        // Store in known cycles for future reference
        self.known_cycles.insert(opportunity.id.clone(), opportunity.cycle.clone());
        
        Ok(Some(opportunity))
    }
    
    /// Check known profitable cycles with current market conditions
    async fn check_known_cycles(
        &self,
        min_profit_usd: Decimal,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        for entry in self.known_cycles.iter() {
            let cycle_id = entry.key().clone();
            let cycle = entry.value().clone();
            
            // Re-calculate profitability with current prices
            if let Some(updated_opportunity) = self.analyze_cycle_profitability(
                cycle,
                min_profit_usd,
            ).await? {
                // Preserve the original cycle ID
                let mut opp = updated_opportunity;
                opp.id = cycle_id;
                opportunities.push(opp);
            }
        }
        
        Ok(opportunities)
    }
    
    /// Filter and rank opportunities by profitability and safety
    async fn filter_and_rank_opportunities(
        &self,
        mut opportunities: Vec<ArbitrageOpportunity>,
        min_profit_usd: Decimal,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        // Filter by minimum profit
        opportunities.retain(|opp| opp.expected_profit_usd >= min_profit_usd);
        
        // Remove opportunities with excessive risk
        opportunities.retain(|opp| opp.risk_score <= 8.0);
        
        // Remove opportunities with very low confidence
        opportunities.retain(|opp| opp.confidence >= rust_decimal_macros::dec!(0.3));
        
        // Sort by execution priority (combination of profit, risk, and confidence)
        opportunities.sort_by(|a, b| {
            let score_a = a.execution_priority();
            let score_b = b.execution_priority();
            score_b.cmp(&score_a)
        });
        
        // Limit to top opportunities
        opportunities.truncate(10);
        
        Ok(opportunities)
    }
    
    // Helper methods
    
    async fn is_potentially_profitable_cycle(
        &self,
        cycle: &[ArbitrageCycleHop],
        final_amount: Decimal,
    ) -> Result<bool> {
        // Quick profitability check without detailed calculations
        let initial_amount = Decimal::ONE;
        let profit_threshold = rust_decimal_macros::dec!(0.005); // 0.5% minimum profit
        
        Ok(final_amount > initial_amount + profit_threshold)
    }
    
    async fn calculate_arbitrage_hop_output(
        &self,
        from_token: Pubkey,
        to_token: Pubkey,
        amount_in: Decimal,
        pool_address: Pubkey,
    ) -> Result<ArbitrageHopResult> {
        let pool_info = self.pool_graph.get_pool_info(pool_address)
            .ok_or_else(|| RoutingError::GraphConstructionFailed {
                reason: "Pool info not found".to_string()
            })?;
        
        // Get current prices
        let price_monitor = self.price_monitor.read().await;
        let from_price = price_monitor.get(&from_token).unwrap_or(&Decimal::ONE);
        let to_price = price_monitor.get(&to_token).unwrap_or(&Decimal::ONE);
        
        // Mock arbitrage calculation - in real implementation, use precise DLMM math
        let exchange_rate = from_price / to_price;
        let fee = pool_info.fee_tier;
        
        // Calculate price impact based on trade size vs liquidity
        let trade_size_usd = amount_in * from_price;
        let price_impact = (trade_size_usd / pool_info.liquidity_usd) 
            * rust_decimal_macros::dec!(0.3); // Impact factor
        
        let amount_after_fee = amount_in * (Decimal::ONE - fee);
        let amount_out = amount_after_fee * exchange_rate * (Decimal::ONE - price_impact);
        
        Ok(ArbitrageHopResult {
            expected_amount_out: amount_out,
            price_impact,
            exchange_rate,
        })
    }
    
    async fn calculate_optimal_arbitrage_size(
        &self,
        token: Pubkey,
        profit_ratio: Decimal,
    ) -> Result<(Decimal, Decimal)> {
        // Estimate optimal trade size based on available liquidity
        let token_info = self.pool_graph.get_token_info(token);
        let base_capital = rust_decimal_macros::dec!(1000); // $1000 base
        
        let capital_multiplier = if let Some(info) = token_info {
            // Scale based on token liquidity and volatility
            let liquidity_factor = (info.market_cap / rust_decimal_macros::dec!(1000000))
                .max(rust_decimal_macros::dec!(0.1))
                .min(rust_decimal_macros::dec!(10.0));
            liquidity_factor
        } else {
            Decimal::ONE
        };
        
        let required_capital_usd = base_capital * capital_multiplier;
        let expected_profit_usd = required_capital_usd * profit_ratio * rust_decimal_macros::dec!(0.8); // 80% efficiency
        
        Ok((required_capital_usd, expected_profit_usd))
    }
    
    async fn calculate_arbitrage_risk(&self, cycle: &[ArbitrageCycleHop]) -> Result<f64> {
        let mut risk_score = 0.0;
        
        // Risk factors
        risk_score += cycle.len() as f64 * 0.5; // Length risk
        
        for hop in cycle {
            // Price impact risk
            let price_impact_risk = hop.price_impact.to_f64().unwrap_or(0.0) * 10.0;
            risk_score += price_impact_risk;
            
            // Liquidity risk
            if let Some(pool) = self.pool_graph.get_pool_info(hop.pool_address) {
                if pool.liquidity_usd < rust_decimal_macros::dec!(10000) {
                    risk_score += 2.0;
                }
                
                // Volume risk
                if pool.volume_24h < rust_decimal_macros::dec!(1000) {
                    risk_score += 1.5;
                }
            } else {
                risk_score += 3.0; // Missing pool data is risky
            }
        }
        
        Ok(risk_score.min(10.0))
    }
    
    async fn calculate_arbitrage_confidence(&self, cycle: &[ArbitrageCycleHop]) -> Result<Decimal> {
        let mut confidence = rust_decimal_macros::dec!(0.8);
        
        for hop in cycle {
            // Reduce confidence for high price impact
            let impact_penalty = hop.price_impact * rust_decimal_macros::dec!(2.0);
            confidence -= impact_penalty;
            
            // Reduce confidence for low liquidity pools
            if let Some(pool) = self.pool_graph.get_pool_info(hop.pool_address) {
                if pool.liquidity_usd < rust_decimal_macros::dec!(50000) {
                    confidence -= rust_decimal_macros::dec!(0.1);
                }
            }
        }
        
        Ok(confidence.max(rust_decimal_macros::dec!(0.0)).min(rust_decimal_macros::dec!(1.0)))
    }
    
    async fn calculate_execution_complexity(&self, cycle: &[ArbitrageCycleHop]) -> Result<u8> {
        let mut complexity = cycle.len() as u8;
        
        // Add complexity for cross-DEX arbitrage (if pools are on different platforms)
        // This is simplified - in practice, you'd check pool protocols
        
        // Add complexity for tokens with low decimals or special characteristics
        for hop in cycle {
            if let Some(token_info) = self.pool_graph.get_token_info(hop.token) {
                if token_info.decimals < 6 {
                    complexity += 1;
                }
            }
        }
        
        Ok(complexity.min(10))
    }
    
    async fn deduplicate_cycles(
        &self,
        mut cycles: Vec<Vec<ArbitrageCycleHop>>,
    ) -> Result<Vec<Vec<ArbitrageCycleHop>>> {
        // Sort cycles for easier deduplication
        cycles.sort_by(|a, b| {
            let path_a: Vec<_> = a.iter().map(|h| h.token).collect();
            let path_b: Vec<_> = b.iter().map(|h| h.token).collect();
            path_a.cmp(&path_b)
        });
        
        // Remove duplicates
        cycles.dedup_by(|a, b| {
            let path_a: Vec<_> = a.iter().map(|h| h.token).collect();
            let path_b: Vec<_> = b.iter().map(|h| h.token).collect();
            
            // Consider cycles equivalent if they visit the same tokens in the same order
            // (regardless of starting point in the cycle)
            self.cycles_equivalent(&path_a, &path_b)
        });
        
        Ok(cycles)
    }
    
    fn cycles_equivalent(&self, cycle_a: &[Pubkey], cycle_b: &[Pubkey]) -> bool {
        if cycle_a.len() != cycle_b.len() {
            return false;
        }
        
        // Check if cycle_b is a rotation of cycle_a
        for i in 0..cycle_a.len() {
            let rotated: Vec<_> = cycle_b.iter()
                .cycle()
                .skip(i)
                .take(cycle_b.len())
                .collect();
            
            if cycle_a.iter().collect::<Vec<_>>() == rotated {
                return true;
            }
        }
        
        false
    }
    
    async fn background_price_monitoring(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            match self.update_token_prices().await {
                Ok(_) => debug!("Token prices updated"),
                Err(e) => warn!("Failed to update token prices: {}", e),
            }
        }
    }
    
    async fn update_token_prices(&self) -> Result<()> {
        // Update price monitor with current token prices
        let tokens = self.client.get_all_tokens().await?;
        let mut price_monitor = self.price_monitor.write().await;
        
        for token in tokens {
            if let Some(price) = token.price_usd {
                price_monitor.insert(token.mint, price);
            }
        }
        
        Ok(())
    }
    
    async fn update_detection_metrics(
        &self,
        opportunities: &[ArbitrageOpportunity],
        detection_time: u64,
    ) {
        let mut metrics = self.metrics.write().await;
        
        metrics.cycles_detected += opportunities.len() as u64;
        metrics.detection_time_ms = (metrics.detection_time_ms * 9 + detection_time) / 10;
        
        if !opportunities.is_empty() {
            metrics.profitable_opportunities += opportunities.len() as u64;
            
            let avg_profit: Decimal = opportunities.iter()
                .map(|opp| opp.expected_profit_usd)
                .sum::<Decimal>() / Decimal::from(opportunities.len());
            
            metrics.avg_profit_usd = (metrics.avg_profit_usd * rust_decimal_macros::dec!(0.9)) + 
                (avg_profit * rust_decimal_macros::dec!(0.1));
        }
    }
}

impl Clone for ArbitrageDetector {
    fn clone(&self) -> Self {
        Self {
            pool_graph: self.pool_graph.clone(),
            client: self.client.clone(),
            arbitrage_cache: self.arbitrage_cache.clone(),
            known_cycles: self.known_cycles.clone(),
            price_monitor: self.price_monitor.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

#[derive(Debug)]
struct ArbitrageHopResult {
    expected_amount_out: Decimal,
    price_impact: Decimal,
    exchange_rate: Decimal,
}