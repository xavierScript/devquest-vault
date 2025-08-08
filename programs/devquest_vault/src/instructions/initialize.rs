// src/instructions/initialize.rs
// Initialize instruction implementation

use anchor_lang::{prelude::*, system_program::{Transfer, transfer}};
use crate::{errors::CustomError, state::VaultState};

/// Accounts required for initializing the vault
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The admin initializing the vault
    #[account(mut)]
    pub user: Signer<'info>,
    /// The vault state account (PDA)
    #[account(
        init,
        payer = user,
        seeds = [b"state", user.key().as_ref()], 
        bump,
        space = VaultState::INIT_SPACE,
    )]
    pub vault_state: Account<'info, VaultState>,
    /// The vault account (PDA)
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    ctx.accounts.initialize(&ctx.bumps)
}

impl<'info> Initialize<'info> {
    /// Handler for vault initialization logic
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        require!(!self.vault_state.is_initialized, CustomError::AlreadyInitialized);
        // Calculate rent-exempt minimum for the vault
        let rent_exempt = Rent::get()?.minimum_balance(self.vault.to_account_info().data_len());
        // Transfer rent-exempt lamports from user to vault
        let cpi_program = self.system_program.to_account_info();
        let cpi_accounts = Transfer {
            from: self.user.to_account_info(),
            to: self.vault.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, rent_exempt)?;
        // Initialize vault state fields
        self.vault_state.vault_bump = bumps.vault;
        self.vault_state.state_bump = bumps.vault_state;
        self.vault_state.admin = self.user.key();
        self.vault_state.payees = Vec::new();
        self.vault_state.is_initialized = true;
        Ok(())
    }  
}