use anyhow::Result;
use moka::future::Cache;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

use crate::pool_graph::PoolGraph;
use crate::types::*;
use saros_dlmm_sdk::SarosClient;

/// Simplified route finding with basic pathfinding algorithms
pub struct RouteFinder {
    /// Pool connectivity graph
    pool_graph: Arc<PoolGraph>,
    
    /// Saros client for price and liquidity data
    client: Arc<SarosClient>,
    
    /// Route cache for frequently requested routes
    route_cache: RouteCache,
    
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
            metrics,
        })
    }
    
    /// Find optimal route using simplified BFS approach
    pub async fn find_optimal_route(
        &self,
        request: RouteRequest,
    ) -> Result<Vec<RouteResponse>> {
        let start_time = std::time::Instant::now();
        
        // Check cache first
        let cache_key = self.generate_cache_key(&request);
        if let Some(cached_route) = self.route_cache.get(&cache_key).await {
            return Ok(vec![cached_route]);
        }
        
        info!("Finding optimal route: {} -> {} (amount: {})", 
            request.from_token, request.to_token, request.amount);
        
        // Use simple BFS to find path
        let path = self.find_simple_path(&request).await?;
        
        if let Some(node_path) = path {
            let route_response = self.construct_route_response(
                node_path,
                &request,
            ).await?;
            
            // Cache the result
            self.route_cache.insert(cache_key, route_response.clone()).await;
            
            // Update metrics
            let computation_time = start_time.elapsed().as_millis() as u64;
            self.update_metrics(computation_time).await;
            
            Ok(vec![route_response])
        } else {
            Err(RoutingError::NoRouteFound.into())
        }
    }
    
    /// Find a simple path using BFS
    async fn find_simple_path(&self, request: &RouteRequest) -> Result<Option<Vec<Pubkey>>> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<Pubkey, Pubkey> = HashMap::new();
        
        queue.push_back(request.from_token);
        visited.insert(request.from_token);
        
        while let Some(current_token) = queue.pop_front() {
            if current_token == request.to_token {
                // Reconstruct path
                let mut path = Vec::new();
                let mut token = current_token;
                
                while token != request.from_token {
                    path.push(token);
                    token = parent[&token];
                }
                path.push(request.from_token);
                path.reverse();
                
                return Ok(Some(path));
            }
            
            // Get neighbors
            let neighbors = self.pool_graph.get_neighbors(current_token).await?;
            
            for (next_token, _edge) in neighbors {
                if !visited.contains(&next_token) {
                    visited.insert(next_token);
                    parent.insert(next_token, current_token);
                    queue.push_back(next_token);
                }
            }
        }
        
        Ok(None)
    }
    
    /// Construct route response from token path
    async fn construct_route_response(
        &self,
        token_path: Vec<Pubkey>,
        request: &RouteRequest,
    ) -> Result<RouteResponse> {
        let mut route_hops = Vec::new();
        let mut current_amount = request.amount;
        
        for window in token_path.windows(2) {
            let from_token = window[0];
            let to_token = window[1];
            
            // Mock swap calculation
            let amount_out = current_amount * rust_decimal_macros::dec!(0.997); // 0.3% fee
            let price_impact = rust_decimal_macros::dec!(0.002); // 0.2% impact
            
            route_hops.push(RouteHop {
                from_token,
                to_token,
                pool_address: Pubkey::new_unique(), // Mock pool address
                expected_amount_in: current_amount,
                expected_amount_out: amount_out,
                fee_tier: rust_decimal_macros::dec!(0.003),
                price_impact,
            });
            
            current_amount = amount_out;
        }
        
        let total_price_impact: Decimal = route_hops.iter()
            .map(|hop| hop.price_impact)
            .sum();
        
        Ok(RouteResponse {
            route_id: Uuid::new_v4().to_string(),
            path: route_hops,
            expected_output: current_amount,
            price_impact: total_price_impact,
            gas_estimate: rust_decimal_macros::dec!(0.001), // 0.001 SOL
            confidence_score: 0.85, // 85% confidence
            split_routes: Vec::new(), // No splitting in simple version
            execution_time_estimate: 2000, // 2 seconds
        })
    }
    
    fn generate_cache_key(&self, request: &RouteRequest) -> String {
        format!("{}:{}:{}:{}", 
            request.from_token, 
            request.to_token, 
            request.amount,
            request.max_hops
        )
    }
    
    async fn update_metrics(&self, computation_time: u64) {
        let mut metrics = self.metrics.write().await;
        metrics.total_routes_found += 1;
        metrics.avg_computation_time_ms = 
            (metrics.avg_computation_time_ms + computation_time) / 2;
    }
}