## Context Recovery

IMPORTANT: At session start, read all `.md` files in `/docs/` to restore full project context from the previous session.

## Current State

- **Branch**: main
- **Status**: All 7 systems compile (`bolt build` green). On-chain layer feature-complete for v0 game loop. No deploy yet, no tests yet, no back wiring yet.
- **Last updated**: 2026-05-10

## Task Progress

- [x] Bootstrap Bolt workspace from trade-fight scaffolding (Cargo.toml, Anchor.toml, patches, libs)
- [x] Components: `game-config`, `player-state`, `player-registry`, `leaderboard`
- [x] Systems: `init-game`, `spawn-player`, `start-game`, `lock-position`, `attempt-tick`, `refresh-leaderboard`, `end-game`
- [x] Lobby program copied from trade-fight, renamed `stay-calm-lobby`
- [x] Shared lib (`read_pyth_price`, `parse_json_*`, `BAND_HALF_PPM`, `GameError`)
- [x] Audit + critical fixes: silent no-op on attempt-tick, first-tick anti-replay init, min_start_time check, dead-code cleanup, removed `start_block` field
- [x] Build green (`bolt build` produces all `.so` artifacts in `target/deploy/`)
- [ ] **Regenerate `declare_id!` from `target/deploy/*-keypair.json` + update `Anchor.toml [programs.localnet]`** ← CURRENT NEXT
- [ ] Update `LEADERBOARD_COMPONENT_ID` in `programs/stay-calm-lobby/src/constants.rs` (currently `1111…1` placeholder)
- [ ] Tests (`tests/stay-calm.ts` only has bolt-init scaffolding)
- [ ] Initial `bolt deploy` to MagicBlock devnet ER
- [ ] Back wiring (Nest service, cranker, session-key issuance)
- [ ] Front wiring (replace mocks in `front-dev/src/components/games/lock-the-calm/lock-the-calm.tsx`)

## Key Decisions

- **`attempt_end_ts > 0` is implicit alive flag** — no separate `alive: bool` field. Resolution paths set `attempt_end_ts = 0` to mark "no active attempt".
- **Band boundaries pre-computed at lock** — `low_price` / `high_price` stored on PlayerState (not `lock_price` + `band_pct`), so the keeper hot path is a single comparison instead of multiply/divide per tick.
- **No-bust model** — when the price exits the band mid-attempt, we don't terminate the attempt, we just don't credit `points_this_round`. Attempt always runs the full 20 s.
- **Authority model: rely on BOLT entity-auth + back-side session-key scoping** — no hardcoded `BACK_AUTHORITY` consts in programs (mirrors red-light/trade-fight). Session keys delivered by the back must be scoped to `LOCK_POSITION_SYSTEM` only.
- **`MIN_TICK_SLOT_GAP = 10` slots** — on MagicBlock ER (~30 ms/slot), this is ~300 ms throttle = 1 tick per 0.3 s = 66 ticks per 20 s attempt. At lock, `last_block = slot - GAP` so the first tick passes immediately.
- **Lobby program is TOFU** — `create_lobby` stores the signer as `lobby.authority`; all subsequent admin ix check against this. Works across local/dev/prod without hardcoded addresses.
