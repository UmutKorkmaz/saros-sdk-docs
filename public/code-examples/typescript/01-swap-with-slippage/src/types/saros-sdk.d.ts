// Type declarations for @saros-finance/sdk
declare module '@saros-finance/sdk' {
  export interface SwapEstimate {
    amountOut: number;
    amountOutWithSlippage: number;
    priceImpact: number;
    fee: number;
    route: string[];
    executionPrice: number;
  }

  export interface SwapResult {
    hash: string;
    amountIn: number;
    amountOut: number;
    priceImpact: number;
    fee: number;
    slot: number;
  }

  export interface PoolInfo {
    address: string;
    tokenA: TokenInfo;
    tokenB: TokenInfo;
    reserveA: number;
    reserveB: number;
    lpSupply: number;
    feeRate: number;
    volume24h: number;
    tvl: number;
    apy: number;
    poolType: 'AMM' | 'STABLE' | 'CONCENTRATED';
  }

  export interface TokenInfo {
    mint: string;
    symbol: string;
    name: string;
    decimals: number;
    logoURI?: string;
    price?: number;
  }

  export function getSwapAmountSaros(
    connection: any,
    fromMint: string,
    toMint: string,
    amount: number,
    slippage: number,
    poolParams?: any
  ): Promise<SwapEstimate>;

  export function swapSaros(
    connection: any,
    fromTokenAccount: any,
    toTokenAccount: any,
    amountIn: any,
    minAmountOut: any,
    referrer: any,
    poolAddress: any,
    swapProgram: any,
    payer: any,
    fromMint: any,
    toMint: any
  ): Promise<SwapResult>;

  export function genConnectionSolana(rpcUrl?: string): any;

  export function getPoolInfo(connection: any, poolAddress: any): Promise<PoolInfo>;

  export const sarosSdk: {
    SarosFarmService: any;
    SarosStakeServices: any;
  };
}




