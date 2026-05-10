use bolt_lang::*;
use game_config::GameConfig;
use leaderboard::{Leaderboard, LeaderboardEntry, MAX_LEADERBOARD};
use shared::*;

declare_id!("2vS1gLo4GZ31QvBzdve6EdgnoSkX5EJ1kcNxmQSXWG4n");

/// Bolt prepends one AccountInfo per #[system_input] component to remaining_
/// accounts before our extras. PlayerRegistry is NOT in #[system_input] —
/// see refresh-leaderboard for the 1024-byte return-data rationale — so the
/// registry sits at NUM_COMPONENTS and PlayerState PDAs at NUM_COMPONENTS+1+i.
const NUM_COMPONENTS: usize = 2;

// PlayerRegistry layout to read `count` from raw bytes — same offset as in
// refresh-leaderboard. Bumping MAX_PLAYERS in the registry component
// requires updating this offset here too.
const PR_COUNT_OFFSET: usize = 8 + 4 + 32 * 10 + 4 + 32 * 10;

/// PlayerState byte layout (8-byte Anchor discriminator + fields, little-endian):
///   8..40    authority    (Pubkey, 32)  — back signer; do NOT use for leaderboard
///   40..72   owner        (Pubkey, 32)  — player wallet — leaderboard pubkey
///   72       alive        (u8: 0/1)
///   73..81   balance      (u64)
///   81       position     (u8)
///   82..84   leverage     (u16)
///   84..92   entry_price  (u64)
///   92..100  position_size(u64)
///   100..108 liq_price    (u64)
///   108..116 realized_pnl (i64)
///   116..124 unrealized_pnl(i64)
///   124..132 opened_at    (i64)        ← added with the entry-timestamp field
const PS_OWNER: usize = 40;
const PS_ALIVE: usize = 72;
const PS_BALANCE: usize = 73;
const PS_LEVERAGE: usize = 82;       // 2 bytes (u16)
const PS_POSITION_SIZE: usize = 92;
const PS_REALIZED_PNL: usize = 108;
const PS_UNREALIZED_PNL: usize = 116;
const PS_MIN_LEN: usize = 132;

/// Finalizes a game: sets status=Finished and writes the sorted leaderboard.
///
/// remaining_accounts (after the 2 component-program slots Bolt prepends):
///   [NUM_COMPONENTS]              PlayerRegistry PDA (raw bytes — count read)
///   [NUM_COMPONENTS + 1 + i]      PlayerState PDA for player i (0..count)
#[system]
pub mod end_game {

    pub fn execute(ctx: Context<Components>, _args_p: Vec<u8>) -> Result<Components>
    {
        require!(ctx.accounts.game_config.status == 1, GameError::GameNotPlaying);
        let now = Clock::get()?.unix_timestamp;
        require!(now >= ctx.accounts.game_config.game_end, GameError::GameNotOver);

        // Same trick as refresh-leaderboard: read PlayerRegistry.count from
        // the raw bytes of an extra account so it doesn't get echoed back
        // as return data and blow the 1024-byte set_return_data limit.
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

            // Net worth = free cash + locked margin + flying PnL
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
        ctx.accounts.game_config.status = 2;

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub game_config: GameConfig,
        pub leaderboard: Leaderboard,
    }
}
