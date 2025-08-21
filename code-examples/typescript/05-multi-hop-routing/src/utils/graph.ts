/**
 * Graph utilities for route finding
 * 
 * Helper functions for graph operations and
 * path finding algorithms.
 */

import * as graphlib from 'graphlib';

export interface GraphNode {
  id: string;
  data: any;
}

export interface GraphEdge {
  from: string;
  to: string;
  weight: number;
  data: any;
}

/**
 * Build directed graph from nodes and edges
 */
export function buildGraph(
  nodes: GraphNode[],
  edges: GraphEdge[],
  directed: boolean = false
): graphlib.Graph {
  const graph = new graphlib.Graph({ directed, multigraph: true });
  
  // Add nodes
  for (const node of nodes) {
    graph.setNode(node.id, node.data);
  }
  
  // Add edges
  for (const edge of edges) {
    graph.setEdge(edge.from, edge.to, {
      weight: edge.weight,
      ...edge.data
    });
  }
  
  return graph;
}

/**
 * Find all simple paths between two nodes
 */
export function findAllPaths(
  graph: graphlib.Graph,
  start: string,
  end: string,
  maxLength: number
): string[][] {
  const paths: string[][] = [];
  const visited = new Set<string>();
  
  function dfs(current: string, path: string[]) {
    if (path.length > maxLength) return;
    
    if (current === end) {
      paths.push([...path]);
      return;
    }
    
    visited.add(current);
    
    const neighbors = graph.neighbors(current);
    if (neighbors) {
      for (const neighbor of neighbors) {
        if (!visited.has(neighbor)) {
          dfs(neighbor, [...path, neighbor]);
        }
      }
    }
    
    visited.delete(current);
  }
  
  dfs(start, [start]);
  
  return paths;
}

/**
 * Find shortest path using Dijkstra's algorithm
 */
export function dijkstra(
  graph: graphlib.Graph,
  start: string,
  end: string
): { path: string[]; distance: number } | null {
  const distances: { [key: string]: number } = {};
  const previous: { [key: string]: string | null } = {};
  const unvisited = new Set(graph.nodes());
  
  // Initialize distances
  for (const node of graph.nodes()) {
    distances[node] = node === start ? 0 : Infinity;
    previous[node] = null;
  }
  
  while (unvisited.size > 0) {
    // Find unvisited node with minimum distance
    let current: string | null = null;
    let minDistance = Infinity;
    
    for (const node of unvisited) {
      if (distances[node] < minDistance) {
        minDistance = distances[node];
        current = node;
      }
    }
    
    if (!current || distances[current] === Infinity) break;
    
    if (current === end) {
      // Reconstruct path
      const path: string[] = [];
      let node: string | null = end;
      
      while (node) {
        path.unshift(node);
        node = previous[node];
      }
      
      return {
        path,
        distance: distances[end]
      };
    }
    
    unvisited.delete(current);
    
    // Update distances to neighbors
    const neighbors = graph.neighbors(current);
    if (neighbors) {
      for (const neighbor of neighbors) {
        if (unvisited.has(neighbor)) {
          const edge = graph.edge(current, neighbor);
          const weight = edge?.weight || 1;
          const alt = distances[current] + weight;
          
          if (alt < distances[neighbor]) {
            distances[neighbor] = alt;
            previous[neighbor] = current;
          }
        }
      }
    }
  }
  
  return null;
}

/**
 * Find negative cycles using Bellman-Ford
 */
export function bellmanFord(
  graph: graphlib.Graph,
  start: string
): { distances: { [key: string]: number }; hasNegativeCycle: boolean } {
  const distances: { [key: string]: number } = {};
  const nodes = graph.nodes();
  
  // Initialize distances
  for (const node of nodes) {
    distances[node] = node === start ? 0 : Infinity;
  }
  
  // Relax edges V-1 times
  for (let i = 0; i < nodes.length - 1; i++) {
    for (const edge of graph.edges()) {
      const edgeData = graph.edge(edge);
      const weight = edgeData?.weight || 1;
      
      if (distances[edge.v] !== Infinity &&
          distances[edge.v] + weight < distances[edge.w]) {
        distances[edge.w] = distances[edge.v] + weight;
      }
    }
  }
  
  // Check for negative cycles
  let hasNegativeCycle = false;
  for (const edge of graph.edges()) {
    const edgeData = graph.edge(edge);
    const weight = edgeData?.weight || 1;
    
    if (distances[edge.v] !== Infinity &&
        distances[edge.v] + weight < distances[edge.w]) {
      hasNegativeCycle = true;
      break;
    }
  }
  
  return { distances, hasNegativeCycle };
}

/**
 * Find strongly connected components
 */
export function findSCC(graph: graphlib.Graph): string[][] {
  return graphlib.alg.tarjan(graph);
}

/**
 * Check if graph is cyclic
 */
export function isAcyclic(graph: graphlib.Graph): boolean {
  return graphlib.alg.isAcyclic(graph);
}

/**
 * Topological sort (for DAGs)
 */
export function topologicalSort(graph: graphlib.Graph): string[] | null {
  if (!isAcyclic(graph)) {
    return null;
  }
  
  return graphlib.alg.topsort(graph);
}

/**
 * Find minimum spanning tree using Prim's algorithm
 */
export function primMST(graph: graphlib.Graph): GraphEdge[] {
  const mst: GraphEdge[] = [];
  const visited = new Set<string>();
  const nodes = graph.nodes();
  
  if (nodes.length === 0) return mst;
  
  // Start from first node
  visited.add(nodes[0]);
  
  while (visited.size < nodes.length) {
    let minEdge: GraphEdge | null = null;
    let minWeight = Infinity;
    
    // Find minimum weight edge connecting visited to unvisited
    for (const visitedNode of visited) {
      const neighbors = graph.neighbors(visitedNode);
      if (!neighbors) continue;
      
      for (const neighbor of neighbors) {
        if (!visited.has(neighbor)) {
          const edge = graph.edge(visitedNode, neighbor);
          const weight = edge?.weight || 1;
          
          if (weight < minWeight) {
            minWeight = weight;
            minEdge = {
              from: visitedNode,
              to: neighbor,
              weight,
              data: edge
            };
          }
        }
      }
    }
    
    if (minEdge) {
      mst.push(minEdge);
      visited.add(minEdge.to);
    } else {
      break; // Graph is not connected
    }
  }
  
  return mst;
}

/**
 * Calculate graph metrics
 */
export function calculateMetrics(graph: graphlib.Graph): {
  nodeCount: number;
  edgeCount: number;
  density: number;
  avgDegree: number;
  isConnected: boolean;
  diameter: number;
} {
  const nodeCount = graph.nodeCount();
  const edgeCount = graph.edgeCount();
  const maxEdges = nodeCount * (nodeCount - 1) / 2;
  const density = edgeCount / maxEdges;
  
  // Calculate average degree
  let totalDegree = 0;
  for (const node of graph.nodes()) {
    const neighbors = graph.neighbors(node);
    totalDegree += neighbors ? neighbors.length : 0;
  }
  const avgDegree = totalDegree / nodeCount;
  
  // Check connectivity
  const components = graphlib.alg.components(graph);
  const isConnected = components.length === 1;
  
  // Calculate diameter (longest shortest path)
  let diameter = 0;
  const nodes = graph.nodes();
  for (let i = 0; i < nodes.length; i++) {
    for (let j = i + 1; j < nodes.length; j++) {
      const result = dijkstra(graph, nodes[i], nodes[j]);
      if (result && result.distance > diameter) {
        diameter = result.distance;
      }
    }
  }
  
  return {
    nodeCount,
    edgeCount,
    density,
    avgDegree,
    isConnected,
    diameter
  };
}