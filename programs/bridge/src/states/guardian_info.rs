use anchor_lang::prelude::*;

use crate::constants::MAX_GUARDIAN_COUNT;

#[account]
#[derive(InitSpace, Debug)]
pub struct GuardianInfo {
    pub bump: u8,
    #[max_len(MAX_GUARDIAN_COUNT)]
    pub guardians: Vec<Pubkey>,
}
