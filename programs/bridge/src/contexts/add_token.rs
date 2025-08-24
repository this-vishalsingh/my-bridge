use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{
        create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
        Metadata,
    },
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::states::{Chain, TokenInfo};
use crate::{errors::BridgeHandlerError, states::BridgeHandler};

#[derive(Accounts)]
#[instruction(decimal: u8)]
pub struct AddToken<'info> {
    #[account(mut)]
    operator: Signer<'info>,
    #[account(
        has_one = operator @ BridgeHandlerError::InvalidOperator,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
    #[account(
        init_if_needed,
        payer = operator,
        mint::authority = bridge_handler,
        mint::token_program = token_program,
        mint::decimals = decimal,
        seeds = [b"mint", bridge_handler.key().as_ref(), source_mint.key().as_ref()],
        bump
    )]
    mint: Box<InterfaceAccount<'info, Mint>>,
    /// CHECK: source_mint won't exist on current chain
    source_mint: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Instead of checking in macro, we do manual check to reduce stack size
    bridge_handler_vault: AccountInfo<'info>,
    #[account(
        mut,
        address=mpl_token_metadata::accounts::Metadata::find_pda(&mint.key()).0
    )]
    /// CHECK: This is the metadata account
    metadata: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = operator,
        space = 8 + TokenInfo::INIT_SPACE,
        seeds = [b"token_info", bridge_handler.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    token_info: Box<Account<'info, TokenInfo>>,
    token_metadata_program: Program<'info, Metadata>,
    associated_token_program: Program<'info, AssociatedToken>,
    token_program: Interface<'info, TokenInterface>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

impl AddToken<'_> {
    pub fn add_token(
        &mut self,
        token_info_bump: u8,
        decimal: u8,
        name: Option<String>,
        symbol: Option<String>,
        uri: Option<String>,
    ) -> Result<()> {
        self.init_if_needed_and_check_bridge_handler_vault()?;

        let chain = self.bridge_handler.chain;

        let is_token_already_exists =
            self.token_info.is_solana_native_token || self.token_info.is_solayer_native_token;

        if !is_token_already_exists {
            let (solana_mint, solayer_mint) = if chain == Chain::Solana {
                (self.mint.key(), self.source_mint.key())
            } else {
                (self.source_mint.key(), self.mint.key())
            };

            let (is_solana_native_token, is_solayer_native_token) = if chain == Chain::Solana {
                (false, true)
            } else {
                (true, false)
            };

            msg!("Creating token info with decimal: {}, solana_mint: {:?}, solayer_mint: {:?},  is_solana_native_token: {:?}, is_solayer_native_token: {:?}",
                decimal, solana_mint, solayer_mint, is_solana_native_token, is_solayer_native_token
            );

            self.token_info.bump = token_info_bump;
            self.token_info.solana_mint = solana_mint;
            self.token_info.solayer_mint = solayer_mint;
            self.token_info.is_solana_native_token = is_solana_native_token;
            self.token_info.is_solayer_native_token = is_solayer_native_token;
            self.token_info.pause = false;

            // only create metadata on solana
            if name.is_some() && symbol.is_some() && uri.is_some() && chain == Chain::Solana {
                let token_metadata = DataV2 {
                    name: name.unwrap(),
                    symbol: symbol.unwrap(),
                    uri: uri.unwrap(),
                    seller_fee_basis_points: 0,
                    creators: None,
                    collection: None,
                    uses: None,
                };

                let bump = [self.bridge_handler.bump];
                let init_nonce_bytes = self.bridge_handler.init_nonce.to_be_bytes();
                let signer_seeds: [&[&[u8]]; 1] =
                    [&[b"bridge_handler", init_nonce_bytes.as_ref(), &bump][..]];

                let ctx = CpiContext::new_with_signer(
                    self.token_metadata_program.to_account_info(),
                    CreateMetadataAccountsV3 {
                        metadata: self.metadata.to_account_info(),
                        mint: self.mint.to_account_info(),
                        mint_authority: self.bridge_handler.to_account_info(),
                        payer: self.operator.to_account_info(),
                        update_authority: self.bridge_handler.to_account_info(),
                        system_program: self.system_program.to_account_info(),
                        rent: self.rent.to_account_info(),
                    },
                    &signer_seeds[..],
                );

                create_metadata_accounts_v3(ctx, token_metadata, true, true, None)?;
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn init_if_needed_and_check_bridge_handler_vault(&mut self) -> Result<()> {
        let rent = Rent::get()?;
        let owner_program = self.bridge_handler_vault.to_account_info().owner;

        if owner_program == self.system_program.key {
            // do init
            let cpi_program = self.associated_token_program.to_account_info();
            let cpi_accounts = ::anchor_spl::associated_token::Create {
                payer: self.operator.to_account_info(),
                associated_token: self.bridge_handler_vault.to_account_info(),
                authority: self.bridge_handler.to_account_info(),
                mint: self.mint.to_account_info(),
                system_program: self.system_program.to_account_info(),
                token_program: self.token_program.to_account_info(),
            };
            let cpi_ctx = anchor_lang::context::CpiContext::new(cpi_program, cpi_accounts);
            ::anchor_spl::associated_token::create(cpi_ctx)?;
        }

        {
            let data = self
                .bridge_handler_vault
                .to_account_info()
                .data
                .borrow()
                .to_vec();
            let token_account = match TokenAccount::try_deserialize(&mut data.as_slice()) {
                Ok(val) => val,
                Err(e) => {
                    return Err(e.with_account_name("bridge_handler_vault"));
                }
            };

            if token_account.mint != self.mint.key() {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintTokenMint,
                )
                .with_account_name("bridge_handler_vault")
                .with_pubkeys((token_account.mint, self.mint.key())));
            }

            if token_account.owner != self.bridge_handler.key() {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintTokenOwner,
                )
                .with_account_name("bridge_handler_vault")
                .with_pubkeys((token_account.owner, self.bridge_handler.key())));
            }

            if owner_program != self.token_program.key {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintAssociatedTokenTokenProgram,
                )
                .with_account_name("bridge_handler_vault")
                .with_pubkeys((*owner_program, self.token_program.key())));
            }

            if self.bridge_handler_vault.key()
                != ::anchor_spl::associated_token::get_associated_token_address_with_program_id(
                    &self.bridge_handler.key(),
                    &self.mint.key(),
                    &self.token_program.key(),
                )
            {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::AccountNotAssociatedTokenAccount,
                )
                .with_account_name("bridge_handler_vault"));
            }
        }

        if !rent.is_exempt(
            self.bridge_handler_vault.to_account_info().lamports(),
            self.bridge_handler_vault.to_account_info().try_data_len()?,
        ) {
            return Err(anchor_lang::error::Error::from(
                anchor_lang::error::ErrorCode::ConstraintRentExempt,
            )
            .with_account_name("bridge_handler_vault"));
        }

        Ok(())
    }
}
