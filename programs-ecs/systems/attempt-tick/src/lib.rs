use bolt_lang::*;
use game_config::GameConfig;
use player_state::PlayerState;
use shared::{read_pyth_price, GameError, MIN_TICK_SLOT_GAP};

declare_id!("DhXFbMSeCfLPh3xy3ua1JkukEVFuRLfwuYXpgknJw7YB");

// Bolt prepends one AccountInfo per `#[system_input]` component, so the
// 2 component slots come first; the Pyth oracle is the FIRST extra
// account after that.
const NUM_COMPONENTS: usize = 2;

/// `attempt-tick` — keeper-signed (back).
#[system]
pub mod attempt_tick {

    pub fn execute(ctx: Context<Components>, _args_p: Vec<u8>) -> Result<Components> {
        // Match must be Playing.
        require!(
            ctx.accounts.game_config.status == 1,
            GameError::GameNotPlaying
        );

        // No active attempt — silent no-op so the keeper can blast every
        // PlayerState in one batch without per-player pre-filtering. The
        // TX still succeeds, just doesn't write anything.
        if ctx.accounts.player_state.attempt_end_ts == 0 {
            return Ok(ctx.accounts);
        }

        // Anti-replay: slot must strictly advance. Use checked_sub to
        // dodge u64 underflow in the rare case last_block somehow ends
        // up ahead of slot (shouldn't happen but never trust raw subs).
        let slot = Clock::get()?.slot;
        let gap = slot
            .checked_sub(ctx.accounts.player_state.last_block)
            .unwrap_or(0);
        require!(gap >= MIN_TICK_SLOT_GAP, GameError::StaleSlot);

        let now = Clock::get()?.unix_timestamp;
        let attempt_end_ts = ctx.accounts.player_state.attempt_end_ts;

        // ── Branch 1 — SUCCESS: time elapsed (with grace), bank score ──
        // The 20 s attempt has expired AND we're still inside the
        // `score`, clear all live-attempt fields. After this call the
        // player can LOCK again (if attempts_left > 0).
        if now >= attempt_end_ts {
            let ps = &mut ctx.accounts.player_state;
            ps.points_this_round = 0;
            ps.leverage = 0;
            ps.low_price = 0;
            ps.high_price = 0;
            ps.attempt_end_ts = 0;
            ps.last_block = slot;
            return Ok(ctx.accounts);
        }

        // Sanity guard: keeper fired well past the active window without
        // landing the success bank. Treat as success anyway (the player
        // did survive 20 s) — but flag it so the back can investigate
        // why the prior tick missed.

        // ── Branch 2 — read Pyth, only credit points if inside the band ──
        // No "bust" any more — the attempt keeps running until
        // `attempt_end_ts` regardless of whether the price exits the
        // corridor. Out-of-band ticks just skip the `+= leverage` so the
        // player loses scoring time but doesn't lose the attempt.
        let price = read_pyth_price(&ctx.remaining_accounts[NUM_COMPONENTS])?;
        let in_band = price >= ctx.accounts.player_state.low_price
            && price <= ctx.accounts.player_state.high_price;

        let ps = &mut ctx.accounts.player_state;
        if in_band {
            // Credit `leverage` to both the live `points_this_round`
            // accumulator AND the cumulative `score` so the leaderboard
            // reflects the live ranking — no bank-roll at attempt end
            // (the SUCCESS branch above just clears the live-attempt
            // fields without touching `score`).
            let credit = ps.leverage as u64;
            ps.points_this_round = ps.points_this_round.saturating_add(credit);
            ps.score = ps.score.saturating_add(credit);
        }
        ps.last_block = slot;

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub game_config: GameConfig,
        pub player_state: PlayerState,
    }
}
