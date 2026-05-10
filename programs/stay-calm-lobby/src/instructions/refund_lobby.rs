use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::*;
use crate::state::*;

/// Refund all players their entry fee from the vault. Called by the back
/// when a match launch fails after players have already paid. The lobby
/// must NOT already be settled.
///
/// remaining_accounts: player wallets in the same order as lobby.players[]
/// (verified on-chain — anti-scam).
pub fn refund_lobby(ctx: Context<RefundLobby>, _lobby_id: u64) -> Result<()> {
    let lobby = &ctx.accounts.lobby;

    require!(lobby.status != STATUS_SETTLED, LobbyError::AlreadySettled);
    require_keys_eq!(
        ctx.accounts.authority.key(),
        lobby.authority,
        LobbyError::Unauthorized
    );

    let count = lobby.player_count as usize;
    let entry_fee = lobby.entry_fee;
    let rem = ctx.remaining_accounts;
    require!(rem.len() >= count, LobbyError::NotEnoughAccounts);

    // Verify each remaining_account matches lobby.players[i]
    for i in 0..count {
        require_keys_eq!(
            rem[i].key(),
            lobby.players[i],
            LobbyError::LeaderboardMismatch
        );
    }

    // Refund each player
    for i in 0..count {
        ctx.accounts.vault.sub_lamports(entry_fee)?;
        rem[i].add_lamports(entry_fee)?;
    }

    // Zero out the pot and mark as settled so close_lobby can reclaim rent
    let vault_mut = &mut ctx.accounts.vault;
    vault_mut.total_pot = 0;

    let lobby_mut = &mut ctx.accounts.lobby;
    lobby_mut.status = STATUS_SETTLED;

    Ok(())
}

#[derive(Accounts)]
#[instruction(lobby_id: u64)]
pub struct RefundLobby<'info> {
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

    pub authority: Signer<'info>,
}
