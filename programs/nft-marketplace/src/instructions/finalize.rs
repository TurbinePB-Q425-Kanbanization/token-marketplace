
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