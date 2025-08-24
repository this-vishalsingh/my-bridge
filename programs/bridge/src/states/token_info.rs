use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace, Debug)]
pub struct TokenInfo {
    pub bump: u8,
    pub solana_mint: Pubkey,
    pub solayer_mint: Pubkey,
    pub is_solana_native_token: bool,
    pub is_solayer_native_token: bool,
    pub pause: bool,
}
