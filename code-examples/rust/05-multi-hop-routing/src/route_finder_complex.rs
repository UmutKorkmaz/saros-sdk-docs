use anyhow::Result;
use moka::future::Cache;
use priority_queue::PriorityQueue;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use solana_sdk::pubkey::Pubkey;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::pool_graph::PoolGraph;
use crate::types::*;
use saros_dlmm_sdk::SarosClient;

/// Advanced route finding with A* algorithm and route optimization
pub struct RouteFinder {
    /// Pool connectivity graph
    pool_graph: Arc<PoolGraph>,
    
    /// Saros client for price and liquidity data
    client: Arc<SarosClient>,
    
    /// Route cache for frequently requested routes
    route_cache: RouteCache,
    
    /// Route optimization parameters
    optimization_params: RouteOptimizationParams,
    
    /// Performance metrics
    metrics: Arc<tokio::sync::RwLock<RouteMetrics>>,
}

impl RouteFinder {
    pub async fn new(pool_graph: Arc<PoolGraph>) -> Result<Self> {
        let client = Arc::new(SarosClient::new_mock()?);
        
        // Initialize cache with 30-second TTL
        let route_cache = Cache::builder()
            .time_to_live(tokio::time::Duration::from_secs(DEFAULT_CACHE_TTL))
            .max_capacity(1000)
            .build();
        
        let optimization_params = RouteOptimizationParams {
            weight_price_impact: 0.4,
            weight_gas_cost: 0.2,
            weight_liquidity_depth: 0.3,
            weight_execution_certainty: 0.1,
            max_split_routes: MAX_SPLIT_ROUTES,
            min_split_amount_usd: rust_decimal_macros::dec!(100),
        };
        
        let metrics = Arc::new(tokio::sync::RwLock::new(RouteMetrics {
            total_routes_found: 0,
            avg_route_length: 0.0,
            avg_price_impact: Decimal::ZERO,
            success_rate: Decimal::ONE,
            cache_hit_rate: 0.0,
            avg_computation_time_ms: 0,
        }));
        
        Ok(Self {
            pool_graph,
            client,
            route_cache,
            optimization_params,
            metrics,
        })
    }
    
    /// Find optimal route using A* algorithm with multiple optimizations
    pub async fn find_optimal_route(
        &self,
        request: RouteRequest,
    ) -> Result<Vec<RouteResponse>> {
        let start_time = std::time::Instant::now();
        
        // Check cache first
        let cache_key = self.generate_cache_key(&request);
        if let Some(cached_route) = self.route_cache.get(&cache_key).await {
            self.update_cache_hit_metrics().await;
            return Ok(vec![cached_route]);
        }
        
        info!("Finding optimal route: {} -> {} (amount: {})", 
            request.from_token, request.to_token, request.amount);
        
        // Validate request
        self.validate_route_request(&request)?;
        
        // Find multiple route options
        let mut route_candidates = self.find_route_candidates(&request).await?;
        
        // Optimize and rank routes
        self.optimize_routes(&mut route_candidates, &request).await?;
        
        // Apply route splitting if requested and beneficial
        if request.split_routes {
            route_candidates = self.apply_route_splitting(route_candidates, &request).await?;
        }
        
        // Select best routes (up to 3)
        route_candidates.truncate(3);
        
        // Cache the best route
        if let Some(best_route) = route_candidates.first() {
            self.route_cache.insert(cache_key, best_route.clone()).await;
        }
        
        // Update metrics
        let computation_time = start_time.elapsed().as_millis() as u64;
        self.update_route_metrics(&route_candidates, computation_time).await;
        
        info!("Found {} route options in {}ms", 
            route_candidates.len(), computation_time);
        
        Ok(route_candidates)
    }
    
    /// Find route candidates using multiple algorithms
    async fn find_route_candidates(
        &self,
        request: &RouteRequest,
    ) -> Result<Vec<RouteResponse>> {
        let mut candidates = Vec::new();
        
        // 1. A* algorithm for shortest weighted path
        if let Some(astar_route) = self.find_astar_route(request).await? {
            candidates.push(astar_route);
        }
        
        // 2. Dijkstra for true shortest path
        if let Some(dijkstra_route) = self.find_dijkstra_route(request).await? {
            candidates.push(dijkstra_route);
        }
        
        // 3. Alternative paths for redundancy
        let alternative_routes = self.find_alternative_routes(request, 3).await?;
        candidates.extend(alternative_routes);
        
        // 4. Liquidity-optimized path
        if let Some(liquidity_route) = self.find_liquidity_optimized_route(request).await? {
            candidates.push(liquidity_route);
        }
        
        // Remove duplicates and invalid routes
        candidates = self.deduplicate_routes(candidates).await?;
        
        Ok(candidates)
    }
    
    /// Find route using A* algorithm with custom heuristic
    async fn find_astar_route(
        &self,
        request: &RouteRequest,
    ) -> Result<Option<RouteResponse>> {
        debug!("Finding A* route");
        
        // Use a simpler approach - BFS with cost tracking
        use std::collections::VecDeque;
        
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut best_cost = HashMap::new();
        
        // Initialize with starting token
        queue.push_back((request.from_token, Vec::new(), request.amount, 0.0f64));
        best_cost.insert(request.from_token, 0.0);
        
        while let Some((current_token, path, current_amount, cost)) = queue.pop_front() {
            if current_token == request.to_token {
                // Found target, construct route response
                return Ok(Some(self.construct_route_response(
                    path, 
                    request.amount,
                    current_amount,
                    cost,
                ).await?));
            }
            
            if path.len() >= request.max_hops as usize {
                continue;
            }
            
            if visited.contains(&current_token) {
                continue;
            }
            visited.insert(current_token);
            
            // Get neighbors and evaluate routes
            let neighbors = self.pool_graph.get_neighbors(current_token).await?;
            
            for (next_token, edge) in neighbors {
                if visited.contains(&next_token) {
                    continue;
                }
                
                // Calculate swap amounts and costs
                let swap_result = self.calculate_swap_output(
                    current_token,
                    next_token,
                    current_amount,
                    &edge,
                ).await?;
                
                if swap_result.amount_out.is_zero() {
                    continue;
                }
                
                let new_cost = cost + swap_result.total_cost;
                let heuristic = self.calculate_heuristic(
                    next_token, 
                    request.to_token,
                    swap_result.amount_out,
                ).await?;
                let priority_cost = new_cost + heuristic;
                
                // Check if this path is better
                if let Some(&existing_cost) = best_cost.get(&next_token) {
                    if priority_cost >= existing_cost {
                        continue;
                    }
                }
                best_cost.insert(next_token, priority_cost);
                
                // Add to queue
                let mut new_path = path.clone();
                new_path.push(RouteHop {
                    from_token: current_token,
                    to_token: next_token,
                    pool_address: edge.pool,
                    expected_amount_in: current_amount,
                    expected_amount_out: swap_result.amount_out,
                    fee_tier: swap_result.fee_tier,
                    price_impact: swap_result.price_impact,
                });
                
                queue.push(
                    (next_token, new_path, swap_result.amount_out),
                    Reverse(priority_cost),
                );
            }
        }
        
        Ok(None)
    }
    
    /// Find route using Dijkstra's algorithm
    async fn find_dijkstra_route(
        &self,
        request: &RouteRequest,
    ) -> Result<Option<RouteResponse>> {
        debug!("Finding Dijkstra route");
        
        // Use pool graph's shortest path finding
        if let Some(node_path) = self.pool_graph.find_shortest_path(
            request.from_token,
            request.to_token,
        ).await? {
            return self.convert_node_path_to_route(node_path, request).await;
        }
        
        Ok(None)
    }
    
    /// Find alternative routes with different trade-offs
    async fn find_alternative_routes(
        &self,
        request: &RouteRequest,
        max_alternatives: usize,
    ) -> Result<Vec<RouteResponse>> {
        debug!("Finding {} alternative routes", max_alternatives);
        
        let node_paths = self.pool_graph.find_alternative_paths(
            request.from_token,
            request.to_token,
            max_alternatives,
            request.max_hops,
        ).await?;
        
        let mut routes = Vec::new();
        for path in node_paths {
            if let Some(route) = self.convert_node_path_to_route(path, request).await? {
                routes.push(route);
            }
        }
        
        Ok(routes)
    }
    
    /// Find liquidity-optimized route prioritizing pool liquidity
    async fn find_liquidity_optimized_route(
        &self,
        request: &RouteRequest,
    ) -> Result<Option<RouteResponse>> {
        debug!("Finding liquidity-optimized route");
        
        // Similar to A* but with liquidity-weighted costs
        let mut queue = PriorityQueue::new();
        let mut visited = HashSet::new();
        
        queue.push((request.from_token, Vec::new(), request.amount), Reverse(0.0));
        
        while let Some(((current_token, path, current_amount), Reverse(cost))) = queue.pop() {
            if current_token == request.to_token {
                return Ok(Some(self.construct_route_response(
                    path, 
                    request.amount,
                    current_amount,
                    cost,
                ).await?));
            }
            
            if path.len() >= request.max_hops as usize || visited.contains(&current_token) {
                continue;
            }
            visited.insert(current_token);
            
            let neighbors = self.pool_graph.get_neighbors(current_token).await?;
            
            for (next_token, edge) in neighbors {
                if visited.contains(&next_token) {
                    continue;
                }
                
                let pool_info = self.pool_graph.get_pool_info(edge.pool)
                    .ok_or_else(|| RoutingError::GraphConstructionFailed { 
                        reason: "Pool info not found".to_string() 
                    })?;
                
                // Liquidity-weighted cost (higher liquidity = lower cost)
                let liquidity_weight = 1.0 / (pool_info.liquidity_usd.to_f64().unwrap_or(1.0) + 1.0);
                let new_cost = cost + liquidity_weight;
                
                let swap_result = self.calculate_swap_output(
                    current_token,
                    next_token,
                    current_amount,
                    &edge,
                ).await?;
                
                if swap_result.amount_out.is_zero() {
                    continue;
                }
                
                let mut new_path = path.clone();
                new_path.push(RouteHop {
                    from_token: current_token,
                    to_token: next_token,
                    pool_address: edge.pool,
                    expected_amount_in: current_amount,
                    expected_amount_out: swap_result.amount_out,
                    fee_tier: swap_result.fee_tier,
                    price_impact: swap_result.price_impact,
                });
                
                queue.push_back((next_token, new_path, swap_result.amount_out, new_cost));
            }
        }
        
        Ok(None)
    }
    
    /// Optimize routes using multiple criteria
    async fn optimize_routes(
        &self,
        routes: &mut Vec<RouteResponse>,
        request: &RouteRequest,
    ) -> Result<()> {
        debug!("Optimizing {} route candidates", routes.len());
        
        for route in routes.iter_mut() {
            // Calculate comprehensive scoring
            route.confidence_score = self.calculate_route_confidence(route).await?;
            
            // Estimate gas costs
            route.gas_estimate = self.estimate_gas_cost(&route.path).await?;
            
            // Validate price impact
            if route.price_impact > request.max_slippage {
                route.confidence_score *= 0.5; // Penalize high slippage
            }
        }
        
        // Sort by confidence score (highest first)
        routes.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score)
            .unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(())
    }
    
    /// Apply route splitting for large orders
    async fn apply_route_splitting(
        &self,
        routes: Vec<RouteResponse>,
        request: &RouteRequest,
    ) -> Result<Vec<RouteResponse>> {
        if routes.is_empty() {
            return Ok(routes);
        }
        
        let amount_usd = self.estimate_amount_usd(request.from_token, request.amount).await?;
        
        // Only split if amount is significant and we have multiple routes
        if amount_usd < self.optimization_params.min_split_amount_usd || routes.len() < 2 {
            return Ok(routes);
        }
        
        debug!("Applying route splitting for ${:.2} order", amount_usd);
        
        // Take best route and enhance with splits
        let mut best_route = routes.into_iter().next().unwrap();
        
        // Calculate optimal split percentages
        let split_percentages = self.calculate_optimal_splits(
            request.amount, 
            &best_route,
        ).await?;
        
        // Create split routes
        let mut split_routes = Vec::new();
        for (i, &percentage) in split_percentages.iter().enumerate() {
            let split_amount = request.amount * percentage;
            let split_request = RouteRequest {
                amount: split_amount,
                split_routes: false, // Avoid recursive splitting
                ..request.clone()
            };
            
            if let Ok(split_route_results) = self.find_route_candidates(&split_request).await {
                if let Some(split_route) = split_route_results.first() {
                    split_routes.push(SplitRoute {
                        route_id: format!("{}-split-{}", best_route.route_id, i),
                        percentage,
                        amount: split_amount,
                        expected_output: split_route.expected_output,
                        path: split_route.path.clone(),
                    });
                }
            }
        }
        
        best_route.split_routes = split_routes;
        Ok(vec![best_route])
    }
    
    // Helper methods
    
    fn validate_route_request(&self, request: &RouteRequest) -> Result<()> {
        if request.from_token == request.to_token {
            return Err(RoutingError::InvalidTokenPair {
                from: request.from_token,
                to: request.to_token,
            }.into());
        }
        
        if request.amount.is_zero() {
            return Err(RoutingError::CalculationError {
                message: "Amount must be greater than zero".to_string(),
            }.into());
        }
        
        if request.max_hops > MAX_ROUTE_HOPS {
            return Err(RoutingError::CalculationError {
                message: format!("Max hops {} exceeds limit {}", request.max_hops, MAX_ROUTE_HOPS),
            }.into());
        }
        
        Ok(())
    }
    
    fn generate_cache_key(&self, request: &RouteRequest) -> String {
        format!("{}-{}-{}-{}", 
            request.from_token,
            request.to_token,
            request.amount,
            request.max_hops
        )
    }
    
    async fn calculate_swap_output(
        &self,
        from_token: Pubkey,
        to_token: Pubkey,
        amount_in: Decimal,
        edge: &GraphEdge,
    ) -> Result<SwapResult> {
        let pool_info = self.pool_graph.get_pool_info(edge.pool)
            .ok_or_else(|| RoutingError::GraphConstructionFailed { 
                reason: "Pool info not found".to_string() 
            })?;
        
        // Mock swap calculation - in real implementation, use proper DLMM math
        let fee_tier = pool_info.fee_tier;
        let fee_amount = amount_in * fee_tier;
        let amount_after_fee = amount_in - fee_amount;
        
        // Simplified price impact calculation
        let liquidity = pool_info.liquidity_usd;
        let amount_usd = self.estimate_amount_usd(from_token, amount_in).await?;
        let price_impact = (amount_usd / liquidity) * rust_decimal_macros::dec!(0.5);
        
        let amount_out = amount_after_fee * (Decimal::ONE - price_impact);
        
        let total_cost = edge.weight + price_impact.to_f64().unwrap_or(0.0);
        
        Ok(SwapResult {
            amount_out,
            fee_tier,
            price_impact,
            total_cost,
        })
    }
    
    async fn calculate_heuristic(
        &self,
        from_token: Pubkey,
        to_token: Pubkey,
        amount: Decimal,
    ) -> Result<f64> {
        // Estimate remaining cost to reach target
        // This is a simplified heuristic - in practice, could use price ratios
        
        if from_token == to_token {
            return Ok(0.0);
        }
        
        // Use token connectivity as heuristic
        if let Ok(analysis) = self.pool_graph.analyze_token_connectivity(from_token).await {
            let connectivity_factor = 1.0 / (analysis.direct_pairs as f64 + 1.0);
            Ok(connectivity_factor)
        } else {
            Ok(1.0) // Default heuristic
        }
    }
    
    async fn construct_route_response(
        &self,
        path: Vec<RouteHop>,
        initial_amount: Decimal,
        final_amount: Decimal,
        total_cost: f64,
    ) -> Result<RouteResponse> {
        let route_id = Uuid::new_v4().to_string();
        
        let total_price_impact = path.iter()
            .map(|hop| hop.price_impact)
            .sum();
        
        let gas_estimate = self.estimate_gas_cost(&path).await?;
        
        let confidence_score = self.calculate_path_confidence(&path, total_cost).await?;
        
        let execution_time_estimate = (path.len() as u64) * 2000; // 2s per hop estimate
        
        Ok(RouteResponse {
            route_id,
            path,
            expected_output: final_amount,
            price_impact: total_price_impact,
            gas_estimate,
            confidence_score,
            split_routes: Vec::new(),
            execution_time_estimate,
        })
    }
    
    async fn convert_node_path_to_route(
        &self,
        node_path: Vec<NodeIndex>,
        request: &RouteRequest,
    ) -> Result<Option<RouteResponse>> {
        if node_path.len() < 2 {
            return Ok(None);
        }
        
        let mut route_hops = Vec::new();
        let mut current_amount = request.amount;
        
        for window in node_path.windows(2) {
            let from_node = window[0];
            let to_node = window[1];
            
            // Get token addresses
            let from_token = self.pool_graph.get_token_from_node(from_node)
                .ok_or_else(|| RoutingError::GraphConstructionFailed {
                    reason: "Node to token mapping not found".to_string()
                })?;
            let to_token = self.pool_graph.get_token_from_node(to_node)
                .ok_or_else(|| RoutingError::GraphConstructionFailed {
                    reason: "Node to token mapping not found".to_string()
                })?;
            
            // Find edge between nodes
            let neighbors = self.pool_graph.get_neighbors(*from_token).await?;
            let edge = neighbors.iter()
                .find(|(token, _)| token == &to_token)
                .map(|(_, edge)| edge)
                .ok_or_else(|| RoutingError::GraphConstructionFailed {
                    reason: "Edge not found between nodes".to_string()
                })?;
            
            // Calculate swap
            let swap_result = self.calculate_swap_output(
                *from_token,
                *to_token,
                current_amount,
                edge,
            ).await?;
            
            route_hops.push(RouteHop {
                from_token: *from_token,
                to_token: *to_token,
                pool_address: edge.pool,
                expected_amount_in: current_amount,
                expected_amount_out: swap_result.amount_out,
                fee_tier: swap_result.fee_tier,
                price_impact: swap_result.price_impact,
            });
            
            current_amount = swap_result.amount_out;
        }
        
        let total_cost = route_hops.iter()
            .map(|hop| hop.price_impact.to_f64().unwrap_or(0.0))
            .sum();
        
        let route = self.construct_route_response(
            route_hops,
            request.amount,
            current_amount,
            total_cost,
        ).await?;
        
        Ok(Some(route))
    }
    
    async fn deduplicate_routes(
        &self,
        mut routes: Vec<RouteResponse>,
    ) -> Result<Vec<RouteResponse>> {
        // Remove routes with identical paths
        routes.sort_by(|a, b| {
            let path_a: Vec<_> = a.path.iter().map(|h| (h.from_token, h.to_token)).collect();
            let path_b: Vec<_> = b.path.iter().map(|h| (h.from_token, h.to_token)).collect();
            path_a.cmp(&path_b)
        });
        
        routes.dedup_by(|a, b| {
            let path_a: Vec<_> = a.path.iter().map(|h| (h.from_token, h.to_token)).collect();
            let path_b: Vec<_> = b.path.iter().map(|h| (h.from_token, h.to_token)).collect();
            path_a == path_b
        });
        
        Ok(routes)
    }
    
    async fn calculate_route_confidence(&self, route: &RouteResponse) -> Result<f64> {
        let mut confidence = 10.0;
        
        // Penalize long routes
        confidence -= (route.path.len() as f64 - 1.0) * 1.0;
        
        // Penalize high price impact
        let price_impact_penalty = route.price_impact.to_f64().unwrap_or(0.0) * 20.0;
        confidence -= price_impact_penalty;
        
        // Bonus for high liquidity pools
        for hop in &route.path {
            if let Some(pool) = self.pool_graph.get_pool_info(hop.pool_address) {
                let liquidity_bonus = (pool.liquidity_usd.to_f64().unwrap_or(0.0) / 100000.0).min(2.0);
                confidence += liquidity_bonus;
            }
        }
        
        Ok(confidence.max(0.0).min(10.0))
    }
    
    async fn calculate_path_confidence(&self, path: &[RouteHop], _cost: f64) -> Result<f64> {
        let mut confidence = 8.0;
        
        // Path length penalty
        confidence -= (path.len() as f64 - 1.0) * 0.5;
        
        // Liquidity analysis
        for hop in path {
            if let Some(pool) = self.pool_graph.get_pool_info(hop.pool_address) {
                if pool.liquidity_usd < rust_decimal_macros::dec!(10000) {
                    confidence -= 1.0;
                }
            }
        }
        
        Ok(confidence.max(0.0).min(10.0))
    }
    
    async fn estimate_gas_cost(&self, path: &[RouteHop]) -> Result<Decimal> {
        let base_gas = 20000; // Base transaction cost
        let per_hop_gas = 50000; // Cost per swap
        
        let total_gas = base_gas + (path.len() as u64 * per_hop_gas);
        let gas_price = rust_decimal_macros::dec!(0.000005); // SOL per compute unit
        
        Ok(Decimal::from(total_gas) * gas_price)
    }
    
    async fn estimate_amount_usd(&self, token: Pubkey, amount: Decimal) -> Result<Decimal> {
        if let Some(token_info) = self.pool_graph.get_token_info(token) {
            Ok(amount * token_info.price_usd)
        } else {
            Ok(amount) // Assume $1 if price unknown
        }
    }
    
    async fn calculate_optimal_splits(&self, total_amount: Decimal, _route: &RouteResponse) -> Result<Vec<Decimal>> {
        // Simplified split calculation - could be more sophisticated
        Ok(vec![
            rust_decimal_macros::dec!(0.5),  // 50%
            rust_decimal_macros::dec!(0.3),  // 30%
            rust_decimal_macros::dec!(0.2),  // 20%
        ])
    }
    
    async fn update_cache_hit_metrics(&self) {
        // Update cache hit rate metric
        let mut metrics = self.metrics.write().await;
        metrics.cache_hit_rate = (metrics.cache_hit_rate * 0.9) + 0.1;
    }
    
    async fn update_route_metrics(
        &self,
        routes: &[RouteResponse],
        computation_time: u64,
    ) {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_routes_found += routes.len() as u64;
        
        if !routes.is_empty() {
            let avg_length: f64 = routes.iter()
                .map(|r| r.path.len() as f64)
                .sum::<f64>() / routes.len() as f64;
            
            let avg_impact: Decimal = routes.iter()
                .map(|r| r.price_impact)
                .sum::<Decimal>() / Decimal::from(routes.len());
            
            metrics.avg_route_length = (metrics.avg_route_length * 0.9) + (avg_length * 0.1);
            metrics.avg_price_impact = (metrics.avg_price_impact * rust_decimal_macros::dec!(0.9)) + 
                (avg_impact * rust_decimal_macros::dec!(0.1));
        }
        
        metrics.avg_computation_time_ms = (metrics.avg_computation_time_ms * 9 + computation_time) / 10;
    }
}

#[derive(Debug)]
struct SwapResult {
    amount_out: Decimal,
    fee_tier: Decimal,
    price_impact: Decimal,
    total_cost: f64,
}