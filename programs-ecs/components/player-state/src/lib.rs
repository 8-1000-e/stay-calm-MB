use bolt_lang::*;

declare_id!("Ba9QeK5PB6bF8fkfA64pyd4p3fkd6dco8tmEf2yToBtb");

/// Per-player state for Stay Calm. The game runs in `attempts_left × 20 s`
/// rounds — the player picks a leverage (1-5x), clicks LOCK, and the keeper
/// then verifies every tick that the live Pyth price is still inside
/// `[locked_price * (1 - band_pct), locked_price * (1 + band_pct)]`. If the
/// price exits the band before `attempt_end_ts` the player busts (no points
/// for that attempt). If they survive the full 20 s, the score for the
/// attempt = `20 * leverage` is banked into `score` and `attempts_left`
/// decrements. Round ends when `attempts_left == 0`.
///
/// All Pyth-related values use the SOL/USD feed's raw 8-decimal unit so no
/// conversions are needed in band-check arithmetic.
#[component(delegate)]
pub struct PlayerState {
    /// BOLT entity authority (the wallet that owns this entity).
    pub authority: Pubkey,
    /// Real wallet pubkey — kept for leaderboard / payout matching.
    pub owner: Pubkey,

    // ─── Round state ───
    /// Number of LOCK attempts still available this round. Starts at 3,
    /// decrements when an attempt resolves (success OR bust). When it hits
    /// 0 the round is done for this player.
    pub attempts_left: u8,
    /// Cumulative score banked across resolved attempts this round
    /// (`pts = duration_seconds × leverage` for each successful attempt).
    /// Sorted descending by `score` on the leaderboard.
    pub score: u64,

    // ─── Live attempt state — only meaningful when `alive == true` ───
    /// True between `lock-position` and the resolution (success or bust).
    /// While true, the keeper checks the band every tick; while false, the
    /// keeper skips this player and the player can pick a new leverage and
    /// LOCK again (assuming `attempts_left > 0`).
    pub alive: bool,
    /// Leverage multiplier in effect for the CURRENT attempt (1-5). Frozen
    /// at lock so changing the dial mid-attempt doesn't retroactively shift
    /// the band or the per-second points rate.
    pub leverage: u8,
    /// Pyth raw price (8 decimals) captured at LOCK. Center of the validity
    /// band — the player busts if the live price exits
    /// `[locked_price × (1 − band), locked_price × (1 + band)]`.
    pub locked_price: u64,
    /// Band half-width in basis points (1 bp = 0.01%). e.g. `5` ⇒ ±0.05 %.
    /// Frozen at lock from the chosen leverage's `band_frac` so the band
    /// stays fixed even if the player bumps the leverage selector after.
    pub band_pct_bps: u32,
    /// Unix second at which the current attempt expires (lock + 20 s).
    /// Keeper auto-finalizes (banks score, sets `alive = false`) when
    /// `Clock::unix_timestamp >= attempt_end_ts`.
    pub attempt_end_ts: i64,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            authority: Pubkey::default(),
            owner: Pubkey::default(),
            attempts_left: 0,
            score: 0,
            alive: false,
            leverage: 0,
            locked_price: 0,
            band_pct_bps: 0,
            attempt_end_ts: 0,
            bolt_metadata: BoltMetadata::default(),
        }
    }
}
