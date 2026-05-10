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

/// Starting fake-USD balance handed to each player on spawn. 2500 USD,
/// stored with 8 decimals so the unit matches the Pyth Lazer price feed
/// (USD-with-8-decimals). PnL math stays in one unit system, no conversion.
/// 2500 × 10^8 = 2.5e11.
pub const STARTING_BALANCE: u64 = 250_000_000_000;

/// Lobby duration before the game auto-starts (seconds).
pub const LOBBY_DURATION_SEC: i64 = 60;

/// Trading round duration once the game has started (seconds).
pub const GAME_DURATION_SEC: i64 = 5 * 60;

/// Minimum interval between PnL updates (seconds). Front-end / cranker should
/// not call `close-position` more often than this.

/// Max players per lobby. Mirrors red-light.
pub const MAX_PLAYERS: usize = 20;

/// Position direction encoding used in PlayerState.position.
pub const POS_FLAT: u8 = 0;
pub const POS_LONG: u8 = 1;
pub const POS_SHORT: u8 = 2;

/// Allowed leverage tiers (front-end should restrict to these values).
/// Aggressive scale tuned for 5-min SOL matches — floor at 400× (~0.25%
/// liq distance, even the safest tier still fires often enough during
/// a round), ceiling at 1500× for the glass-cannon top.
/// Liq distance per tier (= 1/leverage of entry):
///   400×  → 0.25%
///   500×  → 0.20%
///   750×  → 0.13%
///   1000× → 0.10%
///   1250× → 0.08%
///   1500× → 0.067%
pub const LEVERAGE_TIERS: [u16; 6] = [400, 500, 750, 1000, 1250, 1500];

// ─── Errors ───

#[error_code]
pub enum GameError {
    #[msg("Game is not in Waiting state")]
    GameNotWaiting,
    #[msg("Game is not in Playing state")]
    GameNotPlaying,
    #[msg("Game is already finished")]
    GameAlreadyFinished,
    #[msg("Too many players")]
    TooManyPlayers,
    #[msg("Player is dead (balance hit zero)")]
    PlayerDead,
    #[msg("Lobby not over yet")]
    LobbyNotOver,
    #[msg("Game timer not expired yet")]
    GameNotOver,
    #[msg("Player already has an open position")]
    PositionAlreadyOpen,
    #[msg("Player has no open position to close")]
    NoOpenPosition,
    #[msg("Invalid leverage tier")]
    InvalidLeverage,
    #[msg("Invalid position direction (must be 1=long or 2=short)")]
    InvalidDirection,
    #[msg("Insufficient balance for requested margin")]
    InsufficientBalance,
    #[msg("Invalid Pyth account / price feed")]
    InvalidAccount,
    #[msg("Unauthorized — signer is not the player owner")]
    Unauthorized,
}
