import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";

import {
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createMint,
    getOrCreateAssociatedTokenAccount,
    mintTo,
    getAccount,
    createAssociatedTokenAccountIdempotentInstruction,
    createInitializeMint2Instruction,
    createMintToInstruction,
    getAssociatedTokenAddressSync,
    getMinimumBalanceForRentExemptMint,
    MINT_SIZE,
} from "@solana/spl-token";

import {
    PublicKey,
    Keypair,
    SystemProgram,
    LAMPORTS_PER_SOL,
    Transaction,
} from "@solana/web3.js";
import { randomBytes } from "crypto";
import { nft_marketplace_token2022 } from "../../target/types/nft_marketplace_token2022";

describe("nft marketplace — token2022", () => {

    // -----------------------------
    // Setup & global vars
    // -----------------------------
    const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
    const provider = anchor.AnchorProvider.local();
    anchor.setProvider(provider);
    const connection = provider.connection;
    const program = anchor.workspace
        .NftMarketplaceToken2022 as Program<nft_marketplace_token2022>;

    // const seller = provider.wallet.publicKey;
    // let nftMint: PublicKey;
    // let sellerTokenAccount: PublicKey;
    const tokenProgram = TOKEN_2022_PROGRAM_ID;
    let auctionPda: PublicKey;
    let auctionBump: number;

    let vaultAta: PublicKey;

      // Helper function to confirm a transaction
    const confirm = async (signature: string): Promise<string> => {
      const block = await connection.getLatestBlockhash();
      await connection.confirmTransaction({
        signature,
        ...block,
      });
      return signature;
    };

    // Helper function to log a transaction signature with a link to the explorer
    const log = async (signature: string): Promise<string> => {
      console.log(
        `Your transaction signature: https://explorer.solana.com/transaction/${signature}?cluster=custom&customUrl=${connection.rpcEndpoint}`
      );
      return signature;
    };
  
    // Generate a random seed for the escrow
    const seed = new BN(randomBytes(8));
  
    // Generate keypairs for maker, taker, and two mints
    const [seller, bidder, bidder1, bidder2, mint] = Array.from({ length: 5 }, () =>
      Keypair.generate()
    );
  
    // Derive associated token accounts for maker and taker for both mints
    const [sellerAta, bidder1Ata, bidder2Ata] = [seller, bidder1, bidder2]
      .map((a) =>
          getAssociatedTokenAddressSync(mint.publicKey, a.publicKey, false, tokenProgram)
      )
      .flat();
  
    // Derive the escrow PDA using the seed and maker's public key
    const auction = PublicKey.findProgramAddressSync(
      [Buffer.from("auction"), seller.publicKey.toBuffer(), mint.publicKey.toBuffer()],
      program.programId
    )[0];
  
    // Derive the vault associated token account for the escrow
    const [vault, vaultBump] = PublicKey.findProgramAddressSync(
      [
          Buffer.from("vault"),
          auction.toBuffer(),   // auction PDA
      ],
      program.programId
  );
    // Prepare all accounts needed for the tests
    const accounts = {
      seller: seller.publicKey,
      bidder1: bidder1.publicKey,
      bidder2: bidder2.publicKey,
      mint: mint.publicKey,
      sellerAta,
      bidder1Ata,
      bidder2Ata,
      auction,
      vault,
      tokenProgram,
    }
    //
    // Rent exemption for mint account
    //



    // -----------------------------
    // 1. Setup mint + seller NFT
    // -----------------------------
    it("SETUP: Create Token-2022 mint + mint 1 NFT to seller", async () => {
      const tx = new Transaction();
      const mintRent =  await getMinimumBalanceForRentExemptMint(connection);
      [seller, bidder1, bidder2].forEach((account) => {

        
        tx.add(
          SystemProgram.transfer({
            fromPubkey: provider.publicKey,
            toPubkey: account.publicKey,
            lamports: 1 * LAMPORTS_PER_SOL,
          })
        );
      });


      tx.add(
        SystemProgram.createAccount({
          fromPubkey: provider.publicKey,
          newAccountPubkey: mint.publicKey,
          lamports: mintRent,
          space: MINT_SIZE,
          programId: TOKEN_2022_PROGRAM_ID,
        })
      );

      tx.add(
        createInitializeMint2Instruction(
          mint.publicKey,
          0,                  // NFT: no decimals
          seller.publicKey,   // seller is mint authority
          null,
          TOKEN_2022_PROGRAM_ID
        )
      );

      tx.add(
        createAssociatedTokenAccountIdempotentInstruction(
          provider.publicKey,
          sellerAta,
          seller.publicKey,
          mint.publicKey,
          TOKEN_2022_PROGRAM_ID
        )
      );
    
      tx.add(
        createMintToInstruction(
          mint.publicKey,
          sellerAta,
          seller.publicKey,
          1,
          [],
          TOKEN_2022_PROGRAM_ID
        )
      );
    
      await provider.sendAndConfirm(tx, [mint, seller]);
     
    });

    // -----------------------------
    // 2. Create Auction
    // -----------------------------
    it("Create Auction", async () => {

        await program.methods
            .createAuction(
                new BN(1_000_000),  // starting bid
                new BN(5),         // duration
                new BN(10)          // cooldown
            )
            .accounts({
              auction: auction,
              seller: seller.publicKey,
              nftMint: mint.publicKey,          // MUST match your Rust name
              nftVault: vault, // PDA vault you created manually
              sellerTokenAccount: sellerAta,   // 🟢 FIXED
              associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
              tokenProgram: TOKEN_2022_PROGRAM_ID,
              systemProgram: SystemProgram.programId,
              rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          })
          .signers([ seller ])
            .rpc();

        const state = await program.account.auction.fetch(auction);
        console.log("Auction created:", state);
    });

    // -----------------------------
    // 3. Place Bid
    // -----------------------------
    it("Place Bid", async () => {
      
  
      // Airdrop SOL to bidder
      await provider.connection.confirmTransaction(
          await provider.connection.requestAirdrop(bidder.publicKey, 5e9),
          "confirmed"
      );
  
      // Fetch current auction state
      const auctionData = await program.account.auction.fetch(auction);
  
      // Determine previous bidder (or none)
      const previousBidder = auctionData.highestBidder.equals(PublicKey.default)
          ? PublicKey.default      // first bid → no previous bidder
          : auctionData.highestBidder;
  
      await program.methods
          .placeBid(new BN(1_500_000))
          .accounts({
              auction: auction,           // MUST be the PDA, not auction struct
              bidder: bidder.publicKey,      // signer placing bid
              // previousBidder: previousBidder, 
              systemProgram: SystemProgram.programId,
          })
          .signers([bidder])
          .rpc();
  
      // Fetch updated auction after the bid
      // const updatedAuction = await program.account.auction.fetch(auction);
      // console.log("highest bid:", updatedAuction.highestBid.toString());
      // console.log("highest bidder:", updatedAuction.highestBidder.toBase58());
  });

    // -----------------------------
    // 4. Finalize Auction
    // -----------------------------
    it("Finalize Auction (winner receives NFT)", async () => {
        const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
        await sleep(10000);
        const auctionData = await program.account.auction.fetch(auction);
        const winner = auctionData.highestBidder;
        const sellerSol = await provider.connection.getBalance(seller.publicKey);
        console.log("Seller SOL balance:", sellerSol.toString());
        console.log("Winner:", winner.toBase58());
        const winnerAta = getAssociatedTokenAddressSync(mint.publicKey, winner, false, tokenProgram);
        const ataInfo = await provider.connection.getAccountInfo(winnerAta);
        if (!ataInfo) {
          const tx = new Transaction().add(
            createAssociatedTokenAccountIdempotentInstruction(
              provider.wallet.publicKey,       // payer
              winnerAta,                       // ATA
              winner,                          // owner
              mint.publicKey,                  // mint
              TOKEN_2022_PROGRAM_ID,           // token-2022 program
              ASSOCIATED_TOKEN_PROGRAM_ID
            )
          );

        await provider.sendAndConfirm(tx, []);
        // Winner ATA (ensure exists)
        }
        console.log("Created winner ATA:", winnerAta.toBase58());
        // Finalize auction
        await program.methods
        .finalizeAuction()
        .accounts({
          auction: auction,
          seller: seller.publicKey,
          winner: winner,
          nftVault: vault,
          winnerAta: winnerAta,
          sellerTokenAccount: sellerAta,   // <-- required!
          nftMint: mint.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

        const nftAcc = await getAccount(
            provider.connection,
            winnerAta,
            undefined,
            TOKEN_2022_PROGRAM_ID
        );

        console.log("Winner NFT balance:", nftAcc.amount.toString());
        const sellerSol_after = await provider.connection.getBalance(seller.publicKey);
        console.log("Seller SOL balance:", sellerSol_after.toString());
    });
});
