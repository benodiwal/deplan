use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

declare_id!("GT8F97mEorgZ5PJzcFGgbZuQi9sNnmhdxydvZtzNkGb5");

#[program]
pub mod subscription_service {
    use super::*;

    pub fn initialize_content_provider(
        ctx: Context<InitializeContentProvider>,
        subscription_price: u64,
        subscription_duration: u64,
    ) -> Result<()> {
        let provider = &mut ctx.accounts.content_provider;
        provider.authority = provider.authority.key();
        provider.subscription_price = subscription_price;
        provider.subscription_duration = subscription_duration;
        provider.total_subscribers = 0;
        Ok(())
    }

    pub fn subscribe(ctx: Context<Subscribe>, subscription_start: i64) -> Result<()> {
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;
        require!(
            subscription_start >= current_time,
            SubscriptionError::InvalidStartTime
        );

        let provider = &ctx.accounts.content_provider;
        let subscription = &mut ctx.accounts.subscription;

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.subscriber_token_account.to_account_info(),
                    to: ctx.accounts.provider_token_account.to_account_info(),
                    authority: ctx.accounts.subscriber.to_account_info(),
                },
            ),
            provider.subscription_price,
        )?;

        subscription.subscriber = ctx.accounts.subscriber.key();
        subscription.provider = provider.key();
        subscription.start_time = subscription_start;
        subscription.end_time = subscription_start + provider.subscription_duration as i64;
        subscription.last_payment = current_time;
        subscription.auto_renewal = true;

        let provider = &mut ctx.accounts.content_provider;
        provider.total_subscribers += 1;

        Ok(())
    }

    pub fn process_renewal(ctx: Context<ProcessRenewal>) -> Result<()> {
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;
        let subscription = &mut ctx.accounts.subscription;
        let provider = &ctx.accounts.content_provider;

        require!(
            subscription.auto_renewal,
            SubscriptionError::AutoRenewalDisabled
        );

        require!(
            current_time >= subscription.end_time,
            SubscriptionError::SubscriptionStillActive,
        );

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.subscriber_token_account.to_account_info(),
                    to: ctx.accounts.provider_token_account.to_account_info(),
                    authority: ctx.accounts.subscriber.to_account_info(),
                },
            ),
            provider.subscription_price,
        )?;

        subscription.start_time = subscription.end_time;
        subscription.end_time = subscription.end_time + provider.subscription_duration as i64;
        subscription.last_payment = current_time;

        Ok(())
    }

    pub fn toggle_auto_renewal(ctx: Context<ToggleAutoRenewal>) -> Result<()> {
        let subscription = &mut ctx.accounts.subscription;
        subscription.auto_renewal = !subscription.auto_renewal;
        Ok(())
    }

    pub fn add_content(
        ctx: Context<AddContent>,
        content_id: String,
        content_hash: String,
        content_type: ContentType,
    ) -> Result<()> {
        let content = &mut ctx.accounts.content;
        content.content_id = content_id;
        content.content_hash = content_hash;
        content.content_type = content_type;
        content.timestamp = Clock::get()?.unix_timestamp;
        Ok(())
    }

    pub fn access_content(ctx: Context<AccessContext>) -> Result<()> {
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;
        let subscription = &ctx.accounts.subscription;

        require!(
            current_time >= subscription.start_time && current_time <= subscription.end_time,
            SubscriptionError::InactiveSubscription
        );

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeContentProvider<'info> {
    #[account(init, payer = authority, space = 8 + ContentProvider::LEN)]
    pub content_provider: Account<'info, ContentProvider>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Subscribe<'info> {
    #[account(mut)]
    pub content_provider: Account<'info, ContentProvider>,
    #[account(init, payer = subscriber, space = 8 + Subscription::LEN, seeds = [b"subscription", content_provider.key().as_ref(), subscriber.key.as_ref()], bump)]
    pub subscription: Account<'info, Subscription>,
    #[account(mut)]
    pub subscriber_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub provider_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    #[account(mut)]
    pub subscriber: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ProcessRenewal<'info> {
    pub content_provider: Account<'info, ContentProvider>,
    #[account(mut, seeds = [b"subscription", content_provider.key().as_ref(), subscriber.key.as_ref()], bump)]
    pub subscription: Account<'info, Subscription>,
    #[account(mut)]
    pub subscriber_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub provider_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub subscriber: Signer<'info>,
}

#[derive(Accounts)]
pub struct ToggleAutoRenewal<'info> {
    #[account(mut, constraint = subscription.subscriber == subscriber.key())]
    pub subscription: Account<'info, Subscription>,
    pub subscriber: Signer<'info>,
}

#[derive(Accounts)]
pub struct AddContent<'info> {
    #[account(constraint = content_provider.authority == authority.key())]
    pub content_provider: Account<'info, ContentProvider>,
    #[account(init, payer = authority, space = 8 + Content::LEN)]
    pub content: Account<'info, Content>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AccessContext<'info> {
    pub content: Account<'info, Content>,
    #[account(constraint = subscription.subscriber == subscriber.key() && subscription.provider == content.provider)]
    pub subscription: Account<'info, Subscription>,
    pub subscriber: Signer<'info>,
}

#[account]
pub struct ContentProvider {
    pub authority: Pubkey,
    pub subscription_price: u64,
    pub subscription_duration: u64,
    pub total_subscribers: u64,
}

impl ContentProvider {
    pub const LEN: usize = 32 + 8 + 8 + 8;
}

#[account]
pub struct Subscription {
    pub subscriber: Pubkey,
    pub provider: Pubkey,
    pub start_time: i64,
    pub end_time: i64,
    pub last_payment: i64,
    pub auto_renewal: bool,
}

impl Subscription {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 8 + 1;
}

#[account]
pub struct Content {
    pub provider: Pubkey,
    pub content_id: String,
    pub content_hash: String,
    pub content_type: ContentType,
    pub timestamp: i64,
}

impl Content {
    pub const LEN: usize = 32 + 64 + 64 + 1 + 8;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum ContentType {
    Video,
    Article,
    Audio,
    Other,
}

#[error_code]
pub enum SubscriptionError {
    #[msg("Invalid subscription start time")]
    InvalidStartTime,
    #[msg("Auto-renewal is disabled for this subscription")]
    AutoRenewalDisabled,
    #[msg("Subscription is still active")]
    SubscriptionStillActive,
    #[msg("Subscription is not active")]
    InactiveSubscription,
}
