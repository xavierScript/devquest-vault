// src/instructions/deposit.rs
// Deposit instruction implementation

use anchor_lang::{prelude::*, system_program::{Transfer, transfer}};
use crate::state::VaultState;

/// Accounts required for depositing SOL into the vault
#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()], 
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        seeds = [b"state", vault_state.admin.key().as_ref()],
        bump = vault_state.state_bump,
    )]
    pub vault_state: Account<'info, VaultState>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    ctx.accounts.deposit(amount)
}

impl<'info> Deposit<'info> {
    /// Handler for deposit logic
    pub fn deposit(&mut self, amount: u64) -> Result<()> {
        // Transfer lamports from user to vault
        let cpi_program = self.system_program.to_account_info();
        let cpi_accounts = Transfer {
            from: self.user.to_account_info(),
            to: self.vault.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, amount)?;
        Ok(())
    }
}