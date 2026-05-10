// lock-position — player-signed (session key).
// Captures the LOCK moment: reads Pyth, computes [low_price, high_price]
// from the chosen leverage, sets attempt_end_ts = now + ATTEMPT_DURATION_S,
// freezes leverage, decrements attempts_left, resets points_this_round.
