# First Transaction

Get up and running with your first Saros transaction in under 5 minutes.

## Quick Start: Simple Token Swap

Let's start with the most common operation - swapping tokens using the Saros AMM.

### Step 1: Basic Setup

```typescript
// first-swap.ts
import { Connection, PublicKey, Keypair } from '@solana/web3.js';
import { getSwapAmountSaros, swapSaros, genConnectionSolana } from '@saros-finance/sdk';
import bs58 from 'bs58';

// Configuration
const connection = genConnectionSolana(); // Uses devnet by default
const SAROS_SWAP_PROGRAM = new PublicKey('SSwapUtytfBdBn1b9NUGG6foMVPtcWgpRU32HToDUZr');

// Load wallet (replace with your private key)
const wallet = Keypair.fromSecretKey(bs58.decode(process.env.WALLET_PRIVATE_KEY!));
console.log('Wallet:', wallet.publicKey.toString());
```

### Step 2: Check Token Balances

```typescript
import { getAccount, getAssociatedTokenAddress } from '@solana/spl-token';

// Token addresses (devnet)
const USDC_MINT = new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v');
const C98_MINT = new PublicKey('C98A4nkJXhpVZNAZdHUA95RpTF3T4whtQubL3YobiUX9');

async function checkBalances() {
  try {
    // Get token accounts
    const usdcTokenAccount = await getAssociatedTokenAddress(USDC_MINT, wallet.publicKey);
    const c98TokenAccount = await getAssociatedTokenAddress(C98_MINT, wallet.publicKey);
    
    // Check balances
    const usdcBalance = await getAccount(connection, usdcTokenAccount);
    const c98Balance = await getAccount(connection, c98TokenAccount);
    
    console.log('USDC Balance:', Number(usdcBalance.amount) / 1e6);
    console.log('C98 Balance:', Number(c98Balance.amount) / 1e9);
    
    return { usdcTokenAccount, c98TokenAccount };
  } catch (error) {
    console.error('Error checking balances:', error);
    throw error;
  }
}
```

### Step 3: Execute Token Swap

```typescript
// Swap configuration
const swapParams = {
  poolAddress: new PublicKey('POOL_ADDRESS_HERE'), // Get from Saros UI or API
  fromMint: USDC_MINT,
  toMint: C98_MINT,
  amount: 1, // 1 USDC
  slippagePercent: 0.5, // 0.5% slippage
};

async function performSwap() {
  try {
    console.log('🔄 Starting swap...');
    
    // Step 1: Get token accounts
    const { usdcTokenAccount, c98TokenAccount } = await checkBalances();
    
    // Step 2: Calculate swap amount with slippage
    console.log('📊 Calculating swap amounts...');
    const swapEstimate = await getSwapAmountSaros(
      connection,
      swapParams.fromMint.toString(),
      swapParams.toMint.toString(),
      swapParams.amount,
      swapParams.slippagePercent,
      poolParams // Pool parameters from your configuration
    );
    
    console.log('Expected output:', swapEstimate.amountOut, 'C98');
    console.log('With slippage:', swapEstimate.amountOutWithSlippage, 'C98');
    
    // Step 3: Execute the swap
    console.log('⚡ Executing swap transaction...');
    const result = await swapSaros(
      connection,
      usdcTokenAccount, // From token account
      c98TokenAccount,  // To token account
      swapParams.amount * 1e6, // Amount in token decimals (USDC = 6 decimals)
      swapEstimate.amountOutWithSlippage,
      null, // No referrer
      swapParams.poolAddress,
      SAROS_SWAP_PROGRAM,
      wallet.publicKey,
      swapParams.fromMint,
      swapParams.toMint
    );
    
    console.log('✅ Swap completed!');
    console.log('Transaction:', result.hash);
    console.log('Explorer:', `https://explorer.solana.com/tx/${result.hash}?cluster=devnet`);
    
    // Step 4: Check new balances
    await new Promise(resolve => setTimeout(resolve, 5000)); // Wait for confirmation
    await checkBalances();
    
  } catch (error) {
    console.error('❌ Swap failed:', error);
    throw error;
  }
}
```

### Step 4: Complete Example

```typescript
// complete-first-swap.ts
async function main() {
  try {
    console.log('🚀 Starting Saros SDK Demo');
    console.log('Network: Devnet');
    console.log('Wallet:', wallet.publicKey.toString());
    
    // Check SOL balance for transaction fees
    const solBalance = await connection.getBalance(wallet.publicKey);
    console.log('SOL Balance:', solBalance / 1e9, 'SOL');
    
    if (solBalance < 0.01 * 1e9) {
      throw new Error('Insufficient SOL for transaction fees. Request airdrop: solana airdrop 1');
    }
    
    // Perform the swap
    await performSwap();
    
    console.log('🎉 Demo completed successfully!');
    
  } catch (error) {
    console.error('Demo failed:', error);
    process.exit(1);
  }
}

// Run the demo
main();
```

## Running Your First Transaction

### 1. Setup Environment
```bash
# Create new project
mkdir my-saros-app
cd my-saros-app
npm init -y

# Install dependencies
npm install @saros-finance/sdk @solana/web3.js @solana/spl-token bs58
npm install --save-dev typescript @types/node ts-node

# Create tsconfig.json
npx tsc --init
```

### 2. Environment Configuration
```bash
# Create .env file
echo "WALLET_PRIVATE_KEY=your_base58_private_key_here" > .env
echo "SOLANA_RPC_URL=https://api.devnet.solana.com" >> .env
```

### 3. Get Devnet Tokens
```bash
# Airdrop SOL for fees
solana airdrop 2

# Get devnet USDC from faucet
# Visit: https://spl-token-faucet.com/
# Or use Solana CLI to create and mint test tokens
```

### 4. Run the Example
```bash
# Create the script file
cp complete-first-swap.ts first-swap.ts

# Run it
npx ts-node first-swap.ts
```

## Expected Output

```
🚀 Starting Saros SDK Demo
Network: Devnet
Wallet: 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU
SOL Balance: 2.5 SOL
USDC Balance: 10.0
C98 Balance: 0.0
🔄 Starting swap...
📊 Calculating swap amounts...
Expected output: 15.234 C98
With slippage: 15.158 C98
⚡ Executing swap transaction...
✅ Swap completed!
Transaction: 2ZE7A1gKXt9f8Wq7Q9YjP5uM3N9kR8sT6oA4hL2xV7cB3nF1
Explorer: https://explorer.solana.com/tx/2ZE7A1gKXt9f8Wq7Q9YjP5uM3N9kR8sT6oA4hL2xV7cB3nF1?cluster=devnet
USDC Balance: 9.0
C98 Balance: 15.158
🎉 Demo completed successfully!
```

## DLMM SDK First Transaction

For more advanced users, here's a DLMM swap example:

```typescript
import { LiquidityBookServices, MODE } from '@saros-finance/dlmm-sdk';
import { PublicKey } from '@solana/web3.js';

async function dlmmSwap() {
  // Initialize DLMM service
  const dlmmService = new LiquidityBookServices({
    mode: MODE.DEVNET,
  });
  
  // Get quote for 1 USDC -> SOL
  const quote = await dlmmService.getQuote({
    amount: BigInt(1_000_000), // 1 USDC (6 decimals)
    isExactInput: true,
    swapForY: true, // USDC -> SOL
    pair: new PublicKey('PAIR_ADDRESS_HERE'),
    tokenBase: new PublicKey('So11111111111111111111111111111111111111112'), // SOL
    tokenQuote: new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'), // USDC
    tokenBaseDecimal: 9,
    tokenQuoteDecimal: 6,
    slippage: 0.5
  });
  
  console.log('DLMM Quote:', quote);
  
  // Execute swap
  const transaction = await dlmmService.swap({
    amount: quote.amount,
    otherAmountOffset: quote.otherAmountOffset,
    isExactInput: true,
    swapForY: true,
    pair: new PublicKey('PAIR_ADDRESS_HERE'),
    payer: wallet.publicKey
  });
  
  console.log('DLMM Swap Transaction:', transaction);
}
```

## Troubleshooting Your First Transaction

### Common Issues

**"Insufficient SOL for transaction fees"**
```bash
# Request devnet SOL airdrop
solana airdrop 2
solana balance
```

**"Token account does not exist"**
```bash
# Create associated token accounts first
solana-keygen new --outfile token-account.json
```

**"Pool not found" or invalid addresses**
- Use addresses from [Saros UI](https://app.saros.finance) 
- Verify you're on the correct network (devnet/mainnet)
- Check pool exists and has liquidity

**RPC connection timeout**
```typescript
// Try different RPC endpoint
const connection = new Connection(
  'https://devnet.genesysgo.net',
  { commitment: 'confirmed' }
);
```

**Slippage tolerance exceeded**
```typescript
// Increase slippage for volatile pairs
const slippagePercent = 1.0; // 1% instead of 0.5%
```

### Debug Mode

Enable detailed logging:
```typescript
// Add debug logging
console.log('Pool address:', poolAddress.toString());
console.log('From token:', fromMint.toString());
console.log('To token:', toMint.toString());
console.log('Amount:', amount);
console.log('Slippage:', slippagePercent);
```

## Next Steps

🎉 Congratulations! You've completed your first Saros transaction.

### Continue Learning:
1. [📚 Core Concepts](../core-concepts/amm-vs-dlmm.md) - Understand AMM vs DLMM
2. [📖 SDK Guides](../sdk-guides/typescript-sdk/swap-operations.md) - Deep dive into swap operations  
3. [📝 Tutorials](../tutorials/01-basic-swap.md) - More advanced tutorials
4. [💻 Code Examples](../code-examples/typescript/01-swap-with-slippage/) - Production-ready examples

### Build Something:
- Token swap interface
- Portfolio tracker
- Arbitrage bot
- Liquidity management tool

Need help? Join our [Discord community](https://discord.gg/saros) for support!

## Resources

- **Saros App**: [app.saros.finance](https://app.saros.finance)
- **Explorer**: [explorer.solana.com](https://explorer.solana.com/?cluster=devnet)
- **Documentation**: [docs.saros.finance](https://docs.saros.finance)
- **GitHub**: [github.com/saros-finance](https://github.com/saros-finance)