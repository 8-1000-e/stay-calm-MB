# Decisions log (append-only)

## 2026-05-10 — Bootstrap from trade-fight

**Decision**: Copy the entire trade-fight Bolt workspace structure (Cargo.toml, Anchor.toml, patches, libs/shared, components, systems) as scaffolding for stay-calm.

**Rationale**: trade-fight already solved the BOLT setup (bolt-lang from git main, session-keys local patch, Cargo.lock pinned), and the component shapes (GameConfig, PlayerState, PlayerRegistry, Leaderboard) are 80% the same shape we need.

**Cost**: Some trade-fight-specific dead code copied over (USD positions, leverage tiers 400-1500, position direction enums) — cleaned up afterwards in the dead-code pass.

---

## 2026-05-10 — `attempt_end_ts > 0` as implicit alive flag

**Decision**: Don't add a separate `alive: bool` field on PlayerState. Use `attempt_end_ts > 0` as the implicit "currently in a locked attempt" check.

**Rationale**: Resolution paths (success / bust) all reset `attempt_end_ts = 0` along with the rest of the live-attempt fields. A boolean would just be redundant state. Saves 1 byte per PlayerState and removes a possible inconsistency vector (alive=true but attempt_end_ts=0).

**Alternative considered**: Keep `alive: bool` for explicit semantics. Rejected — adds maintenance burden without behavioral benefit.

---

## 2026-05-10 — Pre-compute `[low_price, high_price]` at lock instead of `lock_price + band_pct`

**Decision**: Store `low_price: u64` and `high_price: u64` directly on PlayerState rather than `locked_price` + `band_pct_bps`.

**Rationale**: The keeper hot path (`attempt-tick`) needs to check `low ≤ live ≤ high` every tick. If we stored `lock_price` + `band_pct`, we'd recompute the bounds on every tick (multiply + divide). Pre-computing saves CU per tick at the cost of 1 extra u64 of state.

**Math**: `low_price = lock_price * (PPM_DENOM - half_ppm) / PPM_DENOM`, computed once in `lock-position`. Half-band stored in `BAND_HALF_PPM[leverage]` (indexed 1..=5).

---

## 2026-05-10 — No-bust model

**Decision**: When the live price exits `[low_price, high_price]` mid-attempt, the attempt does NOT terminate. The keeper just doesn't credit `points_this_round += leverage` for that tick. Attempt always runs the full 20 s.

**Rationale**: Simpler keeper logic (no bust path), and the player still earns proportional points for the time they kept SOL inside the band. Discourages "all-or-nothing" gameplay.

**Alternative considered**: Bust = immediate attempt termination + lose all `points_this_round`. Rejected as too punitive and adds a separate code path.

---

## 2026-05-10 — `attempts_left` decremented at LOCK, not at resolution

**Decision**: `lock-position` decrements `attempts_left -= 1`. Resolution paths (success / late-success) don't touch it.

**Rationale**: Engagement is firm — the moment the player commits to a LOCK, the attempt is consumed regardless of how it resolves. Without this, a player could lock → see the price moving badly → "cancel" by ignoring → relock at a better moment, gaming the system. Decrementing at lock removes that loophole.

---

## 2026-05-10 — No hardcoded `BACK_AUTHORITY` const in programs

**Decision**: Don't enforce `authority == BACK_AUTHORITY` in keeper systems. Rely on:
1. BOLT's natural entity-authority model (only the back can sign for GameConfig/Leaderboard entities)
2. Session keys delivered by the back are scoped to `LOCK_POSITION_SYSTEM` only — players can't sign `attempt-tick` even if they wanted to

**Rationale**: Hardcoded pubkeys force redeployment on keypair rotation and don't work across local/dev/prod environments without a build-time variant. Mirrors what red-light and trade-fight do.

**Risk acknowledged**: The session-key scoping is a back-side concern. If the back issues a session key with a wider scope, a player could call `attempt-tick` directly and exploit selective ticking. This is a runtime invariant the back must respect, not an on-chain enforcement.

---

## 2026-05-10 — `MIN_TICK_SLOT_GAP = 10`

**Decision**: Anti-replay gap of 10 slots between consecutive `attempt-tick` calls for the same player.

**Rationale**: MagicBlock ER ≈ 30 ms/slot, so 10 slots = ~300 ms throttle. At ATTEMPT_DURATION_SEC = 20 s, this caps the keeper at ~66 ticks per attempt = max ~330 points per attempt at 5× leverage. Higher gap throttles the keeper more (cheaper but coarser scoring); lower gap allows finer scoring at higher CU cost.

**First-tick fix**: At `lock-position`, set `last_block = slot.saturating_sub(MIN_TICK_SLOT_GAP)` so the first tick after lock passes the gap check immediately. Without this, the player would lose 300 ms of scoring time at the start of every attempt.

---

## 2026-05-10 — Removed `start_block` field from PlayerState

**Decision**: Drop the `start_block: u64` field that was set at spawn. It was never read by any system and offered no real anti-cheat value.

**Rationale**: The hypothetical use case ("anti-replay against pre-spawn injections") is already covered by `last_block` which gets set at lock. Saved 8 bytes per PlayerState; required updating `PS_SCORE` offset from 81 → 73 in `refresh-leaderboard` and `end-game`.

---

## 2026-05-10 — Lobby program TOFU authority (no hardcoded admin)

**Decision**: `stay-calm-lobby`'s `create_lobby` stores the first signer's pubkey as `lobby.authority`. All subsequent admin ix (`start_match`, `distribute_prize`, `close_lobby`, `refund_lobby`) check `signer == lobby.authority`.

**Rationale**: Same pattern as trade-fight and red-light. Works across local/dev/prod environments without hardcoded addresses. The back's keypair (whoever it is at deploy time) becomes the per-match admin.

---

## 2026-05-10 — Cargo.lock copied from trade-fight

**Decision**: Use trade-fight's `Cargo.lock` as the starting point instead of letting Cargo regenerate it.

**Rationale**: Fresh resolution of the workspace pulled `indexmap v2.14.0` which requires `edition2024` (Cargo nightly only). trade-fight's lock pins `indexmap v2.13.0` and the rest of the deps to known-working versions. Copying their lockfile avoids the transitive-version-resolution rabbit hole. (See also `~/.claude/skills/brain-dump/extracted/anchor-edition2024-build-fix/SKILL.md`.)

---

## 2026-05-10 — Lobby_id stamped in GameConfig

**Decision**: Add `lobby_id: u64` to GameConfig, set by `init-game` from JSON args. The lobby program's `distribute_prize` reads this and gates on `gameConfig.lobby_id == lobby_pda.lobby_id`.

**Rationale**: Without this binding, an attacker could `init-game` against a foreign lobby's Leaderboard and `distribute_prize` would happily pay the attacker's lobby with the foreign match's ranking. Stamping `lobby_id` makes the leaderboard ↔ lobby relationship verifiable on-chain.
