use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, CloseAccount};

declare_id!("YourProgramIdHere12345678901234567890123456789012");

#[program]
pub mod nft_auction {
    use super::*;

    pub fn create_auction(
        ctx: Context<CreateAuction>,
        starting_bid: u64,
        end_time: i64,
    ) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        auction.seller = *ctx.accounts.seller.key;
        auction.nft_mint = *ctx.accounts.nft_mint.to_account_info().key;
        auction.nft_vault = *ctx.accounts.nft_vault.to_account_info().key;
        auction.highest_bid = starting_bid;
        auction.highest_bidder = Pubkey::default();
        auction.end_time = end_time;
        auction.is_active = true;
        Ok(())
    }

    pub fn place_bid(ctx: Context<PlaceBid>, bid_amount: u64) -> Result<()> {
        let auction = &mut ctx.accounts.auction;

        require!(auction.is_active, AuctionError::AuctionInactive);
        require!(bid_amount > auction.highest_bid, AuctionError::BidTooLow);
        require!(Clock::get()?.unix_timestamp < auction.end_time, AuctionError::AuctionEnded);

        // Refund previous highest bidder
        if auction.highest_bidder != Pubkey::default() {
            **ctx.accounts.previous_bidder.lamports.borrow_mut() += auction.highest_bid;
        }

        // Transfer lamports from new bidder to program
        **ctx.accounts.bidder.lamports.borrow_mut() -= bid_amount;
        **ctx.accounts.auction.to_account_info().lamports.borrow_mut() += bid_amount;

        auction.highest_bid = bid_amount;
        auction.highest_bidder = *ctx.accounts.bidder.key;

        Ok(())
    }

    pub fn finalize_auction(ctx: Context<FinalizeAuction>) -> Result<()> {
        let auction = &mut ctx.accounts.auction;

        require!(Clock::get()?.unix_timestamp >= auction.end_time, AuctionError::AuctionNotEnded);

        // Transfer NFT to highest bidder
        let cpi_accounts = Transfer {
            from: ctx.accounts.nft_vault.to_account_info(),
            to: ctx.accounts.winner_nft_account.to_account_info(),
            authority: ctx.accounts.auction.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        token::transfer(CpiContext::new(cpi_program, cpi_accounts), 1)?;

        // Transfer funds to seller
        **ctx.accounts.seller.lamports.borrow_mut() += auction.highest_bid;

        auction.is_active = false;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(starting_bid: u64, end_time: i64)]
pub struct CreateAuction<'info> {
    #[account(init, payer = seller, space = 8 + 128)]
    pub auction: Account<'info, Auction>,
    #[account(mut)]
    pub seller: Signer<'info>,
    pub nft_mint: Account<'info, Mint>,
    #[account(mut)]
    pub nft_vault: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct PlaceBid<'info> {
    #[account(mut)]
    pub auction: Account<'info, Auction>,
    #[account(mut)]
    pub bidder: Signer<'info>,
    /// CHECK: previous bidder account for refund
    #[account(mut)]
    pub previous_bidder: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct FinalizeAuction<'info> {
    #[account(mut)]
    pub auction: Account<'info, Auction>,
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut)]
    pub nft_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub winner_nft_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct Auction {
    pub seller: Pubkey,
    pub nft_mint: Pubkey,
    pub nft_vault: Pubkey,
    pub highest_bid: u64,
    pub highest_bidder: Pubkey,
    pub end_time: i64,
    pub is_active: bool,
}

#[error_code]
pub enum AuctionError {
    #[msg("Auction is not active.")]
    AuctionInactive,
    #[msg("Bid is too low.")]
    BidTooLow,
    #[msg("Auction has already ended.")]
    AuctionEnded,
    #[msg("Auction has not ended yet.")]
    AuctionNotEnded,
}
