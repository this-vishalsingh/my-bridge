// Token mint addresses with their dollar values (in cents for precision)
pub const SOLANA_WHITELISTED_TOKENS: &[(Pubkey, f64)] = &[
    (
        pubkey_from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        1.0,
    ), // USDC: $1.00
    (
        pubkey_from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"),
        1.0,
    ), // USDT: $1.00
    (
        pubkey_from_str("LAYER4xPpTCb3QL8S9u41EAhAX7mhBn8Q6xMTwY2Yzc"),
        1.0,
    ), // LAYER: $1.00
    (
        pubkey_from_str("2zMMhcVQEXDtdE6vsFS7S7D5oUodfJHE8vd1gnBouauv"),
        0.04,
    ), // PENGU: $0.04
    (
        pubkey_from_str("9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump"),
        1.5,
    ), // FARTCOIN: $1.5
    (
        pubkey_from_str("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"),
        3.0,
    ), // RAY: $3.00 (replaced empty string)
];

// Helper function to get token price by mint address
pub fn get_whitelisted_token_price(mint_address: Pubkey, amount: u64, decimals: u8) -> Option<f64> {
    let mut amount_with_decimal = amount as f64 / 10f64.powi(decimals as i32);
    if amount_with_decimal < 1.0 {
        amount_with_decimal = 1.0;
    }
    SOLANA_WHITELISTED_TOKENS
        .iter()
        .find(|(mint, _)| *mint == mint_address)
        .map(|(_, price)| *price * amount_with_decimal as f64)
}

// Helper function to check if token is whitelisted
pub fn is_token_whitelisted(mint_address: Pubkey) -> bool {
    SOLANA_WHITELISTED_TOKENS
        .iter()
        .any(|(mint, _)| *mint == mint_address)
}

pub const SOLANA_DOLLAR_VALUE: u64 = 200;
pub const SOLANA_DOLLAR_CAP_PER_EPOCH: u64 = 1_000_000;
pub const SOLANA_POST_INSTANT_CAP_AWAITING_TIME_SECONDS: u64 = 6 * 60 * 60; // 6 hours
