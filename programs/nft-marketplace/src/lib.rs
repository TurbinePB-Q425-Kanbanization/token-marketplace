#![allow(unexpected_cfgs, deprecated)]
use anchor_lang::{prelude::*, solana_program};

// Token-2022 program ID
const TOKEN_2022_PROGRAM_ID: Pubkey = anchor_spl::token_2022::ID;
use anchor_lang::prelude::{Interface, InterfaceAccount};
use anchor_lang::solana_program::system_instruction;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::InitializeAccount, // Import InitializeAccount from token_2022
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use solana_program::program::invoke_signed;
use solana_program::sysvar::rent::Rent;
// use anchor_spl::token_2022::{Mint as SplMint, TokenAccount as SplTokenAccount};
use anchor_spl::token_interface::initialize_account;

declare_id!("CAYsmbRRvTsiSgDwWSgQPZcJdp6khx9xLoQguT9aWCcC");

#[program]
pub mod nft_marketplace_token2022 {
    use anchor_lang::prelude::{clock::Epoch, program::invoke, program_pack::Pack};
    use anchor_spl::{
        associated_token::spl_associated_token_account::solana_program::lamports,
        token_2022::{
            close_account, spl_token_2022, transfer_checked, CloseAccount, TransferChecked,
        },
    };

    use super::*;

    pub fn create_auction(
        ctx: Context<CreateAuction>,
        auction_id: u64,
        starting_bid: u64,
        duration_seconds: i64,
        cooldown_seconds: i64,
    ) -> Result<()> {
        let clock = Clock::get()?;

        // initialize auction state
        let auction = &mut ctx.accounts.auction;
        auction.auction_id = auction_id;
        auction.seller = ctx.accounts.seller.key();
        auction.mint = ctx.accounts.nft_mint.key();
        auction.highest_bid = starting_bid;
        auction.highest_bidder = Pubkey::default();
        auction.start_time = clock.unix_timestamp;
        auction.end_time = clock.unix_timestamp + duration_seconds;
        auction.cooldown = cooldown_seconds;
        auction.is_active = true;
        auction.bump = ctx.bumps.auction; // Anchor 0.32 bump struct field

        let vault_address = ctx.accounts.nft_vault.key();
        let rent = Rent::get()?.minimum_balance(spl_token_2022::state::Account::LEN);

        let create_ix = solana_program::system_instruction::create_account(
            ctx.accounts.seller.key,
            &vault_address,
            rent,
            spl_token_2022::state::Account::LEN as u64,
            &ctx.accounts.token_program.key(), // Token-2022 program
        );

        let auction_key = auction.key();
        let signer_seeds: &[&[u8]] = &[
            b"vault",
            auction_key.as_ref(),
            &[ctx.bumps.nft_vault], // Anchor 0.32 bump struct field
        ];

        invoke_signed(
            &create_ix,
            &[
                ctx.accounts.seller.to_account_info(),
                ctx.accounts.nft_vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[signer_seeds],
        )?;

        // initialize vault token account
        // Initialize the token account via token_interface CPI (Token-2022)
        let init_accounts = InitializeAccount {
            account: ctx.accounts.nft_vault.to_account_info(),
            mint: ctx.accounts.nft_mint.to_account_info(),
            authority: ctx.accounts.auction.to_account_info(), // auction PDA becomes owner of the token account
            rent: ctx.accounts.rent.to_account_info(),
        };

        initialize_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            init_accounts,
            &[signer_seeds],
        ))?;

        // Now transfer the NFT from seller_token_account -> vault
        let transfer_accounts = TransferChecked {
            from: ctx.accounts.seller_token_account.to_account_info(), // <-- MUST be the seller's token account
            to: ctx.accounts.nft_vault.to_account_info(),
            authority: ctx.accounts.seller.to_account_info(),
            mint: ctx.accounts.nft_mint.to_account_info(),
        };

        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_accounts,
        );

        // seller is a signer in this instruction, so no signer_seeds are required here
        transfer_checked(transfer_ctx, 1u64, 0u8)?; // amount = 1, decimals = 0 (NFT)

        Ok(())
    }

    pub fn place_bid(ctx: Context<PlaceBid>, amount: u64) -> Result<()> {
        // read auction snapshot
        let (prev_bid, prev_bidder_pubkey, is_active, end_time, cooldown) = {
            let a = &ctx.accounts.auction;
            (
                a.highest_bid,
                a.highest_bidder,
                a.is_active,
                a.end_time,
                a.cooldown,
            )
        };
        let now = Clock::get()?.unix_timestamp;

        require!(is_active, AuctionError::AuctionInactive);
        require!(now < end_time, AuctionError::AuctionEnded);
        require!(amount > prev_bid, AuctionError::BidTooLow);

        // Transfer lamports from bidder -> auction PDA (real transfer)
        let transfer_to_auction_ix = system_instruction::transfer(
            &ctx.accounts.bidder.key(),
            &ctx.accounts.auction.key(),
            amount,
        );

        invoke(
            &transfer_to_auction_ix,
            &[
                ctx.accounts.bidder.to_account_info(),
                ctx.accounts.auction.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Refund previous bidder from auction PDA (if applicable)
        // 2) Refund previous bidder if needed AND client provided previous_bidder
        if prev_bidder_pubkey != Pubkey::default() && prev_bid > 0 {
            // previous_bidder must be provided by client
            let prev_acct = ctx
                .accounts
                .previous_bidder
                .as_ref()
                .expect("previous bidder account required for refund");

            // ensure the account passed matches the recorded previous bidder
            require!(
                prev_acct.key() == prev_bidder_pubkey,
                AuctionError::InvalidPreviousBidder
            );

            **ctx
                .accounts
                .auction
                .to_account_info()
                .try_borrow_mut_lamports()? -= prev_bid;

            // Credit to previous bidder
            **prev_acct
                .to_account_info()
                .try_borrow_mut_lamports()? += prev_bid;

            // // Build signer seeds for auction PDA
            // let auction = &ctx.accounts.auction;
            // let auction_id_bytes = auction.auction_id.to_le_bytes();
            // let seeds: &[&[u8]] = &[
            //     b"auction",
            //     auction.seller.as_ref(),
            //     auction.mint.as_ref(),
            //     auction_id_bytes.as_ref(),
            //     &[auction.bump],
            // ];
            // let signer_seeds = &[seeds];

            // // Refund: auction PDA -> previous bidder (PDA signs)
            // let refund_ix = system_instruction::transfer(
            //     &ctx.accounts.auction.key(),
            //     &prev_acct.key(),
            //     prev_bid,
            // );

            // // IMPORTANT: include auction AND previous bidder as accounts (previous bidder must be writable)
            // invoke_signed(
            //     &refund_ix,
            //     &[
            //         ctx.accounts.auction.to_account_info(),
            //         prev_acct.to_account_info(),               // << include prev bidder here!
            //         ctx.accounts.system_program.to_account_info(),
            //     ],
            //     signer_seeds,
            // )?;
        }

        // Update auction state (now safe to mutably borrow)
        
            let auction: &mut Account<'_, Auction> = &mut ctx.accounts.auction;
            auction.highest_bid = amount;
            auction.highest_bidder = ctx.accounts.bidder.key();

            // extend cooldown if needed
            if auction.end_time < now + auction.cooldown {
                auction.end_time = now + auction.cooldown;
            }
        

        Ok(())
    }

    pub fn finalize_auction(ctx: Context<FinalizeAuction>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;

        require!(
            ctx.accounts.auction.is_active,
            AuctionError::AuctionInactive
        );
        require!(
            now >= ctx.accounts.auction.end_time,
            AuctionError::AuctionNotEnded
        );

        // Mark inactive early to prevent reentrancy-style issues
        let auction = &mut ctx.accounts.auction;
        auction.is_active = false;

        let auction_bump = auction.bump;
        let auction_id = auction.auction_id.to_le_bytes();
        let seeds = &[
            b"auction",
            auction.seller.as_ref(),
            auction.mint.as_ref(),
            auction_id.as_ref(),
            &[auction_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        // If there is no winner, return NFT to seller
        if auction.highest_bidder == Pubkey::default() || auction.highest_bid == 0 {
            // transfer back to seller_token_account (PDA signs)
            transfer_checked(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    TransferChecked {
                        from: ctx.accounts.nft_vault.to_account_info(),
                        to: ctx.accounts.seller_token_account.to_account_info(),
                        authority: auction.to_account_info(),
                        mint: ctx.accounts.nft_mint.to_account_info(),
                    },
                    signer_seeds,
                ),
                1u64,
                0u8,
            )?;

            return Ok(());
        }

        // There is a winner: validate winner matches record
        require!(
            ctx.accounts.winner.key() == auction.highest_bidder,
            AuctionError::InvalidWinnerAccount
        );

        // transfer NFT from vault -> winner_ata (PDA signs)
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.nft_vault.to_account_info(),
                    to: ctx.accounts.winner_ata.to_account_info(),
                    authority: auction.to_account_info(),
                    mint: ctx.accounts.nft_mint.to_account_info(),
                },
                signer_seeds,
            ),
            1u64,
            0u8,
        )?;

        // Pay seller from auction PDA lamports
        let final_price = auction.highest_bid;
        **auction.to_account_info().try_borrow_mut_lamports()? -= final_price;
        **ctx.accounts.seller.try_borrow_mut_lamports()? += final_price;

        // CLOSE VAULT TOKEN ACCOUNT
        close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: ctx.accounts.nft_vault.to_account_info(),
                destination: ctx.accounts.seller.to_account_info(),
                authority: auction.to_account_info(),
            },
            signer_seeds,
        ))?;

        // CLOSE VAULT TOKEN ACCOUNT
        // CLOSE AUCTION PDA
        let lamports = ctx.accounts.auction.to_account_info().lamports();
        **ctx
            .accounts
            .auction
            .to_account_info()
            .try_borrow_mut_lamports()? -= lamports;
        **ctx.accounts.seller.try_borrow_mut_lamports()? += lamports;

        Ok(())
    }
}

/// ----------------- Accounts / Contexts -----------------

#[derive(Accounts)]
#[instruction(auction_id: u64)]
pub struct CreateAuction<'info> {
    #[account(
        init,
        payer = seller,
        seeds = [
            b"auction",
            seller.key().as_ref(),
            nft_mint.key().as_ref(),
            &auction_id.to_le_bytes()
        ],
        bump,
        space = 8 + Auction::MAX_SIZE,
    )]
    pub auction: Account<'info, Auction>,

    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(
        mint::token_program = token_program
    )]
    pub nft_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = seller_token_account.owner == seller.key(),
        constraint = seller_token_account.mint == nft_mint.key(),
    )]
    pub seller_token_account: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: created programmatically
    #[account(
        mut,
        seeds = [b"vault", auction.key().as_ref()],
        bump
    )]
    pub nft_vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct PlaceBid<'info> {
    #[account(
        mut,
        seeds = [b"auction", auction.seller.as_ref(), auction.mint.as_ref(), auction.auction_id.to_le_bytes().as_ref()],
        bump
    )]
    pub auction: Account<'info, Auction>,

    #[account(mut)]
    pub bidder: Signer<'info>,

    /// CHECK: refunded manually; client must pass previous bidder account when present.
    #[account(mut)]
    pub previous_bidder: Option<UncheckedAccount<'info>>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FinalizeAuction<'info> {
    #[account(
        mut,
        seeds = [b"auction", auction.seller.as_ref(), auction.mint.as_ref(), auction.auction_id.to_le_bytes().as_ref()],
        bump = auction.bump,
        close = seller
    )]
    pub auction: Account<'info, Auction>,

    /// CHECK: seller receives lamports - not required to sign
    #[account(mut)]
    pub seller: UncheckedAccount<'info>,

    ///CHECK:  winner provided for validation — no signer required
    pub winner: UncheckedAccount<'info>,

    /// vault PDA token-2022 account (must be the same pubkey created in create_auction)
    #[account(mut)]
    pub nft_vault: InterfaceAccount<'info, TokenAccount>,

    /// Winner's token-2022 ATA (must already exist or be created by client)
    #[account(mut,
        constraint = winner_ata.owner == winner.key(),
        constraint = winner_ata.mint == nft_mint.key(),)]
    pub winner_ata: InterfaceAccount<'info, TokenAccount>,

    /// seller's token account (for returning NFT if no bids)
    #[account(mut,
        constraint = seller_token_account.owner == seller.key(),
        constraint = seller_token_account.mint == nft_mint.key(),)]
    pub seller_token_account: InterfaceAccount<'info, TokenAccount>,

    /// mint
    #[account(
        mint::token_program = token_program
    )]
    pub nft_mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
}

//////////////////////////////////////
/// State & Errors
//////////////////////////////////////

#[account]
pub struct Auction {
    pub auction_id: u64,
    pub seller: Pubkey,
    pub mint: Pubkey,
    pub highest_bidder: Pubkey,
    pub highest_bid: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub cooldown: i64,
    pub is_active: bool,
    pub bump: u8,
}

impl Auction {
    // Conservative packing size (bytes). Adjust if you add fields.
    pub const MAX_SIZE: usize = 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1 + 8;

    // Define LEN as an alias for MAX_SIZE
    pub const LEN: usize = Self::MAX_SIZE;
}

#[error_code]
pub enum AuctionError {
    #[msg("Auction is not active")]
    AuctionInactive,
    #[msg("Bid is too low")]
    BidTooLow,
    #[msg("Auction already ended")]
    AuctionEnded,
    #[msg("Auction has not ended yet")]
    AuctionNotEnded,
    #[msg("Provided previous bidder does not match recorded highest bidder")]
    InvalidPreviousBidder,
    #[msg("Provided winner account does not match recorded highest bidder")]
    InvalidWinnerAccount,
}
