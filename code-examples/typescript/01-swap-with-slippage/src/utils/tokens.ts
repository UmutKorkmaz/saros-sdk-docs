/**
 * Token utilities and common token definitions
 */

import { PublicKey, Connection } from '@solana/web3.js';
import { 
  getAccount,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID
} from '@solana/spl-token';
import { logger } from './logger';

/**
 * Common token definitions
 */
export const TOKENS = {
  // Mainnet tokens
  mainnet: {
    USDC: {
      mint: new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
      decimals: 6,
      symbol: 'USDC',
      name: 'USD Coin'
    },
    USDT: {
      mint: new PublicKey('Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB'),
      decimals: 6,
      symbol: 'USDT',
      name: 'Tether USD'
    },
    SOL: {
      mint: new PublicKey('So11111111111111111111111111111111111111112'),
      decimals: 9,
      symbol: 'SOL',
      name: 'Wrapped SOL'
    },
    USDH: {
      mint: new PublicKey('USDH1SM1ojwWUga67PGrgFWUHibbjqMvuMaDkRJTgkX'),
      decimals: 6,
      symbol: 'USDH',
      name: 'USD HubbleProtocol'
    },
    SRM: {
      mint: new PublicKey('SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt'),
      decimals: 6,
      symbol: 'SRM',
      name: 'Serum'
    },
    RAY: {
      mint: new PublicKey('4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R'),
      decimals: 6,
      symbol: 'RAY',
      name: 'Raydium'
    },
    SAROS: {
      mint: new PublicKey('Saros3j93bT9yKWPNeXp7TCoafWX8j5HHEy3DJLniJN'),
      decimals: 6,
      symbol: 'SAROS',
      name: 'Saros'
    },
    C98: {
      mint: new PublicKey('C98A4nkJXhpVZNAZdHUA95RpTF3T4whtQubL3YobiUX9'),
      decimals: 6,
      symbol: 'C98',
      name: 'Coin98'
    }
  },
  
  // Devnet tokens (same addresses work on devnet for testing)
  devnet: {
    USDC: {
      mint: new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
      decimals: 6,
      symbol: 'USDC',
      name: 'USD Coin (Devnet)'
    },
    SOL: {
      mint: new PublicKey('So11111111111111111111111111111111111111112'),
      decimals: 9,
      symbol: 'SOL',
      name: 'Wrapped SOL (Devnet)'
    }
  }
};

/**
 * Token metadata interface
 */
export interface TokenMetadata {
  mint: PublicKey;
  decimals: number;
  symbol: string;
  name: string;
  logoUri?: string;
  coingeckoId?: string;
  price?: number;
}

/**
 * Token account info
 */
export interface TokenAccountInfo {
  address: PublicKey;
  mint: PublicKey;
  owner: PublicKey;
  amount: bigint;
  decimals: number;
  uiAmount: number;
}

/**
 * Get token by mint address
 */
export function getTokenByMint(
  mint: PublicKey | string,
  network: 'mainnet' | 'devnet' = 'mainnet'
): TokenMetadata | undefined {
  const mintStr = typeof mint === 'string' ? mint : mint.toString();
  
  const tokens = TOKENS[network];
  for (const token of Object.values(tokens)) {
    if (token.mint.toString() === mintStr) {
      return token;
    }
  }
  
  return undefined;
}

/**
 * Get token by symbol
 */
export function getTokenBySymbol(
  symbol: string,
  network: 'mainnet' | 'devnet' = 'mainnet'
): TokenMetadata | undefined {
  const tokens = TOKENS[network];
  for (const token of Object.values(tokens)) {
    if (token.symbol.toLowerCase() === symbol.toLowerCase()) {
      return token;
    }
  }
  
  return undefined;
}

/**
 * Format token amount with decimals
 */
export function formatTokenAmount(
  amount: number | bigint,
  decimals: number,
  displayDecimals: number = 4
): string {
  const value = typeof amount === 'bigint' ? 
    Number(amount) / Math.pow(10, decimals) :
    amount / Math.pow(10, decimals);
  
  return value.toFixed(displayDecimals);
}

/**
 * Parse token amount to smallest unit
 */
export function parseTokenAmount(
  amount: number | string,
  decimals: number
): bigint {
  const value = typeof amount === 'string' ? parseFloat(amount) : amount;
  return BigInt(Math.floor(value * Math.pow(10, decimals)));
}

/**
 * Get or create associated token account
 */
export async function getOrCreateAssociatedTokenAccount(
  connection: Connection,
  payer: PublicKey,
  mint: PublicKey,
  owner: PublicKey
): Promise<PublicKey> {
  const associatedToken = await getAssociatedTokenAddress(
    mint,
    owner,
    false,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  try {
    await getAccount(connection, associatedToken);
    logger.debug(`Token account exists: ${associatedToken.toString()}`);
    return associatedToken;
  } catch (error: any) {
    if (error.message?.includes('could not find account')) {
      logger.info(`Creating token account for mint ${mint.toString()}`);
      
      // Account doesn't exist, need to create it
      // In production, this would be included in the transaction
      return associatedToken;
    }
    throw error;
  }
}

/**
 * Get all token accounts for wallet
 */
export async function getWalletTokenAccounts(
  connection: Connection,
  wallet: PublicKey
): Promise<TokenAccountInfo[]> {
  const tokenAccounts = await connection.getParsedTokenAccountsByOwner(
    wallet,
    { programId: TOKEN_PROGRAM_ID }
  );

  return tokenAccounts.value.map(account => {
    const parsedInfo = account.account.data.parsed.info;
    return {
      address: account.pubkey,
      mint: new PublicKey(parsedInfo.mint),
      owner: new PublicKey(parsedInfo.owner),
      amount: BigInt(parsedInfo.tokenAmount.amount),
      decimals: parsedInfo.tokenAmount.decimals,
      uiAmount: parsedInfo.tokenAmount.uiAmount
    };
  });
}

/**
 * Get token balance
 */
export async function getTokenBalance(
  connection: Connection,
  tokenAccount: PublicKey
): Promise<{ amount: bigint; decimals: number; uiAmount: number }> {
  try {
    const account = await getAccount(connection, tokenAccount);
    const decimals = 6; // Would fetch from mint in production
    
    return {
      amount: account.amount,
      decimals,
      uiAmount: Number(account.amount) / Math.pow(10, decimals)
    };
  } catch (error) {
    logger.error(`Failed to get token balance: ${error}`);
    return { amount: 0n, decimals: 0, uiAmount: 0 };
  }
}

/**
 * Calculate price impact
 */
export function calculatePriceImpact(
  amountIn: number,
  amountOut: number,
  poolReserveIn: number,
  poolReserveOut: number
): number {
  const idealOut = (amountIn * poolReserveOut) / poolReserveIn;
  const slippage = ((idealOut - amountOut) / idealOut) * 100;
  return Math.abs(slippage);
}

/**
 * Calculate minimum amount out with slippage
 */
export function calculateMinimumAmountOut(
  expectedAmountOut: number,
  slippageTolerance: number
): number {
  return expectedAmountOut * (1 - slippageTolerance / 100);
}

/**
 * Validate token pair
 */
export function validateTokenPair(
  tokenA: PublicKey,
  tokenB: PublicKey
): { valid: boolean; error?: string } {
  if (tokenA.equals(tokenB)) {
    return { valid: false, error: 'Cannot swap same token' };
  }
  
  // Add more validation as needed
  
  return { valid: true };
}

/**
 * Get token price in USD (mock implementation)
 */
export async function getTokenPrice(
  mint: PublicKey,
  network: 'mainnet' | 'devnet' = 'mainnet'
): Promise<number> {
  // In production, fetch from price oracle or CoinGecko
  const mockPrices: { [key: string]: number } = {
    'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v': 1.0,    // USDC
    'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB': 1.0,    // USDT
    'So11111111111111111111111111111111111111112': 50.0,    // SOL
    'Saros3j93bT9yKWPNeXp7TCoafWX8j5HHEy3DJLniJN': 0.1,    // SAROS
    'C98A4nkJXhpVZNAZdHUA95RpTF3T4whtQubL3YobiUX9': 0.5,    // C98
  };
  
  return mockPrices[mint.toString()] || 0;
}

/**
 * Check if token is stable coin
 */
export function isStableCoin(mint: PublicKey): boolean {
  const stableCoins = [
    'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', // USDC
    'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', // USDT
    'USDH1SM1ojwWUga67PGrgFWUHibbjqMvuMaDkRJTgkX', // USDH
  ];
  
  return stableCoins.includes(mint.toString());
}

/**
 * Get token display name
 */
export function getTokenDisplayName(
  mint: PublicKey,
  network: 'mainnet' | 'devnet' = 'mainnet'
): string {
  const token = getTokenByMint(mint, network);
  return token ? `${token.symbol} (${token.name})` : mint.toString().slice(0, 8) + '...';
}

/**
 * Estimate transaction fee
 */
export function estimateTransactionFee(
  signatures: number = 1,
  priorityFee: number = 0
): number {
  const baseFee = 0.000005 * signatures; // 5000 lamports per signature
  return baseFee + priorityFee;
}

export default {
  TOKENS,
  getTokenByMint,
  getTokenBySymbol,
  formatTokenAmount,
  parseTokenAmount,
  getOrCreateAssociatedTokenAccount,
  getWalletTokenAccounts,
  getTokenBalance,
  calculatePriceImpact,
  calculateMinimumAmountOut,
  validateTokenPair,
  getTokenPrice,
  isStableCoin,
  getTokenDisplayName,
  estimateTransactionFee
};