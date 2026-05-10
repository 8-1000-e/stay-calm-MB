use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::*;
use crate::state::*;
use crate::PrizeDistributed;

/// Distribution:
///   - Top half = floor(player_count / 2) slots, equal shares of (1 - fee_bps) of pot
///   - Ties at the cutoff (same net_worth as entries[cutoff-1]) split the last slot
///   - Treasury always gets fee_bps (100% if no winners)
///
/// Anti-scam: the handler decodes the on-chain `leaderboard` account and verifies
/// that remaining_accounts[i] matches leaderboard.entries[i].pubkey for every
/// paid position. The back cannot substitute its own wallets.
///
/// `player_count` is the total number of players at match start (= active_players
/// from GameConfig, never decremented). Passed by the back as an arg.
pub fn distribute_prize(ctx: Context<DistributePrize>, _lobby_id: u64, player_count: u8) -> Result<()> {
    let lobby = &ctx.accounts.lobby;

    require!(lobby.status == STATUS_STARTED, LobbyError::LobbyNotStarted);
    require_keys_eq!(
        ctx.accounts.authority.key(),
        lobby.authority,
        LobbyError::Unauthorized
    );
    require_keys_eq!(
        ctx.accounts.treasury.key(),
        lobby.authority,
        LobbyError::InvalidTreasury
    );

    // 1. Verify the leaderboard account is owned by the BOLT leaderboard component program
    require_keys_eq!(
        *ctx.accounts.leaderboard.owner,
        LEADERBOARD_COMPONENT_ID,
        LobbyError::InvalidLeaderboardOwner
    );

    // 2. Decode count + entries manually from raw bytes
    let lb_data = ctx.accounts.leaderboard.try_borrow_data()?;
    require!(
        lb_data.len() >= LEADERBOARD_COUNT_OFFSET + 1,
        LobbyError::LeaderboardTooSmall
    );
    let count = lb_data[LEADERBOARD_COUNT_OFFSET].min(MAX_PLAYERS as u8);
    let entries_slice =
        &lb_data[LEADERBOARD_ENTRIES_OFFSET..LEADERBOARD_ENTRIES_OFFSET + LEADERBOARD_ENTRIES_LEN];

    // 3. Compute cutoff. Clamp to on-chain leaderboard count — players who
    //    never opened a position may not be in the leaderboard.
    let desired_cutoff = player_count as usize / 2;
    let cutoff = desired_cutoff.min(count as usize);

    let total_pot = ctx.accounts.vault.total_pot;

    // If no winners (nobody ranked), treasury takes everything
    if cutoff == 0 {
        ctx.accounts.vault.sub_lamports(total_pot)?;
        ctx.accounts.treasury.add_lamports(total_pot)?;
        let vault_mut = &mut ctx.accounts.vault;
        vault_mut.total_pot = 0;
        let lobby_mut = &mut ctx.accounts.lobby;
        lobby_mut.status = STATUS_SETTLED;
        emit!(PrizeDistributed {
            lobby_id: lobby_mut.lobby_id,
            total_pot,
            treasury_cut: total_pot,
            winner_count: 0,
            winner_pubkeys: Vec::new(),
            winner_amounts: Vec::new(),
        });
        return Ok(());
    }

    // Dynamic fee: 2-player matches (1 winner) pay higher % to cover tx fees.
    //   player_count <= 2 → 8%
    //   player_count >= 3 → PLATFORM_FEE_BPS (5%)
    let fee_bps: u64 = if player_count <= 2 { 800 } else { PLATFORM_FEE_BPS };
    let treasury_cut = total_pot * fee_bps / 10000;
    let winner_pool = total_pot - treasury_cut;
    let prize_per_slot = winner_pool / cutoff as u64;

    // 4. Detect tie at cutoff: last winning index is cutoff-1. If any entries
    //    at indices >= cutoff have the same net_worth as entries[cutoff-1],
    //    include them as tied winners sharing the last slot.
    let last_idx = cutoff - 1;
    let last_entry_offset = last_idx * LEADERBOARD_ENTRY_SIZE;
    let last_entry_net_worth = i64::from_le_bytes(
        entries_slice
            [last_entry_offset + LEADERBOARD_ENTRY_NET_WORTH_OFFSET
                ..last_entry_offset + LEADERBOARD_ENTRY_NET_WORTH_OFFSET + 8]
            .try_into()
            .expect("slice of length 8 must convert"),
    );

    let mut tied_count: usize = 1; // includes last_idx itself
    let mut i = cutoff; // index after last_idx
    while i < count as usize {
        let off = i * LEADERBOARD_ENTRY_SIZE;
        let e_net_worth = i64::from_le_bytes(
            entries_slice
                [off + LEADERBOARD_ENTRY_NET_WORTH_OFFSET
                    ..off + LEADERBOARD_ENTRY_NET_WORTH_OFFSET + 8]
                .try_into()
                .expect("slice of length 8 must convert"),
        );
        if e_net_worth == last_entry_net_worth {
            tied_count += 1;
            i += 1;
        } else {
            break;
        }
    }

    // Total number of addresses that receive a prize
    let total_winners = cutoff - 1 + tied_count; // (cutoff-1) normal + tied_count tied

    // 5. Verify remaining_accounts match leaderboard.entries[i].pubkey for 0..total_winners
    let rem = ctx.remaining_accounts;
    require!(rem.len() >= total_winners, LobbyError::NotEnoughAccounts);
    for i in 0..total_winners {
        let off = i * LEADERBOARD_ENTRY_SIZE;
        let expected_bytes: [u8; 32] = entries_slice[off..off + 32]
            .try_into()
            .expect("slice of length 32 must convert");
        let expected_pk = Pubkey::new_from_array(expected_bytes);
        require_keys_eq!(rem[i].key(), expected_pk, LobbyError::LeaderboardMismatch);
    }
    drop(lb_data);

    // 6. Compute per-winner cuts
    // Ranks 1..cutoff-1 (i.e. indices 0..cutoff-2): prize_per_slot each.
    // Tied group (indices cutoff-1..cutoff-1+tied_count-1): prize_per_slot / tied_count each.
    let tied_prize = prize_per_slot / tied_count as u64;

    // 7. Perform the transfers
    let mut winner_pubkeys = Vec::with_capacity(total_winners);
    let mut winner_amounts = Vec::with_capacity(total_winners);

    for i in 0..(cutoff - 1) {
        ctx.accounts.vault.sub_lamports(prize_per_slot)?;
        rem[i].add_lamports(prize_per_slot)?;
        winner_pubkeys.push(rem[i].key());
        winner_amounts.push(prize_per_slot);
    }
    for i in 0..tied_count {
        let idx = (cutoff - 1) + i;
        ctx.accounts.vault.sub_lamports(tied_prize)?;
        rem[idx].add_lamports(tied_prize)?;
        winner_pubkeys.push(rem[idx].key());
        winner_amounts.push(tied_prize);
    }

    // Treasury transfer
    ctx.accounts.vault.sub_lamports(treasury_cut)?;
    ctx.accounts.treasury.add_lamports(treasury_cut)?;

    // 8. Bookkeeping
    let paid: u64 = winner_amounts.iter().sum::<u64>() + treasury_cut;
    let vault_mut = &mut ctx.accounts.vault;
    vault_mut.total_pot = vault_mut.total_pot.saturating_sub(paid);

    let lobby_mut = &mut ctx.accounts.lobby;
    lobby_mut.status = STATUS_SETTLED;

    // 9. Emit event
    emit!(PrizeDistributed {
        lobby_id: lobby_mut.lobby_id,
        total_pot,
        treasury_cut,
        winner_count: total_winners as u8,
        winner_pubkeys,
        winner_amounts,
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(lobby_id: u64)]
pub struct DistributePrize<'info> {
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

    /// Trade-fight leaderboard PDA. Ownership check performed in the handler.
    /// CHECK: raw data, decoded manually.
    pub leaderboard: UncheckedAccount<'info>,

    /// Must equal lobby.authority — guards against rake redirection.
    /// CHECK: key match enforced in handler.
    #[account(mut)]
    pub treasury: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
}
