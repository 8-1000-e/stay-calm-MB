use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::*;
use crate::state::*;

/// Close a settled lobby. The Lobby and Vault PDAs are closed, returning
/// their rent-exempt lamports to the authority. Any dust left in the vault
/// (rounding residuals from distribute_prize) is also swept to authority.
pub fn close_lobby(ctx: Context<CloseLobby>, _lobby_id: u64) -> Result<()> {
    let lobby = &ctx.accounts.lobby;
    require!(lobby.status == STATUS_SETTLED, LobbyError::AlreadySettled);
    require_keys_eq!(
        ctx.accounts.authority.key(),
        lobby.authority,
        LobbyError::Unauthorized
    );
    Ok(())
}

#[derive(Accounts)]
#[instruction(lobby_id: u64)]
pub struct CloseLobby<'info> {
    #[account(
        mut,
        seeds = [LOBBY_SEED, lobby_id.to_le_bytes().as_ref()],
        bump = lobby.bump,
        close = authority,
    )]
    pub lobby: Box<Account<'info, Lobby>>,

    #[account(
        mut,
        seeds = [VAULT_SEED, lobby.key().as_ref()],
        bump = vault.bump,
        close = authority,
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(mut)]
    pub authority: Signer<'info>,
}
