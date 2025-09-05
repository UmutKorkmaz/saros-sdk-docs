# Security Best Practices

## Building Secure Applications with Saros SDKs

Security is paramount when building DeFi applications. This guide covers essential security practices for Saros SDK integration.

## Table of Contents
- [Private Key Management](#private-key-management)
- [Transaction Security](#transaction-security)
- [Smart Contract Interactions](#smart-contract-interactions)
- [MEV Protection](#mev-protection)
- [Input Validation](#input-validation)
- [Error Handling](#error-handling)
- [Audit Checklist](#audit-checklist)

## Private Key Management

### Never Expose Private Keys

```typescript
// ❌ NEVER DO THIS
const privateKey = "5JcK9jF5x6..."; // Hardcoded private key

// ✅ DO THIS INSTEAD
import dotenv from 'dotenv';
dotenv.config();

const privateKey = process.env.PRIVATE_KEY;
if (!privateKey) {
  throw new Error('Private key not configured');
}
```

### Use Hardware Wallets in Production

```typescript
import { LedgerWallet } from '@solana/wallet-adapter-ledger';

class SecureWalletManager {
  async connectHardwareWallet(): Promise<Wallet> {
    if (process.env.NODE_ENV === 'production') {
      // Use hardware wallet in production
      const ledger = new LedgerWallet();
      await ledger.connect();
      return ledger;
    } else {
      // Use local keypair for development only
      return Keypair.fromSecretKey(
        bs58.decode(process.env.DEV_PRIVATE_KEY!)
      );
    }
  }
  
  // Implement secure key rotation
  async rotateKeys() {
    const newKeypair = Keypair.generate();
    
    // Transfer assets to new address
    await this.transferAssets(this.currentWallet, newKeypair);
    
    // Update all references
    await this.updateWalletReferences(newKeypair.publicKey);
    
    // Securely store new key
    await this.secureStore.save(newKeypair);
    
    // Destroy old key
    this.secureDestroy(this.currentWallet);
  }
}
```

### Secure Key Storage

```typescript
import * as crypto from 'crypto';
import * as fs from 'fs';

class SecureKeyStorage {
  private algorithm = 'aes-256-gcm';
  
  encryptKey(privateKey: string, password: string): EncryptedKey {
    const salt = crypto.randomBytes(32);
    const key = crypto.pbkdf2Sync(password, salt, 100000, 32, 'sha256');
    const iv = crypto.randomBytes(16);
    
    const cipher = crypto.createCipheriv(this.algorithm, key, iv);
    
    let encrypted = cipher.update(privateKey, 'utf8', 'hex');
    encrypted += cipher.final('hex');
    
    const authTag = cipher.getAuthTag();
    
    return {
      encrypted,
      salt: salt.toString('hex'),
      iv: iv.toString('hex'),
      authTag: authTag.toString('hex')
    };
  }
  
  decryptKey(encryptedKey: EncryptedKey, password: string): string {
    const key = crypto.pbkdf2Sync(
      password,
      Buffer.from(encryptedKey.salt, 'hex'),
      100000,
      32,
      'sha256'
    );
    
    const decipher = crypto.createDecipheriv(
      this.algorithm,
      key,
      Buffer.from(encryptedKey.iv, 'hex')
    );
    
    decipher.setAuthTag(Buffer.from(encryptedKey.authTag, 'hex'));
    
    let decrypted = decipher.update(encryptedKey.encrypted, 'hex', 'utf8');
    decrypted += decipher.final('utf8');
    
    return decrypted;
  }
  
  // Use OS keychain when available
  async useSystemKeychain(keyName: string, privateKey: string) {
    if (process.platform === 'darwin') {
      // macOS Keychain
      const keychain = await import('keychain');
      await keychain.setPassword({
        account: keyName,
        service: 'saros-wallet',
        password: privateKey
      });
    } else if (process.platform === 'win32') {
      // Windows Credential Manager
      const credentialManager = await import('windows-credential-manager');
      await credentialManager.save(keyName, privateKey);
    }
  }
}
```

## Transaction Security

### Transaction Validation

```typescript
class TransactionValidator {
  async validateTransaction(tx: Transaction): Promise<ValidationResult> {
    const checks = {
      signature: await this.verifySignatures(tx),
      accounts: await this.validateAccounts(tx),
      amounts: await this.validateAmounts(tx),
      fees: await this.checkFees(tx),
      recentBlockhash: await this.validateBlockhash(tx)
    };
    
    const issues = Object.entries(checks)
      .filter(([_, valid]) => !valid)
      .map(([check, _]) => check);
    
    if (issues.length > 0) {
      throw new Error(`Transaction validation failed: ${issues.join(', ')}`);
    }
    
    return { valid: true, checks };
  }
  
  private async validateAmounts(tx: Transaction): Promise<boolean> {
    for (const instruction of tx.instructions) {
      const amount = this.extractAmount(instruction);
      
      // Check for overflow
      if (amount > Number.MAX_SAFE_INTEGER) {
        return false;
      }
      
      // Check for suspicious amounts
      if (this.isSuspiciousAmount(amount)) {
        console.warn(`Suspicious amount detected: ${amount}`);
        return false;
      }
    }
    
    return true;
  }
  
  private isSuspiciousAmount(amount: number): boolean {
    // Check for common attack patterns
    const suspiciousPatterns = [
      amount === 0,                    // Zero amount
      amount === Number.MAX_VALUE,     // Max value
      amount.toString().includes('e'), // Scientific notation
    ];
    
    return suspiciousPatterns.some(pattern => pattern);
  }
}
```

### Simulation Before Execution

```typescript
class SafeTransactionExecutor {
  async executeWithSimulation(
    transaction: Transaction,
    options: ExecutionOptions
  ): Promise<TransactionResult> {
    // 1. Simulate transaction
    const simulation = await this.connection.simulateTransaction(transaction);
    
    if (simulation.value.err) {
      throw new Error(`Simulation failed: ${JSON.stringify(simulation.value.err)}`);
    }
    
    // 2. Analyze simulation results
    const analysis = this.analyzeSimulation(simulation);
    
    if (analysis.suspiciousActivity) {
      throw new Error(`Suspicious activity detected: ${analysis.reason}`);
    }
    
    // 3. Check expected vs actual
    if (options.expectedChanges) {
      const matches = this.validateExpectedChanges(
        simulation,
        options.expectedChanges
      );
      
      if (!matches) {
        throw new Error('Transaction outcome does not match expectations');
      }
    }
    
    // 4. Execute with retry logic
    return await this.executeWithRetry(transaction, options.maxRetries || 3);
  }
  
  private analyzeSimulation(simulation: SimulationResult): Analysis {
    const logs = simulation.value.logs || [];
    
    // Check for common attack patterns in logs
    const suspiciousPatterns = [
      /withdraw.*all/i,
      /transfer.*owner/i,
      /close.*account/i,
      /drain/i
    ];
    
    for (const pattern of suspiciousPatterns) {
      if (logs.some(log => pattern.test(log))) {
        return {
          suspiciousActivity: true,
          reason: `Suspicious pattern detected: ${pattern}`
        };
      }
    }
    
    return { suspiciousActivity: false };
  }
}
```

## Smart Contract Interactions

### Program ID Verification

```typescript
class ProgramVerifier {
  private readonly VERIFIED_PROGRAMS = {
    SAROS_AMM: 'SSwapUtytfBdBn1b9NUGG6foMVPtcWgpRU32HToDUZr',
    SAROS_DLMM: 'LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo',
    TOKEN_PROGRAM: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
  };
  
  verifyProgramId(programId: PublicKey, expectedProgram: string): boolean {
    const expected = this.VERIFIED_PROGRAMS[expectedProgram];
    
    if (!expected) {
      throw new Error(`Unknown program: ${expectedProgram}`);
    }
    
    if (!programId.equals(new PublicKey(expected))) {
      throw new Error(
        `Invalid program ID. Expected: ${expected}, Got: ${programId.toString()}`
      );
    }
    
    return true;
  }
  
  async verifyProgramDeployment(programId: PublicKey): Promise<boolean> {
    const accountInfo = await this.connection.getAccountInfo(programId);
    
    if (!accountInfo) {
      throw new Error('Program account not found');
    }
    
    if (!accountInfo.executable) {
      throw new Error('Account is not an executable program');
    }
    
    // Verify program owner (BPF Loader)
    const BPF_LOADER = new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111');
    if (!accountInfo.owner.equals(BPF_LOADER)) {
      throw new Error('Program not owned by BPF Loader');
    }
    
    return true;
  }
}
```

### Account Validation

```typescript
class AccountValidator {
  async validatePoolAccount(poolAddress: PublicKey): Promise<void> {
    const poolAccount = await this.connection.getAccountInfo(poolAddress);
    
    if (!poolAccount) {
      throw new Error('Pool account not found');
    }
    
    // Verify owner is Saros program
    if (!poolAccount.owner.equals(SAROS_PROGRAM_ID)) {
      throw new Error('Pool not owned by Saros program');
    }
    
    // Verify account data structure
    const poolData = PoolLayout.decode(poolAccount.data);
    
    // Validate pool parameters
    if (poolData.fee > 10000) { // Max 100% fee
      throw new Error('Invalid pool fee');
    }
    
    if (poolData.liquidity === 0) {
      throw new Error('Pool has no liquidity');
    }
  }
  
  async validateTokenAccount(
    tokenAccount: PublicKey,
    expectedOwner: PublicKey,
    expectedMint: PublicKey
  ): Promise<void> {
    const account = await getAccount(this.connection, tokenAccount);
    
    if (!account.owner.equals(expectedOwner)) {
      throw new Error('Token account owner mismatch');
    }
    
    if (!account.mint.equals(expectedMint)) {
      throw new Error('Token account mint mismatch');
    }
    
    if (account.state !== 'initialized') {
      throw new Error('Token account not initialized');
    }
  }
}
```

## MEV Protection

### Front-Running Protection

```typescript
class MEVProtection {
  private jitoClient: JitoClient;
  
  async protectedSwap(
    swapParams: SwapParams,
    protection: ProtectionLevel
  ): Promise<TransactionResult> {
    switch (protection) {
      case 'HIGH':
        return await this.executeViaJito(swapParams);
      
      case 'MEDIUM':
        return await this.executeWithDecoy(swapParams);
      
      case 'LOW':
        return await this.executeWithPriorityFee(swapParams);
      
      default:
        return await this.normalExecution(swapParams);
    }
  }
  
  private async executeViaJito(params: SwapParams): Promise<TransactionResult> {
    const tx = await this.buildSwapTransaction(params);
    
    // Add Jito tip
    const tipAmount = this.calculateOptimalTip(params.amount);
    tx.add(this.createTipInstruction(tipAmount));
    
    // Send via Jito bundle
    const bundle = await this.jitoClient.sendBundle([tx]);
    
    return await this.waitForConfirmation(bundle.id);
  }
  
  private async executeWithDecoy(params: SwapParams): Promise<TransactionResult> {
    // Create decoy transactions
    const decoys = await this.createDecoyTransactions(params);
    
    // Mix real transaction with decoys
    const allTxs = this.shuffleTransactions([
      ...decoys,
      await this.buildSwapTransaction(params)
    ]);
    
    // Send all transactions
    const results = await Promise.allSettled(
      allTxs.map(tx => this.sendTransaction(tx))
    );
    
    // Return real transaction result
    return this.extractRealResult(results);
  }
  
  private calculateOptimalTip(amount: BN): number {
    // Dynamic tip based on transaction value
    const value = amount.toNumber() / LAMPORTS_PER_SOL;
    
    if (value < 100) {
      return 0.0001 * LAMPORTS_PER_SOL; // 0.0001 SOL
    } else if (value < 1000) {
      return 0.001 * LAMPORTS_PER_SOL;  // 0.001 SOL
    } else {
      return 0.01 * LAMPORTS_PER_SOL;   // 0.01 SOL
    }
  }
}
```

### Sandwich Attack Prevention

```typescript
class SandwichProtection {
  async detectSandwichRisk(
    pool: PoolInfo,
    swapAmount: BN
  ): Promise<RiskLevel> {
    // Check pool liquidity
    const impactPercent = this.calculatePriceImpact(pool, swapAmount);
    
    if (impactPercent > 5) {
      return 'HIGH'; // High risk of sandwich attack
    } else if (impactPercent > 2) {
      return 'MEDIUM';
    } else {
      return 'LOW';
    }
  }
  
  async protectFromSandwich(
    swapParams: SwapParams
  ): Promise<TransactionResult> {
    const risk = await this.detectSandwichRisk(
      swapParams.pool,
      swapParams.amount
    );
    
    if (risk === 'HIGH') {
      // Split into smaller swaps
      return await this.executeSplitSwap(swapParams, 5);
    } else if (risk === 'MEDIUM') {
      // Use commit-reveal pattern
      return await this.executeCommitReveal(swapParams);
    } else {
      // Normal execution with priority fee
      return await this.executeWithPriority(swapParams);
    }
  }
  
  private async executeCommitReveal(
    params: SwapParams
  ): Promise<TransactionResult> {
    // Step 1: Commit hash of swap parameters
    const commitment = this.hashSwapParams(params);
    await this.commitSwap(commitment);
    
    // Step 2: Wait for commitment confirmation
    await this.waitBlocks(2);
    
    // Step 3: Reveal and execute
    return await this.revealAndExecute(params, commitment);
  }
}
```

## Input Validation

### Parameter Sanitization

```typescript
class InputSanitizer {
  sanitizeAmount(input: string | number | BN): BN {
    let amount: BN;
    
    if (typeof input === 'string') {
      // Remove non-numeric characters
      const cleaned = input.replace(/[^0-9.]/g, '');
      
      // Check for multiple decimal points
      if ((cleaned.match(/\./g) || []).length > 1) {
        throw new Error('Invalid amount format');
      }
      
      // Convert to BN
      const [whole, decimal = ''] = cleaned.split('.');
      const decimals = 9; // SOL decimals
      const multiplier = new BN(10).pow(new BN(decimals));
      
      amount = new BN(whole).mul(multiplier);
      if (decimal) {
        const decimalBN = new BN(decimal.padEnd(decimals, '0').slice(0, decimals));
        amount = amount.add(decimalBN);
      }
    } else if (typeof input === 'number') {
      if (!Number.isFinite(input) || input < 0) {
        throw new Error('Invalid amount');
      }
      amount = new BN(Math.floor(input));
    } else {
      amount = input;
    }
    
    // Validate range
    if (amount.isNeg()) {
      throw new Error('Amount cannot be negative');
    }
    
    if (amount.gt(new BN(Number.MAX_SAFE_INTEGER))) {
      throw new Error('Amount too large');
    }
    
    return amount;
  }
  
  sanitizeAddress(input: string): PublicKey {
    // Remove whitespace
    const cleaned = input.trim();
    
    // Validate base58 format
    if (!/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(cleaned)) {
      throw new Error('Invalid address format');
    }
    
    try {
      const pubkey = new PublicKey(cleaned);
      
      // Additional validation
      if (pubkey.equals(SystemProgram.programId)) {
        throw new Error('Cannot use system program as address');
      }
      
      return pubkey;
    } catch (error) {
      throw new Error(`Invalid Solana address: ${error.message}`);
    }
  }
  
  sanitizeSlippage(input: number): number {
    if (!Number.isFinite(input) || input < 0 || input > 100) {
      throw new Error('Slippage must be between 0 and 100');
    }
    
    // Warn for high slippage
    if (input > 5) {
      console.warn(`High slippage detected: ${input}%`);
    }
    
    return input;
  }
}
```

### SQL Injection Prevention (for off-chain data)

```typescript
class DatabaseSecurity {
  async queryPoolStats(poolAddress: string): Promise<PoolStats> {
    // Parameterized query - NEVER concatenate user input
    const query = `
      SELECT * FROM pool_stats 
      WHERE pool_address = $1 
      AND timestamp > NOW() - INTERVAL '24 hours'
    `;
    
    // Validate address before query
    const sanitizedAddress = this.sanitizer.sanitizeAddress(poolAddress);
    
    const result = await this.db.query(query, [sanitizedAddress.toString()]);
    
    return this.parsePoolStats(result.rows[0]);
  }
  
  // Use query builders for complex queries
  async complexQuery(filters: FilterParams) {
    const query = this.queryBuilder
      .select('*')
      .from('pools')
      .where('tvl', '>', filters.minTvl || 0)
      .andWhere('fee', '<', filters.maxFee || 10000)
      .orderBy('volume_24h', 'DESC')
      .limit(filters.limit || 10);
    
    return await query.execute();
  }
}
```

## Error Handling

### Secure Error Messages

```typescript
class SecureErrorHandler {
  handleError(error: any, context: string): ErrorResponse {
    // Log full error internally
    logger.error(`Error in ${context}:`, error);
    
    // Return sanitized error to user
    if (error instanceof UserError) {
      return {
        error: error.message,
        code: error.code
      };
    }
    
    // Don't leak sensitive information
    const sanitizedMessages = {
      'Invalid signature': 'Authentication failed',
      'Insufficient funds': 'Transaction cannot be completed',
      'Program failed': 'Operation failed',
      // Map other sensitive errors
    };
    
    const message = sanitizedMessages[error.message] || 'An error occurred';
    
    return {
      error: message,
      code: 'INTERNAL_ERROR',
      requestId: this.generateRequestId()
    };
  }
  
  // Implement circuit breaker for repeated errors
  async withCircuitBreaker<T>(
    operation: () => Promise<T>,
    options: CircuitBreakerOptions
  ): Promise<T> {
    const key = options.key;
    const failures = this.failureCount.get(key) || 0;
    
    if (failures >= options.threshold) {
      const resetTime = this.resetTimes.get(key);
      if (resetTime && Date.now() < resetTime) {
        throw new Error('Service temporarily unavailable');
      }
      
      // Reset circuit breaker
      this.failureCount.delete(key);
      this.resetTimes.delete(key);
    }
    
    try {
      const result = await operation();
      this.failureCount.delete(key); // Reset on success
      return result;
    } catch (error) {
      this.failureCount.set(key, failures + 1);
      
      if (failures + 1 >= options.threshold) {
        this.resetTimes.set(key, Date.now() + options.resetAfter);
      }
      
      throw error;
    }
  }
}
```

## Audit Checklist

### Pre-Deployment Security Checklist

```typescript
class SecurityAuditor {
  async performSecurityAudit(): Promise<AuditReport> {
    const checks = {
      // Key Management
      privateKeysSecure: await this.checkKeyManagement(),
      hardwareWalletSupport: await this.checkHardwareWalletSupport(),
      
      // Transaction Security
      simulationEnabled: await this.checkSimulation(),
      mevProtection: await this.checkMEVProtection(),
      
      // Input Validation
      inputSanitization: await this.checkInputSanitization(),
      parameterValidation: await this.checkParameterValidation(),
      
      // Error Handling
      secureErrorMessages: await this.checkErrorMessages(),
      loggingEnabled: await this.checkLogging(),
      
      // Smart Contract
      programVerification: await this.checkProgramVerification(),
      accountValidation: await this.checkAccountValidation(),
      
      // Dependencies
      dependencyAudit: await this.auditDependencies(),
      versionPinning: await this.checkVersionPinning(),
      
      // Rate Limiting
      rateLimitingEnabled: await this.checkRateLimiting(),
      ddosProtection: await this.checkDDoSProtection(),
    };
    
    const issues = Object.entries(checks)
      .filter(([_, passed]) => !passed)
      .map(([check, _]) => check);
    
    return {
      passed: issues.length === 0,
      issues,
      checks,
      timestamp: new Date(),
      recommendations: this.generateRecommendations(issues)
    };
  }
  
  private async auditDependencies(): Promise<boolean> {
    const vulnerabilities = await this.runNpmAudit();
    
    if (vulnerabilities.high > 0 || vulnerabilities.critical > 0) {
      console.error('High/Critical vulnerabilities found in dependencies');
      return false;
    }
    
    return true;
  }
  
  private generateRecommendations(issues: string[]): string[] {
    const recommendations = [];
    
    for (const issue of issues) {
      recommendations.push(this.getRecommendation(issue));
    }
    
    return recommendations;
  }
}
```

### Continuous Security Monitoring

```typescript
class SecurityMonitor {
  async startMonitoring() {
    // Monitor for suspicious transactions
    this.monitorTransactions();
    
    // Monitor for abnormal gas usage
    this.monitorGasUsage();
    
    // Monitor for repeated failed attempts
    this.monitorFailedAttempts();
    
    // Monitor for known attack patterns
    this.monitorAttackPatterns();
  }
  
  private async monitorTransactions() {
    this.connection.onLogs(
      'all',
      (logs) => {
        if (this.isSuspiciousLog(logs)) {
          this.alert('Suspicious transaction detected', logs);
        }
      }
    );
  }
  
  private isSuspiciousLog(logs: Logs): boolean {
    const suspiciousPatterns = [
      /error.*program/i,
      /failed.*verify/i,
      /insufficient/i,
      /unauthorized/i
    ];
    
    return suspiciousPatterns.some(pattern => 
      logs.logs.some(log => pattern.test(log))
    );
  }
}
```

## Security Resources

- [Solana Security Best Practices](https://docs.solana.com/security-best-practices)
- [Saros Audit Reports](/security/audits)
- [Bug Bounty Program](https://immunefi.com/bounty/saros)
- [Security Contact](mailto:security@saros.finance)

## Conclusion

Security is not a one-time consideration but an ongoing process. Always:
1. Keep dependencies updated
2. Follow the principle of least privilege
3. Implement defense in depth
4. Conduct regular security audits
5. Have an incident response plan

Remember: In DeFi, security is paramount. When in doubt, choose the more secure option.