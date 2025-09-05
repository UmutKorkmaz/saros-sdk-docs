
declare module '@saros-finance/sdk' {
  export function genConnectionSolana(): any;
  export const AMM_PROGRAM_ID: any;
  export const DLMM_PROGRAM_ID: any;
  export class SwapManager {
    constructor(config: any);
    swap(params: any): Promise<any>;
  }
  export function createSwapTransaction(params: any): Promise<any>;
  export function getQuote(params: any): Promise<any>;
}

declare module '@saros-finance/dlmm-sdk' {
  export class LiquidityBookServices {
    constructor(config: any);
  }
  export enum MODE {
    DEVNET = 'devnet',
    MAINNET = 'mainnet'
  }
}
      