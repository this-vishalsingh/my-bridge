use anchor_lang::prelude::*;

use crate::constants::MAX_GUARDIAN_SIGNATURES;

#[account]
#[derive(InitSpace, Debug)]
pub struct VerifiedSignatures {
    pub bump: u8,
    #[max_len(MAX_GUARDIAN_SIGNATURES)]
    pub pubkey_index: Vec<u8>,
    pub created_at: u64,
}
