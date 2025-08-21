# Troubleshooting Guide & FAQ

Comprehensive guide to solving common issues when integrating with Saros Finance SDKs.

## Quick Fixes

| Problem | Solution |
|---------|----------|
| Transaction fails with "insufficient funds" | Ensure wallet has SOL for fees (minimum 0.01 SOL) |
| "Slippage tolerance exceeded" | Increase slippage to 1-3% for volatile pairs |
| "Cannot find module" | Run `npm install` or check package.json |
| RPC rate limit | Use dedicated RPC endpoint (Helius, QuickNode) |
| Transaction timeout | Increase confirmation timeout or retry |
| DLMM position out of range | Rebalance or wait for price to return |

---

## Common Issues & Solutions

### Installation Issues

#### Problem: Module not found after installation
```bash
Error: Cannot find module '@saros-finance/sdk'
```

**Solution:**
```bash
# Clear npm cache
npm cache clean --force

# Remove node_modules
rm -rf node_modules package-lock.json

# Reinstall
npm install

# For specific version
npm install @saros-finance/sdk@2.4.0
```

#### Problem: TypeScript errors after installation
```typescript
// Error: Cannot find type definitions
```

**Solution:**
```bash
# Install required types
npm install --save-dev @types/node @solana/web3.js

# Update tsconfig.json
{
  "compilerOptions": {
    "moduleResolution": "node",
    "esModuleInterop": true,
    "skipLibCheck": true
  }
}
```

#### Problem: Version conflicts
```bash
npm ERR! peer dep missing: @solana/web3.js@^1.87.0
```

**Solution:**
```bash
# Install specific versions
npm install @solana/web3.js@1.87.6 @saros-finance/sdk@2.4.0

# Or force install
npm install --force
```

---

### Transaction Errors

#### Problem: Transaction simulation failed
```javascript
Error: Transaction simulation failed: Blockhash not found
```

**Solution:**
```javascript
// Use fresh blockhash
const { blockhash } = await connection.getLatestBlockhash('confirmed');
transaction.recentBlockhash = blockhash;

// Retry with new blockhash
for (let i = 0; i < 3; i++) {
  try {
    const { blockhash } = await connection.getLatestBlockhash();
    transaction.recentBlockhash = blockhash;
    const sig = await sendAndConfirmTransaction(connection, transaction, signers);
    break;
  } catch (error) {
    if (i === 2) throw error;
    await new Promise(resolve => setTimeout(resolve, 1000));
  }
}
```

#### Problem: Slippage exceeded
```javascript
Error: Slippage tolerance exceeded. Expected 100 USDC, got 95 USDC
```

**Solution:**
```javascript
// Dynamic slippage based on volatility
function calculateDynamicSlippage(volatility, tradeSize, poolLiquidity) {
  let baseSlippage = 0.5; // 0.5%
  
  // Adjust for volatility
  if (volatility > 50) baseSlippage = 2.0;
  else if (volatility > 30) baseSlippage = 1.0;
  
  // Adjust for trade size impact
  const sizeImpact = (tradeSize / poolLiquidity) * 100;
  baseSlippage += sizeImpact;
  
  return Math.min(baseSlippage, 5.0); // Cap at 5%
}
```

#### Problem: Insufficient SOL for fees
```javascript
Error: Attempt to debit an account but found no record of a prior credit
```

**Solution:**
```javascript
// Check SOL balance before transaction
const balance = await connection.getBalance(wallet.publicKey);
const requiredFee = 0.01 * LAMPORTS_PER_SOL; // 0.01 SOL minimum

if (balance < requiredFee) {
  throw new Error(`Insufficient SOL. Need ${requiredFee/LAMPORTS_PER_SOL} SOL for fees`);
}

// Add priority fee for faster execution
transaction.add(
  ComputeBudgetProgram.setComputeUnitPrice({
    microLamports: 1000 // Priority fee
  })
);
```

---

### DLMM-Specific Issues

#### Problem: Position out of range
```javascript
Error: Current price 55 is outside position range [45, 50]
```

**Solution:**
```javascript
// Check if position is in range
async function checkPositionInRange(position, currentPrice) {
  const inRange = currentPrice >= position.lowerPrice && 
                  currentPrice <= position.upperPrice;
  
  if (!inRange) {
    // Option 1: Rebalance
    const newRange = calculateOptimalRange(currentPrice, volatility);
    await rebalancePosition(position, newRange);
    
    // Option 2: Wait for price return
    console.log('Position out of range. Not earning fees.');
    
    // Option 3: Close and reopen
    await closePosition(position);
    await createNewPosition(newRange);
  }
}
```

#### Problem: Invalid bin ID
```javascript
Error: Bin ID 500000 exceeds maximum 443636
```

**Solution:**
```javascript
// Validate bin ID before use
const MAX_BIN_ID = 443636;
const MIN_BIN_ID = -443636;

function validateBinId(binId) {
  if (binId > MAX_BIN_ID || binId < MIN_BIN_ID) {
    throw new Error(`Bin ID ${binId} out of valid range [${MIN_BIN_ID}, ${MAX_BIN_ID}]`);
  }
  return binId;
}

// Calculate valid bin range
function calculateValidBinRange(targetPrice, binStep, rangePercent) {
  const activeBin = priceToBindId(targetPrice, binStep);
  const range = Math.floor(rangePercent / (binStep / 10000));
  
  return {
    lower: Math.max(activeBin - range, MIN_BIN_ID),
    upper: Math.min(activeBin + range, MAX_BIN_ID)
  };
}
```

#### Problem: Liquidity shape not applying
```javascript
Error: Distribution mode not recognized
```

**Solution:**
```javascript
// Correct liquidity distribution setup
const validDistributions = {
  uniform: { type: 'UNIFORM' },
  spot: { type: 'SPOT' },
  curve: { type: 'CURVE', alpha: 0.5 },
  normal: { type: 'NORMAL', sigma: 1.5 },
  bidAsk: { type: 'BID_ASK' }
};

// Use valid distribution
const position = await dlmmClient.createPosition({
  poolAddress,
  lowerBinId: -50,
  upperBinId: 50,
  totalLiquidity: new BN(10000),
  distributionMode: validDistributions.normal // Correct format
});
```

---

### Connection & RPC Issues

#### Problem: RPC rate limiting
```javascript
Error: 429 Too Many Requests
```

**Solution:**
```javascript
// Use connection pooling
class ConnectionPool {
  constructor(endpoints) {
    this.connections = endpoints.map(url => new Connection(url));
    this.current = 0;
  }
  
  getConnection() {
    const conn = this.connections[this.current];
    this.current = (this.current + 1) % this.connections.length;
    return conn;
  }
}

// Or use retry with backoff
async function retryWithBackoff(fn, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (error.message.includes('429') && i < maxRetries - 1) {
        await new Promise(resolve => setTimeout(resolve, Math.pow(2, i) * 1000));
      } else {
        throw error;
      }
    }
  }
}
```

#### Problem: WebSocket disconnection
```javascript
Error: WebSocket connection closed
```

**Solution:**
```javascript
// Implement reconnection logic
class ResilientWebSocket {
  constructor(endpoint) {
    this.endpoint = endpoint;
    this.connect();
  }
  
  connect() {
    this.ws = new WebSocket(this.endpoint);
    
    this.ws.onclose = () => {
      console.log('WebSocket closed, reconnecting...');
      setTimeout(() => this.connect(), 5000);
    };
    
    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      this.ws.close();
    };
  }
}
```

---

### Wallet Integration Issues

#### Problem: Wallet not connected
```javascript
Error: Wallet not connected
```

**Solution:**
```javascript
// Proper wallet connection flow
async function connectWallet() {
  try {
    // Check if wallet is installed
    if (!window.solana) {
      alert('Please install a Solana wallet (Phantom, Solflare, etc.)');
      return;
    }
    
    // Connect with proper error handling
    const response = await window.solana.connect();
    
    // Verify connection
    if (!response.publicKey) {
      throw new Error('Failed to get public key');
    }
    
    // Listen for disconnect
    window.solana.on('disconnect', () => {
      console.log('Wallet disconnected');
      // Handle disconnect
    });
    
    return response.publicKey;
  } catch (error) {
    if (error.code === 4001) {
      console.log('User rejected connection');
    } else {
      console.error('Wallet connection error:', error);
    }
    throw error;
  }
}
```

#### Problem: Transaction rejected by wallet
```javascript
Error: User rejected the request
```

**Solution:**
```javascript
// Provide clear transaction details
async function requestSignature(transaction, wallet) {
  try {
    // Add memo for clarity
    transaction.add(
      new TransactionInstruction({
        keys: [],
        programId: new PublicKey('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr'),
        data: Buffer.from('Saros: Swap 1 SOL for USDC')
      })
    );
    
    // Request signature with proper error handling
    const signed = await wallet.signTransaction(transaction);
    return signed;
    
  } catch (error) {
    if (error.code === 4001) {
      console.log('Transaction rejected by user');
      // Show user-friendly message
    }
    throw error;
  }
}
```

---

## Performance Optimization

### Slow Transactions

**Problem:** Transactions taking too long to confirm

**Solutions:**

1. **Use Priority Fees**
```javascript
import { ComputeBudgetProgram } from '@solana/web3.js';

// Add priority fee
transaction.add(
  ComputeBudgetProgram.setComputeUnitPrice({
    microLamports: 10000 // Higher fee for priority
  })
);
```

2. **Optimize Compute Units**
```javascript
// Set appropriate compute budget
transaction.add(
  ComputeBudgetProgram.setComputeUnitLimit({
    units: 200000 // Adjust based on transaction complexity
  })
);
```

3. **Use Dedicated RPC**
```javascript
// Use premium RPC endpoints
const connection = new Connection(
  'https://solana-mainnet.g.alchemy.com/v2/YOUR-API-KEY',
  {
    commitment: 'confirmed',
    confirmTransactionInitialTimeout: 60000
  }
);
```

### High RPC Costs

**Problem:** Too many RPC calls

**Solutions:**

1. **Batch Requests**
```javascript
// Batch multiple requests
const accounts = await connection.getMultipleAccountsInfo([
  account1,
  account2,
  account3
]);
```

2. **Implement Caching**
```javascript
class CachedConnection {
  constructor(connection) {
    this.connection = connection;
    this.cache = new Map();
  }
  
  async getAccountInfo(pubkey, useCache = true) {
    const key = pubkey.toString();
    
    if (useCache && this.cache.has(key)) {
      const cached = this.cache.get(key);
      if (Date.now() - cached.timestamp < 5000) {
        return cached.data;
      }
    }
    
    const data = await this.connection.getAccountInfo(pubkey);
    this.cache.set(key, { data, timestamp: Date.now() });
    return data;
  }
}
```

---

## FAQ

### General Questions

**Q: What's the minimum SOL needed for transactions?**
A: Keep at least 0.01 SOL for transaction fees. Complex transactions may require more.

**Q: Which RPC endpoint should I use?**
A: For production, use dedicated endpoints like Helius, QuickNode, or Alchemy. Free endpoints have rate limits.

**Q: How do I handle network congestion?**
A: Use priority fees, implement retry logic, and consider batching transactions during high congestion.

### SDK-Specific Questions

**Q: Can I use TypeScript SDK in React Native?**
A: Yes, the TypeScript SDK works with React Native. You may need polyfills for some Node.js modules.

**Q: How do I migrate from AMM to DLMM?**
A: Remove liquidity from AMM, calculate optimal DLMM range, and create new position using DLMM SDK.

**Q: What's the difference between bin step and fee rate?**
A: Bin step determines price granularity (space between bins), fee rate is the trading fee percentage.

### DLMM Questions

**Q: How do I choose the right bin step?**
A: 
- Stable pairs: 1 (0.01%)
- Low volatility: 10 (0.10%)
- Medium volatility: 20 (0.20%)
- High volatility: 50+ (0.50%+)

**Q: What happens when my position goes out of range?**
A: You stop earning fees but don't lose your liquidity. You can wait for price to return or rebalance.

**Q: How many bins can I provide liquidity to?**
A: Maximum 70 bins per position.

### Trading Questions

**Q: How is slippage calculated?**
A: Slippage = ((expected output - minimum output) / expected output) × 100

**Q: What causes high price impact?**
A: Large trade size relative to pool liquidity. Split into smaller trades or find deeper liquidity pools.

**Q: How do I find the best swap route?**
A: Use the SDK's routing functions or Jupiter Aggregator for optimal paths.

### Security Questions

**Q: How do I secure my private keys?**
A: Never commit keys to git, use environment variables, and consider hardware wallets for production.

**Q: Is it safe to increase slippage?**
A: Higher slippage increases MEV risk. Use reasonable values (0.5-3%) and consider private mempools.

**Q: How do I prevent sandwich attacks?**
A: Use minimal slippage, priority fees, and consider Jito bundles for MEV protection.

---

## Error Codes Reference

| Code | Description | Solution |
|------|-------------|----------|
| 0x1 | Insufficient funds | Add more tokens or SOL for fees |
| 0x2 | Slippage exceeded | Increase slippage tolerance |
| 0x3 | Pool not found | Verify pool address |
| 0x4 | Invalid amount | Check amount is > 0 and within limits |
| 0x5 | Account not found | Ensure token accounts exist |
| 0x6 | Program error | Check program logs for details |
| 0x7 | Oracle stale | Wait for oracle update |
| 0x8 | Out of range | Position outside price range |
| 0x9 | Max bins exceeded | Reduce number of bins |
| 0xA | Insufficient liquidity | Find alternative pool |

---

## Debug Checklist

When encountering issues, check:

- [ ] Wallet has SOL for fees
- [ ] Using correct network (mainnet/devnet)
- [ ] Token accounts exist
- [ ] Amounts are in correct units (consider decimals)
- [ ] Slippage is reasonable for pair volatility
- [ ] RPC endpoint is responsive
- [ ] Transaction size within limits
- [ ] Using latest SDK version
- [ ] Pool has sufficient liquidity
- [ ] Price oracles are updated

---

## Getting Help

### Resources
- [Discord Support](https://discord.gg/saros)
- [GitHub Issues](https://github.com/saros-finance/sdk/issues)
- [Documentation](https://docs.saros.finance)
- [Status Page](https://status.saros.finance)

### When Reporting Issues
Include:
1. SDK version
2. Error message and stack trace
3. Code snippet
4. Network (mainnet/devnet)
5. Transaction signature (if applicable)

### Emergency Contacts
- Critical bugs: security@saros.finance
- Integration support: support@saros.finance
- Business inquiries: partnerships@saros.finance