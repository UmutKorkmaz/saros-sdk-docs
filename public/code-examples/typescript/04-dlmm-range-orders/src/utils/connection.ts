/**
 * Connection utilities for Solana RPC
 */

import { Connection, Keypair, Commitment } from '@solana/web3.js';
import * as bs58 from 'bs58';
import dotenv from 'dotenv';

dotenv.config();

/**
 * Get Solana connection
 */
export function getConnection(): Connection {
  const rpcUrl = process.env.SOLANA_RPC_URL || 'https://api.mainnet-beta.solana.com';
  const commitment: Commitment = 'confirmed';
  
  return new Connection(rpcUrl, {
    commitment,
    confirmTransactionInitialTimeout: 60000
  });
}

/**
 * Get wallet keypair from environment
 */
export function getWallet(): Keypair {
  const privateKey = process.env.WALLET_PRIVATE_KEY;
  
  if (!privateKey) {
    throw new Error('WALLET_PRIVATE_KEY not found in environment');
  }
  
  try {
    // Try to parse as base58
    const decoded = bs58.decode(privateKey);
    return Keypair.fromSecretKey(decoded);
  } catch {
    // Try as array of numbers
    const secretKey = Uint8Array.from(JSON.parse(privateKey));
    return Keypair.fromSecretKey(secretKey);
  }
}

/**
 * Get network from environment
 */
export function getNetwork(): 'mainnet-beta' | 'devnet' | 'testnet' {
  const network = process.env.NETWORK || 'mainnet-beta';
  
  if (!['mainnet-beta', 'devnet', 'testnet'].includes(network)) {
    throw new Error(`Invalid network: ${network}`);
  }
  
  return network as 'mainnet-beta' | 'devnet' | 'testnet';
}

/**
 * Get priority fee from environment
 */
export function getPriorityFee(): number {
  return parseInt(process.env.PRIORITY_FEE || '10000');
}