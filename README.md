# NFT Marketplace on Solana

A decentralized NFT marketplace built on Solana using the Anchor framework, supporting Token-2022 standard NFTs. This marketplace enables users to create auctions, place bids, and finalize sales in a trustless, on-chain environment.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [User Stories](#user-stories)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Usage](#usage)
- [Program Structure](#program-structure)
- [Testing](#testing)
- [Deployment](#deployment)
- [Security Considerations](#security-considerations)
- [Contributing](#contributing)
- [License](#license)

## Overview

This NFT marketplace is a Solana program that facilitates auction-based trading of NFTs. It leverages Solana's high throughput and low transaction costs to provide a seamless experience for NFT sellers and buyers. The program uses Token-2022 standard, which provides enhanced token functionality compared to the standard SPL Token program.

### Key Capabilities

- **Auction Creation**: Sellers can create time-limited auctions for their NFTs
- **Bidding System**: Buyers can place bids, with automatic refunds for outbid participants
- **Cooldown Extension**: Auctions automatically extend when bids are placed near the end time
- **Automatic Settlement**: Winners receive NFTs and sellers receive payment upon finalization
- **No-Bid Handling**: NFTs are returned to sellers if no bids are placed

## Features

### 🎯 Core Features

- **Token-2022 Support**: Full compatibility with Token-2022 standard NFTs
- **Program Derived Addresses (PDAs)**: Secure, deterministic account management
- **Automatic Refunds**: Previous bidders are automatically refunded when outbid
- **Cooldown Mechanism**: Prevents last-second sniping by extending auction time
- **Trustless Escrow**: NFTs are held in a program-controlled vault during auctions
- **Flexible Auction Parameters**: Configurable starting bid, duration, and cooldown periods

### 🔒 Security Features

- **Account Validation**: Comprehensive constraints ensure account ownership and validity
- **Reentrancy Protection**: State is marked inactive before transfers to prevent reentrancy
- **Bid Validation**: Ensures bids are higher than current highest bid
- **Time-based Validation**: Prevents actions on inactive or expired auctions

## User Stories

### As a Seller

1. **As a seller, I want to create an auction for my NFT** so that I can sell it to the highest bidder
   - I can specify a starting bid amount
   - I can set the auction duration
   - I can set a cooldown period to prevent last-second sniping
   - My NFT is securely held in escrow during the auction

2. **As a seller, I want to receive payment automatically** when my auction ends
   - Payment is transferred directly to my wallet
   - No manual intervention required
   - If no bids are placed, my NFT is returned to me

3. **As a seller, I want to see the current highest bid** so I can track my auction's progress
   - I can query the auction state at any time
   - I can see who the current highest bidder is

### As a Buyer

1. **As a buyer, I want to place bids on NFTs** so I can participate in auctions
   - I can place a bid higher than the current highest bid
   - My bid is automatically recorded on-chain
   - I can see if my bid is the current highest

2. **As a buyer, I want to be refunded automatically** if I'm outbid
   - When someone places a higher bid, my previous bid is automatically refunded
   - I don't need to manually claim my refund
   - I can immediately use the refunded funds for other bids

3. **As a buyer, I want to win the NFT** when I place the highest bid and the auction ends
   - The NFT is automatically transferred to my wallet
   - I receive the NFT in my associated token account
   - The transaction is atomic and trustless

4. **As a buyer, I want protection from last-second sniping** so I have time to respond to bids
   - When a bid is placed near the auction end time, the auction extends by the cooldown period
   - This gives me a fair chance to place a counter-bid

### As a Developer

1. **As a developer, I want to integrate this marketplace** into my application
   - The program provides a clear IDL for type-safe integration
   - All instructions are well-documented
   - Test suite demonstrates usage patterns

2. **As a developer, I want to query auction state** to display information to users
   - Auction accounts are publicly readable
   - All state is stored on-chain and queryable

## Architecture

### System Architecture Diagram

![alt text](arch_diagram.png)

### Account Relationships

```
┌──────────────────────────────────────────────────────────────┐
│                    Auction Lifecycle                         │
└──────────────────────────────────────────────────────────────┘

1. CREATE AUCTION
   ┌──────────┐
   │  Seller  │──┐
   └──────────┘  │
                 │  create_auction()
                 ▼
   ┌─────────────────────────────┐
   │      Auction PDA            │
   │  ┌───────────────────────┐  │
   │  │ seller: Pubkey        │  │
   │  │ mint: Pubkey          │  │
   │  │ highest_bid: u64      │  │
   │  │ highest_bidder: Pubkey│  │
   │  │ start_time: i64       │  │
   │  │ end_time: i64         │  │
   │  │ cooldown: i64         │  │
   │  │ is_active: bool       │  │
   │  └───────────────────────┘  │
   └─────────────────────────────┘
                 │
                 │  NFT Transfer
                 ▼
   ┌─────────────────────────────┐
   │      Vault PDA              │
   │  (Token-2022 Account)       │
   │  Holds NFT during auction   │
   └─────────────────────────────┘

2. PLACE BID
   ┌──────────┐
   │  Bidder  │──┐
   └──────────┘  │
                 │  place_bid(amount)
                 ▼
   ┌─────────────────────────────┐
   │      Auction PDA            │
   │  • Updates highest_bid      │
   │  • Updates highest_bidder   │
   │  • Extends end_time if      │
   │    needed (cooldown)        │
   └─────────────────────────────┘
                 │
                 │  Refund previous bidder
                 ▼
   ┌─────────────────────────────┐
   │  Previous Bidder (if any)   │
   │  Receives refund            │
   └─────────────────────────────┘

3. FINALIZE AUCTION
                 │
                 │  finalize_auction()
                 ▼
   ┌─────────────────────────────┐
   │      Auction PDA            │
   │  • Marks is_active = false  │
   │  • Validates winner         │
   └─────────────────────────────┘
                 │
        ┌────────┴────────┐
        │                 │
        ▼                 ▼
   ┌──────────┐     ┌──────────┐
   │  Winner  │     │  Seller  │
   │ Receives │     │ Receives │
   │   NFT    │     │ Payment  │
   └──────────┘     └──────────┘
```

### Instruction Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Instruction Flow                         │
└─────────────────────────────────────────────────────────────┘

create_auction()
├── Validate accounts (seller, mint, token account)
├── Initialize Auction PDA
│   └── Seeds: [b"auction", seller, mint]
├── Create Vault PDA
│   └── Seeds: [b"vault", auction_pda]
├── Initialize Vault Token Account
└── Transfer NFT: seller → vault
    └── Amount: 1 (NFT)

place_bid()
├── Validate auction (active, not ended)
├── Validate bid amount (higher than current)
├── Transfer SOL: bidder → auction_pda
├── Refund previous bidder (if exists)
│   └── Transfer SOL: auction_pda → previous_bidder
├── Update auction state
│   ├── highest_bid = new_amount
│   ├── highest_bidder = bidder
│   └── Extend end_time if needed (cooldown)
└── Return success

finalize_auction()
├── Validate auction (active, ended)
├── Mark auction inactive (prevent reentrancy)
├── Check if there's a winner
│   ├── No winner → Return NFT to seller
│   └── Has winner:
│       ├── Transfer NFT: vault → winner_ata
│       └── Transfer SOL: auction_pda → seller
└── Return success
```

### Data Structures

```
Auction Account Structure:
┌──────────────────────────────────────────┐
│ Field           │ Type      │ Size(bytes)│
├─────────────────┼───────────┼────────────┤
│ seller          │ Pubkey    │ 32         │
│ mint            │ Pubkey    │ 32         │
│ highest_bidder  │ Pubkey    │ 32         │
│ highest_bid     │ u64       │ 8          │
│ start_time      │ i64       │ 8          │
│ end_time        │ i64       │ 8          │
│ cooldown        │ i64       │ 8          │
│ is_active       │ bool      │ 1          │
│ bump            │ u8        │ 1          │
│ Discriminator   │           │ 8          │
└─────────────────┴───────────┴────────────┘
Total: ~146 bytes
```

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (latest stable version): [Install Rust](https://www.rust-lang.org/tools/install)
- **Solana CLI** (v1.18+): [Install Solana](https://docs.solana.com/cli/install-solana-cli-tools)
- **Anchor Framework** (v0.32.1+): [Install Anchor](https://www.anchor-lang.com/docs/installation)
- **Node.js** (v18+): [Install Node.js](https://nodejs.org/)
- **Yarn** or **npm**: Package manager

### Verify Installation

```bash
rustc --version
solana --version
anchor --version
node --version
```

## Installation

1. **Clone the repository** (if not already done):
   ```bash
   git clone <repository-url>
   cd nft-marketplace
   ```

2. **Install dependencies**:
   ```bash
   yarn install
   # or
   npm install
   ```

3. **Build the program**:
   ```bash
   anchor build
   ```

4. **Generate TypeScript types**:
   ```bash
   anchor build
   ```
   This will generate TypeScript types in `target/types/`

## Usage

### Setting Up Local Validator

1. **Start a local Solana validator**:
   ```bash
   solana-test-validator
   ```

2. **In a new terminal, set the cluster to localnet**:
   ```bash
   solana config set --url localhost
   ```

3. **Airdrop SOL to your wallet** (for testing):
   ```bash
   solana airdrop 10
   ```

### Running Tests

The test suite demonstrates the complete auction lifecycle:

```bash
anchor test
```

This will:
1. Create a Token-2022 NFT mint
2. Mint an NFT to the seller
3. Create an auction
4. Place bids
5. Finalize the auction

### Program Instructions

#### 1. Create Auction

```typescript
await program.methods
  .createAuction(
    new BN(1_000_000),  // starting_bid (in lamports)
    new BN(3600),       // duration_seconds
    new BN(300)         // cooldown_seconds
  )
  .accounts({
    auction: auctionPda,
    seller: seller.publicKey,
    nftMint: nftMint,
    nftVault: vaultPda,
    sellerTokenAccount: sellerTokenAccount,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
    rent: anchor.web3.SYSVAR_RENT_PUBKEY,
  })
  .signers([seller])
  .rpc();
```

#### 2. Place Bid

```typescript
await program.methods
  .placeBid(new BN(1_500_000))  // bid amount in lamports
  .accounts({
    auction: auctionPda,
    bidder: bidder.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .signers([bidder])
  .rpc();
```

#### 3. Finalize Auction

```typescript
await program.methods
  .finalizeAuction()
  .accounts({
    auction: auctionPda,
    seller: seller.publicKey,
    winner: winner.publicKey,
    nftVault: vaultPda,
    winnerAta: winnerTokenAccount,
    sellerTokenAccount: sellerTokenAccount,
    nftMint: nftMint,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### Key Components

- **`lib.rs`**: Contains all program logic
  - `create_auction`: Initializes auction and transfers NFT to vault
  - `place_bid`: Handles bidding with automatic refunds
  - `finalize_auction`: Settles auction and transfers assets

- **Account Contexts**:
  - `CreateAuction`: Validates accounts for auction creation
  - `PlaceBid`: Validates accounts for placing bids
  - `FinalizeAuction`: Validates accounts for auction finalization

- **State**:
  - `Auction`: On-chain auction state structure

- **Errors**:
  - `AuctionError`: Custom error codes for various failure scenarios

## Testing

The test suite (`tests/nft-marketplace.ts`) covers:

1. **Setup**: Creating Token-2022 mint and minting NFT
2. **Auction Creation**: Creating an auction with specified parameters
3. **Bidding**: Placing bids and verifying state updates
4. **Finalization**: Completing auction and verifying asset transfers

Run tests with:
```bash
anchor test
```

## Deployment

### Deploy to Localnet

```bash
anchor deploy
```

### Deploy to Devnet

1. **Update `Anchor.toml`**:
   ```toml
   [provider]
   cluster = "devnet"
   ```

2. **Set your wallet**:
   ```bash
   solana config set --url devnet
   solana airdrop 2  # Get some devnet SOL
   ```

3. **Deploy**:
   ```bash
   anchor deploy
   ```

### Deploy to Mainnet

⚠️ **Warning**: Only deploy to mainnet after thorough testing and security audits.

1. **Update `Anchor.toml`**:
   ```toml
   [provider]
   cluster = "mainnet-beta"
   ```

2. **Deploy**:
   ```bash
   anchor deploy
   ```

## Security Considerations

### Current Security Features

- ✅ Account ownership validation
- ✅ Reentrancy protection (state marked inactive before transfers)
- ✅ Bid amount validation
- ✅ Time-based validation
- ✅ PDA-based account management

### Best Practices

1. **Always validate accounts** on the client side before submitting transactions
2. **Handle errors gracefully** - check for all possible error codes
3. **Monitor auction state** - ensure auctions are finalized after end time
4. **Test thoroughly** - use the test suite and additional integration tests
5. **Audit before mainnet** - consider professional security audits

### Known Limitations

- No fee mechanism (all proceeds go to seller)
- No minimum bid increment enforcement
- No maximum bid limit
- Cooldown extension is automatic (no configurable threshold)

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow Rust and Anchor best practices
- Write tests for new features
- Update documentation as needed
- Ensure all tests pass before submitting PR

## Resources

- [Anchor Documentation](https://www.anchor-lang.com/)
- [Solana Documentation](https://docs.solana.com/)
- [Token-2022 Program](https://spl.solana.com/token-2022)
- [Solana Cookbook](https://solanacookbook.com/)

## Support

For issues, questions, or contributions, please open an issue on the repository.

---

**Built with ❤️ using Anchor and Solana**

