import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";

import {
  TOKEN_2022_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
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
  const sleep = (ms: number) =>
    new Promise((resolve) => setTimeout(resolve, ms));
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);
  const connection = provider.connection;
  const program = anchor.workspace
    .NftMarketplaceToken2022 as Program<nft_marketplace_token2022>;
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
  const [seller, bidder, bidder1, bidder2, mint] = Array.from(
    { length: 5 },
    () => Keypair.generate()
  );

  // Derive associated token accounts for maker and taker for both mints
  const sellerAta = getAssociatedTokenAddressSync(
    mint.publicKey,
    seller.publicKey,
    false,
    tokenProgram
  );

  // const auctionId = new BN(1);
  const auctionId = new BN(Date.now());
  // Derive the escrow PDA using the seed and maker's public key
  const auction = PublicKey.findProgramAddressSync(
    [
      Buffer.from("auction"),
      seller.publicKey.toBuffer(),
      mint.publicKey.toBuffer(),
      auctionId.toArrayLike(Buffer, "le", 8),
    ], // unique seed],
    program.programId
  )[0];

  // Derive the vault associated token account for the escrow
  const [vault, vaultBump] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("vault"),
      auction.toBuffer(), // auction PDA
    ],
    program.programId
  );

  // -----------------------------
  // 1. Setup mint + seller NFT
  // -----------------------------
  it("SETUP: Create Token-2022 mint + mint 1 NFT to seller", async () => {
    const tx = new Transaction();
    const mintRent = await getMinimumBalanceForRentExemptMint(connection);
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
        0, // NFT: no decimals
        seller.publicKey, // seller is mint authority
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

  //   // -----------------------------
  //   // 2. Create Auction
  //   // -----------------------------
  //   it("Create Auction", async () => {

  //       await program.methods
  //           .createAuction(
  //               new BN(auctionId), // start time (15 seconds from now)
  //               new BN(1_000_000),  // starting bid
  //               new BN(5),         // duration
  //               new BN(10)          // cooldown
  //           )
  //           .accounts({
  //             auction: auction,
  //             seller: seller.publicKey,
  //             nftMint: mint.publicKey,
  //             nftVault: vault,
  //             sellerTokenAccount: sellerAta,
  //             associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //             tokenProgram: TOKEN_2022_PROGRAM_ID,
  //             systemProgram: SystemProgram.programId,
  //             rent: anchor.web3.SYSVAR_RENT_PUBKEY,
  //         })
  //         .signers([ seller ])
  //           .rpc();

  //       const state = await program.account.auction.fetch(auction);
  //       console.log("Auction created:", state);
  //   });

  //   // -----------------------------
  //   // 3. Place Bid
  //   // -----------------------------
  //   it("Place Bid", async () => {

  //     // Airdrop SOL to bidder
  //     await provider.connection.confirmTransaction(
  //         await provider.connection.requestAirdrop(bidder.publicKey, 5e9),
  //         "confirmed"
  //     );

  //     await program.methods
  //         .placeBid(new BN(1_500_000))
  //         .accounts({
  //             auction: auction,           // MUST be the PDA, not auction struct
  //             bidder: bidder.publicKey,      // signer placing bid
  //             systemProgram: SystemProgram.programId,
  //         })
  //         .signers([bidder])
  //         .rpc();

  // });

  //   // -----------------------------
  //   // 4. Finalize Auction
  //   // -----------------------------
  //   it("Finalize Auction (winner receives NFT)", async () => {
  //       const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
  //       await sleep(10000);
  //       const auctionData = await program.account.auction.fetch(auction);
  //       const winner = auctionData.highestBidder;
  //       const sellerSol = await provider.connection.getBalance(seller.publicKey);
  //       console.log("Seller SOL balance:", sellerSol.toString());
  //       console.log("Winner:", winner.toBase58());
  //       const winnerAta = getAssociatedTokenAddressSync(mint.publicKey, winner, false, tokenProgram);
  //       const ataInfo = await provider.connection.getAccountInfo(winnerAta);
  //       if (!ataInfo) {
  //         const tx = new Transaction().add(
  //           createAssociatedTokenAccountIdempotentInstruction(
  //             provider.wallet.publicKey,       // payer
  //             winnerAta,                       // ATA
  //             winner,                          // owner
  //             mint.publicKey,                  // mint
  //             TOKEN_2022_PROGRAM_ID,           // token-2022 program
  //             ASSOCIATED_TOKEN_PROGRAM_ID
  //           )
  //         );

  //       await provider.sendAndConfirm(tx, []);
  //       // Winner ATA (ensure exists)
  //       }
  //       console.log("Created winner ATA:", winnerAta.toBase58());
  //       // Finalize auction
  //       await program.methods
  //       .finalizeAuction()
  //       .accounts({
  //         auction: auction,
  //         seller: seller.publicKey,
  //         winner: winner,
  //         nftVault: vault,
  //         winnerAta: winnerAta,
  //         sellerTokenAccount: sellerAta,
  //         nftMint: mint.publicKey,
  //         tokenProgram: TOKEN_2022_PROGRAM_ID,
  //         systemProgram: SystemProgram.programId,
  //       })
  //       .rpc();

  //       const nftAcc = await getAccount(
  //           provider.connection,
  //           winnerAta,
  //           undefined,
  //           TOKEN_2022_PROGRAM_ID
  //       );

  //       console.log("Winner NFT balance:", nftAcc.amount.toString());
  //       const sellerSol_after = await provider.connection.getBalance(seller.publicKey);
  //       console.log("Seller SOL balance:", sellerSol_after.toString());
  //   });

  // -----------------------------
  // 4. End to End Test Complete
  // -----------------------------
  it("End to End Test Complete (two bidders)", async () => {
    const sellerSol_before = await provider.connection.getBalance(
      seller.publicKey
    );
    console.log("Seller SOL balance:", sellerSol_before.toString());
    const bidder1Sol_before = await provider.connection.getBalance(
      bidder1.publicKey
    );
    console.log("bidder1 SOL balance:", bidder1Sol_before.toString());
    const bidder2Sol_before = await provider.connection.getBalance(
      bidder2.publicKey
    );
    console.log("bidder2 SOL balance:", bidder2Sol_before.toString());

    // const auctionId = new BN(1);
    const auctionId = new BN(3563456);
    // Derive the escrow PDA using the seed and maker's public key
    const auction = PublicKey.findProgramAddressSync(
      [
        Buffer.from("auction"),
        seller.publicKey.toBuffer(),
        mint.publicKey.toBuffer(),
        auctionId.toArrayLike(Buffer, "le", 8),
      ], // unique seed],
      program.programId
    )[0];

    // Derive the vault associated token account for the escrow
    const [vault, _] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault"),
        auction.toBuffer(), // auction PDA
      ],
      program.programId
    );
    const tx = new Transaction();

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

    await provider.sendAndConfirm(tx, [seller]);

    await program.methods
      .createAuction(
        new BN(auctionId), // start time (15 seconds from now)
        new BN(1_000_000), // starting bid
        new BN(5), // duration
        new BN(10) // cooldown
      )
      .accounts({
        auction: auction,
        seller: seller.publicKey,
        nftMint: mint.publicKey,
        nftVault: vault,
        sellerTokenAccount: sellerAta,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([seller])
      .rpc();

    const state = await program.account.auction.fetch(auction);
    console.log("Auction created:", state);
    let winner = state.highestBidder;
    console.log("Winner:", winner.toBase58());
    let sellerSol = await provider.connection.getBalance(seller.publicKey);
    console.log("Seller SOL balance:", sellerSol.toString());

    await program.methods
      .placeBid(new BN(1_500_000))
      .accounts({
        auction: auction, // MUST be the PDA, not auction struct
        bidder: bidder1.publicKey, // signer placing bid
        systemProgram: SystemProgram.programId,
        previousBidder: null,
      })
      .signers([bidder1])
      .rpc();
    let state1 = await program.account.auction.fetch(auction);
    let winner1 = state1.highestBidder;
    console.log("Winner:", winner1.toBase58());
    let sellerSol1 = await provider.connection.getBalance(seller.publicKey);
    console.log("Seller SOL balance:", sellerSol1.toString());

    await program.methods
      .placeBid(new BN(2_000_000))
      .accounts({
        auction: auction, // MUST be the PDA, not auction struct
        bidder: bidder2.publicKey, // signer placing bid
        systemProgram: SystemProgram.programId,
        previousBidder: winner1,
      })
      .signers([bidder2])
      .rpc();

    await sleep(10000);
    let state2 = await program.account.auction.fetch(auction);
    let winner2 = state2.highestBidder;
    console.log("Winner:", winner2.toBase58());
    let sellerSol2 = await provider.connection.getBalance(seller.publicKey);
    console.log("Seller SOL balance:", sellerSol2.toString());

    const winnerAta = getAssociatedTokenAddressSync(
      mint.publicKey,
      winner2,
      false,
      tokenProgram
    );
    const ataInfo = await provider.connection.getAccountInfo(winnerAta);
    if (!ataInfo) {
      const tx = new Transaction().add(
        createAssociatedTokenAccountIdempotentInstruction(
          provider.wallet.publicKey, // payer
          winnerAta, // ATA
          winner2, // owner
          mint.publicKey, // mint
          TOKEN_2022_PROGRAM_ID, // token-2022 program
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
        winner: winner2,
        nftVault: vault,
        winnerAta: winnerAta,
        sellerTokenAccount: sellerAta,
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
    const sellerSol_after = await provider.connection.getBalance(
      seller.publicKey
    );
    console.log("Seller SOL balance:", sellerSol_after.toString());
    const bidder1Sol_after = await provider.connection.getBalance(
      bidder1.publicKey
    );
    console.log("bidder1 SOL balance:", bidder1Sol_after.toString());
    const bidder2Sol_after = await provider.connection.getBalance(
      bidder2.publicKey
    );
    console.log("bidder2 SOL balance:", bidder2Sol_after.toString());
  });
});
