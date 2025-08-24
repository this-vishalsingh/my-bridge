use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace, Debug)]
pub struct BridgeHandler {
    pub bump: u8,
    pub init_nonce: u64,
    pub pause: bool,
    pub nonce: u64,
    pub chain: Chain,
    pub fee_vault: Pubkey,
    pub manager: Pubkey,
    pub operator: Pubkey,
    pub guardian_info: Pubkey,
    pub guardian_threshold: u8,
    pub instant_bridge_cap_remained_dollar: u64,
    pub instant_bridge_cap_epoch: u64,
    pub fee_info: FixedFeeInfo,
}

#[derive(InitSpace, Clone, Copy, Debug, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub enum Chain {
    Solana = 1,
    Solayer = 2,
}

#[derive(InitSpace, Clone, Copy, Debug, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct FixedFeeInfo {
    // all in form of lamports
    pub bridge_asset_fee: u64,
    pub bridge_message_fee: u64,
    pub cross_chain_call_fee: u64,
}

// TODO: calculate ATA fee + gas fee + bridge proof fee + verify signature fee
// implement default for FixedFeeInfo
impl Default for FixedFeeInfo {
    fn default() -> Self {
        Self {
            bridge_asset_fee: 500000,
            bridge_message_fee: 500000,
            cross_chain_call_fee: 500000,
        }
    }
}
