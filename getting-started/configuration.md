# Configuration Guide

Configure your Saros SDK setup for different networks, RPC endpoints, and development environments.

## Network Configuration

### Supported Networks

| Network | Description | Use Case |
|---------|-------------|----------|
| **Mainnet-beta** | Production Solana network | Live trading, production apps |
| **Devnet** | Development network | Testing, development |
| **Testnet** | Testing network | Integration testing |
| **Localnet** | Local validator | Local development |

## RPC Endpoint Configuration

### Public RPC Endpoints

#### Mainnet
```typescript
const MAINNET_RPC_ENDPOINTS = [
  'https://api.mainnet-beta.solana.com',
  'https://solana-api.projectserum.com',
  'https://rpc.ankr.com/solana'
];
```

#### Devnet  
```typescript
const DEVNET_RPC_ENDPOINTS = [
  'https://api.devnet.solana.com',
  'https://devnet.genesysgo.net',
  'https://rpc-devnet.aws.metaplex.com'
];
```

### Premium RPC Providers

For production applications, consider premium RPC providers:

- **Helius**: High-performance RPC with enhanced APIs
- **QuickNode**: Dedicated endpoints with guaranteed uptime  
- **Alchemy**: Enterprise-grade infrastructure
- **GenesysGo**: Solana-focused RPC provider

## TypeScript SDK Configuration

### Basic Setup

```typescript
import { Connection, PublicKey } from '@solana/web3.js';
import { genConnectionSolana } from '@saros-finance/sdk';

// Using the built-in connection helper
const connection = genConnectionSolana();

// Or create custom connection
const customConnection = new Connection(
  process.env.SOLANA_RPC_URL || 'https://api.devnet.solana.com',
  {
    commitment: 'confirmed',
    wsEndpoint: 'wss://api.devnet.solana.com/',
  }
);
```

### Advanced Connection Configuration

```typescript
import { Connection, ConnectionConfig } from '@solana/web3.js';

const connectionConfig: ConnectionConfig = {
  commitment: 'confirmed',
  confirmTransactionInitialTimeout: 60000,
  disableRetryOnRateLimit: false,
  httpHeaders: {
    'User-Agent': 'MyApp/1.0.0',
  },
  wsEndpoint: 'wss://api.devnet.solana.com/',
};

const connection = new Connection(
  'https://api.devnet.solana.com',
  connectionConfig
);
```

### Program Addresses

```typescript
// Saros program addresses
export const SAROS_PROGRAMS = {
  MAINNET: {
    AMM_PROGRAM: new PublicKey('SSwapUtytfBdBn1b9NUGG6foMVPtcWgpRU32HToDUZr'),
    DLMM_PROGRAM: new PublicKey('LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo'),
    STAKE_PROGRAM: new PublicKey('STAKEkKzbdeKkqzKpLkNQD3SUuLgshDKCD7U8duxAbB'),
    FARM_PROGRAM: new PublicKey('FARMREGUFBmTgmpwXTVNQ9X5eE2K2BzVFFtBZuG7kZP'),
  },
  DEVNET: {
    AMM_PROGRAM: new PublicKey('SSwapUtytfBdBn1b9NUGG6foMVPtcWgpRU32HToDUZr'),
    DLMM_PROGRAM: new PublicKey('LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo'),
    STAKE_PROGRAM: new PublicKey('STAKEkKzbdeKkqzKpLkNQD3SUuLgshDKCD7U8duxAbB'),
    FARM_PROGRAM: new PublicKey('FARMREGUFBmTgmpwXTVNQ9X5eE2K2BzVFFtBZuG7kZP'),
  }
};
```

## DLMM SDK Configuration

### Network Mode Setup

```typescript
import { LiquidityBookServices, MODE } from '@saros-finance/dlmm-sdk';

// Devnet configuration
const dlmmServiceDevnet = new LiquidityBookServices({
  mode: MODE.DEVNET,
});

// Mainnet configuration  
const dlmmServiceMainnet = new LiquidityBookServices({
  mode: MODE.MAINNET,
});

// Custom RPC configuration
const dlmmServiceCustom = new LiquidityBookServices({
  mode: MODE.MAINNET,
  rpcEndpoint: 'https://your-premium-rpc-endpoint.com',
});
```

### DLMM Configuration Options

```typescript
interface DLMMConfig {
  mode: MODE;
  rpcEndpoint?: string;
  commitment?: 'processed' | 'confirmed' | 'finalized';
  skipPreflight?: boolean;
  preflightCommitment?: 'processed' | 'confirmed' | 'finalized';
}

const dlmmService = new LiquidityBookServices({
  mode: MODE.MAINNET,
  rpcEndpoint: process.env.SOLANA_RPC_URL,
  commitment: 'confirmed',
  skipPreflight: false,
  preflightCommitment: 'confirmed',
});
```

## Rust SDK Configuration

### Cargo.toml Setup

```toml
[dependencies]
saros-dlmm-sdk = { git = "https://github.com/saros-xyz/saros-dlmm-sdk-rs" }
solana-client = "1.16"
solana-sdk = "1.16"
anchor-client = "0.28"
tokio = { version = "1.0", features = ["full"] }

[features]
default = []
devnet = []
mainnet = []
```

### Rust Configuration

```rust
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use std::str::FromStr;

// Network configuration
pub struct NetworkConfig {
    pub rpc_url: String,
    pub commitment: CommitmentConfig,
}

impl NetworkConfig {
    pub fn devnet() -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            commitment: CommitmentConfig::confirmed(),
        }
    }

    pub fn mainnet() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            commitment: CommitmentConfig::confirmed(),
        }
    }

    pub fn custom(rpc_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            commitment: CommitmentConfig::confirmed(),
        }
    }
}

// Create RPC client
pub fn create_rpc_client(config: &NetworkConfig) -> RpcClient {
    RpcClient::new_with_commitment(config.rpc_url.clone(), config.commitment)
}
```

## Environment Variables

### .env Configuration

```bash
# Network Configuration
NODE_ENV=development
SOLANA_NETWORK=devnet
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_WS_URL=wss://api.devnet.solana.com/

# Transaction Configuration
SOLANA_COMMITMENT=confirmed
TRANSACTION_TIMEOUT=60000
SKIP_PREFLIGHT=false

# Wallet Configuration (for backend/scripts)
WALLET_PRIVATE_KEY=your_base58_private_key_here
WALLET_KEYPAIR_PATH=./keypair.json

# RPC Configuration
RPC_POOL_SIZE=10
RPC_TIMEOUT=30000

# Development Settings
DEBUG_TRANSACTIONS=true
LOG_LEVEL=info

# Production Settings (mainnet)
# SOLANA_NETWORK=mainnet-beta
# SOLANA_RPC_URL=https://your-premium-rpc.com
# DEBUG_TRANSACTIONS=false
# LOG_LEVEL=warn
```

### Environment Validation

```typescript
// config/environment.ts
import { z } from 'zod';

const envSchema = z.object({
  NODE_ENV: z.enum(['development', 'production', 'test']).default('development'),
  SOLANA_NETWORK: z.enum(['devnet', 'testnet', 'mainnet-beta']).default('devnet'),
  SOLANA_RPC_URL: z.string().url(),
  SOLANA_COMMITMENT: z.enum(['processed', 'confirmed', 'finalized']).default('confirmed'),
  WALLET_PRIVATE_KEY: z.string().optional(),
  DEBUG_TRANSACTIONS: z.boolean().default(false),
});

export const env = envSchema.parse(process.env);

// Network helper
export function getNetworkConfig() {
  return {
    network: env.SOLANA_NETWORK,
    rpcUrl: env.SOLANA_RPC_URL,
    commitment: env.SOLANA_COMMITMENT,
    isMainnet: env.SOLANA_NETWORK === 'mainnet-beta',
    isDevnet: env.SOLANA_NETWORK === 'devnet',
  };
}
```

## Wallet Configuration

### Browser Wallet Integration

```typescript
import { WalletAdapterNetwork } from '@solana/wallet-adapter-base';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
  BackpackWalletAdapter,
} from '@solana/wallet-adapter-wallets';
import { Connection, clusterApiUrl } from '@solana/web3.js';

export function WalletConfig({ children }: { children: React.ReactNode }) {
  const network = WalletAdapterNetwork.Devnet;
  const endpoint = clusterApiUrl(network);
  
  const wallets = [
    new PhantomWalletAdapter(),
    new SolflareWalletAdapter({ network }),
    new BackpackWalletAdapter(),
  ];

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          {children}
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
```

### Backend Wallet (Keypair) Setup

```typescript
import { Keypair } from '@solana/web3.js';
import bs58 from 'bs58';
import fs from 'fs';

// Load from private key string
export function loadWalletFromPrivateKey(privateKey: string): Keypair {
  try {
    return Keypair.fromSecretKey(bs58.decode(privateKey));
  } catch (error) {
    throw new Error('Invalid private key format');
  }
}

// Load from JSON file
export function loadWalletFromFile(filepath: string): Keypair {
  try {
    const secretKey = JSON.parse(fs.readFileSync(filepath, 'utf8'));
    return Keypair.fromSecretKey(new Uint8Array(secretKey));
  } catch (error) {
    throw new Error(`Failed to load wallet from ${filepath}`);
  }
}

// Generate new wallet
export function generateWallet(): { keypair: Keypair; privateKey: string } {
  const keypair = Keypair.generate();
  const privateKey = bs58.encode(keypair.secretKey);
  
  return { keypair, privateKey };
}
```

## Configuration Examples

### Development Configuration

```typescript
// config/development.ts
export const developmentConfig = {
  network: 'devnet' as const,
  rpcUrl: 'https://api.devnet.solana.com',
  commitment: 'confirmed' as const,
  programs: SAROS_PROGRAMS.DEVNET,
  settings: {
    skipPreflight: false,
    preflightCommitment: 'confirmed' as const,
    maxRetries: 3,
    timeout: 60000,
  },
  debug: {
    logTransactions: true,
    logErrors: true,
    verboseLogging: true,
  }
};
```

### Production Configuration

```typescript
// config/production.ts  
export const productionConfig = {
  network: 'mainnet-beta' as const,
  rpcUrl: process.env.PREMIUM_RPC_URL!,
  commitment: 'confirmed' as const,
  programs: SAROS_PROGRAMS.MAINNET,
  settings: {
    skipPreflight: false,
    preflightCommitment: 'confirmed' as const,
    maxRetries: 5,
    timeout: 90000,
  },
  debug: {
    logTransactions: false,
    logErrors: true,
    verboseLogging: false,
  }
};
```

## Testing Configuration

### Unit Test Setup

```typescript
// config/test.ts
import { Connection } from '@solana/web3.js';

export const testConfig = {
  network: 'devnet' as const,
  rpcUrl: 'https://api.devnet.solana.com',
  connection: new Connection('https://api.devnet.solana.com', 'confirmed'),
  timeout: 30000,
  mockWallet: true,
};

// Jest setup
export function setupTestEnvironment() {
  process.env.NODE_ENV = 'test';
  process.env.SOLANA_NETWORK = 'devnet';
  process.env.SOLANA_RPC_URL = testConfig.rpcUrl;
}
```

## Next Steps

With configuration complete:
1. [🚀 First Transaction](./first-transaction.md) - Run your first Saros operation
2. [📚 Core Concepts](../core-concepts/amm-vs-dlmm.md) - Understand Saros architecture  
3. [📖 SDK Guides](../sdk-guides/typescript-sdk/swap-operations.md) - Deep dive into SDK usage

## Troubleshooting

### Common Configuration Issues

**RPC connection timeouts**
```typescript
// Increase timeout values
const connection = new Connection(rpcUrl, {
  commitment: 'confirmed',
  confirmTransactionInitialTimeout: 120000, // 2 minutes
});
```

**WebSocket connection issues**
```typescript  
// Use HTTP-only connection if WS fails
const connection = new Connection(rpcUrl, {
  commitment: 'confirmed',
  wsEndpoint: undefined, // Disable WebSocket
});
```

**Program address errors**
```typescript
// Verify program addresses for your network
console.log('AMM Program:', SAROS_PROGRAMS.DEVNET.AMM_PROGRAM.toString());
console.log('DLMM Program:', SAROS_PROGRAMS.DEVNET.DLMM_PROGRAM.toString());
```

Need help? Check our [troubleshooting guide](../resources/troubleshooting.md) or join our [Discord](https://discord.gg/saros).