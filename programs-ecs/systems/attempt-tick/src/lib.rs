// attempt-tick — keeper-signed (back).
// Per-tick band check for one player.
//   - skip if attempt_end_ts == 0 (no active attempt)
//   - if now >= attempt_end_ts → success: score += points_this_round, reset live fields
//   - else if live_price < low_price || live_price > high_price → bust: reset without banking
//   - else → still in band: points_this_round += leverage, last_block = current_slot
