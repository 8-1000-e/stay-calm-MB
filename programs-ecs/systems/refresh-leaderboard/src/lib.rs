use bolt_lang::*;
use game_config::GameConfig;
use leaderboard::{Leaderboard, LeaderboardEntry, MAX_LEADERBOARD};
use shared::*;

declare_id!("GPnGj91d5ZueVC5YLjZXLrWHHgVxJiQFPcpq4srWUkBG");

// Bolt prepends one AccountInfo per #[system_input] component, so player
// extras start at index 2 (game_config + leaderboard) — but we also pass
// PlayerRegistry as an extra (read-only, raw bytes), so player_state PDAs
// start at NUM_COMPONENTS + 1.
const NUM_COMPONENTS: usize = 2;

// PlayerRegistry layout to read `count` from raw bytes:
//   8 (disc) + 4 (Vec1.len = u32) + 32 * MAX_PLAYERS players +
//             4 (Vec2.len = u32) + 32 * MAX_PLAYERS player_states +
//             1 (count: u8) + bolt_metadata
//   At MAX_PLAYERS=10 (current cap, see player-registry component): count
//   sits at byte 656. Bumping MAX_PLAYERS requires updating this offset.
const PR_COUNT_OFFSET: usize = 8 + 4 + 32 * 10 + 4 + 32 * 10;

// PlayerState byte layout — see end-game for full doc.
//   authority = the back's signer (`DEVdk3sz...`) — NOT what we want
//   owner     = the player's wallet — leaderboard pubkey is sourced from here
// Layout offsets — leverage was bumped u8 → u16 for the ultra-aggressive
// tier set (up to 5000×), so every field after PS_POSITION shifts by +1.
const PS_OWNER: usize = 40;
const PS_ALIVE: usize = 72;
const PS_BALANCE: usize = 73;
const PS_LEVERAGE: usize = 82;       // 2 bytes (u16, little-endian)
const PS_POSITION_SIZE: usize = 92;  // was 91
const PS_REALIZED_PNL: usize = 108;  // was 107
const PS_UNREALIZED_PNL: usize = 116; // was 115
// PS_OPENED_AT = 124 (i64) — added when the entry-timestamp on-chain
// field landed. Read by the back's watcher, NOT by this system, but the
// min-len check has to bump so we don't reject the new account size.
const PS_MIN_LEN: usize = 132;       // was 124

/// Live leaderboard refresh, called by the cranker each tick (after the
/// per-player close-position passes). Same logic as end-game minus the
/// time/status guard and without flipping the game to Finished — so the front
/// always reads a fresh ranking on-chain.
///
/// PlayerRegistry is intentionally NOT in `#[system_input]`: Bolt echoes
/// every input component as return data, and at MAX_PLAYERS=10 the registry
/// is ~650 bytes — combined with GameConfig + Leaderboard it would blow
/// Solana's 1024-byte `set_return_data` cap. We pass it as the FIRST
/// extra account and read `count` from raw bytes instead.
///
/// remaining_accounts (after the 2 component-program slots Bolt prepends):
///   [NUM_COMPONENTS]              PlayerRegistry PDA (raw bytes — count read)
///   [NUM_COMPONENTS + 1 + i]      PlayerState PDA for player i (0..count)
#[system]
pub mod refresh_leaderboard {

    pub fn execute(ctx: Context<Components>, _args_p: Vec<u8>) -> Result<Components>
    {
        require!(ctx.accounts.game_config.status == 1, GameError::GameNotPlaying);

        // Read the registered player count from the PlayerRegistry PDA
        // passed as the first extra account.
        let registry_acc = &ctx.remaining_accounts[NUM_COMPONENTS];
        let registry_data = registry_acc.try_borrow_data()?;
        require!(registry_data.len() > PR_COUNT_OFFSET, GameError::InvalidAccount);
        let count = (registry_data[PR_COUNT_OFFSET] as usize).min(MAX_LEADERBOARD);
        drop(registry_data);

        // Heap-allocated so we don't burn stack on a large MAX_LEADERBOARD.
        let mut entries: Vec<LeaderboardEntry> = Vec::with_capacity(count);

        for i in 0..count {
            let acc = &ctx.remaining_accounts[NUM_COMPONENTS + 1 + i];
            let data = acc.try_borrow_data()?;
            if data.len() < PS_MIN_LEN { continue; }

            let mut owner = [0u8; 32];
            owner.copy_from_slice(&data[PS_OWNER..PS_OWNER + 32]);

            let alive = data[PS_ALIVE] != 0;
            let balance = u64::from_le_bytes(data[PS_BALANCE..PS_BALANCE + 8].try_into().unwrap());
            let leverage = u16::from_le_bytes(data[PS_LEVERAGE..PS_LEVERAGE + 2].try_into().unwrap());
            let position_size = u64::from_le_bytes(
                data[PS_POSITION_SIZE..PS_POSITION_SIZE + 8].try_into().unwrap()
            );
            let realized_pnl = i64::from_le_bytes(
                data[PS_REALIZED_PNL..PS_REALIZED_PNL + 8].try_into().unwrap()
            );
            let unrealized_pnl = i64::from_le_bytes(
                data[PS_UNREALIZED_PNL..PS_UNREALIZED_PNL + 8].try_into().unwrap()
            );
            drop(data);

            let margin = if leverage > 0 { (position_size / leverage as u64) as i64 } else { 0 };
            let net_worth = (balance as i64)
                .saturating_add(margin)
                .saturating_add(unrealized_pnl);

            entries.push(LeaderboardEntry {
                pubkey: owner,
                net_worth,
                balance,
                unrealized_pnl,
                realized_pnl,
                alive,
            });
        }

        // Insertion sort, net_worth DESC. Tie-breaker: alive ranks above dead.
        let filled = entries.len();
        for i in 1..filled {
            let mut j = i;
            while j > 0 {
                let a = &entries[j - 1];
                let b = &entries[j];
                let swap = b.net_worth > a.net_worth
                    || (b.net_worth == a.net_worth && b.alive && !a.alive);
                if !swap { break; }
                entries.swap(j - 1, j);
                j -= 1;
            }
        }

        for i in 0..filled {
            ctx.accounts.leaderboard.entries[i] = entries[i];
        }
        ctx.accounts.leaderboard.count = filled as u8;

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub game_config: GameConfig,
        pub leaderboard: Leaderboard,
    }
}
