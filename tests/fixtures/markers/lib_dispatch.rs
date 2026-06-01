// Fixture: `lib.rs` containing `dispatch` segment inside `#[program]`.
// Inline `mod` blocks are used (instead of `mod foo;`) so rustfmt does not
// try to resolve sibling files that aren't shipped with the fixture.

use anchor_lang::prelude::*;

pub mod instructions {}
pub mod state {}
pub mod events {}
pub mod errors {}

declare_id!("Esc11111111111111111111111111111111111111");

#[program]
pub mod escrow {
    use super::*;

    // === sunscreen:auto-generated:begin segment=dispatch version=1 ===
    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        instructions::initialize::handler(ctx, fee_bps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handler(ctx, amount)
    }
    // === sunscreen:auto-generated:end segment=dispatch ===
}
