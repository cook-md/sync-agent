# Daily JWT Rotation (sleep-robust)

**Date:** 2026-06-06
**Status:** Approved

## Problem

JWT token refresh only triggers within 1 hour of expiry. `JwtToken::should_refresh()`
(`src/auth/jwt.rs:70`) returns true only when `expires_in() < 1 hour`, and the hourly
refresh task (`src/auth/mod.rs:59`) acts solely on that condition.

The server issues long-lived tokens (~14 days observed). Over a 4-month production log span
there were **zero** refresh events of any kind — the 1-hour window never coincided with the
daemon being awake and checking. If the laptop is asleep through that single hour before
expiry, the token expires and the user must log in again.

We want a refresh attempt **at least every 24 hours**, decided by wall-clock elapsed time so
that sleep cannot cause the window to be missed.

## Goal

Rotate the token at least daily. The decision must be based on **wall-clock elapsed time**,
not on counting timer ticks — a paused timer during sleep is precisely the failure we are
guarding against. With daily attempts and a ~14-day token lifetime, the laptop would have to
be asleep for many consecutive days to lose the token.

## Design

### 1. Persisted "last refresh" timestamp

Add a fourth keyring key, `last_refresh`, alongside the existing `jwt_token` / `user_id` /
`user_email` keys in `src/auth/secure_session.rs`. Stored as a unix-epoch string (i64).

Rationale for the keyring (vs. a file in the config dir):
- Co-located with the session it describes.
- Cleared automatically on logout/reset via the existing `delete_with_store`.
- Follows the existing storage pattern (one more key).

On upgrade from a version that never wrote this key, a session may exist with no
`last_refresh`. That case is treated as "due" (see trigger logic) so it self-heals on the
next tick.

### 2. Single write point: `set_session`

Every token acquisition — browser login and refresh alike — funnels through
`AuthManager::set_session` (`src/auth/mod.rs:33`). `set_session` will stamp
`last_refresh = now` whenever it saves a token. The timestamp therefore always means
"when we last obtained this token."

Daemon restart does not reset it: `AuthManager::new` loads the session from the keyring and
does **not** call `set_session`, so the persisted timestamp survives restarts.

### 3. Trigger logic (the fix)

Extract a pure, testable helper used by the hourly task:

```
refresh_due(last_refresh: Option<i64>, now: i64, jwt: &JwtToken) -> bool
  = jwt.should_refresh()            // existing near-expiry safety net, kept
  || last_refresh.is_none()         // unknown -> refresh once (self-heals upgrades)
  || (now - last_refresh) >= 24h    // daily rotation, wall-clock based
```

The hourly task (`src/auth/mod.rs:59`) replaces its bare `should_refresh()` check with
`refresh_due(...)`. Because the comparison reads `Utc::now()` on each tick, a tick that fires
late after wake still observes ">= 24h elapsed" and refreshes — this is what makes the
behavior sleep-proof.

The check cadence stays hourly. On wake, tokio fires the missed tick promptly
(`MissedTickBehavior::Burst`, the default), so within an hour of waking we rotate, and the
~14-day token buffer makes an hour of slack irrelevant.

### 4. Error handling — unchanged

Existing behavior is preserved:
- Refresh failure logs `error!` and clears the session.
- Invalid JWT logs `error!` and clears the session.

No new failure modes are introduced.

## Testing

The current refresh logic is buried in an async closure and is untested. Extracting
`refresh_due` makes the decision a pure unit test.

`refresh_due` cases:
- No `last_refresh` -> due.
- Refreshed 1h ago, far from expiry -> not due.
- Refreshed 25h ago -> due.
- Near expiry but refreshed recently -> due (safety net wins).

`secure_session` cases (using the existing `MockKeyring`):
- `last_refresh` save/load roundtrip.
- `delete` clears `last_refresh` along with the other keys.

## Out of scope (YAGNI)

- No change to the hourly check cadence.
- No refresh-on-wake event hooks.
- No server-side changes.
