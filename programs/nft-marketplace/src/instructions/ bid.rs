use anchor_lang::prelude::*;

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::states::{Escrow, ShelfItem};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Bid<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,

    #[account(
        mint::token_program = token_program
    )]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(
        mint::token_program = token_program
    )]
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = bidder,
        associated_token::token_program = token_program,
    )]
    pub bidder_ata_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = bidder,
        seeds = [b"escrow", bidder.key().as_ref(), seed.to_le_bytes().as_ref()],
        space = 8 + Escrow::INIT_SPACE,
        bump,
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        seeds = [b"shelf", bidder.key().as_ref(), seed.to_le_bytes().as_ref()],
        space = 8 + ShelfItem::INIT_SPACE,
        bump,
    )]
    pub escrow: Account<'info, ShelfItem>,

    #[account(
        init,
        payer = bidder,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Bid<'info> {
    pub fn init_escrow(&mut self, seed: u64, receive: u64, bumps: &BidBumps) -> Result<()> {
        self.escrow.set_inner(Escrow {
            seed,
            bidder: self.bidder.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            receive,
            bump: bumps.escrow,
        });
        Ok(())
    }

    pub fn deposit(&mut self, deposit: u64) -> Result<()> {
        let transfer_accounts = TransferChecked {
            from: self.bidder_ata_a.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.maker.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), transfer_accounts);

        transfer_checked(cpi_ctx, deposit, self.mint_a.decimals)
    }
}