
// Import Anchor framework and system program transfer utilities
use anchor_lang::{prelude::*, system_program::{Transfer, transfer}};

// Program ID for the deployed contract
// declare_id!("8GXzdcCDdBr7MaLwAULrxG1KBYB5yjVVva2cs8cemoRb");
declare_id!("8b4G7EpokrxCpb1BMK5kjFMtLFnGFqLetxvR3ou17cS8");

/// Custom errors for the vault program
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
    #[msg("Invalid payout schedule")]
    InvalidPayoutSchedule,
    #[msg("Maximum number of payout schedules reached")]
    MaxSchedulesReached,
    #[msg("Payout schedule not found")]
    ScheduleNotFound,
    #[msg("Payout time not reached")]
    PayoutTimeNotReached,
    #[msg("Epoch spending limit reached")]
    EpochSpendingLimitReached,
    #[msg("Invalid epoch configuration")]
    InvalidEpochConfig,
}

/// Main program module for devquest_vault
#[program]
pub mod devquest_vault {
    use super::*;

    /// Initializes the vault and vault state accounts
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)?;
        Ok(())
    }

    /// Adds a new payee to the vault (admin only)
    pub fn add_payee(ctx: Context<UpdatePayee>, payee: Pubkey) -> Result<()> {
        ctx.accounts.add_payee(payee)
    }

    /// Removes a payee from the vault (admin only)
    pub fn remove_payee(ctx: Context<UpdatePayee>, payee: Pubkey) -> Result<()> {
        ctx.accounts.remove_payee(payee)
    }

    /// Deposits SOL into the vault
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)?;
        Ok(())
    }

    /// Sets an epoch spending limit for a payee (admin only)
    pub fn set_epoch_limit(
        ctx: Context<UpdatePayee>,
        payee: Pubkey,
        limit: u64,
        duration: i64
    ) -> Result<()> {
        require!(duration > 0, CustomError::InvalidEpochConfig);
        require!(limit > 0, CustomError::InvalidEpochConfig);

        let state = &mut ctx.accounts.vault_state;
        // Only admin can set limits
        require!(ctx.accounts.user.key() == state.admin, CustomError::UnauthorizedAdmin);
        // Check if payee exists
        require!(state.payees.contains(&payee), CustomError::PayeeNotFound);
        let now = Clock::get()?.unix_timestamp;
        // Find existing epoch limit or create new one
        if let Some(index) = state.epoch_limits.iter().position(|(p, _)| p == &payee) {
            state.epoch_limits[index].1 = EpochSpending {
                epoch_start: now,
                spent_amount: 0,
                limit,
                duration,
            };
        } else {
            state.epoch_limits.push((
                payee,
                EpochSpending {
                    epoch_start: now,
                    spent_amount: 0,
                    limit,
                    duration,
                }
            ));
        }
        Ok(())
    }

    /// Withdraws SOL from the vault (admin or authorized payee)
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)?;
        Ok(())
    }

    /// Closes the vault and returns remaining funds to the admin
    pub fn close(ctx: Context<Close>) -> Result<()> {
        ctx.accounts.close()?;
        Ok(())
    }

    /// Schedules a recurring payout for a payee (admin only)
    pub fn schedule_payout(
        ctx: Context<UpdatePayee>,
        payee: Pubkey,
        amount: u64,
        start_time: i64,
        interval: i64,
    ) -> Result<()> {
        ctx.accounts.schedule_payout(payee, amount, start_time, interval)
    }

    /// Cancels a payout schedule for a payee (admin only)
    pub fn cancel_payout(
        ctx: Context<UpdatePayee>,
        payee: Pubkey,
    ) -> Result<()> {
        ctx.accounts.cancel_payout(payee)
    }

    /// Allows a payee to claim their scheduled payout
    pub fn claim_payout(
        ctx: Context<Withdraw>,
    ) -> Result<()> {
        ctx.accounts.claim_payout()
    }
}

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

/// Accounts required for withdrawing SOL from the vault
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
    /// Handler for withdrawal logic (admin or authorized payee)
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        // Check if user is admin or authorized payee
        if self.user.key() != self.vault_state.admin &&
           !self.vault_state.payees.contains(&self.user.key()) {
            return err!(CustomError::UnauthorizedPayee);
        }
        // If user is not admin, check epoch spending limits
        if self.user.key() != self.vault_state.admin {
            let now = Clock::get()?.unix_timestamp;
            if let Some((_, epoch_spending)) = self.vault_state.epoch_limits
                .iter_mut()
                .find(|(p, _)| p == &self.user.key())
            {
                // Reset epoch if needed
                if now >= epoch_spending.epoch_start + epoch_spending.duration {
                    epoch_spending.epoch_start = now;
                    epoch_spending.spent_amount = 0;
                }
                // Check if withdrawal exceeds limit
                if epoch_spending.spent_amount + amount > epoch_spending.limit {
                    return err!(CustomError::EpochSpendingLimitReached);
                }
                // Update spent amount
                epoch_spending.spent_amount += amount;
            }
        }
        // Perform the withdrawal from vault to user
        let cpi_program = self.system_program.to_account_info();
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };
        let vault_state_key = self.vault_state.to_account_info().key;
        let vault_bump = self.vault_state.vault_bump;
        let seeds = &[
            b"vault",
            vault_state_key.as_ref(),
            &[vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer(cpi_ctx, amount)?;
        Ok(())
    }

    /// Handler for claiming a scheduled payout (payee only)
    pub fn claim_payout(&mut self) -> Result<()> {
        let user_key = self.user.key();
        require!(self.vault_state.payees.contains(&user_key), CustomError::UnauthorizedPayee);
        let current_time = Clock::get()?.unix_timestamp;
        // Find the active payout schedule
        let schedule_index = self.vault_state.payout_schedules
            .iter()
            .position(|s| s.is_active)
            .ok_or(error!(CustomError::ScheduleNotFound))?;
        // Check if it's time for payout
        let schedule = &self.vault_state.payout_schedules[schedule_index];
        require!(current_time >= schedule.next_payout_time, CustomError::PayoutTimeNotReached);
        let amount = schedule.amount;
        // Transfer the scheduled amount from vault to user
        let cpi_program = self.system_program.to_account_info();
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };
        let vault_state_key = self.vault_state.to_account_info().key;
        let vault_bump = self.vault_state.vault_bump;
        let seeds = &[
            b"vault",
            vault_state_key.as_ref(),
            &[vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer(cpi_ctx, amount)?;
        // Update next payout time for the schedule
        self.vault_state.payout_schedules[schedule_index].next_payout_time += 
            self.vault_state.payout_schedules[schedule_index].interval;
        Ok(())
    }
}

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

/// Data structure for a scheduled payout
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Default)]
pub struct PayoutSchedule {
    pub amount: u64,                 // Amount to be paid
    pub next_payout_time: i64,       // Timestamp for next payout
    pub interval: i64,               // Time between payouts (in seconds)
    pub is_active: bool,             // Whether this schedule is active
}

/// Data structure for tracking epoch-based spending limits
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Default)]
pub struct EpochSpending {
    pub epoch_start: i64,            // Start time of current epoch
    pub spent_amount: u64,           // Amount spent in current epoch
    pub limit: u64,                  // Maximum amount that can be spent in an epoch
    pub duration: i64,               // Duration of epoch in seconds (e.g., 86400 for daily)
}

/// Main vault state account
#[account]
pub struct VaultState {
    pub vault_bump: u8,
    pub state_bump: u8,
    pub admin: Pubkey,
    pub payees: Vec<Pubkey>,         // List of authorized payees
    pub payout_schedules: Vec<PayoutSchedule>,  // Scheduled payouts for each payee
    pub epoch_limits: Vec<(Pubkey, EpochSpending)>,  // Spending limits per payee
    pub is_initialized: bool,
}

impl Space for VaultState {
    // Calculate the required space for the VaultState account
    // 8 discriminator + 1 vault_bump + 1 state_bump + 32 admin + 
    // 4 vec length + (32 * 5) max payees + 
    // 4 vec length + (8 + 8 + 8 + 1) * 5 max schedules + 
    // 4 vec length + (32 + (8 + 8 + 8 + 8)) * 5 max epoch limits +
    // 1 is_initialized
    const INIT_SPACE: usize = 8 + 1 + 1 + 32 + 4 + (32 * 5) + 4 + (25 * 5) + 4 + (64 * 5) + 1;
}

/// Accounts required for updating payees and payout schedules
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
    /// Handler for adding a new payee (admin only)
    pub fn add_payee(&mut self, payee: Pubkey) -> Result<()> {
        require!(self.vault_state.payees.len() < 5, CustomError::MaxPayeesReached);
        require!(!self.vault_state.payees.contains(&payee), CustomError::PayeeAlreadyExists);
        self.vault_state.payees.push(payee);
        Ok(())
    }

    /// Handler for removing a payee (admin only)
    pub fn remove_payee(&mut self, payee: Pubkey) -> Result<()> {
        if let Some(index) = self.vault_state.payees.iter().position(|x| *x == payee) {
            self.vault_state.payees.remove(index);
            // Also remove any associated payout schedule
            let schedule_index = self.vault_state.payout_schedules
                .iter()
                .position(|s| s.is_active);
            if let Some(idx) = schedule_index {
                self.vault_state.payout_schedules.remove(idx);
            }
            Ok(())
        } else {
            err!(CustomError::PayeeNotFound)
        }
    }

    /// Handler for scheduling a payout (admin only)
    pub fn schedule_payout(
        &mut self,
        payee: Pubkey,
        amount: u64,
        start_time: i64,
        interval: i64,
    ) -> Result<()> {
        // Validate inputs
        require!(amount > 0, CustomError::InvalidPayoutSchedule);
        require!(interval > 0, CustomError::InvalidPayoutSchedule);
        require!(start_time > Clock::get()?.unix_timestamp, CustomError::InvalidPayoutSchedule);
        require!(self.vault_state.payees.contains(&payee), CustomError::PayeeNotFound);
        require!(self.vault_state.payout_schedules.len() < 5, CustomError::MaxSchedulesReached);
        let schedule = PayoutSchedule {
            amount,
            next_payout_time: start_time,
            interval,
            is_active: true,
        };
        self.vault_state.payout_schedules.push(schedule);
        Ok(())
    }

    /// Handler for cancelling a payout schedule (admin only)
    pub fn cancel_payout(&mut self, payee: Pubkey) -> Result<()> {
        require!(self.vault_state.payees.contains(&payee), CustomError::PayeeNotFound);
        if let Some(schedule_index) = self.vault_state.payout_schedules.iter().position(|_| true) {
            self.vault_state.payout_schedules[schedule_index].is_active = false;
            Ok(())
        } else {
            err!(CustomError::ScheduleNotFound)
        }
    }
}