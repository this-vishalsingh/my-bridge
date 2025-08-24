use anchor_lang::prelude::*;

#[error_code]
pub enum BridgeHandlerError {
    #[msg("bridge paused")]
    BridgePaused,

    #[msg("invalid chain")]
    InvalidChain,

    #[msg("invalid operator")]
    InvalidOperator,

    #[msg("invalid fee vault")]
    InvalidFeeVault,

    #[msg("invalid msg hash")]
    InvalidMsgHash,

    #[msg("source mint already exists")]
    SourceMintAlreadyExists,

    #[msg("token info already exists")]
    TokenAlreadyExists,

    #[msg("token not exists")]
    TokenNotExists,

    #[msg("invalid fee")]
    InvalidFee,

    #[msg("signer unauthorized")]
    Unauthorized,

    #[msg("same operator")]
    SameOperator,

    #[msg("token paused")]
    TokenPaused,

    #[msg("insufficient token amount")]
    InsufficientAmount,

    #[msg("fee calculation error")]
    FeeCalculationError,

    #[msg("bridge amount calculation error")]
    BridgeAmtCalculationError,

    #[msg("invalid recipient vault")]
    InvalidRecipientVault,

    #[msg("insufficient bridge handler funds")]
    InsufficientFunds,

    #[msg("invalid ed25519 program id")]
    InvalidEd25519ProgramId,

    #[msg("invalid ed25519 accounts")]
    InvalidEd25519Accounts,

    #[msg("invalid ed25519 data")]
    InvalidEd25519Data,

    #[msg("invalid guardian signature count")]
    InvalidGuardianSigCount,

    #[msg("invalid guardian signature")]
    InvalidGuardianSignatureMessage,

    #[msg("invalid guardian signature pubkey")]
    InvalidGuardianSignaturePubKey,

    #[msg("invalid ed25519 sig input")]
    InvalidEd25519SigInput,

    #[msg("guardian already exists")]
    GuardianAlreadyExists,

    #[msg("guardian not found")]
    GuardianNotFound,

    #[msg("invalid guardian threshold")]
    InvalidGuardianThreshold,

    #[msg("invalid signer count")]
    InvalidSignerCount,

    #[msg("invalid signer indexes")]
    InvalidSignerIndexes,

    #[msg("guardian signature already exists")]
    GuardianSignatureAlreadyExists,

    #[msg("invalid verified signatures bump")]
    InvalidVerifiedSignaturesBump,

    #[msg("guardian threshold not met")]
    GuardianThresholdNotMet,

    #[msg("too much additional sol gas")]
    TooMuchAdditionalSolGas,

    #[msg("too little additional sol gas")]
    TooLittleAdditionalSolGas,

    #[msg("invalid additional sol gas")]
    InvalidAdditionalSolGas,

    #[msg("too little sol bridge amount")]
    TooLittleSolBridgeAmount,

    #[msg("failed to mint sol")]
    FailToMintSol,

    #[msg("failed to burn sol")]
    FailToBurnSol,

    #[msg("invalid os, solana does not support minting sol")]
    InvalidOSForMintingSol,

    #[msg("invalid os, solana does not support burning sol")]
    InvalidOSForBurningSol,

    #[msg("invalid token info")]
    InvalidTokenInfo,

    #[msg("instant bridge cap exceeded")]
    InstantBridgeCapExceeded,
}
