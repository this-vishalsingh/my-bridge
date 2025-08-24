use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace, Debug)]
pub struct BridgeProof {
    pub bump: u8,
    pub msg_hash: [u8; 32],
    pub tx_id: [u8; 64],
    pub user_account: Pubkey,
    pub created_at: u64,
}

#[account]
#[derive(InitSpace, Debug)]
pub struct BridgeProofSourceChain {
    pub bump: u8,
    pub msg_hash: [u8; 32], // Note that this msg_hash is not the same as the msg_hash in BridgeProof
    pub user_account: Pubkey,
    pub created_at: u64,
}
