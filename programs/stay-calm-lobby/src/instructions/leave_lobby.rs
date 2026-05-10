use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::*;
use crate::state::*;

/// Remove a player from an open lobby and refund their entry fee.
/// Signed by the backend authority on behalf of the player (verified via JWT on the API).
///
/// remaining_accounts[0] = player wallet (writable, receives refund)
pub fn leave_lobby(ctx: Context<LeaveLobby>, _lobby_id: u64, player: Pubkey) -> Result<()> {
    let lobby = &mut ctx.accounts.lobby;

    require!(lobby.status == STATUS_OPEN, LobbyError::LobbyNotOpen);
    require_keys_eq!(
        ctx.accounts.authority.key(),
        lobby.authority,
        LobbyError::Unauthorized
    );
    // Once the lobby has reached MIN_PLAYERS the match is committed: the
    // back's tick is about to call `start_match` and any leave at this
    // point would either drop us back below the threshold or race the
    // launch. Reject on-chain so a malicious client can't bypass the
    // front's UI guard. Same UX as red-light + airport-carousel.
    require!(
        (lobby.player_count as usize) < MIN_PLAYERS,
        LobbyError::LobbyLocked,
    );

    let count = lobby.player_count as usize;
    let idx = lobby.players[..count]
        .iter()
        .position(|p| p == &player)
        .ok_or(LobbyError::NotInLobby)?;

    let rem = ctx.remaining_accounts;
    require!(rem.len() >= 1, LobbyError::NotEnoughAccounts);
    require_keys_eq!(rem[0].key(), player, LobbyError::LeaderboardMismatch);

    // Refund entry_fee minus leave fee — leave fee goes to authority (treasury)
    let refund = lobby.entry_fee.saturating_sub(LEAVE_FEE);
    let fee = lobby.entry_fee - refund;
    ctx.accounts.vault.sub_lamports(lobby.entry_fee)?;
    rem[0].add_lamports(refund)?;
    ctx.accounts.authority.add_lamports(fee)?;

    for i in idx..(count - 1) {
        lobby.players[i] = lobby.players[i + 1];
    }
    lobby.players[count - 1] = Pubkey::default();
    lobby.player_count -= 1;

    let vault = &mut ctx.accounts.vault;
    vault.total_pot = vault.total_pot.saturating_sub(lobby.entry_fee);

    Ok(())
}

#[derive(Accounts)]
#[instruction(lobby_id: u64)]
pub struct LeaveLobby<'info> {
    #[account(
        mut,
        seeds = [LOBBY_SEED, lobby_id.to_le_bytes().as_ref()],
        bump = lobby.bump,
    )]
    pub lobby: Box<Account<'info, Lobby>>,

    #[account(
        mut,
        seeds = [VAULT_SEED, lobby.key().as_ref()],
        bump = vault.bump,
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(mut)]
    pub authority: Signer<'info>,
}
