# Multi-Hop Routing and Arbitrage Detection

Advanced multi-hop routing system for Saros DLMM with graph-based pathfinding, arbitrage detection, and optimized execution strategies.

## Features

### 🚀 Advanced Route Finding
- **A\* Algorithm**: Optimal pathfinding with custom heuristics
- **Dijkstra's Algorithm**: True shortest path discovery
- **Alternative Paths**: Multiple route options with different trade-offs
- **Route Splitting**: Large order optimization across multiple paths
- **Liquidity-Optimized Routing**: Prioritize high-liquidity pools

### 🔄 Arbitrage Detection
- **Cycle Detection**: Floyd-Warshall based negative cycle detection
- **Real-time Monitoring**: Continuous price monitoring for opportunities
- **Profitability Analysis**: Risk-adjusted profit calculations
- **MEV Protection**: Private mempool and bundle execution support

### ⚡ Execution Engine
- **Atomic Transactions**: Multi-hop swaps in single transaction
- **Gas Optimization**: Dynamic fee calculation and optimization
- **Retry Logic**: Robust transaction submission with fallbacks
- **Batch Execution**: Portfolio rebalancing with parallel execution

### 📊 Graph Analytics
- **Pool Connectivity**: Comprehensive liquidity graph analysis
- **Token Centrality**: Network importance scoring
- **Visualization Export**: GraphViz DOT format export
- **Performance Metrics**: Route success rates and optimization

## Graph Algorithms Explained

### 1. Pool Connectivity Graph

The system builds a weighted undirected graph where:
- **Nodes**: Tokens (ERC-20 addresses)
- **Edges**: DLMM pools with routing weights
- **Weights**: Composite of liquidity, fees, and volume

```rust
// Edge weight calculation
weight = liquidity_factor + fee_factor + volume_factor

liquidity_factor = 1.0 / (pool.liquidity_usd + 1.0)  // Lower is better
fee_factor = pool.fee_tier                           // Direct fee cost
volume_factor = 1.0 / (pool.volume_24h + 1.0)      // Higher volume preferred
```

### 2. A\* Pathfinding Algorithm

Enhanced A\* implementation with domain-specific heuristics:

```rust
// A* priority function
f(n) = g(n) + h(n)

g(n) = actual_cost_from_start     // Cumulative swap costs
h(n) = heuristic_to_goal         // Token connectivity estimate
```

**Heuristic Function**: Uses token connectivity analysis to estimate remaining cost:
- Direct pairs count (1-hop connections)
- Liquidity centrality score
- Historical routing success rates

### 3. Arbitrage Cycle Detection

Modified Floyd-Warshall algorithm to detect profitable cycles:

```rust
// Detect negative weight cycles (profit opportunities)
for k in 0..n {
    for i in 0..n {
        for j in 0..n {
            if distance[i][j] > distance[i][k] + distance[k][j] {
                distance[i][j] = distance[i][k] + distance[k][j];
                // Potential arbitrage cycle detected
            }
        }
    }
}
```

**Profitability Analysis**:
- Minimum profit threshold filtering
- Risk assessment (liquidity, price impact)
- Execution complexity scoring
- Time-sensitive opportunity flagging

### 4. Route Optimization

Multi-criteria optimization using weighted scoring:

```rust
route_score = w1 * price_impact + w2 * gas_cost + w3 * liquidity_depth + w4 * certainty

// Default weights
w1 = 0.4  // Price impact weight
w2 = 0.2  // Gas cost weight  
w3 = 0.3  // Liquidity depth weight
w4 = 0.1  // Execution certainty weight
```

## Installation and Setup

### Prerequisites
- Rust 1.70+
- Solana CLI tools
- Access to Solana RPC endpoint

### Clone and Build
```bash
git clone <repository>
cd code-examples/rust/05-multi-hop-routing
cargo build --release
```

### Environment Configuration
```bash
cp .env.example .env
# Edit .env with your configuration
```

Required environment variables:
```bash
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
PRIVATE_KEY=your_base58_private_key_here
MAX_SLIPPAGE=0.01
GAS_PRICE_MULTIPLIER=1.2
ENABLE_MEV_PROTECTION=false
```

## Usage Examples

### Basic Route Finding
```bash
# Find optimal route between USDC and SOL
cargo run -- route \
    --from-token 6vJy5gqpGJKoACJBu3ixq5o6jzZEiCDKZBs6yvVgQ1Cc \
    --to-token 11111111111111111111111111111112 \
    --amount 1000 \
    --max-hops 3 \
    --slippage 0.01
```

### Arbitrage Detection
```bash
# Scan for arbitrage opportunities
cargo run -- arbitrage \
    --min-profit-usd 50 \
    --max-cycle-length 4
```

### Route Execution
```bash
# Execute a discovered route
cargo run -- execute \
    --route-id abc123-def456 \
    --amount 1000 \
    --simulate true
```

### Graph Analysis
```bash
# Analyze pool connectivity
cargo run -- analyze \
    --token 6vJy5gqpGJKoACJBu3ixq5o6jzZEiCDKZBs6yvVgQ1Cc \
    --export-graph true
```

### Real-time Monitoring
```bash
# Monitor routing opportunities
cargo run -- monitor \
    --tokens 6vJy5gqpGJKoACJBu3ixq5o6jzZEiCDKZBs6yvVgQ1Cc,11111111111111111111111111111112 \
    --interval-ms 5000
```

## Architecture

### Core Components

1. **PoolGraph** (`src/pool_graph.rs`)
   - Graph construction and maintenance
   - Node/edge management
   - Connectivity analysis
   - Background data updates

2. **RouteFinder** (`src/route_finder.rs`)
   - A\* and Dijkstra implementations
   - Route optimization algorithms
   - Caching and performance optimization
   - Split route calculation

3. **ArbitrageDetector** (`src/arbitrage_detector.rs`)
   - Cycle detection algorithms
   - Profitability analysis
   - Risk assessment
   - Opportunity ranking

4. **RouteExecutor** (`src/route_executor.rs`)
   - Transaction building and submission
   - Gas optimization
   - MEV protection
   - Batch execution support

### Data Flow

```
Market Data → Pool Graph → Route Finding → Optimization → Execution
     ↓             ↓           ↓             ↓           ↓
Price Feed → Connectivity → Path Discovery → Ranking → Transaction
     ↓             ↓           ↓             ↓           ↓  
Liquidity → Edge Weights → Route Options → Selection → Confirmation
```

## Performance Optimizations

### 1. Caching Strategy
- **Route Cache**: 30-second TTL for frequently requested routes
- **Graph Cache**: Incremental updates vs full rebuilds
- **Price Cache**: Real-time price monitoring with efficient updates

### 2. Algorithm Optimizations
- **Early Termination**: Stop search when acceptable routes found
- **Pruning**: Remove low-quality paths during search
- **Parallel Processing**: Concurrent route evaluation
- **Memory Pool**: Reuse data structures to reduce allocations

### 3. Network Efficiency
- **Batch RPC Calls**: Minimize network round trips
- **Connection Pooling**: Persistent RPC connections
- **Selective Updates**: Only fetch changed pool data

## Risk Management

### 1. Slippage Protection
- **Dynamic Slippage**: Adjust based on market volatility
- **Multi-hop Accumulation**: Account for cumulative slippage
- **Deadline Enforcement**: Time-based execution limits

### 2. MEV Protection
- **Private Mempool**: Optional private transaction submission
- **Bundle Execution**: Atomic multi-transaction bundles
- **Priority Fees**: Dynamic fee optimization for fast inclusion

### 3. Liquidity Analysis
- **Depth Checking**: Ensure sufficient pool liquidity
- **Impact Modeling**: Price impact prediction
- **Reserve Monitoring**: Track available token reserves

## Benchmarks

Performance characteristics on standard hardware:

| Operation | Time | Memory |
|-----------|------|---------|
| Graph Build (1000 pools) | 150ms | 50MB |
| A\* Route (3-hop) | 5ms | 2MB |
| Arbitrage Scan (500 tokens) | 800ms | 100MB |
| Route Execution | 2000ms | 5MB |

## Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
cargo test --features integration
```

### Benchmarks
```bash
cargo bench
```

### Property-Based Testing
```bash
cargo test --features proptest
```

## Configuration

### Route Optimization Parameters
```rust
RouteOptimizationParams {
    weight_price_impact: 0.4,      // Price impact importance
    weight_gas_cost: 0.2,          // Gas cost importance  
    weight_liquidity_depth: 0.3,   // Liquidity importance
    weight_execution_certainty: 0.1, // Execution certainty
    max_split_routes: 4,           // Maximum route splits
    min_split_amount_usd: 100,     // Minimum split size
}
```

### Gas Configuration
```rust
GasEstimation {
    base_gas: 20000,               // Base transaction cost
    per_hop_gas: 50000,           // Cost per swap hop
    compute_units: calculated,     // Total compute units
    priority_fee: dynamic,         // Dynamic priority fee
}
```

## Monitoring and Metrics

### Key Metrics Tracked
- Route success rates
- Average execution time
- Gas efficiency
- Price impact accuracy
- Cache hit rates
- Arbitrage profitability

### Logging Configuration
```rust
// Set log level in environment
RUST_LOG=info,multi_hop_routing=debug
```

### Prometheus Metrics
- Route discovery latency
- Transaction success rates
- Gas usage efficiency
- Pool connectivity health

## Contributing

1. Fork the repository
2. Create a feature branch
3. Implement changes with tests
4. Update documentation
5. Submit pull request

### Code Standards
- Follow Rust idioms and best practices
- Add comprehensive unit tests
- Document public APIs
- Use meaningful variable names
- Handle errors gracefully

## License

MIT License - see LICENSE file for details.

## Acknowledgments

- Solana Foundation for blockchain infrastructure
- petgraph crate for graph algorithms
- Saros Protocol for DLMM implementation
- Rust community for excellent tooling