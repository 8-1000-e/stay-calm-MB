# Stay Calm — Architecture

## Game design

Inverse-volatility game on SOL/USD. Players join a lobby, get 3 LOCK attempts of 20 s each per match, pick a leverage tier (1–5×) for each attempt. Higher leverage = tighter price band but bigger per-second points. The player who scores the most points across the match wins the pot.

| Phase | Trigger | Duration |
|---|---|---|
| Lobby (Waiting) | `init-game` | `LOBBY_DURATION_SEC = 60 s` |
| Match (Playing) | `start-game` (≥ `MIN_PLAYERS = 2`) | `GAME_DURATION_SEC = 5 min` |
| Settled (Finished) | `end-game` (after match clock) | terminal |

Within a match, each player has 3 attempts of `ATTEMPT_DURATION_SEC = 20 s` each. The keeper can fire the `attempt-tick` up to `SUCCESS_GRACE_SEC = 1 s` past `attempt_end_ts` for the auto-bank window.

## On-chain layout

### Components (Bolt ECS)

| Component | Per-entity | Key fields |
|---|---|---|
| `GameConfig` | one per match | `status`, `active_players`, `min_start_time`, `game_end`, `lobby_id` |
| `PlayerState` | one per player | `attempts_left`, `score`, `points_this_round`, `leverage`, `low_price`, `high_price`, `attempt_end_ts`, `last_block` |
| `PlayerRegistry` | one per match | `players: Vec<[u8; 32]>` (32-byte pubkeys), `player_states: Vec<[u8; 32]>` (PDAs), `count: u8` — `MAX_PLAYERS = 10` |
| `Leaderboard` | one per match | `entries: Vec<{pubkey, score, attempts_left}>`, `count: u8` — `MAX_LEADERBOARD = 10` |

### Systems

| System | Signer | Trigger |
|---|---|---|
| `init-game` | back | once per match — sets `GameConfig` to Waiting + stamps `lobby_id` |
| `spawn-player` | back | per-player — initializes `PlayerState`, pushes to `PlayerRegistry` |
| `start-game` | back | flips status to Playing, sets `game_end = now + GAME_DURATION_SEC` |
| `lock-position` | player (session key) | reads Pyth, computes `[low, high]` from leverage, decrements `attempts_left` |
| `attempt-tick` | back | per-player tick (~3 Hz on ER) — checks band, accumulates `points_this_round`, banks at `attempt_end_ts` |
| `refresh-leaderboard` | back | per-tick — reads PlayerState extras, sorts by `score`, writes Leaderboard |
| `end-game` | back | once at `now >= game_end` — final leaderboard + flip status to Finished |

### Lobby program (`stay-calm-lobby`, Anchor)

Mirror of trade-fight-lobby's pattern. TOFU authority via `create_lobby` storing the signer as `lobby.authority`. Subsequent admin ix (`start_match`, `distribute_prize`, `close_lobby`, `refund_lobby`) gate on this.

## Data flow

```
[back] init-game (lobby_id) → GameConfig.lobby_id = lobby.id
[player] join_lobby (pays entry fee) ────┐
                                          ↓
[back] spawn-player × N → PlayerState entities
[back] start-game (now ≥ min_start_time) → status = Playing
                                          ↓
[player] lock-position (leverage 1-5) → low/high/attempt_end_ts/leverage frozen
[back] attempt-tick (loop ~3 Hz) ─┬→ in band: points_this_round += leverage
                                   ├→ out-of-band: skip credit, attempt continues
                                   └→ now ≥ attempt_end_ts: bank to score, reset live
[back] refresh-leaderboard (loop ~1 Hz) → sorted snapshot in Leaderboard component
                                          ↓
[back] end-game (now ≥ game_end) → status = Finished, final leaderboard
[back] distribute_prize (lobby program) → check status==Finished + lobby_id match → payout
```

## Anti-cheat invariants

- **`MIN_TICK_SLOT_GAP = 10`** — `attempt-tick` requires `slot - last_block ≥ 10`. At lock, `last_block = slot - GAP` so the first tick passes immediately. Prevents stacking N × leverage in one slot.
- **`attempts_left` decremented at LOCK** (not resolution) — engagement is firm, the player can't lock-cancel-relock to game the system.
- **`leverage` and `[low, high]` frozen at LOCK** — changing the dial mid-attempt has no effect.
- **`attempt-tick` is not authority-gated** but BOLT session keys delivered to players MUST be scoped to `LOCK_POSITION_SYSTEM` program ID only — otherwise a player could call `attempt-tick` themselves and selectively tick only when in-band, cheating the time-in-band sampling.
- **`lobby_id` stamped in `GameConfig` by `init-game`** — `distribute_prize` (lobby program) verifies the Leaderboard's parent GameConfig's `lobby_id` matches the lobby PDA being settled. Prevents leaderboard substitution attacks.

## Constants

All in `programs-ecs/libs/shared/src/lib.rs`:

| Const | Value | Purpose |
|---|---|---|
| `LOBBY_DURATION_SEC` | 60 | Lobby open window |
| `GAME_DURATION_SEC` | 300 | Match length |
| `MAX_ATTEMPTS` | 3 | LOCKs per player per match |
| `ATTEMPT_DURATION_SEC` | 20 | Single attempt length |
| `SUCCESS_GRACE_SEC` | 1 | Keeper finalization window past `attempt_end_ts` |
| `MIN_TICK_SLOT_GAP` | 10 | Anti-replay gap (~300 ms on ER) |
| `MIN_PLAYERS` | 2 | Min to start match |
| `BAND_HALF_PPM` | `[0, 250, 200, 150, 100, 50]` | Half-band in ppm, indexed by leverage 1..=5 |
| `PPM_DENOM` | 1_000_000 | ppm denominator |

## PlayerState byte layout (relevant for raw reads)

Borsh packs tight, no padding for primitives.

```
[0..8]      Anchor discriminator
[8..40]     authority: Pubkey
[40..72]    owner: Pubkey
[72..73]    attempts_left: u8
[73..81]    score: u64                 ← sort key
[81..89]    points_this_round: u64
[89..90]    leverage: u8
[90..98]    low_price: u64
[98..106]   high_price: u64
[106..114]  attempt_end_ts: i64
[114..122]  last_block: u64
[122..]     bolt_metadata
```

Used by `refresh-leaderboard` and `end-game` (which read PlayerState as raw bytes via `remaining_accounts` to dodge the 1024-byte `set_return_data` cap).

## PlayerRegistry byte layout

```
[0..8]      Anchor discriminator
[8..12]     u32 LE — players Vec length prefix (= MAX_PLAYERS = 10)
[12..332]   players: 10 × [u8; 32]
[332..336]  u32 LE — player_states Vec length prefix
[336..656]  player_states: 10 × [u8; 32]
[656]       count: u8
[657..]     bolt_metadata
```

`PR_COUNT_OFFSET = 656` in `refresh-leaderboard` and `end-game`.
