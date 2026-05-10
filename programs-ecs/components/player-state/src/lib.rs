use bolt_lang::*;

declare_id!("Ba9QeK5PB6bF8fkfA64pyd4p3fkd6dco8tmEf2yToBtb");

/// Per-player state for Stay Calm. The match runs in `attempts_left × 20 s`
/// rounds — the player picks a leverage (1-5x), clicks LOCK, and the keeper
/// then verifies every tick that the live Pyth price is still inside
/// `[low_price, high_price]` (precomputed at lock from the chosen leverage).
/// If the price exits before `attempt_end_ts` the player busts (no points
/// banked for that attempt). If they survive the full 20 s, the running
/// `points_this_round` is added to the cumulative `score`. Match ends for
/// this player when `attempts_left == 0`.
///
/// **"Alive" is implicit** — `attempt_end_ts > 0` ⇔ currently locked (= the
/// keeper hot path should run the band check). Resolution paths set
/// `attempt_end_ts = 0` along with the rest of the live-attempt fields.
///
/// All Pyth-related values use the SOL/USD feed's raw 8-decimal unit so
/// the band check is just `low_price <= live <= high_price` — no per-tick
/// arithmetic, no division, no leverage lookup on the keeper hot path.
#[component(delegate)]
pub struct PlayerState {
    /// BOLT entity authority (the wallet that owns this entity).
    pub authority: Pubkey,
    /// Real wallet pubkey — kept for leaderboard / payout matching.
    pub owner: Pubkey,

    // ─── Match state ───
    /// Number of LOCK attempts still available this match. Starts at 3,
    /// decrements when an attempt resolves (success OR bust). When it hits
    /// 0 the match is done for this player.
    pub attempts_left: u8,
    /// Cumulative score banked across resolved attempts so far in the
    /// match (sum of past `points_this_round` snapshots that survived).
    /// Sorted descending by `score` on the leaderboard.
    pub start_block: u64,
    pub score: u64,
    /// Live accumulator for the CURRENT attempt — the keeper increments it
    /// each tick (`+= leverage` per second alive in the band). On success
    /// at `attempt_end_ts` it's banked into `score` and reset to 0; on
    /// bust it's discarded (just reset to 0, never banked).
    pub points_this_round: u64,

    // ─── Live attempt state — only meaningful when `attempt_end_ts > 0` ───
    /// Leverage multiplier in effect for the CURRENT attempt (1-5). Frozen
    /// at lock so changing the dial mid-attempt doesn't retroactively shift
    /// the band or the per-second points rate. Drives `points_this_round`
    /// growth on the keeper hot path.
    pub leverage: u8,
    /// Lower band boundary (Pyth raw, 8 decimals) — precomputed at lock
    /// as `locked_price * (1 - band_pct)`. Stored directly so the keeper
    /// band check is a single comparison, not a multiply/divide.
    pub low_price: u64,
    /// Upper band boundary (Pyth raw, 8 decimals) — precomputed at lock
    /// as `locked_price * (1 + band_pct)`.
    pub high_price: u64,
    /// Unix second at which the current attempt expires (lock + 20 s).
    /// Keeper auto-finalizes (banks score, decrements `attempts_left`,
    /// resets the live-attempt fields) when `Clock::unix_timestamp >=
    /// attempt_end_ts`. **Also doubles as the alive flag** — `0` means
    /// "no active attempt, skip this player on the keeper tick".
    pub attempt_end_ts: i64,


    pub last_block: u64, // for anti cheat
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            authority: Pubkey::default(),
            owner: Pubkey::default(),
            attempts_left: 0,
            start_block: 0,
            score: 0,
            points_this_round: 0,
            leverage: 0,
            low_price: 0,
            high_price: 0,
            attempt_end_ts: 0,
            last_block: 0,
            bolt_metadata: BoltMetadata::default(),
        }
    }
}
