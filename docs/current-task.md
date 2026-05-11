# Current Task — Pre-deploy steps

All on-chain code is written and compiles. The workspace is ready for an initial deploy
to MagicBlock devnet ER, but a few infrastructure items must be cleared first.

## Step 1 — Regenerate `declare_id!`s

`bolt build` auto-generated keypairs in `target/deploy/*-keypair.json`. Each
program's `declare_id!` macro and the corresponding entry in `Anchor.toml`'s
`[programs.localnet]` section must point to those pubkeys.

Files with `declare_id!` to update:
- `programs-ecs/components/game-config/src/lib.rs:3`
- `programs-ecs/components/player-state/src/lib.rs:3`
- `programs-ecs/components/player-registry/src/lib.rs:3`
- `programs-ecs/components/leaderboard/src/lib.rs:3`
- `programs-ecs/systems/init-game/src/lib.rs:5`
- `programs-ecs/systems/spawn-player/src/lib.rs:7`
- `programs-ecs/systems/start-game/src/lib.rs:5`
- `programs-ecs/systems/lock-position/src/lib.rs:9`
- `programs-ecs/systems/attempt-tick/src/lib.rs:8`
- `programs-ecs/systems/refresh-leaderboard/src/lib.rs:6`
- `programs-ecs/systems/end-game/src/lib.rs:6`
- `programs/stay-calm-lobby/src/lib.rs:10`

Per-program command:
```bash
solana address -k target/deploy/<program_name>-keypair.json
```

Mirror those values in `Anchor.toml`'s `[programs.localnet]`.

## Step 2 — Update `LEADERBOARD_COMPONENT_ID`

`programs/stay-calm-lobby/src/constants.rs:30` currently has placeholder `11111…1`.
Replace with the deployed `leaderboard` component program ID. This binding
prevents `distribute_prize` from accepting a foreign Leaderboard account.

## Step 3 — Re-run `bolt build`

After updating IDs, rebuild to ensure consistency:
```bash
bolt build
```

## Step 4 — Tests

`tests/stay-calm.ts` only has the default `bolt init` scaffold. At minimum write:
- Happy path: init → spawn × N → start → lock → tick × M (in-band) → success bank
- Out-of-band ticks don't credit
- `attempt-tick` returns `Ok` silently when `attempt_end_ts == 0`
- `attempts_left` decremented at lock (not at resolution)
- `StaleSlot` enforced (ticks at slot ≤ `last_block + MIN_TICK_SLOT_GAP - 1`)
- Multi-player + leaderboard sort (score DESC, attempts_left DESC tie-break)
- `end-game` flips status to 2 only after `now >= game_end`
- `init-game` correctly stamps `lobby_id` from JSON args

## Step 5 — Deploy

```bash
bolt deploy
```
…to the MagicBlock devnet ER URL configured in `Anchor.toml [test.validator]`.

## Step 6 — Wire back-dev

After deploy, in `back-dev`:
- Mirror trade-fight's chain service structure for stay-calm
- Cranker calls `attempt-tick` per locked player + `refresh-leaderboard` once per tick
- Session-key issuance scoped to `LOCK_POSITION_SYSTEM` program ID only
- Lobby gateway emits `txEmitted` for keeper TXs (init / start / refresh / end)

## Step 7 — Wire front-dev

In `front-dev/src/components/games/lock-the-calm/lock-the-calm.tsx`:
- Replace mocked roster with on-chain `Leaderboard` reads
- Replace mocked countdown with `GameConfig.game_end` reads
- Replace mocked LOCK with real `lock-position` ApplySystem call (session-key signed)
- Subscribe to `onAccountChange` on `PlayerState` for live `points_this_round` ticking
