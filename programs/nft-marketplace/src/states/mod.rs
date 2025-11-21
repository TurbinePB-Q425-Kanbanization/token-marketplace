use anchor_lang::prelude::*;
#[account]
#[derive(InitSpace)]
pub struct Escrow {
    pub seed: u64,
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub receive: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct ShelfItem {
    pub seed: u64,
    pub mint: Pubkey,
    pub price: u64,
    pub seller: Pubkey,
    pub bump: u8,
}