// Fixture: `errors.rs` — `error_variants` segment.

use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    // === sunscreen:auto-generated:begin segment=error_variants version=1 generator=error ===
    #[msg("Arithmetic overflow occurred")]
    ArithmeticOverflow,
    #[msg("Insufficient funds in vault")]
    InsufficientFunds,
    #[msg("Unauthorized signer")]
    Unauthorized,
    // === sunscreen:auto-generated:end segment=error_variants ===
}
