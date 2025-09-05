/**
 * PathFinder - Graph-based path finding for route discovery
 * 
 * Implements Dijkstra's algorithm and other graph algorithms
 * to find optimal paths through the pool network.
 */

import { PublicKey } from '@solana/web3.js';
import * as graphlib from 'graphlib';
import BN from 'bn.js';
import { logger } from './utils/logger';

export interface PathParams {
  fromMint: PublicKey;
  toMint: PublicKey;
  maxHops: number;
  minLiquidity?: number;
}

export interface PoolNode {
  address: PublicKey;
  tokenA: PublicKey;
  tokenB: PublicKey;
  fee: number;
  liquidity: BN;
  volume24h?: BN;
}

export interface GraphEdge {
  pool: PublicKey;
  weight: number;
  fee: number;
  liquidity: BN;
}

export class PathFinder {
  private graph: graphlib.Graph;
  private poolMap: Map<string, PoolNode> = new Map();
  private tokenGraph: Map<string, Set<string>> = new Map();

  constructor() {
    this.graph = new graphlib.Graph({ directed: false, multigraph: true });
  }

  /**
   * Build graph from pool data
   */
  async buildGraph(pools: PoolNode[]): Promise<void> {
    try {
      logger.info(`Building graph with ${pools.length} pools`);
      
      // Clear existing graph
      this.graph = new graphlib.Graph({ directed: false, multigraph: true });
      this.poolMap.clear();
      this.tokenGraph.clear();
      
      // Add nodes (tokens) and edges (pools)
      for (const pool of pools) {
        const tokenA = pool.tokenA.toString();
        const tokenB = pool.tokenB.toString();
        const poolKey = `${pool.address.toString()}`;
        
        // Store pool data
        this.poolMap.set(poolKey, pool);
        
        // Add token nodes
        if (!this.graph.hasNode(tokenA)) {
          this.graph.setNode(tokenA);
        }
        if (!this.graph.hasNode(tokenB)) {
          this.graph.setNode(tokenB);
        }
        
        // Track token connections
        if (!this.tokenGraph.has(tokenA)) {
          this.tokenGraph.set(tokenA, new Set());
        }
        if (!this.tokenGraph.has(tokenB)) {
          this.tokenGraph.set(tokenB, new Set());
        }
        
        this.tokenGraph.get(tokenA)!.add(tokenB);
        this.tokenGraph.get(tokenB)!.add(tokenA);
        
        // Calculate edge weight (lower is better)
        const weight = this.calculateEdgeWeight(pool);
        
        // Add edge with pool data
        const edgeData: GraphEdge = {
          pool: pool.address,
          weight,
          fee: pool.fee,
          liquidity: pool.liquidity
        };
        
        this.graph.setEdge(tokenA, tokenB, edgeData, poolKey);
      }
      
      logger.info(`Graph built: ${this.graph.nodeCount()} tokens, ${this.graph.edgeCount()} pools`);
      
    } catch (error) {
      logger.error('Error building graph:', error);
      throw error;
    }
  }

  /**
   * Find all paths between two tokens
   */
  async findPaths(params: PathParams): Promise<PublicKey[][]> {
    try {
      const fromToken = params.fromMint.toString();
      const toToken = params.toMint.toString();
      
      logger.debug(`Finding paths: ${fromToken} → ${toToken} (max ${params.maxHops} hops)`);
      
      // Check if tokens exist in graph
      if (!this.graph.hasNode(fromToken) || !this.graph.hasNode(toToken)) {
        logger.debug('One or both tokens not in graph');
        return [];
      }
      
      const paths: PublicKey[][] = [];
      
      // Strategy 1: Find shortest path using Dijkstra
      const shortestPath = this.findShortestPath(fromToken, toToken);
      if (shortestPath && shortestPath.length - 1 <= params.maxHops) {
        paths.push(shortestPath.map(s => new PublicKey(s)));
      }
      
      // Strategy 2: Find alternative paths using BFS
      const alternativePaths = this.findAlternativePaths(
        fromToken, 
        toToken, 
        params.maxHops,
        params.minLiquidity
      );
      
      for (const path of alternativePaths) {
        if (path.length - 1 <= params.maxHops) {
          paths.push(path.map(s => new PublicKey(s)));
        }
      }
      
      // Remove duplicates
      const uniquePaths = this.removeDuplicatePaths(paths);
      
      logger.debug(`Found ${uniquePaths.length} unique paths`);
      
      return uniquePaths;
      
    } catch (error) {
      logger.error('Error finding paths:', error);
      return [];
    }
  }

  /**
   * Find shortest path using Dijkstra's algorithm
   */
  private findShortestPath(from: string, to: string): string[] | null {
    try {
      const result = graphlib.alg.dijkstra(this.graph, from, (e) => {
        const edge = this.graph.edge(e) as GraphEdge;
        return edge ? edge.weight : 1;
      });
      
      if (!result[to] || result[to].distance === Infinity) {
        return null;
      }
      
      // Reconstruct path
      const path: string[] = [];
      let current = to;
      
      while (current !== from) {
        path.unshift(current);
        const predecessor = result[current].predecessor;
        if (!predecessor) break;
        current = predecessor;
      }
      
      path.unshift(from);
      
      return path;
      
    } catch (error) {
      logger.error('Error in Dijkstra:', error);
      return null;
    }
  }

  /**
   * Find alternative paths using BFS
   */
  private findAlternativePaths(
    from: string,
    to: string,
    maxHops: number,
    minLiquidity?: number
  ): string[][] {
    const paths: string[][] = [];
    const visited = new Set<string>();
    const queue: Array<{ node: string; path: string[]; hops: number }> = [];
    
    queue.push({ node: from, path: [from], hops: 0 });
    
    while (queue.length > 0) {
      const { node, path, hops } = queue.shift()!;
      
      if (node === to) {
        paths.push([...path]);
        continue;
      }
      
      if (hops >= maxHops) {
        continue;
      }
      
      // Mark as visited for this path
      const pathKey = path.join(',');
      if (visited.has(`${pathKey},${node}`)) {
        continue;
      }
      visited.add(`${pathKey},${node}`);
      
      // Get neighbors
      const neighbors = this.graph.neighbors(node);
      if (!neighbors) continue;
      
      for (const neighbor of neighbors) {
        // Skip if already in path (no cycles)
        if (path.includes(neighbor)) {
          continue;
        }
        
        // Check liquidity if specified
        if (minLiquidity) {
          const edge = this.graph.edge(node, neighbor) as GraphEdge;
          if (edge && edge.liquidity.lt(new BN(minLiquidity))) {
            continue;
          }
        }
        
        queue.push({
          node: neighbor,
          path: [...path, neighbor],
          hops: hops + 1
        });
      }
    }
    
    return paths;
  }

  /**
   * Find paths using A* algorithm with heuristic
   */
  findAStarPath(from: string, to: string, maxHops: number): string[] | null {
    // Priority queue for A*
    const openSet = new Set([from]);
    const cameFrom = new Map<string, string>();
    
    const gScore = new Map<string, number>();
    gScore.set(from, 0);
    
    const fScore = new Map<string, number>();
    fScore.set(from, this.heuristic(from, to));
    
    while (openSet.size > 0) {
      // Find node with lowest fScore
      let current: string | null = null;
      let lowestScore = Infinity;
      
      for (const node of openSet) {
        const score = fScore.get(node) || Infinity;
        if (score < lowestScore) {
          lowestScore = score;
          current = node;
        }
      }
      
      if (!current) break;
      
      if (current === to) {
        // Reconstruct path
        const path: string[] = [];
        let node: string | undefined = current;
        
        while (node) {
          path.unshift(node);
          node = cameFrom.get(node);
        }
        
        if (path.length - 1 <= maxHops) {
          return path;
        }
      }
      
      openSet.delete(current);
      
      // Check neighbors
      const neighbors = this.graph.neighbors(current);
      if (!neighbors) continue;
      
      for (const neighbor of neighbors) {
        const edge = this.graph.edge(current, neighbor) as GraphEdge;
        if (!edge) continue;
        
        const tentativeGScore = (gScore.get(current) || 0) + edge.weight;
        
        if (tentativeGScore < (gScore.get(neighbor) || Infinity)) {
          cameFrom.set(neighbor, current);
          gScore.set(neighbor, tentativeGScore);
          fScore.set(neighbor, tentativeGScore + this.heuristic(neighbor, to));
          openSet.add(neighbor);
        }
      }
    }
    
    return null;
  }

  /**
   * Heuristic function for A*
   */
  private heuristic(from: string, to: string): number {
    // Simple heuristic: minimum possible hops
    // In reality, would use price ratios or other domain knowledge
    const connected = this.tokenGraph.get(from);
    if (connected && connected.has(to)) {
      return 1; // Direct connection
    }
    return 2; // Assume at least 2 hops
  }

  /**
   * Calculate edge weight based on pool properties
   */
  private calculateEdgeWeight(pool: PoolNode): number {
    // Lower weight = better path
    // Consider: fees, liquidity, volume
    
    let weight = 0;
    
    // Fee component (0.01% = 1, 1% = 100)
    weight += pool.fee * 100;
    
    // Liquidity component (inverse relationship)
    const liquidityScore = 1000000 / Math.max(pool.liquidity.toNumber(), 1);
    weight += liquidityScore;
    
    // Volume component (if available)
    if (pool.volume24h) {
      const volumeScore = 100000 / Math.max(pool.volume24h.toNumber(), 1);
      weight += volumeScore * 0.5;
    }
    
    return Math.max(weight, 0.01);
  }

  /**
   * Remove duplicate paths
   */
  private removeDuplicatePaths(paths: PublicKey[][]): PublicKey[][] {
    const seen = new Set<string>();
    const unique: PublicKey[][] = [];
    
    for (const path of paths) {
      const key = path.map(p => p.toString()).join(',');
      if (!seen.has(key)) {
        seen.add(key);
        unique.push(path);
      }
    }
    
    return unique;
  }

  /**
   * Find cycles (for arbitrage)
   */
  findCycles(startToken: string, maxLength: number): string[][] {
    const cycles: string[][] = [];
    const visited = new Set<string>();
    
    const dfs = (node: string, path: string[], depth: number) => {
      if (depth > maxLength) return;
      
      visited.add(node);
      
      const neighbors = this.graph.neighbors(node);
      if (!neighbors) return;
      
      for (const neighbor of neighbors) {
        if (neighbor === startToken && path.length >= 3) {
          // Found a cycle
          cycles.push([...path, neighbor]);
        } else if (!visited.has(neighbor)) {
          dfs(neighbor, [...path, neighbor], depth + 1);
        }
      }
      
      visited.delete(node);
    };
    
    dfs(startToken, [startToken], 0);
    
    return cycles;
  }

  /**
   * Check if path exists
   */
  hasPath(from: string, to: string): boolean {
    return this.findShortestPath(from, to) !== null;
  }

  /**
   * Get connected tokens
   */
  getConnectedTokens(token: string): string[] {
    const neighbors = this.graph.neighbors(token);
    return neighbors || [];
  }

  /**
   * Get graph statistics
   */
  getStatistics(): any {
    return {
      tokenCount: this.graph.nodeCount(),
      poolCount: this.graph.edgeCount(),
      avgDegree: this.graph.edgeCount() * 2 / this.graph.nodeCount(),
      components: graphlib.alg.components(this.graph).length
    };
  }
}