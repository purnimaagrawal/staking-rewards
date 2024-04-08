use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer, TokenAccount, Token};

declare_id!("4bo6Z1PdaES2ZrdQ1xP8hSXtb365HFytJiFzbANi5X6X");

#[program]
pub mod staking_rewards {
    use super::*;

   
    pub fn initialize(ctx: Context<Initialize>, reward_rate: u64,duration:u64) -> Result<()> {
        msg!("Instruction: Initialize");

        let staking_rewards = &mut ctx.accounts.staking_rewards;
        staking_rewards.owner = *ctx.accounts.owner.key;
        staking_rewards.staking_token = ctx.accounts.staking_token.key();
        staking_rewards.rewards_token = ctx.accounts.rewards_token.key();
        staking_rewards.duration = duration;
        staking_rewards.total_supply = 0;
        staking_rewards.updated_at = Clock::get()?.unix_timestamp as u64;
        staking_rewards.reward_per_token_stored = 0;
        staking_rewards.finish_at = staking_rewards.updated_at + duration;
        staking_rewards.reward_rate = reward_rate;

        Ok(())

    }

    impl<'info> Stake<'info> {
        fn into_transfer_to_pool_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
            let cpi_accounts = Transfer {
                from: self.staker_token_account.to_account_info(),
                to: self.pool_token_account.to_account_info(),
                authority: self.staker.to_account_info(),
            };
            CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
        }
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        msg!("Instruction: Stake");

        // Ensure the amount is greater than 0
    require!(amount > 0, StakingError::AmountIsZero);

    //update rewards
    ctx.accounts.staking_rewards.update_reward(&mut ctx.accounts.staker_state)?;

    // Transfer tokens from the staker's account to the staking pool's account
    token::transfer(ctx.accounts.into_transfer_to_pool_context(), amount)?;

    // Update the staker's state
    ctx.accounts.staker_state.amount += amount;
    // Update the total supply in the staking rewards state
    ctx.accounts.staking_rewards.total_supply += amount;

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>,amount: u64) -> Result<()> {
        msg!("Instruction: Withdraw");

        // Ensure the withdrawal amount is greater than 0 and that the staker has enough staked tokens.
        require!(amount > 0, StakingError::AmountIsZero);
        require!(ctx.accounts.staker_state.amount >= amount, StakingError::InsufficientStakedAmount);
    
        // Update reward for the staker before proceeding with the withdrawal
        ctx.accounts.staking_rewards.update_reward(&mut ctx.accounts.staker_state)?;

        // Transfer tokens from the pool's account back to the staker's account
        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_token_account.to_account_info(),
            to: ctx.accounts.staker_token_account.to_account_info(),
            authority: ctx.accounts.staking_rewards.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;
    
        // Update the staker's state and the total supply in the staking rewards state
        ctx.accounts.staker_state.amount -= amount;
        ctx.accounts.staking_rewards.total_supply -= amount;
    
        Ok(())

    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        msg!("Instruction: ClaimRewards");

        //update the staker's rewards
        ctx.accounts.staking_rewards_state.update_reward(&mut ctx.accounts.staker_state)?;
    
        // Calculate the amount of rewards to claim.
        let rewards_to_claim = ctx.accounts.staker_state.rewards;
    
        // Ensure there are rewards to claim.
        require!(rewards_to_claim > 0, StakingError::NoRewards);
    
        // Prepare for the SPL Token transfer.
        let cpi_accounts = Transfer {
            from: ctx.accounts.rewards_pool.to_account_info(),
            to: ctx.accounts.claimer_rewards_account.to_account_info(),
            authority: ctx.accounts.staking_rewards_state.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    
        // Perform the token transfer from the rewards pool to the claimer's rewards account.
        token::transfer(cpi_ctx, rewards_to_claim)?;
    
        // Reset the staker's rewards balance.
        ctx.accounts.staker_state.rewards = 0;
    

        Ok(())
    }
}
impl StakingRewardsState {
    fn reward_per_token(&self) -> Result<u64> {
        if self.total_supply == 0 {
            Ok(self.reward_per_token_stored)
        } else {
            let time_since_last_update = Clock::get()?.unix_timestamp as u64 - self.updated_at;
            let reward_added = self.reward_rate* time_since_last_update;
            let reward_per_token_increment = reward_added * 1_000_000_000 / self.total_supply;
            Ok(self.reward_per_token_stored + reward_per_token_increment)
        }
    }

    fn update_reward(&mut self, staker_state: &mut Account<StakerState>) -> Result<()> {
        let reward_per_token = self.reward_per_token()?;
        let earned = staker_state.calculate_earned(reward_per_token)?;

        self.updated_at = Clock::get()?.unix_timestamp as u64;
        staker_state.rewards = earned;
        staker_state.reward_per_token_paid = reward_per_token;

        Ok(())
    }
}

impl StakerState {
    fn calculate_earned(&self, reward_per_token: u64) -> Result<u64> {
        let accumulated_reward = self.amount * (reward_per_token - self.reward_per_token_paid) / 1_000_000_000;
        Ok(accumulated_reward + self.rewards)
    }
}
#[derive(Accounts)]
pub struct Initialize<'info> {

    #[account(init, payer = owner, space = 8 + 256)]
    pub staking_rewards: Account<'info, StakingRewardsState>,
    #[account(mut)]
        /// The account that pays for the initialization and will be the owner of the staking_rewards account.
    pub owner: Signer<'info>,
    pub staking_token: Account<'info, TokenAccount>,
    pub rewards_token: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
      /// The staker's main account, which signs the transaction.
    #[account(mut)]
    pub staker: Signer<'info>,
    /// The staker's SPL token account for the staking token.
    #[account(mut)]
    pub staker_token_account: Account<'info, TokenAccount>,
    // The staking pool's SPL token account for the staking token
    #[account(mut)]
    pub pool_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub staker_state: Account<'info, StakerState>,
    /// The state of the staking rewards program.
    #[account(mut)]
    pub staking_rewards: Account<'info, StakingRewardsState>,
    /// The SPL Token program.
    pub token_program: Program<'info, Token>,
     /// To pay for transaction fees and potentially create new accounts.
     pub system_program: Program<'info, System>,
     /// To access clock or timestamp information if needed.
     pub clock: Sysvar<'info, Clock>,

}
#[account]
pub struct StakerState {
    pub amount: u64,
    pub reward_per_token_paid: u64,
    pub rewards: u64,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub staker: Signer<'info>,
    #[account(mut)]
    pub staker_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub staker_state: Account<'info, StakerState>,
    #[account(mut)]
    pub staking_rewards: Account<'info, StakingRewardsState>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,

}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub claimer: Signer<'info>,
    /// Account that holds the staker's state.
    #[account(mut)]
    pub staker_state: Account<'info, StakerState>,
    /// Account to send rewards from (the rewards pool).
    #[account(mut)]
    pub rewards_pool: Account<'info, TokenAccount>,
    /// The claimer's rewards token account to receive the rewards.
    #[account(mut)]
    pub claimer_rewards_account: Account<'info, TokenAccount>,
    /// The SPL Token program.
    pub token_program: Program<'info, Token>,
    /// Account holding the staking program's state.
    #[account(mut)]
    pub staking_rewards_state: Account<'info, StakingRewardsState>,
}


#[account]
pub struct StakingRewardsState {
  pub owner: Pubkey,
  pub duration: u64,
  pub staking_token: Pubkey,
  pub rewards_token: Pubkey,
  pub total_supply: u64,
  pub updated_at: u64,
  pub reward_per_token_stored: u64,
  pub reward_rate: u64,
  pub finish_at: u64,

}
#[error_code]
pub enum StakingError {
    #[msg("The staking amount cannot be zero.")]
    AmountIsZero,
    #[msg("Insufficient staked amount for withdrawal.")]
    InsufficientStakedAmount,

    #[msg("An error occurred during arithmetic operation.")]
    MathError,
    #[msg("No rewards available to claim.")]
    NoRewards,
}
