# Prerequisites

Before diving into Saros SDK development, ensure your development environment is properly configured.

## System Requirements

### Node.js Environment (for TypeScript SDKs)
- **Node.js**: v16.0.0 or higher (recommended: v18+)
- **npm**: v8+ or **yarn**: v1.22+
- **TypeScript**: v4.5+ (for TypeScript projects)

```bash
# Check your versions
node --version  # Should be 16+
npm --version   # Should be 8+
```

### Rust Environment (for Rust SDK)
- **Rust**: 1.70.0 or higher
- **Cargo**: Latest version (comes with Rust)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Check version
rustc --version  # Should be 1.70+
cargo --version
```

## Solana Development Setup

### 1. Solana CLI Tools
```bash
# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/v1.17.0/install)"

# Add to PATH (add to your shell profile)
export PATH="/home/$(whoami)/.local/share/solana/install/active_release/bin:$PATH"

# Verify installation
solana --version  # Should be 1.16+
```

### 2. Configure Solana CLI
```bash
# Set cluster to devnet for development
solana config set --url https://api.devnet.solana.com

# Create or import a keypair
solana-keygen new --outfile ~/my-solana-wallet.json

# Set as default keypair
solana config set --keypair ~/my-solana-wallet.json

# Check configuration
solana config get
```

### 3. Get Devnet SOL
```bash
# Airdrop SOL for testing (devnet only)
solana airdrop 2

# Check balance
solana balance
```

## Wallet Setup

### Supported Wallets
- **Phantom** (Recommended for development)
- **Solflare** 
- **Backpack**
- **Any Wallet Adapter compatible wallet**

### Browser Extension Setup
1. Install [Phantom](https://phantom.app/) browser extension
2. Create or import a wallet
3. Switch network to Devnet for testing
4. Ensure you have some devnet SOL for transactions

## Development Tools (Optional)

### Code Editor
- **VS Code** with Solana extensions:
  - Solana Developer Tools
  - Rust Analyzer (for Rust development)

### Solana Explorer
- **Devnet Explorer**: [explorer.solana.com](https://explorer.solana.com/?cluster=devnet)
- **Mainnet Explorer**: [explorer.solana.com](https://explorer.solana.com/)

## Verification Checklist

Before proceeding, verify your setup:

```bash
# ✅ Node.js and npm/yarn
node --version && npm --version

# ✅ Rust and Cargo (if using Rust SDK)
rustc --version && cargo --version

# ✅ Solana CLI
solana --version

# ✅ Solana config
solana config get

# ✅ Wallet balance (devnet)
solana balance
```

## Next Steps

Once your environment is ready:
1. [📦 Install SDKs](./installation.md)
2. [⚙️ Configure Networks](./configuration.md)  
3. [🚀 Run First Transaction](./first-transaction.md)

## Troubleshooting

### Common Issues

**Node.js version too old**
```bash
# Update Node.js using nvm
nvm install 18
nvm use 18
```

**Solana CLI not found**
```bash
# Check if PATH includes Solana binaries
echo $PATH | grep solana

# Re-run install if needed
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
```

**Insufficient SOL balance**
```bash
# Request more devnet SOL
solana airdrop 2

# Note: Mainnet SOL must be purchased from exchanges
```

**RPC connection issues**
```bash
# Test connection to devnet
solana cluster-version

# Try different RPC if needed
solana config set --url https://api.devnet.solana.com
```

Need help? Join our [Discord community](https://discord.gg/saros) for support!