use bolt_lang::*;
use game_config::GameConfig;
use leaderboard::{Leaderboard, LeaderboardEntry, MAX_LEADERBOARD};
use shared::*;

declare_id!("GPnGj91d5ZueVC5YLjZXLrWHHgVxJiQFPcpq4srWUkBG");

// Bolt prepends one AccountInfo per #[system_input] component, so player
// extras start at index 2 (game_config + leaderboard) — but we also pass
// PlayerRegistry as an extra (read-only, raw bytes), so PlayerState PDAs
// start at NUM_COMPONENTS + 1.
const NUM_COMPONENTS: usize = 2;

// PlayerRegistry layout to read `count` from raw bytes:
//   8 (disc) + 4 (Vec1.len = u32) + 32 * MAX_PLAYERS (players) +
//             4 (Vec2.len = u32) + 32 * MAX_PLAYERS (player_states) +
//             1 (count: u8)
// At MAX_PLAYERS = 10 (current cap, see player-registry component):
// count sits at byte 656. Bumping MAX_PLAYERS in the registry component
// requires updating this offset here too.
const PR_COUNT_OFFSET: usize = 8 + 4 + 32 * 10 + 4 + 32 * 10;

// PlayerState byte layout (Stay Calm shape — NOT trade-fight's). Borsh
// packs fields tight, so the offsets are just sums of preceding sizes:
//   [0..8]      Anchor discriminator
//   [8..40]     authority: Pubkey (32) — back's signer (not what we want)
//   [40..72]    owner:     Pubkey (32) — the real wallet, leaderboard key
//   [72..73]    attempts_left: u8
//   [73..81]    score: u64                ← sort key
//   [81..89]    points_this_round: u64   (live attempt — not surfaced here)
//   [89..90]    leverage: u8             (live attempt)
//   [90..98]    low_price: u64           (live attempt)
//   [98..106]   high_price: u64          (live attempt)
//   [106..114]  attempt_end_ts: i64      (live attempt)
//   [114..122]  last_block: u64          (anti-cheat)
//   [122..]     bolt_metadata
const PS_OWNER: usize = 40;
const PS_ATTEMPTS_LEFT: usize = 72;
const PS_SCORE: usize = 73;
// Min length the system is willing to deserialize from — corresponds to
// having the score read fully. Older PlayerState versions written before
// adding `last_block` would still pass (we only read up to `score`).
const PS_MIN_LEN: usize = PS_SCORE + 8;

/// Live leaderboard refresh, called by the cranker each tick. Reads every
/// player's `score` + `attempts_left` from the PlayerState PDAs passed as
/// extra accounts, sorts them descending by `score`, and writes the
/// snapshot into the on-chain `Leaderboard` component.
///
/// PlayerRegistry is intentionally NOT in `#[system_input]`: Bolt echoes
/// every input component as return data, and at MAX_PLAYERS = 10 the
/// registry is ~650 bytes — combined with GameConfig + Leaderboard it
/// would blow Solana's 1024-byte `set_return_data` cap. We pass it as
/// the FIRST extra account and read `count` from raw bytes instead.
///
/// remaining_accounts (after the 2 component-program slots Bolt prepends):
///   [NUM_COMPONENTS]              PlayerRegistry PDA (raw bytes — count read)
///   [NUM_COMPONENTS + 1 + i]      PlayerState PDA for player i (0..count)
#[system]
pub mod refresh_leaderboard {

    pub fn execute(ctx: Context<Components>, _args_p: Vec<u8>) -> Result<Components> {
        require!(ctx.accounts.game_config.status == 1, GameError::GameNotPlaying);

        // Read the registered player count from the PlayerRegistry PDA
        // passed as the first extra account.
        let registry_acc = &ctx.remaining_accounts[NUM_COMPONENTS];
        let registry_data = registry_acc.try_borrow_data()?;
        require!(
            registry_data.len() > PR_COUNT_OFFSET,
            GameError::InvalidAccount
        );
        let count = (registry_data[PR_COUNT_OFFSET] as usize).min(MAX_LEADERBOARD);
        drop(registry_data);

        // Heap-allocated so we don't burn stack on a large MAX_LEADERBOARD.
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

        // Insertion sort, score DESC. Tie-breaker: more attempts_left ranks
        // above fewer (a tied player who still has tries left is "higher
        // potential" than one who's done).
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
