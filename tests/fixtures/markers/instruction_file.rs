// Fixture: instruction file containing both `file` (auto) and `handler` (user) segments.

// === sunscreen:auto-generated:begin segment=file version=1 generator=instruction ===
// This file is initial scaffolding. The handler body below is a user-region.

use anchor_lang::prelude::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub depositor: Signer<'info>,
    pub system_program: Program<'info, System>,
}
// === sunscreen:auto-generated:end segment=file ===

// === sunscreen:user-region:begin segment=handler ===
pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.total = vault
        .total
        .checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    Ok(())
}
// === sunscreen:user-region:end segment=handler ===
