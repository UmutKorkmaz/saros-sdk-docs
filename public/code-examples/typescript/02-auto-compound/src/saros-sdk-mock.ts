/**
 * Mock Saros SDK implementation for auto-compound example
 */

export interface PoolInfo {
  exists: boolean;
  tvl: number;
  apy: number;
  tokenA: string;
  tokenB: string;
  liquidity: number;
}

export interface UserPosition {
  amount: number;
  rewards: number;
  lpTokens: number;
  lastStakeTime?: Date;
}

export interface ClaimResult {
  success: boolean;
  signature: string;
  rewardsClaimed: number;
  gasUsed: number;
}

export interface StakeResult {
  success: boolean;
  signature: string;
  amountStaked: number;
  gasUsed: number;
}

export interface LiquidityResult {
  success: boolean;
  signature: string;
  lpTokensReceived: number;
  gasUsed: number;
}

/**
 * Mock Saros SDK functions for auto-compound
 */
export async function claimRewardsSaros(
  connection: any,
  poolAddress: any,
  userPublicKey: any
): Promise<ClaimResult> {
  // Simulate API delay
  await new Promise(resolve => setTimeout(resolve, 200));
  
  return {
    success: true,
    signature: 'claim-signature-' + Date.now(),
    rewardsClaimed: Math.random() * 10 + 1,
    gasUsed: 0.0001
  };
}

export async function addLiquiditySaros(
  connection: any,
  poolAddress: any,
  tokenAAmount: number,
  tokenBAmount: number,
  userPublicKey: any,
  slippage?: number
): Promise<LiquidityResult> {
  // Simulate API delay
  await new Promise(resolve => setTimeout(resolve, 300));
  
  return {
    success: true,
    signature: 'liquidity-signature-' + Date.now(),
    lpTokensReceived: (tokenAAmount + tokenBAmount) * 0.98, // 2% slippage
    gasUsed: 0.0002
  };
}

export async function stakeSaros(
  connection: any,
  poolAddress: any,
  amount: number,
  userPublicKey: any
): Promise<StakeResult> {
  // Simulate API delay
  await new Promise(resolve => setTimeout(resolve, 250));
  
  return {
    success: true,
    signature: 'stake-signature-' + Date.now(),
    amountStaked: amount,
    gasUsed: 0.00015
  };
}

export async function getPoolInfo(
  connection: any,
  poolAddress: any
): Promise<PoolInfo> {
  // Simulate API delay
  await new Promise(resolve => setTimeout(resolve, 150));
  
  return {
    exists: true,
    tvl: 1000000 + Math.random() * 500000,
    apy: 30 + Math.random() * 40,
    tokenA: 'USDC',
    tokenB: 'SOL',
    liquidity: 500000 + Math.random() * 200000
  };
}

export async function getUserPosition(
  connection: any,
  poolAddress: any,
  userPublicKey: any
): Promise<UserPosition> {
  // Simulate API delay
  await new Promise(resolve => setTimeout(resolve, 100));
  
  return {
    amount: 1000 + Math.random() * 500,
    rewards: Math.random() * 15 + 5,
    lpTokens: 800 + Math.random() * 300,
    lastStakeTime: new Date(Date.now() - Math.random() * 86400000) // Random within last day
  };
}

// Default export
export default {
  claimRewardsSaros,
  addLiquiditySaros,
  stakeSaros,
  getPoolInfo,
  getUserPosition
};