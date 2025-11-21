

#[derive(Accounts)]
#[instruction(starting_bid: u64, end_time: i64)]
pub struct List<'info> {
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




impl<'info> List<'info> {
pub fn list(
    ctx: Context<List>,
    starting_bid: u64,
    end_time: i64,
) -> Result<()> {
    let list = &mut ctx.accounts.list;
    list.seller = *ctx.accounts.seller.key;
    list.nft_mint = *ctx.accounts.nft_mint.to_account_info().key;
    list.nft_vault = *ctx.accounts.nft_vault.to_account_info().key;
    list.highest_bid = starting_bid;
    list.highest_bidder = Pubkey::default();
    list.end_time = end_time;
    list.is_active = true;
    Ok(())
}
}