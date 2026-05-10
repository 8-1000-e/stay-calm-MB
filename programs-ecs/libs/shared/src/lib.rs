use bolt_lang::*;

// ─── JSON parsers (stack-only, no heap) ───

/// Parse a u64 value from JSON bytes by key name.
pub fn parse_json_u64(json: &[u8], key: &[u8]) -> u64 {
    let mut i = 0;
    while i + key.len() + 3 < json.len() {
        if json[i] == b'"'
            && i + 1 + key.len() + 1 < json.len()
            && &json[i + 1..i + 1 + key.len()] == key
            && json[i + 1 + key.len()] == b'"'
            && json[i + 2 + key.len()] == b':'
        {
            let mut j = i + 3 + key.len();
            while j < json.len() && json[j] == b' ' { j += 1; }
            let mut val: u64 = 0;
            while j < json.len() && json[j].is_ascii_digit() {
                val = val * 10 + (json[j] - b'0') as u64;
                j += 1;
            }
            return val;
        }
        i += 1;
    }
    0
}

/// Parse an i64 value from JSON bytes (supports negative).
pub fn parse_json_i64(json: &[u8], key: &[u8]) -> i64 {
    let mut i = 0;
    while i + key.len() + 3 < json.len() {
        if json[i] == b'"'
            && i + 1 + key.len() + 1 < json.len()
            && &json[i + 1..i + 1 + key.len()] == key
            && json[i + 1 + key.len()] == b'"'
            && json[i + 2 + key.len()] == b':'
        {
            let mut j = i + 3 + key.len();
            while j < json.len() && json[j] == b' ' { j += 1; }
            let neg = j < json.len() && json[j] == b'-';
            if neg { j += 1; }
            let mut val: i64 = 0;
            while j < json.len() && json[j].is_ascii_digit() {
                val = val * 10 + (json[j] - b'0') as i64;
                j += 1;
            }
            return if neg { -val } else { val };
        }
        i += 1;
    }
    0
}

/// Parse a string value from JSON bytes. Returns the bytes between quotes.
pub fn parse_json_str<'a>(json: &'a [u8], key: &[u8]) -> &'a [u8] {
    let mut i = 0;
    while i + key.len() + 4 < json.len() {
        if json[i] == b'"'
            && i + 1 + key.len() + 1 < json.len()
            && &json[i + 1..i + 1 + key.len()] == key
            && json[i + 1 + key.len()] == b'"'
            && json[i + 2 + key.len()] == b':'
        {
            let mut j = i + 3 + key.len();
            while j < json.len() && json[j] == b' ' { j += 1; }
            if j < json.len() && json[j] == b'"' {
                j += 1;
                let start = j;
                while j < json.len() && json[j] != b'"' { j += 1; }
                return &json[start..j];
            }
        }
        i += 1;
    }
    &[]
}

// ─── Pyth Lazer oracle ───

const PRICE_OFFSET: usize = 73;

/// Read SOL/USD price from a Pyth Lazer account.
/// The account must be passed as remaining_accounts.
/// Returns raw u64 (8 decimals — divide by 10^8 for dollars).
pub fn read_pyth_price(account: &AccountInfo) -> Result<u64> {
    let data = account.try_borrow_data()?;
    require!(data.len() >= PRICE_OFFSET + 8, GameError::InvalidAccount);
    Ok(u64::from_le_bytes(
        data[PRICE_OFFSET..PRICE_OFFSET + 8].try_into().unwrap()
    ))
}

// ─── Game-wide constants ───

/// Lobby duration before the game auto-starts (seconds).
pub const LOBBY_DURATION_SEC: i64 = 60;

/// Match duration once the game has started (seconds).
pub const GAME_DURATION_SEC: i64 = 5 * 60;

/// Number of LOCK attempts each player gets per match. Set on spawn,
/// decremented on every resolution (bust or success).
pub const MAX_ATTEMPTS: u8 = 3;

/// Duration of a single locked attempt (seconds). The keeper banks
/// `points_this_round` into `score` once `Clock::unix_timestamp >=
/// attempt_end_ts + SUCCESS_GRACE_SEC` (the grace gives the keeper a
/// small window past the 20 s mark to actually fire the success TX).
pub const ATTEMPT_DURATION_SEC: i64 = 20;

/// Extra seconds tacked onto the attempt window for the keeper's
/// finalization tick. The total active period is `ATTEMPT_DURATION_SEC +
/// SUCCESS_GRACE_SEC` ≈ 20.5 s — within those final ~0.5 s the band
/// check is no longer enforced and the next tick auto-banks.
pub const SUCCESS_GRACE_SEC: i64 = 1;

/// Minimum slot gap between two consecutive `attempt-tick` calls for the
/// same player. Solana slots are ~400 ms each; gap = 1 means strict
/// advancement (no same-slot double-tick). Bump higher to throttle the
/// keeper hot-path further.
pub const MIN_TICK_SLOT_GAP: u64 = 10;

/// Minimum number of players required for `start-game` to fire. Below
/// this, the back keeps the lobby open until more players join (or
/// rotates the lobby ID after the timeout).
pub const MIN_PLAYERS: u8 = 2;

/// Half-band width per leverage tier, in parts-per-million (1 ppm = 1e-6).
/// Mirrors the front's `BAND_PCT_BASE * bandFrac`:
///   1x → 0.025 % = 250 ppm
///   2x → 0.020 % = 200 ppm
///   3x → 0.015 % = 150 ppm
///   4x → 0.010 % = 100 ppm
///   5x → 0.005 %  = 50 ppm
/// Index 0 unused so the leverage value (1..=5) maps directly.
pub const BAND_HALF_PPM: [u64; 6] = [0, 250, 200, 150, 100, 50];

/// Denominator for ppm arithmetic. `lock_price * (PPM_DENOM ± half_ppm)
/// / PPM_DENOM` gives the band edges as Pyth-raw u64.
pub const PPM_DENOM: u64 = 1_000_000;

// ─── Errors ───

#[error_code]
pub enum GameError {
    #[msg("Game is not in Waiting state")]
    GameNotWaiting,
    #[msg("Game is not in Playing state")]
    GameNotPlaying,
    #[msg("Too many players")]
    TooManyPlayers,
    #[msg("Not enough players to start")]
    NotEnoughPlayers,
    #[msg("Player has no attempts left")]
    NoAttemptsLeft,
    #[msg("Player is already in a locked attempt")]
    AlreadyLocked,
    #[msg("Stale slot — replayed tick")]
    StaleSlot,
    #[msg("Lobby not over yet")]
    LobbyNotOver,
    #[msg("Game timer not expired yet")]
    GameNotOver,
    #[msg("Invalid leverage tier")]
    InvalidLeverage,
    #[msg("Invalid Pyth account / price feed")]
    InvalidAccount,
}
