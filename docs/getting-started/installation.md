# Installation Guide

Install the Saros SDKs for your preferred programming language and use case.

## Installation Workflow

```mermaid
flowchart TD
    Start[Start Installation] --> Choice{Choose Platform}
    
    Choice -->|Web/React| Web[Frontend Setup]
    Choice -->|Node.js| Node[Backend Setup]
    Choice -->|Rust| Rust[Rust Setup]
    
    Web --> WebSDK[Install TypeScript SDK]
    Web --> WebWallet[Setup Wallet Adapter]
    Web --> WebTest[Test in Browser]
    
    Node --> NodeSDK[Install TypeScript SDK]
    Node --> NodeEnv[Setup Environment]
    Node --> NodeTest[Test API Calls]
    
    Rust --> RustSDK[Install Rust SDK]
    Rust --> RustDeps[Add Dependencies]
    Rust --> RustTest[Test Compilation]
    
    WebTest --> Verify[Verify Installation]
    NodeTest --> Verify
    RustTest --> Verify
    
    Verify --> Success[✅ Ready to Build!]
    
    style Success fill:#4caf50,color:#fff
    style WebSDK fill:#2196f3,color:#fff
    style NodeSDK fill:#2196f3,color:#fff
    style RustSDK fill:#ff9800,color:#fff
```

## Quick Start Decision Tree

```mermaid
graph TD
    A[I want to build with Saros] --> B{What are you building?}
    
    B -->|Web dApp| C[TypeScript SDK + DLMM SDK]
    B -->|Trading Bot| D{Performance Priority?}
    B -->|Mobile App| E[TypeScript SDK]
    B -->|Backend API| F[TypeScript SDK]
    
    D -->|High Performance| G[Rust SDK]
    D -->|Rapid Development| H[TypeScript SDK]
    
    C --> C1["npm install @saros-finance/sdk @saros-finance/dlmm-sdk"]
    E --> E1["npm install @saros-finance/sdk"]
    F --> F1["npm install @saros-finance/sdk"]
    H --> H1["npm install @saros-finance/sdk"]
    G --> G1["cargo add saros-dlmm-sdk"]
    
    style C1 fill:#e3f2fd
    style E1 fill:#e3f2fd
    style F1 fill:#e3f2fd
    style H1 fill:#e3f2fd
    style G1 fill:#fff3e0
```

## TypeScript/JavaScript SDKs

### @saros-finance/sdk (Main SDK)

The primary SDK for AMM operations, staking, and farming.

```bash
# Using npm
npm install @saros-finance/sdk

# Using yarn
yarn add @saros-finance/sdk

# Using pnpm
pnpm add @saros-finance/sdk
```

**Dependencies automatically installed:**
- `@solana/web3.js`
- `@solana/spl-token`
- `bn.js`
- Other Solana ecosystem packages

### @saros-finance/dlmm-sdk (DLMM SDK)

Specialized SDK for Dynamic Liquidity Market Maker features.

```bash
# Using npm
npm install @saros-finance/dlmm-sdk

# Using yarn
yarn add @saros-finance/dlmm-sdk

# Using pnpm
pnpm add @saros-finance/dlmm-sdk
```

### Install Both SDKs
```bash
# For comprehensive Saros integration
npm install @saros-finance/sdk @saros-finance/dlmm-sdk

# Additional recommended packages
npm install @solana/wallet-adapter-react @solana/wallet-adapter-wallets
```

## Rust SDK

### saros-dlmm-sdk-rs

Add to your `Cargo.toml`:

```toml
[dependencies]
saros-dlmm-sdk = { git = "https://github.com/saros-finance/saros-dlmm-sdk-rs" }

# Required Solana dependencies
solana-client = "1.16"
solana-sdk = "1.16"
anchor-client = "0.28"
```

Install from command line:
```bash
cargo add --git https://github.com/saros-finance/saros-dlmm-sdk-rs saros-dlmm-sdk
```

## Package Information

### Latest Versions
| Package | Version | Release Date |
|---------|---------|--------------|
| @saros-finance/sdk | 2.4.0 | Latest |
| @saros-finance/dlmm-sdk | 1.3.2 | Latest |
| saros-dlmm-sdk-rs | Main branch | Active development |

### Bundle Sizes
| Package | Minified | Gzipped |
|---------|----------|---------|
| @saros-finance/sdk | ~850KB | ~180KB |
| @saros-finance/dlmm-sdk | ~420KB | ~95KB |

## Framework-Specific Setup

### React Application
```bash
# Create React app with Saros
npx create-react-app my-saros-app --template typescript
cd my-saros-app

# Install Saros SDKs and wallet adapters
npm install @saros-finance/sdk @saros-finance/dlmm-sdk
npm install @solana/wallet-adapter-react @solana/wallet-adapter-react-ui
npm install @solana/wallet-adapter-wallets
```

### Next.js Application  
```bash
# Create Next.js app
npx create-next-app@latest my-saros-app --typescript
cd my-saros-app

# Install Saros SDKs
npm install @saros-finance/sdk @saros-finance/dlmm-sdk

# For client-side wallet integration
npm install @solana/wallet-adapter-react @solana/wallet-adapter-next
```

### Node.js Backend
```bash
# Install for backend/API usage
npm install @saros-finance/sdk @saros-finance/dlmm-sdk
npm install @solana/web3.js

# Optional: For advanced RPC management
npm install @solana/rpc-websockets
```

### Rust Project
```bash
# Initialize new Rust project
cargo new my-saros-project
cd my-saros-project

# Add dependencies via Cargo.toml or CLI
cargo add --git https://github.com/saros-finance/saros-dlmm-sdk-rs saros-dlmm-sdk
cargo add solana-client solana-sdk anchor-client
```

## Development Dependencies

### TypeScript Projects
```bash
# Recommended dev dependencies
npm install --save-dev @types/node typescript ts-node

# Testing (optional)
npm install --save-dev jest @types/jest ts-jest

# Build tools (optional) 
npm install --save-dev webpack webpack-cli
```

### ESLint & Prettier (Recommended)
```bash
npm install --save-dev eslint @typescript-eslint/parser @typescript-eslint/eslint-plugin
npm install --save-dev prettier eslint-config-prettier eslint-plugin-prettier
```

## Verification

### Test TypeScript SDK Installation
```typescript
// test-installation.ts
import { genConnectionSolana } from '@saros-finance/sdk';
import { LiquidityBookServices, MODE } from '@saros-finance/dlmm-sdk';

console.log('✅ TypeScript SDK imported successfully');

// Test connection
const connection = genConnectionSolana();
console.log('✅ Connection created');

// Test DLMM SDK
const dlmmService = new LiquidityBookServices({ mode: MODE.DEVNET });
console.log('✅ DLMM service created');

console.log('🚀 All SDKs installed and working!');
```

```bash
# Run test
npx ts-node test-installation.ts
```

### Test Rust SDK Installation
```rust
// src/main.rs
use saros_dlmm_sdk::prelude::*;

fn main() {
    println!("✅ Rust SDK imported successfully");
    println!("🚀 Ready to build with Saros!");
}
```

```bash
# Run test
cargo run
```

## IDE Setup

### VS Code Extensions (Recommended)
- **Solana Developer Tools**
- **Rust Analyzer** (for Rust)
- **TypeScript Importer**
- **Solidity** (if working with cross-chain)

### VS Code Settings
Create `.vscode/settings.json`:
```json
{
  "typescript.preferences.includePackageJsonAutoImports": "auto",
  "editor.formatOnSave": true,
  "editor.codeActionsOnSave": {
    "source.fixAll.eslint": true
  }
}
```

## Environment Configuration

### Create .env file
```bash
# .env
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_COMMITMENT=confirmed
WALLET_PRIVATE_KEY=your_base58_private_key_here
```

### TypeScript types (create types/env.d.ts)
```typescript
declare namespace NodeJS {
  interface ProcessEnv {
    SOLANA_RPC_URL: string;
    SOLANA_COMMITMENT: 'processed' | 'confirmed' | 'finalized';
    WALLET_PRIVATE_KEY: string;
  }
}
```

## Troubleshooting Installation

### Common Issues

**Module not found errors**
```bash
# Clear npm cache and reinstall
npm cache clean --force
rm -rf node_modules package-lock.json
npm install
```

**TypeScript compilation errors**
```bash
# Ensure TypeScript is installed
npm install -g typescript
# or locally
npm install --save-dev typescript
```

**Solana web3.js version conflicts**
```bash
# Check for version conflicts
npm list @solana/web3.js

# Force specific version if needed
npm install @solana/web3.js@1.78.0 --save --exact
```

**Rust build failures**
```bash
# Update Rust toolchain
rustup update

# Clear build cache
cargo clean
cargo build
```

**Buffer polyfill issues (webpack 5)**
```bash
# Install polyfills for browser builds
npm install --save-dev buffer process
```

Add to `webpack.config.js`:
```javascript
module.exports = {
  resolve: {
    fallback: {
      buffer: require.resolve('buffer'),
      process: require.resolve('process/browser'),
    },
  },
};
```

## Next Steps

With SDKs installed, continue to:
1. [⚙️ Configuration](./configuration.md) - Set up networks and RPC endpoints
2. [🚀 First Transaction](./first-transaction.md) - Run your first Saros operation
3. [📚 Core Concepts](../core-concepts/amm-vs-dlmm.md) - Understand Saros architecture

Need help? Check our [troubleshooting guide](../resources/troubleshooting.md) or join our [Discord](https://discord.gg/saros).