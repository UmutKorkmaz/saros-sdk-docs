# Saros Architecture Diagrams

Visual representations of Saros Finance protocol architecture and transaction flows.

## Table of Contents

- [System Architecture](#system-architecture)
- [AMM Swap Flow](#amm-swap-flow)
- [DLMM Architecture](#dlmm-architecture)
- [DLMM Liquidity Provision](#dlmm-liquidity-provision)
- [Auto-Compound Flow](#auto-compound-flow)
- [Multi-Hop Routing](#multi-hop-routing)
- [Yield Farming Architecture](#yield-farming-architecture)
- [SDK Integration Flow](#sdk-integration-flow)

---

## System Architecture

Overall Saros Finance protocol architecture showing all components.

```mermaid
graph TB
    subgraph "Frontend Applications"
        UI[Web UI]
        Mobile[Mobile App]
        CLI[CLI Tools]
    end
    
    subgraph "SDK Layer"
        TS[TypeScript SDK]
        DLMM[DLMM SDK]
        Rust[Rust SDK]
    end
    
    subgraph "Saros Protocol"
        AMM[AMM Program]
        DLMMProg[DLMM Program]
        Staking[Staking Program]
        Farming[Farming Program]
        Router[Router Program]
    end
    
    subgraph "Solana Blockchain"
        Accounts[Token Accounts]
        Oracle[Price Oracles]
        SPL[SPL Token Program]
        Serum[Serum DEX]
    end
    
    UI --> TS
    Mobile --> TS
    CLI --> Rust
    
    TS --> AMM
    TS --> Staking
    TS --> Farming
    
    DLMM --> DLMMProg
    Rust --> DLMMProg
    
    AMM --> Router
    DLMMProg --> Router
    
    Router --> Accounts
    Router --> SPL
    DLMMProg --> Oracle
    AMM --> Serum
    
    style UI fill:#e1f5fe
    style Mobile fill:#e1f5fe
    style CLI fill:#e1f5fe
    style TS fill:#fff3e0
    style DLMM fill:#fff3e0
    style Rust fill:#fff3e0
    style AMM fill:#f3e5f5
    style DLMMProg fill:#f3e5f5
    style Staking fill:#f3e5f5
    style Farming fill:#f3e5f5
    style Router fill:#f3e5f5
```

---

## AMM Swap Flow

Traditional AMM swap execution flow.

```mermaid
sequenceDiagram
    participant User
    participant SDK
    participant AMM as AMM Program
    participant Pool
    participant TokenA as Token A Account
    participant TokenB as Token B Account
    participant Oracle
    
    User->>SDK: Initiate Swap
    SDK->>Oracle: Get Price Feed
    Oracle-->>SDK: Current Prices
    SDK->>SDK: Calculate Quote
    SDK->>SDK: Check Slippage
    
    SDK->>AMM: Create Swap Instruction
    AMM->>Pool: Check Reserves
    Pool-->>AMM: Reserve Balances
    AMM->>AMM: Calculate Output
    
    AMM->>TokenA: Transfer In
    TokenA-->>Pool: Tokens Deposited
    
    AMM->>Pool: Update Reserves
    Pool->>TokenB: Transfer Out
    TokenB-->>User: Tokens Received
    
    AMM-->>SDK: Transaction Result
    SDK-->>User: Swap Complete
    
    Note over Pool: K = X * Y invariant maintained
```

---

## DLMM Architecture

Dynamic Liquidity Market Maker (DLMM) bin-based architecture.

```mermaid
graph TB
    subgraph "DLMM Pool Structure"
        ActiveBin[Active Bin<br/>Current Trading Price]
        
        subgraph "Price Bins"
            Bin1[Bin -2<br/>$45.00]
            Bin2[Bin -1<br/>$47.50]
            ActiveBin
            Bin3[Bin +1<br/>$52.50]
            Bin4[Bin +2<br/>$55.00]
        end
        
        subgraph "Liquidity Distribution"
            L1[Liquidity]
            L2[Liquidity]
            L3[Active Liquidity]
            L4[Liquidity]
            L5[Liquidity]
        end
        
        subgraph "Token Composition"
            T1[100% Token Y]
            T2[75% Y / 25% X]
            T3[50% X / 50% Y]
            T4[25% Y / 75% X]
            T5[100% Token X]
        end
    end
    
    Bin1 --> L1
    Bin2 --> L2
    ActiveBin --> L3
    Bin3 --> L4
    Bin4 --> L5
    
    L1 --> T1
    L2 --> T2
    L3 --> T3
    L4 --> T4
    L5 --> T5
    
    style ActiveBin fill:#4caf50,color:#fff
    style L3 fill:#8bc34a
```

---

## DLMM Liquidity Provision

Flow for providing concentrated liquidity in DLMM.

```mermaid
flowchart LR
    Start([User Provides Liquidity])
    
    Start --> SelectRange{Select Price Range}
    
    SelectRange --> Narrow[Narrow Range<br/>High Capital Efficiency<br/>Higher IL Risk]
    SelectRange --> Wide[Wide Range<br/>Lower Capital Efficiency<br/>Lower IL Risk]
    
    Narrow --> Distribution{Choose Distribution}
    Wide --> Distribution
    
    Distribution --> Uniform[Uniform<br/>Even across bins]
    Distribution --> Normal[Normal<br/>Concentrated at center]
    Distribution --> Spot[Spot<br/>All at active bin]
    Distribution --> BidAsk[Bid-Ask<br/>Split at edges]
    
    Uniform --> Calculate[Calculate Bin Allocation]
    Normal --> Calculate
    Spot --> Calculate
    BidAsk --> Calculate
    
    Calculate --> Deposit[Deposit Tokens]
    
    Deposit --> Mint[Mint LP NFT]
    
    Mint --> Monitor[Monitor Position]
    
    Monitor --> InRange{In Range?}
    
    InRange -->|Yes| Earn[Earn Fees]
    InRange -->|No| Rebalance[Consider Rebalancing]
    
    Earn --> Monitor
    Rebalance --> SelectRange
    
    style Start fill:#e3f2fd
    style Mint fill:#c8e6c9
    style Earn fill:#fff9c4
```

---

## Auto-Compound Flow

Automated yield farming and reinvestment flow.

```mermaid
stateDiagram-v2
    [*] --> Monitoring: Start Auto-Compound
    
    Monitoring --> CheckRewards: Check Interval
    
    CheckRewards --> BelowThreshold: Rewards < Threshold
    CheckRewards --> AboveThreshold: Rewards >= Threshold
    
    BelowThreshold --> Monitoring: Wait
    
    AboveThreshold --> CheckGas: Check Gas Price
    
    CheckGas --> GasHigh: Gas Too High
    CheckGas --> GasOK: Gas Acceptable
    
    GasHigh --> Monitoring: Delay
    
    GasOK --> Harvest: Harvest Rewards
    
    Harvest --> Calculate: Calculate Reinvest Amount
    
    Calculate --> Swap: Swap to LP Tokens
    
    Swap --> AddLiquidity: Add to Position
    
    AddLiquidity --> UpdateStats: Update Statistics
    
    UpdateStats --> Notify: Send Notification
    
    Notify --> Monitoring: Continue Monitoring
    
    note right of CheckRewards
        Configurable threshold
        prevents small compounds
    end note
    
    note right of CheckGas
        Ensures profitability
        after gas costs
    end note
```

---

## Multi-Hop Routing

Optimal path finding for token swaps.

```mermaid
graph LR
    subgraph "Direct Route"
        A1[Token A] -->|Pool 1| B1[Token B]
    end
    
    subgraph "2-Hop Route"
        A2[Token A] -->|Pool 2| USDC1[USDC]
        USDC1 -->|Pool 3| B2[Token B]
    end
    
    subgraph "3-Hop Route"
        A3[Token A] -->|Pool 4| SOL[SOL]
        SOL -->|Pool 5| USDC2[USDC]
        USDC2 -->|Pool 6| B3[Token B]
    end
    
    subgraph "Route Selection"
        Compare[Compare Routes]
        Compare --> Best[Best Route<br/>Lowest Price Impact<br/>Highest Output]
    end
    
    B1 --> Compare
    B2 --> Compare
    B3 --> Compare
    
    style Best fill:#4caf50,color:#fff
```

---

## Yield Farming Architecture

Complete yield farming ecosystem flow.

```mermaid
flowchart TB
    subgraph "User Actions"
        Deposit[Deposit LP Tokens]
        Stake[Stake in Farm]
        Harvest[Harvest Rewards]
        Compound[Auto-Compound]
        Withdraw[Withdraw]
    end
    
    subgraph "Farming Program"
        FarmPool[Farm Pool]
        RewardCalc[Reward Calculator]
        Distributor[Reward Distributor]
        
        FarmPool --> RewardCalc
        RewardCalc --> Distributor
    end
    
    subgraph "Rewards"
        SAROS[SAROS Tokens]
        Bonus[Bonus Rewards]
        Fees[Trading Fees]
    end
    
    subgraph "Strategies"
        SingleFarm[Single Farm]
        MultiPool[Multi-Pool Strategy]
        Leveraged[Leveraged Farming]
    end
    
    Deposit --> FarmPool
    Stake --> FarmPool
    
    Distributor --> SAROS
    Distributor --> Bonus
    FarmPool --> Fees
    
    SAROS --> Harvest
    Bonus --> Harvest
    Fees --> Harvest
    
    Harvest --> Compound
    Compound --> FarmPool
    
    SingleFarm --> FarmPool
    MultiPool --> FarmPool
    Leveraged --> FarmPool
    
    FarmPool --> Withdraw
    
    style SAROS fill:#ffd54f
    style Compound fill:#81c784
```

---

## SDK Integration Flow

How developers integrate with Saros SDKs.

```mermaid
sequenceDiagram
    participant Dev as Developer
    participant App as dApp
    participant SDK
    participant Protocol as Saros Protocol
    participant Chain as Solana
    participant User as End User
    
    Dev->>SDK: Install SDK
    Dev->>SDK: Initialize with Config
    SDK-->>Dev: SDK Instance
    
    Dev->>App: Integrate SDK
    
    User->>App: Connect Wallet
    App->>SDK: Create Connection
    SDK->>Chain: Verify Wallet
    Chain-->>SDK: Wallet Confirmed
    SDK-->>App: Connection Ready
    
    User->>App: Request Swap
    App->>SDK: Prepare Transaction
    SDK->>Protocol: Get Quote
    Protocol-->>SDK: Quote Details
    
    SDK->>SDK: Build Transaction
    SDK-->>App: Transaction Ready
    
    App->>User: Request Signature
    User->>App: Sign Transaction
    
    App->>SDK: Send Transaction
    SDK->>Chain: Submit to Network
    Chain->>Protocol: Execute Swap
    Protocol-->>Chain: Swap Complete
    Chain-->>SDK: Confirmation
    SDK-->>App: Success Result
    App-->>User: Display Result
```

---

## DLMM Position Lifecycle

Complete lifecycle of a DLMM liquidity position.

```mermaid
stateDiagram-v2
    [*] --> Research: Analyze Pool
    
    Research --> SelectRange: Choose Price Range
    note right of Research
        - Check volatility
        - Analyze volume
        - Review fees
    end note
    
    SelectRange --> SetDistribution: Set Liquidity Shape
    
    SetDistribution --> CreatePosition: Create Position
    
    CreatePosition --> Active: Position Active
    
    Active --> InRange: Price In Range
    Active --> OutOfRange: Price Out of Range
    
    InRange --> EarnFees: Earning Fees
    EarnFees --> Monitor: Monitor Performance
    
    OutOfRange --> NoFees: Not Earning Fees
    NoFees --> ConsiderAction: Evaluate Options
    
    Monitor --> Active
    
    ConsiderAction --> Rebalance: Rebalance Position
    ConsiderAction --> Wait: Wait for Price Return
    ConsiderAction --> Close: Close Position
    
    Rebalance --> SelectRange
    Wait --> Active
    
    Close --> Withdraw: Withdraw Liquidity
    Withdraw --> ClaimRewards: Claim Rewards
    ClaimRewards --> [*]
    
    note right of Active
        Track:
        - Fees earned
        - IL incurred
        - APR achieved
    end note
```

---

## Transaction Flow with Error Handling

Complete transaction flow including error scenarios.

```mermaid
flowchart TB
    Start([User Initiates Transaction])
    
    Start --> Validate{Validate Input}
    
    Validate -->|Invalid| Error1[Show Error]
    Error1 --> Start
    
    Validate -->|Valid| Simulate[Simulate Transaction]
    
    Simulate --> SimResult{Simulation OK?}
    
    SimResult -->|Failed| Error2[Show Simulation Error]
    Error2 --> Adjust[Adjust Parameters]
    Adjust --> Start
    
    SimResult -->|Success| Build[Build Transaction]
    
    Build --> Sign[Request Signature]
    
    Sign --> UserSign{User Signs?}
    
    UserSign -->|Rejected| Cancel[Transaction Cancelled]
    UserSign -->|Approved| Send[Send Transaction]
    
    Send --> Confirm[Await Confirmation]
    
    Confirm --> Result{Confirmed?}
    
    Result -->|Failed| Retry{Retry?}
    Retry -->|Yes| Send
    Retry -->|No| Failed[Transaction Failed]
    
    Result -->|Success| Success[Transaction Success]
    
    Success --> Update[Update UI]
    Failed --> Update
    Cancel --> Update
    
    Update --> End([Complete])
    
    style Success fill:#4caf50,color:#fff
    style Failed fill:#f44336,color:#fff
    style Cancel fill:#ff9800,color:#fff
```

---

## Oracle Integration

Price oracle integration for accurate pricing.

```mermaid
graph TB
    subgraph "Price Sources"
        Pyth[Pyth Network]
        Switch[Switchboard]
        Chain[Chainlink]
        Internal[Internal TWAP]
    end
    
    subgraph "Aggregation Layer"
        Aggregator[Price Aggregator]
        Validator[Price Validator]
        Cache[Price Cache]
    end
    
    subgraph "Consumers"
        AMM[AMM Pools]
        DLMM[DLMM Pools]
        Liquidation[Liquidation Engine]
        UI[User Interface]
    end
    
    Pyth --> Aggregator
    Switch --> Aggregator
    Chain --> Aggregator
    Internal --> Aggregator
    
    Aggregator --> Validator
    Validator --> Cache
    
    Cache --> AMM
    Cache --> DLMM
    Cache --> Liquidation
    Cache --> UI
    
    Validator -.->|Fallback| Internal
    
    style Aggregator fill:#e1bee7
    style Cache fill:#c5e1a5
```

---

## Liquidity Migration Flow

Migrating liquidity between different pool types.

```mermaid
flowchart LR
    subgraph "Source"
        AMM[AMM Position]
        V2[V2 Pool]
        Comp[Competitor DEX]
    end
    
    subgraph "Migration Process"
        Remove[Remove Liquidity]
        Convert[Convert Tokens]
        Calculate[Calculate Optimal Range]
        Deposit[Deposit to DLMM]
    end
    
    subgraph "Destination"
        DLMM[DLMM Position]
        Benefits[Benefits:<br/>• Higher Capital Efficiency<br/>• Better Fee APR<br/>• Concentrated Liquidity]
    end
    
    AMM --> Remove
    V2 --> Remove
    Comp --> Remove
    
    Remove --> Convert
    Convert --> Calculate
    Calculate --> Deposit
    Deposit --> DLMM
    DLMM --> Benefits
    
    style DLMM fill:#4caf50,color:#fff
    style Benefits fill:#e8f5e9
```

---

## Notes

- All diagrams are rendered using Mermaid.js
- Colors indicate different system layers
- Arrows show data/control flow
- Dotted lines indicate optional or fallback paths

These diagrams provide a visual understanding of:
1. Overall system architecture
2. Transaction flows
3. DLMM's unique bin-based system
4. Complex operations like auto-compounding
5. Integration patterns for developers

For interactive versions, paste these diagrams into any Mermaid-compatible viewer or documentation system.