/**
 * Type declarations for @saros-finance/sdk
 * Since the SDK doesn't provide TypeScript declarations, we declare it as module
 */

declare module '@saros-finance/sdk' {
  export function genConnectionSolana(): any;
  export const AMM_PROGRAM_ID: any;
  export const DLMM_PROGRAM_ID: any;
  export const SWAP_PROGRAM_ID: any;
  export const AMM_PROGRAM: any;
  export const DLMM_PROGRAM: any;
  export const SWAP_PROGRAM: any;
  export const SAROS_AMM_PROGRAM: any;
  export const SAROS_DLMM_PROGRAM: any;
  export const SAROS_SWAP_PROGRAM: any;
  export const SOL_MINT: any;
  export const USDC_MINT: any;
  export const USDT_MINT: any;
  export const SRM_MINT: any;
  export const RAY_MINT: any;
  export const SABER_MINT: any;
  export const ORCA_MINT: any;
  export const STEP_MINT: any;
  export const ROPE_MINT: any;
  export const COPE_MINT: any;
  export const MAPS_MINT: any;
  export const FIDA_MINT: any;
  export const KIN_MINT: any;
  export const ALEPH_MINT: any;
  export const TULIP_MINT: any;
  export const SNY_MINT: any;
  export const SLRS_MINT: any;
  export const SAMO_MINT: any;
  export const UXD_MINT: any;
  export const JET_MINT: any;
  export const MNGO_MINT: any;
  export const BONK_MINT: any;
  export const WIF_MINT: any;
  export const MEW_MINT: any;
  export const POPCAT_MINT: any;
  export const JTO_MINT: any;
  export const PYTH_MINT: any;
  export const JUP_MINT: any;
  export const W_MINT: any;
  export const RENDER_MINT: any;
  export const HNT_MINT: any;
  
  export class SwapManager {
    constructor(config: any);
    swap(params: any): Promise<any>;
  }
  
  export class AMM {
    constructor(connection: any);
    swap(params: any): Promise<any>;
    quote(params: any): Promise<any>;
  }
  
  export class DLMM {
    constructor(connection: any);
    swap(params: any): Promise<any>;
    quote(params: any): Promise<any>;
  }
  
  export function createSwapTransaction(params: any): Promise<any>;
  export function createLiquidityTransaction(params: any): Promise<any>;
  export function getPoolInfo(poolAddress: any): Promise<any>;
  export function getQuote(params: any): Promise<any>;
  export function getAllPools(): Promise<any>;
  export function getTokenList(): Promise<any>;
}