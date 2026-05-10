use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::*;
use crate::state::*;

pub fn start_match(ctx: Context<StartMatch>, _lobby_id: u64) -> Result<()> {
    let lobby = &mut ctx.accounts.lobby;

    require!(lobby.status == STATUS_OPEN, LobbyError::LobbyNotOpen);
    require_keys_eq!(
        ctx.accounts.authority.key(),
        lobby.authority,
        LobbyError::Unauthorized
    );

    lobby.status = STATUS_STARTED;
    lobby.started_at = Clock::get()?.unix_timestamp;

    Ok(())
}

#[derive(Accounts)]
#[instruction(lobby_id: u64)]
pub struct StartMatch<'info> {
    #[account(
        mut,
        seeds = [LOBBY_SEED, lobby_id.to_le_bytes().as_ref()],
        bump = lobby.bump,
    )]
    pub lobby: Box<Account<'info, Lobby>>,

    /// Must match lobby.authority — enforced in the handler.
    pub authority: Signer<'info>,
}
