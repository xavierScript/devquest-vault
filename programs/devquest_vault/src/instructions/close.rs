// src/instructions/close.rs
// Close instruction implementation

use anchor_lang::{prelude::*, system_program::{Transfer, transfer}};
use crate::{errors::CustomError, state::VaultState};

/// Accounts required for closing the vault
#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"state", vault_state.admin.key().as_ref()],
        bump = vault_state.state_bump,
        close = user,
        constraint = user.key() == vault_state.admin @ CustomError::UnauthorizedAdmin,
    )]
    pub vault_state: Account<'info, VaultState>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Close>) -> Result<()> {
    ctx.accounts.close()
}

impl<'info> Close<'info> {
    /// Handler for closing the vault and returning all funds to the admin
    pub fn close(&mut self) -> Result<()> {
        let cpi_program = self.system_program.to_account_info();
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };
        let seeds = &[
            b"vault",
            self.vault_state.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer(cpi_ctx, self.vault.lamports())?;
        Ok(())
    }
}