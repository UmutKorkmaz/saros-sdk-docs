# DLMM Range Orders Trading System

An advanced automated trading system for Saros DLMM (Dynamic Liquidity Market Maker) with support for sophisticated order types, real-time monitoring, and risk management.

## Features

### Order Types
- **Limit Orders**: Buy/sell at specific bin prices with precise execution
- **DCA Ladder**: Dollar-cost averaging with multiple distribution strategies
- **Grid Trading**: Automated buy/sell orders across price ranges
- **Take Profit/Stop Loss**: Risk management for existing positions
- **Range Orders**: Multiple orders across bin ranges with smart execution

### Trading Strategies
- **Uniform Distribution**: Equal amounts across all price levels
- **Weighted Distribution**: More allocation at better prices
- **Fibonacci Distribution**: Mathematical distribution based on Fibonacci sequence
- **Custom Distribution**: User-defined allocation patterns
- **Grid Trading**: Automated market making with rebalancing

### Advanced Features
- **Real-time Monitoring**: Live price tracking and execution signals
- **Smart Execution**: Optimal timing based on liquidity and market conditions
- **Risk Management**: Position size limits, exposure controls, and stop losses
- **Gas Optimization**: Dynamic gas pricing based on network conditions
- **MEV Protection**: Protection against maximal extractable value attacks
- **Slippage Protection**: Automatic slippage detection and prevention

## Architecture

The system consists of four main components:

### 1. Range Order Manager (`range_order_manager.rs`)
- Core order management and strategy creation
- Risk validation and exposure tracking
- Order lifecycle management
- Strategy execution coordination

### 2. Order Monitor (`order_monitor.rs`)
- Real-time market data collection
- Price history and technical analysis
- Execution signal generation
- Stop loss monitoring with trailing support

### 3. Execution Engine (`execution_engine.rs`)
- Automated order execution
- Queue management with priority handling
- Retry logic and error handling
- Gas optimization and MEV protection

### 4. Bin Calculator (`bin_calculations.rs`)
- DLMM bin math and price calculations
- Distribution algorithms for strategies
- Slippage and price impact calculations
- Grid level generation

## Installation

1. Clone the repository:
```bash
git clone https://github.com/saros-finance/saros-sdk-docs.git
cd saros-sdk-docs/code-examples/rust/04-dlmm-range-orders
```

2. Install dependencies:
```bash
cargo build --release
```

3. Set up configuration:
```bash
cargo run -- config init --rpc-url https://api.mainnet-beta.solana.com
```

## Configuration

Create a `config.toml` file or use the CLI to initialize:

```toml
[rpc]
url = "https://api.mainnet-beta.solana.com"
timeout_secs = 30
max_retries = 3

[trading]
wallet_path = "~/.config/solana/id.json"
default_slippage_bps = 100
bin_step = 20
base_price = "100.0"

[risk]
max_position_size = "1000.0"
max_total_exposure = "10000.0"
max_active_orders = 50
max_slippage_bps = 500
global_stop_loss_pct = "5.0"

[monitoring]
polling_interval_ms = 1000
enable_websocket = false
price_change_threshold_pct = "0.5"

[execution]
max_concurrent_executions = 10
execution_timeout_secs = 30
gas_strategy = "dynamic"
enable_slippage_protection = true

[notifications]
enable_notifications = true
webhook_url = "https://your-webhook-url.com"
```

## Usage

### Basic Commands

#### Configuration Management
```bash
# Initialize configuration
cargo run -- config init

# Show current configuration
cargo run -- config show

# Validate configuration
cargo run -- config validate
```

#### Order Management
```bash
# Create a limit buy order
cargo run -- order limit \
  --pool 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
  --side buy \
  --price 95.50 \
  --amount 100

# Create a limit sell order
cargo run -- order limit \
  --pool 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
  --side sell \
  --price 105.25 \
  --amount 100 \
  --expires 2024-12-31T23:59:59Z

# Create take profit order
cargo run -- order take-profit \
  --position 5KJz8jVz9t3rFyQJsN8dR6R5u8Td4w2nP3qL9vW1xM2Y \
  --price 110.00 \
  --percentage 100

# Create stop loss with trailing
cargo run -- order stop-loss \
  --position 5KJz8jVz9t3rFyQJsN8dR6R5u8Td4w2nP3qL9vW1xM2Y \
  --price 90.00 \
  --trailing 2.5 \
  --percentage 100

# List all orders
cargo run -- order list

# Show order details
cargo run -- order show <order-id>

# Cancel order
cargo run -- order cancel <order-id>
```

#### Strategy Management
```bash
# Create DCA ladder strategy
cargo run -- strategy dca-ladder \
  --pool 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
  --amount 1000 \
  --orders 10 \
  --start-price 90.00 \
  --end-price 100.00 \
  --distribution weighted \
  --bias 1.5

# Create grid trading strategy
cargo run -- strategy grid \
  --pool 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
  --center-price 100.00 \
  --spacing 50 \
  --buy-orders 5 \
  --sell-orders 5 \
  --order-amount 50

# List strategies
cargo run -- strategy list

# Show strategy details
cargo run -- strategy show <strategy-id>

# Cancel strategy
cargo run -- strategy cancel <strategy-id>
```

#### Monitoring
```bash
# Start real-time monitoring
cargo run -- monitor start --interval 1000 --websocket

# Show market data
cargo run -- monitor market 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM

# Show price history
cargo run -- monitor history 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM --period 1h
```

#### System Status
```bash
# Show overall system status
cargo run -- status
```

### Advanced Usage Examples

#### DCA Strategy with Fibonacci Distribution
```bash
cargo run -- strategy dca-ladder \
  --pool 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
  --amount 5000 \
  --orders 20 \
  --start-price 80.00 \
  --end-price 120.00 \
  --distribution fibonacci
```

#### Grid Trading with Tight Spreads
```bash
cargo run -- strategy grid \
  --pool 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
  --center-price 100.00 \
  --spacing 25 \
  --buy-orders 10 \
  --sell-orders 10 \
  --order-amount 25
```

#### Risk Management Setup
```bash
# Set global stop loss at 5%
export GLOBAL_STOP_LOSS=5.0

# Run with strict risk controls
cargo run -- --config strict-config.toml order limit \
  --pool 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
  --side buy \
  --price 95.00 \
  --amount 50
```

## Strategy Explanations

### DCA Ladder Strategy

Dollar-Cost Averaging (DCA) with multiple orders across a price range:

**Use Case**: Accumulating positions during market downturns
**Risk Level**: Medium
**Capital Efficiency**: High

**Distribution Types**:
- **Uniform**: Equal amounts at each price level
- **Weighted**: More allocation at lower prices (better entries)
- **Fibonacci**: Mathematical distribution favoring better prices
- **Custom**: User-defined allocation patterns

**Example**: $1000 DCA from $90-$100 with 10 orders using weighted distribution (bias 1.5):
- More allocation at lower bins (better prices)
- Automatic execution when price reaches each bin
- Position building over time

### Grid Trading Strategy

Automated market making with buy and sell orders:

**Use Case**: Profiting from price oscillations in ranging markets
**Risk Level**: Medium-High
**Capital Efficiency**: Very High

**Features**:
- Symmetrical buy/sell orders around center price
- Automatic rebalancing based on fills
- Take profit levels for each trade
- Grid spacing optimization

**Example**: Grid around $100 with 0.5% spacing:
- Buy orders: $99.50, $99.00, $98.50, $98.00, $97.50
- Sell orders: $100.50, $101.00, $101.50, $102.00, $102.50
- Each fill triggers opposite side order

### Range Orders with Stop Loss

Position management with automatic risk controls:

**Use Case**: Protecting profits and limiting losses
**Risk Level**: Low-Medium
**Capital Efficiency**: Medium

**Features**:
- Trailing stops that adjust with favorable price moves
- Multiple stop levels for partial position closing
- Time-based stop loss adjustments
- Integration with existing positions

## DLMM-Specific Constraints

### Bin-Based Trading
- Orders must be placed at specific bin IDs
- Price precision is limited to bin step size
- Liquidity is concentrated in discrete bins

### Order Placement Rules
- **Buy orders**: Must be placed below current active bin
- **Sell orders**: Must be placed above current active bin
- **No market orders**: All orders are limit orders by nature

### Liquidity Considerations
- Execution depends on bin liquidity availability
- Large orders may need to be split across multiple bins
- Price impact calculations are bin-specific

### Gas Optimization
- Bin operations have different gas costs
- Batch operations can reduce overall gas usage
- Network congestion affects execution timing

## Risk Management

### Position Size Controls
```rust
// Maximum single order size
max_position_size: 1000.0

// Maximum total exposure across all orders
max_total_exposure: 10000.0

// Maximum number of active orders
max_active_orders: 50
```

### Slippage Protection
- Real-time slippage calculation
- Automatic order cancellation on excessive slippage
- Bin-specific slippage thresholds

### Stop Loss Implementation
Since DLMM doesn't support native stop losses, the system implements:
- **Monitoring Approach**: Continuous price monitoring with trigger execution
- **Trailing Stops**: Dynamic stop price adjustment
- **Multiple Stop Levels**: Partial position closing at different levels

### MEV Protection
- Transaction timing randomization
- Gas price optimization
- Front-running detection and prevention

## Performance Optimization

### Concurrent Execution
- Multiple orders can be executed simultaneously
- Semaphore-based concurrency control
- Priority-based queue management

### Memory Management
- Bounded data structures for price history
- Efficient order book representation
- Automatic cleanup of old data

### Network Optimization
- Connection pooling for RPC calls
- Batch operations where possible
- Retry logic with exponential backoff

## Monitoring and Alerting

### Real-time Metrics
- Order execution rate and success percentage
- Average execution time and gas costs
- Slippage and price impact tracking
- Strategy performance analytics

### Notification System
- Webhook integration for order events
- Discord/Slack notifications
- Email alerts for critical events
- Custom notification filters

### Logging
- Structured logging with multiple levels
- Execution traces for debugging
- Performance metrics collection
- Error tracking and analysis

## Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
cargo test --test integration
```

### Dry Run Mode
```bash
cargo run -- --dry-run order limit \
  --pool 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
  --side buy \
  --price 95.00 \
  --amount 100
```

## Environment Variables

Create a `.env` file:
```bash
# Required
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
WALLET_PATH=~/.config/solana/id.json

# Optional
LOG_LEVEL=info
MAX_CONCURRENT_EXECUTIONS=10
ENABLE_WEBSOCKET=false
WEBHOOK_URL=https://your-webhook.com/notifications
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/your-webhook

# Risk Management
MAX_POSITION_SIZE=1000.0
MAX_TOTAL_EXPOSURE=10000.0
GLOBAL_STOP_LOSS_PCT=5.0
```

## Troubleshooting

### Common Issues

1. **Orders not executing**
   - Check bin liquidity availability
   - Verify price hasn't moved past order
   - Ensure wallet has sufficient balance

2. **High slippage**
   - Reduce order size
   - Split large orders across multiple bins
   - Check market volatility

3. **Connection errors**
   - Verify RPC endpoint is working
   - Check network connectivity
   - Increase timeout settings

4. **Strategy not working**
   - Verify strategy parameters are valid
   - Check order placement constraints
   - Monitor execution logs

### Debug Mode
```bash
RUST_LOG=debug cargo run -- --verbose monitor start
```

### Log Analysis
```bash
# View execution logs
tail -f logs/execution.log

# Filter error logs
grep "ERROR" logs/system.log

# Monitor performance
grep "execution_time" logs/performance.log
```

## Security Considerations

### Wallet Security
- Use hardware wallets for production
- Implement key rotation policies
- Separate trading and custody keys

### Network Security
- Use secure RPC endpoints
- Implement request signing
- Monitor for unusual activity

### Code Security
- Regular dependency updates
- Security audit of critical paths
- Input validation and sanitization

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

### Development Setup
```bash
# Install development dependencies
cargo install cargo-watch cargo-audit

# Run tests continuously
cargo watch -x test

# Security audit
cargo audit
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Disclaimer

This software is for educational purposes only. Trading cryptocurrencies involves substantial risk of loss and is not suitable for all investors. Past performance is not indicative of future results. Always do your own research and consider your financial situation before trading.

## Support

- Documentation: [Saros SDK Docs](https://docs.saros.finance)
- Discord: [Saros Community](https://discord.gg/saros)
- Issues: [GitHub Issues](https://github.com/saros-finance/saros-sdk-docs/issues)
- Email: support@saros.finance