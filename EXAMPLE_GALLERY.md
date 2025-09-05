# 💻 Saros SDK Example Gallery

> **Production-Ready Code Examples** - Complete implementations with testing, error handling, and best practices

## 🎯 Choose Your Language

### ⚡ TypeScript Examples

<div class="examples-grid">

#### 🔄 [01. Swap with Dynamic Slippage](./code-examples/typescript/01-swap-with-slippage/)
**Difficulty**: Beginner | **Focus**: Core Swaps

Production swap implementation featuring:
- ✅ **Dynamic slippage calculation** based on market volatility
- ✅ **MEV protection** mechanisms  
- ✅ **Real-time price monitoring** with alerts
- ✅ **Comprehensive error recovery** strategies
- ✅ **Performance analytics** and reporting

```bash
cd code-examples/typescript/01-swap-with-slippage
npm install && npm run dev
```

**Key Features:**
- Smart slippage adjustment (0.1% - 2.0%)
- Price impact analysis
- Failed transaction recovery
- Detailed swap analytics

---

#### 🏦 [02. Auto-Compound Yield](./code-examples/typescript/02-auto-compound/)
**Difficulty**: Intermediate | **Focus**: Yield Optimization  

Advanced yield farming automation with:
- ✅ **Multi-strategy management** (Conservative, Balanced, Aggressive)
- ✅ **Gas optimization** and batching
- ✅ **Notification system** (Discord, Telegram, Email)
- ✅ **Performance tracking** with detailed statistics
- ✅ **Risk management** with position limits

```bash
cd code-examples/typescript/02-auto-compound
npm install && npm run dev
```

**Key Features:**
- Automated harvest scheduling
- Multi-pool yield comparison
- Real-time APY tracking
- Risk-adjusted position sizing

---

#### 📊 [03. Impermanent Loss Calculator](./code-examples/typescript/03-impermanent-loss-calc/)
**Difficulty**: Intermediate | **Focus**: Risk Analysis

Comprehensive IL analysis toolkit:
- ✅ **AMM & DLMM calculations** with precise math
- ✅ **Fee compensation analysis** 
- ✅ **Real-time monitoring** with position tracking
- ✅ **Historical data analysis** and trends
- ✅ **Report generation** with actionable insights

```bash
cd code-examples/typescript/03-impermanent-loss-calc  
npm install && npm run dev
```

**Key Features:**
- IL prediction models
- Fee vs IL breakeven analysis  
- Position optimization suggestions
- Export to Excel/PDF reports

---

#### 🎯 [04. DLMM Range Orders](./code-examples/typescript/04-dlmm-range-orders/)
**Difficulty**: Advanced | **Focus**: Limit Orders

Sophisticated limit order implementation:
- ✅ **Range order placement** with precision targeting
- ✅ **Automated execution** when price targets hit
- ✅ **Position management** with partial fills
- ✅ **Fee optimization** across multiple bins
- ✅ **Market making strategies** for advanced users

```bash
cd code-examples/typescript/04-dlmm-range-orders
npm install && npm run dev
```

**Key Features:**
- Grid trading strategies
- Stop-loss integration
- Advanced order types (GTC, IOC, FOK)
- Portfolio-level risk management

---

#### 🛤️ [05. Multi-Hop Routing](./code-examples/typescript/05-multi-hop-routing/)  
**Difficulty**: Advanced | **Focus**: Routing & Arbitrage

Advanced routing algorithms with:
- ✅ **Path optimization** across multiple pools
- ✅ **Arbitrage detection** and execution
- ✅ **Route analysis** with cost/benefit modeling
- ✅ **Multi-pool execution** with atomic transactions
- ✅ **MEV protection** and sandwich attack prevention

```bash
cd code-examples/typescript/05-multi-hop-routing
npm install && npm run dev
```

**Key Features:**
- Cross-DEX routing
- Flash loan arbitrage
- Route simulation and testing
- Gas-optimized batch transactions

</div>

---

### 🦀 Rust Examples  

<div class="examples-grid">

#### ⚡ [01. High-Performance Swap](./code-examples/rust/01-basic-swap/)
**Difficulty**: Intermediate | **Focus**: Performance

Native Rust implementation with:
- ✅ **MEV protection strategies** with advanced techniques
- ✅ **Batch operations** for reduced transaction costs  
- ✅ **Price analysis tools** with statistical modeling
- ✅ **Connection pooling** for optimal RPC usage
- ✅ **Memory optimization** for high-frequency trading

```bash
cd code-examples/rust/01-basic-swap
cargo run -- swap --amount 1.5 --token-in SOL --token-out USDC
```

**Key Features:**
- Sub-100ms execution times
- Advanced mempool monitoring  
- Custom RPC client optimizations
- Zero-copy serialization

---

#### 🔄 [02. Auto-Compound Engine](./code-examples/rust/02-auto-compound/)
**Difficulty**: Advanced | **Focus**: System Architecture

Multi-threaded yield optimization:
- ✅ **Async execution engine** with worker pools
- ✅ **Multiple strategy support** with plugin architecture
- ✅ **Gas optimization** with intelligent batching
- ✅ **Real-time monitoring** with Prometheus metrics
- ✅ **Fault tolerance** with automatic recovery

```bash  
cd code-examples/rust/02-auto-compound
cargo run -- --config production.toml
```

**Key Features:**
- Tokio-based async runtime
- Custom strategy development
- Kubernetes deployment ready
- Comprehensive logging and metrics

---

#### 🧮 [03. Impermanent Loss Calculator](./code-examples/rust/03-impermanent-loss-calc/)
**Difficulty**: Intermediate | **Focus**: Mathematical Precision

High-precision IL calculations:
- ✅ **Mathematical precision** with fixed-point arithmetic
- ✅ **Historical analysis** with time-series database
- ✅ **Performance optimization** for large datasets
- ✅ **Report generation** with customizable formats
- ✅ **API endpoints** for integration

```bash
cd code-examples/rust/03-impermanent-loss-calc  
cargo run -- analyze --pool <POOL_ADDRESS> --days 30
```

**Key Features:**
- Decimal precision mathematics
- Efficient data structures
- REST API with OpenAPI docs
- Export to multiple formats

---

#### 🎯 [04. DLMM Range Orders](./code-examples/rust/04-dlmm-range-orders/)
**Difficulty**: Advanced | **Focus**: Trading Systems

Production trading system:
- ✅ **Low-latency execution** with custom networking
- ✅ **Advanced order management** with state machines  
- ✅ **Risk management** with real-time position monitoring
- ✅ **Market data processing** with high-frequency updates
- ✅ **Strategy backtesting** with historical simulation

```bash
cd code-examples/rust/04-dlmm-range-orders
cargo run --release -- trade --strategy grid --config strategies/grid.toml
```

**Key Features:**
- Microsecond-level latency
- Advanced order types
- Real-time risk monitoring
- Strategy performance analytics

---

#### 🌐 [05. Multi-Hop Routing](./code-examples/rust/05-multi-hop-routing/)
**Difficulty**: Expert | **Focus**: Graph Algorithms  

Advanced routing engine:
- ✅ **Graph-based pathfinding** with optimized algorithms
- ✅ **Concurrent execution** across multiple routes
- ✅ **Advanced arbitrage** with flash loan integration
- ✅ **Market impact modeling** with slippage prediction
- ✅ **Performance benchmarking** with detailed analytics

```bash
cd code-examples/rust/05-multi-hop-routing  
cargo run --release -- route --from SOL --to USDC --amount 100
```

**Key Features:**
- A* pathfinding algorithm
- Parallel route execution
- Advanced arbitrage detection  
- Comprehensive route analytics

</div>

---

## 🚀 Quick Start Commands

### Development Setup
```bash
# Clone repository
git clone https://github.com/UmutKorkmaz/saros-sdk-docs
cd saros-sdk-docs

# Install all dependencies
npm run install:all

# Start documentation server  
npm run dev
```

### TypeScript Examples
```bash  
# Run specific example
npm run dev:swap          # Swap with slippage
npm run dev:compound      # Auto-compound
npm run dev:il           # IL calculator
npm run dev:range        # Range orders  
npm run dev:routing      # Multi-hop routing
```

### Rust Examples
```bash
# Build all Rust examples
cd code-examples/rust && cargo build --release

# Run specific example
cd 01-basic-swap && cargo run -- --help
cd 02-auto-compound && cargo run -- --help  
cd 03-impermanent-loss-calc && cargo run -- --help
cd 04-dlmm-range-orders && cargo run -- --help
cd 05-multi-hop-routing && cargo run -- --help
```

---

## 📋 Example Categories

### By Difficulty Level
- **🟢 Beginner**: Basic swaps, simple integrations
- **🟡 Intermediate**: Yield farming, IL calculations, position management  
- **🔴 Advanced**: Range orders, multi-hop routing, MEV strategies
- **⚫ Expert**: Custom strategies, high-frequency trading, arbitrage

### By Use Case  
- **🔄 Trading**: Swaps, range orders, routing
- **🏦 Yield**: Auto-compounding, farming strategies
- **📊 Analytics**: IL calculation, performance tracking
- **⚡ Performance**: High-frequency, low-latency systems

### By Technology
- **⚡ TypeScript**: Web applications, integrations
- **🦀 Rust**: High-performance, system-level
- **🌐 Full-Stack**: End-to-end applications

---

## 🛠️ Development Tools

Each example includes:
- ✅ **Complete documentation** with setup instructions
- ✅ **Unit test coverage** with Jest/Cargo test  
- ✅ **Integration tests** with real Solana integration
- ✅ **Performance benchmarks** and profiling
- ✅ **Error handling** with comprehensive recovery
- ✅ **Configuration management** with environment support
- ✅ **Logging and monitoring** with structured output

## 🔗 Related Resources

- 📚 [API Reference](./api-reference/typescript-sdk.md) - Complete SDK documentation
- 🎯 [Tutorials](./tutorials/01-basic-swap.md) - Step-by-step guides  
- 🔧 [Guides](./guides/advanced-swap-operations.md) - Advanced techniques
- 🛠️ [Troubleshooting](./troubleshooting.md) - Common issues and solutions

---

**Ready to build the future of DeFi? Pick an example and start coding! 🚀**