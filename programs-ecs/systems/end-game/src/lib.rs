use bolt_lang::*;
use game_config::GameConfig;
use leaderboard::{Leaderboard, LeaderboardEntry, MAX_LEADERBOARD};
use shared::*;

declare_id!("3kgTK9xexSGMbYdEnDpy5337XmPdWuA3SZJxc6ru1kDv");

// Same indexing convention as refresh-leaderboard: Bolt prepends one
// AccountInfo per `#[system_input]` component, so the 2 component slots
// come first, then PlayerRegistry as extras[0], then PlayerState PDAs.
const NUM_COMPONENTS: usize = 2;

// PlayerRegistry layout — read `count` from raw bytes. Cap mirrors
// player-registry's MAX_PLAYERS = 10. Bumping it requires updating this
// offset everywhere it appears (see refresh-leaderboard for the full
// layout commentary).
const PR_COUNT_OFFSET: usize = 8 + 4 + 32 * 10 + 4 + 32 * 10;

// PlayerState byte layout — see refresh-leaderboard for the full table.
const PS_OWNER: usize = 40;
const PS_ATTEMPTS_LEFT: usize = 72;
const PS_SCORE: usize = 73;
const PS_MIN_LEN: usize = PS_SCORE + 8;

/// Finalizes a match: gates on `now >= game_end`, writes the final sorted
/// leaderboard from every PlayerState's `score`, and flips
/// `GameConfig.status` to 2 (Finished). Same `remaining_accounts` shape
/// as `refresh-leaderboard`:
///   [NUM_COMPONENTS]              PlayerRegistry PDA (raw bytes — count read)
///   [NUM_COMPONENTS + 1 + i]      PlayerState PDA for player i (0..count)
///
/// After this lands, the lobby program can read the on-chain Leaderboard
/// component and call `distribute_prize` to settle the pot.
#[system]
pub mod end_game {

    pub fn execute(ctx: Context<Components>, _args_p: Vec<u8>) -> Result<Components> {
        // Guard: only valid if the match is currently Playing AND the
        // schedule has elapsed. Idempotency (already Finished) is rejected
        // so we don't accidentally re-write the leaderboard or shift state.
        require!(ctx.accounts.game_config.status == 1, GameError::GameNotPlaying);
        let now = Clock::get()?.unix_timestamp;
        require!(now >= ctx.accounts.game_config.game_end, GameError::GameNotOver);

        // ── Read PlayerRegistry count from the first extra account. ──
        let registry_acc = &ctx.remaining_accounts[NUM_COMPONENTS];
        let registry_data = registry_acc.try_borrow_data()?;
        require!(
            registry_data.len() > PR_COUNT_OFFSET,
            GameError::InvalidAccount
        );
        let count = (registry_data[PR_COUNT_OFFSET] as usize).min(MAX_LEADERBOARD);
        drop(registry_data);

        // ── Build the final entries from every PlayerState extra. ──
        let mut entries: Vec<LeaderboardEntry> = Vec::with_capacity(count);
        for i in 0..count {
            let acc = &ctx.remaining_accounts[NUM_COMPONENTS + 1 + i];
            let data = acc.try_borrow_data()?;
            if data.len() < PS_MIN_LEN {
                continue;
            }

            let mut owner = [0u8; 32];
            owner.copy_from_slice(&data[PS_OWNER..PS_OWNER + 32]);
            let attempts_left = data[PS_ATTEMPTS_LEFT];
            let score = u64::from_le_bytes(
                data[PS_SCORE..PS_SCORE + 8].try_into().unwrap(),
            );
            drop(data);

            entries.push(LeaderboardEntry {
                pubkey: owner,
                score,
                attempts_left,
            });
        }

        // ── Insertion sort, score DESC, attempts_left DESC tie-breaker. ──
        let filled = entries.len();
        for i in 1..filled {
            let mut j = i;
            while j > 0 {
                let a = &entries[j - 1];
                let b = &entries[j];
                let swap = b.score > a.score
                    || (b.score == a.score && b.attempts_left > a.attempts_left);
                if !swap {
                    break;
                }
                entries.swap(j - 1, j);
                j -= 1;
            }
        }

        // ── Write final snapshot to the Leaderboard component. ──
        for i in 0..filled {
            ctx.accounts.leaderboard.entries[i] = entries[i];
        }
        ctx.accounts.leaderboard.count = filled as u8;

        // ── Flip the match status. The lobby program checks this before
        //    `distribute_prize` accepts a settlement. ──
        ctx.accounts.game_config.status = 2; // Finished

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub game_config: GameConfig,
        pub leaderboard: Leaderboard,
    }
}
