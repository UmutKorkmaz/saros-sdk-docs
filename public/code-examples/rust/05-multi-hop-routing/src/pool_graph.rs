use anyhow::Result;
use dashmap::DashMap;
use petgraph::{Graph, Undirected};
use petgraph::visit::EdgeRef;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{info, warn, debug};

use crate::types::*;
use saros_dlmm_sdk::SarosClient;

/// Pool connectivity graph manager for efficient route finding
pub struct PoolGraph {
    /// The main graph structure with tokens as nodes and pools as edges
    graph: Arc<tokio::sync::RwLock<TokenGraph>>,
    
    /// Fast lookups: token address -> node index
    token_to_node: Arc<DashMap<Pubkey, NodeIndex>>,
    
    /// Fast lookups: node index -> token address
    node_to_token: Arc<DashMap<NodeIndex, Pubkey>>,
    
    /// Pool data cache
    pool_cache: Arc<DashMap<Pubkey, PoolNode>>,
    
    /// Token data cache
    token_cache: Arc<DashMap<Pubkey, TokenNode>>,
    
    /// Saros client for data fetching
    client: Arc<SarosClient>,
    
    /// Last graph update timestamp
    last_update: Arc<tokio::sync::RwLock<u64>>,
}

impl PoolGraph {
    pub async fn new() -> Result<Arc<Self>> {
        info!("Initializing pool connectivity graph");
        
        let client = Arc::new(SarosClient::new_mock()?);
        let graph = Arc::new(tokio::sync::RwLock::new(Graph::new_undirected()));
        
        let pool_graph = Arc::new(Self {
            graph,
            token_to_node: Arc::new(DashMap::new()),
            node_to_token: Arc::new(DashMap::new()),
            pool_cache: Arc::new(DashMap::new()),
            token_cache: Arc::new(DashMap::new()),
            client,
            last_update: Arc::new(tokio::sync::RwLock::new(0)),
        });
        
        // Initial graph construction
        pool_graph.rebuild_graph().await?;
        
        // Start background update task
        let graph_clone = pool_graph.clone();
        tokio::spawn(async move {
            graph_clone.background_update_loop().await;
        });
        
        Ok(pool_graph)
    }
    
    /// Rebuild the entire graph from scratch
    pub async fn rebuild_graph(&self) -> Result<()> {
        info!("Rebuilding pool connectivity graph");
        
        // Fetch all pools and tokens
        let pools = self.client.get_all_pools().await?;
        let tokens = self.client.get_all_tokens().await?;
        
        debug!("Found {} pools and {} tokens", pools.len(), tokens.len());
        
        // Clear existing data
        {
            let mut graph = self.graph.write().await;
            graph.clear();
        }
        self.token_to_node.clear();
        self.node_to_token.clear();
        self.pool_cache.clear();
        self.token_cache.clear();
        
        // Add tokens as nodes
        let mut node_indices = HashMap::new();
        {
            let mut graph = self.graph.write().await;
            
            for token in &tokens {
                let token_node = TokenNode {
                    address: token.mint,
                    symbol: token.symbol.clone(),
                    decimals: token.decimals,
                    price_usd: token.price_usd.unwrap_or_default(),
                    market_cap: Decimal::ZERO,
                    pools: Vec::new(),
                };
                
                let node_idx = graph.add_node(token_node.clone());
                node_indices.insert(token.mint, node_idx);
                
                self.token_to_node.insert(token.mint, node_idx);
                self.node_to_token.insert(node_idx, token.mint);
                self.token_cache.insert(token.mint, token_node);
            }
        }
        
        // Add pools as edges
        {
            let mut graph = self.graph.write().await;
            
            for pool in &pools {
                let pool_node = PoolNode {
                    address: pool.address,
                    token_a: pool.token_a,
                    token_b: pool.token_b,
                    liquidity_usd: pool.liquidity_usd,
                    fee_tier: pool.fee_rate,
                    volume_24h: pool.volume_24h.unwrap_or_default(),
                    active_bins: pool.active_bins.unwrap_or(0),
                    bin_step: pool.bin_step.unwrap_or(0),
                    last_updated: chrono::Utc::now().timestamp() as u64,
                };
                
                // Skip pools with insufficient liquidity
                if pool_node.liquidity_usd < MIN_LIQUIDITY_USD {
                    continue;
                }
                
                if let (Some(&node_a), Some(&node_b)) = (
                    node_indices.get(&pool.token_a),
                    node_indices.get(&pool.token_b),
                ) {
                    // Calculate edge weight (lower is better for routing)
                    let weight = self.calculate_edge_weight(&pool_node);
                    
                    let edge = GraphEdge {
                        pool: pool.address,
                        weight,
                        gas_cost: Decimal::from(5000), // Base gas cost
                        price_impact_factor: self.calculate_price_impact_factor(&pool_node),
                    };
                    
                    graph.add_edge(node_a, node_b, edge);
                    self.pool_cache.insert(pool.address, pool_node);
                }
            }
        }
        
        // Update token pool associations
        self.update_token_pool_associations().await?;
        
        let mut last_update = self.last_update.write().await;
        *last_update = chrono::Utc::now().timestamp() as u64;
        
        let graph_read = self.graph.read().await;
        info!("Graph rebuilt: {} nodes, {} edges", 
            graph_read.node_count(), 
            graph_read.edge_count()
        );
        
        Ok(())
    }
    
    /// Get all neighbor nodes for a given token
    pub async fn get_neighbors(&self, token: Pubkey) -> Result<Vec<(Pubkey, GraphEdge)>> {
        let graph = self.graph.read().await;
        
        let node_idx = self.token_to_node
            .get(&token)
            .ok_or_else(|| RoutingError::InvalidTokenPair { 
                from: token, 
                to: Pubkey::default() 
            })?;
        
        let mut neighbors = Vec::new();
        
        let mut edges = graph.edges(*node_idx);
        while let Some(edge_ref) = edges.next() {
            let neighbor_idx = edge_ref.target();
            if let Some(neighbor_token) = self.node_to_token.get(&neighbor_idx) {
                neighbors.push((*neighbor_token, edge_ref.weight().clone()));
            }
        }
        
        Ok(neighbors)
    }
    
    /// Find the shortest path between two tokens using Dijkstra's algorithm
    pub async fn find_shortest_path(
        &self, 
        from: Pubkey, 
        to: Pubkey
    ) -> Result<Option<Vec<NodeIndex>>> {
        let graph = self.graph.read().await;
        
        let from_node = self.token_to_node
            .get(&from)
            .ok_or_else(|| RoutingError::InvalidTokenPair { from, to })?;
        
        let to_node = self.token_to_node
            .get(&to)
            .ok_or_else(|| RoutingError::InvalidTokenPair { from, to })?;
        
        // Use Dijkstra's algorithm
        let path = petgraph::algo::dijkstra(
            &*graph,
            *from_node,
            Some(*to_node),
            |edge| (edge.weight().weight * 1000.0) as i32, // Convert to integer for dijkstra
        );
        
        if path.contains_key(&to_node) {
            // Reconstruct path
            let shortest_path = petgraph::algo::astar(
                &*graph,
                *from_node,
                |node| node == *to_node,
                |edge| (edge.weight().weight * 1000.0) as i32,
                |_| 0, // No heuristic for exact shortest path
            );
            
            Ok(shortest_path.map(|(_, path)| path))
        } else {
            Ok(None)
        }
    }
    
    /// Find multiple paths with different trade-offs
    pub async fn find_alternative_paths(
        &self,
        from: Pubkey,
        to: Pubkey,
        max_paths: usize,
        max_hops: u8,
    ) -> Result<Vec<Vec<NodeIndex>>> {
        let graph = self.graph.read().await;
        
        let from_node = self.token_to_node
            .get(&from)
            .ok_or_else(|| RoutingError::InvalidTokenPair { from, to })?;
        
        let to_node = self.token_to_node
            .get(&to)
            .ok_or_else(|| RoutingError::InvalidTokenPair { from, to })?;
        
        let mut paths = Vec::new();
        let mut used_edges = HashSet::new();
        
        // Find multiple paths by temporarily removing edges from previous paths
        for _ in 0..max_paths {
            let path = self.find_path_excluding_edges(
                &graph,
                *from_node,
                *to_node,
                &used_edges,
                max_hops,
            )?;
            
            if let Some(path_nodes) = path {
                if path_nodes.len() <= max_hops as usize + 1 {
                    // Add edges from this path to exclusion set for next iteration
                    for window in path_nodes.windows(2) {
                        if let Some(edge_idx) = graph.find_edge(window[0], window[1]) {
                            used_edges.insert(edge_idx);
                        }
                    }
                    paths.push(path_nodes);
                }
            } else {
                break; // No more paths available
            }
        }
        
        Ok(paths)
    }
    
    /// Get comprehensive graph statistics
    pub async fn get_graph_statistics(&self) -> Result<GraphStatistics> {
        let graph = self.graph.read().await;
        
        let total_pools = graph.edge_count();
        let total_tokens = graph.node_count();
        
        // Calculate average liquidity
        let total_liquidity: Decimal = self.pool_cache
            .iter()
            .map(|entry| entry.value().liquidity_usd)
            .sum();
        let avg_liquidity_usd = if total_pools > 0 {
            total_liquidity / Decimal::from(total_pools)
        } else {
            Decimal::ZERO
        };
        
        // Calculate graph density
        let max_edges = if total_tokens > 1 {
            total_tokens * (total_tokens - 1) / 2
        } else {
            1
        };
        let graph_density = total_pools as f64 / max_edges as f64;
        
        // Find largest connected component
        let component_count = petgraph::algo::connected_components(&*graph);
        let largest_component_size = if component_count == 0 { 0 } else { total_tokens };
        
        // Calculate clustering coefficient (simplified)
        let clustering_coefficient = self.calculate_clustering_coefficient(&graph).await;
        
        Ok(GraphStatistics {
            total_pools,
            total_tokens,
            avg_liquidity_usd,
            graph_density,
            largest_component_size,
            clustering_coefficient,
        })
    }
    
    /// Analyze connectivity for a specific token
    pub async fn analyze_token_connectivity(
        &self,
        token: Pubkey,
    ) -> Result<TokenConnectivityAnalysis> {
        let graph = self.graph.read().await;
        
        let node_idx = self.token_to_node
            .get(&token)
            .ok_or_else(|| RoutingError::InvalidTokenPair { 
                from: token, 
                to: Pubkey::default() 
            })?;
        
        // Direct pairs (1-hop)
        let direct_pairs = graph.edges(*node_idx).count();
        
        // 2-hop reachability
        let mut two_hop_tokens = HashSet::new();
        for edge in graph.edges(*node_idx) {
            let neighbor = edge.target();
            for second_edge in graph.edges(neighbor) {
                let second_neighbor = second_edge.target();
                if second_neighbor != *node_idx {
                    two_hop_tokens.insert(second_neighbor);
                }
            }
        }
        let two_hop_count = two_hop_tokens.len();
        
        // 3-hop reachability (sample-based for performance)
        let mut three_hop_tokens = HashSet::new();
        for &two_hop_node in two_hop_tokens.iter().take(50) { // Sample to avoid performance issues
            for edge in graph.edges(two_hop_node) {
                let third_neighbor = edge.target();
                if third_neighbor != *node_idx && !two_hop_tokens.contains(&third_neighbor) {
                    three_hop_tokens.insert(third_neighbor);
                }
            }
        }
        let three_hop_count = three_hop_tokens.len();
        
        // Calculate centrality score (simplified betweenness centrality)
        let centrality_score = direct_pairs as f64 / graph.node_count().max(1) as f64;
        
        // Calculate liquidity centrality
        let liquidity_centrality: Decimal = graph.edges(*node_idx)
            .filter_map(|edge| self.pool_cache.get(&edge.weight().pool))
            .map(|pool| pool.liquidity_usd)
            .sum();
        
        Ok(TokenConnectivityAnalysis {
            token,
            direct_pairs,
            two_hop_tokens: two_hop_count,
            three_hop_tokens: three_hop_count,
            centrality_score,
            liquidity_centrality,
        })
    }
    
    /// Export graph for visualization
    pub async fn export_graph_visualization(&self, filename: &str) -> Result<()> {
        use std::io::Write;
        
        let graph = self.graph.read().await;
        let mut file = std::fs::File::create(filename)?;
        
        writeln!(file, "graph G {{")?;
        writeln!(file, "  rankdir=LR;")?;
        writeln!(file, "  node [shape=circle];")?;
        
        // Write nodes
        for node_idx in graph.node_indices() {
            if let Some(token_addr) = self.node_to_token.get(&node_idx) {
                if let Some(token) = self.token_cache.get(&*token_addr) {
                    writeln!(file, "  \"{}\" [label=\"{}\"];", 
                        *token_addr, 
                        token.symbol
                    )?;
                }
            }
        }
        
        // Write edges
        for edge_idx in graph.edge_indices() {
            if let Some((node_a, node_b)) = graph.edge_endpoints(edge_idx) {
                if let (Some(token_a), Some(token_b)) = (
                    self.node_to_token.get(&node_a),
                    self.node_to_token.get(&node_b),
                ) {
                    let edge_data = &graph[edge_idx];
                    if let Some(pool) = self.pool_cache.get(&edge_data.pool) {
                        writeln!(file, "  \"{}\" -- \"{}\" [label=\"${:.0}k\"];", 
                            *token_a,
                            *token_b,
                            pool.liquidity_usd / Decimal::from(1000)
                        )?;
                    }
                }
            }
        }
        
        writeln!(file, "}}")?;
        Ok(())
    }
    
    /// Get pool information for a specific pool address
    pub fn get_pool_info(&self, pool_address: Pubkey) -> Option<PoolNode> {
        self.pool_cache.get(&pool_address).map(|entry| entry.clone())
    }
    
    /// Get token information for a specific token address
    pub fn get_token_info(&self, token_address: Pubkey) -> Option<TokenNode> {
        self.token_cache.get(&token_address).map(|entry| entry.clone())
    }
    
    /// Get token address from node index (for route finding)
    pub fn get_token_from_node(&self, node_idx: NodeIndex) -> Option<Pubkey> {
        self.node_to_token.get(&node_idx).map(|entry| *entry)
    }
    
    /// Get node index from token address (for route finding)
    pub fn get_node_from_token(&self, token: Pubkey) -> Option<NodeIndex> {
        self.token_to_node.get(&token).map(|entry| *entry)
    }
    
    /// Get the underlying graph (for advanced operations)
    pub async fn get_graph(&self) -> tokio::sync::RwLockReadGuard<TokenGraph> {
        self.graph.read().await
    }
    
    // Private helper methods
    
    fn calculate_edge_weight(&self, pool: &PoolNode) -> f64 {
        // Lower weight = better for routing
        // Factors: liquidity (higher is better), fees (lower is better), volume (higher is better)
        
        let liquidity_factor = 1.0 / (pool.liquidity_usd.to_f64().unwrap_or(1.0) + 1.0);
        let fee_factor = pool.fee_tier.to_f64().unwrap_or(0.003);
        let volume_factor = 1.0 / (pool.volume_24h.to_f64().unwrap_or(1.0) + 1.0);
        
        liquidity_factor + fee_factor + volume_factor
    }
    
    fn calculate_price_impact_factor(&self, pool: &PoolNode) -> f64 {
        // Higher active bins and liquidity = lower price impact
        let bin_factor = 1.0 / (pool.active_bins as f64 + 1.0);
        let liquidity_factor = 1.0 / (pool.liquidity_usd.to_f64().unwrap_or(1.0) + 1.0);
        
        bin_factor + liquidity_factor
    }
    
    fn find_path_excluding_edges(
        &self,
        graph: &TokenGraph,
        from: NodeIndex,
        to: NodeIndex,
        excluded_edges: &HashSet<EdgeIndex>,
        max_hops: u8,
    ) -> Result<Option<Vec<NodeIndex>>> {
        // Custom BFS/DFS with edge exclusion
        use std::collections::VecDeque;
        
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<NodeIndex, NodeIndex> = HashMap::new();
        
        queue.push_back((from, 0));
        visited.insert(from);
        
        while let Some((current, hops)) = queue.pop_front() {
            if current == to {
                // Reconstruct path
                let mut path = Vec::new();
                let mut node = to;
                
                while node != from {
                    path.push(node);
                    node = parent[&node];
                }
                path.push(from);
                path.reverse();
                
                return Ok(Some(path));
            }
            
            if hops >= max_hops {
                continue;
            }
            
            for edge in graph.edges(current) {
                let edge_idx = edge.id();
                if excluded_edges.contains(&edge_idx) {
                    continue;
                }
                
                let neighbor = edge.target();
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent.insert(neighbor, current);
                    queue.push_back((neighbor, hops + 1));
                }
            }
        }
        
        Ok(None)
    }
    
    async fn calculate_clustering_coefficient(&self, graph: &TokenGraph) -> f64 {
        // Simplified clustering coefficient calculation
        let mut total_coefficient = 0.0;
        let mut node_count = 0;
        
        for node in graph.node_indices().take(100) { // Sample for performance
            let neighbors: Vec<_> = graph.neighbors(node).collect();
            if neighbors.len() < 2 {
                continue;
            }
            
            let mut edges_between_neighbors = 0;
            for i in 0..neighbors.len() {
                for j in (i + 1)..neighbors.len() {
                    if graph.find_edge(neighbors[i], neighbors[j]).is_some() {
                        edges_between_neighbors += 1;
                    }
                }
            }
            
            let possible_edges = neighbors.len() * (neighbors.len() - 1) / 2;
            if possible_edges > 0 {
                total_coefficient += edges_between_neighbors as f64 / possible_edges as f64;
                node_count += 1;
            }
        }
        
        if node_count > 0 {
            total_coefficient / node_count as f64
        } else {
            0.0
        }
    }
    
    async fn update_token_pool_associations(&self) -> Result<()> {
        let mut token_pools: HashMap<Pubkey, Vec<Pubkey>> = HashMap::new();
        
        for entry in self.pool_cache.iter() {
            let pool = entry.value();
            token_pools.entry(pool.token_a).or_default().push(pool.address);
            token_pools.entry(pool.token_b).or_default().push(pool.address);
        }
        
        for (token_addr, pools) in token_pools {
            if let Some(mut token) = self.token_cache.get_mut(&token_addr) {
                token.pools = pools;
            }
        }
        
        Ok(())
    }
    
    async fn background_update_loop(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        
        loop {
            interval.tick().await;
            
            match self.update_pool_data().await {
                Ok(_) => debug!("Pool data updated successfully"),
                Err(e) => warn!("Failed to update pool data: {}", e),
            }
        }
    }
    
    async fn update_pool_data(&self) -> Result<()> {
        // Update pool liquidity and other dynamic data
        let pools = self.client.get_all_pools().await?;
        
        for pool in pools {
            if let Some(mut cached_pool) = self.pool_cache.get_mut(&pool.address) {
                cached_pool.liquidity_usd = pool.liquidity_usd;
                cached_pool.volume_24h = pool.volume_24h.unwrap_or_default();
                cached_pool.last_updated = chrono::Utc::now().timestamp() as u64;
            }
        }
        
        Ok(())
    }
}