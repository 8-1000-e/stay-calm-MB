use bolt_lang::*;
use game_config::GameConfig;
use player_state::PlayerState;
use shared::{
    parse_json_u64, read_pyth_price, GameError, ATTEMPT_DURATION_SEC, BAND_HALF_PPM,
    MIN_TICK_SLOT_GAP, PPM_DENOM,
};

declare_id!("11111111111111111111111111111111");

// Bolt prepends one AccountInfo per `#[system_input]` component, so the
// 2 component slots come first; the Pyth oracle is the FIRST extra
// account after that.
const NUM_COMPONENTS: usize = 2;

/// `lock-position` — player-signed (session key).
///
/// Captures the LOCK moment for one player:
///   - reads the live Pyth price from the extra account at index NUM_COMPONENTS
///   - computes [low_price, high_price] from the chosen leverage tier
///   - sets `attempt_end_ts = now + ATTEMPT_DURATION_SEC`
///   - freezes `leverage`
///   - decrements `attempts_left` (engagement is firm — bust or success
///     consumes the attempt)
///   - resets `points_this_round = 0`
///
/// args: JSON `{"leverage": <u8>}` — tier index 1..=5
///
/// extra accounts (after the 2 component-program slots Bolt prepends):
///   [NUM_COMPONENTS]    Pyth Lazer SOL/USD oracle PDA (read-only)
#[system]
pub mod lock_position {

    pub fn execute(ctx: Context<Components>, args_p: Vec<u8>) -> Result<Components> {
        // ── Guards ──────────────────────────────────────────────────
        // Match must be Playing — no locking before start or after end.
        require!(
            ctx.accounts.game_config.status == 1,
            GameError::GameNotPlaying
        );
        // Player must have attempts left.
        require!(
            ctx.accounts.player_state.attempts_left > 0,
            GameError::NoAttemptsLeft
        );
        // Reject double-lock: only one live attempt at a time.
        require!(
            ctx.accounts.player_state.attempt_end_ts == 0,
            GameError::AlreadyLocked
        );

        // ── Parse leverage from args (tier index 1..=5) ─────────────
        let leverage = parse_json_u64(&args_p, b"leverage") as u8;
        require!(
            (1..=5).contains(&leverage),
            GameError::InvalidLeverage
        );

        // ── Read live Pyth price ────────────────────────────────────
        let lock_price = read_pyth_price(&ctx.remaining_accounts[NUM_COMPONENTS])?;
        require!(lock_price > 0, GameError::InvalidAccount);

        // ── Compute band edges in u64 ppm arithmetic ────────────────
        // For SOL ≈ 9.3e9 raw, lock_price × (1_000_000 ± 250) ≈ 9.3e15,
        // well under u64::MAX (~1.8e19). checked_mul anyway in case the
        // feed ever spits a degenerate value.
        let half_ppm = BAND_HALF_PPM[leverage as usize];
        let low_price = lock_price
            .checked_mul(PPM_DENOM - half_ppm)
            .ok_or(GameError::InvalidAccount)?
            / PPM_DENOM;
        let high_price = lock_price
            .checked_mul(PPM_DENOM + half_ppm)
            .ok_or(GameError::InvalidAccount)?
            / PPM_DENOM;

        // ── Stamp the captured attempt onto PlayerState ─────────────
        let now = Clock::get()?.unix_timestamp;
        let slot = Clock::get()?.slot;
        let ps = &mut ctx.accounts.player_state;
        ps.leverage = leverage;
        ps.low_price = low_price;
        ps.high_price = high_price;
        ps.attempt_end_ts = now + ATTEMPT_DURATION_SEC;
        ps.attempts_left -= 1;
        ps.points_this_round = 0;
        // Initialize `last_block` so the FIRST `attempt-tick` after this
        // lock can pass the `slot - last_block >= MIN_TICK_SLOT_GAP`
        // check immediately. Without this, the keeper would have to wait
        // MIN_TICK_SLOT_GAP slots after lock before the first tick lands,
        // costing the player ~MIN_TICK_SLOT_GAP × slot_ms of scoring time.
        // saturating_sub handles the (impossible) case where slot is < gap.
        ps.last_block = slot.saturating_sub(MIN_TICK_SLOT_GAP);

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub game_config: GameConfig,
        pub player_state: PlayerState,
    }
}
