#![allow(unexpected_cfgs)]

use crate::errors::BridgeHandlerError;
use crate::states::{Chain, FixedFeeInfo};
use anchor_lang::prelude::*;
use contexts::*;

mod constants;
mod contexts;
mod errors;
pub mod states;
mod utils;

declare_id!("6kpxYKjqe8z66hnDHbbjhEUxha46cnz2UqrneGECmFBg");

#[program]
pub mod bridge_program {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>, init_nonce: u64, chain: u8) -> Result<()> {
        let chain = match chain {
            1 => Chain::Solana,
            2 => Chain::Solayer,
            _ => return Err(BridgeHandlerError::InvalidChain.into()),
        };
        ctx.accounts.initialize(ctx.bumps, init_nonce, chain)?;
        Ok(())
    }

    pub fn add_token(
        ctx: Context<AddToken>,
        decimal: u8,
        name: Option<String>,
        symbol: Option<String>,
        uri: Option<String>,
    ) -> Result<()> {
        ctx.accounts
            .add_token(ctx.bumps.token_info, decimal, name, symbol, uri)?;
        Ok(())
    }

    pub fn pause_token(ctx: Context<PauseToken>) -> Result<()> {
        ctx.accounts.pause_token()?;
        Ok(())
    }

    pub fn unpause_token(ctx: Context<PauseToken>) -> Result<()> {
        ctx.accounts.unpause_token()?;
        Ok(())
    }

    pub fn pause_bridge(ctx: Context<PauseBridge>) -> Result<()> {
        ctx.accounts.pause_bridge()?;
        Ok(())
    }

    pub fn unpause_bridge(ctx: Context<PauseBridge>) -> Result<()> {
        ctx.accounts.unpause_bridge()?;
        Ok(())
    }

    pub fn bridge_asset_source_chain(
        ctx: Context<BridgeAssetSourceChain>,
        bridge_proof_nonce: u64,
        amount: u64,
        recipient: Pubkey,
        target_mint: Pubkey,
        additional_sol_gas: u64,
    ) -> Result<u64> {
        let nonce = ctx.accounts.bridge_asset_source_chain(
            ctx.bumps.token_info,
            amount,
            recipient,
            target_mint,
            additional_sol_gas,
        )?;
        ctx.accounts.issue_bridge_proof(
            ctx.bumps.bridge_proof,
            bridge_proof_nonce,
            amount,
            recipient,
        )?;
        Ok(nonce)
    }

    pub fn bridge_asset_source_chain_sol(
        ctx: Context<BridgeAssetSourceChainSol>,
        bridge_proof_nonce: u64,
        amount: u64,
        recipient: Pubkey,
    ) -> Result<u64> {
        let nonce = ctx
            .accounts
            .bridge_asset_source_chain_sol(amount, recipient)?;
        ctx.accounts
            .issue_bridge_proof(ctx.bumps, bridge_proof_nonce, amount, recipient)?;
        Ok(nonce)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn bridge_asset_target_chain(
        ctx: Context<BridgeAssetTargetChain>,
        msg_hash: [u8; 32],
        source_tx_id: [u8; 64],
        sender: Pubkey,
        source_mint: Pubkey,
        receive_amount: u64,
        nonce: u64,
        additional_sol_gas: u64,
    ) -> Result<()> {
        ctx.accounts.bridge_asset_target_chain(
            ctx.bumps,
            msg_hash,
            source_tx_id,
            sender,
            source_mint,
            receive_amount,
            nonce,
            additional_sol_gas,
        )?;
        Ok(())
    }

    pub fn bridge_asset_target_chain_sol(
        ctx: Context<BridgeAssetTargetChainSol>,
        msg_hash: [u8; 32],
        source_tx_id: [u8; 64],
        sender: Pubkey,
        receive_amount: u64,
        nonce: u64,
    ) -> Result<()> {
        ctx.accounts.bridge_asset_target_chain_sol(
            ctx.bumps,
            msg_hash,
            source_tx_id,
            sender,
            receive_amount,
            nonce,
        )?;
        Ok(())
    }

    pub fn add_guardian(ctx: Context<AddGuardian>) -> Result<()> {
        ctx.accounts.add_guardian()?;
        Ok(())
    }

    pub fn remove_guardian(ctx: Context<RemoveGuardian>) -> Result<()> {
        ctx.accounts.remove_guardian()?;
        Ok(())
    }

    pub fn update_guardian_threshold(
        ctx: Context<UpdateGuardianThreshold>,
        guardian_threshold: u8,
    ) -> Result<()> {
        ctx.accounts.update_guardian_threshold(guardian_threshold)?;
        Ok(())
    }

    pub fn verify_signature(
        ctx: Context<VerifySignature>,
        msg_hash: [u8; 32],
        signer_indexes: Vec<u8>,
    ) -> Result<()> {
        ctx.accounts
            .verify_signature(ctx.bumps, msg_hash, signer_indexes)?;
        Ok(())
    }

    pub fn update_fee_vault(ctx: Context<UpdateFeeVault>) -> Result<()> {
        ctx.accounts.update_fee_vault()?;
        Ok(())
    }

    pub fn update_fee_info(ctx: Context<UpdateFeeInfo>, fee_info: FixedFeeInfo) -> Result<()> {
        ctx.accounts.update_fee_info(fee_info)?;
        Ok(())
    }

    pub fn update_instant_bridge_cap(
        ctx: Context<UpdateInstantBridgeCap>,
        instant_bridge_cap: u64,
    ) -> Result<()> {
        ctx.accounts.update_instant_bridge_cap(instant_bridge_cap)?;
        Ok(())
    }

    pub fn update_operator(ctx: Context<UpdateOperator>) -> Result<()> {
        ctx.accounts.update_operator()?;
        Ok(())
    }

    pub fn update_manager(ctx: Context<UpdateManager>) -> Result<()> {
        ctx.accounts.update_manager()?;
        Ok(())
    }
}
