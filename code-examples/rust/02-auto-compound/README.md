# Saros Auto-Compound Yield Farming (Rust)

High-performance Rust implementation of automated yield farming with compound interest strategies using Saros Finance. Built with async Rust patterns, tokio scheduling, and production-ready error handling.

## Features

- 🚀 **High-Performance Async Runtime** - Built on Tokio with efficient concurrent execution
- 🔄 **Automated Compound Strategies** - LP, Staking, and Farming compound operations
- ⏰ **Advanced Scheduling** - Cron-based scheduling with dynamic frequency adjustment
- 🛡️ **Gas Optimization** - Intelligent gas price monitoring and cost optimization
- 📊 **Comprehensive Analytics** - Real-time performance tracking and reporting
- 🔔 **Multi-Channel Notifications** - Webhook, Discord, Slack integration
- 🎯 **Position Monitoring** - Continuous position tracking with change detection
- 📈 **Performance Metrics** - APY tracking, efficiency scoring, and trend analysis
- 🔧 **Auto-tuning** - Self-adjusting intervals based on performance
- 💪 **Production Ready** - Comprehensive error handling, retry logic, graceful shutdown

## Prerequisites

- Rust 1.70+ with Cargo
- Solana CLI tools
- Funded wallet with SOL for transaction fees
- LP tokens, staked positions, or farm positions

## Installation

```bash
# Navigate to the Rust auto-compound example
cd code-examples/rust/02-auto-compound

# Build the project
cargo build --release

# Run tests
cargo test
```

## Configuration

Create a `.env` file based on the provided template:

```bash
cp .env.example .env
```

### Environment Variables

```env
# Network Configuration
SOLANA_NETWORK=devnet
SOLANA_RPC_URL=https://api.devnet.solana.com

# Wallet Configuration (base58 or JSON array format)
WALLET_PRIVATE_KEY=your_base58_private_key_here

# Primary Strategy Configuration
STRATEGY_TYPE=LP           # Options: LP, STAKING, FARMING
POOL_ADDRESS=pool_address_here
COMPOUND_INTERVAL=3600000  # 1 hour in milliseconds
MIN_REWARD_THRESHOLD=1.0   # Minimum rewards before compounding
REINVEST_PERCENTAGE=100    # Percentage of rewards to reinvest (0-100)
MAX_SLIPPAGE=1.0          # Maximum slippage for trades (%)

# Gas Optimization
MAX_GAS_PRICE=0.01        # Maximum SOL to spend on gas

# Multiple Pool Support (optional)
POOL_ADDRESS_1=second_pool_address
STRATEGY_TYPE_1=STAKING
COMPOUND_INTERVAL_1=7200000
MIN_REWARD_THRESHOLD_1=2.0
REINVEST_PERCENTAGE_1=80

# Notifications
ENABLE_NOTIFICATIONS=true
WEBHOOK_URL=https://your-webhook-url.com/webhook
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/...

# Logging Level
RUST_LOG=info
```

## Usage

### Basic Usage

Run the auto-compounder with default configuration:

```bash
# Development mode with logging
RUST_LOG=info cargo run

# Release mode for production
./target/release/saros-auto-compound
```

### Advanced Usage

```rust
use saros_auto_compound::{AutoCompounder, CompoundStrategyConfig, StrategyType};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize auto-compounder
    let config = AutoCompoundConfig {
        rpc_url: "https://api.devnet.solana.com".to_string(),
        private_key: "your_private_key".to_string(),
        network: "devnet".to_string(),
        max_gas_price: 0.01,
        enable_notifications: true,
        webhook_url: Some("https://your-webhook.com".to_string()),
    };

    let mut compounder = AutoCompounder::new(config).await?;

    // Configure LP compound strategy
    let lp_strategy = CompoundStrategyConfig {
        pool_address: Pubkey::from_str("pool_address_here")?,
        strategy_type: StrategyType::LP,
        interval_ms: 3600000, // 1 hour
        min_reward_threshold: 1.0,
        reinvest_percentage: 100,
        max_slippage: Some(1.0),
        emergency_withdraw: false,
    };

    // Start auto-compounding
    let result = compounder.start_strategy(lp_strategy).await?;
    println!("Strategy started: {:?}", result);

    // Manual compound trigger
    let manual_result = compounder.compound_now(pool_address).await?;
    println!("Manual compound: {:?}", manual_result);

    // Get statistics
    let stats = compounder.get_global_statistics().await?;
    println!("Global stats: {:?}", stats);

    Ok(())
}
```

## Project Structure

```
02-auto-compound/
├── src/
│   ├── main.rs                 # Entry point and configuration loading
│   ├── auto_compounder.rs      # Core auto-compounder implementation
│   ├── compound_strategy.rs    # Individual strategy execution logic
│   ├── gas_optimizer.rs        # Gas price optimization and analysis
│   ├── notification_service.rs # Multi-channel notification system
│   ├── position_monitor.rs     # Position tracking and change detection
│   ├── reward_harvester.rs     # Specialized reward harvesting logic
│   ├── scheduler.rs            # Advanced cron scheduling with auto-tuning
│   ├── statistics.rs           # Performance analytics and reporting
│   └── types.rs               # Type definitions and data structures
├── Cargo.toml                  # Dependencies and metadata
├── README.md                   # This file
└── .env.example               # Environment variable template
```

## Strategy Types

### 1. LP Token Compounding
- Harvests trading fees and LP rewards
- Automatically reinvests into LP position
- Handles rebalancing when needed
- Optimizes for maximum capital efficiency

### 2. Staking Compound
- Claims staking rewards automatically
- Re-stakes rewards to compound returns
- Maximizes staking APY through frequent compounding
- Handles validator selection optimization

### 3. Farm Compounding
- Harvests farm token rewards
- Swaps rewards to optimal tokens
- Adds liquidity and re-deposits to farm
- Manages multiple reward tokens

## Gas Optimization

The system includes sophisticated gas optimization:

- **Real-time Gas Monitoring** - Tracks network congestion and gas prices
- **Cost-Benefit Analysis** - Only compounds when rewards exceed gas costs
- **Dynamic Thresholds** - Adjusts minimum thresholds based on gas prices
- **Batch Operations** - Combines multiple operations when beneficial
- **Priority Fee Optimization** - Uses optimal priority fees for execution
- **Network Congestion Detection** - Delays operations during high congestion

## Performance Analytics

### Real-time Metrics
```rust
let stats = compounder.get_global_statistics().await?;
println!("Total compounds: {}", stats.total_compounds);
println!("Success rate: {:.2}%", stats.success_rate);
println!("Net profit: {:.6} SOL", stats.net_profit);
println!("Average APY boost: {:.2}%", stats.average_apy_boost);
```

### Pool-specific Analytics
```rust
let pool_stats = compounder.get_pool_statistics(pool_address).await?;
if let Some(stats) = pool_stats {
    println!("Pool compounds: {}", stats.compounds);
    println!("Total harvested: {:.6}", stats.total_harvested);
    println!("Average reward: {:.6}", stats.average_reward);
}
```

### Performance Reports
```rust
use saros_auto_compound::StatisticsManager;

let mut stats_manager = StatisticsManager::new();
let report = stats_manager.generate_performance_report().await;
println!("Performance Report: {:#?}", report);
```

## Notification System

### Webhook Notifications
```json
{
  "event_type": "COMPOUND_SUCCESS",
  "pool_address": "pool_address_here",
  "message": "Compound successful: harvested 5.123, reinvested 5.123",
  "data": {
    "rewards_harvested": 5.123,
    "amount_reinvested": 5.123,
    "new_position": 1000.456,
    "gas_used": 0.000005
  },
  "timestamp": "2024-01-01T12:00:00Z",
  "source": "saros-auto-compound-bot"
}
```

### Discord Integration
Set `DISCORD_WEBHOOK_URL` for rich Discord notifications with embeds.

### Slack Integration
Set `SLACK_WEBHOOK_URL` for formatted Slack notifications.

## Advanced Features

### Dynamic Interval Adjustment
The system automatically adjusts compound intervals based on:
- Success rate trends
- Gas price patterns
- Reward accumulation rates
- Market volatility

### Position Change Detection
```rust
let changes = position_monitor.check_position_changes(pool_address).await?;
if let Some(change) = changes {
    println!("Position changed by: {:.6} LP tokens", change.lp_token_change);
}
```

### Emergency Stop Mechanism
```rust
// Set custom emergency stop logic
compounder.set_emergency_stop(|| async {
    // Custom logic to determine if operations should stop
    check_market_conditions().await
}).await;
```

### Batch Operations
```rust
// Harvest from multiple pools simultaneously
let results = reward_harvester.batch_harvest(
    vec![pool1, pool2, pool3],
    vec![StrategyType::LP, StrategyType::Staking, StrategyType::Farming]
).await?;
```

## Monitoring & Logging

### Structured Logging
```bash
# Set logging level
export RUST_LOG=saros_auto_compound=debug,info

# View logs with timestamps
cargo run 2>&1 | grep -E "(INFO|WARN|ERROR)"
```

### Performance Monitoring
```rust
// Get dashboard metrics
let metrics = stats_manager.get_dashboard_metrics().await;
println!("Active pools: {}/{}", metrics.active_pools, metrics.total_pools);
println!("Average efficiency: {:.1}%", metrics.average_efficiency);
```

## Error Handling & Recovery

The system includes comprehensive error handling:

- **Exponential Backoff** - Automatic retry with increasing delays
- **Circuit Breaker** - Prevents cascading failures
- **Graceful Degradation** - Continues with remaining operations
- **Transaction Confirmation** - Ensures all operations complete
- **Recovery Mechanisms** - Automatic recovery from temporary failures

## Security Considerations

- ✅ **Private Key Security** - Keys never logged or exposed
- ✅ **Slippage Protection** - Configurable slippage limits on all trades
- ✅ **Gas Price Limits** - Maximum gas price enforcement
- ✅ **Position Validation** - Continuous position monitoring
- ✅ **Emergency Stops** - Immediate shutdown capability
- ✅ **Secure Defaults** - Conservative default settings

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test statistics

# Run integration tests
cargo test --test integration
```

## Performance Tuning

### Optimal Configurations

**High-Frequency Trading (APY > 100%)**
```env
COMPOUND_INTERVAL=900000      # 15 minutes
MIN_REWARD_THRESHOLD=0.1
REINVEST_PERCENTAGE=100
```

**Stable Yield Farming (APY 10-50%)**
```env
COMPOUND_INTERVAL=14400000    # 4 hours
MIN_REWARD_THRESHOLD=2.0
REINVEST_PERCENTAGE=100
```

**Conservative Strategy (APY < 20%)**
```env
COMPOUND_INTERVAL=86400000    # 24 hours
MIN_REWARD_THRESHOLD=5.0
REINVEST_PERCENTAGE=80
```

### Memory and CPU Optimization
- Built with `--release` flag for optimal performance
- Uses memory-efficient data structures (DashMap, etc.)
- Minimal memory allocation in hot paths
- Efficient async task management

## Production Deployment

### Docker Deployment
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/saros-auto-compound /usr/local/bin/
CMD ["saros-auto-compound"]
```

### Systemd Service
```ini
[Unit]
Description=Saros Auto-Compound Bot
After=network.target

[Service]
Type=simple
User=saros
ExecStart=/usr/local/bin/saros-auto-compound
Restart=always
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

### Monitoring Setup
- Use `RUST_LOG=info` for production logging
- Monitor process health with systemd
- Set up log rotation for long-term operation
- Configure alerting for critical failures

## Troubleshooting

### Common Issues

**"Gas price too high"**
- Current network gas exceeds `MAX_GAS_PRICE` setting
- Solution: Increase limit or wait for better conditions

**"Rewards below threshold"**
- Not enough rewards accumulated yet
- Solution: Wait longer or reduce `MIN_REWARD_THRESHOLD`

**"Insufficient SOL for fees"**
- Wallet needs more SOL for transaction fees
- Solution: Add SOL to wallet

**"Pool not found"**
- Invalid `POOL_ADDRESS` in configuration
- Solution: Verify pool address is correct

### Debug Mode
```bash
RUST_LOG=debug cargo run
```

### Health Checks
```bash
# Check if process is running
ps aux | grep saros-auto-compound

# Check recent logs
journalctl -u saros-auto-compound -f

# Check wallet balance
solana balance --keypair your_wallet.json
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## Support

- [Saros Documentation](https://docs.saros.finance)
- [Discord Community](https://discord.gg/saros)
- [GitHub Issues](https://github.com/saros-finance/sdk-examples)

## License

MIT License - see LICENSE file for details.