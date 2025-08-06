
use anchor_lang::{prelude::*, system_program::{Transfer, transfer}};

// declare_id!("8GXzdcCDdBr7MaLwAULrxG1KBYB5yjVVva2cs8cemoRb");
declare_id!("8b4G7EpokrxCpb1BMK5kjFMtLFnGFqLetxvR3ou17cS8");

#[error_code]
pub enum CustomError {
    #[msg("Vault is already initialized")]
    AlreadyInitialized,
    #[msg("Only admin can perform this action")]
    UnauthorizedAdmin,
    #[msg("Maximum number of payees reached")]
    MaxPayeesReached,
    #[msg("Payee already exists")]
    PayeeAlreadyExists,
    #[msg("Payee not found")]
    PayeeNotFound,
    #[msg("Only authorized payees can withdraw")]
    UnauthorizedPayee,
}

#[program]
pub mod devquest_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)?;
        Ok(())
    }

    pub fn add_payee(ctx: Context<UpdatePayee>, payee: Pubkey) -> Result<()> {
        ctx.accounts.add_payee(payee)
    }

    pub fn remove_payee(ctx: Context<UpdatePayee>, payee: Pubkey) -> Result<()> {
        ctx.accounts.remove_payee(payee)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)?;
        Ok(())
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        ctx.accounts.close()?;
        Ok(())
    }
}
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        seeds = [b"state", user.key().as_ref()], 
        bump,
        space = VaultState::INIT_SPACE,
    )]
    pub vault_state: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        require!(!self.vault_state.is_initialized, CustomError::AlreadyInitialized);
        
        // Get the amount of lamports needed to make the vault rent exempt
        let rent_exempt = Rent::get()?.minimum_balance(self.vault.to_account_info().data_len());

        // Transfer the rent-exempt amount from the user to the vault
        let cpi_program = self.system_program.to_account_info();
        let cpi_accounts = Transfer {
            from: self.user.to_account_info(),
            to: self.vault.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(cpi_ctx, rent_exempt)?;

        self.vault_state.vault_bump = bumps.vault;
        self.vault_state.state_bump = bumps.vault_state;
        self.vault_state.admin = self.user.key();
        self.vault_state.payees = Vec::new();
        self.vault_state.is_initialized = true;

        Ok(())
    }  
}

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

impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64) -> Result<()> {

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

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        // Check if the user is either admin or an authorized payee
        let user_key = self.user.key();
        require!(
            user_key == self.vault_state.admin || self.vault_state.payees.contains(&user_key),
            CustomError::UnauthorizedPayee
        );

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

        transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

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

impl<'info> Close<'info> {
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
#[account]
pub struct VaultState {
    pub vault_bump: u8,
    pub state_bump: u8,
    pub admin: Pubkey,
    pub payees: Vec<Pubkey>,         // List of authorized payees
    pub is_initialized: bool,
}

impl Space for VaultState {
    // 8 discriminator + 1 vault_bump + 1 state_bump + 32 admin + 4 vec length + (32 * 5) max payees + 1 is_initialized
    const INIT_SPACE: usize = 8 + 1 + 1 + 32 + 4 + (32 * 5) + 1;
}

#[derive(Accounts)]
pub struct UpdatePayee<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"state", vault_state.admin.key().as_ref()],
        bump = vault_state.state_bump,
        constraint = user.key() == vault_state.admin @ CustomError::UnauthorizedAdmin,
    )]
    pub vault_state: Account<'info, VaultState>,
}

impl<'info> UpdatePayee<'info> {
    pub fn add_payee(&mut self, payee: Pubkey) -> Result<()> {
        require!(self.vault_state.payees.len() < 5, CustomError::MaxPayeesReached);
        require!(!self.vault_state.payees.contains(&payee), CustomError::PayeeAlreadyExists);

        self.vault_state.payees.push(payee);
        Ok(())
    }

    pub fn remove_payee(&mut self, payee: Pubkey) -> Result<()> {
        if let Some(index) = self.vault_state.payees.iter().position(|x| *x == payee) {
            self.vault_state.payees.remove(index);
            Ok(())
        } else {
            err!(CustomError::PayeeNotFound)
        }
    }
}